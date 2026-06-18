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
    let pairs6 = [sonare_codec_rs::HuffmanEntry {
        symbol: sonare_codec_rs::AacSpectralMagnitudePair::new(1, 1),
        code: sonare_codec_rs::HuffmanCode::new(0b1, 1).expect("valid AAC codebook 6 unit code"),
    }];
    let sections = sonare_codec_rs::plan_sections_by_bit_cost(
        &quantized,
        band_width,
        sonare_codec_rs::AacSpectralMagnitudeTables {
            pairs1: &[],
            pairs5: &[],
            pairs6: &pairs6,
            escape: &[],
        },
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
    module.add_function(wrap_pyfunction!(encode_vorbis, module)?)?;
    module.add_function(wrap_pyfunction!(encode_opus, module)?)?;
    module.add_function(wrap_pyfunction!(encode_aac, module)?)?;
    module.add_function(wrap_pyfunction!(encode_aac_with_bitrate, module)?)?;
    module.add_function(wrap_pyfunction!(
        encode_aac_with_selected_scale_factors_and_bitrate,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(encode_m4a, module)?)?;
    module.add_function(wrap_pyfunction!(encode_m4a_with_bitrate, module)?)?;
    module.add_function(wrap_pyfunction!(
        encode_m4a_with_selected_scale_factors_and_bitrate,
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
    module.add_function(wrap_pyfunction!(
        aac_unsigned_pairs7_unit_magnitude_table,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(aac_unsigned_pairs7_table, module)?)?;
    module.add_function(wrap_pyfunction!(aac_unsigned_pairs8_table, module)?)?;
    module.add_function(wrap_pyfunction!(aac_unsigned_pairs9_table, module)?)?;
    module.add_function(wrap_pyfunction!(aac_unsigned_pairs10_table, module)?)?;
    module.add_function(wrap_pyfunction!(aac_escape_table, module)?)?;
    module.add_function(wrap_pyfunction!(aac_scale_factor_delta_table, module)?)?;
    module.add_function(wrap_pyfunction!(aac_codebook6_unit_section_plan, module)?)?;
    module.add_function(wrap_pyfunction!(
        mp3_layer3_main_data_capacity_bytes,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(
        mp3_layer3_main_data_capacity_bits,
        module
    )?)?;
    Ok(())
}

fn pcm_tuple(pcm: sonare_codec_rs::AudioBuffer) -> (u32, u16, Vec<f32>) {
    (pcm.sample_rate, pcm.channels, pcm.samples)
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
