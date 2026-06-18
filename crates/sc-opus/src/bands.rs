//! CELT band energy and normalisation.
//!
//! Hand-ported to safe Rust from libopus `celt/bands.c` (`compute_band_energies`,
//! `normalise_bands`, `denormalise_bands`) and the float build's `celt_exp2`
//! from `celt/mathops.h`. The float (`!FIXED_POINT`) build is ported: the
//! shift/saturate macros are identities and `MULT16_16` is a plain product, so
//! the band energy is `sqrt` of the per-band power, normalisation divides by it,
//! and denormalisation multiplies by `exp2(logE + eMean)`.

// Consumed by the CELT band-quantization stage; the live encoder still ships
// via the Opus FFI path.
#![allow(dead_code)]

/// `celt_exp2`: the float build's fast base-2 exponential (FLOAT_APPROX cubic).
/// Bit-for-bit the libopus polynomial, including the integer-into-exponent add.
#[must_use]
#[allow(clippy::excessive_precision)]
pub(crate) fn celt_exp2(x: f32) -> f32 {
    let integer = x.floor() as i32;
    if integer < -50 {
        return 0.0;
    }
    let frac = x - integer as f32;
    // K0=1, K1=log2, K2=3-4log2, K3=3log2-2 (cubic on the fractional part).
    let res = 0.999_925_22_f32
        + frac * (0.695_833_54_f32 + frac * (0.226_067_16_f32 + 0.078_024_523_f32 * frac));
    let bits = res
        .to_bits()
        .wrapping_add((integer as u32).wrapping_shl(23))
        & 0x7fff_ffff;
    f32::from_bits(bits)
}

/// The CELT band layout: the `e_bands` boundary table (in short-MDCT bins, with
/// `nb_e_bands + 1` entries), the short MDCT size, and the band count. This is
/// the slice of `CELTMode` the band energy/normalisation routines read.
pub struct BandLayout<'a> {
    pub e_bands: &'a [i16],
    pub short_mdct_size: usize,
    pub nb_e_bands: usize,
}

impl BandLayout<'_> {
    /// `compute_band_energies`: the per-band RMS energy `sqrt(sum |X|^2)` for
    /// each of the `end` bands in each of the `c` channels. `x` is the
    /// interleaved-by-channel MDCT spectrum; `band_e` is `nb_e_bands * c` long.
    pub fn compute_band_energies(
        &self,
        x: &[f32],
        band_e: &mut [f32],
        end: usize,
        c: usize,
        lm: u32,
    ) {
        let n = self.short_mdct_size << lm;
        for ch in 0..c {
            for i in 0..end {
                let lo = (self.e_bands[i] as usize) << lm;
                let hi = (self.e_bands[i + 1] as usize) << lm;
                let mut sum = 1e-27f32;
                for &v in &x[ch * n + lo..ch * n + hi] {
                    sum += v * v;
                }
                band_e[i + ch * self.nb_e_bands] = sum.sqrt();
            }
        }
    }

    /// `normalise_bands`: scales each band of `freq` to unit energy into `x`,
    /// dividing by the band energy `band_e`. `m` is `1 << LM`.
    pub fn normalise_bands(
        &self,
        freq: &[f32],
        x: &mut [f32],
        band_e: &[f32],
        end: usize,
        c: usize,
        m: usize,
    ) {
        let n = m * self.short_mdct_size;
        for ch in 0..c {
            for i in 0..end {
                let g = 1.0 / (1e-27f32 + band_e[i + ch * self.nb_e_bands]);
                for j in m * self.e_bands[i] as usize..m * self.e_bands[i + 1] as usize {
                    x[j + ch * n] = freq[j + ch * n] * g;
                }
            }
        }
    }

    /// `denormalise_bands` (float path, single channel): synthesises `freq` from
    /// the unit-energy band shapes `x` and the quantized log-energies
    /// `band_log_e`, scaling each band by `exp2(min(32, logE + eMean))`. Zeros
    /// the spectrum outside `[start, end)` and past the `downsample`/`silence`
    /// bound.
    #[allow(clippy::too_many_arguments)]
    pub fn denormalise_bands(
        &self,
        x: &[f32],
        freq: &mut [f32],
        band_log_e: &[f32],
        e_means: &[f32],
        start: usize,
        end: usize,
        m: usize,
        downsample: usize,
        silence: bool,
    ) {
        let n = m * self.short_mdct_size;
        let mut start = start;
        let mut end = end;
        let mut bound = m * self.e_bands[end] as usize;
        if downsample != 1 {
            bound = bound.min(n / downsample);
        }
        if silence {
            bound = 0;
            start = 0;
            end = 0;
        }
        // Below the first coded band the synthesis is silent.
        for f in &mut freq[..m * self.e_bands[start] as usize] {
            *f = 0.0;
        }
        for i in start..end {
            let lg = (band_log_e[i] + e_means[i]).min(32.0);
            let g = celt_exp2(lg);
            for j in m * self.e_bands[i] as usize..m * self.e_bands[i + 1] as usize {
                freq[j] = x[j] * g;
            }
        }
        // Clear everything past the downsample/silence bound.
        for f in &mut freq[bound..n] {
            *f = 0.0;
        }
    }
}

