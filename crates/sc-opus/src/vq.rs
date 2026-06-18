//! Opus CELT PVQ vector quantization.
//!
//! Hand-ported to safe Rust from libopus `celt/vq.c` (the float build): the
//! L2-optimal pulse search (`op_pvq_search`), the spreading rotation
//! (`exp_rotation` / `exp_rotation1`), residual normalisation, the collapse-mask
//! extraction, and the `alg_quant` / `alg_unquant` band quantizers that wrap the
//! CWRS pulse coder. Derivative work of libopus (BSD-3-Clause); see
//! `LICENSE-THIRDPARTY`.
//!
//! As in [`crate::quant_bands`], the float build reduces every fixed-point
//! Q-shift macro to the identity, so the arithmetic here is the plain
//! floating-point form. The pulse *indices* are carried by the bit-exact CWRS
//! coder; the rotation and normalisation are ordinary DSP (lossy), so they only
//! need to be computed identically on the encode and decode sides.

#![allow(dead_code)]

use crate::cwrs::{decode_pulses, encode_pulses};
use crate::range_coder::{RangeDecoder, RangeEncoder};

const EPSILON: f32 = 1e-15;
const SPREAD_NONE: i32 = 0;

/// `celt_cos_norm`: `cos(pi/2 * x)`, evaluated in double precision as the C does.
fn celt_cos_norm(x: f32) -> f32 {
    (0.5 * std::f64::consts::PI * f64::from(x)).cos() as f32
}

/// One pass of the smoothing rotation over a single block of `X`.
fn exp_rotation1(x: &mut [f32], len: usize, stride: usize, c: f32, s: f32) {
    let ms = -s;
    for i in 0..len - stride {
        let x1 = x[i];
        let x2 = x[i + stride];
        x[i + stride] = c * x2 + s * x1;
        x[i] = c * x1 + ms * x2;
    }
    let mut idx = len as isize - 2 * stride as isize - 1;
    while idx >= 0 {
        let i = idx as usize;
        let x1 = x[i];
        let x2 = x[i + stride];
        x[i + stride] = c * x2 + s * x1;
        x[i] = c * x1 + ms * x2;
        idx -= 1;
    }
}

/// Applies (`dir > 0`) or removes (`dir < 0`) the PVQ spreading rotation.
fn exp_rotation(x: &mut [f32], len: usize, dir: i32, stride: usize, k: i32, spread: i32) {
    const SPREAD_FACTOR: [i32; 3] = [15, 10, 5];
    if 2 * k >= len as i32 || spread == SPREAD_NONE {
        return;
    }
    let factor = SPREAD_FACTOR[(spread - 1) as usize];

    let gain = len as f32 / (len as f32 + factor as f32 * k as f32);
    let theta = 0.5 * (gain * gain);

    let c = celt_cos_norm(theta);
    let s = celt_cos_norm(1.0 - theta); // sin(theta)

    let mut stride2 = 0usize;
    if len >= 8 * stride {
        stride2 = 1;
        // Integer sqrt(len/stride) with rounding.
        while (stride2 * stride2 + stride2) * stride + (stride >> 2) < len {
            stride2 += 1;
        }
    }
    let seg_len = len / stride;
    for i in 0..stride {
        let seg = &mut x[i * seg_len..i * seg_len + seg_len];
        if dir < 0 {
            if stride2 != 0 {
                exp_rotation1(seg, seg_len, stride2, s, c);
            }
            exp_rotation1(seg, seg_len, 1, c, s);
        } else {
            exp_rotation1(seg, seg_len, 1, c, -s);
            if stride2 != 0 {
                exp_rotation1(seg, seg_len, stride2, s, -c);
            }
        }
    }
}

/// Mixes the integer pulse vector `iy` back into `X` with unit norm and `gain`.
fn normalise_residual(iy: &[i32], x: &mut [f32], n: usize, ryy: f32, gain: f32) {
    let g = (1.0 / ryy.sqrt()) * gain;
    for i in 0..n {
        x[i] = g * iy[i] as f32;
    }
}

/// Builds the anti-collapse mask: one bit per block that holds any pulse.
fn extract_collapse_mask(iy: &[i32], n: usize, b: usize) -> u32 {
    if b <= 1 {
        return 1;
    }
    let n0 = n / b;
    let mut collapse_mask = 0u32;
    for i in 0..b {
        let mut tmp = 0i32;
        for j in 0..n0 {
            tmp |= iy[i * n0 + j];
        }
        collapse_mask |= u32::from(tmp != 0) << i;
    }
    collapse_mask
}

