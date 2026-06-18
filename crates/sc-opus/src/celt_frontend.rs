//! CELT encoder front-end: PCM in, normalised spectrum out.
//!
//! Hand-ported to safe Rust from the front of libopus `celt/celt_encoder.c`
//! (`celt_encode_with_ec`): the stage chain that turns time-domain PCM into the
//! normalised MDCT spectrum the range coder in [`crate::celt_frame`] consumes.
//! Derivative work of libopus (BSD-3-Clause); see `LICENSE-THIRDPARTY`.
//!
//! The chain is pre-emphasis -> forward MDCT -> band energies -> log-domain
//! conversion -> per-band normalisation. The overlap history and the
//! pre-emphasis filter memory live in [`FrontendState`] so successive frames
//! window correctly across the boundary, exactly as the C keeps `in_mem` and
//! `preemph_memE` on the encoder state. The inverse path (de-normalise + inverse
//! MDCT + de-emphasis) is the decoder's job and is owned by Symphonia.

// Consumed by the CELT encode entry point; the live encoder still ships via the
// Opus FFI path.
#![allow(dead_code)]

use crate::bands::BandLayout;
use crate::mdct::{compute_mdcts, MdctConfig};
use crate::mode::CeltMode;
use crate::preemph::celt_preemphasis;
use crate::quant_bands::amp2_log2;

/// Per-encoder state the front-end carries across frames.
pub struct FrontendState {
    /// The overlap-length tail of each channel's pre-emphasised input, reused as
    /// the window history for the next frame (`cc * overlap`).
    pub in_mem: Vec<f32>,
    /// The first-order pre-emphasis filter memory, one per input channel (`cc`).
    pub preemph_mem: Vec<f32>,
}

impl FrontendState {
    /// A zeroed state (silent history), matching a freshly reset encoder.
    #[must_use]
    pub fn new(cc: usize, overlap: usize) -> Self {
        Self {
            in_mem: vec![0.0; cc * overlap],
            preemph_mem: vec![0.0; cc],
        }
    }
}

/// What [`celt_frame_frontend`] produces for one frame.
pub struct FrontendOut {
    /// The raw MDCT spectrum (`cc` planes of `n`; for a stereo-to-mono downmix
    /// the mono result is in the first plane).
    pub freq: Vec<f32>,
    /// The per-band linear energies (`c` planes of `nb_e_bands`).
    pub band_e: Vec<f32>,
    /// The per-band log2 energies with the band means removed (`c * nb_e_bands`).
    pub band_log_e: Vec<f32>,
    /// The normalised (unit per-band energy) spectrum (`c` planes of `n`).
    pub x: Vec<f32>,
}

