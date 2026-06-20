use super::*;

#[wasm_bindgen]
pub fn encode_aac(sample_rate: u32, channels: u16, samples: &[f32]) -> Result<Vec<u8>, String> {
    encode_format(sample_rate, channels, samples, sonare_codec::Format::Aac)
}

#[wasm_bindgen]
pub fn encode_aac_with_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: &[f32],
    target_bitrate_bps: u32,
) -> Result<Vec<u8>, String> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    sonare_codec::encode_aac_adts_with_bitrate(&pcm, target_bitrate_bps)
        .map_err(|err| err.to_string())
}

#[wasm_bindgen]
pub fn encode_aac_with_selected_scale_factors_and_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: &[f32],
    target_bitrate_bps: u32,
) -> Result<Vec<u8>, String> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    sonare_codec::encode_aac_adts_with_selected_scale_factors_and_bitrate(&pcm, target_bitrate_bps)
        .map_err(|err| err.to_string())
}

#[wasm_bindgen]
pub fn encode_aac_with_standard_spectral_offsets_and_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: &[f32],
    target_bitrate_bps: u32,
    global_gain: u8,
) -> Result<Vec<u8>, String> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    sonare_codec::encode_aac_adts_with_standard_spectral_offsets_and_bitrate(
        &pcm,
        target_bitrate_bps,
        global_gain,
    )
    .map_err(|err| err.to_string())
}

#[wasm_bindgen]
pub fn encode_aac_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: &[f32],
    target_bitrate_bps: u32,
    global_gain: u8,
    scale_factor_magnitude_bias: i16,
) -> Result<Vec<u8>, String> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    sonare_codec::encode_aac_adts_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_bitrate(
        &pcm,
        target_bitrate_bps,
        global_gain,
        scale_factor_magnitude_bias,
    )
    .map_err(|err| err.to_string())
}

#[wasm_bindgen]
pub fn encode_aac_with_recommended_standard_spectral_offsets_and_selected_scale_factors_and_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: &[f32],
    target_bitrate_bps: u32,
) -> Result<Vec<u8>, String> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    sonare_codec::encode_aac_adts_with_recommended_standard_spectral_offsets_and_selected_scale_factors_and_bitrate(
        &pcm,
        target_bitrate_bps,
    )
    .map_err(|err| err.to_string())
}

#[wasm_bindgen]
pub fn encode_aac_with_standard_spectral_offsets_and_selected_scale_factors_max_quantized_abs_and_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: &[f32],
    target_bitrate_bps: u32,
    global_gain: u8,
    scale_factor_magnitude_bias: i16,
    max_quantized_abs: u32,
) -> Result<Vec<u8>, String> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    sonare_codec::encode_aac_adts_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_max_quantized_abs_and_bitrate(
        &pcm,
        target_bitrate_bps,
        global_gain,
        scale_factor_magnitude_bias,
        max_quantized_abs,
    )
    .map_err(|err| err.to_string())
}

#[wasm_bindgen]
pub fn encode_aac_with_recommended_standard_spectral_offsets_and_selected_scale_factors_max_quantized_abs_and_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: &[f32],
    target_bitrate_bps: u32,
    max_quantized_abs: u32,
) -> Result<Vec<u8>, String> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    sonare_codec::encode_aac_adts_with_recommended_standard_spectral_offsets_and_selected_scale_factors_max_quantized_abs_and_bitrate(
        &pcm,
        target_bitrate_bps,
        max_quantized_abs,
    )
    .map_err(|err| err.to_string())
}

#[wasm_bindgen]
pub fn encode_aac_with_balanced_standard_spectral_offsets_and_selected_scale_factors_and_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: &[f32],
    target_bitrate_bps: u32,
) -> Result<Vec<u8>, String> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    sonare_codec::encode_aac_adts_with_balanced_standard_spectral_offsets_and_selected_scale_factors_and_bitrate(
        &pcm,
        target_bitrate_bps,
    )
    .map_err(|err| err.to_string())
}

