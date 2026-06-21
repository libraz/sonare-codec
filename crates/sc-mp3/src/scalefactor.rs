use super::*;

pub const MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT: usize = 21;

/// Number of boundary entries in an MPEG-1 Layer III long-block scale-factor
/// band index. The 23 boundaries delimit 22 spectral bands; scale factors are
/// transmitted for the first 21 (the highest band carries none).
pub const MPEG1_LAYER3_LONG_SCALEFACTOR_BAND_BOUNDARIES: usize = 23;

/// MPEG-1 Layer III long-block scale-factor band boundaries at 44.1 kHz
/// (ISO/IEC 11172-3 Annex B, `sfBandIndex`). Entry `b` is the first spectral
/// line of band `b`; the final entry (576) terminates the last band.
pub(crate) const MPEG1_LAYER3_LONG_SFB_44100: [u16; MPEG1_LAYER3_LONG_SCALEFACTOR_BAND_BOUNDARIES] = [
    0, 4, 8, 12, 16, 20, 24, 30, 36, 44, 52, 62, 74, 90, 110, 134, 162, 196, 238, 288, 342, 418,
    576,
];

/// MPEG-1 Layer III long-block scale-factor band boundaries at 48 kHz.
pub(crate) const MPEG1_LAYER3_LONG_SFB_48000: [u16; MPEG1_LAYER3_LONG_SCALEFACTOR_BAND_BOUNDARIES] = [
    0, 4, 8, 12, 16, 20, 24, 30, 36, 42, 50, 60, 72, 88, 106, 128, 156, 190, 230, 276, 330, 384,
    576,
];

/// MPEG-1 Layer III long-block scale-factor band boundaries at 32 kHz.
pub(crate) const MPEG1_LAYER3_LONG_SFB_32000: [u16; MPEG1_LAYER3_LONG_SCALEFACTOR_BAND_BOUNDARIES] = [
    0, 4, 8, 12, 16, 20, 24, 30, 36, 44, 54, 66, 82, 102, 126, 156, 194, 240, 296, 364, 448, 550,
    576,
];

/// Returns the MPEG-1 Layer III long-block scale-factor band index for the
/// given sample rate (ISO/IEC 11172-3 Annex B). Only the three MPEG-1 rates
/// are defined; other rates are rejected.
pub fn mpeg1_layer3_long_scalefactor_band_index(
    sample_rate: u32,
) -> Result<&'static [u16; MPEG1_LAYER3_LONG_SCALEFACTOR_BAND_BOUNDARIES], Error> {
    match sample_rate {
        44_100 => Ok(&MPEG1_LAYER3_LONG_SFB_44100),
        48_000 => Ok(&MPEG1_LAYER3_LONG_SFB_48000),
        32_000 => Ok(&MPEG1_LAYER3_LONG_SFB_32000),
        _ => Err(Error::InvalidInput(
            "MP3 long-block scale-factor band index undefined for sample rate",
        )),
    }
}

/// Returns the `[start, end)` spectral-line range of one long-block transmitted
/// scale-factor band (`band` in `0..MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT`) at
/// the given sample rate.
pub fn mpeg1_layer3_long_scalefactor_band_range(
    band: usize,
    sample_rate: u32,
) -> Result<(usize, usize), Error> {
    if band >= MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT {
        return Err(Error::InvalidInput(
            "MP3 long-block scale-factor band index out of range",
        ));
    }
    let index = mpeg1_layer3_long_scalefactor_band_index(sample_rate)?;
    Ok((usize::from(index[band]), usize::from(index[band + 1])))
}

/// MPEG-2 LSF Layer III long-block scale-factor band boundaries at 22.05 kHz
/// (ISO/IEC 13818-3 Table B.8, `sfBandIndex`). The low-sampling-frequency
/// extension keeps the same 22-band / 21-transmitted-factor layout as MPEG-1.
/// 16 kHz shares this table; 24 kHz differs above band 12.
pub(crate) const MPEG2_LAYER3_LONG_SFB_22050: [u16; MPEG1_LAYER3_LONG_SCALEFACTOR_BAND_BOUNDARIES] = [
    0, 6, 12, 18, 24, 30, 36, 44, 54, 66, 80, 96, 116, 140, 168, 200, 238, 284, 336, 396, 464, 522,
    576,
];

/// MPEG-2 LSF Layer III long-block scale-factor band boundaries at 24 kHz
/// (ISO/IEC 13818-3 Table B.8, `sfBandIndex`).
pub(crate) const MPEG2_LAYER3_LONG_SFB_24000: [u16; MPEG1_LAYER3_LONG_SCALEFACTOR_BAND_BOUNDARIES] = [
    0, 6, 12, 18, 24, 30, 36, 44, 54, 66, 80, 96, 114, 136, 162, 194, 232, 278, 332, 394, 464, 540,
    576,
];

