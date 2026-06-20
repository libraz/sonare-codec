use super::*;

/// Assembles one MPEG-1 Layer III frame from PCM long-block payload scaffolding.
pub fn assemble_mpeg1_layer3_pcm_frame_with_selected_scale_factors(
    header: FrameHeader,
    pcm: &AudioBuffer,
    start_frame: usize,
    step: f32,
    tables: Layer3EntropyTables<'_>,
) -> Result<Vec<u8>, Error> {
    let mut side_info = prepare_mpeg1_layer3_pcm_frame_side_info(header, pcm)?;
    let mut payloads = Vec::with_capacity(header.layer3_granule_count() * header.channel_count());
    for granule in 0..header.layer3_granule_count() {
        let granule_start = start_frame
            .checked_add(
                granule
                    .checked_mul(576)
                    .ok_or(Error::InvalidInput("MP3 granule start frame overflows"))?,
            )
            .ok_or(Error::InvalidInput("MP3 granule start frame overflows"))?;
        for channel in 0..header.channel_count() {
            let payload = pack_mpeg1_layer3_pcm_long_block_with_calibrated_gain_for_granule(
                &mut side_info.granules[granule][channel],
                pcm,
                channel,
                granule_start,
                step,
                tables,
            )?;
            payloads.push(payload);
        }
    }
    assemble_layer3_frame_from_payloads(header, &side_info, &payloads)
}