#[wasm_bindgen]
pub fn encode_m4a(sample_rate: u32, channels: u16, samples: &[f32]) -> Result<Vec<u8>, String> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    encode_by_name("m4a", &pcm)
}

#[wasm_bindgen]
pub fn encode_m4a_with_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: &[f32],
    target_bitrate_bps: u32,
) -> Result<Vec<u8>, String> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    sonare_codec::encode_m4a_with_bitrate(&pcm, target_bitrate_bps).map_err(|err| err.to_string())
}

#[wasm_bindgen]
pub fn encode_m4a_with_selected_scale_factors_and_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: &[f32],
    target_bitrate_bps: u32,
) -> Result<Vec<u8>, String> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    sonare_codec::encode_m4a_with_selected_scale_factors_and_bitrate(&pcm, target_bitrate_bps)
        .map_err(|err| err.to_string())
}

#[wasm_bindgen]
pub fn encode_m4a_with_standard_spectral_offsets_and_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: &[f32],
    target_bitrate_bps: u32,
    global_gain: u8,
) -> Result<Vec<u8>, String> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    sonare_codec::encode_m4a_with_standard_spectral_offsets_and_bitrate(
        &pcm,
        target_bitrate_bps,
        global_gain,
    )
    .map_err(|err| err.to_string())
}

#[wasm_bindgen]
pub fn encode_m4a_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: &[f32],
    target_bitrate_bps: u32,
    global_gain: u8,
    scale_factor_magnitude_bias: i16,
) -> Result<Vec<u8>, String> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    sonare_codec::encode_m4a_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_bitrate(
        &pcm,
        target_bitrate_bps,
        global_gain,
        scale_factor_magnitude_bias,
    )
    .map_err(|err| err.to_string())
}

#[wasm_bindgen]
pub fn encode_m4a_with_recommended_standard_spectral_offsets_and_selected_scale_factors_and_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: &[f32],
    target_bitrate_bps: u32,
) -> Result<Vec<u8>, String> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    sonare_codec::encode_m4a_with_recommended_standard_spectral_offsets_and_selected_scale_factors_and_bitrate(
        &pcm,
        target_bitrate_bps,
    )
    .map_err(|err| err.to_string())
}

#[wasm_bindgen]
pub fn encode_m4a_with_standard_spectral_offsets_and_selected_scale_factors_max_quantized_abs_and_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: &[f32],
    target_bitrate_bps: u32,
    global_gain: u8,
    scale_factor_magnitude_bias: i16,
    max_quantized_abs: u32,
) -> Result<Vec<u8>, String> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    sonare_codec::encode_m4a_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_max_quantized_abs_and_bitrate(
        &pcm,
        target_bitrate_bps,
        global_gain,
        scale_factor_magnitude_bias,
        max_quantized_abs,
    )
    .map_err(|err| err.to_string())
}

#[wasm_bindgen]
pub fn encode_m4a_with_recommended_standard_spectral_offsets_and_selected_scale_factors_max_quantized_abs_and_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: &[f32],
    target_bitrate_bps: u32,
    max_quantized_abs: u32,
) -> Result<Vec<u8>, String> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    sonare_codec::encode_m4a_with_recommended_standard_spectral_offsets_and_selected_scale_factors_max_quantized_abs_and_bitrate(
        &pcm,
        target_bitrate_bps,
        max_quantized_abs,
    )
    .map_err(|err| err.to_string())
}

#[wasm_bindgen]
pub fn encode_m4a_with_balanced_standard_spectral_offsets_and_selected_scale_factors_and_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: &[f32],
    target_bitrate_bps: u32,
) -> Result<Vec<u8>, String> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    sonare_codec::encode_m4a_with_balanced_standard_spectral_offsets_and_selected_scale_factors_and_bitrate(
        &pcm,
        target_bitrate_bps,
    )
    .map_err(|err| err.to_string())
}

