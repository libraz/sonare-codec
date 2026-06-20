use super::*;

pub(crate) fn validate_aac_standard_id_high_level_selected_bias_surface(
    ffmpeg: &OsStr,
    label: &str,
    expected_pcm: &sonare_codec::AudioBuffer,
    bitrate: u32,
    out_dir: &Path,
    file_stem: &str,
) -> Result<LossyOraclePcmQuality, String> {
    let expected_rms = rms(&expected_pcm.samples);
    let mut selected: Option<AacStandardSelectedHighLevelCandidate> = None;
    let mut last_rejection: Option<String> = None;
    for &global_gain in AAC_STANDARD_HIGH_LEVEL_SELECTED_SURFACE_GLOBAL_GAIN_CANDIDATES {
        for &magnitude_bias in AAC_STANDARD_HIGH_LEVEL_SELECTED_SURFACE_MAGNITUDE_BIAS_CANDIDATES {
            let frame_details = match sonare_codec::aac_standard_selected_scale_factor_frame_details_with_magnitude_bias_and_bitrate(
                expected_pcm,
                bitrate,
                global_gain,
                magnitude_bias,
            ) {
                Ok(details) => details,
                Err(err) => {
                    last_rejection = Some(format!(
                        "global_gain={global_gain}, magnitude_bias={magnitude_bias}: frame detail selection failed: {err}"
                    ));
                    continue;
                }
            };
            let adts = match sonare_codec::encode_aac_adts_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_bitrate(
                expected_pcm,
                bitrate,
                global_gain,
                magnitude_bias,
            ) {
                Ok(adts) => adts,
                Err(err) => {
                    last_rejection = Some(format!(
                        "global_gain={global_gain}, magnitude_bias={magnitude_bias}: ADTS encode failed: {err}"
                    ));
                    continue;
                }
            };
            let adts_quality = match validate_aac_standard_id_high_level_artifact(
                ffmpeg,
                &format!("{label} ADTS gain {global_gain} bias {magnitude_bias}"),
                expected_pcm,
                &adts,
                ProductionArtifactKind::Aac,
                bitrate,
                &out_dir.join(format!(
                    "{file_stem}-gain-{global_gain}-bias-{magnitude_bias}.aac"
                )),
            ) {
                Ok(quality) => quality,
                Err(err) => {
                    last_rejection = Some(format!(
                        "global_gain={global_gain}, magnitude_bias={magnitude_bias}: {err}"
                    ));
                    continue;
                }
            };

            let m4a = match sonare_codec::encode_m4a_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_bitrate(
                expected_pcm,
                bitrate,
                global_gain,
                magnitude_bias,
            ) {
                Ok(m4a) => m4a,
                Err(err) => {
                    last_rejection = Some(format!(
                        "global_gain={global_gain}, magnitude_bias={magnitude_bias}: M4A encode failed: {err}"
                    ));
                    continue;
                }
            };
            let m4a_quality = match validate_aac_standard_id_high_level_artifact(
                ffmpeg,
                &format!("{label} M4A gain {global_gain} bias {magnitude_bias}"),
                expected_pcm,
                &m4a,
                ProductionArtifactKind::M4a,
                bitrate,
                &out_dir.join(format!(
                    "{file_stem}-gain-{global_gain}-bias-{magnitude_bias}.m4a"
                )),
            ) {
                Ok(quality) => quality,
                Err(err) => {
                    last_rejection = Some(format!(
                        "global_gain={global_gain}, magnitude_bias={magnitude_bias}: {err}"
                    ));
                    continue;
                }
            };
            if m4a_quality.best_correlation + f64::EPSILON < adts_quality.best_correlation {
                last_rejection = Some(format!(
                    "global_gain={global_gain}, magnitude_bias={magnitude_bias}: M4A quality lagged ADTS: m4a={m4a_quality:?}, adts={adts_quality:?}"
                ));
                continue;
            }

            let candidate = AacStandardSelectedHighLevelCandidate {
                global_gain,
                magnitude_bias,
                frame_details,
                adts_quality,
                m4a_quality,
            };
            selected = match selected {
                Some(previous)
                    if lossy_oracle_quality_is_at_least_as_good(
                        &previous.adts_quality,
                        &candidate.adts_quality,
                        expected_rms,
                    ) =>
                {
                    Some(previous)
                }
                _ => Some(candidate),
            };
        }
    }

    let selected = selected.ok_or_else(|| {
        format!(
            "{label} found no selected-scale-factor candidate: last rejection={}",
            last_rejection.unwrap_or_else(|| "none".to_owned())
        )
    })?;
    let step_summary = selected
        .frame_details
        .iter()
        .map(|selection| selection.step.to_string())
        .collect::<Vec<_>>()
        .join(",");
    let selection_summary = aac_step_selection_summary(&selected.frame_details);
    eprintln!(
        "{label}: global_gain={}, scale_factor_magnitude_bias={}, steps=[{}], {}, adts_rms={:.4}, adts_correlation={:.3}, m4a_rms={:.4}, m4a_correlation={:.3}",
        selected.global_gain,
        selected.magnitude_bias,
        step_summary,
        selection_summary,
        selected.adts_quality.decoded_rms,
        selected.adts_quality.best_correlation,
        selected.m4a_quality.decoded_rms,
        selected.m4a_quality.best_correlation
    );
    Ok(selected.adts_quality)
}

