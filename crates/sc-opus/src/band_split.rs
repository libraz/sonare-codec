//! CELT band-split angle (`compute_theta`) assembly.
//!
//! Hand-ported to safe Rust from libopus `celt/bands.c` (`compute_theta`) and
//! `celt/mathops.c` (`isqrt32`): given a band's mid/side content, decide the
//! resolution `qn`, quantize and entropy-code the split angle theta (triangular,
//! uniform, or stereo-step pdf), dispatch the mid/side rotation, and derive the
//! `imid`/`iside` gains and bit-allocation `delta`. The encoder and decoder
//! share the angle pdf so the split is reconstructed bit-exactly.

// Consumed by the CELT band-quantization stage; the live encoder still ships
// via the Opus FFI path.
#![allow(dead_code)]

use crate::bands::{intensity_stereo, stereo_itheta, stereo_split};
use crate::range_coder::{RangeDecoder, RangeEncoder};
use crate::theta::{bitexact_cos, bitexact_log2tan, compute_qn, frac_mul16, BITRES};

const QTHETA_OFFSET: i32 = 4;
const QTHETA_OFFSET_TWOPHASE: i32 = 16;

/// `isqrt32`: the integer square root (`floor(sqrt(val))`), ported bit-for-bit
/// from libopus.
#[must_use]
pub fn isqrt32(mut val: u32) -> u32 {
    if val == 0 {
        return 0;
    }
    let mut g = 0u32;
    let mut bshift = ((32 - val.leading_zeros()) as i32 - 1) >> 1;
    let mut b = 1u32 << bshift;
    loop {
        let t = ((g << 1) + b) << bshift;
        if t <= val {
            g += b;
            val -= t;
        }
        b >>= 1;
        bshift -= 1;
        if bshift < 0 {
            break;
        }
    }
    g
}

/// The per-band context `compute_theta` reads: `log_n` is `m->logN[i]`,
/// `band_e` the band energies (used by intensity stereo), and the rest mirror
/// the `band_ctx` fields that affect the split.
pub struct BandCtx<'a> {
    pub log_n: i32,
    pub i: usize,
    pub intensity: usize,
    pub band_e: &'a [f32],
    pub nb_e_bands: usize,
    pub remaining_bits: i32,
    pub theta_round: i32,
    pub avoid_split_noise: bool,
    pub disable_inv: bool,
}

/// The split decision `compute_theta` produces (`split_ctx`): the stereo
/// inversion flag, the mid/side gains `imid`/`iside`, the allocation `delta`,
/// the quantized angle `itheta`, and the bits `qalloc` the angle consumed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SplitCtx {
    pub inv: bool,
    pub imid: i32,
    pub iside: i32,
    pub delta: i32,
    pub itheta: i32,
    pub qalloc: i32,
}

/// The split resolution and offset shared by both code paths.
fn qn_and_offset(ctx: &BandCtx, n: i32, b: i32, lm: i32, stereo: bool) -> i32 {
    let pulse_cap = ctx.log_n + lm * (1 << BITRES);
    let offset = (pulse_cap >> 1)
        - if stereo && n == 2 {
            QTHETA_OFFSET_TWOPHASE
        } else {
            QTHETA_OFFSET
        };
    let mut qn = compute_qn(n, b, offset, pulse_cap, stereo);
    if stereo && ctx.i >= ctx.intensity {
        qn = 1;
    }
    qn
}

/// Derives the gains, delta and updated `fill` from the final `itheta` — the
/// shared tail of `compute_theta`.
fn finish_split(
    itheta: i32,
    n: i32,
    big_b: i32,
    inv: bool,
    qalloc: i32,
    fill: &mut i32,
) -> SplitCtx {
    let (imid, iside, delta);
    if itheta == 0 {
        imid = 32767;
        iside = 0;
        *fill &= (1 << big_b) - 1;
        delta = -16384;
    } else if itheta == 16384 {
        imid = 0;
        iside = 32767;
        *fill &= ((1 << big_b) - 1) << big_b;
        delta = 16384;
    } else {
        imid = bitexact_cos(itheta as i16) as i32;
        iside = bitexact_cos((16384 - itheta) as i16) as i32;
        // The mid/side allocation that minimizes squared error in this band.
        delta = frac_mul16((n - 1) << 7, bitexact_log2tan(iside, imid));
    }
    SplitCtx {
        inv,
        imid,
        iside,
        delta,
        itheta,
        qalloc,
    }
}

