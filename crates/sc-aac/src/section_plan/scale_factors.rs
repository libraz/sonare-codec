use super::*;

pub fn plan_sections_by_offsets(
    quantized: &[i32],
    offsets: &[usize],
    tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<AacSection>, Error> {
    validate_scale_factor_band_offsets(quantized, offsets)?;

    let mut sections = Vec::<AacSection>::new();
    for band in offsets.windows(2) {
        let start = band[0];
        let end = band[1];
        let codebook = select_codebook_by_bit_cost(&quantized[start..end], tables)?;
        match sections.last_mut() {
            Some(section) if section.codebook == codebook => section.end = end,
            _ => sections.push(AacSection {
                start,
                end,
                codebook,
            }),
        }
    }
    Ok(sections)
}

pub(crate) fn plan_magnitude_sections_by_offsets<'a>(
    quantized: &[i32],
    offsets: &[usize],
    tables: AacSpectralMagnitudeTables<'a>,
) -> Result<Vec<AacMagnitudeSection<'a>>, Error> {
    validate_scale_factor_band_offsets(quantized, offsets)?;

    let mut sections = Vec::<AacMagnitudeSection<'a>>::new();
    for band in offsets.windows(2) {
        let start = band[0];
        let end = band[1];
        let planned =
            select_magnitude_section_by_bit_cost(start, end, &quantized[start..end], tables)?;
        match sections.last_mut() {
            Some(section) if section.codebook_id == planned.codebook_id => section.end = end,
            _ => sections.push(planned),
        }
    }
    Ok(sections)
}

/// Computes scale-factor DPCM deltas for non-zero AAC sections.
pub fn plan_scale_factor_deltas(
    sections: &[AacSection],
    band_width: usize,
    scale_factors: &[i16],
    initial_scale_factor: i16,
) -> Result<Vec<AacScaleFactorDelta>, Error> {
    if band_width == 0 {
        return Err(Error::InvalidInput(
            "AAC section band width must be non-zero",
        ));
    }

    let mut previous = initial_scale_factor;
    let mut deltas = Vec::new();
    for section in sections {
        if section.end <= section.start
            || section.start % band_width != 0
            || section.end % band_width != 0
        {
            return Err(Error::InvalidInput("invalid AAC section range"));
        }
        if section.codebook == AacCodebook::Zero {
            continue;
        }

        let start_band = section.start / band_width;
        let end_band = section.end / band_width;
        if end_band > scale_factors.len() {
            return Err(Error::InvalidInput("missing AAC scale factor"));
        }
        for &scale_factor in &scale_factors[start_band..end_band] {
            let delta = scale_factor
                .checked_sub(previous)
                .ok_or(Error::InvalidInput("AAC scale-factor delta overflows"))?;
            deltas.push(AacScaleFactorDelta::new(delta));
            previous = scale_factor;
        }
    }
    Ok(deltas)
}

pub fn plan_scale_factor_deltas_by_offsets(
    sections: &[AacSection],
    offsets: &[usize],
    scale_factors: &[i16],
    initial_scale_factor: i16,
) -> Result<Vec<AacScaleFactorDelta>, Error> {
    if offsets.len() < 2 {
        return Err(Error::InvalidInput("AAC scale-factor offsets are empty"));
    }
    if scale_factors.len() + 1 != offsets.len() {
        return Err(Error::InvalidInput("missing AAC scale factor"));
    }

    let mut previous = initial_scale_factor;
    let mut deltas = Vec::new();
    for section in sections {
        if section.end <= section.start {
            return Err(Error::InvalidInput("invalid AAC section range"));
        }
        if section.codebook == AacCodebook::Zero {
            continue;
        }

        let start_band = offset_band_index(offsets, section.start)?;
        let end_band = offset_band_index(offsets, section.end)?;
        if end_band > scale_factors.len() {
            return Err(Error::InvalidInput("missing AAC scale factor"));
        }
        for &scale_factor in &scale_factors[start_band..end_band] {
            let delta = scale_factor
                .checked_sub(previous)
                .ok_or(Error::InvalidInput("AAC scale-factor delta overflows"))?;
            deltas.push(AacScaleFactorDelta::new(delta));
            previous = scale_factor;
        }
    }
    Ok(deltas)
}

/// Computes scale-factor DPCM deltas for non-zero standard id-based AAC sections.
pub fn plan_spectral_scale_factor_deltas_by_offsets(
    sections: &[AacSpectralSection],
    offsets: &[usize],
    scale_factors: &[i16],
    initial_scale_factor: i16,
) -> Result<Vec<AacScaleFactorDelta>, Error> {
    if offsets.len() < 2 {
        return Err(Error::InvalidInput("AAC scale-factor offsets are empty"));
    }
    if scale_factors.len() + 1 != offsets.len() {
        return Err(Error::InvalidInput("missing AAC scale factor"));
    }

    let mut previous = initial_scale_factor;
    let mut deltas = Vec::new();
    for section in sections {
        if section.end <= section.start {
            return Err(Error::InvalidInput("invalid AAC section range"));
        }
        if section.codebook_id == 0 {
            continue;
        }

        let start_band = offset_band_index(offsets, section.start)?;
        let end_band = offset_band_index(offsets, section.end)?;
        if end_band > scale_factors.len() {
            return Err(Error::InvalidInput("missing AAC scale factor"));
        }
        for &scale_factor in &scale_factors[start_band..end_band] {
            let delta = scale_factor
                .checked_sub(previous)
                .ok_or(Error::InvalidInput("AAC scale-factor delta overflows"))?;
            deltas.push(AacScaleFactorDelta::new(delta));
            previous = scale_factor;
        }
    }
    Ok(deltas)
}