pub(crate) struct AacStandardIdBalancedSurfaceCheck<'a> {
    pub(crate) ffmpeg: &'a OsStr,
    pub(crate) label: &'a str,
    pub(crate) expected_pcm: &'a sonare_codec::AudioBuffer,
    pub(crate) bitrate: u32,
    pub(crate) baseline_quality: LossyOraclePcmQuality,
    pub(crate) min_correlation: f64,
    pub(crate) out_dir: &'a Path,
    pub(crate) file_stem: &'a str,
}

pub(crate) fn validate_aac_standard_id_balanced_surface(
    check: AacStandardIdBalancedSurfaceCheck<'_>,
) -> Result<(LossyOraclePcmQuality, AacStandardIdPayloadBreakdown), String> {
    let AacStandardIdBalancedSurfaceCheck {
        ffmpeg,
        label,
        expected_pcm,
        bitrate,
        baseline_quality,
        min_correlation,
        out_dir,
        file_stem,
    } = check;
    let max_quantized_abs =
        sonare_codec::aac_standard_id_selected_scale_factor_balanced_max_quantized_abs(
            expected_pcm.channels,
        )
        .map_err(|err| format!("{label} balanced max_abs lookup failed: {err}"))?;
    let (balanced_global_gain, balanced_magnitude_bias, balanced_max_quantized_abs) =
        sonare_codec::aac_standard_id_selected_scale_factor_balanced_parameters(
            expected_pcm.channels,
        )
        .map_err(|err| format!("{label} balanced parameter lookup failed: {err}"))?;
    if balanced_max_quantized_abs != max_quantized_abs {
        return Err(format!(
            "{label} balanced parameter max_abs={balanced_max_quantized_abs} diverged from max_abs helper={max_quantized_abs}"
        ));
    }
    let baseline_details =
        sonare_codec::aac_recommended_standard_selected_scale_factor_frame_details_with_bitrate(
            expected_pcm,
            bitrate,
        )
        .map_err(|err| format!("{label} baseline frame details failed: {err}"))?;
    let balanced_details =
        sonare_codec::aac_balanced_standard_selected_scale_factor_frame_details_with_bitrate(
            expected_pcm,
            bitrate,
        )
        .map_err(|err| format!("{label} balanced frame details failed: {err}"))?;
    let expected_balanced_details =
        sonare_codec::aac_standard_selected_scale_factor_frame_details_with_magnitude_bias_max_quantized_abs_and_bitrate(
            expected_pcm,
            bitrate,
            balanced_global_gain,
            balanced_magnitude_bias,
            max_quantized_abs,
        )
        .map_err(|err| format!("{label} expected balanced frame details failed: {err}"))?;
    if balanced_details != expected_balanced_details {
        return Err(format!(
            "{label} balanced details diverged from gain={balanced_global_gain}, bias={balanced_magnitude_bias}, max_abs={max_quantized_abs}"
        ));
    }

    let baseline_breakdown =
        aac_standard_id_payload_breakdown_for_frame_selection(expected_pcm, &baseline_details)?;
    let balanced_breakdown =
        aac_standard_id_payload_breakdown_for_frame_selection(expected_pcm, &balanced_details)?;
    let balanced_quality_control_profile =
        sonare_codec::aac_balanced_standard_id_quality_control_profile_for_frame_details(
            expected_pcm,
            &balanced_details,
        )
        .map_err(|err| format!("{label} balanced quality-control profile failed: {err}"))?;
    if balanced_quality_control_profile.max_abs != balanced_breakdown.max_abs
        || balanced_quality_control_profile.escape_spectral_bits
            != balanced_breakdown.escape_spectral_bits
        || balanced_quality_control_profile.min_frame_budget_slack < 0
        || balanced_quality_control_profile.max_abs
            > i32::try_from(balanced_quality_control_profile.max_quantized_abs_limit)
                .unwrap_or(i32::MAX)
    {
        return Err(format!(
            "{label} balanced quality-control profile diverged from payload/frame constraints: profile={balanced_quality_control_profile:?}, breakdown={balanced_breakdown:?}"
        ));
    }
    if balanced_breakdown.max_abs > i32::try_from(max_quantized_abs).unwrap_or(i32::MAX) {
        return Err(format!(
            "{label} balanced max_abs exceeded limit {max_quantized_abs}: {balanced_breakdown:?}"
        ));
    }
    if balanced_breakdown.max_abs >= baseline_breakdown.max_abs
        || balanced_breakdown.escape_spectral_bits >= baseline_breakdown.escape_spectral_bits
    {
        return Err(format!(
            "{label} balanced path did not reduce escape pressure: baseline={baseline_breakdown:?}, balanced={balanced_breakdown:?}"
        ));
    }

    let balanced_adts =
        sonare_codec::encode_aac_adts_with_balanced_standard_spectral_offsets_and_selected_scale_factors_and_bitrate(
            expected_pcm,
            bitrate,
        )
        .map_err(|err| format!("{label} balanced ADTS encode failed: {err}"))?;
    let expected_balanced_adts =
        sonare_codec::encode_aac_adts_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_max_quantized_abs_and_bitrate(
            expected_pcm,
            bitrate,
            balanced_global_gain,
            balanced_magnitude_bias,
            max_quantized_abs,
        )
        .map_err(|err| format!("{label} expected balanced ADTS encode failed: {err}"))?;
    if balanced_adts != expected_balanced_adts {
        return Err(format!(
            "{label} balanced ADTS diverged from gain={balanced_global_gain}, bias={balanced_magnitude_bias}, max_abs={max_quantized_abs}"
        ));
    }
    let adts_quality = validate_aac_standard_id_balanced_artifact(
        ffmpeg,
        &format!("{label} ADTS"),
        expected_pcm,
        &balanced_adts,
        ProductionArtifactKind::Aac,
        bitrate,
        &out_dir.join(format!("{file_stem}.aac")),
    )?;
    if adts_quality.best_correlation < min_correlation
        || adts_quality.best_correlation + 0.10 < baseline_quality.best_correlation
        || adts_quality.decoded_rms < baseline_quality.decoded_rms * 0.35
    {
        return Err(format!(
            "{label} balanced quality failed guard: balanced={adts_quality:?}, baseline={baseline_quality:?}"
        ));
    }

    let balanced_m4a =
        sonare_codec::encode_m4a_with_balanced_standard_spectral_offsets_and_selected_scale_factors_and_bitrate(
            expected_pcm,
            bitrate,
        )
        .map_err(|err| format!("{label} balanced M4A encode failed: {err}"))?;
    let demuxed = sonare_codec::demux_m4a_as_aac_adts(&balanced_m4a)
        .map_err(|err| format!("{label} balanced M4A demux failed: {err}"))?;
    if demuxed != balanced_adts {
        return Err(format!(
            "{label} balanced M4A did not mux the expected ADTS"
        ));
    }
    let m4a_quality = validate_aac_standard_id_balanced_artifact(
        ffmpeg,
        &format!("{label} M4A"),
        expected_pcm,
        &balanced_m4a,
        ProductionArtifactKind::M4a,
        bitrate,
        &out_dir.join(format!("{file_stem}.m4a")),
    )?;
    if m4a_quality.best_correlation + f64::EPSILON < adts_quality.best_correlation {
        return Err(format!(
            "{label} balanced M4A quality lagged ADTS: m4a={m4a_quality:?}, adts={adts_quality:?}"
        ));
    }

    eprintln!(
        "{label}: max_abs_limit={max_quantized_abs}, decoded_rms={:.4}, best_correlation={:.3}, baseline_escape_bits={}, balanced_escape_bits={}, baseline_max_abs={}, balanced_max_abs={}",
        adts_quality.decoded_rms,
        adts_quality.best_correlation,
        baseline_breakdown.escape_spectral_bits,
        balanced_breakdown.escape_spectral_bits,
        baseline_breakdown.max_abs,
        balanced_breakdown.max_abs
    );
    Ok((adts_quality, balanced_breakdown))
}

