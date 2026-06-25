use super::*;

#[wasm_bindgen]
pub fn detect_format(input: &[u8]) -> Option<String> {
    if is_m4a_container(input) {
        return Some("m4a".to_owned());
    }
    // Map to stable JS-facing strings explicitly instead of deriving them from
    // the `Debug` impl, so the string contract cannot drift if `Debug` changes.
    sonare_codec::detect(input).map(|format| {
        match format {
            sonare_codec::Format::Wav => "wav",
            sonare_codec::Format::Flac => "flac",
            sonare_codec::Format::Mp3 => "mp3",
            sonare_codec::Format::Vorbis => "vorbis",
            sonare_codec::Format::Opus => "opus",
            sonare_codec::Format::Aac => "aac",
        }
        .to_owned()
    })
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
