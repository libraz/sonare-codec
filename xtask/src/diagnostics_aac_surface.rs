use super::*;

pub(crate) fn readiness_pcm(
    sample_rate: u32,
    channels: u16,
) -> Result<sonare_codec::AudioBuffer, sonare_codec::Error> {
    let mut samples = Vec::with_capacity(2304 * usize::from(channels));
    for frame in 0..2304 {
        for channel in 0..channels {
            let phase = if channel == 0 { 0.01 } else { 0.013 };
            samples.push(((frame as f32) * phase).sin() * 0.25);
        }
    }
    sonare_codec::AudioBuffer::new(sample_rate, channels, samples)
}

pub(crate) fn compatibility_lossy_encode_diagnostics(
    ffmpeg: &OsStr,
    expected_pcm: &sonare_codec::AudioBuffer,
) -> Result<Vec<String>, String> {
    let out_dir = env::temp_dir().join(format!(
        "sonare-codec-compatibility-readiness-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |duration| duration.as_millis())
    ));
    fs::create_dir_all(&out_dir)
        .map_err(|err| format!("failed to create {}: {err}", out_dir.display()))?;

    let mut diagnostics = Vec::new();
    for (label, format) in [
        ("MP3", sonare_codec::Format::Mp3),
        ("AAC-LC", sonare_codec::Format::Aac),
    ] {
        let diagnostic =
            compatibility_lossy_encode_diagnostic(ffmpeg, expected_pcm, &out_dir, label, format);
        diagnostics.push(match diagnostic {
            Ok(quality) => format!(
                "{label} compatibility scaffold passes current oracle: decoded_rms={:.4}, best_correlation={:.3}",
                quality.decoded_rms, quality.best_correlation
            ),
            Err(err) => format!("{label} compatibility scaffold cannot be promoted: {err}"),
        });
    }
    let mp3_standard = standard_mp3_nonzero_encode_diagnostic(ffmpeg, expected_pcm, &out_dir);
    diagnostics.push(match mp3_standard {
        Ok(quality) => format!(
            "MP3 standard-table scaffold is still not production-gated: decoded_rms={:.4}, best_correlation={:.3}",
            quality.decoded_rms, quality.best_correlation
        ),
        Err(err) => format!("MP3 standard-table scaffold is not publish-ready: {err}"),
    });
    let mp3_perceptual = mp3_perceptual_nonzero_encode_diagnostic(ffmpeg, expected_pcm, &out_dir);
    diagnostics.push(match mp3_perceptual {
        Ok(quality) => format!(
            "MP3 perceptual-scale-factor scaffold is still not production-gated: decoded_rms={:.4}, best_correlation={:.3}",
            quality.decoded_rms, quality.best_correlation
        ),
        Err(err) => {
            format!("MP3 perceptual-scale-factor scaffold is not publish-ready: {err}")
        }
    });
    let mp3_perceptual_reservoir =
        mp3_perceptual_reservoir_nonzero_encode_diagnostic(ffmpeg, expected_pcm, &out_dir);
    diagnostics.push(match mp3_perceptual_reservoir {
        Ok(quality) => format!(
            "MP3 perceptual reservoir scaffold is still not production-gated: decoded_rms={:.4}, best_correlation={:.3}",
            quality.decoded_rms, quality.best_correlation
        ),
        Err(err) => {
            format!("MP3 perceptual reservoir scaffold is not publish-ready: {err}")
        }
    });
    let aac_experimental =
        experimental_aac_lc_nonzero_encode_diagnostic(ffmpeg, expected_pcm, &out_dir);
    diagnostics.push(match aac_experimental {
        Ok(quality) => format!(
            "AAC-LC experimental nonzero scaffold is still not production-gated: decoded_rms={:.4}, best_correlation={:.3}",
            quality.decoded_rms, quality.best_correlation
        ),
        Err(err) => format!("AAC-LC experimental nonzero scaffold is not publish-ready: {err}"),
    });
    let aac_standard = standard_aac_lc_nonzero_encode_diagnostic(ffmpeg, expected_pcm, &out_dir);
    diagnostics.push(match aac_standard {
        Ok(quality) => format!(
            "AAC-LC standard-table scaffold is still not production-gated: decoded_rms={:.4}, best_correlation={:.3}",
            quality.decoded_rms, quality.best_correlation
        ),
        Err(err) => format!("AAC-LC standard-table scaffold is not publish-ready: {err}"),
    });

    fs::remove_dir_all(&out_dir)
        .map_err(|err| format!("failed to remove {}: {err}", out_dir.display()))?;
    Ok(diagnostics)
}