/// `op_pvq_search`: searches for the `K`-pulse vector closest (in direction) to
/// `X`, writing the signed pulse counts into `iy` and returning `sum(iy^2)`.
///
/// `X` is overwritten with its absolute values (the C does the same).
fn op_pvq_search(x: &mut [f32], iy: &mut [i32], k: i32, n: usize) -> f32 {
    let mut y = vec![0.0f32; n];
    let mut signx = vec![0i32; n];

    // Strip the sign; the search runs on magnitudes.
    for j in 0..n {
        signx[j] = i32::from(x[j] < 0.0);
        x[j] = x[j].abs();
        iy[j] = 0;
        y[j] = 0.0;
    }

    let mut xy = 0.0f32;
    let mut yy = 0.0f32;
    let mut pulses_left = k;

    // Pre-search by projecting onto the pyramid when pulses are plentiful.
    if k > (n as i32 >> 1) {
        let mut sum = x[..n].iter().sum::<f32>();
        // Guard against infinities/NaNs producing too many pulses (64 ~ inf).
        if !(sum > EPSILON && sum < 64.0) {
            x[0] = 1.0;
            x[1..n].fill(0.0);
            sum = 1.0;
        }
        // K+0.8 guarantees we never exceed K pulses.
        let rcp = (k as f32 + 0.8) * (1.0 / sum);
        for j in 0..n {
            iy[j] = (rcp * x[j]).floor() as i32;
            y[j] = iy[j] as f32;
            yy += y[j] * y[j];
            xy += x[j] * y[j];
            y[j] *= 2.0;
            pulses_left -= iy[j];
        }
    }

    // Should never happen except on silence; dump the rest into bin 0.
    if pulses_left > n as i32 + 3 {
        let tmp = pulses_left as f32;
        yy += tmp * tmp;
        yy += tmp * y[0];
        iy[0] += pulses_left;
        pulses_left = 0;
    }

    for _ in 0..pulses_left {
        // The squared-magnitude term is added regardless, so hoist it out.
        yy += 1.0;

        // Position 0 handled out of the loop (the branch is usually not taken).
        let mut rxy = xy + x[0];
        let mut best_den = yy + y[0];
        rxy *= rxy;
        let mut best_num = rxy;
        let mut best_id = 0usize;
        for j in 1..n {
            let mut rxy = xy + x[j];
            let ryy = yy + y[j];
            rxy *= rxy;
            // num/den >= best_num/best_den without a division.
            if best_den * rxy > ryy * best_num {
                best_den = ryy;
                best_num = rxy;
                best_id = j;
            }
        }

        xy += x[best_id];
        yy += y[best_id];
        y[best_id] += 2.0; // keep y == 2*iy
        iy[best_id] += 1;
    }

    // Restore the signs: (iy ^ -signx) + signx == signx ? -iy : iy.
    for j in 0..n {
        iy[j] = (iy[j] ^ -signx[j]) + signx[j];
    }
    yy
}

/// Quantizes the normalised band vector `X` (`N` dims, `K` pulses) into the
/// range coder and, when `resynth` is set, replaces `X` with the reconstruction.
/// Returns the anti-collapse mask.
#[allow(clippy::too_many_arguments)]
pub fn alg_quant(
    x: &mut [f32],
    n: usize,
    k: i32,
    spread: i32,
    b: usize,
    enc: &mut RangeEncoder,
    gain: f32,
    resynth: bool,
) -> u32 {
    debug_assert!(k > 0, "alg_quant needs at least one pulse");
    debug_assert!(n > 1, "alg_quant needs at least two dimensions");

    let mut iy = vec![0i32; n + 3];
    exp_rotation(x, n, 1, b, k, spread);
    let yy = op_pvq_search(x, &mut iy, k, n);
    encode_pulses(&iy, n, k as u32, enc);

    if resynth {
        normalise_residual(&iy, x, n, yy, gain);
        exp_rotation(x, n, -1, b, k, spread);
    }
    extract_collapse_mask(&iy, n, b)
}

