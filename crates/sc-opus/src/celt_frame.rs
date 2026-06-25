//! CELT frame orchestration: the encode/decode core that wires the per-stage
//! bricks into one range-coded CELT frame.
//!
//! Hand-ported to safe Rust from libopus `celt/celt_encoder.c`
//! (`celt_encode_with_ec`) and `celt/celt_decoder.c` (`celt_decode_with_ec`):
//! the exact field order and budget bookkeeping that string the energy, tf,
//! spreading, dynalloc, allocation and residual coders together. Derivative work
//! of libopus (BSD-3-Clause); see `LICENSE-THIRDPARTY`.
//!
//! This is the integration layer that operates on the *normalised spectrum*
//! domain (the MDCT front-end in `mdct.rs` produces that spectrum and is
//! validated separately). [`encode_celt_frame`] forces resynthesis so the
//! encoder reconstructs exactly the spectrum the decoder produces, which lets
//! the tests round-trip a whole frame bit-exactly. Coarse energy is the two-pass
//! intra/inter coder with cross-frame energy state threaded through
//! [`CeltEncoderState`] / [`CeltDecoderState`] (the reconstructed energies, the
//! two previous frames' log energies, the consecutive-transient counter and the
//! PVQ seed). Transient frames code the anti-collapse bit (gated by
//! `consec_transient < 2`) and run the resynthesis refill on both sides using the
//! `old_log_e` / `old_log_e2` history, so the transient path is complete too.

// Consumed by the CELT encode/decode entry points; the live encoder still ships
// via the Opus FFI path.
#![allow(dead_code)]

use crate::allocation::{
    clt_compute_allocation, decode_dynalloc_boost, encode_dynalloc_boost, init_caps, AllocInput,
};
use crate::bands::anti_collapse;
use crate::mode::CeltMode;
use crate::pitch::{decode_postfilter, encode_postfilter, PostfilterParams};
use crate::quant_all_bands::{quant_all_bands, AllBandsInput};
use crate::quant_band::Coder;
use crate::quant_bands::{
    decode_intra_flag, quant_coarse_energy, quant_energy_finalise, quant_fine_energy,
    unquant_coarse_energy, unquant_energy_finalise, unquant_fine_energy,
};
use crate::range_coder::{RangeDecoder, RangeEncoder};
use crate::tf::{tf_decode, tf_encode};
use crate::theta::BITRES;

/// `spread_icdf` (libopus `celt.h`): the cumulative model for the spreading flag.
const SPREAD_ICDF: [u8; 4] = [25, 23, 2, 0];
/// `trim_icdf` (libopus `celt.h`): the cumulative model for the allocation trim.
const TRIM_ICDF: [u8; 11] = [126, 124, 119, 109, 87, 41, 19, 9, 4, 2, 0];
/// `SPREAD_NORMAL`: the default spreading decision when none is coded.
pub const SPREAD_NORMAL: i32 = 2;

/// The encoder-chosen per-frame parameters [`encode_celt_frame`] transmits.
pub struct FrameParams {
    pub start: usize,
    pub end: usize,
    pub lm: i32,
    pub c: usize,
    pub is_transient: bool,
    pub spread: i32,
    pub tf_select: i32,
    pub alloc_trim: i32,
    pub intensity: i32,
    pub dual_stereo: i32,
    pub complexity: i32,
    pub disable_inv: bool,
}

/// libopus initialises the log-energy histories to `-28 dB`.
const ENERGY_INIT: f32 = -28.0;
/// CELT's maximum channel count; the log-energy histories are always sized for
/// it because [`anti_collapse`] takes a cross-channel max even on mono frames.
const MAX_CHANNELS: usize = 2;

/// The CELT encoder's cross-frame state: the reconstructed band energies (the
/// inter-prediction base), the previous two frames' log energies (for
/// anti-collapse), the intra/inter distortion accumulator, the consecutive
/// transient counter (gates anti-collapse), and the PVQ noise seed.
pub struct CeltEncoderState {
    pub old_band_e: Vec<f32>,
    pub old_log_e: Vec<f32>,
    pub old_log_e2: Vec<f32>,
    pub delayed_intra: f32,
    pub consec_transient: i32,
    pub rng: u32,
}

