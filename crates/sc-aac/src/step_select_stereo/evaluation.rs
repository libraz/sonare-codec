use super::*;

pub(crate) fn validate_aac_max_frame_len(max_frame_len_bytes: usize) -> Result<(), Error> {
    if max_frame_len_bytes == 0 {
        return Err(Error::InvalidInput(
            "AAC max frame length must be greater than zero",
        ));
    }
    Ok(())
}

pub(crate) fn limit_aac_pcm_frame_step_selection(
    mut selection: AacPcmFrameStepSelection,
    max_frame_len_bytes: usize,
) -> Option<AacPcmFrameStepSelection> {
    if selection.frame_len > max_frame_len_bytes {
        return None;
    }
    selection.frame_capacity_bytes = max_frame_len_bytes;
    Some(selection)
}

pub(crate) fn max_quantized_spectrum_abs(quantized: &[i32]) -> u32 {
    quantized
        .iter()
        .map(|sample| sample.unsigned_abs())
        .max()
        .unwrap_or(0)
}

pub(crate) fn select_better_aac_pcm_frame_step(
    selected: Option<AacPcmFrameStepSelection>,
    selection: AacPcmFrameStepSelection,
) -> Option<AacPcmFrameStepSelection> {
    match selected {
        Some(previous)
            if selection.step > previous.step
                || (selection.step == previous.step
                    && selection.frame_len <= previous.frame_len) =>
        {
            Some(previous)
        }
        _ => Some(selection),
    }
}

/// Folds one evaluated candidate into a per-frame step search that degrades
/// gracefully when no quantizer step fits the requested frame budget.
///
/// A budget-respecting candidate always supersedes an over-budget one, and
/// among fitting candidates the existing [`select_better_aac_pcm_frame_step`]
/// ranking is preserved exactly. When every candidate exceeds the budget (an
/// extremely low target bitrate), the smallest over-budget frame is kept as a
/// best-effort result instead of failing the whole encode: the caller then
/// emits a valid — if larger than requested — frame rather than an error.
///
/// The accumulator distinguishes a fitting choice from a best-effort fallback
/// by comparing `frame_len` against the budget, so no second accumulator is
/// needed. Over-budget fallbacks have their declared capacity clamped to the
/// requested budget, matching [`limit_aac_pcm_frame_step_selection`].
pub(crate) fn fold_aac_pcm_frame_step_within_budget(
    selected: Option<AacPcmFrameStepSelection>,
    candidate: AacPcmFrameStepSelection,
    max_frame_len_bytes: usize,
) -> Option<AacPcmFrameStepSelection> {
    match limit_aac_pcm_frame_step_selection(candidate, max_frame_len_bytes) {
        Some(fitting) => {
            let prior_fitting = selected.filter(|s| s.frame_len <= max_frame_len_bytes);
            select_better_aac_pcm_frame_step(prior_fitting, fitting)
        }
        None => match selected {
            Some(previous) if previous.frame_len <= max_frame_len_bytes => Some(previous),
            Some(previous) if previous.frame_len <= candidate.frame_len => Some(previous),
            _ => {
                let mut best_effort = candidate;
                best_effort.frame_capacity_bytes = max_frame_len_bytes;
                Some(best_effort)
            }
        },
    }
}

