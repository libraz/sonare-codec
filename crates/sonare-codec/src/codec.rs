use super::*;

pub fn decode(input: &[u8]) -> Result<AudioBuffer, Error> {
    decode_impl(input)
}

/// Controls whether `encode_with_mode` may return experimental codec output.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EncodeMode {
    /// Preserve the regular `encode` behavior, including documented experimental scaffolds.
    Compatibility,
    /// Reject outputs that are not yet production-grade for non-silent lossy encoders.
    ProductionOnly,
}

/// Upper bound on buffered partial input before a stream is rejected, so a
/// never-completing or garbage stream cannot grow the buffer without limit.
const MAX_STREAM_BUFFER: usize = 64 << 20;

/// Stateful decoder that buffers chunks until a complete audio stream decodes.
#[derive(Default)]
pub struct StreamDecoder {
    pending: Vec<u8>,
}

impl StreamDecoder {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends a chunk and returns PCM once the buffered input forms a complete stream.
    pub fn decode_stream(&mut self, chunk: &[u8]) -> Result<Option<AudioBuffer>, Error> {
        if chunk.is_empty() && self.pending.is_empty() {
            return Ok(None);
        }
        if self.pending.len().saturating_add(chunk.len()) > MAX_STREAM_BUFFER {
            self.pending.clear();
            return Err(Error::InvalidInput("stream exceeded maximum buffered size"));
        }
        self.pending.extend_from_slice(chunk);
        match decode(&self.pending) {
            Ok(pcm) => {
                self.pending.clear();
                Ok(Some(pcm))
            }
            Err(err) if is_incomplete_stream_error(&err) => Ok(None),
            Err(err) => {
                // A hard decode error is terminal for this stream; drop the
                // buffer so the next chunk starts fresh instead of re-decoding
                // (and re-failing on) an ever-growing buffer.
                self.pending.clear();
                Err(err)
            }
        }
    }

    /// Drops any buffered partial input.
    pub fn reset(&mut self) {
        self.pending.clear();
    }

    #[must_use]
    pub fn buffered_len(&self) -> usize {
        self.pending.len()
    }
}

/// Encodes interleaved PCM in the requested format.
pub fn encode(format: Format, pcm: &AudioBuffer) -> Result<Vec<u8>, Error> {
    encode_with_mode(format, pcm, EncodeMode::Compatibility)
}

/// Encodes interleaved PCM while applying a caller-selected stability policy.
///
/// `ProductionOnly` accepts the currently production-grade paths and rejects
/// non-silent lossy output that still relies on unsupported scaffold logic.
pub fn encode_with_mode(
    format: Format,
    pcm: &AudioBuffer,
    mode: EncodeMode,
) -> Result<Vec<u8>, Error> {
    if mode == EncodeMode::ProductionOnly {
        if let Some(reason) = production_encode_rejection_reason(format, pcm) {
            return Err(Error::UnsupportedFeature(reason));
        }
    }

    match format {
        Format::Wav => encode_wav_impl(pcm),
        Format::Flac => encode_flac_impl(pcm),
        Format::Mp3 => encode_mp3_impl(pcm),
        Format::Vorbis => encode_vorbis_impl(pcm),
        Format::Opus => encode_opus_impl(pcm),
        Format::Aac => encode_aac_impl(pcm),
    }
}

pub(crate) fn production_encode_rejection_reason(
    format: Format,
    pcm: &AudioBuffer,
) -> Option<&'static str> {
    if is_silent_pcm(pcm) {
        return None;
    }

    match format {
        Format::Mp3 if !is_mp3_non_silent_production_candidate(pcm) => Some(
            "production MP3 encode currently supports mono/stereo MPEG-1 (32/44.1/48 kHz) and MPEG-2 LSF (16/22.05/24 kHz) sample rates only",
        ),
        Format::Aac if !is_aac_non_silent_production_candidate(pcm) => Some(
            "production AAC-LC encode currently supports mono/stereo 7.35/8/11.025/12/16/22.05/24/32/44.1/48/64/88.2/96kHz only",
        ),
        _ => None,
    }
}

pub(crate) fn is_mp3_non_silent_production_candidate(pcm: &AudioBuffer) -> bool {
    matches!(pcm.channels, 1 | 2)
        && matches!(
            pcm.sample_rate,
            // MPEG-1 (ISO/IEC 11172-3) and MPEG-2 LSF (ISO/IEC 13818-3).
            16_000 | 22_050 | 24_000 | 32_000 | 44_100 | 48_000
        )
}

