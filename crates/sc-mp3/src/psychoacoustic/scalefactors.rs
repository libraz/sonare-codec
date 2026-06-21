use super::*;

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
pub(crate) const MIN_ALLOWED_NOISE: f64 = 1.0e-12;

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

        *slot = if mdct_energy <= 0.0 {
            // Silent band: no signal to mask, so the target is irrelevant; pin it
            // to the floor rather than forming 0 · ∞.
            MIN_ALLOWED_NOISE
        } else if fft_signal > 0.0 {
            (fft_thresh / fft_signal * mdct_energy).max(MIN_ALLOWED_NOISE)
        } else {
            // Signal present in the MDCT band but none in the FFT span: treat as
            // fully masked so the band is never forced to spend bits.
            f64::INFINITY
        };
    }
    Ok(allowed)
}

/// Syntax width cap for a transmitted long-block scale-factor band: bands 0..11
/// carry `slen1` (up to 4 bits → 15), bands 11..21 carry `slen2` (up to 3 bits
/// → 7).
pub(crate) fn band_scalefactor_cap(band: usize) -> u8 {
    if band < 11 {
        15
    } else {
        7
    }
}

/// Allocates per-band long-block scale factors so quantization noise stays below
/// the perceptual allowed-noise target.
///
/// Starting from zero, the noise-control loop quantizes the spectrum, measures
/// the requantization-noise energy in each scale-factor band, and raises the
/// scale factor of every band whose noise exceeds its target and still has
/// headroom in its syntax width. It repeats until all bands are satisfied or
/// capped. If amplification would push a band past the quantizer's magnitude
/// bound, the last allocation that quantized cleanly is returned. The result
/// feeds [`crate::quantize_mpeg1_layer3_long_spectrum_with_scalefactors`].
/// Largest quantized magnitude the Layer III big-value/count1 partition can
/// carry, so the `pow(4/3)` reconstruction table covers every valid index.
const POW_4_3_TABLE_LEN: usize = 8192;

/// Reconstruction power `x^(4/3)` for integer quantizer outputs, precomputed.
///
/// The noise-control loop evaluates `is^(4/3)` for every spectral line on every
/// pass; the integer domain `[0, 8191]` makes a lookup table exact and removes
/// the per-line `powf` from the inner loop.
fn pow_4_3_table() -> &'static [f64; POW_4_3_TABLE_LEN] {
    static TABLE: std::sync::OnceLock<Box<[f64; POW_4_3_TABLE_LEN]>> = std::sync::OnceLock::new();
    TABLE.get_or_init(|| {
        let mut table = Box::new([0.0_f64; POW_4_3_TABLE_LEN]);
        for (value, slot) in table.iter_mut().enumerate() {
            *slot = (value as f64).powf(4.0 / 3.0);
        }
        table
    })
}

/// `magnitude^(4/3)`, table-backed for in-range integer magnitudes and falling
/// back to a direct `powf` (bit-identical) for any out-of-range index.
fn pow_4_3(magnitude: u32) -> f64 {
    let index = magnitude as usize;
    match pow_4_3_table().get(index) {
        Some(&value) => value,
        None => (index as f64).powf(4.0 / 3.0),
    }
}

