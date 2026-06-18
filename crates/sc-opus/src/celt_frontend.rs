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
use crate::pitch::{run_prefilter, PostfilterParams, PrefilterState};
use crate::preemph::celt_preemphasis;
use crate::quant_bands::amp2_log2;

/// Per-encoder state the front-end carries across frames.
pub struct FrontendState {
    /// The first-order pre-emphasis filter memory, one per input channel (`cc`).
    pub preemph_mem: Vec<f32>,
    /// The pitch pre-filter state: comb history, the overlap window memory, and
    /// the previous frame's period/gain/tapset.
    pub prefilter: PrefilterState,
}

impl FrontendState {
    /// A zeroed state (silent history), matching a freshly reset encoder.
    #[must_use]
    pub fn new(cc: usize, overlap: usize) -> Self {
        Self {
            preemph_mem: vec![0.0; cc],
            prefilter: PrefilterState::new(cc, overlap),
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

/// `frontend_preprocess`: phase one of the front-end — pre-emphasis and the
/// pitch pre-filter. Returns the time-domain analysis buffer (`cc` planes of
/// `n + overlap`, the overlap prefix being the comb-filtered previous tail) and
/// the post-filter decision. The transient detector runs on this buffer before
/// the MDCT, so the two phases are separate entry points.
#[allow(clippy::too_many_arguments)]
pub fn frontend_preprocess(
    mode: &CeltMode,
    pcm: &[f32],
    lm: i32,
    cc: usize,
    upsample: usize,
    clip: bool,
    tapset: usize,
    pf_enabled: bool,
    nb_available_bytes: i32,
    state: &mut FrontendState,
) -> (Vec<f32>, PostfilterParams) {
    let overlap = mode.overlap;
    let n = mode.short_mdct_size << lm;

    // Pre-emphasise this frame's samples into each channel's frame region; the
    // overlap prefix is restored from the pre-filter's window memory inside
    // `run_prefilter`, which then comb-filters the result in place.
    let mut input = vec![0.0f32; cc * (n + overlap)];
    for ch in 0..cc {
        let base = ch * (n + overlap);
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
    }
    let pf = run_prefilter(
        &mut input,
        n,
        cc,
        overlap,
        mode.short_mdct_size,
        &mode.window,
        tapset,
        pf_enabled,
        nb_available_bytes,
        &mut state.prefilter,
    );
    (input, pf)
}

/// `frontend_transform`: phase two of the front-end — the forward MDCT, band
/// energies, log-domain conversion and per-band normalisation of a pre-processed
/// time-domain `input` buffer (as produced by [`frontend_preprocess`]).
/// `is_transient` selects short MDCT blocks.
#[allow(clippy::too_many_arguments)]
pub fn frontend_transform(
    mode: &CeltMode,
    input: &[f32],
    lm: i32,
    c: usize,
    cc: usize,
    end: usize,
    is_transient: bool,
    upsample: usize,
) -> FrontendOut {
    let overlap = mode.overlap;
    let n = mode.short_mdct_size << lm;
    let m = 1usize << lm;
    let nb = mode.nb_e_bands;

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
        input,
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

/// `celt_frame_frontend`: run both front-end phases for one frame with a fixed
/// transient decision. A convenience wrapper over [`frontend_preprocess`] +
/// [`frontend_transform`] for callers that decide the transient flag elsewhere.
///
/// `pcm` is interleaved at channel stride `cc` (the input channel count); `c` is
/// the coded channel count (`1` downmixes a stereo input). `lm` selects the
/// frame size (`short_mdct_size << lm`), `end` the number of coded bands.
/// `is_transient` switches to short blocks; `upsample > 1` zero-stuffs the input.
/// `tapset` is this frame's post-filter tapset; `pf_enabled` gates the pitch
/// pre-filter and `nb_available_bytes` drives its enable threshold.
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
    tapset: usize,
    pf_enabled: bool,
    nb_available_bytes: i32,
    state: &mut FrontendState,
) -> (FrontendOut, PostfilterParams) {
    let (input, pf) = frontend_preprocess(
        mode,
        pcm,
        lm,
        cc,
        upsample,
        clip,
        tapset,
        pf_enabled,
        nb_available_bytes,
        state,
    );
    let out = frontend_transform(mode, &input, lm, c, cc, end, is_transient, upsample);
    (out, pf)
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
        let (fe, _pf) = celt_frame_frontend(
            &mode, &pcm, p.lm, 1, 1, 21, false, 1, false, 0, false, 150, &mut st,
        );

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
            None,
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
        let _ = celt_frame_frontend(
            &mode,
            &prev,
            3,
            1,
            1,
            21,
            false,
            1,
            false,
            0,
            false,
            150,
            &mut primed,
        );

        let (a, _) = celt_frame_frontend(
            &mode, &pcm, 3, 1, 1, 21, false, 1, false, 0, false, 150, &mut fresh,
        );
        let (b, _) = celt_frame_frontend(
            &mode,
            &pcm,
            3,
            1,
            1,
            21,
            false,
            1,
            false,
            0,
            false,
            150,
            &mut primed,
        );
        let diff: f32 = a.freq.iter().zip(&b.freq).map(|(x, y)| (x - y).abs()).sum();
        assert!(diff > 1e-3, "overlap history had no effect: {diff}");
    }

    #[test]
    fn frontend_normalised_bands_are_unit_energy() {
        let mode = celt_mode_48k();
        let n = mode.short_mdct_size << 3;
        let pcm = make_pcm(n, 1, 21);
        let mut st = FrontendState::new(1, mode.overlap);
        let (fe, _pf) = celt_frame_frontend(
            &mode, &pcm, 3, 1, 1, 21, false, 1, false, 0, false, 150, &mut st,
        );
        // Each coded band of the normalised spectrum has unit energy by construction.
        for b in 0..21 {
            let lo = 8 * mode.e_bands[b] as usize;
            let hi = 8 * mode.e_bands[b + 1] as usize;
            let e: f32 = fe.x[lo..hi].iter().map(|v| v * v).sum();
            assert!((e - 1.0).abs() < 1e-3, "band {b} energy {e} not unit");
        }
    }

    /// A harmonically rich, periodic PCM frame (a pure tone would be annihilated
    /// by the pre-filter's LPC whitening).
    fn periodic_pcm(n: usize, period: f32) -> Vec<f32> {
        (0..n)
            .map(|i| {
                let phase = i as f32 / period * std::f32::consts::TAU;
                let s: f32 = (1..=8).map(|h| (phase * h as f32).sin() / h as f32).sum();
                3000.0 * s
            })
            .collect()
    }

    #[test]
    fn frontend_postfilter_engages_and_round_trips_through_the_frame() {
        // With the pre-filter enabled on a strongly periodic frame the front-end
        // returns an "on" decision; encoding it with `encode_celt_frame` and
        // decoding must recover the same pitch / gain index / tapset bit-exactly.
        let mode = celt_mode_48k();
        let p = mono_params();
        let nb = mode.nb_e_bands;
        let n = mode.short_mdct_size << p.lm;
        let pcm = periodic_pcm(n, 96.0);

        let mut st = FrontendState::new(1, mode.overlap);
        let (fe, pf) = celt_frame_frontend(
            &mode, &pcm, p.lm, 1, 1, 21, false, 1, false, 0, true, 100, &mut st,
        );
        assert!(pf.pf_on, "post-filter should engage on a periodic frame");

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
            200,
            Some(&pf),
            &mut enc_state,
        );
        let mut dec_state = CeltDecoderState::new(1, nb);
        let dec = decode_celt_frame(&mode, &bytes, 0, 21, 3, 1, 5, false, &mut dec_state);
        let dpf = dec
            .postfilter
            .expect("post-filter params should decode back");
        assert_eq!(dpf.pitch_index, pf.pitch_index, "pitch survived");
        assert_eq!(dpf.qg, pf.qg, "gain index survived");
        assert_eq!(dpf.tapset, pf.tapset, "tapset survived");
        assert!((dpf.gain - pf.gain).abs() < 1e-9, "gain survived");
    }
}
