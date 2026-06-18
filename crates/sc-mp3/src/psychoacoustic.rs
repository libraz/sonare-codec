//! Psychoacoustic model primitives for MPEG-1 Layer III encoding.
//!
//! These are clean-room building blocks implemented from the public literature
//! (ISO/IEC 11172-3 Annex D Psychoacoustic Model 2, the Davis Pan tutorial, and
//! Painter & Spanias, "Perceptual Coding of Digital Audio"). Rather than copying
//! the spec's sample-rate-specific partition tables, the masking math is derived
//! from closed-form psychoacoustic functions — the Zwicker bark scale, the
//! Terhardt absolute threshold of hearing, and the Schroeder spreading function
//! — which are evaluated at runtime for the FFT bin frequencies.
//!
//! The transforms intentionally favor clarity over speed (a direct DFT), matching
//! the rest of the crate; a factorized FFT can replace [`power_spectrum`] later
//! without changing the surrounding model.

use sc_core::Error;

/// Builds a periodic Hann (raised-cosine) analysis window of the given length.
///
/// The window is `0.5 · (1 − cos(2π·n / N))`, the standard window for the Layer
/// III psychoacoustic FFT. Returns an error for a zero-length request.
pub fn hann_window(len: usize) -> Result<Vec<f64>, Error> {
    if len == 0 {
        return Err(Error::InvalidInput(
            "psychoacoustic window length must be non-zero",
        ));
    }
    let denom = len as f64;
    Ok((0..len)
        .map(|n| 0.5 * (1.0 - (std::f64::consts::TAU * n as f64 / denom).cos()))
        .collect())
}

/// One complex frequency bin of a discrete Fourier transform.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ComplexBin {
    pub re: f64,
    pub im: f64,
}

impl ComplexBin {
    /// Squared magnitude (energy) of the bin.
    #[must_use]
    pub fn energy(self) -> f64 {
        self.re * self.re + self.im * self.im
    }

    /// Magnitude of the bin.
    #[must_use]
    pub fn magnitude(self) -> f64 {
        self.energy().sqrt()
    }

    /// Phase angle of the bin in radians.
    #[must_use]
    pub fn phase(self) -> f64 {
        self.im.atan2(self.re)
    }
}

/// Computes the lower half-spectrum (`0..=N/2`) of a real signal via a direct
/// DFT, returning one [`ComplexBin`] per retained bin.
///
/// Only the non-redundant bins of a real input are returned (`N/2 + 1` of them);
/// the remaining bins are conjugate mirrors. The signal must be non-empty.
pub fn forward_dft_half(signal: &[f64]) -> Result<Vec<ComplexBin>, Error> {
    let n = signal.len();
    if n == 0 {
        return Err(Error::InvalidInput(
            "psychoacoustic DFT input must be non-empty",
        ));
    }
    let bins = n / 2 + 1;
    let scale = std::f64::consts::TAU / n as f64;
    let mut out = Vec::with_capacity(bins);
    for k in 0..bins {
        let mut re = 0.0_f64;
        let mut im = 0.0_f64;
        for (t, &sample) in signal.iter().enumerate() {
            let angle = scale * k as f64 * t as f64;
            re += sample * angle.cos();
            im -= sample * angle.sin();
        }
        out.push(ComplexBin { re, im });
    }
    Ok(out)
}

/// Returns the per-bin energy (squared magnitude) of the half-spectrum of a real
/// signal.
pub fn power_spectrum(signal: &[f64]) -> Result<Vec<f64>, Error> {
    Ok(forward_dft_half(signal)?
        .into_iter()
        .map(ComplexBin::energy)
        .collect())
}

/// Maps a frequency in Hz to the critical-band rate (bark) via the Zwicker
/// approximation `13·atan(0.00076 f) + 3.5·atan((f / 7500)²)`.
#[must_use]
pub fn bark(freq_hz: f64) -> f64 {
    let f = freq_hz.max(0.0);
    13.0 * (0.000_76 * f).atan() + 3.5 * (f / 7500.0).powi(2).atan()
}

/// Absolute threshold of hearing in dB SPL at the given frequency, using the
/// Terhardt closed form (Painter & Spanias eq. 1). Below ~20 Hz the model is
/// clamped to the 20 Hz value to avoid the low-frequency pole.
#[must_use]
pub fn absolute_threshold_db(freq_hz: f64) -> f64 {
    let khz = freq_hz.max(20.0) / 1000.0;
    3.64 * khz.powf(-0.8) - 6.5 * (-0.6 * (khz - 3.3).powi(2)).exp() + 1.0e-3 * khz.powi(4)
}

