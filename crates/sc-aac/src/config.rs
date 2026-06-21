use super::*;

pub(crate) const AAC_ESCAPE_MAGNITUDE: u16 = 16;
pub(crate) const AAC_ADTS_MAX_FRAME_LEN: usize = 0x1fff;
pub(crate) const AAC_ADTS_HEADER_LEN: usize = 7;
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
pub const AAC_STANDARD_ID_PCM_STEP_CANDIDATES: &[f32] = &[
    0.0005,
    0.001,
    0.002,
    0.005,
    0.01,
    0.02,
    0.05,
    0.075,
    0.1,
    0.15,
    0.2,
    0.3,
    0.5,
    0.75,
    1.0,
    1.5,
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

/// Calibration offset (in scalefactor units) bridging our forward MDCT scale and
/// the spec-normalized coefficient scale the decoder's inverse MDCT assumes.
///
/// [`mdct_long_block`] computes an unnormalized transform, so its coefficients
/// are larger than the ISO-normalized ones by a constant factor of `2^16`. Each
/// scalefactor unit scales the dequantized amplitude by `2^0.25`, so absorbing
/// `2^16` takes `4 * 16 = 64` units. Adding this keeps the decoded output at the
/// input level instead of `2^16` too quiet.
pub(crate) const AAC_MDCT_NORMALIZATION_SCALE_FACTOR_OFFSET: f64 = 64.0;

/// Returns the uniform AAC scalefactor (and global gain) that inverts a scalar
/// quantizer step for this encoder's analysis pipeline.
///
/// The scalar quantizer emits `q = round(|x|^0.75 / step)` and the AAC
/// dequantizer reconstructs `|q|^(4/3) * 2^(0.25 * (scalefactor - 100))`
/// (ISO/IEC 14496-3, with `SF_OFFSET = 100`). Equating the two gives
/// `scalefactor = 100 + (16 / 3) * log2(step)`, identical for every band because
/// a single step is shared across the whole spectrum. The constant
/// [`AAC_MDCT_NORMALIZATION_SCALE_FACTOR_OFFSET`] then rescales for our
/// unnormalized forward MDCT. Encoding this scalefactor makes the decoded
/// spectrum track the input at the correct level instead of being tilted by an
/// ad-hoc per-band magnitude class.
pub fn aac_uniform_scale_factor_for_step(step: f32) -> Result<u8, Error> {
    if !step.is_finite() || step <= 0.0 {
        return Err(Error::InvalidInput(
            "AAC quantizer step must be positive and finite",
        ));
    }
    let scale_factor = (100.0
        + (16.0 / 3.0) * f64::from(step).log2()
        + AAC_MDCT_NORMALIZATION_SCALE_FACTOR_OFFSET)
        .round();
    if !(0.0..=255.0).contains(&scale_factor) {
        return Err(Error::UnsupportedFeature(
            "AAC uniform scale factor for step is out of range",
        ));
    }
    // The bounds check above guarantees a lossless cast into the 8-bit range.
    Ok(scale_factor as u8)
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
    pub(crate) fn adts_profile(self) -> u8 {
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

    pub(crate) fn channel_for_frame(
        self,
        frame_index: usize,
    ) -> Result<AacScaleFactorChannel<'a>, Error> {
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
