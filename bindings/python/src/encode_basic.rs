use super::*;

#[pyfunction]
pub(crate) fn encode_audio(
    format: &str,
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
) -> PyResult<Vec<u8>> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    encode_by_name(format, &pcm)
}

#[pyfunction]
pub(crate) fn encode_audio_production(
    format: &str,
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
) -> PyResult<Vec<u8>> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    encode_by_name_with_mode(format, &pcm, sonare_codec_rs::EncodeMode::ProductionOnly)
}

#[pyfunction]
pub(crate) fn encode_wav(sample_rate: u32, channels: u16, samples: Vec<f32>) -> PyResult<Vec<u8>> {
    encode_format(sample_rate, channels, samples, sonare_codec_rs::Format::Wav)
}

#[pyfunction]
pub(crate) fn encode_flac(sample_rate: u32, channels: u16, samples: Vec<f32>) -> PyResult<Vec<u8>> {
    encode_format(
        sample_rate,
        channels,
        samples,
        sonare_codec_rs::Format::Flac,
    )
}

#[pyfunction]
pub(crate) fn encode_mp3(sample_rate: u32, channels: u16, samples: Vec<f32>) -> PyResult<Vec<u8>> {
    encode_format(sample_rate, channels, samples, sonare_codec_rs::Format::Mp3)
}

#[pyfunction]
pub(crate) fn encode_mp3_with_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    bitrate_kbps: u16,
    padding: bool,
    crc_protected: bool,
) -> PyResult<Vec<u8>> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
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
pub(crate) fn encode_mp3_cbr_with_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    bitrate_kbps: u16,
    crc_protected: bool,
) -> PyResult<Vec<u8>> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
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
pub(crate) fn encode_mp3_perceptual_active_cbr_with_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    bitrate_kbps: u16,
    crc_protected: bool,
) -> PyResult<Vec<u8>> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
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
pub(crate) fn encode_mp3_perceptual_reservoir_with_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    bitrate_kbps: u16,
    crc_protected: bool,
) -> PyResult<Vec<u8>> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
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
pub(crate) fn encode_mp3_entropy_targeted_perceptual_reservoir_with_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    bitrate_kbps: u16,
    crc_protected: bool,
    min_bits_per_granule_channel: usize,
) -> PyResult<Vec<u8>> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
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
pub(crate) fn encode_mp3_quality_guarded_perceptual_reservoir_with_bitrate(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    bitrate_kbps: u16,
    crc_protected: bool,
) -> PyResult<Vec<u8>> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
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
pub(crate) fn encode_mp3_perceptual_scale_factor_band_bias(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    step: f32,
    band_start: usize,
    band_end: usize,
    bias: i8,
) -> PyResult<Vec<u8>> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
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
pub(crate) fn encode_mp3_perceptual_quantized_band_gain(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    step: f32,
    band_start: usize,
    band_end: usize,
    gain: f32,
) -> PyResult<Vec<u8>> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
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
pub(crate) fn encode_mp3_perceptual_quantized_band_gain_global_gain_bias(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    step: f32,
    band_start: usize,
    band_end: usize,
    gain: f32,
    global_gain_bias: i16,
) -> PyResult<Vec<u8>> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
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
pub(crate) fn encode_vorbis(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
) -> PyResult<Vec<u8>> {
    encode_format(
        sample_rate,
        channels,
        samples,
        sonare_codec_rs::Format::Vorbis,
    )
}

#[pyfunction]
pub(crate) fn encode_opus(sample_rate: u32, channels: u16, samples: Vec<f32>) -> PyResult<Vec<u8>> {
    encode_format(
        sample_rate,
        channels,
        samples,
        sonare_codec_rs::Format::Opus,
    )
}

pub(crate) fn add_py_functions(module: &Bound<'_, PyModule>) -> PyResult<()> {
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
        encode_mp3_quality_guarded_perceptual_reservoir_with_bitrate,
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
    module.add_function(wrap_pyfunction!(encode_vorbis, module)?)?;
    module.add_function(wrap_pyfunction!(encode_opus, module)?)?;
    Ok(())
}
