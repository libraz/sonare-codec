use super::*;

/// Encodes interleaved PCM into an Ogg Vorbis stream, validating the layout.
pub fn encode(pcm: &AudioBuffer) -> Result<Vec<u8>, Error> {
    if pcm.sample_rate == 0 {
        return Err(Error::InvalidPcm("sample rate must be non-zero"));
    }
    if pcm.channels == 0 || pcm.channels > 255 {
        return Err(Error::InvalidPcm("unsupported Vorbis channel count"));
    }
    let encoder = VorbisEncoder::new(pcm.channels, pcm.sample_rate);
    Ok(encoder.encode(pcm))
}
