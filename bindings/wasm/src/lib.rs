#![deny(unsafe_code)]
#![warn(clippy::all)]

use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen]
pub struct WavPcm {
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
}

#[wasm_bindgen]
pub struct StreamDecoder {
    inner: sonare_codec::StreamDecoder,
}

#[wasm_bindgen]
impl WavPcm {
    #[wasm_bindgen(getter)]
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    #[wasm_bindgen(getter)]
    pub fn channels(&self) -> u16 {
        self.channels
    }

    pub fn samples(&self) -> Vec<f32> {
        self.samples.clone()
    }
}

#[wasm_bindgen]
impl StreamDecoder {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn decode_stream(&mut self, chunk: &[u8]) -> Result<Option<WavPcm>, String> {
        self.inner
            .decode_stream(chunk)
            .map(|decoded| decoded.map(Into::into))
            .map_err(|err| err.to_string())
    }

    pub fn reset(&mut self) {
        self.inner.reset();
    }

    pub fn buffered_len(&self) -> usize {
        self.inner.buffered_len()
    }
}

impl Default for StreamDecoder {
    fn default() -> Self {
        Self {
            inner: sonare_codec::StreamDecoder::new(),
        }
    }
}

#[wasm_bindgen]
pub fn detect_format(input: &[u8]) -> Option<String> {
    if is_m4a_container(input) {
        return Some("m4a".to_owned());
    }
    sonare_codec::detect(input).map(|format| format!("{format:?}").to_ascii_lowercase())
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

#[wasm_bindgen]
pub fn encode_audio(
    format: &str,
    sample_rate: u32,
    channels: u16,
    samples: &[f32],
) -> Result<Vec<u8>, String> {
    let pcm = sonare_codec::AudioBuffer::new(sample_rate, channels, samples.to_vec())
        .map_err(|err| err.to_string())?;
    encode_by_name(format, &pcm)
}

#[wasm_bindgen]
pub fn encode_audio_production(
    format: &str,
    sample_rate: u32,
    channels: u16,
    samples: &[f32],
) -> Result<Vec<u8>, String> {
    let pcm = sonare_codec::AudioBuffer::new(sample_rate, channels, samples.to_vec())
        .map_err(|err| err.to_string())?;
    encode_by_name_with_mode(format, &pcm, sonare_codec::EncodeMode::ProductionOnly)
}

#[wasm_bindgen]
pub fn encode_wav(sample_rate: u32, channels: u16, samples: &[f32]) -> Result<Vec<u8>, String> {
    let pcm = sonare_codec::AudioBuffer::new(sample_rate, channels, samples.to_vec())
        .map_err(|err| err.to_string())?;
    sonare_codec::encode(sonare_codec::Format::Wav, &pcm).map_err(|err| err.to_string())
}

#[wasm_bindgen]
pub fn encode_flac(sample_rate: u32, channels: u16, samples: &[f32]) -> Result<Vec<u8>, String> {
    let pcm = sonare_codec::AudioBuffer::new(sample_rate, channels, samples.to_vec())
        .map_err(|err| err.to_string())?;
    sonare_codec::encode(sonare_codec::Format::Flac, &pcm).map_err(|err| err.to_string())
}

#[wasm_bindgen]
pub fn encode_mp3(sample_rate: u32, channels: u16, samples: &[f32]) -> Result<Vec<u8>, String> {
    let pcm = sonare_codec::AudioBuffer::new(sample_rate, channels, samples.to_vec())
        .map_err(|err| err.to_string())?;
    sonare_codec::encode(sonare_codec::Format::Mp3, &pcm).map_err(|err| err.to_string())
}

#[wasm_bindgen]
pub fn encode_aac(sample_rate: u32, channels: u16, samples: &[f32]) -> Result<Vec<u8>, String> {
    let pcm = sonare_codec::AudioBuffer::new(sample_rate, channels, samples.to_vec())
        .map_err(|err| err.to_string())?;
    sonare_codec::encode(sonare_codec::Format::Aac, &pcm).map_err(|err| err.to_string())
}

#[wasm_bindgen]
pub fn encode_m4a(sample_rate: u32, channels: u16, samples: &[f32]) -> Result<Vec<u8>, String> {
    let aac = encode_aac(sample_rate, channels, samples)?;
    sonare_codec::mux_aac_adts_as_m4a(&aac).map_err(|err| err.to_string())
}

#[wasm_bindgen]
pub fn demux_m4a_as_aac_adts(input: &[u8]) -> Result<Vec<u8>, String> {
    sonare_codec::demux_m4a_as_aac_adts(input).map_err(|err| err.to_string())
}

impl From<sonare_codec::AudioBuffer> for WavPcm {
    fn from(pcm: sonare_codec::AudioBuffer) -> Self {
        Self {
            sample_rate: pcm.sample_rate,
            channels: pcm.channels,
            samples: pcm.samples,
        }
    }
}

fn parse_format(format: &str) -> Result<sonare_codec::Format, String> {
    match format.to_ascii_lowercase().as_str() {
        "wav" => Ok(sonare_codec::Format::Wav),
        "flac" => Ok(sonare_codec::Format::Flac),
        "mp3" => Ok(sonare_codec::Format::Mp3),
        "vorbis" => Ok(sonare_codec::Format::Vorbis),
        "opus" => Ok(sonare_codec::Format::Opus),
        "aac" | "m4a" | "mp4" => Ok(sonare_codec::Format::Aac),
        _ => Err("unsupported format".to_owned()),
    }
}

fn encode_by_name(format: &str, pcm: &sonare_codec::AudioBuffer) -> Result<Vec<u8>, String> {
    encode_by_name_with_mode(format, pcm, sonare_codec::EncodeMode::Compatibility)
}

fn encode_by_name_with_mode(
    format: &str,
    pcm: &sonare_codec::AudioBuffer,
    mode: sonare_codec::EncodeMode,
) -> Result<Vec<u8>, String> {
    match format.to_ascii_lowercase().as_str() {
        "m4a" | "mp4" => {
            let aac = sonare_codec::encode_with_mode(sonare_codec::Format::Aac, pcm, mode)
                .map_err(|err| err.to_string())?;
            sonare_codec::mux_aac_adts_as_m4a(&aac).map_err(|err| err.to_string())
        }
        _ => {
            let format = parse_format(format)?;
            sonare_codec::encode_with_mode(format, pcm, mode).map_err(|err| err.to_string())
        }
    }
}

fn is_m4a_container(input: &[u8]) -> bool {
    input.len() >= 12
        && input.get(4..8) == Some(b"ftyp")
        && matches!(
            input.get(8..12),
            Some(b"M4A ") | Some(b"mp42") | Some(b"isom") | Some(b"iso2")
        )
}

#[cfg(test)]
mod tests {
    use super::{
        decode_aac, decode_audio, decode_m4a, decode_mp3, demux_m4a_as_aac_adts, detect_format,
        encode_aac, encode_audio, encode_audio_production, encode_m4a, encode_mp3, StreamDecoder,
    };

    #[test]
    fn unified_wav_api_roundtrips_pcm() {
        let samples = vec![0.0, 0.25, -0.25, 0.5];

        let encoded = encode_audio("wav", 44_100, 1, &samples).unwrap();
        let decoded = decode_audio(&encoded).unwrap();

        assert_eq!(detect_format(&encoded), Some("wav".to_owned()));
        assert_eq!(decoded.sample_rate(), 44_100);
        assert_eq!(decoded.channels(), 1);
        assert_eq!(decoded.samples().len(), samples.len());
    }

    #[test]
    fn unified_flac_api_roundtrips_pcm() {
        let samples = (0..128)
            .map(|sample| sample as f32 / 32_767.0)
            .collect::<Vec<_>>();

        let encoded = encode_audio("flac", 48_000, 1, &samples).unwrap();
        let decoded = decode_audio(&encoded).unwrap();

        assert_eq!(detect_format(&encoded), Some("flac".to_owned()));
        assert_eq!(decoded.sample_rate(), 48_000);
        assert_eq!(decoded.channels(), 1);
        assert_eq!(decoded.samples().len(), samples.len());
    }

    #[test]
    fn stream_decoder_buffers_until_complete_input() {
        let samples = vec![0.0, 0.25, -0.25, 0.5];
        let encoded = encode_audio("wav", 44_100, 1, &samples).unwrap();
        let split = encoded.len() - 2;
        let mut decoder = StreamDecoder::new();

        assert!(decoder.decode_stream(&encoded[..split]).unwrap().is_none());
        assert!(decoder.buffered_len() > 0);
        let decoded = decoder
            .decode_stream(&encoded[split..])
            .unwrap()
            .expect("complete stream should decode");

        assert_eq!(decoded.sample_rate(), 44_100);
        assert_eq!(decoded.channels(), 1);
        assert_eq!(decoded.samples().len(), samples.len());
        assert_eq!(decoder.buffered_len(), 0);
    }

    #[test]
    fn unified_aac_api_encodes_silent_pcm() {
        let samples = vec![0.0; 1024];

        let encoded = encode_audio("aac", 44_100, 1, &samples).unwrap();
        let decoded = decode_audio(&encoded).unwrap();

        assert_eq!(
            encode_audio_production("aac", 44_100, 1, &samples).unwrap(),
            encoded
        );
        assert_eq!(detect_format(&encoded), Some("aac".to_owned()));
        assert_eq!(decoded.sample_rate(), 44_100);
        assert_eq!(decoded.channels(), 1);
        assert_eq!(decode_aac(&encoded).unwrap().samples().len(), samples.len());
        assert_eq!(encode_aac(44_100, 1, &samples).unwrap(), encoded);
    }

    #[test]
    fn unified_aac_api_encodes_non_silent_pcm_scaffold() {
        let samples = (0..2048)
            .map(|sample| ((sample as f32) * 0.01).sin() * 0.25)
            .collect::<Vec<_>>();

        let encoded = encode_audio("aac", 44_100, 1, &samples).unwrap();
        let err = encode_audio_production("aac", 44_100, 1, &samples).unwrap_err();
        let decoded = decode_audio(&encoded).unwrap();

        assert_eq!(
            err,
            "unsupported codec feature: production non-silent lossy encode"
        );
        assert_eq!(detect_format(&encoded), Some("aac".to_owned()));
        assert_eq!(&encoded[..2], &[0xff, 0xf1]);
        assert_eq!(encode_aac(44_100, 1, &samples).unwrap(), encoded);
        assert_eq!(decoded.sample_rate(), 44_100);
        assert_eq!(decoded.channels(), 1);
        assert_eq!(decoded.samples().len(), 2048);
    }

    #[test]
    fn unified_mp3_api_encodes_silent_pcm() {
        let samples = vec![0.0; 1152];

        let encoded = encode_audio("mp3", 44_100, 1, &samples).unwrap();
        let decoded = decode_audio(&encoded).unwrap();

        assert_eq!(
            encode_audio_production("mp3", 44_100, 1, &samples).unwrap(),
            encoded
        );
        assert_eq!(detect_format(&encoded), Some("mp3".to_owned()));
        assert_eq!(&encoded[..2], &[0xff, 0xfb]);
        assert_eq!(decode_mp3(&encoded).unwrap().samples().len(), samples.len());
        assert_eq!(encode_mp3(44_100, 1, &samples).unwrap(), encoded);
        assert_eq!(decoded.sample_rate(), 44_100);
        assert_eq!(decoded.channels(), 1);
        assert_eq!(decoded.samples().len(), samples.len());
    }

    #[test]
    fn unified_mp3_api_encodes_non_silent_pcm_scaffold() {
        let samples = (0..2048)
            .map(|sample| ((sample as f32) * 0.01).sin() * 0.25)
            .collect::<Vec<_>>();

        let encoded = encode_audio("mp3", 44_100, 1, &samples).unwrap();
        let err = encode_audio_production("mp3", 44_100, 1, &samples).unwrap_err();
        let decoded = decode_audio(&encoded).unwrap();

        assert_eq!(
            err,
            "unsupported codec feature: production non-silent lossy encode"
        );
        assert_eq!(detect_format(&encoded), Some("mp3".to_owned()));
        assert_eq!(&encoded[..2], &[0xff, 0xfb]);
        assert_eq!(encode_mp3(44_100, 1, &samples).unwrap(), encoded);
        assert_eq!(decoded.sample_rate(), 44_100);
        assert_eq!(decoded.channels(), 1);
        assert_eq!(decoded.samples().len(), 2304);
    }

    #[test]
    fn unified_m4a_api_muxes_silent_aac() {
        let samples = vec![0.0; 1024];

        let encoded = encode_audio("m4a", 44_100, 1, &samples).unwrap();
        let decoded = decode_audio(&encoded).unwrap();

        assert_eq!(detect_format(&encoded), Some("m4a".to_owned()));
        assert!(encoded.windows(4).any(|window| window == b"ftyp"));
        assert_eq!(decode_m4a(&encoded).unwrap().samples().len(), samples.len());
        assert_eq!(encode_m4a(44_100, 1, &samples).unwrap(), encoded);
        assert_eq!(
            demux_m4a_as_aac_adts(&encoded).unwrap(),
            encode_aac(44_100, 1, &samples).unwrap()
        );
        assert_eq!(decoded.sample_rate(), 44_100);
        assert_eq!(decoded.channels(), 1);
        assert_eq!(decoded.samples().len(), samples.len());
    }

    #[test]
    fn unified_encode_rejects_unknown_format() {
        let err = encode_audio("unknown", 44_100, 1, &[0.0]).unwrap_err();

        assert_eq!(err, "unsupported format");
    }

    #[test]
    fn unified_encode_reports_unimplemented_lossy_encoders() {
        let samples = vec![0.0; 128];

        // Opus encoding depends on native libopus. Browser-targeted WASM builds
        // keep it unsupported, while native unit tests can see the encoder
        // through Cargo feature unification.
        match encode_audio("opus", 48_000, 1, &samples) {
            Err(err) => assert_eq!(err, "unsupported codec feature: Opus encode"),
            Ok(stream) => assert_eq!(&stream[..4], b"OggS"),
        }
        // Vorbis encoding relies on a native C library (libvorbis) that cannot
        // be compiled into the wasm bundle, so the wasm surface does not enable
        // the `vorbis` feature and reports it as unsupported. Workspace-wide
        // `--all-features` builds can unify the native encoder into the umbrella
        // via Cargo feature unification, in which case a real Ogg stream is
        // produced instead; accept either outcome.
        match encode_audio("vorbis", 48_000, 1, &samples) {
            Err(err) => assert_eq!(err, "unsupported format"),
            Ok(stream) => assert_eq!(&stream[..4], b"OggS"),
        }
    }
}
