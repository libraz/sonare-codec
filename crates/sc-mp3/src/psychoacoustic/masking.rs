use super::*;

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
pub(crate) const TONE_MASKING_NOISE_DB: f64 = 18.0;

/// Signal-to-mask ratio (dB) demanded by a fully noise-like masker (noise-
/// masking-tone).
pub(crate) const NOISE_MASKING_TONE_DB: f64 = 6.0;

/// Floor used in place of zero energy when forming the geometric mean.
pub(crate) const TONALITY_ENERGY_FLOOR: f64 = 1.0e-12;

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

/// Sliding-window width (in FFT bins) for the per-bin tonality estimate. Wide
/// enough to average out a single tone's main lobe yet narrow relative to the
/// half-spectrum, so tonal and noise-like regions are distinguished locally.
pub(crate) const TONALITY_WINDOW_BINS: usize = 17;

/// Estimates a per-bin tonality index (0 = noise-like … 1 = tonal) from a local
/// spectral flatness measure over a sliding window of `window` bins.
///
/// Unlike [`spectral_flatness_tonality`], which collapses the whole spectrum to a
/// single value, this resolves tonality by frequency: a region dominated by a
/// pure tone reads near 1 while a flat, noisy region reads near 0. Each bin's
/// index is the flatness of the window centred on it (clamped at the edges). An
/// empty spectrum yields an empty result; a zero window width is rejected.
pub fn windowed_tonality(energy: &[f64], window: usize) -> Result<Vec<f64>, Error> {
    if energy.is_empty() {
        return Ok(Vec::new());
    }
    if window == 0 {
        return Err(Error::InvalidInput(
            "psychoacoustic tonality window width must be non-zero",
        ));
    }
    let half = window / 2;
    let n = energy.len();
    let mut out = Vec::with_capacity(n);
    for center in 0..n {
        let lo = center.saturating_sub(half);
        let hi = (center + half + 1).min(n);
        out.push(spectral_flatness_tonality(&energy[lo..hi]));
    }
    Ok(out)
}

/// Per-bin variant of [`spread_masking_threshold`]: every masker contributes with
/// its own tonality-dependent signal-to-mask ratio.
///
/// Each masker's required SMR is interpolated between the tone- and noise-masking
/// values from its local `tonality`, then applied to its spread contribution
/// before accumulation — so a tonal peak imposes the full 18 dB ratio on the
/// bands it spreads into while a noise-like region imposes only 6 dB. `energy`,
/// `bark`, and `tonality` must all be the same length. This is a strict
/// generalization: a constant tonality reproduces [`spread_masking_threshold`].
pub fn spread_masking_threshold_per_bin(
    energy: &[f64],
    bark: &[f64],
    tonality: &[f64],
) -> Result<Vec<f64>, Error> {
    if energy.len() != bark.len() || energy.len() != tonality.len() {
        return Err(Error::InvalidInput(
            "psychoacoustic energy, bark, and tonality arrays must match in length",
        ));
    }
    let smr_gain: Vec<f64> = tonality
        .iter()
        .map(|&t| {
            let t = t.clamp(0.0, 1.0);
            let smr_db = t * TONE_MASKING_NOISE_DB + (1.0 - t) * NOISE_MASKING_TONE_DB;
            10.0_f64.powf(-smr_db / 10.0)
        })
        .collect();

    let mut threshold = Vec::with_capacity(energy.len());
    for &maskee in bark {
        let mut spread = 0.0_f64;
        for ((&masker, &masker_energy), &gain) in
            bark.iter().zip(energy.iter()).zip(smr_gain.iter())
        {
            spread += masker_energy * gain * 10.0_f64.powf(spreading_db(masker, maskee) / 10.0);
        }
        threshold.push(spread);
    }
    Ok(threshold)
}

