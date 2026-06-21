use super::*;

/// Selects MPEG-1 Layer III long-block scale-factor bit widths.
pub fn select_mpeg1_layer3_long_scale_factor_compress(
    scale_factors: &[u8; MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT],
) -> Result<Layer3ScaleFactorCompress, Error> {
    let max_slen1_value = scale_factors[..11].iter().copied().max().unwrap_or(0);
    let max_slen2_value = scale_factors[11..].iter().copied().max().unwrap_or(0);

    for selection in MPEG1_LAYER3_SCALE_FACTOR_COMPRESS {
        if scale_factor_fits_width(max_slen1_value, selection.slen1)
            && scale_factor_fits_width(max_slen2_value, selection.slen2)
        {
            return Ok(selection);
        }
    }

    Err(Error::InvalidInput(
        "MP3 scale factor exceeds MPEG-1 Layer III compress range",
    ))
}

/// Applies MPEG-1 Layer III scale-factor compression metadata to side info.
pub fn apply_scale_factor_compress_to_granule(
    granule: &mut Layer3GranuleChannelInfo,
    selection: Layer3ScaleFactorCompress,
) {
    granule.scalefac_compress = selection.scalefac_compress;
}

/// Packs MPEG-1 Layer III long-block scale-factor values.
pub fn pack_mpeg1_layer3_long_scale_factors(
    scale_factors: &[u8; MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT],
    selection: Layer3ScaleFactorCompress,
) -> Result<PackedBits, Error> {
    if !MPEG1_LAYER3_SCALE_FACTOR_COMPRESS.contains(&selection) {
        return Err(Error::InvalidInput(
            "invalid MPEG-1 Layer III scalefac_compress selection",
        ));
    }

    let mut writer = CoreBitWriter::new();
    for &scale_factor in &scale_factors[..11] {
        write_mp3_scale_factor(&mut writer, scale_factor, selection.slen1)?;
    }
    for &scale_factor in &scale_factors[11..] {
        write_mp3_scale_factor(&mut writer, scale_factor, selection.slen2)?;
    }

    let bit_len = writer.bit_len();
    Ok(PackedBits {
        bytes: writer.finish_byte_aligned(),
        bit_len,
    })
}

/// Packs MPEG-1 Layer III long-block scale factors and updates side-info metadata.
pub fn pack_mpeg1_layer3_long_scale_factors_for_granule(
    granule: &mut Layer3GranuleChannelInfo,
    scale_factors: &[u8; MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT],
) -> Result<PackedBits, Error> {
    let selection = select_mpeg1_layer3_long_scale_factor_compress(scale_factors)?;
    apply_scale_factor_compress_to_granule(granule, selection);
    pack_mpeg1_layer3_long_scale_factors(scale_factors, selection)
}

/// Minimum bit width that represents `value` as an unsigned integer (0 → 0).
pub(crate) fn min_scale_factor_width(value: u8) -> u8 {
    let mut width = 0_u8;
    while u16::from(value) >= (1_u16 << width) {
        width += 1;
    }
    width
}