pub(crate) fn plan_magnitude_scale_factor_deltas_by_offsets(
    sections: &[AacMagnitudeSection<'_>],
    offsets: &[usize],
    scale_factors: &[i16],
    initial_scale_factor: i16,
) -> Result<Vec<AacScaleFactorDelta>, Error> {
    if offsets.len() < 2 {
        return Err(Error::InvalidInput("AAC scale-factor offsets are empty"));
    }
    if scale_factors.len() + 1 != offsets.len() {
        return Err(Error::InvalidInput("missing AAC scale factor"));
    }

    let mut previous = initial_scale_factor;
    let mut deltas = Vec::new();
    for section in sections {
        if section.end <= section.start {
            return Err(Error::InvalidInput("invalid AAC section range"));
        }
        if section.is_zero() {
            continue;
        }

        let start_band = offset_band_index(offsets, section.start)?;
        let end_band = offset_band_index(offsets, section.end)?;
        if end_band > scale_factors.len() {
            return Err(Error::InvalidInput("missing AAC scale factor"));
        }
        for &scale_factor in &scale_factors[start_band..end_band] {
            let delta = scale_factor
                .checked_sub(previous)
                .ok_or(Error::InvalidInput("AAC scale-factor delta overflows"))?;
            deltas.push(AacScaleFactorDelta::new(delta));
            previous = scale_factor;
        }
    }
    Ok(deltas)
}

/// Selects a deterministic per-band scale-factor seed from quantized magnitudes.
pub fn select_scale_factors_for_quantized_bands(
    quantized: &[i32],
    band_width: usize,
    base_scale_factor: i16,
) -> Result<Vec<i16>, Error> {
    if band_width == 0 {
        return Err(Error::InvalidInput(
            "AAC section band width must be non-zero",
        ));
    }
    if quantized.len() % band_width != 0 {
        return Err(Error::InvalidInput(
            "AAC section band width must divide spectrum length",
        ));
    }

    quantized
        .chunks(band_width)
        .map(|band| {
            let max_abs = band
                .iter()
                .map(|coeff| coeff.checked_abs())
                .collect::<Option<Vec<_>>>()
                .ok_or(Error::InvalidInput("AAC spectral coefficient overflows"))?
                .into_iter()
                .max()
                .unwrap_or(0);
            let magnitude_class = if max_abs == 0 {
                0
            } else {
                i16::try_from(32 - max_abs.leading_zeros()).map_err(|_| {
                    Error::InvalidInput("AAC scale-factor magnitude class overflows")
                })?
            };
            base_scale_factor
                .checked_add(magnitude_class)
                .ok_or(Error::InvalidInput("AAC scale factor overflows"))
        })
        .collect()
}

pub fn select_scale_factors_for_quantized_bands_by_offsets(
    quantized: &[i32],
    offsets: &[usize],
    base_scale_factor: i16,
) -> Result<Vec<i16>, Error> {
    select_scale_factors_for_quantized_bands_by_offsets_with_magnitude_bias(
        quantized,
        offsets,
        base_scale_factor,
        0,
    )
}

pub fn select_scale_factors_for_quantized_bands_by_offsets_with_magnitude_bias(
    quantized: &[i32],
    offsets: &[usize],
    base_scale_factor: i16,
    magnitude_bias: i16,
) -> Result<Vec<i16>, Error> {
    validate_scale_factor_band_offsets(quantized, offsets)?;

    offsets
        .windows(2)
        .map(|band| {
            let max_abs = quantized[band[0]..band[1]]
                .iter()
                .map(|coeff| coeff.checked_abs())
                .collect::<Option<Vec<_>>>()
                .ok_or(Error::InvalidInput("AAC spectral coefficient overflows"))?
                .into_iter()
                .max()
                .unwrap_or(0);
            let magnitude_class = if max_abs == 0 {
                0
            } else {
                i16::try_from(32 - max_abs.leading_zeros()).map_err(|_| {
                    Error::InvalidInput("AAC scale-factor magnitude class overflows")
                })?
            };
            let biased_magnitude_class = magnitude_class.saturating_sub(magnitude_bias).max(0);
            base_scale_factor
                .checked_add(biased_magnitude_class)
                .ok_or(Error::InvalidInput("AAC scale factor overflows"))
        })
        .collect()
}
