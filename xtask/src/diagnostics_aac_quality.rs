use super::*;

pub(crate) fn validate_aac_standard_id_mixed_workbench() -> Result<(), String> {
    let quantized = [1, -1, 0, 1, 17, 0, 0, 0];
    let band_width = 4;
    let offsets = [0, 4, 8];
    let sections =
        sonare_codec::plan_aac_lc_standard_spectral_sections_by_bit_cost(&quantized, band_width)
            .map_err(|err| format!("AAC standard-id mixed workbench planning failed: {err}"))?;
    let flattened = sections
        .iter()
        .flat_map(|section| [section.start, section.end, usize::from(section.codebook_id)])
        .collect::<Vec<_>>();
    if flattened != [0, 4, 4, 4, 8, 11] {
        return Err(format!(
            "AAC standard-id mixed workbench selected unexpected sections: {flattened:?}"
        ));
    }
    let offset_sections =
        sonare_codec::plan_aac_lc_standard_spectral_sections_by_offsets_by_bit_cost(
            &quantized, &offsets,
        )
        .map_err(|err| format!("AAC standard-id mixed offsets workbench planning failed: {err}"))?;
    let offset_flattened = offset_sections
        .iter()
        .flat_map(|section| [section.start, section.end, usize::from(section.codebook_id)])
        .collect::<Vec<_>>();
    if offset_flattened != flattened {
        return Err(format!(
            "AAC standard-id mixed offsets workbench diverged: offsets={offset_flattened:?}, fixed={flattened:?}"
        ));
    }

    let split = sonare_codec::split_aac_lc_standard_spectral_payload_with_sign_bits_by_bit_cost(
        &quantized, band_width,
    )
    .map_err(|err| format!("AAC standard-id mixed workbench split failed: {err}"))?;
    let packed = sonare_codec::pack_aac_lc_standard_spectral_payload_with_sign_bits_by_bit_cost(
        &quantized, band_width,
    )
    .map_err(|err| format!("AAC standard-id mixed workbench packing failed: {err}"))?;
    let expected_bit_len = split
        .section_and_scale_factor_bits
        .bit_len
        .checked_add(split.spectral_bits.bit_len)
        .ok_or_else(|| "AAC standard-id mixed workbench bit length overflowed".to_owned())?;
    if packed.bit_len != expected_bit_len {
        return Err(format!(
            "AAC standard-id mixed workbench split/packed bit lengths diverged: split={expected_bit_len}, packed={}",
            packed.bit_len
        ));
    }
    if split.spectral_bits.bit_len == 0 {
        return Err("AAC standard-id mixed workbench produced empty spectral bits".to_owned());
    }
    let offset_split =
        sonare_codec::split_aac_lc_standard_spectral_payload_with_offsets_and_sign_bits_by_bit_cost(
            &quantized, &offsets,
        )
        .map_err(|err| format!("AAC standard-id mixed offsets workbench split failed: {err}"))?;
    let offset_packed =
        sonare_codec::pack_aac_lc_standard_spectral_payload_with_offsets_and_sign_bits_by_bit_cost(
            &quantized, &offsets,
        )
        .map_err(|err| format!("AAC standard-id mixed offsets workbench packing failed: {err}"))?;
    if offset_split.section_and_scale_factor_bits.bit_len
        != split.section_and_scale_factor_bits.bit_len
        || offset_split.spectral_bits.bit_len != split.spectral_bits.bit_len
        || offset_packed.bit_len != packed.bit_len
    {
        return Err(format!(
            "AAC standard-id mixed offsets workbench bit lengths diverged: fixed=({}, {}, {}), offsets=({}, {}, {})",
            split.section_and_scale_factor_bits.bit_len,
            split.spectral_bits.bit_len,
            packed.bit_len,
            offset_split.section_and_scale_factor_bits.bit_len,
            offset_split.spectral_bits.bit_len,
            offset_packed.bit_len
        ));
    }
    eprintln!(
        "AAC standard-id mixed workbench: sections={flattened:?}, section_bits={}, spectral_bits={}, packed_bits={}, offsets_section_bits={}",
        split.section_and_scale_factor_bits.bit_len,
        split.spectral_bits.bit_len,
        packed.bit_len,
        offset_split.section_and_scale_factor_bits.bit_len
    );
    Ok(())
}