pub(crate) fn compatibility_lossy_encode_diagnostic(
    ffmpeg: &OsStr,
    expected_pcm: &sonare_codec::AudioBuffer,
    out_dir: &Path,
    label: &str,
    format: sonare_codec::Format,
) -> Result<LossyOraclePcmQuality, String> {
    let encoded = sonare_codec::encode(format, expected_pcm)
        .map_err(|err| format!("compatibility encode failed: {err}"))?;
    let extension = match format {
        sonare_codec::Format::Mp3 => "mp3",
        sonare_codec::Format::Aac => "aac",
        _ => {
            return Err(format!(
                "unexpected compatibility lossy format for oracle: {format:?}"
            ))
        }
    };
    let path = out_dir.join(format!(
        "{}-compatibility.{}",
        label.to_ascii_lowercase().replace('-', ""),
        extension
    ));
    fs::write(&path, encoded)
        .map_err(|err| format!("failed to write {}: {err}", path.display()))?;
    run_ffmpeg_acceptance(ffmpeg, &path)
        .map_err(|err| format!("FFmpeg acceptance failed: {err}"))?;
    let decoded = run_ffmpeg_decode_f32le(
        ffmpeg,
        &path,
        expected_pcm.sample_rate,
        expected_pcm.channels,
    )
    .map_err(|err| format!("FFmpeg PCM decode failed: {err}"))?;
    validate_lossy_oracle_pcm_quality(&expected_pcm.samples, &decoded)
}

pub(crate) fn standard_mp3_nonzero_encode_diagnostic(
    ffmpeg: &OsStr,
    expected_pcm: &sonare_codec::AudioBuffer,
    out_dir: &Path,
) -> Result<LossyOraclePcmQuality, String> {
    let encoded = sonare_codec::encode_mpeg1_layer3_pcm_frames_with_auto_step_and_table_provider(
        expected_pcm,
        sonare_codec::MPEG1_LAYER3_PCM_STEP_CANDIDATES,
        sonare_codec::mpeg1_layer3_standard_table_provider(),
    )
    .map_err(|err| format!("standard-table encode failed: {err}"))?;
    let path = out_dir.join("mp3-standard-table-nonzero.mp3");
    fs::write(&path, encoded)
        .map_err(|err| format!("failed to write {}: {err}", path.display()))?;
    run_ffmpeg_acceptance(ffmpeg, &path)
        .map_err(|err| format!("FFmpeg acceptance failed: {err}"))?;
    let decoded = run_ffmpeg_decode_f32le(
        ffmpeg,
        &path,
        expected_pcm.sample_rate,
        expected_pcm.channels,
    )
    .map_err(|err| format!("FFmpeg PCM decode failed: {err}"))?;
    validate_lossy_oracle_pcm_quality(&expected_pcm.samples, &decoded)
}

pub(crate) fn mp3_perceptual_nonzero_encode_diagnostic(
    ffmpeg: &OsStr,
    expected_pcm: &sonare_codec::AudioBuffer,
    out_dir: &Path,
) -> Result<LossyOraclePcmQuality, String> {
    eprintln!(
        "{}",
        mp3_perceptual_diagnostic_summary(expected_pcm, MP3_PERCEPTUAL_DIAGNOSTIC_STEP_CANDIDATES)?
    );
    let encoded =
        sonare_codec::encode_mpeg1_layer3_pcm_frames_with_perceptual_active_cbr_bitrate_and_table_provider(
            expected_pcm,
            MP3_PERCEPTUAL_DIAGNOSTIC_STEP_CANDIDATES,
            128,
            false,
            sonare_codec::mpeg1_layer3_standard_table_provider(),
        )
        .map_err(|err| format!("perceptual-scale-factor encode failed: {err}"))?;
    let path = out_dir.join("mp3-perceptual-scale-factor-nonzero.mp3");
    verify_mp3_cbr_bitrate_budget(
        "MP3 perceptual-scale-factor diagnostic",
        128,
        expected_pcm,
        &encoded,
    )?;
    fs::write(&path, encoded)
        .map_err(|err| format!("failed to write {}: {err}", path.display()))?;
    run_ffmpeg_acceptance(ffmpeg, &path)
        .map_err(|err| format!("FFmpeg acceptance failed: {err}"))?;
    let decoded = run_ffmpeg_decode_f32le(
        ffmpeg,
        &path,
        expected_pcm.sample_rate,
        expected_pcm.channels,
    )
    .map_err(|err| format!("FFmpeg PCM decode failed: {err}"))?;
    let quality = validate_lossy_oracle_pcm_quality(&expected_pcm.samples, &decoded)?;
    validate_diagnostic_quality_floor(
        "MP3 perceptual-scale-factor diagnostic",
        quality,
        MP3_PERCEPTUAL_DIAGNOSTIC_MIN_DECODED_RMS,
        MP3_PERCEPTUAL_DIAGNOSTIC_MIN_CORRELATION,
    )?;
    Ok(quality)
}

