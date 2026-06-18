#![deny(unsafe_code)]
#![warn(clippy::all)]

use sc_core::{
    apply_window, concat_packed_bits, mdct, pack_huffman_codes, pack_huffman_codes_with_len,
    pack_huffman_symbols_with_len, quantize_spectrum, sine_window, write_packed_bits, AudioBuffer,
    BitWriter as CoreBitWriter, Decoder, Encoder, Error, HuffmanCode, HuffmanEntry, PackedBits,
};
use std::sync::OnceLock;

const AAC_ESCAPE_MAGNITUDE: u16 = 16;
const AAC_ADTS_MAX_FRAME_LEN: usize = 0x1fff;
const AAC_ADTS_HEADER_LEN: usize = 7;
pub const AAC_LC_PCM_STEP_CANDIDATES: &[f32] = &[
    0.0005,
    0.001,
    0.002,
    0.005,
    0.01,
    0.02,
    0.05,
    0.1,
    0.2,
    0.5,
    1.0,
    2.0,
    5.0,
    10.0,
    20.0,
    50.0,
    100.0,
    200.0,
    500.0,
    1_000.0,
    f32::MAX,
];
pub const AAC_LC_48K_LONG_WINDOW_SCALE_FACTOR_BAND_OFFSETS: &[usize] = &[
    0, 4, 8, 12, 16, 20, 24, 28, 32, 36, 40, 48, 56, 64, 72, 80, 88, 96, 108, 120, 132, 144, 160,
    176, 196, 216, 240, 264, 292, 320, 352, 384, 416, 448, 480, 512, 544, 576, 608, 640, 672, 704,
    736, 768, 800, 832, 864, 896, 928, 1024,
];
pub const AAC_LC_96K_LONG_WINDOW_SCALE_FACTOR_BAND_OFFSETS: &[usize] = &[
    0, 4, 8, 12, 16, 20, 24, 28, 32, 36, 40, 44, 48, 52, 56, 64, 72, 80, 88, 96, 108, 120, 132,
    144, 156, 172, 188, 212, 240, 276, 320, 384, 448, 512, 576, 640, 704, 768, 832, 896, 960, 1024,
];
pub const AAC_LC_64K_LONG_WINDOW_SCALE_FACTOR_BAND_OFFSETS: &[usize] = &[
    0, 4, 8, 12, 16, 20, 24, 28, 32, 36, 40, 44, 48, 52, 56, 64, 72, 80, 88, 100, 112, 124, 140,
    156, 172, 192, 216, 240, 268, 304, 344, 384, 424, 464, 504, 544, 584, 624, 664, 704, 744, 784,
    824, 864, 904, 944, 984, 1024,
];
pub const AAC_LC_32K_LONG_WINDOW_SCALE_FACTOR_BAND_OFFSETS: &[usize] = &[
    0, 4, 8, 12, 16, 20, 24, 28, 32, 36, 40, 48, 56, 64, 72, 80, 88, 96, 108, 120, 132, 144, 160,
    176, 196, 216, 240, 264, 292, 320, 352, 384, 416, 448, 480, 512, 544, 576, 608, 640, 672, 704,
    736, 768, 800, 832, 864, 896, 928, 960, 992, 1024,
];
pub const AAC_LC_24K_LONG_WINDOW_SCALE_FACTOR_BAND_OFFSETS: &[usize] = &[
    0, 4, 8, 12, 16, 20, 24, 28, 32, 36, 40, 44, 52, 60, 68, 76, 84, 92, 100, 108, 116, 124, 136,
    148, 160, 172, 188, 204, 220, 240, 260, 284, 308, 336, 364, 396, 432, 468, 508, 552, 600, 652,
    704, 768, 832, 896, 960, 1024,
];
pub const AAC_LC_16K_LONG_WINDOW_SCALE_FACTOR_BAND_OFFSETS: &[usize] = &[
    0, 8, 16, 24, 32, 40, 48, 56, 64, 72, 80, 88, 100, 112, 124, 136, 148, 160, 172, 184, 196, 212,
    228, 244, 260, 280, 300, 320, 344, 368, 396, 424, 456, 492, 532, 572, 616, 664, 716, 772, 832,
    896, 960, 1024,
];
pub const AAC_LC_8K_LONG_WINDOW_SCALE_FACTOR_BAND_OFFSETS: &[usize] = &[
    0, 12, 24, 36, 48, 60, 72, 84, 96, 108, 120, 132, 144, 156, 172, 188, 204, 220, 236, 252, 268,
    288, 308, 328, 348, 372, 396, 420, 448, 476, 508, 544, 580, 620, 664, 712, 764, 820, 880, 944,
    1024,
];

/// Converts an AAC-LC target bitrate into a maximum ADTS frame length.
///
/// The returned budget includes the 7-byte ADTS header because ADTS
/// `frame_length` covers the whole transport frame.
pub fn aac_lc_adts_max_frame_len_for_bitrate(
    sample_rate: u32,
    target_bitrate_bps: u32,
) -> Result<usize, Error> {
    if sample_rate == 0 {
        return Err(Error::InvalidInput(
            "AAC sample rate must be greater than zero",
        ));
    }
    if target_bitrate_bps == 0 {
        return Err(Error::InvalidInput(
            "AAC target bitrate must be greater than zero",
        ));
    }

    let numerator = u64::from(target_bitrate_bps)
        .checked_mul(1024)
        .ok_or(Error::InvalidInput("AAC bitrate budget overflows"))?;
    let denominator = u64::from(sample_rate)
        .checked_mul(8)
        .ok_or(Error::InvalidInput("AAC bitrate budget overflows"))?;
    let frame_len = numerator
        .checked_add(denominator - 1)
        .ok_or(Error::InvalidInput("AAC bitrate budget overflows"))?
        / denominator;
    let frame_len = usize::try_from(frame_len)
        .map_err(|_| Error::InvalidInput("AAC bitrate budget overflows"))?;
    if frame_len < AAC_ADTS_HEADER_LEN {
        return Err(Error::InvalidInput("AAC bitrate budget is too small"));
    }
    validate_aac_max_frame_len(frame_len)?;
    Ok(frame_len)
}

/// Returns the conservative bitrate budget used by the AAC-LC production candidate.
///
/// The current encoder is still a limited long-block scaffold; this default is
/// intentionally generous so production `encode()` can exercise the same
/// budget-aware selected-scale-factor path as caller-driven bitrate helpers
/// without lowering the existing FFmpeg readiness gate.
pub fn aac_lc_default_production_bitrate_bps(channels: u8) -> Result<u32, Error> {
    if channels == 0 || channels > 2 {
        return Err(Error::InvalidInput(
            "AAC production bitrate requires mono or stereo channels",
        ));
    }
    128_000_u32
        .checked_mul(u32::from(channels))
        .ok_or(Error::InvalidInput("AAC production bitrate overflows"))
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AacPcmFrameStepSelection {
    pub step: f32,
    pub frame_len: usize,
    pub frame_capacity_bytes: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AdtsConfig {
    pub profile: AacProfile,
    pub sample_rate: u32,
    pub channels: u8,
}

impl AdtsConfig {
    #[must_use]
    pub fn aac_lc(sample_rate: u32, channels: u8) -> Self {
        Self {
            profile: AacProfile::LowComplexity,
            sample_rate,
            channels,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AacProfile {
    Main,
    LowComplexity,
    ScalableSampleRate,
}

impl AacProfile {
    fn adts_profile(self) -> u8 {
        match self {
            Self::Main => 0,
            Self::LowComplexity => 1,
            Self::ScalableSampleRate => 2,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AacLongBlockConfig {
    pub global_gain: u8,
    pub max_sfb: u8,
}

impl AacLongBlockConfig {
    #[must_use]
    pub fn new(global_gain: u8, max_sfb: u8) -> Self {
        Self {
            global_gain,
            max_sfb,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AacPcmLongBlockConfig {
    pub start_frame: usize,
    pub step: f32,
    pub band_width: usize,
}

impl AacPcmLongBlockConfig {
    #[must_use]
    pub fn new(start_frame: usize, step: f32, band_width: usize) -> Self {
        Self {
            start_frame,
            step,
            band_width,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct AacPcmStepSearchConfig<'a> {
    pub start_frame: usize,
    pub band_width: usize,
    pub candidates: &'a [f32],
    pub scale_factor_table: &'a [HuffmanEntry<AacScaleFactorDelta>],
    pub spectral_tables: AacSpectralMagnitudeTables<'a>,
}

impl<'a> AacPcmStepSearchConfig<'a> {
    #[must_use]
    pub fn new(
        start_frame: usize,
        band_width: usize,
        candidates: &'a [f32],
        scale_factor_table: &'a [HuffmanEntry<AacScaleFactorDelta>],
        spectral_tables: AacSpectralMagnitudeTables<'a>,
    ) -> Self {
        Self {
            start_frame,
            band_width,
            candidates,
            scale_factor_table,
            spectral_tables,
        }
    }
}

#[must_use]
pub fn aac_lc_long_window_scale_factor_band_offsets(sample_rate: u32) -> Option<&'static [usize]> {
    match sample_rate {
        88_200 | 96_000 => Some(AAC_LC_96K_LONG_WINDOW_SCALE_FACTOR_BAND_OFFSETS),
        64_000 => Some(AAC_LC_64K_LONG_WINDOW_SCALE_FACTOR_BAND_OFFSETS),
        44_100 | 48_000 => Some(AAC_LC_48K_LONG_WINDOW_SCALE_FACTOR_BAND_OFFSETS),
        32_000 => Some(AAC_LC_32K_LONG_WINDOW_SCALE_FACTOR_BAND_OFFSETS),
        22_050 | 24_000 => Some(AAC_LC_24K_LONG_WINDOW_SCALE_FACTOR_BAND_OFFSETS),
        11_025 | 12_000 | 16_000 => Some(AAC_LC_16K_LONG_WINDOW_SCALE_FACTOR_BAND_OFFSETS),
        7_350 | 8_000 => Some(AAC_LC_8K_LONG_WINDOW_SCALE_FACTOR_BAND_OFFSETS),
        _ => None,
    }
}

#[derive(Clone, Copy, Debug)]
pub struct AacQuantizedChannel<'a> {
    pub config: AacLongBlockConfig,
    pub quantized: &'a [i32],
    pub scale_factors: &'a [i16],
}

impl<'a> AacQuantizedChannel<'a> {
    #[must_use]
    pub fn new(config: AacLongBlockConfig, quantized: &'a [i32], scale_factors: &'a [i16]) -> Self {
        Self {
            config,
            quantized,
            scale_factors,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct AacQuantizedSpectrum<'a> {
    pub config: AacLongBlockConfig,
    pub quantized: &'a [i32],
}

impl<'a> AacQuantizedSpectrum<'a> {
    #[must_use]
    pub fn new(config: AacLongBlockConfig, quantized: &'a [i32]) -> Self {
        Self { config, quantized }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct AacScaleFactorChannel<'a> {
    pub config: AacLongBlockConfig,
    pub scale_factors: &'a [i16],
}

impl<'a> AacScaleFactorChannel<'a> {
    #[must_use]
    pub fn new(config: AacLongBlockConfig, scale_factors: &'a [i16]) -> Self {
        Self {
            config,
            scale_factors,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct AacScaleFactorSequence<'a> {
    pub config: AacLongBlockConfig,
    pub scale_factors_by_frame: &'a [&'a [i16]],
}

impl<'a> AacScaleFactorSequence<'a> {
    #[must_use]
    pub fn new(config: AacLongBlockConfig, scale_factors_by_frame: &'a [&'a [i16]]) -> Self {
        Self {
            config,
            scale_factors_by_frame,
        }
    }

    fn channel_for_frame(self, frame_index: usize) -> Result<AacScaleFactorChannel<'a>, Error> {
        let scale_factors = self
            .scale_factors_by_frame
            .get(frame_index)
            .copied()
            .ok_or(Error::InvalidInput(
                "AAC scale-factor frame count does not match PCM frame count",
            ))?;
        Ok(AacScaleFactorChannel::new(self.config, scale_factors))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AacIndividualChannelPayload {
    pub section_and_scale_factor_bits: PackedBits,
    pub spectral_bits: PackedBits,
}

impl AacIndividualChannelPayload {
    #[must_use]
    pub fn new(section_and_scale_factor_bits: PackedBits, spectral_bits: PackedBits) -> Self {
        Self {
            section_and_scale_factor_bits,
            spectral_bits,
        }
    }
}

#[derive(Default)]
pub struct AacDecoder;

impl AacDecoder {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Decoder for AacDecoder {
    fn decode(&mut self, input: &[u8]) -> Result<AudioBuffer, Error> {
        decode(input)
    }

    fn decode_stream(&mut self, chunk: &[u8]) -> Result<Option<AudioBuffer>, Error> {
        decode(chunk).map(Some)
    }
}

#[derive(Default)]
pub struct AacEncoder;

impl AacEncoder {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Encoder for AacEncoder {
    fn encode(&mut self, pcm: &AudioBuffer) -> Result<Vec<u8>, Error> {
        encode(pcm)
    }
}

pub fn decode(input: &[u8]) -> Result<AudioBuffer, Error> {
    match decode_silent_adts(input) {
        Ok(decoded) => Ok(decoded),
        Err(adts_err) => match sc_mp4::demux_aac(input) {
            Ok(adts) => decode_silent_adts(&adts),
            Err(Error::UnsupportedFormat) => Err(adts_err),
            Err(err) => Err(err),
        },
    }
}

/// Encodes mono/stereo PCM as AAC-LC ADTS frames.
///
/// Silent input keeps the compact zero-spectral raw-data-block path. Non-silent
/// input currently routes through the experimental long-block analysis scaffold
/// with a small standards-shaped codebook subset, so production-quality
/// psychoacoustic modeling, full spectral Huffman tables, and rate control are
/// still incomplete.
pub fn encode(pcm: &AudioBuffer) -> Result<Vec<u8>, Error> {
    if pcm.channels > 2 {
        return Err(Error::UnsupportedFeature(
            "AAC-LC encode currently supports mono/stereo only",
        ));
    }

    let config = AdtsConfig::aac_lc(
        pcm.sample_rate,
        u8::try_from(pcm.channels).map_err(|_| Error::InvalidPcm("too many channels"))?,
    );
    sample_rate_index(config.sample_rate)?;

    if pcm.samples.iter().any(|sample| sample.abs() > 1.0e-8) {
        return encode_pcm_long_block_scaffold(config, pcm);
    }

    let frame_count = pcm.frames().div_ceil(1024).max(1);
    let mut out = Vec::new();
    for _ in 0..frame_count {
        let access_unit = encode_silent_raw_data_block(config.channels)?;
        out.extend_from_slice(&frame_adts(config, &access_unit)?);
    }
    Ok(out)
}

fn encode_pcm_long_block_scaffold(config: AdtsConfig, pcm: &AudioBuffer) -> Result<Vec<u8>, Error> {
    if config.channels == 1 {
        if let Some(offsets) = aac_lc_long_window_scale_factor_band_offsets(config.sample_rate) {
            let channel_config = AacLongBlockConfig::new(
                180,
                u8::try_from(offsets.len() - 1)
                    .map_err(|_| Error::InvalidInput("AAC scale-factor band count exceeds u8"))?,
            );
            let scale_factor_table = aac_scale_factor_delta_table();
            return encode_pcm_mono_long_block_adts_stream_with_offsets_and_selected_scale_factors_and_bitrate_by_bit_cost(
                config,
                channel_config,
                pcm,
                offsets,
                AAC_LC_PCM_STEP_CANDIDATES,
                aac_lc_default_production_bitrate_bps(config.channels)?,
                &scale_factor_table,
                aac_unsigned_pairs7_unit_magnitude_spectral_tables(),
            );
        }
    }
    if config.channels == 2 {
        if let Some(offsets) = aac_lc_long_window_scale_factor_band_offsets(config.sample_rate) {
            let channel_config = AacLongBlockConfig::new(
                180,
                u8::try_from(offsets.len() - 1)
                    .map_err(|_| Error::InvalidInput("AAC scale-factor band count exceeds u8"))?,
            );
            let scale_factor_table = aac_scale_factor_delta_table();
            return encode_pcm_stereo_long_block_adts_stream_with_offsets_and_selected_scale_factors_and_bitrate_by_bit_cost(
                config,
                channel_config,
                channel_config,
                pcm,
                offsets,
                AAC_LC_PCM_STEP_CANDIDATES,
                aac_lc_default_production_bitrate_bps(config.channels)?,
                &scale_factor_table,
                aac_unsigned_pairs7_unit_magnitude_spectral_tables(),
            );
        }
    }

    let channel = AacLongBlockConfig::new(0, 1);
    let pcm_config = AacPcmLongBlockConfig::new(0, f32::MAX, 1024);
    match config.channels {
        1 => encode_pcm_mono_long_block_adts_stream_with_selected_scale_factors_by_bit_cost(
            config,
            channel,
            pcm,
            pcm_config,
            &[],
            AacSpectralMagnitudeTables::default(),
        ),
        2 => encode_pcm_stereo_long_block_adts_stream_with_selected_scale_factors_by_bit_cost(
            config,
            channel,
            channel,
            pcm,
            pcm_config,
            &[],
            AacSpectralMagnitudeTables::default(),
        ),
        _ => Err(Error::UnsupportedFeature(
            "AAC-LC encode currently supports mono/stereo only",
        )),
    }
}

/// Wraps one raw AAC access unit in an ADTS frame.
pub fn frame_adts(config: AdtsConfig, access_unit: &[u8]) -> Result<Vec<u8>, Error> {
    let sample_rate_index = sample_rate_index(config.sample_rate)?;
    if config.channels == 0 {
        return Err(Error::UnsupportedFeature(
            "AAC program config elements are not supported",
        ));
    }
    if config.channels > 7 {
        return Err(Error::InvalidInput("AAC ADTS channel count exceeds 7"));
    }

    let frame_len = access_unit
        .len()
        .checked_add(7)
        .ok_or(Error::InvalidInput("AAC ADTS frame is too large"))?;
    if frame_len > AAC_ADTS_MAX_FRAME_LEN {
        return Err(Error::InvalidInput("AAC ADTS frame exceeds 13-bit length"));
    }

    let profile = config.profile.adts_profile();
    let channels = config.channels;
    let mut out = Vec::with_capacity(frame_len);
    out.push(0xff);
    out.push(0xf1);
    out.push((profile << 6) | (sample_rate_index << 2) | (channels >> 2));
    out.push(((channels & 0x03) << 6) | (((frame_len >> 11) & 0x03) as u8));
    out.push(((frame_len >> 3) & 0xff) as u8);
    out.push((((frame_len & 0x07) << 5) as u8) | 0x1f);
    out.push(0xfc);
    out.extend_from_slice(access_unit);
    Ok(out)
}

/// Wraps raw AAC access units in consecutive ADTS frames.
pub fn frame_adts_stream<'a>(
    config: AdtsConfig,
    access_units: impl IntoIterator<Item = &'a [u8]>,
) -> Result<Vec<u8>, Error> {
    let mut out = Vec::new();
    for access_unit in access_units {
        out.extend_from_slice(&frame_adts(config, access_unit)?);
    }
    Ok(out)
}

/// Wraps AAC ADTS frames in a minimal M4A container.
pub fn mux_adts_as_m4a(adts: &[u8]) -> Result<Vec<u8>, Error> {
    sc_mp4::mux_aac(adts)
}

/// Demuxes a locally supported M4A container back into AAC ADTS frames.
pub fn demux_m4a_as_adts(input: &[u8]) -> Result<Vec<u8>, Error> {
    sc_mp4::demux_aac(input)
}

/// Runs the AAC-LC long-block analysis window and MDCT for one channel.
pub fn mdct_long_block(samples: &[f32; 2048]) -> Result<Vec<f32>, Error> {
    let window = sine_window(2048)?;
    mdct(&apply_window(samples, &window)?)
}

/// Runs AAC-LC long-block analysis and scalar spectral quantization.
pub fn quantize_long_block(samples: &[f32; 2048], step: f32) -> Result<Vec<i32>, Error> {
    quantize_spectrum(&mdct_long_block(samples)?, step, 8191)
}

/// Extracts one PCM channel and quantizes one AAC-LC long analysis block.
pub fn quantize_pcm_long_block(
    pcm: &AudioBuffer,
    channel: usize,
    start_frame: usize,
    step: f32,
) -> Result<Vec<i32>, Error> {
    let block = fixed_block::<2048>(&pcm.channel_block(channel, start_frame, 2048)?)?;
    quantize_long_block(&block, step)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AacCodebook {
    Zero,
    SignedPairs1,
    SignedPairs5,
    SignedPairs6,
    UnsignedPairs7,
    UnsignedPairs8,
    UnsignedPairs9,
    UnsignedPairs10,
    Escape,
}

impl AacCodebook {
    #[must_use]
    pub fn id(self) -> u8 {
        match self {
            Self::Zero => 0,
            Self::SignedPairs1 => 1,
            Self::SignedPairs5 => 5,
            Self::SignedPairs6 => 6,
            Self::UnsignedPairs7 => 7,
            Self::UnsignedPairs8 => 8,
            Self::UnsignedPairs9 => 9,
            Self::UnsignedPairs10 => 10,
            Self::Escape => 11,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AacSection {
    pub start: usize,
    pub end: usize,
    pub codebook: AacCodebook,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AacQuadSection {
    pub start: usize,
    pub end: usize,
    pub codebook_id: u8,
}

#[derive(Clone, Copy, Debug)]
struct AacMagnitudeSection<'a> {
    start: usize,
    end: usize,
    codebook_id: u8,
    table: &'a [HuffmanEntry<AacSpectralMagnitudePair>],
}

impl AacMagnitudeSection<'_> {
    fn is_zero(self) -> bool {
        self.codebook_id == AacCodebook::Zero.id()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AacSpectralPair {
    pub x: i16,
    pub y: i16,
}

impl AacSpectralPair {
    #[must_use]
    pub fn new(x: i16, y: i16) -> Self {
        Self { x, y }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AacSpectralMagnitudePair {
    pub x: u16,
    pub y: u16,
}

impl AacSpectralMagnitudePair {
    #[must_use]
    pub fn new(x: u16, y: u16) -> Self {
        Self { x, y }
    }
}

impl TryFrom<AacSpectralPair> for AacSpectralMagnitudePair {
    type Error = Error;

    fn try_from(pair: AacSpectralPair) -> Result<Self, Self::Error> {
        Ok(Self::new(
            pair.x
                .checked_abs()
                .and_then(|value| u16::try_from(value).ok())
                .ok_or(Error::InvalidInput("AAC spectral pair x overflows"))?,
            pair.y
                .checked_abs()
                .and_then(|value| u16::try_from(value).ok())
                .ok_or(Error::InvalidInput("AAC spectral pair y overflows"))?,
        ))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AacSpectralQuad {
    pub v: i16,
    pub w: i16,
    pub x: i16,
    pub y: i16,
}

impl AacSpectralQuad {
    #[must_use]
    pub fn new(v: i16, w: i16, x: i16, y: i16) -> Self {
        Self { v, w, x, y }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AacSpectralMagnitudeQuad {
    pub v: u16,
    pub w: u16,
    pub x: u16,
    pub y: u16,
}

impl AacSpectralMagnitudeQuad {
    #[must_use]
    pub fn new(v: u16, w: u16, x: u16, y: u16) -> Self {
        Self { v, w, x, y }
    }
}

impl TryFrom<AacSpectralQuad> for AacSpectralMagnitudeQuad {
    type Error = Error;

    fn try_from(quad: AacSpectralQuad) -> Result<Self, Self::Error> {
        Ok(Self::new(
            quad.v
                .checked_abs()
                .and_then(|value| u16::try_from(value).ok())
                .ok_or(Error::InvalidInput("AAC spectral quad v overflows"))?,
            quad.w
                .checked_abs()
                .and_then(|value| u16::try_from(value).ok())
                .ok_or(Error::InvalidInput("AAC spectral quad w overflows"))?,
            quad.x
                .checked_abs()
                .and_then(|value| u16::try_from(value).ok())
                .ok_or(Error::InvalidInput("AAC spectral quad x overflows"))?,
            quad.y
                .checked_abs()
                .and_then(|value| u16::try_from(value).ok())
                .ok_or(Error::InvalidInput("AAC spectral quad y overflows"))?,
        ))
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct AacSpectralTables<'a> {
    pub signed_pairs1: &'a [HuffmanEntry<AacSpectralPair>],
    pub signed_pairs5: &'a [HuffmanEntry<AacSpectralPair>],
    pub signed_pairs6: &'a [HuffmanEntry<AacSpectralPair>],
    pub escape: &'a [HuffmanEntry<AacSpectralPair>],
}

impl<'a> AacSpectralTables<'a> {
    fn table_for(
        self,
        codebook: AacCodebook,
    ) -> Result<&'a [HuffmanEntry<AacSpectralPair>], Error> {
        match codebook {
            AacCodebook::Zero => Ok(&[]),
            AacCodebook::SignedPairs1 => non_empty_table(self.signed_pairs1, "AAC codebook 1"),
            AacCodebook::SignedPairs5 => non_empty_table(self.signed_pairs5, "AAC codebook 5"),
            AacCodebook::SignedPairs6 => non_empty_table(self.signed_pairs6, "AAC codebook 6"),
            AacCodebook::UnsignedPairs7
            | AacCodebook::UnsignedPairs8
            | AacCodebook::UnsignedPairs9
            | AacCodebook::UnsignedPairs10 => Err(Error::UnsupportedFeature(
                "AAC unsigned-pairs codebooks 7/8/9/10 require magnitude tables",
            )),
            AacCodebook::Escape => non_empty_table(self.escape, "AAC escape codebook"),
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct AacSpectralMagnitudeTables<'a> {
    pub pairs1: &'a [HuffmanEntry<AacSpectralMagnitudePair>],
    pub pairs5: &'a [HuffmanEntry<AacSpectralMagnitudePair>],
    pub pairs6: &'a [HuffmanEntry<AacSpectralMagnitudePair>],
    pub escape: &'a [HuffmanEntry<AacSpectralMagnitudePair>],
}

impl<'a> AacSpectralMagnitudeTables<'a> {
    fn table_for(
        self,
        codebook: AacCodebook,
    ) -> Result<&'a [HuffmanEntry<AacSpectralMagnitudePair>], Error> {
        match codebook {
            AacCodebook::Zero => Ok(&[]),
            AacCodebook::SignedPairs1 => {
                non_empty_magnitude_table(self.pairs1, "AAC magnitude codebook 1")
            }
            AacCodebook::SignedPairs5 => {
                non_empty_magnitude_table(self.pairs5, "AAC magnitude codebook 5")
            }
            AacCodebook::SignedPairs6 => {
                non_empty_magnitude_table(self.pairs6, "AAC magnitude codebook 6")
            }
            AacCodebook::UnsignedPairs7 => Ok(aac_unsigned_pairs7_table()),
            AacCodebook::UnsignedPairs8 => Ok(aac_unsigned_pairs8_table()),
            AacCodebook::UnsignedPairs9 => Ok(aac_unsigned_pairs9_table()),
            AacCodebook::UnsignedPairs10 => Ok(aac_unsigned_pairs10_table()),
            AacCodebook::Escape => {
                non_empty_magnitude_table(self.escape, "AAC magnitude escape codebook")
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct AacSpectralMagnitudeQuadTables<'a> {
    pub quads1: &'a [HuffmanEntry<AacSpectralMagnitudeQuad>],
    pub quads2: &'a [HuffmanEntry<AacSpectralMagnitudeQuad>],
    pub quads3: &'a [HuffmanEntry<AacSpectralMagnitudeQuad>],
    pub quads4: &'a [HuffmanEntry<AacSpectralMagnitudeQuad>],
}

impl<'a> AacSpectralMagnitudeQuadTables<'a> {
    fn table_for_codebook_id(
        self,
        codebook_id: u8,
    ) -> Result<&'a [HuffmanEntry<AacSpectralMagnitudeQuad>], Error> {
        match codebook_id {
            1 => non_empty_quad_table(self.quads1, "AAC quad codebook 1"),
            2 => non_empty_quad_table(self.quads2, "AAC quad codebook 2"),
            3 => non_empty_quad_table(self.quads3, "AAC quad codebook 3"),
            4 => non_empty_quad_table(self.quads4, "AAC quad codebook 4"),
            _ => Err(Error::InvalidInput("AAC quad codebook id must be 1..=4")),
        }
    }
}

const EXPERIMENTAL_AAC_PAIRS1_TABLE: &[HuffmanEntry<AacSpectralMagnitudePair>] = &[
    HuffmanEntry {
        symbol: AacSpectralMagnitudePair { x: 0, y: 0 },
        code: HuffmanCode { bits: 0b0, len: 1 },
    },
    HuffmanEntry {
        symbol: AacSpectralMagnitudePair { x: 0, y: 1 },
        code: HuffmanCode { bits: 0b10, len: 2 },
    },
    HuffmanEntry {
        symbol: AacSpectralMagnitudePair { x: 1, y: 0 },
        code: HuffmanCode {
            bits: 0b110,
            len: 3,
        },
    },
    HuffmanEntry {
        symbol: AacSpectralMagnitudePair { x: 1, y: 1 },
        code: HuffmanCode {
            bits: 0b111,
            len: 3,
        },
    },
];

const AAC_UNSIGNED_PAIRS7_UNIT_MAGNITUDE_TABLE: &[HuffmanEntry<AacSpectralMagnitudePair>] = &[
    HuffmanEntry {
        symbol: AacSpectralMagnitudePair { x: 0, y: 0 },
        code: HuffmanCode { bits: 0b0, len: 1 },
    },
    HuffmanEntry {
        symbol: AacSpectralMagnitudePair { x: 0, y: 1 },
        code: HuffmanCode {
            bits: 0b101,
            len: 3,
        },
    },
    HuffmanEntry {
        symbol: AacSpectralMagnitudePair { x: 1, y: 0 },
        code: HuffmanCode {
            bits: 0b100,
            len: 3,
        },
    },
    HuffmanEntry {
        symbol: AacSpectralMagnitudePair { x: 1, y: 1 },
        code: HuffmanCode {
            bits: 0b1100,
            len: 4,
        },
    },
];

#[rustfmt::skip]
const AAC_UNSIGNED_PAIRS7_LENS: [u8; 64] = [
     1,  3,  6,  7,  8,  9, 10, 11,  3,  4,  6,  7,  8,  8,  9,  9,
     6,  6,  7,  8,  8,  9,  9, 10,  7,  7,  8,  8,  9,  9, 10, 10,
     8,  8,  9,  9, 10, 10, 10, 11,  9,  8,  9,  9, 10, 10, 11, 11,
    10,  9,  9, 10, 10, 11, 12, 12, 11, 10, 10, 10, 11, 11, 12, 12,
];

#[rustfmt::skip]
const AAC_UNSIGNED_PAIRS7_CODES: [u32; 64] = [
    0x000, 0x005, 0x037, 0x074, 0x0f2, 0x1eb, 0x3ed, 0x7f7,
    0x004, 0x00c, 0x035, 0x071, 0x0ec, 0x0ee, 0x1ee, 0x1f5,
    0x036, 0x034, 0x072, 0x0ea, 0x0f1, 0x1e9, 0x1f3, 0x3f5,
    0x073, 0x070, 0x0eb, 0x0f0, 0x1f1, 0x1f0, 0x3ec, 0x3fa,
    0x0f3, 0x0ed, 0x1e8, 0x1ef, 0x3ef, 0x3f1, 0x3f9, 0x7fb,
    0x1ed, 0x0ef, 0x1ea, 0x1f2, 0x3f3, 0x3f8, 0x7f9, 0x7fc,
    0x3ee, 0x1ec, 0x1f4, 0x3f4, 0x3f7, 0x7f8, 0xffd, 0xffe,
    0x7f6, 0x3f0, 0x3f2, 0x3f6, 0x7fa, 0x7fd, 0xffc, 0xfff,
];

static AAC_UNSIGNED_PAIRS7_TABLE: OnceLock<Vec<HuffmanEntry<AacSpectralMagnitudePair>>> =
    OnceLock::new();

#[rustfmt::skip]
const AAC_UNSIGNED_PAIRS8_LENS: [u8; 64] = [
     5,  4,  5,  6,  7,  8,  9, 10,  4,  3,  4,  5,  6,  7,  7,  8,
     5,  4,  4,  5,  6,  7,  7,  8,  6,  5,  5,  6,  6,  7,  8,  8,
     7,  6,  6,  6,  7,  7,  8,  9,  8,  7,  6,  7,  7,  8,  8, 10,
     9,  7,  7,  8,  8,  8,  9,  9, 10,  8,  8,  8,  9,  9,  9, 10,
];

#[rustfmt::skip]
const AAC_UNSIGNED_PAIRS8_CODES: [u32; 64] = [
    0x00e, 0x005, 0x010, 0x030, 0x06f, 0x0f1, 0x1fa, 0x3fe,
    0x003, 0x000, 0x004, 0x012, 0x02c, 0x06a, 0x075, 0x0f8,
    0x00f, 0x002, 0x006, 0x014, 0x02e, 0x069, 0x072, 0x0f5,
    0x02f, 0x011, 0x013, 0x02a, 0x032, 0x06c, 0x0ec, 0x0fa,
    0x071, 0x02b, 0x02d, 0x031, 0x06d, 0x070, 0x0f2, 0x1f9,
    0x0ef, 0x068, 0x033, 0x06b, 0x06e, 0x0ee, 0x0f9, 0x3fc,
    0x1f8, 0x074, 0x073, 0x0ed, 0x0f0, 0x0f6, 0x1f6, 0x1fd,
    0x3fd, 0x0f3, 0x0f4, 0x0f7, 0x1f7, 0x1fb, 0x1fc, 0x3ff,
];

static AAC_UNSIGNED_PAIRS8_TABLE: OnceLock<Vec<HuffmanEntry<AacSpectralMagnitudePair>>> =
    OnceLock::new();

#[rustfmt::skip]
const AAC_UNSIGNED_PAIRS9_LENS: [u8; 169] = [
     1,  3,  6,  8,  9, 10, 10, 11, 11, 12, 12, 13, 13,  3,  4,  6,
     7,  8,  8,  9, 10, 10, 10, 11, 12, 12,  6,  6,  7,  8,  8,  9,
    10, 10, 10, 11, 12, 12, 12,  8,  7,  8,  9,  9, 10, 10, 11, 11,
    11, 12, 12, 13,  9,  8,  9,  9, 10, 10, 11, 11, 11, 12, 12, 12,
    13, 10,  9,  9, 10, 11, 11, 11, 12, 11, 12, 12, 13, 13, 11,  9,
    10, 11, 11, 11, 12, 12, 12, 12, 13, 13, 13, 11, 10, 10, 11, 11,
    12, 12, 13, 13, 13, 13, 13, 13, 11, 10, 10, 11, 11, 11, 12, 12,
    13, 13, 14, 13, 14, 11, 10, 11, 11, 12, 12, 12, 12, 13, 13, 14,
    14, 14, 12, 11, 11, 12, 12, 12, 13, 13, 13, 14, 14, 14, 15, 12,
    11, 12, 12, 12, 13, 13, 13, 13, 14, 14, 15, 15, 13, 12, 12, 12,
    13, 13, 13, 13, 14, 14, 14, 14, 15,
];

#[rustfmt::skip]
const AAC_UNSIGNED_PAIRS9_CODES: [u32; 169] = [
    0x0000, 0x0005, 0x0037, 0x00e7, 0x01de, 0x03ce, 0x03d9, 0x07c8,
    0x07cd, 0x0fc8, 0x0fdd, 0x1fe4, 0x1fec, 0x0004, 0x000c, 0x0035,
    0x0072, 0x00ea, 0x00ed, 0x01e2, 0x03d1, 0x03d3, 0x03e0, 0x07d8,
    0x0fcf, 0x0fd5, 0x0036, 0x0034, 0x0071, 0x00e8, 0x00ec, 0x01e1,
    0x03cf, 0x03dd, 0x03db, 0x07d0, 0x0fc7, 0x0fd4, 0x0fe4, 0x00e6,
    0x0070, 0x00e9, 0x01dd, 0x01e3, 0x03d2, 0x03dc, 0x07cc, 0x07ca,
    0x07de, 0x0fd8, 0x0fea, 0x1fdb, 0x01df, 0x00eb, 0x01dc, 0x01e6,
    0x03d5, 0x03de, 0x07cb, 0x07dd, 0x07dc, 0x0fcd, 0x0fe2, 0x0fe7,
    0x1fe1, 0x03d0, 0x01e0, 0x01e4, 0x03d6, 0x07c5, 0x07d1, 0x07db,
    0x0fd2, 0x07e0, 0x0fd9, 0x0feb, 0x1fe3, 0x1fe9, 0x07c4, 0x01e5,
    0x03d7, 0x07c6, 0x07cf, 0x07da, 0x0fcb, 0x0fda, 0x0fe3, 0x0fe9,
    0x1fe6, 0x1ff3, 0x1ff7, 0x07d3, 0x03d8, 0x03e1, 0x07d4, 0x07d9,
    0x0fd3, 0x0fde, 0x1fdd, 0x1fd9, 0x1fe2, 0x1fea, 0x1ff1, 0x1ff6,
    0x07d2, 0x03d4, 0x03da, 0x07c7, 0x07d7, 0x07e2, 0x0fce, 0x0fdb,
    0x1fd8, 0x1fee, 0x3ff0, 0x1ff4, 0x3ff2, 0x07e1, 0x03df, 0x07c9,
    0x07d6, 0x0fca, 0x0fd0, 0x0fe5, 0x0fe6, 0x1feb, 0x1fef, 0x3ff3,
    0x3ff4, 0x3ff5, 0x0fe0, 0x07ce, 0x07d5, 0x0fc6, 0x0fd1, 0x0fe1,
    0x1fe0, 0x1fe8, 0x1ff0, 0x3ff1, 0x3ff8, 0x3ff6, 0x7ffc, 0x0fe8,
    0x07df, 0x0fc9, 0x0fd7, 0x0fdc, 0x1fdc, 0x1fdf, 0x1fed, 0x1ff5,
    0x3ff9, 0x3ffb, 0x7ffd, 0x7ffe, 0x1fe7, 0x0fcc, 0x0fd6, 0x0fdf,
    0x1fde, 0x1fda, 0x1fe5, 0x1ff2, 0x3ffa, 0x3ff7, 0x3ffc, 0x3ffd,
    0x7fff,
];

static AAC_UNSIGNED_PAIRS9_TABLE: OnceLock<Vec<HuffmanEntry<AacSpectralMagnitudePair>>> =
    OnceLock::new();

#[rustfmt::skip]
const AAC_UNSIGNED_PAIRS10_LENS: [u8; 169] = [
     6,  5,  6,  6,  7,  8,  9, 10, 10, 10, 11, 11, 12,  5,  4,  4,
     5,  6,  7,  7,  8,  8,  9, 10, 10, 11,  6,  4,  5,  5,  6,  6,
     7,  8,  8,  9,  9, 10, 10,  6,  5,  5,  5,  6,  7,  7,  8,  8,
     9,  9, 10, 10,  7,  6,  6,  6,  6,  7,  7,  8,  8,  9,  9, 10,
    10,  8,  7,  6,  7,  7,  7,  8,  8,  8,  9, 10, 10, 11,  9,  7,
     7,  7,  7,  8,  8,  9,  9,  9, 10, 10, 11,  9,  8,  8,  8,  8,
     8,  9,  9,  9, 10, 10, 11, 11,  9,  8,  8,  8,  8,  8,  9,  9,
    10, 10, 10, 11, 11, 10,  9,  9,  9,  9,  9,  9, 10, 10, 10, 11,
    11, 12, 10,  9,  9,  9,  9, 10, 10, 10, 10, 11, 11, 11, 12, 11,
    10,  9, 10, 10, 10, 10, 10, 11, 11, 11, 11, 12, 11, 10, 10, 10,
    10, 10, 10, 11, 11, 12, 12, 12, 12,
];

#[rustfmt::skip]
const AAC_UNSIGNED_PAIRS10_CODES: [u32; 169] = [
    0x022, 0x008, 0x01d, 0x026, 0x05f, 0x0d3, 0x1cf, 0x3d0,
    0x3d7, 0x3ed, 0x7f0, 0x7f6, 0xffd, 0x007, 0x000, 0x001,
    0x009, 0x020, 0x054, 0x060, 0x0d5, 0x0dc, 0x1d4, 0x3cd,
    0x3de, 0x7e7, 0x01c, 0x002, 0x006, 0x00c, 0x01e, 0x028,
    0x05b, 0x0cd, 0x0d9, 0x1ce, 0x1dc, 0x3d9, 0x3f1, 0x025,
    0x00b, 0x00a, 0x00d, 0x024, 0x057, 0x061, 0x0cc, 0x0dd,
    0x1cc, 0x1de, 0x3d3, 0x3e7, 0x05d, 0x021, 0x01f, 0x023,
    0x027, 0x059, 0x064, 0x0d8, 0x0df, 0x1d2, 0x1e2, 0x3dd,
    0x3ee, 0x0d1, 0x055, 0x029, 0x056, 0x058, 0x062, 0x0ce,
    0x0e0, 0x0e2, 0x1da, 0x3d4, 0x3e3, 0x7eb, 0x1c9, 0x05e,
    0x05a, 0x05c, 0x063, 0x0ca, 0x0da, 0x1c7, 0x1ca, 0x1e0,
    0x3db, 0x3e8, 0x7ec, 0x1e3, 0x0d2, 0x0cb, 0x0d0, 0x0d7,
    0x0db, 0x1c6, 0x1d5, 0x1d8, 0x3ca, 0x3da, 0x7ea, 0x7f1,
    0x1e1, 0x0d4, 0x0cf, 0x0d6, 0x0de, 0x0e1, 0x1d0, 0x1d6,
    0x3d1, 0x3d5, 0x3f2, 0x7ee, 0x7fb, 0x3e9, 0x1cd, 0x1c8,
    0x1cb, 0x1d1, 0x1d7, 0x1df, 0x3cf, 0x3e0, 0x3ef, 0x7e6,
    0x7f8, 0xffa, 0x3eb, 0x1dd, 0x1d3, 0x1d9, 0x1db, 0x3d2,
    0x3cc, 0x3dc, 0x3ea, 0x7ed, 0x7f3, 0x7f9, 0xff9, 0x7f2,
    0x3ce, 0x1e4, 0x3cb, 0x3d8, 0x3d6, 0x3e2, 0x3e5, 0x7e8,
    0x7f4, 0x7f5, 0x7f7, 0xffb, 0x7fa, 0x3ec, 0x3df, 0x3e1,
    0x3e4, 0x3e6, 0x3f0, 0x7e9, 0x7ef, 0xff8, 0xffe, 0xffc,
    0xfff,
];

static AAC_UNSIGNED_PAIRS10_TABLE: OnceLock<Vec<HuffmanEntry<AacSpectralMagnitudePair>>> =
    OnceLock::new();

#[rustfmt::skip]
const AAC_ESCAPE_LENS: [u8; 289] = [
     4,  5,  6,  7,  8,  8,  9, 10, 10, 10, 11, 11, 12, 11, 12, 12,
    10,  5,  4,  5,  6,  7,  7,  8,  8,  9,  9,  9, 10, 10, 10, 10,
    11,  8,  6,  5,  5,  6,  7,  7,  8,  8,  8,  9,  9,  9, 10, 10,
    10, 10,  8,  7,  6,  6,  6,  7,  7,  8,  8,  8,  9,  9,  9, 10,
    10, 10, 10,  8,  8,  7,  7,  7,  7,  8,  8,  8,  8,  9,  9,  9,
    10, 10, 10, 10,  8,  8,  7,  7,  7,  7,  8,  8,  8,  9,  9,  9,
     9, 10, 10, 10, 10,  8,  9,  8,  8,  8,  8,  8,  8,  8,  9,  9,
     9, 10, 10, 10, 10, 10,  8,  9,  8,  8,  8,  8,  8,  8,  9,  9,
     9, 10, 10, 10, 10, 10, 10,  8, 10,  9,  8,  8,  9,  9,  9,  9,
     9, 10, 10, 10, 10, 10, 10, 11,  8, 10,  9,  9,  9,  9,  9,  9,
     9, 10, 10, 10, 10, 10, 10, 11, 11,  8, 11,  9,  9,  9,  9,  9,
     9, 10, 10, 10, 10, 10, 11, 10, 11, 11,  8, 11, 10,  9,  9, 10,
     9, 10, 10, 10, 10, 10, 11, 11, 11, 11, 11,  8, 11, 10, 10, 10,
    10, 10, 10, 10, 10, 10, 10, 11, 11, 11, 11, 11,  9, 11, 10,  9,
     9, 10, 10, 10, 10, 10, 10, 11, 11, 11, 11, 11, 11,  9, 11, 10,
    10, 10, 10, 10, 10, 10, 10, 10, 11, 11, 11, 11, 11, 11,  9, 12,
    10, 10, 10, 10, 10, 10, 10, 11, 11, 11, 11, 11, 11, 12, 12,  9,
     9,  8,  8,  8,  8,  8,  8,  8,  8,  8,  8,  8,  8,  8,  8,  9,
     5,
];

#[rustfmt::skip]
const AAC_ESCAPE_CODES: [u32; 289] = [
    0x000, 0x006, 0x019, 0x03d, 0x09c, 0x0c6, 0x1a7, 0x390,
    0x3c2, 0x3df, 0x7e6, 0x7f3, 0xffb, 0x7ec, 0xffa, 0xffe,
    0x38e, 0x005, 0x001, 0x008, 0x014, 0x037, 0x042, 0x092,
    0x0af, 0x191, 0x1a5, 0x1b5, 0x39e, 0x3c0, 0x3a2, 0x3cd,
    0x7d6, 0x0ae, 0x017, 0x007, 0x009, 0x018, 0x039, 0x040,
    0x08e, 0x0a3, 0x0b8, 0x199, 0x1ac, 0x1c1, 0x3b1, 0x396,
    0x3be, 0x3ca, 0x09d, 0x03c, 0x015, 0x016, 0x01a, 0x03b,
    0x044, 0x091, 0x0a5, 0x0be, 0x196, 0x1ae, 0x1b9, 0x3a1,
    0x391, 0x3a5, 0x3d5, 0x094, 0x09a, 0x036, 0x038, 0x03a,
    0x041, 0x08c, 0x09b, 0x0b0, 0x0c3, 0x19e, 0x1ab, 0x1bc,
    0x39f, 0x38f, 0x3a9, 0x3cf, 0x093, 0x0bf, 0x03e, 0x03f,
    0x043, 0x045, 0x09e, 0x0a7, 0x0b9, 0x194, 0x1a2, 0x1ba,
    0x1c3, 0x3a6, 0x3a7, 0x3bb, 0x3d4, 0x09f, 0x1a0, 0x08f,
    0x08d, 0x090, 0x098, 0x0a6, 0x0b6, 0x0c4, 0x19f, 0x1af,
    0x1bf, 0x399, 0x3bf, 0x3b4, 0x3c9, 0x3e7, 0x0a8, 0x1b6,
    0x0ab, 0x0a4, 0x0aa, 0x0b2, 0x0c2, 0x0c5, 0x198, 0x1a4,
    0x1b8, 0x38c, 0x3a4, 0x3c4, 0x3c6, 0x3dd, 0x3e8, 0x0ad,
    0x3af, 0x192, 0x0bd, 0x0bc, 0x18e, 0x197, 0x19a, 0x1a3,
    0x1b1, 0x38d, 0x398, 0x3b7, 0x3d3, 0x3d1, 0x3db, 0x7dd,
    0x0b4, 0x3de, 0x1a9, 0x19b, 0x19c, 0x1a1, 0x1aa, 0x1ad,
    0x1b3, 0x38b, 0x3b2, 0x3b8, 0x3ce, 0x3e1, 0x3e0, 0x7d2,
    0x7e5, 0x0b7, 0x7e3, 0x1bb, 0x1a8, 0x1a6, 0x1b0, 0x1b2,
    0x1b7, 0x39b, 0x39a, 0x3ba, 0x3b5, 0x3d6, 0x7d7, 0x3e4,
    0x7d8, 0x7ea, 0x0ba, 0x7e8, 0x3a0, 0x1bd, 0x1b4, 0x38a,
    0x1c4, 0x392, 0x3aa, 0x3b0, 0x3bc, 0x3d7, 0x7d4, 0x7dc,
    0x7db, 0x7d5, 0x7f0, 0x0c1, 0x7fb, 0x3c8, 0x3a3, 0x395,
    0x39d, 0x3ac, 0x3ae, 0x3c5, 0x3d8, 0x3e2, 0x3e6, 0x7e4,
    0x7e7, 0x7e0, 0x7e9, 0x7f7, 0x190, 0x7f2, 0x393, 0x1be,
    0x1c0, 0x394, 0x397, 0x3ad, 0x3c3, 0x3c1, 0x3d2, 0x7da,
    0x7d9, 0x7df, 0x7eb, 0x7f4, 0x7fa, 0x195, 0x7f8, 0x3bd,
    0x39c, 0x3ab, 0x3a8, 0x3b3, 0x3b9, 0x3d0, 0x3e3, 0x3e5,
    0x7e2, 0x7de, 0x7ed, 0x7f1, 0x7f9, 0x7fc, 0x193, 0xffd,
    0x3dc, 0x3b6, 0x3c7, 0x3cc, 0x3cb, 0x3d9, 0x3da, 0x7d3,
    0x7e1, 0x7ee, 0x7ef, 0x7f5, 0x7f6, 0xffc, 0xfff, 0x19d,
    0x1c2, 0x0b5, 0x0a1, 0x096, 0x097, 0x095, 0x099, 0x0a0,
    0x0a2, 0x0ac, 0x0a9, 0x0b1, 0x0b3, 0x0bb, 0x0c0, 0x18f,
    0x004,
];

static AAC_ESCAPE_TABLE: OnceLock<Vec<HuffmanEntry<AacSpectralMagnitudePair>>> = OnceLock::new();

pub const AAC_SCALE_FACTOR_DELTA_ZERO_TABLE: &[HuffmanEntry<AacScaleFactorDelta>] =
    &[HuffmanEntry {
        symbol: AacScaleFactorDelta { delta: 0 },
        code: HuffmanCode { bits: 0, len: 1 },
    }];

#[rustfmt::skip]
const AAC_SCALE_FACTOR_CODEBOOK_LENS: [u8; 121] = [
    18, 18, 18, 18, 19, 19, 19, 19, 19, 19, 19, 19, 19, 19, 19, 19,
    19, 19, 19, 18, 19, 18, 17, 17, 16, 17, 16, 16, 16, 16, 15, 15,
    14, 14, 14, 14, 14, 14, 13, 13, 12, 12, 12, 11, 12, 11, 10, 10,
    10,  9,  9,  8,  8,  8,  7,  6,  6,  5,  4,  3,  1,  4,  4,  5,
     6,  6,  7,  7,  8,  8,  9,  9, 10, 10, 10, 11, 11, 11, 11, 12,
    12, 13, 13, 13, 14, 14, 16, 15, 16, 15, 18, 19, 19, 19, 19, 19,
    19, 19, 19, 19, 19, 19, 19, 19, 19, 19, 19, 19, 19, 19, 19, 19,
    19, 19, 19, 19, 19, 19, 19, 19, 19,
];

#[rustfmt::skip]
const AAC_SCALE_FACTOR_CODEBOOK_CODES: [u32; 121] = [
    0x3FFE8, 0x3FFE6, 0x3FFE7, 0x3FFE5, 0x7FFF5, 0x7FFF1, 0x7FFED, 0x7FFF6,
    0x7FFEE, 0x7FFEF, 0x7FFF0, 0x7FFFC, 0x7FFFD, 0x7FFFF, 0x7FFFE, 0x7FFF7,
    0x7FFF8, 0x7FFFB, 0x7FFF9, 0x3FFE4, 0x7FFFA, 0x3FFE3, 0x1FFEF, 0x1FFF0,
    0x0FFF5, 0x1FFEE, 0x0FFF2, 0x0FFF3, 0x0FFF4, 0x0FFF1, 0x07FF6, 0x07FF7,
    0x03FF9, 0x03FF5, 0x03FF7, 0x03FF3, 0x03FF6, 0x03FF2, 0x01FF7, 0x01FF5,
    0x00FF9, 0x00FF7, 0x00FF6, 0x007F9, 0x00FF4, 0x007F8, 0x003F9, 0x003F7,
    0x003F5, 0x001F8, 0x001F7, 0x000FA, 0x000F8, 0x000F6, 0x00079, 0x0003A,
    0x00038, 0x0001A, 0x0000B, 0x00004, 0x00000, 0x0000A, 0x0000C, 0x0001B,
    0x00039, 0x0003B, 0x00078, 0x0007A, 0x000F7, 0x000F9, 0x001F6, 0x001F9,
    0x003F4, 0x003F6, 0x003F8, 0x007F5, 0x007F4, 0x007F6, 0x007F7, 0x00FF5,
    0x00FF8, 0x01FF4, 0x01FF6, 0x01FF8, 0x03FF8, 0x03FF4, 0x0FFF0, 0x07FF4,
    0x0FFF6, 0x07FF5, 0x3FFE2, 0x7FFD9, 0x7FFDA, 0x7FFDB, 0x7FFDC, 0x7FFDD,
    0x7FFDE, 0x7FFD8, 0x7FFD2, 0x7FFD3, 0x7FFD4, 0x7FFD5, 0x7FFD6, 0x7FFF2,
    0x7FFDF, 0x7FFE7, 0x7FFE8, 0x7FFE9, 0x7FFEA, 0x7FFEB, 0x7FFE6, 0x7FFE0,
    0x7FFE1, 0x7FFE2, 0x7FFE3, 0x7FFE4, 0x7FFE5, 0x7FFD7, 0x7FFEC, 0x7FFF4,
    0x7FFF3,
];

/// Returns a minimal experimental AAC spectral table set for zero/one pairs.
///
/// This is not the AAC-LC standard Huffman table set. It exists to exercise
/// non-zero section and sign-bit payload plumbing while the full clean-room
/// codebooks and rate control are being implemented.
#[must_use]
pub fn experimental_unit_magnitude_spectral_tables() -> AacSpectralMagnitudeTables<'static> {
    AacSpectralMagnitudeTables {
        pairs1: EXPERIMENTAL_AAC_PAIRS1_TABLE,
        ..Default::default()
    }
}

/// Returns a compatibility table set for callers of the current AAC production helper.
///
/// The offset-based production path enables the standard codebook-7 zero/one
/// subset internally so the public magnitude-table struct can remain semver
/// compatible while the full table surface is still being designed.
#[must_use]
pub fn aac_unsigned_pairs7_unit_magnitude_spectral_tables() -> AacSpectralMagnitudeTables<'static> {
    AacSpectralMagnitudeTables::default()
}

/// Returns the standard AAC-LC spectral table set currently implemented.
///
/// The unsigned-pairs codebooks 7/8/9/10 are provided implicitly by
/// [`AacSpectralMagnitudeTables::table_for`]. This table set adds the standard
/// escape codebook 11 so bitrate/step search can keep larger magnitudes instead
/// of forcing them out of range. Signed/quad codebooks remain pending.
#[must_use]
pub fn aac_lc_standard_spectral_tables() -> AacSpectralMagnitudeTables<'static> {
    AacSpectralMagnitudeTables {
        escape: aac_escape_table(),
        ..Default::default()
    }
}

/// Returns the standard AAC unsigned-pairs codebook 7 for magnitudes 0..=7.
#[must_use]
pub fn aac_unsigned_pairs7_table() -> &'static [HuffmanEntry<AacSpectralMagnitudePair>] {
    AAC_UNSIGNED_PAIRS7_TABLE
        .get_or_init(|| {
            AAC_UNSIGNED_PAIRS7_CODES
                .iter()
                .zip(AAC_UNSIGNED_PAIRS7_LENS)
                .enumerate()
                .map(|(index, (&bits, len))| HuffmanEntry {
                    symbol: AacSpectralMagnitudePair {
                        x: (index / 8) as u16,
                        y: (index % 8) as u16,
                    },
                    code: HuffmanCode { bits, len },
                })
                .collect()
        })
        .as_slice()
}

/// Returns the standard AAC unsigned-pairs codebook 8 for magnitudes 0..=7.
#[must_use]
pub fn aac_unsigned_pairs8_table() -> &'static [HuffmanEntry<AacSpectralMagnitudePair>] {
    AAC_UNSIGNED_PAIRS8_TABLE
        .get_or_init(|| {
            AAC_UNSIGNED_PAIRS8_CODES
                .iter()
                .zip(AAC_UNSIGNED_PAIRS8_LENS)
                .enumerate()
                .map(|(index, (&bits, len))| HuffmanEntry {
                    symbol: AacSpectralMagnitudePair {
                        x: (index / 8) as u16,
                        y: (index % 8) as u16,
                    },
                    code: HuffmanCode { bits, len },
                })
                .collect()
        })
        .as_slice()
}

/// Returns the standard AAC unsigned-pairs codebook 9 for magnitudes 0..=12.
#[must_use]
pub fn aac_unsigned_pairs9_table() -> &'static [HuffmanEntry<AacSpectralMagnitudePair>] {
    AAC_UNSIGNED_PAIRS9_TABLE
        .get_or_init(|| {
            AAC_UNSIGNED_PAIRS9_CODES
                .iter()
                .zip(AAC_UNSIGNED_PAIRS9_LENS)
                .enumerate()
                .map(|(index, (&bits, len))| HuffmanEntry {
                    symbol: AacSpectralMagnitudePair {
                        x: (index / 13) as u16,
                        y: (index % 13) as u16,
                    },
                    code: HuffmanCode { bits, len },
                })
                .collect()
        })
        .as_slice()
}

/// Returns the standard AAC unsigned-pairs codebook 10 for magnitudes 0..=12.
#[must_use]
pub fn aac_unsigned_pairs10_table() -> &'static [HuffmanEntry<AacSpectralMagnitudePair>] {
    AAC_UNSIGNED_PAIRS10_TABLE
        .get_or_init(|| {
            AAC_UNSIGNED_PAIRS10_CODES
                .iter()
                .zip(AAC_UNSIGNED_PAIRS10_LENS)
                .enumerate()
                .map(|(index, (&bits, len))| HuffmanEntry {
                    symbol: AacSpectralMagnitudePair {
                        x: (index / 13) as u16,
                        y: (index % 13) as u16,
                    },
                    code: HuffmanCode { bits, len },
                })
                .collect()
        })
        .as_slice()
}

/// Returns the standard AAC escape codebook 11 for magnitudes 0..=16.
///
/// Magnitude 16 is the escape sentinel; actual magnitudes above 16 are packed
/// by appending escape suffix bits after the Huffman codeword.
#[must_use]
pub fn aac_escape_table() -> &'static [HuffmanEntry<AacSpectralMagnitudePair>] {
    AAC_ESCAPE_TABLE
        .get_or_init(|| {
            AAC_ESCAPE_CODES
                .iter()
                .zip(AAC_ESCAPE_LENS)
                .enumerate()
                .map(|(index, (&bits, len))| HuffmanEntry {
                    symbol: AacSpectralMagnitudePair {
                        x: (index / 17) as u16,
                        y: (index % 17) as u16,
                    },
                    code: HuffmanCode { bits, len },
                })
                .collect()
        })
        .as_slice()
}

/// Returns the standard AAC unsigned-pairs codebook 7 entries for magnitudes 0/1.
///
/// This compatibility helper exposes the compact subset older diagnostics used;
/// new code should prefer `aac_unsigned_pairs7_table()`.
#[must_use]
pub fn aac_unsigned_pairs7_unit_magnitude_table(
) -> &'static [HuffmanEntry<AacSpectralMagnitudePair>] {
    AAC_UNSIGNED_PAIRS7_UNIT_MAGNITUDE_TABLE
}

/// Returns a minimal experimental AAC scale-factor delta table.
///
/// This is not the AAC-LC standard scale-factor Huffman table. It exists to
/// keep older tests deterministic; new production-shaped paths should use
/// `aac_scale_factor_delta_table()`.
#[must_use]
pub fn experimental_aac_scale_factor_delta_table() -> Vec<HuffmanEntry<AacScaleFactorDelta>> {
    (-16..=16)
        .enumerate()
        .map(|(index, delta)| HuffmanEntry {
            symbol: AacScaleFactorDelta::new(delta),
            code: HuffmanCode {
                bits: index as u32,
                len: 6,
            },
        })
        .collect()
}

/// Returns the standard AAC scale-factor Huffman table for DPCM deltas -60..=60.
#[must_use]
pub fn aac_scale_factor_delta_table() -> Vec<HuffmanEntry<AacScaleFactorDelta>> {
    AAC_SCALE_FACTOR_CODEBOOK_CODES
        .iter()
        .zip(AAC_SCALE_FACTOR_CODEBOOK_LENS)
        .enumerate()
        .map(|(index, (&bits, len))| HuffmanEntry {
            symbol: AacScaleFactorDelta::new(index as i16 - 60),
            code: HuffmanCode { bits, len },
        })
        .collect()
}

/// Returns the standard AAC scale-factor codebook entry for a zero DPCM delta.
///
/// Prefer `aac_scale_factor_delta_table()` when non-zero deltas are possible.
#[must_use]
pub fn aac_scale_factor_delta_zero_table() -> &'static [HuffmanEntry<AacScaleFactorDelta>] {
    AAC_SCALE_FACTOR_DELTA_ZERO_TABLE
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AacScaleFactorDelta {
    pub delta: i16,
}

impl AacScaleFactorDelta {
    #[must_use]
    pub fn new(delta: i16) -> Self {
        Self { delta }
    }
}

/// Converts one AAC section's quantized coefficients into pair symbols.
pub fn spectral_pairs_for_section(
    quantized: &[i32],
    section: &AacSection,
) -> Result<Vec<AacSpectralPair>, Error> {
    if section.end <= section.start || section.end > quantized.len() {
        return Err(Error::InvalidInput("invalid AAC section range"));
    }
    if section.codebook == AacCodebook::Zero {
        return Ok(Vec::new());
    }

    let coeffs = &quantized[section.start..section.end];
    if coeffs.len() % 2 != 0 {
        return Err(Error::InvalidInput(
            "AAC spectral pair section must have even length",
        ));
    }

    coeffs
        .chunks_exact(2)
        .map(|pair| {
            Ok(AacSpectralPair::new(
                i16::try_from(pair[0]).map_err(|_| {
                    Error::InvalidInput("AAC spectral pair coefficient exceeds i16 range")
                })?,
                i16::try_from(pair[1]).map_err(|_| {
                    Error::InvalidInput("AAC spectral pair coefficient exceeds i16 range")
                })?,
            ))
        })
        .collect()
}

/// Converts one AAC section's quantized coefficients into quadruple symbols.
pub fn spectral_quads_for_section(
    quantized: &[i32],
    section: &AacSection,
) -> Result<Vec<AacSpectralQuad>, Error> {
    if section.end <= section.start || section.end > quantized.len() {
        return Err(Error::InvalidInput("invalid AAC section range"));
    }
    if section.codebook == AacCodebook::Zero {
        return Ok(Vec::new());
    }

    let coeffs = &quantized[section.start..section.end];
    if coeffs.len() % 4 != 0 {
        return Err(Error::InvalidInput(
            "AAC spectral quad section must have length divisible by four",
        ));
    }

    coeffs
        .chunks_exact(4)
        .map(|quad| {
            Ok(AacSpectralQuad::new(
                i16::try_from(quad[0]).map_err(|_| {
                    Error::InvalidInput("AAC spectral quad coefficient exceeds i16 range")
                })?,
                i16::try_from(quad[1]).map_err(|_| {
                    Error::InvalidInput("AAC spectral quad coefficient exceeds i16 range")
                })?,
                i16::try_from(quad[2]).map_err(|_| {
                    Error::InvalidInput("AAC spectral quad coefficient exceeds i16 range")
                })?,
                i16::try_from(quad[3]).map_err(|_| {
                    Error::InvalidInput("AAC spectral quad coefficient exceeds i16 range")
                })?,
            ))
        })
        .collect()
}

/// Groups quantized AAC spectral coefficients into contiguous codebook sections.
pub fn plan_sections(quantized: &[i32], band_width: usize) -> Result<Vec<AacSection>, Error> {
    if band_width == 0 {
        return Err(Error::InvalidInput(
            "AAC section band width must be non-zero",
        ));
    }
    if quantized.len() % band_width != 0 {
        return Err(Error::InvalidInput(
            "AAC section band width must divide spectrum length",
        ));
    }

    let mut sections = Vec::<AacSection>::new();
    for (band_index, band) in quantized.chunks(band_width).enumerate() {
        let codebook = classify_aac_codebook(band)?;
        let start = band_index * band_width;
        let end = start + band_width;
        match sections.last_mut() {
            Some(section) if section.codebook == codebook => section.end = end,
            _ => sections.push(AacSection {
                start,
                end,
                codebook,
            }),
        }
    }
    Ok(sections)
}

/// Selects the shortest available AAC spectral codebook from magnitude tables.
pub fn select_codebook_by_bit_cost(
    quantized: &[i32],
    tables: AacSpectralMagnitudeTables<'_>,
) -> Result<AacCodebook, Error> {
    if quantized.iter().all(|&coeff| coeff == 0) {
        return Ok(AacCodebook::Zero);
    }

    let section = AacSection {
        start: 0,
        end: quantized.len(),
        codebook: AacCodebook::SignedPairs1,
    };
    let pairs = spectral_pairs_for_section(quantized, &section)?;
    let candidates = [
        (AacCodebook::SignedPairs1, tables.pairs1),
        (AacCodebook::SignedPairs5, tables.pairs5),
        (AacCodebook::SignedPairs6, tables.pairs6),
        (AacCodebook::UnsignedPairs7, aac_unsigned_pairs7_table()),
        (AacCodebook::UnsignedPairs8, aac_unsigned_pairs8_table()),
        (AacCodebook::UnsignedPairs9, aac_unsigned_pairs9_table()),
        (AacCodebook::UnsignedPairs10, aac_unsigned_pairs10_table()),
        (AacCodebook::Escape, tables.escape),
    ];
    let mut best: Option<(AacCodebook, usize)> = None;
    for (codebook, table) in candidates {
        if table.is_empty() {
            continue;
        }
        let Ok(packed) = pack_spectral_pairs_with_sign_bits(&pairs, table) else {
            continue;
        };
        if best
            .as_ref()
            .is_none_or(|(_, bit_len)| packed.bit_len < *bit_len)
        {
            best = Some((codebook, packed.bit_len));
        }
    }

    best.map(|(codebook, _)| codebook)
        .ok_or(Error::UnsupportedFeature("AAC spectral codebook"))
}

fn select_magnitude_section_by_bit_cost<'a>(
    start: usize,
    end: usize,
    quantized: &[i32],
    tables: AacSpectralMagnitudeTables<'a>,
) -> Result<AacMagnitudeSection<'a>, Error> {
    if quantized.iter().all(|&coeff| coeff == 0) {
        return Ok(AacMagnitudeSection {
            start,
            end,
            codebook_id: AacCodebook::Zero.id(),
            table: &[],
        });
    }

    let section = AacSection {
        start: 0,
        end: quantized.len(),
        codebook: AacCodebook::SignedPairs1,
    };
    let pairs = spectral_pairs_for_section(quantized, &section)?;
    let candidates = [
        (AacCodebook::SignedPairs1.id(), tables.pairs1),
        (AacCodebook::SignedPairs5.id(), tables.pairs5),
        (AacCodebook::SignedPairs6.id(), tables.pairs6),
        (7, aac_unsigned_pairs7_table()),
        (8, aac_unsigned_pairs8_table()),
        (9, aac_unsigned_pairs9_table()),
        (10, aac_unsigned_pairs10_table()),
        (AacCodebook::Escape.id(), tables.escape),
    ];
    let mut best: Option<(u8, &'a [HuffmanEntry<AacSpectralMagnitudePair>], usize)> = None;
    for (codebook_id, table) in candidates {
        if table.is_empty() {
            continue;
        }
        let Ok(packed) = pack_spectral_pairs_with_sign_bits(&pairs, table) else {
            continue;
        };
        if best
            .as_ref()
            .is_none_or(|(_, _, bit_len)| packed.bit_len < *bit_len)
        {
            best = Some((codebook_id, table, packed.bit_len));
        }
    }

    best.map(|(codebook_id, table, _)| AacMagnitudeSection {
        start,
        end,
        codebook_id,
        table,
    })
    .ok_or(Error::UnsupportedFeature("AAC spectral codebook"))
}

/// Groups quantized AAC coefficients into sections using available-table bit costs.
pub fn plan_sections_by_bit_cost(
    quantized: &[i32],
    band_width: usize,
    tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<AacSection>, Error> {
    if band_width == 0 {
        return Err(Error::InvalidInput(
            "AAC section band width must be non-zero",
        ));
    }
    if quantized.len() % band_width != 0 {
        return Err(Error::InvalidInput(
            "AAC section band width must divide spectrum length",
        ));
    }

    let mut sections = Vec::<AacSection>::new();
    for (band_index, band) in quantized.chunks(band_width).enumerate() {
        let codebook = select_codebook_by_bit_cost(band, tables)?;
        let start = band_index * band_width;
        let end = start + band_width;
        match sections.last_mut() {
            Some(section) if section.codebook == codebook => section.end = end,
            _ => sections.push(AacSection {
                start,
                end,
                codebook,
            }),
        }
    }
    Ok(sections)
}

pub fn plan_sections_by_offsets(
    quantized: &[i32],
    offsets: &[usize],
    tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<AacSection>, Error> {
    validate_scale_factor_band_offsets(quantized, offsets)?;

    let mut sections = Vec::<AacSection>::new();
    for band in offsets.windows(2) {
        let start = band[0];
        let end = band[1];
        let codebook = select_codebook_by_bit_cost(&quantized[start..end], tables)?;
        match sections.last_mut() {
            Some(section) if section.codebook == codebook => section.end = end,
            _ => sections.push(AacSection {
                start,
                end,
                codebook,
            }),
        }
    }
    Ok(sections)
}

fn plan_magnitude_sections_by_offsets<'a>(
    quantized: &[i32],
    offsets: &[usize],
    tables: AacSpectralMagnitudeTables<'a>,
) -> Result<Vec<AacMagnitudeSection<'a>>, Error> {
    validate_scale_factor_band_offsets(quantized, offsets)?;

    let mut sections = Vec::<AacMagnitudeSection<'a>>::new();
    for band in offsets.windows(2) {
        let start = band[0];
        let end = band[1];
        let planned =
            select_magnitude_section_by_bit_cost(start, end, &quantized[start..end], tables)?;
        match sections.last_mut() {
            Some(section) if section.codebook_id == planned.codebook_id => section.end = end,
            _ => sections.push(planned),
        }
    }
    Ok(sections)
}

/// Computes scale-factor DPCM deltas for non-zero AAC sections.
pub fn plan_scale_factor_deltas(
    sections: &[AacSection],
    band_width: usize,
    scale_factors: &[i16],
    initial_scale_factor: i16,
) -> Result<Vec<AacScaleFactorDelta>, Error> {
    if band_width == 0 {
        return Err(Error::InvalidInput(
            "AAC section band width must be non-zero",
        ));
    }

    let mut previous = initial_scale_factor;
    let mut deltas = Vec::new();
    for section in sections {
        if section.end <= section.start
            || section.start % band_width != 0
            || section.end % band_width != 0
        {
            return Err(Error::InvalidInput("invalid AAC section range"));
        }
        if section.codebook == AacCodebook::Zero {
            continue;
        }

        let start_band = section.start / band_width;
        let end_band = section.end / band_width;
        if end_band > scale_factors.len() {
            return Err(Error::InvalidInput("missing AAC scale factor"));
        }
        for &scale_factor in &scale_factors[start_band..end_band] {
            let delta = scale_factor
                .checked_sub(previous)
                .ok_or(Error::InvalidInput("AAC scale-factor delta overflows"))?;
            deltas.push(AacScaleFactorDelta::new(delta));
            previous = scale_factor;
        }
    }
    Ok(deltas)
}

pub fn plan_scale_factor_deltas_by_offsets(
    sections: &[AacSection],
    offsets: &[usize],
    scale_factors: &[i16],
    initial_scale_factor: i16,
) -> Result<Vec<AacScaleFactorDelta>, Error> {
    if offsets.len() < 2 {
        return Err(Error::InvalidInput("AAC scale-factor offsets are empty"));
    }
    if scale_factors.len() + 1 != offsets.len() {
        return Err(Error::InvalidInput("missing AAC scale factor"));
    }

    let mut previous = initial_scale_factor;
    let mut deltas = Vec::new();
    for section in sections {
        if section.end <= section.start {
            return Err(Error::InvalidInput("invalid AAC section range"));
        }
        if section.codebook == AacCodebook::Zero {
            continue;
        }

        let start_band = offset_band_index(offsets, section.start)?;
        let end_band = offset_band_index(offsets, section.end)?;
        if end_band > scale_factors.len() {
            return Err(Error::InvalidInput("missing AAC scale factor"));
        }
        for &scale_factor in &scale_factors[start_band..end_band] {
            let delta = scale_factor
                .checked_sub(previous)
                .ok_or(Error::InvalidInput("AAC scale-factor delta overflows"))?;
            deltas.push(AacScaleFactorDelta::new(delta));
            previous = scale_factor;
        }
    }
    Ok(deltas)
}

fn plan_magnitude_scale_factor_deltas_by_offsets(
    sections: &[AacMagnitudeSection<'_>],
    offsets: &[usize],
    scale_factors: &[i16],
    initial_scale_factor: i16,
) -> Result<Vec<AacScaleFactorDelta>, Error> {
    if offsets.len() < 2 {
        return Err(Error::InvalidInput("AAC scale-factor offsets are empty"));
    }
    if scale_factors.len() + 1 != offsets.len() {
        return Err(Error::InvalidInput("missing AAC scale factor"));
    }

    let mut previous = initial_scale_factor;
    let mut deltas = Vec::new();
    for section in sections {
        if section.end <= section.start {
            return Err(Error::InvalidInput("invalid AAC section range"));
        }
        if section.is_zero() {
            continue;
        }

        let start_band = offset_band_index(offsets, section.start)?;
        let end_band = offset_band_index(offsets, section.end)?;
        if end_band > scale_factors.len() {
            return Err(Error::InvalidInput("missing AAC scale factor"));
        }
        for &scale_factor in &scale_factors[start_band..end_band] {
            let delta = scale_factor
                .checked_sub(previous)
                .ok_or(Error::InvalidInput("AAC scale-factor delta overflows"))?;
            deltas.push(AacScaleFactorDelta::new(delta));
            previous = scale_factor;
        }
    }
    Ok(deltas)
}

/// Selects a deterministic per-band scale-factor seed from quantized magnitudes.
pub fn select_scale_factors_for_quantized_bands(
    quantized: &[i32],
    band_width: usize,
    base_scale_factor: i16,
) -> Result<Vec<i16>, Error> {
    if band_width == 0 {
        return Err(Error::InvalidInput(
            "AAC section band width must be non-zero",
        ));
    }
    if quantized.len() % band_width != 0 {
        return Err(Error::InvalidInput(
            "AAC section band width must divide spectrum length",
        ));
    }

    quantized
        .chunks(band_width)
        .map(|band| {
            let max_abs = band
                .iter()
                .map(|coeff| coeff.checked_abs())
                .collect::<Option<Vec<_>>>()
                .ok_or(Error::InvalidInput("AAC spectral coefficient overflows"))?
                .into_iter()
                .max()
                .unwrap_or(0);
            let magnitude_class = if max_abs == 0 {
                0
            } else {
                i16::try_from(32 - max_abs.leading_zeros()).map_err(|_| {
                    Error::InvalidInput("AAC scale-factor magnitude class overflows")
                })?
            };
            base_scale_factor
                .checked_add(magnitude_class)
                .ok_or(Error::InvalidInput("AAC scale factor overflows"))
        })
        .collect()
}

pub fn select_scale_factors_for_quantized_bands_by_offsets(
    quantized: &[i32],
    offsets: &[usize],
    base_scale_factor: i16,
) -> Result<Vec<i16>, Error> {
    validate_scale_factor_band_offsets(quantized, offsets)?;

    offsets
        .windows(2)
        .map(|band| {
            let max_abs = quantized[band[0]..band[1]]
                .iter()
                .map(|coeff| coeff.checked_abs())
                .collect::<Option<Vec<_>>>()
                .ok_or(Error::InvalidInput("AAC spectral coefficient overflows"))?
                .into_iter()
                .max()
                .unwrap_or(0);
            let magnitude_class = if max_abs == 0 {
                0
            } else {
                i16::try_from(32 - max_abs.leading_zeros()).map_err(|_| {
                    Error::InvalidInput("AAC scale-factor magnitude class overflows")
                })?
            };
            base_scale_factor
                .checked_add(magnitude_class)
                .ok_or(Error::InvalidInput("AAC scale factor overflows"))
        })
        .collect()
}

/// Packs AAC section codebook and length metadata.
pub fn pack_section_data(sections: &[AacSection], band_width: usize) -> Result<Vec<u8>, Error> {
    Ok(pack_section_data_with_len(sections, band_width)?.bytes)
}

/// Packs AAC section codebook and length metadata while preserving bit length.
pub fn pack_section_data_with_len(
    sections: &[AacSection],
    band_width: usize,
) -> Result<PackedBits, Error> {
    if band_width == 0 {
        return Err(Error::InvalidInput(
            "AAC section band width must be non-zero",
        ));
    }

    let mut writer = CoreBitWriter::new();
    for section in sections {
        if section.end <= section.start
            || section.start % band_width != 0
            || section.end % band_width != 0
        {
            return Err(Error::InvalidInput("invalid AAC section range"));
        }

        writer.write_bits(u32::from(section.codebook.id()), 4)?;
        let mut band_count = (section.end - section.start) / band_width;
        if band_count == 0 {
            return Err(Error::InvalidInput("invalid AAC section length"));
        }
        while band_count >= 31 {
            writer.write_bits(31, 5)?;
            band_count -= 31;
        }
        writer.write_bits(
            u32::try_from(band_count)
                .map_err(|_| Error::InvalidInput("AAC section is too long"))?,
            5,
        )?;
    }
    let bit_len = writer.bit_len();
    Ok(PackedBits {
        bytes: writer.finish_byte_aligned(),
        bit_len,
    })
}

/// Packs AAC quad-codebook section metadata with caller-supplied codebook ids.
pub fn pack_quad_section_data_with_len(
    sections: &[AacQuadSection],
    band_width: usize,
) -> Result<PackedBits, Error> {
    if band_width == 0 {
        return Err(Error::InvalidInput(
            "AAC section band width must be non-zero",
        ));
    }

    let mut writer = CoreBitWriter::new();
    for section in sections {
        if !(1..=4).contains(&section.codebook_id) {
            return Err(Error::InvalidInput("AAC quad codebook id must be 1..=4"));
        }
        if section.end <= section.start
            || section.start % band_width != 0
            || section.end % band_width != 0
        {
            return Err(Error::InvalidInput("invalid AAC section range"));
        }

        writer.write_bits(u32::from(section.codebook_id), 4)?;
        let mut band_count = (section.end - section.start) / band_width;
        if band_count == 0 {
            return Err(Error::InvalidInput("invalid AAC section length"));
        }
        while band_count >= 31 {
            writer.write_bits(31, 5)?;
            band_count -= 31;
        }
        writer.write_bits(
            u32::try_from(band_count)
                .map_err(|_| Error::InvalidInput("AAC section is too long"))?,
            5,
        )?;
    }
    let bit_len = writer.bit_len();
    Ok(PackedBits {
        bytes: writer.finish_byte_aligned(),
        bit_len,
    })
}

pub fn pack_section_data_with_offsets(
    sections: &[AacSection],
    offsets: &[usize],
) -> Result<PackedBits, Error> {
    if offsets.len() < 2 {
        return Err(Error::InvalidInput("AAC scale-factor offsets are empty"));
    }

    let mut writer = CoreBitWriter::new();
    for section in sections {
        if section.end <= section.start {
            return Err(Error::InvalidInput("invalid AAC section range"));
        }
        let start_band = offset_band_index(offsets, section.start)?;
        let end_band = offset_band_index(offsets, section.end)?;

        writer.write_bits(u32::from(section.codebook.id()), 4)?;
        let mut band_count = end_band - start_band;
        if band_count == 0 {
            return Err(Error::InvalidInput("invalid AAC section length"));
        }
        while band_count >= 31 {
            writer.write_bits(31, 5)?;
            band_count -= 31;
        }
        writer.write_bits(
            u32::try_from(band_count)
                .map_err(|_| Error::InvalidInput("AAC section is too long"))?,
            5,
        )?;
    }
    let bit_len = writer.bit_len();
    Ok(PackedBits {
        bytes: writer.finish_byte_aligned(),
        bit_len,
    })
}

fn pack_magnitude_section_data_with_offsets(
    sections: &[AacMagnitudeSection<'_>],
    offsets: &[usize],
) -> Result<PackedBits, Error> {
    if offsets.len() < 2 {
        return Err(Error::InvalidInput("AAC scale-factor offsets are empty"));
    }

    let mut writer = CoreBitWriter::new();
    for section in sections {
        if section.end <= section.start {
            return Err(Error::InvalidInput("invalid AAC section range"));
        }
        let start_band = offset_band_index(offsets, section.start)?;
        let end_band = offset_band_index(offsets, section.end)?;

        writer.write_bits(u32::from(section.codebook_id), 4)?;
        let mut band_count = end_band - start_band;
        if band_count == 0 {
            return Err(Error::InvalidInput("invalid AAC section length"));
        }
        while band_count >= 31 {
            writer.write_bits(31, 5)?;
            band_count -= 31;
        }
        writer.write_bits(
            u32::try_from(band_count)
                .map_err(|_| Error::InvalidInput("AAC section is too long"))?,
            5,
        )?;
    }
    let bit_len = writer.bit_len();
    Ok(PackedBits {
        bytes: writer.finish_byte_aligned(),
        bit_len,
    })
}

/// Packs preselected AAC spectral Huffman codewords.
pub fn pack_spectral_codewords(codes: &[HuffmanCode]) -> Result<Vec<u8>, Error> {
    pack_huffman_codes(codes)
}

/// Packs preselected AAC spectral Huffman codewords and preserves bit length.
pub fn pack_spectral_codewords_with_len(codes: &[HuffmanCode]) -> Result<PackedBits, Error> {
    pack_huffman_codes_with_len(codes)
}

/// Packs scale-factor DPCM deltas with a caller-supplied Huffman table.
pub fn pack_scale_factor_deltas_with_table(
    deltas: &[AacScaleFactorDelta],
    table: &[HuffmanEntry<AacScaleFactorDelta>],
) -> Result<PackedBits, Error> {
    pack_huffman_symbols_with_len(deltas, table)
}

/// Packs AAC spectral pairs using a caller-supplied codebook table.
pub fn pack_spectral_pairs_with_table(
    pairs: &[AacSpectralPair],
    table: &[HuffmanEntry<AacSpectralPair>],
) -> Result<PackedBits, Error> {
    pack_huffman_symbols_with_len(pairs, table)
}

/// Packs AAC spectral quadruples using a caller-supplied codebook table.
pub fn pack_spectral_quads_with_table(
    quads: &[AacSpectralQuad],
    table: &[HuffmanEntry<AacSpectralQuad>],
) -> Result<PackedBits, Error> {
    pack_huffman_symbols_with_len(quads, table)
}

/// Packs AAC spectral pairs with magnitude-keyed codewords followed by sign bits.
pub fn pack_spectral_pairs_with_sign_bits(
    pairs: &[AacSpectralPair],
    table: &[HuffmanEntry<AacSpectralMagnitudePair>],
) -> Result<PackedBits, Error> {
    let mut writer = CoreBitWriter::new();
    for pair in pairs {
        let magnitude = aac_spectral_pair_magnitude(*pair)?;
        let table_magnitude = AacSpectralMagnitudePair::new(
            magnitude.x.min(AAC_ESCAPE_MAGNITUDE),
            magnitude.y.min(AAC_ESCAPE_MAGNITUDE),
        );
        let code = sc_core::lookup_huffman_code(table, &table_magnitude)?;
        writer.write_bits(code.bits, code.len)?;
        write_aac_sign_bit(&mut writer, pair.x)?;
        write_aac_sign_bit(&mut writer, pair.y)?;
        write_aac_escape_suffix(&mut writer, magnitude.x)?;
        write_aac_escape_suffix(&mut writer, magnitude.y)?;
    }
    let bit_len = writer.bit_len();
    Ok(PackedBits {
        bytes: writer.finish_byte_aligned(),
        bit_len,
    })
}

/// Packs AAC spectral quadruples with magnitude-keyed codewords followed by sign bits.
pub fn pack_spectral_quads_with_sign_bits(
    quads: &[AacSpectralQuad],
    table: &[HuffmanEntry<AacSpectralMagnitudeQuad>],
) -> Result<PackedBits, Error> {
    let mut writer = CoreBitWriter::new();
    for quad in quads {
        let magnitude = aac_spectral_quad_magnitude(*quad)?;
        let code = sc_core::lookup_huffman_code(table, &magnitude)?;
        writer.write_bits(code.bits, code.len)?;
        write_aac_sign_bit(&mut writer, quad.v)?;
        write_aac_sign_bit(&mut writer, quad.w)?;
        write_aac_sign_bit(&mut writer, quad.x)?;
        write_aac_sign_bit(&mut writer, quad.y)?;
    }
    let bit_len = writer.bit_len();
    Ok(PackedBits {
        bytes: writer.finish_byte_aligned(),
        bit_len,
    })
}

/// Packs all non-zero AAC spectral sections with caller-supplied codebook tables.
pub fn pack_spectral_sections(
    sections: &[AacSection],
    quantized: &[i32],
    tables: AacSpectralTables<'_>,
) -> Result<PackedBits, Error> {
    let mut parts = Vec::new();
    for section in sections {
        let pairs = spectral_pairs_for_section(quantized, section)?;
        if pairs.is_empty() {
            continue;
        }
        parts.push(pack_spectral_pairs_with_table(
            &pairs,
            tables.table_for(section.codebook)?,
        )?);
    }
    concat_packed_bits(&parts)
}

/// Packs all non-zero AAC spectral sections using magnitude tables and sign bits.
pub fn pack_spectral_sections_with_sign_bits(
    sections: &[AacSection],
    quantized: &[i32],
    tables: AacSpectralMagnitudeTables<'_>,
) -> Result<PackedBits, Error> {
    let mut parts = Vec::new();
    for section in sections {
        let pairs = spectral_pairs_for_section(quantized, section)?;
        if pairs.is_empty() {
            continue;
        }
        parts.push(pack_spectral_pairs_with_sign_bits(
            &pairs,
            tables.table_for(section.codebook)?,
        )?);
    }
    concat_packed_bits(&parts)
}

/// Packs all non-zero AAC quad sections using magnitude tables and sign bits.
pub fn pack_spectral_quad_sections_with_sign_bits(
    sections: &[AacQuadSection],
    quantized: &[i32],
    tables: AacSpectralMagnitudeQuadTables<'_>,
) -> Result<PackedBits, Error> {
    let mut parts = Vec::new();
    for section in sections {
        if section.end <= section.start || section.end > quantized.len() {
            return Err(Error::InvalidInput("invalid AAC section range"));
        }
        let public_section = AacSection {
            start: section.start,
            end: section.end,
            codebook: AacCodebook::SignedPairs1,
        };
        let quads = spectral_quads_for_section(quantized, &public_section)?;
        if quads.is_empty() {
            continue;
        }
        parts.push(pack_spectral_quads_with_sign_bits(
            &quads,
            tables.table_for_codebook_id(section.codebook_id)?,
        )?);
    }
    concat_packed_bits(&parts)
}

fn pack_magnitude_spectral_sections_with_sign_bits(
    sections: &[AacMagnitudeSection<'_>],
    quantized: &[i32],
) -> Result<PackedBits, Error> {
    let mut parts = Vec::new();
    for section in sections {
        if section.end <= section.start || section.end > quantized.len() {
            return Err(Error::InvalidInput("invalid AAC section range"));
        }
        if section.is_zero() {
            continue;
        }
        let public_section = AacSection {
            start: section.start,
            end: section.end,
            codebook: AacCodebook::SignedPairs1,
        };
        let pairs = spectral_pairs_for_section(quantized, &public_section)?;
        parts.push(pack_spectral_pairs_with_sign_bits(&pairs, section.table)?);
    }
    concat_packed_bits(&parts)
}

/// Packs AAC section metadata followed by the matching section spectral payloads.
pub fn pack_sectioned_spectral_payload(
    sections: &[AacSection],
    quantized: &[i32],
    band_width: usize,
    tables: AacSpectralTables<'_>,
) -> Result<PackedBits, Error> {
    let section_bits = pack_section_data_with_len(sections, band_width)?;
    let spectral_bits = pack_spectral_sections(sections, quantized, tables)?;
    concat_packed_bits(&[section_bits, spectral_bits])
}

/// Packs AAC section, scale-factor, and spectral payload bits in ICS order.
pub fn pack_channel_payload_parts(
    section_bits: PackedBits,
    scale_factor_bits: PackedBits,
    spectral_bits: PackedBits,
) -> Result<PackedBits, Error> {
    concat_packed_bits(&[section_bits, scale_factor_bits, spectral_bits])
}

/// Splits AAC payload bits at the point where ICS pulse/TNS/gain flags must be inserted.
pub fn split_channel_payload_parts(
    section_bits: PackedBits,
    scale_factor_bits: PackedBits,
    spectral_bits: PackedBits,
) -> Result<AacIndividualChannelPayload, Error> {
    let section_and_scale_factor_bits = concat_packed_bits(&[section_bits, scale_factor_bits])?;
    Ok(AacIndividualChannelPayload::new(
        section_and_scale_factor_bits,
        spectral_bits,
    ))
}

/// Packs AAC section metadata, scale-factor bits, and signed-pair spectral payloads.
pub fn pack_sectioned_spectral_payload_with_scale_factor_bits(
    sections: &[AacSection],
    quantized: &[i32],
    band_width: usize,
    scale_factor_bits: PackedBits,
    tables: AacSpectralTables<'_>,
) -> Result<PackedBits, Error> {
    let section_bits = pack_section_data_with_len(sections, band_width)?;
    let spectral_bits = pack_spectral_sections(sections, quantized, tables)?;
    pack_channel_payload_parts(section_bits, scale_factor_bits, spectral_bits)
}

/// Builds separated signed-pair payload parts for a long-block individual_channel_stream.
pub fn split_sectioned_spectral_payload_with_scale_factor_bits(
    sections: &[AacSection],
    quantized: &[i32],
    band_width: usize,
    scale_factor_bits: PackedBits,
    tables: AacSpectralTables<'_>,
) -> Result<AacIndividualChannelPayload, Error> {
    let section_bits = pack_section_data_with_len(sections, band_width)?;
    let spectral_bits = pack_spectral_sections(sections, quantized, tables)?;
    split_channel_payload_parts(section_bits, scale_factor_bits, spectral_bits)
}

/// Packs AAC section metadata followed by magnitude-keyed spectral payloads.
pub fn pack_sectioned_spectral_payload_with_sign_bits(
    sections: &[AacSection],
    quantized: &[i32],
    band_width: usize,
    tables: AacSpectralMagnitudeTables<'_>,
) -> Result<PackedBits, Error> {
    let section_bits = pack_section_data_with_len(sections, band_width)?;
    let spectral_bits = pack_spectral_sections_with_sign_bits(sections, quantized, tables)?;
    concat_packed_bits(&[section_bits, spectral_bits])
}

/// Packs AAC quad section metadata followed by magnitude-keyed quad spectral payloads.
pub fn pack_sectioned_spectral_quad_payload_with_sign_bits(
    sections: &[AacQuadSection],
    quantized: &[i32],
    band_width: usize,
    tables: AacSpectralMagnitudeQuadTables<'_>,
) -> Result<PackedBits, Error> {
    let section_bits = pack_quad_section_data_with_len(sections, band_width)?;
    let spectral_bits = pack_spectral_quad_sections_with_sign_bits(sections, quantized, tables)?;
    concat_packed_bits(&[section_bits, spectral_bits])
}

/// Builds separated magnitude-keyed payload parts for a long-block individual_channel_stream.
pub fn split_sectioned_spectral_payload_with_sign_bits(
    sections: &[AacSection],
    quantized: &[i32],
    band_width: usize,
    tables: AacSpectralMagnitudeTables<'_>,
) -> Result<AacIndividualChannelPayload, Error> {
    let section_bits = pack_section_data_with_len(sections, band_width)?;
    let spectral_bits = pack_spectral_sections_with_sign_bits(sections, quantized, tables)?;
    split_channel_payload_parts(
        section_bits,
        PackedBits {
            bytes: Vec::new(),
            bit_len: 0,
        },
        spectral_bits,
    )
}

/// Plans AAC sections by bit cost, then packs metadata followed by spectral payloads.
pub fn pack_sectioned_spectral_payload_with_sign_bits_by_bit_cost(
    quantized: &[i32],
    band_width: usize,
    tables: AacSpectralMagnitudeTables<'_>,
) -> Result<PackedBits, Error> {
    let sections = plan_sections_by_bit_cost(quantized, band_width, tables)?;
    pack_sectioned_spectral_payload_with_sign_bits(&sections, quantized, band_width, tables)
}

/// Packs AAC section metadata, scale-factor bits, and magnitude-keyed spectral payloads.
pub fn pack_sectioned_spectral_payload_with_sign_bits_and_scale_factor_bits(
    sections: &[AacSection],
    quantized: &[i32],
    band_width: usize,
    scale_factor_bits: PackedBits,
    tables: AacSpectralMagnitudeTables<'_>,
) -> Result<PackedBits, Error> {
    let section_bits = pack_section_data_with_len(sections, band_width)?;
    let spectral_bits = pack_spectral_sections_with_sign_bits(sections, quantized, tables)?;
    pack_channel_payload_parts(section_bits, scale_factor_bits, spectral_bits)
}

/// Builds separated magnitude-keyed payload parts with scale-factor bits.
pub fn split_sectioned_spectral_payload_with_sign_bits_and_scale_factor_bits(
    sections: &[AacSection],
    quantized: &[i32],
    band_width: usize,
    scale_factor_bits: PackedBits,
    tables: AacSpectralMagnitudeTables<'_>,
) -> Result<AacIndividualChannelPayload, Error> {
    let section_bits = pack_section_data_with_len(sections, band_width)?;
    let spectral_bits = pack_spectral_sections_with_sign_bits(sections, quantized, tables)?;
    split_channel_payload_parts(section_bits, scale_factor_bits, spectral_bits)
}

pub fn split_sectioned_spectral_payload_with_offsets_and_scale_factor_bits(
    sections: &[AacSection],
    quantized: &[i32],
    offsets: &[usize],
    scale_factor_bits: PackedBits,
    tables: AacSpectralMagnitudeTables<'_>,
) -> Result<AacIndividualChannelPayload, Error> {
    validate_scale_factor_band_offsets(quantized, offsets)?;
    let section_bits = pack_section_data_with_offsets(sections, offsets)?;
    let spectral_bits = pack_spectral_sections_with_sign_bits(sections, quantized, tables)?;
    split_channel_payload_parts(section_bits, scale_factor_bits, spectral_bits)
}

fn split_magnitude_sectioned_spectral_payload_with_offsets_and_scale_factor_bits(
    sections: &[AacMagnitudeSection<'_>],
    quantized: &[i32],
    offsets: &[usize],
    scale_factor_bits: PackedBits,
) -> Result<AacIndividualChannelPayload, Error> {
    validate_scale_factor_band_offsets(quantized, offsets)?;
    let section_bits = pack_magnitude_section_data_with_offsets(sections, offsets)?;
    let spectral_bits = pack_magnitude_spectral_sections_with_sign_bits(sections, quantized)?;
    split_channel_payload_parts(section_bits, scale_factor_bits, spectral_bits)
}

/// Plans AAC sections by bit cost, then packs metadata, scale-factor bits, and spectral payloads.
pub fn pack_sectioned_spectral_payload_with_sign_bits_and_scale_factor_bits_by_bit_cost(
    quantized: &[i32],
    band_width: usize,
    scale_factor_bits: PackedBits,
    tables: AacSpectralMagnitudeTables<'_>,
) -> Result<PackedBits, Error> {
    let sections = plan_sections_by_bit_cost(quantized, band_width, tables)?;
    pack_sectioned_spectral_payload_with_sign_bits_and_scale_factor_bits(
        &sections,
        quantized,
        band_width,
        scale_factor_bits,
        tables,
    )
}

/// Packs an AAC-LC long-block individual_channel_stream with caller-built payload bits.
pub fn pack_long_block_individual_channel_stream(
    config: AacLongBlockConfig,
    sectioned_spectral_payload: &PackedBits,
) -> Result<PackedBits, Error> {
    pack_long_block_individual_channel_stream_parts(
        config,
        &AacIndividualChannelPayload::new(
            sectioned_spectral_payload.clone(),
            PackedBits {
                bytes: Vec::new(),
                bit_len: 0,
            },
        ),
    )
}

/// Packs an AAC-LC long-block individual_channel_stream with separated pre-spectral and spectral bits.
pub fn pack_long_block_individual_channel_stream_parts(
    config: AacLongBlockConfig,
    payload: &AacIndividualChannelPayload,
) -> Result<PackedBits, Error> {
    if config.max_sfb > 63 {
        return Err(Error::InvalidInput("AAC max_sfb exceeds 6-bit range"));
    }
    if config.max_sfb == 0
        && (payload.section_and_scale_factor_bits.bit_len != 0
            || payload.spectral_bits.bit_len != 0)
    {
        return Err(Error::InvalidInput(
            "AAC zero max_sfb cannot carry spectral payload",
        ));
    }

    let mut writer = CoreBitWriter::new();
    writer.write_bits(u32::from(config.global_gain), 8)?;
    writer.write_bits(0, 1)?; // ics_reserved_bit
    writer.write_bits(0, 2)?; // ONLY_LONG_SEQUENCE
    writer.write_bits(0, 1)?; // window_shape
    writer.write_bits(u32::from(config.max_sfb), 6)?;
    writer.write_bits(0, 1)?; // predictor_data_present
    write_packed_bits(&mut writer, &payload.section_and_scale_factor_bits)?;
    writer.write_bits(0, 1)?; // pulse_data_present
    writer.write_bits(0, 1)?; // tns_data_present
    writer.write_bits(0, 1)?; // gain_control_data_present
    write_packed_bits(&mut writer, &payload.spectral_bits)?;
    let bit_len = writer.bit_len();
    Ok(PackedBits {
        bytes: writer.finish_byte_aligned(),
        bit_len,
    })
}

/// Packs one single_channel_element raw_data_block with a long-block ICS.
pub fn pack_single_channel_raw_data_block(
    config: AacLongBlockConfig,
    sectioned_spectral_payload: &PackedBits,
) -> Result<Vec<u8>, Error> {
    pack_single_channel_raw_data_block_parts(
        config,
        &AacIndividualChannelPayload::new(
            sectioned_spectral_payload.clone(),
            PackedBits {
                bytes: Vec::new(),
                bit_len: 0,
            },
        ),
    )
}

/// Packs one single_channel_element raw_data_block with separated long-block ICS payload parts.
pub fn pack_single_channel_raw_data_block_parts(
    config: AacLongBlockConfig,
    payload: &AacIndividualChannelPayload,
) -> Result<Vec<u8>, Error> {
    let ics = pack_long_block_individual_channel_stream_parts(config, payload)?;
    let mut writer = CoreBitWriter::new();
    writer.write_bits(0, 3)?; // ID_SCE
    writer.write_bits(0, 4)?; // element_instance_tag
    write_packed_bits(&mut writer, &ics)?;
    writer.write_bits(7, 3)?; // ID_END
    Ok(writer.finish_byte_aligned())
}

/// Packs one channel_pair_element raw_data_block with two independent long-block ICS payloads.
pub fn pack_channel_pair_raw_data_block(
    left_config: AacLongBlockConfig,
    left_sectioned_spectral_payload: &PackedBits,
    right_config: AacLongBlockConfig,
    right_sectioned_spectral_payload: &PackedBits,
) -> Result<Vec<u8>, Error> {
    pack_channel_pair_raw_data_block_parts(
        left_config,
        &AacIndividualChannelPayload::new(
            left_sectioned_spectral_payload.clone(),
            PackedBits {
                bytes: Vec::new(),
                bit_len: 0,
            },
        ),
        right_config,
        &AacIndividualChannelPayload::new(
            right_sectioned_spectral_payload.clone(),
            PackedBits {
                bytes: Vec::new(),
                bit_len: 0,
            },
        ),
    )
}

/// Packs one channel_pair_element raw_data_block with separated long-block ICS payload parts.
pub fn pack_channel_pair_raw_data_block_parts(
    left_config: AacLongBlockConfig,
    left_payload: &AacIndividualChannelPayload,
    right_config: AacLongBlockConfig,
    right_payload: &AacIndividualChannelPayload,
) -> Result<Vec<u8>, Error> {
    let left = pack_long_block_individual_channel_stream_parts(left_config, left_payload)?;
    let right = pack_long_block_individual_channel_stream_parts(right_config, right_payload)?;

    let mut writer = CoreBitWriter::new();
    writer.write_bits(1, 3)?; // ID_CPE
    writer.write_bits(0, 4)?; // element_instance_tag
    writer.write_bits(0, 1)?; // common_window
    write_packed_bits(&mut writer, &left)?;
    write_packed_bits(&mut writer, &right)?;
    writer.write_bits(7, 3)?; // ID_END
    Ok(writer.finish_byte_aligned())
}

/// Encodes one mono AAC-LC ADTS frame from a pre-quantized long-block spectrum.
pub fn encode_quantized_mono_adts(
    adts: AdtsConfig,
    channel: AacLongBlockConfig,
    quantized: &[i32],
    band_width: usize,
    tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<u8>, Error> {
    if adts.channels != 1 {
        return Err(Error::InvalidInput(
            "AAC mono ADTS config must have one channel",
        ));
    }
    let sections = plan_sections(quantized, band_width)?;
    let payload =
        split_sectioned_spectral_payload_with_sign_bits(&sections, quantized, band_width, tables)?;
    let access_unit = pack_single_channel_raw_data_block_parts(channel, &payload)?;
    frame_adts(adts, &access_unit)
}

/// Encodes one mono AAC-LC ADTS frame using bit-cost section planning.
pub fn encode_quantized_mono_adts_by_bit_cost(
    adts: AdtsConfig,
    channel: AacLongBlockConfig,
    quantized: &[i32],
    band_width: usize,
    tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<u8>, Error> {
    if adts.channels != 1 {
        return Err(Error::InvalidInput(
            "AAC mono ADTS config must have one channel",
        ));
    }
    let sections = plan_sections_by_bit_cost(quantized, band_width, tables)?;
    let payload =
        split_sectioned_spectral_payload_with_sign_bits(&sections, quantized, band_width, tables)?;
    let access_unit = pack_single_channel_raw_data_block_parts(channel, &payload)?;
    frame_adts(adts, &access_unit)
}

/// Encodes one mono AAC-LC ADTS frame with scale-factor DPCM payload.
pub fn encode_quantized_mono_adts_with_scale_factors(
    adts: AdtsConfig,
    channel: AacLongBlockConfig,
    quantized: &[i32],
    band_width: usize,
    scale_factors: &[i16],
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<u8>, Error> {
    if adts.channels != 1 {
        return Err(Error::InvalidInput(
            "AAC mono ADTS config must have one channel",
        ));
    }
    let sections = plan_sections(quantized, band_width)?;
    let scale_factor_deltas = plan_scale_factor_deltas(
        &sections,
        band_width,
        scale_factors,
        i16::from(channel.global_gain),
    )?;
    let scale_factor_bits =
        pack_scale_factor_deltas_with_table(&scale_factor_deltas, scale_factor_table)?;
    let payload = split_sectioned_spectral_payload_with_sign_bits_and_scale_factor_bits(
        &sections,
        quantized,
        band_width,
        scale_factor_bits,
        spectral_tables,
    )?;
    let access_unit = pack_single_channel_raw_data_block_parts(channel, &payload)?;
    frame_adts(adts, &access_unit)
}

/// Encodes one mono AAC-LC ADTS frame with scale-factor DPCM and bit-cost section planning.
pub fn encode_quantized_mono_adts_with_scale_factors_by_bit_cost(
    adts: AdtsConfig,
    channel: AacLongBlockConfig,
    quantized: &[i32],
    band_width: usize,
    scale_factors: &[i16],
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<u8>, Error> {
    if adts.channels != 1 {
        return Err(Error::InvalidInput(
            "AAC mono ADTS config must have one channel",
        ));
    }
    let sections = plan_sections_by_bit_cost(quantized, band_width, spectral_tables)?;
    let scale_factor_deltas = plan_scale_factor_deltas(
        &sections,
        band_width,
        scale_factors,
        i16::from(channel.global_gain),
    )?;
    let scale_factor_bits =
        pack_scale_factor_deltas_with_table(&scale_factor_deltas, scale_factor_table)?;
    let payload = split_sectioned_spectral_payload_with_sign_bits_and_scale_factor_bits(
        &sections,
        quantized,
        band_width,
        scale_factor_bits,
        spectral_tables,
    )?;
    let access_unit = pack_single_channel_raw_data_block_parts(channel, &payload)?;
    frame_adts(adts, &access_unit)
}

/// Encodes one mono AAC-LC ADTS frame with internally selected scale factors.
pub fn encode_quantized_mono_adts_with_selected_scale_factors(
    adts: AdtsConfig,
    channel: AacLongBlockConfig,
    quantized: &[i32],
    band_width: usize,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<u8>, Error> {
    let scale_factors = select_scale_factors_for_quantized_bands(
        quantized,
        band_width,
        i16::from(channel.global_gain),
    )?;
    encode_quantized_mono_adts_with_scale_factors(
        adts,
        channel,
        quantized,
        band_width,
        &scale_factors,
        scale_factor_table,
        spectral_tables,
    )
}

/// Encodes one mono AAC-LC ADTS frame with selected scale factors and bit-cost sections.
pub fn encode_quantized_mono_adts_with_selected_scale_factors_by_bit_cost(
    adts: AdtsConfig,
    channel: AacLongBlockConfig,
    quantized: &[i32],
    band_width: usize,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<u8>, Error> {
    let scale_factors = select_scale_factors_for_quantized_bands(
        quantized,
        band_width,
        i16::from(channel.global_gain),
    )?;
    encode_quantized_mono_adts_with_scale_factors_by_bit_cost(
        adts,
        channel,
        quantized,
        band_width,
        &scale_factors,
        scale_factor_table,
        spectral_tables,
    )
}

pub fn encode_quantized_mono_adts_with_offsets_and_selected_scale_factors_by_bit_cost(
    adts: AdtsConfig,
    channel: AacLongBlockConfig,
    quantized: &[i32],
    offsets: &[usize],
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<u8>, Error> {
    if adts.channels != 1 {
        return Err(Error::InvalidInput(
            "AAC mono ADTS config must have one channel",
        ));
    }
    validate_scale_factor_band_offsets(quantized, offsets)?;
    let max_sfb = offsets.len() - 1;
    if usize::from(channel.max_sfb) != max_sfb {
        return Err(Error::InvalidInput(
            "AAC max_sfb must match scale-factor band count",
        ));
    }

    let sections = plan_magnitude_sections_by_offsets(quantized, offsets, spectral_tables)?;
    let scale_factors = select_scale_factors_for_quantized_bands_by_offsets(
        quantized,
        offsets,
        i16::from(channel.global_gain),
    )?;
    let scale_factor_deltas = plan_magnitude_scale_factor_deltas_by_offsets(
        &sections,
        offsets,
        &scale_factors,
        i16::from(channel.global_gain),
    )?;
    let scale_factor_bits =
        pack_scale_factor_deltas_with_table(&scale_factor_deltas, scale_factor_table)?;
    let payload = split_magnitude_sectioned_spectral_payload_with_offsets_and_scale_factor_bits(
        &sections,
        quantized,
        offsets,
        scale_factor_bits,
    )?;
    let access_unit = pack_single_channel_raw_data_block_parts(channel, &payload)?;
    frame_adts(adts, &access_unit)
}

pub fn encode_quantized_mono_adts_with_offsets_and_scale_factors_by_bit_cost(
    adts: AdtsConfig,
    channel: AacLongBlockConfig,
    quantized: &[i32],
    offsets: &[usize],
    scale_factors: &[i16],
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<u8>, Error> {
    if adts.channels != 1 {
        return Err(Error::InvalidInput(
            "AAC mono ADTS config must have one channel",
        ));
    }
    validate_scale_factor_band_offsets(quantized, offsets)?;
    let max_sfb = offsets.len() - 1;
    if usize::from(channel.max_sfb) != max_sfb {
        return Err(Error::InvalidInput(
            "AAC max_sfb must match scale-factor band count",
        ));
    }

    let sections = plan_magnitude_sections_by_offsets(quantized, offsets, spectral_tables)?;
    let scale_factor_deltas = plan_magnitude_scale_factor_deltas_by_offsets(
        &sections,
        offsets,
        scale_factors,
        i16::from(channel.global_gain),
    )?;
    let scale_factor_bits =
        pack_scale_factor_deltas_with_table(&scale_factor_deltas, scale_factor_table)?;
    let payload = split_magnitude_sectioned_spectral_payload_with_offsets_and_scale_factor_bits(
        &sections,
        quantized,
        offsets,
        scale_factor_bits,
    )?;
    let access_unit = pack_single_channel_raw_data_block_parts(channel, &payload)?;
    frame_adts(adts, &access_unit)
}

pub fn encode_quantized_stereo_adts_with_offsets_and_scale_factors_by_bit_cost(
    adts: AdtsConfig,
    left: AacQuantizedChannel<'_>,
    right: AacQuantizedChannel<'_>,
    offsets: &[usize],
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<u8>, Error> {
    if adts.channels != 2 {
        return Err(Error::InvalidInput(
            "AAC stereo ADTS config must have two channels",
        ));
    }
    validate_scale_factor_band_offsets(left.quantized, offsets)?;
    validate_scale_factor_band_offsets(right.quantized, offsets)?;
    let max_sfb = offsets.len() - 1;
    if usize::from(left.config.max_sfb) != max_sfb || usize::from(right.config.max_sfb) != max_sfb {
        return Err(Error::InvalidInput(
            "AAC max_sfb must match scale-factor band count",
        ));
    }

    let left_sections =
        plan_magnitude_sections_by_offsets(left.quantized, offsets, spectral_tables)?;
    let right_sections =
        plan_magnitude_sections_by_offsets(right.quantized, offsets, spectral_tables)?;
    let left_scale_factor_deltas = plan_magnitude_scale_factor_deltas_by_offsets(
        &left_sections,
        offsets,
        left.scale_factors,
        i16::from(left.config.global_gain),
    )?;
    let right_scale_factor_deltas = plan_magnitude_scale_factor_deltas_by_offsets(
        &right_sections,
        offsets,
        right.scale_factors,
        i16::from(right.config.global_gain),
    )?;
    let left_scale_factor_bits =
        pack_scale_factor_deltas_with_table(&left_scale_factor_deltas, scale_factor_table)?;
    let right_scale_factor_bits =
        pack_scale_factor_deltas_with_table(&right_scale_factor_deltas, scale_factor_table)?;
    let left_payload =
        split_magnitude_sectioned_spectral_payload_with_offsets_and_scale_factor_bits(
            &left_sections,
            left.quantized,
            offsets,
            left_scale_factor_bits,
        )?;
    let right_payload =
        split_magnitude_sectioned_spectral_payload_with_offsets_and_scale_factor_bits(
            &right_sections,
            right.quantized,
            offsets,
            right_scale_factor_bits,
        )?;
    let access_unit = pack_channel_pair_raw_data_block_parts(
        left.config,
        &left_payload,
        right.config,
        &right_payload,
    )?;
    frame_adts(adts, &access_unit)
}

/// Encodes one independent-stereo AAC-LC ADTS frame from pre-quantized long-block spectra.
pub fn encode_quantized_stereo_adts(
    adts: AdtsConfig,
    left: AacLongBlockConfig,
    left_quantized: &[i32],
    right: AacLongBlockConfig,
    right_quantized: &[i32],
    band_width: usize,
    tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<u8>, Error> {
    if adts.channels != 2 {
        return Err(Error::InvalidInput(
            "AAC stereo ADTS config must have two channels",
        ));
    }
    let left_sections = plan_sections(left_quantized, band_width)?;
    let right_sections = plan_sections(right_quantized, band_width)?;
    let left_payload = split_sectioned_spectral_payload_with_sign_bits(
        &left_sections,
        left_quantized,
        band_width,
        tables,
    )?;
    let right_payload = split_sectioned_spectral_payload_with_sign_bits(
        &right_sections,
        right_quantized,
        band_width,
        tables,
    )?;
    let access_unit =
        pack_channel_pair_raw_data_block_parts(left, &left_payload, right, &right_payload)?;
    frame_adts(adts, &access_unit)
}

/// Encodes one independent-stereo AAC-LC ADTS frame using bit-cost section planning.
pub fn encode_quantized_stereo_adts_by_bit_cost(
    adts: AdtsConfig,
    left: AacLongBlockConfig,
    left_quantized: &[i32],
    right: AacLongBlockConfig,
    right_quantized: &[i32],
    band_width: usize,
    tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<u8>, Error> {
    if adts.channels != 2 {
        return Err(Error::InvalidInput(
            "AAC stereo ADTS config must have two channels",
        ));
    }
    let left_sections = plan_sections_by_bit_cost(left_quantized, band_width, tables)?;
    let right_sections = plan_sections_by_bit_cost(right_quantized, band_width, tables)?;
    let left_payload = split_sectioned_spectral_payload_with_sign_bits(
        &left_sections,
        left_quantized,
        band_width,
        tables,
    )?;
    let right_payload = split_sectioned_spectral_payload_with_sign_bits(
        &right_sections,
        right_quantized,
        band_width,
        tables,
    )?;
    let access_unit =
        pack_channel_pair_raw_data_block_parts(left, &left_payload, right, &right_payload)?;
    frame_adts(adts, &access_unit)
}

/// Encodes one independent-stereo AAC-LC ADTS frame with scale-factor DPCM payloads.
pub fn encode_quantized_stereo_adts_with_scale_factors(
    adts: AdtsConfig,
    left: AacQuantizedChannel<'_>,
    right: AacQuantizedChannel<'_>,
    band_width: usize,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<u8>, Error> {
    if adts.channels != 2 {
        return Err(Error::InvalidInput(
            "AAC stereo ADTS config must have two channels",
        ));
    }
    let left_sections = plan_sections(left.quantized, band_width)?;
    let right_sections = plan_sections(right.quantized, band_width)?;
    let left_scale_factor_deltas = plan_scale_factor_deltas(
        &left_sections,
        band_width,
        left.scale_factors,
        i16::from(left.config.global_gain),
    )?;
    let right_scale_factor_deltas = plan_scale_factor_deltas(
        &right_sections,
        band_width,
        right.scale_factors,
        i16::from(right.config.global_gain),
    )?;
    let left_scale_factor_bits =
        pack_scale_factor_deltas_with_table(&left_scale_factor_deltas, scale_factor_table)?;
    let right_scale_factor_bits =
        pack_scale_factor_deltas_with_table(&right_scale_factor_deltas, scale_factor_table)?;
    let left_payload = split_sectioned_spectral_payload_with_sign_bits_and_scale_factor_bits(
        &left_sections,
        left.quantized,
        band_width,
        left_scale_factor_bits,
        spectral_tables,
    )?;
    let right_payload = split_sectioned_spectral_payload_with_sign_bits_and_scale_factor_bits(
        &right_sections,
        right.quantized,
        band_width,
        right_scale_factor_bits,
        spectral_tables,
    )?;
    let access_unit = pack_channel_pair_raw_data_block_parts(
        left.config,
        &left_payload,
        right.config,
        &right_payload,
    )?;
    frame_adts(adts, &access_unit)
}

/// Encodes one independent-stereo AAC-LC ADTS frame with scale factors and bit-cost sections.
pub fn encode_quantized_stereo_adts_with_scale_factors_by_bit_cost(
    adts: AdtsConfig,
    left: AacQuantizedChannel<'_>,
    right: AacQuantizedChannel<'_>,
    band_width: usize,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<u8>, Error> {
    if adts.channels != 2 {
        return Err(Error::InvalidInput(
            "AAC stereo ADTS config must have two channels",
        ));
    }
    let left_sections = plan_sections_by_bit_cost(left.quantized, band_width, spectral_tables)?;
    let right_sections = plan_sections_by_bit_cost(right.quantized, band_width, spectral_tables)?;
    let left_scale_factor_deltas = plan_scale_factor_deltas(
        &left_sections,
        band_width,
        left.scale_factors,
        i16::from(left.config.global_gain),
    )?;
    let right_scale_factor_deltas = plan_scale_factor_deltas(
        &right_sections,
        band_width,
        right.scale_factors,
        i16::from(right.config.global_gain),
    )?;
    let left_scale_factor_bits =
        pack_scale_factor_deltas_with_table(&left_scale_factor_deltas, scale_factor_table)?;
    let right_scale_factor_bits =
        pack_scale_factor_deltas_with_table(&right_scale_factor_deltas, scale_factor_table)?;
    let left_payload = split_sectioned_spectral_payload_with_sign_bits_and_scale_factor_bits(
        &left_sections,
        left.quantized,
        band_width,
        left_scale_factor_bits,
        spectral_tables,
    )?;
    let right_payload = split_sectioned_spectral_payload_with_sign_bits_and_scale_factor_bits(
        &right_sections,
        right.quantized,
        band_width,
        right_scale_factor_bits,
        spectral_tables,
    )?;
    let access_unit = pack_channel_pair_raw_data_block_parts(
        left.config,
        &left_payload,
        right.config,
        &right_payload,
    )?;
    frame_adts(adts, &access_unit)
}

/// Encodes one independent-stereo AAC-LC ADTS frame with internally selected scale factors.
pub fn encode_quantized_stereo_adts_with_selected_scale_factors(
    adts: AdtsConfig,
    left: AacQuantizedSpectrum<'_>,
    right: AacQuantizedSpectrum<'_>,
    band_width: usize,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<u8>, Error> {
    let left_scale_factors = select_scale_factors_for_quantized_bands(
        left.quantized,
        band_width,
        i16::from(left.config.global_gain),
    )?;
    let right_scale_factors = select_scale_factors_for_quantized_bands(
        right.quantized,
        band_width,
        i16::from(right.config.global_gain),
    )?;
    encode_quantized_stereo_adts_with_scale_factors(
        adts,
        AacQuantizedChannel::new(left.config, left.quantized, &left_scale_factors),
        AacQuantizedChannel::new(right.config, right.quantized, &right_scale_factors),
        band_width,
        scale_factor_table,
        spectral_tables,
    )
}

/// Encodes one independent-stereo AAC-LC ADTS frame with selected scale factors and bit-cost sections.
pub fn encode_quantized_stereo_adts_with_selected_scale_factors_by_bit_cost(
    adts: AdtsConfig,
    left: AacQuantizedSpectrum<'_>,
    right: AacQuantizedSpectrum<'_>,
    band_width: usize,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<u8>, Error> {
    let left_scale_factors = select_scale_factors_for_quantized_bands(
        left.quantized,
        band_width,
        i16::from(left.config.global_gain),
    )?;
    let right_scale_factors = select_scale_factors_for_quantized_bands(
        right.quantized,
        band_width,
        i16::from(right.config.global_gain),
    )?;
    encode_quantized_stereo_adts_with_scale_factors_by_bit_cost(
        adts,
        AacQuantizedChannel::new(left.config, left.quantized, &left_scale_factors),
        AacQuantizedChannel::new(right.config, right.quantized, &right_scale_factors),
        band_width,
        scale_factor_table,
        spectral_tables,
    )
}

pub fn encode_quantized_stereo_adts_with_offsets_and_selected_scale_factors_by_bit_cost(
    adts: AdtsConfig,
    left: AacQuantizedSpectrum<'_>,
    right: AacQuantizedSpectrum<'_>,
    offsets: &[usize],
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<u8>, Error> {
    let left_scale_factors = select_scale_factors_for_quantized_bands_by_offsets(
        left.quantized,
        offsets,
        i16::from(left.config.global_gain),
    )?;
    let right_scale_factors = select_scale_factors_for_quantized_bands_by_offsets(
        right.quantized,
        offsets,
        i16::from(right.config.global_gain),
    )?;
    encode_quantized_stereo_adts_with_offsets_and_scale_factors_by_bit_cost(
        adts,
        AacQuantizedChannel::new(left.config, left.quantized, &left_scale_factors),
        AacQuantizedChannel::new(right.config, right.quantized, &right_scale_factors),
        offsets,
        scale_factor_table,
        spectral_tables,
    )
}

/// Encodes one mono AAC-LC ADTS frame from a PCM long analysis block.
pub fn encode_pcm_mono_long_block_adts(
    adts: AdtsConfig,
    channel: AacLongBlockConfig,
    pcm: &AudioBuffer,
    pcm_config: AacPcmLongBlockConfig,
    tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<u8>, Error> {
    if adts.channels != 1 || pcm.channels != 1 {
        return Err(Error::InvalidInput(
            "AAC mono PCM encode requires one-channel ADTS and PCM",
        ));
    }
    let quantized = quantize_pcm_long_block(pcm, 0, pcm_config.start_frame, pcm_config.step)?;
    encode_quantized_mono_adts(adts, channel, &quantized, pcm_config.band_width, tables)
}

/// Encodes one mono AAC-LC ADTS frame from PCM using bit-cost section planning.
pub fn encode_pcm_mono_long_block_adts_by_bit_cost(
    adts: AdtsConfig,
    channel: AacLongBlockConfig,
    pcm: &AudioBuffer,
    pcm_config: AacPcmLongBlockConfig,
    tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<u8>, Error> {
    if adts.channels != 1 || pcm.channels != 1 {
        return Err(Error::InvalidInput(
            "AAC mono PCM encode requires one-channel ADTS and PCM",
        ));
    }
    let quantized = quantize_pcm_long_block(pcm, 0, pcm_config.start_frame, pcm_config.step)?;
    encode_quantized_mono_adts_by_bit_cost(adts, channel, &quantized, pcm_config.band_width, tables)
}

/// Encodes one mono AAC-LC ADTS frame from PCM with scale-factor DPCM payload.
pub fn encode_pcm_mono_long_block_adts_with_scale_factors(
    adts: AdtsConfig,
    channel: AacScaleFactorChannel<'_>,
    pcm: &AudioBuffer,
    pcm_config: AacPcmLongBlockConfig,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<u8>, Error> {
    if adts.channels != 1 || pcm.channels != 1 {
        return Err(Error::InvalidInput(
            "AAC mono PCM encode requires one-channel ADTS and PCM",
        ));
    }
    let quantized = quantize_pcm_long_block(pcm, 0, pcm_config.start_frame, pcm_config.step)?;
    encode_quantized_mono_adts_with_scale_factors(
        adts,
        channel.config,
        &quantized,
        pcm_config.band_width,
        channel.scale_factors,
        scale_factor_table,
        spectral_tables,
    )
}

/// Encodes one mono AAC-LC ADTS frame from PCM with scale factors and bit-cost sections.
pub fn encode_pcm_mono_long_block_adts_with_scale_factors_by_bit_cost(
    adts: AdtsConfig,
    channel: AacScaleFactorChannel<'_>,
    pcm: &AudioBuffer,
    pcm_config: AacPcmLongBlockConfig,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<u8>, Error> {
    if adts.channels != 1 || pcm.channels != 1 {
        return Err(Error::InvalidInput(
            "AAC mono PCM encode requires one-channel ADTS and PCM",
        ));
    }
    let quantized = quantize_pcm_long_block(pcm, 0, pcm_config.start_frame, pcm_config.step)?;
    encode_quantized_mono_adts_with_scale_factors_by_bit_cost(
        adts,
        channel.config,
        &quantized,
        pcm_config.band_width,
        channel.scale_factors,
        scale_factor_table,
        spectral_tables,
    )
}

/// Encodes one mono AAC-LC ADTS frame from PCM with internally selected scale factors.
pub fn encode_pcm_mono_long_block_adts_with_selected_scale_factors(
    adts: AdtsConfig,
    channel: AacLongBlockConfig,
    pcm: &AudioBuffer,
    pcm_config: AacPcmLongBlockConfig,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<u8>, Error> {
    if adts.channels != 1 || pcm.channels != 1 {
        return Err(Error::InvalidInput(
            "AAC mono PCM encode requires one-channel ADTS and PCM",
        ));
    }
    let quantized = quantize_pcm_long_block(pcm, 0, pcm_config.start_frame, pcm_config.step)?;
    encode_quantized_mono_adts_with_selected_scale_factors(
        adts,
        channel,
        &quantized,
        pcm_config.band_width,
        scale_factor_table,
        spectral_tables,
    )
}

/// Encodes one mono AAC-LC ADTS frame from PCM with selected scale factors and bit-cost sections.
pub fn encode_pcm_mono_long_block_adts_with_selected_scale_factors_by_bit_cost(
    adts: AdtsConfig,
    channel: AacLongBlockConfig,
    pcm: &AudioBuffer,
    pcm_config: AacPcmLongBlockConfig,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<u8>, Error> {
    if adts.channels != 1 || pcm.channels != 1 {
        return Err(Error::InvalidInput(
            "AAC mono PCM encode requires one-channel ADTS and PCM",
        ));
    }
    let quantized = quantize_pcm_long_block(pcm, 0, pcm_config.start_frame, pcm_config.step)?;
    encode_quantized_mono_adts_with_selected_scale_factors_by_bit_cost(
        adts,
        channel,
        &quantized,
        pcm_config.band_width,
        scale_factor_table,
        spectral_tables,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn encode_pcm_mono_long_block_adts_with_offsets_and_selected_scale_factors_by_bit_cost(
    adts: AdtsConfig,
    channel: AacLongBlockConfig,
    pcm: &AudioBuffer,
    start_frame: usize,
    step: f32,
    offsets: &[usize],
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<u8>, Error> {
    if adts.channels != 1 || pcm.channels != 1 {
        return Err(Error::InvalidInput(
            "AAC mono PCM encode requires one-channel ADTS and PCM",
        ));
    }
    let quantized = quantize_pcm_long_block(pcm, 0, start_frame, step)?;
    encode_quantized_mono_adts_with_offsets_and_selected_scale_factors_by_bit_cost(
        adts,
        channel,
        &quantized,
        offsets,
        scale_factor_table,
        spectral_tables,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn encode_pcm_mono_long_block_adts_with_offsets_and_scale_factors_by_bit_cost(
    adts: AdtsConfig,
    channel: AacScaleFactorChannel<'_>,
    pcm: &AudioBuffer,
    start_frame: usize,
    step: f32,
    offsets: &[usize],
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<u8>, Error> {
    if adts.channels != 1 || pcm.channels != 1 {
        return Err(Error::InvalidInput(
            "AAC mono PCM encode requires one-channel ADTS and PCM",
        ));
    }
    let quantized = quantize_pcm_long_block(pcm, 0, start_frame, step)?;
    encode_quantized_mono_adts_with_offsets_and_scale_factors_by_bit_cost(
        adts,
        channel.config,
        &quantized,
        offsets,
        channel.scale_factors,
        scale_factor_table,
        spectral_tables,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn encode_pcm_stereo_long_block_adts_with_offsets_and_scale_factors_by_bit_cost(
    adts: AdtsConfig,
    left: AacScaleFactorChannel<'_>,
    right: AacScaleFactorChannel<'_>,
    pcm: &AudioBuffer,
    start_frame: usize,
    step: f32,
    offsets: &[usize],
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<u8>, Error> {
    if adts.channels != 2 || pcm.channels != 2 {
        return Err(Error::InvalidInput(
            "AAC stereo PCM encode requires two-channel ADTS and PCM",
        ));
    }
    let left_quantized = quantize_pcm_long_block(pcm, 0, start_frame, step)?;
    let right_quantized = quantize_pcm_long_block(pcm, 1, start_frame, step)?;
    encode_quantized_stereo_adts_with_offsets_and_scale_factors_by_bit_cost(
        adts,
        AacQuantizedChannel::new(left.config, &left_quantized, left.scale_factors),
        AacQuantizedChannel::new(right.config, &right_quantized, right.scale_factors),
        offsets,
        scale_factor_table,
        spectral_tables,
    )
}

/// Encodes one independent-stereo AAC-LC ADTS frame from PCM long analysis blocks.
pub fn encode_pcm_stereo_long_block_adts(
    adts: AdtsConfig,
    left: AacLongBlockConfig,
    right: AacLongBlockConfig,
    pcm: &AudioBuffer,
    pcm_config: AacPcmLongBlockConfig,
    tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<u8>, Error> {
    if adts.channels != 2 || pcm.channels != 2 {
        return Err(Error::InvalidInput(
            "AAC stereo PCM encode requires two-channel ADTS and PCM",
        ));
    }
    let left_quantized = quantize_pcm_long_block(pcm, 0, pcm_config.start_frame, pcm_config.step)?;
    let right_quantized = quantize_pcm_long_block(pcm, 1, pcm_config.start_frame, pcm_config.step)?;
    encode_quantized_stereo_adts(
        adts,
        left,
        &left_quantized,
        right,
        &right_quantized,
        pcm_config.band_width,
        tables,
    )
}

/// Encodes one independent-stereo AAC-LC ADTS frame from PCM using bit-cost section planning.
pub fn encode_pcm_stereo_long_block_adts_by_bit_cost(
    adts: AdtsConfig,
    left: AacLongBlockConfig,
    right: AacLongBlockConfig,
    pcm: &AudioBuffer,
    pcm_config: AacPcmLongBlockConfig,
    tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<u8>, Error> {
    if adts.channels != 2 || pcm.channels != 2 {
        return Err(Error::InvalidInput(
            "AAC stereo PCM encode requires two-channel ADTS and PCM",
        ));
    }
    let left_quantized = quantize_pcm_long_block(pcm, 0, pcm_config.start_frame, pcm_config.step)?;
    let right_quantized = quantize_pcm_long_block(pcm, 1, pcm_config.start_frame, pcm_config.step)?;
    encode_quantized_stereo_adts_by_bit_cost(
        adts,
        left,
        &left_quantized,
        right,
        &right_quantized,
        pcm_config.band_width,
        tables,
    )
}

/// Encodes one independent-stereo AAC-LC ADTS frame from PCM with scale-factor DPCM payloads.
pub fn encode_pcm_stereo_long_block_adts_with_scale_factors(
    adts: AdtsConfig,
    left: AacScaleFactorChannel<'_>,
    right: AacScaleFactorChannel<'_>,
    pcm: &AudioBuffer,
    pcm_config: AacPcmLongBlockConfig,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<u8>, Error> {
    if adts.channels != 2 || pcm.channels != 2 {
        return Err(Error::InvalidInput(
            "AAC stereo PCM encode requires two-channel ADTS and PCM",
        ));
    }
    let left_quantized = quantize_pcm_long_block(pcm, 0, pcm_config.start_frame, pcm_config.step)?;
    let right_quantized = quantize_pcm_long_block(pcm, 1, pcm_config.start_frame, pcm_config.step)?;
    encode_quantized_stereo_adts_with_scale_factors(
        adts,
        AacQuantizedChannel::new(left.config, &left_quantized, left.scale_factors),
        AacQuantizedChannel::new(right.config, &right_quantized, right.scale_factors),
        pcm_config.band_width,
        scale_factor_table,
        spectral_tables,
    )
}

/// Encodes one independent-stereo AAC-LC ADTS frame from PCM with scale factors and bit-cost sections.
pub fn encode_pcm_stereo_long_block_adts_with_scale_factors_by_bit_cost(
    adts: AdtsConfig,
    left: AacScaleFactorChannel<'_>,
    right: AacScaleFactorChannel<'_>,
    pcm: &AudioBuffer,
    pcm_config: AacPcmLongBlockConfig,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<u8>, Error> {
    if adts.channels != 2 || pcm.channels != 2 {
        return Err(Error::InvalidInput(
            "AAC stereo PCM encode requires two-channel ADTS and PCM",
        ));
    }
    let left_quantized = quantize_pcm_long_block(pcm, 0, pcm_config.start_frame, pcm_config.step)?;
    let right_quantized = quantize_pcm_long_block(pcm, 1, pcm_config.start_frame, pcm_config.step)?;
    encode_quantized_stereo_adts_with_scale_factors_by_bit_cost(
        adts,
        AacQuantizedChannel::new(left.config, &left_quantized, left.scale_factors),
        AacQuantizedChannel::new(right.config, &right_quantized, right.scale_factors),
        pcm_config.band_width,
        scale_factor_table,
        spectral_tables,
    )
}

/// Encodes one independent-stereo AAC-LC ADTS frame from PCM with internally selected scale factors.
pub fn encode_pcm_stereo_long_block_adts_with_selected_scale_factors(
    adts: AdtsConfig,
    left: AacLongBlockConfig,
    right: AacLongBlockConfig,
    pcm: &AudioBuffer,
    pcm_config: AacPcmLongBlockConfig,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<u8>, Error> {
    if adts.channels != 2 || pcm.channels != 2 {
        return Err(Error::InvalidInput(
            "AAC stereo PCM encode requires two-channel ADTS and PCM",
        ));
    }
    let left_quantized = quantize_pcm_long_block(pcm, 0, pcm_config.start_frame, pcm_config.step)?;
    let right_quantized = quantize_pcm_long_block(pcm, 1, pcm_config.start_frame, pcm_config.step)?;
    encode_quantized_stereo_adts_with_selected_scale_factors(
        adts,
        AacQuantizedSpectrum::new(left, &left_quantized),
        AacQuantizedSpectrum::new(right, &right_quantized),
        pcm_config.band_width,
        scale_factor_table,
        spectral_tables,
    )
}

/// Encodes one independent-stereo AAC-LC ADTS frame from PCM with selected scale factors and bit-cost sections.
pub fn encode_pcm_stereo_long_block_adts_with_selected_scale_factors_by_bit_cost(
    adts: AdtsConfig,
    left: AacLongBlockConfig,
    right: AacLongBlockConfig,
    pcm: &AudioBuffer,
    pcm_config: AacPcmLongBlockConfig,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<u8>, Error> {
    if adts.channels != 2 || pcm.channels != 2 {
        return Err(Error::InvalidInput(
            "AAC stereo PCM encode requires two-channel ADTS and PCM",
        ));
    }
    let left_quantized = quantize_pcm_long_block(pcm, 0, pcm_config.start_frame, pcm_config.step)?;
    let right_quantized = quantize_pcm_long_block(pcm, 1, pcm_config.start_frame, pcm_config.step)?;
    encode_quantized_stereo_adts_with_selected_scale_factors_by_bit_cost(
        adts,
        AacQuantizedSpectrum::new(left, &left_quantized),
        AacQuantizedSpectrum::new(right, &right_quantized),
        pcm_config.band_width,
        scale_factor_table,
        spectral_tables,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn encode_pcm_stereo_long_block_adts_with_offsets_and_selected_scale_factors_by_bit_cost(
    adts: AdtsConfig,
    left: AacLongBlockConfig,
    right: AacLongBlockConfig,
    pcm: &AudioBuffer,
    start_frame: usize,
    step: f32,
    offsets: &[usize],
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<u8>, Error> {
    if adts.channels != 2 || pcm.channels != 2 {
        return Err(Error::InvalidInput(
            "AAC stereo PCM encode requires two-channel ADTS and PCM",
        ));
    }
    let left_quantized = quantize_pcm_long_block(pcm, 0, start_frame, step)?;
    let right_quantized = quantize_pcm_long_block(pcm, 1, start_frame, step)?;
    encode_quantized_stereo_adts_with_offsets_and_selected_scale_factors_by_bit_cost(
        adts,
        AacQuantizedSpectrum::new(left, &left_quantized),
        AacQuantizedSpectrum::new(right, &right_quantized),
        offsets,
        scale_factor_table,
        spectral_tables,
    )
}

/// Encodes a mono AAC-LC ADTS stream from PCM using 1024-frame long-block hops.
pub fn encode_pcm_mono_long_block_adts_stream(
    adts: AdtsConfig,
    channel: AacLongBlockConfig,
    pcm: &AudioBuffer,
    pcm_config: AacPcmLongBlockConfig,
    tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<u8>, Error> {
    if adts.channels != 1 || pcm.channels != 1 {
        return Err(Error::InvalidInput(
            "AAC mono PCM encode requires one-channel ADTS and PCM",
        ));
    }

    let mut out = Vec::new();
    for start_frame in pcm_frame_starts(pcm, pcm_config.start_frame)? {
        out.extend_from_slice(&encode_pcm_mono_long_block_adts(
            adts,
            channel,
            pcm,
            AacPcmLongBlockConfig::new(start_frame, pcm_config.step, pcm_config.band_width),
            tables,
        )?);
    }
    Ok(out)
}

/// Encodes a mono AAC-LC ADTS stream from PCM using bit-cost section planning.
pub fn encode_pcm_mono_long_block_adts_stream_by_bit_cost(
    adts: AdtsConfig,
    channel: AacLongBlockConfig,
    pcm: &AudioBuffer,
    pcm_config: AacPcmLongBlockConfig,
    tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<u8>, Error> {
    if adts.channels != 1 || pcm.channels != 1 {
        return Err(Error::InvalidInput(
            "AAC mono PCM encode requires one-channel ADTS and PCM",
        ));
    }

    let mut out = Vec::new();
    for start_frame in pcm_frame_starts(pcm, pcm_config.start_frame)? {
        out.extend_from_slice(&encode_pcm_mono_long_block_adts_by_bit_cost(
            adts,
            channel,
            pcm,
            AacPcmLongBlockConfig::new(start_frame, pcm_config.step, pcm_config.band_width),
            tables,
        )?);
    }
    Ok(out)
}

/// Encodes a mono AAC-LC ADTS stream from PCM with per-frame scale-factor payloads.
pub fn encode_pcm_mono_long_block_adts_stream_with_scale_factors(
    adts: AdtsConfig,
    channel: AacScaleFactorSequence<'_>,
    pcm: &AudioBuffer,
    pcm_config: AacPcmLongBlockConfig,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<u8>, Error> {
    if adts.channels != 1 || pcm.channels != 1 {
        return Err(Error::InvalidInput(
            "AAC mono PCM encode requires one-channel ADTS and PCM",
        ));
    }

    let starts = pcm_frame_starts(pcm, pcm_config.start_frame)?;
    if starts.len() != channel.scale_factors_by_frame.len() {
        return Err(Error::InvalidInput(
            "AAC scale-factor frame count does not match PCM frame count",
        ));
    }

    let mut out = Vec::new();
    for (frame_index, start_frame) in starts.into_iter().enumerate() {
        out.extend_from_slice(&encode_pcm_mono_long_block_adts_with_scale_factors(
            adts,
            channel.channel_for_frame(frame_index)?,
            pcm,
            AacPcmLongBlockConfig::new(start_frame, pcm_config.step, pcm_config.band_width),
            scale_factor_table,
            spectral_tables,
        )?);
    }
    Ok(out)
}

/// Encodes a mono AAC-LC ADTS stream from PCM with per-frame scale factors and bit-cost sections.
pub fn encode_pcm_mono_long_block_adts_stream_with_scale_factors_by_bit_cost(
    adts: AdtsConfig,
    channel: AacScaleFactorSequence<'_>,
    pcm: &AudioBuffer,
    pcm_config: AacPcmLongBlockConfig,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<u8>, Error> {
    if adts.channels != 1 || pcm.channels != 1 {
        return Err(Error::InvalidInput(
            "AAC mono PCM encode requires one-channel ADTS and PCM",
        ));
    }

    let starts = pcm_frame_starts(pcm, pcm_config.start_frame)?;
    if starts.len() != channel.scale_factors_by_frame.len() {
        return Err(Error::InvalidInput(
            "AAC scale-factor frame count does not match PCM frame count",
        ));
    }

    let mut out = Vec::new();
    for (frame_index, start_frame) in starts.into_iter().enumerate() {
        out.extend_from_slice(
            &encode_pcm_mono_long_block_adts_with_scale_factors_by_bit_cost(
                adts,
                channel.channel_for_frame(frame_index)?,
                pcm,
                AacPcmLongBlockConfig::new(start_frame, pcm_config.step, pcm_config.band_width),
                scale_factor_table,
                spectral_tables,
            )?,
        );
    }
    Ok(out)
}

/// Encodes a mono AAC-LC ADTS stream from PCM with internally selected scale factors.
pub fn encode_pcm_mono_long_block_adts_stream_with_selected_scale_factors(
    adts: AdtsConfig,
    channel: AacLongBlockConfig,
    pcm: &AudioBuffer,
    pcm_config: AacPcmLongBlockConfig,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<u8>, Error> {
    if adts.channels != 1 || pcm.channels != 1 {
        return Err(Error::InvalidInput(
            "AAC mono PCM encode requires one-channel ADTS and PCM",
        ));
    }

    let mut out = Vec::new();
    for start_frame in pcm_frame_starts(pcm, pcm_config.start_frame)? {
        out.extend_from_slice(
            &encode_pcm_mono_long_block_adts_with_selected_scale_factors(
                adts,
                channel,
                pcm,
                AacPcmLongBlockConfig::new(start_frame, pcm_config.step, pcm_config.band_width),
                scale_factor_table,
                spectral_tables,
            )?,
        );
    }
    Ok(out)
}

/// Encodes a mono AAC-LC ADTS stream from PCM with selected scale factors and bit-cost sections.
pub fn encode_pcm_mono_long_block_adts_stream_with_selected_scale_factors_by_bit_cost(
    adts: AdtsConfig,
    channel: AacLongBlockConfig,
    pcm: &AudioBuffer,
    pcm_config: AacPcmLongBlockConfig,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<u8>, Error> {
    if adts.channels != 1 || pcm.channels != 1 {
        return Err(Error::InvalidInput(
            "AAC mono PCM encode requires one-channel ADTS and PCM",
        ));
    }

    let mut out = Vec::new();
    for start_frame in pcm_frame_starts(pcm, pcm_config.start_frame)? {
        out.extend_from_slice(
            &encode_pcm_mono_long_block_adts_with_selected_scale_factors_by_bit_cost(
                adts,
                channel,
                pcm,
                AacPcmLongBlockConfig::new(start_frame, pcm_config.step, pcm_config.band_width),
                scale_factor_table,
                spectral_tables,
            )?,
        );
    }
    Ok(out)
}

pub fn encode_pcm_mono_long_block_adts_stream_with_offsets_and_selected_scale_factors_by_bit_cost(
    adts: AdtsConfig,
    channel: AacLongBlockConfig,
    pcm: &AudioBuffer,
    offsets: &[usize],
    candidates: &[f32],
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<u8>, Error> {
    if adts.channels != 1 || pcm.channels != 1 {
        return Err(Error::InvalidInput(
            "AAC mono PCM encode requires one-channel ADTS and PCM",
        ));
    }

    let mut out = Vec::new();
    for start_frame in pcm_frame_starts(pcm, 0)? {
        let step = select_aac_lc_mono_pcm_frame_step_with_offsets_by_bit_cost(
            adts,
            channel,
            pcm,
            start_frame,
            offsets,
            candidates,
            scale_factor_table,
            spectral_tables,
        )?;
        out.extend_from_slice(
            &encode_pcm_mono_long_block_adts_with_offsets_and_selected_scale_factors_by_bit_cost(
                adts,
                channel,
                pcm,
                start_frame,
                step,
                offsets,
                scale_factor_table,
                spectral_tables,
            )?,
        );
    }
    Ok(out)
}

#[allow(clippy::too_many_arguments)]
pub fn encode_pcm_mono_long_block_adts_stream_with_offsets_and_selected_scale_factors_and_max_frame_len_by_bit_cost(
    adts: AdtsConfig,
    channel: AacLongBlockConfig,
    pcm: &AudioBuffer,
    offsets: &[usize],
    candidates: &[f32],
    max_frame_len_bytes: usize,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<u8>, Error> {
    if adts.channels != 1 || pcm.channels != 1 {
        return Err(Error::InvalidInput(
            "AAC mono PCM encode requires one-channel ADTS and PCM",
        ));
    }

    let mut out = Vec::new();
    for start_frame in pcm_frame_starts(pcm, 0)? {
        let step = select_aac_lc_mono_pcm_frame_step_with_offsets_and_max_frame_len_by_bit_cost(
            adts,
            channel,
            pcm,
            start_frame,
            offsets,
            candidates,
            max_frame_len_bytes,
            scale_factor_table,
            spectral_tables,
        )?;
        out.extend_from_slice(
            &encode_pcm_mono_long_block_adts_with_offsets_and_selected_scale_factors_by_bit_cost(
                adts,
                channel,
                pcm,
                start_frame,
                step,
                offsets,
                scale_factor_table,
                spectral_tables,
            )?,
        );
    }
    Ok(out)
}

#[allow(clippy::too_many_arguments)]
pub fn encode_pcm_mono_long_block_adts_stream_with_offsets_and_selected_scale_factors_and_bitrate_by_bit_cost(
    adts: AdtsConfig,
    channel: AacLongBlockConfig,
    pcm: &AudioBuffer,
    offsets: &[usize],
    candidates: &[f32],
    target_bitrate_bps: u32,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<u8>, Error> {
    let max_frame_len_bytes =
        aac_lc_adts_max_frame_len_for_bitrate(adts.sample_rate, target_bitrate_bps)?;
    encode_pcm_mono_long_block_adts_stream_with_offsets_and_selected_scale_factors_and_max_frame_len_by_bit_cost(
        adts,
        channel,
        pcm,
        offsets,
        candidates,
        max_frame_len_bytes,
        scale_factor_table,
        spectral_tables,
    )
}

pub fn encode_pcm_mono_long_block_adts_stream_with_offsets_and_scale_factors_by_bit_cost(
    adts: AdtsConfig,
    channel: AacScaleFactorChannel<'_>,
    pcm: &AudioBuffer,
    offsets: &[usize],
    candidates: &[f32],
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<u8>, Error> {
    if adts.channels != 1 || pcm.channels != 1 {
        return Err(Error::InvalidInput(
            "AAC mono PCM encode requires one-channel ADTS and PCM",
        ));
    }

    let mut out = Vec::new();
    for start_frame in pcm_frame_starts(pcm, 0)? {
        let step = select_aac_lc_mono_pcm_frame_step_with_offsets_and_scale_factors_by_bit_cost(
            adts,
            channel,
            pcm,
            start_frame,
            offsets,
            candidates,
            scale_factor_table,
            spectral_tables,
        )?;
        out.extend_from_slice(
            &encode_pcm_mono_long_block_adts_with_offsets_and_scale_factors_by_bit_cost(
                adts,
                channel,
                pcm,
                start_frame,
                step,
                offsets,
                scale_factor_table,
                spectral_tables,
            )?,
        );
    }
    Ok(out)
}

#[allow(clippy::too_many_arguments)]
pub fn encode_pcm_mono_long_block_adts_stream_with_offsets_and_scale_factors_and_max_frame_len_by_bit_cost(
    adts: AdtsConfig,
    channel: AacScaleFactorChannel<'_>,
    pcm: &AudioBuffer,
    offsets: &[usize],
    candidates: &[f32],
    max_frame_len_bytes: usize,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<u8>, Error> {
    if adts.channels != 1 || pcm.channels != 1 {
        return Err(Error::InvalidInput(
            "AAC mono PCM encode requires one-channel ADTS and PCM",
        ));
    }

    let mut out = Vec::new();
    for start_frame in pcm_frame_starts(pcm, 0)? {
        let step =
            select_aac_lc_mono_pcm_frame_step_with_offsets_and_scale_factors_and_max_frame_len_by_bit_cost(
                adts,
                channel,
                pcm,
                start_frame,
                offsets,
                candidates,
                max_frame_len_bytes,
                scale_factor_table,
                spectral_tables,
            )?;
        out.extend_from_slice(
            &encode_pcm_mono_long_block_adts_with_offsets_and_scale_factors_by_bit_cost(
                adts,
                channel,
                pcm,
                start_frame,
                step,
                offsets,
                scale_factor_table,
                spectral_tables,
            )?,
        );
    }
    Ok(out)
}

#[allow(clippy::too_many_arguments)]
pub fn encode_pcm_mono_long_block_adts_stream_with_offsets_and_scale_factors_and_bitrate_by_bit_cost(
    adts: AdtsConfig,
    channel: AacScaleFactorChannel<'_>,
    pcm: &AudioBuffer,
    offsets: &[usize],
    candidates: &[f32],
    target_bitrate_bps: u32,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<u8>, Error> {
    let max_frame_len_bytes =
        aac_lc_adts_max_frame_len_for_bitrate(adts.sample_rate, target_bitrate_bps)?;
    encode_pcm_mono_long_block_adts_stream_with_offsets_and_scale_factors_and_max_frame_len_by_bit_cost(
        adts,
        channel,
        pcm,
        offsets,
        candidates,
        max_frame_len_bytes,
        scale_factor_table,
        spectral_tables,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn encode_pcm_stereo_long_block_adts_stream_with_offsets_and_scale_factors_by_bit_cost(
    adts: AdtsConfig,
    left: AacScaleFactorChannel<'_>,
    right: AacScaleFactorChannel<'_>,
    pcm: &AudioBuffer,
    offsets: &[usize],
    candidates: &[f32],
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<u8>, Error> {
    if adts.channels != 2 || pcm.channels != 2 {
        return Err(Error::InvalidInput(
            "AAC stereo PCM encode requires two-channel ADTS and PCM",
        ));
    }

    let mut out = Vec::new();
    for start_frame in pcm_frame_starts(pcm, 0)? {
        let step = select_aac_lc_stereo_pcm_frame_step_with_offsets_and_scale_factors_by_bit_cost(
            adts,
            left,
            right,
            pcm,
            start_frame,
            offsets,
            candidates,
            scale_factor_table,
            spectral_tables,
        )?;
        out.extend_from_slice(
            &encode_pcm_stereo_long_block_adts_with_offsets_and_scale_factors_by_bit_cost(
                adts,
                left,
                right,
                pcm,
                start_frame,
                step,
                offsets,
                scale_factor_table,
                spectral_tables,
            )?,
        );
    }
    Ok(out)
}

#[allow(clippy::too_many_arguments)]
pub fn encode_pcm_stereo_long_block_adts_stream_with_offsets_and_selected_scale_factors_by_bit_cost(
    adts: AdtsConfig,
    left: AacLongBlockConfig,
    right: AacLongBlockConfig,
    pcm: &AudioBuffer,
    offsets: &[usize],
    candidates: &[f32],
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<u8>, Error> {
    if adts.channels != 2 || pcm.channels != 2 {
        return Err(Error::InvalidInput(
            "AAC stereo PCM encode requires two-channel ADTS and PCM",
        ));
    }

    let mut out = Vec::new();
    for start_frame in pcm_frame_starts(pcm, 0)? {
        let step = select_aac_lc_stereo_pcm_frame_step_with_offsets_by_bit_cost(
            adts,
            left,
            right,
            pcm,
            start_frame,
            offsets,
            candidates,
            scale_factor_table,
            spectral_tables,
        )?;
        out.extend_from_slice(
            &encode_pcm_stereo_long_block_adts_with_offsets_and_selected_scale_factors_by_bit_cost(
                adts,
                left,
                right,
                pcm,
                start_frame,
                step,
                offsets,
                scale_factor_table,
                spectral_tables,
            )?,
        );
    }
    Ok(out)
}

#[allow(clippy::too_many_arguments)]
pub fn encode_pcm_stereo_long_block_adts_stream_with_offsets_and_selected_scale_factors_and_max_frame_len_by_bit_cost(
    adts: AdtsConfig,
    left: AacLongBlockConfig,
    right: AacLongBlockConfig,
    pcm: &AudioBuffer,
    offsets: &[usize],
    candidates: &[f32],
    max_frame_len_bytes: usize,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<u8>, Error> {
    if adts.channels != 2 || pcm.channels != 2 {
        return Err(Error::InvalidInput(
            "AAC stereo PCM encode requires two-channel ADTS and PCM",
        ));
    }

    let mut out = Vec::new();
    for start_frame in pcm_frame_starts(pcm, 0)? {
        let step = select_aac_lc_stereo_pcm_frame_step_with_offsets_and_max_frame_len_by_bit_cost(
            adts,
            left,
            right,
            pcm,
            start_frame,
            offsets,
            candidates,
            max_frame_len_bytes,
            scale_factor_table,
            spectral_tables,
        )?;
        out.extend_from_slice(
            &encode_pcm_stereo_long_block_adts_with_offsets_and_selected_scale_factors_by_bit_cost(
                adts,
                left,
                right,
                pcm,
                start_frame,
                step,
                offsets,
                scale_factor_table,
                spectral_tables,
            )?,
        );
    }
    Ok(out)
}

#[allow(clippy::too_many_arguments)]
pub fn encode_pcm_stereo_long_block_adts_stream_with_offsets_and_selected_scale_factors_and_bitrate_by_bit_cost(
    adts: AdtsConfig,
    left: AacLongBlockConfig,
    right: AacLongBlockConfig,
    pcm: &AudioBuffer,
    offsets: &[usize],
    candidates: &[f32],
    target_bitrate_bps: u32,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<u8>, Error> {
    let max_frame_len_bytes =
        aac_lc_adts_max_frame_len_for_bitrate(adts.sample_rate, target_bitrate_bps)?;
    encode_pcm_stereo_long_block_adts_stream_with_offsets_and_selected_scale_factors_and_max_frame_len_by_bit_cost(
        adts,
        left,
        right,
        pcm,
        offsets,
        candidates,
        max_frame_len_bytes,
        scale_factor_table,
        spectral_tables,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn encode_pcm_stereo_long_block_adts_stream_with_offsets_and_scale_factors_and_max_frame_len_by_bit_cost(
    adts: AdtsConfig,
    left: AacScaleFactorChannel<'_>,
    right: AacScaleFactorChannel<'_>,
    pcm: &AudioBuffer,
    offsets: &[usize],
    candidates: &[f32],
    max_frame_len_bytes: usize,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<u8>, Error> {
    if adts.channels != 2 || pcm.channels != 2 {
        return Err(Error::InvalidInput(
            "AAC stereo PCM encode requires two-channel ADTS and PCM",
        ));
    }

    let mut out = Vec::new();
    for start_frame in pcm_frame_starts(pcm, 0)? {
        let step =
            select_aac_lc_stereo_pcm_frame_step_with_offsets_and_scale_factors_and_max_frame_len_by_bit_cost(
                adts,
                left,
                right,
                pcm,
                start_frame,
                offsets,
                candidates,
                max_frame_len_bytes,
                scale_factor_table,
                spectral_tables,
            )?;
        out.extend_from_slice(
            &encode_pcm_stereo_long_block_adts_with_offsets_and_scale_factors_by_bit_cost(
                adts,
                left,
                right,
                pcm,
                start_frame,
                step,
                offsets,
                scale_factor_table,
                spectral_tables,
            )?,
        );
    }
    Ok(out)
}

#[allow(clippy::too_many_arguments)]
pub fn encode_pcm_stereo_long_block_adts_stream_with_offsets_and_scale_factors_and_bitrate_by_bit_cost(
    adts: AdtsConfig,
    left: AacScaleFactorChannel<'_>,
    right: AacScaleFactorChannel<'_>,
    pcm: &AudioBuffer,
    offsets: &[usize],
    candidates: &[f32],
    target_bitrate_bps: u32,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<u8>, Error> {
    let max_frame_len_bytes =
        aac_lc_adts_max_frame_len_for_bitrate(adts.sample_rate, target_bitrate_bps)?;
    encode_pcm_stereo_long_block_adts_stream_with_offsets_and_scale_factors_and_max_frame_len_by_bit_cost(
        adts,
        left,
        right,
        pcm,
        offsets,
        candidates,
        max_frame_len_bytes,
        scale_factor_table,
        spectral_tables,
    )
}

/// Encodes an independent-stereo AAC-LC ADTS stream from PCM using 1024-frame hops.
pub fn encode_pcm_stereo_long_block_adts_stream(
    adts: AdtsConfig,
    left: AacLongBlockConfig,
    right: AacLongBlockConfig,
    pcm: &AudioBuffer,
    pcm_config: AacPcmLongBlockConfig,
    tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<u8>, Error> {
    if adts.channels != 2 || pcm.channels != 2 {
        return Err(Error::InvalidInput(
            "AAC stereo PCM encode requires two-channel ADTS and PCM",
        ));
    }

    let mut out = Vec::new();
    for start_frame in pcm_frame_starts(pcm, pcm_config.start_frame)? {
        out.extend_from_slice(&encode_pcm_stereo_long_block_adts(
            adts,
            left,
            right,
            pcm,
            AacPcmLongBlockConfig::new(start_frame, pcm_config.step, pcm_config.band_width),
            tables,
        )?);
    }
    Ok(out)
}

/// Encodes an independent-stereo AAC-LC ADTS stream from PCM using bit-cost section planning.
pub fn encode_pcm_stereo_long_block_adts_stream_by_bit_cost(
    adts: AdtsConfig,
    left: AacLongBlockConfig,
    right: AacLongBlockConfig,
    pcm: &AudioBuffer,
    pcm_config: AacPcmLongBlockConfig,
    tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<u8>, Error> {
    if adts.channels != 2 || pcm.channels != 2 {
        return Err(Error::InvalidInput(
            "AAC stereo PCM encode requires two-channel ADTS and PCM",
        ));
    }

    let mut out = Vec::new();
    for start_frame in pcm_frame_starts(pcm, pcm_config.start_frame)? {
        out.extend_from_slice(&encode_pcm_stereo_long_block_adts_by_bit_cost(
            adts,
            left,
            right,
            pcm,
            AacPcmLongBlockConfig::new(start_frame, pcm_config.step, pcm_config.band_width),
            tables,
        )?);
    }
    Ok(out)
}

/// Encodes an independent-stereo AAC-LC ADTS stream from PCM with per-frame scale-factor payloads.
pub fn encode_pcm_stereo_long_block_adts_stream_with_scale_factors(
    adts: AdtsConfig,
    left: AacScaleFactorSequence<'_>,
    right: AacScaleFactorSequence<'_>,
    pcm: &AudioBuffer,
    pcm_config: AacPcmLongBlockConfig,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<u8>, Error> {
    if adts.channels != 2 || pcm.channels != 2 {
        return Err(Error::InvalidInput(
            "AAC stereo PCM encode requires two-channel ADTS and PCM",
        ));
    }

    let starts = pcm_frame_starts(pcm, pcm_config.start_frame)?;
    if starts.len() != left.scale_factors_by_frame.len()
        || starts.len() != right.scale_factors_by_frame.len()
    {
        return Err(Error::InvalidInput(
            "AAC scale-factor frame count does not match PCM frame count",
        ));
    }

    let mut out = Vec::new();
    for (frame_index, start_frame) in starts.into_iter().enumerate() {
        out.extend_from_slice(&encode_pcm_stereo_long_block_adts_with_scale_factors(
            adts,
            left.channel_for_frame(frame_index)?,
            right.channel_for_frame(frame_index)?,
            pcm,
            AacPcmLongBlockConfig::new(start_frame, pcm_config.step, pcm_config.band_width),
            scale_factor_table,
            spectral_tables,
        )?);
    }
    Ok(out)
}

/// Encodes an independent-stereo AAC-LC ADTS stream from PCM with per-frame scale factors and bit-cost sections.
pub fn encode_pcm_stereo_long_block_adts_stream_with_scale_factors_by_bit_cost(
    adts: AdtsConfig,
    left: AacScaleFactorSequence<'_>,
    right: AacScaleFactorSequence<'_>,
    pcm: &AudioBuffer,
    pcm_config: AacPcmLongBlockConfig,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<u8>, Error> {
    if adts.channels != 2 || pcm.channels != 2 {
        return Err(Error::InvalidInput(
            "AAC stereo PCM encode requires two-channel ADTS and PCM",
        ));
    }

    let starts = pcm_frame_starts(pcm, pcm_config.start_frame)?;
    if starts.len() != left.scale_factors_by_frame.len()
        || starts.len() != right.scale_factors_by_frame.len()
    {
        return Err(Error::InvalidInput(
            "AAC scale-factor frame count does not match PCM frame count",
        ));
    }

    let mut out = Vec::new();
    for (frame_index, start_frame) in starts.into_iter().enumerate() {
        out.extend_from_slice(
            &encode_pcm_stereo_long_block_adts_with_scale_factors_by_bit_cost(
                adts,
                left.channel_for_frame(frame_index)?,
                right.channel_for_frame(frame_index)?,
                pcm,
                AacPcmLongBlockConfig::new(start_frame, pcm_config.step, pcm_config.band_width),
                scale_factor_table,
                spectral_tables,
            )?,
        );
    }
    Ok(out)
}

/// Encodes an independent-stereo AAC-LC ADTS stream from PCM with internally selected scale factors.
pub fn encode_pcm_stereo_long_block_adts_stream_with_selected_scale_factors(
    adts: AdtsConfig,
    left: AacLongBlockConfig,
    right: AacLongBlockConfig,
    pcm: &AudioBuffer,
    pcm_config: AacPcmLongBlockConfig,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<u8>, Error> {
    if adts.channels != 2 || pcm.channels != 2 {
        return Err(Error::InvalidInput(
            "AAC stereo PCM encode requires two-channel ADTS and PCM",
        ));
    }

    let mut out = Vec::new();
    for start_frame in pcm_frame_starts(pcm, pcm_config.start_frame)? {
        out.extend_from_slice(
            &encode_pcm_stereo_long_block_adts_with_selected_scale_factors(
                adts,
                left,
                right,
                pcm,
                AacPcmLongBlockConfig::new(start_frame, pcm_config.step, pcm_config.band_width),
                scale_factor_table,
                spectral_tables,
            )?,
        );
    }
    Ok(out)
}

/// Encodes an independent-stereo AAC-LC ADTS stream from PCM with selected scale factors and bit-cost sections.
pub fn encode_pcm_stereo_long_block_adts_stream_with_selected_scale_factors_by_bit_cost(
    adts: AdtsConfig,
    left: AacLongBlockConfig,
    right: AacLongBlockConfig,
    pcm: &AudioBuffer,
    pcm_config: AacPcmLongBlockConfig,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<u8>, Error> {
    if adts.channels != 2 || pcm.channels != 2 {
        return Err(Error::InvalidInput(
            "AAC stereo PCM encode requires two-channel ADTS and PCM",
        ));
    }

    let mut out = Vec::new();
    for start_frame in pcm_frame_starts(pcm, pcm_config.start_frame)? {
        out.extend_from_slice(
            &encode_pcm_stereo_long_block_adts_with_selected_scale_factors_by_bit_cost(
                adts,
                left,
                right,
                pcm,
                AacPcmLongBlockConfig::new(start_frame, pcm_config.step, pcm_config.band_width),
                scale_factor_table,
                spectral_tables,
            )?,
        );
    }
    Ok(out)
}

/// Selects the finest mono AAC-LC quantizer step that the current tables can pack.
pub fn select_aac_lc_mono_pcm_frame_step_by_bit_cost(
    adts: AdtsConfig,
    channel: AacLongBlockConfig,
    pcm: &AudioBuffer,
    search: AacPcmStepSearchConfig<'_>,
) -> Result<f32, Error> {
    Ok(select_aac_lc_mono_pcm_frame_step_details_by_bit_cost(adts, channel, pcm, search)?.step)
}

/// Selects the finest mono AAC-LC quantizer step within a caller-provided ADTS frame budget.
pub fn select_aac_lc_mono_pcm_frame_step_with_max_frame_len_by_bit_cost(
    adts: AdtsConfig,
    channel: AacLongBlockConfig,
    pcm: &AudioBuffer,
    search: AacPcmStepSearchConfig<'_>,
    max_frame_len_bytes: usize,
) -> Result<f32, Error> {
    Ok(
        select_aac_lc_mono_pcm_frame_step_details_with_max_frame_len_by_bit_cost(
            adts,
            channel,
            pcm,
            search,
            max_frame_len_bytes,
        )?
        .step,
    )
}

/// Selects the finest mono AAC-LC quantizer step and reports its ADTS frame size.
pub fn select_aac_lc_mono_pcm_frame_step_details_by_bit_cost(
    adts: AdtsConfig,
    channel: AacLongBlockConfig,
    pcm: &AudioBuffer,
    search: AacPcmStepSearchConfig<'_>,
) -> Result<AacPcmFrameStepSelection, Error> {
    if search.candidates.is_empty() {
        return Err(Error::InvalidInput(
            "AAC quantizer step candidate list is empty",
        ));
    }
    let mut selected: Option<AacPcmFrameStepSelection> = None;
    for &step in search.candidates {
        if !step.is_finite() || step <= 0.0 {
            return Err(Error::InvalidInput(
                "AAC quantizer step must be positive and finite",
            ));
        }
        if let Ok(selection) =
            evaluate_aac_lc_mono_pcm_frame_step_by_bit_cost(adts, channel, pcm, &search, step)
        {
            selected = match selected {
                Some(previous)
                    if selection.step > previous.step
                        || (selection.step == previous.step
                            && selection.frame_len <= previous.frame_len) =>
                {
                    Some(previous)
                }
                _ => Some(selection),
            };
        }
    }
    selected.ok_or(Error::UnsupportedFeature("AAC quantizer step search"))
}

/// Selects the finest mono AAC-LC quantizer step and reports its ADTS frame size
/// relative to a caller-provided frame budget.
pub fn select_aac_lc_mono_pcm_frame_step_details_with_max_frame_len_by_bit_cost(
    adts: AdtsConfig,
    channel: AacLongBlockConfig,
    pcm: &AudioBuffer,
    search: AacPcmStepSearchConfig<'_>,
    max_frame_len_bytes: usize,
) -> Result<AacPcmFrameStepSelection, Error> {
    validate_aac_max_frame_len(max_frame_len_bytes)?;
    if search.candidates.is_empty() {
        return Err(Error::InvalidInput(
            "AAC quantizer step candidate list is empty",
        ));
    }
    let mut selected: Option<AacPcmFrameStepSelection> = None;
    for &step in search.candidates {
        if !step.is_finite() || step <= 0.0 {
            return Err(Error::InvalidInput(
                "AAC quantizer step must be positive and finite",
            ));
        }
        if let Ok(selection) =
            evaluate_aac_lc_mono_pcm_frame_step_by_bit_cost(adts, channel, pcm, &search, step)
        {
            let Some(selection) =
                limit_aac_pcm_frame_step_selection(selection, max_frame_len_bytes)
            else {
                continue;
            };
            selected = select_better_aac_pcm_frame_step(selected, selection);
        }
    }
    selected.ok_or(Error::UnsupportedFeature("AAC quantizer step search"))
}

#[allow(clippy::too_many_arguments)]
pub fn select_aac_lc_mono_pcm_frame_step_with_offsets_by_bit_cost(
    adts: AdtsConfig,
    channel: AacLongBlockConfig,
    pcm: &AudioBuffer,
    start_frame: usize,
    offsets: &[usize],
    candidates: &[f32],
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<f32, Error> {
    Ok(
        select_aac_lc_mono_pcm_frame_step_details_with_offsets_by_bit_cost(
            adts,
            channel,
            pcm,
            start_frame,
            offsets,
            candidates,
            scale_factor_table,
            spectral_tables,
        )?
        .step,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn select_aac_lc_mono_pcm_frame_step_with_offsets_and_max_frame_len_by_bit_cost(
    adts: AdtsConfig,
    channel: AacLongBlockConfig,
    pcm: &AudioBuffer,
    start_frame: usize,
    offsets: &[usize],
    candidates: &[f32],
    max_frame_len_bytes: usize,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<f32, Error> {
    Ok(
        select_aac_lc_mono_pcm_frame_step_details_with_offsets_and_max_frame_len_by_bit_cost(
            adts,
            channel,
            pcm,
            start_frame,
            offsets,
            candidates,
            max_frame_len_bytes,
            scale_factor_table,
            spectral_tables,
        )?
        .step,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn select_aac_lc_mono_pcm_frame_step_with_offsets_and_scale_factors_by_bit_cost(
    adts: AdtsConfig,
    channel: AacScaleFactorChannel<'_>,
    pcm: &AudioBuffer,
    start_frame: usize,
    offsets: &[usize],
    candidates: &[f32],
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<f32, Error> {
    Ok(
        select_aac_lc_mono_pcm_frame_step_details_with_offsets_and_scale_factors_by_bit_cost(
            adts,
            channel,
            pcm,
            start_frame,
            offsets,
            candidates,
            scale_factor_table,
            spectral_tables,
        )?
        .step,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn select_aac_lc_mono_pcm_frame_step_with_offsets_and_scale_factors_and_max_frame_len_by_bit_cost(
    adts: AdtsConfig,
    channel: AacScaleFactorChannel<'_>,
    pcm: &AudioBuffer,
    start_frame: usize,
    offsets: &[usize],
    candidates: &[f32],
    max_frame_len_bytes: usize,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<f32, Error> {
    Ok(
        select_aac_lc_mono_pcm_frame_step_details_with_offsets_and_scale_factors_and_max_frame_len_by_bit_cost(
            adts,
            channel,
            pcm,
            start_frame,
            offsets,
            candidates,
            max_frame_len_bytes,
            scale_factor_table,
            spectral_tables,
        )?
        .step,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn select_aac_lc_stereo_pcm_frame_step_with_offsets_and_scale_factors_by_bit_cost(
    adts: AdtsConfig,
    left: AacScaleFactorChannel<'_>,
    right: AacScaleFactorChannel<'_>,
    pcm: &AudioBuffer,
    start_frame: usize,
    offsets: &[usize],
    candidates: &[f32],
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<f32, Error> {
    Ok(
        select_aac_lc_stereo_pcm_frame_step_details_with_offsets_and_scale_factors_by_bit_cost(
            adts,
            left,
            right,
            pcm,
            start_frame,
            offsets,
            candidates,
            scale_factor_table,
            spectral_tables,
        )?
        .step,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn select_aac_lc_stereo_pcm_frame_step_with_offsets_by_bit_cost(
    adts: AdtsConfig,
    left: AacLongBlockConfig,
    right: AacLongBlockConfig,
    pcm: &AudioBuffer,
    start_frame: usize,
    offsets: &[usize],
    candidates: &[f32],
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<f32, Error> {
    Ok(
        select_aac_lc_stereo_pcm_frame_step_details_with_offsets_by_bit_cost(
            adts,
            left,
            right,
            pcm,
            start_frame,
            offsets,
            candidates,
            scale_factor_table,
            spectral_tables,
        )?
        .step,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn select_aac_lc_stereo_pcm_frame_step_with_offsets_and_max_frame_len_by_bit_cost(
    adts: AdtsConfig,
    left: AacLongBlockConfig,
    right: AacLongBlockConfig,
    pcm: &AudioBuffer,
    start_frame: usize,
    offsets: &[usize],
    candidates: &[f32],
    max_frame_len_bytes: usize,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<f32, Error> {
    Ok(
        select_aac_lc_stereo_pcm_frame_step_details_with_offsets_and_max_frame_len_by_bit_cost(
            adts,
            left,
            right,
            pcm,
            start_frame,
            offsets,
            candidates,
            max_frame_len_bytes,
            scale_factor_table,
            spectral_tables,
        )?
        .step,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn select_aac_lc_stereo_pcm_frame_step_with_offsets_and_scale_factors_and_max_frame_len_by_bit_cost(
    adts: AdtsConfig,
    left: AacScaleFactorChannel<'_>,
    right: AacScaleFactorChannel<'_>,
    pcm: &AudioBuffer,
    start_frame: usize,
    offsets: &[usize],
    candidates: &[f32],
    max_frame_len_bytes: usize,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<f32, Error> {
    Ok(
        select_aac_lc_stereo_pcm_frame_step_details_with_offsets_and_scale_factors_and_max_frame_len_by_bit_cost(
            adts,
            left,
            right,
            pcm,
            start_frame,
            offsets,
            candidates,
            max_frame_len_bytes,
            scale_factor_table,
            spectral_tables,
        )?
        .step,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn select_aac_lc_mono_pcm_frame_step_details_with_offsets_by_bit_cost(
    adts: AdtsConfig,
    channel: AacLongBlockConfig,
    pcm: &AudioBuffer,
    start_frame: usize,
    offsets: &[usize],
    candidates: &[f32],
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<AacPcmFrameStepSelection, Error> {
    if candidates.is_empty() {
        return Err(Error::InvalidInput(
            "AAC quantizer step candidate list is empty",
        ));
    }
    let mut selected: Option<AacPcmFrameStepSelection> = None;
    for &step in candidates {
        if !step.is_finite() || step <= 0.0 {
            return Err(Error::InvalidInput(
                "AAC quantizer step must be positive and finite",
            ));
        }
        if let Ok(selection) = evaluate_aac_lc_mono_pcm_frame_step_with_offsets_by_bit_cost(
            adts,
            channel,
            pcm,
            start_frame,
            offsets,
            step,
            scale_factor_table,
            spectral_tables,
        ) {
            selected = match selected {
                Some(previous)
                    if selection.step > previous.step
                        || (selection.step == previous.step
                            && selection.frame_len <= previous.frame_len) =>
                {
                    Some(previous)
                }
                _ => Some(selection),
            };
        }
    }
    selected.ok_or(Error::UnsupportedFeature("AAC quantizer step search"))
}

#[allow(clippy::too_many_arguments)]
pub fn select_aac_lc_mono_pcm_frame_step_details_with_offsets_and_max_frame_len_by_bit_cost(
    adts: AdtsConfig,
    channel: AacLongBlockConfig,
    pcm: &AudioBuffer,
    start_frame: usize,
    offsets: &[usize],
    candidates: &[f32],
    max_frame_len_bytes: usize,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<AacPcmFrameStepSelection, Error> {
    validate_aac_max_frame_len(max_frame_len_bytes)?;
    if candidates.is_empty() {
        return Err(Error::InvalidInput(
            "AAC quantizer step candidate list is empty",
        ));
    }
    let mut selected: Option<AacPcmFrameStepSelection> = None;
    for &step in candidates {
        if !step.is_finite() || step <= 0.0 {
            return Err(Error::InvalidInput(
                "AAC quantizer step must be positive and finite",
            ));
        }
        if let Ok(selection) = evaluate_aac_lc_mono_pcm_frame_step_with_offsets_by_bit_cost(
            adts,
            channel,
            pcm,
            start_frame,
            offsets,
            step,
            scale_factor_table,
            spectral_tables,
        ) {
            let Some(selection) =
                limit_aac_pcm_frame_step_selection(selection, max_frame_len_bytes)
            else {
                continue;
            };
            selected = select_better_aac_pcm_frame_step(selected, selection);
        }
    }
    selected.ok_or(Error::UnsupportedFeature("AAC quantizer step search"))
}

#[allow(clippy::too_many_arguments)]
pub fn select_aac_lc_mono_pcm_frame_step_details_with_offsets_and_scale_factors_by_bit_cost(
    adts: AdtsConfig,
    channel: AacScaleFactorChannel<'_>,
    pcm: &AudioBuffer,
    start_frame: usize,
    offsets: &[usize],
    candidates: &[f32],
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<AacPcmFrameStepSelection, Error> {
    if candidates.is_empty() {
        return Err(Error::InvalidInput(
            "AAC quantizer step candidate list is empty",
        ));
    }
    let mut selected: Option<AacPcmFrameStepSelection> = None;
    for &step in candidates {
        if !step.is_finite() || step <= 0.0 {
            return Err(Error::InvalidInput(
                "AAC quantizer step must be positive and finite",
            ));
        }
        if let Ok(selection) =
            evaluate_aac_lc_mono_pcm_frame_step_with_offsets_and_scale_factors_by_bit_cost(
                adts,
                channel,
                pcm,
                start_frame,
                offsets,
                step,
                scale_factor_table,
                spectral_tables,
            )
        {
            selected = match selected {
                Some(previous)
                    if selection.step > previous.step
                        || (selection.step == previous.step
                            && selection.frame_len <= previous.frame_len) =>
                {
                    Some(previous)
                }
                _ => Some(selection),
            };
        }
    }
    selected.ok_or(Error::UnsupportedFeature("AAC quantizer step search"))
}

#[allow(clippy::too_many_arguments)]
pub fn select_aac_lc_mono_pcm_frame_step_details_with_offsets_and_scale_factors_and_max_frame_len_by_bit_cost(
    adts: AdtsConfig,
    channel: AacScaleFactorChannel<'_>,
    pcm: &AudioBuffer,
    start_frame: usize,
    offsets: &[usize],
    candidates: &[f32],
    max_frame_len_bytes: usize,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<AacPcmFrameStepSelection, Error> {
    validate_aac_max_frame_len(max_frame_len_bytes)?;
    if candidates.is_empty() {
        return Err(Error::InvalidInput(
            "AAC quantizer step candidate list is empty",
        ));
    }
    let mut selected: Option<AacPcmFrameStepSelection> = None;
    for &step in candidates {
        if !step.is_finite() || step <= 0.0 {
            return Err(Error::InvalidInput(
                "AAC quantizer step must be positive and finite",
            ));
        }
        if let Ok(selection) =
            evaluate_aac_lc_mono_pcm_frame_step_with_offsets_and_scale_factors_by_bit_cost(
                adts,
                channel,
                pcm,
                start_frame,
                offsets,
                step,
                scale_factor_table,
                spectral_tables,
            )
        {
            let Some(selection) =
                limit_aac_pcm_frame_step_selection(selection, max_frame_len_bytes)
            else {
                continue;
            };
            selected = select_better_aac_pcm_frame_step(selected, selection);
        }
    }
    selected.ok_or(Error::UnsupportedFeature("AAC quantizer step search"))
}

#[allow(clippy::too_many_arguments)]
pub fn select_aac_lc_stereo_pcm_frame_step_details_with_offsets_by_bit_cost(
    adts: AdtsConfig,
    left: AacLongBlockConfig,
    right: AacLongBlockConfig,
    pcm: &AudioBuffer,
    start_frame: usize,
    offsets: &[usize],
    candidates: &[f32],
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<AacPcmFrameStepSelection, Error> {
    if candidates.is_empty() {
        return Err(Error::InvalidInput(
            "AAC quantizer step candidate list is empty",
        ));
    }
    let mut selected: Option<AacPcmFrameStepSelection> = None;
    for &step in candidates {
        if !step.is_finite() || step <= 0.0 {
            return Err(Error::InvalidInput(
                "AAC quantizer step must be positive and finite",
            ));
        }
        if let Ok(selection) = evaluate_aac_lc_stereo_pcm_frame_step_with_offsets_by_bit_cost(
            adts,
            left,
            right,
            pcm,
            start_frame,
            offsets,
            step,
            scale_factor_table,
            spectral_tables,
        ) {
            selected = select_better_aac_pcm_frame_step(selected, selection);
        }
    }
    selected.ok_or(Error::UnsupportedFeature("AAC quantizer step search"))
}

#[allow(clippy::too_many_arguments)]
pub fn select_aac_lc_stereo_pcm_frame_step_details_with_offsets_and_max_frame_len_by_bit_cost(
    adts: AdtsConfig,
    left: AacLongBlockConfig,
    right: AacLongBlockConfig,
    pcm: &AudioBuffer,
    start_frame: usize,
    offsets: &[usize],
    candidates: &[f32],
    max_frame_len_bytes: usize,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<AacPcmFrameStepSelection, Error> {
    validate_aac_max_frame_len(max_frame_len_bytes)?;
    if candidates.is_empty() {
        return Err(Error::InvalidInput(
            "AAC quantizer step candidate list is empty",
        ));
    }
    let mut selected: Option<AacPcmFrameStepSelection> = None;
    for &step in candidates {
        if !step.is_finite() || step <= 0.0 {
            return Err(Error::InvalidInput(
                "AAC quantizer step must be positive and finite",
            ));
        }
        if let Ok(selection) = evaluate_aac_lc_stereo_pcm_frame_step_with_offsets_by_bit_cost(
            adts,
            left,
            right,
            pcm,
            start_frame,
            offsets,
            step,
            scale_factor_table,
            spectral_tables,
        ) {
            let Some(selection) =
                limit_aac_pcm_frame_step_selection(selection, max_frame_len_bytes)
            else {
                continue;
            };
            selected = select_better_aac_pcm_frame_step(selected, selection);
        }
    }
    selected.ok_or(Error::UnsupportedFeature("AAC quantizer step search"))
}

#[allow(clippy::too_many_arguments)]
pub fn select_aac_lc_stereo_pcm_frame_step_details_with_offsets_and_scale_factors_by_bit_cost(
    adts: AdtsConfig,
    left: AacScaleFactorChannel<'_>,
    right: AacScaleFactorChannel<'_>,
    pcm: &AudioBuffer,
    start_frame: usize,
    offsets: &[usize],
    candidates: &[f32],
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<AacPcmFrameStepSelection, Error> {
    if candidates.is_empty() {
        return Err(Error::InvalidInput(
            "AAC quantizer step candidate list is empty",
        ));
    }
    let mut selected: Option<AacPcmFrameStepSelection> = None;
    for &step in candidates {
        if !step.is_finite() || step <= 0.0 {
            return Err(Error::InvalidInput(
                "AAC quantizer step must be positive and finite",
            ));
        }
        if let Ok(selection) =
            evaluate_aac_lc_stereo_pcm_frame_step_with_offsets_and_scale_factors_by_bit_cost(
                adts,
                left,
                right,
                pcm,
                start_frame,
                offsets,
                step,
                scale_factor_table,
                spectral_tables,
            )
        {
            selected = select_better_aac_pcm_frame_step(selected, selection);
        }
    }
    selected.ok_or(Error::UnsupportedFeature("AAC quantizer step search"))
}

#[allow(clippy::too_many_arguments)]
pub fn select_aac_lc_stereo_pcm_frame_step_details_with_offsets_and_scale_factors_and_max_frame_len_by_bit_cost(
    adts: AdtsConfig,
    left: AacScaleFactorChannel<'_>,
    right: AacScaleFactorChannel<'_>,
    pcm: &AudioBuffer,
    start_frame: usize,
    offsets: &[usize],
    candidates: &[f32],
    max_frame_len_bytes: usize,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<AacPcmFrameStepSelection, Error> {
    validate_aac_max_frame_len(max_frame_len_bytes)?;
    if candidates.is_empty() {
        return Err(Error::InvalidInput(
            "AAC quantizer step candidate list is empty",
        ));
    }
    let mut selected: Option<AacPcmFrameStepSelection> = None;
    for &step in candidates {
        if !step.is_finite() || step <= 0.0 {
            return Err(Error::InvalidInput(
                "AAC quantizer step must be positive and finite",
            ));
        }
        if let Ok(selection) =
            evaluate_aac_lc_stereo_pcm_frame_step_with_offsets_and_scale_factors_by_bit_cost(
                adts,
                left,
                right,
                pcm,
                start_frame,
                offsets,
                step,
                scale_factor_table,
                spectral_tables,
            )
        {
            let Some(selection) =
                limit_aac_pcm_frame_step_selection(selection, max_frame_len_bytes)
            else {
                continue;
            };
            selected = select_better_aac_pcm_frame_step(selected, selection);
        }
    }
    selected.ok_or(Error::UnsupportedFeature("AAC quantizer step search"))
}

/// Selects the finest stereo AAC-LC quantizer step that the current tables can pack.
pub fn select_aac_lc_stereo_pcm_frame_step_by_bit_cost(
    adts: AdtsConfig,
    left: AacLongBlockConfig,
    right: AacLongBlockConfig,
    pcm: &AudioBuffer,
    search: AacPcmStepSearchConfig<'_>,
) -> Result<f32, Error> {
    Ok(
        select_aac_lc_stereo_pcm_frame_step_details_by_bit_cost(adts, left, right, pcm, search)?
            .step,
    )
}

/// Selects the finest stereo AAC-LC quantizer step within a caller-provided ADTS frame budget.
pub fn select_aac_lc_stereo_pcm_frame_step_with_max_frame_len_by_bit_cost(
    adts: AdtsConfig,
    left: AacLongBlockConfig,
    right: AacLongBlockConfig,
    pcm: &AudioBuffer,
    search: AacPcmStepSearchConfig<'_>,
    max_frame_len_bytes: usize,
) -> Result<f32, Error> {
    Ok(
        select_aac_lc_stereo_pcm_frame_step_details_with_max_frame_len_by_bit_cost(
            adts,
            left,
            right,
            pcm,
            search,
            max_frame_len_bytes,
        )?
        .step,
    )
}

/// Selects the finest stereo AAC-LC quantizer step and reports its ADTS frame size.
pub fn select_aac_lc_stereo_pcm_frame_step_details_by_bit_cost(
    adts: AdtsConfig,
    left: AacLongBlockConfig,
    right: AacLongBlockConfig,
    pcm: &AudioBuffer,
    search: AacPcmStepSearchConfig<'_>,
) -> Result<AacPcmFrameStepSelection, Error> {
    if search.candidates.is_empty() {
        return Err(Error::InvalidInput(
            "AAC quantizer step candidate list is empty",
        ));
    }
    let mut selected: Option<AacPcmFrameStepSelection> = None;
    for &step in search.candidates {
        if !step.is_finite() || step <= 0.0 {
            return Err(Error::InvalidInput(
                "AAC quantizer step must be positive and finite",
            ));
        }
        if let Ok(selection) =
            evaluate_aac_lc_stereo_pcm_frame_step_by_bit_cost(adts, left, right, pcm, &search, step)
        {
            selected = match selected {
                Some(previous)
                    if selection.step > previous.step
                        || (selection.step == previous.step
                            && selection.frame_len <= previous.frame_len) =>
                {
                    Some(previous)
                }
                _ => Some(selection),
            };
        }
    }
    selected.ok_or(Error::UnsupportedFeature("AAC quantizer step search"))
}

/// Selects the finest stereo AAC-LC quantizer step and reports its ADTS frame size
/// relative to a caller-provided frame budget.
pub fn select_aac_lc_stereo_pcm_frame_step_details_with_max_frame_len_by_bit_cost(
    adts: AdtsConfig,
    left: AacLongBlockConfig,
    right: AacLongBlockConfig,
    pcm: &AudioBuffer,
    search: AacPcmStepSearchConfig<'_>,
    max_frame_len_bytes: usize,
) -> Result<AacPcmFrameStepSelection, Error> {
    validate_aac_max_frame_len(max_frame_len_bytes)?;
    if search.candidates.is_empty() {
        return Err(Error::InvalidInput(
            "AAC quantizer step candidate list is empty",
        ));
    }
    let mut selected: Option<AacPcmFrameStepSelection> = None;
    for &step in search.candidates {
        if !step.is_finite() || step <= 0.0 {
            return Err(Error::InvalidInput(
                "AAC quantizer step must be positive and finite",
            ));
        }
        if let Ok(selection) =
            evaluate_aac_lc_stereo_pcm_frame_step_by_bit_cost(adts, left, right, pcm, &search, step)
        {
            let Some(selection) =
                limit_aac_pcm_frame_step_selection(selection, max_frame_len_bytes)
            else {
                continue;
            };
            selected = select_better_aac_pcm_frame_step(selected, selection);
        }
    }
    selected.ok_or(Error::UnsupportedFeature("AAC quantizer step search"))
}

fn validate_aac_max_frame_len(max_frame_len_bytes: usize) -> Result<(), Error> {
    if max_frame_len_bytes == 0 {
        return Err(Error::InvalidInput(
            "AAC max frame length must be greater than zero",
        ));
    }
    Ok(())
}

fn limit_aac_pcm_frame_step_selection(
    mut selection: AacPcmFrameStepSelection,
    max_frame_len_bytes: usize,
) -> Option<AacPcmFrameStepSelection> {
    if selection.frame_len > max_frame_len_bytes {
        return None;
    }
    selection.frame_capacity_bytes = max_frame_len_bytes;
    Some(selection)
}

fn select_better_aac_pcm_frame_step(
    selected: Option<AacPcmFrameStepSelection>,
    selection: AacPcmFrameStepSelection,
) -> Option<AacPcmFrameStepSelection> {
    match selected {
        Some(previous)
            if selection.step > previous.step
                || (selection.step == previous.step
                    && selection.frame_len <= previous.frame_len) =>
        {
            Some(previous)
        }
        _ => Some(selection),
    }
}

fn evaluate_aac_lc_mono_pcm_frame_step_by_bit_cost(
    adts: AdtsConfig,
    channel: AacLongBlockConfig,
    pcm: &AudioBuffer,
    search: &AacPcmStepSearchConfig<'_>,
    step: f32,
) -> Result<AacPcmFrameStepSelection, Error> {
    let frame = encode_pcm_mono_long_block_adts_with_selected_scale_factors_by_bit_cost(
        adts,
        channel,
        pcm,
        AacPcmLongBlockConfig::new(search.start_frame, step, search.band_width),
        search.scale_factor_table,
        search.spectral_tables,
    )?;
    Ok(AacPcmFrameStepSelection {
        step,
        frame_len: frame.len(),
        frame_capacity_bytes: AAC_ADTS_MAX_FRAME_LEN,
    })
}

#[allow(clippy::too_many_arguments)]
fn evaluate_aac_lc_mono_pcm_frame_step_with_offsets_by_bit_cost(
    adts: AdtsConfig,
    channel: AacLongBlockConfig,
    pcm: &AudioBuffer,
    start_frame: usize,
    offsets: &[usize],
    step: f32,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<AacPcmFrameStepSelection, Error> {
    let frame =
        encode_pcm_mono_long_block_adts_with_offsets_and_selected_scale_factors_by_bit_cost(
            adts,
            channel,
            pcm,
            start_frame,
            step,
            offsets,
            scale_factor_table,
            spectral_tables,
        )?;
    Ok(AacPcmFrameStepSelection {
        step,
        frame_len: frame.len(),
        frame_capacity_bytes: AAC_ADTS_MAX_FRAME_LEN,
    })
}

#[allow(clippy::too_many_arguments)]
fn evaluate_aac_lc_mono_pcm_frame_step_with_offsets_and_scale_factors_by_bit_cost(
    adts: AdtsConfig,
    channel: AacScaleFactorChannel<'_>,
    pcm: &AudioBuffer,
    start_frame: usize,
    offsets: &[usize],
    step: f32,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<AacPcmFrameStepSelection, Error> {
    let frame = encode_pcm_mono_long_block_adts_with_offsets_and_scale_factors_by_bit_cost(
        adts,
        channel,
        pcm,
        start_frame,
        step,
        offsets,
        scale_factor_table,
        spectral_tables,
    )?;
    Ok(AacPcmFrameStepSelection {
        step,
        frame_len: frame.len(),
        frame_capacity_bytes: AAC_ADTS_MAX_FRAME_LEN,
    })
}

fn evaluate_aac_lc_stereo_pcm_frame_step_by_bit_cost(
    adts: AdtsConfig,
    left: AacLongBlockConfig,
    right: AacLongBlockConfig,
    pcm: &AudioBuffer,
    search: &AacPcmStepSearchConfig<'_>,
    step: f32,
) -> Result<AacPcmFrameStepSelection, Error> {
    let frame = encode_pcm_stereo_long_block_adts_with_selected_scale_factors_by_bit_cost(
        adts,
        left,
        right,
        pcm,
        AacPcmLongBlockConfig::new(search.start_frame, step, search.band_width),
        search.scale_factor_table,
        search.spectral_tables,
    )?;
    Ok(AacPcmFrameStepSelection {
        step,
        frame_len: frame.len(),
        frame_capacity_bytes: AAC_ADTS_MAX_FRAME_LEN,
    })
}

#[allow(clippy::too_many_arguments)]
fn evaluate_aac_lc_stereo_pcm_frame_step_with_offsets_by_bit_cost(
    adts: AdtsConfig,
    left: AacLongBlockConfig,
    right: AacLongBlockConfig,
    pcm: &AudioBuffer,
    start_frame: usize,
    offsets: &[usize],
    step: f32,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<AacPcmFrameStepSelection, Error> {
    let frame =
        encode_pcm_stereo_long_block_adts_with_offsets_and_selected_scale_factors_by_bit_cost(
            adts,
            left,
            right,
            pcm,
            start_frame,
            step,
            offsets,
            scale_factor_table,
            spectral_tables,
        )?;
    Ok(AacPcmFrameStepSelection {
        step,
        frame_len: frame.len(),
        frame_capacity_bytes: AAC_ADTS_MAX_FRAME_LEN,
    })
}

#[allow(clippy::too_many_arguments)]
fn evaluate_aac_lc_stereo_pcm_frame_step_with_offsets_and_scale_factors_by_bit_cost(
    adts: AdtsConfig,
    left: AacScaleFactorChannel<'_>,
    right: AacScaleFactorChannel<'_>,
    pcm: &AudioBuffer,
    start_frame: usize,
    offsets: &[usize],
    step: f32,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<AacPcmFrameStepSelection, Error> {
    let frame = encode_pcm_stereo_long_block_adts_with_offsets_and_scale_factors_by_bit_cost(
        adts,
        left,
        right,
        pcm,
        start_frame,
        step,
        offsets,
        scale_factor_table,
        spectral_tables,
    )?;
    Ok(AacPcmFrameStepSelection {
        step,
        frame_len: frame.len(),
        frame_capacity_bytes: AAC_ADTS_MAX_FRAME_LEN,
    })
}

/// Encodes a mono AAC-LC ADTS stream with per-frame quantizer step search.
pub fn encode_pcm_mono_long_block_adts_stream_with_auto_step_by_bit_cost(
    adts: AdtsConfig,
    channel: AacLongBlockConfig,
    pcm: &AudioBuffer,
    search: AacPcmStepSearchConfig<'_>,
) -> Result<Vec<u8>, Error> {
    if adts.channels != 1 || pcm.channels != 1 {
        return Err(Error::InvalidInput(
            "AAC mono PCM encode requires one-channel ADTS and PCM",
        ));
    }

    let mut out = Vec::new();
    for frame_start in pcm_frame_starts(pcm, search.start_frame)? {
        let step = select_aac_lc_mono_pcm_frame_step_by_bit_cost(
            adts,
            channel,
            pcm,
            AacPcmStepSearchConfig::new(
                frame_start,
                search.band_width,
                search.candidates,
                search.scale_factor_table,
                search.spectral_tables,
            ),
        )?;
        out.extend_from_slice(
            &encode_pcm_mono_long_block_adts_with_selected_scale_factors_by_bit_cost(
                adts,
                channel,
                pcm,
                AacPcmLongBlockConfig::new(frame_start, step, search.band_width),
                search.scale_factor_table,
                search.spectral_tables,
            )?,
        );
    }
    Ok(out)
}

pub fn encode_pcm_mono_long_block_adts_stream_with_offsets_and_auto_step_by_bit_cost(
    adts: AdtsConfig,
    channel: AacLongBlockConfig,
    pcm: &AudioBuffer,
    offsets: &[usize],
    candidates: &[f32],
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<u8>, Error> {
    if adts.channels != 1 || pcm.channels != 1 {
        return Err(Error::InvalidInput(
            "AAC mono PCM encode requires one-channel ADTS and PCM",
        ));
    }

    let mut out = Vec::new();
    for frame_start in pcm_frame_starts(pcm, 0)? {
        let step = select_aac_lc_mono_pcm_frame_step_with_offsets_by_bit_cost(
            adts,
            channel,
            pcm,
            frame_start,
            offsets,
            candidates,
            scale_factor_table,
            spectral_tables,
        )?;
        out.extend_from_slice(
            &encode_pcm_mono_long_block_adts_with_offsets_and_selected_scale_factors_by_bit_cost(
                adts,
                channel,
                pcm,
                frame_start,
                step,
                offsets,
                scale_factor_table,
                spectral_tables,
            )?,
        );
    }
    Ok(out)
}

/// Encodes a stereo AAC-LC ADTS stream with per-frame quantizer step search.
pub fn encode_pcm_stereo_long_block_adts_stream_with_auto_step_by_bit_cost(
    adts: AdtsConfig,
    left: AacLongBlockConfig,
    right: AacLongBlockConfig,
    pcm: &AudioBuffer,
    search: AacPcmStepSearchConfig<'_>,
) -> Result<Vec<u8>, Error> {
    if adts.channels != 2 || pcm.channels != 2 {
        return Err(Error::InvalidInput(
            "AAC stereo PCM encode requires two-channel ADTS and PCM",
        ));
    }

    let mut out = Vec::new();
    for frame_start in pcm_frame_starts(pcm, search.start_frame)? {
        let step = select_aac_lc_stereo_pcm_frame_step_by_bit_cost(
            adts,
            left,
            right,
            pcm,
            AacPcmStepSearchConfig::new(
                frame_start,
                search.band_width,
                search.candidates,
                search.scale_factor_table,
                search.spectral_tables,
            ),
        )?;
        out.extend_from_slice(
            &encode_pcm_stereo_long_block_adts_with_selected_scale_factors_by_bit_cost(
                adts,
                left,
                right,
                pcm,
                AacPcmLongBlockConfig::new(frame_start, step, search.band_width),
                search.scale_factor_table,
                search.spectral_tables,
            )?,
        );
    }
    Ok(out)
}

fn non_empty_table<'a>(
    table: &'a [HuffmanEntry<AacSpectralPair>],
    name: &'static str,
) -> Result<&'a [HuffmanEntry<AacSpectralPair>], Error> {
    if table.is_empty() {
        return Err(Error::UnsupportedFeature(name));
    }
    Ok(table)
}

fn non_empty_magnitude_table<'a>(
    table: &'a [HuffmanEntry<AacSpectralMagnitudePair>],
    name: &'static str,
) -> Result<&'a [HuffmanEntry<AacSpectralMagnitudePair>], Error> {
    if table.is_empty() {
        return Err(Error::UnsupportedFeature(name));
    }
    Ok(table)
}

fn non_empty_quad_table<'a>(
    table: &'a [HuffmanEntry<AacSpectralMagnitudeQuad>],
    name: &'static str,
) -> Result<&'a [HuffmanEntry<AacSpectralMagnitudeQuad>], Error> {
    if table.is_empty() {
        return Err(Error::UnsupportedFeature(name));
    }
    Ok(table)
}

fn validate_scale_factor_band_offsets(quantized: &[i32], offsets: &[usize]) -> Result<(), Error> {
    if offsets.len() < 2 || offsets.first().copied() != Some(0) {
        return Err(Error::InvalidInput("invalid AAC scale-factor offsets"));
    }
    if offsets.last().copied() != Some(quantized.len()) {
        return Err(Error::InvalidInput(
            "AAC scale-factor offsets must cover the spectrum",
        ));
    }
    for band in offsets.windows(2) {
        if band[0] >= band[1] {
            return Err(Error::InvalidInput("invalid AAC scale-factor offsets"));
        }
    }
    Ok(())
}

fn offset_band_index(offsets: &[usize], offset: usize) -> Result<usize, Error> {
    offsets
        .iter()
        .position(|&candidate| candidate == offset)
        .ok_or(Error::InvalidInput(
            "AAC section boundary is not a scale-factor offset",
        ))
}

fn pcm_frame_starts(pcm: &AudioBuffer, first_start_frame: usize) -> Result<Vec<usize>, Error> {
    let frames = pcm.frames();
    if first_start_frame > frames {
        return Err(Error::InvalidInput(
            "AAC PCM start frame is past end of input",
        ));
    }
    let frame_count = frames
        .saturating_sub(first_start_frame)
        .div_ceil(1024)
        .max(1);
    (0..frame_count)
        .map(|index| {
            first_start_frame
                .checked_add(index * 1024)
                .ok_or(Error::InvalidInput("AAC PCM frame start overflows"))
        })
        .collect()
}

fn aac_spectral_pair_magnitude(pair: AacSpectralPair) -> Result<AacSpectralMagnitudePair, Error> {
    AacSpectralMagnitudePair::try_from(pair)
}

fn aac_spectral_quad_magnitude(quad: AacSpectralQuad) -> Result<AacSpectralMagnitudeQuad, Error> {
    AacSpectralMagnitudeQuad::try_from(quad)
}

fn write_aac_escape_suffix(writer: &mut CoreBitWriter, magnitude: u16) -> Result<(), Error> {
    if magnitude < AAC_ESCAPE_MAGNITUDE {
        return Ok(());
    }

    let mut threshold = AAC_ESCAPE_MAGNITUDE;
    let mut suffix_len = 4_u8;
    while magnitude
        >= threshold
            .checked_mul(2)
            .ok_or(Error::InvalidInput("AAC escape threshold overflows"))?
    {
        writer.write_bits(1, 1)?;
        threshold = threshold
            .checked_mul(2)
            .ok_or(Error::InvalidInput("AAC escape threshold overflows"))?;
        suffix_len = suffix_len
            .checked_add(1)
            .ok_or(Error::InvalidInput("AAC escape suffix length overflows"))?;
    }
    writer.write_bits(0, 1)?;
    writer.write_bits(u32::from(magnitude - threshold), suffix_len)
}

fn write_aac_sign_bit(writer: &mut CoreBitWriter, value: i16) -> Result<(), Error> {
    if value != 0 {
        writer.write_bits(u32::from(value < 0), 1)?;
    }
    Ok(())
}

fn sample_rate_index(sample_rate: u32) -> Result<u8, Error> {
    const SAMPLE_RATES: [u32; 13] = [
        96_000, 88_200, 64_000, 48_000, 44_100, 32_000, 24_000, 22_050, 16_000, 12_000, 11_025,
        8_000, 7_350,
    ];
    SAMPLE_RATES
        .iter()
        .position(|&rate| rate == sample_rate)
        .and_then(|index| u8::try_from(index).ok())
        .ok_or(Error::UnsupportedFeature("AAC sample rate"))
}

fn decode_silent_adts(input: &[u8]) -> Result<AudioBuffer, Error> {
    let mut remaining = input;
    let mut sample_rate = None;
    let mut channels = None;
    let mut frame_count = 0_usize;

    while !remaining.is_empty() {
        let frame = parse_adts_frame(remaining)?;
        let config = AdtsConfig {
            profile: AacProfile::LowComplexity,
            sample_rate: frame.sample_rate,
            channels: frame.channels,
        };
        if frame.profile != AacProfile::LowComplexity || !is_locally_supported_zero_payload(&frame)?
        {
            return Err(Error::UnsupportedFeature(
                "AAC decode currently supports sonare silent AAC-LC ADTS only",
            ));
        }

        match (sample_rate, channels) {
            (Some(sample_rate), Some(channels))
                if sample_rate != frame.sample_rate || channels != frame.channels =>
            {
                return Err(Error::UnsupportedFeature(
                    "AAC ADTS parameter changes within stream",
                ));
            }
            (None, None) => {
                sample_rate = Some(config.sample_rate);
                channels = Some(config.channels);
            }
            _ => return Err(Error::InvalidInput("inconsistent AAC decoder state")),
        }

        frame_count = frame_count
            .checked_add(1)
            .ok_or(Error::InvalidInput("too many AAC ADTS frames"))?;
        remaining = &remaining[frame.frame_len..];
    }

    let sample_rate = sample_rate.ok_or(Error::InvalidInput("AAC stream has no frames"))?;
    let channels = channels.ok_or(Error::InvalidInput("AAC stream has no frames"))?;
    let sample_count = frame_count
        .checked_mul(1024)
        .and_then(|frames| frames.checked_mul(usize::from(channels)))
        .ok_or(Error::InvalidInput("decoded AAC PCM is too large"))?;
    AudioBuffer::new(sample_rate, u16::from(channels), vec![0.0; sample_count])
}

fn is_locally_supported_zero_payload(frame: &ParsedAdtsFrame<'_>) -> Result<bool, Error> {
    Ok(
        frame.payload == encode_silent_raw_data_block(frame.channels)?
            || frame.payload == encode_zero_spectral_long_block_raw_data_block(frame.channels)?,
    )
}

struct ParsedAdtsFrame<'a> {
    profile: AacProfile,
    sample_rate: u32,
    channels: u8,
    frame_len: usize,
    payload: &'a [u8],
}

fn parse_adts_frame(input: &[u8]) -> Result<ParsedAdtsFrame<'_>, Error> {
    if input.len() < 7 {
        return Err(Error::InvalidInput("truncated AAC ADTS header"));
    }
    if input[0] != 0xff || input[1] & 0xf0 != 0xf0 {
        return Err(Error::InvalidInput("missing AAC ADTS sync word"));
    }

    let protection_absent = input[1] & 0x01 != 0;
    let header_len = if protection_absent { 7 } else { 9 };
    let profile = match (input[2] >> 6) & 0x03 {
        0 => AacProfile::Main,
        1 => AacProfile::LowComplexity,
        2 => AacProfile::ScalableSampleRate,
        _ => return Err(Error::UnsupportedFeature("AAC LTP profile")),
    };
    let sample_rate_index = (input[2] >> 2) & 0x0f;
    let channels = ((input[2] & 0x01) << 2) | ((input[3] >> 6) & 0x03);
    let frame_len = (usize::from(input[3] & 0x03) << 11)
        | (usize::from(input[4]) << 3)
        | usize::from(input[5] >> 5);

    if channels == 0 {
        return Err(Error::UnsupportedFeature(
            "AAC program config elements are not supported",
        ));
    }
    if frame_len < header_len {
        return Err(Error::InvalidInput("invalid AAC ADTS frame length"));
    }
    if input.len() < frame_len {
        return Err(Error::InvalidInput("truncated AAC ADTS frame"));
    }

    Ok(ParsedAdtsFrame {
        profile,
        sample_rate: sample_rate_from_index(sample_rate_index)?,
        channels,
        frame_len,
        payload: &input[header_len..frame_len],
    })
}

fn sample_rate_from_index(index: u8) -> Result<u32, Error> {
    const SAMPLE_RATES: [u32; 13] = [
        96_000, 88_200, 64_000, 48_000, 44_100, 32_000, 24_000, 22_050, 16_000, 12_000, 11_025,
        8_000, 7_350,
    ];
    SAMPLE_RATES
        .get(usize::from(index))
        .copied()
        .ok_or(Error::InvalidInput("invalid AAC sample-rate index"))
}

fn fixed_block<const N: usize>(samples: &[f32]) -> Result<[f32; N], Error> {
    samples
        .try_into()
        .map_err(|_| Error::InvalidInput("analysis block length mismatch"))
}

fn classify_aac_codebook(quantized: &[i32]) -> Result<AacCodebook, Error> {
    let max_abs = quantized
        .iter()
        .map(|coeff| coeff.checked_abs())
        .collect::<Option<Vec<_>>>()
        .ok_or(Error::InvalidInput("AAC spectral coefficient overflows"))?
        .into_iter()
        .max()
        .unwrap_or(0);
    Ok(match max_abs {
        0 => AacCodebook::Zero,
        1..=7 => AacCodebook::UnsignedPairs7,
        8..=12 => AacCodebook::UnsignedPairs9,
        13..=8191 => AacCodebook::Escape,
        _ => {
            return Err(Error::InvalidInput(
                "AAC spectral coefficient exceeds supported codebook range",
            ));
        }
    })
}

fn encode_silent_raw_data_block(channels: u8) -> Result<Vec<u8>, Error> {
    let mut writer = BitWriter::new();
    match channels {
        1 => {
            writer.write_bits(0, 3)?;
            writer.write_bits(0, 4)?;
            write_silent_individual_channel_stream(&mut writer)?;
        }
        2 => {
            writer.write_bits(1, 3)?;
            writer.write_bits(0, 4)?;
            writer.write_bits(0, 1)?;
            write_silent_individual_channel_stream(&mut writer)?;
            write_silent_individual_channel_stream(&mut writer)?;
        }
        _ => {
            return Err(Error::UnsupportedFeature(
                "AAC-LC encode currently supports mono/stereo only",
            ));
        }
    }
    writer.write_bits(7, 3)?;
    Ok(writer.finish_byte_aligned())
}

fn encode_zero_spectral_long_block_raw_data_block(channels: u8) -> Result<Vec<u8>, Error> {
    let sections = [AacSection {
        start: 0,
        end: 1024,
        codebook: AacCodebook::Zero,
    }];
    let payload = pack_section_data_with_len(&sections, 1024)?;
    let channel = AacLongBlockConfig::new(0, 1);
    match channels {
        1 => pack_single_channel_raw_data_block(channel, &payload),
        2 => pack_channel_pair_raw_data_block(channel, &payload, channel, &payload),
        _ => Err(Error::UnsupportedFeature(
            "AAC-LC encode currently supports mono/stereo only",
        )),
    }
}

fn write_silent_individual_channel_stream(writer: &mut BitWriter) -> Result<(), Error> {
    writer.write_bits(0, 8)?;
    writer.write_bits(0, 1)?;
    writer.write_bits(0, 2)?;
    writer.write_bits(0, 1)?;
    writer.write_bits(0, 6)?;
    writer.write_bits(0, 1)?;
    writer.write_bits(0, 1)?;
    writer.write_bits(0, 1)?;
    writer.write_bits(0, 1)
}

#[derive(Default)]
struct BitWriter {
    out: Vec<u8>,
    bit_pos: u8,
}

impl BitWriter {
    fn new() -> Self {
        Self::default()
    }

    fn write_bits(&mut self, value: u32, count: u8) -> Result<(), Error> {
        if count > 32 {
            return Err(Error::InvalidInput(
                "cannot write more than 32 bits at once",
            ));
        }
        if count < 32 && value >= (1_u32 << count) {
            return Err(Error::InvalidInput("bit value exceeds width"));
        }

        for bit_index in (0..count).rev() {
            if self.bit_pos == 0 {
                self.out.push(0);
            }
            let bit = ((value >> bit_index) & 1) as u8;
            let byte = self
                .out
                .last_mut()
                .ok_or(Error::InvalidInput("bit writer has no current byte"))?;
            *byte |= bit << (7 - self.bit_pos);
            self.bit_pos = (self.bit_pos + 1) % 8;
        }
        Ok(())
    }

    fn finish_byte_aligned(self) -> Vec<u8> {
        self.out
    }
}

#[cfg(test)]
mod tests {
    use super::{
        aac_escape_table, aac_lc_adts_max_frame_len_for_bitrate,
        aac_lc_default_production_bitrate_bps, aac_lc_long_window_scale_factor_band_offsets,
        aac_lc_standard_spectral_tables, aac_scale_factor_delta_table,
        aac_scale_factor_delta_zero_table, aac_unsigned_pairs10_table, aac_unsigned_pairs7_table,
        aac_unsigned_pairs7_unit_magnitude_spectral_tables, aac_unsigned_pairs8_table,
        aac_unsigned_pairs9_table, encode, encode_pcm_mono_long_block_adts,
        encode_pcm_mono_long_block_adts_by_bit_cost, encode_pcm_mono_long_block_adts_stream,
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
        encode_pcm_stereo_long_block_adts, encode_pcm_stereo_long_block_adts_by_bit_cost,
        encode_pcm_stereo_long_block_adts_stream,
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
        frame_adts, frame_adts_stream, mdct_long_block, mux_adts_as_m4a,
        pack_channel_pair_raw_data_block, pack_channel_pair_raw_data_block_parts,
        pack_channel_payload_parts, pack_long_block_individual_channel_stream,
        pack_quad_section_data_with_len, pack_scale_factor_deltas_with_table, pack_section_data,
        pack_section_data_with_len, pack_section_data_with_offsets,
        pack_sectioned_spectral_payload, pack_sectioned_spectral_payload_with_scale_factor_bits,
        pack_sectioned_spectral_payload_with_sign_bits,
        pack_sectioned_spectral_payload_with_sign_bits_and_scale_factor_bits,
        pack_sectioned_spectral_payload_with_sign_bits_and_scale_factor_bits_by_bit_cost,
        pack_sectioned_spectral_payload_with_sign_bits_by_bit_cost,
        pack_sectioned_spectral_quad_payload_with_sign_bits, pack_single_channel_raw_data_block,
        pack_single_channel_raw_data_block_parts, pack_spectral_codewords,
        pack_spectral_codewords_with_len, pack_spectral_pairs_with_sign_bits,
        pack_spectral_pairs_with_table, pack_spectral_quad_sections_with_sign_bits,
        pack_spectral_quads_with_sign_bits, pack_spectral_quads_with_table, pack_spectral_sections,
        pack_spectral_sections_with_sign_bits, parse_adts_frame, plan_scale_factor_deltas,
        plan_scale_factor_deltas_by_offsets, plan_sections, plan_sections_by_bit_cost,
        plan_sections_by_offsets, quantize_long_block, quantize_pcm_long_block,
        select_aac_lc_mono_pcm_frame_step_by_bit_cost,
        select_aac_lc_mono_pcm_frame_step_details_by_bit_cost,
        select_aac_lc_mono_pcm_frame_step_details_with_max_frame_len_by_bit_cost,
        select_aac_lc_mono_pcm_frame_step_details_with_offsets_and_max_frame_len_by_bit_cost,
        select_aac_lc_mono_pcm_frame_step_details_with_offsets_and_scale_factors_and_max_frame_len_by_bit_cost,
        select_aac_lc_mono_pcm_frame_step_details_with_offsets_and_scale_factors_by_bit_cost,
        select_aac_lc_mono_pcm_frame_step_details_with_offsets_by_bit_cost,
        select_aac_lc_mono_pcm_frame_step_with_max_frame_len_by_bit_cost,
        select_aac_lc_mono_pcm_frame_step_with_offsets_and_max_frame_len_by_bit_cost,
        select_aac_lc_mono_pcm_frame_step_with_offsets_and_scale_factors_and_max_frame_len_by_bit_cost,
        select_aac_lc_stereo_pcm_frame_step_by_bit_cost,
        select_aac_lc_stereo_pcm_frame_step_details_by_bit_cost,
        select_aac_lc_stereo_pcm_frame_step_details_with_max_frame_len_by_bit_cost,
        select_aac_lc_stereo_pcm_frame_step_details_with_offsets_and_max_frame_len_by_bit_cost,
        select_aac_lc_stereo_pcm_frame_step_details_with_offsets_and_scale_factors_and_max_frame_len_by_bit_cost,
        select_aac_lc_stereo_pcm_frame_step_details_with_offsets_and_scale_factors_by_bit_cost,
        select_aac_lc_stereo_pcm_frame_step_details_with_offsets_by_bit_cost,
        select_aac_lc_stereo_pcm_frame_step_with_max_frame_len_by_bit_cost,
        select_aac_lc_stereo_pcm_frame_step_with_offsets_and_max_frame_len_by_bit_cost,
        select_aac_lc_stereo_pcm_frame_step_with_offsets_and_scale_factors_and_max_frame_len_by_bit_cost,
        select_aac_lc_stereo_pcm_frame_step_with_offsets_by_bit_cost, select_codebook_by_bit_cost,
        select_scale_factors_for_quantized_bands,
        select_scale_factors_for_quantized_bands_by_offsets, spectral_pairs_for_section,
        spectral_quads_for_section, split_sectioned_spectral_payload_with_sign_bits, AacCodebook,
        AacLongBlockConfig, AacPcmFrameStepSelection, AacPcmLongBlockConfig,
        AacPcmStepSearchConfig, AacQuadSection, AacQuantizedChannel, AacQuantizedSpectrum,
        AacScaleFactorChannel, AacScaleFactorDelta, AacScaleFactorSequence, AacSection,
        AacSpectralMagnitudePair, AacSpectralMagnitudeQuad, AacSpectralMagnitudeQuadTables,
        AacSpectralMagnitudeTables, AacSpectralPair, AacSpectralQuad, AacSpectralTables,
        AdtsConfig, BitWriter, AAC_ADTS_HEADER_LEN,
        AAC_LC_16K_LONG_WINDOW_SCALE_FACTOR_BAND_OFFSETS,
        AAC_LC_24K_LONG_WINDOW_SCALE_FACTOR_BAND_OFFSETS,
        AAC_LC_32K_LONG_WINDOW_SCALE_FACTOR_BAND_OFFSETS,
        AAC_LC_48K_LONG_WINDOW_SCALE_FACTOR_BAND_OFFSETS,
        AAC_LC_64K_LONG_WINDOW_SCALE_FACTOR_BAND_OFFSETS,
        AAC_LC_8K_LONG_WINDOW_SCALE_FACTOR_BAND_OFFSETS,
        AAC_LC_96K_LONG_WINDOW_SCALE_FACTOR_BAND_OFFSETS, AAC_LC_PCM_STEP_CANDIDATES,
    };
    use sc_core::Error;
    use sc_core::{AudioBuffer, HuffmanCode, HuffmanEntry, PackedBits};

    fn max_adts_frame_len(stream: &[u8]) -> usize {
        let mut remaining = stream;
        let mut max_frame_len = 0;
        while !remaining.is_empty() {
            let frame = parse_adts_frame(remaining).unwrap();
            max_frame_len = max_frame_len.max(frame.frame_len);
            remaining = &remaining[frame.frame_len..];
        }
        max_frame_len
    }

    #[test]
    fn frames_raw_access_unit_as_adts() {
        let frame = frame_adts(AdtsConfig::aac_lc(44_100, 2), &[0x11, 0x22]).unwrap();

        assert_eq!(&frame[..7], &[0xff, 0xf1, 0x50, 0x80, 0x01, 0x3f, 0xfc]);
        assert_eq!(&frame[7..], &[0x11, 0x22]);
    }

    #[test]
    fn frames_multiple_access_units_as_adts_stream() {
        let stream = frame_adts_stream(
            AdtsConfig::aac_lc(48_000, 1),
            [&[0xaa][..], &[0xbb, 0xcc][..]],
        )
        .unwrap();

        assert_eq!(stream[0], 0xff);
        assert_eq!(stream[8], 0xff);
        assert_eq!(stream.len(), 17);
    }

    #[test]
    fn muxes_adts_via_mp4_module() {
        let adts = frame_adts(AdtsConfig::aac_lc(44_100, 2), &[0x11, 0x22]).unwrap();
        let m4a = mux_adts_as_m4a(&adts).unwrap();

        assert_eq!(&m4a[4..8], b"ftyp");
        assert!(m4a.windows(4).any(|window| window == b"mdat"));
        assert!(m4a.windows(4).any(|window| window == b"moov"));
    }

    #[test]
    fn computes_long_block_mdct_for_aac_analysis() {
        let mut samples = [0.0_f32; 2048];
        samples[0] = 1.0;

        let coeffs = mdct_long_block(&samples).unwrap();

        assert_eq!(coeffs.len(), 1024);
        assert!(coeffs.iter().any(|coeff| coeff.abs() > 0.0));
        assert_eq!(mdct_long_block(&[0.0; 2048]).unwrap(), vec![0.0; 1024]);
    }

    #[test]
    fn quantizes_long_block_for_aac_analysis() {
        let mut samples = [0.0_f32; 2048];
        samples[0] = 1.0;

        let quantized = quantize_long_block(&samples, 0.001).unwrap();

        assert_eq!(quantized.len(), 1024);
        assert!(quantized.iter().any(|coeff| *coeff != 0));
        assert_eq!(
            quantize_long_block(&[0.0; 2048], 1.0).unwrap(),
            vec![0; 1024]
        );
        assert!(quantize_long_block(&samples, 0.0).is_err());
    }

    #[test]
    fn quantizes_pcm_long_block_for_aac_analysis() {
        let pcm = AudioBuffer::new(44_100, 2, vec![1.0, -1.0, 0.0, 0.0]).unwrap();

        let left = quantize_pcm_long_block(&pcm, 0, 0, 0.001).unwrap();
        let right = quantize_pcm_long_block(&pcm, 1, 0, 0.001).unwrap();
        let padded = quantize_pcm_long_block(&pcm, 0, 10, 1.0).unwrap();

        assert_eq!(left.len(), 1024);
        assert_eq!(right.len(), 1024);
        assert_ne!(left, right);
        assert_eq!(padded, vec![0; 1024]);
        assert!(quantize_pcm_long_block(&pcm, 2, 0, 1.0).is_err());
    }

    #[test]
    fn plans_aac_codebook_sections() {
        let quantized = [0, 0, 0, 0, 1, -1, 0, 1, 3, -4, 0, 2, 9, 0, -5, 1];

        let sections = plan_sections(&quantized, 4).unwrap();

        assert_eq!(
            sections,
            vec![
                AacSection {
                    start: 0,
                    end: 4,
                    codebook: AacCodebook::Zero,
                },
                AacSection {
                    start: 4,
                    end: 12,
                    codebook: AacCodebook::UnsignedPairs7,
                },
                AacSection {
                    start: 12,
                    end: 16,
                    codebook: AacCodebook::UnsignedPairs9,
                },
            ]
        );
        assert_eq!(AacCodebook::Escape.id(), 11);
        assert!(plan_sections(&quantized, 0).is_err());
        assert!(plan_sections(&quantized[..15], 4).is_err());
        assert!(plan_sections(&[8192], 1).is_err());
    }

    #[test]
    fn default_aac_section_planner_uses_available_standard_unsigned_pair_tables() {
        let quantized = [2, -7, 0, 1, 8, -12, 0, 0, 13, 0, 0, 0];
        let sections = plan_sections(&quantized, 4).unwrap();

        assert_eq!(
            sections,
            vec![
                AacSection {
                    start: 0,
                    end: 4,
                    codebook: AacCodebook::UnsignedPairs7,
                },
                AacSection {
                    start: 4,
                    end: 8,
                    codebook: AacCodebook::UnsignedPairs9,
                },
                AacSection {
                    start: 8,
                    end: 12,
                    codebook: AacCodebook::Escape,
                },
            ]
        );
        assert!(
            pack_spectral_sections_with_sign_bits(
                &sections[..2],
                &quantized,
                AacSpectralMagnitudeTables::default(),
            )
            .unwrap()
            .bit_len
                > 0
        );
        assert!(pack_spectral_sections_with_sign_bits(
            &sections[2..],
            &quantized,
            AacSpectralMagnitudeTables::default(),
        )
        .is_err());
    }

    #[test]
    fn plans_aac_codebook_sections_by_bit_cost() {
        let pairs1 = [HuffmanEntry {
            symbol: AacSpectralMagnitudePair::new(1, 1),
            code: HuffmanCode::new(0b1111, 4).unwrap(),
        }];
        let pairs5 = [HuffmanEntry {
            symbol: AacSpectralMagnitudePair::new(1, 1),
            code: HuffmanCode::new(0b0, 1).unwrap(),
        }];
        let tables = AacSpectralMagnitudeTables {
            pairs1: &pairs1,
            pairs5: &pairs5,
            pairs6: &[],
            escape: &[],
        };

        assert_eq!(
            select_codebook_by_bit_cost(&[1, -1], tables).unwrap(),
            AacCodebook::SignedPairs5
        );
        let pairs6 = [HuffmanEntry {
            symbol: AacSpectralMagnitudePair::new(1, 1),
            code: HuffmanCode::new(0b1, 1).unwrap(),
        }];
        assert_eq!(
            select_codebook_by_bit_cost(
                &[1, -1],
                AacSpectralMagnitudeTables {
                    pairs1: &pairs1,
                    pairs5: &[],
                    pairs6: &pairs6,
                    escape: &[],
                },
            )
            .unwrap(),
            AacCodebook::SignedPairs6
        );
        assert_eq!(
            select_codebook_by_bit_cost(&[0, 0], AacSpectralMagnitudeTables::default()).unwrap(),
            AacCodebook::Zero
        );
        assert_eq!(
            select_codebook_by_bit_cost(&[1, -1], AacSpectralMagnitudeTables::default()).unwrap(),
            AacCodebook::UnsignedPairs8
        );
        assert_eq!(
            select_codebook_by_bit_cost(&[2, 0], AacSpectralMagnitudeTables::default()).unwrap(),
            AacCodebook::UnsignedPairs8
        );
        assert_eq!(
            select_codebook_by_bit_cost(&[12, -12], AacSpectralMagnitudeTables::default()).unwrap(),
            AacCodebook::UnsignedPairs10
        );
        assert!(
            select_codebook_by_bit_cost(&[17, 0], AacSpectralMagnitudeTables::default()).is_err()
        );
        assert_eq!(
            select_codebook_by_bit_cost(
                &[17, 0],
                AacSpectralMagnitudeTables {
                    pairs1: &[],
                    pairs5: &[],
                    pairs6: &[],
                    escape: aac_escape_table(),
                },
            )
            .unwrap(),
            AacCodebook::Escape
        );
        assert_eq!(
            plan_sections_by_bit_cost(&[1, -1, 0, 0], 2, tables).unwrap(),
            vec![
                AacSection {
                    start: 0,
                    end: 2,
                    codebook: AacCodebook::SignedPairs5,
                },
                AacSection {
                    start: 2,
                    end: 4,
                    codebook: AacCodebook::Zero,
                },
            ]
        );
        let default_sections =
            plan_sections_by_bit_cost(&[1, -1, 0, 0], 2, AacSpectralMagnitudeTables::default())
                .unwrap();
        assert_eq!(
            default_sections,
            vec![
                AacSection {
                    start: 0,
                    end: 2,
                    codebook: AacCodebook::UnsignedPairs8,
                },
                AacSection {
                    start: 2,
                    end: 4,
                    codebook: AacCodebook::Zero,
                },
            ]
        );
        assert_eq!(
            pack_section_data_with_len(&default_sections, 2).unwrap(),
            PackedBits {
                bytes: vec![0b1000_0000, 0b1000_0000, 0b0100_0000],
                bit_len: 18,
            }
        );
    }

    #[test]
    fn experimental_unit_tables_pack_nonzero_sections() {
        let tables = experimental_unit_magnitude_spectral_tables();
        let quantized = [1, -1, 0, 1];

        let sections = plan_sections_by_bit_cost(&quantized, 2, tables).unwrap();
        let payload =
            pack_sectioned_spectral_payload_with_sign_bits_by_bit_cost(&quantized, 2, tables)
                .unwrap();
        let adts = encode_quantized_mono_adts_by_bit_cost(
            AdtsConfig::aac_lc(44_100, 1),
            AacLongBlockConfig::new(0, 2),
            &quantized,
            2,
            tables,
        )
        .unwrap();

        assert_eq!(
            sections,
            vec![AacSection {
                start: 0,
                end: 4,
                codebook: AacCodebook::SignedPairs1,
            }]
        );
        assert!(payload.bit_len > 0);
        assert_eq!(&adts[..2], &[0xff, 0xf1]);
        assert!(adts.len() > 7);
    }

    #[test]
    fn plans_and_packs_aac_scale_factor_deltas() {
        let sections = vec![
            AacSection {
                start: 0,
                end: 2,
                codebook: AacCodebook::Zero,
            },
            AacSection {
                start: 2,
                end: 4,
                codebook: AacCodebook::SignedPairs1,
            },
            AacSection {
                start: 4,
                end: 8,
                codebook: AacCodebook::SignedPairs5,
            },
        ];
        let deltas = plan_scale_factor_deltas(&sections, 2, &[7, 10, 12, 11], 9).unwrap();
        let table = [
            HuffmanEntry {
                symbol: AacScaleFactorDelta::new(-1),
                code: HuffmanCode::new(0b0, 1).unwrap(),
            },
            HuffmanEntry {
                symbol: AacScaleFactorDelta::new(1),
                code: HuffmanCode::new(0b10, 2).unwrap(),
            },
            HuffmanEntry {
                symbol: AacScaleFactorDelta::new(2),
                code: HuffmanCode::new(0b110, 3).unwrap(),
            },
        ];

        assert_eq!(
            deltas,
            vec![
                AacScaleFactorDelta::new(1),
                AacScaleFactorDelta::new(2),
                AacScaleFactorDelta::new(-1),
            ]
        );
        assert_eq!(
            pack_scale_factor_deltas_with_table(&deltas, &table).unwrap(),
            PackedBits {
                bytes: vec![0b1011_0000],
                bit_len: 6,
            }
        );
        assert!(plan_scale_factor_deltas(&sections, 0, &[7, 10, 12, 11], 9).is_err());
        assert!(plan_scale_factor_deltas(&sections, 2, &[7, 10], 9).is_err());
        assert!(
            pack_scale_factor_deltas_with_table(&[AacScaleFactorDelta::new(3)], &table).is_err()
        );
    }

    #[test]
    fn exposes_standard_aac_scale_factor_delta_table() {
        let table = aac_scale_factor_delta_table();

        assert_eq!(table.len(), 121);
        assert_eq!(table.first().unwrap().symbol, AacScaleFactorDelta::new(-60));
        assert_eq!(table.last().unwrap().symbol, AacScaleFactorDelta::new(60));
        assert_eq!(table[60].symbol, AacScaleFactorDelta::new(0));
        assert_eq!(table[60].code, HuffmanCode::new(0, 1).unwrap());
        assert_eq!(table[59].symbol, AacScaleFactorDelta::new(-1));
        assert_eq!(table[59].code, HuffmanCode::new(0b100, 3).unwrap());
        assert_eq!(table[61].symbol, AacScaleFactorDelta::new(1));
        assert_eq!(table[61].code, HuffmanCode::new(0b1010, 4).unwrap());
        assert_eq!(
            pack_scale_factor_deltas_with_table(
                &[
                    AacScaleFactorDelta::new(-1),
                    AacScaleFactorDelta::new(0),
                    AacScaleFactorDelta::new(1),
                ],
                &table,
            )
            .unwrap(),
            PackedBits {
                bytes: vec![0b1000_1010],
                bit_len: 8,
            }
        );
        assert!(
            pack_scale_factor_deltas_with_table(&[AacScaleFactorDelta::new(61)], &table).is_err()
        );
    }

    #[test]
    fn selects_aac_scale_factors_from_quantized_band_magnitudes() {
        let quantized = [0, 0, 1, -1, 3, -4, 9, 0];
        let sections = plan_sections(&quantized, 2).unwrap();

        let scale_factors = select_scale_factors_for_quantized_bands(&quantized, 2, 100).unwrap();
        let deltas = plan_scale_factor_deltas(&sections, 2, &scale_factors, 100).unwrap();

        assert_eq!(scale_factors, vec![100, 101, 103, 104]);
        assert_eq!(
            deltas,
            vec![
                AacScaleFactorDelta::new(1),
                AacScaleFactorDelta::new(2),
                AacScaleFactorDelta::new(1),
            ]
        );
        assert!(select_scale_factors_for_quantized_bands(&quantized, 0, 100).is_err());
        assert!(select_scale_factors_for_quantized_bands(&quantized[..7], 2, 100).is_err());
        assert!(select_scale_factors_for_quantized_bands(&[i32::MIN, 0], 2, 100).is_err());
    }

    #[test]
    fn plans_aac_sections_with_standard_long_window_offsets() {
        let offsets = aac_lc_long_window_scale_factor_band_offsets(44_100).unwrap();
        let mut quantized = vec![0; 1024];
        quantized[4] = 1;
        quantized[5] = -1;
        quantized[40] = 1;

        let sections = plan_sections_by_offsets(
            &quantized,
            offsets,
            experimental_unit_magnitude_spectral_tables(),
        )
        .unwrap();
        let scale_factors =
            select_scale_factors_for_quantized_bands_by_offsets(&quantized, offsets, 100).unwrap();
        let deltas =
            plan_scale_factor_deltas_by_offsets(&sections, offsets, &scale_factors, 100).unwrap();
        let section_bits = pack_section_data_with_offsets(&sections, offsets).unwrap();

        for sample_rate in [88_200, 96_000] {
            let offsets_96k = aac_lc_long_window_scale_factor_band_offsets(sample_rate).unwrap();
            assert_eq!(
                offsets_96k,
                AAC_LC_96K_LONG_WINDOW_SCALE_FACTOR_BAND_OFFSETS
            );
            assert_eq!(offsets_96k.first().copied(), Some(0));
            assert_eq!(offsets_96k.last().copied(), Some(1024));
            assert_eq!(offsets_96k.len() - 1, 41);
        }
        let offsets_64k = aac_lc_long_window_scale_factor_band_offsets(64_000).unwrap();
        assert_eq!(
            offsets_64k,
            AAC_LC_64K_LONG_WINDOW_SCALE_FACTOR_BAND_OFFSETS
        );
        assert_eq!(offsets_64k.first().copied(), Some(0));
        assert_eq!(offsets_64k.last().copied(), Some(1024));
        assert_eq!(offsets_64k.len() - 1, 47);
        assert_eq!(offsets, AAC_LC_48K_LONG_WINDOW_SCALE_FACTOR_BAND_OFFSETS);
        assert_eq!(offsets.first().copied(), Some(0));
        assert_eq!(offsets.last().copied(), Some(1024));
        assert_eq!(offsets.len() - 1, 49);
        let offsets_32k = aac_lc_long_window_scale_factor_band_offsets(32_000).unwrap();
        assert_eq!(
            offsets_32k,
            AAC_LC_32K_LONG_WINDOW_SCALE_FACTOR_BAND_OFFSETS
        );
        assert_eq!(offsets_32k.first().copied(), Some(0));
        assert_eq!(offsets_32k.last().copied(), Some(1024));
        assert_eq!(offsets_32k.len() - 1, 51);
        for sample_rate in [22_050, 24_000] {
            let offsets_24k = aac_lc_long_window_scale_factor_band_offsets(sample_rate).unwrap();
            assert_eq!(
                offsets_24k,
                AAC_LC_24K_LONG_WINDOW_SCALE_FACTOR_BAND_OFFSETS
            );
            assert_eq!(offsets_24k.first().copied(), Some(0));
            assert_eq!(offsets_24k.last().copied(), Some(1024));
            assert_eq!(offsets_24k.len() - 1, 47);
        }
        for sample_rate in [11_025, 12_000, 16_000] {
            let offsets_16k = aac_lc_long_window_scale_factor_band_offsets(sample_rate).unwrap();
            assert_eq!(
                offsets_16k,
                AAC_LC_16K_LONG_WINDOW_SCALE_FACTOR_BAND_OFFSETS
            );
            assert_eq!(offsets_16k.first().copied(), Some(0));
            assert_eq!(offsets_16k.last().copied(), Some(1024));
            assert_eq!(offsets_16k.len() - 1, 43);
        }
        for sample_rate in [7_350, 8_000] {
            let offsets_8k = aac_lc_long_window_scale_factor_band_offsets(sample_rate).unwrap();
            assert_eq!(offsets_8k, AAC_LC_8K_LONG_WINDOW_SCALE_FACTOR_BAND_OFFSETS);
            assert_eq!(offsets_8k.first().copied(), Some(0));
            assert_eq!(offsets_8k.last().copied(), Some(1024));
            assert_eq!(offsets_8k.len() - 1, 40);
        }
        assert!(sections
            .iter()
            .any(|section| section.codebook == AacCodebook::SignedPairs1));
        assert_eq!(scale_factors.len(), offsets.len() - 1);
        assert_eq!(deltas.len(), 2);
        assert!(section_bits.bit_len > 0);
        assert!(plan_sections_by_offsets(&quantized[..1023], offsets, Default::default()).is_err());
    }

    #[test]
    fn encodes_mono_stream_with_standard_long_window_offsets() {
        let offsets = aac_lc_long_window_scale_factor_band_offsets(44_100).unwrap();
        let pcm = AudioBuffer::new(
            44_100,
            1,
            (0..2048)
                .map(|sample| ((sample as f32) * 0.01).sin() * 0.25)
                .collect(),
        )
        .unwrap();
        let channel = AacLongBlockConfig::new(0, (offsets.len() - 1) as u8);
        let scale_factor_table = experimental_aac_scale_factor_delta_table();
        let flat_scale_factors = vec![120; offsets.len() - 1];
        let flat_channel = AacScaleFactorChannel::new(
            AacLongBlockConfig::new(120, (offsets.len() - 1) as u8),
            &flat_scale_factors,
        );
        let zero_scale_factor_table = aac_scale_factor_delta_zero_table();

        let details = select_aac_lc_mono_pcm_frame_step_details_with_offsets_by_bit_cost(
            AdtsConfig::aac_lc(44_100, 1),
            channel,
            &pcm,
            0,
            offsets,
            AAC_LC_PCM_STEP_CANDIDATES,
            &scale_factor_table,
            experimental_unit_magnitude_spectral_tables(),
        )
        .unwrap();
        let flat_details =
            select_aac_lc_mono_pcm_frame_step_details_with_offsets_and_scale_factors_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 1),
                flat_channel,
                &pcm,
                0,
                offsets,
                AAC_LC_PCM_STEP_CANDIDATES,
                zero_scale_factor_table,
                aac_unsigned_pairs7_unit_magnitude_spectral_tables(),
            )
            .unwrap();
        let adts = encode_pcm_mono_long_block_adts_stream_with_offsets_and_auto_step_by_bit_cost(
            AdtsConfig::aac_lc(44_100, 1),
            channel,
            &pcm,
            offsets,
            AAC_LC_PCM_STEP_CANDIDATES,
            &scale_factor_table,
            experimental_unit_magnitude_spectral_tables(),
        )
        .unwrap();
        let flat_adts =
            encode_pcm_mono_long_block_adts_stream_with_offsets_and_scale_factors_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 1),
                flat_channel,
                &pcm,
                offsets,
                AAC_LC_PCM_STEP_CANDIDATES,
                zero_scale_factor_table,
                aac_unsigned_pairs7_unit_magnitude_spectral_tables(),
            )
            .unwrap();

        assert!(details.step < f32::MAX);
        assert!(flat_details.step < f32::MAX);
        assert_eq!(
            pack_scale_factor_deltas_with_table(
                &[AacScaleFactorDelta::new(0)],
                zero_scale_factor_table
            )
            .unwrap()
            .bit_len,
            1
        );
        assert!(pack_scale_factor_deltas_with_table(
            &[AacScaleFactorDelta::new(1)],
            zero_scale_factor_table
        )
        .is_err());
        assert_eq!(&adts[..2], &[0xff, 0xf1]);
        assert_eq!(&flat_adts[..2], &[0xff, 0xf1]);
        assert!(adts.len() > 7);
        assert!(flat_adts.len() > 7);
    }

    #[test]
    fn packs_aac_section_data() {
        let sections = vec![
            AacSection {
                start: 0,
                end: 4,
                codebook: AacCodebook::Zero,
            },
            AacSection {
                start: 4,
                end: 12,
                codebook: AacCodebook::SignedPairs5,
            },
            AacSection {
                start: 12,
                end: 16,
                codebook: AacCodebook::Escape,
            },
        ];

        let packed = pack_section_data(&sections, 4).unwrap();

        assert_eq!(packed, &[0x00, 0xa8, 0xac, 0x20]);
        assert_eq!(
            pack_section_data_with_len(&sections, 4).unwrap(),
            PackedBits {
                bytes: vec![0x00, 0xa8, 0xac, 0x20],
                bit_len: 27,
            }
        );
        assert_eq!(
            pack_section_data(
                &[AacSection {
                    start: 0,
                    end: 128,
                    codebook: AacCodebook::SignedPairs1,
                }],
                4
            )
            .unwrap(),
            &[0x1f, 0x84]
        );
        assert!(pack_section_data(&sections, 0).is_err());
        assert!(pack_section_data(
            &[AacSection {
                start: 1,
                end: 4,
                codebook: AacCodebook::Zero,
            }],
            4
        )
        .is_err());
    }

    #[test]
    fn extracts_aac_spectral_pairs_for_section() {
        let quantized = [0, 0, 1, -1, 3, 0, -2, 2];

        assert_eq!(
            spectral_pairs_for_section(
                &quantized,
                &AacSection {
                    start: 0,
                    end: 2,
                    codebook: AacCodebook::Zero,
                },
            )
            .unwrap(),
            Vec::<AacSpectralPair>::new()
        );
        assert_eq!(
            spectral_pairs_for_section(
                &quantized,
                &AacSection {
                    start: 2,
                    end: 8,
                    codebook: AacCodebook::SignedPairs5,
                },
            )
            .unwrap(),
            vec![
                AacSpectralPair::new(1, -1),
                AacSpectralPair::new(3, 0),
                AacSpectralPair::new(-2, 2),
            ]
        );
        assert!(spectral_pairs_for_section(
            &quantized,
            &AacSection {
                start: 1,
                end: 4,
                codebook: AacCodebook::SignedPairs1,
            },
        )
        .is_err());
        assert!(spectral_pairs_for_section(
            &quantized,
            &AacSection {
                start: 6,
                end: 10,
                codebook: AacCodebook::SignedPairs5,
            },
        )
        .is_err());
    }

    #[test]
    fn packs_aac_spectral_codewords() {
        let codes = [
            HuffmanCode::new(0b10, 2).unwrap(),
            HuffmanCode::new(0b011, 3).unwrap(),
            HuffmanCode::new(0b1, 1).unwrap(),
        ];

        assert_eq!(pack_spectral_codewords(&codes).unwrap(), &[0b1001_1100]);
        assert_eq!(
            pack_spectral_codewords_with_len(&codes).unwrap(),
            PackedBits {
                bytes: vec![0b1001_1100],
                bit_len: 6,
            }
        );
    }

    #[test]
    fn packs_aac_spectral_pairs_from_table() {
        let table = [
            HuffmanEntry {
                symbol: AacSpectralPair::new(0, 0),
                code: HuffmanCode::new(0b0, 1).unwrap(),
            },
            HuffmanEntry {
                symbol: AacSpectralPair::new(1, -1),
                code: HuffmanCode::new(0b10, 2).unwrap(),
            },
            HuffmanEntry {
                symbol: AacSpectralPair::new(-2, 1),
                code: HuffmanCode::new(0b110, 3).unwrap(),
            },
        ];
        let pairs = [
            AacSpectralPair::new(1, -1),
            AacSpectralPair::new(0, 0),
            AacSpectralPair::new(-2, 1),
        ];

        assert_eq!(
            pack_spectral_pairs_with_table(&pairs, &table).unwrap(),
            PackedBits {
                bytes: vec![0b1001_1000],
                bit_len: 6,
            }
        );
        assert!(pack_spectral_pairs_with_table(&[AacSpectralPair::new(2, 2)], &table).is_err());
    }

    #[test]
    fn packs_aac_spectral_pairs_with_sign_bits() {
        let table = [
            HuffmanEntry {
                symbol: AacSpectralMagnitudePair::new(0, 0),
                code: HuffmanCode::new(0b0, 1).unwrap(),
            },
            HuffmanEntry {
                symbol: AacSpectralMagnitudePair::new(1, 1),
                code: HuffmanCode::new(0b10, 2).unwrap(),
            },
            HuffmanEntry {
                symbol: AacSpectralMagnitudePair::new(2, 0),
                code: HuffmanCode::new(0b110, 3).unwrap(),
            },
        ];
        let pairs = [
            AacSpectralPair::new(1, -1),
            AacSpectralPair::new(-2, 0),
            AacSpectralPair::new(0, 0),
        ];

        assert_eq!(
            super::pack_spectral_pairs_with_sign_bits(&pairs, &table).unwrap(),
            PackedBits {
                bytes: vec![0b1001_1101, 0b0000_0000],
                bit_len: 9,
            }
        );
        assert!(super::pack_spectral_pairs_with_sign_bits(
            &[AacSpectralPair::new(i16::MIN, 0)],
            &table,
        )
        .is_err());
    }

    #[test]
    fn exposes_aac_unsigned_pairs7_unit_magnitude_table() {
        let table = super::aac_unsigned_pairs7_unit_magnitude_table();
        assert_eq!(table.len(), 4);
        assert_eq!(table[0].symbol, AacSpectralMagnitudePair::new(0, 0));
        assert_eq!(table[0].code, HuffmanCode::new(0b0, 1).unwrap());
        assert_eq!(table[1].symbol, AacSpectralMagnitudePair::new(0, 1));
        assert_eq!(table[1].code, HuffmanCode::new(0b101, 3).unwrap());
        assert_eq!(table[2].symbol, AacSpectralMagnitudePair::new(1, 0));
        assert_eq!(table[2].code, HuffmanCode::new(0b100, 3).unwrap());
        assert_eq!(table[3].symbol, AacSpectralMagnitudePair::new(1, 1));
        assert_eq!(table[3].code, HuffmanCode::new(0b1100, 4).unwrap());

        let pairs = [
            AacSpectralPair::new(0, 0),
            AacSpectralPair::new(1, -1),
            AacSpectralPair::new(-1, 0),
            AacSpectralPair::new(0, 1),
        ];
        assert_eq!(
            super::pack_spectral_pairs_with_sign_bits(&pairs, table).unwrap(),
            PackedBits {
                bytes: vec![0b0110_0011, 0b0011_0100],
                bit_len: 15,
            }
        );
    }

    #[test]
    fn exposes_full_aac_unsigned_pairs7_table() {
        let table = aac_unsigned_pairs7_table();

        assert_eq!(table.len(), 64);
        assert_eq!(table[0].symbol, AacSpectralMagnitudePair::new(0, 0));
        assert_eq!(table[0].code, HuffmanCode::new(0b0, 1).unwrap());
        assert_eq!(table[9].symbol, AacSpectralMagnitudePair::new(1, 1));
        assert_eq!(table[9].code, HuffmanCode::new(0x00c, 4).unwrap());
        assert_eq!(table[18].symbol, AacSpectralMagnitudePair::new(2, 2));
        assert_eq!(table[18].code, HuffmanCode::new(0x072, 7).unwrap());
        assert_eq!(table[63].symbol, AacSpectralMagnitudePair::new(7, 7));
        assert_eq!(table[63].code, HuffmanCode::new(0xfff, 12).unwrap());

        let packed = pack_sectioned_spectral_payload_with_sign_bits_by_bit_cost(
            &[2, -2],
            2,
            AacSpectralMagnitudeTables::default(),
        )
        .unwrap();
        assert_eq!(
            packed,
            PackedBits {
                bytes: vec![0b1000_0000, 0b1011_0010],
                bit_len: 15,
            }
        );
    }

    #[test]
    fn exposes_full_aac_unsigned_pairs8_table() {
        let table = aac_unsigned_pairs8_table();

        assert_eq!(table.len(), 64);
        assert_eq!(table[0].symbol, AacSpectralMagnitudePair::new(0, 0));
        assert_eq!(table[0].code, HuffmanCode::new(0x00e, 5).unwrap());
        assert_eq!(table[9].symbol, AacSpectralMagnitudePair::new(1, 1));
        assert_eq!(table[9].code, HuffmanCode::new(0x000, 3).unwrap());
        assert_eq!(table[63].symbol, AacSpectralMagnitudePair::new(7, 7));
        assert_eq!(table[63].code, HuffmanCode::new(0x3ff, 10).unwrap());

        let packed = pack_sectioned_spectral_payload_with_sign_bits_by_bit_cost(
            &[1, -1],
            2,
            AacSpectralMagnitudeTables::default(),
        )
        .unwrap();
        assert_eq!(
            packed,
            PackedBits {
                bytes: vec![0b1000_0000, 0b1000_0100],
                bit_len: 14,
            }
        );
    }

    #[test]
    fn exposes_full_aac_unsigned_pairs9_table() {
        let table = aac_unsigned_pairs9_table();

        assert_eq!(table.len(), 169);
        assert_eq!(table[0].symbol, AacSpectralMagnitudePair::new(0, 0));
        assert_eq!(table[0].code, HuffmanCode::new(0x0000, 1).unwrap());
        assert_eq!(table[14].symbol, AacSpectralMagnitudePair::new(1, 1));
        assert_eq!(table[14].code, HuffmanCode::new(0x000c, 4).unwrap());
        assert_eq!(table[168].symbol, AacSpectralMagnitudePair::new(12, 12));
        assert_eq!(table[168].code, HuffmanCode::new(0x7fff, 15).unwrap());
    }

    #[test]
    fn exposes_full_aac_unsigned_pairs10_table() {
        let table = aac_unsigned_pairs10_table();

        assert_eq!(table.len(), 169);
        assert_eq!(table[0].symbol, AacSpectralMagnitudePair::new(0, 0));
        assert_eq!(table[0].code, HuffmanCode::new(0x022, 6).unwrap());
        assert_eq!(table[14].symbol, AacSpectralMagnitudePair::new(1, 1));
        assert_eq!(table[14].code, HuffmanCode::new(0x000, 4).unwrap());
        assert_eq!(table[168].symbol, AacSpectralMagnitudePair::new(12, 12));
        assert_eq!(table[168].code, HuffmanCode::new(0xfff, 12).unwrap());
    }

    #[test]
    fn exposes_standard_aac_escape_table() {
        let table = aac_escape_table();

        assert_eq!(table.len(), 289);
        assert_eq!(table[0].symbol, AacSpectralMagnitudePair::new(0, 0));
        assert_eq!(table[0].code, HuffmanCode::new(0x000, 4).unwrap());
        assert_eq!(table[18].symbol, AacSpectralMagnitudePair::new(1, 1));
        assert_eq!(table[18].code, HuffmanCode::new(0x001, 4).unwrap());
        assert_eq!(table[288].symbol, AacSpectralMagnitudePair::new(16, 16));
        assert_eq!(table[288].code, HuffmanCode::new(0x004, 5).unwrap());
        assert_eq!(
            pack_spectral_pairs_with_sign_bits(&[AacSpectralPair::new(-17, 0)], table).unwrap(),
            PackedBits {
                bytes: vec![0b1110_0001, 0b0100_0010],
                bit_len: 15,
            }
        );
        assert_eq!(
            pack_sectioned_spectral_payload_with_sign_bits_by_bit_cost(
                &[12, -12],
                2,
                AacSpectralMagnitudeTables {
                    pairs1: &[],
                    pairs5: &[],
                    pairs6: &[],
                    escape: table,
                },
            )
            .unwrap(),
            PackedBits {
                bytes: vec![0b1011_0000, 0b1111_1110, 0b0111_0100],
                bit_len: 22,
            }
        );
    }

    #[test]
    fn standard_aac_lc_spectral_tables_include_escape_codebook() {
        let tables = aac_lc_standard_spectral_tables();

        assert_eq!(
            select_codebook_by_bit_cost(&[17, 0], tables).unwrap(),
            AacCodebook::Escape
        );
        assert_eq!(
            pack_sectioned_spectral_payload_with_sign_bits_by_bit_cost(&[17, 0], 2, tables)
                .unwrap()
                .bit_len,
            24
        );
    }

    #[test]
    fn packs_aac_escape_spectral_pairs_with_suffix_bits() {
        let table = [
            HuffmanEntry {
                symbol: AacSpectralMagnitudePair::new(16, 0),
                code: HuffmanCode::new(0b10, 2).unwrap(),
            },
            HuffmanEntry {
                symbol: AacSpectralMagnitudePair::new(16, 16),
                code: HuffmanCode::new(0b11, 2).unwrap(),
            },
        ];

        assert_eq!(
            super::pack_spectral_pairs_with_sign_bits(&[AacSpectralPair::new(-17, 0)], &table)
                .unwrap(),
            PackedBits {
                bytes: vec![0b1010_0001],
                bit_len: 8,
            }
        );
        assert_eq!(
            super::pack_spectral_pairs_with_sign_bits(&[AacSpectralPair::new(32, -18)], &table)
                .unwrap(),
            PackedBits {
                bytes: vec![0b1101_1000, 0b0000_0010],
                bit_len: 16,
            }
        );
    }

    #[test]
    fn packs_aac_spectral_sections_with_sign_bits() {
        let quantized = [0, 0, 1, -1, 3, 0, -2, 2];
        let sections = vec![
            AacSection {
                start: 0,
                end: 2,
                codebook: AacCodebook::Zero,
            },
            AacSection {
                start: 2,
                end: 4,
                codebook: AacCodebook::SignedPairs1,
            },
            AacSection {
                start: 4,
                end: 8,
                codebook: AacCodebook::SignedPairs5,
            },
        ];
        let pairs1 = [HuffmanEntry {
            symbol: AacSpectralMagnitudePair::new(1, 1),
            code: HuffmanCode::new(0b10, 2).unwrap(),
        }];
        let pairs5 = [
            HuffmanEntry {
                symbol: AacSpectralMagnitudePair::new(3, 0),
                code: HuffmanCode::new(0b0, 1).unwrap(),
            },
            HuffmanEntry {
                symbol: AacSpectralMagnitudePair::new(2, 2),
                code: HuffmanCode::new(0b11, 2).unwrap(),
            },
        ];

        let packed = pack_spectral_sections_with_sign_bits(
            &sections,
            &quantized,
            AacSpectralMagnitudeTables {
                pairs1: &pairs1,
                pairs5: &pairs5,
                pairs6: &[],
                escape: &[],
            },
        )
        .unwrap();

        assert_eq!(
            packed,
            PackedBits {
                bytes: vec![0b1001_0011, 0b1000_0000],
                bit_len: 10,
            }
        );
    }

    #[test]
    fn packs_aac_spectral_sections_from_tables() {
        let quantized = [0, 0, 1, -1, 3, 0, -2, 2];
        let sections = vec![
            AacSection {
                start: 0,
                end: 2,
                codebook: AacCodebook::Zero,
            },
            AacSection {
                start: 2,
                end: 4,
                codebook: AacCodebook::SignedPairs1,
            },
            AacSection {
                start: 4,
                end: 8,
                codebook: AacCodebook::SignedPairs5,
            },
        ];
        let signed_pairs1 = [HuffmanEntry {
            symbol: AacSpectralPair::new(1, -1),
            code: HuffmanCode::new(0b10, 2).unwrap(),
        }];
        let signed_pairs5 = [
            HuffmanEntry {
                symbol: AacSpectralPair::new(3, 0),
                code: HuffmanCode::new(0b0, 1).unwrap(),
            },
            HuffmanEntry {
                symbol: AacSpectralPair::new(-2, 2),
                code: HuffmanCode::new(0b11, 2).unwrap(),
            },
        ];

        let packed = pack_spectral_sections(
            &sections,
            &quantized,
            AacSpectralTables {
                signed_pairs1: &signed_pairs1,
                signed_pairs5: &signed_pairs5,
                signed_pairs6: &[],
                escape: &[],
            },
        )
        .unwrap();

        assert_eq!(
            packed,
            PackedBits {
                bytes: vec![0b1001_1000],
                bit_len: 5,
            }
        );
    }

    #[test]
    fn packs_aac_sectioned_spectral_payload() {
        let quantized = [0, 0, 1, -1, 3, 0, -2, 2];
        let sections = vec![
            AacSection {
                start: 0,
                end: 2,
                codebook: AacCodebook::Zero,
            },
            AacSection {
                start: 2,
                end: 4,
                codebook: AacCodebook::SignedPairs1,
            },
            AacSection {
                start: 4,
                end: 8,
                codebook: AacCodebook::SignedPairs5,
            },
        ];
        let signed_pairs1 = [HuffmanEntry {
            symbol: AacSpectralPair::new(1, -1),
            code: HuffmanCode::new(0b10, 2).unwrap(),
        }];
        let signed_pairs5 = [
            HuffmanEntry {
                symbol: AacSpectralPair::new(3, 0),
                code: HuffmanCode::new(0b0, 1).unwrap(),
            },
            HuffmanEntry {
                symbol: AacSpectralPair::new(-2, 2),
                code: HuffmanCode::new(0b11, 2).unwrap(),
            },
        ];

        let packed = pack_sectioned_spectral_payload(
            &sections,
            &quantized,
            2,
            AacSpectralTables {
                signed_pairs1: &signed_pairs1,
                signed_pairs5: &signed_pairs5,
                signed_pairs6: &[],
                escape: &[],
            },
        )
        .unwrap();

        assert_eq!(
            packed,
            PackedBits {
                bytes: vec![0x00, 0x88, 0x54, 0x53],
                bit_len: 32,
            }
        );
        assert!(
            pack_spectral_sections(&sections[1..2], &quantized, AacSpectralTables::default(),)
                .is_err()
        );
    }

    #[test]
    fn packs_aac_sectioned_spectral_payload_with_sign_bits() {
        let quantized = [0, 0, 1, -1, 3, 0, -2, 2];
        let sections = vec![
            AacSection {
                start: 0,
                end: 2,
                codebook: AacCodebook::Zero,
            },
            AacSection {
                start: 2,
                end: 4,
                codebook: AacCodebook::SignedPairs1,
            },
            AacSection {
                start: 4,
                end: 8,
                codebook: AacCodebook::SignedPairs5,
            },
        ];
        let pairs1 = [HuffmanEntry {
            symbol: AacSpectralMagnitudePair::new(1, 1),
            code: HuffmanCode::new(0b10, 2).unwrap(),
        }];
        let pairs5 = [
            HuffmanEntry {
                symbol: AacSpectralMagnitudePair::new(3, 0),
                code: HuffmanCode::new(0b0, 1).unwrap(),
            },
            HuffmanEntry {
                symbol: AacSpectralMagnitudePair::new(2, 2),
                code: HuffmanCode::new(0b11, 2).unwrap(),
            },
        ];

        let packed = pack_sectioned_spectral_payload_with_sign_bits(
            &sections,
            &quantized,
            2,
            AacSpectralMagnitudeTables {
                pairs1: &pairs1,
                pairs5: &pairs5,
                pairs6: &[],
                escape: &[],
            },
        )
        .unwrap();

        assert_eq!(
            packed,
            PackedBits {
                bytes: vec![0x00, 0x88, 0x54, 0x52, 0x70],
                bit_len: 37,
            }
        );
    }

    #[test]
    fn packs_aac_codebook6_sections_from_caller_table() {
        let quantized = [1, -1];
        let sections = vec![AacSection {
            start: 0,
            end: 2,
            codebook: AacCodebook::SignedPairs6,
        }];
        let signed_pairs6 = [HuffmanEntry {
            symbol: AacSpectralPair::new(1, -1),
            code: HuffmanCode::new(0b10, 2).unwrap(),
        }];
        let magnitude_pairs6 = [HuffmanEntry {
            symbol: AacSpectralMagnitudePair::new(1, 1),
            code: HuffmanCode::new(0b0, 1).unwrap(),
        }];

        assert_eq!(
            pack_spectral_sections(
                &sections,
                &quantized,
                AacSpectralTables {
                    signed_pairs1: &[],
                    signed_pairs5: &[],
                    signed_pairs6: &signed_pairs6,
                    escape: &[],
                },
            )
            .unwrap(),
            PackedBits {
                bytes: vec![0b1000_0000],
                bit_len: 2,
            }
        );
        assert_eq!(
            pack_sectioned_spectral_payload_with_sign_bits(
                &sections,
                &quantized,
                2,
                AacSpectralMagnitudeTables {
                    pairs1: &[],
                    pairs5: &[],
                    pairs6: &magnitude_pairs6,
                    escape: &[],
                },
            )
            .unwrap(),
            PackedBits {
                bytes: vec![0b0110_0000, 0b1001_0000],
                bit_len: 12,
            }
        );
    }

    #[test]
    fn converts_and_packs_aac_spectral_quads_with_sign_bits() {
        let quantized = [1, -1, 0, 2, 0, 0, 0, 0];
        let section = AacSection {
            start: 0,
            end: 4,
            codebook: AacCodebook::SignedPairs1,
        };
        let quads = spectral_quads_for_section(&quantized, &section).unwrap();
        let signed_table = [HuffmanEntry {
            symbol: AacSpectralQuad::new(1, -1, 0, 2),
            code: HuffmanCode::new(0b11, 2).unwrap(),
        }];
        let magnitude_table = [HuffmanEntry {
            symbol: AacSpectralMagnitudeQuad::new(1, 1, 0, 2),
            code: HuffmanCode::new(0b101, 3).unwrap(),
        }];

        assert_eq!(quads, vec![AacSpectralQuad::new(1, -1, 0, 2)]);
        assert_eq!(
            spectral_quads_for_section(
                &quantized,
                &AacSection {
                    start: 4,
                    end: 8,
                    codebook: AacCodebook::Zero,
                },
            )
            .unwrap(),
            Vec::<AacSpectralQuad>::new()
        );
        assert_eq!(
            pack_spectral_quads_with_table(&quads, &signed_table).unwrap(),
            PackedBits {
                bytes: vec![0b1100_0000],
                bit_len: 2,
            }
        );
        assert_eq!(
            pack_spectral_quads_with_sign_bits(&quads, &magnitude_table).unwrap(),
            PackedBits {
                bytes: vec![0b1010_1000],
                bit_len: 6,
            }
        );
        assert!(spectral_quads_for_section(
            &quantized,
            &AacSection {
                start: 0,
                end: 2,
                codebook: AacCodebook::SignedPairs1,
            },
        )
        .is_err());
    }

    #[test]
    fn packs_aac_quad_sections_with_sign_bits() {
        let quantized = [1, -1, 0, 1, 0, 1, -1, 0];
        let sections = vec![AacQuadSection {
            start: 0,
            end: 8,
            codebook_id: 3,
        }];
        let table = [
            HuffmanEntry {
                symbol: AacSpectralMagnitudeQuad::new(1, 1, 0, 1),
                code: HuffmanCode::new(0b10, 2).unwrap(),
            },
            HuffmanEntry {
                symbol: AacSpectralMagnitudeQuad::new(0, 1, 1, 0),
                code: HuffmanCode::new(0b11, 2).unwrap(),
            },
        ];
        let tables = AacSpectralMagnitudeQuadTables {
            quads3: &table,
            ..Default::default()
        };

        assert_eq!(
            pack_quad_section_data_with_len(&sections, 4).unwrap(),
            PackedBits {
                bytes: vec![0b0011_0001, 0b0000_0000],
                bit_len: 9,
            }
        );
        assert_eq!(
            pack_spectral_quad_sections_with_sign_bits(&sections, &quantized, tables).unwrap(),
            PackedBits {
                bytes: vec![0b1001_0110, 0b1000_0000],
                bit_len: 9,
            }
        );
        assert_eq!(
            pack_sectioned_spectral_quad_payload_with_sign_bits(&sections, &quantized, 4, tables)
                .unwrap(),
            PackedBits {
                bytes: vec![0b0011_0001, 0b0100_1011, 0b0100_0000],
                bit_len: 18,
            }
        );
        assert!(pack_quad_section_data_with_len(
            &[AacQuadSection {
                start: 0,
                end: 4,
                codebook_id: 5,
            }],
            4,
        )
        .is_err());
    }

    #[test]
    fn packs_aac_sectioned_spectral_payload_with_bit_cost_sections() {
        let quantized = [1, -1, 0, 0];
        let pairs1 = [HuffmanEntry {
            symbol: AacSpectralMagnitudePair::new(1, 1),
            code: HuffmanCode::new(0b1111, 4).unwrap(),
        }];
        let pairs5 = [HuffmanEntry {
            symbol: AacSpectralMagnitudePair::new(1, 1),
            code: HuffmanCode::new(0b0, 1).unwrap(),
        }];
        let tables = AacSpectralMagnitudeTables {
            pairs1: &pairs1,
            pairs5: &pairs5,
            pairs6: &[],
            escape: &[],
        };
        let expected_sections = vec![
            AacSection {
                start: 0,
                end: 2,
                codebook: AacCodebook::SignedPairs5,
            },
            AacSection {
                start: 2,
                end: 4,
                codebook: AacCodebook::Zero,
            },
        ];
        let expected = pack_sectioned_spectral_payload_with_sign_bits(
            &expected_sections,
            &quantized,
            2,
            tables,
        )
        .unwrap();

        let packed =
            pack_sectioned_spectral_payload_with_sign_bits_by_bit_cost(&quantized, 2, tables)
                .unwrap();
        let with_scale_factors =
            pack_sectioned_spectral_payload_with_sign_bits_and_scale_factor_bits_by_bit_cost(
                &quantized,
                2,
                PackedBits {
                    bytes: vec![0b1000_0000],
                    bit_len: 1,
                },
                tables,
            )
            .unwrap();

        assert_eq!(packed, expected);
        assert_eq!(packed.bit_len, 21);
        assert_eq!(with_scale_factors.bit_len, 22);
    }

    #[test]
    fn packs_aac_sectioned_escape_spectral_payload_with_sign_bits() {
        let quantized = [0, 0, 17, 0];
        let sections = plan_sections(&quantized, 2).unwrap();
        let escape = [HuffmanEntry {
            symbol: AacSpectralMagnitudePair::new(16, 0),
            code: HuffmanCode::new(0b10, 2).unwrap(),
        }];

        let packed = pack_sectioned_spectral_payload_with_sign_bits(
            &sections,
            &quantized,
            2,
            AacSpectralMagnitudeTables {
                pairs1: &[],
                pairs5: &[],
                pairs6: &[],
                escape: &escape,
            },
        )
        .unwrap();

        assert_eq!(
            packed,
            PackedBits {
                bytes: vec![0x00, 0xd8, 0x60, 0x40],
                bit_len: 26,
            }
        );
    }

    #[test]
    fn packs_aac_channel_payload_parts_with_scale_factor_bits() {
        let section_bits = PackedBits {
            bytes: vec![0x00, 0x42, 0x12, 0x88],
            bit_len: 30,
        };
        let scale_factor_bits = PackedBits {
            bytes: vec![0b1010_0000],
            bit_len: 3,
        };
        let spectral_bits = PackedBits {
            bytes: vec![0b1001_0011, 0b1000_0000],
            bit_len: 10,
        };

        assert_eq!(
            pack_channel_payload_parts(section_bits, scale_factor_bits, spectral_bits).unwrap(),
            PackedBits {
                bytes: vec![0x00, 0x42, 0x12, 0x8a, 0xc9, 0xc0],
                bit_len: 43,
            }
        );
    }

    #[test]
    fn packs_aac_sectioned_payload_with_scale_factor_bits() {
        let quantized = [0, 0, 1, -1, 3, 0, -2, 2];
        let sections = vec![
            AacSection {
                start: 0,
                end: 2,
                codebook: AacCodebook::Zero,
            },
            AacSection {
                start: 2,
                end: 4,
                codebook: AacCodebook::SignedPairs1,
            },
            AacSection {
                start: 4,
                end: 8,
                codebook: AacCodebook::SignedPairs5,
            },
        ];
        let scale_factor_bits = PackedBits {
            bytes: vec![0b1010_0000],
            bit_len: 3,
        };
        let signed_pairs1 = [HuffmanEntry {
            symbol: AacSpectralPair::new(1, -1),
            code: HuffmanCode::new(0b10, 2).unwrap(),
        }];
        let signed_pairs5 = [
            HuffmanEntry {
                symbol: AacSpectralPair::new(3, 0),
                code: HuffmanCode::new(0b0, 1).unwrap(),
            },
            HuffmanEntry {
                symbol: AacSpectralPair::new(-2, 2),
                code: HuffmanCode::new(0b11, 2).unwrap(),
            },
        ];
        let pairs1 = [HuffmanEntry {
            symbol: AacSpectralMagnitudePair::new(1, 1),
            code: HuffmanCode::new(0b10, 2).unwrap(),
        }];
        let pairs5 = [
            HuffmanEntry {
                symbol: AacSpectralMagnitudePair::new(3, 0),
                code: HuffmanCode::new(0b0, 1).unwrap(),
            },
            HuffmanEntry {
                symbol: AacSpectralMagnitudePair::new(2, 2),
                code: HuffmanCode::new(0b11, 2).unwrap(),
            },
        ];

        assert_eq!(
            pack_sectioned_spectral_payload_with_scale_factor_bits(
                &sections,
                &quantized,
                2,
                scale_factor_bits.clone(),
                AacSpectralTables {
                    signed_pairs1: &signed_pairs1,
                    signed_pairs5: &signed_pairs5,
                    signed_pairs6: &[],
                    escape: &[],
                },
            )
            .unwrap(),
            PackedBits {
                bytes: vec![0x00, 0x88, 0x54, 0x56, 0x60],
                bit_len: 35,
            }
        );
        assert_eq!(
            pack_sectioned_spectral_payload_with_sign_bits_and_scale_factor_bits(
                &sections,
                &quantized,
                2,
                scale_factor_bits,
                AacSpectralMagnitudeTables {
                    pairs1: &pairs1,
                    pairs5: &pairs5,
                    pairs6: &[],
                    escape: &[],
                },
            )
            .unwrap(),
            PackedBits {
                bytes: vec![0x00, 0x88, 0x54, 0x56, 0x4e],
                bit_len: 40,
            }
        );
    }

    #[test]
    fn packs_long_block_individual_channel_stream_with_payload_bits() {
        let payload = PackedBits {
            bytes: vec![0b1010_0000],
            bit_len: 3,
        };

        let ics =
            pack_long_block_individual_channel_stream(AacLongBlockConfig::new(120, 3), &payload)
                .unwrap();

        assert_eq!(
            ics,
            PackedBits {
                bytes: vec![0x78, 0x00, 0xd4, 0x00],
                bit_len: 25,
            }
        );
        assert!(pack_long_block_individual_channel_stream(
            AacLongBlockConfig::new(120, 0),
            &payload,
        )
        .is_err());
    }

    #[test]
    fn packs_single_channel_raw_data_block_from_ics_payload() {
        let empty_payload = PackedBits {
            bytes: Vec::new(),
            bit_len: 0,
        };
        let payload = PackedBits {
            bytes: vec![0b1010_0000],
            bit_len: 3,
        };

        assert_eq!(
            pack_single_channel_raw_data_block(AacLongBlockConfig::new(0, 0), &empty_payload)
                .unwrap(),
            [0x00, 0x00, 0x00, 0x07]
        );
        assert_eq!(
            pack_single_channel_raw_data_block(AacLongBlockConfig::new(120, 3), &payload).unwrap(),
            [0x00, 0xf0, 0x01, 0xa8, 0xe0]
        );
    }

    #[test]
    fn packs_channel_pair_raw_data_block_from_ics_payloads() {
        let empty_payload = PackedBits {
            bytes: Vec::new(),
            bit_len: 0,
        };
        let left_payload = PackedBits {
            bytes: vec![0b1010_0000],
            bit_len: 3,
        };
        let right_payload = PackedBits {
            bytes: vec![0b0100_0000],
            bit_len: 2,
        };

        assert_eq!(
            pack_channel_pair_raw_data_block(
                AacLongBlockConfig::new(0, 0),
                &empty_payload,
                AacLongBlockConfig::new(0, 0),
                &empty_payload,
            )
            .unwrap(),
            [0x20, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0e]
        );
        assert_eq!(
            pack_channel_pair_raw_data_block(
                AacLongBlockConfig::new(120, 3),
                &left_payload,
                AacLongBlockConfig::new(64, 2),
                &right_payload,
            )
            .unwrap(),
            [0x20, 0x78, 0x00, 0xd4, 0x20, 0x00, 0x44, 0x70]
        );
    }

    #[test]
    fn encodes_quantized_mono_long_block_as_adts() {
        let quantized = [0, 0, 1, -1, 3, 0, -2, 2];
        let pairs1 = [HuffmanEntry {
            symbol: AacSpectralMagnitudePair::new(1, 1),
            code: HuffmanCode::new(0b10, 2).unwrap(),
        }];
        let pairs5 = [
            HuffmanEntry {
                symbol: AacSpectralMagnitudePair::new(3, 0),
                code: HuffmanCode::new(0b0, 1).unwrap(),
            },
            HuffmanEntry {
                symbol: AacSpectralMagnitudePair::new(2, 2),
                code: HuffmanCode::new(0b11, 2).unwrap(),
            },
        ];

        let adts = encode_quantized_mono_adts(
            AdtsConfig::aac_lc(44_100, 1),
            AacLongBlockConfig::new(120, 3),
            &quantized,
            2,
            AacSpectralMagnitudeTables {
                pairs1: &pairs1,
                pairs5: &pairs5,
                pairs6: &[],
                escape: &[],
            },
        )
        .unwrap();

        assert_eq!(
            adts,
            [
                0xff, 0xf1, 0x50, 0x40, 0x02, 0x3f, 0xfc, 0x00, 0xf0, 0x01, 0x80, 0x2e, 0x31, 0x8f,
                0x37, 0x2b, 0x80,
            ]
        );
        assert!(encode_quantized_mono_adts(
            AdtsConfig::aac_lc(44_100, 2),
            AacLongBlockConfig::new(120, 3),
            &quantized,
            2,
            AacSpectralMagnitudeTables {
                pairs1: &pairs1,
                pairs5: &pairs5,
                pairs6: &[],
                escape: &[],
            },
        )
        .is_err());
    }

    #[test]
    fn encodes_quantized_mono_escape_long_block_as_adts() {
        let quantized = [0, 0, 17, 0];
        let escape = [HuffmanEntry {
            symbol: AacSpectralMagnitudePair::new(16, 0),
            code: HuffmanCode::new(0b10, 2).unwrap(),
        }];
        let sections = plan_sections(&quantized, 2).unwrap();
        let payload = split_sectioned_spectral_payload_with_sign_bits(
            &sections,
            &quantized,
            2,
            AacSpectralMagnitudeTables {
                pairs1: &[],
                pairs5: &[],
                pairs6: &[],
                escape: &escape,
            },
        )
        .unwrap();
        let access_unit =
            pack_single_channel_raw_data_block_parts(AacLongBlockConfig::new(120, 2), &payload)
                .unwrap();

        let adts = encode_quantized_mono_adts(
            AdtsConfig::aac_lc(44_100, 1),
            AacLongBlockConfig::new(120, 2),
            &quantized,
            2,
            AacSpectralMagnitudeTables {
                pairs1: &[],
                pairs5: &[],
                pairs6: &[],
                escape: &escape,
            },
        )
        .unwrap();

        assert_eq!(&adts[..2], &[0xff, 0xf1]);
        assert_eq!(&adts[7..], access_unit);
        assert!(encode_quantized_mono_adts(
            AdtsConfig::aac_lc(44_100, 1),
            AacLongBlockConfig::new(120, 2),
            &quantized,
            2,
            AacSpectralMagnitudeTables::default(),
        )
        .is_err());
    }

    #[test]
    fn encodes_quantized_mono_long_block_with_scale_factors_as_adts() {
        let quantized = [0, 0, 1, -1, 3, 0, -2, 2];
        let scale_factor_table = [
            HuffmanEntry {
                symbol: AacScaleFactorDelta::new(0),
                code: HuffmanCode::new(0b111, 3).unwrap(),
            },
            HuffmanEntry {
                symbol: AacScaleFactorDelta::new(-1),
                code: HuffmanCode::new(0b0, 1).unwrap(),
            },
            HuffmanEntry {
                symbol: AacScaleFactorDelta::new(1),
                code: HuffmanCode::new(0b10, 2).unwrap(),
            },
            HuffmanEntry {
                symbol: AacScaleFactorDelta::new(2),
                code: HuffmanCode::new(0b110, 3).unwrap(),
            },
        ];
        let pairs1 = [HuffmanEntry {
            symbol: AacSpectralMagnitudePair::new(1, 1),
            code: HuffmanCode::new(0b10, 2).unwrap(),
        }];
        let pairs5 = [
            HuffmanEntry {
                symbol: AacSpectralMagnitudePair::new(3, 0),
                code: HuffmanCode::new(0b0, 1).unwrap(),
            },
            HuffmanEntry {
                symbol: AacSpectralMagnitudePair::new(2, 2),
                code: HuffmanCode::new(0b11, 2).unwrap(),
            },
        ];

        let adts = encode_quantized_mono_adts_with_scale_factors(
            AdtsConfig::aac_lc(44_100, 1),
            AacLongBlockConfig::new(120, 3),
            &quantized,
            2,
            &[119, 121, 123, 122],
            &scale_factor_table,
            AacSpectralMagnitudeTables {
                pairs1: &pairs1,
                pairs5: &pairs5,
                pairs6: &[],
                escape: &[],
            },
        )
        .unwrap();

        assert_eq!(
            adts,
            [
                0xff, 0xf1, 0x50, 0x40, 0x02, 0x3f, 0xfc, 0x00, 0xf0, 0x01, 0x80, 0x2e, 0x3b, 0x06,
                0x3c, 0xdc, 0xae,
            ]
        );
        assert!(encode_quantized_mono_adts_with_scale_factors(
            AdtsConfig::aac_lc(44_100, 1),
            AacLongBlockConfig::new(120, 3),
            &quantized,
            2,
            &[119, 121],
            &scale_factor_table,
            AacSpectralMagnitudeTables {
                pairs1: &pairs1,
                pairs5: &pairs5,
                pairs6: &[],
                escape: &[],
            },
        )
        .is_err());
    }

    #[test]
    fn encodes_quantized_mono_long_block_with_selected_scale_factors_as_adts() {
        let quantized = [0, 0, 1, -1, 3, 0, -2, 2];
        let scale_factor_table = [
            HuffmanEntry {
                symbol: AacScaleFactorDelta::new(0),
                code: HuffmanCode::new(0b0, 1).unwrap(),
            },
            HuffmanEntry {
                symbol: AacScaleFactorDelta::new(1),
                code: HuffmanCode::new(0b10, 2).unwrap(),
            },
        ];
        let pairs1 = [HuffmanEntry {
            symbol: AacSpectralMagnitudePair::new(1, 1),
            code: HuffmanCode::new(0b10, 2).unwrap(),
        }];
        let pairs5 = [
            HuffmanEntry {
                symbol: AacSpectralMagnitudePair::new(3, 0),
                code: HuffmanCode::new(0b0, 1).unwrap(),
            },
            HuffmanEntry {
                symbol: AacSpectralMagnitudePair::new(2, 2),
                code: HuffmanCode::new(0b11, 2).unwrap(),
            },
        ];
        let tables = AacSpectralMagnitudeTables {
            pairs1: &pairs1,
            pairs5: &pairs5,
            pairs6: &[],
            escape: &[],
        };

        let selected = encode_quantized_mono_adts_with_selected_scale_factors(
            AdtsConfig::aac_lc(44_100, 1),
            AacLongBlockConfig::new(120, 3),
            &quantized,
            2,
            &scale_factor_table,
            tables,
        )
        .unwrap();
        let manual = encode_quantized_mono_adts_with_scale_factors(
            AdtsConfig::aac_lc(44_100, 1),
            AacLongBlockConfig::new(120, 3),
            &quantized,
            2,
            &[120, 121, 122, 122],
            &scale_factor_table,
            tables,
        )
        .unwrap();

        assert_eq!(selected, manual);
        assert!(encode_quantized_mono_adts_with_selected_scale_factors(
            AdtsConfig::aac_lc(44_100, 2),
            AacLongBlockConfig::new(120, 3),
            &quantized,
            2,
            &scale_factor_table,
            tables,
        )
        .is_err());
    }

    #[test]
    fn encodes_quantized_mono_long_block_with_bit_cost_sections_as_adts() {
        let quantized = [1, -1, 0, 0];
        let pairs1 = [HuffmanEntry {
            symbol: AacSpectralMagnitudePair::new(1, 1),
            code: HuffmanCode::new(0b1111, 4).unwrap(),
        }];
        let pairs5 = [HuffmanEntry {
            symbol: AacSpectralMagnitudePair::new(1, 1),
            code: HuffmanCode::new(0b0, 1).unwrap(),
        }];
        let tables = AacSpectralMagnitudeTables {
            pairs1: &pairs1,
            pairs5: &pairs5,
            pairs6: &[],
            escape: &[],
        };
        let expected_sections = vec![
            AacSection {
                start: 0,
                end: 2,
                codebook: AacCodebook::SignedPairs5,
            },
            AacSection {
                start: 2,
                end: 4,
                codebook: AacCodebook::Zero,
            },
        ];
        let expected_payload = split_sectioned_spectral_payload_with_sign_bits(
            &expected_sections,
            &quantized,
            2,
            tables,
        )
        .unwrap();
        let expected_access_unit = pack_single_channel_raw_data_block_parts(
            AacLongBlockConfig::new(120, 2),
            &expected_payload,
        )
        .unwrap();
        let expected = frame_adts(AdtsConfig::aac_lc(44_100, 1), &expected_access_unit).unwrap();

        let encoded = encode_quantized_mono_adts_by_bit_cost(
            AdtsConfig::aac_lc(44_100, 1),
            AacLongBlockConfig::new(120, 2),
            &quantized,
            2,
            tables,
        )
        .unwrap();
        let with_scale_factors = encode_quantized_mono_adts_with_scale_factors_by_bit_cost(
            AdtsConfig::aac_lc(44_100, 1),
            AacLongBlockConfig::new(120, 2),
            &quantized,
            2,
            &[120, 120],
            &[HuffmanEntry {
                symbol: AacScaleFactorDelta::new(0),
                code: HuffmanCode::new(0b0, 1).unwrap(),
            }],
            tables,
        )
        .unwrap();
        let selected = encode_quantized_mono_adts_with_selected_scale_factors_by_bit_cost(
            AdtsConfig::aac_lc(44_100, 1),
            AacLongBlockConfig::new(120, 2),
            &quantized,
            2,
            &[
                HuffmanEntry {
                    symbol: AacScaleFactorDelta::new(0),
                    code: HuffmanCode::new(0b0, 1).unwrap(),
                },
                HuffmanEntry {
                    symbol: AacScaleFactorDelta::new(1),
                    code: HuffmanCode::new(0b10, 2).unwrap(),
                },
            ],
            tables,
        )
        .unwrap();

        assert_eq!(encoded, expected);
        assert_eq!(&with_scale_factors[..2], &[0xff, 0xf1]);
        assert_eq!(&selected[..2], &[0xff, 0xf1]);
        assert!(encode_quantized_mono_adts_by_bit_cost(
            AdtsConfig::aac_lc(44_100, 2),
            AacLongBlockConfig::new(120, 2),
            &quantized,
            2,
            tables,
        )
        .is_err());
    }

    #[test]
    fn encodes_quantized_stereo_long_blocks_as_adts() {
        let left_quantized = [0, 0, 1, -1, 3, 0, -2, 2];
        let right_quantized = [0, 0, -1, 1, 3, 0, 2, -2];
        let pairs1 = [HuffmanEntry {
            symbol: AacSpectralMagnitudePair::new(1, 1),
            code: HuffmanCode::new(0b10, 2).unwrap(),
        }];
        let pairs5 = [
            HuffmanEntry {
                symbol: AacSpectralMagnitudePair::new(3, 0),
                code: HuffmanCode::new(0b0, 1).unwrap(),
            },
            HuffmanEntry {
                symbol: AacSpectralMagnitudePair::new(2, 2),
                code: HuffmanCode::new(0b11, 2).unwrap(),
            },
        ];

        let adts = encode_quantized_stereo_adts(
            AdtsConfig::aac_lc(44_100, 2),
            AacLongBlockConfig::new(120, 3),
            &left_quantized,
            AacLongBlockConfig::new(100, 3),
            &right_quantized,
            2,
            AacSpectralMagnitudeTables {
                pairs1: &pairs1,
                pairs5: &pairs5,
                pairs6: &[],
                escape: &[],
            },
        )
        .unwrap();

        assert_eq!(
            adts,
            [
                0xff, 0xf1, 0x50, 0x80, 0x03, 0x3f, 0xfc, 0x20, 0x78, 0x00, 0xc0, 0x17, 0x18, 0xc7,
                0x9b, 0x94, 0xc8, 0x01, 0x80, 0x2e, 0x31, 0x97, 0x37, 0x27, 0x80,
            ]
        );
        assert!(encode_quantized_stereo_adts(
            AdtsConfig::aac_lc(44_100, 1),
            AacLongBlockConfig::new(120, 3),
            &left_quantized,
            AacLongBlockConfig::new(100, 3),
            &right_quantized,
            2,
            AacSpectralMagnitudeTables {
                pairs1: &pairs1,
                pairs5: &pairs5,
                pairs6: &[],
                escape: &[],
            },
        )
        .is_err());
    }

    #[test]
    fn encodes_quantized_stereo_long_blocks_with_scale_factors_as_adts() {
        let left_quantized = [0, 0, 1, -1, 3, 0, -2, 2];
        let right_quantized = [0, 0, -1, 1, 3, 0, 2, -2];
        let scale_factor_table = [
            HuffmanEntry {
                symbol: AacScaleFactorDelta::new(-1),
                code: HuffmanCode::new(0b0, 1).unwrap(),
            },
            HuffmanEntry {
                symbol: AacScaleFactorDelta::new(1),
                code: HuffmanCode::new(0b10, 2).unwrap(),
            },
            HuffmanEntry {
                symbol: AacScaleFactorDelta::new(2),
                code: HuffmanCode::new(0b110, 3).unwrap(),
            },
        ];
        let pairs1 = [HuffmanEntry {
            symbol: AacSpectralMagnitudePair::new(1, 1),
            code: HuffmanCode::new(0b10, 2).unwrap(),
        }];
        let pairs5 = [
            HuffmanEntry {
                symbol: AacSpectralMagnitudePair::new(3, 0),
                code: HuffmanCode::new(0b0, 1).unwrap(),
            },
            HuffmanEntry {
                symbol: AacSpectralMagnitudePair::new(2, 2),
                code: HuffmanCode::new(0b11, 2).unwrap(),
            },
        ];

        let adts = encode_quantized_stereo_adts_with_scale_factors(
            AdtsConfig::aac_lc(44_100, 2),
            AacQuantizedChannel::new(
                AacLongBlockConfig::new(120, 3),
                &left_quantized,
                &[119, 121, 123, 122],
            ),
            AacQuantizedChannel::new(
                AacLongBlockConfig::new(100, 3),
                &right_quantized,
                &[99, 101, 103, 102],
            ),
            2,
            &scale_factor_table,
            AacSpectralMagnitudeTables {
                pairs1: &pairs1,
                pairs5: &pairs5,
                pairs6: &[],
                escape: &[],
            },
        )
        .unwrap();

        assert_eq!(
            adts,
            [
                0xff, 0xf1, 0x50, 0x80, 0x03, 0x5f, 0xfc, 0x20, 0x78, 0x00, 0xc0, 0x17, 0x1d, 0x83,
                0x1e, 0x6e, 0x53, 0x20, 0x06, 0x00, 0xb8, 0xec, 0x19, 0x73, 0x72, 0x78,
            ]
        );
        assert!(encode_quantized_stereo_adts_with_scale_factors(
            AdtsConfig::aac_lc(44_100, 1),
            AacQuantizedChannel::new(
                AacLongBlockConfig::new(120, 3),
                &left_quantized,
                &[119, 121, 123, 122],
            ),
            AacQuantizedChannel::new(
                AacLongBlockConfig::new(100, 3),
                &right_quantized,
                &[99, 101, 103, 102],
            ),
            2,
            &scale_factor_table,
            AacSpectralMagnitudeTables {
                pairs1: &pairs1,
                pairs5: &pairs5,
                pairs6: &[],
                escape: &[],
            },
        )
        .is_err());
    }

    #[test]
    fn encodes_quantized_stereo_long_blocks_with_selected_scale_factors_as_adts() {
        let left_quantized = [0, 0, 1, -1, 3, 0, -2, 2];
        let right_quantized = [0, 0, -1, 1, 3, 0, 2, -2];
        let scale_factor_table = [
            HuffmanEntry {
                symbol: AacScaleFactorDelta::new(0),
                code: HuffmanCode::new(0b0, 1).unwrap(),
            },
            HuffmanEntry {
                symbol: AacScaleFactorDelta::new(1),
                code: HuffmanCode::new(0b10, 2).unwrap(),
            },
        ];
        let pairs1 = [HuffmanEntry {
            symbol: AacSpectralMagnitudePair::new(1, 1),
            code: HuffmanCode::new(0b10, 2).unwrap(),
        }];
        let pairs5 = [
            HuffmanEntry {
                symbol: AacSpectralMagnitudePair::new(3, 0),
                code: HuffmanCode::new(0b0, 1).unwrap(),
            },
            HuffmanEntry {
                symbol: AacSpectralMagnitudePair::new(2, 2),
                code: HuffmanCode::new(0b11, 2).unwrap(),
            },
        ];
        let tables = AacSpectralMagnitudeTables {
            pairs1: &pairs1,
            pairs5: &pairs5,
            pairs6: &[],
            escape: &[],
        };

        let selected = encode_quantized_stereo_adts_with_selected_scale_factors(
            AdtsConfig::aac_lc(44_100, 2),
            AacQuantizedSpectrum::new(AacLongBlockConfig::new(120, 3), &left_quantized),
            AacQuantizedSpectrum::new(AacLongBlockConfig::new(100, 3), &right_quantized),
            2,
            &scale_factor_table,
            tables,
        )
        .unwrap();
        let manual = encode_quantized_stereo_adts_with_scale_factors(
            AdtsConfig::aac_lc(44_100, 2),
            AacQuantizedChannel::new(
                AacLongBlockConfig::new(120, 3),
                &left_quantized,
                &[120, 121, 122, 122],
            ),
            AacQuantizedChannel::new(
                AacLongBlockConfig::new(100, 3),
                &right_quantized,
                &[100, 101, 102, 102],
            ),
            2,
            &scale_factor_table,
            tables,
        )
        .unwrap();

        assert_eq!(selected, manual);
        assert!(encode_quantized_stereo_adts_with_selected_scale_factors(
            AdtsConfig::aac_lc(44_100, 1),
            AacQuantizedSpectrum::new(AacLongBlockConfig::new(120, 3), &left_quantized),
            AacQuantizedSpectrum::new(AacLongBlockConfig::new(100, 3), &right_quantized),
            2,
            &scale_factor_table,
            tables,
        )
        .is_err());
    }

    #[test]
    fn encodes_quantized_stereo_long_blocks_with_bit_cost_sections_as_adts() {
        let left_quantized = [1, -1, 0, 0];
        let right_quantized = [-1, 1, 0, 0];
        let pairs1 = [HuffmanEntry {
            symbol: AacSpectralMagnitudePair::new(1, 1),
            code: HuffmanCode::new(0b1111, 4).unwrap(),
        }];
        let pairs5 = [HuffmanEntry {
            symbol: AacSpectralMagnitudePair::new(1, 1),
            code: HuffmanCode::new(0b0, 1).unwrap(),
        }];
        let tables = AacSpectralMagnitudeTables {
            pairs1: &pairs1,
            pairs5: &pairs5,
            pairs6: &[],
            escape: &[],
        };
        let sections = vec![
            AacSection {
                start: 0,
                end: 2,
                codebook: AacCodebook::SignedPairs5,
            },
            AacSection {
                start: 2,
                end: 4,
                codebook: AacCodebook::Zero,
            },
        ];
        let left_payload =
            split_sectioned_spectral_payload_with_sign_bits(&sections, &left_quantized, 2, tables)
                .unwrap();
        let right_payload =
            split_sectioned_spectral_payload_with_sign_bits(&sections, &right_quantized, 2, tables)
                .unwrap();
        let expected_access_unit = pack_channel_pair_raw_data_block_parts(
            AacLongBlockConfig::new(120, 2),
            &left_payload,
            AacLongBlockConfig::new(100, 2),
            &right_payload,
        )
        .unwrap();
        let expected = frame_adts(AdtsConfig::aac_lc(44_100, 2), &expected_access_unit).unwrap();

        let encoded = encode_quantized_stereo_adts_by_bit_cost(
            AdtsConfig::aac_lc(44_100, 2),
            AacLongBlockConfig::new(120, 2),
            &left_quantized,
            AacLongBlockConfig::new(100, 2),
            &right_quantized,
            2,
            tables,
        )
        .unwrap();
        let with_scale_factors = encode_quantized_stereo_adts_with_scale_factors_by_bit_cost(
            AdtsConfig::aac_lc(44_100, 2),
            AacQuantizedChannel::new(
                AacLongBlockConfig::new(120, 2),
                &left_quantized,
                &[120, 120],
            ),
            AacQuantizedChannel::new(
                AacLongBlockConfig::new(100, 2),
                &right_quantized,
                &[100, 100],
            ),
            2,
            &[HuffmanEntry {
                symbol: AacScaleFactorDelta::new(0),
                code: HuffmanCode::new(0b0, 1).unwrap(),
            }],
            tables,
        )
        .unwrap();
        let selected = encode_quantized_stereo_adts_with_selected_scale_factors_by_bit_cost(
            AdtsConfig::aac_lc(44_100, 2),
            AacQuantizedSpectrum::new(AacLongBlockConfig::new(120, 2), &left_quantized),
            AacQuantizedSpectrum::new(AacLongBlockConfig::new(100, 2), &right_quantized),
            2,
            &[
                HuffmanEntry {
                    symbol: AacScaleFactorDelta::new(0),
                    code: HuffmanCode::new(0b0, 1).unwrap(),
                },
                HuffmanEntry {
                    symbol: AacScaleFactorDelta::new(1),
                    code: HuffmanCode::new(0b10, 2).unwrap(),
                },
            ],
            tables,
        )
        .unwrap();

        assert_eq!(encoded, expected);
        assert_eq!(&with_scale_factors[..2], &[0xff, 0xf1]);
        assert_eq!(&selected[..2], &[0xff, 0xf1]);
        assert!(encode_quantized_stereo_adts_by_bit_cost(
            AdtsConfig::aac_lc(44_100, 1),
            AacLongBlockConfig::new(120, 2),
            &left_quantized,
            AacLongBlockConfig::new(100, 2),
            &right_quantized,
            2,
            tables,
        )
        .is_err());
    }

    #[test]
    fn encodes_pcm_mono_long_block_as_adts() {
        let pcm = AudioBuffer::new(44_100, 1, vec![0.0; 2048]).unwrap();

        let adts = encode_pcm_mono_long_block_adts(
            AdtsConfig::aac_lc(44_100, 1),
            AacLongBlockConfig::new(0, 1),
            &pcm,
            AacPcmLongBlockConfig::new(0, 1.0, 1024),
            AacSpectralMagnitudeTables::default(),
        )
        .unwrap();

        assert_eq!(
            adts,
            [0xff, 0xf1, 0x50, 0x40, 0x01, 0xbf, 0xfc, 0x00, 0x00, 0x00, 0x80, 0x23, 0x80,]
        );
        assert!(encode_pcm_mono_long_block_adts(
            AdtsConfig::aac_lc(44_100, 1),
            AacLongBlockConfig::new(0, 1),
            &AudioBuffer::new(44_100, 2, vec![0.0; 4096]).unwrap(),
            AacPcmLongBlockConfig::new(0, 1.0, 1024),
            AacSpectralMagnitudeTables::default(),
        )
        .is_err());
    }

    #[test]
    fn encodes_pcm_mono_long_block_with_scale_factors_as_adts() {
        let pcm = AudioBuffer::new(44_100, 1, vec![0.0; 2048]).unwrap();

        let adts = encode_pcm_mono_long_block_adts_with_scale_factors(
            AdtsConfig::aac_lc(44_100, 1),
            AacScaleFactorChannel::new(AacLongBlockConfig::new(0, 1), &[0]),
            &pcm,
            AacPcmLongBlockConfig::new(0, 1.0, 1024),
            &[],
            AacSpectralMagnitudeTables::default(),
        )
        .unwrap();

        assert_eq!(
            adts,
            [0xff, 0xf1, 0x50, 0x40, 0x01, 0xbf, 0xfc, 0x00, 0x00, 0x00, 0x80, 0x23, 0x80,]
        );
        assert!(encode_pcm_mono_long_block_adts_with_scale_factors(
            AdtsConfig::aac_lc(44_100, 1),
            AacScaleFactorChannel::new(AacLongBlockConfig::new(0, 1), &[0]),
            &AudioBuffer::new(44_100, 2, vec![0.0; 4096]).unwrap(),
            AacPcmLongBlockConfig::new(0, 1.0, 1024),
            &[],
            AacSpectralMagnitudeTables::default(),
        )
        .is_err());
    }

    #[test]
    fn encodes_pcm_mono_long_block_with_selected_scale_factors_as_adts() {
        let pcm = AudioBuffer::new(44_100, 1, vec![0.0; 2048]).unwrap();

        let selected = encode_pcm_mono_long_block_adts_with_selected_scale_factors(
            AdtsConfig::aac_lc(44_100, 1),
            AacLongBlockConfig::new(0, 1),
            &pcm,
            AacPcmLongBlockConfig::new(0, 1.0, 1024),
            &[],
            AacSpectralMagnitudeTables::default(),
        )
        .unwrap();
        let manual = encode_pcm_mono_long_block_adts_with_scale_factors(
            AdtsConfig::aac_lc(44_100, 1),
            AacScaleFactorChannel::new(AacLongBlockConfig::new(0, 1), &[0]),
            &pcm,
            AacPcmLongBlockConfig::new(0, 1.0, 1024),
            &[],
            AacSpectralMagnitudeTables::default(),
        )
        .unwrap();

        assert_eq!(selected, manual);
        assert!(encode_pcm_mono_long_block_adts_with_selected_scale_factors(
            AdtsConfig::aac_lc(44_100, 1),
            AacLongBlockConfig::new(0, 1),
            &AudioBuffer::new(44_100, 2, vec![0.0; 4096]).unwrap(),
            AacPcmLongBlockConfig::new(0, 1.0, 1024),
            &[],
            AacSpectralMagnitudeTables::default(),
        )
        .is_err());
    }

    #[test]
    fn encodes_pcm_mono_long_block_with_bit_cost_sections_as_adts() {
        let pcm = AudioBuffer::new(44_100, 1, vec![0.0; 2048]).unwrap();

        let encoded = encode_pcm_mono_long_block_adts_by_bit_cost(
            AdtsConfig::aac_lc(44_100, 1),
            AacLongBlockConfig::new(0, 1),
            &pcm,
            AacPcmLongBlockConfig::new(0, 1.0, 1024),
            AacSpectralMagnitudeTables::default(),
        )
        .unwrap();
        let with_scale_factors = encode_pcm_mono_long_block_adts_with_scale_factors_by_bit_cost(
            AdtsConfig::aac_lc(44_100, 1),
            AacScaleFactorChannel::new(AacLongBlockConfig::new(0, 1), &[0]),
            &pcm,
            AacPcmLongBlockConfig::new(0, 1.0, 1024),
            &[],
            AacSpectralMagnitudeTables::default(),
        )
        .unwrap();
        let selected = encode_pcm_mono_long_block_adts_with_selected_scale_factors_by_bit_cost(
            AdtsConfig::aac_lc(44_100, 1),
            AacLongBlockConfig::new(0, 1),
            &pcm,
            AacPcmLongBlockConfig::new(0, 1.0, 1024),
            &[],
            AacSpectralMagnitudeTables::default(),
        )
        .unwrap();

        assert_eq!(
            encoded,
            [0xff, 0xf1, 0x50, 0x40, 0x01, 0xbf, 0xfc, 0x00, 0x00, 0x00, 0x80, 0x23, 0x80,]
        );
        assert_eq!(with_scale_factors, encoded);
        assert_eq!(selected, with_scale_factors);
        assert!(encode_pcm_mono_long_block_adts_by_bit_cost(
            AdtsConfig::aac_lc(44_100, 1),
            AacLongBlockConfig::new(0, 1),
            &AudioBuffer::new(44_100, 2, vec![0.0; 4096]).unwrap(),
            AacPcmLongBlockConfig::new(0, 1.0, 1024),
            AacSpectralMagnitudeTables::default(),
        )
        .is_err());
    }

    #[test]
    fn encodes_pcm_stereo_long_block_as_adts() {
        let pcm = AudioBuffer::new(44_100, 2, vec![0.0; 4096]).unwrap();

        let adts = encode_pcm_stereo_long_block_adts(
            AdtsConfig::aac_lc(44_100, 2),
            AacLongBlockConfig::new(0, 1),
            AacLongBlockConfig::new(0, 1),
            &pcm,
            AacPcmLongBlockConfig::new(0, 1.0, 1024),
            AacSpectralMagnitudeTables::default(),
        )
        .unwrap();

        assert_eq!(
            adts,
            [
                0xff, 0xf1, 0x50, 0x80, 0x02, 0x3f, 0xfc, 0x20, 0x00, 0x00, 0x40, 0x10, 0x00, 0x00,
                0x80, 0x23, 0x80,
            ]
        );
        assert!(encode_pcm_stereo_long_block_adts(
            AdtsConfig::aac_lc(44_100, 2),
            AacLongBlockConfig::new(0, 1),
            AacLongBlockConfig::new(0, 1),
            &AudioBuffer::new(44_100, 1, vec![0.0; 2048]).unwrap(),
            AacPcmLongBlockConfig::new(0, 1.0, 1024),
            AacSpectralMagnitudeTables::default(),
        )
        .is_err());
    }

    #[test]
    fn encodes_pcm_stereo_long_block_with_scale_factors_as_adts() {
        let pcm = AudioBuffer::new(44_100, 2, vec![0.0; 4096]).unwrap();

        let adts = encode_pcm_stereo_long_block_adts_with_scale_factors(
            AdtsConfig::aac_lc(44_100, 2),
            AacScaleFactorChannel::new(AacLongBlockConfig::new(0, 1), &[0]),
            AacScaleFactorChannel::new(AacLongBlockConfig::new(0, 1), &[0]),
            &pcm,
            AacPcmLongBlockConfig::new(0, 1.0, 1024),
            &[],
            AacSpectralMagnitudeTables::default(),
        )
        .unwrap();

        assert_eq!(
            adts,
            [
                0xff, 0xf1, 0x50, 0x80, 0x02, 0x3f, 0xfc, 0x20, 0x00, 0x00, 0x40, 0x10, 0x00, 0x00,
                0x80, 0x23, 0x80,
            ]
        );
        assert!(encode_pcm_stereo_long_block_adts_with_scale_factors(
            AdtsConfig::aac_lc(44_100, 2),
            AacScaleFactorChannel::new(AacLongBlockConfig::new(0, 1), &[0]),
            AacScaleFactorChannel::new(AacLongBlockConfig::new(0, 1), &[0]),
            &AudioBuffer::new(44_100, 1, vec![0.0; 2048]).unwrap(),
            AacPcmLongBlockConfig::new(0, 1.0, 1024),
            &[],
            AacSpectralMagnitudeTables::default(),
        )
        .is_err());
    }

    #[test]
    fn encodes_pcm_stereo_long_block_with_selected_scale_factors_as_adts() {
        let pcm = AudioBuffer::new(44_100, 2, vec![0.0; 4096]).unwrap();

        let selected = encode_pcm_stereo_long_block_adts_with_selected_scale_factors(
            AdtsConfig::aac_lc(44_100, 2),
            AacLongBlockConfig::new(0, 1),
            AacLongBlockConfig::new(0, 1),
            &pcm,
            AacPcmLongBlockConfig::new(0, 1.0, 1024),
            &[],
            AacSpectralMagnitudeTables::default(),
        )
        .unwrap();
        let manual = encode_pcm_stereo_long_block_adts_with_scale_factors(
            AdtsConfig::aac_lc(44_100, 2),
            AacScaleFactorChannel::new(AacLongBlockConfig::new(0, 1), &[0]),
            AacScaleFactorChannel::new(AacLongBlockConfig::new(0, 1), &[0]),
            &pcm,
            AacPcmLongBlockConfig::new(0, 1.0, 1024),
            &[],
            AacSpectralMagnitudeTables::default(),
        )
        .unwrap();

        assert_eq!(selected, manual);
        assert!(
            encode_pcm_stereo_long_block_adts_with_selected_scale_factors(
                AdtsConfig::aac_lc(44_100, 2),
                AacLongBlockConfig::new(0, 1),
                AacLongBlockConfig::new(0, 1),
                &AudioBuffer::new(44_100, 1, vec![0.0; 2048]).unwrap(),
                AacPcmLongBlockConfig::new(0, 1.0, 1024),
                &[],
                AacSpectralMagnitudeTables::default(),
            )
            .is_err()
        );
    }

    #[test]
    fn encodes_pcm_stereo_long_block_with_bit_cost_sections_as_adts() {
        let pcm = AudioBuffer::new(44_100, 2, vec![0.0; 4096]).unwrap();

        let encoded = encode_pcm_stereo_long_block_adts_by_bit_cost(
            AdtsConfig::aac_lc(44_100, 2),
            AacLongBlockConfig::new(0, 1),
            AacLongBlockConfig::new(0, 1),
            &pcm,
            AacPcmLongBlockConfig::new(0, 1.0, 1024),
            AacSpectralMagnitudeTables::default(),
        )
        .unwrap();
        let with_scale_factors = encode_pcm_stereo_long_block_adts_with_scale_factors_by_bit_cost(
            AdtsConfig::aac_lc(44_100, 2),
            AacScaleFactorChannel::new(AacLongBlockConfig::new(0, 1), &[0]),
            AacScaleFactorChannel::new(AacLongBlockConfig::new(0, 1), &[0]),
            &pcm,
            AacPcmLongBlockConfig::new(0, 1.0, 1024),
            &[],
            AacSpectralMagnitudeTables::default(),
        )
        .unwrap();
        let selected = encode_pcm_stereo_long_block_adts_with_selected_scale_factors_by_bit_cost(
            AdtsConfig::aac_lc(44_100, 2),
            AacLongBlockConfig::new(0, 1),
            AacLongBlockConfig::new(0, 1),
            &pcm,
            AacPcmLongBlockConfig::new(0, 1.0, 1024),
            &[],
            AacSpectralMagnitudeTables::default(),
        )
        .unwrap();

        assert_eq!(
            encoded,
            [
                0xff, 0xf1, 0x50, 0x80, 0x02, 0x3f, 0xfc, 0x20, 0x00, 0x00, 0x40, 0x10, 0x00, 0x00,
                0x80, 0x23, 0x80,
            ]
        );
        assert_eq!(with_scale_factors, encoded);
        assert_eq!(selected, with_scale_factors);
        assert!(encode_pcm_stereo_long_block_adts_by_bit_cost(
            AdtsConfig::aac_lc(44_100, 2),
            AacLongBlockConfig::new(0, 1),
            AacLongBlockConfig::new(0, 1),
            &AudioBuffer::new(44_100, 1, vec![0.0; 2048]).unwrap(),
            AacPcmLongBlockConfig::new(0, 1.0, 1024),
            AacSpectralMagnitudeTables::default(),
        )
        .is_err());
    }

    #[test]
    fn encodes_pcm_mono_long_block_adts_stream() {
        let pcm = AudioBuffer::new(44_100, 1, vec![0.0; 2048]).unwrap();
        let frame = [
            0xff, 0xf1, 0x50, 0x40, 0x01, 0xbf, 0xfc, 0x00, 0x00, 0x00, 0x80, 0x23, 0x80,
        ];

        let adts = encode_pcm_mono_long_block_adts_stream(
            AdtsConfig::aac_lc(44_100, 1),
            AacLongBlockConfig::new(0, 1),
            &pcm,
            AacPcmLongBlockConfig::new(0, 1.0, 1024),
            AacSpectralMagnitudeTables::default(),
        )
        .unwrap();

        assert_eq!(adts, [frame, frame].concat());
        assert!(encode_pcm_mono_long_block_adts_stream(
            AdtsConfig::aac_lc(44_100, 1),
            AacLongBlockConfig::new(0, 1),
            &pcm,
            AacPcmLongBlockConfig::new(4096, 1.0, 1024),
            AacSpectralMagnitudeTables::default(),
        )
        .is_err());
    }

    #[test]
    fn encodes_pcm_mono_long_block_adts_stream_with_scale_factors() {
        let pcm = AudioBuffer::new(44_100, 1, vec![0.0; 2048]).unwrap();
        let frame = [
            0xff, 0xf1, 0x50, 0x40, 0x01, 0xbf, 0xfc, 0x00, 0x00, 0x00, 0x80, 0x23, 0x80,
        ];
        let scale_factors_by_frame: [&[i16]; 2] = [&[0], &[0]];

        let adts = encode_pcm_mono_long_block_adts_stream_with_scale_factors(
            AdtsConfig::aac_lc(44_100, 1),
            AacScaleFactorSequence::new(AacLongBlockConfig::new(0, 1), &scale_factors_by_frame),
            &pcm,
            AacPcmLongBlockConfig::new(0, 1.0, 1024),
            &[],
            AacSpectralMagnitudeTables::default(),
        )
        .unwrap();

        assert_eq!(adts, [frame, frame].concat());
        assert!(encode_pcm_mono_long_block_adts_stream_with_scale_factors(
            AdtsConfig::aac_lc(44_100, 1),
            AacScaleFactorSequence::new(AacLongBlockConfig::new(0, 1), &[&[0]]),
            &pcm,
            AacPcmLongBlockConfig::new(0, 1.0, 1024),
            &[],
            AacSpectralMagnitudeTables::default(),
        )
        .is_err());
    }

    #[test]
    fn encodes_pcm_mono_long_block_adts_stream_with_selected_scale_factors() {
        let pcm = AudioBuffer::new(44_100, 1, vec![0.0; 2048]).unwrap();
        let frame = [
            0xff, 0xf1, 0x50, 0x40, 0x01, 0xbf, 0xfc, 0x00, 0x00, 0x00, 0x80, 0x23, 0x80,
        ];

        let adts = encode_pcm_mono_long_block_adts_stream_with_selected_scale_factors(
            AdtsConfig::aac_lc(44_100, 1),
            AacLongBlockConfig::new(0, 1),
            &pcm,
            AacPcmLongBlockConfig::new(0, 1.0, 1024),
            &[],
            AacSpectralMagnitudeTables::default(),
        )
        .unwrap();

        assert_eq!(adts, [frame, frame].concat());
        assert!(
            encode_pcm_mono_long_block_adts_stream_with_selected_scale_factors(
                AdtsConfig::aac_lc(44_100, 1),
                AacLongBlockConfig::new(0, 1),
                &pcm,
                AacPcmLongBlockConfig::new(4096, 1.0, 1024),
                &[],
                AacSpectralMagnitudeTables::default(),
            )
            .is_err()
        );
    }

    #[test]
    fn encodes_pcm_mono_long_block_adts_stream_with_bit_cost_sections() {
        let pcm = AudioBuffer::new(44_100, 1, vec![0.0; 2048]).unwrap();
        let frame = [
            0xff, 0xf1, 0x50, 0x40, 0x01, 0xbf, 0xfc, 0x00, 0x00, 0x00, 0x80, 0x23, 0x80,
        ];
        let scale_factors_by_frame: [&[i16]; 2] = [&[0], &[0]];

        let encoded = encode_pcm_mono_long_block_adts_stream_by_bit_cost(
            AdtsConfig::aac_lc(44_100, 1),
            AacLongBlockConfig::new(0, 1),
            &pcm,
            AacPcmLongBlockConfig::new(0, 1.0, 1024),
            AacSpectralMagnitudeTables::default(),
        )
        .unwrap();
        let with_scale_factors =
            encode_pcm_mono_long_block_adts_stream_with_scale_factors_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 1),
                AacScaleFactorSequence::new(AacLongBlockConfig::new(0, 1), &scale_factors_by_frame),
                &pcm,
                AacPcmLongBlockConfig::new(0, 1.0, 1024),
                &[],
                AacSpectralMagnitudeTables::default(),
            )
            .unwrap();
        let selected =
            encode_pcm_mono_long_block_adts_stream_with_selected_scale_factors_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 1),
                AacLongBlockConfig::new(0, 1),
                &pcm,
                AacPcmLongBlockConfig::new(0, 1.0, 1024),
                &[],
                AacSpectralMagnitudeTables::default(),
            )
            .unwrap();

        assert_eq!(encoded, [frame, frame].concat());
        assert_eq!(with_scale_factors, encoded);
        assert_eq!(selected, with_scale_factors);
        assert!(
            encode_pcm_mono_long_block_adts_stream_with_scale_factors_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 1),
                AacScaleFactorSequence::new(AacLongBlockConfig::new(0, 1), &[&[0]]),
                &pcm,
                AacPcmLongBlockConfig::new(0, 1.0, 1024),
                &[],
                AacSpectralMagnitudeTables::default(),
            )
            .is_err()
        );
    }

    #[test]
    fn selects_mono_pcm_frame_step_for_experimental_nonzero_payload() {
        let pcm = AudioBuffer::new(
            44_100,
            1,
            (0..2048)
                .map(|sample| ((sample as f32) * 0.01).sin() * 0.25)
                .collect(),
        )
        .unwrap();
        let channel = AacLongBlockConfig::new(0, 1);
        let scale_factor_table = experimental_aac_scale_factor_delta_table();
        let spectral_tables = experimental_unit_magnitude_spectral_tables();

        let step = select_aac_lc_mono_pcm_frame_step_by_bit_cost(
            AdtsConfig::aac_lc(44_100, 1),
            channel,
            &pcm,
            AacPcmStepSearchConfig::new(
                0,
                1024,
                AAC_LC_PCM_STEP_CANDIDATES,
                &scale_factor_table,
                spectral_tables,
            ),
        )
        .unwrap();
        let reversed_candidates = AAC_LC_PCM_STEP_CANDIDATES
            .iter()
            .copied()
            .rev()
            .collect::<Vec<_>>();
        let details: AacPcmFrameStepSelection =
            select_aac_lc_mono_pcm_frame_step_details_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 1),
                channel,
                &pcm,
                AacPcmStepSearchConfig::new(
                    0,
                    1024,
                    &reversed_candidates,
                    &scale_factor_table,
                    spectral_tables,
                ),
            )
            .unwrap();
        let auto = encode_pcm_mono_long_block_adts_stream_with_auto_step_by_bit_cost(
            AdtsConfig::aac_lc(44_100, 1),
            channel,
            &pcm,
            AacPcmStepSearchConfig::new(
                0,
                1024,
                AAC_LC_PCM_STEP_CANDIDATES,
                &scale_factor_table,
                spectral_tables,
            ),
        )
        .unwrap();
        let selected =
            encode_pcm_mono_long_block_adts_stream_with_selected_scale_factors_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 1),
                channel,
                &pcm,
                AacPcmLongBlockConfig::new(0, step, 1024),
                &scale_factor_table,
                spectral_tables,
            )
            .unwrap();
        let public_scaffold = encode(&pcm).unwrap();
        let offsets = aac_lc_long_window_scale_factor_band_offsets(44_100).unwrap();
        let production_channel = AacLongBlockConfig::new(180, (offsets.len() - 1) as u8);
        let production_bitrate = aac_lc_default_production_bitrate_bps(1).unwrap();
        let production =
            encode_pcm_mono_long_block_adts_stream_with_offsets_and_selected_scale_factors_and_bitrate_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 1),
                production_channel,
                &pcm,
                offsets,
                AAC_LC_PCM_STEP_CANDIDATES,
                production_bitrate,
                &aac_scale_factor_delta_table(),
                aac_unsigned_pairs7_unit_magnitude_spectral_tables(),
            )
            .unwrap();
        let zero_payload =
            encode_pcm_mono_long_block_adts_stream_with_selected_scale_factors_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 1),
                channel,
                &pcm,
                AacPcmLongBlockConfig::new(0, f32::MAX, 1024),
                &[],
                AacSpectralMagnitudeTables::default(),
            )
            .unwrap();

        assert!(step < f32::MAX);
        assert_eq!(details.step, step);
        assert!(details.frame_len > 0);
        assert!(details.frame_len <= details.frame_capacity_bytes);
        assert_eq!(auto, selected);
        assert_eq!(&public_scaffold[..2], &[0xff, 0xf1]);
        assert_eq!(public_scaffold, production);
        assert_ne!(public_scaffold, zero_payload);
        assert_ne!(auto, zero_payload);
    }

    #[test]
    fn selects_mono_pcm_frame_step_with_max_frame_len() {
        let pcm = AudioBuffer::new(
            44_100,
            1,
            (0..2048)
                .map(|sample| ((sample as f32) * 0.01).sin() * 0.25)
                .collect(),
        )
        .unwrap();
        let channel = AacLongBlockConfig::new(0, 1);
        let scale_factor_table = experimental_aac_scale_factor_delta_table();
        let spectral_tables = experimental_unit_magnitude_spectral_tables();
        let fallback_candidate = [f32::MAX];
        let unconstrained = select_aac_lc_mono_pcm_frame_step_details_by_bit_cost(
            AdtsConfig::aac_lc(44_100, 1),
            channel,
            &pcm,
            AacPcmStepSearchConfig::new(
                0,
                1024,
                AAC_LC_PCM_STEP_CANDIDATES,
                &scale_factor_table,
                spectral_tables,
            ),
        )
        .unwrap();
        let fallback = select_aac_lc_mono_pcm_frame_step_details_by_bit_cost(
            AdtsConfig::aac_lc(44_100, 1),
            channel,
            &pcm,
            AacPcmStepSearchConfig::new(
                0,
                1024,
                &fallback_candidate,
                &scale_factor_table,
                spectral_tables,
            ),
        )
        .unwrap();

        let step = select_aac_lc_mono_pcm_frame_step_with_max_frame_len_by_bit_cost(
            AdtsConfig::aac_lc(44_100, 1),
            channel,
            &pcm,
            AacPcmStepSearchConfig::new(
                0,
                1024,
                AAC_LC_PCM_STEP_CANDIDATES,
                &scale_factor_table,
                spectral_tables,
            ),
            fallback.frame_len,
        )
        .unwrap();
        let details = select_aac_lc_mono_pcm_frame_step_details_with_max_frame_len_by_bit_cost(
            AdtsConfig::aac_lc(44_100, 1),
            channel,
            &pcm,
            AacPcmStepSearchConfig::new(
                0,
                1024,
                AAC_LC_PCM_STEP_CANDIDATES,
                &scale_factor_table,
                spectral_tables,
            ),
            fallback.frame_len,
        )
        .unwrap();

        assert_eq!(step, details.step);
        assert!(details.step > unconstrained.step);
        assert_eq!(details.frame_capacity_bytes, fallback.frame_len);
        assert!(details.frame_len <= fallback.frame_len);
        assert!(
            select_aac_lc_mono_pcm_frame_step_details_with_max_frame_len_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 1),
                channel,
                &pcm,
                AacPcmStepSearchConfig::new(
                    0,
                    1024,
                    AAC_LC_PCM_STEP_CANDIDATES,
                    &scale_factor_table,
                    spectral_tables,
                ),
                fallback.frame_len - 1,
            )
            .is_err()
        );
    }

    #[test]
    fn selects_production_offsets_pcm_frame_step_with_max_frame_len() {
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
        let offsets = aac_lc_long_window_scale_factor_band_offsets(44_100).unwrap();
        let channel_config = AacLongBlockConfig::new(180, (offsets.len() - 1) as u8);
        let scale_factors = vec![i16::from(channel_config.global_gain); offsets.len() - 1];
        let channel = AacScaleFactorChannel::new(channel_config, &scale_factors);
        let scale_factor_table = aac_scale_factor_delta_zero_table();
        let spectral_tables = aac_unsigned_pairs7_unit_magnitude_spectral_tables();
        let default_mono_bitrate = aac_lc_default_production_bitrate_bps(1).unwrap();
        let default_stereo_bitrate = aac_lc_default_production_bitrate_bps(2).unwrap();

        assert_eq!(default_mono_bitrate, 128_000);
        assert_eq!(default_stereo_bitrate, 256_000);
        assert!(
            aac_lc_adts_max_frame_len_for_bitrate(44_100, default_mono_bitrate).unwrap()
                >= AAC_ADTS_HEADER_LEN
        );
        assert!(aac_lc_default_production_bitrate_bps(0).is_err());
        assert!(aac_lc_default_production_bitrate_bps(3).is_err());

        let mono_unconstrained =
            select_aac_lc_mono_pcm_frame_step_details_with_offsets_and_scale_factors_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 1),
                channel,
                &mono,
                0,
                offsets,
                AAC_LC_PCM_STEP_CANDIDATES,
                scale_factor_table,
                spectral_tables,
            )
            .unwrap();
        let mono_min_frame_len = AAC_LC_PCM_STEP_CANDIDATES
            .iter()
            .copied()
            .filter_map(|candidate| {
                select_aac_lc_mono_pcm_frame_step_details_with_offsets_and_scale_factors_by_bit_cost(
                    AdtsConfig::aac_lc(44_100, 1),
                    channel,
                    &mono,
                    0,
                    offsets,
                    &[candidate],
                    scale_factor_table,
                    spectral_tables,
                )
                .ok()
            })
            .map(|selection| selection.frame_len)
            .min()
            .unwrap();
        let mono_step =
            select_aac_lc_mono_pcm_frame_step_with_offsets_and_scale_factors_and_max_frame_len_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 1),
                channel,
                &mono,
                0,
                offsets,
                AAC_LC_PCM_STEP_CANDIDATES,
                mono_unconstrained.frame_len,
                scale_factor_table,
                spectral_tables,
            )
            .unwrap();
        let mono_details =
            select_aac_lc_mono_pcm_frame_step_details_with_offsets_and_scale_factors_and_max_frame_len_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 1),
                channel,
                &mono,
                0,
                offsets,
                AAC_LC_PCM_STEP_CANDIDATES,
                mono_unconstrained.frame_len,
                scale_factor_table,
                spectral_tables,
            )
            .unwrap();
        let mono_encoded =
            encode_pcm_mono_long_block_adts_stream_with_offsets_and_scale_factors_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 1),
                channel,
                &mono,
                offsets,
                AAC_LC_PCM_STEP_CANDIDATES,
                scale_factor_table,
                spectral_tables,
            )
            .unwrap();
        let mono_budget_encoded =
            encode_pcm_mono_long_block_adts_stream_with_offsets_and_scale_factors_and_max_frame_len_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 1),
                channel,
                &mono,
                offsets,
                AAC_LC_PCM_STEP_CANDIDATES,
                max_adts_frame_len(&mono_encoded),
                scale_factor_table,
                spectral_tables,
            )
            .unwrap();
        let mono_target_bitrate =
            ((max_adts_frame_len(&mono_encoded) as u64 * 8 * 44_100).div_ceil(1024)) as u32;
        let mono_bitrate_budget =
            aac_lc_adts_max_frame_len_for_bitrate(44_100, mono_target_bitrate).unwrap();
        let mono_bitrate_encoded =
            encode_pcm_mono_long_block_adts_stream_with_offsets_and_scale_factors_and_bitrate_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 1),
                channel,
                &mono,
                offsets,
                AAC_LC_PCM_STEP_CANDIDATES,
                mono_target_bitrate,
                scale_factor_table,
                spectral_tables,
            )
            .unwrap();
        let selected_scale_factor_table = aac_scale_factor_delta_table();
        let mono_selected_unconstrained =
            select_aac_lc_mono_pcm_frame_step_details_with_offsets_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 1),
                channel_config,
                &mono,
                0,
                offsets,
                AAC_LC_PCM_STEP_CANDIDATES,
                &selected_scale_factor_table,
                spectral_tables,
            )
            .unwrap();
        let mono_selected_step =
            select_aac_lc_mono_pcm_frame_step_with_offsets_and_max_frame_len_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 1),
                channel_config,
                &mono,
                0,
                offsets,
                AAC_LC_PCM_STEP_CANDIDATES,
                mono_selected_unconstrained.frame_len,
                &selected_scale_factor_table,
                spectral_tables,
            )
            .unwrap();
        let mono_selected_details =
            select_aac_lc_mono_pcm_frame_step_details_with_offsets_and_max_frame_len_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 1),
                channel_config,
                &mono,
                0,
                offsets,
                AAC_LC_PCM_STEP_CANDIDATES,
                mono_selected_unconstrained.frame_len,
                &selected_scale_factor_table,
                spectral_tables,
            )
            .unwrap();
        let mono_selected_encoded =
            encode_pcm_mono_long_block_adts_stream_with_offsets_and_selected_scale_factors_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 1),
                channel_config,
                &mono,
                offsets,
                AAC_LC_PCM_STEP_CANDIDATES,
                &selected_scale_factor_table,
                spectral_tables,
            )
            .unwrap();
        let mono_selected_budget_encoded =
            encode_pcm_mono_long_block_adts_stream_with_offsets_and_selected_scale_factors_and_max_frame_len_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 1),
                channel_config,
                &mono,
                offsets,
                AAC_LC_PCM_STEP_CANDIDATES,
                max_adts_frame_len(&mono_selected_encoded),
                &selected_scale_factor_table,
                spectral_tables,
            )
            .unwrap();
        let mono_selected_target_bitrate =
            ((max_adts_frame_len(&mono_selected_encoded) as u64 * 8 * 44_100).div_ceil(1024))
                as u32;
        let mono_selected_bitrate_budget =
            aac_lc_adts_max_frame_len_for_bitrate(44_100, mono_selected_target_bitrate).unwrap();
        let mono_selected_bitrate_encoded =
            encode_pcm_mono_long_block_adts_stream_with_offsets_and_selected_scale_factors_and_bitrate_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 1),
                channel_config,
                &mono,
                offsets,
                AAC_LC_PCM_STEP_CANDIDATES,
                mono_selected_target_bitrate,
                &selected_scale_factor_table,
                spectral_tables,
            )
            .unwrap();

        assert_eq!(mono_step, mono_unconstrained.step);
        assert_eq!(mono_details.step, mono_unconstrained.step);
        assert_eq!(
            mono_details.frame_capacity_bytes,
            mono_unconstrained.frame_len
        );
        assert_eq!(mono_budget_encoded, mono_encoded);
        assert!(max_adts_frame_len(&mono_bitrate_encoded) <= mono_bitrate_budget);
        assert_eq!(mono_selected_details.step, mono_selected_unconstrained.step);
        assert_eq!(mono_selected_step, mono_selected_unconstrained.step);
        assert_eq!(mono_selected_budget_encoded, mono_selected_encoded);
        assert!(max_adts_frame_len(&mono_selected_bitrate_encoded) <= mono_selected_bitrate_budget);
        assert!(
            select_aac_lc_mono_pcm_frame_step_details_with_offsets_and_scale_factors_and_max_frame_len_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 1),
                channel,
                &mono,
                0,
                offsets,
                AAC_LC_PCM_STEP_CANDIDATES,
                mono_min_frame_len - 1,
                scale_factor_table,
                spectral_tables,
            )
            .is_err()
        );

        let stereo_unconstrained =
            select_aac_lc_stereo_pcm_frame_step_details_with_offsets_and_scale_factors_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 2),
                channel,
                channel,
                &stereo,
                0,
                offsets,
                AAC_LC_PCM_STEP_CANDIDATES,
                scale_factor_table,
                spectral_tables,
            )
            .unwrap();
        let stereo_min_frame_len = AAC_LC_PCM_STEP_CANDIDATES
            .iter()
            .copied()
            .filter_map(|candidate| {
                select_aac_lc_stereo_pcm_frame_step_details_with_offsets_and_scale_factors_by_bit_cost(
                    AdtsConfig::aac_lc(44_100, 2),
                    channel,
                    channel,
                    &stereo,
                    0,
                    offsets,
                    &[candidate],
                    scale_factor_table,
                    spectral_tables,
                )
                .ok()
            })
            .map(|selection| selection.frame_len)
            .min()
            .unwrap();
        let stereo_step =
            select_aac_lc_stereo_pcm_frame_step_with_offsets_and_scale_factors_and_max_frame_len_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 2),
                channel,
                channel,
                &stereo,
                0,
                offsets,
                AAC_LC_PCM_STEP_CANDIDATES,
                stereo_unconstrained.frame_len,
                scale_factor_table,
                spectral_tables,
            )
            .unwrap();
        let stereo_details =
            select_aac_lc_stereo_pcm_frame_step_details_with_offsets_and_scale_factors_and_max_frame_len_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 2),
                channel,
                channel,
                &stereo,
                0,
                offsets,
                AAC_LC_PCM_STEP_CANDIDATES,
                stereo_unconstrained.frame_len,
                scale_factor_table,
                spectral_tables,
            )
            .unwrap();
        let stereo_encoded =
            encode_pcm_stereo_long_block_adts_stream_with_offsets_and_scale_factors_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 2),
                channel,
                channel,
                &stereo,
                offsets,
                AAC_LC_PCM_STEP_CANDIDATES,
                scale_factor_table,
                spectral_tables,
            )
            .unwrap();
        let stereo_budget_encoded =
            encode_pcm_stereo_long_block_adts_stream_with_offsets_and_scale_factors_and_max_frame_len_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 2),
                channel,
                channel,
                &stereo,
                offsets,
                AAC_LC_PCM_STEP_CANDIDATES,
                max_adts_frame_len(&stereo_encoded),
                scale_factor_table,
                spectral_tables,
            )
            .unwrap();
        let stereo_target_bitrate =
            ((max_adts_frame_len(&stereo_encoded) as u64 * 8 * 44_100).div_ceil(1024)) as u32;
        let stereo_bitrate_budget =
            aac_lc_adts_max_frame_len_for_bitrate(44_100, stereo_target_bitrate).unwrap();
        let stereo_bitrate_encoded =
            encode_pcm_stereo_long_block_adts_stream_with_offsets_and_scale_factors_and_bitrate_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 2),
                channel,
                channel,
                &stereo,
                offsets,
                AAC_LC_PCM_STEP_CANDIDATES,
                stereo_target_bitrate,
                scale_factor_table,
                spectral_tables,
            )
            .unwrap();
        let stereo_selected_unconstrained =
            select_aac_lc_stereo_pcm_frame_step_details_with_offsets_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 2),
                channel_config,
                channel_config,
                &stereo,
                0,
                offsets,
                AAC_LC_PCM_STEP_CANDIDATES,
                &selected_scale_factor_table,
                spectral_tables,
            )
            .unwrap();
        let stereo_selected_unconstrained_step =
            select_aac_lc_stereo_pcm_frame_step_with_offsets_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 2),
                channel_config,
                channel_config,
                &stereo,
                0,
                offsets,
                AAC_LC_PCM_STEP_CANDIDATES,
                &selected_scale_factor_table,
                spectral_tables,
            )
            .unwrap();
        let stereo_selected_step =
            select_aac_lc_stereo_pcm_frame_step_with_offsets_and_max_frame_len_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 2),
                channel_config,
                channel_config,
                &stereo,
                0,
                offsets,
                AAC_LC_PCM_STEP_CANDIDATES,
                stereo_selected_unconstrained.frame_len,
                &selected_scale_factor_table,
                spectral_tables,
            )
            .unwrap();
        let stereo_selected_details =
            select_aac_lc_stereo_pcm_frame_step_details_with_offsets_and_max_frame_len_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 2),
                channel_config,
                channel_config,
                &stereo,
                0,
                offsets,
                AAC_LC_PCM_STEP_CANDIDATES,
                stereo_selected_unconstrained.frame_len,
                &selected_scale_factor_table,
                spectral_tables,
            )
            .unwrap();
        let stereo_selected_encoded =
            encode_pcm_stereo_long_block_adts_stream_with_offsets_and_selected_scale_factors_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 2),
                channel_config,
                channel_config,
                &stereo,
                offsets,
                AAC_LC_PCM_STEP_CANDIDATES,
                &selected_scale_factor_table,
                spectral_tables,
            )
            .unwrap();
        let stereo_selected_budget_encoded =
            encode_pcm_stereo_long_block_adts_stream_with_offsets_and_selected_scale_factors_and_max_frame_len_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 2),
                channel_config,
                channel_config,
                &stereo,
                offsets,
                AAC_LC_PCM_STEP_CANDIDATES,
                max_adts_frame_len(&stereo_selected_encoded),
                &selected_scale_factor_table,
                spectral_tables,
            )
            .unwrap();
        let stereo_selected_target_bitrate =
            ((max_adts_frame_len(&stereo_selected_encoded) as u64 * 8 * 44_100).div_ceil(1024))
                as u32;
        let stereo_selected_bitrate_budget =
            aac_lc_adts_max_frame_len_for_bitrate(44_100, stereo_selected_target_bitrate).unwrap();
        let stereo_selected_bitrate_encoded =
            encode_pcm_stereo_long_block_adts_stream_with_offsets_and_selected_scale_factors_and_bitrate_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 2),
                channel_config,
                channel_config,
                &stereo,
                offsets,
                AAC_LC_PCM_STEP_CANDIDATES,
                stereo_selected_target_bitrate,
                &selected_scale_factor_table,
                spectral_tables,
            )
            .unwrap();

        assert_eq!(stereo_step, stereo_unconstrained.step);
        assert_eq!(stereo_details.step, stereo_unconstrained.step);
        assert_eq!(
            stereo_details.frame_capacity_bytes,
            stereo_unconstrained.frame_len
        );
        assert_eq!(stereo_budget_encoded, stereo_encoded);
        assert!(max_adts_frame_len(&stereo_bitrate_encoded) <= stereo_bitrate_budget);
        assert_eq!(
            stereo_selected_details.step,
            stereo_selected_unconstrained.step
        );
        assert_eq!(
            stereo_selected_unconstrained_step,
            stereo_selected_unconstrained.step
        );
        assert_eq!(stereo_selected_step, stereo_selected_unconstrained.step);
        assert_eq!(stereo_selected_budget_encoded, stereo_selected_encoded);
        assert!(
            max_adts_frame_len(&stereo_selected_bitrate_encoded) <= stereo_selected_bitrate_budget
        );
        assert!(aac_lc_adts_max_frame_len_for_bitrate(44_100, 1).is_err());
        assert!(
            select_aac_lc_stereo_pcm_frame_step_details_with_offsets_and_scale_factors_and_max_frame_len_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 2),
                channel,
                channel,
                &stereo,
                0,
                offsets,
                AAC_LC_PCM_STEP_CANDIDATES,
                stereo_min_frame_len - 1,
                scale_factor_table,
                spectral_tables,
            )
            .is_err()
        );
    }

    #[test]
    fn selects_stereo_pcm_frame_step_for_experimental_nonzero_payload() {
        let mut samples = Vec::new();
        for sample in 0..2048 {
            samples.push(((sample as f32) * 0.01).sin() * 0.25);
            samples.push(((sample as f32) * 0.013).cos() * 0.20);
        }
        let pcm = AudioBuffer::new(44_100, 2, samples).unwrap();
        let channel = AacLongBlockConfig::new(0, 1);
        let scale_factor_table = experimental_aac_scale_factor_delta_table();
        let spectral_tables = experimental_unit_magnitude_spectral_tables();

        let step = select_aac_lc_stereo_pcm_frame_step_by_bit_cost(
            AdtsConfig::aac_lc(44_100, 2),
            channel,
            channel,
            &pcm,
            AacPcmStepSearchConfig::new(
                0,
                1024,
                AAC_LC_PCM_STEP_CANDIDATES,
                &scale_factor_table,
                spectral_tables,
            ),
        )
        .unwrap();
        let reversed_candidates = AAC_LC_PCM_STEP_CANDIDATES
            .iter()
            .copied()
            .rev()
            .collect::<Vec<_>>();
        let details: AacPcmFrameStepSelection =
            select_aac_lc_stereo_pcm_frame_step_details_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 2),
                channel,
                channel,
                &pcm,
                AacPcmStepSearchConfig::new(
                    0,
                    1024,
                    &reversed_candidates,
                    &scale_factor_table,
                    spectral_tables,
                ),
            )
            .unwrap();
        let auto = encode_pcm_stereo_long_block_adts_stream_with_auto_step_by_bit_cost(
            AdtsConfig::aac_lc(44_100, 2),
            channel,
            channel,
            &pcm,
            AacPcmStepSearchConfig::new(
                0,
                1024,
                AAC_LC_PCM_STEP_CANDIDATES,
                &scale_factor_table,
                spectral_tables,
            ),
        )
        .unwrap();
        let selected =
            encode_pcm_stereo_long_block_adts_stream_with_selected_scale_factors_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 2),
                channel,
                channel,
                &pcm,
                AacPcmLongBlockConfig::new(0, step, 1024),
                &scale_factor_table,
                spectral_tables,
            )
            .unwrap();
        let public_scaffold = encode(&pcm).unwrap();
        let offsets = aac_lc_long_window_scale_factor_band_offsets(44_100).unwrap();
        let production_channel = AacLongBlockConfig::new(180, (offsets.len() - 1) as u8);
        let production_bitrate = aac_lc_default_production_bitrate_bps(2).unwrap();
        let production_budget =
            aac_lc_adts_max_frame_len_for_bitrate(44_100, production_bitrate).unwrap();
        let production_step =
            select_aac_lc_stereo_pcm_frame_step_with_offsets_and_max_frame_len_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 2),
                production_channel,
                production_channel,
                &pcm,
                0,
                offsets,
                AAC_LC_PCM_STEP_CANDIDATES,
                production_budget,
                &aac_scale_factor_delta_table(),
                aac_unsigned_pairs7_unit_magnitude_spectral_tables(),
            )
            .unwrap();
        let production_details =
            select_aac_lc_stereo_pcm_frame_step_details_with_offsets_and_max_frame_len_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 2),
                production_channel,
                production_channel,
                &pcm,
                0,
                offsets,
                &reversed_candidates,
                production_budget,
                &aac_scale_factor_delta_table(),
                aac_unsigned_pairs7_unit_magnitude_spectral_tables(),
            )
            .unwrap();
        let production =
            encode_pcm_stereo_long_block_adts_stream_with_offsets_and_selected_scale_factors_and_bitrate_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 2),
                production_channel,
                production_channel,
                &pcm,
                offsets,
                AAC_LC_PCM_STEP_CANDIDATES,
                production_bitrate,
                &aac_scale_factor_delta_table(),
                aac_unsigned_pairs7_unit_magnitude_spectral_tables(),
            )
            .unwrap();
        let production_reversed =
            encode_pcm_stereo_long_block_adts_stream_with_offsets_and_selected_scale_factors_and_bitrate_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 2),
                production_channel,
                production_channel,
                &pcm,
                offsets,
                &reversed_candidates,
                production_bitrate,
                &aac_scale_factor_delta_table(),
                aac_unsigned_pairs7_unit_magnitude_spectral_tables(),
            )
            .unwrap();
        let zero_payload =
            encode_pcm_stereo_long_block_adts_stream_with_selected_scale_factors_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 2),
                channel,
                channel,
                &pcm,
                AacPcmLongBlockConfig::new(0, f32::MAX, 1024),
                &[],
                AacSpectralMagnitudeTables::default(),
            )
            .unwrap();

        assert!(step < f32::MAX);
        assert_eq!(details.step, step);
        assert!(details.frame_len > 0);
        assert!(details.frame_len <= details.frame_capacity_bytes);
        assert_eq!(production_details.step, production_step);
        assert!(production_details.frame_len <= production_details.frame_capacity_bytes);
        assert_eq!(auto, selected);
        assert_eq!(public_scaffold, production);
        assert_eq!(production_reversed, production);
        assert_ne!(auto, zero_payload);
        assert_ne!(public_scaffold, zero_payload);
    }

    #[test]
    fn selects_stereo_pcm_frame_step_with_max_frame_len() {
        let mut samples = Vec::new();
        for sample in 0..2048 {
            samples.push(((sample as f32) * 0.01).sin() * 0.25);
            samples.push(((sample as f32) * 0.013).cos() * 0.20);
        }
        let pcm = AudioBuffer::new(44_100, 2, samples).unwrap();
        let channel = AacLongBlockConfig::new(0, 1);
        let scale_factor_table = experimental_aac_scale_factor_delta_table();
        let spectral_tables = experimental_unit_magnitude_spectral_tables();
        let fallback_candidate = [f32::MAX];
        let unconstrained = select_aac_lc_stereo_pcm_frame_step_details_by_bit_cost(
            AdtsConfig::aac_lc(44_100, 2),
            channel,
            channel,
            &pcm,
            AacPcmStepSearchConfig::new(
                0,
                1024,
                AAC_LC_PCM_STEP_CANDIDATES,
                &scale_factor_table,
                spectral_tables,
            ),
        )
        .unwrap();
        let fallback = select_aac_lc_stereo_pcm_frame_step_details_by_bit_cost(
            AdtsConfig::aac_lc(44_100, 2),
            channel,
            channel,
            &pcm,
            AacPcmStepSearchConfig::new(
                0,
                1024,
                &fallback_candidate,
                &scale_factor_table,
                spectral_tables,
            ),
        )
        .unwrap();

        let step = select_aac_lc_stereo_pcm_frame_step_with_max_frame_len_by_bit_cost(
            AdtsConfig::aac_lc(44_100, 2),
            channel,
            channel,
            &pcm,
            AacPcmStepSearchConfig::new(
                0,
                1024,
                AAC_LC_PCM_STEP_CANDIDATES,
                &scale_factor_table,
                spectral_tables,
            ),
            fallback.frame_len,
        )
        .unwrap();
        let details = select_aac_lc_stereo_pcm_frame_step_details_with_max_frame_len_by_bit_cost(
            AdtsConfig::aac_lc(44_100, 2),
            channel,
            channel,
            &pcm,
            AacPcmStepSearchConfig::new(
                0,
                1024,
                AAC_LC_PCM_STEP_CANDIDATES,
                &scale_factor_table,
                spectral_tables,
            ),
            fallback.frame_len,
        )
        .unwrap();

        assert_eq!(step, details.step);
        assert!(details.step > unconstrained.step);
        assert_eq!(details.frame_capacity_bytes, fallback.frame_len);
        assert!(details.frame_len <= fallback.frame_len);
        assert!(
            select_aac_lc_stereo_pcm_frame_step_details_with_max_frame_len_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 2),
                channel,
                channel,
                &pcm,
                AacPcmStepSearchConfig::new(
                    0,
                    1024,
                    AAC_LC_PCM_STEP_CANDIDATES,
                    &scale_factor_table,
                    spectral_tables,
                ),
                fallback.frame_len - 1,
            )
            .is_err()
        );
    }

    #[test]
    fn encodes_pcm_stereo_long_block_adts_stream() {
        let pcm = AudioBuffer::new(44_100, 2, vec![0.0; 4096]).unwrap();
        let frame = [
            0xff, 0xf1, 0x50, 0x80, 0x02, 0x3f, 0xfc, 0x20, 0x00, 0x00, 0x40, 0x10, 0x00, 0x00,
            0x80, 0x23, 0x80,
        ];

        let adts = encode_pcm_stereo_long_block_adts_stream(
            AdtsConfig::aac_lc(44_100, 2),
            AacLongBlockConfig::new(0, 1),
            AacLongBlockConfig::new(0, 1),
            &pcm,
            AacPcmLongBlockConfig::new(0, 1.0, 1024),
            AacSpectralMagnitudeTables::default(),
        )
        .unwrap();

        assert_eq!(adts, [frame, frame].concat());
        assert!(encode_pcm_stereo_long_block_adts_stream(
            AdtsConfig::aac_lc(44_100, 2),
            AacLongBlockConfig::new(0, 1),
            AacLongBlockConfig::new(0, 1),
            &pcm,
            AacPcmLongBlockConfig::new(4096, 1.0, 1024),
            AacSpectralMagnitudeTables::default(),
        )
        .is_err());
    }

    #[test]
    fn encodes_pcm_stereo_long_block_adts_stream_with_scale_factors() {
        let pcm = AudioBuffer::new(44_100, 2, vec![0.0; 4096]).unwrap();
        let frame = [
            0xff, 0xf1, 0x50, 0x80, 0x02, 0x3f, 0xfc, 0x20, 0x00, 0x00, 0x40, 0x10, 0x00, 0x00,
            0x80, 0x23, 0x80,
        ];
        let scale_factors_by_frame: [&[i16]; 2] = [&[0], &[0]];

        let adts = encode_pcm_stereo_long_block_adts_stream_with_scale_factors(
            AdtsConfig::aac_lc(44_100, 2),
            AacScaleFactorSequence::new(AacLongBlockConfig::new(0, 1), &scale_factors_by_frame),
            AacScaleFactorSequence::new(AacLongBlockConfig::new(0, 1), &scale_factors_by_frame),
            &pcm,
            AacPcmLongBlockConfig::new(0, 1.0, 1024),
            &[],
            AacSpectralMagnitudeTables::default(),
        )
        .unwrap();

        assert_eq!(adts, [frame, frame].concat());
        assert!(encode_pcm_stereo_long_block_adts_stream_with_scale_factors(
            AdtsConfig::aac_lc(44_100, 2),
            AacScaleFactorSequence::new(AacLongBlockConfig::new(0, 1), &scale_factors_by_frame),
            AacScaleFactorSequence::new(AacLongBlockConfig::new(0, 1), &[&[0]]),
            &pcm,
            AacPcmLongBlockConfig::new(0, 1.0, 1024),
            &[],
            AacSpectralMagnitudeTables::default(),
        )
        .is_err());
    }

    #[test]
    fn encodes_pcm_stereo_long_block_adts_stream_with_selected_scale_factors() {
        let pcm = AudioBuffer::new(44_100, 2, vec![0.0; 4096]).unwrap();
        let frame = [
            0xff, 0xf1, 0x50, 0x80, 0x02, 0x3f, 0xfc, 0x20, 0x00, 0x00, 0x40, 0x10, 0x00, 0x00,
            0x80, 0x23, 0x80,
        ];

        let adts = encode_pcm_stereo_long_block_adts_stream_with_selected_scale_factors(
            AdtsConfig::aac_lc(44_100, 2),
            AacLongBlockConfig::new(0, 1),
            AacLongBlockConfig::new(0, 1),
            &pcm,
            AacPcmLongBlockConfig::new(0, 1.0, 1024),
            &[],
            AacSpectralMagnitudeTables::default(),
        )
        .unwrap();

        assert_eq!(adts, [frame, frame].concat());
        assert!(
            encode_pcm_stereo_long_block_adts_stream_with_selected_scale_factors(
                AdtsConfig::aac_lc(44_100, 2),
                AacLongBlockConfig::new(0, 1),
                AacLongBlockConfig::new(0, 1),
                &pcm,
                AacPcmLongBlockConfig::new(4096, 1.0, 1024),
                &[],
                AacSpectralMagnitudeTables::default(),
            )
            .is_err()
        );
    }

    #[test]
    fn encodes_pcm_stereo_long_block_adts_stream_with_bit_cost_sections() {
        let pcm = AudioBuffer::new(44_100, 2, vec![0.0; 4096]).unwrap();
        let frame = [
            0xff, 0xf1, 0x50, 0x80, 0x02, 0x3f, 0xfc, 0x20, 0x00, 0x00, 0x40, 0x10, 0x00, 0x00,
            0x80, 0x23, 0x80,
        ];
        let scale_factors_by_frame: [&[i16]; 2] = [&[0], &[0]];

        let encoded = encode_pcm_stereo_long_block_adts_stream_by_bit_cost(
            AdtsConfig::aac_lc(44_100, 2),
            AacLongBlockConfig::new(0, 1),
            AacLongBlockConfig::new(0, 1),
            &pcm,
            AacPcmLongBlockConfig::new(0, 1.0, 1024),
            AacSpectralMagnitudeTables::default(),
        )
        .unwrap();
        let with_scale_factors =
            encode_pcm_stereo_long_block_adts_stream_with_scale_factors_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 2),
                AacScaleFactorSequence::new(AacLongBlockConfig::new(0, 1), &scale_factors_by_frame),
                AacScaleFactorSequence::new(AacLongBlockConfig::new(0, 1), &scale_factors_by_frame),
                &pcm,
                AacPcmLongBlockConfig::new(0, 1.0, 1024),
                &[],
                AacSpectralMagnitudeTables::default(),
            )
            .unwrap();
        let selected =
            encode_pcm_stereo_long_block_adts_stream_with_selected_scale_factors_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 2),
                AacLongBlockConfig::new(0, 1),
                AacLongBlockConfig::new(0, 1),
                &pcm,
                AacPcmLongBlockConfig::new(0, 1.0, 1024),
                &[],
                AacSpectralMagnitudeTables::default(),
            )
            .unwrap();

        assert_eq!(encoded, [frame, frame].concat());
        assert_eq!(with_scale_factors, encoded);
        assert_eq!(selected, with_scale_factors);
        assert!(
            encode_pcm_stereo_long_block_adts_stream_with_scale_factors_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 2),
                AacScaleFactorSequence::new(AacLongBlockConfig::new(0, 1), &scale_factors_by_frame),
                AacScaleFactorSequence::new(AacLongBlockConfig::new(0, 1), &[&[0]]),
                &pcm,
                AacPcmLongBlockConfig::new(0, 1.0, 1024),
                &[],
                AacSpectralMagnitudeTables::default(),
            )
            .is_err()
        );
    }

    #[test]
    fn rejects_unrepresentable_adts_config() {
        let err = frame_adts(AdtsConfig::aac_lc(44_123, 2), &[0x00]).unwrap_err();
        assert!(matches!(err, Error::UnsupportedFeature("AAC sample rate")));

        let err = frame_adts(AdtsConfig::aac_lc(44_100, 8), &[0x00]).unwrap_err();
        assert!(matches!(
            err,
            Error::InvalidInput("AAC ADTS channel count exceeds 7")
        ));
    }

    #[test]
    fn encodes_silent_mono_pcm_as_adts() {
        let pcm = AudioBuffer::new(44_100, 1, vec![0.0; 1024]).unwrap();

        let adts = encode(&pcm).unwrap();

        assert_eq!(&adts[..7], &[0xff, 0xf1, 0x50, 0x40, 0x01, 0x7f, 0xfc]);
        assert_eq!(&adts[7..], &[0x00, 0x00, 0x00, 0x07]);
    }

    #[test]
    fn encodes_silent_stereo_pcm_as_multiple_adts_frames() {
        let pcm = AudioBuffer::new(48_000, 2, vec![0.0; 1024 * 2 + 8]).unwrap();

        let adts = encode(&pcm).unwrap();

        assert_eq!(&adts[..7], &[0xff, 0xf1, 0x4c, 0x80, 0x01, 0xdf, 0xfc]);
        assert_eq!(adts[14], 0xff);
    }

    #[test]
    fn encodes_non_silent_mono_pcm_as_long_block_scaffold() {
        let pcm = AudioBuffer::new(44_100, 1, vec![0.0, 0.5]).unwrap();

        let adts = encode(&pcm).unwrap();

        assert_eq!(&adts[..2], &[0xff, 0xf1]);
        assert!(adts.len() > 7);
        assert_ne!(&adts[7..], &[0x00, 0x00, 0x00, 0x0e]);
    }

    #[test]
    fn decodes_explicit_zero_payload_mono_scaffold_as_zero_pcm() {
        let pcm = AudioBuffer::new(44_100, 1, vec![0.0, 0.5]).unwrap();
        let adts = encode_pcm_mono_long_block_adts_stream_with_selected_scale_factors_by_bit_cost(
            AdtsConfig::aac_lc(44_100, 1),
            AacLongBlockConfig::new(0, 1),
            &pcm,
            AacPcmLongBlockConfig::new(0, f32::MAX, 1024),
            &[],
            AacSpectralMagnitudeTables::default(),
        )
        .unwrap();

        let decoded = super::decode(&adts).unwrap();

        assert_eq!(decoded.sample_rate, 44_100);
        assert_eq!(decoded.channels, 1);
        assert_eq!(decoded.samples.len(), 1024);
        assert!(decoded.samples.iter().all(|sample| *sample == 0.0));
    }

    #[test]
    fn encodes_non_silent_stereo_pcm_as_long_block_scaffold() {
        let pcm = AudioBuffer::new(48_000, 2, vec![0.0, 0.25, 0.5, -0.25]).unwrap();

        let adts = encode(&pcm).unwrap();

        assert_eq!(&adts[..2], &[0xff, 0xf1]);
        assert!(adts.len() > 7);
        assert_eq!(adts[2] >> 2, 0x13);
    }

    #[test]
    fn decodes_own_silent_adts() {
        let pcm = AudioBuffer::new(44_100, 2, vec![0.0; 2048]).unwrap();
        let adts = encode(&pcm).unwrap();

        let decoded = super::decode(&adts).unwrap();

        assert_eq!(decoded.sample_rate, 44_100);
        assert_eq!(decoded.channels, 2);
        assert_eq!(decoded.samples.len(), 2048);
        assert!(decoded.samples.iter().all(|sample| *sample == 0.0));
    }

    #[test]
    fn rejects_unknown_aac_payload_for_decode() {
        let adts = frame_adts(AdtsConfig::aac_lc(44_100, 1), &[0xaa]).unwrap();

        let err = super::decode(&adts).unwrap_err();

        assert!(matches!(
            err,
            Error::UnsupportedFeature(
                "AAC decode currently supports sonare silent AAC-LC ADTS only"
            )
        ));
    }

    #[test]
    fn bit_writer_writes_msb_first() {
        let mut writer = BitWriter::new();
        writer.write_bits(0b101, 3).unwrap();
        writer.write_bits(0b11, 2).unwrap();

        assert_eq!(writer.finish_byte_aligned(), &[0b1011_1000]);
    }
}