/// MPEG-2 LSF Layer III long-block scale-factor band boundaries at 16 kHz
/// (ISO/IEC 13818-3 Table B.8, `sfBandIndex`). Identical to the 22.05 kHz table.
pub(crate) const MPEG2_LAYER3_LONG_SFB_16000: [u16; MPEG1_LAYER3_LONG_SCALEFACTOR_BAND_BOUNDARIES] =
    MPEG2_LAYER3_LONG_SFB_22050;

/// Returns the Layer III long-block scale-factor band index for any sample rate
/// defined by the MPEG-1 (ISO/IEC 11172-3) or MPEG-2 LSF (ISO/IEC 13818-3)
/// specifications. MPEG-2.5 rates (8/11.025/12 kHz) are outside both ISO
/// specifications and are not covered here.
pub fn layer3_long_scalefactor_band_index(
    sample_rate: u32,
) -> Result<&'static [u16; MPEG1_LAYER3_LONG_SCALEFACTOR_BAND_BOUNDARIES], Error> {
    match sample_rate {
        32_000 | 44_100 | 48_000 => mpeg1_layer3_long_scalefactor_band_index(sample_rate),
        22_050 => Ok(&MPEG2_LAYER3_LONG_SFB_22050),
        24_000 => Ok(&MPEG2_LAYER3_LONG_SFB_24000),
        16_000 => Ok(&MPEG2_LAYER3_LONG_SFB_16000),
        _ => Err(Error::InvalidInput(
            "MP3 long-block scale-factor band index undefined for sample rate",
        )),
    }
}

/// Returns the `[start, end)` spectral-line range of one long-block transmitted
/// scale-factor band at any MPEG-1 or MPEG-2 LSF sample rate.
pub fn layer3_long_scalefactor_band_range(
    band: usize,
    sample_rate: u32,
) -> Result<(usize, usize), Error> {
    if band >= MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT {
        return Err(Error::InvalidInput(
            "MP3 long-block scale-factor band index out of range",
        ));
    }
    let index = layer3_long_scalefactor_band_index(sample_rate)?;
    Ok((usize::from(index[band]), usize::from(index[band + 1])))
}

/// Number of boundary entries in a Layer III short-block scale-factor band
/// index. The 14 boundaries delimit 13 short bands, each measured in
/// window-local spectral lines (0..192).
pub const LAYER3_SHORT_SCALEFACTOR_BAND_BOUNDARIES: usize = 14;

/// Number of spectral lines one short-block window spans (576 / 3).
pub const LAYER3_SHORT_WINDOW_LINES: usize = 192;

/// Number of frequency lines in one granule.
pub const LAYER3_GRANULE_LINES: usize = 576;

/// Number of short-block analysis windows per granule.
pub const LAYER3_SHORT_WINDOWS: usize = 3;

/// Number of short scale-factor bands that carry a transmitted scale factor per
/// window. The highest short band (index 12) carries none and is quantized flat,
/// mirroring the long-block residual band (ISO/IEC 11172-3 §2.4.2.7).
pub const LAYER3_SHORT_SCALE_FACTOR_BANDS: usize = 12;

/// MPEG-1 Layer III short-block scale-factor band boundaries at 44.1 kHz
/// (ISO/IEC 11172-3 Annex B, `sfBandIndex[].s`). Entry `b` is the first
/// window-local line of short band `b`; the final entry (192) terminates the
/// last band. The same partition applies independently to all three windows.
pub(crate) const MPEG1_LAYER3_SHORT_SFB_44100: [u16; LAYER3_SHORT_SCALEFACTOR_BAND_BOUNDARIES] =
    [0, 4, 8, 12, 16, 22, 30, 40, 52, 66, 84, 106, 136, 192];

/// MPEG-1 Layer III short-block scale-factor band boundaries at 48 kHz.
pub(crate) const MPEG1_LAYER3_SHORT_SFB_48000: [u16; LAYER3_SHORT_SCALEFACTOR_BAND_BOUNDARIES] =
    [0, 4, 8, 12, 16, 22, 28, 38, 50, 64, 80, 100, 126, 192];

/// MPEG-1 Layer III short-block scale-factor band boundaries at 32 kHz.
pub(crate) const MPEG1_LAYER3_SHORT_SFB_32000: [u16; LAYER3_SHORT_SCALEFACTOR_BAND_BOUNDARIES] =
    [0, 4, 8, 12, 16, 22, 30, 42, 58, 78, 104, 138, 180, 192];

