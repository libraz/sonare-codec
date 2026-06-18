#![deny(unsafe_code)]
#![warn(clippy::all)]

pub use sc_core::{
    compare_pcm, compare_pcm_with_tolerance, detect, AudioBuffer, Decoder, Encoder, Error, Format,
    HuffmanCode, HuffmanEntry, PackedBits, PcmDiff, PcmTolerance,
};

#[cfg(feature = "mp3")]
pub use sc_mp3::{
    apply_big_value_region_tables_to_granule, assemble_layer3_frame_from_payloads,
    assemble_mpeg1_layer3_pcm_frame_with_perceptual_scale_factors_and_table_provider,
    assemble_mpeg1_layer3_pcm_frame_with_selected_scale_factors,
    assemble_mpeg1_layer3_pcm_frame_with_selected_scale_factors_and_table_provider,
    encode_mpeg1_layer3_pcm_frames_with_auto_step_and_table_provider,
    encode_mpeg1_layer3_pcm_frames_with_bitrate_and_table_provider,
    encode_mpeg1_layer3_pcm_frames_with_cbr_bitrate_and_table_provider,
    encode_mpeg1_layer3_pcm_frames_with_header_and_auto_step_and_table_provider,
    encode_mpeg1_layer3_pcm_frames_with_header_and_max_payload_bits_and_table_provider,
    encode_mpeg1_layer3_pcm_frames_with_header_and_perceptual_auto_step_and_table_provider,
    encode_mpeg1_layer3_pcm_frames_with_header_and_perceptual_max_payload_bits_and_table_provider,
    encode_mpeg1_layer3_pcm_frames_with_header_and_perceptual_scale_factors_and_table_provider,
    encode_mpeg1_layer3_pcm_frames_with_header_and_selected_scale_factors,
    encode_mpeg1_layer3_pcm_frames_with_header_and_selected_scale_factors_and_table_provider,
    encode_mpeg1_layer3_pcm_frames_with_max_payload_bits_and_table_provider,
    encode_mpeg1_layer3_pcm_frames_with_perceptual_active_cbr_bitrate_and_table_provider,
    encode_mpeg1_layer3_pcm_frames_with_perceptual_auto_step_and_table_provider,
    encode_mpeg1_layer3_pcm_frames_with_perceptual_bitrate_and_table_provider,
    encode_mpeg1_layer3_pcm_frames_with_perceptual_cbr_bitrate_and_table_provider,
    encode_mpeg1_layer3_pcm_frames_with_perceptual_max_payload_bits_and_table_provider,
    encode_mpeg1_layer3_pcm_frames_with_perceptual_scale_factors_and_table_provider,
    encode_mpeg1_layer3_pcm_frames_with_reservoir_and_table_provider,
    encode_mpeg1_layer3_pcm_frames_with_selected_scale_factors,
    encode_mpeg1_layer3_pcm_frames_with_selected_scale_factors_and_table_provider,
    experimental_unit_magnitude_table_provider, layer3_header_for_capacity,
    layer3_main_data_capacity_bits, layer3_main_data_capacity_bytes, mdct_long_block,
    mpeg1_layer3_global_gain_for_step, mpeg1_layer3_standard_big_value_table_provider,
    mpeg1_layer3_standard_table_provider, pack_big_value_pairs_with_region_tables_and_provider,
    pack_layer3_main_data_payloads, pack_mpeg1_layer3_long_quantized_spectrum_for_granule,
    pack_mpeg1_layer3_long_quantized_spectrum_with_selected_scale_factors_and_table_provider,
    pack_mpeg1_layer3_long_quantized_spectrum_with_selected_scale_factors_for_granule,
    pack_mpeg1_layer3_long_quantized_spectrum_with_table_provider,
    pack_mpeg1_layer3_long_scale_factors, pack_mpeg1_layer3_long_scale_factors_for_granule,
    pack_mpeg1_layer3_pcm_long_block_with_calibrated_gain_and_table_provider,
    pack_mpeg1_layer3_pcm_long_block_with_calibrated_gain_for_granule,
    select_big_value_region_tables, select_big_value_region_tables_by_bit_cost,
    select_big_value_table_by_bit_cost, select_count1_table_by_bit_cost,
    select_mpeg1_layer3_long_scale_factor_compress,
    select_mpeg1_layer3_long_scale_factors_for_quantized_spectrum,
    select_mpeg1_layer3_pcm_frame_perceptual_active_step_details_with_table_provider,
    select_mpeg1_layer3_pcm_frame_perceptual_active_step_with_table_provider,
    select_mpeg1_layer3_pcm_frame_perceptual_step_details_with_max_payload_bits_and_table_provider,
    select_mpeg1_layer3_pcm_frame_perceptual_step_details_with_table_provider,
    select_mpeg1_layer3_pcm_frame_perceptual_step_with_max_payload_bits_and_table_provider,
    select_mpeg1_layer3_pcm_frame_perceptual_step_with_table_provider,
    select_mpeg1_layer3_pcm_frame_step_details_with_max_payload_bits_and_table_provider,
    select_mpeg1_layer3_pcm_frame_step_details_with_table_provider,
    select_mpeg1_layer3_pcm_frame_step_with_max_payload_bits_and_table_provider,
    select_mpeg1_layer3_pcm_frame_step_with_table_provider,
    select_mpeg1_layer3_psychoacoustic_long_scale_factors, ChannelMode, FrameHeader, Layer,
    Layer3BigValueMagnitude, Layer3BigValuePair, Layer3BigValueRegionTableSelection,
    Layer3BigValueTableSelection, Layer3Count1MagnitudeQuad, Layer3Count1Quad,
    Layer3Count1TableSelection, Layer3EntropyTableProvider, Layer3EntropyTables,
    Layer3GranuleChannelInfo, Layer3PcmFrameStepSelection, Layer3ScaleFactorCompress,
    Layer3SideInfo, Layer3SpectralRegions, MpegVersion, MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT,
    MPEG1_LAYER3_PCM_STEP_CANDIDATES,
};

