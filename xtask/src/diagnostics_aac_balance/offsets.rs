use super::*;

#[allow(clippy::too_many_arguments)]
pub(crate) fn validate_aac_standard_id_offsets_encoded_candidate(
    ffmpeg: &OsStr,
    expected_pcm: &sonare_codec::AudioBuffer,
    out_dir: &Path,
    offsets: &[usize],
    max_sfb: u8,
    candidate: &AacStandardDiagnosticCandidate,
    budget: usize,
    bitrate: u32,
    scale_factor_table: &[sonare_codec::HuffmanEntry<sonare_codec::AacScaleFactorDelta>],
) -> Result<(), String> {
    let channel_config = sonare_codec::AacLongBlockConfig::new(candidate.global_gain, max_sfb);
    let frame_count = expected_pcm
        .samples
        .len()
        .div_ceil(usize::from(expected_pcm.channels) * 1024);
    let scale_factors_by_frame = (0..frame_count)
        .map(|_| vec![i16::from(channel_config.global_gain); offsets.len() - 1])
        .collect::<Vec<_>>();
    let scale_factor_refs = scale_factors_by_frame
        .iter()
        .map(Vec::as_slice)
        .collect::<Vec<_>>();
    let mut selected: Option<(f32, Vec<u8>, usize)> = None;
    let mut last_rejection: Option<String> = None;
    let path = out_dir.join(format!(
        "aaclc-standard-id-offsets-gain-{}.aac",
        candidate.global_gain
    ));
    for &step in sonare_codec::AAC_STANDARD_ID_PCM_STEP_CANDIDATES {
        let encoded = match
            sonare_codec::encode_pcm_mono_long_block_adts_stream_with_standard_spectral_offsets_and_scale_factors_by_bit_cost(
            sonare_codec::AdtsConfig::aac_lc(expected_pcm.sample_rate, 1),
            sonare_codec::AacScaleFactorSequence::new(channel_config, &scale_factor_refs),
            expected_pcm,
            0,
                step,
            offsets,
            scale_factor_table,
        ) {
            Ok(encoded) => encoded,
            Err(err) => {
                last_rejection = Some(format!("step={step}: {err}"));
                continue;
            }
        };
        let max_frame_len = max_adts_frame_len(&encoded)
            .map_err(|err| format!("AAC standard-id offsets ADTS inspection failed: {err}"))?;
        if max_frame_len <= budget {
            fs::write(&path, &encoded)
                .map_err(|err| format!("failed to write {}: {err}", path.display()))?;
            if let Err(err) = run_ffmpeg_clean_acceptance(ffmpeg, &path) {
                last_rejection = Some(format!("step={step}: {err}"));
                continue;
            }
            selected = Some((step, encoded, max_frame_len));
            break;
        }
        last_rejection = Some(format!(
            "step={step}: max_frame_len={max_frame_len} exceeds budget {budget}"
        ));
    }
    let (selected_step, encoded, max_frame_len) = selected.ok_or_else(|| {
        format!(
            "AAC standard-id offsets stream encode diagnostic found no step within budget {budget}: last rejection={}",
            last_rejection.unwrap_or_else(|| "none".to_owned())
        )
    })?;
    fs::write(&path, &encoded)
        .map_err(|err| format!("failed to write {}: {err}", path.display()))?;
    validate_adts_frame_budget(
        "AAC-LC standard-id offsets diagnostic",
        max_frame_len,
        budget,
        bitrate,
    )?;
    eprintln!(
        "AAC-LC standard-id offsets diagnostic ADTS frame budget: selected_step={selected_step}, max_frame_len={max_frame_len}, default_budget={budget}, default_bitrate_bps={bitrate}"
    );

    let expected_rms = rms(&expected_pcm.samples);
    let mut selected_scale_factor_candidate: Option<(
        u8,
        i16,
        Vec<sonare_codec::AacPcmFrameStepSelection>,
        usize,
        LossyOraclePcmQuality,
    )> = None;
    let mut selected_scale_factor_last_rejection: Option<String> = None;
    for &global_gain in AAC_STANDARD_HIGH_LEVEL_SELECTED_SURFACE_GLOBAL_GAIN_CANDIDATES {
        for &scale_factor_magnitude_bias in
            AAC_STANDARD_HIGH_LEVEL_SELECTED_SURFACE_MAGNITUDE_BIAS_CANDIDATES
        {
            let selected_scale_factor_details = match sonare_codec::aac_standard_selected_scale_factor_frame_details_with_magnitude_bias_and_bitrate(
                expected_pcm,
                bitrate,
                global_gain,
                scale_factor_magnitude_bias,
            ) {
                Ok(details) => details,
                Err(err) => {
                    selected_scale_factor_last_rejection =
                        Some(format!("global_gain={global_gain}, scale_factor_magnitude_bias={scale_factor_magnitude_bias}: step selection failed: {err}"));
                    eprintln!(
                        "AAC-LC standard-id selected-scale-factor offsets candidate rejected: global_gain={global_gain}, scale_factor_magnitude_bias={scale_factor_magnitude_bias}, step selection failed: {err}"
                    );
                    continue;
                }
            };
            let selected_scale_factor_encoded = match sonare_codec::encode_aac_adts_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_bitrate(
                expected_pcm,
                bitrate,
                global_gain,
                scale_factor_magnitude_bias,
            ) {
                Ok(encoded) => encoded,
                Err(err) => {
                    selected_scale_factor_last_rejection =
                        Some(format!("global_gain={global_gain}, scale_factor_magnitude_bias={scale_factor_magnitude_bias}: encode failed: {err}"));
                    eprintln!(
                        "AAC-LC standard-id selected-scale-factor offsets candidate rejected: global_gain={global_gain}, scale_factor_magnitude_bias={scale_factor_magnitude_bias}, encode failed: {err}"
                    );
                    continue;
                }
            };
            let selected_scale_factor_path = out_dir.join(format!(
                "aaclc-standard-id-offsets-selected-sf-gain-{global_gain}-bias-{scale_factor_magnitude_bias}.aac"
            ));
            fs::write(&selected_scale_factor_path, &selected_scale_factor_encoded).map_err(
                |err| {
                    format!(
                        "failed to write {}: {err}",
                        selected_scale_factor_path.display()
                    )
                },
            )?;
            if let Err(err) = run_ffmpeg_clean_acceptance(ffmpeg, &selected_scale_factor_path) {
                selected_scale_factor_last_rejection =
                    Some(format!("global_gain={global_gain}, scale_factor_magnitude_bias={scale_factor_magnitude_bias}: {err}"));
                eprintln!(
                    "AAC-LC standard-id selected-scale-factor offsets candidate rejected: global_gain={global_gain}, scale_factor_magnitude_bias={scale_factor_magnitude_bias}, {err}"
                );
                continue;
            }
            let selected_scale_factor_max_frame_len = match max_adts_frame_len(
                &selected_scale_factor_encoded,
            ) {
                Ok(max_frame_len) => max_frame_len,
                Err(err) => {
                    selected_scale_factor_last_rejection = Some(format!(
                    "global_gain={global_gain}, scale_factor_magnitude_bias={scale_factor_magnitude_bias}: ADTS inspection failed: {err}"
                ));
                    eprintln!(
                        "AAC-LC standard-id selected-scale-factor offsets candidate rejected: global_gain={global_gain}, scale_factor_magnitude_bias={scale_factor_magnitude_bias}, ADTS inspection failed: {err}"
                    );
                    continue;
                }
            };
            if let Err(err) = validate_adts_frame_budget(
                "AAC-LC standard-id selected-scale-factor offsets diagnostic",
                selected_scale_factor_max_frame_len,
                budget,
                bitrate,
            ) {
                selected_scale_factor_last_rejection =
                    Some(format!("global_gain={global_gain}, scale_factor_magnitude_bias={scale_factor_magnitude_bias}: {err}"));
                eprintln!(
                    "AAC-LC standard-id selected-scale-factor offsets candidate rejected: global_gain={global_gain}, scale_factor_magnitude_bias={scale_factor_magnitude_bias}, {err}"
                );
                continue;
            }
            let selected_scale_factor_decoded = match run_ffmpeg_decode_f32le(
                ffmpeg,
                &selected_scale_factor_path,
                expected_pcm.sample_rate,
                expected_pcm.channels,
            ) {
                Ok(decoded) => decoded,
                Err(err) => {
                    selected_scale_factor_last_rejection =
                    Some(format!("global_gain={global_gain}, scale_factor_magnitude_bias={scale_factor_magnitude_bias}: decode failed: {err}"));
                    eprintln!(
                    "AAC-LC standard-id selected-scale-factor offsets candidate rejected: global_gain={global_gain}, scale_factor_magnitude_bias={scale_factor_magnitude_bias}, decode failed: {err}"
                );
                    continue;
                }
            };
            let selected_scale_factor_quality = match validate_lossy_oracle_pcm_quality(
                &expected_pcm.samples,
                &selected_scale_factor_decoded,
            ) {
                Ok(quality) => quality,
                Err(err) => {
                    selected_scale_factor_last_rejection =
                    Some(format!("global_gain={global_gain}, scale_factor_magnitude_bias={scale_factor_magnitude_bias}: {err}"));
                    eprintln!(
                    "AAC-LC standard-id selected-scale-factor offsets candidate rejected: global_gain={global_gain}, scale_factor_magnitude_bias={scale_factor_magnitude_bias}, {err}"
                );
                    continue;
                }
            };
            selected_scale_factor_candidate = match selected_scale_factor_candidate {
                Some((
                    previous_gain,
                    previous_bias,
                    previous_details,
                    previous_max_frame_len,
                    previous_quality,
                )) if lossy_oracle_quality_is_at_least_as_good(
                    &previous_quality,
                    &selected_scale_factor_quality,
                    expected_rms,
                ) =>
                {
                    Some((
                        previous_gain,
                        previous_bias,
                        previous_details,
                        previous_max_frame_len,
                        previous_quality,
                    ))
                }
                _ => Some((
                    global_gain,
                    scale_factor_magnitude_bias,
                    selected_scale_factor_details,
                    selected_scale_factor_max_frame_len,
                    selected_scale_factor_quality,
                )),
            };
        }
    }
    let (
        selected_scale_factor_global_gain,
        selected_scale_factor_magnitude_bias,
        selected_scale_factor_details,
        selected_scale_factor_max_frame_len,
        selected_scale_factor_quality,
    ) = selected_scale_factor_candidate.ok_or_else(|| {
        format!(
            "AAC standard-id selected-scale-factor diagnostic found no gain candidate: last rejection={}",
            selected_scale_factor_last_rejection.unwrap_or_else(|| "none".to_owned())
        )
    })?;
    let selected_scale_factor_step_summary = selected_scale_factor_details
        .iter()
        .map(|selection| selection.step.to_string())
        .collect::<Vec<_>>()
        .join(",");
    let selected_scale_factor_selection_summary =
        aac_step_selection_summary(&selected_scale_factor_details);
    eprintln!(
        "AAC-LC standard-id selected-scale-factor offsets diagnostic: global_gain={selected_scale_factor_global_gain}, scale_factor_magnitude_bias={selected_scale_factor_magnitude_bias}, steps=[{selected_scale_factor_step_summary}], {selected_scale_factor_selection_summary}, max_frame_len={selected_scale_factor_max_frame_len}, decoded_rms={:.4}, best_correlation={:.3}",
        selected_scale_factor_quality.decoded_rms,
        selected_scale_factor_quality.best_correlation
    );
    Ok(())
}

