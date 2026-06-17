//! Vorbis analysis/synthesis window.
//!
//! Hand-ported to safe Rust from libvorbis/aoTuV `lib/window.c`. The aoTuV
//! source ships the window as precomputed half-window tables (`vwin64` …
//! `vwin8192`); those tables are the canonical Vorbis window evaluated at
//!
//! ```text
//! w[i] = sin( (pi/2) * sin^2( (i + 0.5) / n * pi ) )
//! ```
//!
//! so this port reproduces them directly from the closed form. Derivative work
//! of libvorbis/aoTuV (BSD-3-Clause); see `LICENSE-THIRDPARTY`.

// Consumed by later Vorbis port stages (MDCT windowing); the live encoder still
// ships via the FFI path until those land.
#![allow(dead_code)]

use std::f64::consts::{FRAC_PI_2, PI};

/// Builds the length-`n` Vorbis window (rising to 1 at the centre, symmetric).
///
/// The closed form is evaluated in `f64` and rounded to `f32`, matching how the
/// upstream static tables were generated.
#[must_use]
pub fn vorbis_window(n: usize) -> Vec<f32> {
    (0..n)
        .map(|i| {
            let x = ((i as f64 + 0.5) / n as f64) * PI;
            let s = x.sin();
            (FRAC_PI_2 * s * s).sin() as f32
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Upstream `vwin64` (the rising half of the length-64 window), from
    /// libvorbis/aoTuV `lib/window.c`, used here purely as a test oracle. The
    /// literals are kept verbatim from the upstream table for fidelity.
    #[allow(clippy::excessive_precision)]
    const VWIN64: [f32; 32] = [
        0.0009460463,
        0.0085006468,
        0.0235352254,
        0.0458950567,
        0.0753351908,
        0.1115073077,
        0.1539457973,
        0.2020557475,
        0.2551056759,
        0.3122276645,
        0.3724270287,
        0.4346027792,
        0.4975789974,
        0.5601459521,
        0.6211085051,
        0.6793382689,
        0.7338252629,
        0.7837245849,
        0.8283939355,
        0.8674186656,
        0.9006222429,
        0.9280614787,
        0.9500073081,
        0.9669131782,
        0.9793740220,
        0.9880792941,
        0.9937636139,
        0.9971582668,
        0.9989462667,
        0.9997230082,
        0.9999638688,
        0.9999995525,
    ];

    #[test]
    fn matches_upstream_vwin64_table() {
        let window = vorbis_window(64);
        for (i, &expected) in VWIN64.iter().enumerate() {
            assert!(
                (window[i] - expected).abs() < 1e-6,
                "vwin64[{i}]: got {}, want {expected}",
                window[i]
            );
        }
    }

    #[test]
    fn is_symmetric() {
        let n = 256;
        let window = vorbis_window(n);
        for i in 0..n {
            assert!(
                (window[i] - window[n - 1 - i]).abs() < 1e-7,
                "asymmetry at {i}"
            );
        }
    }

    #[test]
    fn satisfies_overlap_add_unity() {
        // The Vorbis window obeys w[i]^2 + w[i + n/2]^2 == 1 (Princen-Bradley),
        // which is what makes the overlapped MDCT reconstruct exactly.
        let n = 512;
        let window = vorbis_window(n);
        for i in 0..n / 2 {
            let energy = window[i] * window[i] + window[i + n / 2] * window[i + n / 2];
            assert!(
                (energy - 1.0).abs() < 1e-6,
                "overlap unity broke at {i}: {energy}"
            );
        }
    }

    #[test]
    fn endpoints_and_centre() {
        let window = vorbis_window(128);
        assert!(window[0] > 0.0 && window[0] < 0.01);
        assert!(window[63] > 0.99 && window[64] > 0.99);
    }
}
