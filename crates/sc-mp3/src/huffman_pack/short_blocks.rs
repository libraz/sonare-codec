use super::*;

/// First spectral line of region1 for a short block. With `window_switching`
/// set and `block_type == 2`, the decoder does not read `region0_count`/
/// `region1_count`; it fixes `region1Start = 36` and leaves region2 empty
/// (ISO/IEC 11172-3 §2.4.2.7). The encoder must split the big-value pairs at the
/// same line so the two transmitted `table_select` entries stay in sync.
pub const LAYER3_SHORT_REGION1_START_LINE: usize = 36;

/// Implicit `region0_count` for a window-switched non-short block (block_type 1
/// start / 3 stop). The decoder fixes `region0_count = 7` and
/// `region1_count = 20 − 7 = 13` for these blocks because the fields are not
/// transmitted (ISO/IEC 11172-3 §2.4.1.7), placing region1 over the long
/// scale-factor bands and leaving region2 empty.
pub const LAYER3_TRANSITION_REGION0_COUNT: u8 = 7;
/// Implicit `region1_count` for a window-switched non-short block.
pub const LAYER3_TRANSITION_REGION1_COUNT: u8 = 13;

/// Builds one MPEG-1 Layer III transition-block (block_type 1 start / 3 stop)
/// main-data payload from a long-ordered quantized spectrum.
///
/// Transition blocks share the long-block scale-factor partition and frequency
/// order; only the big-value region boundaries are fixed
/// ([`LAYER3_TRANSITION_REGION0_COUNT`]/[`LAYER3_TRANSITION_REGION1_COUNT`])
/// because `window_switching` suppresses the transmitted region counts, and just
/// two `table_select` entries are carried (region2 stays empty). The granule's
/// `window_switching` side info is filled with the supplied `block_type` and a
/// zero `subblock_gain`; `global_gain` and `scalefac_scale` remain the caller's
/// responsibility.
pub fn pack_mpeg1_layer3_transition_quantized_spectrum_with_table_provider(
    granule: &mut Layer3GranuleChannelInfo,
    scale_factors: &[u8; MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT],
    block_type: Layer3BlockType,
    quantized: &[i32],
    sample_rate: u32,
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<PackedBits, Error> {
    let block_type_bits = match block_type {
        Layer3BlockType::Start | Layer3BlockType::Stop => block_type.block_type_bits(),
        _ => {
            return Err(Error::InvalidInput(
                "MP3 transition packer requires a start or stop block",
            ))
        }
    };

    let scale_factor_bits =
        pack_mpeg1_layer3_long_scale_factors_for_granule(granule, scale_factors)?;

    let regions = plan_spectral_regions(quantized)?;
    apply_spectral_regions_to_granule(granule, regions)?;

    let big_value_pairs = big_value_pairs(quantized, regions)?;
    let (region0_pairs, region1_pairs) = long_block_region_pair_split(
        LAYER3_TRANSITION_REGION0_COUNT,
        LAYER3_TRANSITION_REGION1_COUNT,
        big_value_pairs.len(),
        sample_rate,
    );
    let big_value_selection = select_big_value_region_tables_by_bit_cost(
        &big_value_pairs,
        region0_pairs,
        region1_pairs,
        provider,
    )?;

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

    granule.window_switching = Some(Layer3WindowSwitching {
        block_type: block_type_bits,
        mixed_block_flag: false,
        table_select: [
            big_value_selection.regions[0].table_select,
            big_value_selection.regions[1].table_select,
        ],
        subblock_gain: [0_u8; LAYER3_SHORT_WINDOWS],
    });

    pack_main_data_parts_for_granule(granule, scale_factor_bits, big_values, count1)
}

/// Selects MPEG-1 Layer III short-block scale-factor bit widths.
///
/// Short blocks reuse the long-block `scalefac_compress` table but apply `slen1`
/// to the first six short bands and `slen2` to the next six, each measured over
/// all three windows (ISO/IEC 11172-3 §2.4.2.7).
pub fn select_mpeg1_layer3_short_scale_factor_compress(
    scale_factors: &[[u8; LAYER3_SHORT_WINDOWS]; LAYER3_SHORT_SCALE_FACTOR_BANDS],
) -> Result<Layer3ScaleFactorCompress, Error> {
    let max_in = |bands: &[[u8; LAYER3_SHORT_WINDOWS]]| {
        bands
            .iter()
            .flat_map(|windows| windows.iter().copied())
            .max()
            .unwrap_or(0)
    };
    let max_slen1_value = max_in(&scale_factors[..6]);
    let max_slen2_value = max_in(&scale_factors[6..]);

    for selection in MPEG1_LAYER3_SCALE_FACTOR_COMPRESS {
        if scale_factor_fits_width(max_slen1_value, selection.slen1)
            && scale_factor_fits_width(max_slen2_value, selection.slen2)
        {
            return Ok(selection);
        }
    }

    Err(Error::InvalidInput(
        "MP3 short scale factor exceeds MPEG-1 Layer III compress range",
    ))
}

/// Packs MPEG-1 Layer III short-block scale-factor values.
///
/// The values are written group-major then band-major then window-minor: the
/// first six bands with `slen1`, then the next six with `slen2`, each band's
/// three windows consecutively (ISO/IEC 11172-3 §2.4.1.7).
pub fn pack_mpeg1_layer3_short_scale_factors(
    scale_factors: &[[u8; LAYER3_SHORT_WINDOWS]; LAYER3_SHORT_SCALE_FACTOR_BANDS],
    selection: Layer3ScaleFactorCompress,
) -> Result<PackedBits, Error> {
    if !MPEG1_LAYER3_SCALE_FACTOR_COMPRESS.contains(&selection) {
        return Err(Error::InvalidInput(
            "invalid MPEG-1 Layer III scalefac_compress selection",
        ));
    }

    let mut writer = CoreBitWriter::new();
    for (band, windows) in scale_factors.iter().enumerate() {
        let width = if band < 6 {
            selection.slen1
        } else {
            selection.slen2
        };
        for &scale_factor in windows {
            write_mp3_scale_factor(&mut writer, scale_factor, width)?;
        }
    }

    let bit_len = writer.bit_len();
    Ok(PackedBits {
        bytes: writer.finish_byte_aligned(),
        bit_len,
    })
}

/// Packs MPEG-1 Layer III short-block scale factors and updates side-info
/// metadata.
pub fn pack_mpeg1_layer3_short_scale_factors_for_granule(
    granule: &mut Layer3GranuleChannelInfo,
    scale_factors: &[[u8; LAYER3_SHORT_WINDOWS]; LAYER3_SHORT_SCALE_FACTOR_BANDS],
) -> Result<PackedBits, Error> {
    let selection = select_mpeg1_layer3_short_scale_factor_compress(scale_factors)?;
    granule.scalefac_compress = selection.scalefac_compress;
    pack_mpeg1_layer3_short_scale_factors(scale_factors, selection)
}

/// Builds one MPEG-1 Layer III short-block main-data payload from per-(band,
/// window) scale factors and a reordered quantized spectrum.
///
/// `quantized` must be in bitstream reorder order
/// ([`crate::layer3_short_reorder_map`]). The big-value pairs are split into two
/// regions at the fixed short-block boundary [`LAYER3_SHORT_REGION1_START_LINE`]
/// (region2 stays empty), each coded with its own provider-selected Huffman
/// table. The granule's `window_switching` side info is filled with
/// `block_type == 2`, the two region `table_select` values, and the supplied
/// `subblock_gain`; `global_gain` and `scalefac_scale` remain the caller's
/// responsibility.
pub fn pack_mpeg1_layer3_short_quantized_spectrum_with_table_provider(
    granule: &mut Layer3GranuleChannelInfo,
    scale_factors: &[[u8; LAYER3_SHORT_WINDOWS]; LAYER3_SHORT_SCALE_FACTOR_BANDS],
    subblock_gain: &[u8; LAYER3_SHORT_WINDOWS],
    quantized: &[i32],
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<PackedBits, Error> {
    let scale_factor_bits =
        pack_mpeg1_layer3_short_scale_factors_for_granule(granule, scale_factors)?;

    let regions = plan_spectral_regions(quantized)?;
    apply_spectral_regions_to_granule(granule, regions)?;

    let big_value_pairs = big_value_pairs(quantized, regions)?;
    // Short blocks fix region1 at line 36 (pair 18) and leave region2 empty.
    let region0_pairs = (LAYER3_SHORT_REGION1_START_LINE / 2).min(big_value_pairs.len());
    let region1_pairs = big_value_pairs.len() - region0_pairs;
    let big_value_selection = select_big_value_region_tables_by_bit_cost(
        &big_value_pairs,
        region0_pairs,
        region1_pairs,
        provider,
    )?;

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

    granule.window_switching = Some(Layer3WindowSwitching {
        block_type: Layer3BlockType::Short.block_type_bits(),
        mixed_block_flag: false,
        table_select: [
            big_value_selection.regions[0].table_select,
            big_value_selection.regions[1].table_select,
        ],
        subblock_gain: *subblock_gain,
    });

    pack_main_data_parts_for_granule(granule, scale_factor_bits, big_values, count1)
}
