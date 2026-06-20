use super::*;

#[pyfunction]
pub(crate) fn encode_aac_standard_mono_offsets_with_step(
    sample_rate: u32,
    samples: Vec<f32>,
    step: f32,
    global_gain: u8,
) -> PyResult<Vec<u8>> {
    let pcm = pcm_from_samples(sample_rate, 1, samples)?;
    let offsets = sonare_codec_rs::aac_lc_long_window_scale_factor_band_offsets(sample_rate)
        .ok_or_else(|| {
            pyo3::exceptions::PyValueError::new_err("unsupported AAC-LC long-window sample rate")
        })?;
    let channel_config =
        sonare_codec_rs::AacLongBlockConfig::new(global_gain, aac_offsets_max_sfb(offsets)?);
    let scale_factors_by_frame = constant_aac_scale_factors_by_frame(
        &pcm,
        usize::from(channel_config.global_gain),
        offsets.len() - 1,
    );
    let scale_factor_refs = scale_factors_by_frame
        .iter()
        .map(Vec::as_slice)
        .collect::<Vec<_>>();
    let scale_factor_table = sonare_codec_rs::aac_scale_factor_delta_table();

    sonare_codec_rs::encode_pcm_mono_long_block_adts_stream_with_standard_spectral_offsets_and_scale_factors_by_bit_cost(
        sonare_codec_rs::AdtsConfig::aac_lc(sample_rate, 1),
        sonare_codec_rs::AacScaleFactorSequence::new(channel_config, &scale_factor_refs),
        &pcm,
        0,
        step,
        offsets,
        &scale_factor_table,
    )
    .map_err(to_py_value_error)
}

#[pyfunction]
pub(crate) fn encode_aac_standard_mono_offsets_with_bitrate(
    sample_rate: u32,
    samples: Vec<f32>,
    target_bitrate_bps: u32,
    global_gain: u8,
) -> PyResult<Vec<u8>> {
    let pcm = pcm_from_samples(sample_rate, 1, samples)?;
    sonare_codec_rs::encode_aac_adts_with_standard_spectral_offsets_and_bitrate(
        &pcm,
        target_bitrate_bps,
        global_gain,
    )
    .map_err(to_py_value_error)
}

#[pyfunction]
pub(crate) fn aac_standard_mono_offsets_bitrate_frame_details(
    sample_rate: u32,
    samples: Vec<f32>,
    target_bitrate_bps: u32,
    global_gain: u8,
) -> PyResult<Vec<f64>> {
    let pcm = pcm_from_samples(sample_rate, 1, samples)?;
    let offsets = sonare_codec_rs::aac_lc_long_window_scale_factor_band_offsets(sample_rate)
        .ok_or_else(|| {
            pyo3::exceptions::PyValueError::new_err("unsupported AAC-LC long-window sample rate")
        })?;
    let channel_config =
        sonare_codec_rs::AacLongBlockConfig::new(global_gain, aac_offsets_max_sfb(offsets)?);
    let scale_factors_by_frame = constant_aac_scale_factors_by_frame(
        &pcm,
        usize::from(channel_config.global_gain),
        offsets.len() - 1,
    );
    let scale_factor_refs = scale_factors_by_frame
        .iter()
        .map(Vec::as_slice)
        .collect::<Vec<_>>();
    let scale_factor_table = sonare_codec_rs::aac_scale_factor_delta_table();

    let details = sonare_codec_rs::select_aac_lc_mono_pcm_stream_frame_details_with_standard_spectral_offsets_and_scale_factors_and_bitrate_by_bit_cost(
        sonare_codec_rs::AdtsConfig::aac_lc(sample_rate, 1),
        sonare_codec_rs::AacScaleFactorSequence::new(channel_config, &scale_factor_refs),
        &pcm,
        0,
        offsets,
        sonare_codec_rs::AAC_LC_PCM_STEP_CANDIDATES,
        target_bitrate_bps,
        &scale_factor_table,
    )
    .map_err(to_py_value_error)?;

    Ok(details
        .iter()
        .enumerate()
        .flat_map(|(frame_index, detail)| {
            [
                frame_index as f64,
                f64::from(detail.step),
                detail.frame_len as f64,
                detail.frame_capacity_bytes as f64,
            ]
        })
        .collect())
}

