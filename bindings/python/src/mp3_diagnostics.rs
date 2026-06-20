use super::*;

#[pyfunction]
pub(crate) fn mp3_layer3_main_data_capacity_bytes(
    sample_rate: u32,
    channels: u16,
    bitrate_kbps: u16,
    padding: bool,
    crc_protected: bool,
) -> PyResult<usize> {
    let header = sonare_codec_rs::layer3_header_for_capacity(
        sample_rate,
        channels,
        bitrate_kbps,
        padding,
        crc_protected,
    )
    .map_err(to_py_value_error)?;
    sonare_codec_rs::layer3_main_data_capacity_bytes(header).map_err(to_py_value_error)
}

#[pyfunction]
pub(crate) fn mp3_layer3_main_data_capacity_bits(
    sample_rate: u32,
    channels: u16,
    bitrate_kbps: u16,
    padding: bool,
    crc_protected: bool,
) -> PyResult<usize> {
    let header = sonare_codec_rs::layer3_header_for_capacity(
        sample_rate,
        channels,
        bitrate_kbps,
        padding,
        crc_protected,
    )
    .map_err(to_py_value_error)?;
    sonare_codec_rs::layer3_main_data_capacity_bits(header).map_err(to_py_value_error)
}

#[pyfunction]
pub(crate) fn mp3_pcm_step_candidates() -> Vec<f64> {
    sonare_codec_rs::MPEG1_LAYER3_PCM_STEP_CANDIDATES
        .iter()
        .map(|&step| f64::from(step))
        .collect()
}

#[pyfunction]
pub(crate) fn mp3_production_pcm_step_candidates(channels: u16) -> PyResult<Vec<f64>> {
    sonare_codec_rs::mpeg1_layer3_production_pcm_step_candidates(channels)
        .map(|candidates| candidates.iter().map(|&step| f64::from(step)).collect())
        .map_err(to_py_value_error)
}

#[pyfunction]
pub(crate) fn mp3_first_frame_perceptual_candidate_profile_with_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    bitrate_kbps: u16,
    crc_protected: bool,
) -> PyResult<Vec<f64>> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    let profiles =
        sonare_codec_rs::select_mpeg1_layer3_first_frame_perceptual_candidate_profile_with_table_provider(
            &pcm,
            sonare_codec_rs::MPEG1_LAYER3_PCM_STEP_CANDIDATES,
            bitrate_kbps,
            crc_protected,
            sonare_codec_rs::mpeg1_layer3_standard_table_provider(),
        )
        .map_err(to_py_value_error)?;
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

#[pyfunction]
pub(crate) fn mp3_first_frame_low_band_spectral_shape_candidate_profile_with_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    bitrate_kbps: u16,
    crc_protected: bool,
) -> PyResult<Vec<f64>> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    let profiles =
        sonare_codec_rs::select_mpeg1_layer3_first_frame_low_band_spectral_shape_candidate_profile_with_table_provider(
            &pcm,
            sonare_codec_rs::MPEG1_LAYER3_PCM_STEP_CANDIDATES,
            bitrate_kbps,
            crc_protected,
            sonare_codec_rs::mpeg1_layer3_standard_table_provider(),
        )
        .map_err(to_py_value_error)?;
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

#[pyfunction]
pub(crate) fn mp3_first_frame_band_spectral_shape_candidate_profile_with_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    bitrate_kbps: u16,
    crc_protected: bool,
) -> PyResult<Vec<f64>> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    let profiles =
        sonare_codec_rs::select_mpeg1_layer3_first_frame_band_spectral_shape_candidate_profile_with_table_provider(
            &pcm,
            sonare_codec_rs::MPEG1_LAYER3_PCM_STEP_CANDIDATES,
            bitrate_kbps,
            crc_protected,
            sonare_codec_rs::mpeg1_layer3_standard_table_provider(),
        )
        .map_err(to_py_value_error)?;
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

#[pyfunction]
pub(crate) fn mp3_first_frame_quality_guarded_candidate_profile_with_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    bitrate_kbps: u16,
    crc_protected: bool,
) -> PyResult<Vec<f64>> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    let profiles =
        sonare_codec_rs::select_mpeg1_layer3_first_frame_quality_guarded_candidate_profile_with_table_provider(
            &pcm,
            sonare_codec_rs::MPEG1_LAYER3_PCM_STEP_CANDIDATES,
            bitrate_kbps,
            crc_protected,
            sonare_codec_rs::mpeg1_layer3_standard_table_provider(),
        )
        .map_err(to_py_value_error)?;
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

#[pyfunction]
pub(crate) fn mp3_perceptual_bit_allocation_with_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    bitrate_kbps: u16,
    crc_protected: bool,
    min_bits_per_granule_channel: usize,
) -> PyResult<Vec<f64>> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    let allocations = sonare_codec_rs::select_mpeg1_layer3_perceptual_bit_allocation_with_bitrate(
        &pcm,
        bitrate_kbps,
        crc_protected,
        min_bits_per_granule_channel,
    )
    .map_err(to_py_value_error)?;
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

#[pyfunction]
pub(crate) fn mp3_standard_big_value_table_selects() -> Vec<u32> {
    sonare_codec_rs::MPEG1_LAYER3_STANDARD_BIG_VALUE_TABLE_SELECTS
        .iter()
        .map(|&table_select| u32::from(table_select))
        .collect()
}

#[pyfunction]
pub(crate) fn mp3_missing_standard_big_value_table_selects() -> Vec<u32> {
    sonare_codec_rs::MPEG1_LAYER3_MISSING_STANDARD_BIG_VALUE_TABLE_SELECTS
        .iter()
        .map(|&table_select| u32::from(table_select))
        .collect()
}