pub(crate) fn validate_diagnostic_quality_floor(
    label: &str,
    quality: LossyOraclePcmQuality,
    min_decoded_rms: f64,
    min_correlation: f64,
) -> Result<(), String> {
    if quality.decoded_rms < min_decoded_rms {
        return Err(format!(
            "{label} decoded RMS regressed below diagnostic floor: decoded_rms={:.4}, min_decoded_rms={min_decoded_rms:.4}",
            quality.decoded_rms
        ));
    }
    if quality.best_correlation < min_correlation {
        return Err(format!(
            "{label} correlation regressed below diagnostic floor: best_correlation={:.3}, min_correlation={min_correlation:.3}",
            quality.best_correlation
        ));
    }
    Ok(())
}

pub(crate) fn validate_aac_standard_id_production_correlation_gap(
    label: &str,
    standard_id_quality: LossyOraclePcmQuality,
    production_quality: LossyOraclePcmQuality,
) -> Result<(), String> {
    let gap = production_quality.best_correlation - standard_id_quality.best_correlation;
    if gap > AAC_STANDARD_ID_MAX_PRODUCTION_CORRELATION_GAP {
        return Err(format!(
            "{label} correlation gap to production exceeded diagnostic limit: standard_id_correlation={:.3}, production_correlation={:.3}, gap={gap:.3}, max_gap={:.3}",
            standard_id_quality.best_correlation,
            production_quality.best_correlation,
            AAC_STANDARD_ID_MAX_PRODUCTION_CORRELATION_GAP
        ));
    }
    Ok(())
}

pub(crate) fn validate_aac_standard_id_rms_control_advantage(
    label: &str,
    standard_id_quality: LossyOraclePcmQuality,
    production_quality: LossyOraclePcmQuality,
    expected_rms: f64,
) -> Result<(), String> {
    let standard_id_error = rms_error(standard_id_quality, expected_rms);
    let production_error = rms_error(production_quality, expected_rms);
    if standard_id_error > production_error {
        return Err(format!(
            "{label} RMS control regressed behind production: standard_id_rms={:.4}, production_rms={:.4}, expected_rms={expected_rms:.4}, standard_id_error={standard_id_error:.4}, production_error={production_error:.4}",
            standard_id_quality.decoded_rms,
            production_quality.decoded_rms
        ));
    }
    Ok(())
}

pub(crate) fn compare_aac_standard_id_to_production_frame_selection(
    pcm: &sonare_codec::AudioBuffer,
) -> Result<AacFrameSelectionComparison, String> {
    compare_aac_standard_id_candidate_set_to_production_frame_selection(
        pcm,
        sonare_codec::AAC_STANDARD_ID_PCM_STEP_CANDIDATES,
    )
}

pub(crate) fn compare_aac_standard_id_candidate_set_to_production_frame_selection(
    pcm: &sonare_codec::AudioBuffer,
    candidates: &[f32],
) -> Result<AacFrameSelectionComparison, String> {
    let bitrate = sonare_codec::aac_lc_default_production_bitrate_bps(
        u8::try_from(pcm.channels)
            .map_err(|_| "AAC production frame comparison requires mono/stereo PCM".to_owned())?,
    )
    .map_err(|err| format!("AAC default production bitrate lookup failed: {err}"))?;
    let production_details = sonare_codec::aac_selected_scale_factor_frame_details_with_bitrate(
        pcm, bitrate,
    )
    .map_err(|err| format!("AAC production selected-scale-factor frame details failed: {err}"))?;
    let standard_id_details =
        aac_standard_selected_scale_factor_frame_details_with_candidates_and_bitrate(
            pcm, bitrate, candidates,
        )?;

    compare_aac_frame_selection_details(&production_details, &standard_id_details)
}