/// Selects an MPEG-2 LSF long-block scale-factor partition (ISO/IEC 13818-3
/// §2.4.3.2) for the given per-band scale factors.
///
/// Both `preflag == 0` partition branches are evaluated and the feasible one
/// with the fewest total scale-factor bits is chosen (group sizes `[6,5,5,5]`
/// breaks ties). Returns an error when no `preflag == 0` branch can represent
/// the scale factors (a group exceeds its branch's bit-width capacity).
pub fn select_mpeg2_layer3_lsf_long_scale_factor_compress(
    scale_factors: &[u8; MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT],
) -> Result<Mpeg2Layer3LsfScaleFactorCompress, Error> {
    // (group_sizes, per-group inclusive slen ceilings) for the two preflag=0
    // branches. Branch 1's final group must vanish (slen == 0).
    const BRANCHES: [([u8; 4], [u8; 4]); 2] =
        [([6, 5, 5, 5], [4, 4, 3, 3]), ([6, 5, 7, 3], [4, 4, 3, 0])];

    let mut best: Option<Mpeg2Layer3LsfScaleFactorCompress> = None;
    let mut best_bits = usize::MAX;

    for (branch, (group_sizes, slen_caps)) in BRANCHES.iter().enumerate() {
        let mut slen = [0_u8; 4];
        let mut start = 0_usize;
        let mut feasible = true;
        for group in 0..4 {
            let len = usize::from(group_sizes[group]);
            let group_max = scale_factors[start..start + len]
                .iter()
                .copied()
                .max()
                .unwrap_or(0);
            let width = min_scale_factor_width(group_max);
            if width > slen_caps[group] {
                feasible = false;
                break;
            }
            slen[group] = width;
            start += len;
        }
        if !feasible {
            continue;
        }

        let total_bits: usize = (0..4)
            .map(|group| usize::from(group_sizes[group]) * usize::from(slen[group]))
            .sum();

        let scalefac_compress = if branch == 0 {
            (u16::from(slen[0] * 5 + slen[1]) << 4) | (u16::from(slen[2]) << 2) | u16::from(slen[3])
        } else {
            400 + ((u16::from(slen[0] * 5 + slen[1]) << 2) | u16::from(slen[2]))
        };

        if total_bits < best_bits {
            best_bits = total_bits;
            best = Some(Mpeg2Layer3LsfScaleFactorCompress {
                scalefac_compress,
                group_sizes: *group_sizes,
                slen,
            });
        }
    }

    best.ok_or(Error::InvalidInput(
        "MP3 scale factor exceeds MPEG-2 LSF Layer III compress range",
    ))
}

/// Packs MPEG-2 LSF long-block scale-factor values using a partition selected
/// by [`select_mpeg2_layer3_lsf_long_scale_factor_compress`].
pub fn pack_mpeg2_layer3_lsf_long_scale_factors(
    scale_factors: &[u8; MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT],
    selection: Mpeg2Layer3LsfScaleFactorCompress,
) -> Result<PackedBits, Error> {
    if usize::from(
        selection
            .group_sizes
            .iter()
            .copied()
            .map(u16::from)
            .sum::<u16>(),
    ) != MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT
    {
        return Err(Error::InvalidInput(
            "MPEG-2 LSF Layer III scale-factor groups must cover 21 bands",
        ));
    }

    let mut writer = CoreBitWriter::new();
    let mut start = 0_usize;
    for group in 0..4 {
        let len = usize::from(selection.group_sizes[group]);
        for &scale_factor in &scale_factors[start..start + len] {
            write_mp3_scale_factor(&mut writer, scale_factor, selection.slen[group])?;
        }
        start += len;
    }

    let bit_len = writer.bit_len();
    Ok(PackedBits {
        bytes: writer.finish_byte_aligned(),
        bit_len,
    })
}

/// Applies MPEG-2 LSF scale-factor compression metadata to side info.
pub fn apply_mpeg2_lsf_scale_factor_compress_to_granule(
    granule: &mut Layer3GranuleChannelInfo,
    selection: Mpeg2Layer3LsfScaleFactorCompress,
) {
    granule.scalefac_compress = selection.scalefac_compress;
}

/// Packs MPEG-2 LSF long-block scale factors and updates side-info metadata.
pub fn pack_mpeg2_layer3_lsf_long_scale_factors_for_granule(
    granule: &mut Layer3GranuleChannelInfo,
    scale_factors: &[u8; MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT],
) -> Result<PackedBits, Error> {
    let selection = select_mpeg2_layer3_lsf_long_scale_factor_compress(scale_factors)?;
    apply_mpeg2_lsf_scale_factor_compress_to_granule(granule, selection);
    pack_mpeg2_layer3_lsf_long_scale_factors(scale_factors, selection)
}

/// Selects deterministic MPEG-1 Layer III long-block scale factors from coefficients.
pub fn select_mpeg1_layer3_long_scale_factors_for_quantized_spectrum(
    quantized: &[i32],
) -> Result<[u8; MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT], Error> {
    plan_spectral_regions(quantized)?;

    let mut band_max = [0_u16; MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT];
    for (index, &coefficient) in quantized.iter().enumerate() {
        let magnitude = coefficient
            .checked_abs()
            .ok_or(Error::InvalidInput("MP3 spectral coefficient overflows"))?;
        if magnitude > 8191 {
            return Err(Error::InvalidInput(
                "MP3 spectral coefficient exceeds supported range",
            ));
        }

        let band = index
            .checked_mul(MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT)
            .ok_or(Error::InvalidInput("MP3 scale-factor band index overflows"))?
            / quantized.len();
        band_max[band] = band_max[band].max(
            u16::try_from(magnitude)
                .map_err(|_| Error::InvalidInput("MP3 coefficient magnitude overflows"))?,
        );
    }

    let mut scale_factors = [0_u8; MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT];
    for (band, &max_magnitude) in band_max.iter().enumerate() {
        let raw = if max_magnitude == 0 {
            0
        } else {
            u16::BITS as u8 - max_magnitude.leading_zeros() as u8
        };
        let syntax_cap = if band < 11 { 15 } else { 7 };
        scale_factors[band] = raw.min(syntax_cap);
    }
    Ok(scale_factors)
}

