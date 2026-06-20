use super::*;

/// Long-block scalefactor-band line boundaries shared by all MPEG-1 sample
/// rates. The full per-rate tables diverge only above index 6; the prefix used
/// to place the big-value region addresses (indices 0..=2) is rate-independent
/// across the three MPEG-1 rates.
pub(crate) const LONG_SFB_LOW_BOUNDARIES_MPEG1: [usize; 7] = [0, 4, 8, 12, 16, 20, 24];

/// Long-block scalefactor-band line boundaries shared by the MPEG-2 LSF rates
/// (16/22.05/24 kHz). The full per-rate tables diverge only above index 12, so
/// the low prefix used for the big-value region addresses is rate-independent
/// across the three LSF rates (ISO/IEC 13818-3 Table B.8).
pub(crate) const LONG_SFB_LOW_BOUNDARIES_MPEG2_LSF: [usize; 7] = [0, 6, 12, 18, 24, 30, 36];

/// Maps the written `region_address1`/`region_address2` side-info fields to the
/// big-value region split in pairs, exactly as a spec decoder derives it.
///
/// The decoder reads `region0 = [0, sfb[ra1 + 1])`,
/// `region1 = [sfb[ra1 + 1], sfb[ra1 + ra2 + 2])`, and `region2` the remainder,
/// all in spectral-line units, then capped at the big-value count. The encoder
/// must split pairs at the same boundaries so the bitstream stays in sync. The
/// `sfb` table is sample-rate dependent (MPEG-1 vs MPEG-2 LSF), so the split
/// must use the boundaries for the granule's sample rate.
pub(crate) fn long_block_region_pair_split(
    region0_count: u8,
    region1_count: u8,
    pair_count: usize,
    sample_rate: u32,
) -> (usize, usize) {
    let boundaries = if matches!(sample_rate, 16_000 | 22_050 | 24_000) {
        &LONG_SFB_LOW_BOUNDARIES_MPEG2_LSF
    } else {
        &LONG_SFB_LOW_BOUNDARIES_MPEG1
    };
    let r1_idx = usize::from(region0_count) + 1;
    let r2_idx = usize::from(region0_count) + usize::from(region1_count) + 2;
    let r1_start = boundaries.get(r1_idx).copied().unwrap_or(usize::MAX);
    let r2_start = boundaries.get(r2_idx).copied().unwrap_or(usize::MAX);
    let region0 = (r1_start / 2).min(pair_count);
    let region1 = (r2_start.saturating_sub(r1_start) / 2).min(pair_count - region0);
    (region0, region1)
}

/// Splits quantized Layer III spectral coefficients into entropy-coded regions.
pub fn plan_spectral_regions(quantized: &[i32]) -> Result<Layer3SpectralRegions, Error> {
    if quantized.is_empty() || quantized.len() > 576 {
        return Err(Error::InvalidInput(
            "invalid MP3 spectral coefficient count",
        ));
    }
    for &coeff in quantized {
        if coeff
            .checked_abs()
            .ok_or(Error::InvalidInput("MP3 spectral coefficient overflows"))?
            > 8191
        {
            return Err(Error::InvalidInput(
                "MP3 spectral coefficient exceeds supported range",
            ));
        }
    }

    let Some(last_nonzero) = quantized.iter().rposition(|coeff| *coeff != 0) else {
        return Ok(Layer3SpectralRegions {
            big_values: 0,
            count1: 0,
            rzero: u16::try_from(quantized.len())
                .map_err(|_| Error::InvalidInput("MP3 rzero region is too large"))?,
        });
    };

    let nonzero_end = last_nonzero + 1;
    let mut count1_start = nonzero_end;
    while count1_start >= 4 {
        let start = count1_start - 4;
        if quantized[start..count1_start]
            .iter()
            .all(|coeff| coeff.abs() <= 1)
        {
            count1_start = start;
        } else {
            break;
        }
    }

    let big_values = count1_start.div_ceil(2);
    let count1 = (nonzero_end - count1_start) / 4;
    Ok(Layer3SpectralRegions {
        big_values: u16::try_from(big_values)
            .map_err(|_| Error::InvalidInput("MP3 big_values region is too large"))?,
        count1: u16::try_from(count1)
            .map_err(|_| Error::InvalidInput("MP3 count1 region is too large"))?,
        rzero: u16::try_from(quantized.len() - nonzero_end)
            .map_err(|_| Error::InvalidInput("MP3 rzero region is too large"))?,
    })
}

/// Applies spectral region planning to a Layer III granule/channel side-info entry.
pub fn apply_spectral_regions_to_granule(
    granule: &mut Layer3GranuleChannelInfo,
    regions: Layer3SpectralRegions,
) -> Result<(), Error> {
    if regions.big_values > 288 {
        return Err(Error::InvalidInput(
            "MP3 big_values exceeds side-info range",
        ));
    }

    granule.big_values = regions.big_values;
    if regions.big_values == 0 {
        granule.table_select = [0; 3];
        granule.region0_count = 0;
        granule.region1_count = 0;
        granule.count1table_select = regions.count1 > 0;
        return Ok(());
    }

    granule.table_select = [1, 1, 0];
    // Fixed region addresses 0/0 place the region boundaries at the
    // rate-independent low scalefactor bands (lines 4 and 8). The big-value
    // packer splits pairs at the matching boundaries via
    // `long_block_region_pair_split`, keeping the encoder in sync with the
    // decoder's scalefactor-band interpretation of these fields.
    granule.region0_count = 0;
    granule.region1_count = 0;
    granule.count1table_select = regions.count1 > 0;
    Ok(())
}

/// Packs preselected MP3 Layer III main-data Huffman codewords.
pub fn pack_main_data_codewords(codes: &[HuffmanCode]) -> Result<Vec<u8>, Error> {
    pack_huffman_codes(codes)
}

/// Packs preselected MP3 Layer III main-data codewords and preserves bit length.
pub fn pack_main_data_codewords_with_len(codes: &[HuffmanCode]) -> Result<PackedBits, Error> {
    pack_huffman_codes_with_len(codes)
}

/// Sets `part2_3_length` from already-packed Layer III scale-factor/Huffman bits.
pub fn apply_part2_3_length_to_granule(
    granule: &mut Layer3GranuleChannelInfo,
    bit_len: usize,
) -> Result<(), Error> {
    granule.part2_3_length = u16::try_from(bit_len)
        .map_err(|_| Error::InvalidInput("MP3 part2_3_length exceeds side-info range"))?;
    Ok(())
}

/// Packs preselected Layer III main-data codewords and updates side-info length.
pub fn pack_main_data_codewords_for_granule(
    granule: &mut Layer3GranuleChannelInfo,
    codes: &[HuffmanCode],
) -> Result<PackedBits, Error> {
    let packed = pack_main_data_codewords_with_len(codes)?;
    apply_part2_3_length_to_granule(granule, packed.bit_len)?;
    Ok(packed)
}
