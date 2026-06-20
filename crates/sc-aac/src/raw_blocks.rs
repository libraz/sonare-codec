use super::*;

/// Packs an AAC-LC long-block individual_channel_stream with caller-built payload bits.
pub fn pack_long_block_individual_channel_stream(
    config: AacLongBlockConfig,
    sectioned_spectral_payload: &PackedBits,
) -> Result<PackedBits, Error> {
    pack_long_block_individual_channel_stream_parts(
        config,
        &AacIndividualChannelPayload::new(
            sectioned_spectral_payload.clone(),
            PackedBits {
                bytes: Vec::new(),
                bit_len: 0,
            },
        ),
    )
}

/// Packs an AAC-LC long-block individual_channel_stream with separated pre-spectral and spectral bits.
pub fn pack_long_block_individual_channel_stream_parts(
    config: AacLongBlockConfig,
    payload: &AacIndividualChannelPayload,
) -> Result<PackedBits, Error> {
    if config.max_sfb > 63 {
        return Err(Error::InvalidInput("AAC max_sfb exceeds 6-bit range"));
    }
    if config.max_sfb == 0
        && (payload.section_and_scale_factor_bits.bit_len != 0
            || payload.spectral_bits.bit_len != 0)
    {
        return Err(Error::InvalidInput(
            "AAC zero max_sfb cannot carry spectral payload",
        ));
    }

    let mut writer = CoreBitWriter::new();
    writer.write_bits(u32::from(config.global_gain), 8)?;
    writer.write_bits(0, 1)?; // ics_reserved_bit
    writer.write_bits(0, 2)?; // ONLY_LONG_SEQUENCE
    writer.write_bits(0, 1)?; // window_shape
    writer.write_bits(u32::from(config.max_sfb), 6)?;
    writer.write_bits(0, 1)?; // predictor_data_present
    write_packed_bits(&mut writer, &payload.section_and_scale_factor_bits)?;
    writer.write_bits(0, 1)?; // pulse_data_present
    writer.write_bits(0, 1)?; // tns_data_present
    writer.write_bits(0, 1)?; // gain_control_data_present
    write_packed_bits(&mut writer, &payload.spectral_bits)?;
    let bit_len = writer.bit_len();
    Ok(PackedBits {
        bytes: writer.finish_byte_aligned(),
        bit_len,
    })
}

/// Packs one single_channel_element raw_data_block with a long-block ICS.
pub fn pack_single_channel_raw_data_block(
    config: AacLongBlockConfig,
    sectioned_spectral_payload: &PackedBits,
) -> Result<Vec<u8>, Error> {
    pack_single_channel_raw_data_block_parts(
        config,
        &AacIndividualChannelPayload::new(
            sectioned_spectral_payload.clone(),
            PackedBits {
                bytes: Vec::new(),
                bit_len: 0,
            },
        ),
    )
}

/// Packs one single_channel_element raw_data_block with separated long-block ICS payload parts.
pub fn pack_single_channel_raw_data_block_parts(
    config: AacLongBlockConfig,
    payload: &AacIndividualChannelPayload,
) -> Result<Vec<u8>, Error> {
    let ics = pack_long_block_individual_channel_stream_parts(config, payload)?;
    let mut writer = CoreBitWriter::new();
    writer.write_bits(0, 3)?; // ID_SCE
    writer.write_bits(0, 4)?; // element_instance_tag
    write_packed_bits(&mut writer, &ics)?;
    writer.write_bits(7, 3)?; // ID_END
    Ok(writer.finish_byte_aligned())
}

/// Packs one channel_pair_element raw_data_block with two independent long-block ICS payloads.
pub fn pack_channel_pair_raw_data_block(
    left_config: AacLongBlockConfig,
    left_sectioned_spectral_payload: &PackedBits,
    right_config: AacLongBlockConfig,
    right_sectioned_spectral_payload: &PackedBits,
) -> Result<Vec<u8>, Error> {
    pack_channel_pair_raw_data_block_parts(
        left_config,
        &AacIndividualChannelPayload::new(
            left_sectioned_spectral_payload.clone(),
            PackedBits {
                bytes: Vec::new(),
                bit_len: 0,
            },
        ),
        right_config,
        &AacIndividualChannelPayload::new(
            right_sectioned_spectral_payload.clone(),
            PackedBits {
                bytes: Vec::new(),
                bit_len: 0,
            },
        ),
    )
}

