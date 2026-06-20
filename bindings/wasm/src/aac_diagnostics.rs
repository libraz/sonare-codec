use super::*;

#[wasm_bindgen]
pub fn encode_aac_standard_mono_offsets_with_step(
    sample_rate: u32,
    samples: &[f32],
    step: f32,
    global_gain: u8,
) -> Result<Vec<u8>, String> {
    let pcm = pcm_from_samples(sample_rate, 1, samples)?;
    let offsets = sonare_codec::aac_lc_long_window_scale_factor_band_offsets(sample_rate)
        .ok_or_else(|| "unsupported AAC-LC long-window sample rate".to_owned())?;
    let channel_config =
        sonare_codec::AacLongBlockConfig::new(global_gain, aac_offsets_max_sfb(offsets)?);
    let scale_factors_by_frame = constant_aac_scale_factors_by_frame(
        &pcm,
        usize::from(channel_config.global_gain),
        offsets.len() - 1,
    );
    let scale_factor_refs = scale_factors_by_frame
        .iter()
        .map(Vec::as_slice)
        .collect::<Vec<_>>();
    let scale_factor_table = sonare_codec::aac_scale_factor_delta_table();

    sonare_codec::encode_pcm_mono_long_block_adts_stream_with_standard_spectral_offsets_and_scale_factors_by_bit_cost(
        sonare_codec::AdtsConfig::aac_lc(sample_rate, 1),
        sonare_codec::AacScaleFactorSequence::new(channel_config, &scale_factor_refs),
        &pcm,
        0,
        step,
        offsets,
        &scale_factor_table,
    )
    .map_err(|err| err.to_string())
}

#[wasm_bindgen]
pub fn encode_aac_standard_mono_offsets_with_bitrate(
    sample_rate: u32,
    samples: &[f32],
    target_bitrate_bps: u32,
    global_gain: u8,
) -> Result<Vec<u8>, String> {
    let pcm = pcm_from_samples(sample_rate, 1, samples)?;
    sonare_codec::encode_aac_adts_with_standard_spectral_offsets_and_bitrate(
        &pcm,
        target_bitrate_bps,
        global_gain,
    )
    .map_err(|err| err.to_string())
}