/// `celt_frame_frontend`: run the encoder front-end for one frame.
///
/// `pcm` is interleaved at channel stride `cc` (the input channel count); `c` is
/// the coded channel count (`1` downmixes a stereo input). `lm` selects the
/// frame size (`short_mdct_size << lm`), `end` the number of coded bands.
/// `is_transient` switches to short blocks; `upsample > 1` zero-stuffs the input.
/// `state` carries the overlap and pre-emphasis memory across calls.
#[allow(clippy::too_many_arguments)]
pub fn celt_frame_frontend(
    mode: &CeltMode,
    pcm: &[f32],
    lm: i32,
    c: usize,
    cc: usize,
    end: usize,
    is_transient: bool,
    upsample: usize,
    clip: bool,
    state: &mut FrontendState,
) -> FrontendOut {
    let overlap = mode.overlap;
    let n = mode.short_mdct_size << lm;
    let m = 1usize << lm;
    let nb = mode.nb_e_bands;

    // Build each channel's input region: the previous frame's overlap tail,
    // followed by this frame's pre-emphasised samples.
    let mut input = vec![0.0f32; cc * (n + overlap)];
    for ch in 0..cc {
        let base = ch * (n + overlap);
        input[base..base + overlap]
            .copy_from_slice(&state.in_mem[ch * overlap..(ch + 1) * overlap]);
        celt_preemphasis(
            &pcm[ch..],
            &mut input[base + overlap..base + overlap + n],
            n,
            cc,
            upsample,
            mode.preemph_coef,
            &mut state.preemph_mem[ch],
            clip,
        );
        // Carry this frame's tail as the next frame's window history.
        state.in_mem[ch * overlap..(ch + 1) * overlap]
            .copy_from_slice(&input[base + n..base + n + overlap]);
    }

    // Forward MDCT (short blocks on a transient frame).
    let short_blocks = if is_transient { m } else { 0 };
    let cfg = MdctConfig {
        overlap,
        short_mdct_size: mode.short_mdct_size,
        window: &mode.window,
    };
    let mut freq = vec![0.0f32; cc * n];
    compute_mdcts(
        &cfg,
        short_blocks,
        lm as usize,
        &input,
        &mut freq,
        c,
        cc,
        upsample,
    );

    // Band energies, log-domain conversion, and per-band normalisation.
    let layout = BandLayout {
        e_bands: mode.e_bands,
        short_mdct_size: mode.short_mdct_size,
        nb_e_bands: nb,
    };
    let mut band_e = vec![0.0f32; c * nb];
    layout.compute_band_energies(&freq, &mut band_e, end, c, lm as u32);
    let mut band_log_e = vec![0.0f32; c * nb];
    amp2_log2(nb, end, end, &band_e, &mut band_log_e, c);
    let mut x = vec![0.0f32; c * n];
    layout.normalise_bands(&freq, &mut x, &band_e, end, c, m);

    FrontendOut {
        freq,
        band_e,
        band_log_e,
        x,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::celt_frame::{
        decode_celt_frame, encode_celt_frame, CeltDecoderState, CeltEncoderState, FrameParams,
        SPREAD_NORMAL,
    };
    use crate::mode::celt_mode_48k;
    use crate::quant_bands::E_MEANS;

    /// A deterministic tonal+noise PCM frame for `cc` interleaved channels.
    fn make_pcm(n: usize, cc: usize, salt: u32) -> Vec<f32> {
        let mut s = salt.wrapping_add(1);
        let mut rng = || {
            s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (s >> 9) as f32 / (1u32 << 23) as f32 - 0.5
        };
        let mut pcm = vec![0.0f32; n * cc];
        for i in 0..n {
            let t = i as f32;
            // A few partials plus a little noise, scaled into a typical PCM range.
            let base = (0.02 * t).sin() + 0.5 * (0.05 * t).sin() + 0.25 * (0.11 * t).sin();
            for ch in 0..cc {
                let detune = 1.0 + 0.01 * ch as f32;
                pcm[i * cc + ch] = 3000.0 * (base * detune + 0.05 * rng());
            }
        }
        pcm
    }

    /// Energy-weighted signal-to-noise ratio (dB) between two spectra.
    fn snr_db(reference: &[f32], reconstructed: &[f32]) -> f32 {
        let mut sig = 0.0f64;
        let mut err = 0.0f64;
        for (&r, &q) in reference.iter().zip(reconstructed) {
            sig += (r as f64) * (r as f64);
            let d = (r - q) as f64;
            err += d * d;
        }
        10.0 * ((sig / err.max(1e-30)).log10()) as f32
    }

    fn mono_params() -> FrameParams {
        FrameParams {
            start: 0,
            end: 21,
            lm: 3,
            c: 1,
            is_transient: false,
            spread: SPREAD_NORMAL,
            tf_select: 0,
            alloc_trim: 5,
            intensity: 0,
            dual_stereo: 0,
            complexity: 5,
            disable_inv: false,
        }
    }

    /// PCM -> front-end -> encode -> bytes -> decode -> de-normalise, then compare
    /// the reconstructed spectrum against the encoder's own MDCT spectrum. This
    /// is the full lossy chain minus the inverse MDCT (Symphonia owns that), so it
    /// is checked with an SNR tolerance, never bit-exact.
    #[test]
    fn pcm_frontend_round_trips_within_tolerance_mono() {
        let mode = celt_mode_48k();
        let p = mono_params();
        let nb = mode.nb_e_bands;
        let n = mode.short_mdct_size << p.lm;
        let pcm = make_pcm(n, 1, 5);

        let mut st = FrontendState::new(1, mode.overlap);
        let fe = celt_frame_frontend(&mode, &pcm, p.lm, 1, 1, 21, false, 1, false, &mut st);

        let mut band_log_e = fe.band_log_e.clone();
        let mut x = fe.x.clone();
        let mut tf_res = vec![0i32; nb];
        let mut offsets = vec![0i32; nb];
        let mut enc_state = CeltEncoderState::new(1, nb);
        let bytes = encode_celt_frame(
            &mode,
            &p,
            &mut band_log_e,
            &fe.band_e,
            &mut x,
            &mut tf_res,
            &mut offsets,
            150,
            &mut enc_state,
        );
        let mut dec_state = CeltDecoderState::new(1, nb);
        let dec = decode_celt_frame(&mode, &bytes, 0, 21, 3, 1, 5, false, &mut dec_state);
        // The bitstream chain is bit-exact (verified in celt_frame); here we check
        // the audio path: de-normalise the decoded spectrum and compare to the
        // encoder's MDCT output.
        assert_eq!(enc_state.old_band_e, dec.old_band_e);
        let mut recon = vec![0.0f32; n];
        let layout = BandLayout {
            e_bands: mode.e_bands,
            short_mdct_size: mode.short_mdct_size,
            nb_e_bands: nb,
        };
        layout.denormalise_bands(
            &dec.x,
            &mut recon,
            &dec.old_band_e,
            &E_MEANS,
            0,
            21,
            8,
            1,
            false,
        );
        let snr = snr_db(&fe.freq[..n], &recon);
        assert!(snr > 12.0, "front-end roundtrip SNR too low: {snr} dB");
    }

    #[test]
    fn frontend_overlap_history_carries_across_frames() {
        let mode = celt_mode_48k();
        let n = mode.short_mdct_size << 3;
        let pcm = make_pcm(n, 1, 11);
        // A fresh state windows against silence; a primed state windows against a
        // real previous frame, so the two MDCTs must differ at the leading edge.
        let mut fresh = FrontendState::new(1, mode.overlap);
        let mut primed = FrontendState::new(1, mode.overlap);
        let prev = make_pcm(n, 1, 12);
        let _ = celt_frame_frontend(&mode, &prev, 3, 1, 1, 21, false, 1, false, &mut primed);

        let a = celt_frame_frontend(&mode, &pcm, 3, 1, 1, 21, false, 1, false, &mut fresh);
        let b = celt_frame_frontend(&mode, &pcm, 3, 1, 1, 21, false, 1, false, &mut primed);
        let diff: f32 = a.freq.iter().zip(&b.freq).map(|(x, y)| (x - y).abs()).sum();
        assert!(diff > 1e-3, "overlap history had no effect: {diff}");
    }

    #[test]
    fn frontend_normalised_bands_are_unit_energy() {
        let mode = celt_mode_48k();
        let n = mode.short_mdct_size << 3;
        let pcm = make_pcm(n, 1, 21);
        let mut st = FrontendState::new(1, mode.overlap);
        let fe = celt_frame_frontend(&mode, &pcm, 3, 1, 1, 21, false, 1, false, &mut st);
        // Each coded band of the normalised spectrum has unit energy by construction.
        for b in 0..21 {
            let lo = 8 * mode.e_bands[b] as usize;
            let hi = 8 * mode.e_bands[b + 1] as usize;
            let e: f32 = fe.x[lo..hi].iter().map(|v| v * v).sum();
            assert!((e - 1.0).abs() < 1e-3, "band {b} energy {e} not unit");
        }
    }
}
