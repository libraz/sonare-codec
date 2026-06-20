use super::*;

/// Packs AAC section metadata followed by the matching section spectral payloads.
pub fn pack_sectioned_spectral_payload(
    sections: &[AacSection],
    quantized: &[i32],
    band_width: usize,
    tables: AacSpectralTables<'_>,
) -> Result<PackedBits, Error> {
    let section_bits = pack_section_data_with_len(sections, band_width)?;
    let spectral_bits = pack_spectral_sections(sections, quantized, tables)?;
    concat_packed_bits(&[section_bits, spectral_bits])
}

/// Packs AAC section, scale-factor, and spectral payload bits in ICS order.
pub fn pack_channel_payload_parts(
    section_bits: PackedBits,
    scale_factor_bits: PackedBits,
    spectral_bits: PackedBits,
) -> Result<PackedBits, Error> {
    concat_packed_bits(&[section_bits, scale_factor_bits, spectral_bits])
}

/// Splits AAC payload bits at the point where ICS pulse/TNS/gain flags must be inserted.
pub fn split_channel_payload_parts(
    section_bits: PackedBits,
    scale_factor_bits: PackedBits,
    spectral_bits: PackedBits,
) -> Result<AacIndividualChannelPayload, Error> {
    let section_and_scale_factor_bits = concat_packed_bits(&[section_bits, scale_factor_bits])?;
    Ok(AacIndividualChannelPayload::new(
        section_and_scale_factor_bits,
        spectral_bits,
    ))
}

/// Packs AAC section metadata, scale-factor bits, and signed-pair spectral payloads.
pub fn pack_sectioned_spectral_payload_with_scale_factor_bits(
    sections: &[AacSection],
    quantized: &[i32],
    band_width: usize,
    scale_factor_bits: PackedBits,
    tables: AacSpectralTables<'_>,
) -> Result<PackedBits, Error> {
    let section_bits = pack_section_data_with_len(sections, band_width)?;
    let spectral_bits = pack_spectral_sections(sections, quantized, tables)?;
    pack_channel_payload_parts(section_bits, scale_factor_bits, spectral_bits)
}

/// Builds separated signed-pair payload parts for a long-block individual_channel_stream.
pub fn split_sectioned_spectral_payload_with_scale_factor_bits(
    sections: &[AacSection],
    quantized: &[i32],
    band_width: usize,
    scale_factor_bits: PackedBits,
    tables: AacSpectralTables<'_>,
) -> Result<AacIndividualChannelPayload, Error> {
    let section_bits = pack_section_data_with_len(sections, band_width)?;
    let spectral_bits = pack_spectral_sections(sections, quantized, tables)?;
    split_channel_payload_parts(section_bits, scale_factor_bits, spectral_bits)
}

/// Packs AAC section metadata followed by magnitude-keyed spectral payloads.
pub fn pack_sectioned_spectral_payload_with_sign_bits(
    sections: &[AacSection],
    quantized: &[i32],
    band_width: usize,
    tables: AacSpectralMagnitudeTables<'_>,
) -> Result<PackedBits, Error> {
    let section_bits = pack_section_data_with_len(sections, band_width)?;
    let spectral_bits = pack_spectral_sections_with_sign_bits(sections, quantized, tables)?;
    concat_packed_bits(&[section_bits, spectral_bits])
}

/// Packs AAC quad section metadata followed by magnitude-keyed quad spectral payloads.
pub fn pack_sectioned_spectral_quad_payload_with_sign_bits(
    sections: &[AacQuadSection],
    quantized: &[i32],
    band_width: usize,
    tables: AacSpectralMagnitudeQuadTables<'_>,
) -> Result<PackedBits, Error> {
    let section_bits = pack_quad_section_data_with_len(sections, band_width)?;
    let spectral_bits = pack_spectral_quad_sections_with_sign_bits(sections, quantized, tables)?;
    concat_packed_bits(&[section_bits, spectral_bits])
}

/// Packs AAC quad sections selected by available-table bit costs.
pub fn pack_sectioned_spectral_quad_payload_with_sign_bits_by_bit_cost(
    quantized: &[i32],
    band_width: usize,
    tables: AacSpectralMagnitudeQuadTables<'_>,
) -> Result<PackedBits, Error> {
    let sections = plan_quad_sections_by_bit_cost(quantized, band_width, tables)?;
    pack_sectioned_spectral_quad_payload_with_sign_bits(&sections, quantized, band_width, tables)
}