#[pyfunction]
pub(crate) fn encode_aac_standard_stereo_offsets_with_step(
    sample_rate: u32,
    samples: Vec<f32>,
    step: f32,
    global_gain: u8,
) -> PyResult<Vec<u8>> {
    let pcm = pcm_from_samples(sample_rate, 2, samples)?;
    let offsets = sonare_codec_rs::aac_lc_long_window_scale_factor_band_offsets(sample_rate)
        .ok_or_else(|| {
            pyo3::exceptions::PyValueError::new_err("unsupported AAC-LC long-window sample rate")
        })?;
    let channel_config =
        sonare_codec_rs::AacLongBlockConfig::new(global_gain, aac_offsets_max_sfb(offsets)?);
    let scale_factors_by_frame = constant_aac_scale_factors_by_frame(
        &pcm,
        usize::from(channel_config.global_gain),
        offsets.len() - 1,
    );
    let scale_factor_refs = scale_factors_by_frame
        .iter()
        .map(Vec::as_slice)
        .collect::<Vec<_>>();
    let scale_factor_table = sonare_codec_rs::aac_scale_factor_delta_table();

    sonare_codec_rs::encode_pcm_stereo_long_block_adts_stream_with_standard_spectral_offsets_and_scale_factors_by_bit_cost(
        sonare_codec_rs::AdtsConfig::aac_lc(sample_rate, 2),
        sonare_codec_rs::AacScaleFactorSequence::new(channel_config, &scale_factor_refs),
        sonare_codec_rs::AacScaleFactorSequence::new(channel_config, &scale_factor_refs),
        &pcm,
        0,
        step,
        offsets,
        &scale_factor_table,
    )
    .map_err(to_py_value_error)
}

#[pyfunction]
pub(crate) fn encode_aac_standard_stereo_offsets_with_bitrate(
    sample_rate: u32,
    samples: Vec<f32>,
    target_bitrate_bps: u32,
    global_gain: u8,
) -> PyResult<Vec<u8>> {
    let pcm = pcm_from_samples(sample_rate, 2, samples)?;
    sonare_codec_rs::encode_aac_adts_with_standard_spectral_offsets_and_bitrate(
        &pcm,
        target_bitrate_bps,
        global_gain,
    )
    .map_err(to_py_value_error)
}

#[pyfunction]
pub(crate) fn aac_standard_stereo_offsets_bitrate_frame_details(
    sample_rate: u32,
    samples: Vec<f32>,
    target_bitrate_bps: u32,
    global_gain: u8,
) -> PyResult<Vec<f64>> {
    let pcm = pcm_from_samples(sample_rate, 2, samples)?;
    let offsets = sonare_codec_rs::aac_lc_long_window_scale_factor_band_offsets(sample_rate)
        .ok_or_else(|| {
            pyo3::exceptions::PyValueError::new_err("unsupported AAC-LC long-window sample rate")
        })?;
    let channel_config =
        sonare_codec_rs::AacLongBlockConfig::new(global_gain, aac_offsets_max_sfb(offsets)?);
    let scale_factors_by_frame = constant_aac_scale_factors_by_frame(
        &pcm,
        usize::from(channel_config.global_gain),
        offsets.len() - 1,
    );
    let scale_factor_refs = scale_factors_by_frame
        .iter()
        .map(Vec::as_slice)
        .collect::<Vec<_>>();
    let scale_factor_table = sonare_codec_rs::aac_scale_factor_delta_table();

    let details = sonare_codec_rs::select_aac_lc_stereo_pcm_stream_frame_details_with_standard_spectral_offsets_and_scale_factors_and_bitrate_by_bit_cost(
        sonare_codec_rs::AdtsConfig::aac_lc(sample_rate, 2),
        sonare_codec_rs::AacScaleFactorSequence::new(channel_config, &scale_factor_refs),
        sonare_codec_rs::AacScaleFactorSequence::new(channel_config, &scale_factor_refs),
        &pcm,
        0,
        offsets,
        sonare_codec_rs::AAC_LC_PCM_STEP_CANDIDATES,
        target_bitrate_bps,
        &scale_factor_table,
    )
    .map_err(to_py_value_error)?;

    Ok(details
        .iter()
        .enumerate()
        .flat_map(|(frame_index, detail)| {
            [
                frame_index as f64,
                f64::from(detail.step),
                detail.frame_len as f64,
                detail.frame_capacity_bytes as f64,
            ]
        })
        .collect())
}