pub(crate) fn aac_standard_selected_scale_factor_frame_details_with_candidates_and_bitrate(
    pcm: &sonare_codec::AudioBuffer,
    target_bitrate_bps: u32,
    candidates: &[f32],
) -> Result<Vec<sonare_codec::AacPcmFrameStepSelection>, String> {
    let channels = u8::try_from(pcm.channels)
        .map_err(|_| "AAC standard-id frame comparison requires mono/stereo PCM".to_owned())?;
    let adts = sonare_codec::AdtsConfig::aac_lc(pcm.sample_rate, channels);
    let offsets = sonare_codec::aac_lc_long_window_scale_factor_band_offsets(pcm.sample_rate)
        .ok_or_else(|| "AAC standard-id frame comparison requires AAC-LC offsets".to_owned())?;
    let (global_gain, scale_factor_magnitude_bias) =
        sonare_codec::aac_standard_id_selected_scale_factor_parameters(pcm.channels)
            .map_err(|err| format!("AAC standard-id selected parameters failed: {err}"))?;
    let channel_config = sonare_codec::AacLongBlockConfig::new(
        global_gain,
        u8::try_from(offsets.len() - 1)
            .map_err(|_| "AAC scale-factor band count exceeds u8".to_owned())?,
    );
    let scale_factor_table = sonare_codec::aac_scale_factor_delta_table();
    let max_frame_len_bytes =
        sonare_codec::aac_lc_adts_max_frame_len_for_bitrate(pcm.sample_rate, target_bitrate_bps)
            .map_err(|err| format!("AAC bitrate frame budget failed: {err}"))?;

    match pcm.channels {
        1 => sonare_codec::select_aac_lc_mono_pcm_stream_frame_details_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_max_frame_len_by_bit_cost(
            adts,
            channel_config,
            pcm,
            0,
            offsets,
            scale_factor_magnitude_bias,
            candidates,
            max_frame_len_bytes,
            &scale_factor_table,
        )
        .map_err(|err| {
            format!("AAC mono standard-id selected-scale-factor frame details failed: {err}")
        }),
        2 => sonare_codec::select_aac_lc_stereo_pcm_stream_frame_details_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_max_frame_len_by_bit_cost(
            adts,
            channel_config,
            channel_config,
            pcm,
            0,
            offsets,
            scale_factor_magnitude_bias,
            candidates,
            max_frame_len_bytes,
            &scale_factor_table,
        )
        .map_err(|err| {
            format!("AAC stereo standard-id selected-scale-factor frame details failed: {err}")
        }),
        _ => Err("AAC standard-id frame comparison requires mono/stereo PCM".to_owned()),
    }
}

pub(crate) fn aac_standard_id_payload_breakdown_for_frame_selection(
    pcm: &sonare_codec::AudioBuffer,
    details: &[sonare_codec::AacPcmFrameStepSelection],
) -> Result<AacStandardIdPayloadBreakdown, String> {
    let (global_gain, scale_factor_magnitude_bias) =
        sonare_codec::aac_standard_id_selected_scale_factor_parameters(pcm.channels)
            .map_err(|err| format!("AAC standard-id selected parameters failed: {err}"))?;
    sonare_codec::aac_standard_id_payload_breakdown_for_frame_details_with_magnitude_bias(
        pcm,
        details,
        global_gain,
        scale_factor_magnitude_bias,
    )
    .map_err(|err| format!("AAC standard-id payload breakdown failed: {err}"))
}

#[cfg(test)]
pub(crate) fn aac_selected_scale_factor_profile_for_frame_selection(
    pcm: &sonare_codec::AudioBuffer,
    details: &[sonare_codec::AacPcmFrameStepSelection],
    global_gain: u8,
    magnitude_bias: i16,
) -> Result<AacScaleFactorProfile, String> {
    let profile =
        sonare_codec::aac_standard_selected_scale_factor_profile_for_frame_details_with_magnitude_bias(
            pcm,
            details,
            global_gain,
            magnitude_bias,
        )
        .map_err(|err| format!("AAC scale-factor profile failed: {err}"))?;
    Ok(AacScaleFactorProfile {
        frames: profile.frames,
        channels: profile.channels,
        bands: profile.bands,
        raised_bands: profile.raised_bands,
        max_delta: profile.max_delta,
        mean_delta: profile.mean_delta,
    })
}

#[cfg(test)]
pub(crate) fn aac_balanced_profile_selected_candidate(
    channels: u16,
) -> Result<(u8, i16, u32), String> {
    let profile = sonare_codec::aac_standard_id_selected_scale_factor_balance_profile(channels)
        .map_err(|err| format!("AAC balanced profile lookup failed: {err}"))?;
    Ok((
        profile.selected_global_gain,
        profile.selected_magnitude_bias,
        profile.max_quantized_abs,
    ))
}

#[cfg(test)]
pub(crate) fn aac_loudness_recovery_candidates(
    channels: u16,
) -> Result<Vec<(u8, i16, u32)>, String> {
    let mut candidates = vec![aac_balanced_profile_selected_candidate(channels)?];
    match channels {
        1 => candidates.extend_from_slice(&[
            (140, 8, 2047),
            (144, 8, 2047),
            (144, 4, 3071),
            (148, 4, 4095),
            (152, 0, 8191),
        ]),
        2 => candidates.extend_from_slice(&[
            (142, 4, 1535),
            (146, 4, 2047),
            (146, 0, 3071),
            (150, 0, 4095),
            (154, 0, 8191),
        ]),
        _ => return Err("AAC loudness recovery candidates require mono or stereo".to_owned()),
    }
    Ok(candidates)
}

#[cfg(test)]
pub(crate) type AacGainBiasCandidates = (Vec<u8>, Vec<i16>, Vec<u32>);