#[pyfunction]
pub(crate) fn mp3_standard_count1_table_selects() -> Vec<u32> {
    sonare_codec_rs::MPEG1_LAYER3_STANDARD_COUNT1_TABLE_SELECTS
        .iter()
        .map(|&table_select| u32::from(table_select))
        .collect()
}

#[pyfunction]
pub(crate) fn mp3_reservoir_frame_details_with_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    bitrate_kbps: u16,
    crc_protected: bool,
) -> PyResult<Vec<f64>> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    let details = sonare_codec_rs::select_mpeg1_layer3_reservoir_frame_details_with_table_provider(
        &pcm,
        sonare_codec_rs::MPEG1_LAYER3_PCM_STEP_CANDIDATES,
        bitrate_kbps,
        crc_protected,
        sonare_codec_rs::mpeg1_layer3_standard_table_provider(),
    )
    .map_err(to_py_value_error)?;

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

#[pyfunction]
pub(crate) fn mp3_perceptual_reservoir_frame_details_with_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    bitrate_kbps: u16,
    crc_protected: bool,
) -> PyResult<Vec<f64>> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    let details =
        sonare_codec_rs::select_mpeg1_layer3_perceptual_reservoir_frame_details_with_table_provider(
            &pcm,
            sonare_codec_rs::MPEG1_LAYER3_PCM_STEP_CANDIDATES,
            bitrate_kbps,
            crc_protected,
            sonare_codec_rs::mpeg1_layer3_standard_table_provider(),
        )
        .map_err(to_py_value_error)?;

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

#[pyfunction]
pub(crate) fn mp3_entropy_targeted_perceptual_reservoir_frame_details_with_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    bitrate_kbps: u16,
    crc_protected: bool,
    min_bits_per_granule_channel: usize,
) -> PyResult<Vec<f64>> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    let candidates = sonare_codec_rs::mpeg1_layer3_production_pcm_step_candidates(channels)
        .map_err(to_py_value_error)?;
    let details =
        sonare_codec_rs::select_mpeg1_layer3_entropy_targeted_perceptual_reservoir_frame_details_with_table_provider(
            &pcm,
            candidates,
            bitrate_kbps,
            crc_protected,
            min_bits_per_granule_channel,
            sonare_codec_rs::mpeg1_layer3_standard_table_provider(),
        )
        .map_err(to_py_value_error)?;

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

#[pyfunction]
pub(crate) fn mp3_entropy_targeted_perceptual_reservoir_utilization_profile_with_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    bitrate_kbps: u16,
    crc_protected: bool,
    min_bits_per_granule_channel: usize,
) -> PyResult<Vec<f64>> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    let candidates = sonare_codec_rs::mpeg1_layer3_production_pcm_step_candidates(channels)
        .map_err(to_py_value_error)?;
    let profile =
        sonare_codec_rs::select_mpeg1_layer3_entropy_target_utilization_profile_with_table_provider(
            &pcm,
            candidates,
            bitrate_kbps,
            crc_protected,
            min_bits_per_granule_channel,
            sonare_codec_rs::mpeg1_layer3_standard_table_provider(),
        )
        .map_err(to_py_value_error)?;

    Ok(vec![
        profile.frames as f64,
        profile.used_entropy_target_frames as f64,
        profile.payload_bits as f64,
        profile.entropy_budget_bits as f64,
        profile.utilization,
        profile.max_entropy_budget_slack_bits as f64,
    ])
}

#[pyfunction]
pub(crate) fn mp3_quality_guarded_perceptual_reservoir_frame_details_with_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    bitrate_kbps: u16,
    crc_protected: bool,
) -> PyResult<Vec<f64>> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    let details = sonare_codec_rs::select_mpeg1_layer3_quality_guarded_perceptual_reservoir_frame_details_with_table_provider(
        &pcm,
        sonare_codec_rs::MPEG1_LAYER3_PCM_STEP_CANDIDATES,
        bitrate_kbps,
        crc_protected,
        sonare_codec_rs::mpeg1_layer3_standard_table_provider(),
    )
    .map_err(to_py_value_error)?;

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

pub(crate) fn add_py_functions(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(
        mp3_layer3_main_data_capacity_bytes,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(
        mp3_layer3_main_data_capacity_bits,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(mp3_pcm_step_candidates, module)?)?;
    module.add_function(wrap_pyfunction!(
        mp3_production_pcm_step_candidates,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(
        mp3_first_frame_perceptual_candidate_profile_with_bitrate,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(
        mp3_first_frame_low_band_spectral_shape_candidate_profile_with_bitrate,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(
        mp3_first_frame_band_spectral_shape_candidate_profile_with_bitrate,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(
        mp3_first_frame_quality_guarded_candidate_profile_with_bitrate,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(
        mp3_perceptual_bit_allocation_with_bitrate,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(
        mp3_standard_big_value_table_selects,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(
        mp3_missing_standard_big_value_table_selects,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(mp3_standard_count1_table_selects, module)?)?;
    module.add_function(wrap_pyfunction!(
        mp3_reservoir_frame_details_with_bitrate,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(
        mp3_perceptual_reservoir_frame_details_with_bitrate,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(
        mp3_entropy_targeted_perceptual_reservoir_frame_details_with_bitrate,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(
        mp3_entropy_targeted_perceptual_reservoir_utilization_profile_with_bitrate,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(
        mp3_quality_guarded_perceptual_reservoir_frame_details_with_bitrate,
        module
    )?)?;
    Ok(())
}
