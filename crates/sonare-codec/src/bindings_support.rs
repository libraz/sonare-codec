//! Helpers shared by the wasm and python bindings.
//!
//! Both bindings previously carried their own copies of format-name parsing,
//! encode-by-name (with the M4A/MP4 container wrapper), container detection, and
//! AAC scale-factor scaffolding. The copies had already drifted (different error
//! text, and the python M4A path lacked the `aac`-feature guard the wasm one
//! had). Centralizing them here keeps the surfaces identical and the M4A routing
//! defensively gated.

use crate::{AudioBuffer, EncodeMode, Error, Format};

/// Parses a case-insensitive container/codec name into a [`Format`].
///
/// `m4a`/`mp4` map to [`Format::Aac`] because the MP4 container wrapper rides on
/// the AAC codec; [`encode_by_name`] adds the muxing step for those names.
pub fn parse_format(format: &str) -> Result<Format, Error> {
    match format.to_ascii_lowercase().as_str() {
        "wav" => Ok(Format::Wav),
        "flac" => Ok(Format::Flac),
        "mp3" => Ok(Format::Mp3),
        "vorbis" => Ok(Format::Vorbis),
        "opus" => Ok(Format::Opus),
        "aac" | "m4a" | "mp4" => Ok(Format::Aac),
        _ => Err(Error::UnsupportedFormat),
    }
}

fn requests_m4a_container(format: &str) -> bool {
    matches!(format.to_ascii_lowercase().as_str(), "m4a" | "mp4")
}

/// Encodes PCM by case-insensitive format name; `m4a`/`mp4` produce a container.
pub fn encode_by_name(format: &str, pcm: &AudioBuffer) -> Result<Vec<u8>, Error> {
    encode_by_name_with_mode(format, pcm, EncodeMode::Compatibility)
}

/// Like [`encode_by_name`] with an explicit [`EncodeMode`].
pub fn encode_by_name_with_mode(
    format: &str,
    pcm: &AudioBuffer,
    mode: EncodeMode,
) -> Result<Vec<u8>, Error> {
    if requests_m4a_container(format) {
        return encode_m4a_with_mode(pcm, mode);
    }
    crate::encode_with_mode(parse_format(format)?, pcm, mode)
}

#[cfg(feature = "aac")]
fn encode_m4a_with_mode(pcm: &AudioBuffer, mode: EncodeMode) -> Result<Vec<u8>, Error> {
    let aac = crate::encode_with_mode(Format::Aac, pcm, mode)?;
    crate::mux_aac_adts_as_m4a(&aac)
}

#[cfg(not(feature = "aac"))]
fn encode_m4a_with_mode(_pcm: &AudioBuffer, _mode: EncodeMode) -> Result<Vec<u8>, Error> {
    // Guard the M4A path so a binding built without `aac` reports an actionable
    // error instead of silently mis-routing the request to an unsupported codec.
    Err(Error::UnsupportedFeature(
        "M4A/MP4 container output requires the \"aac\" cargo feature",
    ))
}

/// Whether the bytes are an MP4/M4A audio container, using the same brand
/// detection as [`detect`](crate::detect) so the bindings never disagree with it.
#[must_use]
pub fn is_m4a_container(input: &[u8]) -> bool {
    sc_core::is_mp4_audio_container(input)
}

/// Returns the AAC-LC `max_sfb` (last scale-factor band index) for these offsets.
#[cfg(feature = "aac")]
pub fn aac_offsets_max_sfb(offsets: &[usize]) -> Result<u8, Error> {
    u8::try_from(offsets.len().saturating_sub(1))
        .map_err(|_| Error::InvalidInput("AAC-LC scale-factor band count exceeds max_sfb range"))
}

/// Builds constant per-frame AAC scale factors covering the whole PCM buffer.
///
/// The AAC global gain is a `u8`, so it always fits in `i16` without clamping.
#[cfg(feature = "aac")]
#[must_use]
pub fn constant_aac_scale_factors_by_frame(
    pcm: &AudioBuffer,
    global_gain: u8,
    band_count: usize,
) -> Vec<Vec<i16>> {
    let frame_count = pcm.samples.len().div_ceil(usize::from(pcm.channels) * 1024);
    let scale_factor = i16::from(global_gain);
    (0..frame_count)
        .map(|_| vec![scale_factor; band_count])
        .collect()
}
