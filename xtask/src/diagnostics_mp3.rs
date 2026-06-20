use super::*;

pub(crate) fn verify_production_lossy_oracle_acceptance(
    ffmpeg: OsString,
    artifacts: &[(
        &str,
        ProductionArtifactKind,
        sonare_codec::AudioBuffer,
        Vec<u8>,
    )],
) -> Result<(), String> {
    let out_dir = env::temp_dir().join(format!(
        "sonare-codec-production-readiness-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |duration| duration.as_millis())
    ));
    fs::create_dir_all(&out_dir)
        .map_err(|err| format!("failed to create {}: {err}", out_dir.display()))?;

    for (label, kind, expected_pcm, bytes) in artifacts {
        verify_mp3_default_production_budget(label, *kind, expected_pcm, bytes)?;
        verify_aac_default_production_budget(label, *kind, expected_pcm, bytes)?;

        let extension = kind.extension();
        let path = out_dir.join(format!(
            "{}.{}",
            label.to_ascii_lowercase().replace('-', ""),
            extension
        ));
        fs::write(&path, bytes)
            .map_err(|err| format!("failed to write {}: {err}", path.display()))?;
        run_ffmpeg_acceptance(&ffmpeg, &path)
            .map_err(|err| format!("{label} production oracle acceptance failed: {err}"))?;
        let decoded = run_ffmpeg_decode_f32le(
            &ffmpeg,
            &path,
            expected_pcm.sample_rate,
            expected_pcm.channels,
        )
        .map_err(|err| format!("{label} production oracle PCM decode failed: {err}"))?;
        let quality = validate_lossy_oracle_pcm_quality(&expected_pcm.samples, &decoded)
            .map_err(|err| format!("{label} production oracle PCM quality failed: {err}"))?;
        let min_correlation = production_lossy_min_correlation(*kind, expected_pcm.channels)?;
        if quality.best_correlation < min_correlation {
            return Err(format!(
                "{label} production oracle PCM quality regressed below floor {min_correlation:.3}: decoded_rms={:.4}, best_correlation={:.3}",
                quality.decoded_rms, quality.best_correlation
            ));
        }
        eprintln!(
            "{label} production oracle PCM quality: decoded_rms={:.4}, best_correlation={:.3}, min_correlation={min_correlation:.3}",
            quality.decoded_rms, quality.best_correlation
        );
    }

    fs::remove_dir_all(&out_dir)
        .map_err(|err| format!("failed to remove {}: {err}", out_dir.display()))
}

pub(crate) fn production_lossy_min_correlation(
    kind: ProductionArtifactKind,
    channels: u16,
) -> Result<f64, String> {
    match (kind, channels) {
        (ProductionArtifactKind::Mp3, 1) => Ok(MP3_PRODUCTION_MONO_MIN_CORRELATION),
        (ProductionArtifactKind::Mp3, 2) => Ok(MP3_PRODUCTION_STEREO_MIN_CORRELATION),
        (ProductionArtifactKind::Aac | ProductionArtifactKind::M4a, 1 | 2) => {
            Ok(AAC_PRODUCTION_MIN_CORRELATION)
        }
        (ProductionArtifactKind::Mp3, _) => {
            Err("MP3 production oracle floor supports mono/stereo only".to_owned())
        }
        (ProductionArtifactKind::Aac | ProductionArtifactKind::M4a, _) => {
            Err("AAC-LC production oracle floor supports mono/stereo only".to_owned())
        }
    }
}

pub(crate) fn verify_mp3_default_production_budget(
    label: &str,
    kind: ProductionArtifactKind,
    expected_pcm: &sonare_codec::AudioBuffer,
    bytes: &[u8],
) -> Result<(), String> {
    if !matches!(kind, ProductionArtifactKind::Mp3) {
        return Ok(());
    }
    verify_mp3_cbr_bitrate_budget(label, 128, expected_pcm, bytes)?;
    verify_mp3_production_reservoir(label, expected_pcm, bytes)
}

