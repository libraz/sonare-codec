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

/// Signal-to-mask ratio (dB) demanded by a fully tonal masker (tone-masking-
/// noise), per the psychoacoustic literature.
const TONE_MASKING_NOISE_DB: f64 = 18.0;

/// Signal-to-mask ratio (dB) demanded by a fully noise-like masker (noise-
/// masking-tone).
const NOISE_MASKING_TONE_DB: f64 = 6.0;

/// Floor used in place of zero energy when forming the geometric mean.
const TONALITY_ENERGY_FLOOR: f64 = 1.0e-12;

/// Estimates the tonality of a spectrum (0 = noise-like, 1 = tonal) from its
/// spectral flatness measure (Johnston).
///
/// The spectral flatness is the ratio of the geometric to the arithmetic mean
/// of the per-bin energies; in dB it ranges from 0 (perfectly flat / noise) down
/// toward large negative values for a pure tone. It is mapped to a tonality
/// index by `min(SFM_dB / −60 dB, 1)`. An empty spectrum is treated as fully
/// noise-like.
#[must_use]
pub fn spectral_flatness_tonality(energy: &[f64]) -> f64 {
    if energy.is_empty() {
        return 0.0;
    }
    let n = energy.len() as f64;
    let mut log_sum = 0.0_f64;
    let mut arith_sum = 0.0_f64;
    for &e in energy {
        let clamped = e.max(TONALITY_ENERGY_FLOOR);
        log_sum += clamped.ln();
        arith_sum += clamped;
    }
    let geometric_mean = (log_sum / n).exp();
    let arithmetic_mean = arith_sum / n;
    let sfm_db = 10.0 * (geometric_mean / arithmetic_mean).log10();
    (sfm_db / -60.0).clamp(0.0, 1.0)
}

/// Computes the per-bin masking threshold energy from a power spectrum.
///
/// Each bin's energy is spread across the bark scale by the Schroeder
/// [`spreading_db`] function (accumulated in the energy domain), then lowered by
/// the signal-to-mask ratio interpolated between the tone- and noise-masking
/// values according to `tonality`. The result is the maximum quantization-noise
/// energy each bin can carry while staying masked. `energy` and `bark` must be
/// the same length.
pub fn spread_masking_threshold(
    energy: &[f64],
    bark: &[f64],
    tonality: f64,
) -> Result<Vec<f64>, Error> {
    if energy.len() != bark.len() {
        return Err(Error::InvalidInput(
            "psychoacoustic energy and bark arrays must match in length",
        ));
    }
    let tonality = tonality.clamp(0.0, 1.0);
    let smr_db = tonality * TONE_MASKING_NOISE_DB + (1.0 - tonality) * NOISE_MASKING_TONE_DB;
    let smr_gain = 10.0_f64.powf(-smr_db / 10.0);

    let mut threshold = Vec::with_capacity(energy.len());
    for &maskee in bark {
        let mut spread = 0.0_f64;
        for (&masker, &masker_energy) in bark.iter().zip(energy.iter()) {
            spread += masker_energy * 10.0_f64.powf(spreading_db(masker, maskee) / 10.0);
        }
        threshold.push(spread * smr_gain);
    }
    Ok(threshold)
}

/// Maps each retained half-spectrum bin to its critical-band rate (bark) for an
/// FFT of length `fft_len` sampled at `sample_rate`.
pub fn bin_barks(num_bins: usize, sample_rate: u32, fft_len: usize) -> Result<Vec<f64>, Error> {
    if fft_len == 0 || sample_rate == 0 {
        return Err(Error::InvalidInput(
            "psychoacoustic bin-bark mapping needs a non-zero FFT length and rate",
        ));
    }
    let resolution = f64::from(sample_rate) / fft_len as f64;
    Ok((0..num_bins).map(|k| bark(k as f64 * resolution)).collect())
}

/// Smallest allowed-noise energy assigned to a band, so a fully masked band
/// still yields a finite (rather than zero) target.
const MIN_ALLOWED_NOISE: f64 = 1.0e-12;

