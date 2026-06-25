use super::*;

#[wasm_bindgen]
pub fn encode_audio(
    format: &str,
    sample_rate: u32,
    channels: u16,
    samples: &[f32],
) -> Result<Vec<u8>, String> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    encode_by_name(format, &pcm)
}

#[wasm_bindgen]
pub fn encode_audio_production(
    format: &str,
    sample_rate: u32,
    channels: u16,
    samples: &[f32],
) -> Result<Vec<u8>, String> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    encode_by_name_with_mode(format, &pcm, sonare_codec::EncodeMode::ProductionOnly)
}

#[wasm_bindgen]
pub fn encode_wav(sample_rate: u32, channels: u16, samples: &[f32]) -> Result<Vec<u8>, String> {
    encode_format(sample_rate, channels, samples, sonare_codec::Format::Wav)
}

#[wasm_bindgen]
pub fn encode_flac(sample_rate: u32, channels: u16, samples: &[f32]) -> Result<Vec<u8>, String> {
    encode_format(sample_rate, channels, samples, sonare_codec::Format::Flac)
}

#[wasm_bindgen]
pub fn encode_mp3(sample_rate: u32, channels: u16, samples: &[f32]) -> Result<Vec<u8>, String> {
    encode_format(sample_rate, channels, samples, sonare_codec::Format::Mp3)
}

#[cfg(feature = "mp3")]
#[wasm_bindgen]
pub fn encode_mp3_with_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: &[f32],
    bitrate_kbps: u16,
    padding: bool,
    crc_protected: bool,
) -> Result<Vec<u8>, String> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    sonare_codec::encode_mpeg1_layer3_pcm_frames_with_bitrate_and_table_provider(
        &pcm,
        sonare_codec::MPEG1_LAYER3_PCM_STEP_CANDIDATES,
        bitrate_kbps,
        padding,
        crc_protected,
        sonare_codec::mpeg1_layer3_standard_table_provider(),
    )
    .map_err(|err| err.to_string())
}

#[cfg(feature = "mp3")]
#[wasm_bindgen]
pub fn encode_mp3_cbr_with_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: &[f32],
    bitrate_kbps: u16,
    crc_protected: bool,
) -> Result<Vec<u8>, String> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    sonare_codec::encode_mpeg1_layer3_pcm_frames_with_cbr_bitrate_and_table_provider(
        &pcm,
        sonare_codec::MPEG1_LAYER3_PCM_STEP_CANDIDATES,
        bitrate_kbps,
        crc_protected,
        sonare_codec::mpeg1_layer3_standard_table_provider(),
    )
    .map_err(|err| err.to_string())
}

#[cfg(feature = "mp3")]
#[wasm_bindgen]
pub fn encode_mp3_perceptual_active_cbr_with_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: &[f32],
    bitrate_kbps: u16,
    crc_protected: bool,
) -> Result<Vec<u8>, String> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    sonare_codec::encode_mpeg1_layer3_pcm_frames_with_perceptual_active_cbr_bitrate_and_table_provider(
        &pcm,
        sonare_codec::MPEG1_LAYER3_PCM_STEP_CANDIDATES,
        bitrate_kbps,
        crc_protected,
        sonare_codec::mpeg1_layer3_standard_table_provider(),
    )
    .map_err(|err| err.to_string())
}

#[cfg(feature = "mp3")]
#[wasm_bindgen]
pub fn encode_mp3_perceptual_reservoir_with_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: &[f32],
    bitrate_kbps: u16,
    crc_protected: bool,
) -> Result<Vec<u8>, String> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    sonare_codec::encode_mpeg1_layer3_pcm_frames_with_perceptual_reservoir_and_table_provider(
        &pcm,
        sonare_codec::MPEG1_LAYER3_PCM_STEP_CANDIDATES,
        bitrate_kbps,
        crc_protected,
        sonare_codec::mpeg1_layer3_standard_table_provider(),
    )
    .map_err(|err| err.to_string())
}

