//! Vorbis psychoacoustic model — foundational scales and curve combinators.
//!
//! Hand-ported to safe Rust from libvorbis/aoTuV `lib/scales.h` (the
//! linear↔dB / Bark / Mel / octave conversions) and `lib/psy.c` (`min_curve`,
//! `max_curve`, `attenuate_curve`): the perceptual coordinate transforms and
//! the per-bin curve combinators the masking model is built on. Derivative
//! work of libvorbis/aoTuV (BSD-3-Clause); see `LICENSE-THIRDPARTY`.
//!
//! The dB conversions use the IEEE-754 bit-twiddling fast paths libvorbis
//! enables under `VORBIS_IEEE_FLOAT32`; in Rust those are expressed with
//! `f32::to_bits` / `f32::from_bits`, so no `unsafe` is needed. The remaining
//! transforms are the `scales.h` closed forms verbatim.

// Foundation for the masking model; the live encoder still ships via FFI until
// the analysis stages land.
#![allow(dead_code)]

/// Number of points in an Ehmer masking curve (`EHMER_MAX`).
pub const EHMER_MAX: usize = 56;

/// Index of the curve's centre (driving-tone) point (`EHMER_OFFSET`).
pub const EHMER_OFFSET: usize = 16;

/// Approximate `20*log10(|x|)` via the IEEE-754 exponent (`todB`). The bit
/// pattern of `|x|`, read as an integer, is an affine proxy for the base-2
/// logarithm; the constants recentre it onto the decibel scale.
#[must_use]
pub fn to_db(x: f32) -> f32 {
    let bits = x.to_bits() & 0x7fff_ffff;
    bits as f32 * 7.177_114e-7 - 764.616_2
}

/// `unitnorm`: `|x|`-normalised to magnitude 1, keeping only the sign of `x`
/// (so `+0.0 -> 1.0`, `-3.5 -> -1.0`).
#[must_use]
pub fn unit_norm(x: f32) -> f32 {
    f32::from_bits((x.to_bits() & 0x8000_0000) | 0x3f80_0000)
}

/// Inverse of [`to_db`]: `10^(db/20)` (`fromdB`). `0.11512925 == ln(10)/20`.
#[must_use]
pub fn from_db(db: f32) -> f32 {
    (db * 0.115_129_25).exp()
}

/// Frequency (Hz) to the Bark critical-band scale (`toBARK`), a fit valid from
/// 0 Hz to roughly 30 kHz.
#[must_use]
pub fn to_bark(n: f32) -> f32 {
    13.1 * (0.000_74 * n).atan() + 2.24 * (n * n * 1.85e-8).atan() + 1e-4 * n
}

/// Bark to frequency (Hz) (`fromBARK`), the companion fit to [`to_bark`].
#[must_use]
pub fn from_bark(z: f32) -> f32 {
    102.0 * z - 2.0 * z.powi(2) + 0.4 * z.powi(3) + 1.46f32.powf(z) - 1.0
}

/// Frequency (Hz) to the Mel scale (`toMEL`).
#[must_use]
pub fn to_mel(n: f32) -> f32 {
    (1.0 + n * 0.001).ln() * 1442.695
}

/// Mel to frequency (Hz) (`fromMEL`); the exact inverse of [`to_mel`].
#[must_use]
pub fn from_mel(m: f32) -> f32 {
    1000.0 * (m / 1442.695).exp() - 1000.0
}

/// Frequency (Hz) to octaves, with 63.5 Hz declared octave 0 (`toOC`).
#[must_use]
pub fn to_oc(n: f32) -> f32 {
    n.ln() * std::f32::consts::LOG2_E - 5.965_784
}

/// Octaves to frequency (Hz) (`fromOC`); the inverse of [`to_oc`].
#[must_use]
pub fn from_oc(o: f32) -> f32 {
    ((o + 5.965_784) * std::f32::consts::LN_2).exp()
}

/// Element-wise minimum into `c` (`min_curve`): `c[i] = min(c[i], c2[i])`.
pub fn min_curve(c: &mut [f32], c2: &[f32]) {
    for (dst, &src) in c.iter_mut().zip(c2) {
        if src < *dst {
            *dst = src;
        }
    }
}