/// Computes the allowed quantization-noise energy per long-block scale-factor
/// band in the MDCT domain.
///
/// The masking threshold is computed in the FFT power-spectrum domain, but
/// scale-factor allocation must compare it against quantization noise in the
/// MDCT domain, whose energy normalization differs. Rather than calibrate an
/// absolute constant between the two, this forms a dimensionless masking ratio
/// per band — `fft_threshold / fft_signal`, the fraction of band energy that
/// noise may reach — and applies it to the band's MDCT signal energy. The
/// transform-dependent normalization cancels.
///
/// `fft_energy` and `fft_threshold` are the per-bin power spectrum and masking
/// threshold (same length); `mdct_spectrum` is the granule's MDCT line spectrum.
/// The result covers the 21 transmitted bands (the residual highest band carries
/// no scale factor).
pub fn perceptual_band_allowed_noise(
    mdct_spectrum: &[f32],
    fft_energy: &[f64],
    fft_threshold: &[f64],
    sample_rate: u32,
    fft_len: usize,
) -> Result<[f64; crate::MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT], Error> {
    if fft_energy.len() != fft_threshold.len() {
        return Err(Error::InvalidInput(
            "psychoacoustic energy and threshold arrays must match in length",
        ));
    }
    if mdct_spectrum.is_empty() || fft_len == 0 || sample_rate == 0 {
        return Err(Error::InvalidInput(
            "perceptual band thresholds need a spectrum, FFT length, and rate",
        ));
    }

    let mdct_lines = mdct_spectrum.len() as f64;
    let mdct_resolution = f64::from(sample_rate) / (2.0 * mdct_lines);
    let fft_resolution = f64::from(sample_rate) / fft_len as f64;

    let mut allowed = [MIN_ALLOWED_NOISE; crate::MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT];
    for (band, slot) in allowed.iter_mut().enumerate() {
        let (start, end) = crate::mpeg1_layer3_long_scalefactor_band_range(band, sample_rate)?;
        let freq_lo = start as f64 * mdct_resolution;
        let freq_hi = end as f64 * mdct_resolution;

        let mut mdct_energy = 0.0_f64;
        for &line in &mdct_spectrum[start.min(mdct_spectrum.len())..end.min(mdct_spectrum.len())] {
            mdct_energy += f64::from(line) * f64::from(line);
        }

        // Accumulate the FFT signal and threshold over the band's frequency span.
        let mut fft_signal = 0.0_f64;
        let mut fft_thresh = 0.0_f64;
        let mut covered = false;
        for (bin, (&energy, &threshold)) in fft_energy.iter().zip(fft_threshold.iter()).enumerate()
        {
            let freq = bin as f64 * fft_resolution;
            if freq >= freq_lo && freq < freq_hi {
                fft_signal += energy;
                fft_thresh += threshold;
                covered = true;
            }
        }
        // Narrow low bands may fall between FFT bins; use the nearest bin so the
        // ratio is still defined.
        if !covered {
            let center = 0.5 * (freq_lo + freq_hi);
            let nearest = ((center / fft_resolution).round() as usize).min(fft_energy.len() - 1);
            fft_signal = fft_energy[nearest];
            fft_thresh = fft_threshold[nearest];
        }

        let mask_ratio = if fft_signal > 0.0 {
            fft_thresh / fft_signal
        } else {
            // No signal in band: noise here is inaudible, allow it freely.
            f64::INFINITY
        };
        *slot = (mask_ratio * mdct_energy).max(MIN_ALLOWED_NOISE);
    }
    Ok(allowed)
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

    #[test]
    fn tonality_separates_tones_from_noise() {
        // A perfectly flat spectrum is maximally noise-like (tonality 0).
        assert!(approx(spectral_flatness_tonality(&[1.0; 64]), 0.0, 1.0e-9));
        // A lone, well-isolated spectral spike is maximally tonal (clamps to 1),
        // and a tone with realistic −40 dB sidelobes still reads as strongly tonal
        // and well above a noise-like spectrum.
        let mut isolated = [1.0e-9_f64; 64];
        isolated[8] = 1.0;
        assert!(spectral_flatness_tonality(&isolated) > 0.99);
        let mut leaky = [1.0e-4_f64; 64];
        leaky[8] = 1.0;
        let leaky_tonality = spectral_flatness_tonality(&leaky);
        assert!(leaky_tonality > 0.3);
        assert!(leaky_tonality > spectral_flatness_tonality(&[1.0; 64]));
        // An empty spectrum is treated as noise-like rather than panicking.
        assert!(approx(spectral_flatness_tonality(&[]), 0.0, 1.0e-12));
    }

    #[test]
    fn masking_threshold_peaks_under_a_tone_and_decays_with_bark() {
        // A single tonal masker at bin 20: the masked threshold should peak at the
        // masker and fall off monotonically with bark distance on the high side.
        let bins = 64usize;
        let bark: Vec<f64> = (0..bins).map(|k| k as f64 * 0.25).collect();
        let mut energy = vec![0.0_f64; bins];
        energy[20] = 1.0;
        let threshold = spread_masking_threshold(&energy, &bark, 1.0).unwrap();

        let peak = threshold
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .map(|(index, _)| index)
            .unwrap();
        assert_eq!(peak, 20);
        for j in 21..bins - 1 {
            assert!(threshold[j] >= threshold[j + 1]);
        }
        // The tonal masker demands an 18 dB signal-to-mask ratio, so its own bin's
        // threshold sits ~18 dB below the masker energy (the spreading peak is ~0 dB).
        let smr_db = 10.0 * (energy[20] / threshold[20]).log10();
        assert!(approx(smr_db, 18.0, 1.0));
    }

    #[test]
    fn masking_threshold_rejects_mismatched_lengths() {
        assert!(spread_masking_threshold(&[1.0, 2.0], &[0.0], 0.5).is_err());
    }

    #[test]
    fn bin_barks_increase_with_frequency() {
        let barks = bin_barks(513, 44_100, 1024).unwrap();
        assert_eq!(barks.len(), 513);
        assert!(approx(barks[0], 0.0, 1.0e-9));
        for pair in barks.windows(2) {
            assert!(pair[1] >= pair[0]);
        }
        assert!(bin_barks(0, 0, 1024).is_err());
    }

    #[test]
    fn allowed_noise_applies_the_mask_ratio_to_mdct_energy() {
        // A uniform FFT threshold/signal ratio means every covered band's allowed
        // noise equals ratio * the band's MDCT signal energy, independent of the
        // FFT vs MDCT normalization — the dimensionless ratio cancels it.
        let fft_len = 1024usize;
        let bins = fft_len / 2 + 1;
        let ratio = 0.1_f64;
        let fft_energy = vec![1.0_f64; bins];
        let fft_threshold = vec![ratio; bins];

        let mut mdct = vec![0.0_f32; 576];
        mdct[2] = 3.0; // band 0 (lines 0..4): energy 9
        mdct[50] = 4.0; // band 9 (lines 44..52): energy 16

        let allowed =
            perceptual_band_allowed_noise(&mdct, &fft_energy, &fft_threshold, 44_100, fft_len)
                .unwrap();

        assert!(approx(allowed[0], ratio * 9.0, 1.0e-9));
        assert!(approx(allowed[9], ratio * 16.0, 1.0e-9));
        // A band with no MDCT energy collapses to the floor, not zero.
        assert!(approx(allowed[2], MIN_ALLOWED_NOISE, 1.0e-15));

        // Doubling the masking threshold doubles the allowed noise.
        let louder_threshold = vec![ratio * 2.0; bins];
        let louder =
            perceptual_band_allowed_noise(&mdct, &fft_energy, &louder_threshold, 44_100, fft_len)
                .unwrap();
        assert!(approx(louder[0], 2.0 * allowed[0], 1.0e-9));
    }

    #[test]
    fn allowed_noise_rejects_mismatched_or_empty_inputs() {
        let mdct = vec![1.0_f32; 576];
        assert!(perceptual_band_allowed_noise(&mdct, &[1.0, 2.0], &[1.0], 44_100, 1024).is_err());
        assert!(perceptual_band_allowed_noise(&[], &[1.0], &[1.0], 44_100, 1024).is_err());
    }
}
