use super::*;

/// Number of equal segments a block is split into for transient analysis. MP3
/// short blocks divide the long window into three; a few more segments localizes
/// the onset within the block more sharply.
pub(crate) const TRANSIENT_SEGMENTS: usize = 8;

/// Default attack-ratio threshold above which a block is treated as transient
/// and switched to short blocks to suppress pre-echo.
pub(crate) const TRANSIENT_RATIO_THRESHOLD: f64 = 10.0;

/// Energy floor used when a segment's preceding context is silent, so the attack
/// ratio stays finite instead of dividing by zero.
pub(crate) const TRANSIENT_ENERGY_FLOOR: f64 = 1.0e-12;

/// Returns true if every value in the slice is finite (no NaN or infinity).
pub(crate) fn all_finite_f64(values: &[f64]) -> bool {
    values.iter().all(|v| v.is_finite())
}

/// Returns true if every value in the slice is finite (no NaN or infinity).
pub(crate) fn all_finite_f32(values: &[f32]) -> bool {
    values.iter().all(|v| v.is_finite())
}

/// Splits `pcm` into `segments` equal contiguous parts and returns each part's
/// energy (sum of squares). Each sample maps to exactly one segment.
pub(crate) fn segment_energies(pcm: &[f64], segments: usize) -> Result<Vec<f64>, Error> {
    if segments == 0 {
        return Err(Error::InvalidInput(
            "transient segment count must be non-zero",
        ));
    }
    if pcm.is_empty() {
        return Err(Error::InvalidInput(
            "transient analysis needs a non-empty block",
        ));
    }
    if !all_finite_f64(pcm) {
        return Err(Error::InvalidInput(
            "transient analysis input must be finite",
        ));
    }
    let len = pcm.len();
    let mut energies = vec![0.0_f64; segments];
    for (index, &sample) in pcm.iter().enumerate() {
        // index < len, so (index * segments) / len < segments — always in range.
        let segment = index * segments / len;
        if let Some(slot) = energies.get_mut(segment) {
            *slot += sample * sample;
        }
    }
    Ok(energies)
}

/// Estimates the attack strength of a block as the largest rise in energy from
/// the running mean of the preceding segments to a segment.
///
/// The block is split into `segments` equal parts; the attack ratio is the
/// maximum of `segment_energy / mean(preceding segment energies)`. A steady block
/// yields a ratio near 1, while a sharp onset — the case where short blocks are
/// needed to keep quantization noise from spreading backwards as pre-echo —
/// yields a large ratio. The first segment has no preceding context and is
/// skipped. Returns an error for an empty block or zero segments.
pub fn transient_attack_ratio(pcm: &[f64], segments: usize) -> Result<f64, Error> {
    let energies = segment_energies(pcm, segments)?;
    let mut max_ratio = 1.0_f64;
    let mut preceding_sum = 0.0_f64;
    let mut preceding_count = 0.0_f64;
    for &energy in &energies {
        if preceding_count > 0.0 {
            let mean_preceding = (preceding_sum / preceding_count).max(TRANSIENT_ENERGY_FLOOR);
            let ratio = energy / mean_preceding;
            if ratio > max_ratio {
                max_ratio = ratio;
            }
        }
        preceding_sum += energy;
        preceding_count += 1.0;
    }
    Ok(max_ratio)
}

/// Decides whether a block should switch to short blocks, by comparing its
/// [`transient_attack_ratio`] against [`TRANSIENT_RATIO_THRESHOLD`].
///
/// This is the long/short block-switching trigger: `true` marks a transient
/// (sharp onset) for which short blocks suppress pre-echo. A silent or steady
/// block returns `false`.
pub fn is_transient_block(pcm: &[f64]) -> Result<bool, Error> {
    Ok(transient_attack_ratio(pcm, TRANSIENT_SEGMENTS)? >= TRANSIENT_RATIO_THRESHOLD)
}

/// Side-channel energy fraction below which mid/side coding is chosen over
/// left/right. With strongly correlated channels the side signal is near-silent
/// and codes almost for free, so MS concentrates bits in the mid channel.
pub(crate) const MID_SIDE_ENERGY_FRACTION_THRESHOLD: f64 = 0.2;

/// Applies the energy-preserving mid/side transform to a stereo pair:
/// `mid = (left + right) / √2`, `side = (left − right) / √2`.
///
/// The `1/√2` normalization makes the transform orthonormal, so
/// `mid² + side² = left² + right²` sample-for-sample and the choice of L/R vs
/// M/S never changes the block's total energy. The two channels must be the
/// same length.
pub fn mid_side_transform(left: &[f64], right: &[f64]) -> Result<(Vec<f64>, Vec<f64>), Error> {
    if left.len() != right.len() {
        return Err(Error::InvalidInput(
            "mid/side transform needs equal-length channels",
        ));
    }
    let scale = std::f64::consts::FRAC_1_SQRT_2;
    let mid = left
        .iter()
        .zip(right.iter())
        .map(|(&l, &r)| (l + r) * scale)
        .collect();
    let side = left
        .iter()
        .zip(right.iter())
        .map(|(&l, &r)| (l - r) * scale)
        .collect();
    Ok((mid, side))
}

/// Returns the fraction of stereo energy carried by the side channel,
/// `side / (mid + side)`, in `[0, 1]`.
///
/// A value near 0 means the channels are nearly identical (mono-like) so the
/// side signal is negligible; near 1 means they are anti-correlated. A fully
/// silent pair returns 0. The channels must be the same length.
pub fn side_energy_fraction(left: &[f64], right: &[f64]) -> Result<f64, Error> {
    let (mid, side) = mid_side_transform(left, right)?;
    let mid_energy: f64 = mid.iter().map(|&m| m * m).sum();
    let side_energy: f64 = side.iter().map(|&s| s * s).sum();
    let total = mid_energy + side_energy;
    if total <= 0.0 {
        return Ok(0.0);
    }
    Ok(side_energy / total)
}

/// Decides whether a stereo block should be coded as mid/side rather than
/// left/right, by comparing its [`side_energy_fraction`] against
/// [`MID_SIDE_ENERGY_FRACTION_THRESHOLD`].
///
/// `true` selects mid/side — worthwhile when the channels are correlated enough
/// that the side channel is cheap to code. A silent block returns `true` (MS is
/// harmless and the side channel is empty).
pub fn should_use_mid_side(left: &[f64], right: &[f64]) -> Result<bool, Error> {
    Ok(side_energy_fraction(left, right)? < MID_SIDE_ENERGY_FRACTION_THRESHOLD)
}
