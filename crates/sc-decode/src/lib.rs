#![deny(unsafe_code)]
#![warn(clippy::all)]

use std::io::Cursor;

use sc_core::{AudioBuffer, Decoder, Error};
use symphonia::core::audio::sample::Sample;
use symphonia::core::codecs::audio::AudioDecoderOptions;
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::FormatOptions;
use symphonia::core::formats::TrackType;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;

/// Upper bound on buffered partial input before a stream is rejected, so a
/// never-completing or garbage stream cannot grow the buffer without limit.
const MAX_STREAM_BUFFER: usize = 64 << 20;

#[derive(Default)]
pub struct SymphoniaDecoder {
    pending: Vec<u8>,
}

impl SymphoniaDecoder {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl Decoder for SymphoniaDecoder {
    fn decode(&mut self, input: &[u8]) -> Result<AudioBuffer, Error> {
        decode(input)
    }

    fn decode_stream(&mut self, chunk: &[u8]) -> Result<Option<AudioBuffer>, Error> {
        if chunk.is_empty() && self.pending.is_empty() {
            return Ok(None);
        }
        if self.pending.len().saturating_add(chunk.len()) > MAX_STREAM_BUFFER {
            self.pending.clear();
            return Err(Error::InvalidInput("stream exceeded maximum buffered size"));
        }
        self.pending.extend_from_slice(chunk);
        match decode(&self.pending) {
            Ok(buffer) => {
                self.pending.clear();
                Ok(Some(buffer))
            }
            Err(err) if is_incomplete_stream_error(&err) => Ok(None),
            Err(err) => {
                // Terminal error: drop the buffer so the next chunk starts fresh
                // instead of re-decoding (and re-failing on) a growing buffer.
                self.pending.clear();
                Err(err)
            }
        }
    }
}

pub fn decode(input: &[u8]) -> Result<AudioBuffer, Error> {
    let cursor = Cursor::new(input.to_vec());
    let media_source = Box::new(cursor);
    let media_stream = MediaSourceStream::new(media_source, Default::default());

    let hint = Hint::new();
    let format_options = FormatOptions::default();
    let metadata_options = MetadataOptions::default();
    let decoder_options = AudioDecoderOptions::default();
    let mut format = symphonia::default::get_probe()
        .probe(&hint, media_stream, format_options, metadata_options)
        .map_err(map_probe_error)?;

    let track = format
        .default_track(TrackType::Audio)
        .ok_or(Error::UnsupportedFormat)?;
    let track_id = track.id;
    let codec_params = track
        .codec_params
        .as_ref()
        .ok_or(Error::UnsupportedFormat)?
        .audio()
        .ok_or(Error::UnsupportedFormat)?;
    let mut decoder = symphonia::default::get_codecs()
        .make_audio_decoder(codec_params, &decoder_options)
        .map_err(map_decode_setup_error)?;

    let mut sample_rate = codec_params.sample_rate.unwrap_or(0);
    let mut channels = codec_params
        .channels
        .as_ref()
        .map(|channels| channels.count())
        .unwrap_or(0);
    let mut samples = Vec::new();
    let mut saw_packet = false;
    let mut decoded_any = false;

    loop {
        let packet = match format.next_packet() {
            Ok(Some(packet)) => packet,
            Ok(None) => break,
            Err(SymphoniaError::ResetRequired) => break,
            Err(err) => return Err(map_runtime_error(err)),
        };
        if packet.track_id != track_id {
            continue;
        }
        saw_packet = true;

        let decoded = match decoder.decode(&packet) {
            Ok(decoded) => decoded,
            Err(SymphoniaError::DecodeError(_)) | Err(SymphoniaError::IoError(_)) => continue,
            Err(err) => return Err(map_runtime_error(err)),
        };
        decoded_any = true;

        let spec = decoded.spec();
        if sample_rate == 0 {
            sample_rate = spec.rate();
        }
        if channels == 0 {
            channels = spec.channels().count();
        }
        if spec.rate() != sample_rate || spec.channels().count() != channels {
            return Err(Error::UnsupportedFeature(
                "changing audio stream parameters",
            ));
        }

        let offset = samples.len();
        samples.resize(offset + decoded.samples_interleaved(), f32::MID);
        decoded.copy_to_slice_interleaved(&mut samples[offset..]);
    }

    // If the track held packets but every one failed to decode, report that
    // precisely rather than letting an empty buffer surface as a generic
    // `InvalidPcm` from `AudioBuffer::new`.
    if saw_packet && !decoded_any {
        return Err(Error::InvalidInput("no decodable audio frames"));
    }

    let channels = u16::try_from(channels).map_err(|_| Error::InvalidInput("too many channels"))?;
    AudioBuffer::new(sample_rate, channels, samples)
}

fn map_probe_error(err: SymphoniaError) -> Error {
    match err {
        SymphoniaError::IoError(io_err) if io_err.kind() == std::io::ErrorKind::UnexpectedEof => {
            Error::Incomplete
        }
        SymphoniaError::IoError(_) | SymphoniaError::DecodeError(_) => {
            Error::InvalidInput("media probe failed")
        }
        SymphoniaError::LimitError(_) => Error::InvalidInput("media exceeds decoder limits"),
        SymphoniaError::SeekError(_) => Error::InvalidInput("media seek failed"),
        SymphoniaError::ResetRequired | SymphoniaError::Unsupported(_) => Error::UnsupportedFormat,
        _ => Error::InvalidInput("media probe failed"),
    }
}

fn map_decode_setup_error(err: SymphoniaError) -> Error {
    match err {
        SymphoniaError::Unsupported(_) => Error::UnsupportedFormat,
        SymphoniaError::IoError(io_err) if io_err.kind() == std::io::ErrorKind::UnexpectedEof => {
            Error::Incomplete
        }
        SymphoniaError::IoError(_)
        | SymphoniaError::DecodeError(_)
        | SymphoniaError::ResetRequired => Error::InvalidInput("audio decoder setup failed"),
        _ => Error::InvalidInput("audio decoder setup failed"),
    }
}

fn map_runtime_error(err: SymphoniaError) -> Error {
    match err {
        SymphoniaError::IoError(io_err) if io_err.kind() == std::io::ErrorKind::UnexpectedEof => {
            Error::Incomplete
        }
        SymphoniaError::IoError(_) | SymphoniaError::DecodeError(_) => {
            Error::InvalidInput("audio decode failed")
        }
        SymphoniaError::LimitError(_) => Error::InvalidInput("media exceeds decoder limits"),
        SymphoniaError::SeekError(_) => Error::InvalidInput("media seek failed"),
        SymphoniaError::Unsupported(_) => Error::UnsupportedFormat,
        SymphoniaError::ResetRequired => Error::UnsupportedFeature("audio track reset"),
        _ => Error::InvalidInput("audio decode failed"),
    }
}

fn is_incomplete_stream_error(err: &Error) -> bool {
    matches!(err, Error::Incomplete)
}

#[cfg(test)]
mod tests {
    use sc_core::Decoder;