#[pyfunction]
pub(crate) fn aac_standard_selected_scale_factor_frame_details_with_magnitude_bias_and_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    target_bitrate_bps: u32,
    global_gain: u8,
    scale_factor_magnitude_bias: i16,
) -> PyResult<Vec<f64>> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    let details =
        sonare_codec_rs::aac_standard_selected_scale_factor_frame_details_with_magnitude_bias_and_bitrate(
            &pcm,
            target_bitrate_bps,
            global_gain,
            scale_factor_magnitude_bias,
        )
        .map_err(to_py_value_error)?;

    Ok(details
        .iter()
        .enumerate()
        .flat_map(|(frame_index, detail)| {
            [
                frame_index as f64,
                f64::from(detail.step),
                detail.frame_len as f64,
                detail.frame_capacity_bytes as f64,
            ]
        })
        .collect())
}

#[pyfunction]
pub(crate) fn aac_recommended_standard_selected_scale_factor_frame_details_with_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    target_bitrate_bps: u32,
) -> PyResult<Vec<f64>> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    let details =
        sonare_codec_rs::aac_recommended_standard_selected_scale_factor_frame_details_with_bitrate(
            &pcm,
            target_bitrate_bps,
        )
        .map_err(to_py_value_error)?;

    Ok(details
        .iter()
        .enumerate()
        .flat_map(|(frame_index, detail)| {
            [
                frame_index as f64,
                f64::from(detail.step),
                detail.frame_len as f64,
                detail.frame_capacity_bytes as f64,
            ]
        })
        .collect())
}

#[pyfunction]
pub(crate) fn aac_standard_selected_scale_factor_frame_details_with_magnitude_bias_max_quantized_abs_and_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    target_bitrate_bps: u32,
    global_gain: u8,
    scale_factor_magnitude_bias: i16,
    max_quantized_abs: u32,
) -> PyResult<Vec<f64>> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    let details =
        sonare_codec_rs::aac_standard_selected_scale_factor_frame_details_with_magnitude_bias_max_quantized_abs_and_bitrate(
            &pcm,
            target_bitrate_bps,
            global_gain,
            scale_factor_magnitude_bias,
            max_quantized_abs,
        )
        .map_err(to_py_value_error)?;

    Ok(details
        .iter()
        .enumerate()
        .flat_map(|(frame_index, detail)| {
            [
                frame_index as f64,
                f64::from(detail.step),
                detail.frame_len as f64,
                detail.frame_capacity_bytes as f64,
            ]
        })
        .collect())
}

#[pyfunction]
pub(crate) fn aac_recommended_standard_selected_scale_factor_frame_details_with_max_quantized_abs_and_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    target_bitrate_bps: u32,
    max_quantized_abs: u32,
) -> PyResult<Vec<f64>> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    let details =
        sonare_codec_rs::aac_recommended_standard_selected_scale_factor_frame_details_with_max_quantized_abs_and_bitrate(
            &pcm,
            target_bitrate_bps,
            max_quantized_abs,
        )
        .map_err(to_py_value_error)?;

    Ok(details
        .iter()
        .enumerate()
        .flat_map(|(frame_index, detail)| {
            [
                frame_index as f64,
                f64::from(detail.step),
                detail.frame_len as f64,
                detail.frame_capacity_bytes as f64,
            ]
        })
        .collect())
}