use crate::vq::renormalise_vector;
use std::f32::consts::FRAC_1_SQRT_2;

/// Small-energy floor matching libopus `EPSILON` in the float build.
const EPSILON: f32 = 1e-15;

/// Hadamard reordering table (`ordery_table`), indexed by `stride - 2` for the
/// supported strides 2, 4, 8, 16.
const ORDERY_TABLE: [usize; 30] = [
    1, 0, // stride 2
    3, 0, 2, 1, // stride 4
    7, 0, 4, 3, 6, 1, 5, 2, // stride 8
    15, 0, 8, 7, 12, 3, 11, 4, 14, 1, 9, 6, 13, 2, 10, 5, // stride 16
];

/// `intensity_stereo`: collapses bands `X`/`Y` onto the intensity direction
/// weighted by the per-channel band energies in `band_e` (the side is dropped).
/// Float path: weights are `left/norm`, `right/norm` and the output is their
/// linear combination.
pub fn intensity_stereo(
    x: &mut [f32],
    y: &[f32],
    band_e: &[f32],
    band_id: usize,
    nb_e_bands: usize,
    n: usize,
) {
    let left = band_e[band_id];
    let right = band_e[band_id + nb_e_bands];
    let norm = EPSILON + (EPSILON + left * left + right * right).sqrt();
    let a1 = left / norm;
    let a2 = right / norm;
    for j in 0..n {
        let l = x[j];
        let r = y[j];
        x[j] = a1 * l + a2 * r;
    }
}

/// `stereo_split`: the orthonormal mid/side rotation `X' = (X+Y)/√2`,
/// `Y' = (Y-X)/√2`, applied in place.
pub fn stereo_split(x: &mut [f32], y: &mut [f32], n: usize) {
    for j in 0..n {
        let l = FRAC_1_SQRT_2 * x[j];
        let r = FRAC_1_SQRT_2 * y[j];
        x[j] = l + r;
        y[j] = r - l;
    }
}

/// `celt_lcg_rand`: the linear-congruential generator CELT uses to fill empty
/// bands with noise. Wraps modulo 2^32 like the C.
#[must_use]
pub fn celt_lcg_rand(seed: u32) -> u32 {
    seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223)
}

/// `anti_collapse`: after band quantization, refills any MDCT short-block that
/// collapsed to zero (per `collapse_masks`) with energy-scaled noise, so a
/// transient never leaves a silent sub-block. Ported from the float build of
/// libopus `celt/bands.c`. `x_` is the `C`-channel spectrum (stride `size`),
/// `log_e` the current band log-energies and `prev1_log_e`/`prev2_log_e` the two
/// previous frames'. Returns the advanced LCG `seed`.
#[allow(clippy::too_many_arguments)]
pub fn anti_collapse(
    e_bands: &[i16],
    nb_e_bands: usize,
    x_: &mut [f32],
    collapse_masks: &[u8],
    lm: i32,
    c: usize,
    size: usize,
    start: usize,
    end: usize,
    log_e: &[f32],
    prev1_log_e: &[f32],
    prev2_log_e: &[f32],
    pulses: &[i32],
    mut seed: u32,
) -> u32 {
    for i in start..end {
        let n0 = (e_bands[i + 1] - e_bands[i]) as usize;
        // Depth in 1/8 bits.
        let depth = ((1 + pulses[i]) / (e_bands[i + 1] - e_bands[i]) as i32) >> lm;
        let thresh = 0.5 * celt_exp2(-0.125 * depth as f32);
        let sqrt_1 = 1.0 / ((n0 << lm) as f32).sqrt();

        for ch in 0..c {
            let mut prev1 = prev1_log_e[ch * nb_e_bands + i];
            let mut prev2 = prev2_log_e[ch * nb_e_bands + i];
            if c == 1 {
                prev1 = prev1.max(prev1_log_e[nb_e_bands + i]);
                prev2 = prev2.max(prev2_log_e[nb_e_bands + i]);
            }
            let ediff = (log_e[ch * nb_e_bands + i] - prev1.min(prev2)).max(0.0);
            // Short blocks don't carry the same energy as long ones, so scale by
            // 2 (LM<3) or 2*sqrt(2) (LM==3).
            let mut r = 2.0 * celt_exp2(-ediff);
            if lm == 3 {
                r *= core::f32::consts::SQRT_2;
            }
            r = thresh.min(r);
            r *= sqrt_1;

            let base = ch * size + ((e_bands[i] as usize) << lm);
            let x = &mut x_[base..base + (n0 << lm)];
            let mut renormalize = false;
            for k in 0..(1usize << lm) {
                // Detect a collapsed sub-block and fill it with noise.
                if collapse_masks[i * c + ch] & (1 << k) == 0 {
                    for j in 0..n0 {
                        seed = celt_lcg_rand(seed);
                        x[(j << lm) + k] = if seed & 0x8000 != 0 { r } else { -r };
                    }
                    renormalize = true;
                }
            }
            if renormalize {
                renormalise_vector(x, n0 << lm, 1.0);
            }
        }
    }
    seed
}