pub(crate) fn validate_aac_standard_id_offsets_stereo_encoded_candidate(
    ffmpeg: &OsStr,
    expected_pcm: &sonare_codec::AudioBuffer,
    out_dir: &Path,
    offsets: &[usize],
    max_sfb: u8,
    candidate: &AacStandardDiagnosticCandidate,
    scale_factor_table: &[sonare_codec::HuffmanEntry<sonare_codec::AacScaleFactorDelta>],
) -> Result<(), String> {
    let stereo_pcm = sonare_codec::AudioBuffer::new(
        expected_pcm.sample_rate,
        2,
        expected_pcm
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
    .map_err(|err| format!("AAC standard-id offsets stereo diagnostic PCM failed: {err}"))?;
    let channel_config = sonare_codec::AacLongBlockConfig::new(candidate.global_gain, max_sfb);
    let frame_count = stereo_pcm
        .samples
        .len()
        .div_ceil(usize::from(stereo_pcm.channels) * 1024);
    let scale_factors_by_frame = (0..frame_count)
        .map(|_| vec![i16::from(channel_config.global_gain); offsets.len() - 1])
        .collect::<Vec<_>>();
    let scale_factor_refs = scale_factors_by_frame
        .iter()
        .map(Vec::as_slice)
        .collect::<Vec<_>>();
    let bitrate = sonare_codec::aac_lc_default_production_bitrate_bps(2)
        .map_err(|err| format!("AAC standard-id offsets stereo bitrate failed: {err}"))?;
    let budget =
        sonare_codec::aac_lc_adts_max_frame_len_for_bitrate(stereo_pcm.sample_rate, bitrate)
            .map_err(|err| format!("AAC standard-id offsets stereo budget failed: {err}"))?;
    let mut selected: Option<(f32, Vec<u8>, usize)> = None;
    let mut last_rejection: Option<String> = None;
    let path = out_dir.join(format!(
        "aaclc-standard-id-offsets-stereo-gain-{}.aac",
        candidate.global_gain
    ));
    for &step in sonare_codec::AAC_STANDARD_ID_PCM_STEP_CANDIDATES {
        let encoded = match sonare_codec::encode_pcm_stereo_long_block_adts_stream_with_standard_spectral_offsets_and_scale_factors_by_bit_cost(
            sonare_codec::AdtsConfig::aac_lc(stereo_pcm.sample_rate, 2),
            sonare_codec::AacScaleFactorSequence::new(channel_config, &scale_factor_refs),
            sonare_codec::AacScaleFactorSequence::new(channel_config, &scale_factor_refs),
            &stereo_pcm,
            0,
            step,
            offsets,
            scale_factor_table,
        ) {
            Ok(encoded) => encoded,
            Err(err) => {
                last_rejection = Some(format!("step={step}: {err}"));
                continue;
            }
        };
        let max_frame_len = max_adts_frame_len(&encoded).map_err(|err| {
            format!("AAC standard-id offsets stereo ADTS inspection failed: {err}")
        })?;
        if max_frame_len <= budget {
            fs::write(&path, &encoded)
                .map_err(|err| format!("failed to write {}: {err}", path.display()))?;
            if let Err(err) = run_ffmpeg_clean_acceptance(ffmpeg, &path) {
                last_rejection = Some(format!("step={step}: {err}"));
                continue;
            }
            selected = Some((step, encoded, max_frame_len));
            break;
        }
        last_rejection = Some(format!(
            "step={step}: max_frame_len={max_frame_len} exceeds budget {budget}"
        ));
    }
    let (selected_step, encoded, max_frame_len) = selected.ok_or_else(|| {
        format!(
            "AAC standard-id offsets stereo stream encode diagnostic found no step within budget {budget}: last rejection={}",
            last_rejection.unwrap_or_else(|| "none".to_owned())
        )
    })?;
    fs::write(&path, &encoded)
        .map_err(|err| format!("failed to write {}: {err}", path.display()))?;
    validate_adts_frame_budget(
        "AAC-LC standard-id offsets stereo diagnostic",
        max_frame_len,
        budget,
        bitrate,
    )?;
    eprintln!(
        "AAC-LC standard-id offsets stereo diagnostic ADTS frame budget: selected_step={selected_step}, max_frame_len={max_frame_len}, default_budget={budget}, default_bitrate_bps={bitrate}"
    );

    let expected_rms = rms(&stereo_pcm.samples);
    let mut selected_scale_factor_candidate: Option<(
        u8,
        i16,
        Vec<sonare_codec::AacPcmFrameStepSelection>,
        usize,
        LossyOraclePcmQuality,
    )> = None;
    let mut selected_scale_factor_last_rejection: Option<String> = None;
    for &global_gain in AAC_STANDARD_HIGH_LEVEL_SELECTED_SURFACE_GLOBAL_GAIN_CANDIDATES {
        for &scale_factor_magnitude_bias in
            AAC_STANDARD_HIGH_LEVEL_SELECTED_SURFACE_MAGNITUDE_BIAS_CANDIDATES
        {
            let selected_scale_factor_details = match sonare_codec::aac_standard_selected_scale_factor_frame_details_with_magnitude_bias_and_bitrate(
                &stereo_pcm,
                bitrate,
                global_gain,
                scale_factor_magnitude_bias,
            ) {
                Ok(details) => details,
                Err(err) => {
                    selected_scale_factor_last_rejection =
                        Some(format!("global_gain={global_gain}, scale_factor_magnitude_bias={scale_factor_magnitude_bias}: step selection failed: {err}"));
                    eprintln!(
                        "AAC-LC standard-id selected-scale-factor stereo offsets candidate rejected: global_gain={global_gain}, scale_factor_magnitude_bias={scale_factor_magnitude_bias}, step selection failed: {err}"
                    );
                    continue;
                }
            };
            let selected_scale_factor_encoded = match sonare_codec::encode_aac_adts_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_bitrate(
                &stereo_pcm,
                bitrate,
                global_gain,
                scale_factor_magnitude_bias,
            ) {
                Ok(encoded) => encoded,
                Err(err) => {
                    selected_scale_factor_last_rejection =
                        Some(format!("global_gain={global_gain}, scale_factor_magnitude_bias={scale_factor_magnitude_bias}: encode failed: {err}"));
                    eprintln!(
                        "AAC-LC standard-id selected-scale-factor stereo offsets candidate rejected: global_gain={global_gain}, scale_factor_magnitude_bias={scale_factor_magnitude_bias}, encode failed: {err}"
                    );
                    continue;
                }
            };
            let selected_scale_factor_path = out_dir.join(format!(
                "aaclc-standard-id-offsets-stereo-selected-sf-gain-{global_gain}-bias-{scale_factor_magnitude_bias}.aac"
            ));
            fs::write(&selected_scale_factor_path, &selected_scale_factor_encoded).map_err(
                |err| {
                    format!(
                        "failed to write {}: {err}",
                        selected_scale_factor_path.display()
                    )
                },
            )?;
            if let Err(err) = run_ffmpeg_clean_acceptance(ffmpeg, &selected_scale_factor_path) {
                selected_scale_factor_last_rejection =
                    Some(format!("global_gain={global_gain}, scale_factor_magnitude_bias={scale_factor_magnitude_bias}: {err}"));
                eprintln!(
                    "AAC-LC standard-id selected-scale-factor stereo offsets candidate rejected: global_gain={global_gain}, scale_factor_magnitude_bias={scale_factor_magnitude_bias}, {err}"
                );
                continue;
            }
            let selected_scale_factor_max_frame_len = match max_adts_frame_len(
                &selected_scale_factor_encoded,
            ) {
                Ok(max_frame_len) => max_frame_len,
                Err(err) => {
                    selected_scale_factor_last_rejection = Some(format!(
                            "global_gain={global_gain}, scale_factor_magnitude_bias={scale_factor_magnitude_bias}: ADTS inspection failed: {err}"
                        ));
                    eprintln!(
                            "AAC-LC standard-id selected-scale-factor stereo offsets candidate rejected: global_gain={global_gain}, scale_factor_magnitude_bias={scale_factor_magnitude_bias}, ADTS inspection failed: {err}"
                        );
                    continue;
                }
            };
            if let Err(err) = validate_adts_frame_budget(
                "AAC-LC standard-id selected-scale-factor stereo offsets diagnostic",
                selected_scale_factor_max_frame_len,
                budget,
                bitrate,
            ) {
                selected_scale_factor_last_rejection =
                    Some(format!("global_gain={global_gain}, scale_factor_magnitude_bias={scale_factor_magnitude_bias}: {err}"));
                eprintln!(
                    "AAC-LC standard-id selected-scale-factor stereo offsets candidate rejected: global_gain={global_gain}, scale_factor_magnitude_bias={scale_factor_magnitude_bias}, {err}"
                );
                continue;
            }
            let selected_scale_factor_decoded = match run_ffmpeg_decode_f32le(
                ffmpeg,
                &selected_scale_factor_path,
                stereo_pcm.sample_rate,
                stereo_pcm.channels,
            ) {
                Ok(decoded) => decoded,
                Err(err) => {
                    selected_scale_factor_last_rejection =
                        Some(format!("global_gain={global_gain}, scale_factor_magnitude_bias={scale_factor_magnitude_bias}: decode failed: {err}"));
                    eprintln!(
                        "AAC-LC standard-id selected-scale-factor stereo offsets candidate rejected: global_gain={global_gain}, scale_factor_magnitude_bias={scale_factor_magnitude_bias}, decode failed: {err}"
                    );
                    continue;
                }
            };
            let selected_scale_factor_quality = match validate_lossy_oracle_pcm_quality(
                &stereo_pcm.samples,
                &selected_scale_factor_decoded,
            ) {
                Ok(quality) => quality,
                Err(err) => {
                    selected_scale_factor_last_rejection =
                        Some(format!("global_gain={global_gain}, scale_factor_magnitude_bias={scale_factor_magnitude_bias}: {err}"));
                    eprintln!(
                        "AAC-LC standard-id selected-scale-factor stereo offsets candidate rejected: global_gain={global_gain}, scale_factor_magnitude_bias={scale_factor_magnitude_bias}, {err}"
                    );
                    continue;
                }
            };
            selected_scale_factor_candidate = match selected_scale_factor_candidate {
                Some((
                    previous_gain,
                    previous_bias,
                    previous_details,
                    previous_max_frame_len,
                    previous_quality,
                )) if lossy_oracle_quality_is_at_least_as_good(
                    &previous_quality,
                    &selected_scale_factor_quality,
                    expected_rms,
                ) =>
                {
                    Some((
                        previous_gain,
                        previous_bias,
                        previous_details,
                        previous_max_frame_len,
                        previous_quality,
                    ))
                }
                _ => Some((
                    global_gain,
                    scale_factor_magnitude_bias,
                    selected_scale_factor_details,
                    selected_scale_factor_max_frame_len,
                    selected_scale_factor_quality,
                )),
            };
        }
    }
    let (
        selected_scale_factor_global_gain,
        selected_scale_factor_magnitude_bias,
        selected_scale_factor_details,
        selected_scale_factor_max_frame_len,
        selected_scale_factor_quality,
    ) = selected_scale_factor_candidate.ok_or_else(|| {
        format!(
            "AAC standard-id selected-scale-factor stereo diagnostic found no gain candidate: last rejection={}",
            selected_scale_factor_last_rejection.unwrap_or_else(|| "none".to_owned())
        )
    })?;
    let selected_scale_factor_step_summary = selected_scale_factor_details
        .iter()
        .map(|selection| selection.step.to_string())
        .collect::<Vec<_>>()
        .join(",");
    let selected_scale_factor_selection_summary =
        aac_step_selection_summary(&selected_scale_factor_details);
    eprintln!(
        "AAC-LC standard-id selected-scale-factor stereo offsets diagnostic: global_gain={selected_scale_factor_global_gain}, scale_factor_magnitude_bias={selected_scale_factor_magnitude_bias}, steps=[{selected_scale_factor_step_summary}], {selected_scale_factor_selection_summary}, max_frame_len={selected_scale_factor_max_frame_len}, decoded_rms={:.4}, best_correlation={:.3}",
        selected_scale_factor_quality.decoded_rms,
        selected_scale_factor_quality.best_correlation
    );
    Ok(())
}

pub(crate) fn validate_aac_standard_id_offsets_payload_for_diagnostic(
    quantized: &[i32],
    offsets: &[usize],
) -> Result<(), String> {
    let sections = sonare_codec::plan_aac_lc_standard_spectral_sections_by_offsets_by_bit_cost(
        quantized, offsets,
    )
    .map_err(|err| format!("AAC standard-id offsets diagnostic planning failed: {err}"))?;
    let split =
        sonare_codec::split_aac_lc_standard_spectral_payload_with_offsets_and_sign_bits_by_bit_cost(
            quantized, offsets,
        )
        .map_err(|err| format!("AAC standard-id offsets diagnostic split failed: {err}"))?;
    let packed =
        sonare_codec::pack_aac_lc_standard_spectral_payload_with_offsets_and_sign_bits_by_bit_cost(
            quantized, offsets,
        )
        .map_err(|err| format!("AAC standard-id offsets diagnostic packing failed: {err}"))?;
    let expected_bit_len = split
        .section_and_scale_factor_bits
        .bit_len
        .checked_add(split.spectral_bits.bit_len)
        .ok_or_else(|| "AAC standard-id offsets diagnostic bit length overflowed".to_owned())?;
    if packed.bit_len != expected_bit_len {
        return Err(format!(
            "AAC standard-id offsets diagnostic split/packed bit lengths diverged: split={expected_bit_len}, packed={}",
            packed.bit_len
        ));
    }
    if split.spectral_bits.bit_len == 0 {
        return Err("AAC standard-id offsets diagnostic produced empty spectral bits".to_owned());
    }
    eprintln!(
        "{}",
        aac_spectral_section_diagnostic_summary(
            "AAC-LC standard-id offsets diagnostic sections",
            &sections,
            quantized,
            split.section_and_scale_factor_bits.bit_len,
            split.spectral_bits.bit_len,
            packed.bit_len,
        )
    );
    Ok(())
}
