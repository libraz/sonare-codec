#![deny(unsafe_code)]
#![warn(clippy::all)]

use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen]
pub struct WavPcm {
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
}

#[wasm_bindgen]
pub struct StreamDecoder {
    inner: sonare_codec::StreamDecoder,
}

#[wasm_bindgen]
impl WavPcm {
    #[wasm_bindgen(getter)]
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    #[wasm_bindgen(getter)]
    pub fn channels(&self) -> u16 {
        self.channels
    }

    pub fn samples(&self) -> Vec<f32> {
        self.samples.clone()
    }
}

#[wasm_bindgen]
impl StreamDecoder {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn decode_stream(&mut self, chunk: &[u8]) -> Result<Option<WavPcm>, String> {
        self.inner
            .decode_stream(chunk)
            .map(|decoded| decoded.map(Into::into))
            .map_err(|err| err.to_string())
    }

    pub fn reset(&mut self) {
        self.inner.reset();
    }

    pub fn buffered_len(&self) -> usize {
        self.inner.buffered_len()
    }
}

impl Default for StreamDecoder {
    fn default() -> Self {
        Self {
            inner: sonare_codec::StreamDecoder::new(),
        }
    }
}

#[wasm_bindgen]
pub fn detect_format(input: &[u8]) -> Option<String> {
    if is_m4a_container(input) {
        return Some("m4a".to_owned());
    }
    sonare_codec::detect(input).map(|format| format!("{format:?}").to_ascii_lowercase())
}

#[wasm_bindgen]
pub fn decode_audio(input: &[u8]) -> Result<WavPcm, String> {
    let pcm = sonare_codec::decode(input).map_err(|err| err.to_string())?;
    Ok(pcm.into())
}

#[wasm_bindgen]
pub fn decode_wav(input: &[u8]) -> Result<WavPcm, String> {
    let pcm = sonare_codec::decode_wav(input).map_err(|err| err.to_string())?;
    Ok(pcm.into())
}

#[wasm_bindgen]
pub fn decode_flac(input: &[u8]) -> Result<WavPcm, String> {
    let pcm = sonare_codec::decode_flac(input).map_err(|err| err.to_string())?;
    Ok(pcm.into())
}

#[wasm_bindgen]
pub fn decode_mp3(input: &[u8]) -> Result<WavPcm, String> {
    let pcm = sonare_codec::decode_mp3(input).map_err(|err| err.to_string())?;
    Ok(pcm.into())
}

#[wasm_bindgen]
pub fn decode_vorbis(input: &[u8]) -> Result<WavPcm, String> {
    let pcm = sonare_codec::decode_vorbis(input).map_err(|err| err.to_string())?;
    Ok(pcm.into())
}

#[wasm_bindgen]
pub fn decode_opus(input: &[u8]) -> Result<WavPcm, String> {
    let pcm = sonare_codec::decode_opus(input).map_err(|err| err.to_string())?;
    Ok(pcm.into())
}

#[wasm_bindgen]
pub fn decode_aac(input: &[u8]) -> Result<WavPcm, String> {
    let pcm = sonare_codec::decode_aac(input).map_err(|err| err.to_string())?;
    Ok(pcm.into())
}

#[wasm_bindgen]
pub fn decode_m4a(input: &[u8]) -> Result<WavPcm, String> {
    let pcm = sonare_codec::decode_aac(input).map_err(|err| err.to_string())?;
    Ok(pcm.into())
}

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

#[wasm_bindgen]
pub fn aac_unsigned_pairs7_unit_magnitude_table() -> Vec<u32> {
    sonare_codec::aac_unsigned_pairs7_unit_magnitude_table()
        .iter()
        .flat_map(|entry| {
            [
                u32::from(entry.symbol.x),
                u32::from(entry.symbol.y),
                entry.code.bits,
                u32::from(entry.code.len),
            ]
        })
        .collect()
}

#[wasm_bindgen]
pub fn aac_unsigned_pairs7_table() -> Vec<u32> {
    sonare_codec::aac_unsigned_pairs7_table()
        .iter()
        .flat_map(|entry| {
            [
                u32::from(entry.symbol.x),
                u32::from(entry.symbol.y),
                entry.code.bits,
                u32::from(entry.code.len),
            ]
        })
        .collect()
}

#[wasm_bindgen]
pub fn aac_signed_pairs5_table() -> Vec<i32> {
    sonare_codec::aac_signed_pairs5_table()
        .iter()
        .flat_map(|entry| {
            [
                i32::from(entry.symbol.x),
                i32::from(entry.symbol.y),
                entry.code.bits as i32,
                i32::from(entry.code.len),
            ]
        })
        .collect()
}

#[wasm_bindgen]
pub fn aac_signed_pairs6_table() -> Vec<i32> {
    sonare_codec::aac_signed_pairs6_table()
        .iter()
        .flat_map(|entry| {
            [
                i32::from(entry.symbol.x),
                i32::from(entry.symbol.y),
                entry.code.bits as i32,
                i32::from(entry.code.len),
            ]
        })
        .collect()
}

#[wasm_bindgen]
pub fn aac_unsigned_pairs8_table() -> Vec<u32> {
    sonare_codec::aac_unsigned_pairs8_table()
        .iter()
        .flat_map(|entry| {
            [
                u32::from(entry.symbol.x),
                u32::from(entry.symbol.y),
                entry.code.bits,
                u32::from(entry.code.len),
            ]
        })
        .collect()
}

#[wasm_bindgen]
pub fn aac_unsigned_pairs9_table() -> Vec<u32> {
    sonare_codec::aac_unsigned_pairs9_table()
        .iter()
        .flat_map(|entry| {
            [
                u32::from(entry.symbol.x),
                u32::from(entry.symbol.y),
                entry.code.bits,
                u32::from(entry.code.len),
            ]
        })
        .collect()
}

#[wasm_bindgen]
pub fn aac_unsigned_pairs10_table() -> Vec<u32> {
    sonare_codec::aac_unsigned_pairs10_table()
        .iter()
        .flat_map(|entry| {
            [
                u32::from(entry.symbol.x),
                u32::from(entry.symbol.y),
                entry.code.bits,
                u32::from(entry.code.len),
            ]
        })
        .collect()
}

#[wasm_bindgen]
pub fn aac_signed_quads1_table() -> Vec<i32> {
    sonare_codec::aac_signed_quads1_table()
        .iter()
        .flat_map(|entry| {
            [
                i32::from(entry.symbol.v),
                i32::from(entry.symbol.w),
                i32::from(entry.symbol.x),
                i32::from(entry.symbol.y),
                i32::try_from(entry.code.bits).unwrap_or(i32::MAX),
                i32::from(entry.code.len),
            ]
        })
        .collect()
}

#[wasm_bindgen]
pub fn aac_signed_quads2_table() -> Vec<i32> {
    sonare_codec::aac_signed_quads2_table()
        .iter()
        .flat_map(|entry| {
            [
                i32::from(entry.symbol.v),
                i32::from(entry.symbol.w),
                i32::from(entry.symbol.x),
                i32::from(entry.symbol.y),
                i32::try_from(entry.code.bits).unwrap_or(i32::MAX),
                i32::from(entry.code.len),
            ]
        })
        .collect()
}

#[wasm_bindgen]
pub fn aac_unsigned_quads3_table() -> Vec<u32> {
    sonare_codec::aac_unsigned_quads3_table()
        .iter()
        .flat_map(|entry| {
            [
                u32::from(entry.symbol.v),
                u32::from(entry.symbol.w),
                u32::from(entry.symbol.x),
                u32::from(entry.symbol.y),
                entry.code.bits,
                u32::from(entry.code.len),
            ]
        })
        .collect()
}

#[wasm_bindgen]
pub fn aac_unsigned_quads4_table() -> Vec<u32> {
    sonare_codec::aac_unsigned_quads4_table()
        .iter()
        .flat_map(|entry| {
            [
                u32::from(entry.symbol.v),
                u32::from(entry.symbol.w),
                u32::from(entry.symbol.x),
                u32::from(entry.symbol.y),
                entry.code.bits,
                u32::from(entry.code.len),
            ]
        })
        .collect()
}

#[wasm_bindgen]
pub fn aac_escape_table() -> Vec<u32> {
    sonare_codec::aac_escape_table()
        .iter()
        .flat_map(|entry| {
            [
                u32::from(entry.symbol.x),
                u32::from(entry.symbol.y),
                entry.code.bits,
                u32::from(entry.code.len),
            ]
        })
        .collect()
}

#[wasm_bindgen]
pub fn aac_scale_factor_delta_table() -> Vec<i32> {
    sonare_codec::aac_scale_factor_delta_table()
        .iter()
        .flat_map(|entry| {
            [
                i32::from(entry.symbol.delta),
                i32::try_from(entry.code.bits).unwrap_or(i32::MAX),
                i32::from(entry.code.len),
            ]
        })
        .collect()
}

#[wasm_bindgen]
pub fn aac_codebook6_unit_section_plan(
    quantized: &[i32],
    band_width: usize,
) -> Result<Vec<u32>, String> {
    let sections = sonare_codec::plan_sections_by_bit_cost(
        quantized,
        band_width,
        sonare_codec::aac_unit_codebook6_spectral_tables(),
    )
    .map_err(|err| err.to_string())?;

    Ok(sections
        .iter()
        .flat_map(|section| {
            [
                section.start as u32,
                section.end as u32,
                u32::from(section.codebook.id()),
            ]
        })
        .collect())
}