impl CeltEncoderState {
    /// A freshly reset encoder state for `c` channels (matching libopus
    /// `celt_encoder_init`: energies zeroed, log histories at `-28 dB`).
    #[must_use]
    pub fn new(c: usize, nb_e_bands: usize) -> Self {
        Self {
            old_band_e: vec![0.0; c * nb_e_bands],
            old_log_e: vec![ENERGY_INIT; MAX_CHANNELS * nb_e_bands],
            old_log_e2: vec![ENERGY_INIT; MAX_CHANNELS * nb_e_bands],
            delayed_intra: 0.0,
            consec_transient: 0,
            rng: 0,
        }
    }
}

/// The CELT decoder's cross-frame state, mirroring the encoder's: the
/// reconstructed band energies and the previous two frames' log energies, plus
/// the PVQ noise seed kept in lock-step with the encoder.
pub struct CeltDecoderState {
    pub old_band_e: Vec<f32>,
    pub old_log_e: Vec<f32>,
    pub old_log_e2: Vec<f32>,
    pub rng: u32,
}

impl CeltDecoderState {
    /// A freshly reset decoder state for `c` channels (matching the encoder).
    #[must_use]
    pub fn new(c: usize, nb_e_bands: usize) -> Self {
        Self {
            old_band_e: vec![0.0; c * nb_e_bands],
            old_log_e: vec![ENERGY_INIT; MAX_CHANNELS * nb_e_bands],
            old_log_e2: vec![ENERGY_INIT; MAX_CHANNELS * nb_e_bands],
            rng: 0,
        }
    }
}

/// The frame size (MDCT bins per channel) for this mode and `lm`.
fn frame_size(mode: &CeltMode, lm: i32) -> usize {
    mode.short_mdct_size << lm
}

