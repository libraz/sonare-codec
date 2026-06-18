//! Opus CELT band-energy quantization.
//!
//! Hand-ported to safe Rust from libopus `celt/quant_bands.c` (the float build):
//! coarse energy quantization (`quant_coarse_energy_impl` / `unquant_coarse_energy`),
//! fine energy (`quant_fine_energy` / `unquant_fine_energy`), the leftover-bit
//! finaliser (`quant_energy_finalise` / `unquant_energy_finalise`), and the
//! `amp2Log2` log-domain conversion. Derivative work of libopus (BSD-3-Clause);
//! see `LICENSE-THIRDPARTY`.
//!
//! In the float build of libopus every fixed-point Q-shift macro (`SHL*`,
//! `SHR*`, `PSHR*`, `QCONST*`, `EXTEND32`, `EXTRACT16`) is the identity, so the
//! arithmetic here is the plain floating-point form. The coarse path sits on the
//! Laplace coder, which is the bit-exact-critical piece. The two-pass
//! intra/inter *decision* in the C `quant_coarse_energy` wrapper is implemented
//! on top of [`quant_coarse_energy_impl`]: rather than splice range-coder
//! buffers in place as the C does, each candidate pass runs on a clone of the
//! start state and the cheaper one is adopted wholesale.

#![allow(dead_code)]

use crate::laplace::{ec_laplace_decode, ec_laplace_encode};
use crate::range_coder::{RangeDecoder, RangeEncoder};

const MAX_FINE_BITS: i32 = 8;

/// Mean energy in each band (Q4, converted back to float in the C source).
/// Kept in the verbatim Q4 form (trailing zeros) to match the upstream table.
#[allow(clippy::excessive_precision)]
pub(crate) const E_MEANS: [f32; 25] = [
    6.437_500, 6.250_000, 5.750_000, 5.312_500, 5.062_500, 4.812_500, 4.500_000, 4.375_000,
    4.875_000, 4.687_500, 4.562_500, 4.437_500, 4.875_000, 4.625_000, 4.312_500, 4.500_000,
    4.375_000, 4.625_000, 4.750_000, 4.437_500, 3.750_000, 3.750_000, 3.750_000, 3.750_000,
    3.750_000,
];

/// Inter-frame prediction coefficients (0.9, 0.8, 0.65, 0.5), one per frame size.
const PRED_COEF: [f32; 4] = [
    29440.0 / 32768.0,
    26112.0 / 32768.0,
    21248.0 / 32768.0,
    16384.0 / 32768.0,
];
/// Inter-frame leakage coefficients, one per frame size.
const BETA_COEF: [f32; 4] = [
    30147.0 / 32768.0,
    22282.0 / 32768.0,
    12124.0 / 32768.0,
    6554.0 / 32768.0,
];
const BETA_INTRA: f32 = 4915.0 / 32768.0;

/// Inverse-CDF for the 2-bit "small energy" fallback model.
const SMALL_ENERGY_ICDF: [u8; 3] = [2, 1, 0];