/// Packs one channel_pair_element raw_data_block with separated long-block ICS payload parts.
pub fn pack_channel_pair_raw_data_block_parts(
    left_config: AacLongBlockConfig,
    left_payload: &AacIndividualChannelPayload,
    right_config: AacLongBlockConfig,
    right_payload: &AacIndividualChannelPayload,
) -> Result<Vec<u8>, Error> {
    let left = pack_long_block_individual_channel_stream_parts(left_config, left_payload)?;
    let right = pack_long_block_individual_channel_stream_parts(right_config, right_payload)?;

    let mut writer = CoreBitWriter::new();
    writer.write_bits(1, 3)?; // ID_CPE
    writer.write_bits(0, 4)?; // element_instance_tag
    writer.write_bits(0, 1)?; // common_window
    write_packed_bits(&mut writer, &left)?;
    write_packed_bits(&mut writer, &right)?;
    writer.write_bits(7, 3)?; // ID_END
    Ok(writer.finish_byte_aligned())
}

/// Encodes one mono AAC-LC ADTS frame from a pre-quantized long-block spectrum.
pub fn encode_quantized_mono_adts(
    adts: AdtsConfig,
    channel: AacLongBlockConfig,
    quantized: &[i32],
    band_width: usize,
    tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<u8>, Error> {
    if adts.channels != 1 {
        return Err(Error::InvalidInput(
            "AAC mono ADTS config must have one channel",
        ));
    }
    let sections = plan_sections(quantized, band_width)?;
    let payload =
        split_sectioned_spectral_payload_with_sign_bits(&sections, quantized, band_width, tables)?;
    let access_unit = pack_single_channel_raw_data_block_parts(channel, &payload)?;
    frame_adts(adts, &access_unit)
}

/// Encodes one mono AAC-LC ADTS frame using bit-cost section planning.
pub fn encode_quantized_mono_adts_by_bit_cost(
    adts: AdtsConfig,
    channel: AacLongBlockConfig,
    quantized: &[i32],
    band_width: usize,
    tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<u8>, Error> {
    if adts.channels != 1 {
        return Err(Error::InvalidInput(
            "AAC mono ADTS config must have one channel",
        ));
    }
    let sections = plan_sections_by_bit_cost(quantized, band_width, tables)?;
    let payload =
        split_sectioned_spectral_payload_with_sign_bits(&sections, quantized, band_width, tables)?;
    let access_unit = pack_single_channel_raw_data_block_parts(channel, &payload)?;
    frame_adts(adts, &access_unit)
}

/// Encodes one mono AAC-LC ADTS frame with scale-factor DPCM payload.
pub fn encode_quantized_mono_adts_with_scale_factors(
    adts: AdtsConfig,
    channel: AacLongBlockConfig,
    quantized: &[i32],
    band_width: usize,
    scale_factors: &[i16],
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<u8>, Error> {
    if adts.channels != 1 {
        return Err(Error::InvalidInput(
            "AAC mono ADTS config must have one channel",
        ));
    }
    let sections = plan_sections(quantized, band_width)?;
    let scale_factor_deltas = plan_scale_factor_deltas(
        &sections,
        band_width,
        scale_factors,
        i16::from(channel.global_gain),
    )?;
    let scale_factor_bits =
        pack_scale_factor_deltas_with_table(&scale_factor_deltas, scale_factor_table)?;
    let payload = split_sectioned_spectral_payload_with_sign_bits_and_scale_factor_bits(
        &sections,
        quantized,
        band_width,
        scale_factor_bits,
        spectral_tables,
    )?;
    let access_unit = pack_single_channel_raw_data_block_parts(channel, &payload)?;
    frame_adts(adts, &access_unit)
}

/// Encodes one mono AAC-LC ADTS frame with scale-factor DPCM and bit-cost section planning.
pub fn encode_quantized_mono_adts_with_scale_factors_by_bit_cost(
    adts: AdtsConfig,
    channel: AacLongBlockConfig,
    quantized: &[i32],
    band_width: usize,
    scale_factors: &[i16],
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<u8>, Error> {
    if adts.channels != 1 {
        return Err(Error::InvalidInput(
            "AAC mono ADTS config must have one channel",
        ));
    }
    let sections = plan_sections_by_bit_cost(quantized, band_width, spectral_tables)?;
    let scale_factor_deltas = plan_scale_factor_deltas(
        &sections,
        band_width,
        scale_factors,
        i16::from(channel.global_gain),
    )?;
    let scale_factor_bits =
        pack_scale_factor_deltas_with_table(&scale_factor_deltas, scale_factor_table)?;
    let payload = split_sectioned_spectral_payload_with_sign_bits_and_scale_factor_bits(
        &sections,
        quantized,
        band_width,
        scale_factor_bits,
        spectral_tables,
    )?;
    let access_unit = pack_single_channel_raw_data_block_parts(channel, &payload)?;
    frame_adts(adts, &access_unit)
}

/// Encodes one mono AAC-LC ADTS frame with internally selected scale factors.
pub fn encode_quantized_mono_adts_with_selected_scale_factors(
    adts: AdtsConfig,
    channel: AacLongBlockConfig,
    quantized: &[i32],
    band_width: usize,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<u8>, Error> {
    let scale_factors = select_scale_factors_for_quantized_bands(
        quantized,
        band_width,
        i16::from(channel.global_gain),
    )?;
    encode_quantized_mono_adts_with_scale_factors(
        adts,
        channel,
        quantized,
        band_width,
        &scale_factors,
        scale_factor_table,
        spectral_tables,
    )
}

/// Encodes one mono AAC-LC ADTS frame with selected scale factors and bit-cost sections.
pub fn encode_quantized_mono_adts_with_selected_scale_factors_by_bit_cost(
    adts: AdtsConfig,
    channel: AacLongBlockConfig,
    quantized: &[i32],
    band_width: usize,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<u8>, Error> {
    let scale_factors = select_scale_factors_for_quantized_bands(
        quantized,
        band_width,
        i16::from(channel.global_gain),
    )?;
    encode_quantized_mono_adts_with_scale_factors_by_bit_cost(
        adts,
        channel,
        quantized,
        band_width,
        &scale_factors,
        scale_factor_table,
        spectral_tables,
    )
}

pub fn encode_quantized_mono_adts_with_offsets_and_selected_scale_factors_by_bit_cost(
    adts: AdtsConfig,
    channel: AacLongBlockConfig,
    quantized: &[i32],
    offsets: &[usize],
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<u8>, Error> {
    if adts.channels != 1 {
        return Err(Error::InvalidInput(
            "AAC mono ADTS config must have one channel",
        ));
    }
    validate_scale_factor_band_offsets(quantized, offsets)?;
    let max_sfb = offsets.len() - 1;
    if usize::from(channel.max_sfb) != max_sfb {
        return Err(Error::InvalidInput(
            "AAC max_sfb must match scale-factor band count",
        ));
    }

    let sections = plan_magnitude_sections_by_offsets(quantized, offsets, spectral_tables)?;
    let scale_factors = select_scale_factors_for_quantized_bands_by_offsets(
        quantized,
        offsets,
        i16::from(channel.global_gain),
    )?;
    let scale_factor_deltas = plan_magnitude_scale_factor_deltas_by_offsets(
        &sections,
        offsets,
        &scale_factors,
        i16::from(channel.global_gain),
    )?;
    let scale_factor_bits =
        pack_scale_factor_deltas_with_table(&scale_factor_deltas, scale_factor_table)?;
    let payload = split_magnitude_sectioned_spectral_payload_with_offsets_and_scale_factor_bits(
        &sections,
        quantized,
        offsets,
        scale_factor_bits,
    )?;
    let access_unit = pack_single_channel_raw_data_block_parts(channel, &payload)?;
    frame_adts(adts, &access_unit)
}

pub fn encode_quantized_mono_adts_with_offsets_and_scale_factors_by_bit_cost(
    adts: AdtsConfig,
    channel: AacLongBlockConfig,
    quantized: &[i32],
    offsets: &[usize],
    scale_factors: &[i16],
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<u8>, Error> {
    if adts.channels != 1 {
        return Err(Error::InvalidInput(
            "AAC mono ADTS config must have one channel",
        ));
    }
    validate_scale_factor_band_offsets(quantized, offsets)?;
    let max_sfb = offsets.len() - 1;
    if usize::from(channel.max_sfb) != max_sfb {
        return Err(Error::InvalidInput(
            "AAC max_sfb must match scale-factor band count",
        ));
    }

    let sections = plan_magnitude_sections_by_offsets(quantized, offsets, spectral_tables)?;
    let scale_factor_deltas = plan_magnitude_scale_factor_deltas_by_offsets(
        &sections,
        offsets,
        scale_factors,
        i16::from(channel.global_gain),
    )?;
    let scale_factor_bits =
        pack_scale_factor_deltas_with_table(&scale_factor_deltas, scale_factor_table)?;
    let payload = split_magnitude_sectioned_spectral_payload_with_offsets_and_scale_factor_bits(
        &sections,
        quantized,
        offsets,
        scale_factor_bits,
    )?;
    let access_unit = pack_single_channel_raw_data_block_parts(channel, &payload)?;
    frame_adts(adts, &access_unit)
}

pub fn encode_quantized_mono_adts_with_standard_spectral_offsets_and_scale_factors_by_bit_cost(
    adts: AdtsConfig,
    channel: AacLongBlockConfig,
    quantized: &[i32],
    offsets: &[usize],
    scale_factors: &[i16],
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
) -> Result<Vec<u8>, Error> {
    if adts.channels != 1 {
        return Err(Error::InvalidInput(
            "AAC mono ADTS config must have one channel",
        ));
    }
    validate_scale_factor_band_offsets(quantized, offsets)?;
    let max_sfb = offsets.len() - 1;
    if usize::from(channel.max_sfb) != max_sfb {
        return Err(Error::InvalidInput(
            "AAC max_sfb must match scale-factor band count",
        ));
    }

    let sections =
        plan_aac_lc_standard_spectral_sections_by_offsets_by_bit_cost(quantized, offsets)?;
    let scale_factor_deltas = plan_spectral_scale_factor_deltas_by_offsets(
        &sections,
        offsets,
        scale_factors,
        i16::from(channel.global_gain),
    )?;
    let scale_factor_bits =
        pack_scale_factor_deltas_with_table(&scale_factor_deltas, scale_factor_table)?;
    let payload =
        split_aac_lc_standard_spectral_payload_with_offsets_and_sign_bits_and_scale_factor_bits_by_bit_cost(
            quantized,
            offsets,
            scale_factor_bits,
        )?;
    let access_unit = pack_single_channel_raw_data_block_parts(channel, &payload)?;
    frame_adts(adts, &access_unit)
}

pub fn encode_quantized_mono_adts_with_standard_spectral_offsets_and_selected_scale_factors_by_bit_cost(
    adts: AdtsConfig,
    channel: AacLongBlockConfig,
    quantized: &[i32],
    offsets: &[usize],
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
) -> Result<Vec<u8>, Error> {
    encode_quantized_mono_adts_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_by_bit_cost(
        adts,
        channel,
        quantized,
        offsets,
        0,
        scale_factor_table,
    )
}

pub fn encode_quantized_mono_adts_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_by_bit_cost(
    adts: AdtsConfig,
    channel: AacLongBlockConfig,
    quantized: &[i32],
    offsets: &[usize],
    scale_factor_magnitude_bias: i16,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
) -> Result<Vec<u8>, Error> {
    let scale_factors = select_scale_factors_for_quantized_bands_by_offsets_with_magnitude_bias(
        quantized,
        offsets,
        i16::from(channel.global_gain),
        scale_factor_magnitude_bias,
    )?;
    encode_quantized_mono_adts_with_standard_spectral_offsets_and_scale_factors_by_bit_cost(
        adts,
        channel,
        quantized,
        offsets,
        &scale_factors,
        scale_factor_table,
    )
}

pub fn encode_quantized_stereo_adts_with_offsets_and_scale_factors_by_bit_cost(
    adts: AdtsConfig,
    left: AacQuantizedChannel<'_>,
    right: AacQuantizedChannel<'_>,
    offsets: &[usize],
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<u8>, Error> {
    if adts.channels != 2 {
        return Err(Error::InvalidInput(
            "AAC stereo ADTS config must have two channels",
        ));
    }
    validate_scale_factor_band_offsets(left.quantized, offsets)?;
    validate_scale_factor_band_offsets(right.quantized, offsets)?;
    let max_sfb = offsets.len() - 1;
    if usize::from(left.config.max_sfb) != max_sfb || usize::from(right.config.max_sfb) != max_sfb {
        return Err(Error::InvalidInput(
            "AAC max_sfb must match scale-factor band count",
        ));
    }

    let left_sections =
        plan_magnitude_sections_by_offsets(left.quantized, offsets, spectral_tables)?;
    let right_sections =
        plan_magnitude_sections_by_offsets(right.quantized, offsets, spectral_tables)?;
    let left_scale_factor_deltas = plan_magnitude_scale_factor_deltas_by_offsets(
        &left_sections,
        offsets,
        left.scale_factors,
        i16::from(left.config.global_gain),
    )?;
    let right_scale_factor_deltas = plan_magnitude_scale_factor_deltas_by_offsets(
        &right_sections,
        offsets,
        right.scale_factors,
        i16::from(right.config.global_gain),
    )?;
    let left_scale_factor_bits =
        pack_scale_factor_deltas_with_table(&left_scale_factor_deltas, scale_factor_table)?;
    let right_scale_factor_bits =
        pack_scale_factor_deltas_with_table(&right_scale_factor_deltas, scale_factor_table)?;
    let left_payload =
        split_magnitude_sectioned_spectral_payload_with_offsets_and_scale_factor_bits(
            &left_sections,
            left.quantized,
            offsets,
            left_scale_factor_bits,
        )?;
    let right_payload =
        split_magnitude_sectioned_spectral_payload_with_offsets_and_scale_factor_bits(
            &right_sections,
            right.quantized,
            offsets,
            right_scale_factor_bits,
        )?;
    let access_unit = pack_channel_pair_raw_data_block_parts(
        left.config,
        &left_payload,
        right.config,
        &right_payload,
    )?;
    frame_adts(adts, &access_unit)
}

pub fn encode_quantized_stereo_adts_with_standard_spectral_offsets_and_scale_factors_by_bit_cost(
    adts: AdtsConfig,
    left: AacQuantizedChannel<'_>,
    right: AacQuantizedChannel<'_>,
    offsets: &[usize],
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
) -> Result<Vec<u8>, Error> {
    if adts.channels != 2 {
        return Err(Error::InvalidInput(
            "AAC stereo ADTS config must have two channels",
        ));
    }
    validate_scale_factor_band_offsets(left.quantized, offsets)?;
    validate_scale_factor_band_offsets(right.quantized, offsets)?;
    let max_sfb = offsets.len() - 1;
    if usize::from(left.config.max_sfb) != max_sfb || usize::from(right.config.max_sfb) != max_sfb {
        return Err(Error::InvalidInput(
            "AAC max_sfb must match scale-factor band count",
        ));
    }

    let left_sections =
        plan_aac_lc_standard_spectral_sections_by_offsets_by_bit_cost(left.quantized, offsets)?;
    let right_sections =
        plan_aac_lc_standard_spectral_sections_by_offsets_by_bit_cost(right.quantized, offsets)?;
    let left_scale_factor_deltas = plan_spectral_scale_factor_deltas_by_offsets(
        &left_sections,
        offsets,
        left.scale_factors,
        i16::from(left.config.global_gain),
    )?;
    let right_scale_factor_deltas = plan_spectral_scale_factor_deltas_by_offsets(
        &right_sections,
        offsets,
        right.scale_factors,
        i16::from(right.config.global_gain),
    )?;
    let left_scale_factor_bits =
        pack_scale_factor_deltas_with_table(&left_scale_factor_deltas, scale_factor_table)?;
    let right_scale_factor_bits =
        pack_scale_factor_deltas_with_table(&right_scale_factor_deltas, scale_factor_table)?;
    let left_payload =
        split_aac_lc_standard_spectral_payload_with_offsets_and_sign_bits_and_scale_factor_bits_by_bit_cost(
            left.quantized,
            offsets,
            left_scale_factor_bits,
        )?;
    let right_payload =
        split_aac_lc_standard_spectral_payload_with_offsets_and_sign_bits_and_scale_factor_bits_by_bit_cost(
            right.quantized,
            offsets,
            right_scale_factor_bits,
        )?;
    let access_unit = pack_channel_pair_raw_data_block_parts(
        left.config,
        &left_payload,
        right.config,
        &right_payload,
    )?;
    frame_adts(adts, &access_unit)
}

pub fn encode_quantized_stereo_adts_with_standard_spectral_offsets_and_selected_scale_factors_by_bit_cost(
    adts: AdtsConfig,
    left: AacQuantizedSpectrum<'_>,
    right: AacQuantizedSpectrum<'_>,
    offsets: &[usize],
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
) -> Result<Vec<u8>, Error> {
    encode_quantized_stereo_adts_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_by_bit_cost(
        adts,
        left,
        right,
        offsets,
        0,
        scale_factor_table,
    )
}

pub fn encode_quantized_stereo_adts_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_by_bit_cost(
    adts: AdtsConfig,
    left: AacQuantizedSpectrum<'_>,
    right: AacQuantizedSpectrum<'_>,
    offsets: &[usize],
    scale_factor_magnitude_bias: i16,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
) -> Result<Vec<u8>, Error> {
    let left_scale_factors =
        select_scale_factors_for_quantized_bands_by_offsets_with_magnitude_bias(
            left.quantized,
            offsets,
            i16::from(left.config.global_gain),
            scale_factor_magnitude_bias,
        )?;
    let right_scale_factors =
        select_scale_factors_for_quantized_bands_by_offsets_with_magnitude_bias(
            right.quantized,
            offsets,
            i16::from(right.config.global_gain),
            scale_factor_magnitude_bias,
        )?;
    encode_quantized_stereo_adts_with_standard_spectral_offsets_and_scale_factors_by_bit_cost(
        adts,
        AacQuantizedChannel::new(left.config, left.quantized, &left_scale_factors),
        AacQuantizedChannel::new(right.config, right.quantized, &right_scale_factors),
        offsets,
        scale_factor_table,
    )
}

/// Encodes one independent-stereo AAC-LC ADTS frame from pre-quantized long-block spectra.
pub fn encode_quantized_stereo_adts(
    adts: AdtsConfig,
    left: AacLongBlockConfig,
    left_quantized: &[i32],
    right: AacLongBlockConfig,
    right_quantized: &[i32],
    band_width: usize,
    tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<u8>, Error> {
    if adts.channels != 2 {
        return Err(Error::InvalidInput(
            "AAC stereo ADTS config must have two channels",
        ));
    }
    let left_sections = plan_sections(left_quantized, band_width)?;
    let right_sections = plan_sections(right_quantized, band_width)?;
    let left_payload = split_sectioned_spectral_payload_with_sign_bits(
        &left_sections,
        left_quantized,
        band_width,
        tables,
    )?;
    let right_payload = split_sectioned_spectral_payload_with_sign_bits(
        &right_sections,
        right_quantized,
        band_width,
        tables,
    )?;
    let access_unit =
        pack_channel_pair_raw_data_block_parts(left, &left_payload, right, &right_payload)?;
    frame_adts(adts, &access_unit)
}

/// Encodes one independent-stereo AAC-LC ADTS frame using bit-cost section planning.
pub fn encode_quantized_stereo_adts_by_bit_cost(
    adts: AdtsConfig,
    left: AacLongBlockConfig,
    left_quantized: &[i32],
    right: AacLongBlockConfig,
    right_quantized: &[i32],
    band_width: usize,
    tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<u8>, Error> {
    if adts.channels != 2 {
        return Err(Error::InvalidInput(
            "AAC stereo ADTS config must have two channels",
        ));
    }
    let left_sections = plan_sections_by_bit_cost(left_quantized, band_width, tables)?;
    let right_sections = plan_sections_by_bit_cost(right_quantized, band_width, tables)?;
    let left_payload = split_sectioned_spectral_payload_with_sign_bits(
        &left_sections,
        left_quantized,
        band_width,
        tables,
    )?;
    let right_payload = split_sectioned_spectral_payload_with_sign_bits(
        &right_sections,
        right_quantized,
        band_width,
        tables,
    )?;
    let access_unit =
        pack_channel_pair_raw_data_block_parts(left, &left_payload, right, &right_payload)?;
    frame_adts(adts, &access_unit)
}

/// Encodes one independent-stereo AAC-LC ADTS frame with scale-factor DPCM payloads.
pub fn encode_quantized_stereo_adts_with_scale_factors(
    adts: AdtsConfig,
    left: AacQuantizedChannel<'_>,
    right: AacQuantizedChannel<'_>,
    band_width: usize,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<u8>, Error> {
    if adts.channels != 2 {
        return Err(Error::InvalidInput(
            "AAC stereo ADTS config must have two channels",
        ));
    }
    let left_sections = plan_sections(left.quantized, band_width)?;
    let right_sections = plan_sections(right.quantized, band_width)?;
    let left_scale_factor_deltas = plan_scale_factor_deltas(
        &left_sections,
        band_width,
        left.scale_factors,
        i16::from(left.config.global_gain),
    )?;
    let right_scale_factor_deltas = plan_scale_factor_deltas(
        &right_sections,
        band_width,
        right.scale_factors,
        i16::from(right.config.global_gain),
    )?;
    let left_scale_factor_bits =
        pack_scale_factor_deltas_with_table(&left_scale_factor_deltas, scale_factor_table)?;
    let right_scale_factor_bits =
        pack_scale_factor_deltas_with_table(&right_scale_factor_deltas, scale_factor_table)?;
    let left_payload = split_sectioned_spectral_payload_with_sign_bits_and_scale_factor_bits(
        &left_sections,
        left.quantized,
        band_width,
        left_scale_factor_bits,
        spectral_tables,
    )?;
    let right_payload = split_sectioned_spectral_payload_with_sign_bits_and_scale_factor_bits(
        &right_sections,
        right.quantized,
        band_width,
        right_scale_factor_bits,
        spectral_tables,
    )?;
    let access_unit = pack_channel_pair_raw_data_block_parts(
        left.config,
        &left_payload,
        right.config,
        &right_payload,
    )?;
    frame_adts(adts, &access_unit)
}

/// Encodes one independent-stereo AAC-LC ADTS frame with scale factors and bit-cost sections.
pub fn encode_quantized_stereo_adts_with_scale_factors_by_bit_cost(
    adts: AdtsConfig,
    left: AacQuantizedChannel<'_>,
    right: AacQuantizedChannel<'_>,
    band_width: usize,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<u8>, Error> {
    if adts.channels != 2 {
        return Err(Error::InvalidInput(
            "AAC stereo ADTS config must have two channels",
        ));
    }
    let left_sections = plan_sections_by_bit_cost(left.quantized, band_width, spectral_tables)?;
    let right_sections = plan_sections_by_bit_cost(right.quantized, band_width, spectral_tables)?;
    let left_scale_factor_deltas = plan_scale_factor_deltas(
        &left_sections,
        band_width,
        left.scale_factors,
        i16::from(left.config.global_gain),
    )?;
    let right_scale_factor_deltas = plan_scale_factor_deltas(
        &right_sections,
        band_width,
        right.scale_factors,
        i16::from(right.config.global_gain),
    )?;
    let left_scale_factor_bits =
        pack_scale_factor_deltas_with_table(&left_scale_factor_deltas, scale_factor_table)?;
    let right_scale_factor_bits =
        pack_scale_factor_deltas_with_table(&right_scale_factor_deltas, scale_factor_table)?;
    let left_payload = split_sectioned_spectral_payload_with_sign_bits_and_scale_factor_bits(
        &left_sections,
        left.quantized,
        band_width,
        left_scale_factor_bits,
        spectral_tables,
    )?;
    let right_payload = split_sectioned_spectral_payload_with_sign_bits_and_scale_factor_bits(
        &right_sections,
        right.quantized,
        band_width,
        right_scale_factor_bits,
        spectral_tables,
    )?;
    let access_unit = pack_channel_pair_raw_data_block_parts(
        left.config,
        &left_payload,
        right.config,
        &right_payload,
    )?;
    frame_adts(adts, &access_unit)
}

/// Encodes one independent-stereo AAC-LC ADTS frame with internally selected scale factors.
pub fn encode_quantized_stereo_adts_with_selected_scale_factors(
    adts: AdtsConfig,
    left: AacQuantizedSpectrum<'_>,
    right: AacQuantizedSpectrum<'_>,
    band_width: usize,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<u8>, Error> {
    let left_scale_factors = select_scale_factors_for_quantized_bands(
        left.quantized,
        band_width,
        i16::from(left.config.global_gain),
    )?;
    let right_scale_factors = select_scale_factors_for_quantized_bands(
        right.quantized,
        band_width,
        i16::from(right.config.global_gain),
    )?;
    encode_quantized_stereo_adts_with_scale_factors(
        adts,
        AacQuantizedChannel::new(left.config, left.quantized, &left_scale_factors),
        AacQuantizedChannel::new(right.config, right.quantized, &right_scale_factors),
        band_width,
        scale_factor_table,
        spectral_tables,
    )
}

/// Encodes one independent-stereo AAC-LC ADTS frame with selected scale factors and bit-cost sections.
pub fn encode_quantized_stereo_adts_with_selected_scale_factors_by_bit_cost(
    adts: AdtsConfig,
    left: AacQuantizedSpectrum<'_>,
    right: AacQuantizedSpectrum<'_>,
    band_width: usize,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<u8>, Error> {
    let left_scale_factors = select_scale_factors_for_quantized_bands(
        left.quantized,
        band_width,
        i16::from(left.config.global_gain),
    )?;
    let right_scale_factors = select_scale_factors_for_quantized_bands(
        right.quantized,
        band_width,
        i16::from(right.config.global_gain),
    )?;
    encode_quantized_stereo_adts_with_scale_factors_by_bit_cost(
        adts,
        AacQuantizedChannel::new(left.config, left.quantized, &left_scale_factors),
        AacQuantizedChannel::new(right.config, right.quantized, &right_scale_factors),
        band_width,
        scale_factor_table,
        spectral_tables,
    )
}

pub fn encode_quantized_stereo_adts_with_offsets_and_selected_scale_factors_by_bit_cost(
    adts: AdtsConfig,
    left: AacQuantizedSpectrum<'_>,
    right: AacQuantizedSpectrum<'_>,
    offsets: &[usize],
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<u8>, Error> {
    let left_scale_factors = select_scale_factors_for_quantized_bands_by_offsets(
        left.quantized,
        offsets,
        i16::from(left.config.global_gain),
    )?;
    let right_scale_factors = select_scale_factors_for_quantized_bands_by_offsets(
        right.quantized,
        offsets,
        i16::from(right.config.global_gain),
    )?;
    encode_quantized_stereo_adts_with_offsets_and_scale_factors_by_bit_cost(
        adts,
        AacQuantizedChannel::new(left.config, left.quantized, &left_scale_factors),
        AacQuantizedChannel::new(right.config, right.quantized, &right_scale_factors),
        offsets,
        scale_factor_table,
        spectral_tables,
    )
}