pub fn allocate_long_block_scalefactors(
    mdct_spectrum: &[f32],
    allowed_noise: &[f64; crate::MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT],
    step: f32,
    scalefac_scale: bool,
    sample_rate: u32,
) -> Result<[u8; crate::MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT], Error> {
    if !step.is_finite() || step <= 0.0 {
        return Err(Error::InvalidInput("quantization step must be positive"));
    }
    let global_gain = crate::mpeg1_layer3_global_gain_for_step(step);
    let gain = 2.0_f64.powf(0.25 * (f64::from(global_gain) - 210.0));
    let multiplier = if scalefac_scale { 1.0 } else { 0.5 };

    let mut scale_factors = [0_u8; crate::MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT];
    let mut last_good = scale_factors;
    // `|xr|^0.75` is invariant across passes; compute it once for the loop. A
    // non-finite line bails to the (all-zero) last-good fit, matching the
    // per-pass quantizer's error handling.
    let magnitudes = match crate::layer3_long_spectrum_quantizer_magnitudes(mdct_spectrum) {
        Ok(magnitudes) => magnitudes,
        Err(_) => return Ok(last_good),
    };
    // Each pass raises at least one band by one; bound iterations by the total
    // scale-factor headroom so the loop always terminates.
    let max_iterations: usize = (0..crate::MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT)
        .map(|band| usize::from(band_scalefactor_cap(band)))
        .sum();

    for _ in 0..=max_iterations {
        let quantized =
            match crate::quantize_mpeg1_layer3_long_spectrum_with_scalefactors_and_magnitudes(
                mdct_spectrum,
                &magnitudes,
                step,
                &scale_factors,
                scalefac_scale,
                sample_rate,
            ) {
                Ok(quantized) => quantized,
                // Amplification clipped the quantizer; fall back to the last clean fit.
                Err(_) => return Ok(last_good),
            };
        last_good = scale_factors;

        let mut raised = false;
        for band in 0..crate::MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT {
            if scale_factors[band] >= band_scalefactor_cap(band) {
                continue;
            }
            let (start, end) = crate::mpeg1_layer3_long_scalefactor_band_range(band, sample_rate)?;
            let attenuation = 2.0_f64.powf(-multiplier * f64::from(scale_factors[band]));

            let mut noise = 0.0_f64;
            for line in start..end.min(mdct_spectrum.len()) {
                let is = quantized[line];
                let sign = if is < 0 { -1.0 } else { 1.0 };
                let reconstructed = pow_4_3(is.unsigned_abs()) * gain * attenuation * sign;
                let error = f64::from(mdct_spectrum[line]) - reconstructed;
                noise += error * error;
            }
            if noise > allowed_noise[band] {
                scale_factors[band] += 1;
                raised = true;
            }
        }
        if !raised {
            return Ok(scale_factors);
        }
    }
    Ok(scale_factors)
}

/// Derives perceptual long-block scale factors for one granule.
///
/// Runs the full model: Hann-window and transform `pcm_window` to a power
/// spectrum, estimate its tonality, spread it into a masking threshold, convert
/// that to per-band allowed noise against the granule's MDCT spectrum, and run
/// the noise-control allocation. `pcm_window` is the block of PCM samples the
/// FFT analyses (its length is the FFT length); `mdct_spectrum` is the granule's
/// MDCT line spectrum in whatever sign convention the caller quantizes (energy
/// is sign-independent). The returned scale factors feed
/// [`crate::quantize_mpeg1_layer3_long_spectrum_with_scalefactors`].
pub fn perceptual_long_block_scalefactors(
    mdct_spectrum: &[f32],
    pcm_window: &[f64],
    step: f32,
    scalefac_scale: bool,
    sample_rate: u32,
) -> Result<[u8; crate::MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT], Error> {
    perceptual_long_block_scalefactors_with_allowed_noise_scale(
        mdct_spectrum,
        pcm_window,
        step,
        scalefac_scale,
        sample_rate,
        1.0,
    )
}

/// Derives perceptual long-block scale factors with a caller-supplied
/// allowed-noise multiplier. Values below 1.0 tighten the masking target and
/// can force more scale-factor allocation; 1.0 matches
/// [`perceptual_long_block_scalefactors`].
pub fn perceptual_long_block_scalefactors_with_allowed_noise_scale(
    mdct_spectrum: &[f32],
    pcm_window: &[f64],
    step: f32,
    scalefac_scale: bool,
    sample_rate: u32,
    allowed_noise_scale: f64,
) -> Result<[u8; crate::MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT], Error> {
    if !allowed_noise_scale.is_finite() || allowed_noise_scale <= 0.0 {
        return Err(Error::InvalidInput(
            "MP3 allowed-noise scale must be positive and finite",
        ));
    }
    let fft_len = pcm_window.len();
    let window = hann_window(fft_len)?;
    let windowed: Vec<f64> = pcm_window
        .iter()
        .zip(window.iter())
        .map(|(&sample, &scale)| sample * scale)
        .collect();
    let energy = power_spectrum(&windowed)?;
    let tonality = windowed_tonality(&energy, TONALITY_WINDOW_BINS)?;
    let barks = bin_barks(energy.len(), sample_rate, fft_len)?;
    let threshold = spread_masking_threshold_per_bin(&energy, &barks, &tonality)?;
    let mut allowed =
        perceptual_band_allowed_noise(mdct_spectrum, &energy, &threshold, sample_rate, fft_len)?;
    for target in &mut allowed {
        *target *= allowed_noise_scale;
    }
    allocate_long_block_scalefactors(mdct_spectrum, &allowed, step, scalefac_scale, sample_rate)
}

