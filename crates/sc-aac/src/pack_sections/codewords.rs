use super::*;

/// Packs preselected AAC spectral Huffman codewords.
pub fn pack_spectral_codewords(codes: &[HuffmanCode]) -> Result<Vec<u8>, Error> {
    pack_huffman_codes(codes)
}

/// Packs preselected AAC spectral Huffman codewords and preserves bit length.
pub fn pack_spectral_codewords_with_len(codes: &[HuffmanCode]) -> Result<PackedBits, Error> {
    pack_huffman_codes_with_len(codes)
}

/// Packs scale-factor DPCM deltas with a caller-supplied Huffman table.
pub fn pack_scale_factor_deltas_with_table(
    deltas: &[AacScaleFactorDelta],
    table: &[HuffmanEntry<AacScaleFactorDelta>],
) -> Result<PackedBits, Error> {
    pack_huffman_symbols_with_len(deltas, table)
}

/// Packs AAC spectral pairs using a caller-supplied codebook table.
pub fn pack_spectral_pairs_with_table(
    pairs: &[AacSpectralPair],
    table: &[HuffmanEntry<AacSpectralPair>],
) -> Result<PackedBits, Error> {
    pack_huffman_symbols_with_len(pairs, table)
}

/// Packs AAC spectral quadruples using a caller-supplied codebook table.
pub fn pack_spectral_quads_with_table(
    quads: &[AacSpectralQuad],
    table: &[HuffmanEntry<AacSpectralQuad>],
) -> Result<PackedBits, Error> {
    pack_huffman_symbols_with_len(quads, table)
}

/// Packs AAC spectral pairs with magnitude-keyed codewords followed by sign bits.
pub fn pack_spectral_pairs_with_sign_bits(
    pairs: &[AacSpectralPair],
    table: &[HuffmanEntry<AacSpectralMagnitudePair>],
) -> Result<PackedBits, Error> {
    let mut writer = CoreBitWriter::new();
    for pair in pairs {
        let magnitude = aac_spectral_pair_magnitude(*pair)?;
        let table_magnitude = AacSpectralMagnitudePair::new(
            magnitude.x.min(AAC_ESCAPE_MAGNITUDE),
            magnitude.y.min(AAC_ESCAPE_MAGNITUDE),
        );
        let code = sc_core::lookup_huffman_code(table, &table_magnitude)?;
        writer.write_bits(code.bits, code.len)?;
        write_aac_sign_bit(&mut writer, pair.x)?;
        write_aac_sign_bit(&mut writer, pair.y)?;
        write_aac_escape_suffix(&mut writer, magnitude.x)?;
        write_aac_escape_suffix(&mut writer, magnitude.y)?;
    }
    let bit_len = writer.bit_len();
    Ok(PackedBits {
        bytes: writer.finish_byte_aligned(),
        bit_len,
    })
}

/// Packs AAC spectral quadruples with magnitude-keyed codewords followed by sign bits.
pub fn pack_spectral_quads_with_sign_bits(
    quads: &[AacSpectralQuad],
    table: &[HuffmanEntry<AacSpectralMagnitudeQuad>],
) -> Result<PackedBits, Error> {
    let mut writer = CoreBitWriter::new();
    for quad in quads {
        let magnitude = aac_spectral_quad_magnitude(*quad)?;
        let code = sc_core::lookup_huffman_code(table, &magnitude)?;
        writer.write_bits(code.bits, code.len)?;
        write_aac_sign_bit(&mut writer, quad.v)?;
        write_aac_sign_bit(&mut writer, quad.w)?;
        write_aac_sign_bit(&mut writer, quad.x)?;
        write_aac_sign_bit(&mut writer, quad.y)?;
    }
    let bit_len = writer.bit_len();
    Ok(PackedBits {
        bytes: writer.finish_byte_aligned(),
        bit_len,
    })
}

/// Packs all non-zero AAC spectral sections with caller-supplied codebook tables.
pub fn pack_spectral_sections(
    sections: &[AacSection],
    quantized: &[i32],
    tables: AacSpectralTables<'_>,
) -> Result<PackedBits, Error> {
    let mut parts = Vec::new();
    for section in sections {
        let pairs = spectral_pairs_for_section(quantized, section)?;
        if pairs.is_empty() {
            continue;
        }
        parts.push(pack_spectral_pairs_with_table(
            &pairs,
            tables.table_for(section.codebook)?,
        )?);
    }
    concat_packed_bits(&parts)
}

/// Packs all non-zero AAC spectral sections using magnitude tables and sign bits.
pub fn pack_spectral_sections_with_sign_bits(
    sections: &[AacSection],
    quantized: &[i32],
    tables: AacSpectralMagnitudeTables<'_>,
) -> Result<PackedBits, Error> {
    let mut parts = Vec::new();
    for section in sections {
        let pairs = spectral_pairs_for_section(quantized, section)?;
        if pairs.is_empty() {
            continue;
        }
        parts.push(pack_spectral_pairs_with_sign_bits(
            &pairs,
            tables.table_for(section.codebook)?,
        )?);
    }
    concat_packed_bits(&parts)
}

