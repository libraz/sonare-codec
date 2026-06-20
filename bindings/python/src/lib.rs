#![deny(unsafe_code)]
#![warn(clippy::all)]

use pyo3::prelude::*;

#[pyclass]
struct StreamDecoder {
    inner: sonare_codec_rs::StreamDecoder,
}

#[pymethods]
impl StreamDecoder {
    #[new]
    fn new() -> Self {
        Self {
            inner: sonare_codec_rs::StreamDecoder::new(),
        }
    }

    fn decode_stream(&mut self, chunk: &[u8]) -> PyResult<Option<(u32, u16, Vec<f32>)>> {
        self.inner
            .decode_stream(chunk)
            .map(|decoded| decoded.map(pcm_tuple))
            .map_err(to_py_value_error)
    }

    fn reset(&mut self) {
        self.inner.reset();
    }

    fn buffered_len(&self) -> usize {
        self.inner.buffered_len()
    }
}

#[pyfunction]
fn detect_format(input: &[u8]) -> Option<String> {
    if is_m4a_container(input) {
        return Some("m4a".to_owned());
    }
    sonare_codec_rs::detect(input).map(|format| format!("{format:?}").to_ascii_lowercase())
}

#[pyfunction]
fn decode_audio(input: &[u8]) -> PyResult<(u32, u16, Vec<f32>)> {
    let pcm = sonare_codec_rs::decode(input).map_err(to_py_value_error)?;
    Ok(pcm_tuple(pcm))
}

#[pyfunction]
fn decode_wav(input: &[u8]) -> PyResult<(u32, u16, Vec<f32>)> {
    let pcm = sonare_codec_rs::decode_wav(input).map_err(to_py_value_error)?;
    Ok(pcm_tuple(pcm))
}

#[pyfunction]
fn decode_flac(input: &[u8]) -> PyResult<(u32, u16, Vec<f32>)> {
    let pcm = sonare_codec_rs::decode_flac(input).map_err(to_py_value_error)?;
    Ok(pcm_tuple(pcm))
}

#[pyfunction]
fn decode_mp3(input: &[u8]) -> PyResult<(u32, u16, Vec<f32>)> {
    let pcm = sonare_codec_rs::decode_mp3(input).map_err(to_py_value_error)?;
    Ok(pcm_tuple(pcm))
}

#[pyfunction]
fn decode_vorbis(input: &[u8]) -> PyResult<(u32, u16, Vec<f32>)> {
    let pcm = sonare_codec_rs::decode_vorbis(input).map_err(to_py_value_error)?;
    Ok(pcm_tuple(pcm))
}

#[pyfunction]
fn decode_opus(input: &[u8]) -> PyResult<(u32, u16, Vec<f32>)> {
    let pcm = sonare_codec_rs::decode_opus(input).map_err(to_py_value_error)?;
    Ok(pcm_tuple(pcm))
}

#[pyfunction]
fn decode_aac(input: &[u8]) -> PyResult<(u32, u16, Vec<f32>)> {
    let pcm = sonare_codec_rs::decode_aac(input).map_err(to_py_value_error)?;
    Ok(pcm_tuple(pcm))
}

#[pyfunction]
fn decode_m4a(input: &[u8]) -> PyResult<(u32, u16, Vec<f32>)> {
    let pcm = sonare_codec_rs::decode_aac(input).map_err(to_py_value_error)?;
    Ok(pcm_tuple(pcm))
}

#[pyfunction]
fn encode_audio(
    format: &str,
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
) -> PyResult<Vec<u8>> {
    let pcm = sonare_codec_rs::AudioBuffer::new(sample_rate, channels, samples)
        .map_err(to_py_value_error)?;
    encode_by_name(format, &pcm)
}

#[pyfunction]
fn encode_audio_production(
    format: &str,
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
) -> PyResult<Vec<u8>> {
    let pcm = sonare_codec_rs::AudioBuffer::new(sample_rate, channels, samples)
        .map_err(to_py_value_error)?;
    encode_by_name_with_mode(format, &pcm, sonare_codec_rs::EncodeMode::ProductionOnly)
}

#[pyfunction]
fn encode_wav(sample_rate: u32, channels: u16, samples: Vec<f32>) -> PyResult<Vec<u8>> {
    let pcm = sonare_codec_rs::AudioBuffer::new(sample_rate, channels, samples)
        .map_err(to_py_value_error)?;
    sonare_codec_rs::encode(sonare_codec_rs::Format::Wav, &pcm).map_err(to_py_value_error)
}

#[pyfunction]
fn encode_flac(sample_rate: u32, channels: u16, samples: Vec<f32>) -> PyResult<Vec<u8>> {
    let pcm = sonare_codec_rs::AudioBuffer::new(sample_rate, channels, samples)
        .map_err(to_py_value_error)?;
    sonare_codec_rs::encode(sonare_codec_rs::Format::Flac, &pcm).map_err(to_py_value_error)
}

#[pyfunction]
fn encode_mp3(sample_rate: u32, channels: u16, samples: Vec<f32>) -> PyResult<Vec<u8>> {
    let pcm = sonare_codec_rs::AudioBuffer::new(sample_rate, channels, samples)
        .map_err(to_py_value_error)?;
    sonare_codec_rs::encode(sonare_codec_rs::Format::Mp3, &pcm).map_err(to_py_value_error)
}

#[pyfunction]
fn encode_mp3_with_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    bitrate_kbps: u16,
    padding: bool,
    crc_protected: bool,
) -> PyResult<Vec<u8>> {
    let pcm = sonare_codec_rs::AudioBuffer::new(sample_rate, channels, samples)
        .map_err(to_py_value_error)?;
    sonare_codec_rs::encode_mpeg1_layer3_pcm_frames_with_bitrate_and_table_provider(
        &pcm,
        sonare_codec_rs::MPEG1_LAYER3_PCM_STEP_CANDIDATES,
        bitrate_kbps,
        padding,
        crc_protected,
        sonare_codec_rs::mpeg1_layer3_standard_table_provider(),
    )
    .map_err(to_py_value_error)
}

#[pyfunction]
fn encode_mp3_cbr_with_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    bitrate_kbps: u16,
    crc_protected: bool,
) -> PyResult<Vec<u8>> {
    let pcm = sonare_codec_rs::AudioBuffer::new(sample_rate, channels, samples)
        .map_err(to_py_value_error)?;
    sonare_codec_rs::encode_mpeg1_layer3_pcm_frames_with_cbr_bitrate_and_table_provider(
        &pcm,
        sonare_codec_rs::MPEG1_LAYER3_PCM_STEP_CANDIDATES,
        bitrate_kbps,
        crc_protected,
        sonare_codec_rs::mpeg1_layer3_standard_table_provider(),
    )
    .map_err(to_py_value_error)
}

#[pyfunction]
fn encode_mp3_perceptual_active_cbr_with_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    bitrate_kbps: u16,
    crc_protected: bool,
) -> PyResult<Vec<u8>> {
    let pcm = sonare_codec_rs::AudioBuffer::new(sample_rate, channels, samples)
        .map_err(to_py_value_error)?;
    sonare_codec_rs::encode_mpeg1_layer3_pcm_frames_with_perceptual_active_cbr_bitrate_and_table_provider(
        &pcm,
        sonare_codec_rs::MPEG1_LAYER3_PCM_STEP_CANDIDATES,
        bitrate_kbps,
        crc_protected,
        sonare_codec_rs::mpeg1_layer3_standard_table_provider(),
    )
    .map_err(to_py_value_error)
}

