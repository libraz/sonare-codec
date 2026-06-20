use super::*;

#[pyfunction]
pub(crate) fn detect_format(input: &[u8]) -> Option<String> {
    if is_m4a_container(input) {
        return Some("m4a".to_owned());
    }
    sonare_codec_rs::detect(input).map(|format| format!("{format:?}").to_ascii_lowercase())
}

#[pyfunction]
pub(crate) fn decode_audio(input: &[u8]) -> PyResult<(u32, u16, Vec<f32>)> {
    let pcm = sonare_codec_rs::decode(input).map_err(to_py_value_error)?;
    Ok(pcm_tuple(pcm))
}

#[pyfunction]
pub(crate) fn decode_wav(input: &[u8]) -> PyResult<(u32, u16, Vec<f32>)> {
    let pcm = sonare_codec_rs::decode_wav(input).map_err(to_py_value_error)?;
    Ok(pcm_tuple(pcm))
}

#[pyfunction]
pub(crate) fn decode_flac(input: &[u8]) -> PyResult<(u32, u16, Vec<f32>)> {
    let pcm = sonare_codec_rs::decode_flac(input).map_err(to_py_value_error)?;
    Ok(pcm_tuple(pcm))
}

#[pyfunction]
pub(crate) fn decode_mp3(input: &[u8]) -> PyResult<(u32, u16, Vec<f32>)> {
    let pcm = sonare_codec_rs::decode_mp3(input).map_err(to_py_value_error)?;
    Ok(pcm_tuple(pcm))
}

#[pyfunction]
pub(crate) fn decode_vorbis(input: &[u8]) -> PyResult<(u32, u16, Vec<f32>)> {
    let pcm = sonare_codec_rs::decode_vorbis(input).map_err(to_py_value_error)?;
    Ok(pcm_tuple(pcm))
}

#[pyfunction]
pub(crate) fn decode_opus(input: &[u8]) -> PyResult<(u32, u16, Vec<f32>)> {
    let pcm = sonare_codec_rs::decode_opus(input).map_err(to_py_value_error)?;
    Ok(pcm_tuple(pcm))
}

#[pyfunction]
pub(crate) fn decode_aac(input: &[u8]) -> PyResult<(u32, u16, Vec<f32>)> {
    let pcm = sonare_codec_rs::decode_aac(input).map_err(to_py_value_error)?;
    Ok(pcm_tuple(pcm))
}

#[pyfunction]
pub(crate) fn decode_m4a(input: &[u8]) -> PyResult<(u32, u16, Vec<f32>)> {
    let pcm = sonare_codec_rs::decode_aac(input).map_err(to_py_value_error)?;
    Ok(pcm_tuple(pcm))
}

pub(crate) fn pcm_from_samples(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
) -> PyResult<sonare_codec_rs::AudioBuffer> {
    sonare_codec_rs::AudioBuffer::new(sample_rate, channels, samples).map_err(to_py_value_error)
}

pub(crate) fn encode_format(
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
    format: sonare_codec_rs::Format,
) -> PyResult<Vec<u8>> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    sonare_codec_rs::encode(format, &pcm).map_err(to_py_value_error)
}

pub(crate) fn add_py_functions(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(detect_format, module)?)?;
    module.add_function(wrap_pyfunction!(decode_audio, module)?)?;
    module.add_function(wrap_pyfunction!(decode_wav, module)?)?;
    module.add_function(wrap_pyfunction!(decode_flac, module)?)?;
    module.add_function(wrap_pyfunction!(decode_mp3, module)?)?;
    module.add_function(wrap_pyfunction!(decode_vorbis, module)?)?;
    module.add_function(wrap_pyfunction!(decode_opus, module)?)?;
    module.add_function(wrap_pyfunction!(decode_aac, module)?)?;
    module.add_function(wrap_pyfunction!(decode_m4a, module)?)?;
    Ok(())
}
