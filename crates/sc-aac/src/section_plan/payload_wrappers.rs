use super::*;

/// Packs the package-facing AAC-LC standard-id workbench payload.
///
/// This path combines the implemented standard quad, pair, and escape tables.
pub fn pack_aac_lc_standard_spectral_payload_with_sign_bits_by_bit_cost(
    quantized: &[i32],
    band_width: usize,
) -> Result<PackedBits, Error> {
    let sections = plan_aac_lc_standard_spectral_sections_by_bit_cost(quantized, band_width)?;
    pack_sectioned_spectral_payload_by_codebook_id_with_signed_pairs(
        &sections,
        quantized,
        band_width,
        aac_lc_standard_spectral_tables(),
        aac_lc_standard_signed_pair_tables(),
        aac_lc_standard_signed_quad_tables(),
        aac_lc_standard_unsigned_quad_tables(),
    )
}

/// Packs the package-facing AAC-LC standard-id workbench payload using offsets.
pub fn pack_aac_lc_standard_spectral_payload_with_offsets_and_sign_bits_by_bit_cost(
    quantized: &[i32],
    offsets: &[usize],
) -> Result<PackedBits, Error> {
    let sections =
        plan_aac_lc_standard_spectral_sections_by_offsets_by_bit_cost(quantized, offsets)?;
    pack_sectioned_spectral_payload_by_codebook_id_with_offsets_and_signed_pairs(
        &sections,
        quantized,
        offsets,
        aac_lc_standard_spectral_tables(),
        aac_lc_standard_signed_pair_tables(),
        aac_lc_standard_signed_quad_tables(),
        aac_lc_standard_unsigned_quad_tables(),
    )
}

/// Builds split ICS payload parts for the package-facing AAC-LC standard-id
/// workbench using offsets.
pub fn split_aac_lc_standard_spectral_payload_with_offsets_and_sign_bits_by_bit_cost(
    quantized: &[i32],
    offsets: &[usize],
) -> Result<AacIndividualChannelPayload, Error> {
    let sections =
        plan_aac_lc_standard_spectral_sections_by_offsets_by_bit_cost(quantized, offsets)?;
    split_sectioned_spectral_payload_by_codebook_id_with_offsets_and_signed_pairs(
        &sections,
        quantized,
        offsets,
        aac_lc_standard_spectral_tables(),
        aac_lc_standard_signed_pair_tables(),
        aac_lc_standard_signed_quad_tables(),
        aac_lc_standard_unsigned_quad_tables(),
    )
}

/// Builds split ICS payload parts for caller-selected standard AAC-LC sections.
pub fn split_aac_lc_standard_sectioned_spectral_payload_with_offsets_and_sign_bits(
    sections: &[AacSpectralSection],
    quantized: &[i32],
    offsets: &[usize],
) -> Result<AacIndividualChannelPayload, Error> {
    split_sectioned_spectral_payload_by_codebook_id_with_offsets_and_signed_pairs(
        sections,
        quantized,
        offsets,
        aac_lc_standard_spectral_tables(),
        aac_lc_standard_signed_pair_tables(),
        aac_lc_standard_signed_quad_tables(),
        aac_lc_standard_unsigned_quad_tables(),
    )
}

/// Packs the package-facing AAC-LC standard-id workbench payload with offsets
/// and scale-factor bits.
pub fn pack_aac_lc_standard_spectral_payload_with_offsets_and_sign_bits_and_scale_factor_bits_by_bit_cost(
    quantized: &[i32],
    offsets: &[usize],
    scale_factor_bits: PackedBits,
) -> Result<PackedBits, Error> {
    let sections =
        plan_aac_lc_standard_spectral_sections_by_offsets_by_bit_cost(quantized, offsets)?;
    pack_sectioned_spectral_payload_by_codebook_id_with_offsets_and_signed_pairs_and_scale_factor_bits(
        &sections,
        quantized,
        offsets,
        scale_factor_bits,
        aac_lc_standard_spectral_tables(),
        aac_lc_standard_signed_pair_tables(),
        aac_lc_standard_signed_quad_tables(),
        aac_lc_standard_unsigned_quad_tables(),
    )
}

/// Builds split ICS payload parts for the package-facing AAC-LC standard-id
/// workbench with offsets and scale-factor bits.
pub fn split_aac_lc_standard_spectral_payload_with_offsets_and_sign_bits_and_scale_factor_bits_by_bit_cost(
    quantized: &[i32],
    offsets: &[usize],
    scale_factor_bits: PackedBits,
) -> Result<AacIndividualChannelPayload, Error> {
    let sections =
        plan_aac_lc_standard_spectral_sections_by_offsets_by_bit_cost(quantized, offsets)?;
    split_sectioned_spectral_payload_by_codebook_id_with_offsets_and_signed_pairs_and_scale_factor_bits(
        &sections,
        quantized,
        offsets,
        scale_factor_bits,
        aac_lc_standard_spectral_tables(),
        aac_lc_standard_signed_pair_tables(),
        aac_lc_standard_signed_quad_tables(),
        aac_lc_standard_unsigned_quad_tables(),
    )
}

/// Builds split ICS payload parts for the package-facing AAC-LC standard-id workbench.
pub fn split_aac_lc_standard_spectral_payload_with_sign_bits_by_bit_cost(
    quantized: &[i32],
    band_width: usize,
) -> Result<AacIndividualChannelPayload, Error> {
    let sections = plan_aac_lc_standard_spectral_sections_by_bit_cost(quantized, band_width)?;
    split_sectioned_spectral_payload_by_codebook_id_with_signed_pairs(
        &sections,
        quantized,
        band_width,
        aac_lc_standard_spectral_tables(),
        aac_lc_standard_signed_pair_tables(),
        aac_lc_standard_signed_quad_tables(),
        aac_lc_standard_unsigned_quad_tables(),
    )
}

/// Packs the package-facing AAC-LC standard-id workbench payload with scale-factor bits.
pub fn pack_aac_lc_standard_spectral_payload_with_sign_bits_and_scale_factor_bits_by_bit_cost(
    quantized: &[i32],
    band_width: usize,
    scale_factor_bits: PackedBits,
) -> Result<PackedBits, Error> {
    let sections = plan_aac_lc_standard_spectral_sections_by_bit_cost(quantized, band_width)?;
    pack_sectioned_spectral_payload_by_codebook_id_with_signed_pairs_and_scale_factor_bits(
        &sections,
        quantized,
        band_width,
        scale_factor_bits,
        aac_lc_standard_spectral_tables(),
        aac_lc_standard_signed_pair_tables(),
        aac_lc_standard_signed_quad_tables(),
        aac_lc_standard_unsigned_quad_tables(),
    )
}

/// Builds split ICS payload parts for the package-facing AAC-LC standard-id workbench with scale-factor bits.
pub fn split_aac_lc_standard_spectral_payload_with_sign_bits_and_scale_factor_bits_by_bit_cost(
    quantized: &[i32],
    band_width: usize,
    scale_factor_bits: PackedBits,
) -> Result<AacIndividualChannelPayload, Error> {
    let sections = plan_aac_lc_standard_spectral_sections_by_bit_cost(quantized, band_width)?;
    split_sectioned_spectral_payload_by_codebook_id_with_signed_pairs_and_scale_factor_bits(
        &sections,
        quantized,
        band_width,
        scale_factor_bits,
        aac_lc_standard_spectral_tables(),
        aac_lc_standard_signed_pair_tables(),
        aac_lc_standard_signed_quad_tables(),
        aac_lc_standard_unsigned_quad_tables(),
    )
}