#[cfg(test)]
pub(crate) fn aac_aggressive_gain_bias_candidates(
    channels: u16,
) -> Result<AacGainBiasCandidates, String> {
    let profile = sonare_codec::aac_standard_id_selected_scale_factor_balance_profile(channels)
        .map_err(|err| format!("AAC balanced profile lookup failed: {err}"))?;
    let mut gain_deltas = vec![profile
        .selected_global_gain
        .saturating_sub(profile.recommended_global_gain)];
    let mut magnitude_biases = vec![profile.selected_magnitude_bias];
    let mut max_quantized_abs = vec![profile.max_quantized_abs];
    match channels {
        1 => {
            gain_deltas.extend_from_slice(&[10, 12]);
            magnitude_biases.push(12);
        }
        2 => {
            gain_deltas.push(12);
            magnitude_biases.push(8);
            max_quantized_abs.push(2047);
        }
        _ => return Err("AAC aggressive gain/bias candidates require mono or stereo".to_owned()),
    }
    Ok((gain_deltas, magnitude_biases, max_quantized_abs))
}

#[cfg(test)]
pub(crate) fn aac_pressure_recovered_scale_factors_for_quantized_bands(
    quantized: &[i32],
    offsets: &[usize],
    base_scale_factor: i16,
    balanced_bias: i16,
    restored_bias: i16,
    restored_bands: usize,
) -> Result<Vec<i16>, String> {
    let balanced =
        sonare_codec::select_scale_factors_for_quantized_bands_by_offsets_with_magnitude_bias(
            quantized,
            offsets,
            base_scale_factor,
            balanced_bias,
        )
        .map_err(|err| format!("AAC balanced scale-factor selection failed: {err}"))?;
    if restored_bands == 0 {
        return Ok(balanced);
    }
    let restored =
        sonare_codec::select_scale_factors_for_quantized_bands_by_offsets_with_magnitude_bias(
            quantized,
            offsets,
            base_scale_factor,
            restored_bias,
        )
        .map_err(|err| format!("AAC restored scale-factor selection failed: {err}"))?;
    let mut ranked_bands = offsets
        .windows(2)
        .enumerate()
        .map(|(index, band)| {
            let max_abs = quantized[band[0]..band[1]]
                .iter()
                .map(|coeff| coeff.checked_abs())
                .collect::<Option<Vec<_>>>()
                .ok_or_else(|| "AAC spectral coefficient overflows".to_owned())?
                .into_iter()
                .max()
                .unwrap_or(0);
            let energy = quantized[band[0]..band[1]]
                .iter()
                .map(|coeff| i64::from(*coeff) * i64::from(*coeff))
                .sum::<i64>();
            Ok((index, max_abs, energy))
        })
        .collect::<Result<Vec<_>, String>>()?;
    ranked_bands.sort_by(|left, right| {
        right
            .1
            .cmp(&left.1)
            .then_with(|| right.2.cmp(&left.2))
            .then_with(|| left.0.cmp(&right.0))
    });

    let mut recovered = balanced;
    for (index, _, _) in ranked_bands.into_iter().take(restored_bands) {
        recovered[index] = restored[index];
    }
    Ok(recovered)
}

#[cfg(test)]
pub(crate) fn aac_pressure_recovered_profile_accumulate(
    profile: &mut AacScaleFactorProfile,
    scale_factors: &[i16],
    base_scale_factor: i16,
) {
    for scale_factor in scale_factors {
        let delta = *scale_factor - base_scale_factor;
        profile.bands += 1;
        profile.raised_bands += usize::from(delta > 0);
        profile.max_delta = profile.max_delta.max(delta);
        profile.mean_delta += f64::from(delta);
    }
}