#[wasm_bindgen]
pub fn aac_quad_unit_section_plan(
    quantized: &[i32],
    band_width: usize,
) -> Result<Vec<u32>, String> {
    let sections = sonare_codec::plan_quad_sections_by_bit_cost(
        quantized,
        band_width,
        sonare_codec::aac_unit_quad_spectral_tables(),
    )
    .map_err(|err| err.to_string())?;

    Ok(sections
        .iter()
        .flat_map(|section| {
            [
                section.start as u32,
                section.end as u32,
                u32::from(section.codebook_id),
            ]
        })
        .collect())
}

#[wasm_bindgen]
pub fn aac_mixed_unit_section_plan(
    quantized: &[i32],
    band_width: usize,
) -> Result<Vec<u32>, String> {
    let sections = sonare_codec::plan_spectral_sections_by_bit_cost(
        quantized,
        band_width,
        sonare_codec::aac_unit_codebook6_spectral_tables(),
        sonare_codec::aac_unit_quad_spectral_tables(),
    )
    .map_err(|err| err.to_string())?;

    Ok(sections
        .iter()
        .flat_map(|section| {
            [
                section.start as u32,
                section.end as u32,
                u32::from(section.codebook_id),
            ]
        })
        .collect())
}

#[wasm_bindgen]
pub fn aac_mixed_unit_payload_bit_lengths(
    quantized: &[i32],
    band_width: usize,
) -> Result<Vec<u32>, String> {
    let pair_tables = sonare_codec::aac_unit_codebook6_spectral_tables();
    let quad_tables = sonare_codec::aac_unit_quad_spectral_tables();
    let sections = sonare_codec::plan_spectral_sections_by_bit_cost(
        quantized,
        band_width,
        pair_tables,
        quad_tables,
    )
    .map_err(|err| err.to_string())?;
    let split = sonare_codec::split_sectioned_spectral_payload_by_codebook_id_with_sign_bits(
        &sections,
        quantized,
        band_width,
        pair_tables,
        quad_tables,
    )
    .map_err(|err| err.to_string())?;
    let packed = sonare_codec::pack_sectioned_spectral_payload_by_codebook_id_with_sign_bits(
        &sections,
        quantized,
        band_width,
        pair_tables,
        quad_tables,
    )
    .map_err(|err| err.to_string())?;
    let scale_factor_bits = sonare_codec::PackedBits {
        bytes: vec![0b1100_0000],
        bit_len: 2,
    };
    let split_with_scale =
        sonare_codec::split_sectioned_spectral_payload_by_codebook_id_with_sign_bits_and_scale_factor_bits(
            &sections,
            quantized,
            band_width,
            scale_factor_bits.clone(),
            pair_tables,
            quad_tables,
        )
        .map_err(|err| err.to_string())?;
    let packed_with_scale =
        sonare_codec::pack_sectioned_spectral_payload_by_codebook_id_with_sign_bits_and_scale_factor_bits(
            &sections,
            quantized,
            band_width,
            scale_factor_bits,
            pair_tables,
            quad_tables,
        )
        .map_err(|err| err.to_string())?;

    Ok(vec![
        split.section_and_scale_factor_bits.bit_len as u32,
        split.spectral_bits.bit_len as u32,
        packed.bit_len as u32,
        split_with_scale.section_and_scale_factor_bits.bit_len as u32,
        split_with_scale.spectral_bits.bit_len as u32,
        packed_with_scale.bit_len as u32,
    ])
}

#[wasm_bindgen]
pub fn aac_standard_unit_section_plan(
    quantized: &[i32],
    band_width: usize,
) -> Result<Vec<u32>, String> {
    let sections =
        sonare_codec::plan_aac_lc_standard_spectral_sections_by_bit_cost(quantized, band_width)
            .map_err(|err| err.to_string())?;

    Ok(sections
        .iter()
        .flat_map(|section| {
            [
                section.start as u32,
                section.end as u32,
                u32::from(section.codebook_id),
            ]
        })
        .collect())
}

#[wasm_bindgen]
pub fn aac_standard_offsets_section_plan(
    quantized: &[i32],
    offsets: &[u32],
) -> Result<Vec<u32>, String> {
    let offsets = wasm_offsets_to_usize(offsets)?;
    let sections = sonare_codec::plan_aac_lc_standard_spectral_sections_by_offsets_by_bit_cost(
        quantized, &offsets,
    )
    .map_err(|err| err.to_string())?;

    Ok(sections
        .iter()
        .flat_map(|section| {
            [
                section.start as u32,
                section.end as u32,
                u32::from(section.codebook_id),
            ]
        })
        .collect())
}

#[wasm_bindgen]
pub fn aac_standard_escape_payload_bit_lengths() -> Result<Vec<u32>, String> {
    let quantized = [17, 0];
    let band_width = 2;
    let pair_tables = sonare_codec::aac_lc_standard_spectral_tables();
    let quad_tables = sonare_codec::AacSpectralMagnitudeQuadTables::default();
    let sections = sonare_codec::plan_spectral_sections_by_bit_cost(
        &quantized,
        band_width,
        pair_tables,
        quad_tables,
    )
    .map_err(|err| err.to_string())?;
    if sections.first().map(|section| section.codebook_id)
        != Some(sonare_codec::AacCodebook::Escape.id())
    {
        return Err("AAC standard escape fixture did not select codebook 11".to_owned());
    }
    let split = sonare_codec::split_sectioned_spectral_payload_by_codebook_id_with_sign_bits(
        &sections,
        &quantized,
        band_width,
        pair_tables,
        quad_tables,
    )
    .map_err(|err| err.to_string())?;
    let packed = sonare_codec::pack_sectioned_spectral_payload_by_codebook_id_with_sign_bits(
        &sections,
        &quantized,
        band_width,
        pair_tables,
        quad_tables,
    )
    .map_err(|err| err.to_string())?;

    Ok(vec![
        split.section_and_scale_factor_bits.bit_len as u32,
        split.spectral_bits.bit_len as u32,
        packed.bit_len as u32,
    ])
}

#[wasm_bindgen]
pub fn aac_standard_mixed_payload_bit_lengths(
    quantized: &[i32],
    band_width: usize,
) -> Result<Vec<u32>, String> {
    let split = sonare_codec::split_aac_lc_standard_spectral_payload_with_sign_bits_by_bit_cost(
        quantized, band_width,
    )
    .map_err(|err| err.to_string())?;
    let packed = sonare_codec::pack_aac_lc_standard_spectral_payload_with_sign_bits_by_bit_cost(
        quantized, band_width,
    )
    .map_err(|err| err.to_string())?;
    let scale_factor_bits = sonare_codec::PackedBits {
        bytes: vec![0b1100_0000],
        bit_len: 2,
    };
    let split_with_scale =
        sonare_codec::split_aac_lc_standard_spectral_payload_with_sign_bits_and_scale_factor_bits_by_bit_cost(
            quantized,
            band_width,
            scale_factor_bits.clone(),
        )
        .map_err(|err| err.to_string())?;
    let packed_with_scale =
        sonare_codec::pack_aac_lc_standard_spectral_payload_with_sign_bits_and_scale_factor_bits_by_bit_cost(
            quantized,
            band_width,
            scale_factor_bits,
        )
        .map_err(|err| err.to_string())?;

    Ok(vec![
        split.section_and_scale_factor_bits.bit_len as u32,
        split.spectral_bits.bit_len as u32,
        packed.bit_len as u32,
        split_with_scale.section_and_scale_factor_bits.bit_len as u32,
        split_with_scale.spectral_bits.bit_len as u32,
        packed_with_scale.bit_len as u32,
    ])
}

#[wasm_bindgen]
pub fn aac_standard_mixed_offsets_payload_bit_lengths(
    quantized: &[i32],
    offsets: &[u32],
) -> Result<Vec<u32>, String> {
    let offsets = wasm_offsets_to_usize(offsets)?;
    let split =
        sonare_codec::split_aac_lc_standard_spectral_payload_with_offsets_and_sign_bits_by_bit_cost(
            quantized,
            &offsets,
        )
        .map_err(|err| err.to_string())?;
    let packed =
        sonare_codec::pack_aac_lc_standard_spectral_payload_with_offsets_and_sign_bits_by_bit_cost(
            quantized, &offsets,
        )
        .map_err(|err| err.to_string())?;
    let scale_factor_bits = sonare_codec::PackedBits {
        bytes: vec![0b1100_0000],
        bit_len: 2,
    };
    let split_with_scale =
        sonare_codec::split_aac_lc_standard_spectral_payload_with_offsets_and_sign_bits_and_scale_factor_bits_by_bit_cost(
            quantized,
            &offsets,
            scale_factor_bits.clone(),
        )
        .map_err(|err| err.to_string())?;
    let packed_with_scale =
        sonare_codec::pack_aac_lc_standard_spectral_payload_with_offsets_and_sign_bits_and_scale_factor_bits_by_bit_cost(
            quantized,
            &offsets,
            scale_factor_bits,
        )
        .map_err(|err| err.to_string())?;

    Ok(vec![
        split.section_and_scale_factor_bits.bit_len as u32,
        split.spectral_bits.bit_len as u32,
        packed.bit_len as u32,
        split_with_scale.section_and_scale_factor_bits.bit_len as u32,
        split_with_scale.spectral_bits.bit_len as u32,
        packed_with_scale.bit_len as u32,
    ])
}

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