/// Derives the long-block allowed-noise target used by
/// [`perceptual_long_block_scalefactors`].
///
/// This exposes the same clean-room masking threshold path to encoder-side
/// quality guards, so they can compare candidate payloads in perceptual units
/// instead of raw unweighted spectral error.
pub fn perceptual_long_block_allowed_noise(
    mdct_spectrum: &[f32],
    pcm_window: &[f64],
    sample_rate: u32,
) -> Result<[f64; crate::MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT], Error> {
    let fft_len = pcm_window.len();
    let window = hann_window(fft_len)?;
    let windowed: Vec<f64> = pcm_window
        .iter()
        .zip(window.iter())
        .map(|(&sample, &scale)| sample * scale)
        .collect();
    let energy = power_spectrum(&windowed)?;
    let tonality = windowed_tonality(&energy, TONALITY_WINDOW_BINS)?;
    let barks = bin_barks(energy.len(), sample_rate, fft_len)?;
    let threshold = spread_masking_threshold_per_bin(&energy, &barks, &tonality)?;
    perceptual_band_allowed_noise(mdct_spectrum, &energy, &threshold, sample_rate, fft_len)
}

/// Consolidated perceptual analysis of one long block.
///
/// Bundles every perceptual decision an encoder needs for a granule: the per-band
/// allowed quantization noise (for scale-factor allocation), the perceptual
/// entropy (the granule's bit demand, for rate control), and the transient flag
/// (the long/short block-switching decision). It is the single entry point the
/// encoder calls; the individual functions remain available for finer use.
#[derive(Clone, Debug, PartialEq)]
pub struct LongBlockAnalysis {
    /// Per-band allowed quantization-noise energy in the MDCT domain, one entry
    /// per transmitted long-block scale-factor band.
    pub allowed_noise: [f64; crate::MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT],
    /// Estimated bits the granule demands to keep its noise below the masking
    /// threshold.
    pub perceptual_entropy: f64,
    /// Whether the block should switch to short blocks (a transient onset).
    pub transient: bool,
}

/// Runs the full long-block perceptual analysis in one pass.
///
/// The masking threshold, allowed noise, and perceptual entropy are all derived
/// from a single shared FFT of `pcm_window`, so the encoder pays for the
/// transform once rather than once per signal. The transient decision is taken
/// directly from `pcm_window`. Returns a [`LongBlockAnalysis`] bundle.
pub fn analyze_long_block(
    mdct_spectrum: &[f32],
    pcm_window: &[f64],
    sample_rate: u32,
) -> Result<LongBlockAnalysis, Error> {
    if !all_finite_f64(pcm_window) || !all_finite_f32(mdct_spectrum) {
        return Err(Error::InvalidInput(
            "perceptual analysis input must be finite",
        ));
    }
    let fft_len = pcm_window.len();
    let window = hann_window(fft_len)?;
    let windowed: Vec<f64> = pcm_window
        .iter()
        .zip(window.iter())
        .map(|(&sample, &scale)| sample * scale)
        .collect();
    let energy = power_spectrum(&windowed)?;
    let tonality = windowed_tonality(&energy, TONALITY_WINDOW_BINS)?;
    let barks = bin_barks(energy.len(), sample_rate, fft_len)?;
    let threshold = spread_masking_threshold_per_bin(&energy, &barks, &tonality)?;
    let allowed_noise =
        perceptual_band_allowed_noise(mdct_spectrum, &energy, &threshold, sample_rate, fft_len)?;
    let entropy = perceptual_entropy(&energy, &threshold)?;
    let transient = is_transient_block(pcm_window)?;
    Ok(LongBlockAnalysis {
        allowed_noise,
        perceptual_entropy: entropy,
        transient,
    })
}