#[pyfunction]
pub(crate) fn aac_balanced_standard_selected_scale_factor_frame_details_with_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    target_bitrate_bps: u32,
) -> PyResult<Vec<f64>> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    let details =
        sonare_codec_rs::aac_balanced_standard_selected_scale_factor_frame_details_with_bitrate(
            &pcm,
            target_bitrate_bps,
        )
        .map_err(to_py_value_error)?;

    Ok(details
        .iter()
        .enumerate()
        .flat_map(|(frame_index, detail)| {
            [
                frame_index as f64,
                f64::from(detail.step),
                detail.frame_len as f64,
                detail.frame_capacity_bytes as f64,
            ]
        })
        .collect())
}

#[pyfunction]
pub(crate) fn aac_standard_selected_scale_factor_profile_with_magnitude_bias_and_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    target_bitrate_bps: u32,
    global_gain: u8,
    scale_factor_magnitude_bias: i16,
) -> PyResult<Vec<f64>> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    let details =
        sonare_codec_rs::aac_standard_selected_scale_factor_frame_details_with_magnitude_bias_and_bitrate(
            &pcm,
            target_bitrate_bps,
            global_gain,
            scale_factor_magnitude_bias,
        )
        .map_err(to_py_value_error)?;
    let profile =
        sonare_codec_rs::aac_standard_selected_scale_factor_profile_for_frame_details_with_magnitude_bias(
            &pcm,
            &details,
            global_gain,
            scale_factor_magnitude_bias,
        )
        .map_err(to_py_value_error)?;

    Ok(vec![
        profile.frames as f64,
        profile.channels as f64,
        profile.bands as f64,
        profile.raised_bands as f64,
        f64::from(profile.max_delta),
        profile.mean_delta,
    ])
}

#[pyfunction]
pub(crate) fn aac_recommended_standard_selected_scale_factor_profile_with_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    target_bitrate_bps: u32,
) -> PyResult<Vec<f64>> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    let details =
        sonare_codec_rs::aac_recommended_standard_selected_scale_factor_frame_details_with_bitrate(
            &pcm,
            target_bitrate_bps,
        )
        .map_err(to_py_value_error)?;
    let profile =
        sonare_codec_rs::aac_recommended_standard_selected_scale_factor_profile_for_frame_details(
            &pcm, &details,
        )
        .map_err(to_py_value_error)?;

    Ok(vec![
        profile.frames as f64,
        profile.channels as f64,
        profile.bands as f64,
        profile.raised_bands as f64,
        f64::from(profile.max_delta),
        profile.mean_delta,
    ])
}

#[pyfunction]
pub(crate) fn aac_balanced_standard_selected_scale_factor_profile_with_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    target_bitrate_bps: u32,
) -> PyResult<Vec<f64>> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    let details =
        sonare_codec_rs::aac_balanced_standard_selected_scale_factor_frame_details_with_bitrate(
            &pcm,
            target_bitrate_bps,
        )
        .map_err(to_py_value_error)?;
    let profile =
        sonare_codec_rs::aac_balanced_standard_selected_scale_factor_profile_for_frame_details(
            &pcm, &details,
        )
        .map_err(to_py_value_error)?;

    Ok(vec![
        profile.frames as f64,
        profile.channels as f64,
        profile.bands as f64,
        profile.raised_bands as f64,
        f64::from(profile.max_delta),
        profile.mean_delta,
    ])
}

pub(crate) fn flatten_aac_standard_id_payload_breakdown(
    breakdown: sonare_codec_rs::AacStandardIdPayloadBreakdown,
) -> Vec<f64> {
    vec![
        breakdown.frames as f64,
        breakdown.channels as f64,
        breakdown.sections as f64,
        breakdown.escape_sections as f64,
        breakdown.max_abs as f64,
        breakdown.section_bits as f64,
        breakdown.scale_factor_bits as f64,
        breakdown.spectral_bits as f64,
        breakdown.escape_spectral_bits as f64,
        breakdown
            .dominant_spectral_section
            .map_or(0.0, |section| section.spectral_bits as f64),
        breakdown
            .dominant_escape_section
            .map_or(0.0, |section| section.spectral_bits as f64),
    ]
}