/// Packs standard id-based AAC sections followed by matching spectral payloads.
pub fn pack_sectioned_spectral_payload_by_codebook_id_with_sign_bits(
    sections: &[AacSpectralSection],
    quantized: &[i32],
    band_width: usize,
    pair_tables: AacSpectralMagnitudeTables<'_>,
    quad_tables: AacSpectralMagnitudeQuadTables<'_>,
) -> Result<PackedBits, Error> {
    pack_sectioned_spectral_payload_by_codebook_id_with_signed_pairs(
        sections,
        quantized,
        band_width,
        pair_tables,
        AacSpectralTables::default(),
        AacSpectralQuadTables::default(),
        quad_tables,
    )
}

pub(crate) fn pack_sectioned_spectral_payload_by_codebook_id_with_signed_pairs(
    sections: &[AacSpectralSection],
    quantized: &[i32],
    band_width: usize,
    pair_tables: AacSpectralMagnitudeTables<'_>,
    signed_pair_tables: AacSpectralTables<'_>,
    signed_quad_tables: AacSpectralQuadTables<'_>,
    quad_tables: AacSpectralMagnitudeQuadTables<'_>,
) -> Result<PackedBits, Error> {
    let section_bits = pack_spectral_section_data_with_len(sections, band_width)?;
    let spectral_bits = pack_spectral_sections_by_codebook_id_with_signed_pairs(
        sections,
        quantized,
        pair_tables,
        signed_pair_tables,
        signed_quad_tables,
        quad_tables,
    )?;
    concat_packed_bits(&[section_bits, spectral_bits])
}

/// Packs standard id-based AAC sections followed by matching spectral payloads
/// using scale-factor band offsets.
pub fn pack_sectioned_spectral_payload_by_codebook_id_with_offsets_and_sign_bits(
    sections: &[AacSpectralSection],
    quantized: &[i32],
    offsets: &[usize],
    pair_tables: AacSpectralMagnitudeTables<'_>,
    quad_tables: AacSpectralMagnitudeQuadTables<'_>,
) -> Result<PackedBits, Error> {
    pack_sectioned_spectral_payload_by_codebook_id_with_offsets_and_signed_pairs(
        sections,
        quantized,
        offsets,
        pair_tables,
        AacSpectralTables::default(),
        AacSpectralQuadTables::default(),
        quad_tables,
    )
}

pub(crate) fn pack_sectioned_spectral_payload_by_codebook_id_with_offsets_and_signed_pairs(
    sections: &[AacSpectralSection],
    quantized: &[i32],
    offsets: &[usize],
    pair_tables: AacSpectralMagnitudeTables<'_>,
    signed_pair_tables: AacSpectralTables<'_>,
    signed_quad_tables: AacSpectralQuadTables<'_>,
    quad_tables: AacSpectralMagnitudeQuadTables<'_>,
) -> Result<PackedBits, Error> {
    let section_bits = pack_spectral_section_data_with_offsets(sections, offsets)?;
    let spectral_bits = pack_spectral_sections_by_codebook_id_with_signed_pairs(
        sections,
        quantized,
        pair_tables,
        signed_pair_tables,
        signed_quad_tables,
        quad_tables,
    )?;
    concat_packed_bits(&[section_bits, spectral_bits])
}

/// Builds separated standard id-based payload parts for a long-block individual_channel_stream.
pub fn split_sectioned_spectral_payload_by_codebook_id_with_sign_bits(
    sections: &[AacSpectralSection],
    quantized: &[i32],
    band_width: usize,
    pair_tables: AacSpectralMagnitudeTables<'_>,
    quad_tables: AacSpectralMagnitudeQuadTables<'_>,
) -> Result<AacIndividualChannelPayload, Error> {
    split_sectioned_spectral_payload_by_codebook_id_with_signed_pairs(
        sections,
        quantized,
        band_width,
        pair_tables,
        AacSpectralTables::default(),
        AacSpectralQuadTables::default(),
        quad_tables,
    )
}