#[cfg(test)]
pub(crate) fn encode_aac_standard_id_pressure_recovered_stream_for_frame_selection(
    pcm: &sonare_codec::AudioBuffer,
    details: &[sonare_codec::AacPcmFrameStepSelection],
    global_gain: u8,
    balanced_bias: i16,
    candidate: AacScaleFactorPressureRecoveryCandidate,
) -> Result<(Vec<u8>, AacScaleFactorProfile), String> {
    let offsets = sonare_codec::aac_lc_long_window_scale_factor_band_offsets(pcm.sample_rate)
        .ok_or_else(|| "AAC pressure recovery requires AAC-LC offsets".to_owned())?;
    let max_sfb = u8::try_from(offsets.len() - 1)
        .map_err(|_| "AAC scale-factor band count exceeds u8".to_owned())?;
    let adts = sonare_codec::AdtsConfig::aac_lc(
        pcm.sample_rate,
        u8::try_from(pcm.channels).map_err(|_| "AAC channel count exceeds u8".to_owned())?,
    );
    let channel_config = sonare_codec::AacLongBlockConfig::new(global_gain, max_sfb);
    let scale_factor_table = sonare_codec::aac_scale_factor_delta_table();
    let mut out = Vec::new();
    let mut profile = AacScaleFactorProfile {
        frames: details.len(),
        channels: usize::from(pcm.channels),
        bands: 0,
        raised_bands: 0,
        max_delta: 0,
        mean_delta: 0.0,
    };

    for (frame_index, detail) in details.iter().enumerate() {
        let start_frame = frame_index
            .checked_mul(1024)
            .ok_or_else(|| "AAC frame index overflows".to_owned())?;
        match pcm.channels {
            1 => {
                let quantized =
                    sonare_codec::quantize_pcm_long_block(pcm, 0, start_frame, detail.step)
                        .map_err(|err| format!("AAC mono quantization failed: {err}"))?;
                let scale_factors = aac_pressure_recovered_scale_factors_for_quantized_bands(
                    &quantized,
                    offsets,
                    i16::from(global_gain),
                    balanced_bias,
                    candidate.restored_bias,
                    candidate.restored_bands_per_channel,
                )?;
                aac_pressure_recovered_profile_accumulate(
                    &mut profile,
                    &scale_factors,
                    i16::from(global_gain),
                );
                out.extend_from_slice(
                    &sonare_codec::encode_quantized_mono_adts_with_standard_spectral_offsets_and_scale_factors_by_bit_cost(
                        adts,
                        channel_config,
                        &quantized,
                        offsets,
                        &scale_factors,
                        &scale_factor_table,
                    )
                    .map_err(|err| format!("AAC mono pressure recovery encode failed: {err}"))?,
                );
            }
            2 => {
                let left_quantized =
                    sonare_codec::quantize_pcm_long_block(pcm, 0, start_frame, detail.step)
                        .map_err(|err| format!("AAC stereo left quantization failed: {err}"))?;
                let right_quantized =
                    sonare_codec::quantize_pcm_long_block(pcm, 1, start_frame, detail.step)
                        .map_err(|err| format!("AAC stereo right quantization failed: {err}"))?;
                let left_scale_factors = aac_pressure_recovered_scale_factors_for_quantized_bands(
                    &left_quantized,
                    offsets,
                    i16::from(global_gain),
                    balanced_bias,
                    candidate.restored_bias,
                    candidate.restored_bands_per_channel,
                )?;
                let right_scale_factors = aac_pressure_recovered_scale_factors_for_quantized_bands(
                    &right_quantized,
                    offsets,
                    i16::from(global_gain),
                    balanced_bias,
                    candidate.restored_bias,
                    candidate.restored_bands_per_channel,
                )?;
                aac_pressure_recovered_profile_accumulate(
                    &mut profile,
                    &left_scale_factors,
                    i16::from(global_gain),
                );
                aac_pressure_recovered_profile_accumulate(
                    &mut profile,
                    &right_scale_factors,
                    i16::from(global_gain),
                );
                out.extend_from_slice(
                    &sonare_codec::encode_quantized_stereo_adts_with_standard_spectral_offsets_and_scale_factors_by_bit_cost(
                        adts,
                        sonare_codec::AacQuantizedChannel::new(
                            channel_config,
                            &left_quantized,
                            &left_scale_factors,
                        ),
                        sonare_codec::AacQuantizedChannel::new(
                            channel_config,
                            &right_quantized,
                            &right_scale_factors,
                        ),
                        offsets,
                        &scale_factor_table,
                    )
                    .map_err(|err| format!("AAC stereo pressure recovery encode failed: {err}"))?,
                );
            }
            _ => return Err("AAC pressure recovery requires mono/stereo PCM".to_owned()),
        }
    }

    if profile.bands == 0 {
        return Err("AAC pressure recovery profile requires at least one band".to_owned());
    }
    profile.mean_delta /= profile.bands as f64;
    Ok((out, profile))
}

#[cfg(test)]
pub(crate) fn aac_scaled_frame_selection_steps(
    details: &[sonare_codec::AacPcmFrameStepSelection],
    step_scale: f32,
) -> Result<Vec<sonare_codec::AacPcmFrameStepSelection>, String> {
    if !step_scale.is_finite() || step_scale <= 0.0 {
        return Err("AAC step scale must be positive and finite".to_owned());
    }
    details
        .iter()
        .map(|detail| {
            let step = detail.step * step_scale;
            if !step.is_finite() || step <= 0.0 {
                return Err("AAC scaled quantizer step must be positive and finite".to_owned());
            }
            Ok(sonare_codec::AacPcmFrameStepSelection {
                step,
                frame_len: detail.frame_len,
                frame_capacity_bytes: detail.frame_capacity_bytes,
            })
        })
        .collect()
}