/// Laplace model parameters per `[LM][intra]`: 42 bytes are 21 `(p0, decay)`
/// pairs (probability of zero and decay rate, both Q8).
pub(crate) const E_PROB_MODEL: [[[u8; 42]; 2]; 4] = [
    [
        [
            72, 127, 65, 129, 66, 128, 65, 128, 64, 128, 62, 128, 64, 128, 64, 128, 92, 78, 92, 79,
            92, 78, 90, 79, 116, 41, 115, 40, 114, 40, 132, 26, 132, 26, 145, 17, 161, 12, 176, 10,
            177, 11,
        ],
        [
            24, 179, 48, 138, 54, 135, 54, 132, 53, 134, 56, 133, 55, 132, 55, 132, 61, 114, 70,
            96, 74, 88, 75, 88, 87, 74, 89, 66, 91, 67, 100, 59, 108, 50, 120, 40, 122, 37, 97, 43,
            78, 50,
        ],
    ],
    [
        [
            83, 78, 84, 81, 88, 75, 86, 74, 87, 71, 90, 73, 93, 74, 93, 74, 109, 40, 114, 36, 117,
            34, 117, 34, 143, 17, 145, 18, 146, 19, 162, 12, 165, 10, 178, 7, 189, 6, 190, 8, 177,
            9,
        ],
        [
            23, 178, 54, 115, 63, 102, 66, 98, 69, 99, 74, 89, 71, 91, 73, 91, 78, 89, 86, 80, 92,
            66, 93, 64, 102, 59, 103, 60, 104, 60, 117, 52, 123, 44, 138, 35, 133, 31, 97, 38, 77,
            45,
        ],
    ],
    [
        [
            61, 90, 93, 60, 105, 42, 107, 41, 110, 45, 116, 38, 113, 38, 112, 38, 124, 26, 132, 27,
            136, 19, 140, 20, 155, 14, 159, 16, 158, 18, 170, 13, 177, 10, 187, 8, 192, 6, 175, 9,
            159, 10,
        ],
        [
            21, 178, 59, 110, 71, 86, 75, 85, 84, 83, 91, 66, 88, 73, 87, 72, 92, 75, 98, 72, 105,
            58, 107, 54, 115, 52, 114, 55, 112, 56, 129, 51, 132, 40, 150, 33, 140, 29, 98, 35, 77,
            42,
        ],
    ],
    [
        [
            42, 121, 96, 66, 108, 43, 111, 40, 117, 44, 123, 32, 120, 36, 119, 33, 127, 33, 134,
            34, 139, 21, 147, 23, 152, 20, 158, 25, 154, 26, 166, 21, 173, 16, 184, 13, 184, 10,
            150, 13, 139, 15,
        ],
        [
            22, 178, 63, 114, 74, 82, 84, 83, 92, 82, 103, 62, 96, 72, 96, 67, 101, 73, 107, 72,
            113, 55, 118, 52, 125, 52, 118, 52, 117, 55, 135, 49, 137, 39, 157, 32, 145, 29, 97,
            33, 77, 40,
        ],
    ],
];

/// `celt_log2`: the float build's fast base-2 logarithm (polynomial on the
/// mantissa). Ported verbatim from `celt/mathops.h`.
#[allow(clippy::excessive_precision)]
pub(crate) fn celt_log2(x: f32) -> f32 {
    let mut bits = x.to_bits();
    let integer = (bits >> 23) as i32 - 127;
    bits = bits.wrapping_sub((integer as u32).wrapping_shl(23));
    let frac = f32::from_bits(bits) - 1.5;
    let frac = -0.414_454_18 + frac * (0.959_092_3 + frac * (-0.339_512_9 + frac * 0.165_410_97));
    1.0 + integer as f32 + frac
}

/// Converts band amplitudes to the log domain used by energy quantization.
///
/// `band_e`/`band_log_e` are laid out as `C` planes of `nb_e_bands` each.
pub fn amp2_log2(
    nb_e_bands: usize,
    eff_end: usize,
    end: usize,
    band_e: &[f32],
    band_log_e: &mut [f32],
    channels: usize,
) {
    for c in 0..channels {
        for i in 0..eff_end {
            band_log_e[i + c * nb_e_bands] = celt_log2(band_e[i + c * nb_e_bands]) - E_MEANS[i];
        }
        for i in eff_end..end {
            band_log_e[c * nb_e_bands + i] = -14.0;
        }
    }
}