#[wasm_bindgen]
pub fn aac_standard_mono_offsets_bitrate_frame_details(
    sample_rate: u32,
    samples: &[f32],
    target_bitrate_bps: u32,
    global_gain: u8,
) -> Result<Vec<f64>, String> {
    let pcm = pcm_from_samples(sample_rate, 1, samples)?;
    let offsets = sonare_codec::aac_lc_long_window_scale_factor_band_offsets(sample_rate)
        .ok_or_else(|| "unsupported AAC-LC long-window sample rate".to_owned())?;
    let channel_config =
        sonare_codec::AacLongBlockConfig::new(global_gain, aac_offsets_max_sfb(offsets)?);
    let scale_factors_by_frame = constant_aac_scale_factors_by_frame(
        &pcm,
        usize::from(channel_config.global_gain),
        offsets.len() - 1,
    );
    let scale_factor_refs = scale_factors_by_frame
        .iter()
        .map(Vec::as_slice)
        .collect::<Vec<_>>();
    let scale_factor_table = sonare_codec::aac_scale_factor_delta_table();

    let details = sonare_codec::select_aac_lc_mono_pcm_stream_frame_details_with_standard_spectral_offsets_and_scale_factors_and_bitrate_by_bit_cost(
        sonare_codec::AdtsConfig::aac_lc(sample_rate, 1),
        sonare_codec::AacScaleFactorSequence::new(channel_config, &scale_factor_refs),
        &pcm,
        0,
        offsets,
        sonare_codec::AAC_LC_PCM_STEP_CANDIDATES,
        target_bitrate_bps,
        &scale_factor_table,
    )
    .map_err(|err| err.to_string())?;

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

#[wasm_bindgen]
pub fn encode_aac_standard_stereo_offsets_with_step(
    sample_rate: u32,
    samples: &[f32],
    step: f32,
    global_gain: u8,
) -> Result<Vec<u8>, String> {
    let pcm = pcm_from_samples(sample_rate, 2, samples)?;
    let offsets = sonare_codec::aac_lc_long_window_scale_factor_band_offsets(sample_rate)
        .ok_or_else(|| "unsupported AAC-LC long-window sample rate".to_owned())?;
    let channel_config =
        sonare_codec::AacLongBlockConfig::new(global_gain, aac_offsets_max_sfb(offsets)?);
    let scale_factors_by_frame = constant_aac_scale_factors_by_frame(
        &pcm,
        usize::from(channel_config.global_gain),
        offsets.len() - 1,
    );
    let scale_factor_refs = scale_factors_by_frame
        .iter()
        .map(Vec::as_slice)
        .collect::<Vec<_>>();
    let scale_factor_table = sonare_codec::aac_scale_factor_delta_table();

    sonare_codec::encode_pcm_stereo_long_block_adts_stream_with_standard_spectral_offsets_and_scale_factors_by_bit_cost(
        sonare_codec::AdtsConfig::aac_lc(sample_rate, 2),
        sonare_codec::AacScaleFactorSequence::new(channel_config, &scale_factor_refs),
        sonare_codec::AacScaleFactorSequence::new(channel_config, &scale_factor_refs),
        &pcm,
        0,
        step,
        offsets,
        &scale_factor_table,
    )
    .map_err(|err| err.to_string())
}

#[wasm_bindgen]
pub fn encode_aac_standard_stereo_offsets_with_bitrate(
    sample_rate: u32,
    samples: &[f32],
    target_bitrate_bps: u32,
    global_gain: u8,
) -> Result<Vec<u8>, String> {
    let pcm = pcm_from_samples(sample_rate, 2, samples)?;
    sonare_codec::encode_aac_adts_with_standard_spectral_offsets_and_bitrate(
        &pcm,
        target_bitrate_bps,
        global_gain,
    )
    .map_err(|err| err.to_string())
}

#[wasm_bindgen]
pub fn aac_standard_stereo_offsets_bitrate_frame_details(
    sample_rate: u32,
    samples: &[f32],
    target_bitrate_bps: u32,
    global_gain: u8,
) -> Result<Vec<f64>, String> {
    let pcm = pcm_from_samples(sample_rate, 2, samples)?;
    let offsets = sonare_codec::aac_lc_long_window_scale_factor_band_offsets(sample_rate)
        .ok_or_else(|| "unsupported AAC-LC long-window sample rate".to_owned())?;
    let channel_config =
        sonare_codec::AacLongBlockConfig::new(global_gain, aac_offsets_max_sfb(offsets)?);
    let scale_factors_by_frame = constant_aac_scale_factors_by_frame(
        &pcm,
        usize::from(channel_config.global_gain),
        offsets.len() - 1,
    );
    let scale_factor_refs = scale_factors_by_frame
        .iter()
        .map(Vec::as_slice)
        .collect::<Vec<_>>();
    let scale_factor_table = sonare_codec::aac_scale_factor_delta_table();

    let details = sonare_codec::select_aac_lc_stereo_pcm_stream_frame_details_with_standard_spectral_offsets_and_scale_factors_and_bitrate_by_bit_cost(
        sonare_codec::AdtsConfig::aac_lc(sample_rate, 2),
        sonare_codec::AacScaleFactorSequence::new(channel_config, &scale_factor_refs),
        sonare_codec::AacScaleFactorSequence::new(channel_config, &scale_factor_refs),
        &pcm,
        0,
        offsets,
        sonare_codec::AAC_LC_PCM_STEP_CANDIDATES,
        target_bitrate_bps,
        &scale_factor_table,
    )
    .map_err(|err| err.to_string())?;

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

#[wasm_bindgen]
pub fn aac_standard_selected_scale_factor_frame_details_with_magnitude_bias_and_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: &[f32],
    target_bitrate_bps: u32,
    global_gain: u8,
    scale_factor_magnitude_bias: i16,
) -> Result<Vec<f64>, String> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    let details =
        sonare_codec::aac_standard_selected_scale_factor_frame_details_with_magnitude_bias_and_bitrate(
            &pcm,
            target_bitrate_bps,
            global_gain,
            scale_factor_magnitude_bias,
        )
        .map_err(|err| err.to_string())?;

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

#[wasm_bindgen]
pub fn aac_recommended_standard_selected_scale_factor_frame_details_with_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: &[f32],
    target_bitrate_bps: u32,
) -> Result<Vec<f64>, String> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    let details =
        sonare_codec::aac_recommended_standard_selected_scale_factor_frame_details_with_bitrate(
            &pcm,
            target_bitrate_bps,
        )
        .map_err(|err| err.to_string())?;

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

#[wasm_bindgen]
pub fn aac_standard_selected_scale_factor_frame_details_with_magnitude_bias_max_quantized_abs_and_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: &[f32],
    target_bitrate_bps: u32,
    global_gain: u8,
    scale_factor_magnitude_bias: i16,
    max_quantized_abs: u32,
) -> Result<Vec<f64>, String> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    let details =
        sonare_codec::aac_standard_selected_scale_factor_frame_details_with_magnitude_bias_max_quantized_abs_and_bitrate(
            &pcm,
            target_bitrate_bps,
            global_gain,
            scale_factor_magnitude_bias,
            max_quantized_abs,
        )
        .map_err(|err| err.to_string())?;

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