pub(crate) fn mp3_perceptual_reservoir_nonzero_encode_diagnostic(
    ffmpeg: &OsStr,
    expected_pcm: &sonare_codec::AudioBuffer,
    out_dir: &Path,
) -> Result<LossyOraclePcmQuality, String> {
    let label = if expected_pcm.channels == 2 {
        "MP3 stereo perceptual reservoir diagnostic"
    } else {
        "MP3 perceptual reservoir diagnostic"
    };
    let encoded =
        sonare_codec::encode_mpeg1_layer3_pcm_frames_with_perceptual_reservoir_and_table_provider(
            expected_pcm,
            MP3_PERCEPTUAL_DIAGNOSTIC_STEP_CANDIDATES,
            128,
            false,
            sonare_codec::mpeg1_layer3_standard_table_provider(),
        )
        .map_err(|err| format!("perceptual reservoir encode failed: {err}"))?;
    let path = out_dir.join(if expected_pcm.channels == 2 {
        "mp3-stereo-perceptual-reservoir-nonzero.mp3"
    } else {
        "mp3-perceptual-reservoir-nonzero.mp3"
    });
    verify_mp3_cbr_bitrate_budget(label, 128, expected_pcm, &encoded)?;
    verify_mp3_perceptual_reservoir(label, expected_pcm, &encoded)?;
    fs::write(&path, encoded)
        .map_err(|err| format!("failed to write {}: {err}", path.display()))?;
    run_ffmpeg_acceptance(ffmpeg, &path)
        .map_err(|err| format!("FFmpeg acceptance failed: {err}"))?;
    let decoded = run_ffmpeg_decode_f32le(
        ffmpeg,
        &path,
        expected_pcm.sample_rate,
        expected_pcm.channels,
    )
    .map_err(|err| format!("FFmpeg PCM decode failed: {err}"))?;
    let quality = validate_lossy_oracle_pcm_quality(&expected_pcm.samples, &decoded)?;
    validate_diagnostic_quality_floor(
        label,
        quality,
        MP3_PERCEPTUAL_DIAGNOSTIC_MIN_DECODED_RMS,
        MP3_PERCEPTUAL_DIAGNOSTIC_MIN_CORRELATION,
    )?;
    Ok(quality)
}

pub(crate) fn validate_mp3_production_benchmark_surface(
    ffmpeg: &OsStr,
    mono_pcm: &sonare_codec::AudioBuffer,
    stereo_pcm: &sonare_codec::AudioBuffer,
    out_dir: &Path,
) -> Result<(LossyOraclePcmQuality, LossyOraclePcmQuality), String> {
    if mono_pcm.channels != 1 {
        return Err("MP3 production benchmark mono PCM must be mono".to_owned());
    }
    if stereo_pcm.channels != 2 {
        return Err("MP3 production benchmark stereo PCM must be stereo".to_owned());
    }
    let mono_quality = validate_mp3_production_benchmark_artifact(
        ffmpeg,
        "MP3 production benchmark mono",
        mono_pcm,
        out_dir,
        "mp3-production-benchmark-mono",
    )?;
    let stereo_quality = validate_mp3_production_benchmark_artifact(
        ffmpeg,
        "MP3 production benchmark stereo",
        stereo_pcm,
        out_dir,
        "mp3-production-benchmark-stereo",
    )?;
    Ok((mono_quality, stereo_quality))
}

pub(crate) fn validate_mp3_production_benchmark_artifact(
    ffmpeg: &OsStr,
    label: &str,
    expected_pcm: &sonare_codec::AudioBuffer,
    out_dir: &Path,
    file_stem: &str,
) -> Result<LossyOraclePcmQuality, String> {
    let mp3 = sonare_codec::encode_with_mode(
        sonare_codec::Format::Mp3,
        expected_pcm,
        sonare_codec::EncodeMode::ProductionOnly,
    )
    .map_err(|err| format!("{label} encode failed: {err}"))?;
    verify_mp3_default_production_budget(label, ProductionArtifactKind::Mp3, expected_pcm, &mp3)?;
    let path = out_dir.join(format!("{file_stem}.mp3"));
    fs::write(&path, mp3).map_err(|err| format!("failed to write {}: {err}", path.display()))?;
    run_ffmpeg_acceptance(ffmpeg, &path)
        .map_err(|err| format!("{label} FFmpeg acceptance failed: {err}"))?;
    let decoded = run_ffmpeg_decode_f32le(
        ffmpeg,
        &path,
        expected_pcm.sample_rate,
        expected_pcm.channels,
    )
    .map_err(|err| format!("{label} FFmpeg PCM decode failed: {err}"))?;
    let quality = validate_lossy_oracle_pcm_quality(&expected_pcm.samples, &decoded)
        .map_err(|err| format!("{label} PCM quality failed: {err}"))?;
    eprintln!(
        "{label}: decoded_rms={:.4}, best_correlation={:.3}",
        quality.decoded_rms, quality.best_correlation
    );
    Ok(quality)
}