#[cfg(feature = "aac")]
pub use sc_aac::{
    aac_escape_table, aac_lc_adts_max_frame_len_for_bitrate, aac_lc_default_production_bitrate_bps,
    aac_lc_long_window_scale_factor_band_offsets, aac_lc_standard_spectral_tables,
    aac_scale_factor_delta_table, aac_scale_factor_delta_zero_table, aac_unsigned_pairs10_table,
    aac_unsigned_pairs7_table, aac_unsigned_pairs7_unit_magnitude_spectral_tables,
    aac_unsigned_pairs7_unit_magnitude_table, aac_unsigned_pairs8_table, aac_unsigned_pairs9_table,
    encode_pcm_mono_long_block_adts_by_bit_cost,
    encode_pcm_mono_long_block_adts_stream_by_bit_cost,
    encode_pcm_mono_long_block_adts_stream_with_auto_step_by_bit_cost,
    encode_pcm_mono_long_block_adts_stream_with_offsets_and_auto_step_by_bit_cost,
    encode_pcm_mono_long_block_adts_stream_with_offsets_and_scale_factors_and_bitrate_by_bit_cost,
    encode_pcm_mono_long_block_adts_stream_with_offsets_and_scale_factors_and_max_frame_len_by_bit_cost,
    encode_pcm_mono_long_block_adts_stream_with_offsets_and_scale_factors_by_bit_cost,
    encode_pcm_mono_long_block_adts_stream_with_offsets_and_selected_scale_factors_and_bitrate_by_bit_cost,
    encode_pcm_mono_long_block_adts_stream_with_offsets_and_selected_scale_factors_and_max_frame_len_by_bit_cost,
    encode_pcm_mono_long_block_adts_stream_with_offsets_and_selected_scale_factors_by_bit_cost,
    encode_pcm_mono_long_block_adts_stream_with_scale_factors,
    encode_pcm_mono_long_block_adts_stream_with_scale_factors_by_bit_cost,
    encode_pcm_mono_long_block_adts_stream_with_selected_scale_factors,
    encode_pcm_mono_long_block_adts_stream_with_selected_scale_factors_by_bit_cost,
    encode_pcm_mono_long_block_adts_with_scale_factors,
    encode_pcm_mono_long_block_adts_with_scale_factors_by_bit_cost,
    encode_pcm_mono_long_block_adts_with_selected_scale_factors,
    encode_pcm_mono_long_block_adts_with_selected_scale_factors_by_bit_cost,
    encode_pcm_stereo_long_block_adts_by_bit_cost,
    encode_pcm_stereo_long_block_adts_stream_by_bit_cost,
    encode_pcm_stereo_long_block_adts_stream_with_auto_step_by_bit_cost,
    encode_pcm_stereo_long_block_adts_stream_with_offsets_and_scale_factors_and_bitrate_by_bit_cost,
    encode_pcm_stereo_long_block_adts_stream_with_offsets_and_scale_factors_and_max_frame_len_by_bit_cost,
    encode_pcm_stereo_long_block_adts_stream_with_offsets_and_scale_factors_by_bit_cost,
    encode_pcm_stereo_long_block_adts_stream_with_offsets_and_selected_scale_factors_and_bitrate_by_bit_cost,
    encode_pcm_stereo_long_block_adts_stream_with_offsets_and_selected_scale_factors_and_max_frame_len_by_bit_cost,
    encode_pcm_stereo_long_block_adts_stream_with_offsets_and_selected_scale_factors_by_bit_cost,
    encode_pcm_stereo_long_block_adts_stream_with_scale_factors,
    encode_pcm_stereo_long_block_adts_stream_with_scale_factors_by_bit_cost,
    encode_pcm_stereo_long_block_adts_stream_with_selected_scale_factors,
    encode_pcm_stereo_long_block_adts_stream_with_selected_scale_factors_by_bit_cost,
    encode_pcm_stereo_long_block_adts_with_offsets_and_scale_factors_by_bit_cost,
    encode_pcm_stereo_long_block_adts_with_offsets_and_selected_scale_factors_by_bit_cost,
    encode_pcm_stereo_long_block_adts_with_scale_factors,
    encode_pcm_stereo_long_block_adts_with_scale_factors_by_bit_cost,
    encode_pcm_stereo_long_block_adts_with_selected_scale_factors,
    encode_pcm_stereo_long_block_adts_with_selected_scale_factors_by_bit_cost,
    encode_quantized_mono_adts, encode_quantized_mono_adts_by_bit_cost,
    encode_quantized_mono_adts_with_scale_factors,
    encode_quantized_mono_adts_with_scale_factors_by_bit_cost,
    encode_quantized_mono_adts_with_selected_scale_factors,
    encode_quantized_mono_adts_with_selected_scale_factors_by_bit_cost,
    encode_quantized_stereo_adts, encode_quantized_stereo_adts_by_bit_cost,
    encode_quantized_stereo_adts_with_scale_factors,
    encode_quantized_stereo_adts_with_scale_factors_by_bit_cost,
    encode_quantized_stereo_adts_with_selected_scale_factors,
    encode_quantized_stereo_adts_with_selected_scale_factors_by_bit_cost,
    experimental_aac_scale_factor_delta_table, experimental_unit_magnitude_spectral_tables,
    pack_quad_section_data_with_len,
    pack_sectioned_spectral_payload_with_sign_bits_and_scale_factor_bits_by_bit_cost,
    pack_sectioned_spectral_payload_with_sign_bits_by_bit_cost,
    pack_sectioned_spectral_quad_payload_with_sign_bits,
    pack_spectral_quad_sections_with_sign_bits, pack_spectral_quads_with_sign_bits,
    pack_spectral_quads_with_table, plan_sections_by_bit_cost, plan_sections_by_offsets,
    quantize_pcm_long_block, select_aac_lc_mono_pcm_frame_step_by_bit_cost,
    select_aac_lc_mono_pcm_frame_step_details_by_bit_cost,
    select_aac_lc_mono_pcm_frame_step_details_with_max_frame_len_by_bit_cost,
    select_aac_lc_mono_pcm_frame_step_details_with_offsets_and_max_frame_len_by_bit_cost,
    select_aac_lc_mono_pcm_frame_step_details_with_offsets_and_scale_factors_and_max_frame_len_by_bit_cost,
    select_aac_lc_mono_pcm_frame_step_details_with_offsets_and_scale_factors_by_bit_cost,
    select_aac_lc_mono_pcm_frame_step_details_with_offsets_by_bit_cost,
    select_aac_lc_mono_pcm_frame_step_with_max_frame_len_by_bit_cost,
    select_aac_lc_mono_pcm_frame_step_with_offsets_and_max_frame_len_by_bit_cost,
    select_aac_lc_mono_pcm_frame_step_with_offsets_and_scale_factors_and_max_frame_len_by_bit_cost,
    select_aac_lc_mono_pcm_frame_step_with_offsets_and_scale_factors_by_bit_cost,
    select_aac_lc_mono_pcm_frame_step_with_offsets_by_bit_cost,
    select_aac_lc_stereo_pcm_frame_step_by_bit_cost,
    select_aac_lc_stereo_pcm_frame_step_details_by_bit_cost,
    select_aac_lc_stereo_pcm_frame_step_details_with_max_frame_len_by_bit_cost,
    select_aac_lc_stereo_pcm_frame_step_details_with_offsets_and_max_frame_len_by_bit_cost,
    select_aac_lc_stereo_pcm_frame_step_details_with_offsets_and_scale_factors_and_max_frame_len_by_bit_cost,
    select_aac_lc_stereo_pcm_frame_step_details_with_offsets_by_bit_cost,
    select_aac_lc_stereo_pcm_frame_step_with_max_frame_len_by_bit_cost,
    select_aac_lc_stereo_pcm_frame_step_with_offsets_and_max_frame_len_by_bit_cost,
    select_aac_lc_stereo_pcm_frame_step_with_offsets_and_scale_factors_and_max_frame_len_by_bit_cost,
    select_aac_lc_stereo_pcm_frame_step_with_offsets_and_scale_factors_by_bit_cost,
    select_aac_lc_stereo_pcm_frame_step_with_offsets_by_bit_cost, select_codebook_by_bit_cost,
    select_scale_factors_for_quantized_bands, AacCodebook, AacLongBlockConfig,
    AacPcmFrameStepSelection, AacPcmLongBlockConfig, AacPcmStepSearchConfig, AacProfile,
    AacQuadSection, AacQuantizedChannel, AacQuantizedSpectrum, AacScaleFactorChannel,
    AacScaleFactorDelta, AacScaleFactorSequence, AacSection, AacSpectralMagnitudePair,
    AacSpectralMagnitudeQuad, AacSpectralMagnitudeQuadTables, AacSpectralMagnitudeTables,
    AacSpectralPair, AacSpectralQuad, AacSpectralTables, AdtsConfig,
    AAC_LC_48K_LONG_WINDOW_SCALE_FACTOR_BAND_OFFSETS, AAC_LC_PCM_STEP_CANDIDATES,
    AAC_SCALE_FACTOR_DELTA_ZERO_TABLE,
};