#[cfg(test)]
pub(crate) fn aac_max_quantized_abs_for_frame_selection(
    pcm: &sonare_codec::AudioBuffer,
    details: &[sonare_codec::AacPcmFrameStepSelection],
) -> Result<i32, String> {
    let mut max_abs = 0i32;
    for (frame_index, detail) in details.iter().enumerate() {
        let start_frame = frame_index
            .checked_mul(1024)
            .ok_or_else(|| "AAC frame index overflows".to_owned())?;
        for channel in 0..usize::from(pcm.channels) {
            let quantized =
                sonare_codec::quantize_pcm_long_block(pcm, channel, start_frame, detail.step)
                    .map_err(|err| {
                        format!("AAC quantizer step sweep quantization failed: {err}")
                    })?;
            let frame_max_abs = quantized
                .iter()
                .map(|coeff| coeff.checked_abs())
                .collect::<Option<Vec<_>>>()
                .ok_or_else(|| "AAC spectral coefficient overflows".to_owned())?
                .into_iter()
                .max()
                .unwrap_or(0);
            max_abs = max_abs.max(frame_max_abs);
        }
    }
    Ok(max_abs)
}

pub(crate) fn compare_aac_frame_selection_details(
    production_details: &[sonare_codec::AacPcmFrameStepSelection],
    standard_id_details: &[sonare_codec::AacPcmFrameStepSelection],
) -> Result<AacFrameSelectionComparison, String> {
    if production_details.len() != standard_id_details.len() {
        return Err(format!(
            "AAC standard-id frame count diverged from production: production={}, standard_id={}",
            production_details.len(),
            standard_id_details.len()
        ));
    }
    if production_details.is_empty() {
        return Err("AAC frame selection comparison requires at least one frame".to_owned());
    }

    let production_max_frame_len = production_details
        .iter()
        .map(|selection| selection.frame_len)
        .max()
        .unwrap_or(0);
    let standard_id_max_frame_len = standard_id_details
        .iter()
        .map(|selection| selection.frame_len)
        .max()
        .unwrap_or(0);
    let production_min_budget_slack = production_details
        .iter()
        .map(|selection| {
            selection
                .frame_capacity_bytes
                .saturating_sub(selection.frame_len)
        })
        .min()
        .unwrap_or(0);
    let standard_id_min_budget_slack = standard_id_details
        .iter()
        .map(|selection| {
            selection
                .frame_capacity_bytes
                .saturating_sub(selection.frame_len)
        })
        .min()
        .unwrap_or(0);
    let production_max_step = production_details
        .iter()
        .map(|selection| selection.step)
        .fold(0.0_f32, f32::max);
    let standard_id_max_step = standard_id_details
        .iter()
        .map(|selection| selection.step)
        .fold(0.0_f32, f32::max);

    Ok(AacFrameSelectionComparison {
        frames: production_details.len(),
        production_max_frame_len,
        standard_id_max_frame_len,
        max_frame_len_delta: standard_id_max_frame_len as isize - production_max_frame_len as isize,
        production_min_budget_slack,
        standard_id_min_budget_slack,
        min_budget_slack_delta: standard_id_min_budget_slack as isize
            - production_min_budget_slack as isize,
        production_max_step,
        standard_id_max_step,
        max_step_delta: standard_id_max_step - production_max_step,
    })
}

pub(crate) fn validate_mp3_perceptual_reservoir_production_correlation_gap(
    label: &str,
    reservoir_quality: LossyOraclePcmQuality,
    production_quality: LossyOraclePcmQuality,
) -> Result<(), String> {
    let gap = production_quality.best_correlation - reservoir_quality.best_correlation;
    if gap > MP3_PERCEPTUAL_RESERVOIR_MAX_PRODUCTION_CORRELATION_GAP {
        return Err(format!(
            "{label} correlation gap to production exceeded diagnostic limit: reservoir_correlation={:.3}, production_correlation={:.3}, gap={gap:.3}, max_gap={:.3}",
            reservoir_quality.best_correlation,
            production_quality.best_correlation,
            MP3_PERCEPTUAL_RESERVOIR_MAX_PRODUCTION_CORRELATION_GAP
        ));
    }
    Ok(())
}

