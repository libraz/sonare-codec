//! Vorbis MDCT / inverse MDCT.
//!
//! Corresponds to libvorbis/aoTuV `lib/mdct.c`. That file is a heavily
//! factorized (butterfly + bit-reversal) fast transform; since Vorbis is lossy
//! and only the entropy layer must be bit-identical, this port implements the
//! same transform from its closed-form definition rather than transliterating
//! the butterflies. The forward scale matches libvorbis (`scale = 4/n`) so the
//! coefficient magnitudes line up with the floor/residue stages ported later.
//! Derivative work of libvorbis/aoTuV (BSD-3-Clause); see `LICENSE-THIRDPARTY`.
//!
//! For a full window of `m = 2 * half` samples:
//!
//! ```text
//! forward:  X[k] = (4/m) * sum_{i=0}^{m-1} x[i] cos( (2pi/m)(i + 1/2 + m/4)(k + 1/2) )
//! inverse:  y[i] =          sum_{k=0}^{half-1} X[k] cos( (2pi/m)(i + 1/2 + m/4)(k + 1/2) )
//! ```
//!
//! The forward scale `4/m` matches libvorbis so coefficient magnitudes line up
//! with the floor/residue stages; the inverse scale is chosen so a
//! Princen-Bradley window (see [`crate::window`]) reconstructs exactly under
//! 50%-overlap add (time-domain aliasing cancellation).

// Consumed by later Vorbis port stages; the live encoder still ships via FFI.
#![allow(dead_code)]

use std::f64::consts::PI;

/// Forward MDCT: `m` time-domain samples to `m/2` spectral coefficients.
#[must_use]
pub fn mdct_forward(input: &[f32]) -> Vec<f32> {
    let m = input.len();
    debug_assert!(m >= 2 && m % 2 == 0);
    let half = m / 2;
    let mf = m as f64;
    let two_pi_over_m = 2.0 * PI / mf;
    let n0 = 0.5 + mf / 4.0;
    let scale = 4.0 / mf;

    (0..half)
        .map(|k| {
            let kk = k as f64 + 0.5;
            let mut acc = 0.0f64;
            for (i, &sample) in input.iter().enumerate() {
                acc += f64::from(sample) * (two_pi_over_m * (i as f64 + n0) * kk).cos();
            }
            (scale * acc) as f32
        })
        .collect()
}

/// Inverse MDCT: `m/2` spectral coefficients to `m` time-domain samples.
///
/// The result is pre-aliased; apply the synthesis window and overlap-add 50%
/// to reconstruct.
#[must_use]
pub fn imdct(spectrum: &[f32]) -> Vec<f32> {
    let half = spectrum.len();
    let m = half * 2;
    let mf = m as f64;
    let two_pi_over_m = 2.0 * PI / mf;
    let n0 = 0.5 + mf / 4.0;

    (0..m)
        .map(|i| {
            let ii = i as f64 + n0;
            let mut acc = 0.0f64;
            for (k, &coeff) in spectrum.iter().enumerate() {
                acc += f64::from(coeff) * (two_pi_over_m * ii * (k as f64 + 0.5)).cos();
            }
            acc as f32
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::window::vorbis_window;

    fn test_signal(len: usize) -> Vec<f32> {
        (0..len)
            .map(|i| {
                let t = i as f32;
                0.5 * (0.013 * t).sin() + 0.3 * (0.071 * t).cos() + 0.1 * (0.211 * t + 1.0).sin()
            })
            .collect()
    }

    #[test]
    fn output_lengths() {
        assert_eq!(mdct_forward(&[0.0; 64]).len(), 32);
        assert_eq!(imdct(&[0.0; 32]).len(), 64);
    }

    #[test]
    fn zero_in_zero_out() {
        assert!(mdct_forward(&[0.0; 128]).iter().all(|&x| x == 0.0));
        assert!(imdct(&[0.0; 64]).iter().all(|&x| x == 0.0));
    }

    #[test]
    fn windowed_overlap_add_reconstructs() {
        // The definitive MDCT correctness check: window -> forward -> inverse ->
        // window -> 50%-overlap add must reproduce the interior signal exactly.
        for &m in &[64usize, 128, 256] {
            let half = m / 2;
            let window = vorbis_window(m);
            let len = m * 5;
            let signal = test_signal(len);
            let mut recon = vec![0.0f32; len];

            let mut pos = 0;
            while pos + m <= len {
                let frame: Vec<f32> = (0..m).map(|j| signal[pos + j] * window[j]).collect();
                let spectrum = mdct_forward(&frame);
                let time = imdct(&spectrum);
                for j in 0..m {
                    recon[pos + j] += time[j] * window[j];
                }
                pos += half;
            }

            // The first and last half-blocks lack their overlapping partner.
            for i in m..(len - m) {
                assert!(
                    (recon[i] - signal[i]).abs() < 1e-3,
                    "m={m} reconstruct mismatch at {i}: {} vs {}",
                    recon[i],
                    signal[i]
                );
            }
        }
    }

    #[test]
    fn forward_is_linear() {
        let a = test_signal(64);
        let b: Vec<f32> = test_signal(64).iter().map(|x| 0.5 - x).collect();
        let sum: Vec<f32> = a.iter().zip(&b).map(|(x, y)| x + y).collect();

        let fa = mdct_forward(&a);
        let fb = mdct_forward(&b);
        let fsum = mdct_forward(&sum);
        for k in 0..fsum.len() {
            assert!((fsum[k] - (fa[k] + fb[k])).abs() < 1e-4);
        }
    }
}