#[wasm_bindgen]
pub fn demux_m4a_as_aac_adts(input: &[u8]) -> Result<Vec<u8>, String> {
    sonare_codec::demux_m4a_as_aac_adts(input).map_err(|err| err.to_string())
}

#[wasm_bindgen]
pub fn aac_lc_adts_max_frame_len_for_bitrate(
    sample_rate: u32,
    target_bitrate_bps: u32,
) -> Result<usize, String> {
    sonare_codec::aac_lc_adts_max_frame_len_for_bitrate(sample_rate, target_bitrate_bps)
        .map_err(|err| err.to_string())
}

#[wasm_bindgen]
pub fn aac_lc_default_production_bitrate_bps(channels: u8) -> Result<u32, String> {
    sonare_codec::aac_lc_default_production_bitrate_bps(channels).map_err(|err| err.to_string())
}

#[wasm_bindgen]
pub fn aac_lc_pcm_step_candidates() -> Vec<f32> {
    sonare_codec::AAC_LC_PCM_STEP_CANDIDATES.to_vec()
}

#[wasm_bindgen]
pub fn aac_standard_id_pcm_step_candidates() -> Vec<f32> {
    sonare_codec::AAC_STANDARD_ID_PCM_STEP_CANDIDATES.to_vec()
}

#[wasm_bindgen]
pub fn aac_standard_id_selected_scale_factor_global_gain(channels: u16) -> Result<u8, String> {
    sonare_codec::aac_standard_id_selected_scale_factor_global_gain(channels)
        .map_err(|err| err.to_string())
}

#[wasm_bindgen]
pub fn aac_standard_id_selected_scale_factor_magnitude_bias() -> i16 {
    sonare_codec::aac_standard_id_selected_scale_factor_magnitude_bias()
}

#[wasm_bindgen]
pub fn aac_standard_id_selected_scale_factor_balanced_max_quantized_abs(
    channels: u16,
) -> Result<u32, String> {
    sonare_codec::aac_standard_id_selected_scale_factor_balanced_max_quantized_abs(channels)
        .map_err(|err| err.to_string())
}

#[wasm_bindgen]
pub fn aac_standard_id_selected_scale_factor_balanced_parameters(
    channels: u16,
) -> Result<Vec<f64>, String> {
    let (global_gain, magnitude_bias, max_quantized_abs) =
        sonare_codec::aac_standard_id_selected_scale_factor_balanced_parameters(channels)
            .map_err(|err| err.to_string())?;
    Ok(vec![
        f64::from(global_gain),
        f64::from(magnitude_bias),
        f64::from(max_quantized_abs),
    ])
}

#[wasm_bindgen]
pub fn aac_standard_id_selected_scale_factor_balanced_gain_deltas(
    channels: u16,
) -> Result<Vec<f64>, String> {
    let profile = sonare_codec::aac_standard_id_selected_scale_factor_balance_profile(channels)
        .map_err(|err| err.to_string())?;
    Ok(profile
        .global_gain_deltas
        .iter()
        .map(|&delta| f64::from(delta))
        .collect())
}

#[wasm_bindgen]
pub fn aac_standard_id_selected_scale_factor_balanced_magnitude_biases(
    channels: u16,
) -> Result<Vec<f64>, String> {
    let profile = sonare_codec::aac_standard_id_selected_scale_factor_balance_profile(channels)
        .map_err(|err| err.to_string())?;
    Ok(profile
        .magnitude_biases
        .iter()
        .map(|&bias| f64::from(bias))
        .collect())
}

#[wasm_bindgen]
pub fn aac_standard_id_selected_scale_factor_parameters(channels: u16) -> Result<Vec<f64>, String> {
    let (global_gain, magnitude_bias) =
        sonare_codec::aac_standard_id_selected_scale_factor_parameters(channels)
            .map_err(|err| err.to_string())?;
    Ok(vec![f64::from(global_gain), f64::from(magnitude_bias)])
}