pub(crate) fn mp3_perceptual_diagnostic_summary(
    expected_pcm: &sonare_codec::AudioBuffer,
    candidates: &[f32],
) -> Result<String, String> {
    const BITRATE_KBPS: u16 = 128;

    let base_header = sonare_codec::layer3_header_for_capacity(
        expected_pcm.sample_rate,
        expected_pcm.channels,
        BITRATE_KBPS,
        false,
        false,
    )
    .map_err(|err| format!("MP3 perceptual diagnostic header failed: {err}"))?;
    let samples_per_frame = usize::from(base_header.samples_per_frame());
    let channels = usize::from(expected_pcm.channels);
    let frames = expected_pcm.samples.len().div_ceil(channels);
    let frame_count = frames.div_ceil(samples_per_frame).max(1);
    let coefficient = if samples_per_frame == 1152 {
        144_u64
    } else {
        72_u64
    };
    let slots = coefficient
        .checked_mul(u64::from(BITRATE_KBPS))
        .and_then(|value| value.checked_mul(1000))
        .ok_or_else(|| "MP3 perceptual diagnostic bitrate slots overflow".to_owned())?;
    let sample_rate = u64::from(expected_pcm.sample_rate);
    let slot_remainder = slots % sample_rate;
    let mut accumulator = 0_u64;
    let mut padded_frames = 0usize;
    let mut min_step = f32::INFINITY;
    let mut max_step = 0.0_f32;
    let mut max_payload_bits = 0usize;
    let mut min_capacity_bits = usize::MAX;
    let mut nonzero_scale_factors = 0usize;
    let mut max_scale_factor = 0u8;
    let mut scale_factor_sum = 0usize;
    let mut scale_factor_bands = 0usize;
    let mut first_nonzero_scale_factor_step: Option<(f32, usize, usize)> = None;
    let mut first_frame_candidate_profile = Vec::new();
    for frame_index in 0..frame_count {
        accumulator += slot_remainder;
        let padded = if accumulator >= sample_rate {
            accumulator -= sample_rate;
            true
        } else {
            false
        };
        padded_frames += usize::from(padded);
        let frame_header = sonare_codec::layer3_header_for_capacity(
            expected_pcm.sample_rate,
            expected_pcm.channels,
            BITRATE_KBPS,
            padded,
            false,
        )
        .map_err(|err| format!("MP3 perceptual diagnostic frame header failed: {err}"))?;
        let start_frame = frame_index
            .checked_mul(samples_per_frame)
            .ok_or_else(|| "MP3 perceptual diagnostic frame start overflows".to_owned())?;
        let selection =
            sonare_codec::select_mpeg1_layer3_pcm_frame_perceptual_active_step_details_with_table_provider(
                frame_header,
                expected_pcm,
                start_frame,
                candidates,
                sonare_codec::mpeg1_layer3_standard_table_provider(),
            )
            .map_err(|err| format!("MP3 perceptual diagnostic step selection failed: {err}"))?;
        min_step = min_step.min(selection.step);
        max_step = max_step.max(selection.step);
        max_payload_bits = max_payload_bits.max(selection.payload_bit_len);
        min_capacity_bits = min_capacity_bits.min(selection.frame_capacity_bits);
        for granule in 0..(samples_per_frame / 576).max(1) {
            let granule_start = start_frame
                .checked_add(granule * 576)
                .ok_or_else(|| "MP3 perceptual diagnostic granule start overflows".to_owned())?;
            for channel in 0..usize::from(expected_pcm.channels) {
                let scale_factors =
                    sonare_codec::select_mpeg1_layer3_psychoacoustic_long_scale_factors(
                        expected_pcm,
                        channel,
                        granule_start,
                        selection.step,
                        false,
                        1024,
                    )
                    .map_err(|err| {
                        format!("MP3 perceptual diagnostic scale-factor selection failed: {err}")
                    })?;
                for scale_factor in scale_factors {
                    nonzero_scale_factors += usize::from(scale_factor != 0);
                    max_scale_factor = max_scale_factor.max(scale_factor);
                    scale_factor_sum += usize::from(scale_factor);
                    scale_factor_bands += 1;
                }
            }
        }
    }
    let candidate_profiles =
        sonare_codec::select_mpeg1_layer3_first_frame_perceptual_candidate_profile_with_table_provider(
            expected_pcm,
            candidates,
            BITRATE_KBPS,
            false,
            sonare_codec::mpeg1_layer3_standard_table_provider(),
        )
        .map_err(|err| format!("MP3 perceptual diagnostic candidate profile failed: {err}"))?;
    for profile in candidate_profiles {
        first_frame_candidate_profile.push(format!(
            "{}:{}b,{}/{},max{}",
            profile.step,
            profile.payload_bit_len,
            profile.nonzero_scale_factors,
            profile.scale_factor_bands,
            profile.max_scale_factor
        ));
        if profile.nonzero_scale_factors > 0 {
            first_nonzero_scale_factor_step = Some((
                profile.step,
                profile.payload_bit_len,
                profile.frame_capacity_bits,
            ));
            break;
        }
    }
    let first_nonzero_scale_factor_step = first_nonzero_scale_factor_step
        .map(|(step, payload_bits, capacity_bits)| {
            format!("{step} (payload_bits={payload_bits}, capacity_bits={capacity_bits})")
        })
        .unwrap_or_else(|| "none".to_owned());
    let mean_scale_factor = if scale_factor_bands == 0 {
        0.0
    } else {
        scale_factor_sum as f64 / scale_factor_bands as f64
    };
    let first_frame_candidate_profile = first_frame_candidate_profile.join("|");

    Ok(format!(
        "MP3 perceptual-scale-factor diagnostic selection: frames={frame_count}, padded_frames={padded_frames}, bitrate_kbps={BITRATE_KBPS}, step_range={min_step}..{max_step}, max_payload_bits={max_payload_bits}, min_capacity_bits={min_capacity_bits}, nonzero_scale_factors={nonzero_scale_factors}/{scale_factor_bands}, max_scale_factor={max_scale_factor}, mean_scale_factor={mean_scale_factor:.2}, first_nonzero_scale_factor_step={first_nonzero_scale_factor_step}, first_frame_candidate_profile=[{first_frame_candidate_profile}]"
    ))
}

