use super::*;

pub fn big_value_pairs(
    quantized: &[i32],
    regions: Layer3SpectralRegions,
) -> Result<Vec<Layer3BigValuePair>, Error> {
    let coeff_count = usize::from(regions.big_values)
        .checked_mul(2)
        .ok_or(Error::InvalidInput("MP3 big-values region is too large"))?;
    if coeff_count > quantized.len() {
        return Err(Error::InvalidInput(
            "MP3 big-values region exceeds spectrum length",
        ));
    }

    quantized[..coeff_count]
        .chunks_exact(2)
        .map(|pair| {
            Ok(Layer3BigValuePair::new(
                i16::try_from(pair[0]).map_err(|_| {
                    Error::InvalidInput("MP3 big-value coefficient exceeds i16 range")
                })?,
                i16::try_from(pair[1]).map_err(|_| {
                    Error::InvalidInput("MP3 big-value coefficient exceeds i16 range")
                })?,
            ))
        })
        .collect()
}

/// Maps an escape-class magnitude to the ISO `table_select` (16..=23) whose
/// fixed `linbits` covers it.
///
/// Big-values tables 16 through 23 share the table-16 Huffman codeword tree and
/// differ only in their fixed `linbits` widths (1, 2, 3, 4, 6, 8, 10, 13 in
/// ISO/IEC 11172-3 Annex B). The decoder derives `linbits` from `table_select`,
/// so the encoder must emit the table whose fixed width matches — not a free
/// `linbits` paired with `table_select` 16. This picks the smallest such table
/// that still represents `max_magnitude`.
pub(crate) fn escape_table_select_for_magnitude(max_magnitude: u16) -> Result<(u8, u8), Error> {
    const ESCAPE_TABLES: [(u8, u8); 8] = [
        (16, 1),
        (17, 2),
        (18, 3),
        (19, 4),
        (20, 6),
        (21, 8),
        (22, 10),
        (23, 13),
    ];
    let required = linbits_for_big_value_magnitude(max_magnitude)?;
    ESCAPE_TABLES
        .into_iter()
        .find(|&(_, linbits)| linbits >= required)
        .ok_or(Error::InvalidInput(
            "MP3 big-values magnitude exceeds table range",
        ))
}

pub(crate) fn escape_table_24_select_for_magnitude(max_magnitude: u16) -> Result<(u8, u8), Error> {
    const ESCAPE_TABLES: [(u8, u8); 8] = [
        (24, 4),
        (25, 5),
        (26, 6),
        (27, 7),
        (28, 8),
        (29, 9),
        (30, 11),
        (31, 13),
    ];
    let required = linbits_for_big_value_magnitude(max_magnitude)?;
    ESCAPE_TABLES
        .into_iter()
        .find(|&(_, linbits)| linbits >= required)
        .ok_or(Error::InvalidInput(
            "MP3 big-values magnitude exceeds table range",
        ))
}

/// Selects the smallest implemented Layer III big-values table class.
pub fn select_big_value_table(
    pairs: &[Layer3BigValuePair],
) -> Result<Layer3BigValueTableSelection, Error> {
    let max_magnitude = max_big_value_magnitude(pairs)?;

    let (table_select, linbits) = match max_magnitude {
        0 => (0, 0),
        1 => (1, 0),
        2..=3 => (5, 0),
        4..=5 => (7, 0),
        6..=7 => (10, 0),
        8..=15 => (13, 0),
        _ => escape_table_select_for_magnitude(max_magnitude)?,
    };

    Ok(Layer3BigValueTableSelection {
        table_select,
        linbits,
        max_magnitude,
    })
}

