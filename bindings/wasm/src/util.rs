use super::*;

impl From<sonare_codec::AudioBuffer> for WavPcm {
    fn from(pcm: sonare_codec::AudioBuffer) -> Self {
        Self {
            sample_rate: pcm.sample_rate,
            channels: pcm.channels,
            samples: pcm.samples,
        }
    }
}

pub(crate) fn pcm_from_samples(
    sample_rate: u32,
    channels: u16,
    samples: &[f32],
) -> Result<sonare_codec::AudioBuffer, String> {
    sonare_codec::AudioBuffer::new(sample_rate, channels, samples.to_vec())
        .map_err(|err| err.to_string())
}

pub(crate) fn encode_format(
    sample_rate: u32,
    channels: u16,
    samples: &[f32],
    format: sonare_codec::Format,
) -> Result<Vec<u8>, String> {
    let pcm = pcm_from_samples(sample_rate, channels, samples)?;
    sonare_codec::encode(format, &pcm).map_err(|err| err.to_string())
}

pub(crate) fn encode_by_name(
    format: &str,
    pcm: &sonare_codec::AudioBuffer,
) -> Result<Vec<u8>, String> {
    sonare_codec::bindings_support::encode_by_name(format, pcm).map_err(|err| err.to_string())
}

pub(crate) fn encode_by_name_with_mode(
    format: &str,
    pcm: &sonare_codec::AudioBuffer,
    mode: sonare_codec::EncodeMode,
) -> Result<Vec<u8>, String> {
    sonare_codec::bindings_support::encode_by_name_with_mode(format, pcm, mode)
        .map_err(|err| err.to_string())
}

pub(crate) fn is_m4a_container(input: &[u8]) -> bool {
    sonare_codec::bindings_support::is_m4a_container(input)
}