#[pyfunction]
fn encode_mp3_perceptual_reservoir_with_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    bitrate_kbps: u16,
    crc_protected: bool,
) -> PyResult<Vec<u8>> {
    let pcm = sonare_codec_rs::AudioBuffer::new(sample_rate, channels, samples)
        .map_err(to_py_value_error)?;
    sonare_codec_rs::encode_mpeg1_layer3_pcm_frames_with_perceptual_reservoir_and_table_provider(
        &pcm,
        sonare_codec_rs::MPEG1_LAYER3_PCM_STEP_CANDIDATES,
        bitrate_kbps,
        crc_protected,
        sonare_codec_rs::mpeg1_layer3_standard_table_provider(),
    )
    .map_err(to_py_value_error)
}

#[pyfunction]
fn encode_mp3_entropy_targeted_perceptual_reservoir_with_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    bitrate_kbps: u16,
    crc_protected: bool,
    min_bits_per_granule_channel: usize,
) -> PyResult<Vec<u8>> {
    let pcm = sonare_codec_rs::AudioBuffer::new(sample_rate, channels, samples)
        .map_err(to_py_value_error)?;
    let candidates = sonare_codec_rs::mpeg1_layer3_production_pcm_step_candidates(channels)
        .map_err(to_py_value_error)?;
    sonare_codec_rs::encode_mpeg1_layer3_pcm_frames_with_entropy_targeted_perceptual_reservoir_and_table_provider(
        &pcm,
        candidates,
        bitrate_kbps,
        crc_protected,
        min_bits_per_granule_channel,
        sonare_codec_rs::mpeg1_layer3_standard_table_provider(),
    )
    .map_err(to_py_value_error)
}

#[pyfunction]
fn encode_mp3_quality_guarded_perceptual_reservoir_with_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    bitrate_kbps: u16,
    crc_protected: bool,
) -> PyResult<Vec<u8>> {
    let pcm = sonare_codec_rs::AudioBuffer::new(sample_rate, channels, samples)
        .map_err(to_py_value_error)?;
    sonare_codec_rs::encode_mpeg1_layer3_pcm_frames_with_quality_guarded_perceptual_reservoir_and_table_provider(
        &pcm,
        sonare_codec_rs::MPEG1_LAYER3_PCM_STEP_CANDIDATES,
        bitrate_kbps,
        crc_protected,
        sonare_codec_rs::mpeg1_layer3_standard_table_provider(),
    )
    .map_err(to_py_value_error)
}

#[pyfunction]
fn encode_mp3_perceptual_scale_factor_band_bias(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    step: f32,
    band_start: usize,
    band_end: usize,
    bias: i8,
) -> PyResult<Vec<u8>> {
    let pcm = sonare_codec_rs::AudioBuffer::new(sample_rate, channels, samples)
        .map_err(to_py_value_error)?;
    sonare_codec_rs::encode_mpeg1_layer3_pcm_frames_with_perceptual_scale_factor_band_bias_and_table_provider(
        &pcm,
        step,
        sonare_codec_rs::Layer3ScaleFactorBandBias {
            band_start,
            band_end,
            bias,
        },
        sonare_codec_rs::mpeg1_layer3_standard_table_provider(),
    )
    .map_err(to_py_value_error)
}

#[pyfunction]
fn encode_mp3_perceptual_quantized_band_gain(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    step: f32,
    band_start: usize,
    band_end: usize,
    gain: f32,
) -> PyResult<Vec<u8>> {
    let pcm = sonare_codec_rs::AudioBuffer::new(sample_rate, channels, samples)
        .map_err(to_py_value_error)?;
    sonare_codec_rs::encode_mpeg1_layer3_pcm_frames_with_perceptual_quantized_band_gain_and_table_provider(
        &pcm,
        step,
        sonare_codec_rs::Layer3QuantizedBandGain {
            band_start,
            band_end,
            gain,
        },
        sonare_codec_rs::mpeg1_layer3_standard_table_provider(),
    )
    .map_err(to_py_value_error)
}

#[pyfunction]
#[allow(clippy::too_many_arguments)]
fn encode_mp3_perceptual_quantized_band_gain_global_gain_bias(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    step: f32,
    band_start: usize,
    band_end: usize,
    gain: f32,
    global_gain_bias: i16,
) -> PyResult<Vec<u8>> {
    let pcm = sonare_codec_rs::AudioBuffer::new(sample_rate, channels, samples)
        .map_err(to_py_value_error)?;
    sonare_codec_rs::encode_mpeg1_layer3_pcm_frames_with_perceptual_quantized_band_gain_and_global_gain_bias_and_table_provider(
        &pcm,
        step,
        sonare_codec_rs::Layer3QuantizedBandGain {
            band_start,
            band_end,
            gain,
        },
        global_gain_bias,
        sonare_codec_rs::mpeg1_layer3_standard_table_provider(),
    )
    .map_err(to_py_value_error)
}

#[pyfunction]
fn encode_vorbis(sample_rate: u32, channels: u16, samples: Vec<f32>) -> PyResult<Vec<u8>> {
    let pcm = sonare_codec_rs::AudioBuffer::new(sample_rate, channels, samples)
        .map_err(to_py_value_error)?;
    sonare_codec_rs::encode(sonare_codec_rs::Format::Vorbis, &pcm).map_err(to_py_value_error)
}

#[pyfunction]
fn encode_opus(sample_rate: u32, channels: u16, samples: Vec<f32>) -> PyResult<Vec<u8>> {
    let pcm = sonare_codec_rs::AudioBuffer::new(sample_rate, channels, samples)
        .map_err(to_py_value_error)?;
    sonare_codec_rs::encode(sonare_codec_rs::Format::Opus, &pcm).map_err(to_py_value_error)
}

#[pyfunction]
fn encode_aac(sample_rate: u32, channels: u16, samples: Vec<f32>) -> PyResult<Vec<u8>> {
    let pcm = sonare_codec_rs::AudioBuffer::new(sample_rate, channels, samples)
        .map_err(to_py_value_error)?;
    sonare_codec_rs::encode(sonare_codec_rs::Format::Aac, &pcm).map_err(to_py_value_error)
}

#[pyfunction]
fn encode_aac_with_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    target_bitrate_bps: u32,
) -> PyResult<Vec<u8>> {
    let pcm = sonare_codec_rs::AudioBuffer::new(sample_rate, channels, samples)
        .map_err(to_py_value_error)?;
    sonare_codec_rs::encode_aac_adts_with_bitrate(&pcm, target_bitrate_bps)
        .map_err(to_py_value_error)
}

#[pyfunction]
fn encode_aac_with_selected_scale_factors_and_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    target_bitrate_bps: u32,
) -> PyResult<Vec<u8>> {
    let pcm = sonare_codec_rs::AudioBuffer::new(sample_rate, channels, samples)
        .map_err(to_py_value_error)?;
    sonare_codec_rs::encode_aac_adts_with_selected_scale_factors_and_bitrate(
        &pcm,
        target_bitrate_bps,
    )
    .map_err(to_py_value_error)
}