pub(crate) fn verify_mp3_production_reservoir(
    label: &str,
    expected_pcm: &sonare_codec::AudioBuffer,
    bytes: &[u8],
) -> Result<(), String> {
    if expected_pcm.channels == 1 {
        let expected = sonare_codec::encode_mpeg1_layer3_pcm_frames_with_entropy_targeted_perceptual_quantized_band_gain_and_global_gain_bias_reservoir_and_table_provider(
            expected_pcm,
            &[2.0],
            128,
            false,
            0,
            sonare_codec::Layer3QuantizedBandGain {
                band_start: 0,
                band_end: 7,
                gain: 1.5,
            },
            -4,
            sonare_codec::mpeg1_layer3_standard_table_provider(),
        )
        .map_err(|err| {
            format!("{label} MP3 low-band gain/global-gain-bias reservoir encode failed: {err}")
        })?;
        if bytes != expected {
            return Err(format!(
                "{label} MP3 production did not match the low-band gain/global-gain-bias reservoir profile"
            ));
        }

        let mut offset = 0usize;
        let mut frame_count = 0usize;
        let mut max_main_data_begin = 0u32;
        while offset < bytes.len() {
            let header = sonare_codec::FrameHeader::parse(&bytes[offset..])
                .map_err(|err| format!("{label} MP3 reservoir check failed: {err}"))?;
            let side_info_offset = offset
                .checked_add(4)
                .ok_or_else(|| format!("{label} MP3 reservoir check offset overflows"))?;
            if side_info_offset + 1 >= bytes.len() {
                return Err(format!(
                    "{label} MP3 reservoir check failed: frame side-info extends past stream length {}",
                    bytes.len()
                ));
            }
            let main_data_begin = (u32::from(bytes[side_info_offset]) << 1)
                | (u32::from(bytes[side_info_offset + 1]) >> 7);
            max_main_data_begin = max_main_data_begin.max(main_data_begin);
            offset = offset
                .checked_add(header.frame_len())
                .ok_or_else(|| format!("{label} MP3 reservoir check frame length overflows"))?;
            frame_count += 1;
        }
        if frame_count == 0 || max_main_data_begin == 0 {
            return Err(format!(
                "{label} MP3 low-band gain reservoir check failed: production stream never used main_data_begin"
            ));
        }
        eprintln!(
            "{label} MP3 production low-band gain reservoir: frame_count={frame_count}, max_main_data_begin={max_main_data_begin}"
        );
        return Ok(());
    }

    let production_candidates =
        sonare_codec::mpeg1_layer3_production_pcm_step_candidates(expected_pcm.channels)
            .map_err(|err| format!("{label} MP3 production candidate lookup failed: {err}"))?;
    let reservoir_details = sonare_codec::select_mpeg1_layer3_entropy_targeted_perceptual_reservoir_frame_details_with_table_provider(
            expected_pcm,
            production_candidates,
            128,
            false,
            0,
            sonare_codec::mpeg1_layer3_standard_table_provider(),
        )
        .map_err(|err| {
            format!("{label} MP3 entropy-targeted perceptual reservoir detail selection failed: {err}")
        })?;
    let frame_entropy_targets =
        mp3_perceptual_bit_allocation_targets_by_frame(label, expected_pcm, &reservoir_details)?;

    let mut offset = 0usize;
    let mut frame_count = 0usize;
    let mut max_main_data_begin = 0u32;
    while offset < bytes.len() {
        let header = sonare_codec::FrameHeader::parse(&bytes[offset..])
            .map_err(|err| format!("{label} MP3 reservoir check failed: {err}"))?;
        let Some(detail) = reservoir_details.get(frame_count) else {
            return Err(format!(
                "{label} MP3 reservoir check failed: encoded stream has more frames than selector details"
            ));
        };
        let borrowed_budget_bits = detail
            .frame_capacity_bytes
            .checked_add(detail.main_data_begin)
            .and_then(|bytes| bytes.checked_mul(8))
            .ok_or_else(|| format!("{label} MP3 reservoir detail budget overflows"))?;
        if detail.payload_bit_len > borrowed_budget_bits {
            return Err(format!(
                "{label} MP3 reservoir check failed: selector detail frame {frame_count} payload_bits={} exceeds borrowed budget {borrowed_budget_bits}",
                detail.payload_bit_len
            ));
        }
        let side_info_offset = offset
            .checked_add(4)
            .ok_or_else(|| format!("{label} MP3 reservoir check offset overflows"))?;
        if side_info_offset + 1 >= bytes.len() {
            return Err(format!(
                "{label} MP3 reservoir check failed: frame side-info extends past stream length {}",
                bytes.len()
            ));
        }
        let main_data_begin = (u32::from(bytes[side_info_offset]) << 1)
            | (u32::from(bytes[side_info_offset + 1]) >> 7);
        if detail.main_data_begin != main_data_begin as usize {
            return Err(format!(
                "{label} MP3 reservoir check failed: frame {frame_count} side-info main_data_begin={main_data_begin} does not match selector detail {}",
                detail.main_data_begin
            ));
        }
        max_main_data_begin = max_main_data_begin.max(main_data_begin);
        offset = offset
            .checked_add(header.frame_len())
            .ok_or_else(|| format!("{label} MP3 reservoir check frame length overflows"))?;
        frame_count += 1;
    }
    if frame_count != reservoir_details.len() {
        return Err(format!(
            "{label} MP3 reservoir check failed: encoded frame_count={frame_count} does not match selector detail count {}",
            reservoir_details.len()
        ));
    }
    if max_main_data_begin == 0 {
        return Err(format!(
            "{label} MP3 reservoir check failed: production stream never used main_data_begin"
        ));
    }
    let granules_per_frame = if expected_pcm.channels == 1 {
        2_usize
    } else {
        4_usize
    };
    for (frame_index, detail) in reservoir_details.iter().enumerate() {
        if detail.perceptual_granules + detail.calibrated_granules != granules_per_frame {
            return Err(format!(
                "{label} MP3 reservoir check failed: frame {frame_index} granule telemetry is inconsistent: perceptual={}, calibrated={}, expected={granules_per_frame}",
                detail.perceptual_granules, detail.calibrated_granules
            ));
        }
        if detail.quality_guard_compared_granules != 0
            || detail.quality_guard_distortion_delta != 0.0
        {
            return Err(format!(
                "{label} MP3 reservoir check failed: production unexpectedly reported quality guard telemetry on frame {frame_index}"
            ));
        }
        if detail.entropy_target_bits == 0 {
            return Err(format!(
                "{label} MP3 reservoir check failed: frame {frame_index} did not receive entropy target bits"
            ));
        }
        if detail.entropy_target_bits != frame_entropy_targets[frame_index] {
            return Err(format!(
                "{label} MP3 reservoir check failed: frame {frame_index} entropy target bits {} did not match perceptual allocation target {}",
                detail.entropy_target_bits, frame_entropy_targets[frame_index]
            ));
        }
        if detail.used_entropy_target_budget {
            let entropy_budget_bytes = detail
                .entropy_target_bits
                .saturating_add(7)
                .checked_div(8)
                .unwrap_or(0)
                .clamp(1, detail.frame_capacity_bytes + detail.main_data_begin);
            let entropy_budget_bits = entropy_budget_bytes
                .checked_mul(8)
                .ok_or_else(|| format!("{label} MP3 entropy target budget bits overflow"))?;
            if detail.payload_bit_len > entropy_budget_bits {
                return Err(format!(
                    "{label} MP3 reservoir check failed: frame {frame_index} used entropy target budget but payload_bits={} exceeds entropy_budget_bits={entropy_budget_bits}",
                    detail.payload_bit_len
                ));
            }
        }
    }
    let max_reservoir_after = reservoir_details
        .iter()
        .map(|detail| detail.reservoir_after)
        .max()
        .unwrap_or(0);
    let min_step = reservoir_details
        .iter()
        .map(|detail| detail.step)
        .fold(f32::INFINITY, f32::min);
    let max_payload_bits = reservoir_details
        .iter()
        .map(|detail| detail.payload_bit_len)
        .max()
        .unwrap_or(0);
    let perceptual_granules = reservoir_details
        .iter()
        .map(|detail| detail.perceptual_granules)
        .sum::<usize>();
    let calibrated_granules = reservoir_details
        .iter()
        .map(|detail| detail.calibrated_granules)
        .sum::<usize>();
    let quality_guard_compared_granules = reservoir_details
        .iter()
        .map(|detail| detail.quality_guard_compared_granules)
        .sum::<usize>();
    let quality_guard_distortion_delta = reservoir_details
        .iter()
        .map(|detail| detail.quality_guard_distortion_delta)
        .sum::<f64>();
    let entropy_target_bits = reservoir_details
        .iter()
        .map(|detail| detail.entropy_target_bits)
        .sum::<usize>();
    let capacity_bits = reservoir_details
        .iter()
        .map(|detail| detail.frame_capacity_bytes * 8)
        .sum::<usize>();
    if entropy_target_bits != capacity_bits {
        return Err(format!(
            "{label} MP3 reservoir check failed: entropy target bits {entropy_target_bits} did not match capacity bits {capacity_bits}"
        ));
    }
    let entropy_target_budget_frames = reservoir_details
        .iter()
        .filter(|detail| detail.used_entropy_target_budget)
        .count();
    let entropy_profile =
        sonare_codec::mpeg1_layer3_entropy_target_utilization_profile(&reservoir_details);
    let selected_entropy_profile =
        sonare_codec::select_mpeg1_layer3_entropy_target_utilization_profile_with_table_provider(
            expected_pcm,
            production_candidates,
            128,
            false,
            0,
            sonare_codec::mpeg1_layer3_standard_table_provider(),
        )
        .map_err(|err| {
            format!("{label} MP3 entropy-target utilization profile selection failed: {err}")
        })?;
    if entropy_profile != selected_entropy_profile {
        return Err(format!(
            "{label} MP3 reservoir check failed: entropy utilization profile drifted: detail_profile={entropy_profile:?}, selected_profile={selected_entropy_profile:?}"
        ));
    }
    if entropy_target_budget_frames == 0 {
        return Err(format!(
            "{label} MP3 reservoir check failed: no frame used the entropy target budget path"
        ));
    }
    if entropy_profile.payload_bits == 0 {
        return Err(format!(
            "{label} MP3 reservoir check failed: entropy target budget path carried no payload bits"
        ));
    }
    eprintln!(
        "{label} MP3 production entropy-targeted reservoir: min_step={min_step}, max_payload_bits={max_payload_bits}, max_main_data_begin={max_main_data_begin}, max_reservoir_after={max_reservoir_after}, perceptual_granules={perceptual_granules}, calibrated_granules={calibrated_granules}, quality_guard_compared_granules={quality_guard_compared_granules}, quality_guard_distortion_delta={quality_guard_distortion_delta:.9e}, entropy_target_bits={entropy_target_bits}, entropy_target_budget_frames={entropy_target_budget_frames}, entropy_payload_bits={}, entropy_budget_bits={}, entropy_budget_utilization={:.3}, max_entropy_budget_slack_bits={}, allocation_frames={}",
        entropy_profile.payload_bits,
        entropy_profile.entropy_budget_bits,
        entropy_profile.utilization,
        entropy_profile.max_entropy_budget_slack_bits,
        frame_entropy_targets.len()
    );
    Ok(())
}

