use super::*;

pub fn spectral_pairs_for_section(
    quantized: &[i32],
    section: &AacSection,
) -> Result<Vec<AacSpectralPair>, Error> {
    if section.end <= section.start || section.end > quantized.len() {
        return Err(Error::InvalidInput("invalid AAC section range"));
    }
    if section.codebook == AacCodebook::Zero {
        return Ok(Vec::new());
    }

    let coeffs = &quantized[section.start..section.end];
    if coeffs.len() % 2 != 0 {
        return Err(Error::InvalidInput(
            "AAC spectral pair section must have even length",
        ));
    }

    coeffs
        .chunks_exact(2)
        .map(|pair| {
            Ok(AacSpectralPair::new(
                i16::try_from(pair[0]).map_err(|_| {
                    Error::InvalidInput("AAC spectral pair coefficient exceeds i16 range")
                })?,
                i16::try_from(pair[1]).map_err(|_| {
                    Error::InvalidInput("AAC spectral pair coefficient exceeds i16 range")
                })?,
            ))
        })
        .collect()
}

/// Converts one AAC section's quantized coefficients into quadruple symbols.
pub fn spectral_quads_for_section(
    quantized: &[i32],
    section: &AacSection,
) -> Result<Vec<AacSpectralQuad>, Error> {
    if section.end <= section.start || section.end > quantized.len() {
        return Err(Error::InvalidInput("invalid AAC section range"));
    }
    if section.codebook == AacCodebook::Zero {
        return Ok(Vec::new());
    }

    let coeffs = &quantized[section.start..section.end];
    if coeffs.len() % 4 != 0 {
        return Err(Error::InvalidInput(
            "AAC spectral quad section must have length divisible by four",
        ));
    }

    coeffs
        .chunks_exact(4)
        .map(|quad| {
            Ok(AacSpectralQuad::new(
                i16::try_from(quad[0]).map_err(|_| {
                    Error::InvalidInput("AAC spectral quad coefficient exceeds i16 range")
                })?,
                i16::try_from(quad[1]).map_err(|_| {
                    Error::InvalidInput("AAC spectral quad coefficient exceeds i16 range")
                })?,
                i16::try_from(quad[2]).map_err(|_| {
                    Error::InvalidInput("AAC spectral quad coefficient exceeds i16 range")
                })?,
                i16::try_from(quad[3]).map_err(|_| {
                    Error::InvalidInput("AAC spectral quad coefficient exceeds i16 range")
                })?,
            ))
        })
        .collect()
}

