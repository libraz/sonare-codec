use super::*;

#[wasm_bindgen]
pub fn aac_unsigned_pairs7_unit_magnitude_table() -> Vec<u32> {
    sonare_codec::aac_unsigned_pairs7_unit_magnitude_table()
        .iter()
        .flat_map(|entry| {
            [
                u32::from(entry.symbol.x),
                u32::from(entry.symbol.y),
                entry.code.bits,
                u32::from(entry.code.len),
            ]
        })
        .collect()
}

#[wasm_bindgen]
pub fn aac_unsigned_pairs7_table() -> Vec<u32> {
    sonare_codec::aac_unsigned_pairs7_table()
        .iter()
        .flat_map(|entry| {
            [
                u32::from(entry.symbol.x),
                u32::from(entry.symbol.y),
                entry.code.bits,
                u32::from(entry.code.len),
            ]
        })
        .collect()
}

#[wasm_bindgen]
pub fn aac_signed_pairs5_table() -> Vec<i32> {
    sonare_codec::aac_signed_pairs5_table()
        .iter()
        .flat_map(|entry| {
            [
                i32::from(entry.symbol.x),
                i32::from(entry.symbol.y),
                entry.code.bits as i32,
                i32::from(entry.code.len),
            ]
        })
        .collect()
}

#[wasm_bindgen]
pub fn aac_signed_pairs6_table() -> Vec<i32> {
    sonare_codec::aac_signed_pairs6_table()
        .iter()
        .flat_map(|entry| {
            [
                i32::from(entry.symbol.x),
                i32::from(entry.symbol.y),
                entry.code.bits as i32,
                i32::from(entry.code.len),
            ]
        })
        .collect()
}

#[wasm_bindgen]
pub fn aac_unsigned_pairs8_table() -> Vec<u32> {
    sonare_codec::aac_unsigned_pairs8_table()
        .iter()
        .flat_map(|entry| {
            [
                u32::from(entry.symbol.x),
                u32::from(entry.symbol.y),
                entry.code.bits,
                u32::from(entry.code.len),
            ]
        })
        .collect()
}

#[wasm_bindgen]
pub fn aac_unsigned_pairs9_table() -> Vec<u32> {
    sonare_codec::aac_unsigned_pairs9_table()
        .iter()
        .flat_map(|entry| {
            [
                u32::from(entry.symbol.x),
                u32::from(entry.symbol.y),
                entry.code.bits,
                u32::from(entry.code.len),
            ]
        })
        .collect()
}

#[wasm_bindgen]
pub fn aac_unsigned_pairs10_table() -> Vec<u32> {
    sonare_codec::aac_unsigned_pairs10_table()
        .iter()
        .flat_map(|entry| {
            [
                u32::from(entry.symbol.x),
                u32::from(entry.symbol.y),
                entry.code.bits,
                u32::from(entry.code.len),
            ]
        })
        .collect()
}

#[wasm_bindgen]
pub fn aac_signed_quads1_table() -> Vec<i32> {
    sonare_codec::aac_signed_quads1_table()
        .iter()
        .flat_map(|entry| {
            [
                i32::from(entry.symbol.v),
                i32::from(entry.symbol.w),
                i32::from(entry.symbol.x),
                i32::from(entry.symbol.y),
                i32::try_from(entry.code.bits).unwrap_or(i32::MAX),
                i32::from(entry.code.len),
            ]
        })
        .collect()
}

#[wasm_bindgen]
pub fn aac_signed_quads2_table() -> Vec<i32> {
    sonare_codec::aac_signed_quads2_table()
        .iter()
        .flat_map(|entry| {
            [
                i32::from(entry.symbol.v),
                i32::from(entry.symbol.w),
                i32::from(entry.symbol.x),
                i32::from(entry.symbol.y),
                i32::try_from(entry.code.bits).unwrap_or(i32::MAX),
                i32::from(entry.code.len),
            ]
        })
        .collect()
}