/// Encodes coarse band energies, predicting from `old_e_bands` (the previous
/// frame, updated in place to the reconstructed values) and writing the
/// per-band residual into `error`. Returns the C "badness" metric.
///
/// `intra` selects intra (`coef = 0`) vs inter prediction; `tell` is
/// `enc.ec_tell()` at entry, `budget` the total bit budget.
#[allow(clippy::too_many_arguments)]
pub fn quant_coarse_energy_impl(
    nb_e_bands: usize,
    start: usize,
    end: usize,
    e_bands: &[f32],
    old_e_bands: &mut [f32],
    budget: i32,
    tell: i32,
    prob_model: &[u8; 42],
    error: &mut [f32],
    enc: &mut RangeEncoder,
    channels: usize,
    lm: usize,
    intra: bool,
    max_decay: f32,
    lfe: bool,
) -> i32 {
    let mut badness = 0i32;
    let mut prev = [0.0f32; 2];
    let (coef, beta) = if intra {
        (0.0, BETA_INTRA)
    } else {
        (PRED_COEF[lm], BETA_COEF[lm])
    };

    if tell + 3 <= budget {
        enc.enc_bit_logp(intra, 3);
    }

    for i in start..end {
        for c in 0..channels {
            let x = e_bands[i + c * nb_e_bands];
            let old_e = old_e_bands[i + c * nb_e_bands].max(-9.0);
            let f = x - coef * old_e - prev[c];
            // Rounding to nearest integer here is really important.
            let mut qi = (0.5 + f).floor() as i32;
            let decay_bound = old_e_bands[i + c * nb_e_bands].max(-28.0) - max_decay;
            // Prevent the energy from going down too quickly.
            if qi < 0 && x < decay_bound {
                qi += (decay_bound - x) as i32;
                if qi > 0 {
                    qi = 0;
                }
            }
            let qi0 = qi;
            // If we don't have enough bits to encode all the energy, assume
            // something safe.
            let tell = enc.ec_tell();
            let bits_left = budget - tell - 3 * channels as i32 * (end - i) as i32;
            if i != start && bits_left < 30 {
                if bits_left < 24 {
                    qi = qi.min(1);
                }
                if bits_left < 16 {
                    qi = qi.max(-1);
                }
            }
            if lfe && i >= 2 {
                qi = qi.min(0);
            }
            if budget - tell >= 15 {
                let pi = 2 * i.min(20);
                let mut q = qi;
                ec_laplace_encode(
                    enc,
                    &mut q,
                    u32::from(prob_model[pi]) << 7,
                    i32::from(prob_model[pi + 1]) << 6,
                );
                qi = q;
            } else if budget - tell >= 2 {
                qi = qi.clamp(-1, 1);
                let sym = (2 * qi) ^ -i32::from(qi < 0);
                enc.enc_icdf(sym as usize, &SMALL_ENERGY_ICDF, 2);
            } else if budget - tell >= 1 {
                qi = qi.min(0);
                enc.enc_bit_logp(-qi != 0, 1);
            } else {
                qi = -1;
            }
            error[i + c * nb_e_bands] = f - qi as f32;
            badness += (qi0 - qi).abs();
            let q = qi as f32;

            let tmp = coef * old_e + prev[c] + q;
            old_e_bands[i + c * nb_e_bands] = tmp;
            prev[c] = prev[c] + q - beta * q;
        }
    }
    if lfe {
        0
    } else {
        badness
    }
}

/// `loss_distortion`: the squared error between the target and the previous
/// frame's energies, used to bias the intra/inter decision toward intra when the
/// signal changed a lot. Float build: the `SHR` macros are identity, so this is
/// `min(200, sum (e - old_e)^2)` over the coded bands and channels.
fn loss_distortion(
    e_bands: &[f32],
    old_e_bands: &[f32],
    start: usize,
    end: usize,
    nb_e_bands: usize,
    channels: usize,
) -> f32 {
    let mut dist = 0.0f32;
    for c in 0..channels {
        for i in start..end {
            let d = e_bands[i + c * nb_e_bands] - old_e_bands[i + c * nb_e_bands];
            dist += d * d;
        }
    }
    dist.min(200.0)
}