/// Assembles one MPEG-1 Layer III frame from PCM long-block payloads using provider lookup.
pub fn assemble_mpeg1_layer3_pcm_frame_with_selected_scale_factors_and_table_provider(
    header: FrameHeader,
    pcm: &AudioBuffer,
    start_frame: usize,
    step: f32,
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<Vec<u8>, Error> {
    let (side_info, main_data) = pack_mpeg1_layer3_pcm_frame_payloads_with_table_provider(
        header,
        pcm,
        start_frame,
        step,
        provider,
    )?;
    assemble_layer3_frame(header, &side_info, &main_data.bytes)
}

/// Assembles one self-contained MPEG-1 Layer III frame using perceptual
/// scale-factor allocation (provider lookup).
///
/// Each granule/channel is packed with
/// [`pack_mpeg1_layer3_pcm_long_block_with_perceptual_scale_factors_and_table_provider`],
/// so the quantization noise is shaped per scale-factor band. The frame is
/// self-contained (`main_data_begin = 0`); the caller must pick a `step` and
/// header bitrate whose main data fits one slot.
pub fn assemble_mpeg1_layer3_pcm_frame_with_perceptual_scale_factors_and_table_provider(
    header: FrameHeader,
    pcm: &AudioBuffer,
    start_frame: usize,
    step: f32,
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<Vec<u8>, Error> {
    assemble_mpeg1_layer3_pcm_frame_with_perceptual_scalefac_scale_and_table_provider(
        header,
        pcm,
        start_frame,
        step,
        false,
        provider,
    )
}

/// Assembles one perceptual frame with caller-selected `scalefac_scale`.
pub fn assemble_mpeg1_layer3_pcm_frame_with_perceptual_scalefac_scale_and_table_provider(
    header: FrameHeader,
    pcm: &AudioBuffer,
    start_frame: usize,
    step: f32,
    scalefac_scale: bool,
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<Vec<u8>, Error> {
    let (side_info, main_data) =
        pack_mpeg1_layer3_pcm_frame_perceptual_scalefac_scale_payloads_with_table_provider(
            header,
            pcm,
            start_frame,
            step,
            scalefac_scale,
            provider,
        )?;
    assemble_layer3_frame(header, &side_info, &main_data.bytes)
}

/// Assembles one perceptual frame with an allowed-noise multiplier.
pub fn assemble_mpeg1_layer3_pcm_frame_with_perceptual_allowed_noise_scale_and_table_provider(
    header: FrameHeader,
    pcm: &AudioBuffer,
    start_frame: usize,
    step: f32,
    allowed_noise_scale: f64,
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<Vec<u8>, Error> {
    let (side_info, main_data) =
        pack_mpeg1_layer3_pcm_frame_perceptual_allowed_noise_scale_payloads_with_table_provider(
            header,
            pcm,
            start_frame,
            step,
            allowed_noise_scale,
            provider,
        )?;
    assemble_layer3_frame(header, &side_info, &main_data.bytes)
}

/// Assembles one self-contained MPEG-1 Layer III frame using perceptual
/// scale-factor allocation plus a diagnostic per-band bias.
pub fn assemble_mpeg1_layer3_pcm_frame_with_perceptual_scale_factor_band_bias_and_table_provider(
    header: FrameHeader,
    pcm: &AudioBuffer,
    start_frame: usize,
    step: f32,
    band_bias: Layer3ScaleFactorBandBias,
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<Vec<u8>, Error> {
    let (side_info, main_data) =
        pack_mpeg1_layer3_pcm_frame_perceptual_band_biased_payloads_with_table_provider(
            header,
            pcm,
            start_frame,
            step,
            band_bias,
            provider,
        )?;
    assemble_layer3_frame(header, &side_info, &main_data.bytes)
}

/// Assembles one self-contained MPEG-1 Layer III frame using perceptual
/// scale-factor allocation plus a diagnostic quantized band gain.
pub fn assemble_mpeg1_layer3_pcm_frame_with_perceptual_quantized_band_gain_and_table_provider(
    header: FrameHeader,
    pcm: &AudioBuffer,
    start_frame: usize,
    step: f32,
    band_gain: Layer3QuantizedBandGain,
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<Vec<u8>, Error> {
    let (side_info, main_data) =
        pack_mpeg1_layer3_pcm_frame_perceptual_quantized_band_gain_payloads_with_table_provider(
            header,
            pcm,
            start_frame,
            step,
            band_gain,
            provider,
        )?;
    assemble_layer3_frame(header, &side_info, &main_data.bytes)
}

/// Assembles one self-contained MPEG-1 Layer III frame using perceptual
/// scale-factor allocation plus diagnostic quantized band gain and global-gain
/// bias.
pub fn assemble_mpeg1_layer3_pcm_frame_with_perceptual_quantized_band_gain_and_global_gain_bias_and_table_provider(
    header: FrameHeader,
    pcm: &AudioBuffer,
    start_frame: usize,
    step: f32,
    band_gain: Layer3QuantizedBandGain,
    global_gain_bias: i16,
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<Vec<u8>, Error> {
    let (side_info, main_data) =
        pack_mpeg1_layer3_pcm_frame_perceptual_quantized_band_gain_and_global_gain_bias_payloads_with_table_provider(
            header,
            pcm,
            start_frame,
            step,
            band_gain,
            global_gain_bias,
            provider,
        )?;
    assemble_layer3_frame(header, &side_info, &main_data.bytes)
}

pub(crate) fn pack_mpeg1_layer3_pcm_frame_perceptual_payloads_with_table_provider(
    header: FrameHeader,
    pcm: &AudioBuffer,
    start_frame: usize,
    step: f32,
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<(Layer3SideInfo, PackedBits), Error> {
    pack_mpeg1_layer3_pcm_frame_perceptual_scalefac_scale_payloads_with_table_provider(
        header,
        pcm,
        start_frame,
        step,
        false,
        provider,
    )
}

pub(crate) fn pack_mpeg1_layer3_pcm_frame_perceptual_scalefac_scale_payloads_with_table_provider(
    header: FrameHeader,
    pcm: &AudioBuffer,
    start_frame: usize,
    step: f32,
    scalefac_scale: bool,
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<(Layer3SideInfo, PackedBits), Error> {
    let mut side_info = prepare_mpeg1_layer3_pcm_frame_side_info(header, pcm)?;
    let mut payloads = Vec::with_capacity(header.layer3_granule_count() * header.channel_count());
    for granule in 0..header.layer3_granule_count() {
        let granule_start = start_frame
            .checked_add(
                granule
                    .checked_mul(576)
                    .ok_or(Error::InvalidInput("MP3 granule start frame overflows"))?,
            )
            .ok_or(Error::InvalidInput("MP3 granule start frame overflows"))?;
        for channel in 0..header.channel_count() {
            let payload =
                pack_mpeg1_layer3_pcm_long_block_with_perceptual_scalefac_scale_and_table_provider(
                    &mut side_info.granules[granule][channel],
                    pcm,
                    channel,
                    granule_start,
                    step,
                    scalefac_scale,
                    provider,
                )?;
            payloads.push(payload);
        }
    }
    let main_data = pack_layer3_main_data_payloads(&header, &payloads)?;
    Ok((side_info, main_data))
}

pub(crate) fn pack_mpeg1_layer3_pcm_frame_perceptual_allowed_noise_scale_payloads_with_table_provider(
    header: FrameHeader,
    pcm: &AudioBuffer,
    start_frame: usize,
    step: f32,
    allowed_noise_scale: f64,
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<(Layer3SideInfo, PackedBits), Error> {
    let mut side_info = prepare_mpeg1_layer3_pcm_frame_side_info(header, pcm)?;
    let mut payloads = Vec::with_capacity(header.layer3_granule_count() * header.channel_count());
    for granule in 0..header.layer3_granule_count() {
        let granule_start = start_frame
            .checked_add(
                granule
                    .checked_mul(576)
                    .ok_or(Error::InvalidInput("MP3 granule start frame overflows"))?,
            )
            .ok_or(Error::InvalidInput("MP3 granule start frame overflows"))?;
        for channel in 0..header.channel_count() {
            let payload =
                pack_mpeg1_layer3_pcm_long_block_with_perceptual_allowed_noise_scale_and_table_provider(
                    &mut side_info.granules[granule][channel],
                    pcm,
                    channel,
                    granule_start,
                    step,
                    allowed_noise_scale,
                    provider,
                )?;
            payloads.push(payload);
        }
    }
    let main_data = pack_layer3_main_data_payloads(&header, &payloads)?;
    Ok((side_info, main_data))
}

pub(crate) fn pack_mpeg1_layer3_pcm_frame_perceptual_band_biased_payloads_with_table_provider(
    header: FrameHeader,
    pcm: &AudioBuffer,
    start_frame: usize,
    step: f32,
    band_bias: Layer3ScaleFactorBandBias,
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<(Layer3SideInfo, PackedBits), Error> {
    let mut side_info = prepare_mpeg1_layer3_pcm_frame_side_info(header, pcm)?;
    let mut payloads = Vec::with_capacity(header.layer3_granule_count() * header.channel_count());
    for granule in 0..header.layer3_granule_count() {
        let granule_start = start_frame
            .checked_add(
                granule
                    .checked_mul(576)
                    .ok_or(Error::InvalidInput("MP3 granule start frame overflows"))?,
            )
            .ok_or(Error::InvalidInput("MP3 granule start frame overflows"))?;
        for channel in 0..header.channel_count() {
            let payload =
                pack_mpeg1_layer3_pcm_long_block_with_perceptual_scale_factor_band_bias_and_table_provider(
                    &mut side_info.granules[granule][channel],
                    pcm,
                    channel,
                    granule_start,
                    step,
                    band_bias,
                    provider,
                )?;
            payloads.push(payload);
        }
    }
    let main_data = pack_layer3_main_data_payloads(&header, &payloads)?;
    Ok((side_info, main_data))
}

pub(crate) fn pack_mpeg1_layer3_pcm_frame_perceptual_quantized_band_gain_payloads_with_table_provider(
    header: FrameHeader,
    pcm: &AudioBuffer,
    start_frame: usize,
    step: f32,
    band_gain: Layer3QuantizedBandGain,
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<(Layer3SideInfo, PackedBits), Error> {
    let mut side_info = prepare_mpeg1_layer3_pcm_frame_side_info(header, pcm)?;
    let mut payloads = Vec::with_capacity(header.layer3_granule_count() * header.channel_count());
    for granule in 0..header.layer3_granule_count() {
        let granule_start = start_frame
            .checked_add(
                granule
                    .checked_mul(576)
                    .ok_or(Error::InvalidInput("MP3 granule start frame overflows"))?,
            )
            .ok_or(Error::InvalidInput("MP3 granule start frame overflows"))?;
        for channel in 0..header.channel_count() {
            let payload =
                pack_mpeg1_layer3_pcm_long_block_with_perceptual_quantized_band_gain_and_table_provider(
                    &mut side_info.granules[granule][channel],
                    pcm,
                    channel,
                    granule_start,
                    step,
                    band_gain,
                    provider,
                )?;
            payloads.push(payload);
        }
    }
    let main_data = pack_layer3_main_data_payloads(&header, &payloads)?;
    Ok((side_info, main_data))
}

pub(crate) fn pack_mpeg1_layer3_pcm_frame_perceptual_quantized_band_gain_and_global_gain_bias_payloads_with_table_provider(
    header: FrameHeader,
    pcm: &AudioBuffer,
    start_frame: usize,
    step: f32,
    band_gain: Layer3QuantizedBandGain,
    global_gain_bias: i16,
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<(Layer3SideInfo, PackedBits), Error> {
    let mut side_info = prepare_mpeg1_layer3_pcm_frame_side_info(header, pcm)?;
    let mut payloads = Vec::with_capacity(header.layer3_granule_count() * header.channel_count());
    for granule in 0..header.layer3_granule_count() {
        let granule_start = start_frame
            .checked_add(
                granule
                    .checked_mul(576)
                    .ok_or(Error::InvalidInput("MP3 granule start frame overflows"))?,
            )
            .ok_or(Error::InvalidInput("MP3 granule start frame overflows"))?;
        for channel in 0..header.channel_count() {
            let payload =
                pack_mpeg1_layer3_pcm_long_block_with_perceptual_quantized_band_gain_and_global_gain_bias_and_table_provider(
                    &mut side_info.granules[granule][channel],
                    pcm,
                    channel,
                    granule_start,
                    step,
                    band_gain,
                    global_gain_bias,
                    provider,
                )?;
            payloads.push(payload);
        }
    }
    let main_data = pack_layer3_main_data_payloads(&header, &payloads)?;
    Ok((side_info, main_data))
}

pub(crate) fn pack_mpeg1_layer3_pcm_frame_perceptual_quality_guard_payloads_with_table_provider(
    header: FrameHeader,
    pcm: &AudioBuffer,
    start_frame: usize,
    step: f32,
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<(Layer3SideInfo, PackedBits, usize, usize, usize, f64), Error> {
    let mut side_info = prepare_mpeg1_layer3_pcm_frame_side_info(header, pcm)?;
    let mut payloads = Vec::with_capacity(header.layer3_granule_count() * header.channel_count());
    let mut perceptual_granules = 0_usize;
    let mut calibrated_granules = 0_usize;
    let mut quality_guard_compared_granules = 0_usize;
    let mut quality_guard_distortion_delta = 0.0_f64;
    for granule in 0..header.layer3_granule_count() {
        let granule_start = start_frame
            .checked_add(
                granule
                    .checked_mul(576)
                    .ok_or(Error::InvalidInput("MP3 granule start frame overflows"))?,
            )
            .ok_or(Error::InvalidInput("MP3 granule start frame overflows"))?;
        for channel in 0..header.channel_count() {
            let payload =
                pack_mpeg1_layer3_pcm_long_block_with_perceptual_quality_guard_and_table_provider(
                    &mut side_info.granules[granule][channel],
                    pcm,
                    channel,
                    granule_start,
                    step,
                    provider,
                )?;
            if payload.used_perceptual {
                perceptual_granules += 1;
            } else {
                calibrated_granules += 1;
            }
            quality_guard_compared_granules += payload.compared_granules;
            quality_guard_distortion_delta += payload.distortion_delta;
            payloads.push(payload.bits);
        }
    }
    let main_data = pack_layer3_main_data_payloads(&header, &payloads)?;
    Ok((
        side_info,
        main_data,
        perceptual_granules,
        calibrated_granules,
        quality_guard_compared_granules,
        quality_guard_distortion_delta,
    ))
}

pub(crate) fn pack_mpeg1_layer3_pcm_frame_payloads_with_table_provider(
    header: FrameHeader,
    pcm: &AudioBuffer,
    start_frame: usize,
    step: f32,
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<(Layer3SideInfo, PackedBits), Error> {
    let mut side_info = prepare_mpeg1_layer3_pcm_frame_side_info(header, pcm)?;
    let mut payloads = Vec::with_capacity(header.layer3_granule_count() * header.channel_count());
    for granule in 0..header.layer3_granule_count() {
        let granule_start = start_frame
            .checked_add(
                granule
                    .checked_mul(576)
                    .ok_or(Error::InvalidInput("MP3 granule start frame overflows"))?,
            )
            .ok_or(Error::InvalidInput("MP3 granule start frame overflows"))?;
        for channel in 0..header.channel_count() {
            // Rate-aware variant: for the three MPEG-1 rates this resolves to the
            // MPEG-1 region boundaries (byte-identical output); for MPEG-2 LSF it
            // uses the LSF boundaries so the big-value regions match the decoder.
            let payload =
                pack_layer3_pcm_long_block_with_calibrated_gain_for_rate_and_table_provider(
                    &mut side_info.granules[granule][channel],
                    pcm,
                    channel,
                    granule_start,
                    step,
                    provider,
                )?;
            payloads.push(payload);
        }
    }
    let main_data = pack_layer3_main_data_payloads(&header, &payloads)?;
    Ok((side_info, main_data))
}

/// Concatenates Layer III big-values and count1 main-data bits.
pub fn pack_main_data_regions(
    big_values: PackedBits,
    count1: PackedBits,
) -> Result<PackedBits, Error> {
    concat_packed_bits(&[big_values, count1])
}

/// Concatenates Layer III scale-factor, big-values, and count1 main-data bits.
pub fn pack_main_data_parts(
    scale_factors: PackedBits,
    big_values: PackedBits,
    count1: PackedBits,
) -> Result<PackedBits, Error> {
    concat_packed_bits(&[scale_factors, big_values, count1])
}

/// Concatenates Layer III entropy regions and updates side-info length.
pub fn pack_main_data_regions_for_granule(
    granule: &mut Layer3GranuleChannelInfo,
    big_values: PackedBits,
    count1: PackedBits,
) -> Result<PackedBits, Error> {
    let packed = pack_main_data_regions(big_values, count1)?;
    apply_part2_3_length_to_granule(granule, packed.bit_len)?;
    Ok(packed)
}

/// Concatenates Layer III main-data parts and updates side-info length.
pub fn pack_main_data_parts_for_granule(
    granule: &mut Layer3GranuleChannelInfo,
    scale_factors: PackedBits,
    big_values: PackedBits,
    count1: PackedBits,
) -> Result<PackedBits, Error> {
    let packed = pack_main_data_parts(scale_factors, big_values, count1)?;
    apply_part2_3_length_to_granule(granule, packed.bit_len)?;
    Ok(packed)
}

/// Packs Layer III big-values pairs using a caller-supplied Huffman table.
pub fn pack_big_value_pairs_with_table(
    pairs: &[Layer3BigValuePair],
    table: &[HuffmanEntry<Layer3BigValuePair>],
) -> Result<PackedBits, Error> {
    pack_huffman_symbols_with_len(pairs, table)
}

/// Packs Layer III big-values pairs as magnitude codewords followed by sign bits.
pub fn pack_big_value_pairs_with_sign_bits(
    pairs: &[Layer3BigValuePair],
    table: &[HuffmanEntry<Layer3BigValueMagnitude>],
) -> Result<PackedBits, Error> {
    pack_big_value_pairs_with_linbits(pairs, table, 0)
}

/// Packs Layer III big-values pairs with optional escape-table linbits.
pub fn pack_big_value_pairs_with_linbits(
    pairs: &[Layer3BigValuePair],
    table: &[HuffmanEntry<Layer3BigValueMagnitude>],
    linbits: u8,
) -> Result<PackedBits, Error> {
    if linbits > 16 {
        return Err(Error::InvalidInput(
            "MP3 linbits width exceeds supported range",
        ));
    }

    let mut writer = CoreBitWriter::new();
    for pair in pairs {
        let x_magnitude = abs_i16_to_u16(pair.x)?;
        let y_magnitude = abs_i16_to_u16(pair.y)?;
        let table_magnitude = Layer3BigValueMagnitude::new(
            table_magnitude_with_linbits(x_magnitude, linbits)?,
            table_magnitude_with_linbits(y_magnitude, linbits)?,
        );
        let code = lookup_huffman_code(table, &table_magnitude)?;
        writer.write_bits(code.bits, code.len)?;
        // ISO/IEC 11172-3 emits each value's escape linbits immediately before
        // its sign, interleaved per value: linbits_x, sign_x, linbits_y, sign_y.
        // Grouping all linbits before all signs desyncs the decoder.
        write_mp3_linbits(&mut writer, x_magnitude, linbits)?;
        write_mp3_sign_bit(&mut writer, pair.x)?;
        write_mp3_linbits(&mut writer, y_magnitude, linbits)?;
        write_mp3_sign_bit(&mut writer, pair.y)?;
    }
    let bit_len = writer.bit_len();
    Ok(PackedBits {
        bytes: writer.finish_byte_aligned(),
        bit_len,
    })
}

pub(crate) fn pack_big_value_pairs_with_selection(
    pairs: &[Layer3BigValuePair],
    table: &[HuffmanEntry<Layer3BigValueMagnitude>],
    selection: Layer3BigValueTableSelection,
) -> Result<PackedBits, Error> {
    if selection.table_select == 0 {
        if max_big_value_magnitude(pairs)? != 0 {
            return Err(Error::InvalidInput(
                "MP3 table 0 requires zero big-values coefficients",
            ));
        }
        return Ok(PackedBits {
            bytes: Vec::new(),
            bit_len: 0,
        });
    }

    pack_big_value_pairs_with_linbits(pairs, table, selection.linbits)
}

/// Packs Layer III count1 quadruples using a caller-supplied Huffman table.
pub fn pack_count1_quads_with_table(
    quads: &[Layer3Count1Quad],
    table: &[HuffmanEntry<Layer3Count1Quad>],
) -> Result<PackedBits, Error> {
    pack_huffman_symbols_with_len(quads, table)
}

/// Packs Layer III count1 quadruples as magnitude codewords followed by sign bits.
pub fn pack_count1_quads_with_sign_bits(
    quads: &[Layer3Count1Quad],
    table: &[HuffmanEntry<Layer3Count1MagnitudeQuad>],
) -> Result<PackedBits, Error> {
    let mut writer = CoreBitWriter::new();
    for quad in quads {
        let magnitude = Layer3Count1MagnitudeQuad::new(
            count1_abs_to_u8(quad.v)?,
            count1_abs_to_u8(quad.w)?,
            count1_abs_to_u8(quad.x)?,
            count1_abs_to_u8(quad.y)?,
        );
        let code = lookup_huffman_code(table, &magnitude)?;
        writer.write_bits(code.bits, code.len)?;
        write_mp3_sign_bit(&mut writer, i16::from(quad.v))?;
        write_mp3_sign_bit(&mut writer, i16::from(quad.w))?;
        write_mp3_sign_bit(&mut writer, i16::from(quad.x))?;
        write_mp3_sign_bit(&mut writer, i16::from(quad.y))?;
    }
    let bit_len = writer.bit_len();
    Ok(PackedBits {
        bytes: writer.finish_byte_aligned(),
        bit_len,
    })
}

pub(crate) fn pack_count1_quads_with_table_selection(
    quads: &[Layer3Count1Quad],
    table: &[HuffmanEntry<Layer3Count1MagnitudeQuad>],
    selection: Layer3Count1TableSelection,
) -> Result<PackedBits, Error> {
    if selection.max_nonzero_values == 0 {
        return Ok(PackedBits {
            bytes: Vec::new(),
            bit_len: 0,
        });
    }

    pack_count1_quads_with_sign_bits(quads, table)
}

pub(crate) fn abs_i16_to_u16(value: i16) -> Result<u16, Error> {
    let magnitude = value
        .checked_abs()
        .ok_or(Error::InvalidInput("MP3 coefficient magnitude overflows"))?;
    u16::try_from(magnitude).map_err(|_| Error::InvalidInput("MP3 coefficient magnitude overflows"))
}

pub(crate) fn count1_abs_to_u8(value: i8) -> Result<u8, Error> {
    let magnitude = value.checked_abs().ok_or(Error::InvalidInput(
        "MP3 count1 coefficient magnitude overflows",
    ))?;
    if magnitude > 1 {
        return Err(Error::InvalidInput(
            "MP3 count1 coefficient exceeds unit magnitude",
        ));
    }
    u8::try_from(magnitude)
        .map_err(|_| Error::InvalidInput("MP3 count1 coefficient magnitude overflows"))
}

pub(crate) fn max_count1_nonzero_values(quads: &[Layer3Count1Quad]) -> Result<u8, Error> {
    let mut max_nonzero_values = 0_u8;
    for quad in quads {
        let values = [quad.v, quad.w, quad.x, quad.y];
        for value in values {
            count1_abs_to_u8(value)?;
        }
        let nonzero = values.iter().filter(|&&value| value != 0).count();
        max_nonzero_values = max_nonzero_values.max(
            u8::try_from(nonzero)
                .map_err(|_| Error::InvalidInput("MP3 count1 nonzero count overflows"))?,
        );
    }
    Ok(max_nonzero_values)
}

pub(crate) fn max_big_value_magnitude(pairs: &[Layer3BigValuePair]) -> Result<u16, Error> {
    let mut max_magnitude = 0_u16;
    for pair in pairs {
        max_magnitude = max_magnitude.max(abs_i16_to_u16(pair.x)?);
        max_magnitude = max_magnitude.max(abs_i16_to_u16(pair.y)?);
    }
    Ok(max_magnitude)
}

pub(crate) fn linbits_for_big_value_magnitude(max_magnitude: u16) -> Result<u8, Error> {
    if max_magnitude <= 15 {
        return Ok(0);
    }

    let extra = max_magnitude - 15;
    let linbits = (16 - extra.leading_zeros()) as u8;
    if linbits > 13 {
        return Err(Error::InvalidInput(
            "MP3 big-values magnitude exceeds table range",
        ));
    }
    Ok(linbits)
}

pub(crate) fn prepare_mpeg1_layer3_pcm_frame_side_info(
    header: FrameHeader,
    pcm: &AudioBuffer,
) -> Result<Layer3SideInfo, Error> {
    // MPEG-1 and MPEG-2 LSF Layer III share this single-granule-aware side-info
    // layout; MPEG-2.5 (ISO-unspecified) is not produced by this encoder.
    if header.version == MpegVersion::Mpeg25 || header.layer != Layer::Layer3 {
        return Err(Error::UnsupportedFeature(
            "MP3 PCM frame payload currently requires MPEG-1 or MPEG-2 LSF Layer III",
        ));
    }
    if header.sample_rate != pcm.sample_rate {
        return Err(Error::InvalidInput(
            "MP3 header sample rate does not match PCM",
        ));
    }
    if header.channel_count() != usize::from(pcm.channels) {
        return Err(Error::InvalidInput(
            "MP3 header channel count does not match PCM",
        ));
    }

    Ok(Layer3SideInfo::silent(&header))
}

pub(crate) fn mpeg1_layer3_header_for_pcm(pcm: &AudioBuffer) -> Result<FrameHeader, Error> {
    if pcm.channels != 1 && pcm.channels != 2 {
        return Err(Error::UnsupportedFeature(
            "MP3 encode currently supports mono/stereo only",
        ));
    }

    let header = FrameHeader {
        version: MpegVersion::Mpeg1,
        layer: Layer::Layer3,
        protection_absent: true,
        bitrate_kbps: 128,
        sample_rate: pcm.sample_rate,
        padding: false,
        channel_mode: if pcm.channels == 1 {
            ChannelMode::SingleChannel
        } else {
            ChannelMode::Stereo
        },
    };
    header.to_bytes()?;
    Ok(header)
}

pub(crate) fn mpeg2_layer3_header_for_pcm(
    pcm: &AudioBuffer,
    bitrate_kbps: u16,
) -> Result<FrameHeader, Error> {
    if pcm.channels != 1 && pcm.channels != 2 {
        return Err(Error::UnsupportedFeature(
            "MP3 encode currently supports mono/stereo only",
        ));
    }
    if !matches!(pcm.sample_rate, 16_000 | 22_050 | 24_000) {
        return Err(Error::UnsupportedFeature(
            "MPEG-2 LSF Layer III encode supports 16/22.05/24 kHz",
        ));
    }

    let header = FrameHeader {
        version: MpegVersion::Mpeg2,
        layer: Layer::Layer3,
        protection_absent: true,
        bitrate_kbps,
        sample_rate: pcm.sample_rate,
        padding: false,
        channel_mode: if pcm.channels == 1 {
            ChannelMode::SingleChannel
        } else {
            ChannelMode::Stereo
        },
    };
    header.to_bytes()?;
    Ok(header)
}

pub(crate) fn layer3_frame_count(header: FrameHeader, pcm: &AudioBuffer) -> Result<usize, Error> {
    prepare_mpeg1_layer3_pcm_frame_side_info(header, pcm)?;
    Ok(pcm
        .frames()
        .div_ceil(usize::from(header.samples_per_frame())))
}