pub(crate) fn mp3_perceptual_bit_allocation_targets_by_frame(
    label: &str,
    expected_pcm: &sonare_codec::AudioBuffer,
    reservoir_details: &[sonare_codec::Layer3EntropyTargetedReservoirFrameSelection],
) -> Result<Vec<usize>, String> {
    let allocations = sonare_codec::select_mpeg1_layer3_perceptual_bit_allocation_with_bitrate(
        expected_pcm,
        128,
        false,
        0,
    )
    .map_err(|err| format!("{label} MP3 perceptual bit allocation failed: {err}"))?;
    let mut frame_targets = vec![0usize; reservoir_details.len()];
    for allocation in allocations {
        let Some(frame_target) = frame_targets.get_mut(allocation.frame_index) else {
            return Err(format!(
                "{label} MP3 perceptual bit allocation returned out-of-range frame {} for {} reservoir frames",
                allocation.frame_index,
                reservoir_details.len()
            ));
        };
        *frame_target = frame_target
            .checked_add(allocation.target_bits)
            .ok_or_else(|| format!("{label} MP3 perceptual bit allocation target overflows"))?;
    }
    if let Some((frame_index, _)) = frame_targets
        .iter()
        .enumerate()
        .find(|(_, target_bits)| **target_bits == 0)
    {
        return Err(format!(
            "{label} MP3 perceptual bit allocation returned zero target bits for frame {frame_index}"
        ));
    }
    let allocation_target_bits = frame_targets.iter().sum::<usize>();
    let reservoir_target_bits = reservoir_details
        .iter()
        .map(|detail| detail.entropy_target_bits)
        .sum::<usize>();
    if allocation_target_bits != reservoir_target_bits {
        return Err(format!(
            "{label} MP3 perceptual bit allocation total target bits {allocation_target_bits} did not match reservoir entropy target bits {reservoir_target_bits}"
        ));
    }
    Ok(frame_targets)
}

