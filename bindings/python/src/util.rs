use super::*;

pub(crate) fn pcm_tuple(pcm: sonare_codec_rs::AudioBuffer) -> (u32, u16, Vec<f32>) {
    (pcm.sample_rate, pcm.channels, pcm.samples)
}

pub(crate) fn aac_offsets_max_sfb(offsets: &[usize]) -> PyResult<u8> {
    sonare_codec_rs::bindings_support::aac_offsets_max_sfb(offsets).map_err(to_py_value_error)
}

pub(crate) fn constant_aac_scale_factors_by_frame(
    pcm: &sonare_codec_rs::AudioBuffer,
    global_gain: u8,
    band_count: usize,
) -> Vec<Vec<i16>> {
    sonare_codec_rs::bindings_support::constant_aac_scale_factors_by_frame(
        pcm,
        global_gain,
        band_count,
    )
}

pub(crate) fn encode_by_name(
    format: &str,
    pcm: &sonare_codec_rs::AudioBuffer,
) -> PyResult<Vec<u8>> {
    sonare_codec_rs::bindings_support::encode_by_name(format, pcm).map_err(to_py_value_error)
}

pub(crate) fn encode_by_name_with_mode(
    format: &str,
    pcm: &sonare_codec_rs::AudioBuffer,
    mode: sonare_codec_rs::EncodeMode,
) -> PyResult<Vec<u8>> {
    sonare_codec_rs::bindings_support::encode_by_name_with_mode(format, pcm, mode)
        .map_err(to_py_value_error)
}

pub(crate) fn to_py_value_error(err: sonare_codec_rs::Error) -> PyErr {
    pyo3::exceptions::PyValueError::new_err(err.to_string())
}

pub(crate) fn is_m4a_container(input: &[u8]) -> bool {
    sonare_codec_rs::bindings_support::is_m4a_container(input)
}