#[wasm_bindgen]
pub fn aac_unsigned_quads3_table() -> Vec<u32> {
    sonare_codec::aac_unsigned_quads3_table()
        .iter()
        .flat_map(|entry| {
            [
                u32::from(entry.symbol.v),
                u32::from(entry.symbol.w),
                u32::from(entry.symbol.x),
                u32::from(entry.symbol.y),
                entry.code.bits,
                u32::from(entry.code.len),
            ]
        })
        .collect()
}

#[wasm_bindgen]
pub fn aac_unsigned_quads4_table() -> Vec<u32> {
    sonare_codec::aac_unsigned_quads4_table()
        .iter()
        .flat_map(|entry| {
            [
                u32::from(entry.symbol.v),
                u32::from(entry.symbol.w),
                u32::from(entry.symbol.x),
                u32::from(entry.symbol.y),
                entry.code.bits,
                u32::from(entry.code.len),
            ]
        })
        .collect()
}

#[wasm_bindgen]
pub fn aac_escape_table() -> Vec<u32> {
    sonare_codec::aac_escape_table()
        .iter()
        .flat_map(|entry| {
            [
                u32::from(entry.symbol.x),
                u32::from(entry.symbol.y),
                entry.code.bits,
                u32::from(entry.code.len),
            ]
        })
        .collect()
}

#[wasm_bindgen]
pub fn aac_scale_factor_delta_table() -> Vec<i32> {
    sonare_codec::aac_scale_factor_delta_table()
        .iter()
        .flat_map(|entry| {
            [
                i32::from(entry.symbol.delta),
                i32::try_from(entry.code.bits).unwrap_or(i32::MAX),
                i32::from(entry.code.len),
            ]
        })
        .collect()
}

#[wasm_bindgen]
pub fn aac_codebook6_unit_section_plan(
    quantized: &[i32],
    band_width: usize,
) -> Result<Vec<u32>, String> {
    let sections = sonare_codec::plan_sections_by_bit_cost(
        quantized,
        band_width,
        sonare_codec::aac_unit_codebook6_spectral_tables(),
    )
    .map_err(|err| err.to_string())?;

    Ok(sections
        .iter()
        .flat_map(|section| {
            [
                section.start as u32,
                section.end as u32,
                u32::from(section.codebook.id()),
            ]
        })
        .collect())
}

#[wasm_bindgen]
pub fn aac_quad_unit_section_plan(
    quantized: &[i32],
    band_width: usize,
) -> Result<Vec<u32>, String> {
    let sections = sonare_codec::plan_quad_sections_by_bit_cost(
        quantized,
        band_width,
        sonare_codec::aac_unit_quad_spectral_tables(),
    )
    .map_err(|err| err.to_string())?;

    Ok(sections
        .iter()
        .flat_map(|section| {
            [
                section.start as u32,
                section.end as u32,
                u32::from(section.codebook_id),
            ]
        })
        .collect())
}

#[wasm_bindgen]
pub fn aac_mixed_unit_section_plan(
    quantized: &[i32],
    band_width: usize,
) -> Result<Vec<u32>, String> {
    let sections = sonare_codec::plan_spectral_sections_by_bit_cost(
        quantized,
        band_width,
        sonare_codec::aac_unit_codebook6_spectral_tables(),
        sonare_codec::aac_unit_quad_spectral_tables(),
    )
    .map_err(|err| err.to_string())?;

    Ok(sections
        .iter()
        .flat_map(|section| {
            [
                section.start as u32,
                section.end as u32,
                u32::from(section.codebook_id),
            ]
        })
        .collect())
}

