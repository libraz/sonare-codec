#![deny(unsafe_code)]
#![warn(clippy::all)]

use sc_core::{
    apply_window, concat_packed_bits, mdct, pack_huffman_codes, pack_huffman_codes_with_len,
    pack_huffman_symbols_with_len, quantize_spectrum, sine_window, write_packed_bits, AudioBuffer,
    BitWriter as CoreBitWriter, Decoder, Encoder, Error, HuffmanCode, HuffmanEntry, PackedBits,
};

const AAC_ESCAPE_MAGNITUDE: u16 = 16;
const AAC_ADTS_MAX_FRAME_LEN: usize = 0x1fff;
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
        44_100 | 48_000 => Some(AAC_LC_48K_LONG_WINDOW_SCALE_FACTOR_BAND_OFFSETS),
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
/// with an intentionally coarse quantizer, so production-quality psychoacoustic
/// modeling, non-zero spectral Huffman tables, and rate control are still
/// incomplete.
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
    Escape,
}

impl AacCodebook {
    #[must_use]
    pub fn id(self) -> u8 {
        match self {
            Self::Zero => 0,
            Self::SignedPairs1 => 1,
            Self::SignedPairs5 => 5,
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

#[derive(Clone, Copy, Debug, Default)]
pub struct AacSpectralTables<'a> {
    pub signed_pairs1: &'a [HuffmanEntry<AacSpectralPair>],
    pub signed_pairs5: &'a [HuffmanEntry<AacSpectralPair>],
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
            AacCodebook::Escape => non_empty_table(self.escape, "AAC escape codebook"),
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct AacSpectralMagnitudeTables<'a> {
    pub pairs1: &'a [HuffmanEntry<AacSpectralMagnitudePair>],
    pub pairs5: &'a [HuffmanEntry<AacSpectralMagnitudePair>],
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
            AacCodebook::Escape => {
                non_empty_magnitude_table(self.escape, "AAC magnitude escape codebook")
            }
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

pub const AAC_SCALE_FACTOR_DELTA_ZERO_TABLE: &[HuffmanEntry<AacScaleFactorDelta>] =
    &[HuffmanEntry {
        symbol: AacScaleFactorDelta { delta: 0 },
        code: HuffmanCode { bits: 0, len: 1 },
    }];

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

/// Returns a minimal experimental AAC scale-factor delta table.
///
/// This is not the AAC-LC standard scale-factor Huffman table. It exists to
/// exercise scale-factor DPCM placement while the standard table is filled in.
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

/// Returns the standard AAC scale-factor codebook entry for a zero DPCM delta.
///
/// This intentionally exposes only the canonical delta=0 codeword while the
/// full clean-room scale-factor Huffman table is filled in.
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
        write_aac_escape_suffix(&mut writer, magnitude.x)?;
        write_aac_escape_suffix(&mut writer, magnitude.y)?;
        write_aac_sign_bit(&mut writer, pair.x)?;
        write_aac_sign_bit(&mut writer, pair.y)?;
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

    let sections = plan_sections_by_offsets(quantized, offsets, spectral_tables)?;
    let scale_factors = select_scale_factors_for_quantized_bands_by_offsets(
        quantized,
        offsets,
        i16::from(channel.global_gain),
    )?;
    let scale_factor_deltas = plan_scale_factor_deltas_by_offsets(
        &sections,
        offsets,
        &scale_factors,
        i16::from(channel.global_gain),
    )?;
    let scale_factor_bits =
        pack_scale_factor_deltas_with_table(&scale_factor_deltas, scale_factor_table)?;
    let payload = split_sectioned_spectral_payload_with_offsets_and_scale_factor_bits(
        &sections,
        quantized,
        offsets,
        scale_factor_bits,
        spectral_tables,
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

    let sections = plan_sections_by_offsets(quantized, offsets, spectral_tables)?;
    let scale_factor_deltas = plan_scale_factor_deltas_by_offsets(
        &sections,
        offsets,
        scale_factors,
        i16::from(channel.global_gain),
    )?;
    let scale_factor_bits =
        pack_scale_factor_deltas_with_table(&scale_factor_deltas, scale_factor_table)?;
    let payload = split_sectioned_spectral_payload_with_offsets_and_scale_factor_bits(
        &sections,
        quantized,
        offsets,
        scale_factor_bits,
        spectral_tables,
    )?;
    let access_unit = pack_single_channel_raw_data_block_parts(channel, &payload)?;
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
        1 => AacCodebook::SignedPairs1,
        2..=4 => AacCodebook::SignedPairs5,
        5..=8191 => AacCodebook::Escape,
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
        aac_lc_long_window_scale_factor_band_offsets, aac_scale_factor_delta_zero_table, encode,
        encode_pcm_mono_long_block_adts, encode_pcm_mono_long_block_adts_by_bit_cost,
        encode_pcm_mono_long_block_adts_stream, encode_pcm_mono_long_block_adts_stream_by_bit_cost,
        encode_pcm_mono_long_block_adts_stream_with_auto_step_by_bit_cost,
        encode_pcm_mono_long_block_adts_stream_with_offsets_and_auto_step_by_bit_cost,
        encode_pcm_mono_long_block_adts_stream_with_offsets_and_scale_factors_by_bit_cost,
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
        pack_scale_factor_deltas_with_table, pack_section_data, pack_section_data_with_len,
        pack_section_data_with_offsets, pack_sectioned_spectral_payload,
        pack_sectioned_spectral_payload_with_scale_factor_bits,
        pack_sectioned_spectral_payload_with_sign_bits,
        pack_sectioned_spectral_payload_with_sign_bits_and_scale_factor_bits,
        pack_sectioned_spectral_payload_with_sign_bits_and_scale_factor_bits_by_bit_cost,
        pack_sectioned_spectral_payload_with_sign_bits_by_bit_cost,
        pack_single_channel_raw_data_block, pack_single_channel_raw_data_block_parts,
        pack_spectral_codewords, pack_spectral_codewords_with_len, pack_spectral_pairs_with_table,
        pack_spectral_sections, pack_spectral_sections_with_sign_bits, plan_scale_factor_deltas,
        plan_scale_factor_deltas_by_offsets, plan_sections, plan_sections_by_bit_cost,
        plan_sections_by_offsets, quantize_long_block, quantize_pcm_long_block,
        select_aac_lc_mono_pcm_frame_step_by_bit_cost,
        select_aac_lc_mono_pcm_frame_step_details_by_bit_cost,
        select_aac_lc_mono_pcm_frame_step_details_with_offsets_and_scale_factors_by_bit_cost,
        select_aac_lc_mono_pcm_frame_step_details_with_offsets_by_bit_cost,
        select_aac_lc_stereo_pcm_frame_step_by_bit_cost,
        select_aac_lc_stereo_pcm_frame_step_details_by_bit_cost, select_codebook_by_bit_cost,
        select_scale_factors_for_quantized_bands,
        select_scale_factors_for_quantized_bands_by_offsets, spectral_pairs_for_section,
        split_sectioned_spectral_payload_with_sign_bits, AacCodebook, AacLongBlockConfig,
        AacPcmFrameStepSelection, AacPcmLongBlockConfig, AacPcmStepSearchConfig,
        AacQuantizedChannel, AacQuantizedSpectrum, AacScaleFactorChannel, AacScaleFactorDelta,
        AacScaleFactorSequence, AacSection, AacSpectralMagnitudePair, AacSpectralMagnitudeTables,
        AacSpectralPair, AacSpectralTables, AdtsConfig, BitWriter,
        AAC_LC_48K_LONG_WINDOW_SCALE_FACTOR_BAND_OFFSETS, AAC_LC_PCM_STEP_CANDIDATES,
    };
    use sc_core::Error;
    use sc_core::{AudioBuffer, HuffmanCode, HuffmanEntry, PackedBits};

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
                    end: 8,
                    codebook: AacCodebook::SignedPairs1,
                },
                AacSection {
                    start: 8,
                    end: 12,
                    codebook: AacCodebook::SignedPairs5,
                },
                AacSection {
                    start: 12,
                    end: 16,
                    codebook: AacCodebook::Escape,
                },
            ]
        );
        assert_eq!(AacCodebook::Escape.id(), 11);
        assert!(plan_sections(&quantized, 0).is_err());
        assert!(plan_sections(&quantized[..15], 4).is_err());
        assert!(plan_sections(&[8192], 1).is_err());
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
            escape: &[],
        };

        assert_eq!(
            select_codebook_by_bit_cost(&[1, -1], tables).unwrap(),
            AacCodebook::SignedPairs5
        );
        assert_eq!(
            select_codebook_by_bit_cost(&[0, 0], AacSpectralMagnitudeTables::default()).unwrap(),
            AacCodebook::Zero
        );
        assert!(matches!(
            select_codebook_by_bit_cost(&[1, -1], AacSpectralMagnitudeTables::default())
                .unwrap_err(),
            Error::UnsupportedFeature("AAC spectral codebook")
        ));
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

        assert_eq!(offsets, AAC_LC_48K_LONG_WINDOW_SCALE_FACTOR_BAND_OFFSETS);
        assert_eq!(offsets.first().copied(), Some(0));
        assert_eq!(offsets.last().copied(), Some(1024));
        assert_eq!(offsets.len() - 1, 49);
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
                experimental_unit_magnitude_spectral_tables(),
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
                experimental_unit_magnitude_spectral_tables(),
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
                bytes: vec![0b1000_0011],
                bit_len: 8,
            }
        );
        assert_eq!(
            super::pack_spectral_pairs_with_sign_bits(&[AacSpectralPair::new(32, -18)], &table)
                .unwrap(),
            PackedBits {
                bytes: vec![0b1110_0000, 0b0000_1001],
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
                escape: &escape,
            },
        )
        .unwrap();

        assert_eq!(
            packed,
            PackedBits {
                bytes: vec![0x00, 0xd8, 0x60, 0x80],
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
                escape: &[],
            },
        )
        .unwrap();

        assert_eq!(
            adts,
            [
                0xff, 0xf1, 0x50, 0x40, 0x02, 0x1f, 0xfc, 0x00, 0xf0, 0x01, 0x80, 0x22, 0x15, 0x10,
                0x93, 0xb8,
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
                escape: &[],
            },
        )
        .unwrap();

        assert_eq!(
            adts,
            [
                0xff, 0xf1, 0x50, 0x40, 0x02, 0x3f, 0xfc, 0x00, 0xf0, 0x01, 0x80, 0x22, 0x15, 0x15,
                0x82, 0x4e, 0xe0,
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
                escape: &[],
            },
        )
        .unwrap();

        assert_eq!(
            adts,
            [
                0xff, 0xf1, 0x50, 0x80, 0x03, 0x1f, 0xfc, 0x20, 0x78, 0x00, 0xc0, 0x11, 0x0a, 0x88,
                0x49, 0xcc, 0x80, 0x18, 0x02, 0x21, 0x51, 0x0a, 0x37, 0x80,
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
                escape: &[],
            },
        )
        .unwrap();

        assert_eq!(
            adts,
            [
                0xff, 0xf1, 0x50, 0x80, 0x03, 0x3f, 0xfc, 0x20, 0x78, 0x00, 0xc0, 0x11, 0x0a, 0x8a,
                0xc1, 0x27, 0x32, 0x00, 0x60, 0x08, 0x85, 0x45, 0x60, 0xa3, 0x78,
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
        assert_eq!(public_scaffold, zero_payload);
        assert_ne!(auto, zero_payload);
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
        assert_eq!(auto, selected);
        assert_eq!(public_scaffold, zero_payload);
        assert_ne!(auto, zero_payload);
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