/// `quant_coarse_energy`: the two-pass intra/inter coarse-energy coder.
///
/// Hand-ported to safe Rust from libopus `celt/quant_bands.c`
/// (`quant_coarse_energy`). Derivative work of libopus (BSD-3-Clause); see
/// `LICENSE-THIRDPARTY`. Where the C juggles one encoder's buffer in place, this
/// runs each candidate pass on a *clone* of the start state and adopts the
/// winner wholesale, which yields the identical bitstream without splicing.
///
/// `e_bands` is the target log2 energies; `old_e_bands` the previous frame's
/// reconstruction, updated in place to this frame's reconstruction; `error`
/// receives the per-band quantisation residual the fine stage refines.
/// `delayed_intra` is the inter/intra distortion accumulator carried across
/// frames. Returns the chosen `intra` flag.
#[allow(clippy::too_many_arguments)]
pub fn quant_coarse_energy(
    nb_e_bands: usize,
    start: usize,
    end: usize,
    eff_end: usize,
    e_bands: &[f32],
    old_e_bands: &mut [f32],
    budget: i32,
    error: &mut [f32],
    enc: &mut RangeEncoder,
    channels: usize,
    lm: usize,
    nb_available_bytes: i32,
    force_intra: bool,
    delayed_intra: &mut f32,
    two_pass: bool,
    loss_rate: i32,
    lfe: bool,
) -> bool {
    let c = channels as i32;
    let span = (end - start) as i32;
    let mut intra = force_intra
        || (!two_pass
            && *delayed_intra > 2.0 * c as f32 * span as f32
            && nb_available_bytes > span * c);
    let intra_bias = ((i64::from(budget) * (*delayed_intra as i64) * i64::from(loss_rate))
        / (i64::from(c) * 512)) as i32;
    let new_distortion =
        loss_distortion(e_bands, old_e_bands, start, eff_end, nb_e_bands, channels);

    let tell = enc.ec_tell();
    let mut two_pass = two_pass;
    if tell + 3 > budget {
        two_pass = false;
        intra = false;
    }

    let mut max_decay = 16.0f32;
    if end - start > 10 {
        max_decay = max_decay.min(0.125 * nb_available_bytes as f32);
    }
    if lfe {
        max_decay = 3.0;
    }

    let enc_start = enc.clone();

    // Intra pass on a clone, into private energy/error copies.
    let mut old_intra = old_e_bands.to_vec();
    let mut error_intra = vec![0.0f32; channels * nb_e_bands];
    let mut enc_intra = enc_start.clone();
    let mut badness1 = 0i32;
    let mut tell_intra_frac = 0u32;
    if two_pass || intra {
        badness1 = quant_coarse_energy_impl(
            nb_e_bands,
            start,
            end,
            e_bands,
            &mut old_intra,
            budget,
            tell,
            &E_PROB_MODEL[lm][1],
            &mut error_intra,
            &mut enc_intra,
            channels,
            lm,
            true,
            max_decay,
            lfe,
        );
        tell_intra_frac = enc_intra.ec_tell_frac();
    }

    if intra {
        // Intra was forced: adopt it directly.
        *enc = enc_intra;
        old_e_bands.copy_from_slice(&old_intra);
        error.copy_from_slice(&error_intra);
    } else {
        // Inter pass from the start state, writing into the caller's buffers.
        let mut enc_inter = enc_start;
        let badness2 = quant_coarse_energy_impl(
            nb_e_bands,
            start,
            end,
            e_bands,
            old_e_bands,
            budget,
            tell,
            &E_PROB_MODEL[lm][0],
            error,
            &mut enc_inter,
            channels,
            lm,
            false,
            max_decay,
            lfe,
        );
        let tell_inter_frac = enc_inter.ec_tell_frac();
        let intra_wins = two_pass
            && (badness1 < badness2
                || (badness1 == badness2
                    && tell_inter_frac as i32 + intra_bias > tell_intra_frac as i32));
        if intra_wins {
            *enc = enc_intra;
            old_e_bands.copy_from_slice(&old_intra);
            error.copy_from_slice(&error_intra);
            intra = true;
        } else {
            *enc = enc_inter;
        }
    }

    if intra {
        *delayed_intra = new_distortion;
    } else {
        let p = PRED_COEF[lm];
        *delayed_intra = p * p * *delayed_intra + new_distortion;
    }
    intra
}

/// Decodes coarse band energies into `old_e_bands` (reconstructed in place).
///
/// Mirrors `unquant_coarse_energy`; the caller decodes the `intra` flag first
/// (see [`decode_intra_flag`]).
#[allow(clippy::too_many_arguments)]
pub fn unquant_coarse_energy(
    nb_e_bands: usize,
    start: usize,
    end: usize,
    old_e_bands: &mut [f32],
    intra: bool,
    dec: &mut RangeDecoder,
    channels: usize,
    lm: usize,
) {
    let prob_model = &E_PROB_MODEL[lm][usize::from(intra)];
    let mut prev = [0.0f32; 2];
    let (coef, beta) = if intra {
        (0.0, BETA_INTRA)
    } else {
        (PRED_COEF[lm], BETA_COEF[lm])
    };
    let budget = dec.storage() as i32 * 8;

    for i in start..end {
        for c in 0..channels {
            let tell = dec.ec_tell();
            let qi = if budget - tell >= 15 {
                let pi = 2 * i.min(20);
                ec_laplace_decode(
                    dec,
                    u32::from(prob_model[pi]) << 7,
                    i32::from(prob_model[pi + 1]) << 6,
                )
            } else if budget - tell >= 2 {
                let qi = dec.dec_icdf(&SMALL_ENERGY_ICDF, 2) as i32;
                (qi >> 1) ^ -(qi & 1)
            } else if budget - tell >= 1 {
                -i32::from(dec.dec_bit_logp(1))
            } else {
                -1
            };
            let q = qi as f32;
            let old = old_e_bands[i + c * nb_e_bands].max(-9.0);
            let tmp = coef * old + prev[c] + q;
            old_e_bands[i + c * nb_e_bands] = tmp;
            prev[c] = prev[c] + q - beta * q;
        }
    }
}