pub(crate) fn aac_standard_candidate_is_at_least_as_good(
    previous: &AacStandardDiagnosticCandidate,
    candidate: &AacStandardDiagnosticCandidate,
    expected_rms: f64,
) -> bool {
    lossy_oracle_quality_is_at_least_as_good(&previous.quality, &candidate.quality, expected_rms)
}

pub(crate) fn lossy_oracle_quality_is_at_least_as_good(
    previous: &LossyOraclePcmQuality,
    candidate: &LossyOraclePcmQuality,
    expected_rms: f64,
) -> bool {
    let correlation_delta = previous.best_correlation - candidate.best_correlation;
    if correlation_delta.abs() > 1.0e-6 {
        return correlation_delta > 0.0;
    }
    let previous_rms_error = (previous.decoded_rms - expected_rms).abs();
    let candidate_rms_error = (candidate.decoded_rms - expected_rms).abs();
    previous_rms_error <= candidate_rms_error
}

pub(crate) fn rms_error(quality: LossyOraclePcmQuality, expected_rms: f64) -> f64 {
    (quality.decoded_rms - expected_rms).abs()
}

pub(crate) fn aac_step_selection_summary(
    details: &[sonare_codec::AacPcmFrameStepSelection],
) -> String {
    let frames = details.len();
    let min_step = details
        .iter()
        .map(|selection| selection.step)
        .fold(f32::INFINITY, f32::min);
    let max_step = details
        .iter()
        .map(|selection| selection.step)
        .fold(0.0_f32, f32::max);
    let max_frame_len = details
        .iter()
        .map(|selection| selection.frame_len)
        .max()
        .unwrap_or(0);
    let min_budget_slack = details
        .iter()
        .map(|selection| {
            selection
                .frame_capacity_bytes
                .saturating_sub(selection.frame_len)
        })
        .min()
        .unwrap_or(0);
    format!(
        "frames={frames}, min_step={min_step}, max_step={max_step}, max_frame_len={max_frame_len}, min_budget_slack={min_budget_slack}"
    )
}

pub(crate) struct AacStandardDiagnosticCandidate {
    pub(crate) global_gain: u8,
    pub(crate) selected: sonare_codec::AacPcmFrameStepSelection,
    pub(crate) encoded: Vec<u8>,
    pub(crate) quality: LossyOraclePcmQuality,
}

pub(crate) struct AacStandardHighLevelCandidate {
    pub(crate) global_gain: u8,
    pub(crate) max_frame_len: usize,
    pub(crate) quality: LossyOraclePcmQuality,
}

