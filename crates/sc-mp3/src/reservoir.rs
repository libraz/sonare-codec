use super::*;

/// MPEG-1 main-data backward pointer width: `main_data_begin` is 9 bits.
pub(crate) const MAX_MAIN_DATA_BEGIN: usize = 511;

/// Maximum `part2_3_length` for one granule: the side-info field is 12 bits.
pub(crate) const MAX_PART2_3_LENGTH: u16 = 4095;

/// Reports whether any granule's `part2_3_length` overflows its 12-bit field.
pub(crate) fn layer3_side_info_exceeds_part2_3_limit(
    side_info: &Layer3SideInfo,
    header: FrameHeader,
) -> bool {
    for granule in 0..header.layer3_granule_count() {
        for channel in 0..header.channel_count() {
            if side_info.granules[granule][channel].part2_3_length > MAX_PART2_3_LENGTH {
                return true;
            }
        }
    }
    false
}

/// One frame's reservoir-aware packing result, retained for the layout pass.
pub(crate) struct Layer3ReservoirFrame {
    pub(crate) header: FrameHeader,
    pub(crate) side_info: Layer3SideInfo,
    pub(crate) payload: Vec<u8>,
    pub(crate) payload_bit_len: usize,
    pub(crate) capacity: usize,
    pub(crate) main_data_begin: usize,
    pub(crate) reservoir_after: usize,
    pub(crate) step: f32,
    pub(crate) perceptual_granules: usize,
    pub(crate) calibrated_granules: usize,
    pub(crate) quality_guard_compared_granules: usize,
    pub(crate) quality_guard_distortion_delta: f64,
}

#[derive(Clone)]
pub(crate) struct Layer3ReservoirPackedFrame {
    pub(crate) step: f32,
    pub(crate) side_info: Layer3SideInfo,
    pub(crate) main_data: PackedBits,
    pub(crate) perceptual_granules: usize,
    pub(crate) calibrated_granules: usize,
    pub(crate) quality_guard_compared_granules: usize,
    pub(crate) quality_guard_distortion_delta: f64,
}

pub(crate) struct Layer3QualityGuardGranulePayload {
    pub(crate) bits: PackedBits,
    pub(crate) used_perceptual: bool,
    pub(crate) compared_granules: usize,
    pub(crate) distortion_delta: f64,
}

#[derive(Clone)]
pub(crate) struct Layer3QualityGuardPerceptualCandidate {
    pub(crate) scale_factors: [u8; MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT],
    pub(crate) quantized: Vec<i32>,
    pub(crate) scalefac_scale: bool,
    pub(crate) global_gain: u8,
    pub(crate) distortion: f64,
}

#[derive(Clone, Copy)]
pub(crate) enum Layer3ReservoirPayloadMode {
    Calibrated,
    PerceptualActive,
    PerceptualQualityGuarded,
    PerceptualQuantizedBandGainGlobalGainBias {
        band_gain: Layer3QuantizedBandGain,
        global_gain_bias: i16,
    },
}