/// Packs all non-zero AAC quad sections using magnitude tables and sign bits.
pub fn pack_spectral_quad_sections_with_sign_bits(
    sections: &[AacQuadSection],
    quantized: &[i32],
    tables: AacSpectralMagnitudeQuadTables<'_>,
) -> Result<PackedBits, Error> {
    let mut parts = Vec::new();
    for section in sections {
        if section.end <= section.start || section.end > quantized.len() {
            return Err(Error::InvalidInput("invalid AAC section range"));
        }
        if section.codebook_id == 0 {
            continue;
        }
        let public_section = AacSection {
            start: section.start,
            end: section.end,
            codebook: AacCodebook::SignedPairs1,
        };
        let quads = spectral_quads_for_section(quantized, &public_section)?;
        if quads.is_empty() {
            continue;
        }
        parts.push(pack_spectral_quads_with_sign_bits(
            &quads,
            tables.table_for_codebook_id(section.codebook_id)?,
        )?);
    }
    concat_packed_bits(&parts)
}

/// Packs standard id-based AAC spectral sections using pair and quad magnitude tables.
pub fn pack_spectral_sections_by_codebook_id_with_sign_bits(
    sections: &[AacSpectralSection],
    quantized: &[i32],
    pair_tables: AacSpectralMagnitudeTables<'_>,
    quad_tables: AacSpectralMagnitudeQuadTables<'_>,
) -> Result<PackedBits, Error> {
    pack_spectral_sections_by_codebook_id_with_signed_pairs(
        sections,
        quantized,
        pair_tables,
        AacSpectralTables::default(),
        AacSpectralQuadTables::default(),
        quad_tables,
    )
}

pub(crate) fn pack_spectral_sections_by_codebook_id_with_signed_pairs(
    sections: &[AacSpectralSection],
    quantized: &[i32],
    pair_tables: AacSpectralMagnitudeTables<'_>,
    signed_pair_tables: AacSpectralTables<'_>,
    signed_quad_tables: AacSpectralQuadTables<'_>,
    quad_tables: AacSpectralMagnitudeQuadTables<'_>,
) -> Result<PackedBits, Error> {
    let mut parts = Vec::new();
    for section in sections {
        if section.end <= section.start || section.end > quantized.len() {
            return Err(Error::InvalidInput("invalid AAC section range"));
        }
        match section.codebook_id {
            0 => {}
            1 | 2
                if match section.codebook_id {
                    1 => !signed_quad_tables.quads1.is_empty(),
                    2 => !signed_quad_tables.quads2.is_empty(),
                    _ => false,
                } =>
            {
                let public_section = AacSection {
                    start: section.start,
                    end: section.end,
                    codebook: AacCodebook::SignedPairs1,
                };
                let quads = spectral_quads_for_section(quantized, &public_section)?;
                parts.push(pack_spectral_quads_with_table(
                    &quads,
                    signed_quad_tables.table_for_codebook_id(section.codebook_id)?,
                )?);
            }
            1..=4 => {
                let public_section = AacSection {
                    start: section.start,
                    end: section.end,
                    codebook: AacCodebook::SignedPairs1,
                };
                let quads = spectral_quads_for_section(quantized, &public_section)?;
                parts.push(pack_spectral_quads_with_sign_bits(
                    &quads,
                    quad_tables.table_for_codebook_id(section.codebook_id)?,
                )?);
            }
            5 | 6
                if match section.codebook_id {
                    5 => !signed_pair_tables.signed_pairs5.is_empty(),
                    6 => !signed_pair_tables.signed_pairs6.is_empty(),
                    _ => false,
                } =>
            {
                let codebook = if section.codebook_id == 5 {
                    AacCodebook::SignedPairs5
                } else {
                    AacCodebook::SignedPairs6
                };
                let public_section = AacSection {
                    start: section.start,
                    end: section.end,
                    codebook,
                };
                let pairs = spectral_pairs_for_section(quantized, &public_section)?;
                parts.push(pack_spectral_pairs_with_table(
                    &pairs,
                    signed_pair_tables.table_for(codebook)?,
                )?);
            }
            5..=11 => {
                let codebook = match section.codebook_id {
                    5 => AacCodebook::SignedPairs5,
                    6 => AacCodebook::SignedPairs6,
                    7 => AacCodebook::UnsignedPairs7,
                    8 => AacCodebook::UnsignedPairs8,
                    9 => AacCodebook::UnsignedPairs9,
                    10 => AacCodebook::UnsignedPairs10,
                    11 => AacCodebook::Escape,
                    _ => unreachable!(),
                };
                let public_section = AacSection {
                    start: section.start,
                    end: section.end,
                    codebook,
                };
                let pairs = spectral_pairs_for_section(quantized, &public_section)?;
                parts.push(pack_spectral_pairs_with_sign_bits(
                    &pairs,
                    pair_tables.table_for(codebook)?,
                )?);
            }
            _ => {
                return Err(Error::InvalidInput(
                    "AAC spectral codebook id must be 0..=11",
                ))
            }
        }
    }
    concat_packed_bits(&parts)
}

pub(crate) fn pack_magnitude_spectral_sections_with_sign_bits(
    sections: &[AacMagnitudeSection<'_>],
    quantized: &[i32],
) -> Result<PackedBits, Error> {
    let mut parts = Vec::new();
    for section in sections {
        if section.end <= section.start || section.end > quantized.len() {
            return Err(Error::InvalidInput("invalid AAC section range"));
        }
        if section.is_zero() {
            continue;
        }
        let public_section = AacSection {
            start: section.start,
            end: section.end,
            codebook: AacCodebook::SignedPairs1,
        };
        let pairs = spectral_pairs_for_section(quantized, &public_section)?;
        parts.push(pack_spectral_pairs_with_sign_bits(&pairs, section.table)?);
    }
    concat_packed_bits(&parts)
}