#[cfg(feature = "aac")]
pub(crate) fn is_aac_non_silent_production_candidate(pcm: &AudioBuffer) -> bool {
    matches!(pcm.channels, 1 | 2)
        && sc_aac::aac_lc_long_window_scale_factor_band_offsets(pcm.sample_rate).is_some()
}

#[cfg(not(feature = "aac"))]
pub(crate) fn is_aac_non_silent_production_candidate(_pcm: &AudioBuffer) -> bool {
    false
}

pub(crate) fn is_silent_pcm(pcm: &AudioBuffer) -> bool {
    pcm.samples.iter().all(|sample| *sample == 0.0)
}

pub(crate) fn is_incomplete_stream_error(err: &Error) -> bool {
    matches!(err, Error::InvalidInput(reason) if reason.contains("truncated"))
}

#[cfg(all(feature = "decode", not(feature = "aac")))]
pub(crate) fn decode_impl(input: &[u8]) -> Result<AudioBuffer, Error> {
    match sc_decode::decode(input) {
        Err(Error::UnsupportedFormat) => decode_mp3_fallback(input)
            .or_else(|| decode_opus_fallback(input))
            .unwrap_or(Err(Error::UnsupportedFormat)),
        result => result,
    }
}

#[cfg(all(feature = "decode", feature = "aac"))]
pub(crate) fn decode_impl(input: &[u8]) -> Result<AudioBuffer, Error> {
    if detect(input) == Some(Format::Aac) || is_m4a_container(input) {
        if let Ok(decoded) = sc_aac::decode(input) {
            return Ok(decoded);
        }
    }

    match sc_decode::decode(input) {
        Err(err) => decode_mp3_fallback(input)
            .or_else(|| decode_opus_fallback(input))
            .unwrap_or_else(|| {
                if detect(input) == Some(Format::Aac) {
                    sc_aac::decode(input)
                } else {
                    Err(err)
                }
            }),
        result => result,
    }
}

#[cfg(feature = "decode")]
pub(crate) fn is_m4a_container(input: &[u8]) -> bool {
    input.len() >= 12
        && input.get(4..8) == Some(b"ftyp")
        && matches!(
            input.get(8..12),
            Some(b"M4A ") | Some(b"mp42") | Some(b"isom") | Some(b"iso2")
        )
}

#[cfg(all(feature = "decode", feature = "mp3"))]
pub(crate) fn decode_mp3_fallback(input: &[u8]) -> Option<Result<AudioBuffer, Error>> {
    sc_mp3::FrameHeader::parse(input)
        .is_ok()
        .then(|| sc_mp3::decode(input))
}

#[cfg(all(feature = "decode", not(feature = "mp3")))]
pub(crate) fn decode_mp3_fallback(_input: &[u8]) -> Option<Result<AudioBuffer, Error>> {
    None
}

#[cfg(all(feature = "decode", feature = "opus"))]
pub(crate) fn decode_opus_fallback(input: &[u8]) -> Option<Result<AudioBuffer, Error>> {
    (detect(input) == Some(Format::Opus)).then(|| sc_opus::decode(input))
}

#[cfg(all(feature = "decode", not(feature = "opus")))]
pub(crate) fn decode_opus_fallback(input: &[u8]) -> Option<Result<AudioBuffer, Error>> {
    // Symphonia is not built with Opus support, so surface an actionable error
    // (rather than a bare UnsupportedFormat) when the input is clearly Opus.
    (detect(input) == Some(Format::Opus)).then(|| {
        Err(Error::UnsupportedFeature(
            "Opus decode requires the \"opus\" cargo feature",
        ))
    })
}

#[cfg(not(feature = "decode"))]
pub(crate) fn decode_impl(input: &[u8]) -> Result<AudioBuffer, Error> {
    match detect(input) {
        Some(Format::Wav) => decode_wav(input),
        Some(Format::Flac) => decode_flac(input),
        Some(Format::Mp3) => decode_mp3(input),
        Some(Format::Vorbis) => decode_vorbis(input),
        Some(Format::Opus) => decode_opus(input),
        Some(Format::Aac) => decode_aac(input),
        None => Err(Error::UnsupportedFormat),
    }
}

#[cfg(feature = "wav")]
/// Decodes WAV bytes into interleaved PCM.
pub fn decode_wav(input: &[u8]) -> Result<AudioBuffer, Error> {
    sc_wav::decode(input)
}

#[cfg(not(feature = "wav"))]
/// Decodes WAV bytes into interleaved PCM.
pub fn decode_wav(_input: &[u8]) -> Result<AudioBuffer, Error> {
    Err(Error::UnsupportedFormat)
}