/// `stereo_merge`: reconstructs the left/right pair from the resynthesized
/// mid (`X`, scaled by `mid`) and side (`Y`) channels. If either reconstructed
/// energy collapses, the side is dropped and `Y` mirrors `X`.
pub fn stereo_merge(x: &mut [f32], y: &mut [f32], mid: f32, n: usize) {
    // norm of X+Y and X-Y as |X|^2 + |Y|^2 -/+ sum(xy), via Y.X and Y.Y.
    let mut xp = 0.0f32;
    let mut side = 0.0f32;
    for j in 0..n {
        xp += y[j] * x[j];
        side += y[j] * y[j];
    }
    // Compensate for the mid normalization.
    xp *= mid;
    let mid2 = mid;
    let el = mid2 * mid2 + side - 2.0 * xp;
    let er = mid2 * mid2 + side + 2.0 * xp;
    if er < 6e-4 || el < 6e-4 {
        y[..n].copy_from_slice(&x[..n]);
        return;
    }
    let lgain = 1.0 / el.sqrt();
    let rgain = 1.0 / er.sqrt();
    for j in 0..n {
        let l = mid * x[j];
        let r = y[j];
        x[j] = lgain * (l - r);
        y[j] = rgain * (l + r);
    }
}

/// `spreading_decision`: chooses the PVQ spreading aggressiveness (`SPREAD_*`)
/// from the sparsity of the normalised spectrum `x`, and (when `update_hf`)
/// updates the high-frequency tapset decision. Ported from the float build of
/// libopus `celt/bands.c`. `average` / `hf_average` / `tapset_decision` are the
/// running encoder state, updated in place.
#[allow(clippy::too_many_arguments)]
pub fn spreading_decision(
    e_bands: &[i16],
    nb_e_bands: usize,
    short_mdct_size: usize,
    x: &[f32],
    average: &mut i32,
    last_decision: i32,
    hf_average: &mut i32,
    tapset_decision: &mut i32,
    update_hf: bool,
    end: usize,
    c: usize,
    m: usize,
    spread_weight: &[i32],
) -> i32 {
    let n0 = m * short_mdct_size;
    if m * (e_bands[end] - e_bands[end - 1]) as usize <= 8 {
        return 0; // SPREAD_NONE
    }
    let mut sum = 0i32;
    let mut nb_bands = 0i32;
    let mut hf_sum = 0i32;

    for ch in 0..c {
        for i in 0..end {
            let big_n = m * (e_bands[i + 1] - e_bands[i]) as usize;
            if big_n <= 8 {
                continue;
            }
            let base = ch * n0 + m * e_bands[i] as usize;
            let xb = &x[base..base + big_n];
            let mut tcount = [0i32; 3];
            for &v in xb {
                // |x|^2 * N, the Q13 magnitude estimate in the float build.
                let x2n = v * v * big_n as f32;
                tcount[0] += i32::from(x2n < 0.25);
                tcount[1] += i32::from(x2n < 0.0625);
                tcount[2] += i32::from(x2n < 0.015625);
            }
            let n_i = big_n as i32;
            // Only the four highest bands (8 kHz and up) feed the HF decision.
            if i > nb_e_bands - 4 {
                hf_sum += 32 * (tcount[1] + tcount[0]) / n_i;
            }
            let tmp = i32::from(2 * tcount[2] >= n_i)
                + i32::from(2 * tcount[1] >= n_i)
                + i32::from(2 * tcount[0] >= n_i);
            sum += tmp * spread_weight[i];
            nb_bands += spread_weight[i];
        }
    }

    if update_hf {
        if hf_sum != 0 {
            hf_sum /= c as i32 * (4 - nb_e_bands as i32 + end as i32);
        }
        *hf_average = (*hf_average + hf_sum) >> 1;
        hf_sum = *hf_average;
        if *tapset_decision == 2 {
            hf_sum += 4;
        } else if *tapset_decision == 0 {
            hf_sum -= 4;
        }
        *tapset_decision = if hf_sum > 22 {
            2
        } else if hf_sum > 18 {
            1
        } else {
            0
        };
    }

    debug_assert!(nb_bands > 0);
    sum = (sum << 8) / nb_bands;
    // Recursive averaging.
    sum = (sum + *average) >> 1;
    *average = sum;
    // Hysteresis.
    sum = (3 * sum + (((3 - last_decision) << 7) + 64) + 2) >> 2;
    if sum < 80 {
        3 // SPREAD_AGGRESSIVE
    } else if sum < 256 {
        2 // SPREAD_NORMAL
    } else if sum < 384 {
        1 // SPREAD_LIGHT
    } else {
        0 // SPREAD_NONE
    }
}