pub(crate) fn verify_mp3_perceptual_reservoir(
    label: &str,
    expected_pcm: &sonare_codec::AudioBuffer,
    bytes: &[u8],
) -> Result<(), String> {
    let reservoir_details =
        sonare_codec::select_mpeg1_layer3_perceptual_reservoir_frame_details_with_table_provider(
            expected_pcm,
            MP3_PERCEPTUAL_DIAGNOSTIC_STEP_CANDIDATES,
            128,
            false,
            sonare_codec::mpeg1_layer3_standard_table_provider(),
        )
        .map_err(|err| {
            format!("{label} MP3 perceptual reservoir detail selection failed: {err}")
        })?;

    let mut offset = 0usize;
    let mut frame_count = 0usize;
    let mut max_main_data_begin = 0u32;
    while offset < bytes.len() {
        let header = sonare_codec::FrameHeader::parse(&bytes[offset..])
            .map_err(|err| format!("{label} MP3 perceptual reservoir check failed: {err}"))?;
        let Some(detail) = reservoir_details.get(frame_count) else {
            return Err(format!(
                "{label} MP3 perceptual reservoir check failed: encoded stream has more frames than selector details"
            ));
        };
        let borrowed_budget_bits = detail
            .frame_capacity_bytes
            .checked_add(detail.main_data_begin)
            .and_then(|bytes| bytes.checked_mul(8))
            .ok_or_else(|| format!("{label} MP3 perceptual reservoir detail budget overflows"))?;
        if detail.payload_bit_len > borrowed_budget_bits {
            return Err(format!(
                "{label} MP3 perceptual reservoir check failed: selector detail frame {frame_count} payload_bits={} exceeds borrowed budget {borrowed_budget_bits}",
                detail.payload_bit_len
            ));
        }
        let side_info_offset = offset
            .checked_add(4)
            .ok_or_else(|| format!("{label} MP3 perceptual reservoir check offset overflows"))?;
        if side_info_offset + 1 >= bytes.len() {
            return Err(format!(
                "{label} MP3 perceptual reservoir check failed: frame side-info extends past stream length {}",
                bytes.len()
            ));
        }
        let main_data_begin = (u32::from(bytes[side_info_offset]) << 1)
            | (u32::from(bytes[side_info_offset + 1]) >> 7);
        if detail.main_data_begin != main_data_begin as usize {
            return Err(format!(
                "{label} MP3 perceptual reservoir check failed: frame {frame_count} side-info main_data_begin={main_data_begin} does not match selector detail {}",
                detail.main_data_begin
            ));
        }
        max_main_data_begin = max_main_data_begin.max(main_data_begin);
        offset = offset.checked_add(header.frame_len()).ok_or_else(|| {
            format!("{label} MP3 perceptual reservoir check frame length overflows")
        })?;
        frame_count += 1;
    }
    if frame_count != reservoir_details.len() {
        return Err(format!(
            "{label} MP3 perceptual reservoir check failed: encoded frame_count={frame_count} does not match selector detail count {}",
            reservoir_details.len()
        ));
    }
    if max_main_data_begin == 0 {
        return Err(format!(
            "{label} MP3 perceptual reservoir check failed: stream never used main_data_begin"
        ));
    }
    let max_reservoir_after = reservoir_details
        .iter()
        .map(|detail| detail.reservoir_after)
        .max()
        .unwrap_or(0);
    let min_step = reservoir_details
        .iter()
        .map(|detail| detail.step)
        .fold(f32::INFINITY, f32::min);
    let max_payload_bits = reservoir_details
        .iter()
        .map(|detail| detail.payload_bit_len)
        .max()
        .unwrap_or(0);
    let perceptual_granules = reservoir_details
        .iter()
        .map(|detail| detail.perceptual_granules)
        .sum::<usize>();
    let calibrated_granules = reservoir_details
        .iter()
        .map(|detail| detail.calibrated_granules)
        .sum::<usize>();
    let quality_guard_compared_granules = reservoir_details
        .iter()
        .map(|detail| detail.quality_guard_compared_granules)
        .sum::<usize>();
    let quality_guard_distortion_delta = reservoir_details
        .iter()
        .map(|detail| detail.quality_guard_distortion_delta)
        .sum::<f64>();
    eprintln!(
        "{label} MP3 perceptual reservoir: min_step={min_step}, max_payload_bits={max_payload_bits}, max_main_data_begin={max_main_data_begin}, max_reservoir_after={max_reservoir_after}, perceptual_granules={perceptual_granules}, calibrated_granules={calibrated_granules}, quality_guard_compared_granules={quality_guard_compared_granules}, quality_guard_distortion_delta={quality_guard_distortion_delta:.9e}"
    );
    Ok(())
}