#[pyfunction]
fn encode_aac_with_standard_spectral_offsets_and_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    target_bitrate_bps: u32,
    global_gain: u8,
) -> PyResult<Vec<u8>> {
    let pcm = sonare_codec_rs::AudioBuffer::new(sample_rate, channels, samples)
        .map_err(to_py_value_error)?;
    sonare_codec_rs::encode_aac_adts_with_standard_spectral_offsets_and_bitrate(
        &pcm,
        target_bitrate_bps,
        global_gain,
    )
    .map_err(to_py_value_error)
}

#[pyfunction]
fn encode_aac_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    target_bitrate_bps: u32,
    global_gain: u8,
    scale_factor_magnitude_bias: i16,
) -> PyResult<Vec<u8>> {
    let pcm = sonare_codec_rs::AudioBuffer::new(sample_rate, channels, samples)
        .map_err(to_py_value_error)?;
    sonare_codec_rs::encode_aac_adts_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_bitrate(
        &pcm,
        target_bitrate_bps,
        global_gain,
        scale_factor_magnitude_bias,
    )
    .map_err(to_py_value_error)
}

#[pyfunction]
fn encode_aac_with_recommended_standard_spectral_offsets_and_selected_scale_factors_and_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    target_bitrate_bps: u32,
) -> PyResult<Vec<u8>> {
    let pcm = sonare_codec_rs::AudioBuffer::new(sample_rate, channels, samples)
        .map_err(to_py_value_error)?;
    sonare_codec_rs::encode_aac_adts_with_recommended_standard_spectral_offsets_and_selected_scale_factors_and_bitrate(
        &pcm,
        target_bitrate_bps,
    )
    .map_err(to_py_value_error)
}

#[pyfunction]
fn encode_aac_with_standard_spectral_offsets_and_selected_scale_factors_max_quantized_abs_and_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    target_bitrate_bps: u32,
    global_gain: u8,
    scale_factor_magnitude_bias: i16,
    max_quantized_abs: u32,
) -> PyResult<Vec<u8>> {
    let pcm = sonare_codec_rs::AudioBuffer::new(sample_rate, channels, samples)
        .map_err(to_py_value_error)?;
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
fn encode_aac_with_recommended_standard_spectral_offsets_and_selected_scale_factors_max_quantized_abs_and_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    target_bitrate_bps: u32,
    max_quantized_abs: u32,
) -> PyResult<Vec<u8>> {
    let pcm = sonare_codec_rs::AudioBuffer::new(sample_rate, channels, samples)
        .map_err(to_py_value_error)?;
    sonare_codec_rs::encode_aac_adts_with_recommended_standard_spectral_offsets_and_selected_scale_factors_max_quantized_abs_and_bitrate(
        &pcm,
        target_bitrate_bps,
        max_quantized_abs,
    )
    .map_err(to_py_value_error)
}

#[pyfunction]
fn encode_aac_with_balanced_standard_spectral_offsets_and_selected_scale_factors_and_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    target_bitrate_bps: u32,
) -> PyResult<Vec<u8>> {
    let pcm = sonare_codec_rs::AudioBuffer::new(sample_rate, channels, samples)
        .map_err(to_py_value_error)?;
    sonare_codec_rs::encode_aac_adts_with_balanced_standard_spectral_offsets_and_selected_scale_factors_and_bitrate(
        &pcm,
        target_bitrate_bps,
    )
    .map_err(to_py_value_error)
}

#[pyfunction]
fn encode_m4a(sample_rate: u32, channels: u16, samples: Vec<f32>) -> PyResult<Vec<u8>> {
    let aac = encode_aac(sample_rate, channels, samples)?;
    sonare_codec_rs::mux_aac_adts_as_m4a(&aac).map_err(to_py_value_error)
}

#[pyfunction]
fn encode_m4a_with_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    target_bitrate_bps: u32,
) -> PyResult<Vec<u8>> {
    let pcm = sonare_codec_rs::AudioBuffer::new(sample_rate, channels, samples)
        .map_err(to_py_value_error)?;
    sonare_codec_rs::encode_m4a_with_bitrate(&pcm, target_bitrate_bps).map_err(to_py_value_error)
}

#[pyfunction]
fn encode_m4a_with_selected_scale_factors_and_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    target_bitrate_bps: u32,
) -> PyResult<Vec<u8>> {
    let pcm = sonare_codec_rs::AudioBuffer::new(sample_rate, channels, samples)
        .map_err(to_py_value_error)?;
    sonare_codec_rs::encode_m4a_with_selected_scale_factors_and_bitrate(&pcm, target_bitrate_bps)
        .map_err(to_py_value_error)
}

#[pyfunction]
fn encode_m4a_with_standard_spectral_offsets_and_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    target_bitrate_bps: u32,
    global_gain: u8,
) -> PyResult<Vec<u8>> {
    let pcm = sonare_codec_rs::AudioBuffer::new(sample_rate, channels, samples)
        .map_err(to_py_value_error)?;
    sonare_codec_rs::encode_m4a_with_standard_spectral_offsets_and_bitrate(
        &pcm,
        target_bitrate_bps,
        global_gain,
    )
    .map_err(to_py_value_error)
}

#[pyfunction]
fn encode_m4a_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    target_bitrate_bps: u32,
    global_gain: u8,
    scale_factor_magnitude_bias: i16,
) -> PyResult<Vec<u8>> {
    let pcm = sonare_codec_rs::AudioBuffer::new(sample_rate, channels, samples)
        .map_err(to_py_value_error)?;
    sonare_codec_rs::encode_m4a_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_bitrate(
        &pcm,
        target_bitrate_bps,
        global_gain,
        scale_factor_magnitude_bias,
    )
    .map_err(to_py_value_error)
}

#[pyfunction]
fn encode_m4a_with_recommended_standard_spectral_offsets_and_selected_scale_factors_and_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    target_bitrate_bps: u32,
) -> PyResult<Vec<u8>> {
    let pcm = sonare_codec_rs::AudioBuffer::new(sample_rate, channels, samples)
        .map_err(to_py_value_error)?;
    sonare_codec_rs::encode_m4a_with_recommended_standard_spectral_offsets_and_selected_scale_factors_and_bitrate(
        &pcm,
        target_bitrate_bps,
    )
    .map_err(to_py_value_error)
}

#[pyfunction]
fn encode_m4a_with_standard_spectral_offsets_and_selected_scale_factors_max_quantized_abs_and_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    target_bitrate_bps: u32,
    global_gain: u8,
    scale_factor_magnitude_bias: i16,
    max_quantized_abs: u32,
) -> PyResult<Vec<u8>> {
    let pcm = sonare_codec_rs::AudioBuffer::new(sample_rate, channels, samples)
        .map_err(to_py_value_error)?;
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
fn encode_m4a_with_recommended_standard_spectral_offsets_and_selected_scale_factors_max_quantized_abs_and_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    target_bitrate_bps: u32,
    max_quantized_abs: u32,
) -> PyResult<Vec<u8>> {
    let pcm = sonare_codec_rs::AudioBuffer::new(sample_rate, channels, samples)
        .map_err(to_py_value_error)?;
    sonare_codec_rs::encode_m4a_with_recommended_standard_spectral_offsets_and_selected_scale_factors_max_quantized_abs_and_bitrate(
        &pcm,
        target_bitrate_bps,
        max_quantized_abs,
    )
    .map_err(to_py_value_error)
}

