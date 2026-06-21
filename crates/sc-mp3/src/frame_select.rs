use super::*;

pub(crate) fn assemble_mpeg1_layer3_reservoir_frames(
    frames: Vec<Layer3ReservoirFrame>,
) -> Result<Vec<u8>, Error> {
    // Pass 2: lay each payload into the shared main-data stream at the byte its
    // `main_data_begin` resolves to, then slice the stream back into frame slots.
    let mut total_slots = 0_usize;
    for frame in &frames {
        total_slots = total_slots
            .checked_add(frame.capacity)
            .ok_or(Error::InvalidInput("MP3 total main-data size overflows"))?;
    }
    let mut stream = vec![0_u8; total_slots];
    let mut slot_start = 0_usize;
    for frame in &frames {
        let payload_start =
            slot_start
                .checked_sub(frame.main_data_begin)
                .ok_or(Error::InvalidInput(
                    "MP3 main_data_begin precedes stream start",
                ))?;
        let payload_end = payload_start
            .checked_add(frame.payload.len())
            .filter(|end| *end <= stream.len())
            .ok_or(Error::InvalidInput(
                "MP3 payload overflows main-data stream",
            ))?;
        stream[payload_start..payload_end].copy_from_slice(&frame.payload);
        slot_start = slot_start
            .checked_add(frame.capacity)
            .ok_or(Error::InvalidInput("MP3 main-data stream overflows"))?;
    }

    let mut out = Vec::with_capacity(total_slots + frames.len() * 64);
    let mut slot_start = 0_usize;
    for frame in &frames {
        let slot = &stream[slot_start..slot_start + frame.capacity];
        out.extend_from_slice(&assemble_layer3_frame(
            frame.header,
            &frame.side_info,
            slot,
        )?);
        slot_start += frame.capacity;
    }
    Ok(out)
}

