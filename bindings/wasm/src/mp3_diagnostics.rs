use super::*;

#[wasm_bindgen]
pub fn mp3_layer3_main_data_capacity_bytes(
    sample_rate: u32,
    channels: u16,
    bitrate_kbps: u16,
    padding: bool,
    crc_protected: bool,
) -> Result<usize, String> {
    let header = sonare_codec::layer3_header_for_capacity(
        sample_rate,
        channels,
        bitrate_kbps,
        padding,
        crc_protected,
    )
    .map_err(|err| err.to_string())?;
    sonare_codec::layer3_main_data_capacity_bytes(header).map_err(|err| err.to_string())
}

#[wasm_bindgen]
pub fn mp3_layer3_main_data_capacity_bits(
    sample_rate: u32,
    channels: u16,
    bitrate_kbps: u16,
    padding: bool,
    crc_protected: bool,
) -> Result<usize, String> {
    let header = sonare_codec::layer3_header_for_capacity(
        sample_rate,
        channels,
        bitrate_kbps,
        padding,
        crc_protected,
    )
    .map_err(|err| err.to_string())?;
    sonare_codec::layer3_main_data_capacity_bits(header).map_err(|err| err.to_string())
}

#[wasm_bindgen]
pub fn mp3_pcm_step_candidates() -> Vec<f32> {
    sonare_codec::MPEG1_LAYER3_PCM_STEP_CANDIDATES.to_vec()
}

#[wasm_bindgen]
pub fn mp3_production_pcm_step_candidates(channels: u16) -> Result<Vec<f32>, String> {
    sonare_codec::mpeg1_layer3_production_pcm_step_candidates(channels)
        .map(|candidates| candidates.to_vec())
        .map_err(|err| err.to_string())
}

#[wasm_bindgen]
pub fn mp3_first_frame_perceptual_candidate_profile_with_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: &[f32],
    bitrate_kbps: u16,
    crc_protected: bool,
) -> Result<Vec<f64>, String> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    let profiles =
        sonare_codec::select_mpeg1_layer3_first_frame_perceptual_candidate_profile_with_table_provider(
            &pcm,
            sonare_codec::MPEG1_LAYER3_PCM_STEP_CANDIDATES,
            bitrate_kbps,
            crc_protected,
            sonare_codec::mpeg1_layer3_standard_table_provider(),
        )
        .map_err(|err| err.to_string())?;
    Ok(profiles
        .into_iter()
        .flat_map(|profile| {
            [
                f64::from(profile.step),
                profile.payload_bit_len as f64,
                profile.frame_capacity_bits as f64,
                profile.nonzero_scale_factors as f64,
                profile.scale_factor_bands as f64,
                f64::from(profile.max_scale_factor),
            ]
        })
        .collect())
}

#[wasm_bindgen]
pub fn mp3_first_frame_low_band_spectral_shape_candidate_profile_with_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: &[f32],
    bitrate_kbps: u16,
    crc_protected: bool,
) -> Result<Vec<f64>, String> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    let profiles =
        sonare_codec::select_mpeg1_layer3_first_frame_low_band_spectral_shape_candidate_profile_with_table_provider(
            &pcm,
            sonare_codec::MPEG1_LAYER3_PCM_STEP_CANDIDATES,
            bitrate_kbps,
            crc_protected,
            sonare_codec::mpeg1_layer3_standard_table_provider(),
        )
        .map_err(|err| err.to_string())?;
    Ok(profiles
        .into_iter()
        .flat_map(|profile| {
            [
                f64::from(profile.step),
                profile.payload_bit_len as f64,
                profile.frame_capacity_bits as f64,
                profile.low_band_abs_sum as f64,
                profile.total_abs_sum as f64,
                profile.low_band_nonzero_lines as f64,
                profile.total_nonzero_lines as f64,
            ]
        })
        .collect())
}

#[wasm_bindgen]
pub fn mp3_first_frame_band_spectral_shape_candidate_profile_with_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: &[f32],
    bitrate_kbps: u16,
    crc_protected: bool,
) -> Result<Vec<f64>, String> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    let profiles =
        sonare_codec::select_mpeg1_layer3_first_frame_band_spectral_shape_candidate_profile_with_table_provider(
            &pcm,
            sonare_codec::MPEG1_LAYER3_PCM_STEP_CANDIDATES,
            bitrate_kbps,
            crc_protected,
            sonare_codec::mpeg1_layer3_standard_table_provider(),
        )
        .map_err(|err| err.to_string())?;
    Ok(profiles
        .into_iter()
        .flat_map(|profile| {
            [
                f64::from(profile.step),
                profile.payload_bit_len as f64,
                profile.frame_capacity_bits as f64,
                profile.band as f64,
                profile.band_start as f64,
                profile.band_end as f64,
                profile.band_abs_sum as f64,
                profile.band_nonzero_lines as f64,
                profile.total_abs_sum as f64,
                profile.total_nonzero_lines as f64,
            ]
        })
        .collect())
}