/// Selects the shortest available Layer III big-values table from a provider.
pub fn select_big_value_table_by_bit_cost(
    pairs: &[Layer3BigValuePair],
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<Layer3BigValueTableSelection, Error> {
    let max_magnitude = max_big_value_magnitude(pairs)?;
    if max_magnitude == 0 {
        return Ok(Layer3BigValueTableSelection {
            table_select: 0,
            linbits: 0,
            max_magnitude,
        });
    }

    let (escape_table_select, escape_linbits) = escape_table_select_for_magnitude(max_magnitude)?;
    let (escape_table_24_select, escape_table_24_linbits) =
        escape_table_24_select_for_magnitude(max_magnitude)?;
    let candidates = [
        (1, 0, provider.big_value_table_1),
        (2, 0, provider.big_value_table_2),
        (3, 0, provider.big_value_table_3),
        (5, 0, provider.big_value_table_5),
        (6, 0, provider.big_value_table_6),
        (7, 0, provider.big_value_table_7),
        (8, 0, provider.big_value_table_8),
        (9, 0, provider.big_value_table_9),
        (10, 0, provider.big_value_table_10),
        (11, 0, provider.big_value_table_11),
        (12, 0, provider.big_value_table_12),
        (13, 0, provider.big_value_table_13),
        (15, 0, provider.big_value_table_15),
        (
            escape_table_select,
            escape_linbits,
            provider.big_value_table_16,
        ),
        (
            escape_table_24_select,
            escape_table_24_linbits,
            provider.big_value_table_24,
        ),
    ];
    let mut best: Option<(Layer3BigValueTableSelection, usize)> = None;
    for (table_select, linbits, table) in candidates {
        if table.is_empty() {
            continue;
        }
        let selection = Layer3BigValueTableSelection {
            table_select,
            linbits,
            max_magnitude,
        };
        let Ok(packed) = pack_big_value_pairs_with_selection(pairs, table, selection) else {
            continue;
        };
        if best
            .as_ref()
            .is_none_or(|(_, bit_len)| packed.bit_len < *bit_len)
        {
            best = Some((selection, packed.bit_len));
        }
    }

    best.map(|(selection, _)| selection)
        .ok_or(Error::UnsupportedFeature("MP3 big-values Huffman table"))
}

/// Applies one big-values Huffman table selection to Layer III side info.
pub fn apply_big_value_table_to_granule(
    granule: &mut Layer3GranuleChannelInfo,
    selection: Layer3BigValueTableSelection,
) {
    let table = if granule.big_values == 0 {
        0
    } else {
        selection.table_select
    };
    granule.table_select = [table, table, table];
}

/// Selects Layer III big-values Huffman table classes independently per region.
pub fn select_big_value_region_tables(
    pairs: &[Layer3BigValuePair],
    region0_pairs: usize,
    region1_pairs: usize,
) -> Result<Layer3BigValueRegionTableSelection, Error> {
    let region1_start = region0_pairs;
    let region2_start = region1_start
        .checked_add(region1_pairs)
        .ok_or(Error::InvalidInput("MP3 big-values region is too large"))?;
    if region2_start > pairs.len() {
        return Err(Error::InvalidInput(
            "MP3 big-values region exceeds spectrum length",
        ));
    }

    let region0 = &pairs[..region1_start];
    let region1 = &pairs[region1_start..region2_start];
    let region2 = &pairs[region2_start..];

    Ok(Layer3BigValueRegionTableSelection {
        regions: [
            select_big_value_table(region0)?,
            select_big_value_table(region1)?,
            select_big_value_table(region2)?,
        ],
        region0_pairs: u16::try_from(region0.len())
            .map_err(|_| Error::InvalidInput("MP3 region0 count exceeds side-info range"))?,
        region1_pairs: u16::try_from(region1.len())
            .map_err(|_| Error::InvalidInput("MP3 region1 count exceeds side-info range"))?,
    })
}

/// Selects Layer III big-values Huffman tables independently per region by bit cost.
pub fn select_big_value_region_tables_by_bit_cost(
    pairs: &[Layer3BigValuePair],
    region0_pairs: usize,
    region1_pairs: usize,
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<Layer3BigValueRegionTableSelection, Error> {
    let region1_start = region0_pairs;
    let region2_start = region1_start
        .checked_add(region1_pairs)
        .ok_or(Error::InvalidInput("MP3 big-values region is too large"))?;
    if region2_start > pairs.len() {
        return Err(Error::InvalidInput(
            "MP3 big-values region exceeds spectrum length",
        ));
    }

    let region0 = &pairs[..region1_start];
    let region1 = &pairs[region1_start..region2_start];
    let region2 = &pairs[region2_start..];

    Ok(Layer3BigValueRegionTableSelection {
        regions: [
            select_big_value_table_by_bit_cost(region0, provider)?,
            select_big_value_table_by_bit_cost(region1, provider)?,
            select_big_value_table_by_bit_cost(region2, provider)?,
        ],
        region0_pairs: u16::try_from(region0.len())
            .map_err(|_| Error::InvalidInput("MP3 region0 count exceeds side-info range"))?,
        region1_pairs: u16::try_from(region1.len())
            .map_err(|_| Error::InvalidInput("MP3 region1 count exceeds side-info range"))?,
    })
}

/// Applies region-specific big-values Huffman selections to Layer III side info.
pub fn apply_big_value_region_tables_to_granule(
    granule: &mut Layer3GranuleChannelInfo,
    selection: Layer3BigValueRegionTableSelection,
) -> Result<(), Error> {
    granule.table_select = [
        selection.regions[0].table_select,
        selection.regions[1].table_select,
        selection.regions[2].table_select,
    ];
    // `region0_count`/`region1_count` are the scalefactor-band region addresses
    // (set by `apply_spectral_regions_to_granule`), not pair counts. The pair
    // split carried by `selection` drives the bit packing and must match the
    // decoder's sfb-derived boundaries for those addresses; it is not copied
    // back into the side-info fields here.
    Ok(())
}

/// Packs Layer III big-values regions with provider-selected Huffman tables.
pub fn pack_big_value_pairs_with_region_tables_and_provider(
    pairs: &[Layer3BigValuePair],
    selection: Layer3BigValueRegionTableSelection,
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<PackedBits, Error> {
    let region1_start = usize::from(selection.region0_pairs);
    let region2_start = region1_start
        .checked_add(usize::from(selection.region1_pairs))
        .ok_or(Error::InvalidInput("MP3 big-values region is too large"))?;
    if region2_start > pairs.len() {
        return Err(Error::InvalidInput(
            "MP3 big-values region exceeds spectrum length",
        ));
    }

    let region0 = pack_big_value_pairs_with_selection(
        &pairs[..region1_start],
        provider.big_value_table(selection.regions[0])?,
        selection.regions[0],
    )?;
    let region1 = pack_big_value_pairs_with_selection(
        &pairs[region1_start..region2_start],
        provider.big_value_table(selection.regions[1])?,
        selection.regions[1],
    )?;
    let region2 = pack_big_value_pairs_with_selection(
        &pairs[region2_start..],
        provider.big_value_table(selection.regions[2])?,
        selection.regions[2],
    )?;

    concat_packed_bits(&[region0, region1, region2])
}

/// Converts the Layer III count1 region into quadruple symbols.
pub fn count1_quads(
    quantized: &[i32],
    regions: Layer3SpectralRegions,
) -> Result<Vec<Layer3Count1Quad>, Error> {
    let start = usize::from(regions.big_values)
        .checked_mul(2)
        .ok_or(Error::InvalidInput("MP3 big-values region is too large"))?;
    let coeff_count = usize::from(regions.count1)
        .checked_mul(4)
        .ok_or(Error::InvalidInput("MP3 count1 region is too large"))?;
    let end = start
        .checked_add(coeff_count)
        .ok_or(Error::InvalidInput("MP3 count1 region is too large"))?;
    if end > quantized.len() {
        return Err(Error::InvalidInput(
            "MP3 count1 region exceeds spectrum length",
        ));
    }

    quantized[start..end]
        .chunks_exact(4)
        .map(|quad| {
            for &coeff in quad {
                if coeff.abs() > 1 {
                    return Err(Error::InvalidInput(
                        "MP3 count1 coefficient exceeds unit magnitude",
                    ));
                }
            }
            Ok(Layer3Count1Quad::new(
                i8::try_from(quad[0])
                    .map_err(|_| Error::InvalidInput("MP3 count1 coefficient exceeds i8 range"))?,
                i8::try_from(quad[1])
                    .map_err(|_| Error::InvalidInput("MP3 count1 coefficient exceeds i8 range"))?,
                i8::try_from(quad[2])
                    .map_err(|_| Error::InvalidInput("MP3 count1 coefficient exceeds i8 range"))?,
                i8::try_from(quad[3])
                    .map_err(|_| Error::InvalidInput("MP3 count1 coefficient exceeds i8 range"))?,
            ))
        })
        .collect()
}

/// Selects a conservative Layer III count1 table class.
pub fn select_count1_table(
    quads: &[Layer3Count1Quad],
) -> Result<Layer3Count1TableSelection, Error> {
    let max_nonzero_values = max_count1_nonzero_values(quads)?;

    Ok(Layer3Count1TableSelection {
        table_select: max_nonzero_values >= 3,
        max_nonzero_values,
    })
}

/// Selects the shortest available Layer III count1 table from a provider.
pub fn select_count1_table_by_bit_cost(
    quads: &[Layer3Count1Quad],
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<Layer3Count1TableSelection, Error> {
    let max_nonzero_values = max_count1_nonzero_values(quads)?;
    if max_nonzero_values == 0 {
        return Ok(Layer3Count1TableSelection {
            table_select: false,
            max_nonzero_values,
        });
    }

    let candidates = [
        (false, provider.count1_table_0),
        (true, provider.count1_table_1),
    ];
    let mut best: Option<(Layer3Count1TableSelection, usize)> = None;
    for (table_select, table) in candidates {
        if table.is_empty() {
            continue;
        }
        let selection = Layer3Count1TableSelection {
            table_select,
            max_nonzero_values,
        };
        let Ok(packed) = pack_count1_quads_with_table_selection(quads, table, selection) else {
            continue;
        };
        if best
            .as_ref()
            .is_none_or(|(_, bit_len)| packed.bit_len < *bit_len)
        {
            best = Some((selection, packed.bit_len));
        }
    }

    best.map(|(selection, _)| selection)
        .ok_or(Error::UnsupportedFeature("MP3 count1 Huffman table"))
}

/// Applies one count1 Huffman table selection to Layer III side info.
pub fn apply_count1_table_to_granule(
    granule: &mut Layer3GranuleChannelInfo,
    selection: Layer3Count1TableSelection,
) {
    granule.count1table_select = selection.table_select;
}

/// Builds one Layer III granule/channel entropy payload from quantized spectrum.
pub fn pack_quantized_spectrum_for_granule(
    granule: &mut Layer3GranuleChannelInfo,
    quantized: &[i32],
    tables: Layer3EntropyTables<'_>,
) -> Result<PackedBits, Error> {
    let regions = plan_spectral_regions(quantized)?;
    apply_spectral_regions_to_granule(granule, regions)?;

    let big_value_pairs = big_value_pairs(quantized, regions)?;
    let big_value_selection = select_big_value_table(&big_value_pairs)?;
    apply_big_value_table_to_granule(granule, big_value_selection);

    let count1_quads = count1_quads(quantized, regions)?;
    let count1_selection = select_count1_table(&count1_quads)?;
    apply_count1_table_to_granule(granule, count1_selection);

    let big_values = pack_big_value_pairs_with_linbits(
        &big_value_pairs,
        tables.big_values,
        big_value_selection.linbits,
    )?;
    let count1 = pack_count1_quads_with_sign_bits(&count1_quads, tables.count1)?;
    pack_main_data_regions_for_granule(granule, big_values, count1)
}

/// Builds one Layer III granule/channel main-data payload with scale factors.
pub fn pack_quantized_spectrum_with_scale_factors_for_granule(
    granule: &mut Layer3GranuleChannelInfo,
    scale_factors: PackedBits,
    quantized: &[i32],
    tables: Layer3EntropyTables<'_>,
) -> Result<PackedBits, Error> {
    let regions = plan_spectral_regions(quantized)?;
    apply_spectral_regions_to_granule(granule, regions)?;

    let big_value_pairs = big_value_pairs(quantized, regions)?;
    let big_value_selection = select_big_value_table(&big_value_pairs)?;
    apply_big_value_table_to_granule(granule, big_value_selection);

    let count1_quads = count1_quads(quantized, regions)?;
    let count1_selection = select_count1_table(&count1_quads)?;
    apply_count1_table_to_granule(granule, count1_selection);

    let big_values = pack_big_value_pairs_with_linbits(
        &big_value_pairs,
        tables.big_values,
        big_value_selection.linbits,
    )?;
    let count1 = pack_count1_quads_with_sign_bits(&count1_quads, tables.count1)?;
    pack_main_data_parts_for_granule(granule, scale_factors, big_values, count1)
}

/// Builds one granule/channel entropy payload using table selection and provider lookup.
pub fn pack_quantized_spectrum_with_table_provider(
    granule: &mut Layer3GranuleChannelInfo,
    quantized: &[i32],
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<PackedBits, Error> {
    let regions = plan_spectral_regions(quantized)?;
    apply_spectral_regions_to_granule(granule, regions)?;

    let big_value_pairs = big_value_pairs(quantized, regions)?;
    // This MPEG-1-only entry uses the MPEG-1 long-block region boundaries; the
    // MPEG-2 LSF path goes through the `_for_rate_` variant below.
    let (region0_pairs, region1_pairs) = long_block_region_pair_split(
        granule.region0_count,
        granule.region1_count,
        big_value_pairs.len(),
        44_100,
    );
    let big_value_selection = select_big_value_region_tables_by_bit_cost(
        &big_value_pairs,
        region0_pairs,
        region1_pairs,
        provider,
    )?;
    apply_big_value_region_tables_to_granule(granule, big_value_selection)?;

    let count1_quads = count1_quads(quantized, regions)?;
    let count1_selection = select_count1_table_by_bit_cost(&count1_quads, provider)?;
    apply_count1_table_to_granule(granule, count1_selection);

    let big_values = pack_big_value_pairs_with_region_tables_and_provider(
        &big_value_pairs,
        big_value_selection,
        provider,
    )?;
    let count1 = pack_count1_quads_with_table_selection(
        &count1_quads,
        provider.count1_table(count1_selection)?,
        count1_selection,
    )?;
    pack_main_data_regions_for_granule(granule, big_values, count1)
}

/// Builds one granule/channel main-data payload with scale factors and provider
/// lookup, using the MPEG-1 long-block region boundaries.
pub fn pack_quantized_spectrum_with_scale_factors_and_table_provider(
    granule: &mut Layer3GranuleChannelInfo,
    scale_factors: PackedBits,
    quantized: &[i32],
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<PackedBits, Error> {
    pack_quantized_spectrum_with_scale_factors_for_rate_and_table_provider(
        granule,
        scale_factors,
        quantized,
        44_100,
        provider,
    )
}

/// Builds one granule/channel main-data payload with scale factors, resolving
/// the big-value region boundaries for the granule's sample rate (MPEG-1 vs
/// MPEG-2 LSF). A spec decoder maps `region_address1`/`region_address2` to
/// spectral lines through the rate-specific `sfBandIndex`, so the encoder must
/// split the big-value pairs at the matching boundaries to stay in sync.
pub fn pack_quantized_spectrum_with_scale_factors_for_rate_and_table_provider(
    granule: &mut Layer3GranuleChannelInfo,
    scale_factors: PackedBits,
    quantized: &[i32],
    sample_rate: u32,
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<PackedBits, Error> {
    let regions = plan_spectral_regions(quantized)?;
    apply_spectral_regions_to_granule(granule, regions)?;

    let big_value_pairs = big_value_pairs(quantized, regions)?;
    let (region0_pairs, region1_pairs) = long_block_region_pair_split(
        granule.region0_count,
        granule.region1_count,
        big_value_pairs.len(),
        sample_rate,
    );
    let big_value_selection = select_big_value_region_tables_by_bit_cost(
        &big_value_pairs,
        region0_pairs,
        region1_pairs,
        provider,
    )?;
    apply_big_value_region_tables_to_granule(granule, big_value_selection)?;

    let count1_quads = count1_quads(quantized, regions)?;
    let count1_selection = select_count1_table_by_bit_cost(&count1_quads, provider)?;
    apply_count1_table_to_granule(granule, count1_selection);

    let big_values = pack_big_value_pairs_with_region_tables_and_provider(
        &big_value_pairs,
        big_value_selection,
        provider,
    )?;
    let count1 = pack_count1_quads_with_table_selection(
        &count1_quads,
        provider.count1_table(count1_selection)?,
        count1_selection,
    )?;
    pack_main_data_parts_for_granule(granule, scale_factors, big_values, count1)
}