/// Encoder side of `compute_theta`: estimates theta from `X`/`Y`, quantizes and
/// codes it, dispatches the mid/side rotation in place, and returns the split.
/// `b` (available eighth-bits) and `fill` are updated in place.
#[allow(clippy::too_many_arguments)]
pub fn compute_theta_encode(
    ctx: &BandCtx,
    ec: &mut RangeEncoder,
    x: &mut [f32],
    y: &mut [f32],
    n: i32,
    big_b: i32,
    b0: i32,
    lm: i32,
    stereo: bool,
    b: &mut i32,
    fill: &mut i32,
) -> SplitCtx {
    let qn = qn_and_offset(ctx, n, *b, lm, stereo);
    let nu = n as usize;
    let mut itheta = stereo_itheta(x, y, stereo, nu);
    let tell = ec.ec_tell_frac() as i32;
    let mut inv = false;

    if qn != 1 {
        if !stereo || ctx.theta_round == 0 {
            itheta = (itheta * qn + 8192) >> 14;
            if !stereo && ctx.avoid_split_noise && itheta > 0 && itheta < qn {
                // Avoid a theta that would inject noise into a zero-energy side.
                let unquantized = itheta * 16384 / qn;
                let imid = bitexact_cos(unquantized as i16) as i32;
                let iside = bitexact_cos((16384 - unquantized) as i16) as i32;
                let delta = frac_mul16((n - 1) << 7, bitexact_log2tan(iside, imid));
                if delta > *b {
                    itheta = qn;
                } else if delta < -*b {
                    itheta = 0;
                }
            }
        } else {
            // Bias quantization towards the extremes (theta_round != 0).
            let bias = if itheta > 8192 {
                32767 / qn
            } else {
                -32767 / qn
            };
            let down = (qn - 1).min(0.max((itheta * qn + bias) >> 14));
            itheta = if ctx.theta_round < 0 { down } else { down + 1 };
        }

        // Entropy-code the angle: a step pdf for stereo, uniform for time
        // splits, triangular otherwise.
        if stereo && n > 2 {
            let p0 = 3;
            let x0 = qn / 2;
            let ft = p0 * (x0 + 1) + x0;
            let xx = itheta;
            let (fl, fh) = if xx <= x0 {
                (p0 * xx, p0 * (xx + 1))
            } else {
                ((xx - 1 - x0) + (x0 + 1) * p0, (xx - x0) + (x0 + 1) * p0)
            };
            ec.encode(fl as u32, fh as u32, ft as u32);
        } else if b0 > 1 || stereo {
            ec.enc_uint(itheta as u32, (qn + 1) as u32);
        } else {
            let ft = ((qn >> 1) + 1) * ((qn >> 1) + 1);
            let fs = if itheta <= qn >> 1 {
                itheta + 1
            } else {
                qn + 1 - itheta
            };
            let fl = if itheta <= qn >> 1 {
                (itheta * (itheta + 1)) >> 1
            } else {
                ft - (((qn + 1 - itheta) * (qn + 2 - itheta)) >> 1)
            };
            ec.encode(fl as u32, (fl + fs) as u32, ft as u32);
        }
        itheta = itheta * 16384 / qn;
        if stereo {
            if itheta == 0 {
                intensity_stereo(x, y, ctx.band_e, ctx.i, ctx.nb_e_bands, nu);
            } else {
                stereo_split(x, y, nu);
            }
        }
    } else if stereo {
        inv = itheta > 8192 && !ctx.disable_inv;
        if inv {
            for v in y.iter_mut().take(nu) {
                *v = -*v;
            }
        }
        intensity_stereo(x, y, ctx.band_e, ctx.i, ctx.nb_e_bands, nu);
        if *b > 2 << BITRES && ctx.remaining_bits > 2 << BITRES {
            ec.enc_bit_logp(inv, 2);
        } else {
            inv = false;
        }
        if ctx.disable_inv {
            inv = false;
        }
        itheta = 0;
    }

    let qalloc = ec.ec_tell_frac() as i32 - tell;
    *b -= qalloc;
    finish_split(itheta, n, big_b, inv, qalloc, fill)
}

