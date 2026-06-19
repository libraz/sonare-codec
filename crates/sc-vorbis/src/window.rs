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

/// Builds the length-`n` Vorbis window with independent left/right overlap
/// half-lengths, per the Vorbis I window construction (§1.3.2). `left` and
/// `right` are the *half* sizes of the neighbouring blocks' windows (e.g. `n/2`
/// for a long neighbour of a long block, `128` for a short neighbour). When
/// `left == right == n/2` this is exactly the symmetric [`vorbis_window`]; a
/// smaller `left`/`right` compresses that edge's slope so a long block overlaps
/// a short neighbour by only `2 * left` (resp. `2 * right`) samples — the window
/// shaping that makes block switching reconstruct (Princen-Bradley still holds
/// over each shared overlap because both edges are slopes of the same curve).
#[must_use]
pub fn vorbis_window_lr(n: usize, left: usize, right: usize) -> Vec<f32> {
    let mut w = vec![0.0f32; n];
    if n == 0 || left == 0 || right == 0 {
        return w;
    }
    let q = n / 4;
    let leftbegin = q.saturating_sub(left / 2);
    let leftend = (q + left / 2).min(n);
    let rightbegin = (3 * q).saturating_sub(right / 2);
    let rightend = (3 * q + right / 2).min(n);

    for (i, slot) in w.iter_mut().enumerate().take(leftend).skip(leftbegin) {
        let p = (i - leftbegin) as f64 + 0.5;
        let s = (FRAC_PI_2 * (p / left as f64)).sin();
        *slot = (FRAC_PI_2 * s * s).sin() as f32;
    }
    for slot in w.iter_mut().take(rightbegin.min(n)).skip(leftend) {
        *slot = 1.0;
    }
    for (i, slot) in w.iter_mut().enumerate().take(rightend).skip(rightbegin) {
        let p = (i - rightbegin) as f64 + 0.5;
        // Falling edge: the mirror of the rising window function.
        let s = (FRAC_PI_2 * (1.0 - p / right as f64)).sin();
        *slot = (FRAC_PI_2 * s * s).sin() as f32;
    }
    w
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
    fn lr_window_reduces_to_the_symmetric_window() {
        // With equal full-size overlaps on both sides, the left/right window is
        // exactly the symmetric Vorbis window.
        for &n in &[256usize, 512, 2048] {
            let sym = vorbis_window(n);
            let lr = vorbis_window_lr(n, n / 2, n / 2);
            for i in 0..n {
                assert!((sym[i] - lr[i]).abs() < 1e-6, "n={n} i={i}");
            }
        }
    }

    #[test]
    fn lr_window_satisfies_overlap_unity_across_a_transition() {
        // A long block (2048) with a short right neighbour overlaps the following
        // short block (256) by 256 samples. Over that shared region the long
        // block's falling edge and the short block's rising edge must obey
        // Princen-Bradley (squares sum to 1), which is what makes the switched
        // overlap-add reconstruct exactly.
        let long = vorbis_window_lr(2048, 1024, 128);
        let short = vorbis_window_lr(256, 128, 128);
        // Long falling edge spans [3*512 - 64, 3*512 + 64) = [1472, 1600); the
        // short block's rising edge is its first 128 samples.
        for k in 0..128 {
            let energy = long[1472 + k] * long[1472 + k] + short[k] * short[k];
            assert!(
                (energy - 1.0).abs() < 1e-5,
                "transition overlap unity broke at {k}: {energy}"
            );
        }
        // Outside the overlap the long block is flat 1 before its edge and 0 after.
        assert!((long[1471] - 1.0).abs() < 1e-6);
        assert!(long[1600..].iter().all(|&v| v == 0.0));
    }

    #[test]
    fn lr_window_satisfies_overlap_unity_on_the_left_transition() {
        // The mirror case: a long block with a short *left* neighbour (lW = 0)
        // compresses its rising edge to 128 samples, overlapping the preceding
        // short block's falling edge. Princen-Bradley must hold over that shared
        // region too (the closing bracket of a short-block group).
        let long = vorbis_window_lr(2048, 128, 1024);
        let short = vorbis_window_lr(256, 128, 128);
        // Long rising edge spans [3*512/... ] = [512 - 64, 512 + 64) = [448, 576);
        // the short block's falling edge is its last 128 samples [128, 256).
        for k in 0..128 {
            let energy = long[448 + k] * long[448 + k] + short[128 + k] * short[128 + k];
            assert!(
                (energy - 1.0).abs() < 1e-5,
                "left-transition overlap unity broke at {k}: {energy}"
            );
        }
        // Before its rising edge the long block is zero; after it, flat 1.
        assert!(long[..448].iter().all(|&v| v == 0.0));
        assert!((long[576] - 1.0).abs() < 1e-6);
    }

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
