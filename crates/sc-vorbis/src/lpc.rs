//! Vorbis LPC analysis and prediction.
//!
//! Hand-ported to safe Rust from libvorbis/aoTuV `lib/lpc.c`:
//! `vorbis_lpc_from_data` (autocorrelation + Levinson-Durbin recursion with the
//! 0.99 spectral damping) and `vorbis_lpc_predict`. Derivative work of
//! libvorbis/aoTuV (BSD-3-Clause); see `LICENSE-THIRDPARTY`.

// Consumed by later Vorbis port stages; the live encoder still ships via FFI.
#![allow(dead_code)]

/// Computes `m` LPC coefficients from `data` via autocorrelation and the
/// Levinson-Durbin recursion. Returns the residual error (as in libvorbis).
///
/// `lpci` receives the `m` coefficients (its length must be at least `m`).
pub fn vorbis_lpc_from_data(data: &[f32], lpci: &mut [f32], m: usize) -> f32 {
    let n = data.len();
    let mut aut = vec![0.0f64; m + 1];
    let mut lpc = vec![0.0f64; m];

    // Autocorrelation, m+1 lag coefficients (double accumulator for depth).
    for j in (0..=m).rev() {
        let mut d = 0.0f64;
        for i in j..n {
            d += f64::from(data[i]) * f64::from(data[i - j]);
        }
        aut[j] = d;
    }

    // Noise floor at about -100 dB.
    let mut error = aut[0] * (1.0 + 1e-10);
    let epsilon = 1e-9 * aut[0] + 1e-10;

    for i in 0..m {
        let mut r = -aut[i + 1];

        if error < epsilon {
            for slot in lpc.iter_mut().take(m).skip(i) {
                *slot = 0.0;
            }
            break;
        }

        for j in 0..i {
            r -= lpc[j] * aut[i - j];
        }
        r /= error;

        lpc[i] = r;
        let mut j = 0;
        while j < i / 2 {
            let tmp = lpc[j];
            lpc[j] += r * lpc[i - 1 - j];
            lpc[i - 1 - j] += r * tmp;
            j += 1;
        }
        if i & 1 == 1 {
            lpc[j] += lpc[j] * r;
        }

        error *= 1.0 - r * r;
    }

    // Slightly damp the filter.
    let g = 0.99f64;
    let mut damp = g;
    for value in lpc.iter_mut().take(m) {
        *value *= damp;
        damp *= g;
    }

    for (dst, &src) in lpci.iter_mut().zip(lpc.iter()).take(m) {
        *dst = src as f32;
    }

    error as f32
}

/// Runs the LPC predictor forward, generating `n` samples.
///
/// `coeff` holds the `m` LPC coefficients; `prime` (if given) holds the `m`
/// priming samples, otherwise priming is zero.
#[must_use]
pub fn vorbis_lpc_predict(coeff: &[f32], prime: Option<&[f32]>, m: usize, n: usize) -> Vec<f32> {
    let mut work = vec![0.0f32; m + n];
    if let Some(prime) = prime {
        work[..m].copy_from_slice(&prime[..m]);
    }

    let mut data = vec![0.0f32; n];
    for (i, out) in data.iter_mut().enumerate() {
        let mut y = 0.0f32;
        let mut o = i;
        let mut p = m;
        for _ in 0..m {
            p -= 1;
            y -= work[o] * coeff[p];
            o += 1;
        }
        *out = y;
        work[o] = y; // o == i + m
    }
    data
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_data_matches_hand_computation() {
        // For a constant signal [1,1,1,1] with m=1: aut[0]=4, aut[1]=3,
        // r = -3/4 = -0.75, then 0.99 damping -> -0.7425, error -> 1.75.
        let mut lpci = [0.0f32; 1];
        let error = vorbis_lpc_from_data(&[1.0, 1.0, 1.0, 1.0], &mut lpci, 1);
        assert!((lpci[0] - (-0.7425)).abs() < 1e-5, "lpc[0]={}", lpci[0]);
        assert!((error - 1.75).abs() < 1e-4, "error={error}");
    }

    #[test]
    fn predict_runs_recurrence() {
        // m=1, coeff=[-0.5] => each sample is 0.5 * previous.
        let out = vorbis_lpc_predict(&[-0.5], Some(&[1.0]), 1, 4);
        let expected = [0.5, 0.25, 0.125, 0.0625];
        for (got, want) in out.iter().zip(&expected) {
            assert!((got - want).abs() < 1e-6, "got {got}, want {want}");
        }
    }

    #[test]
    fn predict_zero_prime_is_zero() {
        let out = vorbis_lpc_predict(&[0.3, -0.2, 0.1], None, 3, 5);
        assert!(out.iter().all(|&x| x == 0.0));
    }

    #[test]
    fn tracks_a_sinusoid_short_horizon() {
        // A sinusoid is well modelled by a low-order LPC; predicting a few
        // samples from its own recent history should track closely.
        let m = 16;
        let train: Vec<f32> = (0..256).map(|i| (0.18 * i as f32).sin() * 0.7).collect();
        let mut coeff = vec![0.0f32; m];
        vorbis_lpc_from_data(&train, &mut coeff, m);

        let start = train.len();
        let prime: Vec<f32> = train[start - m..start].to_vec();
        let predicted = vorbis_lpc_predict(&coeff, Some(&prime), m, 8);
        let actual: Vec<f32> = (start..start + 8)
            .map(|i| (0.18 * i as f32).sin() * 0.7)
            .collect();

        let rms = (predicted
            .iter()
            .zip(&actual)
            .map(|(p, a)| (p - a) * (p - a))
            .sum::<f32>()
            / 8.0)
            .sqrt();
        assert!(rms < 0.1, "prediction RMS too high: {rms}");
    }
}