/// Decodes supported audio bytes into interleaved PCM.
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
        self.pending.extend_from_slice(chunk);
        match decode(&self.pending) {
            Ok(pcm) => {
                self.pending.clear();
                Ok(Some(pcm))
            }
            Err(err) if is_incomplete_stream_error(&err) => Ok(None),
            Err(err) => Err(err),
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

fn production_encode_rejection_reason(format: Format, pcm: &AudioBuffer) -> Option<&'static str> {
    if is_silent_pcm(pcm) {
        return None;
    }

    match format {
        Format::Mp3 if !is_mp3_non_silent_production_candidate(pcm) => {
            Some("production MP3 encode currently supports mono/stereo MPEG-1 sample rates only")
        }
        Format::Aac if !is_aac_non_silent_production_candidate(pcm) => Some(
            "production AAC-LC encode currently supports mono/stereo 7.35/8/11.025/12/16/22.05/24/32/44.1/48/64/88.2/96kHz only",
        ),
        _ => None,
    }
}

fn is_mp3_non_silent_production_candidate(pcm: &AudioBuffer) -> bool {
    matches!(pcm.channels, 1 | 2) && matches!(pcm.sample_rate, 32_000 | 44_100 | 48_000)
}

#[cfg(feature = "aac")]
fn is_aac_non_silent_production_candidate(pcm: &AudioBuffer) -> bool {
    matches!(pcm.channels, 1 | 2)
        && sc_aac::aac_lc_long_window_scale_factor_band_offsets(pcm.sample_rate).is_some()
}

#[cfg(not(feature = "aac"))]
fn is_aac_non_silent_production_candidate(_pcm: &AudioBuffer) -> bool {
    false
}

fn is_silent_pcm(pcm: &AudioBuffer) -> bool {
    pcm.samples.iter().all(|sample| *sample == 0.0)
}

fn is_incomplete_stream_error(err: &Error) -> bool {
    matches!(err, Error::InvalidInput(reason) if reason.contains("truncated"))
}

#[cfg(all(feature = "decode", not(feature = "aac")))]
fn decode_impl(input: &[u8]) -> Result<AudioBuffer, Error> {
    match sc_decode::decode(input) {
        Err(Error::UnsupportedFormat) => decode_mp3_fallback(input)
            .or_else(|| decode_opus_fallback(input))
            .unwrap_or(Err(Error::UnsupportedFormat)),
        result => result,
    }
}