pub(crate) fn split_sectioned_spectral_payload_by_codebook_id_with_signed_pairs(
    sections: &[AacSpectralSection],
    quantized: &[i32],
    band_width: usize,
    pair_tables: AacSpectralMagnitudeTables<'_>,
    signed_pair_tables: AacSpectralTables<'_>,
    signed_quad_tables: AacSpectralQuadTables<'_>,
    quad_tables: AacSpectralMagnitudeQuadTables<'_>,
) -> Result<AacIndividualChannelPayload, Error> {
    let section_bits = pack_spectral_section_data_with_len(sections, band_width)?;
    let spectral_bits = pack_spectral_sections_by_codebook_id_with_signed_pairs(
        sections,
        quantized,
        pair_tables,
        signed_pair_tables,
        signed_quad_tables,
        quad_tables,
    )?;
    split_channel_payload_parts(
        section_bits,
        PackedBits {
            bytes: Vec::new(),
            bit_len: 0,
        },
        spectral_bits,
    )
}

/// Builds separated standard id-based payload parts using scale-factor band offsets.
pub fn split_sectioned_spectral_payload_by_codebook_id_with_offsets_and_sign_bits(
    sections: &[AacSpectralSection],
    quantized: &[i32],
    offsets: &[usize],
    pair_tables: AacSpectralMagnitudeTables<'_>,
    quad_tables: AacSpectralMagnitudeQuadTables<'_>,
) -> Result<AacIndividualChannelPayload, Error> {
    split_sectioned_spectral_payload_by_codebook_id_with_offsets_and_signed_pairs(
        sections,
        quantized,
        offsets,
        pair_tables,
        AacSpectralTables::default(),
        AacSpectralQuadTables::default(),
        quad_tables,
    )
}

pub(crate) fn split_sectioned_spectral_payload_by_codebook_id_with_offsets_and_signed_pairs(
    sections: &[AacSpectralSection],
    quantized: &[i32],
    offsets: &[usize],
    pair_tables: AacSpectralMagnitudeTables<'_>,
    signed_pair_tables: AacSpectralTables<'_>,
    signed_quad_tables: AacSpectralQuadTables<'_>,
    quad_tables: AacSpectralMagnitudeQuadTables<'_>,
) -> Result<AacIndividualChannelPayload, Error> {
    let section_bits = pack_spectral_section_data_with_offsets(sections, offsets)?;
    let spectral_bits = pack_spectral_sections_by_codebook_id_with_signed_pairs(
        sections,
        quantized,
        pair_tables,
        signed_pair_tables,
        signed_quad_tables,
        quad_tables,
    )?;
    split_channel_payload_parts(
        section_bits,
        PackedBits {
            bytes: Vec::new(),
            bit_len: 0,
        },
        spectral_bits,
    )
}

