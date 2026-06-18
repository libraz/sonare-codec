//! CELT band-split angle (theta) bit allocation primitives.
//!
//! Hand-ported to safe Rust from libopus `celt/bands.c` (`bitexact_cos`,
//! `bitexact_log2tan`, `compute_qn`). These are the integer, platform-bit-exact
//! helpers behind `compute_theta`: the resolution `qn` given to the split
//! angle, and the cosine / log2-of-tangent used to turn the chosen angle into
//! the mid/side gain split and the bit-allocation delta. Their bit-exactness is
//! required because it drives the bit allocation.

// Consumed by the CELT band-split stage; the live encoder still ships via FFI.
#![allow(dead_code)]

/// `BITRES`: bit-allocation fractional-bit resolution (eighths of a bit).
pub const BITRES: i32 = 3;

/// `FRAC_MUL16`: rounded Q15 product of two values truncated to 16 bits.
pub(crate) fn frac_mul16(a: i32, b: i32) -> i32 {
    (16384 + (a as i16 as i32) * (b as i16 as i32)) >> 15
}

/// `EC_ILOG`: number of significant bits in `x` (`0` for `x == 0`).
fn ec_ilog(x: u32) -> i32 {
    (32 - x.leading_zeros()) as i32
}

/// `bitexact_cos`: a platform-bit-exact cosine approximation. Input is a Q14
/// quarter-angle in `(0, 16384)`; output is the Q15 cosine in `(0, 32767]`.
#[must_use]
pub fn bitexact_cos(x: i16) -> i16 {
    let tmp = (4096 + (x as i32) * (x as i32)) >> 13;
    let x2 = tmp;
    let x2 = (32767 - x2) + frac_mul16(x2, -7651 + frac_mul16(x2, 8277 + frac_mul16(-626, x2)));
    (1 + x2) as i16
}

/// `bitexact_log2tan`: a bit-exact `2048 * log2(isin/icos)` (Q11), used to
/// derive the mid/side bit-allocation split. Antisymmetric in its arguments.
#[must_use]
pub fn bitexact_log2tan(isin: i32, icos: i32) -> i32 {
    let lc = ec_ilog(icos as u32);
    let ls = ec_ilog(isin as u32);
    let icos = icos << (15 - lc);
    let isin = isin << (15 - ls);
    (ls - lc) * (1 << 11) + frac_mul16(isin, frac_mul16(isin, -2597) + 7932)
        - frac_mul16(icos, frac_mul16(icos, -2597) + 7932)
}

/// `compute_qn`: the number of quantization levels for the split angle theta,
/// given the band size `N`, available bits `b` (eighths), the `offset`, the
/// `pulse_cap`, and whether this is a stereo split. Always even (or 1).
#[must_use]
pub fn compute_qn(n: i32, b: i32, offset: i32, pulse_cap: i32, stereo: bool) -> i32 {
    const EXP2_TABLE8: [i32; 8] = [16384, 17866, 19483, 21247, 23170, 25267, 27554, 30048];
    let mut n2 = 2 * n - 1;
    if stereo && n == 2 {
        n2 -= 1;
    }
    // celt_sudiv is plain integer division.
    let mut qb = (b + n2 * offset) / n2;
    qb = qb.min(b - pulse_cap - (4 << BITRES));
    qb = qb.min(8 << BITRES);
    if qb < (1 << BITRES >> 1) {
        1
    } else {
        let qn = EXP2_TABLE8[(qb & 0x7) as usize] >> (14 - (qb >> BITRES));
        (qn + 1) >> 1 << 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bitexact_cos_approximates_cosine() {
        // x is a Q14 quarter-angle: cos(x/16384 * pi/2) in Q15. Valid theta
        // points are multiples of 16384/qn (qn <= 256), so x >= 64.
        for &x in &[64i16, 2048, 4096, 8192, 12288, 16320] {
            let approx = bitexact_cos(x) as f32 / 32768.0;
            let exact = (x as f32 / 16384.0 * std::f32::consts::FRAC_PI_2).cos();
            assert!(
                (approx - exact).abs() < 0.001,
                "cos({x}): approx={approx} exact={exact}"
            );
        }
        // The 45-degree point is the well-known 1/sqrt(2) value.
        assert_eq!(bitexact_cos(8192), 23171);
    }

    #[test]
    fn log2tan_is_zero_at_equality_and_antisymmetric() {
        assert_eq!(bitexact_log2tan(10000, 10000), 0);
        for &(a, b) in &[(30000, 10000), (5000, 25000), (16384, 32767)] {
            assert_eq!(bitexact_log2tan(a, b), -bitexact_log2tan(b, a), "({a},{b})");
        }
    }

    #[test]
    fn log2tan_tracks_log2_ratio() {
        // isin = 2*icos -> tan ratio 2 -> ~2048*log2(2) = 2048 (Q11).
        let v = bitexact_log2tan(20000, 10000);
        assert!((v - 2048).abs() < 32, "log2tan(2x)={v}");
    }

    #[test]
    fn compute_qn_is_even_or_one_and_bounded() {
        for &n in &[2i32, 4, 8, 16] {
            for &b in &[0i32, 40, 120, 400, 1000] {
                for &stereo in &[false, true] {
                    let qn = compute_qn(n, b, 4, 40, stereo);
                    assert!(qn == 1 || qn % 2 == 0, "qn={qn}");
                    assert!((1..=256).contains(&qn), "qn={qn} out of range");
                }
            }
        }
    }

    #[test]
    fn compute_qn_collapses_to_one_when_starved() {
        // Very few bits -> qb below threshold -> qn == 1.
        assert_eq!(compute_qn(16, 0, 4, 40, false), 1);
    }

    #[test]
    fn compute_qn_grows_with_bits() {
        let low = compute_qn(16, 120, 4, 40, false);
        let high = compute_qn(16, 600, 4, 40, false);
        assert!(
            high >= low,
            "qn should not shrink with more bits: {low} -> {high}"
        );
        assert!(high > 1);
    }
}