/// `encode_celt_frame`: range-code one CELT frame from the normalised spectrum.
///
/// `band_log_e` holds the per-band log2 energies (consumed by coarse energy);
/// `band_e` the linear energies (the residual coder keeps them for API parity
/// but does not read them — the decoder passes none); `x` is the normalised
/// spectrum (`c` channels of `frame_size` bins) and is overwritten in place with
/// the resynthesised spectrum. `tf_res` / `offsets` are rewritten as the coders
/// consume them. `state` carries the cross-frame energy histories, the PVQ noise
/// seed and the consecutive-transient counter, all updated in place. Returns the
/// packet bytes, or an error if the range coder overflowed the byte budget.
#[allow(clippy::too_many_arguments)]
pub fn encode_celt_frame(
    mode: &CeltMode,
    params: &FrameParams,
    band_log_e: &mut [f32],
    band_e: &[f32],
    x: &mut [f32],
    tf_res: &mut [i32],
    offsets: &mut [i32],
    nb_bytes: usize,
    pf: Option<&PostfilterParams>,
    state: &mut CeltEncoderState,
) -> Result<Vec<u8>, sc_core::Error> {
    let nb = mode.nb_e_bands;
    let (c, lm, start, end) = (params.c, params.lm, params.start, params.end);
    let n = frame_size(mode, lm);
    let total_bits = (nb_bytes as i32) * 8;
    let lmu = lm as usize;
    let seed = state.rng;

    // Consecutive-transient counter gates anti-collapse (isolated transients
    // only). Updated here so the very first transient frame still gets it.
    if params.is_transient {
        state.consec_transient += 1;
    } else {
        state.consec_transient = 0;
    }

    let mut enc = RangeEncoder::new(nb_bytes as u32);

    // 1. Silence flag (only at the very start of the stream).
    enc.enc_bit_logp(false, 15);
    // 2. Post-filter section (off when no params supplied).
    let pf_off = PostfilterParams {
        pf_on: false,
        pitch_index: 0,
        gain: 0.0,
        qg: 0,
        tapset: 0,
    };
    encode_postfilter(&mut enc, pf.unwrap_or(&pf_off), start, total_bits);
    // 3. Transient flag.
    if lm > 0 && enc.ec_tell() + 3 <= total_bits {
        enc.enc_bit_logp(params.is_transient, 3);
    }

    // 4. Coarse energy (two-pass intra/inter selection, cross-frame state).
    let mut error = vec![0.0f32; c * nb];
    let two_pass = params.complexity >= 4;
    quant_coarse_energy(
        nb,
        start,
        end,
        end,
        band_log_e,
        &mut state.old_band_e,
        total_bits,
        &mut error,
        &mut enc,
        c,
        lmu,
        nb_bytes as i32,
        false,
        &mut state.delayed_intra,
        two_pass,
        0,
        false,
    );

    // 5. Time-frequency resolution flags.
    tf_encode(
        start,
        end,
        params.is_transient,
        tf_res,
        lm,
        params.tf_select,
        &mut enc,
    );

    // 6. Spreading decision.
    if enc.ec_tell() + 4 <= total_bits {
        enc.enc_icdf(params.spread as usize, &SPREAD_ICDF, 5);
    }

    // 7. Dynalloc boosts.
    let cap = init_caps(&mode.cache[lmu].caps, mode.e_bands, nb, lm, c as i32);
    let total_boost = encode_dynalloc_boost(
        &mut enc,
        mode.e_bands,
        start,
        end,
        c as i32,
        lm,
        &cap,
        total_bits,
        offsets,
    );

    // 8. Allocation trim.
    let mut alloc_trim = 5;
    if enc.ec_tell_frac() as i32 + (6 << BITRES) <= (total_bits << BITRES) - total_boost {
        alloc_trim = params.alloc_trim;
        enc.enc_icdf(alloc_trim as usize, &TRIM_ICDF, 7);
    }

    // 9. Bit budget and the anti-collapse reservation.
    let mut bits = (total_bits << BITRES) - enc.ec_tell_frac() as i32 - 1;
    let anti_collapse_rsv = if params.is_transient && lm >= 2 && bits >= ((lm + 2) << BITRES) {
        1 << BITRES
    } else {
        0
    };
    bits -= anti_collapse_rsv;

    // 10. Bit allocation (codes skip / intensity / dual-stereo flags).
    let inp = alloc_input(mode, start, end, c, lm);
    let mut pulses = vec![0i32; nb];
    let mut ebits = vec![0i32; nb];
    let mut fine_priority = vec![0i32; nb];
    let alloc = {
        let mut coder = Coder::Enc(&mut enc);
        clt_compute_allocation(
            &inp,
            &mut coder,
            offsets,
            &cap,
            alloc_trim,
            params.intensity,
            params.dual_stereo,
            bits,
            &mut pulses,
            &mut ebits,
            &mut fine_priority,
            0,
            end as i32 - 1,
        )
    };

    // 11. Fine energy.
    quant_fine_energy(
        nb,
        start,
        end,
        &mut state.old_band_e,
        &mut error,
        &ebits,
        &mut enc,
        c,
    );

    // 12. Residual (PVQ), with resynthesis to reconstruct the spectrum.
    let ab = all_bands_input(
        mode,
        params,
        band_e,
        &pulses,
        tf_res,
        nb_bytes,
        anti_collapse_rsv,
        &alloc,
    );
    let mut collapse_masks = vec![0u8; c * nb];
    let new_seed = {
        let mut coder = Coder::Enc(&mut enc);
        if c == 2 {
            let (x0, x1) = x.split_at_mut(n);
            quant_all_bands(
                &ab,
                &mut coder,
                x0,
                Some(x1),
                &mut collapse_masks,
                alloc.balance,
                alloc.dual_stereo != 0,
                true,
                seed,
            )
        } else {
            quant_all_bands(
                &ab,
                &mut coder,
                &mut x[..n],
                None,
                &mut collapse_masks,
                alloc.balance,
                false,
                true,
                seed,
            )
        }
    };

    // 13. Anti-collapse flag: on for isolated transients (consec_transient < 2).
    let anti_collapse_on = anti_collapse_rsv > 0 && state.consec_transient < 2;
    if anti_collapse_rsv > 0 {
        enc.enc_bits(u32::from(anti_collapse_on), 1);
    }

    // 14. Finalise: spend the leftover bits on 1-bit energy refinements.
    quant_energy_finalise(
        nb,
        start,
        end,
        &mut state.old_band_e,
        &mut error,
        &ebits,
        &fine_priority,
        (nb_bytes as i32) * 8 - enc.ec_tell(),
        &mut enc,
        c,
    );

    // 15. Anti-collapse resynthesis: refill collapsed transient sub-blocks with
    // energy-scaled noise so the decoder reconstructs the identical spectrum.
    if anti_collapse_on {
        anti_collapse(
            mode.e_bands,
            nb,
            x,
            &collapse_masks,
            lm,
            c,
            n,
            start,
            end,
            &state.old_band_e,
            &state.old_log_e,
            &state.old_log_e2,
            &pulses,
            new_seed,
        );
    }

    // Surface a budget overflow rather than emitting a silently corrupted
    // (truncated) packet.
    if enc.is_error() {
        return Err(sc_core::Error::InvalidInput(
            "Opus range coder overflowed the frame budget",
        ));
    }

    // Carry the range register as next frame's PVQ seed, then roll the energy
    // histories forward (this frame becomes the previous one). `old_band_e` only
    // spans the coded channels; any unused history plane keeps its prior value.
    state.rng = enc.rng();
    state.old_log_e2.copy_from_slice(&state.old_log_e);
    let cn = c * nb;
    state.old_log_e[..cn].copy_from_slice(&state.old_band_e);

    Ok(enc.done())
}