/// Schroeder spreading function in dB: the masking contribution a masker at
/// `masker_bark` exerts on a maskee at `maskee_bark`.
///
/// `15.81 + 7.5·(z + 0.474) − 17.5·√(1 + (z + 0.474)²)` with `z = maskee − masker`
/// (Painter & Spanias eq. 18). The peak (0 dB) sits just above the masker; the
/// skirts fall off asymmetrically, faster toward lower barks.
#[must_use]
pub fn spreading_db(masker_bark: f64, maskee_bark: f64) -> f64 {
    let z = maskee_bark - masker_bark;
    let shifted = z + 0.474;
    15.81 + 7.5 * shifted - 17.5 * (1.0 + shifted * shifted).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() <= tol
    }

    #[test]
    fn hann_window_is_symmetric_and_zero_at_the_start() {
        let window = hann_window(1024).unwrap();
        assert_eq!(window.len(), 1024);
        assert!(approx(window[0], 0.0, 1.0e-12));
        // A periodic Hann peaks at the midpoint.
        assert!(approx(window[512], 1.0, 1.0e-9));
        // Symmetric about the midpoint (n and N-n match).
        for n in 1..512 {
            assert!(approx(window[n], window[1024 - n], 1.0e-9));
        }
    }

    #[test]
    fn hann_window_rejects_zero_length() {
        assert!(hann_window(0).is_err());
    }

    #[test]
    fn forward_dft_localizes_a_pure_tone() {
        // A cosine at exactly bin 8 of a 64-point DFT must concentrate all energy
        // in bin 8 (and its conjugate, which the half-spectrum drops).
        let n = 64usize;
        let bin = 8usize;
        let signal: Vec<f64> = (0..n)
            .map(|t| (std::f64::consts::TAU * bin as f64 * t as f64 / n as f64).cos())
            .collect();
        let spectrum = power_spectrum(&signal).unwrap();
        assert_eq!(spectrum.len(), n / 2 + 1);
        let peak = spectrum
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .map(|(index, _)| index)
            .unwrap();
        assert_eq!(peak, bin);
        // Off-peak bins carry negligible energy relative to the peak.
        let peak_energy = spectrum[bin];
        for (index, &energy) in spectrum.iter().enumerate() {
            if index != bin {
                assert!(energy < peak_energy * 1.0e-6);
            }
        }
    }

    #[test]
    fn forward_dft_rejects_empty_input() {
        assert!(forward_dft_half(&[]).is_err());
        assert!(power_spectrum(&[]).is_err());
    }

    #[test]
    fn complex_bin_reports_energy_magnitude_and_phase() {
        let bin = ComplexBin { re: 3.0, im: 4.0 };
        assert!(approx(bin.energy(), 25.0, 1.0e-12));
        assert!(approx(bin.magnitude(), 5.0, 1.0e-12));
        assert!(approx(bin.phase(), 4.0_f64.atan2(3.0), 1.0e-12));
    }

    #[test]
    fn bark_scale_tracks_known_anchors() {
        // The bark scale is ~0 at DC, ~8.5 near 1 kHz, and monotone increasing.
        assert!(approx(bark(0.0), 0.0, 1.0e-9));
        assert!(approx(bark(1000.0), 8.5, 0.6));
        assert!(bark(2000.0) > bark(1000.0));
        assert!(bark(8000.0) > bark(4000.0));
    }

    #[test]
    fn absolute_threshold_dips_in_the_most_sensitive_band() {
        // Hearing is most sensitive around 3–4 kHz, where the ATH is near its
        // minimum, and rises steeply at both extremes.
        let mid = absolute_threshold_db(3500.0);
        assert!(mid < absolute_threshold_db(200.0));
        assert!(mid < absolute_threshold_db(15000.0));
        assert!(mid < 5.0);
    }

    #[test]
    fn spreading_function_peaks_at_the_masker() {
        // The spreading function maxes out just above the masker and falls off on
        // both sides; the low-bark skirt drops faster than the high-bark skirt.
        let at_masker = spreading_db(10.0, 10.0);
        assert!(spreading_db(10.0, 12.0) < at_masker);
        assert!(spreading_db(10.0, 8.0) < at_masker);
        // Asymmetry: two barks below the masker is attenuated more than two above.
        assert!(spreading_db(10.0, 8.0) < spreading_db(10.0, 12.0));
    }
}