/// MPEG-2 LSF Layer III short-block scale-factor band boundaries at 22.05 kHz
/// (ISO/IEC 13818-3 Table B.8, `sfBandIndex[].s`).
pub(crate) const MPEG2_LAYER3_SHORT_SFB_22050: [u16; LAYER3_SHORT_SCALEFACTOR_BAND_BOUNDARIES] =
    [0, 4, 8, 12, 18, 24, 32, 42, 56, 74, 100, 132, 174, 192];

/// MPEG-2 LSF Layer III short-block scale-factor band boundaries at 24 kHz
/// (ISO/IEC 13818-3 Table B.8, `sfBandIndex[].s`).
pub(crate) const MPEG2_LAYER3_SHORT_SFB_24000: [u16; LAYER3_SHORT_SCALEFACTOR_BAND_BOUNDARIES] =
    [0, 4, 8, 12, 18, 26, 36, 48, 62, 80, 104, 136, 180, 192];

/// MPEG-2 LSF Layer III short-block scale-factor band boundaries at 16 kHz
/// (ISO/IEC 13818-3 Table B.8, `sfBandIndex[].s`).
pub(crate) const MPEG2_LAYER3_SHORT_SFB_16000: [u16; LAYER3_SHORT_SCALEFACTOR_BAND_BOUNDARIES] =
    [0, 4, 8, 12, 18, 26, 36, 48, 62, 80, 104, 134, 174, 192];

/// Returns the Layer III short-block scale-factor band index for any sample
/// rate defined by the MPEG-1 (ISO/IEC 11172-3) or MPEG-2 LSF (ISO/IEC
/// 13818-3) specifications. MPEG-2.5 rates are outside both and are rejected.
pub fn layer3_short_scalefactor_band_index(
    sample_rate: u32,
) -> Result<&'static [u16; LAYER3_SHORT_SCALEFACTOR_BAND_BOUNDARIES], Error> {
    match sample_rate {
        44_100 => Ok(&MPEG1_LAYER3_SHORT_SFB_44100),
        48_000 => Ok(&MPEG1_LAYER3_SHORT_SFB_48000),
        32_000 => Ok(&MPEG1_LAYER3_SHORT_SFB_32000),
        22_050 => Ok(&MPEG2_LAYER3_SHORT_SFB_22050),
        24_000 => Ok(&MPEG2_LAYER3_SHORT_SFB_24000),
        16_000 => Ok(&MPEG2_LAYER3_SHORT_SFB_16000),
        _ => Err(Error::InvalidInput(
            "MP3 short-block scale-factor band index undefined for sample rate",
        )),
    }
}

/// Builds the Layer III short-block reorder map for the given sample rate.
///
/// The short-block hybrid filterbank emits spectral lines window-major within
/// each subband: `raw[sb*18 + w*6 + line]` holds window `w`'s line for global
/// frequency line `sb*6 + line`. The bitstream, however, groups the lines by
/// scale-factor band and then by window (ISO/IEC 11172-3 §2.4.3.4.6), so that
/// each short band's three windows are coded together.
///
/// The returned gather map has `reordered[p] = raw[map[p]]`: position `p` in
/// bitstream order corresponds to `map[p]` in raw filterbank order. It is a
/// permutation of `0..576` and depends only on the sample-rate band table.
pub fn layer3_short_reorder_map(sample_rate: u32) -> Result<[usize; LAYER3_GRANULE_LINES], Error> {
    let index = layer3_short_scalefactor_band_index(sample_rate)?;
    let mut map = [0_usize; LAYER3_GRANULE_LINES];
    let mut pos = 0_usize;
    for band in 0..index.len() - 1 {
        let start = usize::from(index[band]);
        let width = usize::from(index[band + 1]) - start;
        for window in 0..LAYER3_SHORT_WINDOWS {
            for line in 0..width {
                let global_freq = start + line;
                let subband = global_freq / 6;
                let subband_line = global_freq % 6;
                let raw = subband * SHORT_BLOCK_LINES + window * 6 + subband_line;
                if pos >= LAYER3_GRANULE_LINES || raw >= LAYER3_GRANULE_LINES {
                    return Err(Error::InvalidInput(
                        "MP3 short-block reorder map exceeds granule bounds",
                    ));
                }
                map[pos] = raw;
                pos += 1;
            }
        }
    }
    if pos != LAYER3_GRANULE_LINES {
        return Err(Error::InvalidInput(
            "MP3 short-block scale-factor band index does not tile the granule",
        ));
    }
    Ok(map)
}