/// Packs one Layer III frame at the finest quantizer step whose byte-padded
/// payload fits a main-data byte budget (frame capacity plus borrowed reservoir).
///
/// Unlike the single-frame step search, the budget may exceed one frame's own
/// capacity, so the per-step capacity guard is replaced by the supplied budget.
pub(crate) fn pack_mpeg1_layer3_reservoir_frame_with_table_provider(
    header: FrameHeader,
    pcm: &AudioBuffer,
    start_frame: usize,
    candidates: &[f32],
    budget_bytes: usize,
    provider: Layer3EntropyTableProvider<'_>,
    mode: Layer3ReservoirPayloadMode,
) -> Result<Layer3ReservoirPackedFrame, Error> {
    if candidates.is_empty() {
        return Err(Error::InvalidInput(
            "MP3 quantizer step candidate list is empty",
        ));
    }
    let frame_granules = header.layer3_granule_count() * header.channel_count();
    let mut best: Option<Layer3ReservoirPackedFrame> = None;
    let mut best_active: Option<(Layer3ReservoirPackedFrame, usize)> = None;
    for &step in candidates {
        if !step.is_finite() || step <= 0.0 {
            return Err(Error::InvalidInput(
                "MP3 quantizer step must be positive and finite",
            ));
        }
        let candidate_result: Result<Layer3ReservoirPackedFrame, Error> = match mode {
            Layer3ReservoirPayloadMode::Calibrated => {
                pack_mpeg1_layer3_pcm_frame_payloads_with_table_provider(
                    header,
                    pcm,
                    start_frame,
                    step,
                    provider,
                )
                .map(|(side_info, main_data)| Layer3ReservoirPackedFrame {
                    step,
                    side_info,
                    main_data,
                    perceptual_granules: 0,
                    calibrated_granules: frame_granules,
                    quality_guard_compared_granules: 0,
                    quality_guard_distortion_delta: 0.0,
                })
            }
            Layer3ReservoirPayloadMode::PerceptualActive => {
                pack_mpeg1_layer3_pcm_frame_perceptual_payloads_with_table_provider(
                    header,
                    pcm,
                    start_frame,
                    step,
                    provider,
                )
                .map(|(side_info, main_data)| Layer3ReservoirPackedFrame {
                    step,
                    side_info,
                    main_data,
                    perceptual_granules: frame_granules,
                    calibrated_granules: 0,
                    quality_guard_compared_granules: 0,
                    quality_guard_distortion_delta: 0.0,
                })
            }
            Layer3ReservoirPayloadMode::PerceptualQualityGuarded => {
                pack_mpeg1_layer3_pcm_frame_perceptual_quality_guard_payloads_with_table_provider(
                    header,
                    pcm,
                    start_frame,
                    step,
                    provider,
                )
                .map(
                    |(
                        side_info,
                        main_data,
                        perceptual_granules,
                        calibrated_granules,
                        quality_guard_compared_granules,
                        quality_guard_distortion_delta,
                    )| {
                        Layer3ReservoirPackedFrame {
                            step,
                            side_info,
                            main_data,
                            perceptual_granules,
                            calibrated_granules,
                            quality_guard_compared_granules,
                            quality_guard_distortion_delta,
                        }
                    },
                )
            }
            Layer3ReservoirPayloadMode::PerceptualQuantizedBandGainGlobalGainBias {
                band_gain,
                global_gain_bias,
            } => {
                pack_mpeg1_layer3_pcm_frame_perceptual_quantized_band_gain_and_global_gain_bias_payloads_with_table_provider(
                    header,
                    pcm,
                    start_frame,
                    step,
                    band_gain,
                    global_gain_bias,
                    provider,
                )
                .map(|(side_info, main_data)| Layer3ReservoirPackedFrame {
                    step,
                    side_info,
                    main_data,
                    perceptual_granules: frame_granules,
                    calibrated_granules: 0,
                    quality_guard_compared_granules: 0,
                    quality_guard_distortion_delta: 0.0,
                })
            }
        };
        let Ok(candidate) = candidate_result else {
            continue;
        };
        if candidate.main_data.bytes.len() > budget_bytes {
            continue;
        }
        // The reservoir can widen the byte budget past the point where one
        // granule's part2_3_length overflows its 12-bit side-info field; reject
        // any step that does, so a finer step never produces an unpackable frame.
        if layer3_side_info_exceeds_part2_3_limit(&candidate.side_info, header) {
            continue;
        }
        let active_scale_factors = if matches!(
            mode,
            Layer3ReservoirPayloadMode::PerceptualActive
                | Layer3ReservoirPayloadMode::PerceptualQualityGuarded
        ) && header.channel_count() == 1
        {
            count_mpeg1_layer3_pcm_frame_perceptual_nonzero_scale_factors(
                header,
                pcm,
                start_frame,
                step,
            )?
        } else {
            0
        };
        if active_scale_factors > 0 {
            best = None;
            best_active = select_better_mpeg1_layer3_active_reservoir_candidate(
                best_active,
                candidate,
                active_scale_factors,
            );
            continue;
        }
        if best_active.is_none() {
            // Prefer the smallest fitting step (finest quantization, best quality).
            best = select_better_mpeg1_layer3_reservoir_candidate(best, candidate);
        }
    }
    best_active
        .map(|(candidate, _)| candidate)
        .or(best)
        .ok_or(Error::UnsupportedFeature("MP3 reservoir step search"))
}