#[cfg(all(feature = "decode", feature = "aac"))]
fn decode_impl(input: &[u8]) -> Result<AudioBuffer, Error> {
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

#[cfg(all(feature = "decode", feature = "aac"))]
fn is_m4a_container(input: &[u8]) -> bool {
    input.len() >= 12
        && input.get(4..8) == Some(b"ftyp")
        && matches!(
            input.get(8..12),
            Some(b"M4A ") | Some(b"mp42") | Some(b"isom") | Some(b"iso2")
        )
}

#[cfg(all(feature = "decode", feature = "mp3"))]
fn decode_mp3_fallback(input: &[u8]) -> Option<Result<AudioBuffer, Error>> {
    sc_mp3::FrameHeader::parse(input)
        .is_ok()
        .then(|| sc_mp3::decode(input))
}

#[cfg(all(feature = "decode", not(feature = "mp3")))]
fn decode_mp3_fallback(_input: &[u8]) -> Option<Result<AudioBuffer, Error>> {
    None
}

#[cfg(all(feature = "decode", feature = "opus"))]
fn decode_opus_fallback(input: &[u8]) -> Option<Result<AudioBuffer, Error>> {
    (detect(input) == Some(Format::Opus)).then(|| sc_opus::decode(input))
}

#[cfg(all(feature = "decode", not(feature = "opus")))]
fn decode_opus_fallback(_input: &[u8]) -> Option<Result<AudioBuffer, Error>> {
    None
}

#[cfg(not(feature = "decode"))]
fn decode_impl(input: &[u8]) -> Result<AudioBuffer, Error> {
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
/// Decodes Opus bytes into interleaved PCM when the decode backend supports it.
pub fn decode_opus(input: &[u8]) -> Result<AudioBuffer, Error> {
    if detect(input) != Some(Format::Opus) {
        return Err(Error::UnsupportedFormat);
    }
    sc_decode::decode(input)
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

#[cfg(all(feature = "decode", feature = "aac"))]
fn is_m4a_container_for_decode(input: &[u8]) -> bool {
    is_m4a_container(input)
}

#[cfg(all(feature = "decode", not(feature = "aac")))]
fn is_m4a_container_for_decode(input: &[u8]) -> bool {
    input.len() >= 12
        && input.get(4..8) == Some(b"ftyp")
        && matches!(
            input.get(8..12),
            Some(b"M4A ") | Some(b"mp42") | Some(b"isom") | Some(b"iso2")
        )
}

/// Encodes interleaved PCM as WAV.
#[cfg(feature = "wav")]
pub fn encode_wav(pcm: &AudioBuffer) -> Result<Vec<u8>, Error> {
    encode_wav_impl(pcm)
}

#[cfg(feature = "flac")]
pub fn encode_flac(pcm: &AudioBuffer) -> Result<Vec<u8>, Error> {
    encode_flac_impl(pcm)
}

#[cfg(feature = "mp3")]
pub fn encode_mp3(pcm: &AudioBuffer) -> Result<Vec<u8>, Error> {
    encode_mp3_impl(pcm)
}

#[cfg(feature = "vorbis")]
pub fn encode_vorbis(pcm: &AudioBuffer) -> Result<Vec<u8>, Error> {
    encode_vorbis_impl(pcm)
}

#[cfg(feature = "opus")]
pub fn encode_opus(pcm: &AudioBuffer) -> Result<Vec<u8>, Error> {
    encode_opus_impl(pcm)
}

#[cfg(feature = "aac")]
pub fn encode_aac(pcm: &AudioBuffer) -> Result<Vec<u8>, Error> {
    encode_aac_impl(pcm)
}

#[cfg(feature = "aac")]
pub fn encode_aac_adts_with_bitrate(
    pcm: &AudioBuffer,
    target_bitrate_bps: u32,
) -> Result<Vec<u8>, Error> {
    let channels = u8::try_from(pcm.channels)
        .map_err(|_| Error::InvalidInput("AAC channel count is unsupported"))?;
    let adts = AdtsConfig::aac_lc(pcm.sample_rate, channels);
    let offsets = aac_lc_long_window_scale_factor_band_offsets(pcm.sample_rate)
        .ok_or(Error::UnsupportedFeature("AAC-LC sample rate"))?;
    let channel_config = AacLongBlockConfig::new(180, (offsets.len() - 1) as u8);
    let scale_factors = vec![i16::from(channel_config.global_gain); offsets.len() - 1];
    let channel = AacScaleFactorChannel::new(channel_config, &scale_factors);
    let scale_factor_table = aac_scale_factor_delta_table();
    let spectral_tables = aac_unsigned_pairs7_unit_magnitude_spectral_tables();

    match pcm.channels {
        1 => encode_pcm_mono_long_block_adts_stream_with_offsets_and_scale_factors_and_bitrate_by_bit_cost(
            adts,
            channel,
            pcm,
            offsets,
            AAC_LC_PCM_STEP_CANDIDATES,
            target_bitrate_bps,
            &scale_factor_table,
            spectral_tables,
        ),
        2 => encode_pcm_stereo_long_block_adts_stream_with_offsets_and_scale_factors_and_bitrate_by_bit_cost(
            adts,
            channel,
            channel,
            pcm,
            offsets,
            AAC_LC_PCM_STEP_CANDIDATES,
            target_bitrate_bps,
            &scale_factor_table,
            spectral_tables,
        ),
        _ => Err(Error::InvalidInput(
            "AAC bitrate encode requires mono or stereo PCM",
        )),
    }
}

#[cfg(feature = "aac")]
pub fn encode_aac_adts_with_selected_scale_factors_and_bitrate(
    pcm: &AudioBuffer,
    target_bitrate_bps: u32,
) -> Result<Vec<u8>, Error> {
    let channels = u8::try_from(pcm.channels)
        .map_err(|_| Error::InvalidInput("AAC channel count is unsupported"))?;
    let adts = AdtsConfig::aac_lc(pcm.sample_rate, channels);
    let offsets = aac_lc_long_window_scale_factor_band_offsets(pcm.sample_rate)
        .ok_or(Error::UnsupportedFeature("AAC-LC sample rate"))?;
    let channel_config = AacLongBlockConfig::new(
        180,
        u8::try_from(offsets.len() - 1)
            .map_err(|_| Error::InvalidInput("AAC scale-factor band count exceeds u8"))?,
    );
    let scale_factor_table = aac_scale_factor_delta_table();
    let spectral_tables = aac_unsigned_pairs7_unit_magnitude_spectral_tables();

    match pcm.channels {
        1 => encode_pcm_mono_long_block_adts_stream_with_offsets_and_selected_scale_factors_and_bitrate_by_bit_cost(
            adts,
            channel_config,
            pcm,
            offsets,
            AAC_LC_PCM_STEP_CANDIDATES,
            target_bitrate_bps,
            &scale_factor_table,
            spectral_tables,
        ),
        2 => encode_pcm_stereo_long_block_adts_stream_with_offsets_and_selected_scale_factors_and_bitrate_by_bit_cost(
            adts,
            channel_config,
            channel_config,
            pcm,
            offsets,
            AAC_LC_PCM_STEP_CANDIDATES,
            target_bitrate_bps,
            &scale_factor_table,
            spectral_tables,
        ),
        _ => Err(Error::InvalidInput(
            "AAC selected-scale-factor bitrate encode requires mono or stereo PCM",
        )),
    }
}

#[cfg(feature = "aac")]
pub fn encode_m4a_with_bitrate(
    pcm: &AudioBuffer,
    target_bitrate_bps: u32,
) -> Result<Vec<u8>, Error> {
    mux_aac_adts_as_m4a(&encode_aac_adts_with_bitrate(pcm, target_bitrate_bps)?)
}

#[cfg(feature = "aac")]
pub fn encode_m4a_with_selected_scale_factors_and_bitrate(
    pcm: &AudioBuffer,
    target_bitrate_bps: u32,
) -> Result<Vec<u8>, Error> {
    mux_aac_adts_as_m4a(&encode_aac_adts_with_selected_scale_factors_and_bitrate(
        pcm,
        target_bitrate_bps,
    )?)
}

#[cfg(feature = "aac")]
pub fn frame_aac_adts(config: AdtsConfig, access_unit: &[u8]) -> Result<Vec<u8>, Error> {
    sc_aac::frame_adts(config, access_unit)
}

#[cfg(feature = "aac")]
pub fn frame_aac_adts_stream<'a>(
    config: AdtsConfig,
    access_units: impl IntoIterator<Item = &'a [u8]>,
) -> Result<Vec<u8>, Error> {
    sc_aac::frame_adts_stream(config, access_units)
}

#[cfg(feature = "aac")]
pub fn mux_aac_adts_as_m4a(adts: &[u8]) -> Result<Vec<u8>, Error> {
    sc_aac::mux_adts_as_m4a(adts)
}

#[cfg(feature = "aac")]
pub fn demux_m4a_as_aac_adts(input: &[u8]) -> Result<Vec<u8>, Error> {
    sc_aac::demux_m4a_as_adts(input)
}

#[cfg(feature = "wav")]
fn encode_wav_impl(pcm: &AudioBuffer) -> Result<Vec<u8>, Error> {
    sc_wav::encode(pcm)
}

#[cfg(not(feature = "wav"))]
fn encode_wav_impl(_pcm: &AudioBuffer) -> Result<Vec<u8>, Error> {
    Err(Error::UnsupportedFormat)
}

#[cfg(feature = "flac")]
fn encode_flac_impl(pcm: &AudioBuffer) -> Result<Vec<u8>, Error> {
    sc_flac::encode(pcm)
}

#[cfg(not(feature = "flac"))]
fn encode_flac_impl(_pcm: &AudioBuffer) -> Result<Vec<u8>, Error> {
    Err(Error::UnsupportedFormat)
}

#[cfg(feature = "mp3")]
fn encode_mp3_impl(pcm: &AudioBuffer) -> Result<Vec<u8>, Error> {
    sc_mp3::encode(pcm)
}

#[cfg(not(feature = "mp3"))]
fn encode_mp3_impl(_pcm: &AudioBuffer) -> Result<Vec<u8>, Error> {
    Err(Error::UnsupportedFormat)
}

#[cfg(feature = "vorbis")]
fn encode_vorbis_impl(pcm: &AudioBuffer) -> Result<Vec<u8>, Error> {
    sc_vorbis::encode(pcm)
}

#[cfg(not(feature = "vorbis"))]
fn encode_vorbis_impl(_pcm: &AudioBuffer) -> Result<Vec<u8>, Error> {
    Err(Error::UnsupportedFormat)
}

#[cfg(feature = "opus")]
fn encode_opus_impl(pcm: &AudioBuffer) -> Result<Vec<u8>, Error> {
    sc_opus::encode(pcm)
}

#[cfg(not(feature = "opus"))]
fn encode_opus_impl(_pcm: &AudioBuffer) -> Result<Vec<u8>, Error> {
    Err(Error::UnsupportedFormat)
}

#[cfg(feature = "aac")]
fn encode_aac_impl(pcm: &AudioBuffer) -> Result<Vec<u8>, Error> {
    sc_aac::encode(pcm)
}

#[cfg(not(feature = "aac"))]
fn encode_aac_impl(_pcm: &AudioBuffer) -> Result<Vec<u8>, Error> {
    Err(Error::UnsupportedFormat)
}

#[cfg(test)]
mod tests {
    use super::{
        decode, encode, encode_wav, encode_with_mode, AudioBuffer, EncodeMode, Error, Format,
        StreamDecoder,
    };

    #[cfg(feature = "opus")]
    use super::encode_opus;

    #[test]
    fn dispatches_wav_roundtrip() {
        let pcm = AudioBuffer::new(44_100, 1, vec![0.0, 0.25, -0.25]).unwrap();
        let wav = encode(Format::Wav, &pcm).unwrap();
        let decoded = decode(&wav).unwrap();

        assert_eq!(
            encode_with_mode(Format::Wav, &pcm, EncodeMode::ProductionOnly).unwrap(),
            wav
        );
        assert_eq!(decoded.sample_rate, pcm.sample_rate);
        assert_eq!(decoded.channels, pcm.channels);
        assert_eq!(decoded.samples.len(), pcm.samples.len());
        assert_eq!(encode_wav(&pcm).unwrap(), wav);
        assert!(matches!(
            super::decode_mp3(&wav),
            Err(Error::UnsupportedFormat)
        ));
        assert!(matches!(
            super::decode_vorbis(&wav),
            Err(Error::UnsupportedFormat)
        ));
        assert!(matches!(
            super::decode_opus(&wav),
            Err(Error::UnsupportedFormat)
        ));
    }

    #[test]
    fn stream_decoder_buffers_until_complete_input() {
        let pcm = AudioBuffer::new(44_100, 1, vec![0.0, 0.25, -0.25]).unwrap();
        let wav = encode(Format::Wav, &pcm).unwrap();
        let split = wav.len() - 2;
        let mut decoder = StreamDecoder::new();

        assert!(decoder.decode_stream(&wav[..split]).unwrap().is_none());
        assert!(decoder.buffered_len() > 0);
        let decoded = decoder
            .decode_stream(&wav[split..])
            .unwrap()
            .expect("complete stream should decode");

        assert_eq!(decoded.sample_rate, pcm.sample_rate);
        assert_eq!(decoded.channels, pcm.channels);
        assert_eq!(decoded.samples.len(), pcm.samples.len());
        assert_eq!(decoder.buffered_len(), 0);
    }

    #[test]
    #[cfg(feature = "flac")]
    fn dispatches_flac_roundtrip() {
        let samples = (0..128)
            .map(|sample| sample as f32 / 32_767.0)
            .collect::<Vec<_>>();
        let pcm = AudioBuffer::new(44_100, 1, samples).unwrap();
        let flac = encode(Format::Flac, &pcm).unwrap();
        let decoded = decode(&flac).unwrap();

        assert_eq!(
            encode_with_mode(Format::Flac, &pcm, EncodeMode::ProductionOnly).unwrap(),
            flac
        );
        assert_eq!(decoded.sample_rate, pcm.sample_rate);
        assert_eq!(decoded.channels, pcm.channels);
        assert_eq!(decoded.samples.len(), pcm.samples.len());
    }

    #[test]
    fn dispatches_known_unimplemented_formats_as_unsupported() {
        let err = decode(b"ID3\x04\0\0\0\0\0\0").unwrap_err();
        assert!(matches!(err, Error::UnsupportedFormat));
    }

    #[test]
    #[cfg(feature = "opus")]
    fn dispatches_opus_encode_to_ogg_stream() {
        let pcm = AudioBuffer::new(48_000, 1, vec![0.0; 4800]).unwrap();
        let encoded = encode(Format::Opus, &pcm).expect("opus encode");

        assert_eq!(&encoded[..4], b"OggS");
        assert_eq!(super::detect(&encoded), Some(Format::Opus));
        assert_eq!(encode_opus(&pcm).expect("encode_opus"), encoded);
        let production = encode_with_mode(Format::Opus, &pcm, EncodeMode::ProductionOnly)
            .expect("production opus");
        assert_eq!(&production[..4], b"OggS");
        assert_eq!(super::detect(&production), Some(Format::Opus));
    }

    #[test]
    #[cfg(feature = "vorbis")]
    fn dispatches_vorbis_encode_to_ogg_stream() {
        let pcm = AudioBuffer::new(48_000, 1, vec![0.0; 4800]).unwrap();
        let encoded = encode(Format::Vorbis, &pcm).expect("vorbis encode");
        assert_eq!(&encoded[..4], b"OggS");
        assert_eq!(super::detect(&encoded), Some(Format::Vorbis));
        let production = encode_with_mode(Format::Vorbis, &pcm, EncodeMode::ProductionOnly)
            .expect("production vorbis");
        assert_eq!(&production[..4], b"OggS");
        assert_eq!(super::detect(&production), Some(Format::Vorbis));
    }

    #[test]
    #[cfg(feature = "opus")]
    fn dispatches_ffmpeg_generated_ogg_opus_when_available() {
        let Ok(ffmpeg) = std::env::var("SONARE_FFMPEG") else {
            return;
        };
        let path = std::env::temp_dir().join(format!(
            "sonare-codec-umbrella-opus-smoke-{}.opus",
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
                "libopus",
                "-y",
            ])
            .arg(&path)
            .status()
            .expect("run ffmpeg");
        assert!(status.success(), "ffmpeg failed with {status}");

        let bytes = std::fs::read(&path).expect("read opus");
        let _ = std::fs::remove_file(&path);
        let decoded = decode(&bytes).expect("decode opus");
        assert_eq!(decoded.sample_rate, 48_000);
        assert_eq!(decoded.channels, 1);
        assert!(!decoded.samples.is_empty());
        assert!(decoded.samples.iter().any(|sample| sample.abs() > 0.0001));
    }

    #[test]
    #[cfg(feature = "vorbis")]
    fn dispatches_ffmpeg_generated_ogg_vorbis_when_available() {
        let Ok(ffmpeg) = std::env::var("SONARE_FFMPEG") else {
            return;
        };
        let path = std::env::temp_dir().join(format!(
            "sonare-codec-umbrella-vorbis-smoke-{}.ogg",
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
        let decoded = super::decode_vorbis(&bytes).expect("decode vorbis");
        assert_eq!(decoded.sample_rate, 48_000);
        assert_eq!(decoded.channels, 1);
        assert!(!decoded.samples.is_empty());
        assert!(decoded.samples.iter().any(|sample| sample.abs() > 0.0001));
    }

    #[test]
    #[cfg(feature = "mp3")]
    fn dispatches_silent_mp3_encode() {
        let pcm = AudioBuffer::new(44_100, 2, vec![0.0; 1152 * 2]).unwrap();

        let mp3 = encode(Format::Mp3, &pcm).unwrap();
        let decoded = decode(&mp3).unwrap();

        assert_eq!(
            encode_with_mode(Format::Mp3, &pcm, EncodeMode::ProductionOnly).unwrap(),
            mp3
        );
        assert_eq!(&mp3[..2], &[0xff, 0xfb]);
        assert_eq!(super::detect(&mp3), Some(Format::Mp3));
        assert_eq!(decoded.sample_rate, 44_100);
        assert_eq!(decoded.channels, 2);
        assert_eq!(decoded.samples.len(), pcm.samples.len());
        assert_eq!(
            super::decode_mp3(&mp3).unwrap().samples.len(),
            pcm.samples.len()
        );
    }

    #[test]
    #[cfg(feature = "mp3")]
    fn dispatches_non_silent_mp3_encode_as_layer3_scaffold() {
        for (sample_rate, channels) in [
            (32_000, 1),
            (44_100, 1),
            (48_000, 1),
            (32_000, 2),
            (44_100, 2),
            (48_000, 2),
        ] {
            let mut samples = Vec::new();
            for frame in 0..2048 {
                for channel in 0..channels {
                    let phase = if channel == 0 { 0.01 } else { 0.013 };
                    samples.push(((frame as f32) * phase).sin() * 0.25);
                }
            }
            let pcm = AudioBuffer::new(sample_rate, channels, samples).unwrap();

            let mp3 = encode(Format::Mp3, &pcm).unwrap();
            let production =
                encode_with_mode(Format::Mp3, &pcm, EncodeMode::ProductionOnly).unwrap();
            let zero_payload = super::encode_mpeg1_layer3_pcm_frames_with_selected_scale_factors_and_table_provider(
                    &pcm,
                    f32::MAX,
                    super::Layer3EntropyTableProvider::default(),
                )
                .unwrap();
            let decoded = decode(&mp3).unwrap();

            assert_eq!(&mp3[..2], &[0xff, 0xfb]);
            assert_eq!(
                production, mp3,
                "sample_rate={sample_rate} channels={channels}"
            );
            assert_eq!(super::detect(&mp3), Some(Format::Mp3));
            assert!(mp3.len() > 4);
            assert_ne!(mp3, zero_payload);
            assert_eq!(decoded.sample_rate, sample_rate);
            assert_eq!(decoded.channels, channels);
            assert_eq!(decoded.samples.len(), 2304 * usize::from(channels));
        }
    }

    #[test]
    #[cfg(feature = "mp3")]
    fn exposes_mp3_pcm_frame_scaffold_helper() {
        let pcm = AudioBuffer::new(44_100, 2, vec![0.0; 1153 * 2]).unwrap();

        let scaffold =
            super::encode_mpeg1_layer3_pcm_frames_with_selected_scale_factors_and_table_provider(
                &pcm,
                1.0,
                super::Layer3EntropyTableProvider::default(),
            )
            .unwrap();

        assert_eq!(scaffold, encode(Format::Mp3, &pcm).unwrap());
    }

    #[test]
    #[cfg(feature = "mp3")]
    fn exposes_mp3_pcm_payload_budget_helper() {
        let pcm = AudioBuffer::new(
            44_100,
            1,
            (0..1152)
                .map(|sample| ((sample as f32) * 0.01).sin() * 0.25)
                .collect(),
        )
        .unwrap();
        let header = super::FrameHeader {
            version: super::MpegVersion::Mpeg1,
            layer: super::Layer::Layer3,
            protection_absent: true,
            bitrate_kbps: 128,
            sample_rate: 44_100,
            padding: false,
            channel_mode: super::ChannelMode::SingleChannel,
        };
        let provider = super::mpeg1_layer3_standard_table_provider();
        let unconstrained = super::select_mpeg1_layer3_pcm_frame_step_details_with_table_provider(
            header,
            &pcm,
            0,
            super::MPEG1_LAYER3_PCM_STEP_CANDIDATES,
            provider,
        )
        .unwrap();

        let step =
            super::select_mpeg1_layer3_pcm_frame_step_with_max_payload_bits_and_table_provider(
                header,
                &pcm,
                0,
                super::MPEG1_LAYER3_PCM_STEP_CANDIDATES,
                unconstrained.payload_bit_len,
                provider,
            )
            .unwrap();
        let details =
            super::select_mpeg1_layer3_pcm_frame_step_details_with_max_payload_bits_and_table_provider(
                header,
                &pcm,
                0,
                super::MPEG1_LAYER3_PCM_STEP_CANDIDATES,
                unconstrained.payload_bit_len,
                provider,
            )
            .unwrap();
        let budgeted =
            super::encode_mpeg1_layer3_pcm_frames_with_max_payload_bits_and_table_provider(
                &pcm,
                super::MPEG1_LAYER3_PCM_STEP_CANDIDATES,
                unconstrained.payload_bit_len,
                provider,
            )
            .unwrap();
        let selected =
            super::encode_mpeg1_layer3_pcm_frames_with_selected_scale_factors_and_table_provider(
                &pcm, step, provider,
            )
            .unwrap();

        assert_eq!(step, unconstrained.step);
        assert_eq!(details.step, step);
        assert_eq!(details.frame_capacity_bits, unconstrained.payload_bit_len);
        assert!(details.payload_bit_len <= unconstrained.payload_bit_len);
        assert_eq!(super::layer3_main_data_capacity_bits(header).unwrap(), 3168);
        assert_eq!(super::layer3_main_data_capacity_bytes(header).unwrap(), 396);
        assert_eq!(
            super::layer3_main_data_capacity_bytes(
                super::layer3_header_for_capacity(44_100, 2, 128, false, false).unwrap()
            )
            .unwrap(),
            381
        );
        assert_eq!(budgeted, selected);
        assert!(
            super::select_mpeg1_layer3_pcm_frame_step_details_with_max_payload_bits_and_table_provider(
                header,
                &pcm,
                0,
                super::MPEG1_LAYER3_PCM_STEP_CANDIDATES,
                0,
                provider,
            )
            .is_err()
        );
    }

    #[test]
    #[cfg(feature = "mp3")]
    fn exposes_mp3_psychoacoustic_scalefactor_helper() {
        let pcm = AudioBuffer::new(44_100, 1, vec![0.0; 2304]).unwrap();
        let scale_factors = super::select_mpeg1_layer3_psychoacoustic_long_scale_factors(
            &pcm, 0, 576, 0.05, false, 1024,
        )
        .unwrap();

        assert_eq!(
            scale_factors,
            [0_u8; super::MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT]
        );
    }

    #[test]
    #[cfg(feature = "mp3")]
    fn exposes_mp3_perceptual_scalefactor_stream_helper() {
        let pcm = AudioBuffer::new(
            44_100,
            1,
            (0..1152)
                .map(|sample| ((sample as f32) * 0.02).sin() * 0.2)
                .collect(),
        )
        .unwrap();
        let header = super::layer3_header_for_capacity(44_100, 1, 128, false, false).unwrap();
        let candidates = [0.05_f32, 0.1, 0.2];
        let selected =
            super::select_mpeg1_layer3_pcm_frame_perceptual_step_details_with_table_provider(
                header,
                &pcm,
                0,
                &candidates,
                super::mpeg1_layer3_standard_table_provider(),
            )
            .unwrap();
        let active_selected = super::select_mpeg1_layer3_pcm_frame_perceptual_active_step_details_with_table_provider(
                header,
                &pcm,
                0,
                &candidates,
                super::mpeg1_layer3_standard_table_provider(),
            )
            .unwrap();
        let encoded =
            super::encode_mpeg1_layer3_pcm_frames_with_perceptual_scale_factors_and_table_provider(
                &pcm,
                0.1,
                super::mpeg1_layer3_standard_table_provider(),
            )
            .unwrap();
        let budgeted =
            super::encode_mpeg1_layer3_pcm_frames_with_perceptual_max_payload_bits_and_table_provider(
                &pcm,
                &candidates,
                selected.payload_bit_len,
                super::mpeg1_layer3_standard_table_provider(),
            )
            .unwrap();
        let bitrate_header =
            super::layer3_header_for_capacity(44_100, 1, 96, false, false).unwrap();
        let bitrate_encoded =
            super::encode_mpeg1_layer3_pcm_frames_with_perceptual_bitrate_and_table_provider(
                &pcm,
                &candidates,
                96,
                false,
                false,
                super::mpeg1_layer3_standard_table_provider(),
            )
            .unwrap();
        let cbr_encoded =
            super::encode_mpeg1_layer3_pcm_frames_with_perceptual_cbr_bitrate_and_table_provider(
                &pcm,
                &candidates,
                96,
                false,
                super::mpeg1_layer3_standard_table_provider(),
            )
            .unwrap();
        let active_cbr_encoded =
            super::encode_mpeg1_layer3_pcm_frames_with_perceptual_active_cbr_bitrate_and_table_provider(
                &pcm,
                &candidates,
                96,
                false,
                super::mpeg1_layer3_standard_table_provider(),
            )
            .unwrap();

        assert!(active_selected.payload_bit_len <= active_selected.frame_capacity_bits);
        assert_eq!(encoded.len(), header.frame_len());
        assert_eq!(budgeted.len(), header.frame_len());
        assert_eq!(bitrate_encoded.len(), bitrate_header.frame_len());
        assert_eq!(cbr_encoded.len(), bitrate_header.frame_len());
        assert_eq!(active_cbr_encoded.len(), bitrate_header.frame_len());
        assert_eq!(super::detect(&encoded), Some(Format::Mp3));
        assert_eq!(super::detect(&budgeted), Some(Format::Mp3));
        assert_eq!(
            super::FrameHeader::parse(&bitrate_encoded[..4]).unwrap(),
            bitrate_header
        );
    }

    #[test]
    #[cfg(feature = "mp3")]
    fn exposes_mp3_pcm_bitrate_helper() {
        let pcm = AudioBuffer::new(
            44_100,
            1,
            (0..1152)
                .map(|sample| ((sample as f32) * 0.01).sin() * 0.25)
                .collect(),
        )
        .unwrap();
        let provider = super::mpeg1_layer3_standard_table_provider();
        let header = super::layer3_header_for_capacity(44_100, 1, 96, false, false).unwrap();

        let encoded = super::encode_mpeg1_layer3_pcm_frames_with_bitrate_and_table_provider(
            &pcm,
            super::MPEG1_LAYER3_PCM_STEP_CANDIDATES,
            96,
            false,
            false,
            provider,
        )
        .unwrap();
        let parsed = super::FrameHeader::parse(&encoded[..4]).unwrap();

        assert_eq!(parsed, header);
        assert_eq!(parsed.bitrate_kbps, 96);
        assert_eq!(encoded.len(), header.frame_len());
        assert!(
            super::encode_mpeg1_layer3_pcm_frames_with_bitrate_and_table_provider(
                &pcm,
                super::MPEG1_LAYER3_PCM_STEP_CANDIDATES,
                123,
                false,
                false,
                provider,
            )
            .is_err()
        );
    }

    #[test]
    #[cfg(feature = "aac")]
    fn exposes_aac_adts_to_m4a_mux() {
        let adts =
            super::frame_aac_adts(super::AdtsConfig::aac_lc(44_100, 2), &[0x11, 0x22]).unwrap();
        let m4a = super::mux_aac_adts_as_m4a(&adts).unwrap();
        let demuxed = super::demux_m4a_as_aac_adts(&m4a).unwrap();

        assert_eq!(&m4a[4..8], b"ftyp");
        assert!(m4a.windows(4).any(|window| window == b"mdat"));
        assert_eq!(demuxed, adts);
    }

    #[test]
    #[cfg(feature = "aac")]
    fn exposes_aac_pcm_scale_factor_stream_helper() {
        let pcm = AudioBuffer::new(44_100, 1, vec![0.0; 2048]).unwrap();
        let scale_factors_by_frame: [&[i16]; 2] = [&[0], &[0]];
        let selected =
            super::select_scale_factors_for_quantized_bands(&[0, 0, 1, -1], 2, 100).unwrap();
        let quantized_adts = super::encode_quantized_mono_adts_with_selected_scale_factors(
            super::AdtsConfig::aac_lc(44_100, 1),
            super::AacLongBlockConfig::new(0, 1),
            &[0, 0],
            2,
            &[],
            super::AacSpectralMagnitudeTables::default(),
        )
        .unwrap();
        let selected_pcm_adts = super::encode_pcm_mono_long_block_adts_with_selected_scale_factors(
            super::AdtsConfig::aac_lc(44_100, 1),
            super::AacLongBlockConfig::new(0, 1),
            &pcm,
            super::AacPcmLongBlockConfig::new(0, 1.0, 1024),
            &[],
            super::AacSpectralMagnitudeTables::default(),
        )
        .unwrap();
        let selected_stream_adts =
            super::encode_pcm_mono_long_block_adts_stream_with_selected_scale_factors(
                super::AdtsConfig::aac_lc(44_100, 1),
                super::AacLongBlockConfig::new(0, 1),
                &pcm,
                super::AacPcmLongBlockConfig::new(0, 1.0, 1024),
                &[],
                super::AacSpectralMagnitudeTables::default(),
            )
            .unwrap();

        let adts = super::encode_pcm_mono_long_block_adts_stream_with_scale_factors(
            super::AdtsConfig::aac_lc(44_100, 1),
            super::AacScaleFactorSequence::new(
                super::AacLongBlockConfig::new(0, 1),
                &scale_factors_by_frame,
            ),
            &pcm,
            super::AacPcmLongBlockConfig::new(0, 1.0, 1024),
            &[],
            super::AacSpectralMagnitudeTables::default(),
        )
        .unwrap();

        assert_eq!(&adts[..2], &[0xff, 0xf1]);
        assert_eq!(&quantized_adts[..2], &[0xff, 0xf1]);
        assert_eq!(&selected_pcm_adts[..2], &[0xff, 0xf1]);
        assert_eq!(&selected_stream_adts[..2], &[0xff, 0xf1]);
        assert_eq!(adts.len(), 26);
        assert_eq!(selected_stream_adts.len(), 26);
        assert_eq!(selected, vec![100, 101]);
    }

    #[test]
    #[cfg(feature = "aac")]
    fn exposes_aac_pcm_bitrate_budget_stream_helper() {
        fn max_adts_frame_len(stream: &[u8]) -> usize {
            let mut max_len = 0;
            let mut offset = 0;
            while offset + 7 <= stream.len() {
                let frame_len = (((stream[offset + 3] & 0x03) as usize) << 11)
                    | ((stream[offset + 4] as usize) << 3)
                    | ((stream[offset + 5] as usize) >> 5);
                assert!(frame_len >= 7);
                assert!(offset + frame_len <= stream.len());
                max_len = max_len.max(frame_len);
                offset += frame_len;
            }
            assert_eq!(offset, stream.len());
            max_len
        }

        let mono = AudioBuffer::new(
            44_100,
            1,
            (0..2048)
                .map(|sample| ((sample as f32) * 0.01).sin() * 0.25)
                .collect(),
        )
        .unwrap();
        let mut stereo_samples = Vec::new();
        for sample in 0..2048 {
            stereo_samples.push(((sample as f32) * 0.01).sin() * 0.25);
            stereo_samples.push(((sample as f32) * 0.013).cos() * 0.20);
        }
        let stereo = AudioBuffer::new(44_100, 2, stereo_samples).unwrap();
        let offsets = super::aac_lc_long_window_scale_factor_band_offsets(44_100).unwrap();
        let channel_config = super::AacLongBlockConfig::new(180, (offsets.len() - 1) as u8);
        let scale_factors = vec![i16::from(channel_config.global_gain); offsets.len() - 1];
        let channel = super::AacScaleFactorChannel::new(channel_config, &scale_factors);
        let scale_factor_table = super::aac_scale_factor_delta_zero_table();
        let spectral_tables = super::aac_unsigned_pairs7_unit_magnitude_spectral_tables();

        let mono_target_bitrate = 10_000;
        let mono_budget =
            super::aac_lc_adts_max_frame_len_for_bitrate(44_100, mono_target_bitrate).unwrap();
        let mono_adts =
            super::encode_pcm_mono_long_block_adts_stream_with_offsets_and_scale_factors_and_bitrate_by_bit_cost(
                super::AdtsConfig::aac_lc(44_100, 1),
                channel,
                &mono,
                offsets,
                super::AAC_LC_PCM_STEP_CANDIDATES,
                mono_target_bitrate,
                scale_factor_table,
                spectral_tables,
            )
            .unwrap();

        let stereo_target_bitrate = 14_000;
        let stereo_budget =
            super::aac_lc_adts_max_frame_len_for_bitrate(44_100, stereo_target_bitrate).unwrap();
        let stereo_adts =
            super::encode_pcm_stereo_long_block_adts_stream_with_offsets_and_scale_factors_and_bitrate_by_bit_cost(
                super::AdtsConfig::aac_lc(44_100, 2),
                channel,
                channel,
                &stereo,
                offsets,
                super::AAC_LC_PCM_STEP_CANDIDATES,
                stereo_target_bitrate,
                scale_factor_table,
                spectral_tables,
            )
            .unwrap();
        let mono_adts_high_level =
            super::encode_aac_adts_with_bitrate(&mono, mono_target_bitrate).unwrap();
        let stereo_adts_high_level =
            super::encode_aac_adts_with_bitrate(&stereo, stereo_target_bitrate).unwrap();

        assert_eq!(&mono_adts[..2], &[0xff, 0xf1]);
        assert_eq!(&stereo_adts[..2], &[0xff, 0xf1]);
        assert_eq!(mono_adts_high_level, mono_adts);
        assert_eq!(stereo_adts_high_level, stereo_adts);
        assert!(max_adts_frame_len(&mono_adts) <= mono_budget);
        assert!(max_adts_frame_len(&stereo_adts) <= stereo_budget);
        assert!(super::aac_lc_adts_max_frame_len_for_bitrate(44_100, 1).is_err());
        assert!(super::encode_aac_adts_with_bitrate(&mono, 1).is_err());
    }

    #[test]
    #[cfg(feature = "aac")]
    fn exposes_aac_unsigned_pairs7_unit_magnitude_table() {
        let table = super::aac_unsigned_pairs7_unit_magnitude_table();
        assert_eq!(table.len(), 4);
        assert_eq!(table[0].symbol, super::AacSpectralMagnitudePair::new(0, 0));
        assert_eq!(table[0].code, super::HuffmanCode::new(0b0, 1).unwrap());
        assert_eq!(table[1].symbol, super::AacSpectralMagnitudePair::new(0, 1));
        assert_eq!(table[1].code, super::HuffmanCode::new(0b101, 3).unwrap());
        assert_eq!(table[2].symbol, super::AacSpectralMagnitudePair::new(1, 0));
        assert_eq!(table[2].code, super::HuffmanCode::new(0b100, 3).unwrap());
        assert_eq!(table[3].symbol, super::AacSpectralMagnitudePair::new(1, 1));
        assert_eq!(table[3].code, super::HuffmanCode::new(0b1100, 4).unwrap());
    }

    #[test]
    #[cfg(feature = "aac")]
    fn exposes_aac_unsigned_pairs7_table() {
        let table = super::aac_unsigned_pairs7_table();

        assert_eq!(table.len(), 64);
        assert_eq!(table[0].symbol, super::AacSpectralMagnitudePair::new(0, 0));
        assert_eq!(table[0].code, super::HuffmanCode::new(0, 1).unwrap());
        assert_eq!(table[63].symbol, super::AacSpectralMagnitudePair::new(7, 7));
        assert_eq!(table[63].code, super::HuffmanCode::new(0xfff, 12).unwrap());
    }

    #[test]
    #[cfg(feature = "aac")]
    fn exposes_aac_unsigned_pairs8_table() {
        let table = super::aac_unsigned_pairs8_table();

        assert_eq!(table.len(), 64);
        assert_eq!(table[0].symbol, super::AacSpectralMagnitudePair::new(0, 0));
        assert_eq!(table[0].code, super::HuffmanCode::new(0x00e, 5).unwrap());
        assert_eq!(table[9].symbol, super::AacSpectralMagnitudePair::new(1, 1));
        assert_eq!(table[9].code, super::HuffmanCode::new(0, 3).unwrap());
        assert_eq!(table[63].symbol, super::AacSpectralMagnitudePair::new(7, 7));
        assert_eq!(table[63].code, super::HuffmanCode::new(0x3ff, 10).unwrap());
    }

    #[test]
    #[cfg(feature = "aac")]
    fn exposes_aac_scale_factor_delta_table() {
        let table = super::aac_scale_factor_delta_table();

        assert_eq!(table.len(), 121);
        assert_eq!(table[0].symbol, super::AacScaleFactorDelta::new(-60));
        assert_eq!(table[60].symbol, super::AacScaleFactorDelta::new(0));
        assert_eq!(table[60].code, super::HuffmanCode::new(0, 1).unwrap());
        assert_eq!(table[120].symbol, super::AacScaleFactorDelta::new(60));
    }

    #[test]
    #[cfg(feature = "aac")]
    fn exposes_aac_spectral_quad_helpers() {
        let quads = [super::AacSpectralQuad::new(1, -1, 0, 1)];
        let sections = [super::AacQuadSection {
            start: 0,
            end: 4,
            codebook_id: 2,
        }];
        let quantized = [1, -1, 0, 1];
        let signed_table = [super::HuffmanEntry {
            symbol: quads[0],
            code: super::HuffmanCode::new(0b11, 2).unwrap(),
        }];
        let magnitude_table = [super::HuffmanEntry {
            symbol: super::AacSpectralMagnitudeQuad::new(1, 1, 0, 1),
            code: super::HuffmanCode::new(0b10, 2).unwrap(),
        }];

        assert_eq!(
            super::pack_spectral_quads_with_table(&quads, &signed_table)
                .unwrap()
                .bit_len,
            2
        );
        assert_eq!(
            super::pack_spectral_quads_with_sign_bits(&quads, &magnitude_table)
                .unwrap()
                .bit_len,
            5
        );
        let tables = super::AacSpectralMagnitudeQuadTables {
            quads2: &magnitude_table,
            ..Default::default()
        };
        assert_eq!(
            super::pack_quad_section_data_with_len(&sections, 4)
                .unwrap()
                .bit_len,
            9
        );
        assert_eq!(
            super::pack_spectral_quad_sections_with_sign_bits(&sections, &quantized, tables)
                .unwrap()
                .bit_len,
            5
        );
        assert_eq!(
            super::pack_sectioned_spectral_quad_payload_with_sign_bits(
                &sections, &quantized, 4, tables,
            )
            .unwrap()
            .bit_len,
            14
        );
    }

    #[test]
    #[cfg(feature = "aac")]
    fn exposes_aac_codebook7_section_planning() {
        let sections = super::plan_sections_by_bit_cost(
            &[1, -1, 0, 0],
            2,
            super::AacSpectralMagnitudeTables::default(),
        )
        .unwrap();

        assert_eq!(
            sections,
            vec![
                super::AacSection {
                    start: 0,
                    end: 2,
                    codebook: super::AacCodebook::UnsignedPairs8,
                },
                super::AacSection {
                    start: 2,
                    end: 4,
                    codebook: super::AacCodebook::Zero,
                },
            ]
        );
    }

    #[test]
    #[cfg(feature = "aac")]
    fn dispatches_silent_aac_encode_as_adts() {
        let pcm = AudioBuffer::new(44_100, 1, vec![0.0; 1024]).unwrap();

        let adts = encode(Format::Aac, &pcm).unwrap();
        let decoded = decode(&adts).unwrap();
        let m4a = super::mux_aac_adts_as_m4a(&adts).unwrap();
        let decoded_m4a = decode(&m4a).unwrap();
        let decoded_aac_helper = super::decode_aac(&adts).unwrap();
        let decoded_m4a_helper = super::decode_aac(&m4a).unwrap();

        assert_eq!(
            encode_with_mode(Format::Aac, &pcm, EncodeMode::ProductionOnly).unwrap(),
            adts
        );
        assert_eq!(&adts[..2], &[0xff, 0xf1]);
        assert_eq!(super::detect(&adts), Some(Format::Aac));
        assert_eq!(decoded.sample_rate, 44_100);
        assert_eq!(decoded.channels, 1);
        assert_eq!(decoded_m4a.sample_rate, 44_100);
        assert_eq!(decoded_m4a.channels, 1);
        assert_eq!(decoded_m4a.samples.len(), pcm.samples.len());
        assert_eq!(decoded_aac_helper.samples.len(), pcm.samples.len());
        assert_eq!(decoded_m4a_helper.samples.len(), pcm.samples.len());
    }

    #[test]
    #[cfg(feature = "aac")]
    fn dispatches_non_silent_aac_encode_as_adts_scaffold() {
        for (sample_rate, channels) in [
            (7_350, 1),
            (8_000, 1),
            (11_025, 1),
            (12_000, 1),
            (16_000, 1),
            (22_050, 1),
            (24_000, 1),
            (32_000, 1),
            (44_100, 1),
            (48_000, 1),
            (64_000, 1),
            (88_200, 1),
            (96_000, 1),
            (7_350, 2),
            (8_000, 2),
            (11_025, 2),
            (12_000, 2),
            (16_000, 2),
            (22_050, 2),
            (24_000, 2),
            (32_000, 2),
            (44_100, 2),
            (48_000, 2),
            (64_000, 2),
            (88_200, 2),
            (96_000, 2),
        ] {
            let mut samples = Vec::new();
            for frame in 0..2048 {
                for channel in 0..channels {
                    let phase = if channel == 0 { 0.01 } else { 0.013 };
                    samples.push(((frame as f32) * phase).sin() * 0.25);
                }
            }
            let pcm = AudioBuffer::new(sample_rate, channels, samples).unwrap();

            let adts = encode(Format::Aac, &pcm).unwrap();
            let production =
                encode_with_mode(Format::Aac, &pcm, EncodeMode::ProductionOnly).unwrap();
            let m4a = super::mux_aac_adts_as_m4a(&adts).unwrap();

            assert_eq!(&adts[..2], &[0xff, 0xf1]);
            assert_eq!(&production[..2], &[0xff, 0xf1]);
            assert_eq!(production, adts);
            assert_eq!(super::detect(&adts), Some(Format::Aac));
            assert!(adts.len() > 7);
            assert!(m4a.len() > adts.len());
        }
    }
}