/// Returns the `[start, end)` range, in reordered bitstream lines, of one
/// short-block scale-factor band across all three windows. Short band `b`
/// occupies the reordered lines `[s[b]*3, s[b+1]*3)`.
pub fn layer3_short_scalefactor_band_range(
    band: usize,
    sample_rate: u32,
) -> Result<(usize, usize), Error> {
    let index = layer3_short_scalefactor_band_index(sample_rate)?;
    if band + 1 >= index.len() {
        return Err(Error::InvalidInput(
            "MP3 short-block scale-factor band index out of range",
        ));
    }
    let start = usize::from(index[band]) * LAYER3_SHORT_WINDOWS;
    let end = usize::from(index[band + 1]) * LAYER3_SHORT_WINDOWS;
    Ok((start, end))
}

pub const MPEG1_LAYER3_PCM_STEP_CANDIDATES: &[f32] = &[
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

pub const MPEG1_LAYER3_MONO_PRODUCTION_PCM_STEP_CANDIDATES: &[f32] = &[
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

pub fn mpeg1_layer3_production_pcm_step_candidates(channels: u16) -> Result<&'static [f32], Error> {
    match channels {
        1 => Ok(MPEG1_LAYER3_MONO_PRODUCTION_PCM_STEP_CANDIDATES),
        2 => Ok(MPEG1_LAYER3_PCM_STEP_CANDIDATES),
        _ => Err(Error::UnsupportedFeature(
            "MP3 production step candidates require mono/stereo",
        )),
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Layer3PcmFrameStepSelection {
    pub step: f32,
    pub payload_bit_len: usize,
    pub frame_capacity_bits: usize,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Layer3PerceptualCandidateProfile {
    pub step: f32,
    pub payload_bit_len: usize,
    pub frame_capacity_bits: usize,
    pub nonzero_scale_factors: usize,
    pub scale_factor_bands: usize,
    pub max_scale_factor: u8,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Layer3LowBandSpectralShapeCandidateProfile {
    pub step: f32,
    pub payload_bit_len: usize,
    pub frame_capacity_bits: usize,
    pub low_band_abs_sum: u64,
    pub total_abs_sum: u64,
    pub low_band_nonzero_lines: usize,
    pub total_nonzero_lines: usize,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Layer3BandSpectralShapeCandidateProfile {
    pub step: f32,
    pub payload_bit_len: usize,
    pub frame_capacity_bits: usize,
    pub band: usize,
    pub band_start: usize,
    pub band_end: usize,
    pub band_abs_sum: u64,
    pub band_nonzero_lines: usize,
    pub total_abs_sum: u64,
    pub total_nonzero_lines: usize,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Layer3QualityGuardedCandidateProfile {
    pub step: f32,
    pub payload_bit_len: usize,
    pub frame_capacity_bits: usize,
    pub perceptual_granules: usize,
    pub calibrated_granules: usize,
    pub quality_guard_compared_granules: usize,
    pub quality_guard_distortion_delta: f64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Layer3ScaleFactorBandBias {
    pub band_start: usize,
    pub band_end: usize,
    pub bias: i8,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Layer3QuantizedBandGain {
    pub band_start: usize,
    pub band_end: usize,
    pub gain: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Layer3PerceptualBitAllocation {
    pub frame_index: usize,
    pub granule: usize,
    pub channel: usize,
    pub perceptual_entropy: f64,
    pub target_bits: usize,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Layer3ReservoirFrameSelection {
    pub frame_index: usize,
    pub step: f32,
    pub payload_bit_len: usize,
    pub frame_len: usize,
    pub padding: bool,
    pub frame_capacity_bytes: usize,
    pub main_data_begin: usize,
    pub reservoir_after: usize,
    pub perceptual_granules: usize,
    pub calibrated_granules: usize,
    pub quality_guard_compared_granules: usize,
    pub quality_guard_distortion_delta: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Layer3EntropyTargetedReservoirFrameSelection {
    pub frame_index: usize,
    pub step: f32,
    pub payload_bit_len: usize,
    pub frame_len: usize,
    pub padding: bool,
    pub frame_capacity_bytes: usize,
    pub main_data_begin: usize,
    pub reservoir_after: usize,
    pub perceptual_granules: usize,
    pub calibrated_granules: usize,
    pub quality_guard_compared_granules: usize,
    pub quality_guard_distortion_delta: f64,
    pub entropy_target_bits: usize,
    pub used_entropy_target_budget: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Layer3EntropyTargetUtilizationProfile {
    pub frames: usize,
    pub used_entropy_target_frames: usize,
    pub payload_bits: usize,
    pub entropy_budget_bits: usize,
    pub utilization: f64,
    pub max_entropy_budget_slack_bits: usize,
}