#[pyfunction]
fn encode_m4a_with_balanced_standard_spectral_offsets_and_selected_scale_factors_and_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    target_bitrate_bps: u32,
) -> PyResult<Vec<u8>> {
    let pcm = sonare_codec_rs::AudioBuffer::new(sample_rate, channels, samples)
        .map_err(to_py_value_error)?;
    sonare_codec_rs::encode_m4a_with_balanced_standard_spectral_offsets_and_selected_scale_factors_and_bitrate(
        &pcm,
        target_bitrate_bps,
    )
    .map_err(to_py_value_error)
}

#[pyfunction]
fn demux_m4a_as_aac_adts(input: &[u8]) -> PyResult<Vec<u8>> {
    sonare_codec_rs::demux_m4a_as_aac_adts(input).map_err(to_py_value_error)
}

#[pyfunction]
fn aac_lc_adts_max_frame_len_for_bitrate(
    sample_rate: u32,
    target_bitrate_bps: u32,
) -> PyResult<usize> {
    sonare_codec_rs::aac_lc_adts_max_frame_len_for_bitrate(sample_rate, target_bitrate_bps)
        .map_err(to_py_value_error)
}

#[pyfunction]
fn aac_lc_default_production_bitrate_bps(channels: u8) -> PyResult<u32> {
    sonare_codec_rs::aac_lc_default_production_bitrate_bps(channels).map_err(to_py_value_error)
}

#[pyfunction]
fn aac_lc_pcm_step_candidates() -> Vec<f64> {
    sonare_codec_rs::AAC_LC_PCM_STEP_CANDIDATES
        .iter()
        .map(|&step| f64::from(step))
        .collect()
}

#[pyfunction]
fn aac_standard_id_pcm_step_candidates() -> Vec<f64> {
    sonare_codec_rs::AAC_STANDARD_ID_PCM_STEP_CANDIDATES
        .iter()
        .map(|&step| f64::from(step))
        .collect()
}

#[pyfunction]
fn aac_standard_id_selected_scale_factor_global_gain(channels: u16) -> PyResult<u8> {
    sonare_codec_rs::aac_standard_id_selected_scale_factor_global_gain(channels)
        .map_err(to_py_value_error)
}

#[pyfunction]
fn aac_standard_id_selected_scale_factor_magnitude_bias() -> i16 {
    sonare_codec_rs::aac_standard_id_selected_scale_factor_magnitude_bias()
}

#[pyfunction]
fn aac_standard_id_selected_scale_factor_balanced_max_quantized_abs(
    channels: u16,
) -> PyResult<u32> {
    sonare_codec_rs::aac_standard_id_selected_scale_factor_balanced_max_quantized_abs(channels)
        .map_err(to_py_value_error)
}