#[wasm_bindgen]
pub fn mp3_first_frame_quality_guarded_candidate_profile_with_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: &[f32],
    bitrate_kbps: u16,
    crc_protected: bool,
) -> Result<Vec<f64>, String> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    let profiles = sonare_codec::select_mpeg1_layer3_first_frame_quality_guarded_candidate_profile_with_table_provider(
            &pcm,
            sonare_codec::MPEG1_LAYER3_PCM_STEP_CANDIDATES,
            bitrate_kbps,
            crc_protected,
            sonare_codec::mpeg1_layer3_standard_table_provider(),
        )
        .map_err(|err| err.to_string())?;
    Ok(profiles
        .into_iter()
        .flat_map(|profile| {
            [
                f64::from(profile.step),
                profile.payload_bit_len as f64,
                profile.frame_capacity_bits as f64,
                profile.perceptual_granules as f64,
                profile.calibrated_granules as f64,
                profile.quality_guard_compared_granules as f64,
                profile.quality_guard_distortion_delta,
            ]
        })
        .collect())
}

#[wasm_bindgen]
pub fn mp3_perceptual_bit_allocation_with_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: &[f32],
    bitrate_kbps: u16,
    crc_protected: bool,
    min_bits_per_granule_channel: usize,
) -> Result<Vec<f64>, String> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    let allocations = sonare_codec::select_mpeg1_layer3_perceptual_bit_allocation_with_bitrate(
        &pcm,
        bitrate_kbps,
        crc_protected,
        min_bits_per_granule_channel,
    )
    .map_err(|err| err.to_string())?;
    Ok(allocations
        .into_iter()
        .flat_map(|allocation| {
            [
                allocation.frame_index as f64,
                allocation.granule as f64,
                allocation.channel as f64,
                allocation.perceptual_entropy,
                allocation.target_bits as f64,
            ]
        })
        .collect())
}

#[wasm_bindgen]
pub fn mp3_standard_big_value_table_selects() -> Vec<u8> {
    sonare_codec::MPEG1_LAYER3_STANDARD_BIG_VALUE_TABLE_SELECTS.to_vec()
}

#[wasm_bindgen]
pub fn mp3_missing_standard_big_value_table_selects() -> Vec<u8> {
    sonare_codec::MPEG1_LAYER3_MISSING_STANDARD_BIG_VALUE_TABLE_SELECTS.to_vec()
}

#[wasm_bindgen]
pub fn mp3_standard_count1_table_selects() -> Vec<u8> {
    sonare_codec::MPEG1_LAYER3_STANDARD_COUNT1_TABLE_SELECTS
        .iter()
        .map(|&table_select| u8::from(table_select))
        .collect()
}

#[wasm_bindgen]
pub fn mp3_reservoir_frame_details_with_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: &[f32],
    bitrate_kbps: u16,
    crc_protected: bool,
) -> Result<Vec<f64>, String> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    let details = sonare_codec::select_mpeg1_layer3_reservoir_frame_details_with_table_provider(
        &pcm,
        sonare_codec::MPEG1_LAYER3_PCM_STEP_CANDIDATES,
        bitrate_kbps,
        crc_protected,
        sonare_codec::mpeg1_layer3_standard_table_provider(),
    )
    .map_err(|err| err.to_string())?;

    Ok(details
        .into_iter()
        .flat_map(|detail| {
            [
                detail.frame_index as f64,
                f64::from(detail.step),
                detail.payload_bit_len as f64,
                detail.frame_len as f64,
                u8::from(detail.padding) as f64,
                detail.frame_capacity_bytes as f64,
                detail.main_data_begin as f64,
                detail.reservoir_after as f64,
                detail.perceptual_granules as f64,
                detail.calibrated_granules as f64,
                detail.quality_guard_compared_granules as f64,
                detail.quality_guard_distortion_delta,
            ]
        })
        .collect())
}