/// The result of [`decode_celt_frame`].
pub struct DecodedFrame {
    /// The reconstructed normalised spectrum (`c` channels of `frame_size` bins).
    pub x: Vec<f32>,
    /// The decoded per-band log2 energies.
    pub old_band_e: Vec<f32>,
    /// The updated LCG seed.
    pub seed: u32,
    /// Whether the frame was flagged transient.
    pub is_transient: bool,
    /// The decoded spreading decision.
    pub spread: i32,
    /// The decoded post-filter parameters, or `None` when the filter is off.
    pub postfilter: Option<PostfilterParams>,
}

/// `decode_celt_frame`: the decoder side of [`encode_celt_frame`]; recovers every
/// per-frame parameter and the normalised spectrum from the packet.
#[allow(clippy::too_many_arguments)]
pub fn decode_celt_frame(
    mode: &CeltMode,
    bytes: &[u8],
    start: usize,
    end: usize,
    lm: i32,
    c: usize,
    complexity: i32,
    disable_inv: bool,
    state: &mut CeltDecoderState,
) -> DecodedFrame {
    let nb = mode.nb_e_bands;
    let n = frame_size(mode, lm);
    let total_bits = (bytes.len() as i32) * 8;
    let lmu = lm as usize;
    let seed = state.rng;

    let mut dec = RangeDecoder::new(bytes);

    // 1. Silence flag.
    let _silence = if dec.ec_tell() == 1 {
        dec.dec_bit_logp(15)
    } else {
        false
    };
    // 2. Post-filter section.
    let postfilter = decode_postfilter(&mut dec, start, total_bits);
    // 3. Transient flag.
    let is_transient = if lm > 0 && dec.ec_tell() + 3 <= total_bits {
        dec.dec_bit_logp(3)
    } else {
        false
    };

    // 4. Coarse energy.
    let intra = decode_intra_flag(&mut dec, total_bits);
    unquant_coarse_energy(
        nb,
        start,
        end,
        &mut state.old_band_e,
        intra,
        &mut dec,
        c,
        lmu,
    );

    // 5. Time-frequency resolution flags.
    let mut tf_res = vec![0i32; nb];
    tf_decode(start, end, is_transient, &mut tf_res, lm, &mut dec);

    // 6. Spreading decision.
    let spread = if dec.ec_tell() + 4 <= total_bits {
        dec.dec_icdf(&SPREAD_ICDF, 5) as i32
    } else {
        SPREAD_NORMAL
    };

    // 7. Dynalloc boosts.
    let cap = init_caps(&mode.cache[lmu].caps, mode.e_bands, nb, lm, c as i32);
    let mut offsets = vec![0i32; nb];
    decode_dynalloc_boost(
        &mut dec,
        mode.e_bands,
        start,
        end,
        c as i32,
        lm,
        &cap,
        total_bits,
        &mut offsets,
    );
    let total_boost: i32 = offsets.iter().sum();

    // 8. Allocation trim.
    let alloc_trim =
        if dec.ec_tell_frac() as i32 + (6 << BITRES) <= (total_bits << BITRES) - total_boost {
            dec.dec_icdf(&TRIM_ICDF, 7) as i32
        } else {
            5
        };

    // 9. Bit budget and anti-collapse reservation.
    let mut bits = (total_bits << BITRES) - dec.ec_tell_frac() as i32 - 1;
    let anti_collapse_rsv = if is_transient && lm >= 2 && bits >= ((lm + 2) << BITRES) {
        1 << BITRES
    } else {
        0
    };
    bits -= anti_collapse_rsv;

    // 10. Bit allocation.
    let inp = alloc_input(mode, start, end, c, lm);
    let mut pulses = vec![0i32; nb];
    let mut ebits = vec![0i32; nb];
    let mut fine_priority = vec![0i32; nb];
    let alloc = {
        let mut coder = Coder::Dec(&mut dec);
        clt_compute_allocation(
            &inp,
            &mut coder,
            &offsets,
            &cap,
            alloc_trim,
            0,
            0,
            bits,
            &mut pulses,
            &mut ebits,
            &mut fine_priority,
            0,
            0,
        )
    };

    // 11. Fine energy.
    unquant_fine_energy(nb, start, end, &mut state.old_band_e, &ebits, &mut dec, c);

    // 12. Residual.
    let dummy_band_e = vec![1.0f32; c * nb];
    let params = FrameParams {
        start,
        end,
        lm,
        c,
        is_transient,
        spread,
        tf_select: 0,
        alloc_trim,
        intensity: alloc.intensity,
        dual_stereo: alloc.dual_stereo,
        complexity,
        disable_inv,
    };
    let ab = all_bands_input(
        mode,
        &params,
        &dummy_band_e,
        &pulses,
        &tf_res,
        bytes.len(),
        anti_collapse_rsv,
        &alloc,
    );
    let mut x = vec![0.0f32; c * n];
    let mut collapse_masks = vec![0u8; c * nb];
    let new_seed = {
        let mut coder = Coder::Dec(&mut dec);
        if c == 2 {
            let (x0, x1) = x.split_at_mut(n);
            quant_all_bands(
                &ab,
                &mut coder,
                x0,
                Some(x1),
                &mut collapse_masks,
                alloc.balance,
                alloc.dual_stereo != 0,
                true,
                seed,
            )
        } else {
            quant_all_bands(
                &ab,
                &mut coder,
                &mut x[..n],
                None,
                &mut collapse_masks,
                alloc.balance,
                false,
                true,
                seed,
            )
        }
    };

    // 13. Anti-collapse flag.
    let anti_collapse_on = anti_collapse_rsv > 0 && dec.dec_bits(1) != 0;

    // 14. Finalise.
    unquant_energy_finalise(
        nb,
        start,
        end,
        &mut state.old_band_e,
        &ebits,
        &fine_priority,
        total_bits - dec.ec_tell(),
        &mut dec,
        c,
    );

    // 15. Anti-collapse resynthesis (identical to the encoder's, so the spectra
    // stay bit-exact).
    if anti_collapse_on {
        anti_collapse(
            mode.e_bands,
            nb,
            &mut x,
            &collapse_masks,
            lm,
            c,
            n,
            start,
            end,
            &state.old_band_e,
            &state.old_log_e,
            &state.old_log_e2,
            &pulses,
            new_seed,
        );
    }

    // Carry the range register forward and roll the energy histories, mirroring
    // the encoder.
    state.rng = dec.rng();
    state.old_log_e2.copy_from_slice(&state.old_log_e);
    let cn = c * nb;
    state.old_log_e[..cn].copy_from_slice(&state.old_band_e);

    DecodedFrame {
        x,
        old_band_e: state.old_band_e.to_vec(),
        seed: new_seed,
        is_transient,
        spread,
        postfilter,
    }
}