pub(crate) fn select_better_mpeg1_layer3_active_reservoir_candidate(
    selected: Option<(Layer3ReservoirPackedFrame, usize)>,
    candidate: Layer3ReservoirPackedFrame,
    nonzero_scale_factors: usize,
) -> Option<(Layer3ReservoirPackedFrame, usize)> {
    match selected {
        Some((best, best_nonzero))
            if nonzero_scale_factors < best_nonzero
                || (nonzero_scale_factors == best_nonzero && candidate.step >= best.step) =>
        {
            Some((best, best_nonzero))
        }
        _ => Some((candidate, nonzero_scale_factors)),
    }
}

pub(crate) fn select_better_mpeg1_layer3_reservoir_candidate(
    selected: Option<Layer3ReservoirPackedFrame>,
    candidate: Layer3ReservoirPackedFrame,
) -> Option<Layer3ReservoirPackedFrame> {
    match selected {
        Some(best) if candidate.step >= best.step => Some(best),
        _ => Some(candidate),
    }
}

pub(crate) fn collect_mpeg1_layer3_reservoir_frames_with_table_provider(
    pcm: &AudioBuffer,
    candidates: &[f32],
    bitrate_kbps: u16,
    crc_protected: bool,
    provider: Layer3EntropyTableProvider<'_>,
    mode: Layer3ReservoirPayloadMode,
) -> Result<Vec<Layer3ReservoirFrame>, Error> {
    let base_header = layer3_header_for_capacity(
        pcm.sample_rate,
        pcm.channels,
        bitrate_kbps,
        false,
        crc_protected,
    )?;
    let frame_count = layer3_frame_count(base_header, pcm)?;
    let mut padding = Layer3PaddingSchedule::new(base_header)?;

    let mut frames: Vec<Layer3ReservoirFrame> = Vec::with_capacity(frame_count);
    let mut reservoir = 0_usize;
    for frame_index in 0..frame_count {
        let start_frame = frame_index
            .checked_mul(usize::from(base_header.samples_per_frame()))
            .ok_or(Error::InvalidInput("MP3 frame start overflows"))?;
        let frame_header = padding.next_header();
        let capacity = layer3_main_data_capacity_bytes(frame_header)?;
        let main_data_begin = reservoir.min(MAX_MAIN_DATA_BEGIN);
        let budget_bytes = capacity
            .checked_add(main_data_begin)
            .ok_or(Error::InvalidInput("MP3 reservoir budget overflows"))?;
        let packed = pack_mpeg1_layer3_reservoir_frame_with_table_provider(
            frame_header,
            pcm,
            start_frame,
            candidates,
            budget_bytes,
            provider,
            mode,
        )?;
        let Layer3ReservoirPackedFrame {
            step,
            mut side_info,
            main_data,
            perceptual_granules,
            calibrated_granules,
            quality_guard_compared_granules,
            quality_guard_distortion_delta,
        } = packed;
        side_info.main_data_begin = u16::try_from(main_data_begin)
            .map_err(|_| Error::InvalidInput("MP3 main_data_begin exceeds field width"))?;
        let payload_bit_len = main_data.bit_len;
        let payload = main_data.bytes;
        let reservoir_after = main_data_begin
            .checked_add(capacity)
            .ok_or(Error::InvalidInput("MP3 reservoir overflows"))?
            .checked_sub(payload.len())
            .ok_or(Error::InvalidInput("MP3 reservoir underflows"))?;
        reservoir = reservoir_after;
        frames.push(Layer3ReservoirFrame {
            header: frame_header,
            side_info,
            payload,
            payload_bit_len,
            capacity,
            main_data_begin,
            reservoir_after,
            step,
            perceptual_granules,
            calibrated_granules,
            quality_guard_compared_granules,
            quality_guard_distortion_delta,
        });
    }

    Ok(frames)
}