/// Decoder side of `compute_theta`: reconstructs theta from the bitstream and
/// returns the same split the encoder produced. `b`/`fill` updated in place.
#[allow(clippy::too_many_arguments)]
pub fn compute_theta_decode(
    ctx: &BandCtx,
    ec: &mut RangeDecoder,
    n: i32,
    big_b: i32,
    b0: i32,
    lm: i32,
    stereo: bool,
    b: &mut i32,
    fill: &mut i32,
) -> SplitCtx {
    let qn = qn_and_offset(ctx, n, *b, lm, stereo);
    let tell = ec.ec_tell_frac() as i32;
    let mut itheta = 0;
    let mut inv = false;

    if qn != 1 {
        if stereo && n > 2 {
            let p0 = 3;
            let x0 = qn / 2;
            let ft = p0 * (x0 + 1) + x0;
            let fs = ec.decode(ft as u32) as i32;
            let xx = if fs < (x0 + 1) * p0 {
                fs / p0
            } else {
                x0 + 1 + (fs - (x0 + 1) * p0)
            };
            let (fl, fh) = if xx <= x0 {
                (p0 * xx, p0 * (xx + 1))
            } else {
                ((xx - 1 - x0) + (x0 + 1) * p0, (xx - x0) + (x0 + 1) * p0)
            };
            ec.dec_update(fl as u32, fh as u32, ft as u32);
            itheta = xx;
        } else if b0 > 1 || stereo {
            itheta = ec.dec_uint((qn + 1) as u32) as i32;
        } else {
            let ft = ((qn >> 1) + 1) * ((qn >> 1) + 1);
            let fm = ec.decode(ft as u32) as i32;
            let (fl, fs);
            if fm < (((qn >> 1) * ((qn >> 1) + 1)) >> 1) {
                itheta = (isqrt32((8 * fm + 1) as u32) as i32 - 1) >> 1;
                fs = itheta + 1;
                fl = (itheta * (itheta + 1)) >> 1;
            } else {
                itheta = (2 * (qn + 1) - isqrt32((8 * (ft - fm - 1) + 1) as u32) as i32) >> 1;
                fs = qn + 1 - itheta;
                fl = ft - (((qn + 1 - itheta) * (qn + 2 - itheta)) >> 1);
            }
            ec.dec_update(fl as u32, (fl + fs) as u32, ft as u32);
        }
        itheta = itheta * 16384 / qn;
    } else if stereo {
        if *b > 2 << BITRES && ctx.remaining_bits > 2 << BITRES {
            inv = ec.dec_bit_logp(2);
        } else {
            inv = false;
        }
        if ctx.disable_inv {
            inv = false;
        }
        itheta = 0;
    }

    let qalloc = ec.ec_tell_frac() as i32 - tell;
    *b -= qalloc;
    finish_split(itheta, n, big_b, inv, qalloc, fill)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(band_e: &[f32]) -> BandCtx<'_> {
        BandCtx {
            log_n: 4 << BITRES,
            i: 0,
            intensity: 8,
            band_e,
            nb_e_bands: 1,
            remaining_bits: 1000,
            theta_round: 0,
            avoid_split_noise: false,
            disable_inv: false,
        }
    }

    #[test]
    fn isqrt32_matches_floor_sqrt() {
        for &v in &[
            0u32,
            1,
            2,
            3,
            4,
            8,
            15,
            16,
            17,
            100,
            99_980,
            1_000_000,
            u32::MAX,
        ] {
            let got = isqrt32(v);
            let expect = (v as f64).sqrt().floor() as u32;
            assert_eq!(got, expect, "isqrt32({v})");
            // Defining property: g^2 <= v < (g+1)^2.
            assert!((got as u64) * (got as u64) <= v as u64);
            assert!((got as u64 + 1) * (got as u64 + 1) > v as u64);
        }
    }

    /// Round-trips the split through encode/decode for a given pdf branch and
    /// asserts the reconstructed angle and derived gains match bit-exactly.
    fn round_trip(stereo: bool, n: i32, b0: i32, x: &[f32], y: &[f32]) -> SplitCtx {
        let band_e = [1.0f32, 1.0];
        let lm = 1;
        let big_b = 1;

        let mut ex = x.to_vec();
        let mut ey = y.to_vec();
        let mut be = 1000;
        let mut efill = (1 << big_b) - 1;
        let mut enc = RangeEncoder::new(256);
        let split_e = compute_theta_encode(
            &ctx(&band_e),
            &mut enc,
            &mut ex,
            &mut ey,
            n,
            big_b,
            b0,
            lm,
            stereo,
            &mut be,
            &mut efill,
        );
        let bytes = enc.done();

        let mut bd = 1000;
        let mut dfill = (1 << big_b) - 1;
        let mut dec = RangeDecoder::new(&bytes);
        let split_d = compute_theta_decode(
            &ctx(&band_e),
            &mut dec,
            n,
            big_b,
            b0,
            lm,
            stereo,
            &mut bd,
            &mut dfill,
        );

        // The angle and every value derived from it must agree across coders.
        assert_eq!(split_e.itheta, split_d.itheta, "itheta");
        assert_eq!(split_e.imid, split_d.imid, "imid");
        assert_eq!(split_e.iside, split_d.iside, "iside");
        assert_eq!(split_e.delta, split_d.delta, "delta");
        assert_eq!(split_e.inv, split_d.inv, "inv");
        assert_eq!(split_e.qalloc, split_d.qalloc, "qalloc");
        assert_eq!(be, bd, "remaining bits b");
        assert_eq!(efill, dfill, "fill");
        split_e
    }

    #[test]
    fn triangular_pdf_round_trips() {
        // Mono split (stereo=false, B0=1) uses the triangular pdf.
        round_trip(
            false,
            8,
            1,
            &[0.6, 0.8, 0.0, 0.0, 0.1, 0.0, 0.2, 0.0],
            &[0.1, 0.0, 0.7, 0.3, 0.0, 0.5, 0.0, 0.4],
        );
        round_trip(false, 16, 1, &[0.25f32; 16], &[0.25f32; 16]);
    }

    #[test]
    fn uniform_pdf_round_trips() {
        // A time split (B0 > 1) uses the uniform pdf.
        round_trip(
            false,
            8,
            2,
            &[0.9, 0.1, 0.3, 0.2, 0.0, 0.0, 0.1, 0.0],
            &[0.0, 0.4, 0.0, 0.6, 0.5, 0.2, 0.0, 0.3],
        );
    }

    #[test]
    fn stereo_step_pdf_round_trips() {
        // Stereo split with N > 2 uses the step pdf.
        let split = round_trip(
            true,
            8,
            1,
            &[0.7, 0.2, 0.5, 0.1, 0.0, 0.3, 0.0, 0.0],
            &[0.1, 0.6, 0.0, 0.4, 0.5, 0.0, 0.2, 0.0],
        );
        assert!((0..=16384).contains(&split.itheta));
    }

    #[test]
    fn gains_satisfy_pythagorean_identity() {
        // For an intermediate angle, imid^2 + iside^2 ~= 32767^2 (unit circle).
        let split = round_trip(false, 16, 1, &[0.25f32; 16], &[0.18f32; 16]);
        if split.itheta != 0 && split.itheta != 16384 {
            let sum = (split.imid as f64).powi(2) + (split.iside as f64).powi(2);
            let unit = (32767f64).powi(2);
            assert!(
                (sum / unit - 1.0).abs() < 0.01,
                "imid^2+iside^2={sum} vs {unit}"
            );
        }
    }
}