pub(crate) fn experimental_aac_lc_nonzero_encode_diagnostic(
    ffmpeg: &OsStr,
    expected_pcm: &sonare_codec::AudioBuffer,
    out_dir: &Path,
) -> Result<LossyOraclePcmQuality, String> {
    if expected_pcm.channels != 1 {
        return Err("experimental AAC-LC diagnostic currently expects mono PCM".to_owned());
    }
    let offsets =
        sonare_codec::aac_lc_long_window_scale_factor_band_offsets(expected_pcm.sample_rate)
            .ok_or_else(|| {
                "experimental nonzero encode requires AAC-LC long-window scale-factor band offsets"
                    .to_owned()
            })?;
    let channel_config = sonare_codec::AacLongBlockConfig::new(
        180,
        u8::try_from(offsets.len() - 1)
            .map_err(|_| "AAC-LC scale-factor band count exceeds max_sfb range".to_owned())?,
    );
    let flat_scale_factors = vec![i16::from(channel_config.global_gain); offsets.len() - 1];
    let channel = sonare_codec::AacScaleFactorChannel::new(channel_config, &flat_scale_factors);
    let scale_factor_table = sonare_codec::aac_scale_factor_delta_table();
    let spectral_tables = sonare_codec::aac_unsigned_pairs7_unit_magnitude_spectral_tables();
    let encoded = sonare_codec::encode_pcm_mono_long_block_adts_stream_with_offsets_and_scale_factors_by_bit_cost(
            sonare_codec::AdtsConfig::aac_lc(expected_pcm.sample_rate, 1),
            channel,
            expected_pcm,
            offsets,
            sonare_codec::AAC_LC_PCM_STEP_CANDIDATES,
            &scale_factor_table,
            spectral_tables,
        )
    .map_err(|err| format!("experimental nonzero encode failed: {err}"))?;
    let path = out_dir.join("aaclc-experimental-nonzero.aac");
    fs::write(&path, encoded)
        .map_err(|err| format!("failed to write {}: {err}", path.display()))?;
    run_ffmpeg_acceptance(ffmpeg, &path)
        .map_err(|err| format!("FFmpeg acceptance failed: {err}"))?;
    let decoded = run_ffmpeg_decode_f32le(
        ffmpeg,
        &path,
        expected_pcm.sample_rate,
        expected_pcm.channels,
    )
    .map_err(|err| format!("FFmpeg PCM decode failed: {err}"))?;
    validate_lossy_oracle_pcm_quality(&expected_pcm.samples, &decoded)
}

