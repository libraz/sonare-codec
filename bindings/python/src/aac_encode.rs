use super::*;

#[pyfunction]
pub(crate) fn encode_aac(sample_rate: u32, channels: u16, samples: Vec<f32>) -> PyResult<Vec<u8>> {
    encode_format(sample_rate, channels, samples, sonare_codec_rs::Format::Aac)
}

#[pyfunction]
pub(crate) fn encode_aac_with_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    target_bitrate_bps: u32,
) -> PyResult<Vec<u8>> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    sonare_codec_rs::encode_aac_adts_with_bitrate(&pcm, target_bitrate_bps)
        .map_err(to_py_value_error)
}

#[pyfunction]
pub(crate) fn encode_aac_with_selected_scale_factors_and_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    target_bitrate_bps: u32,
) -> PyResult<Vec<u8>> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    sonare_codec_rs::encode_aac_adts_with_selected_scale_factors_and_bitrate(
        &pcm,
        target_bitrate_bps,
    )
    .map_err(to_py_value_error)
}

#[pyfunction]
pub(crate) fn encode_aac_with_standard_spectral_offsets_and_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    target_bitrate_bps: u32,
    global_gain: u8,
) -> PyResult<Vec<u8>> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    sonare_codec_rs::encode_aac_adts_with_standard_spectral_offsets_and_bitrate(
        &pcm,
        target_bitrate_bps,
        global_gain,
    )
    .map_err(to_py_value_error)
}

#[pyfunction]
pub(crate) fn encode_aac_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    target_bitrate_bps: u32,
    global_gain: u8,
    scale_factor_magnitude_bias: i16,
) -> PyResult<Vec<u8>> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    sonare_codec_rs::encode_aac_adts_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_bitrate(
        &pcm,
        target_bitrate_bps,
        global_gain,
        scale_factor_magnitude_bias,
    )
    .map_err(to_py_value_error)
}

#[pyfunction]
pub(crate) fn encode_aac_with_recommended_standard_spectral_offsets_and_selected_scale_factors_and_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    target_bitrate_bps: u32,
) -> PyResult<Vec<u8>> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    sonare_codec_rs::encode_aac_adts_with_recommended_standard_spectral_offsets_and_selected_scale_factors_and_bitrate(
        &pcm,
        target_bitrate_bps,
    )
    .map_err(to_py_value_error)
}

#[pyfunction]
pub(crate) fn encode_aac_with_standard_spectral_offsets_and_selected_scale_factors_max_quantized_abs_and_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    target_bitrate_bps: u32,
    global_gain: u8,
    scale_factor_magnitude_bias: i16,
    max_quantized_abs: u32,
) -> PyResult<Vec<u8>> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    sonare_codec_rs::encode_aac_adts_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_max_quantized_abs_and_bitrate(
        &pcm,
        target_bitrate_bps,
        global_gain,
        scale_factor_magnitude_bias,
        max_quantized_abs,
    )
    .map_err(to_py_value_error)
}

#[pyfunction]
pub(crate) fn encode_aac_with_recommended_standard_spectral_offsets_and_selected_scale_factors_max_quantized_abs_and_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    target_bitrate_bps: u32,
    max_quantized_abs: u32,
) -> PyResult<Vec<u8>> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    sonare_codec_rs::encode_aac_adts_with_recommended_standard_spectral_offsets_and_selected_scale_factors_max_quantized_abs_and_bitrate(
        &pcm,
        target_bitrate_bps,
        max_quantized_abs,
    )
    .map_err(to_py_value_error)
}

#[pyfunction]
pub(crate) fn encode_aac_with_balanced_standard_spectral_offsets_and_selected_scale_factors_and_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    target_bitrate_bps: u32,
) -> PyResult<Vec<u8>> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    sonare_codec_rs::encode_aac_adts_with_balanced_standard_spectral_offsets_and_selected_scale_factors_and_bitrate(
        &pcm,
        target_bitrate_bps,
    )
    .map_err(to_py_value_error)
}

#[pyfunction]
pub(crate) fn encode_m4a(sample_rate: u32, channels: u16, samples: Vec<f32>) -> PyResult<Vec<u8>> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    encode_by_name("m4a", &pcm)
}

#[pyfunction]
pub(crate) fn encode_m4a_with_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    target_bitrate_bps: u32,
) -> PyResult<Vec<u8>> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    sonare_codec_rs::encode_m4a_with_bitrate(&pcm, target_bitrate_bps).map_err(to_py_value_error)
}

#[pyfunction]
pub(crate) fn encode_m4a_with_selected_scale_factors_and_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    target_bitrate_bps: u32,
) -> PyResult<Vec<u8>> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    sonare_codec_rs::encode_m4a_with_selected_scale_factors_and_bitrate(&pcm, target_bitrate_bps)
        .map_err(to_py_value_error)
}