fn flatten_aac_standard_id_payload_breakdown(
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

fn flatten_aac_standard_id_quality_control_profile(
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

fn flatten_aac_standard_id_quality_control_candidate(
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

fn wasm_offsets_to_usize(offsets: &[u32]) -> Result<Vec<usize>, String> {
    offsets
        .iter()
        .map(|&offset| {
            usize::try_from(offset).map_err(|_| "AAC offset does not fit usize".to_owned())
        })
        .collect()
}

fn aac_offsets_max_sfb(offsets: &[usize]) -> Result<u8, String> {
    u8::try_from(offsets.len().saturating_sub(1))
        .map_err(|_| "AAC-LC scale-factor band count exceeds max_sfb range".to_owned())
}

fn constant_aac_scale_factors_by_frame(
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

impl From<sonare_codec::AudioBuffer> for WavPcm {
    fn from(pcm: sonare_codec::AudioBuffer) -> Self {
        Self {
            sample_rate: pcm.sample_rate,
            channels: pcm.channels,
            samples: pcm.samples,
        }
    }
}

fn pcm_from_samples(
    sample_rate: u32,
    channels: u16,
    samples: &[f32],
) -> Result<sonare_codec::AudioBuffer, String> {
    sonare_codec::AudioBuffer::new(sample_rate, channels, samples.to_vec())
        .map_err(|err| err.to_string())
}

fn encode_format(
    sample_rate: u32,
    channels: u16,
    samples: &[f32],
    format: sonare_codec::Format,
) -> Result<Vec<u8>, String> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    sonare_codec::encode(format, &pcm).map_err(|err| err.to_string())
}

fn parse_format(format: &str) -> Result<sonare_codec::Format, String> {
    match format.to_ascii_lowercase().as_str() {
        "wav" => Ok(sonare_codec::Format::Wav),
        "flac" => Ok(sonare_codec::Format::Flac),
        "mp3" => Ok(sonare_codec::Format::Mp3),
        "vorbis" => Ok(sonare_codec::Format::Vorbis),
        "opus" => Ok(sonare_codec::Format::Opus),
        "aac" | "m4a" | "mp4" => Ok(sonare_codec::Format::Aac),
        _ => Err("unsupported format".to_owned()),
    }
}

fn encode_by_name(format: &str, pcm: &sonare_codec::AudioBuffer) -> Result<Vec<u8>, String> {
    encode_by_name_with_mode(format, pcm, sonare_codec::EncodeMode::Compatibility)
}

fn encode_by_name_with_mode(
    format: &str,
    pcm: &sonare_codec::AudioBuffer,
    mode: sonare_codec::EncodeMode,
) -> Result<Vec<u8>, String> {
    match format.to_ascii_lowercase().as_str() {
        "m4a" | "mp4" => {
            let aac = sonare_codec::encode_with_mode(sonare_codec::Format::Aac, pcm, mode)
                .map_err(|err| err.to_string())?;
            sonare_codec::mux_aac_adts_as_m4a(&aac).map_err(|err| err.to_string())
        }
        _ => {
            let format = parse_format(format)?;
            sonare_codec::encode_with_mode(format, pcm, mode).map_err(|err| err.to_string())
        }
    }
}

fn is_m4a_container(input: &[u8]) -> bool {
    input.len() >= 12
        && input.get(4..8) == Some(b"ftyp")
        && matches!(
            input.get(8..12),
            Some(b"M4A ") | Some(b"mp42") | Some(b"isom") | Some(b"iso2")
        )
}

#[cfg(test)]
mod tests {
    use super::{
        aac_balanced_standard_id_payload_breakdown_with_bitrate,
        aac_balanced_standard_id_quality_control_profile_with_bitrate,
        aac_balanced_standard_selected_scale_factor_profile_with_bitrate,
        aac_codebook6_unit_section_plan, aac_escape_table, aac_lc_adts_max_frame_len_for_bitrate,
        aac_lc_default_production_bitrate_bps, aac_lc_pcm_step_candidates,
        aac_mixed_unit_payload_bit_lengths, aac_mixed_unit_section_plan,
        aac_quad_unit_section_plan, aac_recommended_standard_id_payload_breakdown_with_bitrate,
        aac_recommended_standard_selected_scale_factor_frame_details_with_bitrate,
        aac_recommended_standard_selected_scale_factor_profile_with_bitrate,
        aac_scale_factor_delta_table, aac_selected_scale_factor_frame_details_with_bitrate,
        aac_signed_pairs5_table, aac_signed_pairs6_table, aac_signed_quads1_table,
        aac_signed_quads2_table, aac_standard_escape_payload_bit_lengths,
        aac_standard_id_payload_breakdown_with_magnitude_bias_and_bitrate,
        aac_standard_id_pcm_step_candidates,
        aac_standard_id_quality_control_candidates_for_balance_profile_with_bitrate,
        aac_standard_id_quality_control_profile_with_magnitude_bias_max_quantized_abs_and_bitrate,
        aac_standard_id_selected_scale_factor_balanced_gain_deltas,
        aac_standard_id_selected_scale_factor_balanced_magnitude_biases,
        aac_standard_id_selected_scale_factor_balanced_parameters,
        aac_standard_id_selected_scale_factor_global_gain,
        aac_standard_id_selected_scale_factor_magnitude_bias,
        aac_standard_id_selected_scale_factor_parameters,
        aac_standard_mixed_offsets_payload_bit_lengths, aac_standard_mixed_payload_bit_lengths,
        aac_standard_mono_offsets_bitrate_frame_details, aac_standard_offsets_section_plan,
        aac_standard_selected_scale_factor_frame_details_with_magnitude_bias_and_bitrate,
        aac_standard_selected_scale_factor_profile_with_magnitude_bias_and_bitrate,
        aac_standard_stereo_offsets_bitrate_frame_details, aac_standard_unit_section_plan,
        aac_unsigned_pairs10_table, aac_unsigned_pairs7_table,
        aac_unsigned_pairs7_unit_magnitude_table, aac_unsigned_pairs8_table,
        aac_unsigned_pairs9_table, aac_unsigned_quads3_table, aac_unsigned_quads4_table,
        decode_aac, decode_audio, decode_m4a, decode_mp3, demux_m4a_as_aac_adts, detect_format,
        encode_aac, encode_aac_standard_mono_offsets_with_bitrate,
        encode_aac_standard_mono_offsets_with_step,
        encode_aac_standard_stereo_offsets_with_bitrate,
        encode_aac_standard_stereo_offsets_with_step, encode_aac_with_bitrate,
        encode_aac_with_recommended_standard_spectral_offsets_and_selected_scale_factors_and_bitrate,
        encode_aac_with_selected_scale_factors_and_bitrate,
        encode_aac_with_standard_spectral_offsets_and_bitrate,
        encode_aac_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_bitrate,
        encode_audio, encode_audio_production, encode_m4a, encode_m4a_with_bitrate,
        encode_m4a_with_recommended_standard_spectral_offsets_and_selected_scale_factors_and_bitrate,
        encode_m4a_with_selected_scale_factors_and_bitrate,
        encode_m4a_with_standard_spectral_offsets_and_bitrate, encode_mp3,
        encode_mp3_cbr_with_bitrate, encode_mp3_entropy_targeted_perceptual_reservoir_with_bitrate,
        encode_mp3_perceptual_active_cbr_with_bitrate, encode_mp3_perceptual_quantized_band_gain,
        encode_mp3_perceptual_quantized_band_gain_global_gain_bias,
        encode_mp3_perceptual_reservoir_with_bitrate, encode_mp3_perceptual_scale_factor_band_bias,
        encode_mp3_quality_guarded_perceptual_reservoir_with_bitrate, encode_mp3_with_bitrate,
        encode_opus, encode_vorbis,
        mp3_entropy_targeted_perceptual_reservoir_frame_details_with_bitrate,
        mp3_entropy_targeted_perceptual_reservoir_utilization_profile_with_bitrate,
        mp3_first_frame_band_spectral_shape_candidate_profile_with_bitrate,
        mp3_first_frame_low_band_spectral_shape_candidate_profile_with_bitrate,
        mp3_first_frame_perceptual_candidate_profile_with_bitrate,
        mp3_first_frame_quality_guarded_candidate_profile_with_bitrate,
        mp3_layer3_main_data_capacity_bits, mp3_layer3_main_data_capacity_bytes,
        mp3_missing_standard_big_value_table_selects, mp3_pcm_step_candidates,
        mp3_perceptual_bit_allocation_with_bitrate,
        mp3_perceptual_reservoir_frame_details_with_bitrate, mp3_production_pcm_step_candidates,
        mp3_quality_guarded_perceptual_reservoir_frame_details_with_bitrate,
        mp3_reservoir_frame_details_with_bitrate, mp3_standard_big_value_table_selects,
        mp3_standard_count1_table_selects, StreamDecoder,
    };

    fn assert_mpeg1_layer3_frame_budget(
        encoded: &[u8],
        expected_sample_rate: u32,
        expected_channels: u16,
        expected_bitrate_kbps: u16,
    ) {
        assert!(encoded.len() >= 4);
        let slot_remainder = 144 * usize::from(expected_bitrate_kbps) * 1000
            % usize::try_from(expected_sample_rate).unwrap();
        let mut accumulator = 0usize;
        let mut offset = 0usize;
        let mut frames = 0usize;
        while offset < encoded.len() {
            let frame = &encoded[offset..];
            assert_eq!(frame[0], 0xff);
            assert_eq!(frame[1] & 0xe0, 0xe0);
            assert_eq!((frame[1] >> 3) & 0x03, 0x03);
            assert_eq!((frame[1] >> 1) & 0x03, 0x01);
            let bitrate_kbps: u16 = match frame[2] >> 4 {
                7 => 96,
                9 => 128,
                index => panic!("unexpected MPEG-1 Layer III bitrate index {index}"),
            };
            let sample_rate = match (frame[2] >> 2) & 0x03 {
                0 => 44_100,
                1 => 48_000,
                2 => 32_000,
                index => panic!("unexpected MPEG-1 sample-rate index {index}"),
            };
            let channels = if (frame[3] >> 6) & 0x03 == 0x03 { 1 } else { 2 };
            accumulator += slot_remainder;
            let expected_padding = if accumulator >= usize::try_from(sample_rate).unwrap() {
                accumulator -= usize::try_from(sample_rate).unwrap();
                1
            } else {
                0
            };
            let padding = usize::from(frame[2] & 0x02 != 0);
            let frame_len = 144 * usize::from(bitrate_kbps) * 1000
                / usize::try_from(sample_rate).unwrap()
                + padding;

            assert_eq!(bitrate_kbps, expected_bitrate_kbps);
            assert_eq!(sample_rate, expected_sample_rate);
            assert_eq!(channels, expected_channels);
            assert_eq!(padding, expected_padding);
            assert!(offset + frame_len <= encoded.len());
            offset += frame_len;
            frames += 1;
        }
        assert!(frames > 0);
        assert_eq!(offset, encoded.len());
    }

    fn mpeg1_layer3_main_data_begins(encoded: &[u8]) -> Vec<f64> {
        let mut begins = Vec::new();
        let mut offset = 0usize;
        while offset < encoded.len() {
            let frame = &encoded[offset..];
            assert!(frame.len() >= 6);
            assert_eq!(frame[0], 0xff);
            assert_eq!(frame[1] & 0xe0, 0xe0);
            let bitrate_kbps: u16 = match frame[2] >> 4 {
                7 => 96,
                9 => 128,
                index => panic!("unexpected MPEG-1 Layer III bitrate index {index}"),
            };
            let sample_rate = match (frame[2] >> 2) & 0x03 {
                0 => 44_100,
                1 => 48_000,
                2 => 32_000,
                index => panic!("unexpected MPEG-1 sample-rate index {index}"),
            };
            let padding = usize::from(frame[2] & 0x02 != 0);
            let frame_len = 144 * usize::from(bitrate_kbps) * 1000
                / usize::try_from(sample_rate).unwrap()
                + padding;
            begins.push(f64::from(
                (u16::from(frame[4]) << 1) | u16::from(frame[5] >> 7),
            ));
            offset += frame_len;
        }
        begins
    }

    #[test]
    fn unified_wav_api_roundtrips_pcm() {
        let samples = vec![0.0, 0.25, -0.25, 0.5];

        let encoded = encode_audio("wav", 44_100, 1, &samples).unwrap();
        let decoded = decode_audio(&encoded).unwrap();

        assert_eq!(detect_format(&encoded), Some("wav".to_owned()));
        assert_eq!(decoded.sample_rate(), 44_100);
        assert_eq!(decoded.channels(), 1);
        assert_eq!(decoded.samples().len(), samples.len());

        let production_samples = (0..2048)
            .map(|sample| ((sample as f32) * 0.01).sin() * 0.25)
            .collect::<Vec<_>>();
        let production = encode_audio_production("m4a", 44_100, 1, &production_samples).unwrap();
        let production_adts = demux_m4a_as_aac_adts(&production).unwrap();
        assert_eq!(detect_format(&production), Some("m4a".to_owned()));
        assert!(production.windows(4).any(|window| window == b"ftyp"));
        assert_eq!(
            production_adts,
            encode_audio_production("aac", 44_100, 1, &production_samples).unwrap()
        );
    }

    #[test]
    fn unified_flac_api_roundtrips_pcm() {
        let samples = (0..128)
            .map(|sample| sample as f32 / 32_767.0)
            .collect::<Vec<_>>();

        let encoded = encode_audio("flac", 48_000, 1, &samples).unwrap();
        let decoded = decode_audio(&encoded).unwrap();

        assert_eq!(detect_format(&encoded), Some("flac".to_owned()));
        assert_eq!(decoded.sample_rate(), 48_000);
        assert_eq!(decoded.channels(), 1);
        assert_eq!(decoded.samples().len(), samples.len());
    }

    #[test]
    fn stream_decoder_buffers_until_complete_input() {
        let samples = vec![0.0, 0.25, -0.25, 0.5];
        let encoded = encode_audio("wav", 44_100, 1, &samples).unwrap();
        let split = encoded.len() - 2;
        let mut decoder = StreamDecoder::new();

        assert!(decoder.decode_stream(&encoded[..split]).unwrap().is_none());
        assert!(decoder.buffered_len() > 0);
        let decoded = decoder
            .decode_stream(&encoded[split..])
            .unwrap()
            .expect("complete stream should decode");

        assert_eq!(decoded.sample_rate(), 44_100);
        assert_eq!(decoded.channels(), 1);
        assert_eq!(decoded.samples().len(), samples.len());
        assert_eq!(decoder.buffered_len(), 0);
    }

    #[test]
    fn unified_aac_api_encodes_silent_pcm() {
        let samples = vec![0.0; 1024];

        let encoded = encode_audio("aac", 44_100, 1, &samples).unwrap();
        let decoded = decode_audio(&encoded).unwrap();

        assert_eq!(
            encode_audio_production("aac", 44_100, 1, &samples).unwrap(),
            encoded
        );
        assert_eq!(detect_format(&encoded), Some("aac".to_owned()));
        assert_eq!(decoded.sample_rate(), 44_100);
        assert_eq!(decoded.channels(), 1);
        assert_eq!(decode_aac(&encoded).unwrap().samples().len(), samples.len());
        assert_eq!(encode_aac(44_100, 1, &samples).unwrap(), encoded);
    }

    #[test]
    fn unified_aac_api_encodes_non_silent_pcm_production_candidate() {
        for (sample_rate, channels) in [
            (7_350, 1),
            (8_000, 1),
            (11_025, 1),
            (12_000, 1),
            (16_000, 1),
            (22_050, 1),
            (24_000, 1),
            (32_000, 1),
            (44_100, 1),
            (48_000, 1),
            (64_000, 1),
            (88_200, 1),
            (96_000, 1),
            (7_350, 2),
            (8_000, 2),
            (11_025, 2),
            (12_000, 2),
            (16_000, 2),
            (22_050, 2),
            (24_000, 2),
            (32_000, 2),
            (44_100, 2),
            (48_000, 2),
            (64_000, 2),
            (88_200, 2),
            (96_000, 2),
        ] {
            let mut samples = Vec::new();
            for frame in 0..2048 {
                for channel in 0..channels {
                    let phase = if channel == 0 { 0.01 } else { 0.013 };
                    samples.push(((frame as f32) * phase).sin() * 0.25);
                }
            }

            let encoded = encode_audio("aac", sample_rate, channels, &samples).unwrap();
            let production =
                encode_audio_production("aac", sample_rate, channels, &samples).unwrap();

            assert_eq!(detect_format(&encoded), Some("aac".to_owned()));
            assert_eq!(&encoded[..2], &[0xff, 0xf1]);
            assert!(encoded.len() > 7);
            assert_eq!(production, encoded);
            assert_eq!(
                encode_aac(sample_rate, channels, &samples).unwrap(),
                encoded
            );
        }
    }

    #[test]
    fn unified_mp3_api_encodes_silent_pcm() {
        let samples = vec![0.0; 1152];

        let encoded = encode_audio("mp3", 44_100, 1, &samples).unwrap();
        let decoded = decode_audio(&encoded).unwrap();

        assert_eq!(
            encode_audio_production("mp3", 44_100, 1, &samples).unwrap(),
            encoded
        );
        assert_eq!(detect_format(&encoded), Some("mp3".to_owned()));
        assert_eq!(&encoded[..2], &[0xff, 0xfb]);
        assert_mpeg1_layer3_frame_budget(&encoded, 44_100, 1, 128);
        assert_eq!(decode_mp3(&encoded).unwrap().samples().len(), samples.len());
        assert_eq!(encode_mp3(44_100, 1, &samples).unwrap(), encoded);
        assert_eq!(decoded.sample_rate(), 44_100);
        assert_eq!(decoded.channels(), 1);
        assert_eq!(decoded.samples().len(), samples.len());
    }

    #[test]
    fn unified_mp3_api_encodes_non_silent_pcm_production_candidate() {
        for (sample_rate, channels) in [
            (32_000, 1),
            (44_100, 1),
            (48_000, 1),
            (32_000, 2),
            (44_100, 2),
            (48_000, 2),
        ] {
            let mut samples = Vec::new();
            for frame in 0..2048 {
                for channel in 0..channels {
                    let phase = if channel == 0 { 0.01 } else { 0.013 };
                    samples.push(((frame as f32) * phase).sin() * 0.25);
                }
            }

            let encoded = encode_audio("mp3", sample_rate, channels, &samples).unwrap();
            let production =
                encode_audio_production("mp3", sample_rate, channels, &samples).unwrap();
            let decoded = decode_audio(&encoded).unwrap();

            assert_eq!(detect_format(&encoded), Some("mp3".to_owned()));
            assert_eq!(&encoded[..2], &[0xff, 0xfb]);
            assert_mpeg1_layer3_frame_budget(&encoded, sample_rate, channels, 128);
            assert_eq!(production, encoded);
            assert_eq!(
                encode_mp3(sample_rate, channels, &samples).unwrap(),
                encoded
            );
            assert_eq!(decoded.sample_rate(), sample_rate);
            assert_eq!(decoded.channels(), channels);
            assert_eq!(decoded.samples().len(), 2304 * usize::from(channels));
        }
    }

    #[test]
    fn exposes_mp3_bitrate_encode_helper() {
        let samples = (0..1152)
            .map(|sample| ((sample as f32) * 0.01).sin() * 0.25)
            .collect::<Vec<_>>();

        let encoded = encode_mp3_with_bitrate(44_100, 1, &samples, 96, false, false).unwrap();

        assert_eq!(detect_format(&encoded), Some("mp3".to_owned()));
        assert_eq!(&encoded[..2], &[0xff, 0xfb]);
        assert_mpeg1_layer3_frame_budget(&encoded, 44_100, 1, 96);
        assert_eq!(
            mp3_layer3_main_data_capacity_bytes(44_100, 1, 96, false, false).unwrap(),
            292
        );
        assert!(encode_mp3_with_bitrate(44_100, 1, &samples, 123, false, false).is_err());
    }

    #[test]
    fn exposes_mp3_cbr_bitrate_encode_helper() {
        let samples = (0..(1152 * 3))
            .map(|sample| ((sample as f32) * 0.01).sin() * 0.25)
            .collect::<Vec<_>>();

        let encoded = encode_mp3_cbr_with_bitrate(44_100, 1, &samples, 128, false).unwrap();
        let first_len = 144 * 128_000 / 44_100;
        let padded_len = first_len + 1;

        assert_mpeg1_layer3_frame_budget(&encoded, 44_100, 1, 128);
        assert_eq!(encoded.len(), first_len + 2 * padded_len);
        assert!(encode_mp3_cbr_with_bitrate(44_100, 1, &samples, 123, false).is_err());
    }

    #[test]
    fn exposes_mp3_reservoir_frame_details_helper() {
        let frames = 8_usize;
        let detail_width = 12_usize;
        let samples_per_frame = 1152_usize;
        let samples = (0..(frames * samples_per_frame))
            .map(|index| {
                let frame = index / samples_per_frame;
                let t = (index % samples_per_frame) as f32;
                if frame % 2 == 0 {
                    0.3 * ((t * 0.043).sin()
                        + (t * 0.131).sin()
                        + (t * 0.277).sin()
                        + (t * 0.611).sin())
                } else {
                    0.02 * (t * 0.05).sin()
                }
            })
            .collect::<Vec<_>>();

        let details =
            mp3_reservoir_frame_details_with_bitrate(44_100, 1, &samples, 128, false).unwrap();

        assert_eq!(details.len(), frames * detail_width);
        assert_eq!(details[0], 0.0);
        assert_eq!(details[6], 0.0);
        assert!(details
            .chunks_exact(detail_width)
            .any(|detail| detail[6] > 0.0));
        assert!(details.chunks_exact(detail_width).all(|detail| {
            detail[2] <= (detail[5] + detail[6]) * 8.0
                && detail[7] >= 0.0
                && detail[8] == 0.0
                && detail[9] == 2.0
                && detail[10] == 0.0
                && detail[11] == 0.0
        }));

        let perceptual_details =
            mp3_perceptual_reservoir_frame_details_with_bitrate(44_100, 1, &samples, 128, false)
                .unwrap();
        let perceptual =
            encode_mp3_perceptual_reservoir_with_bitrate(44_100, 1, &samples, 128, false).unwrap();
        let begins = mpeg1_layer3_main_data_begins(&perceptual);
        assert_eq!(perceptual_details.len(), frames * detail_width);
        assert_eq!(perceptual_details[0], 0.0);
        assert_eq!(perceptual_details[6], 0.0);
        assert!(perceptual_details
            .chunks_exact(detail_width)
            .any(|detail| detail[6] > 0.0));
        assert!(perceptual_details.chunks_exact(detail_width).all(|detail| {
            detail[2] <= (detail[5] + detail[6]) * 8.0
                && detail[7] >= 0.0
                && detail[8] == 2.0
                && detail[9] == 0.0
                && detail[10] == 0.0
                && detail[11] == 0.0
        }));
        assert_eq!(begins.len() * detail_width, perceptual_details.len());
        for (frame, main_data_begin) in begins.iter().enumerate() {
            assert_eq!(
                *main_data_begin,
                perceptual_details[frame * detail_width + 6]
            );
        }
        let entropy_targeted_detail_width = 14_usize;
        let entropy_targeted_details =
            mp3_entropy_targeted_perceptual_reservoir_frame_details_with_bitrate(
                44_100, 1, &samples, 128, false, 0,
            )
            .unwrap();
        assert_eq!(
            entropy_targeted_details.len(),
            frames * entropy_targeted_detail_width
        );
        assert_eq!(
            entropy_targeted_details
                .chunks_exact(entropy_targeted_detail_width)
                .map(|detail| detail[12])
                .sum::<f64>(),
            perceptual_details
                .chunks_exact(detail_width)
                .map(|detail| detail[5] * 8.0)
                .sum::<f64>()
        );
        assert!(entropy_targeted_details
            .chunks_exact(entropy_targeted_detail_width)
            .any(|detail| detail[13] == 1.0));
        let entropy_profile =
            mp3_entropy_targeted_perceptual_reservoir_utilization_profile_with_bitrate(
                44_100, 1, &samples, 128, false, 0,
            )
            .unwrap();
        let used_entropy_frames = entropy_targeted_details
            .chunks_exact(entropy_targeted_detail_width)
            .filter(|detail| detail[13] == 1.0)
            .count();
        assert_eq!(entropy_profile.len(), 6);
        assert_eq!(entropy_profile[0], frames as f64);
        assert_eq!(entropy_profile[1], used_entropy_frames as f64);
        assert!(entropy_profile[2] > 0.0);
        assert!(entropy_profile[3] >= entropy_profile[2]);
        assert!(entropy_profile[4] > 0.0 && entropy_profile[4] <= 1.0);
        assert!(entropy_profile[5] >= 0.0);
        let entropy_targeted = encode_mp3_entropy_targeted_perceptual_reservoir_with_bitrate(
            44_100, 1, &samples, 128, false, 0,
        )
        .unwrap();
        let entropy_targeted_begins = mpeg1_layer3_main_data_begins(&entropy_targeted);
        assert_eq!(
            entropy_targeted_begins.len() * entropy_targeted_detail_width,
            entropy_targeted_details.len()
        );
        for (frame, main_data_begin) in entropy_targeted_begins.iter().enumerate() {
            assert_eq!(
                *main_data_begin,
                entropy_targeted_details[frame * entropy_targeted_detail_width + 6]
            );
        }

        let guarded_details = mp3_quality_guarded_perceptual_reservoir_frame_details_with_bitrate(
            44_100, 1, &samples, 128, false,
        )
        .unwrap();
        let guarded = encode_mp3_quality_guarded_perceptual_reservoir_with_bitrate(
            44_100, 1, &samples, 128, false,
        )
        .unwrap();
        let guarded_begins = mpeg1_layer3_main_data_begins(&guarded);
        assert_eq!(guarded_details.len(), frames * detail_width);
        assert_eq!(guarded_details[0], 0.0);
        assert_eq!(guarded_details[6], 0.0);
        assert!(guarded_details
            .chunks_exact(detail_width)
            .any(|detail| detail[6] > 0.0));
        assert!(guarded_details.chunks_exact(detail_width).all(|detail| {
            detail[2] <= (detail[5] + detail[6]) * 8.0
                && detail[7] >= 0.0
                && detail[8] + detail[9] == 2.0
                && detail[10] == 2.0
                && detail[11].is_finite()
        }));
        assert_eq!(guarded_begins.len() * detail_width, guarded_details.len());
        for (frame, main_data_begin) in guarded_begins.iter().enumerate() {
            assert_eq!(*main_data_begin, guarded_details[frame * detail_width + 6]);
        }
    }

    #[test]
    fn unified_m4a_api_muxes_silent_aac() {
        let samples = vec![0.0; 1024];

        let encoded = encode_audio("m4a", 44_100, 1, &samples).unwrap();
        let decoded = decode_audio(&encoded).unwrap();

        assert_eq!(detect_format(&encoded), Some("m4a".to_owned()));
        assert!(encoded.windows(4).any(|window| window == b"ftyp"));
        assert_eq!(decode_m4a(&encoded).unwrap().samples().len(), samples.len());
        assert_eq!(encode_m4a(44_100, 1, &samples).unwrap(), encoded);
        assert_eq!(
            demux_m4a_as_aac_adts(&encoded).unwrap(),
            encode_aac(44_100, 1, &samples).unwrap()
        );
        assert_eq!(decoded.sample_rate(), 44_100);
        assert_eq!(decoded.channels(), 1);
        assert_eq!(decoded.samples().len(), samples.len());
    }

    #[test]
    fn exposes_lossy_budget_helpers() {
        fn max_adts_frame_len(stream: &[u8]) -> usize {
            let mut max_len = 0;
            let mut offset = 0;
            while offset + 7 <= stream.len() {
                let frame_len = (((stream[offset + 3] & 0x03) as usize) << 11)
                    | ((stream[offset + 4] as usize) << 3)
                    | ((stream[offset + 5] as usize) >> 5);
                max_len = max_len.max(frame_len);
                offset += frame_len;
            }
            assert_eq!(offset, stream.len());
            max_len
        }

        assert_eq!(
            aac_lc_adts_max_frame_len_for_bitrate(44_100, 10_000).unwrap(),
            30
        );
        assert_eq!(aac_lc_default_production_bitrate_bps(1).unwrap(), 128_000);
        assert_eq!(aac_lc_default_production_bitrate_bps(2).unwrap(), 256_000);
        assert!(aac_lc_default_production_bitrate_bps(3).is_err());
        assert!(aac_lc_adts_max_frame_len_for_bitrate(44_100, 1).is_err());
        let production_steps = aac_lc_pcm_step_candidates();
        let standard_id_steps = aac_standard_id_pcm_step_candidates();
        assert!(production_steps.contains(&0.2));
        assert!(!production_steps.contains(&0.15));
        assert!(standard_id_steps.contains(&0.075));
        assert!(standard_id_steps.contains(&0.15));
        assert!(standard_id_steps.len() > production_steps.len());
        assert_eq!(
            aac_standard_id_selected_scale_factor_parameters(1).unwrap(),
            vec![128.0, 16.0]
        );
        assert_eq!(
            aac_standard_id_selected_scale_factor_parameters(2).unwrap(),
            vec![126.0, 16.0]
        );
        assert!(aac_standard_id_selected_scale_factor_parameters(3).is_err());
        assert_eq!(
            aac_standard_id_selected_scale_factor_balanced_parameters(1).unwrap(),
            vec![136.0, 8.0, 2047.0]
        );
        assert_eq!(
            aac_standard_id_selected_scale_factor_balanced_parameters(2).unwrap(),
            vec![138.0, 4.0, 1535.0]
        );
        assert!(aac_standard_id_selected_scale_factor_balanced_parameters(3).is_err());
        assert_eq!(
            aac_standard_id_selected_scale_factor_balanced_gain_deltas(1).unwrap(),
            vec![0.0, 2.0, 4.0, 6.0, 8.0]
        );
        assert_eq!(
            aac_standard_id_selected_scale_factor_balanced_gain_deltas(2).unwrap(),
            vec![8.0, 12.0, 16.0]
        );
        assert_eq!(
            aac_standard_id_selected_scale_factor_balanced_magnitude_biases(1).unwrap(),
            vec![8.0, 12.0, 16.0, 20.0]
        );
        assert_eq!(
            aac_standard_id_selected_scale_factor_balanced_magnitude_biases(2).unwrap(),
            vec![4.0, 8.0, 12.0]
        );
        assert!(aac_standard_id_selected_scale_factor_balanced_gain_deltas(3).is_err());
        assert!(aac_standard_id_selected_scale_factor_balanced_magnitude_biases(3).is_err());
        assert_eq!(
            aac_standard_id_selected_scale_factor_global_gain(1).unwrap(),
            128
        );
        assert_eq!(
            aac_standard_id_selected_scale_factor_global_gain(2).unwrap(),
            126
        );
        assert!(aac_standard_id_selected_scale_factor_global_gain(3).is_err());
        assert_eq!(aac_standard_id_selected_scale_factor_magnitude_bias(), 16);
        let aac_samples = (0..2048)
            .map(|sample| ((sample as f32) * 0.01).sin() * 0.25)
            .collect::<Vec<_>>();
        let aac_10k = encode_aac_with_bitrate(44_100, 1, &aac_samples, 10_000).unwrap();
        let selected_aac_10k =
            encode_aac_with_selected_scale_factors_and_bitrate(44_100, 1, &aac_samples, 10_000)
                .unwrap();
        let m4a_10k = encode_m4a_with_bitrate(44_100, 1, &aac_samples, 10_000).unwrap();
        let selected_m4a_10k =
            encode_m4a_with_selected_scale_factors_and_bitrate(44_100, 1, &aac_samples, 10_000)
                .unwrap();
        assert_eq!(&aac_10k[..2], &[0xff, 0xf1]);
        assert_eq!(&selected_aac_10k[..2], &[0xff, 0xf1]);
        assert_eq!(detect_format(&m4a_10k), Some("m4a".to_owned()));
        assert_eq!(detect_format(&selected_m4a_10k), Some("m4a".to_owned()));
        assert_eq!(demux_m4a_as_aac_adts(&m4a_10k).unwrap(), aac_10k);
        assert_eq!(
            demux_m4a_as_aac_adts(&selected_m4a_10k).unwrap(),
            selected_aac_10k
        );
        assert!(max_adts_frame_len(&aac_10k) <= 30);
        assert!(max_adts_frame_len(&selected_aac_10k) <= 30);
        assert!(encode_aac_with_bitrate(44_100, 1, &aac_samples, 1).is_err());
        assert!(
            encode_aac_with_selected_scale_factors_and_bitrate(44_100, 1, &aac_samples, 1).is_err()
        );
        assert_eq!(
            aac_unsigned_pairs7_unit_magnitude_table(),
            vec![0, 0, 0, 1, 0, 1, 0b101, 3, 1, 0, 0b100, 3, 1, 1, 0b1100, 4]
        );
        let pairs7 = aac_unsigned_pairs7_table();
        assert_eq!(pairs7.len(), 256);
        assert_eq!(&pairs7[..4], &[0, 0, 0, 1]);
        assert_eq!(&pairs7[36..40], &[1, 1, 0x00c, 4]);
        assert_eq!(&pairs7[252..256], &[7, 7, 0xfff, 12]);
        let signed5 = aac_signed_pairs5_table();
        assert_eq!(signed5.len(), 324);
        assert_eq!(&signed5[..4], &[-4, -4, 0x1fff, 13]);
        assert_eq!(&signed5[160..164], &[0, 0, 0, 1]);
        assert_eq!(&signed5[320..324], &[4, 4, 0x1ffe, 13]);
        let signed6 = aac_signed_pairs6_table();
        assert_eq!(signed6.len(), 324);
        assert_eq!(&signed6[..4], &[-4, -4, 0x7fe, 11]);
        assert_eq!(&signed6[160..164], &[0, 0, 0, 4]);
        assert_eq!(&signed6[320..324], &[4, 4, 0x7fc, 11]);
        let signed_quads1 = aac_signed_quads1_table();
        assert_eq!(signed_quads1.len(), 486);
        assert_eq!(&signed_quads1[..6], &[-1, -1, -1, -1, 0x7f8, 11]);
        assert_eq!(&signed_quads1[240..246], &[0, 0, 0, 0, 0, 1]);
        assert_eq!(&signed_quads1[480..486], &[1, 1, 1, 1, 0x7f4, 11]);
        let signed_quads2 = aac_signed_quads2_table();
        assert_eq!(signed_quads2.len(), 486);
        assert_eq!(&signed_quads2[..6], &[-1, -1, -1, -1, 0x1f3, 9]);
        assert_eq!(&signed_quads2[240..246], &[0, 0, 0, 0, 0, 3]);
        assert_eq!(&signed_quads2[480..486], &[1, 1, 1, 1, 0x1f6, 9]);
        let quads3 = aac_unsigned_quads3_table();
        assert_eq!(quads3.len(), 486);
        assert_eq!(&quads3[..6], &[0, 0, 0, 0, 0, 1]);
        assert_eq!(&quads3[240..246], &[1, 1, 1, 1, 0x74, 7]);
        assert_eq!(&quads3[480..486], &[2, 2, 2, 2, 0x7ffa, 15]);
        let quads4 = aac_unsigned_quads4_table();
        assert_eq!(quads4.len(), 486);
        assert_eq!(&quads4[..6], &[0, 0, 0, 0, 0x7, 4]);
        assert_eq!(&quads4[240..246], &[1, 1, 1, 1, 0, 4]);
        assert_eq!(&quads4[480..486], &[2, 2, 2, 2, 0x7fc, 11]);
        let pairs8 = aac_unsigned_pairs8_table();
        assert_eq!(pairs8.len(), 256);
        assert_eq!(&pairs8[..4], &[0, 0, 0x00e, 5]);
        assert_eq!(&pairs8[36..40], &[1, 1, 0, 3]);
        assert_eq!(&pairs8[252..256], &[7, 7, 0x3ff, 10]);
        let pairs9 = aac_unsigned_pairs9_table();
        assert_eq!(pairs9.len(), 676);
        assert_eq!(&pairs9[..4], &[0, 0, 0, 1]);
        assert_eq!(&pairs9[56..60], &[1, 1, 0x000c, 4]);
        assert_eq!(&pairs9[672..676], &[12, 12, 0x7fff, 15]);
        let pairs10 = aac_unsigned_pairs10_table();
        assert_eq!(pairs10.len(), 676);
        assert_eq!(&pairs10[..4], &[0, 0, 0x022, 6]);
        assert_eq!(&pairs10[56..60], &[1, 1, 0, 4]);
        assert_eq!(&pairs10[672..676], &[12, 12, 0xfff, 12]);
        let escape = aac_escape_table();
        assert_eq!(escape.len(), 1156);
        assert_eq!(&escape[..4], &[0, 0, 0, 4]);
        assert_eq!(&escape[72..76], &[1, 1, 1, 4]);
        assert_eq!(&escape[1152..1156], &[16, 16, 4, 5]);
        let scale_factor_table = aac_scale_factor_delta_table();
        assert_eq!(scale_factor_table.len(), 363);
        assert_eq!(&scale_factor_table[..3], &[-60, 0x3FFE8, 18]);
        assert_eq!(&scale_factor_table[180..183], &[0, 0, 1]);
        assert_eq!(&scale_factor_table[360..363], &[60, 0x7FFF3, 19]);
        assert_eq!(
            aac_codebook6_unit_section_plan(&[1, -1, 0, 0], 2).unwrap(),
            vec![0, 2, 6, 2, 4, 0]
        );
        assert_eq!(
            aac_quad_unit_section_plan(&[1, -1, 0, 1, 0, 1, -1, 0, 0, 0, 0, 0], 4).unwrap(),
            vec![0, 8, 3, 8, 12, 0]
        );
        assert_eq!(
            aac_mixed_unit_section_plan(&[1, -1, 0, 1, 1, -1, 1, -1, 0, 0, 0, 0], 4).unwrap(),
            vec![0, 4, 3, 4, 8, 6, 8, 12, 0]
        );
        assert_eq!(
            aac_mixed_unit_payload_bit_lengths(&[1, -1, 0, 1, 1, -1, 1, -1, 0, 0, 0, 0], 4)
                .unwrap(),
            vec![27, 11, 38, 29, 11, 40]
        );
        assert_eq!(
            aac_standard_unit_section_plan(&[1, -1, 17, 0], 2).unwrap(),
            vec![0, 2, 6, 2, 4, 11]
        );
        assert_eq!(
            aac_standard_unit_section_plan(&[0, 1], 2).unwrap(),
            vec![0, 2, 5]
        );
        assert_eq!(
            aac_standard_unit_section_plan(&[1, -1, 0, 1, 17, 0, 0, 0], 4).unwrap(),
            vec![0, 4, 4, 4, 8, 11]
        );
        assert_eq!(
            aac_standard_offsets_section_plan(&[1, -1, 0, 1, 17, 0, 0, 0], &[0, 4, 8]).unwrap(),
            vec![0, 4, 4, 4, 8, 11]
        );
        assert_eq!(
            aac_standard_escape_payload_bit_lengths().unwrap(),
            vec![9, 15, 24]
        );
        assert_eq!(
            aac_standard_mixed_payload_bit_lengths(&[1, -1, 0, 1, 17, 0, 0, 0], 4).unwrap(),
            vec![18, 26, 44, 20, 26, 46]
        );
        assert_eq!(
            aac_standard_mixed_offsets_payload_bit_lengths(&[1, -1, 0, 1, 17, 0, 0, 0], &[0, 4, 8])
                .unwrap(),
            vec![18, 26, 44, 20, 26, 46]
        );
        let standard_mono =
            encode_aac_standard_mono_offsets_with_step(44_100, &[0.0; 2048], 20.0, 128).unwrap();
        assert_eq!(&standard_mono[..2], &[0xff, 0xf1]);
        assert!(max_adts_frame_len(&standard_mono) <= 16);
        let standard_mono_bitrate =
            encode_aac_standard_mono_offsets_with_bitrate(44_100, &[0.0; 2048], 128_000, 128)
                .unwrap();
        assert_eq!(&standard_mono_bitrate[..2], &[0xff, 0xf1]);
        assert!(
            max_adts_frame_len(&standard_mono_bitrate)
                <= aac_lc_adts_max_frame_len_for_bitrate(44_100, 128_000).unwrap()
        );
        let standard_generic_adts = encode_aac_with_standard_spectral_offsets_and_bitrate(
            44_100,
            1,
            &[0.0; 2048],
            128_000,
            128,
        )
        .unwrap();
        assert_eq!(&standard_generic_adts[..2], &[0xff, 0xf1]);
        assert!(
            max_adts_frame_len(&standard_generic_adts)
                <= aac_lc_adts_max_frame_len_for_bitrate(44_100, 128_000).unwrap()
        );
        let standard_generic_m4a = encode_m4a_with_standard_spectral_offsets_and_bitrate(
            44_100,
            1,
            &[0.0; 2048],
            128_000,
            128,
        )
        .unwrap();
        assert_eq!(&standard_generic_m4a[4..8], b"ftyp");
        let standard_mono_details =
            aac_standard_mono_offsets_bitrate_frame_details(44_100, &[0.0; 2048], 128_000, 128)
                .unwrap();
        assert_eq!(standard_mono_details.len(), 8);
        assert_eq!(standard_mono_details[0], 0.0);
        assert_eq!(standard_mono_details[4], 1.0);
        assert!(standard_mono_details[2] <= 372.0);
        assert!(standard_mono_details[6] <= 372.0);
        let standard_selected_details =
            aac_standard_selected_scale_factor_frame_details_with_magnitude_bias_and_bitrate(
                44_100,
                1,
                &[0.0; 2048],
                128_000,
                128,
                16,
            )
            .unwrap();
        assert_eq!(standard_selected_details.len(), 8);
        assert_eq!(standard_selected_details[0], 0.0);
        assert_eq!(standard_selected_details[4], 1.0);
        assert!(standard_selected_details[2] <= 372.0);
        assert!(standard_selected_details[6] <= 372.0);
        let recommended_standard_selected_adts =
            encode_aac_with_recommended_standard_spectral_offsets_and_selected_scale_factors_and_bitrate(
                44_100,
                1,
                &[0.0; 2048],
                128_000,
            )
            .unwrap();
        let explicit_standard_selected_adts =
            encode_aac_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_bitrate(
                44_100,
                1,
                &[0.0; 2048],
                128_000,
                128,
                16,
            )
            .unwrap();
        assert_eq!(
            recommended_standard_selected_adts,
            explicit_standard_selected_adts
        );
        let recommended_standard_selected_m4a =
            encode_m4a_with_recommended_standard_spectral_offsets_and_selected_scale_factors_and_bitrate(
                44_100,
                1,
                &[0.0; 2048],
                128_000,
            )
            .unwrap();
        assert_eq!(&recommended_standard_selected_m4a[4..8], b"ftyp");
        let recommended_standard_selected_details =
            aac_recommended_standard_selected_scale_factor_frame_details_with_bitrate(
                44_100,
                1,
                &[0.0; 2048],
                128_000,
            )
            .unwrap();
        assert_eq!(
            recommended_standard_selected_details,
            standard_selected_details
        );
        let standard_selected_profile =
            aac_standard_selected_scale_factor_profile_with_magnitude_bias_and_bitrate(
                44_100,
                1,
                &[0.0; 2048],
                128_000,
                128,
                16,
            )
            .unwrap();
        assert_eq!(
            standard_selected_profile,
            vec![2.0, 1.0, 98.0, 0.0, 0.0, 0.0]
        );
        let recommended_standard_selected_profile =
            aac_recommended_standard_selected_scale_factor_profile_with_bitrate(
                44_100,
                1,
                &[0.0; 2048],
                128_000,
            )
            .unwrap();
        assert_eq!(
            recommended_standard_selected_profile,
            standard_selected_profile
        );
        let balanced_standard_selected_profile =
            aac_balanced_standard_selected_scale_factor_profile_with_bitrate(
                44_100,
                1,
                &[0.0; 2048],
                128_000,
            )
            .unwrap();
        assert_eq!(
            balanced_standard_selected_profile,
            standard_selected_profile
        );
        let standard_payload_breakdown =
            aac_standard_id_payload_breakdown_with_magnitude_bias_and_bitrate(
                44_100,
                1,
                &[0.0; 2048],
                128_000,
                128,
                16,
            )
            .unwrap();
        assert_eq!(standard_payload_breakdown.len(), 11);
        assert_eq!(standard_payload_breakdown[0], 2.0);
        assert_eq!(standard_payload_breakdown[1], 1.0);
        assert_eq!(standard_payload_breakdown[3], 0.0);
        assert_eq!(standard_payload_breakdown[4], 0.0);
        assert_eq!(standard_payload_breakdown[8], 0.0);
        assert_eq!(standard_payload_breakdown[10], 0.0);
        assert_eq!(
            aac_recommended_standard_id_payload_breakdown_with_bitrate(
                44_100,
                1,
                &[0.0; 2048],
                128_000,
            )
            .unwrap(),
            standard_payload_breakdown
        );
        assert_eq!(
            aac_balanced_standard_id_payload_breakdown_with_bitrate(
                44_100,
                1,
                &[0.0; 2048],
                128_000,
            )
            .unwrap(),
            standard_payload_breakdown
        );
        let explicit_balanced_quality_profile =
            aac_standard_id_quality_control_profile_with_magnitude_bias_max_quantized_abs_and_bitrate(
                44_100,
                1,
                &[0.0; 2048],
                128_000,
                136,
                8,
                2047,
            )
            .unwrap();
        let balanced_quality_profile =
            aac_balanced_standard_id_quality_control_profile_with_bitrate(
                44_100,
                1,
                &[0.0; 2048],
                128_000,
            )
            .unwrap();
        assert_eq!(balanced_quality_profile.len(), 16);
        assert_eq!(balanced_quality_profile, explicit_balanced_quality_profile);
        assert_eq!(balanced_quality_profile[0], 2.0);
        assert_eq!(balanced_quality_profile[1], 1.0);
        assert!(balanced_quality_profile[3] >= 0.0);
        assert_eq!(balanced_quality_profile[4], 2047.0);
        assert_eq!(balanced_quality_profile[5], 0.0);
        assert_eq!(balanced_quality_profile[10], 0.0);
        assert_eq!(balanced_quality_profile[13], 0.0);
        let balanced_quality_candidates =
            aac_standard_id_quality_control_candidates_for_balance_profile_with_bitrate(
                44_100,
                1,
                &[0.0; 2048],
                128_000,
            )
            .unwrap();
        assert!(!balanced_quality_candidates.is_empty());
        assert_eq!(balanced_quality_candidates.len() % 19, 0);
        assert!(balanced_quality_candidates
            .chunks_exact(19)
            .any(|candidate| candidate[0] == 136.0
                && candidate[1] == 8.0
                && candidate[2] == 2047.0));
        let production_selected_details =
            aac_selected_scale_factor_frame_details_with_bitrate(44_100, 1, &[0.0; 2048], 128_000)
                .unwrap();
        assert_eq!(production_selected_details.len(), 8);
        assert_eq!(production_selected_details[0], 0.0);
        assert_eq!(production_selected_details[4], 1.0);
        assert!(production_selected_details[2] <= 372.0);
        assert!(production_selected_details[6] <= 372.0);
        let standard_stereo =
            encode_aac_standard_stereo_offsets_with_step(44_100, &[0.0; 4096], 20.0, 128).unwrap();
        assert_eq!(&standard_stereo[..2], &[0xff, 0xf1]);
        assert!(max_adts_frame_len(&standard_stereo) <= 28);
        let standard_stereo_bitrate =
            encode_aac_standard_stereo_offsets_with_bitrate(44_100, &[0.0; 4096], 256_000, 128)
                .unwrap();
        assert_eq!(&standard_stereo_bitrate[..2], &[0xff, 0xf1]);
        assert!(
            max_adts_frame_len(&standard_stereo_bitrate)
                <= aac_lc_adts_max_frame_len_for_bitrate(44_100, 256_000).unwrap()
        );
        let standard_stereo_details =
            aac_standard_stereo_offsets_bitrate_frame_details(44_100, &[0.0; 4096], 256_000, 128)
                .unwrap();
        assert_eq!(standard_stereo_details.len(), 8);
        assert_eq!(standard_stereo_details[0], 0.0);
        assert_eq!(standard_stereo_details[4], 1.0);
        assert!(standard_stereo_details[2] <= 744.0);
        assert!(standard_stereo_details[6] <= 744.0);
        let standard_selected_stereo_details =
            aac_standard_selected_scale_factor_frame_details_with_magnitude_bias_and_bitrate(
                44_100,
                2,
                &[0.0; 4096],
                256_000,
                128,
                16,
            )
            .unwrap();
        assert_eq!(standard_selected_stereo_details.len(), 8);
        assert_eq!(standard_selected_stereo_details[0], 0.0);
        assert_eq!(standard_selected_stereo_details[4], 1.0);
        assert!(standard_selected_stereo_details[2] <= 744.0);
        assert!(standard_selected_stereo_details[6] <= 744.0);
        let recommended_standard_selected_stereo_details =
            aac_recommended_standard_selected_scale_factor_frame_details_with_bitrate(
                44_100,
                2,
                &[0.0; 4096],
                256_000,
            )
            .unwrap();
        let explicit_recommended_standard_selected_stereo_details =
            aac_standard_selected_scale_factor_frame_details_with_magnitude_bias_and_bitrate(
                44_100,
                2,
                &[0.0; 4096],
                256_000,
                126,
                16,
            )
            .unwrap();
        assert_eq!(
            recommended_standard_selected_stereo_details,
            explicit_recommended_standard_selected_stereo_details
        );
        let production_selected_stereo_details =
            aac_selected_scale_factor_frame_details_with_bitrate(44_100, 2, &[0.0; 4096], 256_000)
                .unwrap();
        assert_eq!(production_selected_stereo_details.len(), 8);
        assert_eq!(production_selected_stereo_details[0], 0.0);
        assert_eq!(production_selected_stereo_details[4], 1.0);
        assert!(production_selected_stereo_details[2] <= 744.0);
        assert!(production_selected_stereo_details[6] <= 744.0);
        assert_eq!(
            mp3_layer3_main_data_capacity_bytes(44_100, 1, 128, false, false).unwrap(),
            396
        );
        assert_eq!(
            mp3_layer3_main_data_capacity_bits(44_100, 1, 128, false, false).unwrap(),
            3168
        );
        let mp3_steps = mp3_pcm_step_candidates();
        assert!(mp3_steps.contains(&0.2));
        assert!(!mp3_steps.contains(&0.15));
        let mp3_mono_production_steps = mp3_production_pcm_step_candidates(1).unwrap();
        assert_eq!(mp3_mono_production_steps[0], 2.0);
        assert!(!mp3_mono_production_steps.contains(&0.2));
        assert_eq!(mp3_production_pcm_step_candidates(2).unwrap(), mp3_steps);
        assert!(mp3_production_pcm_step_candidates(3).is_err());
        assert_eq!(
            mp3_standard_big_value_table_selects(),
            vec![
                1, 2, 3, 5, 6, 7, 8, 9, 10, 11, 12, 13, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
                26, 27, 28, 29, 30, 31
            ]
        );
        assert_eq!(
            mp3_missing_standard_big_value_table_selects(),
            Vec::<u8>::new()
        );
        assert_eq!(mp3_standard_count1_table_selects(), vec![0, 1]);
        assert!(mp3_layer3_main_data_capacity_bytes(44_100, 3, 128, false, false).is_err());
        let mp3_samples = (0..(1152 * 3))
            .map(|sample| ((sample as f32) * 0.013).sin() * 0.25)
            .collect::<Vec<_>>();
        let mp3_candidate_profile = mp3_first_frame_perceptual_candidate_profile_with_bitrate(
            44_100,
            1,
            &mp3_samples,
            128,
            false,
        )
        .unwrap();
        assert_eq!(mp3_candidate_profile.len(), mp3_steps.len() * 6);
        assert_eq!(mp3_candidate_profile[0], f64::from(mp3_steps[0]));
        assert_eq!(mp3_candidate_profile[4], 42.0);
        assert!(mp3_candidate_profile
            .chunks_exact(6)
            .any(|candidate| { candidate[3] > 0.0 && candidate[5] > 0.0 }));
        let mp3_low_band_profile =
            mp3_first_frame_low_band_spectral_shape_candidate_profile_with_bitrate(
                44_100,
                1,
                &mp3_samples,
                128,
                false,
            )
            .unwrap();
        assert_eq!(mp3_low_band_profile.len(), mp3_steps.len() * 7);
        assert_eq!(mp3_low_band_profile[0], f64::from(mp3_steps[0]));
        assert!(mp3_low_band_profile.chunks_exact(7).any(|candidate| {
            candidate[3] > 0.0
                && candidate[3] <= candidate[4]
                && candidate[5] > 0.0
                && candidate[5] <= candidate[6]
        }));
        let mp3_band_shape_profile =
            mp3_first_frame_band_spectral_shape_candidate_profile_with_bitrate(
                44_100,
                1,
                &mp3_samples,
                128,
                false,
            )
            .unwrap();
        assert_eq!(mp3_band_shape_profile.len(), mp3_steps.len() * 21 * 10);
        assert_eq!(mp3_band_shape_profile[0], f64::from(mp3_steps[0]));
        assert!(mp3_band_shape_profile.chunks_exact(10).any(|candidate| {
            candidate[3] >= 0.0
                && candidate[3] < 21.0
                && candidate[4] <= candidate[5]
                && candidate[6] > 0.0
                && candidate[6] <= candidate[8]
                && candidate[7] <= candidate[9]
        }));
        let band_biased_mp3 = encode_mp3_perceptual_scale_factor_band_bias(
            44_100,
            1,
            &mp3_samples[..1152],
            0.2,
            0,
            7,
            2,
        )
        .unwrap();
        let band_gain_mp3 = encode_mp3_perceptual_quantized_band_gain(
            44_100,
            1,
            &mp3_samples[..1152],
            0.2,
            0,
            7,
            1.5,
        )
        .unwrap();
        let band_gain_matched_mp3 = encode_mp3_perceptual_quantized_band_gain_global_gain_bias(
            44_100,
            1,
            &mp3_samples[..1152],
            2.0,
            0,
            7,
            1.5,
            -4,
        )
        .unwrap();
        assert_eq!(
            sonare_codec::detect(&band_biased_mp3),
            Some(sonare_codec::Format::Mp3)
        );
        assert_eq!(
            sonare_codec::detect(&band_gain_mp3),
            Some(sonare_codec::Format::Mp3)
        );
        assert_eq!(
            sonare_codec::detect(&band_gain_matched_mp3),
            Some(sonare_codec::Format::Mp3)
        );
        let mp3_guarded_profile = mp3_first_frame_quality_guarded_candidate_profile_with_bitrate(
            44_100,
            1,
            &mp3_samples,
            128,
            false,
        )
        .unwrap();
        assert_eq!(mp3_guarded_profile.len(), mp3_steps.len() * 7);
        assert_eq!(mp3_guarded_profile[0], f64::from(mp3_steps[0]));
        assert!(mp3_guarded_profile
            .chunks_exact(7)
            .any(|candidate| candidate[3] > 0.0 && candidate[5] > 0.0));
        let mp3_bit_allocation =
            mp3_perceptual_bit_allocation_with_bitrate(44_100, 1, &mp3_samples, 128, false, 0)
                .unwrap();
        assert_eq!(mp3_bit_allocation.len(), 30);
        assert_eq!(
            mp3_bit_allocation
                .chunks_exact(5)
                .map(|allocation| allocation[4])
                .sum::<f64>(),
            9520.0
        );
        assert!(mp3_bit_allocation
            .chunks_exact(5)
            .all(|allocation| allocation[3].is_finite()));
        let perceptual =
            encode_mp3_perceptual_active_cbr_with_bitrate(44_100, 1, &mp3_samples, 128, false)
                .unwrap();
        assert_mpeg1_layer3_frame_budget(&perceptual, 44_100, 1, 128);
    }

    #[test]
    fn unified_encode_rejects_unknown_format() {
        let err = encode_audio("unknown", 44_100, 1, &[0.0]).unwrap_err();

        assert_eq!(err, "unsupported format");
    }

    #[test]
    fn unified_encode_produces_ogg_streams_for_lossy_encoders() {
        let samples = vec![0.0; 128];

        // Opus and Vorbis are both pure-Rust encoders that compile to wasm, so
        // the wasm surface enables their features and the unified entry point
        // produces real Ogg streams rather than reporting them as unsupported.
        let opus = encode_audio("opus", 48_000, 1, &samples).expect("opus encode");
        assert_eq!(&opus[..4], b"OggS");

        let vorbis = encode_audio("vorbis", 48_000, 1, &samples).expect("vorbis encode");
        assert_eq!(&vorbis[..4], b"OggS");
    }

    #[test]
    fn dedicated_lossy_encoders_produce_ogg_streams() {
        let samples = vec![0.0; 128];

        let vorbis = encode_vorbis(48_000, 1, &samples).expect("encode_vorbis");
        assert_eq!(&vorbis[..4], b"OggS");
        assert_eq!(
            sonare_codec::detect(&vorbis),
            Some(sonare_codec::Format::Vorbis)
        );

        let opus = encode_opus(48_000, 1, &samples).expect("encode_opus");
        assert_eq!(&opus[..4], b"OggS");
        assert_eq!(
            sonare_codec::detect(&opus),
            Some(sonare_codec::Format::Opus)
        );
    }
}