pub(crate) fn standard_aac_lc_nonzero_encode_diagnostic(
    ffmpeg: &OsStr,
    expected_pcm: &sonare_codec::AudioBuffer,
    out_dir: &Path,
) -> Result<LossyOraclePcmQuality, String> {
    if expected_pcm.channels != 1 {
        return Err("standard AAC-LC diagnostic currently expects mono PCM".to_owned());
    }
    let offsets =
        sonare_codec::aac_lc_long_window_scale_factor_band_offsets(expected_pcm.sample_rate)
            .ok_or_else(|| {
                "standard nonzero encode requires AAC-LC long-window scale-factor band offsets"
                    .to_owned()
            })?;
    let max_sfb = u8::try_from(offsets.len() - 1)
        .map_err(|_| "AAC-LC scale-factor band count exceeds max_sfb range".to_owned())?;
    let scale_factor_table = sonare_codec::aac_scale_factor_delta_table();
    let bitrate = sonare_codec::aac_lc_default_production_bitrate_bps(1)
        .map_err(|err| format!("AAC-LC default bitrate lookup failed: {err}"))?;
    let budget =
        sonare_codec::aac_lc_adts_max_frame_len_for_bitrate(expected_pcm.sample_rate, bitrate)
            .map_err(|err| format!("AAC-LC default bitrate budget lookup failed: {err}"))?;
    let expected_rms = rms(&expected_pcm.samples);
    let mut best_candidate: Option<AacStandardDiagnosticCandidate> = None;
    for &global_gain in AAC_STANDARD_DIAGNOSTIC_GLOBAL_GAIN_CANDIDATES {
        match evaluate_aac_standard_diagnostic_candidate(
            ffmpeg,
            expected_pcm,
            out_dir,
            offsets,
            max_sfb,
            global_gain,
            budget,
            bitrate,
            &scale_factor_table,
        ) {
            Ok(candidate) => {
                eprintln!(
                    "AAC-LC standard-table diagnostic candidate: global_gain={}, step={}, frame_len={}, decoded_rms={:.4}, best_correlation={:.3}",
                    candidate.global_gain,
                    candidate.selected.step,
                    candidate.selected.frame_len,
                    candidate.quality.decoded_rms,
                    candidate.quality.best_correlation
                );
                best_candidate = match best_candidate {
                    Some(previous)
                        if aac_standard_candidate_is_at_least_as_good(
                            &previous,
                            &candidate,
                            expected_rms,
                        ) =>
                    {
                        Some(previous)
                    }
                    _ => Some(candidate),
                };
            }
            Err(err) => {
                eprintln!(
                    "AAC-LC standard-table diagnostic candidate rejected: global_gain={global_gain}, {err}"
                );
            }
        }
    }
    let best_candidate = best_candidate.ok_or_else(|| {
        "standard-table diagnostic found no FFmpeg-decodable candidate".to_owned()
    })?;
    eprintln!(
        "AAC-LC standard-table diagnostic selection: scale_factor_mode=fixed-search, global_gain={}, step={}, candidate_frame_len={}",
        best_candidate.global_gain, best_candidate.selected.step, best_candidate.selected.frame_len
    );
    let quantized =
        sonare_codec::quantize_pcm_long_block(expected_pcm, 0, 0, best_candidate.selected.step)
            .map_err(|err| format!("standard-table quantized diagnostic failed: {err}"))?;
    let sections = sonare_codec::plan_sections_by_offsets(
        &quantized,
        offsets,
        sonare_codec::aac_lc_standard_spectral_tables(),
    )
    .map_err(|err| format!("standard-table section diagnostic failed: {err}"))?;
    eprintln!(
        "{}",
        aac_section_diagnostic_summary(
            "AAC-LC standard-table diagnostic sections",
            &sections,
            &quantized
        )
    );
    validate_aac_standard_id_offsets_payload_for_diagnostic(&quantized, offsets)?;
    validate_aac_standard_id_offsets_encoded_candidate(
        ffmpeg,
        expected_pcm,
        out_dir,
        offsets,
        max_sfb,
        &best_candidate,
        budget,
        bitrate,
        &scale_factor_table,
    )?;
    validate_aac_standard_id_offsets_stereo_encoded_candidate(
        ffmpeg,
        expected_pcm,
        out_dir,
        offsets,
        max_sfb,
        &best_candidate,
        &scale_factor_table,
    )?;
    let max_frame_len = max_adts_frame_len(&best_candidate.encoded)
        .map_err(|err| format!("standard-table ADTS frame budget inspection failed: {err}"))?;
    validate_adts_frame_budget(
        "AAC-LC standard-table diagnostic",
        max_frame_len,
        budget,
        bitrate,
    )?;
    eprintln!(
        "AAC-LC standard-table diagnostic ADTS frame budget: max_frame_len={max_frame_len}, default_budget={budget}, default_bitrate_bps={bitrate}"
    );
    validate_diagnostic_quality_floor(
        "AAC-LC standard-table diagnostic",
        best_candidate.quality,
        AAC_STANDARD_DIAGNOSTIC_MIN_DECODED_RMS,
        AAC_STANDARD_DIAGNOSTIC_MIN_CORRELATION,
    )?;
    Ok(best_candidate.quality)
}

