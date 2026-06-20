use super::*;

#[pyfunction]
pub(crate) fn aac_unsigned_pairs7_unit_magnitude_table() -> Vec<u32> {
    sonare_codec_rs::aac_unsigned_pairs7_unit_magnitude_table()
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

#[pyfunction]
pub(crate) fn aac_unsigned_pairs7_table() -> Vec<u32> {
    sonare_codec_rs::aac_unsigned_pairs7_table()
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

#[pyfunction]
pub(crate) fn aac_signed_pairs5_table() -> Vec<i32> {
    sonare_codec_rs::aac_signed_pairs5_table()
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

#[pyfunction]
pub(crate) fn aac_signed_pairs6_table() -> Vec<i32> {
    sonare_codec_rs::aac_signed_pairs6_table()
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

#[pyfunction]
pub(crate) fn aac_unsigned_pairs8_table() -> Vec<u32> {
    sonare_codec_rs::aac_unsigned_pairs8_table()
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

#[pyfunction]
pub(crate) fn aac_unsigned_pairs9_table() -> Vec<u32> {
    sonare_codec_rs::aac_unsigned_pairs9_table()
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

#[pyfunction]
pub(crate) fn aac_unsigned_pairs10_table() -> Vec<u32> {
    sonare_codec_rs::aac_unsigned_pairs10_table()
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

#[pyfunction]
pub(crate) fn aac_signed_quads1_table() -> Vec<i32> {
    sonare_codec_rs::aac_signed_quads1_table()
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

#[pyfunction]
pub(crate) fn aac_signed_quads2_table() -> Vec<i32> {
    sonare_codec_rs::aac_signed_quads2_table()
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

#[pyfunction]
pub(crate) fn aac_unsigned_quads3_table() -> Vec<u32> {
    sonare_codec_rs::aac_unsigned_quads3_table()
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

#[pyfunction]
pub(crate) fn aac_unsigned_quads4_table() -> Vec<u32> {
    sonare_codec_rs::aac_unsigned_quads4_table()
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

#[pyfunction]
pub(crate) fn aac_escape_table() -> Vec<u32> {
    sonare_codec_rs::aac_escape_table()
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

#[pyfunction]
pub(crate) fn aac_scale_factor_delta_table() -> Vec<i32> {
    sonare_codec_rs::aac_scale_factor_delta_table()
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

#[pyfunction]
pub(crate) fn aac_codebook6_unit_section_plan(
    quantized: Vec<i32>,
    band_width: usize,
) -> PyResult<Vec<u32>> {
    let sections = sonare_codec_rs::plan_sections_by_bit_cost(
        &quantized,
        band_width,
        sonare_codec_rs::aac_unit_codebook6_spectral_tables(),
    )
    .map_err(to_py_value_error)?;

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

#[pyfunction]
pub(crate) fn aac_quad_unit_section_plan(
    quantized: Vec<i32>,
    band_width: usize,
) -> PyResult<Vec<u32>> {
    let sections = sonare_codec_rs::plan_quad_sections_by_bit_cost(
        &quantized,
        band_width,
        sonare_codec_rs::aac_unit_quad_spectral_tables(),
    )
    .map_err(to_py_value_error)?;

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

#[pyfunction]
pub(crate) fn aac_mixed_unit_section_plan(
    quantized: Vec<i32>,
    band_width: usize,
) -> PyResult<Vec<u32>> {
    let sections = sonare_codec_rs::plan_spectral_sections_by_bit_cost(
        &quantized,
        band_width,
        sonare_codec_rs::aac_unit_codebook6_spectral_tables(),
        sonare_codec_rs::aac_unit_quad_spectral_tables(),
    )
    .map_err(to_py_value_error)?;

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

#[pyfunction]
pub(crate) fn aac_mixed_unit_payload_bit_lengths(
    quantized: Vec<i32>,
    band_width: usize,
) -> PyResult<Vec<u32>> {
    let pair_tables = sonare_codec_rs::aac_unit_codebook6_spectral_tables();
    let quad_tables = sonare_codec_rs::aac_unit_quad_spectral_tables();
    let sections = sonare_codec_rs::plan_spectral_sections_by_bit_cost(
        &quantized,
        band_width,
        pair_tables,
        quad_tables,
    )
    .map_err(to_py_value_error)?;
    let split = sonare_codec_rs::split_sectioned_spectral_payload_by_codebook_id_with_sign_bits(
        &sections,
        &quantized,
        band_width,
        pair_tables,
        quad_tables,
    )
    .map_err(to_py_value_error)?;
    let packed = sonare_codec_rs::pack_sectioned_spectral_payload_by_codebook_id_with_sign_bits(
        &sections,
        &quantized,
        band_width,
        pair_tables,
        quad_tables,
    )
    .map_err(to_py_value_error)?;
    let scale_factor_bits = sonare_codec_rs::PackedBits {
        bytes: vec![0b1100_0000],
        bit_len: 2,
    };
    let split_with_scale =
        sonare_codec_rs::split_sectioned_spectral_payload_by_codebook_id_with_sign_bits_and_scale_factor_bits(
            &sections,
            &quantized,
            band_width,
            scale_factor_bits.clone(),
            pair_tables,
            quad_tables,
        )
        .map_err(to_py_value_error)?;
    let packed_with_scale =
        sonare_codec_rs::pack_sectioned_spectral_payload_by_codebook_id_with_sign_bits_and_scale_factor_bits(
            &sections,
            &quantized,
            band_width,
            scale_factor_bits,
            pair_tables,
            quad_tables,
        )
        .map_err(to_py_value_error)?;

    Ok(vec![
        split.section_and_scale_factor_bits.bit_len as u32,
        split.spectral_bits.bit_len as u32,
        packed.bit_len as u32,
        split_with_scale.section_and_scale_factor_bits.bit_len as u32,
        split_with_scale.spectral_bits.bit_len as u32,
        packed_with_scale.bit_len as u32,
    ])
}

#[pyfunction]
pub(crate) fn aac_standard_unit_section_plan(
    quantized: Vec<i32>,
    band_width: usize,
) -> PyResult<Vec<u32>> {
    let sections =
        sonare_codec_rs::plan_aac_lc_standard_spectral_sections_by_bit_cost(&quantized, band_width)
            .map_err(to_py_value_error)?;

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

#[pyfunction]
pub(crate) fn aac_standard_offsets_section_plan(
    quantized: Vec<i32>,
    offsets: Vec<usize>,
) -> PyResult<Vec<u32>> {
    let sections = sonare_codec_rs::plan_aac_lc_standard_spectral_sections_by_offsets_by_bit_cost(
        &quantized, &offsets,
    )
    .map_err(to_py_value_error)?;

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

#[pyfunction]
pub(crate) fn aac_standard_escape_payload_bit_lengths() -> PyResult<Vec<u32>> {
    let quantized = [17, 0];
    let band_width = 2;
    let pair_tables = sonare_codec_rs::aac_lc_standard_spectral_tables();
    let quad_tables = sonare_codec_rs::AacSpectralMagnitudeQuadTables::default();
    let sections = sonare_codec_rs::plan_spectral_sections_by_bit_cost(
        &quantized,
        band_width,
        pair_tables,
        quad_tables,
    )
    .map_err(to_py_value_error)?;
    if sections.first().map(|section| section.codebook_id)
        != Some(sonare_codec_rs::AacCodebook::Escape.id())
    {
        return Err(pyo3::exceptions::PyValueError::new_err(
            "AAC standard escape fixture did not select codebook 11",
        ));
    }
    let split = sonare_codec_rs::split_sectioned_spectral_payload_by_codebook_id_with_sign_bits(
        &sections,
        &quantized,
        band_width,
        pair_tables,
        quad_tables,
    )
    .map_err(to_py_value_error)?;
    let packed = sonare_codec_rs::pack_sectioned_spectral_payload_by_codebook_id_with_sign_bits(
        &sections,
        &quantized,
        band_width,
        pair_tables,
        quad_tables,
    )
    .map_err(to_py_value_error)?;

    Ok(vec![
        split.section_and_scale_factor_bits.bit_len as u32,
        split.spectral_bits.bit_len as u32,
        packed.bit_len as u32,
    ])
}

#[pyfunction]
pub(crate) fn aac_standard_mixed_payload_bit_lengths(
    quantized: Vec<i32>,
    band_width: usize,
) -> PyResult<Vec<u32>> {
    let split = sonare_codec_rs::split_aac_lc_standard_spectral_payload_with_sign_bits_by_bit_cost(
        &quantized, band_width,
    )
    .map_err(to_py_value_error)?;
    let packed = sonare_codec_rs::pack_aac_lc_standard_spectral_payload_with_sign_bits_by_bit_cost(
        &quantized, band_width,
    )
    .map_err(to_py_value_error)?;
    let scale_factor_bits = sonare_codec_rs::PackedBits {
        bytes: vec![0b1100_0000],
        bit_len: 2,
    };
    let split_with_scale =
        sonare_codec_rs::split_aac_lc_standard_spectral_payload_with_sign_bits_and_scale_factor_bits_by_bit_cost(
            &quantized,
            band_width,
            scale_factor_bits.clone(),
        )
        .map_err(to_py_value_error)?;
    let packed_with_scale =
        sonare_codec_rs::pack_aac_lc_standard_spectral_payload_with_sign_bits_and_scale_factor_bits_by_bit_cost(
            &quantized,
            band_width,
            scale_factor_bits,
        )
        .map_err(to_py_value_error)?;

    Ok(vec![
        split.section_and_scale_factor_bits.bit_len as u32,
        split.spectral_bits.bit_len as u32,
        packed.bit_len as u32,
        split_with_scale.section_and_scale_factor_bits.bit_len as u32,
        split_with_scale.spectral_bits.bit_len as u32,
        packed_with_scale.bit_len as u32,
    ])
}

#[pyfunction]
pub(crate) fn aac_standard_mixed_offsets_payload_bit_lengths(
    quantized: Vec<i32>,
    offsets: Vec<usize>,
) -> PyResult<Vec<u32>> {
    let split =
        sonare_codec_rs::split_aac_lc_standard_spectral_payload_with_offsets_and_sign_bits_by_bit_cost(
            &quantized,
            &offsets,
        )
        .map_err(to_py_value_error)?;
    let packed =
        sonare_codec_rs::pack_aac_lc_standard_spectral_payload_with_offsets_and_sign_bits_by_bit_cost(
            &quantized,
            &offsets,
        )
        .map_err(to_py_value_error)?;
    let scale_factor_bits = sonare_codec_rs::PackedBits {
        bytes: vec![0b1100_0000],
        bit_len: 2,
    };
    let split_with_scale =
        sonare_codec_rs::split_aac_lc_standard_spectral_payload_with_offsets_and_sign_bits_and_scale_factor_bits_by_bit_cost(
            &quantized,
            &offsets,
            scale_factor_bits.clone(),
        )
        .map_err(to_py_value_error)?;
    let packed_with_scale =
        sonare_codec_rs::pack_aac_lc_standard_spectral_payload_with_offsets_and_sign_bits_and_scale_factor_bits_by_bit_cost(
            &quantized,
            &offsets,
            scale_factor_bits,
        )
        .map_err(to_py_value_error)?;

    Ok(vec![
        split.section_and_scale_factor_bits.bit_len as u32,
        split.spectral_bits.bit_len as u32,
        packed.bit_len as u32,
        split_with_scale.section_and_scale_factor_bits.bit_len as u32,
        split_with_scale.spectral_bits.bit_len as u32,
        packed_with_scale.bit_len as u32,
    ])
}

pub(crate) fn add_py_functions(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(
        aac_unsigned_pairs7_unit_magnitude_table,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(aac_unsigned_pairs7_table, module)?)?;
    module.add_function(wrap_pyfunction!(aac_signed_pairs5_table, module)?)?;
    module.add_function(wrap_pyfunction!(aac_signed_pairs6_table, module)?)?;
    module.add_function(wrap_pyfunction!(aac_unsigned_pairs8_table, module)?)?;
    module.add_function(wrap_pyfunction!(aac_unsigned_pairs9_table, module)?)?;
    module.add_function(wrap_pyfunction!(aac_unsigned_pairs10_table, module)?)?;
    module.add_function(wrap_pyfunction!(aac_signed_quads1_table, module)?)?;
    module.add_function(wrap_pyfunction!(aac_signed_quads2_table, module)?)?;
    module.add_function(wrap_pyfunction!(aac_unsigned_quads3_table, module)?)?;
    module.add_function(wrap_pyfunction!(aac_unsigned_quads4_table, module)?)?;
    module.add_function(wrap_pyfunction!(aac_escape_table, module)?)?;
    module.add_function(wrap_pyfunction!(aac_scale_factor_delta_table, module)?)?;
    module.add_function(wrap_pyfunction!(aac_codebook6_unit_section_plan, module)?)?;
    module.add_function(wrap_pyfunction!(aac_quad_unit_section_plan, module)?)?;
    module.add_function(wrap_pyfunction!(aac_mixed_unit_section_plan, module)?)?;
    module.add_function(wrap_pyfunction!(
        aac_mixed_unit_payload_bit_lengths,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(aac_standard_unit_section_plan, module)?)?;
    module.add_function(wrap_pyfunction!(aac_standard_offsets_section_plan, module)?)?;
    module.add_function(wrap_pyfunction!(
        aac_standard_escape_payload_bit_lengths,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(
        aac_standard_mixed_payload_bit_lengths,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(
        aac_standard_mixed_offsets_payload_bit_lengths,
        module
    )?)?;
    Ok(())
}