pub(crate) fn verify_mp3_cbr_bitrate_budget(
    label: &str,
    bitrate_kbps: u16,
    expected_pcm: &sonare_codec::AudioBuffer,
    bytes: &[u8],
) -> Result<(), String> {
    let expected_header = sonare_codec::layer3_header_for_capacity(
        expected_pcm.sample_rate,
        expected_pcm.channels,
        bitrate_kbps,
        false,
        false,
    )
    .map_err(|err| format!("{label} MP3 CBR budget failed: {err}"))?;
    let expected_frames = expected_pcm
        .frames()
        .div_ceil(usize::from(expected_header.samples_per_frame()))
        .max(1);
    let slot_remainder = 144 * usize::from(bitrate_kbps) * 1000 % expected_pcm.sample_rate as usize;

    let mut offset = 0usize;
    let mut frame_count = 0usize;
    let mut padding_accumulator = 0usize;
    let mut padded_frames = 0usize;
    while offset < bytes.len() {
        let mut expected_frame_header = expected_header;
        padding_accumulator += slot_remainder;
        if padding_accumulator >= expected_pcm.sample_rate as usize {
            padding_accumulator -= expected_pcm.sample_rate as usize;
            expected_frame_header.padding = true;
            padded_frames += 1;
        }
        let header = sonare_codec::FrameHeader::parse(&bytes[offset..])
            .map_err(|err| format!("{label} MP3 CBR budget failed: {err}"))?;
        if header != expected_frame_header {
            return Err(format!(
                "{label} MP3 CBR budget failed: frame {frame_count} header {header:?} does not match expected {bitrate_kbps}kbps CBR header {expected_frame_header:?}"
            ));
        }
        let frame_len = header.frame_len();
        let expected_frame_len = expected_frame_header.frame_len();
        if frame_len != expected_frame_len {
            return Err(format!(
                "{label} MP3 CBR budget failed: frame {frame_count} length {frame_len} does not match expected {expected_frame_len}"
            ));
        }
        let next = offset
            .checked_add(frame_len)
            .ok_or_else(|| format!("{label} MP3 CBR frame length overflows"))?;
        if next > bytes.len() {
            return Err(format!(
                "{label} MP3 CBR budget failed: frame {frame_count} extends past stream length {}",
                bytes.len()
            ));
        }
        let capacity = sonare_codec::layer3_main_data_capacity_bytes(header)
            .map_err(|err| format!("{label} MP3 CBR capacity failed: {err}"))?;
        let expected_capacity =
            sonare_codec::layer3_main_data_capacity_bytes(expected_frame_header)
                .map_err(|err| format!("{label} MP3 CBR capacity failed: {err}"))?;
        if capacity != expected_capacity {
            return Err(format!(
                "{label} MP3 CBR budget failed: frame {frame_count} capacity {capacity} does not match expected {expected_capacity}"
            ));
        }
        frame_count += 1;
        offset = next;
    }

    if frame_count == 0 {
        return Err(format!(
            "{label} MP3 CBR budget failed: stream has no complete frames"
        ));
    }
    if frame_count != expected_frames {
        return Err(format!(
            "{label} MP3 CBR budget failed: frame_count={frame_count} does not match expected {expected_frames}"
        ));
    }

    eprintln!(
        "{label} MP3 CBR budget: frames={frame_count}, padded_frames={padded_frames}, bitrate_kbps={bitrate_kbps}"
    );
    Ok(())
}