pub(crate) struct AacStandardSelectedHighLevelCandidate {
    pub(crate) global_gain: u8,
    pub(crate) magnitude_bias: i16,
    pub(crate) frame_details: Vec<sonare_codec::AacPcmFrameStepSelection>,
    pub(crate) adts_quality: LossyOraclePcmQuality,
    pub(crate) m4a_quality: LossyOraclePcmQuality,
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn evaluate_aac_standard_diagnostic_candidate(
    ffmpeg: &OsStr,
    expected_pcm: &sonare_codec::AudioBuffer,
    out_dir: &Path,
    offsets: &[usize],
    max_sfb: u8,
    global_gain: u8,
    budget: usize,
    bitrate: u32,
    scale_factor_table: &[sonare_codec::HuffmanEntry<sonare_codec::AacScaleFactorDelta>],
) -> Result<AacStandardDiagnosticCandidate, String> {
    let channel_config = sonare_codec::AacLongBlockConfig::new(global_gain, max_sfb);
    let flat_scale_factors = vec![i16::from(channel_config.global_gain); offsets.len() - 1];
    let channel = sonare_codec::AacScaleFactorChannel::new(channel_config, &flat_scale_factors);
    let selected =
        sonare_codec::select_aac_lc_mono_pcm_frame_step_details_with_offsets_and_scale_factors_and_max_frame_len_by_bit_cost(
            sonare_codec::AdtsConfig::aac_lc(expected_pcm.sample_rate, 1),
            channel,
            expected_pcm,
            0,
            offsets,
            sonare_codec::AAC_LC_PCM_STEP_CANDIDATES,
            budget,
            scale_factor_table,
            sonare_codec::aac_lc_standard_spectral_tables(),
        )
        .map_err(|err| format!("standard-table step selection failed: {err}"))?;
    let encoded = sonare_codec::encode_pcm_mono_long_block_adts_stream_with_offsets_and_scale_factors_and_bitrate_by_bit_cost(
            sonare_codec::AdtsConfig::aac_lc(expected_pcm.sample_rate, 1),
            channel,
            expected_pcm,
            offsets,
            sonare_codec::AAC_LC_PCM_STEP_CANDIDATES,
            bitrate,
            scale_factor_table,
            sonare_codec::aac_lc_standard_spectral_tables(),
        )
    .map_err(|err| format!("standard-table nonzero encode failed: {err}"))?;
    let path = out_dir.join(format!(
        "aaclc-standard-table-nonzero-gain-{global_gain}.aac"
    ));
    fs::write(&path, &encoded)
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
    Ok(AacStandardDiagnosticCandidate {
        global_gain,
        selected,
        encoded,
        quality,
    })
}

pub(crate) fn aac_section_diagnostic_summary(
    label: &str,
    sections: &[sonare_codec::AacSection],
    quantized: &[i32],
) -> String {
    let mut zero_bands = 0usize;
    let mut unsigned7_bands = 0usize;
    let mut unsigned8_bands = 0usize;
    let mut unsigned9_bands = 0usize;
    let mut unsigned10_bands = 0usize;
    let mut escape_bands = 0usize;
    let mut signed_or_other_bands = 0usize;
    let mut max_abs = 0i32;
    let mut max_nonzero_section_width = 0usize;
    for section in sections {
        let width = section.end.saturating_sub(section.start);
        let section_max = quantized
            .get(section.start..section.end)
            .unwrap_or(&[])
            .iter()
            .filter_map(|coeff| coeff.checked_abs())
            .max()
            .unwrap_or(0);
        max_abs = max_abs.max(section_max);
        if section.codebook != sonare_codec::AacCodebook::Zero {
            max_nonzero_section_width = max_nonzero_section_width.max(width);
        }
        match section.codebook {
            sonare_codec::AacCodebook::Zero => zero_bands += 1,
            sonare_codec::AacCodebook::UnsignedPairs7 => unsigned7_bands += 1,
            sonare_codec::AacCodebook::UnsignedPairs8 => unsigned8_bands += 1,
            sonare_codec::AacCodebook::UnsignedPairs9 => unsigned9_bands += 1,
            sonare_codec::AacCodebook::UnsignedPairs10 => unsigned10_bands += 1,
            sonare_codec::AacCodebook::Escape => escape_bands += 1,
            _ => signed_or_other_bands += 1,
        }
    }
    format!(
        "{label}: sections={}, zero={}, unsigned7={}, unsigned8={}, unsigned9={}, unsigned10={}, escape={}, signed_or_other={}, max_abs={}, max_nonzero_width={}",
        sections.len(),
        zero_bands,
        unsigned7_bands,
        unsigned8_bands,
        unsigned9_bands,
        unsigned10_bands,
        escape_bands,
        signed_or_other_bands,
        max_abs,
        max_nonzero_section_width
    )
}

pub(crate) fn aac_spectral_section_diagnostic_summary(
    label: &str,
    sections: &[sonare_codec::AacSpectralSection],
    quantized: &[i32],
    section_bits: usize,
    spectral_bits: usize,
    packed_bits: usize,
) -> String {
    let mut zero_sections = 0usize;
    let mut quad_sections = 0usize;
    let mut signed_pair_sections = 0usize;
    let mut unsigned_pair_sections = 0usize;
    let mut escape_sections = 0usize;
    let mut max_abs = 0i32;
    let mut max_nonzero_section_width = 0usize;
    for section in sections {
        let width = section.end.saturating_sub(section.start);
        let section_max = quantized
            .get(section.start..section.end)
            .unwrap_or(&[])
            .iter()
            .filter_map(|coeff| coeff.checked_abs())
            .max()
            .unwrap_or(0);
        max_abs = max_abs.max(section_max);
        if section.codebook_id != 0 {
            max_nonzero_section_width = max_nonzero_section_width.max(width);
        }
        match section.codebook_id {
            0 => zero_sections += 1,
            1..=4 => quad_sections += 1,
            5 | 6 => signed_pair_sections += 1,
            7..=10 => unsigned_pair_sections += 1,
            11 => escape_sections += 1,
            _ => {}
        }
    }
    format!(
        "{label}: sections={}, zero={}, quad={}, signed_pairs={}, unsigned_pairs={}, escape={}, max_abs={}, max_nonzero_width={}, section_bits={}, spectral_bits={}, packed_bits={}",
        sections.len(),
        zero_sections,
        quad_sections,
        signed_pair_sections,
        unsigned_pair_sections,
        escape_sections,
        max_abs,
        max_nonzero_section_width,
        section_bits,
        spectral_bits,
        packed_bits
    )
}