/// Packs standard id-based AAC sections, scale-factor bits, and spectral payloads.
pub fn pack_sectioned_spectral_payload_by_codebook_id_with_sign_bits_and_scale_factor_bits(
    sections: &[AacSpectralSection],
    quantized: &[i32],
    band_width: usize,
    scale_factor_bits: PackedBits,
    pair_tables: AacSpectralMagnitudeTables<'_>,
    quad_tables: AacSpectralMagnitudeQuadTables<'_>,
) -> Result<PackedBits, Error> {
    pack_sectioned_spectral_payload_by_codebook_id_with_signed_pairs_and_scale_factor_bits(
        sections,
        quantized,
        band_width,
        scale_factor_bits,
        pair_tables,
        AacSpectralTables::default(),
        AacSpectralQuadTables::default(),
        quad_tables,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn pack_sectioned_spectral_payload_by_codebook_id_with_signed_pairs_and_scale_factor_bits(
    sections: &[AacSpectralSection],
    quantized: &[i32],
    band_width: usize,
    scale_factor_bits: PackedBits,
    pair_tables: AacSpectralMagnitudeTables<'_>,
    signed_pair_tables: AacSpectralTables<'_>,
    signed_quad_tables: AacSpectralQuadTables<'_>,
    quad_tables: AacSpectralMagnitudeQuadTables<'_>,
) -> Result<PackedBits, Error> {
    let section_bits = pack_spectral_section_data_with_len(sections, band_width)?;
    let spectral_bits = pack_spectral_sections_by_codebook_id_with_signed_pairs(
        sections,
        quantized,
        pair_tables,
        signed_pair_tables,
        signed_quad_tables,
        quad_tables,
    )?;
    pack_channel_payload_parts(section_bits, scale_factor_bits, spectral_bits)
}

/// Packs standard id-based AAC sections, scale-factor bits, and spectral
/// payloads using scale-factor band offsets.
pub fn pack_sectioned_spectral_payload_by_codebook_id_with_offsets_and_sign_bits_and_scale_factor_bits(
    sections: &[AacSpectralSection],
    quantized: &[i32],
    offsets: &[usize],
    scale_factor_bits: PackedBits,
    pair_tables: AacSpectralMagnitudeTables<'_>,
    quad_tables: AacSpectralMagnitudeQuadTables<'_>,
) -> Result<PackedBits, Error> {
    pack_sectioned_spectral_payload_by_codebook_id_with_offsets_and_signed_pairs_and_scale_factor_bits(
        sections,
        quantized,
        offsets,
        scale_factor_bits,
        pair_tables,
        AacSpectralTables::default(),
        AacSpectralQuadTables::default(),
        quad_tables,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn pack_sectioned_spectral_payload_by_codebook_id_with_offsets_and_signed_pairs_and_scale_factor_bits(
    sections: &[AacSpectralSection],
    quantized: &[i32],
    offsets: &[usize],
    scale_factor_bits: PackedBits,
    pair_tables: AacSpectralMagnitudeTables<'_>,
    signed_pair_tables: AacSpectralTables<'_>,
    signed_quad_tables: AacSpectralQuadTables<'_>,
    quad_tables: AacSpectralMagnitudeQuadTables<'_>,
) -> Result<PackedBits, Error> {
    let section_bits = pack_spectral_section_data_with_offsets(sections, offsets)?;
    let spectral_bits = pack_spectral_sections_by_codebook_id_with_signed_pairs(
        sections,
        quantized,
        pair_tables,
        signed_pair_tables,
        signed_quad_tables,
        quad_tables,
    )?;
    pack_channel_payload_parts(section_bits, scale_factor_bits, spectral_bits)
}

/// Builds separated standard id-based payload parts including scale-factor bits.
pub fn split_sectioned_spectral_payload_by_codebook_id_with_sign_bits_and_scale_factor_bits(
    sections: &[AacSpectralSection],
    quantized: &[i32],
    band_width: usize,
    scale_factor_bits: PackedBits,
    pair_tables: AacSpectralMagnitudeTables<'_>,
    quad_tables: AacSpectralMagnitudeQuadTables<'_>,
) -> Result<AacIndividualChannelPayload, Error> {
    split_sectioned_spectral_payload_by_codebook_id_with_signed_pairs_and_scale_factor_bits(
        sections,
        quantized,
        band_width,
        scale_factor_bits,
        pair_tables,
        AacSpectralTables::default(),
        AacSpectralQuadTables::default(),
        quad_tables,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn split_sectioned_spectral_payload_by_codebook_id_with_signed_pairs_and_scale_factor_bits(
    sections: &[AacSpectralSection],
    quantized: &[i32],
    band_width: usize,
    scale_factor_bits: PackedBits,
    pair_tables: AacSpectralMagnitudeTables<'_>,
    signed_pair_tables: AacSpectralTables<'_>,
    signed_quad_tables: AacSpectralQuadTables<'_>,
    quad_tables: AacSpectralMagnitudeQuadTables<'_>,
) -> Result<AacIndividualChannelPayload, Error> {
    let section_bits = pack_spectral_section_data_with_len(sections, band_width)?;
    let spectral_bits = pack_spectral_sections_by_codebook_id_with_signed_pairs(
        sections,
        quantized,
        pair_tables,
        signed_pair_tables,
        signed_quad_tables,
        quad_tables,
    )?;
    split_channel_payload_parts(section_bits, scale_factor_bits, spectral_bits)
}

/// Builds separated standard id-based payload parts including scale-factor bits
/// using scale-factor band offsets.
pub fn split_sectioned_spectral_payload_by_codebook_id_with_offsets_and_sign_bits_and_scale_factor_bits(
    sections: &[AacSpectralSection],
    quantized: &[i32],
    offsets: &[usize],
    scale_factor_bits: PackedBits,
    pair_tables: AacSpectralMagnitudeTables<'_>,
    quad_tables: AacSpectralMagnitudeQuadTables<'_>,
) -> Result<AacIndividualChannelPayload, Error> {
    split_sectioned_spectral_payload_by_codebook_id_with_offsets_and_signed_pairs_and_scale_factor_bits(
        sections,
        quantized,
        offsets,
        scale_factor_bits,
        pair_tables,
        AacSpectralTables::default(),
        AacSpectralQuadTables::default(),
        quad_tables,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn split_sectioned_spectral_payload_by_codebook_id_with_offsets_and_signed_pairs_and_scale_factor_bits(
    sections: &[AacSpectralSection],
    quantized: &[i32],
    offsets: &[usize],
    scale_factor_bits: PackedBits,
    pair_tables: AacSpectralMagnitudeTables<'_>,
    signed_pair_tables: AacSpectralTables<'_>,
    signed_quad_tables: AacSpectralQuadTables<'_>,
    quad_tables: AacSpectralMagnitudeQuadTables<'_>,
) -> Result<AacIndividualChannelPayload, Error> {
    let section_bits = pack_spectral_section_data_with_offsets(sections, offsets)?;
    let spectral_bits = pack_spectral_sections_by_codebook_id_with_signed_pairs(
        sections,
        quantized,
        pair_tables,
        signed_pair_tables,
        signed_quad_tables,
        quad_tables,
    )?;
    split_channel_payload_parts(section_bits, scale_factor_bits, spectral_bits)
}

/// Plans standard id-based AAC sections by bit cost, then packs metadata and payload.
pub fn pack_sectioned_spectral_payload_by_codebook_id_with_sign_bits_by_bit_cost(
    quantized: &[i32],
    band_width: usize,
    pair_tables: AacSpectralMagnitudeTables<'_>,
    quad_tables: AacSpectralMagnitudeQuadTables<'_>,
) -> Result<PackedBits, Error> {
    let sections =
        plan_spectral_sections_by_bit_cost(quantized, band_width, pair_tables, quad_tables)?;
    pack_sectioned_spectral_payload_by_codebook_id_with_sign_bits(
        &sections,
        quantized,
        band_width,
        pair_tables,
        quad_tables,
    )
}

/// Plans standard id-based AAC sections by bit cost, then packs scale-factor and spectral bits.
pub fn pack_sectioned_spectral_payload_by_codebook_id_with_sign_bits_and_scale_factor_bits_by_bit_cost(
    quantized: &[i32],
    band_width: usize,
    scale_factor_bits: PackedBits,
    pair_tables: AacSpectralMagnitudeTables<'_>,
    quad_tables: AacSpectralMagnitudeQuadTables<'_>,
) -> Result<PackedBits, Error> {
    let sections =
        plan_spectral_sections_by_bit_cost(quantized, band_width, pair_tables, quad_tables)?;
    pack_sectioned_spectral_payload_by_codebook_id_with_sign_bits_and_scale_factor_bits(
        &sections,
        quantized,
        band_width,
        scale_factor_bits,
        pair_tables,
        quad_tables,
    )
}

/// Plans standard id-based AAC sections by bit cost, then returns split ICS payload parts.
pub fn split_sectioned_spectral_payload_by_codebook_id_with_sign_bits_by_bit_cost(
    quantized: &[i32],
    band_width: usize,
    pair_tables: AacSpectralMagnitudeTables<'_>,
    quad_tables: AacSpectralMagnitudeQuadTables<'_>,
) -> Result<AacIndividualChannelPayload, Error> {
    let sections =
        plan_spectral_sections_by_bit_cost(quantized, band_width, pair_tables, quad_tables)?;
    split_sectioned_spectral_payload_by_codebook_id_with_sign_bits(
        &sections,
        quantized,
        band_width,
        pair_tables,
        quad_tables,
    )
}

/// Plans standard id-based AAC sections by bit cost, then returns split ICS parts with scale factors.
pub fn split_sectioned_spectral_payload_by_codebook_id_with_sign_bits_and_scale_factor_bits_by_bit_cost(
    quantized: &[i32],
    band_width: usize,
    scale_factor_bits: PackedBits,
    pair_tables: AacSpectralMagnitudeTables<'_>,
    quad_tables: AacSpectralMagnitudeQuadTables<'_>,
) -> Result<AacIndividualChannelPayload, Error> {
    let sections =
        plan_spectral_sections_by_bit_cost(quantized, band_width, pair_tables, quad_tables)?;
    split_sectioned_spectral_payload_by_codebook_id_with_sign_bits_and_scale_factor_bits(
        &sections,
        quantized,
        band_width,
        scale_factor_bits,
        pair_tables,
        quad_tables,
    )
}

/// Builds separated magnitude-keyed payload parts for a long-block individual_channel_stream.
pub fn split_sectioned_spectral_payload_with_sign_bits(
    sections: &[AacSection],
    quantized: &[i32],
    band_width: usize,
    tables: AacSpectralMagnitudeTables<'_>,
) -> Result<AacIndividualChannelPayload, Error> {
    let section_bits = pack_section_data_with_len(sections, band_width)?;
    let spectral_bits = pack_spectral_sections_with_sign_bits(sections, quantized, tables)?;
    split_channel_payload_parts(
        section_bits,
        PackedBits {
            bytes: Vec::new(),
            bit_len: 0,
        },
        spectral_bits,
    )
}

/// Plans AAC sections by bit cost, then packs metadata followed by spectral payloads.
pub fn pack_sectioned_spectral_payload_with_sign_bits_by_bit_cost(
    quantized: &[i32],
    band_width: usize,
    tables: AacSpectralMagnitudeTables<'_>,
) -> Result<PackedBits, Error> {
    let sections = plan_sections_by_bit_cost(quantized, band_width, tables)?;
    pack_sectioned_spectral_payload_with_sign_bits(&sections, quantized, band_width, tables)
}

/// Packs AAC section metadata, scale-factor bits, and magnitude-keyed spectral payloads.
pub fn pack_sectioned_spectral_payload_with_sign_bits_and_scale_factor_bits(
    sections: &[AacSection],
    quantized: &[i32],
    band_width: usize,
    scale_factor_bits: PackedBits,
    tables: AacSpectralMagnitudeTables<'_>,
) -> Result<PackedBits, Error> {
    let section_bits = pack_section_data_with_len(sections, band_width)?;
    let spectral_bits = pack_spectral_sections_with_sign_bits(sections, quantized, tables)?;
    pack_channel_payload_parts(section_bits, scale_factor_bits, spectral_bits)
}

/// Builds separated magnitude-keyed payload parts with scale-factor bits.
pub fn split_sectioned_spectral_payload_with_sign_bits_and_scale_factor_bits(
    sections: &[AacSection],
    quantized: &[i32],
    band_width: usize,
    scale_factor_bits: PackedBits,
    tables: AacSpectralMagnitudeTables<'_>,
) -> Result<AacIndividualChannelPayload, Error> {
    let section_bits = pack_section_data_with_len(sections, band_width)?;
    let spectral_bits = pack_spectral_sections_with_sign_bits(sections, quantized, tables)?;
    split_channel_payload_parts(section_bits, scale_factor_bits, spectral_bits)
}

pub fn split_sectioned_spectral_payload_with_offsets_and_scale_factor_bits(
    sections: &[AacSection],
    quantized: &[i32],
    offsets: &[usize],
    scale_factor_bits: PackedBits,
    tables: AacSpectralMagnitudeTables<'_>,
) -> Result<AacIndividualChannelPayload, Error> {
    validate_scale_factor_band_offsets(quantized, offsets)?;
    let section_bits = pack_section_data_with_offsets(sections, offsets)?;
    let spectral_bits = pack_spectral_sections_with_sign_bits(sections, quantized, tables)?;
    split_channel_payload_parts(section_bits, scale_factor_bits, spectral_bits)
}

pub(crate) fn split_magnitude_sectioned_spectral_payload_with_offsets_and_scale_factor_bits(
    sections: &[AacMagnitudeSection<'_>],
    quantized: &[i32],
    offsets: &[usize],
    scale_factor_bits: PackedBits,
) -> Result<AacIndividualChannelPayload, Error> {
    validate_scale_factor_band_offsets(quantized, offsets)?;
    let section_bits = pack_magnitude_section_data_with_offsets(sections, offsets)?;
    let spectral_bits = pack_magnitude_spectral_sections_with_sign_bits(sections, quantized)?;
    split_channel_payload_parts(section_bits, scale_factor_bits, spectral_bits)
}

/// Plans AAC sections by bit cost, then packs metadata, scale-factor bits, and spectral payloads.
pub fn pack_sectioned_spectral_payload_with_sign_bits_and_scale_factor_bits_by_bit_cost(
    quantized: &[i32],
    band_width: usize,
    scale_factor_bits: PackedBits,
    tables: AacSpectralMagnitudeTables<'_>,
) -> Result<PackedBits, Error> {
    let sections = plan_sections_by_bit_cost(quantized, band_width, tables)?;
    pack_sectioned_spectral_payload_with_sign_bits_and_scale_factor_bits(
        &sections,
        quantized,
        band_width,
        scale_factor_bits,
        tables,
    )
}