/// Reads the intra-prediction flag at the same point in the stream the encoder
/// wrote it (only present when `tell + 3 <= budget`).
pub fn decode_intra_flag(dec: &mut RangeDecoder, budget: i32) -> bool {
    if dec.ec_tell() + 3 <= budget {
        dec.dec_bit_logp(3)
    } else {
        false
    }
}

/// Encodes the fine-energy refinement bits.
#[allow(clippy::too_many_arguments)]
pub fn quant_fine_energy(
    nb_e_bands: usize,
    start: usize,
    end: usize,
    old_e_bands: &mut [f32],
    error: &mut [f32],
    fine_quant: &[i32],
    enc: &mut RangeEncoder,
    channels: usize,
) {
    for i in start..end {
        if fine_quant[i] <= 0 {
            continue;
        }
        let frac = 1i32 << fine_quant[i];
        for c in 0..channels {
            let mut q2 = ((error[i + c * nb_e_bands] + 0.5) * frac as f32).floor() as i32;
            q2 = q2.clamp(0, frac - 1);
            enc.enc_bits(q2 as u32, fine_quant[i] as u32);
            let offset = (q2 as f32 + 0.5) * (1i32 << (14 - fine_quant[i])) as f32 / 16384.0 - 0.5;
            old_e_bands[i + c * nb_e_bands] += offset;
            error[i + c * nb_e_bands] -= offset;
        }
    }
}

/// Decodes the fine-energy refinement bits into `old_e_bands`.
#[allow(clippy::too_many_arguments)]
pub fn unquant_fine_energy(
    nb_e_bands: usize,
    start: usize,
    end: usize,
    old_e_bands: &mut [f32],
    fine_quant: &[i32],
    dec: &mut RangeDecoder,
    channels: usize,
) {
    for i in start..end {
        if fine_quant[i] <= 0 {
            continue;
        }
        for c in 0..channels {
            let q2 = dec.dec_bits(fine_quant[i] as u32) as i32;
            let offset = (q2 as f32 + 0.5) * (1i32 << (14 - fine_quant[i])) as f32 / 16384.0 - 0.5;
            old_e_bands[i + c * nb_e_bands] += offset;
        }
    }
}

/// Spends any remaining bits on a final 1-bit energy refinement, in priority
/// order. Returns the number of bits left unspent.
#[allow(clippy::too_many_arguments)]
pub fn quant_energy_finalise(
    nb_e_bands: usize,
    start: usize,
    end: usize,
    old_e_bands: &mut [f32],
    error: &mut [f32],
    fine_quant: &[i32],
    fine_priority: &[i32],
    mut bits_left: i32,
    enc: &mut RangeEncoder,
    channels: usize,
) -> i32 {
    for prio in 0..2 {
        let mut i = start;
        while i < end && bits_left >= channels as i32 {
            if fine_quant[i] >= MAX_FINE_BITS || fine_priority[i] != prio {
                i += 1;
                continue;
            }
            for c in 0..channels {
                let q2 = i32::from(error[i + c * nb_e_bands] >= 0.0);
                enc.enc_bits(q2 as u32, 1);
                let offset =
                    (q2 as f32 - 0.5) * (1i32 << (14 - fine_quant[i] - 1)) as f32 / 16384.0;
                old_e_bands[i + c * nb_e_bands] += offset;
                error[i + c * nb_e_bands] -= offset;
                bits_left -= 1;
            }
            i += 1;
        }
    }
    bits_left
}

