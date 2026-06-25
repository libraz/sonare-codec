use super::*;

#[derive(Default)]
pub struct Mp3Decoder;

impl Mp3Decoder {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Decoder for Mp3Decoder {
    fn decode(&mut self, input: &[u8]) -> Result<AudioBuffer, Error> {
        decode(input)
    }

    fn decode_stream(&mut self, chunk: &[u8]) -> Result<Option<AudioBuffer>, Error> {
        decode(chunk).map(Some)
    }
}

#[derive(Default)]
pub struct Mp3Encoder;

impl Mp3Encoder {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Encoder for Mp3Encoder {
    fn encode(&mut self, pcm: &AudioBuffer) -> Result<Vec<u8>, Error> {
        encode(pcm)
    }
}

pub fn decode(input: &[u8]) -> Result<AudioBuffer, Error> {
    decode_silent_layer3(input)
}

/// Reports whether [`encode`] handles `pcm` through a production-grade path.
///
/// `encode` accepts mono/stereo input at MPEG-1 (32/44.1/48 kHz) and MPEG-2 LSF
/// (16/22.05/24 kHz) sample rates; MPEG-2.5 rates (8/11.025/12 kHz) and channel
/// layouts beyond stereo are rejected by the header builder. This is the single
/// source of truth for that support set, so higher layers do not have to keep a
/// drifting copy of the rate list.
#[must_use]
pub fn supports_production_encode(pcm: &AudioBuffer) -> bool {
    matches!(pcm.channels, 1 | 2)
        && matches!(
            pcm.sample_rate,
            16_000 | 22_050 | 24_000 | 32_000 | 44_100 | 48_000
        )
}

/// Encodes mono/stereo PCM as MPEG-1 Layer III frames.
///
/// Silent input routes through the compact zero-spectral frame scaffold.
/// Non-silent mono input uses the psychoacoustic scale-factor long-block
/// scaffold with a constant-bitrate padding schedule and the bit-reservoir
/// packer. Non-silent mono routes through the entropy-targeted low-band
/// gain/global-gain-bias reservoir profile, while stereo keeps the
/// entropy-targeted perceptual reservoir selector. The quantizer and quality
/// proxy are still intentionally coarse, so full rate control, stereo
/// true-polyphase readiness, and VBR are still incomplete.
pub fn encode(pcm: &AudioBuffer) -> Result<Vec<u8>, Error> {
    if pcm.channels != 1 && pcm.channels != 2 {
        return Err(Error::UnsupportedFeature(
            "MP3 encode currently supports mono/stereo only",
        ));
    }

    // Guard the supported sample-rate matrix explicitly at the public boundary
    // rather than letting an unsupported rate fail deep in the header builder.
    if !matches!(
        pcm.sample_rate,
        16_000 | 22_050 | 24_000 | 32_000 | 44_100 | 48_000
    ) {
        return Err(Error::UnsupportedFeature(
            "MP3 encode supports MPEG-1 (32/44.1/48 kHz) and MPEG-2 LSF (16/22.05/24 kHz) sample rates only",
        ));
    }

    // MPEG-2 LSF rates (ISO/IEC 13818-3) use the single-granule calibrated-gain
    // path.
    if matches!(pcm.sample_rate, 16_000 | 22_050 | 24_000) {
        // Correlated MPEG-2 LSF stereo is coded as MS joint stereo, mirroring the
        // MPEG-1 path: the near-silent side channel codes cheaply and decorrelated
        // stereo stays independent so it never regresses.
        if should_encode_stereo_as_mid_side(pcm)? {
            return encode_mpeg2_layer3_pcm_frames_with_auto_step_mid_side_and_table_provider(
                pcm,
                MPEG2_LAYER3_DEFAULT_BITRATE_KBPS,
                MPEG1_LAYER3_PCM_STEP_CANDIDATES,
                mpeg1_layer3_standard_table_provider(),
            );
        }
        return encode_mpeg2_layer3_pcm_frames_with_auto_step_and_table_provider(
            pcm,
            MPEG2_LAYER3_DEFAULT_BITRATE_KBPS,
            MPEG1_LAYER3_PCM_STEP_CANDIDATES,
            mpeg1_layer3_standard_table_provider(),
        );
    }

    if pcm.samples.iter().any(|sample| *sample != 0.0) && pcm.channels == 1 {
        encode_mpeg1_layer3_pcm_frames_with_entropy_targeted_perceptual_quantized_band_gain_and_global_gain_bias_reservoir_and_table_provider(
            pcm,
            &[2.0],
            128,
            false,
            0,
            Layer3QuantizedBandGain {
                band_start: 0,
                band_end: 7,
                gain: 1.5,
            },
            -4,
            mpeg1_layer3_standard_table_provider(),
        )
    } else if pcm.samples.iter().any(|sample| *sample != 0.0) {
        // Correlated stereo is coded as MS joint stereo so the near-silent side
        // channel codes cheaply; decorrelated stereo stays independent and never
        // regresses. The decision is made once for the whole stream.
        if should_encode_stereo_as_mid_side(pcm)? {
            encode_mpeg1_layer3_pcm_frames_with_entropy_targeted_perceptual_reservoir_mid_side_and_table_provider(
                pcm,
                mpeg1_layer3_production_pcm_step_candidates(pcm.channels)?,
                128,
                false,
                0,
                mpeg1_layer3_standard_table_provider(),
            )
        } else {
            encode_mpeg1_layer3_pcm_frames_with_entropy_targeted_perceptual_reservoir_and_table_provider(
                pcm,
                mpeg1_layer3_production_pcm_step_candidates(pcm.channels)?,
                128,
                false,
                0,
                mpeg1_layer3_standard_table_provider(),
            )
        }
    } else {
        encode_mpeg1_layer3_pcm_frames_with_selected_scale_factors_and_table_provider(
            pcm,
            1.0,
            Layer3EntropyTableProvider::default(),
        )
    }
}

/// Encodes PCM through the experimental MPEG-1 Layer III long-block frame scaffold.
pub fn encode_mpeg1_layer3_pcm_frames_with_selected_scale_factors(
    pcm: &AudioBuffer,
    step: f32,
    tables: Layer3EntropyTables<'_>,
) -> Result<Vec<u8>, Error> {
    let header = mpeg1_layer3_header_for_pcm(pcm)?;
    encode_mpeg1_layer3_pcm_frames_with_header_and_selected_scale_factors(header, pcm, step, tables)
}

/// Encodes PCM through the experimental MPEG-1 Layer III frame scaffold using provider lookup.
pub fn encode_mpeg1_layer3_pcm_frames_with_selected_scale_factors_and_table_provider(
    pcm: &AudioBuffer,
    step: f32,
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<Vec<u8>, Error> {
    let header = mpeg1_layer3_header_for_pcm(pcm)?;
    encode_mpeg1_layer3_pcm_frames_with_header_and_selected_scale_factors_and_table_provider(
        header, pcm, step, provider,
    )
}

/// Default constant bitrate (kbit/s) for the MPEG-2 LSF Layer III path. A valid
/// MPEG-2 Layer III bitrate that the per-frame step search can comfortably fit
/// across 16/22.05/24 kHz mono and stereo.
pub const MPEG2_LAYER3_DEFAULT_BITRATE_KBPS: u16 = 64;

/// Encodes PCM as MPEG-2 LSF (ISO/IEC 13818-3) Layer III frames.
///
/// The low-sampling-frequency extension carries a single 576-sample granule per
/// frame and an 8-bit `main_data_begin`. This path reuses the version-agnostic
/// calibrated-gain frame assembler with a per-frame quantizer-step search so the
/// main data always fits one constant-bitrate frame (`main_data_begin = 0`).
/// Sample rate must be an MPEG-2 LSF rate (16/22.05/24 kHz); MPEG-2.5 rates
/// (8/11.025/12 kHz) are outside ISO/IEC 11172-3 and 13818-3 and are rejected.
pub fn encode_mpeg2_layer3_pcm_frames_with_auto_step_and_table_provider(
    pcm: &AudioBuffer,
    bitrate_kbps: u16,
    candidates: &[f32],
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<Vec<u8>, Error> {
    let header = mpeg2_layer3_header_for_pcm(pcm, bitrate_kbps)?;
    encode_mpeg1_layer3_pcm_frames_with_header_and_auto_step_and_table_provider(
        header, pcm, candidates, provider,
    )
}

/// Encodes PCM through the long-block frame scaffold with psychoacoustic scale factors.
///
/// This helper exposes the perceptual scale-factor path for integration tests
/// and rate-control experiments. The public production `encode()` path remains
/// on the calibrated-gain route until this path is validated against the
/// FFmpeg-backed readiness oracle and frame-budget search.
pub fn encode_mpeg1_layer3_pcm_frames_with_perceptual_scale_factors_and_table_provider(
    pcm: &AudioBuffer,
    step: f32,
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<Vec<u8>, Error> {
    let header = mpeg1_layer3_header_for_pcm(pcm)?;
    encode_mpeg1_layer3_pcm_frames_with_header_and_perceptual_scale_factors_and_table_provider(
        header, pcm, step, provider,
    )
}

/// Encodes PCM through the perceptual scale-factor path with explicit
/// `scalefac_scale`.
///
/// This diagnostic helper lets rate-control work compare the normal 0.5-step
/// scale-factor attenuation (`false`) with the coarser 1.0-step attenuation
/// (`true`) before changing production selection.
pub fn encode_mpeg1_layer3_pcm_frames_with_perceptual_scalefac_scale_and_table_provider(
    pcm: &AudioBuffer,
    step: f32,
    scalefac_scale: bool,
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<Vec<u8>, Error> {
    let header = mpeg1_layer3_header_for_pcm(pcm)?;
    encode_mpeg1_layer3_pcm_frames_with_header_and_perceptual_scalefac_scale_and_table_provider(
        header,
        pcm,
        step,
        scalefac_scale,
        provider,
    )
}

/// Encodes PCM through the perceptual scale-factor path with an explicit
/// allowed-noise multiplier.
pub fn encode_mpeg1_layer3_pcm_frames_with_perceptual_allowed_noise_scale_and_table_provider(
    pcm: &AudioBuffer,
    step: f32,
    allowed_noise_scale: f64,
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<Vec<u8>, Error> {
    let header = mpeg1_layer3_header_for_pcm(pcm)?;
    encode_mpeg1_layer3_pcm_frames_with_header_and_perceptual_allowed_noise_scale_and_table_provider(
        header,
        pcm,
        step,
        allowed_noise_scale,
        provider,
    )
}

/// Encodes PCM through the perceptual scale-factor path with a diagnostic
/// per-band scale-factor bias applied after allocation.
pub fn encode_mpeg1_layer3_pcm_frames_with_perceptual_scale_factor_band_bias_and_table_provider(
    pcm: &AudioBuffer,
    step: f32,
    band_bias: Layer3ScaleFactorBandBias,
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<Vec<u8>, Error> {
    let header = mpeg1_layer3_header_for_pcm(pcm)?;
    encode_mpeg1_layer3_pcm_frames_with_header_and_perceptual_scale_factor_band_bias_and_table_provider(
        header, pcm, step, band_bias, provider,
    )
}

/// Encodes PCM through the perceptual scale-factor path with a diagnostic
/// per-band gain applied to quantized spectral coefficients after allocation.
pub fn encode_mpeg1_layer3_pcm_frames_with_perceptual_quantized_band_gain_and_table_provider(
    pcm: &AudioBuffer,
    step: f32,
    band_gain: Layer3QuantizedBandGain,
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<Vec<u8>, Error> {
    let header = mpeg1_layer3_header_for_pcm(pcm)?;
    encode_mpeg1_layer3_pcm_frames_with_header_and_perceptual_quantized_band_gain_and_table_provider(
        header, pcm, step, band_gain, provider,
    )
}

/// Encodes PCM through the perceptual scale-factor path with diagnostic
/// quantized band gain and a global-gain bias.
pub fn encode_mpeg1_layer3_pcm_frames_with_perceptual_quantized_band_gain_and_global_gain_bias_and_table_provider(
    pcm: &AudioBuffer,
    step: f32,
    band_gain: Layer3QuantizedBandGain,
    global_gain_bias: i16,
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<Vec<u8>, Error> {
    let header = mpeg1_layer3_header_for_pcm(pcm)?;
    encode_mpeg1_layer3_pcm_frames_with_header_and_perceptual_quantized_band_gain_and_global_gain_bias_and_table_provider(
        header,
        pcm,
        step,
        band_gain,
        global_gain_bias,
        provider,
    )
}

/// Encodes PCM through the perceptual scale-factor path with per-frame step search.
pub fn encode_mpeg1_layer3_pcm_frames_with_perceptual_auto_step_and_table_provider(
    pcm: &AudioBuffer,
    candidates: &[f32],
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<Vec<u8>, Error> {
    let header = mpeg1_layer3_header_for_pcm(pcm)?;
    encode_mpeg1_layer3_pcm_frames_with_header_and_perceptual_auto_step_and_table_provider(
        header, pcm, candidates, provider,
    )
}

/// Encodes PCM through the perceptual scale-factor path with a payload bit budget.
pub fn encode_mpeg1_layer3_pcm_frames_with_perceptual_max_payload_bits_and_table_provider(
    pcm: &AudioBuffer,
    candidates: &[f32],
    max_payload_bit_len: usize,
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<Vec<u8>, Error> {
    let header = mpeg1_layer3_header_for_pcm(pcm)?;
    encode_mpeg1_layer3_pcm_frames_with_header_and_perceptual_max_payload_bits_and_table_provider(
        header,
        pcm,
        candidates,
        max_payload_bit_len,
        provider,
    )
}

/// Encodes PCM through the perceptual scale-factor path using a caller-selected bitrate.
///
/// This mirrors the calibrated-gain bitrate helper, but drives the
/// psychoacoustic scale-factor workbench so callers can evaluate the future
/// perceptual rate-control path with header-derived frame capacity.
pub fn encode_mpeg1_layer3_pcm_frames_with_perceptual_bitrate_and_table_provider(
    pcm: &AudioBuffer,
    candidates: &[f32],
    bitrate_kbps: u16,
    padding: bool,
    crc_protected: bool,
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<Vec<u8>, Error> {
    let header = layer3_header_for_capacity(
        pcm.sample_rate,
        pcm.channels,
        bitrate_kbps,
        padding,
        crc_protected,
    )?;
    encode_mpeg1_layer3_pcm_frames_with_header_and_perceptual_auto_step_and_table_provider(
        header, pcm, candidates, provider,
    )
}

/// Encodes PCM through the perceptual scale-factor path with a CBR padding schedule.
///
/// Padding is derived from the requested MPEG-1 Layer III bitrate for each
/// frame, while each frame still selects a perceptual quantizer step that fits
/// the actual header capacity for that frame.
pub fn encode_mpeg1_layer3_pcm_frames_with_perceptual_cbr_bitrate_and_table_provider(
    pcm: &AudioBuffer,
    candidates: &[f32],
    bitrate_kbps: u16,
    crc_protected: bool,
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<Vec<u8>, Error> {
    let base_header = layer3_header_for_capacity(
        pcm.sample_rate,
        pcm.channels,
        bitrate_kbps,
        false,
        crc_protected,
    )?;
    encode_mpeg1_layer3_pcm_frames_with_header_perceptual_cbr_padding_and_table_provider(
        base_header,
        pcm,
        candidates,
        provider,
    )
}

/// Encodes PCM through the perceptual scale-factor path with CBR padding,
/// preferring quantizer steps that produce non-zero scale-factor allocation.
///
/// This keeps the experimental perceptual path separate from production
/// `encode()`, but gives the rate-control workbench a selector that does not
/// collapse to the finest zero-allocation candidate when the bit budget has
/// ample headroom.
pub fn encode_mpeg1_layer3_pcm_frames_with_perceptual_active_cbr_bitrate_and_table_provider(
    pcm: &AudioBuffer,
    candidates: &[f32],
    bitrate_kbps: u16,
    crc_protected: bool,
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<Vec<u8>, Error> {
    let base_header = layer3_header_for_capacity(
        pcm.sample_rate,
        pcm.channels,
        bitrate_kbps,
        false,
        crc_protected,
    )?;
    encode_mpeg1_layer3_pcm_frames_with_header_perceptual_active_cbr_padding_and_table_provider(
        base_header,
        pcm,
        candidates,
        provider,
    )
}

/// Encodes PCM through the frame scaffold, selecting one quantizer step per frame.
pub fn encode_mpeg1_layer3_pcm_frames_with_auto_step_and_table_provider(
    pcm: &AudioBuffer,
    candidates: &[f32],
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<Vec<u8>, Error> {
    let header = mpeg1_layer3_header_for_pcm(pcm)?;
    encode_mpeg1_layer3_pcm_frames_with_header_cbr_padding_and_table_provider(
        header, pcm, candidates, provider,
    )
}

/// Encodes PCM through the frame scaffold, selecting each frame within an
/// explicit Layer III payload bit budget.
pub fn encode_mpeg1_layer3_pcm_frames_with_max_payload_bits_and_table_provider(
    pcm: &AudioBuffer,
    candidates: &[f32],
    max_payload_bit_len: usize,
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<Vec<u8>, Error> {
    let header = mpeg1_layer3_header_for_pcm(pcm)?;
    encode_mpeg1_layer3_pcm_frames_with_header_and_max_payload_bits_and_table_provider(
        header,
        pcm,
        candidates,
        max_payload_bit_len,
        provider,
    )
}

/// Encodes PCM through the frame scaffold using a caller-selected Layer III bitrate.
///
/// This builds the MPEG header and derives each frame's main-data capacity from
/// that header, so callers can drive the existing per-frame step search from a
/// bitrate without duplicating header/capacity calculations.
pub fn encode_mpeg1_layer3_pcm_frames_with_bitrate_and_table_provider(
    pcm: &AudioBuffer,
    candidates: &[f32],
    bitrate_kbps: u16,
    padding: bool,
    crc_protected: bool,
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<Vec<u8>, Error> {
    let header = layer3_header_for_capacity(
        pcm.sample_rate,
        pcm.channels,
        bitrate_kbps,
        padding,
        crc_protected,
    )?;
    encode_mpeg1_layer3_pcm_frames_with_header_and_auto_step_and_table_provider(
        header, pcm, candidates, provider,
    )
}

/// Encodes PCM using a caller-selected Layer III bitrate and CBR padding schedule.
///
/// MPEG-1 Layer III CBR at rates such as 44.1kHz needs some frames to carry one
/// padding slot so the average frame length reaches the requested bitrate. This
/// helper derives each frame header from an integer slot accumulator instead of
/// forcing callers to precompute per-frame padding.
pub fn encode_mpeg1_layer3_pcm_frames_with_cbr_bitrate_and_table_provider(
    pcm: &AudioBuffer,
    candidates: &[f32],
    bitrate_kbps: u16,
    crc_protected: bool,
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<Vec<u8>, Error> {
    let base_header = layer3_header_for_capacity(
        pcm.sample_rate,
        pcm.channels,
        bitrate_kbps,
        false,
        crc_protected,
    )?;
    encode_mpeg1_layer3_pcm_frames_with_header_cbr_padding_and_table_provider(
        base_header,
        pcm,
        candidates,
        provider,
    )
}

/// Encodes PCM with an explicit MPEG-1 Layer III header through the frame scaffold.
pub fn encode_mpeg1_layer3_pcm_frames_with_header_and_selected_scale_factors(
    header: FrameHeader,
    pcm: &AudioBuffer,
    step: f32,
    tables: Layer3EntropyTables<'_>,
) -> Result<Vec<u8>, Error> {
    let frame_count = layer3_frame_count(header, pcm)?;
    let mut out = Vec::with_capacity(header.frame_len() * frame_count);
    for frame_index in 0..frame_count {
        let start_frame = frame_index
            .checked_mul(usize::from(header.samples_per_frame()))
            .ok_or(Error::InvalidInput("MP3 frame start overflows"))?;
        out.extend_from_slice(
            &assemble_mpeg1_layer3_pcm_frame_with_selected_scale_factors(
                header,
                pcm,
                start_frame,
                step,
                tables,
            )?,
        );
    }
    Ok(out)
}

/// Encodes PCM with an explicit MPEG-1 Layer III header and per-frame step search.
pub fn encode_mpeg1_layer3_pcm_frames_with_header_and_auto_step_and_table_provider(
    header: FrameHeader,
    pcm: &AudioBuffer,
    candidates: &[f32],
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<Vec<u8>, Error> {
    let frame_count = layer3_frame_count(header, pcm)?;
    let mut out = Vec::with_capacity(header.frame_len() * frame_count);
    for frame_index in 0..frame_count {
        let start_frame = frame_index
            .checked_mul(usize::from(header.samples_per_frame()))
            .ok_or(Error::InvalidInput("MP3 frame start overflows"))?;
        let step = select_mpeg1_layer3_pcm_frame_step_with_table_provider(
            header,
            pcm,
            start_frame,
            candidates,
            provider,
        )?;
        out.extend_from_slice(
            &assemble_mpeg1_layer3_pcm_frame_with_selected_scale_factors_and_table_provider(
                header,
                pcm,
                start_frame,
                step,
                provider,
            )?,
        );
    }
    Ok(out)
}

/// Encodes PCM with an explicit MPEG-1 Layer III header and per-frame step
/// search constrained by a caller-provided payload bit budget.
pub fn encode_mpeg1_layer3_pcm_frames_with_header_and_max_payload_bits_and_table_provider(
    header: FrameHeader,
    pcm: &AudioBuffer,
    candidates: &[f32],
    max_payload_bit_len: usize,
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<Vec<u8>, Error> {
    let frame_count = layer3_frame_count(header, pcm)?;
    let mut out = Vec::with_capacity(header.frame_len() * frame_count);
    for frame_index in 0..frame_count {
        let start_frame = frame_index
            .checked_mul(usize::from(header.samples_per_frame()))
            .ok_or(Error::InvalidInput("MP3 frame start overflows"))?;
        let step = select_mpeg1_layer3_pcm_frame_step_with_max_payload_bits_and_table_provider(
            header,
            pcm,
            start_frame,
            candidates,
            max_payload_bit_len,
            provider,
        )?;
        out.extend_from_slice(
            &assemble_mpeg1_layer3_pcm_frame_with_selected_scale_factors_and_table_provider(
                header,
                pcm,
                start_frame,
                step,
                provider,
            )?,
        );
    }
    Ok(out)
}

/// Encodes PCM with per-frame CBR padding derived from the header bitrate.
pub fn encode_mpeg1_layer3_pcm_frames_with_header_cbr_padding_and_table_provider(
    header: FrameHeader,
    pcm: &AudioBuffer,
    candidates: &[f32],
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<Vec<u8>, Error> {
    let frame_count = layer3_frame_count(header, pcm)?;
    let mut padding = Layer3PaddingSchedule::new(header)?;
    let mut out = Vec::with_capacity(header.frame_len() * frame_count + frame_count);
    for frame_index in 0..frame_count {
        let start_frame = frame_index
            .checked_mul(usize::from(header.samples_per_frame()))
            .ok_or(Error::InvalidInput("MP3 frame start overflows"))?;
        let frame_header = padding.next_header();
        let step = select_mpeg1_layer3_pcm_frame_step_with_table_provider(
            frame_header,
            pcm,
            start_frame,
            candidates,
            provider,
        )?;
        out.extend_from_slice(
            &assemble_mpeg1_layer3_pcm_frame_with_selected_scale_factors_and_table_provider(
                frame_header,
                pcm,
                start_frame,
                step,
                provider,
            )?,
        );
    }
    Ok(out)
}