/// Groups quantized AAC spectral coefficients into contiguous codebook sections.
pub fn plan_sections(quantized: &[i32], band_width: usize) -> Result<Vec<AacSection>, Error> {
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

    let mut sections = Vec::<AacSection>::new();
    for (band_index, band) in quantized.chunks(band_width).enumerate() {
        let codebook = classify_aac_codebook(band)?;
        let start = band_index * band_width;
        let end = start + band_width;
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

/// Selects the shortest available AAC spectral codebook from magnitude tables.
pub fn select_codebook_by_bit_cost(
    quantized: &[i32],
    tables: AacSpectralMagnitudeTables<'_>,
) -> Result<AacCodebook, Error> {
    if quantized.iter().all(|&coeff| coeff == 0) {
        return Ok(AacCodebook::Zero);
    }

    let section = AacSection {
        start: 0,
        end: quantized.len(),
        codebook: AacCodebook::SignedPairs1,
    };
    let pairs = spectral_pairs_for_section(quantized, &section)?;
    let candidates = [
        (AacCodebook::SignedPairs1, tables.pairs1),
        (AacCodebook::SignedPairs5, tables.pairs5),
        (AacCodebook::SignedPairs6, tables.pairs6),
        (AacCodebook::UnsignedPairs7, aac_unsigned_pairs7_table()),
        (AacCodebook::UnsignedPairs8, aac_unsigned_pairs8_table()),
        (AacCodebook::UnsignedPairs9, aac_unsigned_pairs9_table()),
        (AacCodebook::UnsignedPairs10, aac_unsigned_pairs10_table()),
        (AacCodebook::Escape, tables.escape),
    ];
    let mut best: Option<(AacCodebook, usize)> = None;
    for (codebook, table) in candidates {
        if table.is_empty() {
            continue;
        }
        let Ok(packed) = pack_spectral_pairs_with_sign_bits(&pairs, table) else {
            continue;
        };
        if best
            .as_ref()
            .is_none_or(|(_, bit_len)| packed.bit_len < *bit_len)
        {
            best = Some((codebook, packed.bit_len));
        }
    }

    best.map(|(codebook, _)| codebook)
        .ok_or(Error::UnsupportedFeature("AAC spectral codebook"))
}

/// Selects the shortest available AAC quad spectral codebook id from magnitude tables.
pub fn select_quad_codebook_by_bit_cost(
    quantized: &[i32],
    tables: AacSpectralMagnitudeQuadTables<'_>,
) -> Result<u8, Error> {
    if quantized.iter().all(|&coeff| coeff == 0) {
        return Ok(0);
    }

    let section = AacSection {
        start: 0,
        end: quantized.len(),
        codebook: AacCodebook::SignedPairs1,
    };
    let quads = spectral_quads_for_section(quantized, &section)?;
    let mut best: Option<(u8, usize)> = None;
    for (codebook_id, table) in quad_codebook_table_candidates(tables) {
        if table.is_empty() {
            continue;
        }
        let Ok(packed) = pack_spectral_quads_with_sign_bits(&quads, table) else {
            continue;
        };
        if best
            .as_ref()
            .is_none_or(|(_, bit_len)| packed.bit_len < *bit_len)
        {
            best = Some((codebook_id, packed.bit_len));
        }
    }

    best.map(|(codebook_id, _)| codebook_id)
        .ok_or(Error::UnsupportedFeature("AAC quad spectral codebook"))
}

/// Selects the shortest available AAC spectral codebook id using standard id classes.
pub fn select_spectral_codebook_id_by_bit_cost(
    quantized: &[i32],
    pair_tables: AacSpectralMagnitudeTables<'_>,
    quad_tables: AacSpectralMagnitudeQuadTables<'_>,
) -> Result<u8, Error> {
    select_spectral_codebook_id_by_bit_cost_with_signed_pairs(
        quantized,
        pair_tables,
        AacSpectralTables::default(),
        AacSpectralQuadTables::default(),
        quad_tables,
    )
}

pub(crate) fn select_spectral_codebook_id_by_bit_cost_with_signed_pairs(
    quantized: &[i32],
    pair_tables: AacSpectralMagnitudeTables<'_>,
    signed_pair_tables: AacSpectralTables<'_>,
    signed_quad_tables: AacSpectralQuadTables<'_>,
    quad_tables: AacSpectralMagnitudeQuadTables<'_>,
) -> Result<u8, Error> {
    if quantized.iter().all(|&coeff| coeff == 0) {
        return Ok(0);
    }

    let mut best: Option<(u8, usize)> = None;
    if quantized.len() % 4 == 0 {
        let section = AacSection {
            start: 0,
            end: quantized.len(),
            codebook: AacCodebook::SignedPairs1,
        };
        let quads = spectral_quads_for_section(quantized, &section)?;
        for (codebook_id, table) in signed_quad_codebook_table_candidates(signed_quad_tables) {
            if table.is_empty() {
                continue;
            }
            let Ok(packed) = pack_spectral_quads_with_table(&quads, table) else {
                continue;
            };
            if best
                .as_ref()
                .is_none_or(|(_, bit_len)| packed.bit_len < *bit_len)
            {
                best = Some((codebook_id, packed.bit_len));
            }
        }
        for (codebook_id, table) in quad_codebook_table_candidates(quad_tables) {
            if table.is_empty() {
                continue;
            }
            let Ok(packed) = pack_spectral_quads_with_sign_bits(&quads, table) else {
                continue;
            };
            if best
                .as_ref()
                .is_none_or(|(_, bit_len)| packed.bit_len < *bit_len)
            {
                best = Some((codebook_id, packed.bit_len));
            }
        }
    }

    if quantized.len() % 2 == 0 {
        let section = AacSection {
            start: 0,
            end: quantized.len(),
            codebook: AacCodebook::SignedPairs5,
        };
        let pairs = spectral_pairs_for_section(quantized, &section)?;
        let signed_candidates = [
            (
                AacCodebook::SignedPairs5.id(),
                signed_pair_tables.signed_pairs5,
            ),
            (
                AacCodebook::SignedPairs6.id(),
                signed_pair_tables.signed_pairs6,
            ),
        ];
        for (codebook_id, table) in signed_candidates {
            if table.is_empty() {
                continue;
            }
            let Ok(packed) = pack_spectral_pairs_with_table(&pairs, table) else {
                continue;
            };
            if best
                .as_ref()
                .is_none_or(|(_, bit_len)| packed.bit_len < *bit_len)
            {
                best = Some((codebook_id, packed.bit_len));
            }
        }

        let candidates = [
            (AacCodebook::SignedPairs5.id(), pair_tables.pairs5),
            (AacCodebook::SignedPairs6.id(), pair_tables.pairs6),
            (
                AacCodebook::UnsignedPairs7.id(),
                aac_unsigned_pairs7_table(),
            ),
            (
                AacCodebook::UnsignedPairs8.id(),
                aac_unsigned_pairs8_table(),
            ),
            (
                AacCodebook::UnsignedPairs9.id(),
                aac_unsigned_pairs9_table(),
            ),
            (
                AacCodebook::UnsignedPairs10.id(),
                aac_unsigned_pairs10_table(),
            ),
            (AacCodebook::Escape.id(), pair_tables.escape),
        ];
        for (codebook_id, table) in candidates {
            if table.is_empty() {
                continue;
            }
            let Ok(packed) = pack_spectral_pairs_with_sign_bits(&pairs, table) else {
                continue;
            };
            if best
                .as_ref()
                .is_none_or(|(_, bit_len)| packed.bit_len < *bit_len)
            {
                best = Some((codebook_id, packed.bit_len));
            }
        }
    }

    best.map(|(codebook_id, _)| codebook_id)
        .ok_or(Error::UnsupportedFeature("AAC spectral codebook"))
}

pub(crate) fn select_magnitude_section_by_bit_cost<'a>(
    start: usize,
    end: usize,
    quantized: &[i32],
    tables: AacSpectralMagnitudeTables<'a>,
) -> Result<AacMagnitudeSection<'a>, Error> {
    if quantized.iter().all(|&coeff| coeff == 0) {
        return Ok(AacMagnitudeSection {
            start,
            end,
            codebook_id: AacCodebook::Zero.id(),
            table: &[],
        });
    }

    let section = AacSection {
        start: 0,
        end: quantized.len(),
        codebook: AacCodebook::SignedPairs1,
    };
    let pairs = spectral_pairs_for_section(quantized, &section)?;
    let candidates = [
        (AacCodebook::SignedPairs1, tables.pairs1),
        (AacCodebook::SignedPairs5, tables.pairs5),
        (AacCodebook::SignedPairs6, tables.pairs6),
        (AacCodebook::UnsignedPairs7, aac_unsigned_pairs7_table()),
        (AacCodebook::UnsignedPairs8, aac_unsigned_pairs8_table()),
        (AacCodebook::UnsignedPairs9, aac_unsigned_pairs9_table()),
        (AacCodebook::UnsignedPairs10, aac_unsigned_pairs10_table()),
        (AacCodebook::Escape, tables.escape),
    ];
    let mut best: Option<(u8, &'a [HuffmanEntry<AacSpectralMagnitudePair>], usize)> = None;
    for (codebook, table) in candidates {
        if table.is_empty() {
            continue;
        }
        // This selector packs magnitudes through the explicit sign-bit packer, so
        // only unsigned codebooks are valid here: a signed codebook (sign in the
        // codeword) followed by sign bits would be undecodable. Skip them even
        // when a (mis-supplied) magnitude table is provided for one.
        if codebook.embeds_sign() {
            continue;
        }
        let Ok(packed) = pack_spectral_pairs_with_sign_bits(&pairs, table) else {
            continue;
        };
        if best
            .as_ref()
            .is_none_or(|(_, _, bit_len)| packed.bit_len < *bit_len)
        {
            best = Some((codebook.id(), table, packed.bit_len));
        }
    }

    best.map(|(codebook_id, table, _)| AacMagnitudeSection {
        start,
        end,
        codebook_id,
        table,
    })
    .ok_or(Error::UnsupportedFeature("AAC spectral codebook"))
}

/// Groups quantized AAC coefficients into sections using available-table bit costs.
pub fn plan_sections_by_bit_cost(
    quantized: &[i32],
    band_width: usize,
    tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<AacSection>, Error> {
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

    let mut sections = Vec::<AacSection>::new();
    for (band_index, band) in quantized.chunks(band_width).enumerate() {
        let codebook = select_codebook_by_bit_cost(band, tables)?;
        let start = band_index * band_width;
        let end = start + band_width;
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

/// Groups quantized AAC coefficients into quad sections using available-table bit costs.
pub fn plan_quad_sections_by_bit_cost(
    quantized: &[i32],
    band_width: usize,
    tables: AacSpectralMagnitudeQuadTables<'_>,
) -> Result<Vec<AacQuadSection>, Error> {
    if band_width == 0 {
        return Err(Error::InvalidInput(
            "AAC section band width must be non-zero",
        ));
    }
    if band_width % 4 != 0 {
        return Err(Error::InvalidInput(
            "AAC quad section band width must be divisible by four",
        ));
    }
    if quantized.len() % band_width != 0 {
        return Err(Error::InvalidInput(
            "AAC section band width must divide spectrum length",
        ));
    }

    let mut sections = Vec::<AacQuadSection>::new();
    for (band_index, band) in quantized.chunks(band_width).enumerate() {
        let codebook_id = select_quad_codebook_by_bit_cost(band, tables)?;
        let start = band_index * band_width;
        let end = start + band_width;
        match sections.last_mut() {
            Some(section) if section.codebook_id == codebook_id => section.end = end,
            _ => sections.push(AacQuadSection {
                start,
                end,
                codebook_id,
            }),
        }
    }
    Ok(sections)
}

/// Groups quantized AAC coefficients into standard id-based spectral sections.
pub fn plan_spectral_sections_by_bit_cost(
    quantized: &[i32],
    band_width: usize,
    pair_tables: AacSpectralMagnitudeTables<'_>,
    quad_tables: AacSpectralMagnitudeQuadTables<'_>,
) -> Result<Vec<AacSpectralSection>, Error> {
    plan_spectral_sections_by_bit_cost_with_signed_pairs(
        quantized,
        band_width,
        pair_tables,
        AacSpectralTables::default(),
        AacSpectralQuadTables::default(),
        quad_tables,
    )
}

pub(crate) fn plan_spectral_sections_by_bit_cost_with_signed_pairs(
    quantized: &[i32],
    band_width: usize,
    pair_tables: AacSpectralMagnitudeTables<'_>,
    signed_pair_tables: AacSpectralTables<'_>,
    signed_quad_tables: AacSpectralQuadTables<'_>,
    quad_tables: AacSpectralMagnitudeQuadTables<'_>,
) -> Result<Vec<AacSpectralSection>, Error> {
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
    if band_width % 2 != 0 {
        return Err(Error::InvalidInput(
            "AAC spectral section band width must be divisible by two",
        ));
    }

    let mut sections = Vec::<AacSpectralSection>::new();
    for (band_index, band) in quantized.chunks(band_width).enumerate() {
        let codebook_id = select_spectral_codebook_id_by_bit_cost_with_signed_pairs(
            band,
            pair_tables,
            signed_pair_tables,
            signed_quad_tables,
            quad_tables,
        )?;
        let start = band_index * band_width;
        let end = start + band_width;
        match sections.last_mut() {
            Some(section) if section.codebook_id == codebook_id => section.end = end,
            _ => sections.push(AacSpectralSection {
                start,
                end,
                codebook_id,
            }),
        }
    }
    Ok(sections)
}

/// Groups quantized AAC coefficients into standard id-based spectral sections
/// using scale-factor band offsets.
pub fn plan_spectral_sections_by_offsets_by_bit_cost(
    quantized: &[i32],
    offsets: &[usize],
    pair_tables: AacSpectralMagnitudeTables<'_>,
    quad_tables: AacSpectralMagnitudeQuadTables<'_>,
) -> Result<Vec<AacSpectralSection>, Error> {
    plan_spectral_sections_by_offsets_by_bit_cost_with_signed_pairs(
        quantized,
        offsets,
        pair_tables,
        AacSpectralTables::default(),
        AacSpectralQuadTables::default(),
        quad_tables,
    )
}

pub(crate) fn plan_spectral_sections_by_offsets_by_bit_cost_with_signed_pairs(
    quantized: &[i32],
    offsets: &[usize],
    pair_tables: AacSpectralMagnitudeTables<'_>,
    signed_pair_tables: AacSpectralTables<'_>,
    signed_quad_tables: AacSpectralQuadTables<'_>,
    quad_tables: AacSpectralMagnitudeQuadTables<'_>,
) -> Result<Vec<AacSpectralSection>, Error> {
    validate_scale_factor_band_offsets(quantized, offsets)?;

    let mut sections = Vec::<AacSpectralSection>::new();
    for band in offsets.windows(2) {
        let start = band[0];
        let end = band[1];
        let width = end - start;
        if width % 2 != 0 {
            return Err(Error::InvalidInput(
                "AAC spectral section band width must be divisible by two",
            ));
        }
        let codebook_id = select_spectral_codebook_id_by_bit_cost_with_signed_pairs(
            &quantized[start..end],
            pair_tables,
            signed_pair_tables,
            signed_quad_tables,
            quad_tables,
        )?;
        match sections.last_mut() {
            Some(section) if section.codebook_id == codebook_id => section.end = end,
            _ => sections.push(AacSpectralSection {
                start,
                end,
                codebook_id,
            }),
        }
    }
    Ok(sections)
}

/// Groups quantized AAC coefficients with the implemented standard spectral tables.
pub fn plan_aac_lc_standard_spectral_sections_by_bit_cost(
    quantized: &[i32],
    band_width: usize,
) -> Result<Vec<AacSpectralSection>, Error> {
    plan_spectral_sections_by_bit_cost_with_signed_pairs(
        quantized,
        band_width,
        aac_lc_standard_spectral_tables(),
        aac_lc_standard_signed_pair_tables(),
        aac_lc_standard_signed_quad_tables(),
        aac_lc_standard_unsigned_quad_tables(),
    )
}

/// Groups quantized AAC coefficients by scale-factor band offsets with the
/// implemented standard spectral tables.
pub fn plan_aac_lc_standard_spectral_sections_by_offsets_by_bit_cost(
    quantized: &[i32],
    offsets: &[usize],
) -> Result<Vec<AacSpectralSection>, Error> {
    plan_spectral_sections_by_offsets_by_bit_cost_with_signed_pairs(
        quantized,
        offsets,
        aac_lc_standard_spectral_tables(),
        aac_lc_standard_signed_pair_tables(),
        aac_lc_standard_signed_quad_tables(),
        aac_lc_standard_unsigned_quad_tables(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn magnitude_section_never_selects_a_signed_codebook() {
        // The magnitude selector packs explicit sign bits, so it must only ever
        // emit unsigned codebooks (7-11). Even when a signed magnitude table is
        // supplied (codebook 6 here), the sign-embedding codebooks are skipped so
        // the section stays decodable instead of carrying a double sign.
        let tables = AacSpectralMagnitudeTables {
            pairs6: aac_unit_codebook6_spectral_tables().pairs6,
            escape: aac_escape_table(),
            ..Default::default()
        };
        let quantized = [3, -2, 1, -4, 2, -1];
        let section = select_magnitude_section_by_bit_cost(0, quantized.len(), &quantized, tables)
            .expect("a non-zero magnitude section");
        assert!(
            !matches!(
                section.codebook_id,
                id if AacCodebook::SignedPairs1.id() == id
                    || AacCodebook::SignedPairs5.id() == id
                    || AacCodebook::SignedPairs6.id() == id
            ),
            "magnitude selector chose a signed codebook id {}",
            section.codebook_id
        );
        assert!((7..=11).contains(&section.codebook_id));
    }
}
