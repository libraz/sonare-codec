use super::*;

#[allow(clippy::too_many_arguments)]
pub fn select_aac_lc_stereo_pcm_frame_step_details_with_offsets_by_bit_cost(
    adts: AdtsConfig,
    left: AacLongBlockConfig,
    right: AacLongBlockConfig,
    pcm: &AudioBuffer,
    start_frame: usize,
    offsets: &[usize],
    candidates: &[f32],
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<AacPcmFrameStepSelection, Error> {
    if candidates.is_empty() {
        return Err(Error::InvalidInput(
            "AAC quantizer step candidate list is empty",
        ));
    }
    let mut selected: Option<AacPcmFrameStepSelection> = None;
    for &step in candidates {
        if !step.is_finite() || step <= 0.0 {
            return Err(Error::InvalidInput(
                "AAC quantizer step must be positive and finite",
            ));
        }
        if let Ok(selection) = evaluate_aac_lc_stereo_pcm_frame_step_with_offsets_by_bit_cost(
            adts,
            left,
            right,
            pcm,
            start_frame,
            offsets,
            step,
            scale_factor_table,
            spectral_tables,
        ) {
            selected = select_better_aac_pcm_frame_step(selected, selection);
        }
    }
    selected.ok_or(Error::UnsupportedFeature("AAC quantizer step search"))
}

#[allow(clippy::too_many_arguments)]
pub fn select_aac_lc_stereo_pcm_frame_step_details_with_offsets_and_max_frame_len_by_bit_cost(
    adts: AdtsConfig,
    left: AacLongBlockConfig,
    right: AacLongBlockConfig,
    pcm: &AudioBuffer,
    start_frame: usize,
    offsets: &[usize],
    candidates: &[f32],
    max_frame_len_bytes: usize,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<AacPcmFrameStepSelection, Error> {
    validate_aac_max_frame_len(max_frame_len_bytes)?;
    if candidates.is_empty() {
        return Err(Error::InvalidInput(
            "AAC quantizer step candidate list is empty",
        ));
    }
    let mut selected: Option<AacPcmFrameStepSelection> = None;
    for &step in candidates {
        if !step.is_finite() || step <= 0.0 {
            return Err(Error::InvalidInput(
                "AAC quantizer step must be positive and finite",
            ));
        }
        if let Ok(selection) = evaluate_aac_lc_stereo_pcm_frame_step_with_offsets_by_bit_cost(
            adts,
            left,
            right,
            pcm,
            start_frame,
            offsets,
            step,
            scale_factor_table,
            spectral_tables,
        ) {
            selected =
                fold_aac_pcm_frame_step_within_budget(selected, selection, max_frame_len_bytes);
        }
    }
    selected.ok_or(Error::UnsupportedFeature("AAC quantizer step search"))
}

#[allow(clippy::too_many_arguments)]
pub fn select_aac_lc_stereo_pcm_frame_step_details_with_offsets_and_scale_factors_by_bit_cost(
    adts: AdtsConfig,
    left: AacScaleFactorChannel<'_>,
    right: AacScaleFactorChannel<'_>,
    pcm: &AudioBuffer,
    start_frame: usize,
    offsets: &[usize],
    candidates: &[f32],
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<AacPcmFrameStepSelection, Error> {
    if candidates.is_empty() {
        return Err(Error::InvalidInput(
            "AAC quantizer step candidate list is empty",
        ));
    }
    let mut selected: Option<AacPcmFrameStepSelection> = None;
    for &step in candidates {
        if !step.is_finite() || step <= 0.0 {
            return Err(Error::InvalidInput(
                "AAC quantizer step must be positive and finite",
            ));
        }
        if let Ok(selection) =
            evaluate_aac_lc_stereo_pcm_frame_step_with_offsets_and_scale_factors_by_bit_cost(
                adts,
                left,
                right,
                pcm,
                start_frame,
                offsets,
                step,
                scale_factor_table,
                spectral_tables,
            )
        {
            selected = select_better_aac_pcm_frame_step(selected, selection);
        }
    }
    selected.ok_or(Error::UnsupportedFeature("AAC quantizer step search"))
}

#[allow(clippy::too_many_arguments)]
pub fn select_aac_lc_stereo_pcm_frame_step_details_with_offsets_and_scale_factors_and_max_frame_len_by_bit_cost(
    adts: AdtsConfig,
    left: AacScaleFactorChannel<'_>,
    right: AacScaleFactorChannel<'_>,
    pcm: &AudioBuffer,
    start_frame: usize,
    offsets: &[usize],
    candidates: &[f32],
    max_frame_len_bytes: usize,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<AacPcmFrameStepSelection, Error> {
    validate_aac_max_frame_len(max_frame_len_bytes)?;
    if candidates.is_empty() {
        return Err(Error::InvalidInput(
            "AAC quantizer step candidate list is empty",
        ));
    }
    let mut selected: Option<AacPcmFrameStepSelection> = None;
    for &step in candidates {
        if !step.is_finite() || step <= 0.0 {
            return Err(Error::InvalidInput(
                "AAC quantizer step must be positive and finite",
            ));
        }
        if let Ok(selection) =
            evaluate_aac_lc_stereo_pcm_frame_step_with_offsets_and_scale_factors_by_bit_cost(
                adts,
                left,
                right,
                pcm,
                start_frame,
                offsets,
                step,
                scale_factor_table,
                spectral_tables,
            )
        {
            selected =
                fold_aac_pcm_frame_step_within_budget(selected, selection, max_frame_len_bytes);
        }
    }
    selected.ok_or(Error::UnsupportedFeature("AAC quantizer step search"))
}

#[allow(clippy::too_many_arguments)]
pub fn select_aac_lc_stereo_pcm_frame_step_details_with_standard_spectral_offsets_and_scale_factors_and_max_frame_len_by_bit_cost(
    adts: AdtsConfig,
    left: AacScaleFactorChannel<'_>,
    right: AacScaleFactorChannel<'_>,
    pcm: &AudioBuffer,
    start_frame: usize,
    offsets: &[usize],
    candidates: &[f32],
    max_frame_len_bytes: usize,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
) -> Result<AacPcmFrameStepSelection, Error> {
    validate_aac_max_frame_len(max_frame_len_bytes)?;
    if candidates.is_empty() {
        return Err(Error::InvalidInput(
            "AAC quantizer step candidate list is empty",
        ));
    }
    let mut selected: Option<AacPcmFrameStepSelection> = None;
    for &step in candidates {
        if !step.is_finite() || step <= 0.0 {
            return Err(Error::InvalidInput(
                "AAC quantizer step must be positive and finite",
            ));
        }
        if let Ok(selection) =
            evaluate_aac_lc_stereo_pcm_frame_step_with_standard_spectral_offsets_and_scale_factors_by_bit_cost(
                adts,
                left,
                right,
                pcm,
                start_frame,
                offsets,
                step,
                scale_factor_table,
            )
        {
            selected =
                fold_aac_pcm_frame_step_within_budget(selected, selection, max_frame_len_bytes);
        }
    }
    selected.ok_or(Error::UnsupportedFeature("AAC quantizer step search"))
}

#[allow(clippy::too_many_arguments)]
pub fn select_aac_lc_stereo_pcm_stream_frame_details_with_standard_spectral_offsets_and_scale_factors_and_max_frame_len_by_bit_cost(
    adts: AdtsConfig,
    left: AacScaleFactorSequence<'_>,
    right: AacScaleFactorSequence<'_>,
    pcm: &AudioBuffer,
    start_frame: usize,
    offsets: &[usize],
    candidates: &[f32],
    max_frame_len_bytes: usize,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
) -> Result<Vec<AacPcmFrameStepSelection>, Error> {
    if adts.channels != 2 || pcm.channels != 2 {
        return Err(Error::InvalidInput(
            "AAC stereo PCM encode requires two-channel ADTS and PCM",
        ));
    }
    validate_aac_max_frame_len(max_frame_len_bytes)?;

    let starts = pcm_frame_starts(pcm, start_frame)?;
    if starts.len() != left.scale_factors_by_frame.len()
        || starts.len() != right.scale_factors_by_frame.len()
    {
        return Err(Error::InvalidInput(
            "AAC scale-factor frame count does not match PCM frame count",
        ));
    }

    starts
        .into_iter()
        .enumerate()
        .map(|(frame_index, start_frame)| {
            select_aac_lc_stereo_pcm_frame_step_details_with_standard_spectral_offsets_and_scale_factors_and_max_frame_len_by_bit_cost(
                adts,
                left.channel_for_frame(frame_index)?,
                right.channel_for_frame(frame_index)?,
                pcm,
                start_frame,
                offsets,
                candidates,
                max_frame_len_bytes,
                scale_factor_table,
            )
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
pub fn select_aac_lc_stereo_pcm_stream_frame_details_with_standard_spectral_offsets_and_scale_factors_and_bitrate_by_bit_cost(
    adts: AdtsConfig,
    left: AacScaleFactorSequence<'_>,
    right: AacScaleFactorSequence<'_>,
    pcm: &AudioBuffer,
    start_frame: usize,
    offsets: &[usize],
    candidates: &[f32],
    target_bitrate_bps: u32,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
) -> Result<Vec<AacPcmFrameStepSelection>, Error> {
    let max_frame_len_bytes =
        aac_lc_adts_max_frame_len_for_bitrate(adts.sample_rate, target_bitrate_bps)?;
    select_aac_lc_stereo_pcm_stream_frame_details_with_standard_spectral_offsets_and_scale_factors_and_max_frame_len_by_bit_cost(
        adts,
        left,
        right,
        pcm,
        start_frame,
        offsets,
        candidates,
        max_frame_len_bytes,
        scale_factor_table,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn select_aac_lc_stereo_pcm_frame_step_details_with_standard_spectral_offsets_and_selected_scale_factors_and_max_frame_len_by_bit_cost(
    adts: AdtsConfig,
    left: AacLongBlockConfig,
    right: AacLongBlockConfig,
    pcm: &AudioBuffer,
    start_frame: usize,
    offsets: &[usize],
    candidates: &[f32],
    max_frame_len_bytes: usize,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
) -> Result<AacPcmFrameStepSelection, Error> {
    select_aac_lc_stereo_pcm_frame_step_details_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_max_frame_len_by_bit_cost(
        adts,
        left,
        right,
        pcm,
        start_frame,
        offsets,
        0,
        candidates,
        max_frame_len_bytes,
        scale_factor_table,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn select_aac_lc_stereo_pcm_frame_step_details_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_max_frame_len_by_bit_cost(
    adts: AdtsConfig,
    left: AacLongBlockConfig,
    right: AacLongBlockConfig,
    pcm: &AudioBuffer,
    start_frame: usize,
    offsets: &[usize],
    scale_factor_magnitude_bias: i16,
    candidates: &[f32],
    max_frame_len_bytes: usize,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
) -> Result<AacPcmFrameStepSelection, Error> {
    validate_aac_max_frame_len(max_frame_len_bytes)?;
    if candidates.is_empty() {
        return Err(Error::InvalidInput(
            "AAC quantizer step candidate list is empty",
        ));
    }
    let mut selected: Option<AacPcmFrameStepSelection> = None;
    for &step in candidates {
        if !step.is_finite() || step <= 0.0 {
            return Err(Error::InvalidInput(
                "AAC quantizer step must be positive and finite",
            ));
        }
        if let Ok(selection) =
            evaluate_aac_lc_stereo_pcm_frame_step_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_by_bit_cost(
                adts,
                left,
                right,
                pcm,
                start_frame,
                offsets,
                scale_factor_magnitude_bias,
                step,
                scale_factor_table,
            )
        {
            selected =
                fold_aac_pcm_frame_step_within_budget(selected, selection, max_frame_len_bytes);
        }
    }
    selected.ok_or(Error::UnsupportedFeature("AAC quantizer step search"))
}

#[allow(clippy::too_many_arguments)]
pub fn select_aac_lc_stereo_pcm_frame_step_details_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_max_quantized_abs_and_max_frame_len_by_bit_cost(
    adts: AdtsConfig,
    left: AacLongBlockConfig,
    right: AacLongBlockConfig,
    pcm: &AudioBuffer,
    start_frame: usize,
    offsets: &[usize],
    scale_factor_magnitude_bias: i16,
    candidates: &[f32],
    max_quantized_abs: u32,
    max_frame_len_bytes: usize,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
) -> Result<AacPcmFrameStepSelection, Error> {
    validate_aac_max_frame_len(max_frame_len_bytes)?;
    if candidates.is_empty() {
        return Err(Error::InvalidInput(
            "AAC quantizer step candidate list is empty",
        ));
    }
    let mut selected: Option<AacPcmFrameStepSelection> = None;
    for &step in candidates {
        if !step.is_finite() || step <= 0.0 {
            return Err(Error::InvalidInput(
                "AAC quantizer step must be positive and finite",
            ));
        }
        if let Ok(selection) =
            evaluate_aac_lc_stereo_pcm_frame_step_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_max_quantized_abs_by_bit_cost(
                adts,
                left,
                right,
                pcm,
                start_frame,
                offsets,
                scale_factor_magnitude_bias,
                step,
                max_quantized_abs,
                scale_factor_table,
            )
        {
            selected =
                fold_aac_pcm_frame_step_within_budget(selected, selection, max_frame_len_bytes);
        }
    }
    selected.ok_or(Error::UnsupportedFeature("AAC quantizer step search"))
}

#[allow(clippy::too_many_arguments)]
pub fn select_aac_lc_stereo_pcm_stream_frame_details_with_standard_spectral_offsets_and_selected_scale_factors_and_max_frame_len_by_bit_cost(
    adts: AdtsConfig,
    left: AacLongBlockConfig,
    right: AacLongBlockConfig,
    pcm: &AudioBuffer,
    start_frame: usize,
    offsets: &[usize],
    candidates: &[f32],
    max_frame_len_bytes: usize,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
) -> Result<Vec<AacPcmFrameStepSelection>, Error> {
    select_aac_lc_stereo_pcm_stream_frame_details_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_max_frame_len_by_bit_cost(
        adts,
        left,
        right,
        pcm,
        start_frame,
        offsets,
        0,
        candidates,
        max_frame_len_bytes,
        scale_factor_table,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn select_aac_lc_stereo_pcm_stream_frame_details_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_max_frame_len_by_bit_cost(
    adts: AdtsConfig,
    left: AacLongBlockConfig,
    right: AacLongBlockConfig,
    pcm: &AudioBuffer,
    start_frame: usize,
    offsets: &[usize],
    scale_factor_magnitude_bias: i16,
    candidates: &[f32],
    max_frame_len_bytes: usize,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
) -> Result<Vec<AacPcmFrameStepSelection>, Error> {
    if adts.channels != 2 || pcm.channels != 2 {
        return Err(Error::InvalidInput(
            "AAC stereo PCM encode requires two-channel ADTS and PCM",
        ));
    }
    validate_aac_max_frame_len(max_frame_len_bytes)?;

    pcm_frame_starts(pcm, start_frame)?
        .into_iter()
        .map(|start_frame| {
            select_aac_lc_stereo_pcm_frame_step_details_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_max_frame_len_by_bit_cost(
                adts,
                left,
                right,
                pcm,
                start_frame,
                offsets,
                scale_factor_magnitude_bias,
                candidates,
                max_frame_len_bytes,
                scale_factor_table,
            )
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
pub fn select_aac_lc_stereo_pcm_stream_frame_details_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_max_quantized_abs_and_max_frame_len_by_bit_cost(
    adts: AdtsConfig,
    left: AacLongBlockConfig,
    right: AacLongBlockConfig,
    pcm: &AudioBuffer,
    start_frame: usize,
    offsets: &[usize],
    scale_factor_magnitude_bias: i16,
    candidates: &[f32],
    max_quantized_abs: u32,
    max_frame_len_bytes: usize,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
) -> Result<Vec<AacPcmFrameStepSelection>, Error> {
    if adts.channels != 2 || pcm.channels != 2 {
        return Err(Error::InvalidInput(
            "AAC stereo PCM encode requires two-channel ADTS and PCM",
        ));
    }
    validate_aac_max_frame_len(max_frame_len_bytes)?;

    pcm_frame_starts(pcm, start_frame)?
        .into_iter()
        .map(|start_frame| {
            select_aac_lc_stereo_pcm_frame_step_details_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_max_quantized_abs_and_max_frame_len_by_bit_cost(
                adts,
                left,
                right,
                pcm,
                start_frame,
                offsets,
                scale_factor_magnitude_bias,
                candidates,
                max_quantized_abs,
                max_frame_len_bytes,
                scale_factor_table,
            )
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
pub fn select_aac_lc_stereo_pcm_stream_frame_details_with_standard_spectral_offsets_and_selected_scale_factors_and_bitrate_by_bit_cost(
    adts: AdtsConfig,
    left: AacLongBlockConfig,
    right: AacLongBlockConfig,
    pcm: &AudioBuffer,
    start_frame: usize,
    offsets: &[usize],
    candidates: &[f32],
    target_bitrate_bps: u32,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
) -> Result<Vec<AacPcmFrameStepSelection>, Error> {
    select_aac_lc_stereo_pcm_stream_frame_details_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_bitrate_by_bit_cost(
        adts,
        left,
        right,
        pcm,
        start_frame,
        offsets,
        0,
        candidates,
        target_bitrate_bps,
        scale_factor_table,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn select_aac_lc_stereo_pcm_stream_frame_details_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_bitrate_by_bit_cost(
    adts: AdtsConfig,
    left: AacLongBlockConfig,
    right: AacLongBlockConfig,
    pcm: &AudioBuffer,
    start_frame: usize,
    offsets: &[usize],
    scale_factor_magnitude_bias: i16,
    candidates: &[f32],
    target_bitrate_bps: u32,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
) -> Result<Vec<AacPcmFrameStepSelection>, Error> {
    let max_frame_len_bytes =
        aac_lc_adts_max_frame_len_for_bitrate(adts.sample_rate, target_bitrate_bps)?;
    select_aac_lc_stereo_pcm_stream_frame_details_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_max_frame_len_by_bit_cost(
        adts,
        left,
        right,
        pcm,
        start_frame,
        offsets,
        scale_factor_magnitude_bias,
        candidates,
        max_frame_len_bytes,
        scale_factor_table,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn select_aac_lc_stereo_pcm_stream_frame_details_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_max_quantized_abs_and_bitrate_by_bit_cost(
    adts: AdtsConfig,
    left: AacLongBlockConfig,
    right: AacLongBlockConfig,
    pcm: &AudioBuffer,
    start_frame: usize,
    offsets: &[usize],
    scale_factor_magnitude_bias: i16,
    candidates: &[f32],
    max_quantized_abs: u32,
    target_bitrate_bps: u32,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
) -> Result<Vec<AacPcmFrameStepSelection>, Error> {
    let max_frame_len_bytes =
        aac_lc_adts_max_frame_len_for_bitrate(adts.sample_rate, target_bitrate_bps)?;
    select_aac_lc_stereo_pcm_stream_frame_details_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_max_quantized_abs_and_max_frame_len_by_bit_cost(
        adts,
        left,
        right,
        pcm,
        start_frame,
        offsets,
        scale_factor_magnitude_bias,
        candidates,
        max_quantized_abs,
        max_frame_len_bytes,
        scale_factor_table,
    )
}

/// Selects the finest stereo AAC-LC quantizer step that the current tables can pack.
pub fn select_aac_lc_stereo_pcm_frame_step_by_bit_cost(
    adts: AdtsConfig,
    left: AacLongBlockConfig,
    right: AacLongBlockConfig,
    pcm: &AudioBuffer,
    search: AacPcmStepSearchConfig<'_>,
) -> Result<f32, Error> {
    Ok(
        select_aac_lc_stereo_pcm_frame_step_details_by_bit_cost(adts, left, right, pcm, search)?
            .step,
    )
}

/// Selects the finest stereo AAC-LC quantizer step within a caller-provided ADTS frame budget.
pub fn select_aac_lc_stereo_pcm_frame_step_with_max_frame_len_by_bit_cost(
    adts: AdtsConfig,
    left: AacLongBlockConfig,
    right: AacLongBlockConfig,
    pcm: &AudioBuffer,
    search: AacPcmStepSearchConfig<'_>,
    max_frame_len_bytes: usize,
) -> Result<f32, Error> {
    Ok(
        select_aac_lc_stereo_pcm_frame_step_details_with_max_frame_len_by_bit_cost(
            adts,
            left,
            right,
            pcm,
            search,
            max_frame_len_bytes,
        )?
        .step,
    )
}

/// Selects the finest stereo AAC-LC quantizer step and reports its ADTS frame size.
pub fn select_aac_lc_stereo_pcm_frame_step_details_by_bit_cost(
    adts: AdtsConfig,
    left: AacLongBlockConfig,
    right: AacLongBlockConfig,
    pcm: &AudioBuffer,
    search: AacPcmStepSearchConfig<'_>,
) -> Result<AacPcmFrameStepSelection, Error> {
    if search.candidates.is_empty() {
        return Err(Error::InvalidInput(
            "AAC quantizer step candidate list is empty",
        ));
    }
    let mut selected: Option<AacPcmFrameStepSelection> = None;
    for &step in search.candidates {
        if !step.is_finite() || step <= 0.0 {
            return Err(Error::InvalidInput(
                "AAC quantizer step must be positive and finite",
            ));
        }
        if let Ok(selection) =
            evaluate_aac_lc_stereo_pcm_frame_step_by_bit_cost(adts, left, right, pcm, &search, step)
        {
            selected = match selected {
                Some(previous)
                    if selection.step > previous.step
                        || (selection.step == previous.step
                            && selection.frame_len <= previous.frame_len) =>
                {
                    Some(previous)
                }
                _ => Some(selection),
            };
        }
    }
    selected.ok_or(Error::UnsupportedFeature("AAC quantizer step search"))
}

/// Selects the finest stereo AAC-LC quantizer step and reports its ADTS frame size
/// relative to a caller-provided frame budget.
pub fn select_aac_lc_stereo_pcm_frame_step_details_with_max_frame_len_by_bit_cost(
    adts: AdtsConfig,
    left: AacLongBlockConfig,
    right: AacLongBlockConfig,
    pcm: &AudioBuffer,
    search: AacPcmStepSearchConfig<'_>,
    max_frame_len_bytes: usize,
) -> Result<AacPcmFrameStepSelection, Error> {
    validate_aac_max_frame_len(max_frame_len_bytes)?;
    if search.candidates.is_empty() {
        return Err(Error::InvalidInput(
            "AAC quantizer step candidate list is empty",
        ));
    }
    let mut selected: Option<AacPcmFrameStepSelection> = None;
    for &step in search.candidates {
        if !step.is_finite() || step <= 0.0 {
            return Err(Error::InvalidInput(
                "AAC quantizer step must be positive and finite",
            ));
        }
        if let Ok(selection) =
            evaluate_aac_lc_stereo_pcm_frame_step_by_bit_cost(adts, left, right, pcm, &search, step)
        {
            selected =
                fold_aac_pcm_frame_step_within_budget(selected, selection, max_frame_len_bytes);
        }
    }
    selected.ok_or(Error::UnsupportedFeature("AAC quantizer step search"))
}