/// `haar1`: an in-place Haar (length-2 Hadamard) butterfly over `stride`
/// interleaved sub-sequences of `N0` samples. Self-inverse (orthonormal).
pub fn haar1(x: &mut [f32], n0: usize, stride: usize) {
    let n0 = n0 >> 1;
    for i in 0..stride {
        for j in 0..n0 {
            let t1 = FRAC_1_SQRT_2 * x[stride * 2 * j + i];
            let t2 = FRAC_1_SQRT_2 * x[stride * (2 * j + 1) + i];
            x[stride * 2 * j + i] = t1 + t2;
            x[stride * (2 * j + 1) + i] = t1 - t2;
        }
    }
}

/// `deinterleave_hadamard`: gathers the `stride` interleaved sub-sequences of
/// `X` (length `N0*stride`) into contiguous blocks, applying the Hadamard
/// `ordery` permutation when `hadamard` is set. Inverse of
/// [`interleave_hadamard`].
pub fn deinterleave_hadamard(x: &mut [f32], n0: usize, stride: usize, hadamard: bool) {
    let n = n0 * stride;
    let mut tmp = vec![0.0f32; n];
    if hadamard {
        let ordery = &ORDERY_TABLE[stride - 2..];
        for i in 0..stride {
            for j in 0..n0 {
                tmp[ordery[i] * n0 + j] = x[j * stride + i];
            }
        }
    } else {
        for i in 0..stride {
            for j in 0..n0 {
                tmp[i * n0 + j] = x[j * stride + i];
            }
        }
    }
    x[..n].copy_from_slice(&tmp);
}

/// `interleave_hadamard`: scatters contiguous blocks back into the `stride`
/// interleaved sub-sequences, undoing the Hadamard `ordery` permutation when
/// `hadamard` is set. Inverse of [`deinterleave_hadamard`].
pub fn interleave_hadamard(x: &mut [f32], n0: usize, stride: usize, hadamard: bool) {
    let n = n0 * stride;
    let mut tmp = vec![0.0f32; n];
    if hadamard {
        let ordery = &ORDERY_TABLE[stride - 2..];
        for i in 0..stride {
            for j in 0..n0 {
                tmp[j * stride + i] = x[ordery[i] * n0 + j];
            }
        }
    } else {
        for i in 0..stride {
            for j in 0..n0 {
                tmp[j * stride + i] = x[i * n0 + j];
            }
        }
    }
    x[..n].copy_from_slice(&tmp);
}

/// `fast_atan2f`: libopus' float polynomial approximation of `atan2`, returning
/// radians. Bit-for-bit the mathops.h rational approximation.
#[allow(clippy::excessive_precision)]
fn fast_atan2f(y: f32, x: f32) -> f32 {
    const CA: f32 = 0.43157974;
    const CB: f32 = 0.67848403;
    const CC: f32 = 0.08595542;
    let ce = std::f32::consts::FRAC_PI_2;
    let x2 = x * x;
    let y2 = y * y;
    // For very small magnitudes the answer doesn't matter.
    if x2 + y2 < 1e-18 {
        return 0.0;
    }
    if x2 < y2 {
        let den = (y2 + CB * x2) * (y2 + CC * x2);
        -x * y * (y2 + CA * x2) / den + if y < 0.0 { -ce } else { ce }
    } else {
        let den = (x2 + CB * y2) * (x2 + CC * y2);
        x * y * (x2 + CA * y2) / den + (if y < 0.0 { -ce } else { ce })
            - (if x * y < 0.0 { -ce } else { ce })
    }
}