pub(crate) fn select_aac_standard_id_high_level_gain_candidate(
    ffmpeg: &OsStr,
    label: &str,
    expected_pcm: &sonare_codec::AudioBuffer,
    kind: ProductionArtifactKind,
    bitrate: u32,
    out_dir: &Path,
    file_stem: &str,
) -> Result<AacStandardHighLevelCandidate, String> {
    let expected_rms = rms(&expected_pcm.samples);
    let mut selected: Option<AacStandardHighLevelCandidate> = None;
    let mut last_rejection: Option<String> = None;
    for &global_gain in AAC_STANDARD_HIGH_LEVEL_FIXED_SURFACE_GLOBAL_GAIN_CANDIDATES {
        let bytes = match kind {
            ProductionArtifactKind::Aac => {
                match sonare_codec::encode_aac_adts_with_standard_spectral_offsets_and_bitrate(
                    expected_pcm,
                    bitrate,
                    global_gain,
                ) {
                    Ok(bytes) => bytes,
                    Err(err) => {
                        last_rejection = Some(format!(
                            "{label} global_gain={global_gain} encode failed: {err}"
                        ));
                        eprintln!("{label} candidate rejected: global_gain={global_gain}, {err}");
                        continue;
                    }
                }
            }
            ProductionArtifactKind::M4a => {
                match sonare_codec::encode_m4a_with_standard_spectral_offsets_and_bitrate(
                    expected_pcm,
                    bitrate,
                    global_gain,
                ) {
                    Ok(bytes) => bytes,
                    Err(err) => {
                        last_rejection = Some(format!(
                            "{label} global_gain={global_gain} M4A encode failed: {err}"
                        ));
                        eprintln!("{label} candidate rejected: global_gain={global_gain}, {err}");
                        continue;
                    }
                }
            }
            ProductionArtifactKind::Mp3 => {
                return Err(format!("{label} gain sweep received MP3 artifact kind"));
            }
        };
        let extension = match kind {
            ProductionArtifactKind::Aac => "aac",
            ProductionArtifactKind::M4a => "m4a",
            ProductionArtifactKind::Mp3 => unreachable!(),
        };
        let path = out_dir.join(format!("{file_stem}-gain-{global_gain}.{extension}"));
        let quality = match validate_aac_standard_id_high_level_artifact(
            ffmpeg,
            &format!("{label} gain {global_gain}"),
            expected_pcm,
            &bytes,
            kind,
            bitrate,
            &path,
        ) {
            Ok(quality) => quality,
            Err(err) => {
                last_rejection = Some(err.clone());
                eprintln!("{label} candidate rejected: global_gain={global_gain}, {err}");
                continue;
            }
        };
        let adts = match kind {
            ProductionArtifactKind::Aac => bytes,
            ProductionArtifactKind::M4a => match sonare_codec::demux_m4a_as_aac_adts(&bytes) {
                Ok(adts) => adts,
                Err(err) => {
                    last_rejection = Some(format!(
                        "{label} global_gain={global_gain} demux failed: {err}"
                    ));
                    eprintln!(
                        "{label} candidate rejected: global_gain={global_gain}, demux failed: {err}"
                    );
                    continue;
                }
            },
            ProductionArtifactKind::Mp3 => unreachable!(),
        };
        let max_frame_len = match max_adts_frame_len(&adts) {
            Ok(max_frame_len) => max_frame_len,
            Err(err) => {
                last_rejection = Some(format!(
                    "{label} global_gain={global_gain} ADTS inspect failed: {err}"
                ));
                eprintln!(
                    "{label} candidate rejected: global_gain={global_gain}, ADTS inspect failed: {err}"
                );
                continue;
            }
        };
        let candidate = AacStandardHighLevelCandidate {
            global_gain,
            max_frame_len,
            quality,
        };
        selected = match selected {
            Some(previous)
                if lossy_oracle_quality_is_at_least_as_good(
                    &previous.quality,
                    &candidate.quality,
                    expected_rms,
                ) =>
            {
                Some(previous)
            }
            _ => Some(candidate),
        };
    }
    selected.ok_or_else(|| {
        format!(
            "{label} found no global_gain candidate: last rejection={}",
            last_rejection.unwrap_or_else(|| "none".to_owned())
        )
    })
}