pub(crate) fn verify_aac_default_production_budget(
    label: &str,
    kind: ProductionArtifactKind,
    expected_pcm: &sonare_codec::AudioBuffer,
    bytes: &[u8],
) -> Result<(), String> {
    let adts = match kind {
        ProductionArtifactKind::Mp3 => return Ok(()),
        ProductionArtifactKind::Aac => bytes.to_vec(),
        ProductionArtifactKind::M4a => sonare_codec::demux_m4a_as_aac_adts(bytes)
            .map_err(|err| format!("{label} production M4A demux for budget failed: {err}"))?,
    };
    let default_bitrate = sonare_codec::aac_lc_default_production_bitrate_bps(
        u8::try_from(expected_pcm.channels)
            .map_err(|_| format!("{label} production channel count exceeds AAC-LC range"))?,
    )
    .map_err(|err| format!("{label} production AAC default bitrate failed: {err}"))?;
    let max_budget = sonare_codec::aac_lc_adts_max_frame_len_for_bitrate(
        expected_pcm.sample_rate,
        default_bitrate,
    )
    .map_err(|err| format!("{label} production AAC frame budget failed: {err}"))?;
    let max_frame_len = max_adts_frame_len(&adts)
        .map_err(|err| format!("{label} production ADTS frame budget failed: {err}"))?;
    let frame_details = sonare_codec::aac_selected_scale_factor_frame_details_with_bitrate(
        expected_pcm,
        default_bitrate,
    )
    .map_err(|err| format!("{label} production AAC frame details failed: {err}"))?;
    let selector_max_frame_len = frame_details
        .iter()
        .map(|selection| selection.frame_len)
        .max()
        .unwrap_or(0);
    if selector_max_frame_len != max_frame_len {
        return Err(format!(
            "{label} production AAC selector detail mismatch: selector_max_frame_len={selector_max_frame_len}, encoded_max_frame_len={max_frame_len}"
        ));
    }

    validate_adts_frame_budget(label, max_frame_len, max_budget, default_bitrate)?;

    eprintln!(
        "{label} production ADTS frame budget: max_frame_len={max_frame_len}, default_budget={max_budget}, default_bitrate_bps={default_bitrate}, {}",
        aac_step_selection_summary(&frame_details)
    );
    Ok(())
}