/// Element-wise maximum into `c` (`max_curve`): `c[i] = max(c[i], c2[i])`.
pub fn max_curve(c: &mut [f32], c2: &[f32]) {
    for (dst, &src) in c.iter_mut().zip(c2) {
        if src > *dst {
            *dst = src;
        }
    }
}

/// Shifts every point of `c` by `att` dB (`attenuate_curve`).
pub fn attenuate_curve(c: &mut [f32], att: f32) {
    for v in c.iter_mut() {
        *v += att;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_db_centres_unity_at_zero() {
        // 20*log10(1) == 0 dB; the fast path is within a small fraction of a dB.
        assert!(to_db(1.0).abs() < 0.05, "to_db(1.0) = {}", to_db(1.0));
    }

    #[test]
    fn to_db_tracks_twenty_log10() {
        // A factor of 10 in amplitude is +20 dB; a factor of 0.1 is -20 dB.
        // The bit-pattern fast path carries up to ~0.5 dB of log-domain ripple.
        assert!((to_db(10.0) - 20.0).abs() < 0.6, "{}", to_db(10.0));
        assert!((to_db(0.1) + 20.0).abs() < 0.6, "{}", to_db(0.1));
        // Sign is ignored (magnitude only).
        assert_eq!(to_db(-3.5), to_db(3.5));
    }

    #[test]
    fn from_db_inverts_to_db() {
        // to_db's ~0.5 dB ripple maps to a few percent of amplitude on return.
        for &x in &[0.01f32, 0.5, 1.0, 2.0, 100.0] {
            let round = from_db(to_db(x));
            assert!((round - x).abs() / x < 0.07, "x={x} round={round}");
        }
    }

    #[test]
    fn unit_norm_keeps_only_the_sign() {
        assert_eq!(unit_norm(3.5), 1.0);
        assert_eq!(unit_norm(0.001), 1.0);
        assert_eq!(unit_norm(0.0), 1.0);
        assert_eq!(unit_norm(-42.0), -1.0);
        assert_eq!(unit_norm(-0.0), -1.0);
    }

    #[test]
    fn bark_scale_is_monotonic_from_zero() {
        assert!(to_bark(0.0).abs() < 1e-3);
        let mut last = f32::NEG_INFINITY;
        for f in (0..30_000).step_by(500) {
            let z = to_bark(f as f32);
            assert!(z > last, "non-monotonic at {f}: {z} <= {last}");
            last = z;
        }
    }

    #[test]
    fn mel_scale_round_trips_exactly() {
        // from_mel(to_mel(n)) == n analytically; check to float tolerance.
        for &n in &[0.0f32, 100.0, 1000.0, 8000.0, 20000.0] {
            let round = from_mel(to_mel(n));
            assert!((round - n).abs() < 1e-2, "n={n} round={round}");
        }
    }

    #[test]
    fn octave_scale_round_trips() {
        for &n in &[63.5f32, 100.0, 440.0, 4000.0, 16000.0] {
            let round = from_oc(to_oc(n));
            assert!((round - n).abs() / n < 1e-3, "n={n} round={round}");
        }
        // Octave 0 sits near the low-60s Hz the scale is anchored to.
        assert!((from_oc(0.0) - 62.5).abs() < 0.5, "{}", from_oc(0.0));
    }

    #[test]
    fn curve_combinators_match_elementwise_definitions() {
        let mut c = vec![1.0f32, 5.0, 3.0, 2.0];
        let c2 = vec![2.0f32, 4.0, 3.0, 0.0];

        let mut lo = c.clone();
        min_curve(&mut lo, &c2);
        assert_eq!(lo, vec![1.0, 4.0, 3.0, 0.0]);

        let mut hi = c.clone();
        max_curve(&mut hi, &c2);
        assert_eq!(hi, vec![2.0, 5.0, 3.0, 2.0]);

        attenuate_curve(&mut c, -6.0);
        assert_eq!(c, vec![-5.0, -1.0, -3.0, -4.0]);
    }
}