/// Builds the [`AllocInput`] for this mode / frame.
fn alloc_input(mode: &CeltMode, start: usize, end: usize, c: usize, lm: i32) -> AllocInput<'_> {
    AllocInput {
        e_bands: mode.e_bands,
        log_n: mode.log_n,
        alloc_vectors: mode.alloc_vectors,
        nb_alloc_vectors: mode.nb_alloc_vectors,
        nb_e_bands: mode.nb_e_bands,
        start,
        end,
        c: c as i32,
        lm,
    }
}

/// Builds the [`AllBandsInput`] for the residual coder.
#[allow(clippy::too_many_arguments)]
fn all_bands_input<'a>(
    mode: &'a CeltMode,
    params: &FrameParams,
    band_e: &'a [f32],
    pulses: &'a [i32],
    tf_res: &'a [i32],
    nb_bytes: usize,
    anti_collapse_rsv: i32,
    alloc: &crate::allocation::Allocation,
) -> AllBandsInput<'a> {
    AllBandsInput {
        cache: &mode.cache[params.lm as usize],
        e_bands: mode.e_bands,
        log_n: mode.log_n,
        nb_e_bands: mode.nb_e_bands,
        eff_e_bands: mode.eff_e_bands,
        start: params.start,
        end: params.end,
        band_e,
        pulses,
        tf_res,
        short_blocks: params.is_transient,
        spread: params.spread,
        intensity: alloc.intensity as usize,
        total_bits: (nb_bytes as i32) * (8 << BITRES) - anti_collapse_rsv,
        lm: params.lm,
        coded_bands: alloc.coded_bands,
        complexity: params.complexity,
        disable_inv: params.disable_inv,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mode::celt_mode_48k;

    /// Builds a per-band unit-norm normalised spectrum (what the residual coder
    /// expects), deterministically seeded by `salt`.
    fn make_normalised_spectrum(mode: &CeltMode, lm: i32, c: usize, salt: u32) -> Vec<f32> {
        let m = 1usize << lm;
        let n = frame_size(mode, lm);
        let mut x = vec![0.0f32; c * n];
        let mut s = salt.wrapping_add(1);
        let mut rng = || {
            s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (s >> 9) as f32 / (1u32 << 23) as f32 - 0.5
        };
        for ch in 0..c {
            for b in 0..mode.nb_e_bands {
                let lo = m * mode.e_bands[b] as usize;
                let hi = m * mode.e_bands[b + 1] as usize;
                let mut norm = 0.0f32;
                for j in lo..hi {
                    let v = rng();
                    x[ch * n + j] = v;
                    norm += v * v;
                }
                let inv = if norm > 0.0 { 1.0 / norm.sqrt() } else { 0.0 };
                for j in lo..hi {
                    x[ch * n + j] *= inv;
                }
            }
        }
        x
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

    #[test]
    fn frame_round_trips_bit_exact_mono() {
        let mode = celt_mode_48k();
        let p = mono_params();
        let nb = mode.nb_e_bands;
        let mut band_log_e: Vec<f32> = (0..nb).map(|i| 6.0 - 0.2 * i as f32).collect();
        let band_e = vec![1.0f32; nb];
        let mut x = make_normalised_spectrum(&mode, p.lm, p.c, 7);
        let x_in = x.clone();
        let mut tf_res = vec![0i32; nb];
        let mut offsets = vec![0i32; nb];

        let mut enc_state = CeltEncoderState::new(p.c, nb);
        let bytes = encode_celt_frame(
            &mode,
            &p,
            &mut band_log_e,
            &band_e,
            &mut x,
            &mut tf_res,
            &mut offsets,
            200,
            None,
            &mut enc_state,
        )
        .expect("encode");
        let mut dec_state = CeltDecoderState::new(p.c, nb);
        let dec = decode_celt_frame(&mode, &bytes, 0, 21, 3, 1, 5, false, &mut dec_state);

        // Energies and resynthesised spectrum must match the encoder bit-exactly.
        assert_eq!(
            enc_state.old_band_e, dec.old_band_e,
            "band energies diverged"
        );
        assert_eq!(x, dec.x, "resynthesised spectrum diverged");
        assert_eq!(enc_state.rng, dec_state.rng, "carried seed diverged");
        assert!(!dec.is_transient);
        // The resynthesised spectrum is a lossy version of the input, not equal.
        assert_ne!(x_in, x, "resynth should differ from the pristine input");
    }

    #[test]
    fn frame_round_trips_with_dynalloc_boost() {
        let mode = celt_mode_48k();
        let p = mono_params();
        let nb = mode.nb_e_bands;
        let mut band_log_e: Vec<f32> = (0..nb).map(|i| 3.0 + (i as f32 * 0.5).sin()).collect();
        let band_e = vec![1.0f32; nb];
        let mut x = make_normalised_spectrum(&mode, p.lm, p.c, 99);
        let mut tf_res = vec![0i32; nb];
        // Request a few dynalloc boosts.
        let mut offsets = vec![0i32; nb];
        offsets[5] = 2;
        offsets[12] = 3;

        let mut enc_state = CeltEncoderState::new(p.c, nb);
        let bytes = encode_celt_frame(
            &mode,
            &p,
            &mut band_log_e,
            &band_e,
            &mut x,
            &mut tf_res,
            &mut offsets,
            220,
            None,
            &mut enc_state,
        )
        .expect("encode");
        let mut dec_state = CeltDecoderState::new(p.c, nb);
        let dec = decode_celt_frame(&mode, &bytes, 0, 21, 3, 1, 5, false, &mut dec_state);
        assert_eq!(enc_state.old_band_e, dec.old_band_e);
        assert_eq!(x, dec.x);
    }

    #[test]
    fn frame_round_trips_bit_exact_stereo() {
        let mode = celt_mode_48k();
        let mut p = mono_params();
        p.c = 2;
        p.intensity = 21;
        let nb = mode.nb_e_bands;
        let mut band_log_e: Vec<f32> = (0..2 * nb).map(|i| 5.0 - 0.1 * i as f32).collect();
        let band_e = vec![1.0f32; 2 * nb];
        let mut x = make_normalised_spectrum(&mode, p.lm, p.c, 42);
        let mut tf_res = vec![0i32; nb];
        let mut offsets = vec![0i32; nb];

        let mut enc_state = CeltEncoderState::new(p.c, nb);
        let bytes = encode_celt_frame(
            &mode,
            &p,
            &mut band_log_e,
            &band_e,
            &mut x,
            &mut tf_res,
            &mut offsets,
            320,
            None,
            &mut enc_state,
        )
        .expect("encode");
        let mut dec_state = CeltDecoderState::new(p.c, nb);
        let dec = decode_celt_frame(&mode, &bytes, 0, 21, 3, 2, 5, false, &mut dec_state);
        assert_eq!(
            enc_state.old_band_e, dec.old_band_e,
            "stereo energies diverged"
        );
        assert_eq!(x, dec.x, "stereo spectrum diverged");
    }

    #[test]
    fn multi_frame_inter_prediction_round_trips_bit_exact() {
        // Encode several frames threading the energy / delayed-intra state on both
        // sides, as a real stream does. After frame one the coarse coder predicts
        // from the previous reconstruction (inter), so this exercises the
        // cross-frame path; every frame must still round-trip bit-exactly.
        let mode = celt_mode_48k();
        let p = mono_params();
        let nb = mode.nb_e_bands;
        let band_e = vec![1.0f32; nb];

        let mut enc_state = CeltEncoderState::new(p.c, nb);
        let mut dec_state = CeltDecoderState::new(p.c, nb);

        for frame in 0..4u32 {
            // A slowly drifting energy envelope so inter prediction is favoured.
            let mut band_log_e: Vec<f32> = (0..nb)
                .map(|i| 4.0 + (i as f32 * 0.3 + frame as f32 * 0.1).sin())
                .collect();
            let mut x = make_normalised_spectrum(&mode, p.lm, p.c, 100 + frame);
            let mut tf_res = vec![0i32; nb];
            let mut offsets = vec![0i32; nb];

            let bytes = encode_celt_frame(
                &mode,
                &p,
                &mut band_log_e,
                &band_e,
                &mut x,
                &mut tf_res,
                &mut offsets,
                200,
                None,
                &mut enc_state,
            )
            .expect("encode");
            let dec = decode_celt_frame(&mode, &bytes, 0, 21, 3, 1, 5, false, &mut dec_state);
            assert_eq!(
                enc_state.old_band_e, dec.old_band_e,
                "frame {frame} energies diverged"
            );
            assert_eq!(x, dec.x, "frame {frame} spectrum diverged");
            // Encoder and decoder energy state stay in lock-step across frames.
            assert_eq!(
                enc_state.old_band_e, dec_state.old_band_e,
                "frame {frame} state desynced"
            );
        }
    }

    #[test]
    fn transient_frame_anti_collapse_round_trips_bit_exact() {
        // A transient frame (LM>=2) reserves and codes the anti-collapse bit and
        // runs the resynthesis fix-up on both sides. With a tight budget some
        // short sub-blocks collapse, so this exercises the noise refill — which
        // must stay bit-exact between encoder and decoder.
        let mode = celt_mode_48k();
        let mut p = mono_params();
        p.is_transient = true;
        let nb = mode.nb_e_bands;
        let mut band_log_e: Vec<f32> = (0..nb).map(|i| 5.0 - 0.3 * i as f32).collect();
        let band_e = vec![1.0f32; nb];
        let mut x = make_normalised_spectrum(&mode, p.lm, p.c, 314);
        let mut tf_res = vec![0i32; nb];
        let mut offsets = vec![0i32; nb];

        let mut enc_state = CeltEncoderState::new(p.c, nb);
        // A tight budget forces collapsed sub-blocks the refill must repair.
        let bytes = encode_celt_frame(
            &mode,
            &p,
            &mut band_log_e,
            &band_e,
            &mut x,
            &mut tf_res,
            &mut offsets,
            60,
            None,
            &mut enc_state,
        )
        .expect("encode");
        let mut dec_state = CeltDecoderState::new(p.c, nb);
        let dec = decode_celt_frame(&mode, &bytes, 0, 21, 3, 1, 5, false, &mut dec_state);
        assert!(dec.is_transient, "transient flag lost");
        assert_eq!(
            enc_state.old_band_e, dec.old_band_e,
            "transient energies diverged"
        );
        assert_eq!(x, dec.x, "anti-collapse resynth diverged");
        assert_eq!(
            enc_state.consec_transient, 1,
            "first transient should count once"
        );
    }

    #[test]
    fn encode_celt_frame_errors_on_byte_budget_overflow() {
        // The CELT allocator is budget-aware: every coding stage in
        // `encode_celt_frame` is gated on `total_bits = nb_bytes * 8`, so the
        // public frame path never overruns its buffer even at a 1-byte budget
        // (it simply codes fewer bands). The overflow that the new error branch
        // guards therefore can only be provoked by driving the underlying range
        // coder itself past its buffer — which is exactly the corruption the
        // guard turns into an error. So we drive a small `RangeEncoder` past its
        // storage and confirm `is_error()` latches; `encode_celt_frame` then
        // converts that same flag into an `Err`.
        let mut enc = RangeEncoder::new(2);
        // Write far more raw bits than a 2-byte buffer can hold.
        for _ in 0..32 {
            enc.enc_bits(0xFF, 8);
        }
        assert!(
            enc.is_error(),
            "overrunning the range-coder buffer must latch is_error()"
        );
    }
}