pub(crate) fn validate_aac_standard_id_high_level_bitrate_surface(
    ffmpeg: &OsStr,
    expected_pcm: &sonare_codec::AudioBuffer,
    out_dir: &Path,
) -> Result<(LossyOraclePcmQuality, LossyOraclePcmQuality), String> {
    if expected_pcm.channels != 1 {
        return Err("AAC standard-id high-level surface diagnostic expects mono PCM".to_owned());
    }

    let mono_bitrate = sonare_codec::aac_lc_default_production_bitrate_bps(1)
        .map_err(|err| format!("AAC standard-id surface mono bitrate failed: {err}"))?;
    let mono_candidate = select_aac_standard_id_high_level_gain_candidate(
        ffmpeg,
        "AAC-LC standard-id high-level mono ADTS",
        expected_pcm,
        ProductionArtifactKind::Aac,
        mono_bitrate,
        out_dir,
        "aaclc-standard-id-surface-mono",
    )?;
    eprintln!(
        "AAC-LC standard-id high-level mono ADTS selected global_gain={}, max_frame_len={}, decoded_rms={:.4}, best_correlation={:.3}",
        mono_candidate.global_gain,
        mono_candidate.max_frame_len,
        mono_candidate.quality.decoded_rms,
        mono_candidate.quality.best_correlation
    );

    let mono_m4a = sonare_codec::encode_m4a_with_standard_spectral_offsets_and_bitrate(
        expected_pcm,
        mono_bitrate,
        mono_candidate.global_gain,
    )
    .map_err(|err| format!("AAC standard-id surface mono M4A encode failed: {err}"))?;
    let mono_m4a_quality = validate_aac_standard_id_high_level_artifact(
        ffmpeg,
        "AAC-LC standard-id high-level mono M4A",
        expected_pcm,
        &mono_m4a,
        ProductionArtifactKind::M4a,
        mono_bitrate,
        &out_dir.join("aaclc-standard-id-surface-mono.m4a"),
    )?;
    if mono_m4a_quality.best_correlation + f64::EPSILON < mono_candidate.quality.best_correlation {
        return Err(format!(
            "AAC standard-id surface mono M4A quality lagged ADTS: m4a={mono_m4a_quality:?}, adts={:?}",
            mono_candidate.quality
        ));
    }
    let mono_selected_quality = validate_aac_standard_id_high_level_selected_bias_surface(
        ffmpeg,
        "AAC-LC standard-id high-level selected-scale-factor mono",
        expected_pcm,
        mono_bitrate,
        out_dir,
        "aaclc-standard-id-selected-surface-mono",
    )?;
    if mono_selected_quality.best_correlation + f64::EPSILON
        < mono_candidate.quality.best_correlation
    {
        return Err(format!(
            "AAC standard-id selected surface mono quality lagged fixed surface: selected={mono_selected_quality:?}, fixed={:?}",
            mono_candidate.quality
        ));
    }

    let stereo_pcm = aac_standard_surface_stereo_pcm(expected_pcm)?;
    let stereo_bitrate = sonare_codec::aac_lc_default_production_bitrate_bps(2)
        .map_err(|err| format!("AAC standard-id surface stereo bitrate failed: {err}"))?;
    let stereo_candidate = select_aac_standard_id_high_level_gain_candidate(
        ffmpeg,
        "AAC-LC standard-id high-level stereo ADTS",
        &stereo_pcm,
        ProductionArtifactKind::Aac,
        stereo_bitrate,
        out_dir,
        "aaclc-standard-id-surface-stereo",
    )?;
    eprintln!(
        "AAC-LC standard-id high-level stereo ADTS selected global_gain={}, max_frame_len={}, decoded_rms={:.4}, best_correlation={:.3}",
        stereo_candidate.global_gain,
        stereo_candidate.max_frame_len,
        stereo_candidate.quality.decoded_rms,
        stereo_candidate.quality.best_correlation
    );

    let stereo_m4a = sonare_codec::encode_m4a_with_standard_spectral_offsets_and_bitrate(
        &stereo_pcm,
        stereo_bitrate,
        stereo_candidate.global_gain,
    )
    .map_err(|err| format!("AAC standard-id surface stereo M4A encode failed: {err}"))?;
    let stereo_m4a_quality = validate_aac_standard_id_high_level_artifact(
        ffmpeg,
        "AAC-LC standard-id high-level stereo M4A",
        &stereo_pcm,
        &stereo_m4a,
        ProductionArtifactKind::M4a,
        stereo_bitrate,
        &out_dir.join("aaclc-standard-id-surface-stereo.m4a"),
    )?;
    if stereo_m4a_quality.best_correlation + f64::EPSILON
        < stereo_candidate.quality.best_correlation
    {
        return Err(format!(
            "AAC standard-id surface stereo M4A quality lagged ADTS: m4a={stereo_m4a_quality:?}, adts={:?}",
            stereo_candidate.quality
        ));
    }
    let stereo_selected_quality = validate_aac_standard_id_high_level_selected_bias_surface(
        ffmpeg,
        "AAC-LC standard-id high-level selected-scale-factor stereo",
        &stereo_pcm,
        stereo_bitrate,
        out_dir,
        "aaclc-standard-id-selected-surface-stereo",
    )?;
    if stereo_selected_quality.best_correlation + f64::EPSILON
        < stereo_candidate.quality.best_correlation
    {
        return Err(format!(
            "AAC standard-id selected surface stereo quality lagged fixed surface: selected={stereo_selected_quality:?}, fixed={:?}",
            stereo_candidate.quality
        ));
    }

    Ok((mono_selected_quality, stereo_selected_quality))
}