/// Estimates the perceptual entropy of a granule, in bits, from its power
/// spectrum and masking threshold.
///
/// Following Johnston, each bin contributes the bits needed to code its signal
/// down to the masking threshold — `log2(2·round(√(e / thr)) + 1)` — so a bin at
/// or below its threshold is inaudible and adds (almost) nothing while a bin far
/// above it adds roughly the log of its excess. Because `energy` and `threshold`
/// are both FFT-domain powers their ratio is dimensionless, so no cross-domain
/// calibration is needed. The sum is the standard signal for distributing bits
/// across granules and for the long/short block-switching decision (a sharp rise
/// in perceptual entropy marks a transient). Bins with a non-positive threshold
/// are skipped; the two arrays must match in length.
pub fn perceptual_entropy(energy: &[f64], threshold: &[f64]) -> Result<f64, Error> {
    if energy.len() != threshold.len() {
        return Err(Error::InvalidInput(
            "psychoacoustic energy and threshold arrays must match in length",
        ));
    }
    let mut bits = 0.0_f64;
    for (&e, &thr) in energy.iter().zip(threshold.iter()) {
        if thr <= 0.0 || e <= 0.0 {
            continue;
        }
        let steps = (e / thr).sqrt().round();
        bits += (2.0 * steps + 1.0).log2();
    }
    Ok(bits)
}

/// Splits `total` as evenly as possible into `parts` non-negative integers whose
/// sum is exactly `total`; the first `total % parts` parts get one extra.
pub(crate) fn even_split(total: usize, parts: usize) -> Vec<usize> {
    if parts == 0 {
        return Vec::new();
    }
    let base = total / parts;
    let remainder = total % parts;
    (0..parts)
        .map(|i| if i < remainder { base + 1 } else { base })
        .collect()
}

/// Distributes a total bit budget across granules in proportion to their
/// perceptual entropy, guaranteeing each granule at least `min_bits`.
///
/// Granules that demand more bits (higher [`perceptual_entropy`]) receive a
/// larger share — this is how a perceptual bit reservoir spends its budget where
/// it is audibly needed. Each granule is first given `min_bits`; the remainder is
/// split in proportion to perceptual entropy, with largest-remainder rounding so
/// the targets sum to exactly `total_bits`. If the floors alone exceed the budget
/// the budget is split as evenly as possible instead, and if no granule has any
/// perceptual demand the remainder is shared evenly. Returns one target per
/// granule (empty input → empty result); entropy values must be finite and
/// non-negative.
pub fn distribute_bits_by_perceptual_entropy(
    perceptual_entropy: &[f64],
    total_bits: usize,
    min_bits: usize,
) -> Result<Vec<usize>, Error> {
    let n = perceptual_entropy.len();
    if n == 0 {
        return Ok(Vec::new());
    }
    for &pe in perceptual_entropy {
        if !pe.is_finite() || pe < 0.0 {
            return Err(Error::InvalidInput(
                "perceptual entropy values must be finite and non-negative",
            ));
        }
    }

    // The floors cannot all be honored: spread the whole budget as evenly as we
    // can rather than over-committing.
    if min_bits.saturating_mul(n) >= total_bits {
        return Ok(even_split(total_bits, n));
    }

    let remainder = total_bits - min_bits * n;
    let sum: f64 = perceptual_entropy.iter().sum();
    let mut targets = vec![min_bits; n];

    if sum <= 0.0 {
        // No perceptual demand anywhere: share the remainder evenly.
        for (slot, extra) in targets.iter_mut().zip(even_split(remainder, n)) {
            *slot += extra;
        }
        return Ok(targets);
    }

    // Proportional share with largest-remainder rounding for an exact total.
    let mut assigned = 0usize;
    let mut fractional: Vec<(usize, f64)> = Vec::with_capacity(n);
    for (index, (&pe, slot)) in perceptual_entropy
        .iter()
        .zip(targets.iter_mut())
        .enumerate()
    {
        let exact = remainder as f64 * pe / sum;
        let floor = exact.floor();
        let whole = floor as usize;
        *slot += whole;
        assigned += whole;
        fractional.push((index, exact - floor));
    }

    let mut leftover = remainder - assigned;
    fractional.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    for (index, _) in fractional {
        if leftover == 0 {
            break;
        }
        if let Some(slot) = targets.get_mut(index) {
            *slot += 1;
            leftover -= 1;
        }
    }
    Ok(targets)
}