#[pyfunction]
pub(crate) fn encode_m4a_with_standard_spectral_offsets_and_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    target_bitrate_bps: u32,
    global_gain: u8,
) -> PyResult<Vec<u8>> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    sonare_codec_rs::encode_m4a_with_standard_spectral_offsets_and_bitrate(
        &pcm,
        target_bitrate_bps,
        global_gain,
    )
    .map_err(to_py_value_error)
}

#[pyfunction]
pub(crate) fn encode_m4a_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    target_bitrate_bps: u32,
    global_gain: u8,
    scale_factor_magnitude_bias: i16,
) -> PyResult<Vec<u8>> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    sonare_codec_rs::encode_m4a_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_bitrate(
        &pcm,
        target_bitrate_bps,
        global_gain,
        scale_factor_magnitude_bias,
    )
    .map_err(to_py_value_error)
}

#[pyfunction]
pub(crate) fn encode_m4a_with_recommended_standard_spectral_offsets_and_selected_scale_factors_and_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    target_bitrate_bps: u32,
) -> PyResult<Vec<u8>> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    sonare_codec_rs::encode_m4a_with_recommended_standard_spectral_offsets_and_selected_scale_factors_and_bitrate(
        &pcm,
        target_bitrate_bps,
    )
    .map_err(to_py_value_error)
}

#[pyfunction]
pub(crate) fn encode_m4a_with_standard_spectral_offsets_and_selected_scale_factors_max_quantized_abs_and_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    target_bitrate_bps: u32,
    global_gain: u8,
    scale_factor_magnitude_bias: i16,
    max_quantized_abs: u32,
) -> PyResult<Vec<u8>> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    sonare_codec_rs::encode_m4a_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_max_quantized_abs_and_bitrate(
        &pcm,
        target_bitrate_bps,
        global_gain,
        scale_factor_magnitude_bias,
        max_quantized_abs,
    )
    .map_err(to_py_value_error)
}

#[pyfunction]
pub(crate) fn encode_m4a_with_recommended_standard_spectral_offsets_and_selected_scale_factors_max_quantized_abs_and_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    target_bitrate_bps: u32,
    max_quantized_abs: u32,
) -> PyResult<Vec<u8>> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    sonare_codec_rs::encode_m4a_with_recommended_standard_spectral_offsets_and_selected_scale_factors_max_quantized_abs_and_bitrate(
        &pcm,
        target_bitrate_bps,
        max_quantized_abs,
    )
    .map_err(to_py_value_error)
}

#[pyfunction]
pub(crate) fn encode_m4a_with_balanced_standard_spectral_offsets_and_selected_scale_factors_and_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    target_bitrate_bps: u32,
) -> PyResult<Vec<u8>> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    sonare_codec_rs::encode_m4a_with_balanced_standard_spectral_offsets_and_selected_scale_factors_and_bitrate(
        &pcm,
        target_bitrate_bps,
    )
    .map_err(to_py_value_error)
}

#[pyfunction]
pub(crate) fn demux_m4a_as_aac_adts(input: &[u8]) -> PyResult<Vec<u8>> {
    sonare_codec_rs::demux_m4a_as_aac_adts(input).map_err(to_py_value_error)
}

#[pyfunction]
pub(crate) fn aac_lc_adts_max_frame_len_for_bitrate(
    sample_rate: u32,
    target_bitrate_bps: u32,
) -> PyResult<usize> {
    sonare_codec_rs::aac_lc_adts_max_frame_len_for_bitrate(sample_rate, target_bitrate_bps)
        .map_err(to_py_value_error)
}

#[pyfunction]
pub(crate) fn aac_lc_default_production_bitrate_bps(channels: u8) -> PyResult<u32> {
    sonare_codec_rs::aac_lc_default_production_bitrate_bps(channels).map_err(to_py_value_error)
}

#[pyfunction]
pub(crate) fn aac_lc_pcm_step_candidates() -> Vec<f64> {
    sonare_codec_rs::AAC_LC_PCM_STEP_CANDIDATES
        .iter()
        .map(|&step| f64::from(step))
        .collect()
}

#[pyfunction]
pub(crate) fn aac_standard_id_pcm_step_candidates() -> Vec<f64> {
    sonare_codec_rs::AAC_STANDARD_ID_PCM_STEP_CANDIDATES
        .iter()
        .map(|&step| f64::from(step))
        .collect()
}

#[pyfunction]
pub(crate) fn aac_standard_id_selected_scale_factor_global_gain(channels: u16) -> PyResult<u8> {
    sonare_codec_rs::aac_standard_id_selected_scale_factor_global_gain(channels)
        .map_err(to_py_value_error)
}

#[pyfunction]
pub(crate) fn aac_standard_id_selected_scale_factor_magnitude_bias() -> i16 {
    sonare_codec_rs::aac_standard_id_selected_scale_factor_magnitude_bias()
}

#[pyfunction]
pub(crate) fn aac_standard_id_selected_scale_factor_balanced_max_quantized_abs(
    channels: u16,
) -> PyResult<u32> {
    sonare_codec_rs::aac_standard_id_selected_scale_factor_balanced_max_quantized_abs(channels)
        .map_err(to_py_value_error)
}