pub(crate) fn validate_adts_frame_budget(
    label: &str,
    max_frame_len: usize,
    max_budget: usize,
    bitrate_bps: u32,
) -> Result<(), String> {
    if max_frame_len > max_budget {
        return Err(format!(
            "{label} ADTS frame budget failed: max_frame_len={max_frame_len} exceeds budget {max_budget} for {bitrate_bps}bps"
        ));
    }
    Ok(())
}

pub(crate) fn max_adts_frame_len(stream: &[u8]) -> Result<usize, String> {
    let mut offset = 0usize;
    let mut max_frame_len = 0usize;
    let mut frame_count = 0usize;
    while offset + 7 <= stream.len() {
        if stream[offset] != 0xff || stream[offset + 1] & 0xf0 != 0xf0 {
            return Err(format!("missing ADTS syncword at byte offset {offset}"));
        }
        let frame_len = (((stream[offset + 3] & 0x03) as usize) << 11)
            | ((stream[offset + 4] as usize) << 3)
            | ((stream[offset + 5] as usize) >> 5);
        if frame_len < 7 {
            return Err(format!(
                "invalid ADTS frame length {frame_len} at byte offset {offset}"
            ));
        }
        let next = offset
            .checked_add(frame_len)
            .ok_or_else(|| "ADTS frame length overflow".to_owned())?;
        if next > stream.len() {
            return Err(format!(
                "ADTS frame at byte offset {offset} extends past stream length {}",
                stream.len()
            ));
        }
        max_frame_len = max_frame_len.max(frame_len);
        frame_count += 1;
        offset = next;
    }

    if frame_count == 0 {
        return Err("ADTS stream has no complete frames".to_owned());
    }
    if offset != stream.len() {
        return Err(format!(
            "ADTS stream has {} trailing byte(s) after the last complete frame",
            stream.len() - offset
        ));
    }

    Ok(max_frame_len)
}
