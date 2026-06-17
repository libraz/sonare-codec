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
fn encode_m4a(sample_rate: u32, channels: u16, samples: Vec<f32>) -> PyResult<Vec<u8>> {
    let aac = encode_aac(sample_rate, channels, samples)?;
    sonare_codec_rs::mux_aac_adts_as_m4a(&aac).map_err(to_py_value_error)
}

#[pyfunction]
fn demux_m4a_as_aac_adts(input: &[u8]) -> PyResult<Vec<u8>> {
    sonare_codec_rs::demux_m4a_as_aac_adts(input).map_err(to_py_value_error)
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
    module.add_function(wrap_pyfunction!(encode_vorbis, module)?)?;
    module.add_function(wrap_pyfunction!(encode_opus, module)?)?;
    module.add_function(wrap_pyfunction!(encode_aac, module)?)?;
    module.add_function(wrap_pyfunction!(encode_m4a, module)?)?;
    module.add_function(wrap_pyfunction!(demux_m4a_as_aac_adts, module)?)?;
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