/// Decodes a band vector quantized by [`alg_quant`] into `X`. Returns the
/// anti-collapse mask.
pub fn alg_unquant(
    x: &mut [f32],
    n: usize,
    k: i32,
    spread: i32,
    b: usize,
    dec: &mut RangeDecoder,
    gain: f32,
) -> u32 {
    debug_assert!(k > 0, "alg_unquant needs at least one pulse");
    debug_assert!(n > 1, "alg_unquant needs at least two dimensions");

    let mut iy = vec![0i32; n];
    let ryy = decode_pulses(&mut iy, n, k as u32, dec);
    normalise_residual(&iy, x, n, ryy, gain);
    exp_rotation(x, n, -1, b, k, spread);
    extract_collapse_mask(&iy, n, b)
}

/// Renormalises `X` to unit norm scaled by `gain`.
pub fn renormalise_vector(x: &mut [f32], n: usize, gain: f32) {
    let mut e = EPSILON;
    for v in x.iter().take(n) {
        e += v * v;
    }
    let g = (1.0 / e.sqrt()) * gain;
    for v in x.iter_mut().take(n) {
        *v *= g;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn l2_normalise(v: &mut [f32]) {
        let norm = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        for x in v.iter_mut() {
            *x /= norm;
        }
    }

    /// Encodes a band with `alg_quant` (resynth on) and asserts `alg_unquant`
    /// reconstructs the exact same normalised vector and collapse mask.
    fn roundtrip(n: usize, k: i32, spread: i32, b: usize, seed: f32) {
        let mut x: Vec<f32> = (0..n)
            .map(|i| ((i as f32 + 1.0) * seed).sin() - 0.3 * (i as f32 * 0.7).cos())
            .collect();
        l2_normalise(&mut x);
        let mut x_enc = x.clone();

        let mut enc = RangeEncoder::new(256);
        let mask_enc = alg_quant(&mut x_enc, n, k, spread, b, &mut enc, 1.0, true);
        let bytes = enc.done();

        let mut x_dec = vec![0.0f32; n];
        let mut dec = RangeDecoder::new(&bytes);
        let mask_dec = alg_unquant(&mut x_dec, n, k, spread, b, &mut dec, 1.0);

        assert_eq!(
            mask_enc, mask_dec,
            "collapse mask (n={n} k={k} spread={spread} b={b})"
        );
        for i in 0..n {
            assert_eq!(
                x_enc[i].to_bits(),
                x_dec[i].to_bits(),
                "bin {i} mismatch (n={n} k={k} spread={spread} b={b}): enc={} dec={}",
                x_enc[i],
                x_dec[i],
            );
        }
        // The reconstruction is (approximately) unit norm.
        let norm = x_dec.iter().map(|v| v * v).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 0.1, "norm {norm} far from unit");
    }

    #[test]
    fn roundtrips_small_k() {
        for spread in 0..4 {
            roundtrip(16, 6, spread, 1, 0.41);
        }
    }

    #[test]
    fn roundtrips_large_k_presearch() {
        // K > N/2 exercises the pyramid pre-search path. K stays within the
        // range where the codebook size V(N,K) fits in 32 bits, which CELT's
        // bit allocation always guarantees.
        roundtrip(12, 10, 2, 1, 0.93);
        roundtrip(8, 30, 3, 1, 1.27);
    }

    #[test]
    fn roundtrips_multiple_blocks() {
        roundtrip(16, 5, 2, 2, 0.55);
        roundtrip(16, 7, 3, 4, 0.72);
        roundtrip(24, 4, 1, 2, 0.18);
    }

    #[test]
    fn op_pvq_search_uses_exactly_k_pulses() {
        let n = 16;
        let k = 7;
        let mut x: Vec<f32> = (0..n).map(|i| (i as f32 * 0.37).sin()).collect();
        let mut iy = vec![0i32; n + 3];
        op_pvq_search(&mut x, &mut iy, k, n);
        let total: i32 = iy.iter().take(n).map(|v| v.abs()).sum();
        assert_eq!(total, k, "pulse count must equal K");
    }

    #[test]
    fn renormalise_gives_unit_norm() {
        let mut x = vec![3.0f32, -4.0, 0.0, 12.0];
        renormalise_vector(&mut x, 4, 1.0);
        let norm = x.iter().map(|v| v * v).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-4, "norm {norm}");
    }
}
