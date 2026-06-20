use super::*;

#[cfg(feature = "aac")]
pub(crate) fn aac_standard_id_section_codebook_costs(
    quantized: &[i32],
) -> Result<Vec<(u8, usize)>, Error> {
    if quantized.iter().all(|coeff| *coeff == 0) {
        return Ok(vec![(0, 0)]);
    }

    let mut costs = Vec::new();
    if quantized.len() % 4 == 0 {
        let quads = aac_spectral_quads_for_i32_slice(quantized)?;
        for (codebook_id, table) in [
            (1, aac_signed_quads1_table()),
            (2, aac_signed_quads2_table()),
        ] {
            if let Ok(packed) = pack_spectral_quads_with_table(&quads, table) {
                costs.push((codebook_id, packed.bit_len));
            }
        }
        for (codebook_id, table) in [
            (3, aac_unsigned_quads3_table()),
            (4, aac_unsigned_quads4_table()),
        ] {
            if let Ok(packed) = pack_spectral_quads_with_sign_bits(&quads, table) {
                costs.push((codebook_id, packed.bit_len));
            }
        }
    }

    if quantized.len() % 2 == 0 {
        let pairs = aac_spectral_pairs_for_i32_slice(quantized)?;
        for (codebook_id, table) in [
            (5, aac_signed_pairs5_table()),
            (6, aac_signed_pairs6_table()),
        ] {
            if let Ok(packed) = pack_spectral_pairs_with_table(&pairs, table) {
                costs.push((codebook_id, packed.bit_len));
            }
        }
        for (codebook_id, table) in [
            (7, aac_unsigned_pairs7_table()),
            (8, aac_unsigned_pairs8_table()),
            (9, aac_unsigned_pairs9_table()),
            (10, aac_unsigned_pairs10_table()),
            (11, aac_escape_table()),
        ] {
            if let Ok(packed) = pack_spectral_pairs_with_sign_bits(&pairs, table) {
                costs.push((codebook_id, packed.bit_len));
            }
        }
    }

    if costs.is_empty() {
        return Err(Error::UnsupportedFeature(
            "AAC section has no packable standard-id codebook candidates",
        ));
    }
    costs.sort_by_key(|(codebook_id, bit_len)| (*bit_len, *codebook_id));
    costs.dedup_by_key(|(codebook_id, _)| *codebook_id);
    Ok(costs)
}

#[cfg(feature = "aac")]
pub(crate) fn max_abs_i32(values: &[i32]) -> Result<i32, Error> {
    values
        .iter()
        .map(|value| {
            value
                .checked_abs()
                .ok_or(Error::InvalidInput("AAC spectral coefficient overflows"))
        })
        .try_fold(0, |acc, value| value.map(|value| acc.max(value)))
}