pub(crate) fn flatten_aac_standard_id_quality_control_profile(
    profile: sonare_codec_rs::AacStandardIdQualityControlProfile,
) -> Vec<f64> {
    vec![
        profile.frames as f64,
        profile.channels as f64,
        profile.max_frame_len as f64,
        profile.min_frame_budget_slack as f64,
        profile.max_quantized_abs_limit as f64,
        profile.max_abs as f64,
        profile.sections as f64,
        profile.escape_sections as f64,
        profile.total_bits as f64,
        profile.spectral_bits as f64,
        profile.escape_spectral_bits as f64,
        profile.scale_factor_bits as f64,
        profile.scale_factor_bands as f64,
        profile.raised_scale_factor_bands as f64,
        f64::from(profile.max_scale_factor_delta),
        profile.mean_scale_factor_delta,
    ]
}

pub(crate) fn flatten_aac_standard_id_quality_control_candidate(
    candidate: sonare_codec_rs::AacStandardIdQualityControlCandidate,
) -> Vec<f64> {
    let mut flattened = vec![
        f64::from(candidate.global_gain),
        f64::from(candidate.scale_factor_magnitude_bias),
        candidate.max_quantized_abs as f64,
    ];
    flattened.extend(flatten_aac_standard_id_quality_control_profile(
        candidate.profile,
    ));
    flattened
}

#[pyfunction]
pub(crate) fn aac_standard_id_payload_breakdown_with_magnitude_bias_and_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    target_bitrate_bps: u32,
    global_gain: u8,
    scale_factor_magnitude_bias: i16,
) -> PyResult<Vec<f64>> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    let details =
        sonare_codec_rs::aac_standard_selected_scale_factor_frame_details_with_magnitude_bias_and_bitrate(
            &pcm,
            target_bitrate_bps,
            global_gain,
            scale_factor_magnitude_bias,
        )
        .map_err(to_py_value_error)?;
    let breakdown =
        sonare_codec_rs::aac_standard_id_payload_breakdown_for_frame_details_with_magnitude_bias(
            &pcm,
            &details,
            global_gain,
            scale_factor_magnitude_bias,
        )
        .map_err(to_py_value_error)?;
    Ok(flatten_aac_standard_id_payload_breakdown(breakdown))
}

#[pyfunction]
pub(crate) fn aac_recommended_standard_id_payload_breakdown_with_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    target_bitrate_bps: u32,
) -> PyResult<Vec<f64>> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    let details =
        sonare_codec_rs::aac_recommended_standard_selected_scale_factor_frame_details_with_bitrate(
            &pcm,
            target_bitrate_bps,
        )
        .map_err(to_py_value_error)?;
    let breakdown =
        sonare_codec_rs::aac_recommended_standard_id_payload_breakdown_for_frame_details(
            &pcm, &details,
        )
        .map_err(to_py_value_error)?;
    Ok(flatten_aac_standard_id_payload_breakdown(breakdown))
}

#[pyfunction]
pub(crate) fn aac_balanced_standard_id_payload_breakdown_with_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    target_bitrate_bps: u32,
) -> PyResult<Vec<f64>> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    let details =
        sonare_codec_rs::aac_balanced_standard_selected_scale_factor_frame_details_with_bitrate(
            &pcm,
            target_bitrate_bps,
        )
        .map_err(to_py_value_error)?;
    let breakdown = sonare_codec_rs::aac_balanced_standard_id_payload_breakdown_for_frame_details(
        &pcm, &details,
    )
    .map_err(to_py_value_error)?;
    Ok(flatten_aac_standard_id_payload_breakdown(breakdown))
}

#[pyfunction]
pub(crate) fn aac_standard_id_quality_control_profile_with_magnitude_bias_max_quantized_abs_and_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    target_bitrate_bps: u32,
    global_gain: u8,
    scale_factor_magnitude_bias: i16,
    max_quantized_abs: u32,
) -> PyResult<Vec<f64>> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    let details =
        sonare_codec_rs::aac_standard_selected_scale_factor_frame_details_with_magnitude_bias_max_quantized_abs_and_bitrate(
            &pcm,
            target_bitrate_bps,
            global_gain,
            scale_factor_magnitude_bias,
            max_quantized_abs,
        )
        .map_err(to_py_value_error)?;
    let profile =
        sonare_codec_rs::aac_standard_id_quality_control_profile_for_frame_details_with_magnitude_bias_max_quantized_abs(
            &pcm,
            &details,
            global_gain,
            scale_factor_magnitude_bias,
            max_quantized_abs,
        )
        .map_err(to_py_value_error)?;
    Ok(flatten_aac_standard_id_quality_control_profile(profile))
}