pub(crate) fn layer3_entropy_target_bits_by_frame(
    pcm: &AudioBuffer,
    bitrate_kbps: u16,
    crc_protected: bool,
    min_bits_per_granule_channel: usize,
) -> Result<Vec<usize>, Error> {
    let base_header = layer3_header_for_capacity(
        pcm.sample_rate,
        pcm.channels,
        bitrate_kbps,
        false,
        crc_protected,
    )?;
    let frame_count = layer3_frame_count(base_header, pcm)?;
    let mut frame_targets = vec![0usize; frame_count];
    for allocation in select_mpeg1_layer3_perceptual_bit_allocation_with_bitrate(
        pcm,
        bitrate_kbps,
        crc_protected,
        min_bits_per_granule_channel,
    )? {
        if let Some(slot) = frame_targets.get_mut(allocation.frame_index) {
            *slot = slot
                .checked_add(allocation.target_bits)
                .ok_or(Error::InvalidInput("MP3 entropy frame target overflows"))?;
        }
    }
    Ok(frame_targets)
}

pub(crate) fn collect_mpeg1_layer3_entropy_targeted_reservoir_frames_with_table_provider(
    pcm: &AudioBuffer,
    candidates: &[f32],
    bitrate_kbps: u16,
    crc_protected: bool,
    min_bits_per_granule_channel: usize,
    provider: Layer3EntropyTableProvider<'_>,
    mode: Layer3ReservoirPayloadMode,
) -> Result<Vec<(Layer3ReservoirFrame, usize, bool)>, Error> {
    let base_header = layer3_header_for_capacity(
        pcm.sample_rate,
        pcm.channels,
        bitrate_kbps,
        false,
        crc_protected,
    )?;
    let frame_count = layer3_frame_count(base_header, pcm)?;
    let frame_target_bits = layer3_entropy_target_bits_by_frame(
        pcm,
        bitrate_kbps,
        crc_protected,
        min_bits_per_granule_channel,
    )?;
    let mut padding = Layer3PaddingSchedule::new(base_header)?;

    let mut frames: Vec<(Layer3ReservoirFrame, usize, bool)> = Vec::with_capacity(frame_count);
    let mut reservoir = 0_usize;
    for frame_index in 0..frame_count {
        let start_frame = frame_index
            .checked_mul(usize::from(base_header.samples_per_frame()))
            .ok_or(Error::InvalidInput("MP3 frame start overflows"))?;
        let frame_header = padding.next_header();
        let capacity = layer3_main_data_capacity_bytes(frame_header)?;
        let main_data_begin = reservoir.min(MAX_MAIN_DATA_BEGIN);
        let full_budget_bytes = capacity
            .checked_add(main_data_begin)
            .ok_or(Error::InvalidInput("MP3 reservoir budget overflows"))?;
        let entropy_target_bits = *frame_target_bits.get(frame_index).unwrap_or(&0);
        let entropy_budget_bytes = entropy_target_bits
            .saturating_add(7)
            .checked_div(8)
            .unwrap_or(0)
            .clamp(1, full_budget_bytes);
        let entropy_packed = pack_mpeg1_layer3_reservoir_frame_with_table_provider(
            frame_header,
            pcm,
            start_frame,
            candidates,
            entropy_budget_bytes,
            provider,
            mode,
        );
        let (packed, used_entropy_target_budget) = match entropy_packed {
            Ok(packed) => (packed, true),
            Err(_) => (
                pack_mpeg1_layer3_reservoir_frame_with_table_provider(
                    frame_header,
                    pcm,
                    start_frame,
                    candidates,
                    full_budget_bytes,
                    provider,
                    mode,
                )?,
                false,
            ),
        };
        let Layer3ReservoirPackedFrame {
            step,
            mut side_info,
            main_data,
            perceptual_granules,
            calibrated_granules,
            quality_guard_compared_granules,
            quality_guard_distortion_delta,
        } = packed;
        side_info.main_data_begin = u16::try_from(main_data_begin)
            .map_err(|_| Error::InvalidInput("MP3 main_data_begin exceeds field width"))?;
        let payload_bit_len = main_data.bit_len;
        let payload = main_data.bytes;
        let reservoir_after = main_data_begin
            .checked_add(capacity)
            .ok_or(Error::InvalidInput("MP3 reservoir overflows"))?
            .checked_sub(payload.len())
            .ok_or(Error::InvalidInput("MP3 reservoir underflows"))?;
        reservoir = reservoir_after;
        frames.push((
            Layer3ReservoirFrame {
                header: frame_header,
                side_info,
                payload,
                payload_bit_len,
                capacity,
                main_data_begin,
                reservoir_after,
                step,
                perceptual_granules,
                calibrated_granules,
                quality_guard_compared_granules,
                quality_guard_distortion_delta,
            },
            entropy_target_bits,
            used_entropy_target_budget,
        ));
    }

    Ok(frames)
}