#[wasm_bindgen]
pub fn aac_mixed_unit_payload_bit_lengths(
    quantized: &[i32],
    band_width: usize,
) -> Result<Vec<u32>, String> {
    let pair_tables = sonare_codec::aac_unit_codebook6_spectral_tables();
    let quad_tables = sonare_codec::aac_unit_quad_spectral_tables();
    let sections = sonare_codec::plan_spectral_sections_by_bit_cost(
        quantized,
        band_width,
        pair_tables,
        quad_tables,
    )
    .map_err(|err| err.to_string())?;
    let split = sonare_codec::split_sectioned_spectral_payload_by_codebook_id_with_sign_bits(
        &sections,
        quantized,
        band_width,
        pair_tables,
        quad_tables,
    )
    .map_err(|err| err.to_string())?;
    let packed = sonare_codec::pack_sectioned_spectral_payload_by_codebook_id_with_sign_bits(
        &sections,
        quantized,
        band_width,
        pair_tables,
        quad_tables,
    )
    .map_err(|err| err.to_string())?;
    let scale_factor_bits = sonare_codec::PackedBits {
        bytes: vec![0b1100_0000],
        bit_len: 2,
    };
    let split_with_scale =
        sonare_codec::split_sectioned_spectral_payload_by_codebook_id_with_sign_bits_and_scale_factor_bits(
            &sections,
            quantized,
            band_width,
            scale_factor_bits.clone(),
            pair_tables,
            quad_tables,
        )
        .map_err(|err| err.to_string())?;
    let packed_with_scale =
        sonare_codec::pack_sectioned_spectral_payload_by_codebook_id_with_sign_bits_and_scale_factor_bits(
            &sections,
            quantized,
            band_width,
            scale_factor_bits,
            pair_tables,
            quad_tables,
        )
        .map_err(|err| err.to_string())?;

    Ok(vec![
        split.section_and_scale_factor_bits.bit_len as u32,
        split.spectral_bits.bit_len as u32,
        packed.bit_len as u32,
        split_with_scale.section_and_scale_factor_bits.bit_len as u32,
        split_with_scale.spectral_bits.bit_len as u32,
        packed_with_scale.bit_len as u32,
    ])
}

#[wasm_bindgen]
pub fn aac_standard_unit_section_plan(
    quantized: &[i32],
    band_width: usize,
) -> Result<Vec<u32>, String> {
    let sections =
        sonare_codec::plan_aac_lc_standard_spectral_sections_by_bit_cost(quantized, band_width)
            .map_err(|err| err.to_string())?;

    Ok(sections
        .iter()
        .flat_map(|section| {
            [
                section.start as u32,
                section.end as u32,
                u32::from(section.codebook_id),
            ]
        })
        .collect())
}

#[wasm_bindgen]
pub fn aac_standard_offsets_section_plan(
    quantized: &[i32],
    offsets: &[u32],
) -> Result<Vec<u32>, String> {
    let offsets = wasm_offsets_to_usize(offsets)?;
    let sections = sonare_codec::plan_aac_lc_standard_spectral_sections_by_offsets_by_bit_cost(
        quantized, &offsets,
    )
    .map_err(|err| err.to_string())?;

    Ok(sections
        .iter()
        .flat_map(|section| {
            [
                section.start as u32,
                section.end as u32,
                u32::from(section.codebook_id),
            ]
        })
        .collect())
}

#[wasm_bindgen]
pub fn aac_standard_escape_payload_bit_lengths() -> Result<Vec<u32>, String> {
    let quantized = [17, 0];
    let band_width = 2;
    let pair_tables = sonare_codec::aac_lc_standard_spectral_tables();
    let quad_tables = sonare_codec::AacSpectralMagnitudeQuadTables::default();
    let sections = sonare_codec::plan_spectral_sections_by_bit_cost(
        &quantized,
        band_width,
        pair_tables,
        quad_tables,
    )
    .map_err(|err| err.to_string())?;
    if sections.first().map(|section| section.codebook_id)
        != Some(sonare_codec::AacCodebook::Escape.id())
    {
        return Err("AAC standard escape fixture did not select codebook 11".to_owned());
    }
    let split = sonare_codec::split_sectioned_spectral_payload_by_codebook_id_with_sign_bits(
        &sections,
        &quantized,
        band_width,
        pair_tables,
        quad_tables,
    )
    .map_err(|err| err.to_string())?;
    let packed = sonare_codec::pack_sectioned_spectral_payload_by_codebook_id_with_sign_bits(
        &sections,
        &quantized,
        band_width,
        pair_tables,
        quad_tables,
    )
    .map_err(|err| err.to_string())?;

    Ok(vec![
        split.section_and_scale_factor_bits.bit_len as u32,
        split.spectral_bits.bit_len as u32,
        packed.bit_len as u32,
    ])
}