pub(crate) fn validate_aac_standard_id_balanced_artifact(
    ffmpeg: &OsStr,
    label: &str,
    expected_pcm: &sonare_codec::AudioBuffer,
    bytes: &[u8],
    kind: ProductionArtifactKind,
    bitrate: u32,
    path: &Path,
) -> Result<LossyOraclePcmQuality, String> {
    let adts = match kind {
        ProductionArtifactKind::Mp3 => {
            return Err(format!(
                "{label} balanced AAC surface received MP3 artifact kind"
            ));
        }
        ProductionArtifactKind::Aac => bytes.to_vec(),
        ProductionArtifactKind::M4a => sonare_codec::demux_m4a_as_aac_adts(bytes)
            .map_err(|err| format!("{label} demux failed: {err}"))?,
    };
    let budget =
        sonare_codec::aac_lc_adts_max_frame_len_for_bitrate(expected_pcm.sample_rate, bitrate)
            .map_err(|err| format!("{label} bitrate budget failed: {err}"))?;
    let max_frame_len = max_adts_frame_len(&adts)
        .map_err(|err| format!("{label} ADTS inspection failed: {err}"))?;
    validate_adts_frame_budget(label, max_frame_len, budget, bitrate)?;

    fs::write(path, bytes).map_err(|err| format!("failed to write {}: {err}", path.display()))?;
    run_ffmpeg_acceptance(ffmpeg, path)
        .map_err(|err| format!("{label} FFmpeg acceptance failed: {err}"))?;
    let decoded = run_ffmpeg_decode_f32le(
        ffmpeg,
        path,
        expected_pcm.sample_rate,
        expected_pcm.channels,
    )
    .map_err(|err| format!("{label} FFmpeg PCM decode failed: {err}"))?;
    let quality = validate_lossy_oracle_pcm_quality(&expected_pcm.samples, &decoded)
        .map_err(|err| format!("{label} PCM quality failed: {err}"))?;
    eprintln!(
        "{label}: max_frame_len={max_frame_len}, default_budget={budget}, decoded_rms={:.4}, best_correlation={:.3}",
        quality.decoded_rms, quality.best_correlation
    );
    Ok(quality)
}