pub(crate) fn evaluate_aac_lc_mono_pcm_frame_step_by_bit_cost(
    adts: AdtsConfig,
    channel: AacLongBlockConfig,
    pcm: &AudioBuffer,
    search: &AacPcmStepSearchConfig<'_>,
    step: f32,
) -> Result<AacPcmFrameStepSelection, Error> {
    let frame = encode_pcm_mono_long_block_adts_with_selected_scale_factors_by_bit_cost(
        adts,
        channel,
        pcm,
        AacPcmLongBlockConfig::new(search.start_frame, step, search.band_width),
        search.scale_factor_table,
        search.spectral_tables,
    )?;
    Ok(AacPcmFrameStepSelection {
        step,
        frame_len: frame.len(),
        frame_capacity_bytes: AAC_ADTS_MAX_FRAME_LEN,
    })
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn evaluate_aac_lc_mono_pcm_frame_step_with_offsets_by_bit_cost(
    adts: AdtsConfig,
    channel: AacLongBlockConfig,
    pcm: &AudioBuffer,
    start_frame: usize,
    offsets: &[usize],
    step: f32,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<AacPcmFrameStepSelection, Error> {
    let frame =
        encode_pcm_mono_long_block_adts_with_offsets_and_selected_scale_factors_by_bit_cost(
            adts,
            channel,
            pcm,
            start_frame,
            step,
            offsets,
            scale_factor_table,
            spectral_tables,
        )?;
    Ok(AacPcmFrameStepSelection {
        step,
        frame_len: frame.len(),
        frame_capacity_bytes: AAC_ADTS_MAX_FRAME_LEN,
    })
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn evaluate_aac_lc_mono_pcm_frame_step_with_offsets_and_scale_factors_by_bit_cost(
    adts: AdtsConfig,
    channel: AacScaleFactorChannel<'_>,
    pcm: &AudioBuffer,
    start_frame: usize,
    offsets: &[usize],
    step: f32,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<AacPcmFrameStepSelection, Error> {
    let frame = encode_pcm_mono_long_block_adts_with_offsets_and_scale_factors_by_bit_cost(
        adts,
        channel,
        pcm,
        start_frame,
        step,
        offsets,
        scale_factor_table,
        spectral_tables,
    )?;
    Ok(AacPcmFrameStepSelection {
        step,
        frame_len: frame.len(),
        frame_capacity_bytes: AAC_ADTS_MAX_FRAME_LEN,
    })
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn evaluate_aac_lc_mono_pcm_frame_step_with_standard_spectral_offsets_and_scale_factors_by_bit_cost(
    adts: AdtsConfig,
    channel: AacScaleFactorChannel<'_>,
    pcm: &AudioBuffer,
    start_frame: usize,
    offsets: &[usize],
    step: f32,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
) -> Result<AacPcmFrameStepSelection, Error> {
    let frame =
        encode_pcm_mono_long_block_adts_with_standard_spectral_offsets_and_scale_factors_by_bit_cost(
            adts,
            channel,
            pcm,
            start_frame,
            step,
            offsets,
            scale_factor_table,
        )?;
    Ok(AacPcmFrameStepSelection {
        step,
        frame_len: frame.len(),
        frame_capacity_bytes: AAC_ADTS_MAX_FRAME_LEN,
    })
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn evaluate_aac_lc_mono_pcm_frame_step_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_by_bit_cost(
    adts: AdtsConfig,
    channel: AacLongBlockConfig,
    pcm: &AudioBuffer,
    start_frame: usize,
    offsets: &[usize],
    scale_factor_magnitude_bias: i16,
    step: f32,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
) -> Result<AacPcmFrameStepSelection, Error> {
    let frame =
        encode_pcm_mono_long_block_adts_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_by_bit_cost(
            adts,
            channel,
            pcm,
            start_frame,
            step,
            offsets,
            scale_factor_magnitude_bias,
            scale_factor_table,
        )?;
    Ok(AacPcmFrameStepSelection {
        step,
        frame_len: frame.len(),
        frame_capacity_bytes: AAC_ADTS_MAX_FRAME_LEN,
    })
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn evaluate_aac_lc_mono_pcm_frame_step_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_max_quantized_abs_by_bit_cost(
    adts: AdtsConfig,
    channel: AacLongBlockConfig,
    pcm: &AudioBuffer,
    start_frame: usize,
    offsets: &[usize],
    scale_factor_magnitude_bias: i16,
    step: f32,
    max_quantized_abs: u32,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
) -> Result<AacPcmFrameStepSelection, Error> {
    let quantized = quantize_pcm_long_block(pcm, 0, start_frame, step)?;
    if max_quantized_spectrum_abs(&quantized) > max_quantized_abs {
        return Err(Error::UnsupportedFeature("AAC quantized magnitude limit"));
    }
    let frame =
        encode_quantized_mono_adts_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_by_bit_cost(
            adts,
            channel,
            &quantized,
            offsets,
            scale_factor_magnitude_bias,
            scale_factor_table,
        )?;
    Ok(AacPcmFrameStepSelection {
        step,
        frame_len: frame.len(),
        frame_capacity_bytes: AAC_ADTS_MAX_FRAME_LEN,
    })
}

pub(crate) fn evaluate_aac_lc_stereo_pcm_frame_step_by_bit_cost(
    adts: AdtsConfig,
    left: AacLongBlockConfig,
    right: AacLongBlockConfig,
    pcm: &AudioBuffer,
    search: &AacPcmStepSearchConfig<'_>,
    step: f32,
) -> Result<AacPcmFrameStepSelection, Error> {
    let frame = encode_pcm_stereo_long_block_adts_with_selected_scale_factors_by_bit_cost(
        adts,
        left,
        right,
        pcm,
        AacPcmLongBlockConfig::new(search.start_frame, step, search.band_width),
        search.scale_factor_table,
        search.spectral_tables,
    )?;
    Ok(AacPcmFrameStepSelection {
        step,
        frame_len: frame.len(),
        frame_capacity_bytes: AAC_ADTS_MAX_FRAME_LEN,
    })
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn evaluate_aac_lc_stereo_pcm_frame_step_with_offsets_by_bit_cost(
    adts: AdtsConfig,
    left: AacLongBlockConfig,
    right: AacLongBlockConfig,
    pcm: &AudioBuffer,
    start_frame: usize,
    offsets: &[usize],
    step: f32,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<AacPcmFrameStepSelection, Error> {
    let frame =
        encode_pcm_stereo_long_block_adts_with_offsets_and_selected_scale_factors_by_bit_cost(
            adts,
            left,
            right,
            pcm,
            start_frame,
            step,
            offsets,
            scale_factor_table,
            spectral_tables,
        )?;
    Ok(AacPcmFrameStepSelection {
        step,
        frame_len: frame.len(),
        frame_capacity_bytes: AAC_ADTS_MAX_FRAME_LEN,
    })
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn evaluate_aac_lc_stereo_pcm_frame_step_with_offsets_and_scale_factors_by_bit_cost(
    adts: AdtsConfig,
    left: AacScaleFactorChannel<'_>,
    right: AacScaleFactorChannel<'_>,
    pcm: &AudioBuffer,
    start_frame: usize,
    offsets: &[usize],
    step: f32,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<AacPcmFrameStepSelection, Error> {
    let frame = encode_pcm_stereo_long_block_adts_with_offsets_and_scale_factors_by_bit_cost(
        adts,
        left,
        right,
        pcm,
        start_frame,
        step,
        offsets,
        scale_factor_table,
        spectral_tables,
    )?;
    Ok(AacPcmFrameStepSelection {
        step,
        frame_len: frame.len(),
        frame_capacity_bytes: AAC_ADTS_MAX_FRAME_LEN,
    })
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn evaluate_aac_lc_stereo_pcm_frame_step_with_standard_spectral_offsets_and_scale_factors_by_bit_cost(
    adts: AdtsConfig,
    left: AacScaleFactorChannel<'_>,
    right: AacScaleFactorChannel<'_>,
    pcm: &AudioBuffer,
    start_frame: usize,
    offsets: &[usize],
    step: f32,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
) -> Result<AacPcmFrameStepSelection, Error> {
    let frame =
        encode_pcm_stereo_long_block_adts_with_standard_spectral_offsets_and_scale_factors_by_bit_cost(
            adts,
            left,
            right,
            pcm,
            start_frame,
            step,
            offsets,
            scale_factor_table,
        )?;
    Ok(AacPcmFrameStepSelection {
        step,
        frame_len: frame.len(),
        frame_capacity_bytes: AAC_ADTS_MAX_FRAME_LEN,
    })
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn evaluate_aac_lc_stereo_pcm_frame_step_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_by_bit_cost(
    adts: AdtsConfig,
    left: AacLongBlockConfig,
    right: AacLongBlockConfig,
    pcm: &AudioBuffer,
    start_frame: usize,
    offsets: &[usize],
    scale_factor_magnitude_bias: i16,
    step: f32,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
) -> Result<AacPcmFrameStepSelection, Error> {
    let frame =
        encode_pcm_stereo_long_block_adts_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_by_bit_cost(
            adts,
            left,
            right,
            pcm,
            start_frame,
            step,
            offsets,
            scale_factor_magnitude_bias,
            scale_factor_table,
        )?;
    Ok(AacPcmFrameStepSelection {
        step,
        frame_len: frame.len(),
        frame_capacity_bytes: AAC_ADTS_MAX_FRAME_LEN,
    })
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn evaluate_aac_lc_stereo_pcm_frame_step_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_max_quantized_abs_by_bit_cost(
    adts: AdtsConfig,
    left: AacLongBlockConfig,
    right: AacLongBlockConfig,
    pcm: &AudioBuffer,
    start_frame: usize,
    offsets: &[usize],
    scale_factor_magnitude_bias: i16,
    step: f32,
    max_quantized_abs: u32,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
) -> Result<AacPcmFrameStepSelection, Error> {
    let left_quantized = quantize_pcm_long_block(pcm, 0, start_frame, step)?;
    let right_quantized = quantize_pcm_long_block(pcm, 1, start_frame, step)?;
    if max_quantized_spectrum_abs(&left_quantized).max(max_quantized_spectrum_abs(&right_quantized))
        > max_quantized_abs
    {
        return Err(Error::UnsupportedFeature("AAC quantized magnitude limit"));
    }
    let frame =
        encode_quantized_stereo_adts_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_by_bit_cost(
            adts,
            AacQuantizedSpectrum::new(left, &left_quantized),
            AacQuantizedSpectrum::new(right, &right_quantized),
            offsets,
            scale_factor_magnitude_bias,
            scale_factor_table,
        )?;
    Ok(AacPcmFrameStepSelection {
        step,
        frame_len: frame.len(),
        frame_capacity_bytes: AAC_ADTS_MAX_FRAME_LEN,
    })
}