/// Encodes PCM with an explicit MPEG-1 Layer III header using provider lookup.
pub fn encode_mpeg1_layer3_pcm_frames_with_header_and_selected_scale_factors_and_table_provider(
    header: FrameHeader,
    pcm: &AudioBuffer,
    step: f32,
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<Vec<u8>, Error> {
    let frame_count = layer3_frame_count(header, pcm)?;
    let mut out = Vec::with_capacity(header.frame_len() * frame_count);
    for frame_index in 0..frame_count {
        let start_frame = frame_index
            .checked_mul(usize::from(header.samples_per_frame()))
            .ok_or(Error::InvalidInput("MP3 frame start overflows"))?;
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

/// Encodes PCM with an explicit MPEG-1 Layer III header and psychoacoustic scale factors.
pub fn encode_mpeg1_layer3_pcm_frames_with_header_and_perceptual_scale_factors_and_table_provider(
    header: FrameHeader,
    pcm: &AudioBuffer,
    step: f32,
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<Vec<u8>, Error> {
    encode_mpeg1_layer3_pcm_frames_with_header_and_perceptual_scalefac_scale_and_table_provider(
        header, pcm, step, false, provider,
    )
}

/// Encodes PCM with an explicit MPEG-1 Layer III header, psychoacoustic scale
/// factors, and caller-selected `scalefac_scale`.
pub fn encode_mpeg1_layer3_pcm_frames_with_header_and_perceptual_scalefac_scale_and_table_provider(
    header: FrameHeader,
    pcm: &AudioBuffer,
    step: f32,
    scalefac_scale: bool,
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<Vec<u8>, Error> {
    let frame_count = layer3_frame_count(header, pcm)?;
    let mut out = Vec::with_capacity(header.frame_len() * frame_count);
    for frame_index in 0..frame_count {
        let start_frame = frame_index
            .checked_mul(usize::from(header.samples_per_frame()))
            .ok_or(Error::InvalidInput("MP3 frame start overflows"))?;
        out.extend_from_slice(
            &assemble_mpeg1_layer3_pcm_frame_with_perceptual_scalefac_scale_and_table_provider(
                header,
                pcm,
                start_frame,
                step,
                scalefac_scale,
                provider,
            )?,
        );
    }
    Ok(out)
}

/// Encodes PCM with psychoacoustic scale factors and a caller-selected
/// allowed-noise multiplier.
pub fn encode_mpeg1_layer3_pcm_frames_with_header_and_perceptual_allowed_noise_scale_and_table_provider(
    header: FrameHeader,
    pcm: &AudioBuffer,
    step: f32,
    allowed_noise_scale: f64,
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<Vec<u8>, Error> {
    let frame_count = layer3_frame_count(header, pcm)?;
    let mut out = Vec::with_capacity(header.frame_len() * frame_count);
    for frame_index in 0..frame_count {
        let start_frame = frame_index
            .checked_mul(usize::from(header.samples_per_frame()))
            .ok_or(Error::InvalidInput("MP3 frame start overflows"))?;
        out.extend_from_slice(
            &assemble_mpeg1_layer3_pcm_frame_with_perceptual_allowed_noise_scale_and_table_provider(
                header,
                pcm,
                start_frame,
                step,
                allowed_noise_scale,
                provider,
            )?,
        );
    }
    Ok(out)
}

/// Encodes PCM with an explicit MPEG-1 Layer III header and diagnostic
/// per-band scale-factor bias.
pub fn encode_mpeg1_layer3_pcm_frames_with_header_and_perceptual_scale_factor_band_bias_and_table_provider(
    header: FrameHeader,
    pcm: &AudioBuffer,
    step: f32,
    band_bias: Layer3ScaleFactorBandBias,
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<Vec<u8>, Error> {
    let frame_count = layer3_frame_count(header, pcm)?;
    let mut out = Vec::with_capacity(header.frame_len() * frame_count);
    for frame_index in 0..frame_count {
        let start_frame = frame_index
            .checked_mul(usize::from(header.samples_per_frame()))
            .ok_or(Error::InvalidInput("MP3 frame start overflows"))?;
        out.extend_from_slice(
            &assemble_mpeg1_layer3_pcm_frame_with_perceptual_scale_factor_band_bias_and_table_provider(
                header,
                pcm,
                start_frame,
                step,
                band_bias,
                provider,
            )?,
        );
    }
    Ok(out)
}

/// Encodes PCM with an explicit MPEG-1 Layer III header and diagnostic
/// per-band quantized coefficient gain.
pub fn encode_mpeg1_layer3_pcm_frames_with_header_and_perceptual_quantized_band_gain_and_table_provider(
    header: FrameHeader,
    pcm: &AudioBuffer,
    step: f32,
    band_gain: Layer3QuantizedBandGain,
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<Vec<u8>, Error> {
    let frame_count = layer3_frame_count(header, pcm)?;
    let mut out = Vec::with_capacity(header.frame_len() * frame_count);
    for frame_index in 0..frame_count {
        let start_frame = frame_index
            .checked_mul(usize::from(header.samples_per_frame()))
            .ok_or(Error::InvalidInput("MP3 frame start overflows"))?;
        out.extend_from_slice(
            &assemble_mpeg1_layer3_pcm_frame_with_perceptual_quantized_band_gain_and_table_provider(
                header,
                pcm,
                start_frame,
                step,
                band_gain,
                provider,
            )?,
        );
    }
    Ok(out)
}

/// Encodes PCM with an explicit MPEG-1 Layer III header and diagnostic
/// per-band quantized coefficient gain plus global-gain bias.
pub fn encode_mpeg1_layer3_pcm_frames_with_header_and_perceptual_quantized_band_gain_and_global_gain_bias_and_table_provider(
    header: FrameHeader,
    pcm: &AudioBuffer,
    step: f32,
    band_gain: Layer3QuantizedBandGain,
    global_gain_bias: i16,
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<Vec<u8>, Error> {
    let frame_count = layer3_frame_count(header, pcm)?;
    let mut out = Vec::with_capacity(header.frame_len() * frame_count);
    for frame_index in 0..frame_count {
        let start_frame = frame_index
            .checked_mul(usize::from(header.samples_per_frame()))
            .ok_or(Error::InvalidInput("MP3 frame start overflows"))?;
        out.extend_from_slice(
            &assemble_mpeg1_layer3_pcm_frame_with_perceptual_quantized_band_gain_and_global_gain_bias_and_table_provider(
                header,
                pcm,
                start_frame,
                step,
                band_gain,
                global_gain_bias,
                provider,
            )?,
        );
    }
    Ok(out)
}

/// Encodes PCM with an explicit MPEG-1 Layer III header and perceptual step search.
pub fn encode_mpeg1_layer3_pcm_frames_with_header_and_perceptual_auto_step_and_table_provider(
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
        let step = select_mpeg1_layer3_pcm_frame_perceptual_step_with_table_provider(
            header,
            pcm,
            start_frame,
            candidates,
            provider,
        )?;
        out.extend_from_slice(
            &assemble_mpeg1_layer3_pcm_frame_with_perceptual_scale_factors_and_table_provider(
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

/// Encodes PCM with perceptual step search constrained by a payload bit budget.
pub fn encode_mpeg1_layer3_pcm_frames_with_header_and_perceptual_max_payload_bits_and_table_provider(
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
        let step =
            select_mpeg1_layer3_pcm_frame_perceptual_step_with_max_payload_bits_and_table_provider(
                header,
                pcm,
                start_frame,
                candidates,
                max_payload_bit_len,
                provider,
            )?;
        out.extend_from_slice(
            &assemble_mpeg1_layer3_pcm_frame_with_perceptual_scale_factors_and_table_provider(
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

/// Encodes PCM with perceptual per-frame step search and CBR padding.
pub fn encode_mpeg1_layer3_pcm_frames_with_header_perceptual_cbr_padding_and_table_provider(
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
        let step = select_mpeg1_layer3_pcm_frame_perceptual_step_with_table_provider(
            frame_header,
            pcm,
            start_frame,
            candidates,
            provider,
        )?;
        out.extend_from_slice(
            &assemble_mpeg1_layer3_pcm_frame_with_perceptual_scale_factors_and_table_provider(
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

/// Encodes PCM with perceptual per-frame step search and CBR padding, preferring
/// active psychoacoustic scale-factor allocation when such a candidate fits.
pub fn encode_mpeg1_layer3_pcm_frames_with_header_perceptual_active_cbr_padding_and_table_provider(
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
        let step = select_mpeg1_layer3_pcm_frame_perceptual_active_step_with_table_provider(
            frame_header,
            pcm,
            start_frame,
            candidates,
            provider,
        )?;
        out.extend_from_slice(
            &assemble_mpeg1_layer3_pcm_frame_with_perceptual_scale_factors_and_table_provider(
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

/// Selects the finest quantizer step that can be packed into one Layer III frame.
pub fn select_mpeg1_layer3_pcm_frame_step_with_table_provider(
    header: FrameHeader,
    pcm: &AudioBuffer,
    start_frame: usize,
    candidates: &[f32],
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<f32, Error> {
    Ok(
        select_mpeg1_layer3_pcm_frame_step_details_with_table_provider(
            header,
            pcm,
            start_frame,
            candidates,
            provider,
        )?
        .step,
    )
}

/// Selects the finest quantizer step within a caller-provided Layer III payload budget.
pub fn select_mpeg1_layer3_pcm_frame_step_with_max_payload_bits_and_table_provider(
    header: FrameHeader,
    pcm: &AudioBuffer,
    start_frame: usize,
    candidates: &[f32],
    max_payload_bit_len: usize,
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<f32, Error> {
    Ok(
        select_mpeg1_layer3_pcm_frame_step_details_with_max_payload_bits_and_table_provider(
            header,
            pcm,
            start_frame,
            candidates,
            max_payload_bit_len,
            provider,
        )?
        .step,
    )
}

/// Selects the finest perceptual-path quantizer step that fits one Layer III frame.
pub fn select_mpeg1_layer3_pcm_frame_perceptual_step_with_table_provider(
    header: FrameHeader,
    pcm: &AudioBuffer,
    start_frame: usize,
    candidates: &[f32],
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<f32, Error> {
    Ok(
        select_mpeg1_layer3_pcm_frame_perceptual_step_details_with_table_provider(
            header,
            pcm,
            start_frame,
            candidates,
            provider,
        )?
        .step,
    )
}

/// Selects the finest perceptual-path quantizer step that fits one frame,
/// preferring candidates with non-zero psychoacoustic scale factors.
pub fn select_mpeg1_layer3_pcm_frame_perceptual_active_step_with_table_provider(
    header: FrameHeader,
    pcm: &AudioBuffer,
    start_frame: usize,
    candidates: &[f32],
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<f32, Error> {
    Ok(
        select_mpeg1_layer3_pcm_frame_perceptual_active_step_details_with_table_provider(
            header,
            pcm,
            start_frame,
            candidates,
            provider,
        )?
        .step,
    )
}

/// Selects the finest perceptual-path quantizer step within a payload bit budget.
pub fn select_mpeg1_layer3_pcm_frame_perceptual_step_with_max_payload_bits_and_table_provider(
    header: FrameHeader,
    pcm: &AudioBuffer,
    start_frame: usize,
    candidates: &[f32],
    max_payload_bit_len: usize,
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<f32, Error> {
    Ok(
        select_mpeg1_layer3_pcm_frame_perceptual_step_details_with_max_payload_bits_and_table_provider(
            header,
            pcm,
            start_frame,
            candidates,
            max_payload_bit_len,
            provider,
        )?
        .step,
    )
}

/// Selects the finest quantizer step and reports the resulting frame payload cost.
pub fn select_mpeg1_layer3_pcm_frame_step_details_with_table_provider(
    header: FrameHeader,
    pcm: &AudioBuffer,
    start_frame: usize,
    candidates: &[f32],
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<Layer3PcmFrameStepSelection, Error> {
    if candidates.is_empty() {
        return Err(Error::InvalidInput(
            "MP3 quantizer step candidate list is empty",
        ));
    }
    let mut selected: Option<Layer3PcmFrameStepSelection> = None;
    for &step in candidates {
        if !step.is_finite() || step <= 0.0 {
            return Err(Error::InvalidInput(
                "MP3 quantizer step must be positive and finite",
            ));
        }
        if let Ok(selection) = evaluate_mpeg1_layer3_pcm_frame_step_with_table_provider(
            header,
            pcm,
            start_frame,
            step,
            provider,
        ) {
            selected = match selected {
                Some(previous)
                    if selection.step > previous.step
                        || (selection.step == previous.step
                            && selection.payload_bit_len <= previous.payload_bit_len) =>
                {
                    Some(previous)
                }
                _ => Some(selection),
            };
        }
    }
    selected.ok_or(Error::UnsupportedFeature("MP3 quantizer step search"))
}

/// Selects a perceptual-path quantizer step and reports the frame payload cost.
pub fn select_mpeg1_layer3_pcm_frame_perceptual_step_details_with_table_provider(
    header: FrameHeader,
    pcm: &AudioBuffer,
    start_frame: usize,
    candidates: &[f32],
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<Layer3PcmFrameStepSelection, Error> {
    if candidates.is_empty() {
        return Err(Error::InvalidInput(
            "MP3 quantizer step candidate list is empty",
        ));
    }
    let mut selected: Option<Layer3PcmFrameStepSelection> = None;
    for &step in candidates {
        if !step.is_finite() || step <= 0.0 {
            return Err(Error::InvalidInput(
                "MP3 quantizer step must be positive and finite",
            ));
        }
        if let Ok(selection) = evaluate_mpeg1_layer3_pcm_frame_perceptual_step_with_table_provider(
            header,
            pcm,
            start_frame,
            step,
            provider,
        ) {
            selected = select_better_mpeg1_layer3_pcm_frame_step(selected, selection);
        }
    }
    selected.ok_or(Error::UnsupportedFeature(
        "MP3 perceptual quantizer step search",
    ))
}

/// Selects a perceptual-path quantizer step, preferring fitting candidates
/// whose psychoacoustic analysis produces non-zero scale-factor allocation.
///
/// If every fitting candidate has zero allocation, this falls back to the
/// ordinary finest-step selector.
pub fn select_mpeg1_layer3_pcm_frame_perceptual_active_step_details_with_table_provider(
    header: FrameHeader,
    pcm: &AudioBuffer,
    start_frame: usize,
    candidates: &[f32],
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<Layer3PcmFrameStepSelection, Error> {
    if candidates.is_empty() {
        return Err(Error::InvalidInput(
            "MP3 quantizer step candidate list is empty",
        ));
    }
    let mut active: Option<Layer3PcmFrameStepSelection> = None;
    let mut fallback: Option<Layer3PcmFrameStepSelection> = None;
    for &step in candidates {
        if !step.is_finite() || step <= 0.0 {
            return Err(Error::InvalidInput(
                "MP3 quantizer step must be positive and finite",
            ));
        }
        if let Ok(selection) = evaluate_mpeg1_layer3_pcm_frame_perceptual_step_with_table_provider(
            header,
            pcm,
            start_frame,
            step,
            provider,
        ) {
            fallback = select_better_mpeg1_layer3_pcm_frame_step(fallback, selection);
            let nonzero_scale_factors =
                count_mpeg1_layer3_pcm_frame_perceptual_nonzero_scale_factors(
                    header,
                    pcm,
                    start_frame,
                    selection.step,
                )?;
            if nonzero_scale_factors > 0 {
                active = select_better_mpeg1_layer3_pcm_frame_step(active, selection);
            }
        }
    }
    active.or(fallback).ok_or(Error::UnsupportedFeature(
        "MP3 perceptual quantizer step search",
    ))
}

/// Reports first-frame perceptual candidate cost and scale-factor activation.
///
/// This helper uses the same first-frame perceptual payload evaluator and
/// psychoacoustic scale-factor selector as the active CBR diagnostic path. It
/// is intended to explain whether a candidate set is failing because of CBR
/// capacity or because scale-factor allocation never activates.
pub fn select_mpeg1_layer3_first_frame_perceptual_candidate_profile_with_table_provider(
    pcm: &AudioBuffer,
    candidates: &[f32],
    bitrate_kbps: u16,
    crc_protected: bool,
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<Vec<Layer3PerceptualCandidateProfile>, Error> {
    if candidates.is_empty() {
        return Err(Error::InvalidInput(
            "MP3 quantizer step candidate list is empty",
        ));
    }
    let header = layer3_header_for_capacity(
        pcm.sample_rate,
        pcm.channels,
        bitrate_kbps,
        false,
        crc_protected,
    )?;
    let samples_per_frame = usize::from(header.samples_per_frame());
    let mut profiles = Vec::new();
    for &candidate in candidates {
        if !candidate.is_finite() || candidate <= 0.0 {
            return Err(Error::InvalidInput(
                "MP3 quantizer step must be positive and finite",
            ));
        }
        let Ok(selection) =
            select_mpeg1_layer3_pcm_frame_perceptual_step_details_with_table_provider(
                header,
                pcm,
                0,
                &[candidate],
                provider,
            )
        else {
            continue;
        };
        let mut nonzero_scale_factors = 0usize;
        let mut scale_factor_bands = 0usize;
        let mut max_scale_factor = 0u8;
        for granule in 0..(samples_per_frame / 576).max(1) {
            let granule_start = granule
                .checked_mul(576)
                .ok_or(Error::InvalidInput("MP3 granule start overflows"))?;
            for channel in 0..usize::from(pcm.channels) {
                let scale_factors = select_mpeg1_layer3_psychoacoustic_long_scale_factors(
                    pcm,
                    channel,
                    granule_start,
                    selection.step,
                    false,
                    MPEG1_LAYER3_PSY_FFT_LEN,
                )?;
                for scale_factor in scale_factors {
                    nonzero_scale_factors += usize::from(scale_factor != 0);
                    scale_factor_bands += 1;
                    max_scale_factor = max_scale_factor.max(scale_factor);
                }
            }
        }
        profiles.push(Layer3PerceptualCandidateProfile {
            step: selection.step,
            payload_bit_len: selection.payload_bit_len,
            frame_capacity_bits: selection.frame_capacity_bits,
            nonzero_scale_factors,
            scale_factor_bands,
            max_scale_factor,
        });
    }
    if profiles.is_empty() {
        return Err(Error::UnsupportedFeature(
            "MP3 first-frame perceptual candidate profile",
        ));
    }
    Ok(profiles)
}

/// Reports first-frame low-band quantized spectral shape for perceptual
/// candidates.
///
/// The low-band range is scale-factor bands `0..7`, matching the diagnostics
/// that showed mono fine-step recovery is dominated by the lowest long-block
/// bands. This helper is intentionally read-only: it exposes proxy inputs
/// without changing production selection.
pub fn select_mpeg1_layer3_first_frame_low_band_spectral_shape_candidate_profile_with_table_provider(
    pcm: &AudioBuffer,
    candidates: &[f32],
    bitrate_kbps: u16,
    crc_protected: bool,
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<Vec<Layer3LowBandSpectralShapeCandidateProfile>, Error> {
    if candidates.is_empty() {
        return Err(Error::InvalidInput(
            "MP3 quantizer step candidate list is empty",
        ));
    }
    let header = layer3_header_for_capacity(
        pcm.sample_rate,
        pcm.channels,
        bitrate_kbps,
        false,
        crc_protected,
    )?;
    let samples_per_frame = usize::from(header.samples_per_frame());
    let mut profiles = Vec::new();
    for &candidate in candidates {
        if !candidate.is_finite() || candidate <= 0.0 {
            return Err(Error::InvalidInput(
                "MP3 quantizer step must be positive and finite",
            ));
        }
        let Ok(selection) =
            select_mpeg1_layer3_pcm_frame_perceptual_step_details_with_table_provider(
                header,
                pcm,
                0,
                &[candidate],
                provider,
            )
        else {
            continue;
        };
        let mut low_band_abs_sum = 0_u64;
        let mut total_abs_sum = 0_u64;
        let mut low_band_nonzero_lines = 0_usize;
        let mut total_nonzero_lines = 0_usize;
        for granule in 0..(samples_per_frame / 576).max(1) {
            let granule_start = granule
                .checked_mul(576)
                .ok_or(Error::InvalidInput("MP3 granule start overflows"))?;
            for channel in 0..usize::from(pcm.channels) {
                let spectrum = layer3_perceptual_quantizer_spectrum(pcm, channel, granule_start)?;
                let scalefac_scale = false;
                let scale_factors = select_centered_mpeg1_layer3_psychoacoustic_long_scale_factors(
                    pcm,
                    channel,
                    granule_start,
                    &spectrum,
                    selection.step,
                    scalefac_scale,
                )?;
                let quantized = quantize_mpeg1_layer3_long_spectrum_with_scalefactors(
                    &spectrum,
                    selection.step,
                    &scale_factors,
                    scalefac_scale,
                    pcm.sample_rate,
                )?;
                let low_band_end = usize::from(
                    *layer3_long_scalefactor_band_index(pcm.sample_rate)?
                        .get(7)
                        .ok_or(Error::InvalidInput("MP3 low-band boundary missing"))?,
                );
                for (line, coeff) in quantized.iter().enumerate() {
                    let magnitude = u64::from(coeff.unsigned_abs());
                    total_abs_sum = total_abs_sum.saturating_add(magnitude);
                    total_nonzero_lines += usize::from(*coeff != 0);
                    if line < low_band_end {
                        low_band_abs_sum = low_band_abs_sum.saturating_add(magnitude);
                        low_band_nonzero_lines += usize::from(*coeff != 0);
                    }
                }
            }
        }
        profiles.push(Layer3LowBandSpectralShapeCandidateProfile {
            step: selection.step,
            payload_bit_len: selection.payload_bit_len,
            frame_capacity_bits: selection.frame_capacity_bits,
            low_band_abs_sum,
            total_abs_sum,
            low_band_nonzero_lines,
            total_nonzero_lines,
        });
    }
    if profiles.is_empty() {
        return Err(Error::UnsupportedFeature(
            "MP3 first-frame low-band spectral shape candidate profile",
        ));
    }
    Ok(profiles)
}

/// Reports first-frame band-local quantized spectral shape for perceptual
/// candidates.
///
/// Each returned row is one candidate step and one long-block scale-factor
/// band, accumulated across the first MP3 frame's granules and channels. This
/// exposes the band-local proxy inputs needed by rate-control experiments
/// without changing production selection.
pub fn select_mpeg1_layer3_first_frame_band_spectral_shape_candidate_profile_with_table_provider(
    pcm: &AudioBuffer,
    candidates: &[f32],
    bitrate_kbps: u16,
    crc_protected: bool,
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<Vec<Layer3BandSpectralShapeCandidateProfile>, Error> {
    if candidates.is_empty() {
        return Err(Error::InvalidInput(
            "MP3 quantizer step candidate list is empty",
        ));
    }
    let header = layer3_header_for_capacity(
        pcm.sample_rate,
        pcm.channels,
        bitrate_kbps,
        false,
        crc_protected,
    )?;
    let samples_per_frame = usize::from(header.samples_per_frame());
    let band_count = MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT;
    let mut profiles = Vec::new();
    for &candidate in candidates {
        if !candidate.is_finite() || candidate <= 0.0 {
            return Err(Error::InvalidInput(
                "MP3 quantizer step must be positive and finite",
            ));
        }
        let Ok(selection) =
            select_mpeg1_layer3_pcm_frame_perceptual_step_details_with_table_provider(
                header,
                pcm,
                0,
                &[candidate],
                provider,
            )
        else {
            continue;
        };
        let mut band_abs_sums = vec![0_u64; band_count];
        let mut band_nonzero_lines = vec![0_usize; band_count];
        let mut total_abs_sum = 0_u64;
        let mut total_nonzero_lines = 0_usize;
        for granule in 0..(samples_per_frame / 576).max(1) {
            let granule_start = granule
                .checked_mul(576)
                .ok_or(Error::InvalidInput("MP3 granule start overflows"))?;
            for channel in 0..usize::from(pcm.channels) {
                let spectrum = layer3_perceptual_quantizer_spectrum(pcm, channel, granule_start)?;
                let scalefac_scale = false;
                let scale_factors = select_centered_mpeg1_layer3_psychoacoustic_long_scale_factors(
                    pcm,
                    channel,
                    granule_start,
                    &spectrum,
                    selection.step,
                    scalefac_scale,
                )?;
                let quantized = quantize_mpeg1_layer3_long_spectrum_with_scalefactors(
                    &spectrum,
                    selection.step,
                    &scale_factors,
                    scalefac_scale,
                    pcm.sample_rate,
                )?;
                for band in 0..band_count {
                    let (start, end) = layer3_long_scalefactor_band_range(band, pcm.sample_rate)?;
                    for &coeff in quantized.iter().take(end.min(quantized.len())).skip(start) {
                        let magnitude = u64::from(coeff.unsigned_abs());
                        band_abs_sums[band] = band_abs_sums[band].saturating_add(magnitude);
                        band_nonzero_lines[band] += usize::from(coeff != 0);
                    }
                }
                for coeff in &quantized {
                    total_abs_sum = total_abs_sum.saturating_add(u64::from(coeff.unsigned_abs()));
                    total_nonzero_lines += usize::from(*coeff != 0);
                }
            }
        }
        for band in 0..band_count {
            let (band_start, band_end) = layer3_long_scalefactor_band_range(band, pcm.sample_rate)?;
            profiles.push(Layer3BandSpectralShapeCandidateProfile {
                step: selection.step,
                payload_bit_len: selection.payload_bit_len,
                frame_capacity_bits: selection.frame_capacity_bits,
                band,
                band_start,
                band_end,
                band_abs_sum: band_abs_sums[band],
                band_nonzero_lines: band_nonzero_lines[band],
                total_abs_sum,
                total_nonzero_lines,
            });
        }
    }
    if profiles.is_empty() {
        return Err(Error::UnsupportedFeature(
            "MP3 first-frame band spectral shape candidate profile",
        ));
    }
    Ok(profiles)
}

/// Reports first-frame quality-guarded perceptual candidate cost and guard
/// decisions.
///
/// This helper evaluates each supplied step independently against the first
/// frame's own CBR main-data capacity. It exposes the encoder-side proxy state
/// for the quality-guarded bridge, including the perceptual/calibrated granule
/// accounting and distortion delta, without assembling a full reservoir stream.
pub fn select_mpeg1_layer3_first_frame_quality_guarded_candidate_profile_with_table_provider(
    pcm: &AudioBuffer,
    candidates: &[f32],
    bitrate_kbps: u16,
    crc_protected: bool,
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<Vec<Layer3QualityGuardedCandidateProfile>, Error> {
    if candidates.is_empty() {
        return Err(Error::InvalidInput(
            "MP3 quantizer step candidate list is empty",
        ));
    }
    let header = layer3_header_for_capacity(
        pcm.sample_rate,
        pcm.channels,
        bitrate_kbps,
        false,
        crc_protected,
    )?;
    let capacity = layer3_main_data_capacity_bytes(header)?;
    let mut profiles = Vec::new();
    for &candidate in candidates {
        if !candidate.is_finite() || candidate <= 0.0 {
            return Err(Error::InvalidInput(
                "MP3 quantizer step must be positive and finite",
            ));
        }
        let Ok(packed) = pack_mpeg1_layer3_reservoir_frame_with_table_provider(
            header,
            pcm,
            0,
            &[candidate],
            capacity,
            provider,
            Layer3ReservoirPayloadMode::PerceptualQualityGuarded,
        ) else {
            continue;
        };
        profiles.push(Layer3QualityGuardedCandidateProfile {
            step: packed.step,
            payload_bit_len: packed.main_data.bit_len,
            frame_capacity_bits: capacity * 8,
            perceptual_granules: packed.perceptual_granules,
            calibrated_granules: packed.calibrated_granules,
            quality_guard_compared_granules: packed.quality_guard_compared_granules,
            quality_guard_distortion_delta: packed.quality_guard_distortion_delta,
        });
    }
    if profiles.is_empty() {
        return Err(Error::UnsupportedFeature(
            "MP3 first-frame quality-guarded candidate profile",
        ));
    }
    Ok(profiles)
}