    use super::{decode, SymphoniaDecoder};

    #[test]
    fn decodes_minimal_wav() {
        let wav = minimal_wav();

        let decoded = decode(&wav).unwrap();

        assert_eq!(decoded.sample_rate, 8_000);
        assert_eq!(decoded.channels, 1);
        assert_eq!(decoded.frames(), 2);
        assert_eq!(decoded.samples, vec![0.0, 0.5]);
    }

    #[test]
    fn stream_decoder_buffers_until_complete_input() {
        let wav = minimal_wav();
        let split = wav.len() - 2;
        let mut decoder = SymphoniaDecoder::new();

        assert!(decoder.decode_stream(&wav[..split]).unwrap().is_none());
        let decoded = decoder
            .decode_stream(&wav[split..])
            .unwrap()
            .expect("complete stream should decode");

        assert_eq!(decoded.frames(), 2);
    }

    fn minimal_wav() -> Vec<u8> {
        let mut wav = Vec::new();
        wav.extend_from_slice(b"RIFF");
        wav.extend_from_slice(&40_u32.to_le_bytes());
        wav.extend_from_slice(b"WAVEfmt ");
        wav.extend_from_slice(&16_u32.to_le_bytes());
        wav.extend_from_slice(&1_u16.to_le_bytes());
        wav.extend_from_slice(&1_u16.to_le_bytes());
        wav.extend_from_slice(&8_000_u32.to_le_bytes());
        wav.extend_from_slice(&16_000_u32.to_le_bytes());
        wav.extend_from_slice(&2_u16.to_le_bytes());
        wav.extend_from_slice(&16_u16.to_le_bytes());
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&4_u32.to_le_bytes());
        wav.extend_from_slice(&0_i16.to_le_bytes());
        wav.extend_from_slice(&16_384_i16.to_le_bytes());
        wav
    }
}