#[cfg(feature = "flac")]
/// Decodes FLAC bytes into interleaved PCM.
pub fn decode_flac(input: &[u8]) -> Result<AudioBuffer, Error> {
    sc_flac::decode(input)
}

#[cfg(not(feature = "flac"))]
/// Decodes FLAC bytes into interleaved PCM.
pub fn decode_flac(_input: &[u8]) -> Result<AudioBuffer, Error> {
    Err(Error::UnsupportedFormat)
}

#[cfg(feature = "decode")]
/// Decodes MP3 bytes into interleaved PCM.
pub fn decode_mp3(input: &[u8]) -> Result<AudioBuffer, Error> {
    if detect(input) != Some(Format::Mp3) {
        return Err(Error::UnsupportedFormat);
    }
    decode_impl(input)
}

#[cfg(all(feature = "mp3", not(feature = "decode")))]
/// Decodes MP3 bytes into interleaved PCM.
pub fn decode_mp3(input: &[u8]) -> Result<AudioBuffer, Error> {
    sc_mp3::decode(input)
}

#[cfg(all(not(feature = "mp3"), not(feature = "decode")))]
/// Decodes MP3 bytes into interleaved PCM.
pub fn decode_mp3(_input: &[u8]) -> Result<AudioBuffer, Error> {
    Err(Error::UnsupportedFormat)
}

#[cfg(all(feature = "decode", not(feature = "vorbis")))]
/// Decodes Vorbis bytes into interleaved PCM.
pub fn decode_vorbis(input: &[u8]) -> Result<AudioBuffer, Error> {
    if detect(input) != Some(Format::Vorbis) {
        return Err(Error::UnsupportedFormat);
    }
    sc_decode::decode(input)
}

#[cfg(feature = "vorbis")]
/// Decodes Vorbis bytes into interleaved PCM.
pub fn decode_vorbis(input: &[u8]) -> Result<AudioBuffer, Error> {
    sc_vorbis::decode(input)
}

#[cfg(all(not(feature = "vorbis"), not(feature = "decode")))]
/// Decodes Vorbis bytes into interleaved PCM.
pub fn decode_vorbis(_input: &[u8]) -> Result<AudioBuffer, Error> {
    Err(Error::UnsupportedFormat)
}

#[cfg(all(feature = "decode", not(feature = "opus")))]
/// Decodes Opus bytes into interleaved PCM.
///
/// The Symphonia decode backend is not built with Opus support, so Opus decode
/// requires enabling the first-party `opus` feature.
pub fn decode_opus(input: &[u8]) -> Result<AudioBuffer, Error> {
    if detect(input) != Some(Format::Opus) {
        return Err(Error::UnsupportedFormat);
    }
    Err(Error::UnsupportedFeature(
        "Opus decode requires the \"opus\" cargo feature",
    ))
}

#[cfg(feature = "opus")]
/// Decodes Opus bytes into interleaved PCM.
pub fn decode_opus(input: &[u8]) -> Result<AudioBuffer, Error> {
    if detect(input) != Some(Format::Opus) {
        return Err(Error::UnsupportedFormat);
    }
    sc_opus::decode(input)
}

#[cfg(all(not(feature = "opus"), not(feature = "decode")))]
/// Decodes Opus bytes into interleaved PCM.
pub fn decode_opus(_input: &[u8]) -> Result<AudioBuffer, Error> {
    Err(Error::UnsupportedFormat)
}

#[cfg(feature = "decode")]
/// Decodes AAC ADTS or M4A bytes into interleaved PCM.
pub fn decode_aac(input: &[u8]) -> Result<AudioBuffer, Error> {
    if detect(input) != Some(Format::Aac) && !is_m4a_container_for_decode(input) {
        return Err(Error::UnsupportedFormat);
    }
    decode_impl(input)
}

#[cfg(all(feature = "aac", not(feature = "decode")))]
/// Decodes AAC ADTS bytes into interleaved PCM.
pub fn decode_aac(input: &[u8]) -> Result<AudioBuffer, Error> {
    sc_aac::decode(input)
}

#[cfg(all(not(feature = "aac"), not(feature = "decode")))]
/// Decodes AAC ADTS bytes into interleaved PCM.
pub fn decode_aac(_input: &[u8]) -> Result<AudioBuffer, Error> {
    Err(Error::UnsupportedFormat)
}

#[cfg(feature = "decode")]
pub(crate) fn is_m4a_container_for_decode(input: &[u8]) -> bool {
    is_m4a_container(input)
}
