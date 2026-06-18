//! CELT all-bands quantization loop.
//!
//! Hand-ported to safe Rust from libopus `celt/bands.c` (`quant_all_bands` and
//! `special_hybrid_folding`): the top-level pass that walks every coded band,
//! tracks the running bit `balance`, maintains the `norm` folding buffers and
//! the fold collapse-mask estimate, switches dual-stereo to intensity stereo,
//! and dispatches each band to [`crate::quant_band`]. Derivative work of libopus
//! (BSD-3-Clause); see `LICENSE-THIRDPARTY`.
//!
//! The encoder's two-pass theta rate-distortion optimization (`theta_rdo`, gated
//! on `complexity >= 8`) is an encoder-only quality refinement that snapshots the
//! range coder; it is not yet wired here. Without it the loop still produces a
//! fully decodable bitstream and, with `resynth` forced on, reconstructs the
//! exact spectrum the decoder does — which is what the tests pin.

// Consumed by the CELT encode/decode entry points; the live encoder still ships
// via the Opus FFI path.
#![allow(dead_code)]

use crate::quant_band::{quant_band, quant_band_stereo, Coder, QuantCtx};
use crate::rate::PulseCache;
use crate::theta::BITRES;

const SPREAD_AGGRESSIVE: i32 = 3;

/// The immutable mode/allocation inputs to [`quant_all_bands`].
pub struct AllBandsInput<'a> {
    pub cache: &'a PulseCache,
    pub e_bands: &'a [i16],
    pub log_n: &'a [i16],
    pub nb_e_bands: usize,
    pub eff_e_bands: usize,
    pub start: usize,
    pub end: usize,
    pub band_e: &'a [f32],
    pub pulses: &'a [i32],
    pub tf_res: &'a [i32],
    pub short_blocks: bool,
    pub spread: i32,
    pub intensity: usize,
    pub total_bits: i32,
    pub lm: i32,
    pub coded_bands: usize,
    pub complexity: i32,
    pub disable_inv: bool,
}

/// `special_hybrid_folding`: duplicates enough of the first band's folding data
/// to fold the second band (a no-op for CELT-only layouts where the low bands
/// are equal width).
fn special_hybrid_folding(
    e_bands: &[i16],
    norm: &mut [f32],
    norm2: &mut [f32],
    start: usize,
    m: usize,
    dual_stereo: bool,
) {
    let n1 = m * (e_bands[start + 1] - e_bands[start]) as usize;
    let n2 = m * (e_bands[start + 2] - e_bands[start + 1]) as usize;
    if n2 > n1 {
        norm.copy_within(2 * n1 - n2..n1, n1);
        if dual_stereo {
            norm2.copy_within(2 * n1 - n2..n1, n1);
        }
    }
}