/// Selects reservoir-aware CBR frame steps and reports the rate-control state.
///
/// This uses the same pass-1 selection as
/// [`encode_mpeg1_layer3_pcm_frames_with_reservoir_and_table_provider`] without
/// assembling the final bytestream, so callers can inspect whether CBR capacity,
/// borrowed main data, and selected quantizer steps are behaving as expected.
pub fn select_mpeg1_layer3_reservoir_frame_details_with_table_provider(
    pcm: &AudioBuffer,
    candidates: &[f32],
    bitrate_kbps: u16,
    crc_protected: bool,
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<Vec<Layer3ReservoirFrameSelection>, Error> {
    collect_mpeg1_layer3_reservoir_frames_with_table_provider(
        pcm,
        candidates,
        bitrate_kbps,
        crc_protected,
        provider,
        Layer3ReservoirPayloadMode::Calibrated,
    )?
    .into_iter()
    .enumerate()
    .map(|(frame_index, frame)| {
        Ok(Layer3ReservoirFrameSelection {
            frame_index,
            step: frame.step,
            payload_bit_len: frame.payload_bit_len,
            frame_len: frame.header.frame_len(),
            padding: frame.header.padding,
            frame_capacity_bytes: frame.capacity,
            main_data_begin: frame.main_data_begin,
            reservoir_after: frame.reservoir_after,
            perceptual_granules: frame.perceptual_granules,
            calibrated_granules: frame.calibrated_granules,
            quality_guard_compared_granules: frame.quality_guard_compared_granules,
            quality_guard_distortion_delta: frame.quality_guard_distortion_delta,
        })
    })
    .collect()
}

/// Selects perceptual-scale-factor reservoir-aware CBR frame steps and reports
/// the rate-control state.
///
/// This keeps the bit-reservoir layout used by production MP3 candidates while
/// packing each frame through the psychoacoustic scale-factor path. It is a
/// diagnostic promotion candidate; the default production encoder still uses
/// the calibrated-gain reservoir path for stereo, while non-silent mono now
/// uses this path directly.
pub fn select_mpeg1_layer3_perceptual_reservoir_frame_details_with_table_provider(
    pcm: &AudioBuffer,
    candidates: &[f32],
    bitrate_kbps: u16,
    crc_protected: bool,
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<Vec<Layer3ReservoirFrameSelection>, Error> {
    collect_mpeg1_layer3_reservoir_frames_with_table_provider(
        pcm,
        candidates,
        bitrate_kbps,
        crc_protected,
        provider,
        Layer3ReservoirPayloadMode::PerceptualActive,
    )?
    .into_iter()
    .enumerate()
    .map(|(frame_index, frame)| {
        Ok(Layer3ReservoirFrameSelection {
            frame_index,
            step: frame.step,
            payload_bit_len: frame.payload_bit_len,
            frame_len: frame.header.frame_len(),
            padding: frame.header.padding,
            frame_capacity_bytes: frame.capacity,
            main_data_begin: frame.main_data_begin,
            reservoir_after: frame.reservoir_after,
            perceptual_granules: frame.perceptual_granules,
            calibrated_granules: frame.calibrated_granules,
            quality_guard_compared_granules: frame.quality_guard_compared_granules,
            quality_guard_distortion_delta: frame.quality_guard_distortion_delta,
        })
    })
    .collect()
}

/// Selects perceptual reservoir frame steps using entropy-derived frame targets.
///
/// The target bits come from
/// [`select_mpeg1_layer3_perceptual_bit_allocation_with_bitrate`] and are summed
/// per frame before selecting a payload. If no candidate fits the entropy
/// target budget, the selector falls back to the ordinary reservoir budget and
/// marks `used_entropy_target_budget=false` in the returned detail.
pub fn select_mpeg1_layer3_entropy_targeted_perceptual_reservoir_frame_details_with_table_provider(
    pcm: &AudioBuffer,
    candidates: &[f32],
    bitrate_kbps: u16,
    crc_protected: bool,
    min_bits_per_granule_channel: usize,
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<Vec<Layer3EntropyTargetedReservoirFrameSelection>, Error> {
    collect_mpeg1_layer3_entropy_targeted_reservoir_frames_with_table_provider(
        pcm,
        candidates,
        bitrate_kbps,
        crc_protected,
        min_bits_per_granule_channel,
        provider,
        Layer3ReservoirPayloadMode::PerceptualActive,
    )?
    .into_iter()
    .enumerate()
    .map(
        |(frame_index, (frame, entropy_target_bits, used_entropy_target_budget))| {
            Ok(Layer3EntropyTargetedReservoirFrameSelection {
                frame_index,
                step: frame.step,
                payload_bit_len: frame.payload_bit_len,
                frame_len: frame.header.frame_len(),
                padding: frame.header.padding,
                frame_capacity_bytes: frame.capacity,
                main_data_begin: frame.main_data_begin,
                reservoir_after: frame.reservoir_after,
                perceptual_granules: frame.perceptual_granules,
                calibrated_granules: frame.calibrated_granules,
                quality_guard_compared_granules: frame.quality_guard_compared_granules,
                quality_guard_distortion_delta: frame.quality_guard_distortion_delta,
                entropy_target_bits,
                used_entropy_target_budget,
            })
        },
    )
    .collect()
}

/// Summarizes how much of the rounded entropy-target frame budgets were used.
///
/// The budget calculation matches the entropy-targeted reservoir selector:
/// per-frame entropy targets are rounded up to whole bytes, clamped to the
/// available frame plus borrowed reservoir budget, and then compared with the
/// actually selected main-data payload.
pub fn mpeg1_layer3_entropy_target_utilization_profile(
    details: &[Layer3EntropyTargetedReservoirFrameSelection],
) -> Layer3EntropyTargetUtilizationProfile {
    let mut used_entropy_target_frames = 0_usize;
    let mut payload_bits = 0_usize;
    let mut entropy_budget_bits = 0_usize;
    let mut max_entropy_budget_slack_bits = 0_usize;

    for detail in details {
        if !detail.used_entropy_target_budget {
            continue;
        }
        let full_budget_bytes = detail
            .frame_capacity_bytes
            .saturating_add(detail.main_data_begin);
        let budget_bits = detail
            .entropy_target_bits
            .saturating_add(7)
            .checked_div(8)
            .unwrap_or(0)
            .clamp(1, full_budget_bytes)
            .saturating_mul(8);
        used_entropy_target_frames += 1;
        payload_bits = payload_bits.saturating_add(detail.payload_bit_len);
        entropy_budget_bits = entropy_budget_bits.saturating_add(budget_bits);
        max_entropy_budget_slack_bits =
            max_entropy_budget_slack_bits.max(budget_bits.saturating_sub(detail.payload_bit_len));
    }

    let utilization = if entropy_budget_bits == 0 {
        0.0
    } else {
        payload_bits as f64 / entropy_budget_bits as f64
    };

    Layer3EntropyTargetUtilizationProfile {
        frames: details.len(),
        used_entropy_target_frames,
        payload_bits,
        entropy_budget_bits,
        utilization,
        max_entropy_budget_slack_bits,
    }
}

pub fn select_mpeg1_layer3_entropy_target_utilization_profile_with_table_provider(
    pcm: &AudioBuffer,
    candidates: &[f32],
    bitrate_kbps: u16,
    crc_protected: bool,
    min_bits_per_granule_channel: usize,
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<Layer3EntropyTargetUtilizationProfile, Error> {
    let details =
        select_mpeg1_layer3_entropy_targeted_perceptual_reservoir_frame_details_with_table_provider(
            pcm,
            candidates,
            bitrate_kbps,
            crc_protected,
            min_bits_per_granule_channel,
            provider,
        )?;
    Ok(mpeg1_layer3_entropy_target_utilization_profile(&details))
}

/// Selects quality-guarded perceptual reservoir CBR frame steps.
///
/// Granules are evaluated with both calibrated zero-scale-factor quantization
/// and perceptual scale-factor quantization at the same selected step. The
/// helper records the encoder-side guard comparison while keeping the
/// psychoacoustic scale-factor candidate active whenever it can be built. This
/// keeps the bridge aligned with the current perceptual/production reservoir
/// behavior while exposing the remaining proxy state for diagnostics.
pub fn select_mpeg1_layer3_quality_guarded_perceptual_reservoir_frame_details_with_table_provider(
    pcm: &AudioBuffer,
    candidates: &[f32],
    bitrate_kbps: u16,
    crc_protected: bool,
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<Vec<Layer3ReservoirFrameSelection>, Error> {
    collect_mpeg1_layer3_reservoir_frames_with_table_provider(
        pcm,
        candidates,
        bitrate_kbps,
        crc_protected,
        provider,
        Layer3ReservoirPayloadMode::PerceptualQualityGuarded,
    )?
    .into_iter()
    .enumerate()
    .map(|(frame_index, frame)| {
        Ok(Layer3ReservoirFrameSelection {
            frame_index,
            step: frame.step,
            payload_bit_len: frame.payload_bit_len,
            frame_len: frame.header.frame_len(),
            padding: frame.header.padding,
            frame_capacity_bytes: frame.capacity,
            main_data_begin: frame.main_data_begin,
            reservoir_after: frame.reservoir_after,
            perceptual_granules: frame.perceptual_granules,
            calibrated_granules: frame.calibrated_granules,
            quality_guard_compared_granules: frame.quality_guard_compared_granules,
            quality_guard_distortion_delta: frame.quality_guard_distortion_delta,
        })
    })
    .collect()
}

/// Encodes PCM as constant-bitrate MPEG-1 Layer III using a bit reservoir.
///
/// A frame whose granules need more bits than its own main-data slot provides
/// borrows from the unused tail of earlier frames through `main_data_begin`, the
/// spec's backward byte pointer into the shared main-data stream (ISO/IEC
/// 11172-3 §2.4.1.7). Frames that quantize cheaply leave surplus slot bytes that
/// later, busier frames consume, so the average bitrate stays constant while
/// per-frame quality is no longer hard-capped at one frame's capacity.
pub fn encode_mpeg1_layer3_pcm_frames_with_reservoir_and_table_provider(
    pcm: &AudioBuffer,
    candidates: &[f32],
    bitrate_kbps: u16,
    crc_protected: bool,
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<Vec<u8>, Error> {
    let frames = collect_mpeg1_layer3_reservoir_frames_with_table_provider(
        pcm,
        candidates,
        bitrate_kbps,
        crc_protected,
        provider,
        Layer3ReservoirPayloadMode::Calibrated,
    )?;

    assemble_mpeg1_layer3_reservoir_frames(frames)
}

/// Encodes PCM as constant-bitrate MPEG-1 Layer III using psychoacoustic
/// scale-factor payloads plus a bit reservoir.
///
/// This is exposed as a diagnostic path toward production rate-control work:
/// it preserves the shared main-data reservoir layout, keeps mono on the
/// nonzero scale-factor diagnostic path, and lets stereo choose the finest
/// fitting perceptual payload instead of forcing a coarser active candidate.
/// Non-silent production MP3 now uses this path for both mono and stereo.
pub fn encode_mpeg1_layer3_pcm_frames_with_perceptual_reservoir_and_table_provider(
    pcm: &AudioBuffer,
    candidates: &[f32],
    bitrate_kbps: u16,
    crc_protected: bool,
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<Vec<u8>, Error> {
    let frames = collect_mpeg1_layer3_reservoir_frames_with_table_provider(
        pcm,
        candidates,
        bitrate_kbps,
        crc_protected,
        provider,
        Layer3ReservoirPayloadMode::PerceptualActive,
    )?;

    assemble_mpeg1_layer3_reservoir_frames(frames)
}

/// Encodes PCM as constant-bitrate Layer III using entropy-targeted perceptual
/// reservoir step selection.
///
/// This diagnostic encoder uses the same frame choices reported by
/// [`select_mpeg1_layer3_entropy_targeted_perceptual_reservoir_frame_details_with_table_provider`]
/// and then assembles those frames into a bytestream. It is kept separate from
/// production until FFmpeg oracle quality is checked against the current
/// production path.
pub fn encode_mpeg1_layer3_pcm_frames_with_entropy_targeted_perceptual_reservoir_and_table_provider(
    pcm: &AudioBuffer,
    candidates: &[f32],
    bitrate_kbps: u16,
    crc_protected: bool,
    min_bits_per_granule_channel: usize,
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<Vec<u8>, Error> {
    let frames = collect_mpeg1_layer3_entropy_targeted_reservoir_frames_with_table_provider(
        pcm,
        candidates,
        bitrate_kbps,
        crc_protected,
        min_bits_per_granule_channel,
        provider,
        Layer3ReservoirPayloadMode::PerceptualActive,
    )?
    .into_iter()
    .map(|(frame, _, _)| frame)
    .collect();

    assemble_mpeg1_layer3_reservoir_frames(frames)
}

/// Encodes PCM as constant-bitrate Layer III using entropy-targeted reservoir
/// selection with the band-local quantized-gain/global-gain-bias diagnostic
/// payload.
///
/// This keeps the same reservoir and entropy-target byte budgets as production
/// while testing whether the low-band spectral-shape recovery survives across
/// reservoir-packed frames.
#[allow(clippy::too_many_arguments)]
pub fn encode_mpeg1_layer3_pcm_frames_with_entropy_targeted_perceptual_quantized_band_gain_and_global_gain_bias_reservoir_and_table_provider(
    pcm: &AudioBuffer,
    candidates: &[f32],
    bitrate_kbps: u16,
    crc_protected: bool,
    min_bits_per_granule_channel: usize,
    band_gain: Layer3QuantizedBandGain,
    global_gain_bias: i16,
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<Vec<u8>, Error> {
    let frames = collect_mpeg1_layer3_entropy_targeted_reservoir_frames_with_table_provider(
        pcm,
        candidates,
        bitrate_kbps,
        crc_protected,
        min_bits_per_granule_channel,
        provider,
        Layer3ReservoirPayloadMode::PerceptualQuantizedBandGainGlobalGainBias {
            band_gain,
            global_gain_bias,
        },
    )?
    .into_iter()
    .map(|(frame, _, _)| frame)
    .collect();

    assemble_mpeg1_layer3_reservoir_frames(frames)
}

/// Encodes PCM as constant-bitrate MPEG-1 Layer III using a quality-guarded
/// perceptual reservoir path.
///
/// This is a diagnostic bridge for the psychoacoustic workbench: it preserves
/// the bit reservoir and bitrate schedule while recording the guard proxy that
/// compares calibrated and perceptual quantization at each accepted step. When
/// perceptual quantization succeeds, the helper keeps that scale-factor payload
/// active so it can be compared directly with the current perceptual and
/// entropy-targeted production reservoir paths.
pub fn encode_mpeg1_layer3_pcm_frames_with_quality_guarded_perceptual_reservoir_and_table_provider(
    pcm: &AudioBuffer,
    candidates: &[f32],
    bitrate_kbps: u16,
    crc_protected: bool,
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<Vec<u8>, Error> {
    let frames = collect_mpeg1_layer3_reservoir_frames_with_table_provider(
        pcm,
        candidates,
        bitrate_kbps,
        crc_protected,
        provider,
        Layer3ReservoirPayloadMode::PerceptualQualityGuarded,
    )?;

    assemble_mpeg1_layer3_reservoir_frames(frames)
}