#[pyfunction]
pub(crate) fn aac_standard_id_selected_scale_factor_balanced_parameters(
    channels: u16,
) -> PyResult<Vec<f64>> {
    let (global_gain, magnitude_bias, max_quantized_abs) =
        sonare_codec_rs::aac_standard_id_selected_scale_factor_balanced_parameters(channels)
            .map_err(to_py_value_error)?;
    Ok(vec![
        f64::from(global_gain),
        f64::from(magnitude_bias),
        f64::from(max_quantized_abs),
    ])
}

#[pyfunction]
pub(crate) fn aac_standard_id_selected_scale_factor_balanced_gain_deltas(
    channels: u16,
) -> PyResult<Vec<f64>> {
    let profile = sonare_codec_rs::aac_standard_id_selected_scale_factor_balance_profile(channels)
        .map_err(to_py_value_error)?;
    Ok(profile
        .global_gain_deltas
        .iter()
        .map(|&delta| f64::from(delta))
        .collect())
}

#[pyfunction]
pub(crate) fn aac_standard_id_selected_scale_factor_balanced_magnitude_biases(
    channels: u16,
) -> PyResult<Vec<f64>> {
    let profile = sonare_codec_rs::aac_standard_id_selected_scale_factor_balance_profile(channels)
        .map_err(to_py_value_error)?;
    Ok(profile
        .magnitude_biases
        .iter()
        .map(|&bias| f64::from(bias))
        .collect())
}

#[pyfunction]
pub(crate) fn aac_standard_id_selected_scale_factor_parameters(
    channels: u16,
) -> PyResult<Vec<f64>> {
    let (global_gain, magnitude_bias) =
        sonare_codec_rs::aac_standard_id_selected_scale_factor_parameters(channels)
            .map_err(to_py_value_error)?;
    Ok(vec![f64::from(global_gain), f64::from(magnitude_bias)])
}

pub(crate) fn add_py_functions(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(encode_aac, module)?)?;
    module.add_function(wrap_pyfunction!(encode_aac_with_bitrate, module)?)?;
    module.add_function(wrap_pyfunction!(
        encode_aac_with_selected_scale_factors_and_bitrate,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(
        encode_aac_with_standard_spectral_offsets_and_bitrate,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(encode_aac_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_bitrate, module)?)?;
    module.add_function(wrap_pyfunction!(encode_aac_with_recommended_standard_spectral_offsets_and_selected_scale_factors_and_bitrate, module)?)?;
    module.add_function(wrap_pyfunction!(encode_aac_with_standard_spectral_offsets_and_selected_scale_factors_max_quantized_abs_and_bitrate, module)?)?;
    module.add_function(wrap_pyfunction!(encode_aac_with_recommended_standard_spectral_offsets_and_selected_scale_factors_max_quantized_abs_and_bitrate, module)?)?;
    module.add_function(wrap_pyfunction!(
        encode_aac_with_balanced_standard_spectral_offsets_and_selected_scale_factors_and_bitrate,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(encode_m4a, module)?)?;
    module.add_function(wrap_pyfunction!(encode_m4a_with_bitrate, module)?)?;
    module.add_function(wrap_pyfunction!(
        encode_m4a_with_selected_scale_factors_and_bitrate,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(
        encode_m4a_with_standard_spectral_offsets_and_bitrate,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(encode_m4a_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_bitrate, module)?)?;
    module.add_function(wrap_pyfunction!(encode_m4a_with_recommended_standard_spectral_offsets_and_selected_scale_factors_and_bitrate, module)?)?;
    module.add_function(wrap_pyfunction!(encode_m4a_with_standard_spectral_offsets_and_selected_scale_factors_max_quantized_abs_and_bitrate, module)?)?;
    module.add_function(wrap_pyfunction!(encode_m4a_with_recommended_standard_spectral_offsets_and_selected_scale_factors_max_quantized_abs_and_bitrate, module)?)?;
    module.add_function(wrap_pyfunction!(
        encode_m4a_with_balanced_standard_spectral_offsets_and_selected_scale_factors_and_bitrate,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(demux_m4a_as_aac_adts, module)?)?;
    module.add_function(wrap_pyfunction!(
        aac_lc_adts_max_frame_len_for_bitrate,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(
        aac_lc_default_production_bitrate_bps,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(aac_lc_pcm_step_candidates, module)?)?;
    module.add_function(wrap_pyfunction!(
        aac_standard_id_pcm_step_candidates,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(
        aac_standard_id_selected_scale_factor_global_gain,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(
        aac_standard_id_selected_scale_factor_magnitude_bias,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(
        aac_standard_id_selected_scale_factor_balanced_max_quantized_abs,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(
        aac_standard_id_selected_scale_factor_balanced_parameters,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(
        aac_standard_id_selected_scale_factor_balanced_gain_deltas,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(
        aac_standard_id_selected_scale_factor_balanced_magnitude_biases,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(
        aac_standard_id_selected_scale_factor_parameters,
        module
    )?)?;
    Ok(())
}