#[wasm_bindgen]
pub fn aac_recommended_standard_selected_scale_factor_frame_details_with_max_quantized_abs_and_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: &[f32],
    target_bitrate_bps: u32,
    max_quantized_abs: u32,
) -> Result<Vec<f64>, String> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    let details =
        sonare_codec::aac_recommended_standard_selected_scale_factor_frame_details_with_max_quantized_abs_and_bitrate(
            &pcm,
            target_bitrate_bps,
            max_quantized_abs,
        )
        .map_err(|err| err.to_string())?;

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

#[wasm_bindgen]
pub fn aac_balanced_standard_selected_scale_factor_frame_details_with_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: &[f32],
    target_bitrate_bps: u32,
) -> Result<Vec<f64>, String> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    let details =
        sonare_codec::aac_balanced_standard_selected_scale_factor_frame_details_with_bitrate(
            &pcm,
            target_bitrate_bps,
        )
        .map_err(|err| err.to_string())?;

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

#[wasm_bindgen]
pub fn aac_standard_selected_scale_factor_profile_with_magnitude_bias_and_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: &[f32],
    target_bitrate_bps: u32,
    global_gain: u8,
    scale_factor_magnitude_bias: i16,
) -> Result<Vec<f64>, String> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    let details =
        sonare_codec::aac_standard_selected_scale_factor_frame_details_with_magnitude_bias_and_bitrate(
            &pcm,
            target_bitrate_bps,
            global_gain,
            scale_factor_magnitude_bias,
        )
        .map_err(|err| err.to_string())?;
    let profile =
        sonare_codec::aac_standard_selected_scale_factor_profile_for_frame_details_with_magnitude_bias(
            &pcm,
            &details,
            global_gain,
            scale_factor_magnitude_bias,
        )
        .map_err(|err| err.to_string())?;

    Ok(vec![
        profile.frames as f64,
        profile.channels as f64,
        profile.bands as f64,
        profile.raised_bands as f64,
        f64::from(profile.max_delta),
        profile.mean_delta,
    ])
}

#[wasm_bindgen]
pub fn aac_recommended_standard_selected_scale_factor_profile_with_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: &[f32],
    target_bitrate_bps: u32,
) -> Result<Vec<f64>, String> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    let details =
        sonare_codec::aac_recommended_standard_selected_scale_factor_frame_details_with_bitrate(
            &pcm,
            target_bitrate_bps,
        )
        .map_err(|err| err.to_string())?;
    let profile =
        sonare_codec::aac_recommended_standard_selected_scale_factor_profile_for_frame_details(
            &pcm, &details,
        )
        .map_err(|err| err.to_string())?;

    Ok(vec![
        profile.frames as f64,
        profile.channels as f64,
        profile.bands as f64,
        profile.raised_bands as f64,
        f64::from(profile.max_delta),
        profile.mean_delta,
    ])
}

#[wasm_bindgen]
pub fn aac_balanced_standard_selected_scale_factor_profile_with_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: &[f32],
    target_bitrate_bps: u32,
) -> Result<Vec<f64>, String> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    let details =
        sonare_codec::aac_balanced_standard_selected_scale_factor_frame_details_with_bitrate(
            &pcm,
            target_bitrate_bps,
        )
        .map_err(|err| err.to_string())?;
    let profile =
        sonare_codec::aac_balanced_standard_selected_scale_factor_profile_for_frame_details(
            &pcm, &details,
        )
        .map_err(|err| err.to_string())?;

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
    breakdown: sonare_codec::AacStandardIdPayloadBreakdown,
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
    profile: sonare_codec::AacStandardIdQualityControlProfile,
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
    candidate: sonare_codec::AacStandardIdQualityControlCandidate,
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

#[wasm_bindgen]
pub fn aac_standard_id_payload_breakdown_with_magnitude_bias_and_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: &[f32],
    target_bitrate_bps: u32,
    global_gain: u8,
    scale_factor_magnitude_bias: i16,
) -> Result<Vec<f64>, String> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    let details =
        sonare_codec::aac_standard_selected_scale_factor_frame_details_with_magnitude_bias_and_bitrate(
            &pcm,
            target_bitrate_bps,
            global_gain,
            scale_factor_magnitude_bias,
        )
        .map_err(|err| err.to_string())?;
    let breakdown =
        sonare_codec::aac_standard_id_payload_breakdown_for_frame_details_with_magnitude_bias(
            &pcm,
            &details,
            global_gain,
            scale_factor_magnitude_bias,
        )
        .map_err(|err| err.to_string())?;
    Ok(flatten_aac_standard_id_payload_breakdown(breakdown))
}