/// Builds one MPEG-1 Layer III long-block main-data payload from scale factors and spectrum.
pub fn pack_mpeg1_layer3_long_quantized_spectrum_for_granule(
    granule: &mut Layer3GranuleChannelInfo,
    scale_factors: &[u8; MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT],
    quantized: &[i32],
    tables: Layer3EntropyTables<'_>,
) -> Result<PackedBits, Error> {
    let scale_factor_bits =
        pack_mpeg1_layer3_long_scale_factors_for_granule(granule, scale_factors)?;
    pack_quantized_spectrum_with_scale_factors_for_granule(
        granule,
        scale_factor_bits,
        quantized,
        tables,
    )
}

/// Builds one MPEG-1 Layer III long-block main-data payload using provider lookup.
pub fn pack_mpeg1_layer3_long_quantized_spectrum_with_table_provider(
    granule: &mut Layer3GranuleChannelInfo,
    scale_factors: &[u8; MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT],
    quantized: &[i32],
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<PackedBits, Error> {
    let scale_factor_bits =
        pack_mpeg1_layer3_long_scale_factors_for_granule(granule, scale_factors)?;
    pack_quantized_spectrum_with_scale_factors_and_table_provider(
        granule,
        scale_factor_bits,
        quantized,
        provider,
    )
}

/// Builds one long-block payload for a given sample rate, selecting the
/// big-value region boundaries from the rate's `sfBandIndex`.
///
/// The scale factors are packed with the MPEG-1 long-block grouping; for the
/// MPEG-2 LSF calibrated-gain path the factors are all zero, which encodes as
/// `scalefac_compress = 0` (zero scale-factor bits) — identical under both the
/// MPEG-1 and MPEG-2 LSF decoder derivations.
pub(crate) fn pack_layer3_long_quantized_spectrum_for_rate_and_table_provider(
    granule: &mut Layer3GranuleChannelInfo,
    scale_factors: &[u8; MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT],
    quantized: &[i32],
    sample_rate: u32,
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<PackedBits, Error> {
    let scale_factor_bits =
        pack_mpeg1_layer3_long_scale_factors_for_granule(granule, scale_factors)?;
    pack_quantized_spectrum_with_scale_factors_for_rate_and_table_provider(
        granule,
        scale_factor_bits,
        quantized,
        sample_rate,
        provider,
    )
}

/// Builds one MPEG-2 LSF long-block payload: the scale factors are packed with
/// the LSF partition scheme (ISO/IEC 13818-3 §2.4.3.2) and the big-value region
/// boundaries are resolved for the granule's sample rate.
///
/// This is the MPEG-2 counterpart of
/// [`pack_mpeg1_layer3_long_quantized_spectrum_with_table_provider`]: the
/// MPEG-1 path encodes non-zero scale factors with `scalefac_compress`/`slen`
/// pairs, whereas LSF uses the four-group partition, so the two schemes are not
/// interchangeable once any scale factor is non-zero.
pub(crate) fn pack_mpeg2_layer3_lsf_long_quantized_spectrum_for_rate_and_table_provider(
    granule: &mut Layer3GranuleChannelInfo,
    scale_factors: &[u8; MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT],
    quantized: &[i32],
    sample_rate: u32,
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<PackedBits, Error> {
    let scale_factor_bits =
        pack_mpeg2_layer3_lsf_long_scale_factors_for_granule(granule, scale_factors)?;
    pack_quantized_spectrum_with_scale_factors_for_rate_and_table_provider(
        granule,
        scale_factor_bits,
        quantized,
        sample_rate,
        provider,
    )
}

/// Builds one MPEG-1 Layer III long-block payload with internally selected scale factors.
pub fn pack_mpeg1_layer3_long_quantized_spectrum_with_selected_scale_factors_for_granule(
    granule: &mut Layer3GranuleChannelInfo,
    quantized: &[i32],
    tables: Layer3EntropyTables<'_>,
) -> Result<PackedBits, Error> {
    let scale_factors = select_mpeg1_layer3_long_scale_factors_for_quantized_spectrum(quantized)?;
    pack_mpeg1_layer3_long_quantized_spectrum_for_granule(
        granule,
        &scale_factors,
        quantized,
        tables,
    )
}

/// Builds one MPEG-1 Layer III long-block payload with selected scale factors and provider lookup.
pub fn pack_mpeg1_layer3_long_quantized_spectrum_with_selected_scale_factors_and_table_provider(
    granule: &mut Layer3GranuleChannelInfo,
    quantized: &[i32],
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<PackedBits, Error> {
    let scale_factors = select_mpeg1_layer3_long_scale_factors_for_quantized_spectrum(quantized)?;
    pack_mpeg1_layer3_long_quantized_spectrum_with_table_provider(
        granule,
        &scale_factors,
        quantized,
        provider,
    )
}

/// Builds one MPEG-1 Layer III long-block payload from PCM analysis.
///
/// The quantizer `step` is folded entirely into `global_gain`
/// (see [`mpeg1_layer3_global_gain_for_step`]) and all scale factors are left at
/// zero, so the decoder's per-line requantization inverts the encoder's
/// quantization without per-band double scaling. An all-zero granule keeps the
/// ISO reference gain, preserving the canonical silent-frame encoding.
pub fn pack_mpeg1_layer3_pcm_long_block_with_calibrated_gain_for_granule(
    granule: &mut Layer3GranuleChannelInfo,
    pcm: &AudioBuffer,
    channel: usize,
    start_frame: usize,
    step: f32,
    tables: Layer3EntropyTables<'_>,
) -> Result<PackedBits, Error> {
    let quantized = quantize_pcm_long_block(pcm, channel, start_frame, step)?;
    let scale_factors = [0_u8; MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT];
    let packed = pack_mpeg1_layer3_long_quantized_spectrum_for_granule(
        granule,
        &scale_factors,
        &quantized,
        tables,
    )?;
    granule.global_gain = calibrated_global_gain_for_granule(&quantized, step);
    Ok(packed)
}

/// Builds one MPEG-1 Layer III long-block payload from PCM analysis using provider lookup.
///
/// Behaves like [`pack_mpeg1_layer3_pcm_long_block_with_calibrated_gain_for_granule`]
/// but resolves the entropy tables through a [`Layer3EntropyTableProvider`].
pub fn pack_mpeg1_layer3_pcm_long_block_with_calibrated_gain_and_table_provider(
    granule: &mut Layer3GranuleChannelInfo,
    pcm: &AudioBuffer,
    channel: usize,
    start_frame: usize,
    step: f32,
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<PackedBits, Error> {
    let quantized = quantize_pcm_long_block(pcm, channel, start_frame, step)?;
    let scale_factors = [0_u8; MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT];
    let packed = pack_mpeg1_layer3_long_quantized_spectrum_with_table_provider(
        granule,
        &scale_factors,
        &quantized,
        provider,
    )?;
    granule.global_gain = calibrated_global_gain_for_granule(&quantized, step);
    Ok(packed)
}

/// Builds one calibrated-gain long-block payload using the big-value region
/// boundaries for `pcm.sample_rate` (MPEG-1 or MPEG-2 LSF).
pub(crate) fn pack_layer3_pcm_long_block_with_calibrated_gain_for_rate_and_table_provider(
    granule: &mut Layer3GranuleChannelInfo,
    pcm: &AudioBuffer,
    channel: usize,
    start_frame: usize,
    step: f32,
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<PackedBits, Error> {
    let quantized = quantize_pcm_long_block(pcm, channel, start_frame, step)?;
    let scale_factors = [0_u8; MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT];
    let packed = pack_layer3_long_quantized_spectrum_for_rate_and_table_provider(
        granule,
        &scale_factors,
        &quantized,
        pcm.sample_rate,
        provider,
    )?;
    granule.global_gain = calibrated_global_gain_for_granule(&quantized, step);
    Ok(packed)
}