/// Decodes the final 1-bit energy refinement written by
/// [`quant_energy_finalise`].
#[allow(clippy::too_many_arguments)]
pub fn unquant_energy_finalise(
    nb_e_bands: usize,
    start: usize,
    end: usize,
    old_e_bands: &mut [f32],
    fine_quant: &[i32],
    fine_priority: &[i32],
    mut bits_left: i32,
    dec: &mut RangeDecoder,
    channels: usize,
) {
    for prio in 0..2 {
        let mut i = start;
        while i < end && bits_left >= channels as i32 {
            if fine_quant[i] >= MAX_FINE_BITS || fine_priority[i] != prio {
                i += 1;
                continue;
            }
            for c in 0..channels {
                let q2 = dec.dec_bits(1) as i32;
                let offset =
                    (q2 as f32 - 0.5) * (1i32 << (14 - fine_quant[i] - 1)) as f32 / 16384.0;
                old_e_bands[i + c * nb_e_bands] += offset;
                bits_left -= 1;
            }
            i += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn celt_log2_approximates_log2() {
        for &x in &[0.5f32, 1.0, 1.5, 2.0, 3.0, 7.0, 100.0, 0.01] {
            let approx = celt_log2(x);
            assert!((approx - x.log2()).abs() < 0.003, "log2({x}) = {approx}");
        }
    }

    #[test]
    fn amp2_log2_subtracts_band_means() {
        let nb = 21;
        let band_e = vec![4.0f32; nb];
        let mut log_e = vec![0.0f32; nb];
        amp2_log2(nb, nb, nb, &band_e, &mut log_e, 1);
        for i in 0..nb {
            let expected = celt_log2(4.0) - E_MEANS[i];
            assert!((log_e[i] - expected).abs() < 1e-6);
        }
    }

    /// Drives a full coarse + fine + finalise encode and asserts the decoder
    /// reconstructs the encoder's `old_e_bands` bit-exactly.
    fn roundtrip(intra: bool, lm: usize, channels: usize) {
        let nb = 21;
        let start = 0;
        let end = 21;
        let buf_bytes = 1024u32;
        let budget = buf_bytes as i32 * 8;

        // Synthetic current-frame log energies and a previous frame to predict
        // from. Deterministic, varied across bands and channels.
        let mut e_bands = vec![0.0f32; nb * channels];
        let mut old_enc = vec![0.0f32; nb * channels];
        for c in 0..channels {
            for i in 0..nb {
                let k = (i + c * nb) as f32;
                e_bands[i + c * nb] = (k * 0.37).sin() * 5.0 - 2.0;
                old_enc[i + c * nb] = (k * 0.21).cos() * 3.0;
            }
        }
        let old_dec_seed = old_enc.clone();

        let fine_quant: Vec<i32> = (0..nb).map(|i| (i % 4) as i32).collect();
        let fine_priority: Vec<i32> = (0..nb).map(|i| (i % 2) as i32).collect();

        let mut error = vec![0.0f32; nb * channels];
        let mut enc = RangeEncoder::new(buf_bytes);
        let tell = enc.ec_tell();
        let prob_model = &E_PROB_MODEL[lm][usize::from(intra)];
        quant_coarse_energy_impl(
            nb,
            start,
            end,
            &e_bands,
            &mut old_enc,
            budget,
            tell,
            prob_model,
            &mut error,
            &mut enc,
            channels,
            lm,
            intra,
            16.0,
            false,
        );
        quant_fine_energy(
            nb,
            start,
            end,
            &mut old_enc,
            &mut error,
            &fine_quant,
            &mut enc,
            channels,
        );
        let bits_left = 32;
        quant_energy_finalise(
            nb,
            start,
            end,
            &mut old_enc,
            &mut error,
            &fine_quant,
            &fine_priority,
            bits_left,
            &mut enc,
            channels,
        );
        let bytes = enc.done();

        let mut old_dec = old_dec_seed;
        let mut dec = RangeDecoder::new(&bytes);
        let dec_intra = decode_intra_flag(&mut dec, budget);
        assert_eq!(dec_intra, intra, "intra flag");
        unquant_coarse_energy(
            nb,
            start,
            end,
            &mut old_dec,
            dec_intra,
            &mut dec,
            channels,
            lm,
        );
        unquant_fine_energy(
            nb,
            start,
            end,
            &mut old_dec,
            &fine_quant,
            &mut dec,
            channels,
        );
        unquant_energy_finalise(
            nb,
            start,
            end,
            &mut old_dec,
            &fine_quant,
            &fine_priority,
            bits_left,
            &mut dec,
            channels,
        );

        for idx in 0..nb * channels {
            assert_eq!(
                old_enc[idx].to_bits(),
                old_dec[idx].to_bits(),
                "band {idx} mismatch: enc={} dec={} (intra={intra} lm={lm} C={channels})",
                old_enc[idx],
                old_dec[idx],
            );
        }
    }

    #[test]
    fn coarse_fine_roundtrip_intra_mono() {
        roundtrip(true, 3, 1);
    }

    #[test]
    fn coarse_fine_roundtrip_inter_mono() {
        for lm in 0..4 {
            roundtrip(false, lm, 1);
        }
    }

    #[test]
    fn coarse_fine_roundtrip_stereo() {
        roundtrip(false, 3, 2);
        roundtrip(true, 2, 2);
    }

    /// Runs the two-pass wrapper for one frame and decodes it, asserting the
    /// decoder reconstructs the coarse energies bit-exactly. Threads the energy
    /// and `delayed_intra` state in place so frames can be chained.
    #[allow(clippy::too_many_arguments)]
    fn two_pass_frame(
        e_bands: &[f32],
        old_enc: &mut [f32],
        old_dec: &mut [f32],
        delayed_intra: &mut f32,
        lm: usize,
        channels: usize,
        force_intra: bool,
    ) -> bool {
        let nb = 21;
        let buf_bytes = 1024u32;
        let budget = buf_bytes as i32 * 8;
        let mut error = vec![0.0f32; nb * channels];
        let mut enc = RangeEncoder::new(buf_bytes);
        let intra = quant_coarse_energy(
            nb,
            0,
            21,
            21,
            e_bands,
            old_enc,
            budget,
            &mut error,
            &mut enc,
            channels,
            lm,
            buf_bytes as i32,
            force_intra,
            delayed_intra,
            true,
            0,
            false,
        );
        let bytes = enc.done();

        let mut dec = RangeDecoder::new(&bytes);
        let dec_intra = decode_intra_flag(&mut dec, budget);
        assert_eq!(dec_intra, intra, "intra flag mismatch");
        unquant_coarse_energy(nb, 0, 21, old_dec, dec_intra, &mut dec, channels, lm);
        for idx in 0..nb * channels {
            assert_eq!(
                old_enc[idx].to_bits(),
                old_dec[idx].to_bits(),
                "band {idx} mismatch (intra={intra} lm={lm} C={channels})",
            );
        }
        intra
    }

    #[test]
    fn two_pass_coarse_energy_round_trips() {
        let nb = 21;
        for &channels in &[1usize, 2] {
            for lm in 0..4 {
                let mut e = vec![0.0f32; nb * channels];
                for c in 0..channels {
                    for i in 0..nb {
                        let k = (i + c * nb) as f32;
                        e[i + c * nb] = (k * 0.31).sin() * 4.0 - 1.0;
                    }
                }
                let mut old_enc = vec![0.0f32; nb * channels];
                let mut old_dec = vec![0.0f32; nb * channels];
                let mut delayed = 0.0f32;
                two_pass_frame(
                    &e,
                    &mut old_enc,
                    &mut old_dec,
                    &mut delayed,
                    lm,
                    channels,
                    false,
                );
            }
        }
    }

    #[test]
    fn two_pass_selects_inter_on_a_steady_signal() {
        // A constant target across frames: the first frame predicts from silence
        // (intra wins), the second predicts from the first's reconstruction
        // (inter wins). Both must round-trip bit-exactly with shared state.
        let nb = 21;
        let lm = 3usize;
        let e: Vec<f32> = (0..nb).map(|i| 2.0 + (i as f32 * 0.2).sin()).collect();
        let mut old_enc = vec![0.0f32; nb];
        let mut old_dec = vec![0.0f32; nb];
        let mut delayed = 0.0f32;

        // First frame (predicting from silence): roundtrip is verified inside the
        // helper; which model wins is budget-dependent so it is not asserted.
        two_pass_frame(&e, &mut old_enc, &mut old_dec, &mut delayed, lm, 1, false);
        // Second frame predicts from the first's reconstruction, so the inter
        // model is strictly cheaper and must be chosen.
        let intra2 = two_pass_frame(&e, &mut old_enc, &mut old_dec, &mut delayed, lm, 1, false);
        assert!(
            !intra2,
            "steady second frame should choose inter prediction"
        );
    }

    #[test]
    fn two_pass_force_intra_is_honoured() {
        let nb = 21;
        let e: Vec<f32> = (0..nb).map(|i| (i as f32 * 0.3).cos() * 3.0).collect();
        let mut old_enc = vec![1.0f32; nb];
        let mut old_dec = vec![1.0f32; nb];
        let mut delayed = 5.0f32;
        let intra = two_pass_frame(&e, &mut old_enc, &mut old_dec, &mut delayed, 3, 1, true);
        assert!(intra, "force_intra must select the intra model");
        // delayed_intra is reset to the fresh distortion when intra is chosen.
        assert!(delayed <= 200.0);
    }
}