#[wasm_bindgen]
pub fn aac_recommended_standard_id_payload_breakdown_with_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: &[f32],
    target_bitrate_bps: u32,
) -> Result<Vec<f64>, String> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    let details =
        sonare_codec::aac_recommended_standard_selected_scale_factor_frame_details_with_bitrate(
            &pcm,
            target_bitrate_bps,
        )
        .map_err(|err| err.to_string())?;
    let breakdown = sonare_codec::aac_recommended_standard_id_payload_breakdown_for_frame_details(
        &pcm, &details,
    )
    .map_err(|err| err.to_string())?;
    Ok(flatten_aac_standard_id_payload_breakdown(breakdown))
}

#[wasm_bindgen]
pub fn aac_balanced_standard_id_payload_breakdown_with_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: &[f32],
    target_bitrate_bps: u32,
) -> Result<Vec<f64>, String> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    let details =
        sonare_codec::aac_balanced_standard_selected_scale_factor_frame_details_with_bitrate(
            &pcm,
            target_bitrate_bps,
        )
        .map_err(|err| err.to_string())?;
    let breakdown =
        sonare_codec::aac_balanced_standard_id_payload_breakdown_for_frame_details(&pcm, &details)
            .map_err(|err| err.to_string())?;
    Ok(flatten_aac_standard_id_payload_breakdown(breakdown))
}

#[wasm_bindgen]
pub fn aac_standard_id_quality_control_profile_with_magnitude_bias_max_quantized_abs_and_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: &[f32],
    target_bitrate_bps: u32,
    global_gain: u8,
    scale_factor_magnitude_bias: i16,
    max_quantized_abs: u32,
) -> Result<Vec<f64>, String> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    let details =
        sonare_codec::aac_standard_selected_scale_factor_frame_details_with_magnitude_bias_max_quantized_abs_and_bitrate(
            &pcm,
            target_bitrate_bps,
            global_gain,
            scale_factor_magnitude_bias,
            max_quantized_abs,
        )
        .map_err(|err| err.to_string())?;
    let profile =
        sonare_codec::aac_standard_id_quality_control_profile_for_frame_details_with_magnitude_bias_max_quantized_abs(
            &pcm,
            &details,
            global_gain,
            scale_factor_magnitude_bias,
            max_quantized_abs,
        )
        .map_err(|err| err.to_string())?;
    Ok(flatten_aac_standard_id_quality_control_profile(profile))
}

#[wasm_bindgen]
pub fn aac_balanced_standard_id_quality_control_profile_with_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: &[f32],
    target_bitrate_bps: u32,
) -> Result<Vec<f64>, String> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    sonare_codec::aac_balanced_standard_id_quality_control_profile_with_bitrate(
        &pcm,
        target_bitrate_bps,
    )
    .map(flatten_aac_standard_id_quality_control_profile)
    .map_err(|err| err.to_string())
}

#[wasm_bindgen]
pub fn aac_standard_id_quality_control_candidates_for_balance_profile_with_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: &[f32],
    target_bitrate_bps: u32,
) -> Result<Vec<f64>, String> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    let candidates =
        sonare_codec::aac_standard_id_quality_control_candidates_for_balance_profile_with_bitrate(
            &pcm,
            target_bitrate_bps,
        )
        .map_err(|err| err.to_string())?;
    Ok(candidates
        .into_iter()
        .flat_map(flatten_aac_standard_id_quality_control_candidate)
        .collect())
}

#[wasm_bindgen]
pub fn aac_selected_scale_factor_frame_details_with_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: &[f32],
    target_bitrate_bps: u32,
) -> Result<Vec<f64>, String> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    let details = sonare_codec::aac_selected_scale_factor_frame_details_with_bitrate(
        &pcm,
        target_bitrate_bps,
    )
    .map_err(|err| err.to_string())?;

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

pub(crate) fn wasm_offsets_to_usize(offsets: &[u32]) -> Result<Vec<usize>, String> {
    offsets
        .iter()
        .map(|&offset| {
            usize::try_from(offset).map_err(|_| "AAC offset does not fit usize".to_owned())
        })
        .collect()
}

pub(crate) fn aac_offsets_max_sfb(offsets: &[usize]) -> Result<u8, String> {
    u8::try_from(offsets.len().saturating_sub(1))
        .map_err(|_| "AAC-LC scale-factor band count exceeds max_sfb range".to_owned())
}

pub(crate) fn constant_aac_scale_factors_by_frame(
    pcm: &sonare_codec::AudioBuffer,
    global_gain: usize,
    band_count: usize,
) -> Vec<Vec<i16>> {
    let frame_count = pcm.samples.len().div_ceil(usize::from(pcm.channels) * 1024);
    let scale_factor = i16::try_from(global_gain).unwrap_or(i16::MAX);
    (0..frame_count)
        .map(|_| vec![scale_factor; band_count])
        .collect()
}