#[cfg(feature = "mp3")]
#[wasm_bindgen]
pub fn encode_mp3_entropy_targeted_perceptual_reservoir_with_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: &[f32],
    bitrate_kbps: u16,
    crc_protected: bool,
    min_bits_per_granule_channel: usize,
) -> Result<Vec<u8>, String> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    let candidates = sonare_codec::mpeg1_layer3_production_pcm_step_candidates(channels)
        .map_err(|err| err.to_string())?;
    sonare_codec::encode_mpeg1_layer3_pcm_frames_with_entropy_targeted_perceptual_reservoir_and_table_provider(
        &pcm,
        candidates,
        bitrate_kbps,
        crc_protected,
        min_bits_per_granule_channel,
        sonare_codec::mpeg1_layer3_standard_table_provider(),
    )
    .map_err(|err| err.to_string())
}

#[cfg(feature = "mp3")]
#[wasm_bindgen]
pub fn encode_mp3_quality_guarded_perceptual_reservoir_with_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: &[f32],
    bitrate_kbps: u16,
    crc_protected: bool,
) -> Result<Vec<u8>, String> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    sonare_codec::encode_mpeg1_layer3_pcm_frames_with_quality_guarded_perceptual_reservoir_and_table_provider(
        &pcm,
        sonare_codec::MPEG1_LAYER3_PCM_STEP_CANDIDATES,
        bitrate_kbps,
        crc_protected,
        sonare_codec::mpeg1_layer3_standard_table_provider(),
    )
    .map_err(|err| err.to_string())
}

#[cfg(feature = "mp3")]
#[wasm_bindgen]
pub fn encode_mp3_perceptual_scale_factor_band_bias(
    sample_rate: u32,
    channels: u16,
    samples: &[f32],
    step: f32,
    band_start: usize,
    band_end: usize,
    bias: i8,
) -> Result<Vec<u8>, String> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    sonare_codec::encode_mpeg1_layer3_pcm_frames_with_perceptual_scale_factor_band_bias_and_table_provider(
        &pcm,
        step,
        sonare_codec::Layer3ScaleFactorBandBias {
            band_start,
            band_end,
            bias,
        },
        sonare_codec::mpeg1_layer3_standard_table_provider(),
    )
    .map_err(|err| err.to_string())
}

#[cfg(feature = "mp3")]
#[wasm_bindgen]
pub fn encode_mp3_perceptual_quantized_band_gain(
    sample_rate: u32,
    channels: u16,
    samples: &[f32],
    step: f32,
    band_start: usize,
    band_end: usize,
    gain: f32,
) -> Result<Vec<u8>, String> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    sonare_codec::encode_mpeg1_layer3_pcm_frames_with_perceptual_quantized_band_gain_and_table_provider(
        &pcm,
        step,
        sonare_codec::Layer3QuantizedBandGain {
            band_start,
            band_end,
            gain,
        },
        sonare_codec::mpeg1_layer3_standard_table_provider(),
    )
    .map_err(|err| err.to_string())
}

#[cfg(feature = "mp3")]
#[wasm_bindgen]
#[allow(clippy::too_many_arguments)]
pub fn encode_mp3_perceptual_quantized_band_gain_global_gain_bias(
    sample_rate: u32,
    channels: u16,
    samples: &[f32],
    step: f32,
    band_start: usize,
    band_end: usize,
    gain: f32,
    global_gain_bias: i16,
) -> Result<Vec<u8>, String> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    sonare_codec::encode_mpeg1_layer3_pcm_frames_with_perceptual_quantized_band_gain_and_global_gain_bias_and_table_provider(
        &pcm,
        step,
        sonare_codec::Layer3QuantizedBandGain {
            band_start,
            band_end,
            gain,
        },
        global_gain_bias,
        sonare_codec::mpeg1_layer3_standard_table_provider(),
    )
    .map_err(|err| err.to_string())
}

#[wasm_bindgen]
pub fn encode_vorbis(sample_rate: u32, channels: u16, samples: &[f32]) -> Result<Vec<u8>, String> {
    encode_format(sample_rate, channels, samples, sonare_codec::Format::Vorbis)
}

#[wasm_bindgen]
pub fn encode_opus(sample_rate: u32, channels: u16, samples: &[f32]) -> Result<Vec<u8>, String> {
    encode_format(sample_rate, channels, samples, sonare_codec::Format::Opus)
}
