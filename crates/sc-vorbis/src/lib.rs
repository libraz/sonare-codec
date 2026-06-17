#![deny(unsafe_code)]
#![warn(clippy::all)]

use std::num::{NonZeroU32, NonZeroU8};

use sc_core::{detect, AudioBuffer, Decoder, Encoder, Error, Format};
use vorbis_rs::{VorbisEncoderBuilder, VorbisError};

mod floor1;
mod lpc;
mod mdct;
mod oggpack;
mod window;

#[derive(Default)]
pub struct VorbisDecoder;

impl VorbisDecoder {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Decoder for VorbisDecoder {
    fn decode(&mut self, input: &[u8]) -> Result<AudioBuffer, Error> {
        decode(input)
    }

    fn decode_stream(&mut self, chunk: &[u8]) -> Result<Option<AudioBuffer>, Error> {
        decode(chunk).map(Some)
    }
}

#[derive(Default)]
pub struct VorbisEncoder;

impl VorbisEncoder {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Encoder for VorbisEncoder {
    fn encode(&mut self, pcm: &AudioBuffer) -> Result<Vec<u8>, Error> {
        encode(pcm)
    }
}

pub fn decode(input: &[u8]) -> Result<AudioBuffer, Error> {
    if detect(input) != Some(Format::Vorbis) {
        return Err(Error::UnsupportedFormat);
    }
    sc_decode::decode(input)
}

/// Number of frames fed to libvorbis per analysis block. libvorbis accepts
/// arbitrary sizes; a bounded block keeps peak memory independent of input
/// length and matches the library's "smaller blocks when in doubt" guidance.
const ANALYSIS_BLOCK_FRAMES: usize = 4096;

/// Encodes interleaved PCM into an Ogg Vorbis stream using libvorbis (aoTuV)
/// in quality-based VBR mode.
///
/// Vorbis is lossy: the decoded signal approximates the input within a
/// perceptual tolerance and must never be compared bit-exactly against the
/// source.
pub fn encode(pcm: &AudioBuffer) -> Result<Vec<u8>, Error> {
    let sample_rate = NonZeroU32::new(pcm.sample_rate)
        .ok_or(Error::InvalidPcm("sample rate must be non-zero"))?;
    let channels = u8::try_from(pcm.channels)
        .ok()
        .and_then(NonZeroU8::new)
        .ok_or(Error::InvalidPcm("unsupported Vorbis channel count"))?;

    let channel_count = usize::from(pcm.channels);
    let frames = pcm.frames();

    // De-interleave into one planar buffer per channel, as libvorbis expects.
    let mut planar = vec![Vec::with_capacity(frames); channel_count];
    for frame in pcm.samples.chunks_exact(channel_count) {
        for (channel, &sample) in frame.iter().enumerate() {
            planar[channel].push(sample);
        }
    }

    let mut sink = Vec::new();
    let mut builder =
        VorbisEncoderBuilder::new(sample_rate, channels, &mut sink).map_err(map_vorbis_error)?;
    let mut encoder = builder.build().map_err(map_vorbis_error)?;

    let mut start = 0;
    while start < frames {
        let end = (start + ANALYSIS_BLOCK_FRAMES).min(frames);
        let block: Vec<&[f32]> = planar.iter().map(|channel| &channel[start..end]).collect();
        encoder
            .encode_audio_block(&block)
            .map_err(map_vorbis_error)?;
        start = end;
    }

    // Flush trailing blocks and the end-of-stream marker, releasing the sink.
    encoder.finish().map_err(map_vorbis_error)?;
    Ok(sink)
}

fn map_vorbis_error(_err: VorbisError) -> Error {
    Error::InvalidInput("Vorbis encode failed")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine_pcm(sample_rate: u32, channels: u16, frames: usize, freq: f32) -> AudioBuffer {
        let mut samples = Vec::with_capacity(frames * usize::from(channels));
        for frame in 0..frames {
            let t = frame as f32 / sample_rate as f32;
            let value = (2.0 * std::f32::consts::PI * freq * t).sin() * 0.5;
            for _ in 0..channels {
                samples.push(value);
            }
        }
        AudioBuffer::new(sample_rate, channels, samples).expect("pcm")
    }

    #[test]
    fn rejects_non_vorbis_input() {
        let err = decode(b"not vorbis").expect_err("format");
        assert!(matches!(err, Error::UnsupportedFormat));
    }

    #[test]
    fn encodes_ogg_vorbis_stream() {
        let pcm = sine_pcm(48_000, 1, 4800, 440.0);
        let encoded = encode(&pcm).expect("encode");
        // Ogg streams begin with the "OggS" capture pattern.
        assert_eq!(&encoded[..4], b"OggS");
        assert_eq!(detect(&encoded), Some(Format::Vorbis));
    }

    #[test]
    fn roundtrips_within_perceptual_tolerance() {
        let pcm = sine_pcm(48_000, 1, 9600, 440.0);
        let encoded = encode(&pcm).expect("encode");
        let decoded = decode(&encoded).expect("decode");

        assert_eq!(decoded.sample_rate, pcm.sample_rate);
        assert_eq!(decoded.channels, pcm.channels);
        // Lossy: never bit-exact. Assert the decoded tone carries real energy
        // and a length close to the source (codec delay aside).
        let rms = (decoded.samples.iter().map(|s| s * s).sum::<f32>()
            / decoded.samples.len().max(1) as f32)
            .sqrt();
        assert!(rms > 0.1, "decoded RMS too low: {rms}");
        let frame_delta = (decoded.frames() as isize - pcm.frames() as isize).unsigned_abs();
        assert!(frame_delta <= 2048, "frame count drifted: {frame_delta}");
    }

    #[test]
    fn encodes_stereo() {
        let pcm = sine_pcm(44_100, 2, 4410, 440.0);
        let encoded = encode(&pcm).expect("encode");
        let decoded = decode(&encoded).expect("decode");
        assert_eq!(decoded.channels, 2);
    }

    #[test]
    fn rejects_unrepresentable_channel_count() {
        let pcm = AudioBuffer {
            sample_rate: 48_000,
            channels: 300,
            samples: vec![0.0; 600],
        };
        let err = encode(&pcm).expect_err("channels");
        assert!(matches!(err, Error::InvalidPcm(_)));
    }

    #[test]
    fn decodes_ffmpeg_generated_ogg_vorbis_when_available() {
        let Ok(ffmpeg) = std::env::var("SONARE_FFMPEG") else {
            return;
        };
        let path = std::env::temp_dir().join(format!(
            "sonare-codec-vorbis-smoke-{}.ogg",
            std::process::id()
        ));

        let status = std::process::Command::new(ffmpeg)
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-f",
                "lavfi",
                "-i",
                "sine=frequency=440:duration=0.05:sample_rate=48000",
                "-ac",
                "1",
                "-c:a",
                "libvorbis",
                "-y",
            ])
            .arg(&path)
            .status()
            .expect("run ffmpeg");
        assert!(status.success(), "ffmpeg failed with {status}");

        let bytes = std::fs::read(&path).expect("read vorbis");
        let _ = std::fs::remove_file(&path);
        let decoded = decode(&bytes).expect("decode vorbis");
        assert_eq!(decoded.sample_rate, 48_000);
        assert_eq!(decoded.channels, 1);
        assert!(!decoded.samples.is_empty());
        assert!(decoded.samples.iter().any(|sample| sample.abs() > 0.0001));
    }
}