#[wasm_bindgen]
pub fn aac_standard_mixed_payload_bit_lengths(
    quantized: &[i32],
    band_width: usize,
) -> Result<Vec<u32>, String> {
    let split = sonare_codec::split_aac_lc_standard_spectral_payload_with_sign_bits_by_bit_cost(
        quantized, band_width,
    )
    .map_err(|err| err.to_string())?;
    let packed = sonare_codec::pack_aac_lc_standard_spectral_payload_with_sign_bits_by_bit_cost(
        quantized, band_width,
    )
    .map_err(|err| err.to_string())?;
    let scale_factor_bits = sonare_codec::PackedBits {
        bytes: vec![0b1100_0000],
        bit_len: 2,
    };
    let split_with_scale =
        sonare_codec::split_aac_lc_standard_spectral_payload_with_sign_bits_and_scale_factor_bits_by_bit_cost(
            quantized,
            band_width,
            scale_factor_bits.clone(),
        )
        .map_err(|err| err.to_string())?;
    let packed_with_scale =
        sonare_codec::pack_aac_lc_standard_spectral_payload_with_sign_bits_and_scale_factor_bits_by_bit_cost(
            quantized,
            band_width,
            scale_factor_bits,
        )
        .map_err(|err| err.to_string())?;

    Ok(vec![
        split.section_and_scale_factor_bits.bit_len as u32,
        split.spectral_bits.bit_len as u32,
        packed.bit_len as u32,
        split_with_scale.section_and_scale_factor_bits.bit_len as u32,
        split_with_scale.spectral_bits.bit_len as u32,
        packed_with_scale.bit_len as u32,
    ])
}

#[wasm_bindgen]
pub fn aac_standard_mixed_offsets_payload_bit_lengths(
    quantized: &[i32],
    offsets: &[u32],
) -> Result<Vec<u32>, String> {
    let offsets = wasm_offsets_to_usize(offsets)?;
    let split =
        sonare_codec::split_aac_lc_standard_spectral_payload_with_offsets_and_sign_bits_by_bit_cost(
            quantized,
            &offsets,
        )
        .map_err(|err| err.to_string())?;
    let packed =
        sonare_codec::pack_aac_lc_standard_spectral_payload_with_offsets_and_sign_bits_by_bit_cost(
            quantized, &offsets,
        )
        .map_err(|err| err.to_string())?;
    let scale_factor_bits = sonare_codec::PackedBits {
        bytes: vec![0b1100_0000],
        bit_len: 2,
    };
    let split_with_scale =
        sonare_codec::split_aac_lc_standard_spectral_payload_with_offsets_and_sign_bits_and_scale_factor_bits_by_bit_cost(
            quantized,
            &offsets,
            scale_factor_bits.clone(),
        )
        .map_err(|err| err.to_string())?;
    let packed_with_scale =
        sonare_codec::pack_aac_lc_standard_spectral_payload_with_offsets_and_sign_bits_and_scale_factor_bits_by_bit_cost(
            quantized,
            &offsets,
            scale_factor_bits,
        )
        .map_err(|err| err.to_string())?;

    Ok(vec![
        split.section_and_scale_factor_bits.bit_len as u32,
        split.spectral_bits.bit_len as u32,
        packed.bit_len as u32,
        split_with_scale.section_and_scale_factor_bits.bit_len as u32,
        split_with_scale.spectral_bits.bit_len as u32,
        packed_with_scale.bit_len as u32,
    ])
}