pub(crate) fn validate_aac_standard_id_high_level_artifact(
    ffmpeg: &OsStr,
    label: &str,
    expected_pcm: &sonare_codec::AudioBuffer,
    bytes: &[u8],
    kind: ProductionArtifactKind,
    bitrate: u32,
    path: &Path,
) -> Result<LossyOraclePcmQuality, String> {
    let adts = match kind {
        ProductionArtifactKind::Mp3 => {
            return Err(format!(
                "{label} high-level AAC surface received MP3 artifact kind"
            ));
        }
        ProductionArtifactKind::Aac => bytes.to_vec(),
        ProductionArtifactKind::M4a => sonare_codec::demux_m4a_as_aac_adts(bytes)
            .map_err(|err| format!("{label} demux failed: {err}"))?,
    };
    let budget =
        sonare_codec::aac_lc_adts_max_frame_len_for_bitrate(expected_pcm.sample_rate, bitrate)
            .map_err(|err| format!("{label} bitrate budget failed: {err}"))?;
    let max_frame_len = max_adts_frame_len(&adts)
        .map_err(|err| format!("{label} ADTS inspection failed: {err}"))?;
    validate_adts_frame_budget(label, max_frame_len, budget, bitrate)?;

    fs::write(path, bytes).map_err(|err| format!("failed to write {}: {err}", path.display()))?;
    run_ffmpeg_acceptance(ffmpeg, path)
        .map_err(|err| format!("{label} FFmpeg acceptance failed: {err}"))?;
    let decoded = run_ffmpeg_decode_f32le(
        ffmpeg,
        path,
        expected_pcm.sample_rate,
        expected_pcm.channels,
    )
    .map_err(|err| format!("{label} FFmpeg PCM decode failed: {err}"))?;
    let quality = validate_lossy_oracle_pcm_quality(&expected_pcm.samples, &decoded)
        .map_err(|err| format!("{label} PCM quality failed: {err}"))?;
    validate_diagnostic_quality_floor(
        label,
        quality,
        AAC_STANDARD_DIAGNOSTIC_MIN_DECODED_RMS,
        AAC_STANDARD_DIAGNOSTIC_MIN_CORRELATION,
    )?;
    eprintln!(
        "{label}: max_frame_len={max_frame_len}, default_budget={budget}, decoded_rms={:.4}, best_correlation={:.3}",
        quality.decoded_rms, quality.best_correlation
    );
    Ok(quality)
}