pub(crate) fn aac_standard_surface_stereo_pcm(
    mono_pcm: &sonare_codec::AudioBuffer,
) -> Result<sonare_codec::AudioBuffer, String> {
    if mono_pcm.channels != 1 {
        return Err("AAC standard-id high-level stereo fixture expects mono PCM".to_owned());
    }
    sonare_codec::AudioBuffer::new(
        mono_pcm.sample_rate,
        2,
        mono_pcm
            .samples
            .iter()
            .enumerate()
            .flat_map(|(index, &sample)| {
                let right = if index % 2 == 0 {
                    -sample * 0.75
                } else {
                    sample * 0.5
                };
                [sample, right]
            })
            .collect(),
    )
    .map_err(|err| format!("AAC standard-id high-level stereo PCM failed: {err}"))
}

pub(crate) fn validate_aac_production_benchmark_surface(
    ffmpeg: &OsStr,
    mono_pcm: &sonare_codec::AudioBuffer,
    out_dir: &Path,
) -> Result<(LossyOraclePcmQuality, LossyOraclePcmQuality), String> {
    if mono_pcm.channels != 1 {
        return Err("AAC production benchmark surface expects mono PCM".to_owned());
    }
    let mono_quality = validate_aac_production_benchmark_artifact(
        ffmpeg,
        "AAC-LC production benchmark mono",
        mono_pcm,
        out_dir,
        "aaclc-production-benchmark-mono",
    )?;
    let stereo_pcm = aac_standard_surface_stereo_pcm(mono_pcm)?;
    let stereo_quality = validate_aac_production_benchmark_artifact(
        ffmpeg,
        "AAC-LC production benchmark stereo",
        &stereo_pcm,
        out_dir,
        "aaclc-production-benchmark-stereo",
    )?;
    Ok((mono_quality, stereo_quality))
}

pub(crate) fn validate_aac_production_benchmark_artifact(
    ffmpeg: &OsStr,
    label: &str,
    expected_pcm: &sonare_codec::AudioBuffer,
    out_dir: &Path,
    file_stem: &str,
) -> Result<LossyOraclePcmQuality, String> {
    let adts = sonare_codec::encode_with_mode(
        sonare_codec::Format::Aac,
        expected_pcm,
        sonare_codec::EncodeMode::ProductionOnly,
    )
    .map_err(|err| format!("{label} ADTS encode failed: {err}"))?;
    let bitrate = sonare_codec::aac_lc_default_production_bitrate_bps(
        u8::try_from(expected_pcm.channels)
            .map_err(|_| format!("{label} channel count exceeds AAC production range"))?,
    )
    .map_err(|err| format!("{label} default bitrate failed: {err}"))?;
    let adts_quality = validate_aac_standard_id_balanced_artifact(
        ffmpeg,
        &format!("{label} ADTS"),
        expected_pcm,
        &adts,
        ProductionArtifactKind::Aac,
        bitrate,
        &out_dir.join(format!("{file_stem}.aac")),
    )?;

    let m4a = sonare_codec::mux_aac_adts_as_m4a(&adts)
        .map_err(|err| format!("{label} M4A mux failed: {err}"))?;
    let m4a_quality = validate_aac_standard_id_balanced_artifact(
        ffmpeg,
        &format!("{label} M4A"),
        expected_pcm,
        &m4a,
        ProductionArtifactKind::M4a,
        bitrate,
        &out_dir.join(format!("{file_stem}.m4a")),
    )?;
    if m4a_quality.best_correlation + f64::EPSILON < adts_quality.best_correlation {
        return Err(format!(
            "{label} M4A quality lagged ADTS: m4a={m4a_quality:?}, adts={adts_quality:?}"
        ));
    }
    eprintln!(
        "{label}: adts_rms={:.4}, adts_correlation={:.3}, m4a_rms={:.4}, m4a_correlation={:.3}",
        adts_quality.decoded_rms,
        adts_quality.best_correlation,
        m4a_quality.decoded_rms,
        m4a_quality.best_correlation
    );
    Ok(adts_quality)
}