/// `quant_all_bands`: the per-band quantization loop. `x_` (and `y_` for stereo)
/// is the normalised spectrum; `collapse_masks` (length `nb_e_bands * C`) is
/// filled with the per-band anti-collapse masks. `coder` selects encode/decode;
/// `resynth` forces spectrum reconstruction (always on for decode). Returns the
/// updated LCG `seed`.
#[allow(clippy::too_many_arguments)]
pub fn quant_all_bands(
    input: &AllBandsInput,
    coder: &mut Coder,
    x_: &mut [f32],
    mut y_: Option<&mut [f32]>,
    collapse_masks: &mut [u8],
    mut balance: i32,
    mut dual_stereo: bool,
    resynth: bool,
    seed: u32,
) -> u32 {
    debug_assert!(
        input.end <= input.eff_e_bands,
        "bands beyond effEBands (the norm-aliasing fold path) are not ported"
    );
    let m = 1usize << input.lm;
    let big_b = if input.short_blocks { m as i32 } else { 1 };
    let e_bands = input.e_bands;
    let nb = input.nb_e_bands;
    let c = if y_.is_some() { 2 } else { 1 };
    let norm_offset = m * e_bands[input.start] as usize;
    let norm_len = m * e_bands[nb - 1] as usize - norm_offset;

    let mut norm = vec![0.0f32; norm_len.max(1)];
    let mut norm2 = vec![0.0f32; norm_len.max(1)];

    let mut ctx = QuantCtx {
        cache: input.cache,
        e_bands,
        log_n: input.log_n,
        nb_e_bands: nb,
        i: input.start,
        intensity: input.intensity,
        spread: input.spread,
        tf_change: 0,
        remaining_bits: 0,
        seed,
        resynth,
        band_e: input.band_e,
        theta_round: 0,
        avoid_split_noise: big_b > 1,
        disable_inv: input.disable_inv,
    };

    let mut lowband_offset = 0usize;
    let mut update_lowband = true;

    for i in input.start..input.end {
        ctx.i = i;
        let last = i == input.end - 1;
        let band_off = m * e_bands[i] as usize;
        let n = (m * e_bands[i + 1] as usize - band_off) as i32;
        let write_off = band_off - norm_offset;

        let tell = match coder {
            Coder::Enc(ec) => ec.ec_tell_frac() as i32,
            Coder::Dec(dec) => dec.ec_tell_frac() as i32,
        };

        if i != input.start {
            balance -= tell;
        }
        let remaining_bits = input.total_bits - tell - 1;
        ctx.remaining_bits = remaining_bits;
        let b = if i < input.coded_bands {
            let curr_balance = balance / 3i32.min((input.coded_bands - i) as i32);
            0.max(16383.min((remaining_bits + 1).min(input.pulses[i] + curr_balance)))
        } else {
            0
        };

        if resynth
            && (band_off as i32 - n >= norm_offset as i32 || i == input.start + 1)
            && (update_lowband || lowband_offset == 0)
        {
            lowband_offset = i;
        }
        if i == input.start + 1 {
            special_hybrid_folding(e_bands, &mut norm, &mut norm2, input.start, m, dual_stereo);
        }

        ctx.tf_change = input.tf_res[i];

        // Conservative estimate of the collapse masks we'll fold from.
        let (mut x_cm, mut y_cm, effective_lowband);
        if lowband_offset != 0
            && (input.spread != SPREAD_AGGRESSIVE || big_b > 1 || ctx.tf_change < 0)
        {
            // Never repeat spectral content within one band.
            let eff = 0.max((m * e_bands[lowband_offset] as usize) as i32 - norm_offset as i32 - n);
            let bound = eff + norm_offset as i32;
            let mut fold_start = lowband_offset;
            loop {
                fold_start -= 1;
                if (m * e_bands[fold_start] as usize) as i32 <= bound {
                    break;
                }
            }
            let mut fold_end = lowband_offset - 1;
            loop {
                fold_end += 1;
                if !(fold_end < i && ((m * e_bands[fold_end] as usize) as i32) < bound + n) {
                    break;
                }
            }
            let (mut xc, mut yc) = (0u32, 0u32);
            let mut fold_i = fold_start;
            loop {
                xc |= u32::from(collapse_masks[fold_i * c]);
                yc |= u32::from(collapse_masks[fold_i * c + c - 1]);
                fold_i += 1;
                if fold_i >= fold_end {
                    break;
                }
            }
            x_cm = xc;
            y_cm = yc;
            effective_lowband = eff;
        } else {
            // Otherwise the LCG folds, so (almost) all blocks are non-zero.
            x_cm = (1u32 << big_b) - 1;
            y_cm = (1u32 << big_b) - 1;
            effective_lowband = -1;
        }

        if dual_stereo && i == input.intensity {
            // Switch off dual stereo to do intensity stereo.
            dual_stereo = false;
            if resynth {
                for j in 0..write_off {
                    norm[j] = 0.5 * (norm[j] + norm2[j]);
                }
            }
        }

        let nu = n as usize;
        let lb = |src: &[f32]| -> Option<Vec<f32>> {
            (effective_lowband != -1)
                .then(|| src[effective_lowband as usize..effective_lowband as usize + nu].to_vec())
        };

        if dual_stereo {
            let lb_x = lb(&norm);
            let lb_y = lb(&norm2);
            let yb = y_.as_deref_mut().expect("dual stereo requires Y");
            x_cm = quant_band(
                &mut ctx,
                coder,
                &mut x_[band_off..band_off + nu],
                n,
                b / 2,
                big_b,
                lb_x.as_deref(),
                input.lm,
                (!last).then(|| &mut norm[write_off..write_off + nu]),
                1.0,
                None,
                x_cm as i32,
            );
            y_cm = quant_band(
                &mut ctx,
                coder,
                &mut yb[band_off..band_off + nu],
                n,
                b / 2,
                big_b,
                lb_y.as_deref(),
                input.lm,
                (!last).then(|| &mut norm2[write_off..write_off + nu]),
                1.0,
                None,
                y_cm as i32,
            );
        } else if let Some(yb) = y_.as_deref_mut() {
            let lb_x = lb(&norm);
            ctx.theta_round = 0;
            x_cm = quant_band_stereo(
                &mut ctx,
                coder,
                &mut x_[band_off..band_off + nu],
                &mut yb[band_off..band_off + nu],
                n,
                b,
                big_b,
                lb_x.as_deref(),
                input.lm,
                (!last).then(|| &mut norm[write_off..write_off + nu]),
                None,
                (x_cm | y_cm) as i32,
            );
            y_cm = x_cm;
        } else {
            let lb_x = lb(&norm);
            x_cm = quant_band(
                &mut ctx,
                coder,
                &mut x_[band_off..band_off + nu],
                n,
                b,
                big_b,
                lb_x.as_deref(),
                input.lm,
                (!last).then(|| &mut norm[write_off..write_off + nu]),
                1.0,
                None,
                (x_cm | y_cm) as i32,
            );
            y_cm = x_cm;
        }

        collapse_masks[i * c] = x_cm as u8;
        collapse_masks[i * c + c - 1] = y_cm as u8;
        balance += input.pulses[i] + tell;

        // Update the folding position only while we have >=1 bit/sample depth.
        update_lowband = b > (n << BITRES);
        ctx.avoid_split_noise = false;
    }

    ctx.seed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::range_coder::{RangeDecoder, RangeEncoder};
    use crate::rate::compute_pulse_cache;

    const EBAND5MS: [i16; 22] = [
        0, 1, 2, 3, 4, 5, 6, 7, 8, 10, 12, 14, 16, 20, 24, 28, 34, 40, 48, 60, 78, 100,
    ];
    const LOGN400: [i16; 21] = [
        0, 0, 0, 0, 0, 0, 0, 0, 8, 8, 8, 8, 16, 16, 16, 21, 21, 24, 29, 34, 36,
    ];
    const NB_E_BANDS: usize = 21;

    /// A per-band unit-energy spectrum, like the normalised CELT spectrum.
    fn normalized_spectrum(m: usize, end: usize, seed: f32, phase: f32) -> Vec<f32> {
        let total = m * EBAND5MS[end] as usize;
        let mut x = vec![0.0f32; total];
        for i in 0..end {
            let lo = m * EBAND5MS[i] as usize;
            let hi = m * EBAND5MS[i + 1] as usize;
            for (k, v) in x[lo..hi].iter_mut().enumerate() {
                let j = (lo + k) as f32;
                *v = ((j + 1.0) * seed + phase).sin() + 0.3 * (j * 0.11).cos();
            }
            let e = x[lo..hi].iter().map(|v| v * v).sum::<f32>().sqrt();
            if e > 0.0 {
                for v in &mut x[lo..hi] {
                    *v /= e;
                }
            }
        }
        x
    }

    #[allow(clippy::too_many_arguments)]
    fn input<'a>(
        cache: &'a PulseCache,
        band_e: &'a [f32],
        pulses: &'a [i32],
        tf_res: &'a [i32],
        lm: i32,
        end: usize,
        short_blocks: bool,
        total_bits: i32,
    ) -> AllBandsInput<'a> {
        AllBandsInput {
            cache,
            e_bands: &EBAND5MS,
            log_n: &LOGN400,
            nb_e_bands: NB_E_BANDS,
            eff_e_bands: NB_E_BANDS,
            start: 0,
            end,
            band_e,
            pulses,
            tf_res,
            short_blocks,
            spread: 2,
            intensity: NB_E_BANDS,
            total_bits,
            lm,
            coded_bands: end,
            complexity: 5,
            disable_inv: false,
        }
    }

    #[test]
    fn mono_all_bands_round_trips_exactly() {
        let lm = 3;
        let m = 1usize << lm;
        let end = NB_E_BANDS;
        let cache = compute_pulse_cache(&EBAND5MS, &LOGN400, NB_E_BANDS, lm);
        let band_e = vec![1.0f32; NB_E_BANDS];
        let pulses = vec![180i32; NB_E_BANDS];
        let tf_res = vec![0i32; NB_E_BANDS];
        let inp = input(&cache, &band_e, &pulses, &tf_res, lm, end, false, 6000);

        let x = normalized_spectrum(m, end, 0.31, 0.0);
        let mut x_enc = x.clone();
        let mut cm_enc = vec![0u8; NB_E_BANDS];
        let mut enc = RangeEncoder::new(512);
        let mut coder = Coder::Enc(&mut enc);
        let seed_enc = quant_all_bands(
            &inp,
            &mut coder,
            &mut x_enc,
            None,
            &mut cm_enc,
            0,
            false,
            true,
            7,
        );
        let bytes = enc.done();

        let mut x_dec = vec![0.0f32; x.len()];
        let mut cm_dec = vec![0u8; NB_E_BANDS];
        let mut dec = RangeDecoder::new(&bytes);
        let mut coder_d = Coder::Dec(&mut dec);
        let seed_dec = quant_all_bands(
            &inp,
            &mut coder_d,
            &mut x_dec,
            None,
            &mut cm_dec,
            0,
            false,
            true,
            7,
        );

        assert_eq!(cm_enc, cm_dec, "collapse masks");
        assert_eq!(seed_enc, seed_dec, "seed");
        for (j, (a, b)) in x_enc.iter().zip(&x_dec).enumerate() {
            assert_eq!(a.to_bits(), b.to_bits(), "bin {j}: enc={a} dec={b}");
        }
    }

    #[test]
    fn stereo_all_bands_round_trips_exactly() {
        let lm = 3;
        let m = 1usize << lm;
        let end = NB_E_BANDS;
        let cache = compute_pulse_cache(&EBAND5MS, &LOGN400, NB_E_BANDS, lm);
        let mut band_e = vec![1.0f32; NB_E_BANDS * 2];
        for i in 0..NB_E_BANDS {
            band_e[i] = 1.0 + 0.02 * i as f32;
            band_e[i + NB_E_BANDS] = 1.2 - 0.015 * i as f32;
        }
        let pulses = vec![220i32; NB_E_BANDS];
        let tf_res = vec![0i32; NB_E_BANDS];
        // Intensity stereo kicks in for the top few bands.
        let mut inp = input(&cache, &band_e, &pulses, &tf_res, lm, end, false, 9000);
        inp.intensity = NB_E_BANDS - 3;

        let x = normalized_spectrum(m, end, 0.27, 0.0);
        let y = normalized_spectrum(m, end, 0.41, 0.7);
        let mut x_enc = x.clone();
        let mut y_enc = y.clone();
        let mut cm_enc = vec![0u8; NB_E_BANDS * 2];
        let mut enc = RangeEncoder::new(768);
        let mut coder = Coder::Enc(&mut enc);
        let seed_enc = quant_all_bands(
            &inp,
            &mut coder,
            &mut x_enc,
            Some(&mut y_enc),
            &mut cm_enc,
            0,
            false,
            true,
            13,
        );
        let bytes = enc.done();

        let mut x_dec = vec![0.0f32; x.len()];
        let mut y_dec = vec![0.0f32; y.len()];
        let mut cm_dec = vec![0u8; NB_E_BANDS * 2];
        let mut dec = RangeDecoder::new(&bytes);
        let mut coder_d = Coder::Dec(&mut dec);
        let seed_dec = quant_all_bands(
            &inp,
            &mut coder_d,
            &mut x_dec,
            Some(&mut y_dec),
            &mut cm_dec,
            0,
            false,
            true,
            13,
        );

        assert_eq!(cm_enc, cm_dec, "collapse masks");
        assert_eq!(seed_enc, seed_dec, "seed");
        for (j, (a, b)) in x_enc.iter().zip(&x_dec).enumerate() {
            assert_eq!(a.to_bits(), b.to_bits(), "X bin {j}");
        }
        for (j, (a, b)) in y_enc.iter().zip(&y_dec).enumerate() {
            assert_eq!(a.to_bits(), b.to_bits(), "Y bin {j}");
        }
    }

    #[test]
    fn short_blocks_round_trip_exercises_folding() {
        // short_blocks => B=M; tf_res>0 drives recombine across bands.
        let lm = 3;
        let m = 1usize << lm;
        let end = NB_E_BANDS;
        let cache = compute_pulse_cache(&EBAND5MS, &LOGN400, NB_E_BANDS, lm);
        let band_e = vec![1.0f32; NB_E_BANDS];
        let pulses = vec![160i32; NB_E_BANDS];
        let mut tf_res = vec![0i32; NB_E_BANDS];
        for t in tf_res.iter_mut().skip(10) {
            *t = 1;
        }
        let inp = input(&cache, &band_e, &pulses, &tf_res, lm, end, true, 7000);

        let x = normalized_spectrum(m, end, 0.5, 0.0);
        let mut x_enc = x.clone();
        let mut cm_enc = vec![0u8; NB_E_BANDS];
        let mut enc = RangeEncoder::new(640);
        let mut coder = Coder::Enc(&mut enc);
        let seed_enc = quant_all_bands(
            &inp,
            &mut coder,
            &mut x_enc,
            None,
            &mut cm_enc,
            0,
            false,
            true,
            99,
        );
        let bytes = enc.done();

        let mut x_dec = vec![0.0f32; x.len()];
        let mut cm_dec = vec![0u8; NB_E_BANDS];
        let mut dec = RangeDecoder::new(&bytes);
        let mut coder_d = Coder::Dec(&mut dec);
        let seed_dec = quant_all_bands(
            &inp,
            &mut coder_d,
            &mut x_dec,
            None,
            &mut cm_dec,
            0,
            false,
            true,
            99,
        );

        assert_eq!(cm_enc, cm_dec, "collapse masks");
        assert_eq!(seed_enc, seed_dec, "seed");
        for (j, (a, b)) in x_enc.iter().zip(&x_dec).enumerate() {
            assert_eq!(a.to_bits(), b.to_bits(), "bin {j}");
        }
    }
}