#[cfg(feature = "aac")]
pub fn aac_standard_id_payload_breakdown_for_frame_details_with_magnitude_bias(
    pcm: &AudioBuffer,
    details: &[AacPcmFrameStepSelection],
    global_gain: u8,
    scale_factor_magnitude_bias: i16,
) -> Result<AacStandardIdPayloadBreakdown, Error> {
    let offsets = aac_lc_long_window_scale_factor_band_offsets(pcm.sample_rate)
        .ok_or(Error::UnsupportedFeature("AAC-LC sample rate"))?;
    let scale_factor_table = aac_scale_factor_delta_table();

    let mut sections = 0usize;
    let mut escape_sections = 0usize;
    let mut max_abs = 0i32;
    let mut section_bits = 0usize;
    let mut scale_factor_bits = 0usize;
    let mut spectral_bits = 0usize;
    let mut escape_spectral_bits = 0usize;
    let mut dominant_spectral_section = None;
    let mut dominant_escape_section = None;

    for (frame_index, detail) in details.iter().enumerate() {
        let start_frame = frame_index
            .checked_mul(1024)
            .ok_or(Error::InvalidInput("AAC frame index overflows"))?;
        for channel in 0..usize::from(pcm.channels) {
            let quantized = quantize_pcm_long_block(pcm, channel, start_frame, detail.step)?;
            let planned_sections =
                plan_aac_lc_standard_spectral_sections_by_offsets_by_bit_cost(&quantized, offsets)?;
            let scale_factors =
                select_scale_factors_for_quantized_bands_by_offsets_with_magnitude_bias(
                    &quantized,
                    offsets,
                    i16::from(global_gain),
                    scale_factor_magnitude_bias,
                )?;
            let scale_factor_deltas = plan_spectral_scale_factor_deltas_by_offsets(
                &planned_sections,
                offsets,
                &scale_factors,
                i16::from(global_gain),
            )?;
            let packed_scale_factors =
                pack_scale_factor_deltas_with_table(&scale_factor_deltas, &scale_factor_table)?;
            let split_without_scale_factors =
                split_aac_lc_standard_spectral_payload_with_offsets_and_sign_bits_by_bit_cost(
                    &quantized, offsets,
                )?;
            let split_with_scale_factors =
                split_aac_lc_standard_spectral_payload_with_offsets_and_sign_bits_and_scale_factor_bits_by_bit_cost(
                    &quantized,
                    offsets,
                    packed_scale_factors,
                )?;

            if split_without_scale_factors.spectral_bits.bit_len
                != split_with_scale_factors.spectral_bits.bit_len
            {
                return Err(Error::InvalidInput(
                    "AAC standard-id payload split changed spectral bits when adding scale factors",
                ));
            }

            sections += planned_sections.len();
            escape_sections += planned_sections
                .iter()
                .filter(|section| section.codebook_id == 11)
                .count();
            for section in &planned_sections {
                let section_payload =
                    split_aac_lc_standard_sectioned_spectral_payload_with_offsets_and_sign_bits(
                        std::slice::from_ref(section),
                        &quantized,
                        offsets,
                    )?;
                let section_spectral_bits = section_payload.spectral_bits.bit_len;
                let section_max_abs = max_abs_i32(&quantized[section.start..section.end])?;
                let section_codebook_costs =
                    aac_standard_id_section_codebook_costs(&quantized[section.start..section.end])?;
                let best_alternative = section_codebook_costs
                    .iter()
                    .copied()
                    .find(|(codebook_id, _)| *codebook_id != section.codebook_id);
                let section_breakdown = AacStandardIdSpectralSectionBreakdown {
                    frame_index,
                    channel,
                    start_band: aac_scale_factor_band_index(offsets, section.start)?,
                    end_band: aac_scale_factor_band_index(offsets, section.end)?,
                    start: section.start,
                    end: section.end,
                    codebook_id: section.codebook_id,
                    max_abs: section_max_abs,
                    spectral_bits: section_spectral_bits,
                    best_alternative_codebook_id: best_alternative
                        .map(|(codebook_id, _)| codebook_id),
                    best_alternative_spectral_bits: best_alternative.map(|(_, bit_len)| bit_len),
                };
                if section.codebook_id == 11 {
                    escape_spectral_bits += section_spectral_bits;
                    if dominant_escape_section.is_none_or(
                        |dominant: AacStandardIdSpectralSectionBreakdown| {
                            section_breakdown.spectral_bits > dominant.spectral_bits
                        },
                    ) {
                        dominant_escape_section = Some(section_breakdown);
                    }
                }
                if dominant_spectral_section.is_none_or(
                    |dominant: AacStandardIdSpectralSectionBreakdown| {
                        section_breakdown.spectral_bits > dominant.spectral_bits
                    },
                ) {
                    dominant_spectral_section = Some(section_breakdown);
                }
            }

            max_abs = max_abs.max(max_abs_i32(&quantized)?);
            section_bits += split_without_scale_factors
                .section_and_scale_factor_bits
                .bit_len;
            scale_factor_bits += split_with_scale_factors
                .section_and_scale_factor_bits
                .bit_len
                .checked_sub(
                    split_without_scale_factors
                        .section_and_scale_factor_bits
                        .bit_len,
                )
                .ok_or(Error::InvalidInput(
                    "AAC scale-factor bit count underflowed",
                ))?;
            spectral_bits += split_with_scale_factors.spectral_bits.bit_len;
        }
    }

    Ok(AacStandardIdPayloadBreakdown {
        frames: details.len(),
        channels: usize::from(pcm.channels),
        sections,
        escape_sections,
        max_abs,
        section_bits,
        scale_factor_bits,
        spectral_bits,
        escape_spectral_bits,
        dominant_spectral_section,
        dominant_escape_section,
    })
}

#[cfg(feature = "aac")]
pub fn aac_recommended_standard_id_payload_breakdown_for_frame_details(
    pcm: &AudioBuffer,
    details: &[AacPcmFrameStepSelection],
) -> Result<AacStandardIdPayloadBreakdown, Error> {
    let (global_gain, scale_factor_magnitude_bias) =
        aac_standard_id_selected_scale_factor_parameters(pcm.channels)?;
    aac_standard_id_payload_breakdown_for_frame_details_with_magnitude_bias(
        pcm,
        details,
        global_gain,
        scale_factor_magnitude_bias,
    )
}

#[cfg(feature = "aac")]
pub fn aac_balanced_standard_id_payload_breakdown_for_frame_details(
    pcm: &AudioBuffer,
    details: &[AacPcmFrameStepSelection],
) -> Result<AacStandardIdPayloadBreakdown, Error> {
    let profile = aac_standard_id_selected_scale_factor_balance_profile(pcm.channels)?;
    aac_standard_id_payload_breakdown_for_frame_details_with_magnitude_bias(
        pcm,
        details,
        profile.selected_global_gain,
        profile.selected_magnitude_bias,
    )
}