#[pyfunction]
pub(crate) fn aac_balanced_standard_id_quality_control_profile_with_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    target_bitrate_bps: u32,
) -> PyResult<Vec<f64>> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    sonare_codec_rs::aac_balanced_standard_id_quality_control_profile_with_bitrate(
        &pcm,
        target_bitrate_bps,
    )
    .map(flatten_aac_standard_id_quality_control_profile)
    .map_err(to_py_value_error)
}

#[pyfunction]
pub(crate) fn aac_standard_id_quality_control_candidates_for_balance_profile_with_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    target_bitrate_bps: u32,
) -> PyResult<Vec<f64>> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    let candidates =
        sonare_codec_rs::aac_standard_id_quality_control_candidates_for_balance_profile_with_bitrate(
            &pcm,
            target_bitrate_bps,
        )
        .map_err(to_py_value_error)?;
    Ok(candidates
        .into_iter()
        .flat_map(flatten_aac_standard_id_quality_control_candidate)
        .collect())
}

#[pyfunction]
pub(crate) fn aac_selected_scale_factor_frame_details_with_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    target_bitrate_bps: u32,
) -> PyResult<Vec<f64>> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    let details = sonare_codec_rs::aac_selected_scale_factor_frame_details_with_bitrate(
        &pcm,
        target_bitrate_bps,
    )
    .map_err(to_py_value_error)?;

    Ok(details
        .iter()
        .enumerate()
        .flat_map(|(frame_index, detail)| {
            [
                frame_index as f64,
                f64::from(detail.step),
                detail.frame_len as f64,
                detail.frame_capacity_bytes as f64,
            ]
        })
        .collect())
}

pub(crate) fn add_py_functions(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(
        encode_aac_standard_mono_offsets_with_step,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(
        encode_aac_standard_mono_offsets_with_bitrate,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(
        aac_standard_mono_offsets_bitrate_frame_details,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(
        encode_aac_standard_stereo_offsets_with_step,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(
        encode_aac_standard_stereo_offsets_with_bitrate,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(
        aac_standard_stereo_offsets_bitrate_frame_details,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(
        aac_standard_selected_scale_factor_frame_details_with_magnitude_bias_and_bitrate,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(
        aac_recommended_standard_selected_scale_factor_frame_details_with_bitrate,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(aac_standard_selected_scale_factor_frame_details_with_magnitude_bias_max_quantized_abs_and_bitrate, module)?)?;
    module.add_function(wrap_pyfunction!(aac_recommended_standard_selected_scale_factor_frame_details_with_max_quantized_abs_and_bitrate, module)?)?;
    module.add_function(wrap_pyfunction!(
        aac_balanced_standard_selected_scale_factor_frame_details_with_bitrate,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(
        aac_standard_selected_scale_factor_profile_with_magnitude_bias_and_bitrate,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(
        aac_recommended_standard_selected_scale_factor_profile_with_bitrate,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(
        aac_balanced_standard_selected_scale_factor_profile_with_bitrate,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(
        aac_standard_id_payload_breakdown_with_magnitude_bias_and_bitrate,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(
        aac_recommended_standard_id_payload_breakdown_with_bitrate,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(
        aac_balanced_standard_id_payload_breakdown_with_bitrate,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(
        aac_standard_id_quality_control_profile_with_magnitude_bias_max_quantized_abs_and_bitrate,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(
        aac_balanced_standard_id_quality_control_profile_with_bitrate,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(
        aac_standard_id_quality_control_candidates_for_balance_profile_with_bitrate,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(
        aac_selected_scale_factor_frame_details_with_bitrate,
        module
    )?)?;
    Ok(())
}