#[pyfunction]
fn aac_standard_id_selected_scale_factor_balanced_parameters(channels: u16) -> PyResult<Vec<f64>> {
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
fn aac_standard_id_selected_scale_factor_balanced_gain_deltas(channels: u16) -> PyResult<Vec<f64>> {
    let profile = sonare_codec_rs::aac_standard_id_selected_scale_factor_balance_profile(channels)
        .map_err(to_py_value_error)?;
    Ok(profile
        .global_gain_deltas
        .iter()
        .map(|&delta| f64::from(delta))
        .collect())
}

#[pyfunction]
fn aac_standard_id_selected_scale_factor_balanced_magnitude_biases(
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
fn aac_standard_id_selected_scale_factor_parameters(channels: u16) -> PyResult<Vec<f64>> {
    let (global_gain, magnitude_bias) =
        sonare_codec_rs::aac_standard_id_selected_scale_factor_parameters(channels)
            .map_err(to_py_value_error)?;
    Ok(vec![f64::from(global_gain), f64::from(magnitude_bias)])
}

#[pyfunction]
fn aac_unsigned_pairs7_unit_magnitude_table() -> Vec<u32> {
    sonare_codec_rs::aac_unsigned_pairs7_unit_magnitude_table()
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

#[pyfunction]
fn aac_unsigned_pairs7_table() -> Vec<u32> {
    sonare_codec_rs::aac_unsigned_pairs7_table()
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

#[pyfunction]
fn aac_signed_pairs5_table() -> Vec<i32> {
    sonare_codec_rs::aac_signed_pairs5_table()
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

#[pyfunction]
fn aac_signed_pairs6_table() -> Vec<i32> {
    sonare_codec_rs::aac_signed_pairs6_table()
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

#[pyfunction]
fn aac_unsigned_pairs8_table() -> Vec<u32> {
    sonare_codec_rs::aac_unsigned_pairs8_table()
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

#[pyfunction]
fn aac_unsigned_pairs9_table() -> Vec<u32> {
    sonare_codec_rs::aac_unsigned_pairs9_table()
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

#[pyfunction]
fn aac_unsigned_pairs10_table() -> Vec<u32> {
    sonare_codec_rs::aac_unsigned_pairs10_table()
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

#[pyfunction]
fn aac_signed_quads1_table() -> Vec<i32> {
    sonare_codec_rs::aac_signed_quads1_table()
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

#[pyfunction]
fn aac_signed_quads2_table() -> Vec<i32> {
    sonare_codec_rs::aac_signed_quads2_table()
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

#[pyfunction]
fn aac_unsigned_quads3_table() -> Vec<u32> {
    sonare_codec_rs::aac_unsigned_quads3_table()
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

#[pyfunction]
fn aac_unsigned_quads4_table() -> Vec<u32> {
    sonare_codec_rs::aac_unsigned_quads4_table()
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

#[pyfunction]
fn aac_escape_table() -> Vec<u32> {
    sonare_codec_rs::aac_escape_table()
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

#[pyfunction]
fn aac_scale_factor_delta_table() -> Vec<i32> {
    sonare_codec_rs::aac_scale_factor_delta_table()
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

#[pyfunction]
fn aac_codebook6_unit_section_plan(quantized: Vec<i32>, band_width: usize) -> PyResult<Vec<u32>> {
    let sections = sonare_codec_rs::plan_sections_by_bit_cost(
        &quantized,
        band_width,
        sonare_codec_rs::aac_unit_codebook6_spectral_tables(),
    )
    .map_err(to_py_value_error)?;

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

#[pyfunction]
fn aac_quad_unit_section_plan(quantized: Vec<i32>, band_width: usize) -> PyResult<Vec<u32>> {
    let sections = sonare_codec_rs::plan_quad_sections_by_bit_cost(
        &quantized,
        band_width,
        sonare_codec_rs::aac_unit_quad_spectral_tables(),
    )
    .map_err(to_py_value_error)?;

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

#[pyfunction]
fn aac_mixed_unit_section_plan(quantized: Vec<i32>, band_width: usize) -> PyResult<Vec<u32>> {
    let sections = sonare_codec_rs::plan_spectral_sections_by_bit_cost(
        &quantized,
        band_width,
        sonare_codec_rs::aac_unit_codebook6_spectral_tables(),
        sonare_codec_rs::aac_unit_quad_spectral_tables(),
    )
    .map_err(to_py_value_error)?;

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

#[pyfunction]
fn aac_mixed_unit_payload_bit_lengths(
    quantized: Vec<i32>,
    band_width: usize,
) -> PyResult<Vec<u32>> {
    let pair_tables = sonare_codec_rs::aac_unit_codebook6_spectral_tables();
    let quad_tables = sonare_codec_rs::aac_unit_quad_spectral_tables();
    let sections = sonare_codec_rs::plan_spectral_sections_by_bit_cost(
        &quantized,
        band_width,
        pair_tables,
        quad_tables,
    )
    .map_err(to_py_value_error)?;
    let split = sonare_codec_rs::split_sectioned_spectral_payload_by_codebook_id_with_sign_bits(
        &sections,
        &quantized,
        band_width,
        pair_tables,
        quad_tables,
    )
    .map_err(to_py_value_error)?;
    let packed = sonare_codec_rs::pack_sectioned_spectral_payload_by_codebook_id_with_sign_bits(
        &sections,
        &quantized,
        band_width,
        pair_tables,
        quad_tables,
    )
    .map_err(to_py_value_error)?;
    let scale_factor_bits = sonare_codec_rs::PackedBits {
        bytes: vec![0b1100_0000],
        bit_len: 2,
    };
    let split_with_scale =
        sonare_codec_rs::split_sectioned_spectral_payload_by_codebook_id_with_sign_bits_and_scale_factor_bits(
            &sections,
            &quantized,
            band_width,
            scale_factor_bits.clone(),
            pair_tables,
            quad_tables,
        )
        .map_err(to_py_value_error)?;
    let packed_with_scale =
        sonare_codec_rs::pack_sectioned_spectral_payload_by_codebook_id_with_sign_bits_and_scale_factor_bits(
            &sections,
            &quantized,
            band_width,
            scale_factor_bits,
            pair_tables,
            quad_tables,
        )
        .map_err(to_py_value_error)?;

    Ok(vec![
        split.section_and_scale_factor_bits.bit_len as u32,
        split.spectral_bits.bit_len as u32,
        packed.bit_len as u32,
        split_with_scale.section_and_scale_factor_bits.bit_len as u32,
        split_with_scale.spectral_bits.bit_len as u32,
        packed_with_scale.bit_len as u32,
    ])
}

#[pyfunction]
fn aac_standard_unit_section_plan(quantized: Vec<i32>, band_width: usize) -> PyResult<Vec<u32>> {
    let sections =
        sonare_codec_rs::plan_aac_lc_standard_spectral_sections_by_bit_cost(&quantized, band_width)
            .map_err(to_py_value_error)?;

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

#[pyfunction]
fn aac_standard_offsets_section_plan(
    quantized: Vec<i32>,
    offsets: Vec<usize>,
) -> PyResult<Vec<u32>> {
    let sections = sonare_codec_rs::plan_aac_lc_standard_spectral_sections_by_offsets_by_bit_cost(
        &quantized, &offsets,
    )
    .map_err(to_py_value_error)?;

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

#[pyfunction]
fn aac_standard_escape_payload_bit_lengths() -> PyResult<Vec<u32>> {
    let quantized = [17, 0];
    let band_width = 2;
    let pair_tables = sonare_codec_rs::aac_lc_standard_spectral_tables();
    let quad_tables = sonare_codec_rs::AacSpectralMagnitudeQuadTables::default();
    let sections = sonare_codec_rs::plan_spectral_sections_by_bit_cost(
        &quantized,
        band_width,
        pair_tables,
        quad_tables,
    )
    .map_err(to_py_value_error)?;
    if sections.first().map(|section| section.codebook_id)
        != Some(sonare_codec_rs::AacCodebook::Escape.id())
    {
        return Err(pyo3::exceptions::PyValueError::new_err(
            "AAC standard escape fixture did not select codebook 11",
        ));
    }
    let split = sonare_codec_rs::split_sectioned_spectral_payload_by_codebook_id_with_sign_bits(
        &sections,
        &quantized,
        band_width,
        pair_tables,
        quad_tables,
    )
    .map_err(to_py_value_error)?;
    let packed = sonare_codec_rs::pack_sectioned_spectral_payload_by_codebook_id_with_sign_bits(
        &sections,
        &quantized,
        band_width,
        pair_tables,
        quad_tables,
    )
    .map_err(to_py_value_error)?;

    Ok(vec![
        split.section_and_scale_factor_bits.bit_len as u32,
        split.spectral_bits.bit_len as u32,
        packed.bit_len as u32,
    ])
}

#[pyfunction]
fn aac_standard_mixed_payload_bit_lengths(
    quantized: Vec<i32>,
    band_width: usize,
) -> PyResult<Vec<u32>> {
    let split = sonare_codec_rs::split_aac_lc_standard_spectral_payload_with_sign_bits_by_bit_cost(
        &quantized, band_width,
    )
    .map_err(to_py_value_error)?;
    let packed = sonare_codec_rs::pack_aac_lc_standard_spectral_payload_with_sign_bits_by_bit_cost(
        &quantized, band_width,
    )
    .map_err(to_py_value_error)?;
    let scale_factor_bits = sonare_codec_rs::PackedBits {
        bytes: vec![0b1100_0000],
        bit_len: 2,
    };
    let split_with_scale =
        sonare_codec_rs::split_aac_lc_standard_spectral_payload_with_sign_bits_and_scale_factor_bits_by_bit_cost(
            &quantized,
            band_width,
            scale_factor_bits.clone(),
        )
        .map_err(to_py_value_error)?;
    let packed_with_scale =
        sonare_codec_rs::pack_aac_lc_standard_spectral_payload_with_sign_bits_and_scale_factor_bits_by_bit_cost(
            &quantized,
            band_width,
            scale_factor_bits,
        )
        .map_err(to_py_value_error)?;

    Ok(vec![
        split.section_and_scale_factor_bits.bit_len as u32,
        split.spectral_bits.bit_len as u32,
        packed.bit_len as u32,
        split_with_scale.section_and_scale_factor_bits.bit_len as u32,
        split_with_scale.spectral_bits.bit_len as u32,
        packed_with_scale.bit_len as u32,
    ])
}

#[pyfunction]
fn aac_standard_mixed_offsets_payload_bit_lengths(
    quantized: Vec<i32>,
    offsets: Vec<usize>,
) -> PyResult<Vec<u32>> {
    let split =
        sonare_codec_rs::split_aac_lc_standard_spectral_payload_with_offsets_and_sign_bits_by_bit_cost(
            &quantized,
            &offsets,
        )
        .map_err(to_py_value_error)?;
    let packed =
        sonare_codec_rs::pack_aac_lc_standard_spectral_payload_with_offsets_and_sign_bits_by_bit_cost(
            &quantized,
            &offsets,
        )
        .map_err(to_py_value_error)?;
    let scale_factor_bits = sonare_codec_rs::PackedBits {
        bytes: vec![0b1100_0000],
        bit_len: 2,
    };
    let split_with_scale =
        sonare_codec_rs::split_aac_lc_standard_spectral_payload_with_offsets_and_sign_bits_and_scale_factor_bits_by_bit_cost(
            &quantized,
            &offsets,
            scale_factor_bits.clone(),
        )
        .map_err(to_py_value_error)?;
    let packed_with_scale =
        sonare_codec_rs::pack_aac_lc_standard_spectral_payload_with_offsets_and_sign_bits_and_scale_factor_bits_by_bit_cost(
            &quantized,
            &offsets,
            scale_factor_bits,
        )
        .map_err(to_py_value_error)?;

    Ok(vec![
        split.section_and_scale_factor_bits.bit_len as u32,
        split.spectral_bits.bit_len as u32,
        packed.bit_len as u32,
        split_with_scale.section_and_scale_factor_bits.bit_len as u32,
        split_with_scale.spectral_bits.bit_len as u32,
        packed_with_scale.bit_len as u32,
    ])
}

#[pyfunction]
fn encode_aac_standard_mono_offsets_with_step(
    sample_rate: u32,
    samples: Vec<f32>,
    step: f32,
    global_gain: u8,
) -> PyResult<Vec<u8>> {
    let pcm =
        sonare_codec_rs::AudioBuffer::new(sample_rate, 1, samples).map_err(to_py_value_error)?;
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
fn encode_aac_standard_mono_offsets_with_bitrate(
    sample_rate: u32,
    samples: Vec<f32>,
    target_bitrate_bps: u32,
    global_gain: u8,
) -> PyResult<Vec<u8>> {
    let pcm =
        sonare_codec_rs::AudioBuffer::new(sample_rate, 1, samples).map_err(to_py_value_error)?;
    sonare_codec_rs::encode_aac_adts_with_standard_spectral_offsets_and_bitrate(
        &pcm,
        target_bitrate_bps,
        global_gain,
    )
    .map_err(to_py_value_error)
}

#[pyfunction]
fn aac_standard_mono_offsets_bitrate_frame_details(
    sample_rate: u32,
    samples: Vec<f32>,
    target_bitrate_bps: u32,
    global_gain: u8,
) -> PyResult<Vec<f64>> {
    let pcm =
        sonare_codec_rs::AudioBuffer::new(sample_rate, 1, samples).map_err(to_py_value_error)?;
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
fn encode_aac_standard_stereo_offsets_with_step(
    sample_rate: u32,
    samples: Vec<f32>,
    step: f32,
    global_gain: u8,
) -> PyResult<Vec<u8>> {
    let pcm =
        sonare_codec_rs::AudioBuffer::new(sample_rate, 2, samples).map_err(to_py_value_error)?;
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
fn encode_aac_standard_stereo_offsets_with_bitrate(
    sample_rate: u32,
    samples: Vec<f32>,
    target_bitrate_bps: u32,
    global_gain: u8,
) -> PyResult<Vec<u8>> {
    let pcm =
        sonare_codec_rs::AudioBuffer::new(sample_rate, 2, samples).map_err(to_py_value_error)?;
    sonare_codec_rs::encode_aac_adts_with_standard_spectral_offsets_and_bitrate(
        &pcm,
        target_bitrate_bps,
        global_gain,
    )
    .map_err(to_py_value_error)
}

#[pyfunction]
fn aac_standard_stereo_offsets_bitrate_frame_details(
    sample_rate: u32,
    samples: Vec<f32>,
    target_bitrate_bps: u32,
    global_gain: u8,
) -> PyResult<Vec<f64>> {
    let pcm =
        sonare_codec_rs::AudioBuffer::new(sample_rate, 2, samples).map_err(to_py_value_error)?;
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
fn aac_standard_selected_scale_factor_frame_details_with_magnitude_bias_and_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    target_bitrate_bps: u32,
    global_gain: u8,
    scale_factor_magnitude_bias: i16,
) -> PyResult<Vec<f64>> {
    let pcm = sonare_codec_rs::AudioBuffer::new(sample_rate, channels, samples)
        .map_err(to_py_value_error)?;
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
fn aac_recommended_standard_selected_scale_factor_frame_details_with_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    target_bitrate_bps: u32,
) -> PyResult<Vec<f64>> {
    let pcm = sonare_codec_rs::AudioBuffer::new(sample_rate, channels, samples)
        .map_err(to_py_value_error)?;
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
fn aac_standard_selected_scale_factor_frame_details_with_magnitude_bias_max_quantized_abs_and_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    target_bitrate_bps: u32,
    global_gain: u8,
    scale_factor_magnitude_bias: i16,
    max_quantized_abs: u32,
) -> PyResult<Vec<f64>> {
    let pcm = sonare_codec_rs::AudioBuffer::new(sample_rate, channels, samples)
        .map_err(to_py_value_error)?;
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
fn aac_recommended_standard_selected_scale_factor_frame_details_with_max_quantized_abs_and_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    target_bitrate_bps: u32,
    max_quantized_abs: u32,
) -> PyResult<Vec<f64>> {
    let pcm = sonare_codec_rs::AudioBuffer::new(sample_rate, channels, samples)
        .map_err(to_py_value_error)?;
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
fn aac_balanced_standard_selected_scale_factor_frame_details_with_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    target_bitrate_bps: u32,
) -> PyResult<Vec<f64>> {
    let pcm = sonare_codec_rs::AudioBuffer::new(sample_rate, channels, samples)
        .map_err(to_py_value_error)?;
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
fn aac_standard_selected_scale_factor_profile_with_magnitude_bias_and_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    target_bitrate_bps: u32,
    global_gain: u8,
    scale_factor_magnitude_bias: i16,
) -> PyResult<Vec<f64>> {
    let pcm = sonare_codec_rs::AudioBuffer::new(sample_rate, channels, samples)
        .map_err(to_py_value_error)?;
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
fn aac_recommended_standard_selected_scale_factor_profile_with_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    target_bitrate_bps: u32,
) -> PyResult<Vec<f64>> {
    let pcm = sonare_codec_rs::AudioBuffer::new(sample_rate, channels, samples)
        .map_err(to_py_value_error)?;
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
fn aac_balanced_standard_selected_scale_factor_profile_with_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    target_bitrate_bps: u32,
) -> PyResult<Vec<f64>> {
    let pcm = sonare_codec_rs::AudioBuffer::new(sample_rate, channels, samples)
        .map_err(to_py_value_error)?;
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

fn flatten_aac_standard_id_payload_breakdown(
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

fn flatten_aac_standard_id_quality_control_profile(
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

fn flatten_aac_standard_id_quality_control_candidate(
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
fn aac_standard_id_payload_breakdown_with_magnitude_bias_and_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    target_bitrate_bps: u32,
    global_gain: u8,
    scale_factor_magnitude_bias: i16,
) -> PyResult<Vec<f64>> {
    let pcm = sonare_codec_rs::AudioBuffer::new(sample_rate, channels, samples)
        .map_err(to_py_value_error)?;
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
fn aac_recommended_standard_id_payload_breakdown_with_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    target_bitrate_bps: u32,
) -> PyResult<Vec<f64>> {
    let pcm = sonare_codec_rs::AudioBuffer::new(sample_rate, channels, samples)
        .map_err(to_py_value_error)?;
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
fn aac_balanced_standard_id_payload_breakdown_with_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    target_bitrate_bps: u32,
) -> PyResult<Vec<f64>> {
    let pcm = sonare_codec_rs::AudioBuffer::new(sample_rate, channels, samples)
        .map_err(to_py_value_error)?;
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
fn aac_standard_id_quality_control_profile_with_magnitude_bias_max_quantized_abs_and_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    target_bitrate_bps: u32,
    global_gain: u8,
    scale_factor_magnitude_bias: i16,
    max_quantized_abs: u32,
) -> PyResult<Vec<f64>> {
    let pcm = sonare_codec_rs::AudioBuffer::new(sample_rate, channels, samples)
        .map_err(to_py_value_error)?;
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
fn aac_balanced_standard_id_quality_control_profile_with_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    target_bitrate_bps: u32,
) -> PyResult<Vec<f64>> {
    let pcm = sonare_codec_rs::AudioBuffer::new(sample_rate, channels, samples)
        .map_err(to_py_value_error)?;
    sonare_codec_rs::aac_balanced_standard_id_quality_control_profile_with_bitrate(
        &pcm,
        target_bitrate_bps,
    )
    .map(flatten_aac_standard_id_quality_control_profile)
    .map_err(to_py_value_error)
}

#[pyfunction]
fn aac_standard_id_quality_control_candidates_for_balance_profile_with_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    target_bitrate_bps: u32,
) -> PyResult<Vec<f64>> {
    let pcm = sonare_codec_rs::AudioBuffer::new(sample_rate, channels, samples)
        .map_err(to_py_value_error)?;
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
fn aac_selected_scale_factor_frame_details_with_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    target_bitrate_bps: u32,
) -> PyResult<Vec<f64>> {
    let pcm = sonare_codec_rs::AudioBuffer::new(sample_rate, channels, samples)
        .map_err(to_py_value_error)?;
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

#[pyfunction]
fn mp3_layer3_main_data_capacity_bytes(
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
fn mp3_layer3_main_data_capacity_bits(
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
fn mp3_pcm_step_candidates() -> Vec<f64> {
    sonare_codec_rs::MPEG1_LAYER3_PCM_STEP_CANDIDATES
        .iter()
        .map(|&step| f64::from(step))
        .collect()
}

#[pyfunction]
fn mp3_production_pcm_step_candidates(channels: u16) -> PyResult<Vec<f64>> {
    sonare_codec_rs::mpeg1_layer3_production_pcm_step_candidates(channels)
        .map(|candidates| candidates.iter().map(|&step| f64::from(step)).collect())
        .map_err(to_py_value_error)
}

#[pyfunction]
fn mp3_first_frame_perceptual_candidate_profile_with_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    bitrate_kbps: u16,
    crc_protected: bool,
) -> PyResult<Vec<f64>> {
    let pcm = sonare_codec_rs::AudioBuffer::new(sample_rate, channels, samples)
        .map_err(to_py_value_error)?;
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
fn mp3_first_frame_low_band_spectral_shape_candidate_profile_with_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    bitrate_kbps: u16,
    crc_protected: bool,
) -> PyResult<Vec<f64>> {
    let pcm = sonare_codec_rs::AudioBuffer::new(sample_rate, channels, samples)
        .map_err(to_py_value_error)?;
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
fn mp3_first_frame_band_spectral_shape_candidate_profile_with_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    bitrate_kbps: u16,
    crc_protected: bool,
) -> PyResult<Vec<f64>> {
    let pcm = sonare_codec_rs::AudioBuffer::new(sample_rate, channels, samples)
        .map_err(to_py_value_error)?;
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
fn mp3_first_frame_quality_guarded_candidate_profile_with_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    bitrate_kbps: u16,
    crc_protected: bool,
) -> PyResult<Vec<f64>> {
    let pcm = sonare_codec_rs::AudioBuffer::new(sample_rate, channels, samples)
        .map_err(to_py_value_error)?;
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
fn mp3_perceptual_bit_allocation_with_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    bitrate_kbps: u16,
    crc_protected: bool,
    min_bits_per_granule_channel: usize,
) -> PyResult<Vec<f64>> {
    let pcm = sonare_codec_rs::AudioBuffer::new(sample_rate, channels, samples)
        .map_err(to_py_value_error)?;
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
fn mp3_standard_big_value_table_selects() -> Vec<u32> {
    sonare_codec_rs::MPEG1_LAYER3_STANDARD_BIG_VALUE_TABLE_SELECTS
        .iter()
        .map(|&table_select| u32::from(table_select))
        .collect()
}

#[pyfunction]
fn mp3_missing_standard_big_value_table_selects() -> Vec<u32> {
    sonare_codec_rs::MPEG1_LAYER3_MISSING_STANDARD_BIG_VALUE_TABLE_SELECTS
        .iter()
        .map(|&table_select| u32::from(table_select))
        .collect()
}

#[pyfunction]
fn mp3_standard_count1_table_selects() -> Vec<u32> {
    sonare_codec_rs::MPEG1_LAYER3_STANDARD_COUNT1_TABLE_SELECTS
        .iter()
        .map(|&table_select| u32::from(table_select))
        .collect()
}

#[pyfunction]
fn mp3_reservoir_frame_details_with_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    bitrate_kbps: u16,
    crc_protected: bool,
) -> PyResult<Vec<f64>> {
    let pcm = sonare_codec_rs::AudioBuffer::new(sample_rate, channels, samples)
        .map_err(to_py_value_error)?;
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
fn mp3_perceptual_reservoir_frame_details_with_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    bitrate_kbps: u16,
    crc_protected: bool,
) -> PyResult<Vec<f64>> {
    let pcm = sonare_codec_rs::AudioBuffer::new(sample_rate, channels, samples)
        .map_err(to_py_value_error)?;
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
fn mp3_entropy_targeted_perceptual_reservoir_frame_details_with_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    bitrate_kbps: u16,
    crc_protected: bool,
    min_bits_per_granule_channel: usize,
) -> PyResult<Vec<f64>> {
    let pcm = sonare_codec_rs::AudioBuffer::new(sample_rate, channels, samples)
        .map_err(to_py_value_error)?;
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
fn mp3_entropy_targeted_perceptual_reservoir_utilization_profile_with_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    bitrate_kbps: u16,
    crc_protected: bool,
    min_bits_per_granule_channel: usize,
) -> PyResult<Vec<f64>> {
    let pcm = sonare_codec_rs::AudioBuffer::new(sample_rate, channels, samples)
        .map_err(to_py_value_error)?;
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
fn mp3_quality_guarded_perceptual_reservoir_frame_details_with_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    bitrate_kbps: u16,
    crc_protected: bool,
) -> PyResult<Vec<f64>> {
    let pcm = sonare_codec_rs::AudioBuffer::new(sample_rate, channels, samples)
        .map_err(to_py_value_error)?;
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

#[pymodule]
fn sonare_codec(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<StreamDecoder>()?;
    module.add_function(wrap_pyfunction!(detect_format, module)?)?;
    module.add_function(wrap_pyfunction!(decode_audio, module)?)?;
    module.add_function(wrap_pyfunction!(decode_wav, module)?)?;
    module.add_function(wrap_pyfunction!(decode_flac, module)?)?;
    module.add_function(wrap_pyfunction!(decode_mp3, module)?)?;
    module.add_function(wrap_pyfunction!(decode_vorbis, module)?)?;
    module.add_function(wrap_pyfunction!(decode_opus, module)?)?;
    module.add_function(wrap_pyfunction!(decode_aac, module)?)?;
    module.add_function(wrap_pyfunction!(decode_m4a, module)?)?;
    module.add_function(wrap_pyfunction!(encode_audio, module)?)?;
    module.add_function(wrap_pyfunction!(encode_audio_production, module)?)?;
    module.add_function(wrap_pyfunction!(encode_wav, module)?)?;
    module.add_function(wrap_pyfunction!(encode_flac, module)?)?;
    module.add_function(wrap_pyfunction!(encode_mp3, module)?)?;
    module.add_function(wrap_pyfunction!(encode_mp3_with_bitrate, module)?)?;
    module.add_function(wrap_pyfunction!(encode_mp3_cbr_with_bitrate, module)?)?;
    module.add_function(wrap_pyfunction!(
        encode_mp3_perceptual_active_cbr_with_bitrate,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(
        encode_mp3_perceptual_reservoir_with_bitrate,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(
        encode_mp3_entropy_targeted_perceptual_reservoir_with_bitrate,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(
        encode_mp3_perceptual_scale_factor_band_bias,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(
        encode_mp3_perceptual_quantized_band_gain,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(
        encode_mp3_perceptual_quantized_band_gain_global_gain_bias,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(
        encode_mp3_quality_guarded_perceptual_reservoir_with_bitrate,
        module
    )?)?;
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
    module.add_function(wrap_pyfunction!(encode_vorbis, module)?)?;
    module.add_function(wrap_pyfunction!(encode_opus, module)?)?;
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
    module.add_function(wrap_pyfunction!(
        encode_aac_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_bitrate,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(
        encode_aac_with_recommended_standard_spectral_offsets_and_selected_scale_factors_and_bitrate,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(
        encode_aac_with_standard_spectral_offsets_and_selected_scale_factors_max_quantized_abs_and_bitrate,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(
        encode_aac_with_recommended_standard_spectral_offsets_and_selected_scale_factors_max_quantized_abs_and_bitrate,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(
        encode_aac_with_balanced_standard_spectral_offsets_and_selected_scale_factors_and_bitrate,
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
    module.add_function(wrap_pyfunction!(
        encode_m4a_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_bitrate,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(
        encode_m4a_with_recommended_standard_spectral_offsets_and_selected_scale_factors_and_bitrate,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(
        encode_m4a_with_standard_spectral_offsets_and_selected_scale_factors_max_quantized_abs_and_bitrate,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(
        encode_m4a_with_recommended_standard_spectral_offsets_and_selected_scale_factors_max_quantized_abs_and_bitrate,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(
        encode_m4a_with_balanced_standard_spectral_offsets_and_selected_scale_factors_and_bitrate,
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
    module.add_function(wrap_pyfunction!(
        aac_standard_selected_scale_factor_frame_details_with_magnitude_bias_max_quantized_abs_and_bitrate,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(
        aac_recommended_standard_selected_scale_factor_frame_details_with_max_quantized_abs_and_bitrate,
        module
    )?)?;
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
        aac_unsigned_pairs7_unit_magnitude_table,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(aac_unsigned_pairs7_table, module)?)?;
    module.add_function(wrap_pyfunction!(aac_signed_pairs5_table, module)?)?;
    module.add_function(wrap_pyfunction!(aac_signed_pairs6_table, module)?)?;
    module.add_function(wrap_pyfunction!(aac_signed_quads1_table, module)?)?;
    module.add_function(wrap_pyfunction!(aac_signed_quads2_table, module)?)?;
    module.add_function(wrap_pyfunction!(aac_unsigned_pairs8_table, module)?)?;
    module.add_function(wrap_pyfunction!(aac_unsigned_pairs9_table, module)?)?;
    module.add_function(wrap_pyfunction!(aac_unsigned_pairs10_table, module)?)?;
    module.add_function(wrap_pyfunction!(aac_unsigned_quads3_table, module)?)?;
    module.add_function(wrap_pyfunction!(aac_unsigned_quads4_table, module)?)?;
    module.add_function(wrap_pyfunction!(aac_escape_table, module)?)?;
    module.add_function(wrap_pyfunction!(aac_scale_factor_delta_table, module)?)?;
    module.add_function(wrap_pyfunction!(aac_codebook6_unit_section_plan, module)?)?;
    module.add_function(wrap_pyfunction!(aac_quad_unit_section_plan, module)?)?;
    module.add_function(wrap_pyfunction!(aac_mixed_unit_section_plan, module)?)?;
    module.add_function(wrap_pyfunction!(
        aac_mixed_unit_payload_bit_lengths,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(aac_standard_unit_section_plan, module)?)?;
    module.add_function(wrap_pyfunction!(aac_standard_offsets_section_plan, module)?)?;
    module.add_function(wrap_pyfunction!(
        aac_standard_escape_payload_bit_lengths,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(
        aac_standard_mixed_payload_bit_lengths,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(
        aac_standard_mixed_offsets_payload_bit_lengths,
        module
    )?)?;
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
    Ok(())
}

fn pcm_tuple(pcm: sonare_codec_rs::AudioBuffer) -> (u32, u16, Vec<f32>) {
    (pcm.sample_rate, pcm.channels, pcm.samples)
}

fn aac_offsets_max_sfb(offsets: &[usize]) -> PyResult<u8> {
    u8::try_from(offsets.len().saturating_sub(1)).map_err(|_| {
        pyo3::exceptions::PyValueError::new_err(
            "AAC-LC scale-factor band count exceeds max_sfb range",
        )
    })
}

fn constant_aac_scale_factors_by_frame(
    pcm: &sonare_codec_rs::AudioBuffer,
    global_gain: usize,
    band_count: usize,
) -> Vec<Vec<i16>> {
    let frame_count = pcm.samples.len().div_ceil(usize::from(pcm.channels) * 1024);
    let scale_factor = i16::try_from(global_gain).unwrap_or(i16::MAX);
    (0..frame_count)
        .map(|_| vec![scale_factor; band_count])
        .collect()
}

fn parse_format(format: &str) -> PyResult<sonare_codec_rs::Format> {
    match format.to_ascii_lowercase().as_str() {
        "wav" => Ok(sonare_codec_rs::Format::Wav),
        "flac" => Ok(sonare_codec_rs::Format::Flac),
        "mp3" => Ok(sonare_codec_rs::Format::Mp3),
        "vorbis" => Ok(sonare_codec_rs::Format::Vorbis),
        "opus" => Ok(sonare_codec_rs::Format::Opus),
        "aac" | "m4a" | "mp4" => Ok(sonare_codec_rs::Format::Aac),
        _ => Err(pyo3::exceptions::PyValueError::new_err(
            "unsupported format",
        )),
    }
}

fn encode_by_name(format: &str, pcm: &sonare_codec_rs::AudioBuffer) -> PyResult<Vec<u8>> {
    encode_by_name_with_mode(format, pcm, sonare_codec_rs::EncodeMode::Compatibility)
}

fn encode_by_name_with_mode(
    format: &str,
    pcm: &sonare_codec_rs::AudioBuffer,
    mode: sonare_codec_rs::EncodeMode,
) -> PyResult<Vec<u8>> {
    match format.to_ascii_lowercase().as_str() {
        "m4a" | "mp4" => {
            let aac = sonare_codec_rs::encode_with_mode(sonare_codec_rs::Format::Aac, pcm, mode)
                .map_err(to_py_value_error)?;
            sonare_codec_rs::mux_aac_adts_as_m4a(&aac).map_err(to_py_value_error)
        }
        _ => {
            let format = parse_format(format)?;
            sonare_codec_rs::encode_with_mode(format, pcm, mode).map_err(to_py_value_error)
        }
    }
}

fn to_py_value_error(err: sonare_codec_rs::Error) -> PyErr {
    pyo3::exceptions::PyValueError::new_err(err.to_string())
}

fn is_m4a_container(input: &[u8]) -> bool {
    input.len() >= 12
        && input.get(4..8) == Some(b"ftyp")
        && matches!(
            input.get(8..12),
            Some(b"M4A ") | Some(b"mp42") | Some(b"isom") | Some(b"iso2")
        )
}