#[cfg(feature = "aac")]
pub fn aac_standard_id_quality_control_profile_for_frame_details_with_magnitude_bias_max_quantized_abs(
    pcm: &AudioBuffer,
    details: &[AacPcmFrameStepSelection],
    global_gain: u8,
    scale_factor_magnitude_bias: i16,
    max_quantized_abs: u32,
) -> Result<AacStandardIdQualityControlProfile, Error> {
    let breakdown = aac_standard_id_payload_breakdown_for_frame_details_with_magnitude_bias(
        pcm,
        details,
        global_gain,
        scale_factor_magnitude_bias,
    )?;
    let scale_factor_profile =
        aac_standard_selected_scale_factor_profile_for_frame_details_with_magnitude_bias(
            pcm,
            details,
            global_gain,
            scale_factor_magnitude_bias,
        )?;
    let max_frame_len = details
        .iter()
        .map(|detail| detail.frame_len)
        .max()
        .unwrap_or(0);
    let min_frame_budget_slack = details
        .iter()
        .map(|detail| detail.frame_capacity_bytes as isize - detail.frame_len as isize)
        .min()
        .unwrap_or(0);

    Ok(AacStandardIdQualityControlProfile {
        frames: details.len(),
        channels: usize::from(pcm.channels),
        max_frame_len,
        min_frame_budget_slack,
        max_quantized_abs_limit: max_quantized_abs,
        max_abs: breakdown.max_abs,
        sections: breakdown.sections,
        escape_sections: breakdown.escape_sections,
        total_bits: breakdown.total_bits(),
        spectral_bits: breakdown.spectral_bits,
        escape_spectral_bits: breakdown.escape_spectral_bits,
        scale_factor_bits: breakdown.scale_factor_bits,
        scale_factor_bands: scale_factor_profile.bands,
        raised_scale_factor_bands: scale_factor_profile.raised_bands,
        max_scale_factor_delta: scale_factor_profile.max_delta,
        mean_scale_factor_delta: scale_factor_profile.mean_delta,
    })
}

#[cfg(feature = "aac")]
pub fn aac_balanced_standard_id_quality_control_profile_for_frame_details(
    pcm: &AudioBuffer,
    details: &[AacPcmFrameStepSelection],
) -> Result<AacStandardIdQualityControlProfile, Error> {
    let profile = aac_standard_id_selected_scale_factor_balance_profile(pcm.channels)?;
    aac_standard_id_quality_control_profile_for_frame_details_with_magnitude_bias_max_quantized_abs(
        pcm,
        details,
        profile.selected_global_gain,
        profile.selected_magnitude_bias,
        profile.max_quantized_abs,
    )
}

#[cfg(feature = "aac")]
pub fn aac_balanced_standard_id_quality_control_profile_with_bitrate(
    pcm: &AudioBuffer,
    target_bitrate_bps: u32,
) -> Result<AacStandardIdQualityControlProfile, Error> {
    let details = aac_balanced_standard_selected_scale_factor_frame_details_with_bitrate(
        pcm,
        target_bitrate_bps,
    )?;
    aac_balanced_standard_id_quality_control_profile_for_frame_details(pcm, &details)
}

#[cfg(feature = "aac")]
pub fn aac_standard_id_quality_control_candidates_for_balance_profile_with_bitrate(
    pcm: &AudioBuffer,
    target_bitrate_bps: u32,
) -> Result<Vec<AacStandardIdQualityControlCandidate>, Error> {
    let balance_profile = aac_standard_id_selected_scale_factor_balance_profile(pcm.channels)?;
    let mut candidates = Vec::new();

    for &global_gain_delta in balance_profile.global_gain_deltas {
        let global_gain = balance_profile
            .recommended_global_gain
            .saturating_add(global_gain_delta);
        for &scale_factor_magnitude_bias in balance_profile.magnitude_biases {
            let details =
                aac_standard_selected_scale_factor_frame_details_with_magnitude_bias_max_quantized_abs_and_bitrate(
                    pcm,
                    target_bitrate_bps,
                    global_gain,
                    scale_factor_magnitude_bias,
                    balance_profile.max_quantized_abs,
                )?;
            let profile =
                aac_standard_id_quality_control_profile_for_frame_details_with_magnitude_bias_max_quantized_abs(
                    pcm,
                    &details,
                    global_gain,
                    scale_factor_magnitude_bias,
                    balance_profile.max_quantized_abs,
                )?;

            if profile.min_frame_budget_slack >= 0
                && profile.max_abs
                    <= i32::try_from(balance_profile.max_quantized_abs).unwrap_or(i32::MAX)
            {
                candidates.push(AacStandardIdQualityControlCandidate {
                    global_gain,
                    scale_factor_magnitude_bias,
                    max_quantized_abs: balance_profile.max_quantized_abs,
                    profile,
                });
            }
        }
    }

    if candidates.is_empty() {
        return Err(Error::InvalidInput(
            "AAC standard-id balanced quality-control profile found no constrained candidates",
        ));
    }

    Ok(candidates)
}