#[wasm_bindgen]
pub fn mp3_perceptual_reservoir_frame_details_with_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: &[f32],
    bitrate_kbps: u16,
    crc_protected: bool,
) -> Result<Vec<f64>, String> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    let details =
        sonare_codec::select_mpeg1_layer3_perceptual_reservoir_frame_details_with_table_provider(
            &pcm,
            sonare_codec::MPEG1_LAYER3_PCM_STEP_CANDIDATES,
            bitrate_kbps,
            crc_protected,
            sonare_codec::mpeg1_layer3_standard_table_provider(),
        )
        .map_err(|err| err.to_string())?;

    Ok(details
        .into_iter()
        .flat_map(|detail| {
            [
                detail.frame_index as f64,
                f64::from(detail.step),
                detail.payload_bit_len as f64,
                detail.frame_len as f64,
                u8::from(detail.padding) as f64,
                detail.frame_capacity_bytes as f64,
                detail.main_data_begin as f64,
                detail.reservoir_after as f64,
                detail.perceptual_granules as f64,
                detail.calibrated_granules as f64,
                detail.quality_guard_compared_granules as f64,
                detail.quality_guard_distortion_delta,
            ]
        })
        .collect())
}

#[wasm_bindgen]
pub fn mp3_entropy_targeted_perceptual_reservoir_frame_details_with_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: &[f32],
    bitrate_kbps: u16,
    crc_protected: bool,
    min_bits_per_granule_channel: usize,
) -> Result<Vec<f64>, String> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    let candidates = sonare_codec::mpeg1_layer3_production_pcm_step_candidates(channels)
        .map_err(|err| err.to_string())?;
    let details =
        sonare_codec::select_mpeg1_layer3_entropy_targeted_perceptual_reservoir_frame_details_with_table_provider(
            &pcm,
            candidates,
            bitrate_kbps,
            crc_protected,
            min_bits_per_granule_channel,
            sonare_codec::mpeg1_layer3_standard_table_provider(),
        )
        .map_err(|err| err.to_string())?;

    Ok(details
        .into_iter()
        .flat_map(|detail| {
            [
                detail.frame_index as f64,
                f64::from(detail.step),
                detail.payload_bit_len as f64,
                detail.frame_len as f64,
                u8::from(detail.padding) as f64,
                detail.frame_capacity_bytes as f64,
                detail.main_data_begin as f64,
                detail.reservoir_after as f64,
                detail.perceptual_granules as f64,
                detail.calibrated_granules as f64,
                detail.quality_guard_compared_granules as f64,
                detail.quality_guard_distortion_delta,
                detail.entropy_target_bits as f64,
                u8::from(detail.used_entropy_target_budget) as f64,
            ]
        })
        .collect())
}

#[wasm_bindgen]
pub fn mp3_entropy_targeted_perceptual_reservoir_utilization_profile_with_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: &[f32],
    bitrate_kbps: u16,
    crc_protected: bool,
    min_bits_per_granule_channel: usize,
) -> Result<Vec<f64>, String> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    let candidates = sonare_codec::mpeg1_layer3_production_pcm_step_candidates(channels)
        .map_err(|err| err.to_string())?;
    let profile =
        sonare_codec::select_mpeg1_layer3_entropy_target_utilization_profile_with_table_provider(
            &pcm,
            candidates,
            bitrate_kbps,
            crc_protected,
            min_bits_per_granule_channel,
            sonare_codec::mpeg1_layer3_standard_table_provider(),
        )
        .map_err(|err| err.to_string())?;

    Ok(vec![
        profile.frames as f64,
        profile.used_entropy_target_frames as f64,
        profile.payload_bits as f64,
        profile.entropy_budget_bits as f64,
        profile.utilization,
        profile.max_entropy_budget_slack_bits as f64,
    ])
}

#[wasm_bindgen]
pub fn mp3_quality_guarded_perceptual_reservoir_frame_details_with_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: &[f32],
    bitrate_kbps: u16,
    crc_protected: bool,
) -> Result<Vec<f64>, String> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    let details = sonare_codec::select_mpeg1_layer3_quality_guarded_perceptual_reservoir_frame_details_with_table_provider(
        &pcm,
        sonare_codec::MPEG1_LAYER3_PCM_STEP_CANDIDATES,
        bitrate_kbps,
        crc_protected,
        sonare_codec::mpeg1_layer3_standard_table_provider(),
    )
    .map_err(|err| err.to_string())?;

    Ok(details
        .into_iter()
        .flat_map(|detail| {
            [
                detail.frame_index as f64,
                f64::from(detail.step),
                detail.payload_bit_len as f64,
                detail.frame_len as f64,
                u8::from(detail.padding) as f64,
                detail.frame_capacity_bytes as f64,
                detail.main_data_begin as f64,
                detail.reservoir_after as f64,
                detail.perceptual_granules as f64,
                detail.calibrated_granules as f64,
                detail.quality_guard_compared_granules as f64,
                detail.quality_guard_distortion_delta,
            ]
        })
        .collect())
}