/// `stereo_itheta`: the encoder's estimate of the band-split angle theta in Q14
/// (`[0, 16384]`), from the mid/side energies. `itheta = 0` means all energy is
/// in mid, `16384` all in side. Ported from libopus `vq.c`.
///
/// The `0.63662` constant is libopus' truncated `2/pi`; using the full-precision
/// `FRAC_2_PI` would diverge from the reference, so it is kept verbatim.
#[allow(clippy::approx_constant)]
pub fn stereo_itheta(x: &[f32], y: &[f32], stereo: bool, n: usize) -> i32 {
    let mut emid = EPSILON;
    let mut eside = EPSILON;
    if stereo {
        // mid = X+Y, side = X-Y (the fixed-point /2 is dropped; the ratio that
        // theta depends on is unchanged).
        for i in 0..n {
            let m = x[i] + y[i];
            let s = x[i] - y[i];
            emid += m * m;
            eside += s * s;
        }
    } else {
        for i in 0..n {
            emid += x[i] * x[i];
            eside += y[i] * y[i];
        }
    }
    let mid = emid.sqrt();
    let side = eside.sqrt();
    // 0.63662 = 2/pi, mapping atan2's [0, pi/2] onto [0, 16384].
    (0.5 + 16384.0 * 0.63662 * fast_atan2f(side, mid)).floor() as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The float build's `celt_log2` (polynomial), for round-trip tests; matches
    /// the one used in coarse-energy quantization.
    #[allow(clippy::excessive_precision)]
    fn celt_log2(x: f32) -> f32 {
        let mut bits = x.to_bits();
        let integer = (bits >> 23) as i32 - 127;
        bits = bits.wrapping_sub((integer as u32).wrapping_shl(23));
        let f = f32::from_bits(bits) - 1.5;
        let frac = -0.414_454_18_f32 + f * (0.959_092_32 + f * (-0.339_512_9 + f * 0.165_410_97));
        1.0 + integer as f32 + frac
    }

    #[test]
    fn celt_exp2_approximates_exp2() {
        for &x in &[-3.0f32, -1.0, 0.0, 0.5, 1.0, 4.0, 10.5] {
            let approx = celt_exp2(x);
            let exact = x.exp2();
            assert!(
                (approx - exact).abs() / exact < 0.002,
                "exp2({x}): approx={approx} exact={exact}"
            );
        }
        // log2 and exp2 are mutual inverses within the approximation error.
        for &v in &[0.3f32, 1.0, 7.5, 100.0, 5000.0] {
            let rt = celt_exp2(celt_log2(v));
            assert!((rt - v).abs() / v < 0.003, "exp2(log2({v}))={rt}");
        }
    }

    /// A 2-band mono layout: bins [0,2) and [2,4), short MDCT size 4.
    fn layout() -> BandLayout<'static> {
        static E_BANDS: [i16; 3] = [0, 2, 4];
        BandLayout {
            e_bands: &E_BANDS,
            short_mdct_size: 4,
            nb_e_bands: 2,
        }
    }

    #[test]
    fn band_energies_match_rms() {
        let layout = layout();
        let x = [3.0f32, 4.0, 1.0, 0.0]; // band0 sqrt(9+16)=5, band1 sqrt(1)=1
        let mut band_e = [0.0f32; 2];
        layout.compute_band_energies(&x, &mut band_e, 2, 1, 0);
        assert!((band_e[0] - 5.0).abs() < 1e-4, "band0 {}", band_e[0]);
        assert!((band_e[1] - 1.0).abs() < 1e-4, "band1 {}", band_e[1]);
    }

    #[test]
    fn normalise_yields_unit_energy_bands() {
        let layout = layout();
        let freq = [3.0f32, 4.0, 2.0, 1.5];
        let mut band_e = [0.0f32; 2];
        layout.compute_band_energies(&freq, &mut band_e, 2, 1, 0);
        let mut x = [0.0f32; 4];
        layout.normalise_bands(&freq, &mut x, &band_e, 2, 1, 1);
        // Each band's normalised power must be ~1.
        let p0 = x[0] * x[0] + x[1] * x[1];
        let p1 = x[2] * x[2] + x[3] * x[3];
        assert!((p0 - 1.0).abs() < 1e-4, "band0 power {p0}");
        assert!((p1 - 1.0).abs() < 1e-4, "band1 power {p1}");
    }

    #[test]
    fn denormalise_scales_by_exp2_gain() {
        let layout = layout();
        let x = [0.6f32, 0.8, 1.0, 0.0]; // unit-energy band shapes
        let band_log_e = [0.5f32, -1.0];
        let e_means = [1.0f32, 2.0];
        let mut freq = [0.0f32; 4];
        layout.denormalise_bands(&x, &mut freq, &band_log_e, &e_means, 0, 2, 1, 1, false);
        let g0 = celt_exp2(0.5 + 1.0);
        let g1 = celt_exp2(-1.0 + 2.0);
        assert!((freq[0] - 0.6 * g0).abs() < 1e-5);
        assert!((freq[1] - 0.8 * g0).abs() < 1e-5);
        assert!((freq[2] - 1.0 * g1).abs() < 1e-5);
    }

    #[test]
    fn stereo_split_inverts_analytically() {
        let orig_x = [0.3f32, -0.5, 0.8, 0.1];
        let orig_y = [0.7f32, 0.2, -0.4, 0.6];
        let mut x = orig_x;
        let mut y = orig_y;
        stereo_split(&mut x, &mut y, 4);
        // Inverse rotation: X = (X'-Y')/√2, Y = (X'+Y')/√2.
        for j in 0..4 {
            let rx = (x[j] - y[j]) * FRAC_1_SQRT_2;
            let ry = (x[j] + y[j]) * FRAC_1_SQRT_2;
            assert!((rx - orig_x[j]).abs() < 1e-6, "x[{j}]");
            assert!((ry - orig_y[j]).abs() < 1e-6, "y[{j}]");
        }
    }

    #[test]
    fn intensity_stereo_projects_onto_energy_weights() {
        let mut x = [1.0f32, 2.0, 3.0];
        let y = [0.5f32, -1.0, 2.0];
        // band 0 energy 3, band 1 energy 4 (nb_e_bands=1 -> right is band_e[1]).
        let band_e = [3.0f32, 4.0];
        let norm = EPSILON + (EPSILON + 9.0 + 16.0f32).sqrt(); // ~5
        let a1 = 3.0 / norm;
        let a2 = 4.0 / norm;
        let expect: Vec<f32> = (0..3).map(|j| a1 * x[j] + a2 * y[j]).collect();
        intensity_stereo(&mut x, &y, &band_e, 0, 1, 3);
        for j in 0..3 {
            assert!((x[j] - expect[j]).abs() < 1e-6, "x[{j}]");
        }
    }

    #[test]
    fn haar1_is_self_inverse() {
        let orig = [0.3f32, -0.5, 0.8, 0.1, 0.7, 0.2, -0.4, 0.6];
        let mut x = orig;
        haar1(&mut x, 8, 1);
        haar1(&mut x, 8, 1);
        for j in 0..8 {
            assert!((x[j] - orig[j]).abs() < 1e-6, "x[{j}]={}", x[j]);
        }
    }

    #[test]
    fn stereo_itheta_spans_mid_to_side() {
        // Mono mode: theta from the X (mid) vs Y (side) energies.
        let pure_mid = stereo_itheta(&[1.0, 0.0], &[0.0, 0.0], false, 2);
        let pure_side = stereo_itheta(&[0.0, 0.0], &[1.0, 0.0], false, 2);
        let balanced = stereo_itheta(&[1.0, 0.0], &[0.0, 1.0], false, 2);
        assert_eq!(pure_mid, 0, "all mid -> 0");
        assert!(
            (pure_side - 16384).abs() <= 2,
            "all side -> ~16384: {pure_side}"
        );
        assert!((balanced - 8192).abs() <= 2, "equal -> ~8192: {balanced}");
    }

    #[test]
    fn stereo_itheta_stereo_mode_uses_sum_difference() {
        // Stereo mode: mid = X+Y, side = X-Y.
        let x = [0.5f32, 0.3, -0.2];
        let same = stereo_itheta(&x, &x, true, 3); // side = 0 -> all mid
        let opp: Vec<f32> = x.iter().map(|v| -v).collect();
        let anti = stereo_itheta(&x, &opp, true, 3); // mid = 0 -> all side
        assert_eq!(same, 0, "X==Y -> 0");
        assert!((anti - 16384).abs() <= 2, "X==-Y -> ~16384: {anti}");
    }

    #[test]
    fn fast_atan2f_approximates_atan2() {
        for &(y, x) in &[
            (1.0f32, 1.0),
            (0.0, 1.0),
            (1.0, 0.0),
            (0.3, 0.7),
            (2.0, 0.5),
        ] {
            let approx = fast_atan2f(y, x);
            let exact = y.atan2(x);
            assert!(
                (approx - exact).abs() < 0.01,
                "atan2({y},{x})={approx} vs {exact}"
            );
        }
    }

    #[test]
    fn hadamard_interleave_inverts_deinterleave() {
        // N0=3 samples per sub-sequence across each supported stride.
        for &stride in &[2usize, 4, 8] {
            for &hadamard in &[false, true] {
                let n = 3 * stride;
                let orig: Vec<f32> = (0..n).map(|k| k as f32 * 0.1).collect();
                let mut x = orig.clone();
                deinterleave_hadamard(&mut x, 3, stride, hadamard);
                interleave_hadamard(&mut x, 3, stride, hadamard);
                assert_eq!(x, orig, "stride={stride} hadamard={hadamard}");
            }
        }
    }

    #[test]
    fn denormalise_silence_clears_spectrum() {
        let layout = layout();
        let x = [0.6f32, 0.8, 1.0, 0.0];
        let band_log_e = [0.5f32, -1.0];
        let e_means = [1.0f32, 2.0];
        let mut freq = [9.0f32; 4];
        layout.denormalise_bands(&x, &mut freq, &band_log_e, &e_means, 0, 2, 1, 1, true);
        assert_eq!(freq, [0.0; 4]);
    }

    #[test]
    fn normalise_denormalise_reconstructs_spectrum() {
        let layout = layout();
        let freq = [3.0f32, 4.0, 2.0, 1.5];
        let e_means = [1.0f32, 2.0];
        let mut band_e = [0.0f32; 2];
        layout.compute_band_energies(&freq, &mut band_e, 2, 1, 0);
        let mut x = [0.0f32; 4];
        layout.normalise_bands(&freq, &mut x, &band_e, 2, 1, 1);
        // Quantized log-energy as the coarse stage stores it: log2(E) - eMean.
        let band_log_e = [
            celt_log2(band_e[0]) - e_means[0],
            celt_log2(band_e[1]) - e_means[1],
        ];
        let mut out = [0.0f32; 4];
        layout.denormalise_bands(&x, &mut out, &band_log_e, &e_means, 0, 2, 1, 1, false);
        for i in 0..4 {
            assert!(
                (out[i] - freq[i]).abs() / freq[i].abs().max(1.0) < 0.005,
                "bin {i}: out={} freq={}",
                out[i],
                freq[i]
            );
        }
    }

    #[test]
    fn celt_lcg_rand_matches_reference_sequence() {
        // The Numerical-Recipes LCG: x' = 1664525*x + 1013904223 (mod 2^32).
        let mut seed = 0u32;
        seed = celt_lcg_rand(seed);
        assert_eq!(seed, 1_013_904_223);
        seed = celt_lcg_rand(seed);
        assert_eq!(
            seed,
            1_013_904_223u32
                .wrapping_mul(1_664_525)
                .wrapping_add(1_013_904_223)
        );
        // Wraps cleanly from a large seed without panicking.
        assert_eq!(
            celt_lcg_rand(u32::MAX),
            u32::MAX.wrapping_mul(1_664_525).wrapping_add(1_013_904_223)
        );
    }

    #[test]
    fn stereo_merge_reconstructs_normalised_channels() {
        // Hand case: a unit mid X=[1,0] and a side Y=[0,s] (mid=1) reconstruct
        // X=[1,-s]/sqrt(1+s^2), Y=[1,s]/sqrt(1+s^2) -- both unit norm.
        let s = 0.5f32;
        let mut x = [1.0f32, 0.0];
        let mut y = [0.0f32, s];
        stereo_merge(&mut x, &mut y, 1.0, 2);
        let inv = 1.0 / (1.0 + s * s).sqrt();
        for (got, want) in x.iter().zip([inv, -s * inv]) {
            assert!((got - want).abs() < 1e-6, "X {got} vs {want}");
        }
        for (got, want) in y.iter().zip([inv, s * inv]) {
            assert!((got - want).abs() < 1e-6, "Y {got} vs {want}");
        }
        // Each reconstructed channel is unit-norm.
        let nx: f32 = x.iter().map(|v| v * v).sum();
        let ny: f32 = y.iter().map(|v| v * v).sum();
        assert!((nx - 1.0).abs() < 1e-6 && (ny - 1.0).abs() < 1e-6);
    }

    #[test]
    fn stereo_merge_collapses_to_mid_on_degenerate_side() {
        // A zero side channel collapses: Y mirrors X.
        let mut x = [0.3f32, 0.4, 0.5, 0.2];
        let mut y = [0.0f32; 4];
        stereo_merge(&mut x, &mut y, 1.0, 4);
        assert_eq!(x, y);
    }

    #[test]
    fn anti_collapse_is_deterministic_and_fills_collapsed_blocks() {
        // One band (index 13: width 4) at LM=3 -> N = 4<<3 = 32, 8 sub-blocks.
        let e_bands = [
            0i16, 1, 2, 3, 4, 5, 6, 7, 8, 10, 12, 14, 16, 20, 24, 28, 34, 40, 48, 60, 78, 100,
        ];
        let nb = 21;
        let lm = 3;
        let n0 = (e_bands[14] - e_bands[13]) as usize; // 4
        let size = (e_bands[21] as usize) << lm;
        let x = vec![0.5f32; size];
        let log_e = vec![2.0f32; nb];
        let prev1 = vec![1.0f32; nb * 2];
        let prev2 = vec![1.0f32; nb * 2];
        let pulses = vec![40i32; nb];
        // Collapse sub-blocks 0 and 3 of band 13 (bits 0 and 3 cleared).
        let mut masks = vec![0xFFu8; nb];
        masks[13] = !((1 << 0) | (1 << 3));

        let mut x1 = x.clone();
        let s1 = anti_collapse(
            &e_bands, nb, &mut x1, &masks, lm, 1, size, 13, 14, &log_e, &prev1, &prev2, &pulses,
            12345,
        );
        let mut x2 = x.clone();
        let s2 = anti_collapse(
            &e_bands, nb, &mut x2, &masks, lm, 1, size, 13, 14, &log_e, &prev1, &prev2, &pulses,
            12345,
        );
        // Deterministic: identical output and seed for identical inputs.
        assert_eq!(s1, s2, "seed");
        assert_eq!(x1, x2, "output");
        assert_ne!(s1, 12345, "seed advanced");

        let base = (e_bands[13] as usize) << lm;
        // The band was modified (collapsed blocks refilled + renormalised).
        assert_ne!(&x1[base..base + (n0 << lm)], &x[base..base + (n0 << lm)]);
        // A band with no collapse (all mask bits set) is untouched.
        let mut x3 = x.clone();
        let full = vec![0xFFu8; nb];
        anti_collapse(
            &e_bands, nb, &mut x3, &full, lm, 1, size, 13, 14, &log_e, &prev1, &prev2, &pulses, 7,
        );
        assert_eq!(x3, x, "no-collapse band must be unchanged");
    }

    #[test]
    fn spreading_decision_distinguishes_sparse_from_dense() {
        let e_bands = [
            0i16, 1, 2, 3, 4, 5, 6, 7, 8, 10, 12, 14, 16, 20, 24, 28, 34, 40, 48, 60, 78, 100,
        ];
        let nb = 21;
        let smdct = 120;
        let m = 8;
        let end = 21;
        let n0 = m * smdct;
        let weight = vec![1i32; nb];

        // Dense: every band filled to unit energy with equal magnitudes.
        let mut dense = vec![0.0f32; n0];
        // Peaky: a single non-zero bin per band (sparse spectrum).
        let mut peaky = vec![0.0f32; n0];
        for i in 0..end {
            let lo = m * e_bands[i] as usize;
            let hi = m * e_bands[i + 1] as usize;
            let len = hi - lo;
            let val = 1.0 / (len as f32).sqrt();
            for v in &mut dense[lo..hi] {
                *v = val;
            }
            peaky[lo] = 1.0;
        }

        let mut avg = 0;
        let mut hf = 0;
        let mut tap = 0;
        let dense_dec = spreading_decision(
            &e_bands, nb, smdct, &dense, &mut avg, 0, &mut hf, &mut tap, false, end, 1, m, &weight,
        );
        let mut avg2 = 0;
        let mut hf2 = 0;
        let mut tap2 = 0;
        let peaky_dec = spreading_decision(
            &e_bands, nb, smdct, &peaky, &mut avg2, 0, &mut hf2, &mut tap2, false, end, 1, m,
            &weight,
        );
        // A dense (tonal-spread) spectrum gets more spreading than a sparse one.
        assert!(
            dense_dec > peaky_dec,
            "dense={dense_dec} should exceed peaky={peaky_dec}"
        );
        assert!((0..=3).contains(&dense_dec) && (0..=3).contains(&peaky_dec));

        // Deterministic for identical inputs.
        let (mut a, mut h, mut t) = (0, 0, 0);
        let d1 = spreading_decision(
            &e_bands, nb, smdct, &dense, &mut a, 0, &mut h, &mut t, true, end, 1, m, &weight,
        );
        let (mut a2, mut h2, mut t2) = (0, 0, 0);
        let d2 = spreading_decision(
            &e_bands, nb, smdct, &dense, &mut a2, 0, &mut h2, &mut t2, true, end, 1, m, &weight,
        );
        assert_eq!((d1, a, h, t), (d2, a2, h2, t2), "must be deterministic");
        // update_hf set a tapset decision in range.
        assert!((0..=2).contains(&t));
    }

    #[test]
    fn anti_collapse_fully_collapsed_band_is_unit_norm() {
        let e_bands = [
            0i16, 1, 2, 3, 4, 5, 6, 7, 8, 10, 12, 14, 16, 20, 24, 28, 34, 40, 48, 60, 78, 100,
        ];
        let nb = 21;
        let lm = 3;
        let n0 = (e_bands[14] - e_bands[13]) as usize;
        let size = (e_bands[21] as usize) << lm;
        let mut x = vec![0.0f32; size];
        let log_e = vec![3.0f32; nb];
        let prev = vec![0.5f32; nb * 2];
        let pulses = vec![20i32; nb];
        let masks = vec![0u8; nb]; // everything collapsed
        anti_collapse(
            &e_bands, nb, &mut x, &masks, lm, 1, size, 13, 14, &log_e, &prev, &prev, &pulses, 99,
        );
        let base = (e_bands[13] as usize) << lm;
        let norm: f32 = x[base..base + (n0 << lm)]
            .iter()
            .map(|v| v * v)
            .sum::<f32>()
            .sqrt();
        assert!((norm - 1.0).abs() < 1e-4, "renormalised band norm {norm}");
    }
}
