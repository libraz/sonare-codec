use super::*;

pub(crate) fn pcm_tuple(pcm: sonare_codec_rs::AudioBuffer) -> (u32, u16, Vec<f32>) {
    (pcm.sample_rate, pcm.channels, pcm.samples)
}

pub(crate) fn aac_offsets_max_sfb(offsets: &[usize]) -> PyResult<u8> {
    u8::try_from(offsets.len().saturating_sub(1)).map_err(|_| {
        pyo3::exceptions::PyValueError::new_err(
            "AAC-LC scale-factor band count exceeds max_sfb range",
        )
    })
}

pub(crate) fn constant_aac_scale_factors_by_frame(
    pcm: &sonare_codec_rs::AudioBuffer,
    global_gain: u8,
    band_count: usize,
) -> Vec<Vec<i16>> {
    let frame_count = pcm.samples.len().div_ceil(usize::from(pcm.channels) * 1024);
    // The AAC global gain is a u8, so it always fits in i16 — no silent clamp.
    let scale_factor = i16::from(global_gain);
    (0..frame_count)
        .map(|_| vec![scale_factor; band_count])
        .collect()
}

pub(crate) fn parse_format(format: &str) -> PyResult<sonare_codec_rs::Format> {
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

pub(crate) fn encode_by_name(
    format: &str,
    pcm: &sonare_codec_rs::AudioBuffer,
) -> PyResult<Vec<u8>> {
    encode_by_name_with_mode(format, pcm, sonare_codec_rs::EncodeMode::Compatibility)
}

pub(crate) fn encode_by_name_with_mode(
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

pub(crate) fn to_py_value_error(err: sonare_codec_rs::Error) -> PyErr {
    pyo3::exceptions::PyValueError::new_err(err.to_string())
}

pub(crate) fn is_m4a_container(input: &[u8]) -> bool {
    input.len() >= 12
        && input.get(4..8) == Some(b"ftyp")
        && matches!(
            input.get(8..12),
            Some(b"M4A ") | Some(b"mp42") | Some(b"isom") | Some(b"iso2")
        )
}
