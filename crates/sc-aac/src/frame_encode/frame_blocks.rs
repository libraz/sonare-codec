use super::*;

pub fn encode_pcm_mono_long_block_adts(
    adts: AdtsConfig,
    channel: AacLongBlockConfig,
    pcm: &AudioBuffer,
    pcm_config: AacPcmLongBlockConfig,
    tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<u8>, Error> {
    if adts.channels != 1 || pcm.channels != 1 {
        return Err(Error::InvalidInput(
            "AAC mono PCM encode requires one-channel ADTS and PCM",
        ));
    }
    let quantized = quantize_pcm_long_block(pcm, 0, pcm_config.start_frame, pcm_config.step)?;
    encode_quantized_mono_adts(adts, channel, &quantized, pcm_config.band_width, tables)
}

/// Encodes one mono AAC-LC ADTS frame from PCM using bit-cost section planning.
pub fn encode_pcm_mono_long_block_adts_by_bit_cost(
    adts: AdtsConfig,
    channel: AacLongBlockConfig,
    pcm: &AudioBuffer,
    pcm_config: AacPcmLongBlockConfig,
    tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<u8>, Error> {
    if adts.channels != 1 || pcm.channels != 1 {
        return Err(Error::InvalidInput(
            "AAC mono PCM encode requires one-channel ADTS and PCM",
        ));
    }
    let quantized = quantize_pcm_long_block(pcm, 0, pcm_config.start_frame, pcm_config.step)?;
    encode_quantized_mono_adts_by_bit_cost(adts, channel, &quantized, pcm_config.band_width, tables)
}

/// Encodes one mono AAC-LC ADTS frame from PCM with scale-factor DPCM payload.
pub fn encode_pcm_mono_long_block_adts_with_scale_factors(
    adts: AdtsConfig,
    channel: AacScaleFactorChannel<'_>,
    pcm: &AudioBuffer,
    pcm_config: AacPcmLongBlockConfig,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<u8>, Error> {
    if adts.channels != 1 || pcm.channels != 1 {
        return Err(Error::InvalidInput(
            "AAC mono PCM encode requires one-channel ADTS and PCM",
        ));
    }
    let quantized = quantize_pcm_long_block(pcm, 0, pcm_config.start_frame, pcm_config.step)?;
    encode_quantized_mono_adts_with_scale_factors(
        adts,
        channel.config,
        &quantized,
        pcm_config.band_width,
        channel.scale_factors,
        scale_factor_table,
        spectral_tables,
    )
}

/// Encodes one mono AAC-LC ADTS frame from PCM with scale factors and bit-cost sections.
pub fn encode_pcm_mono_long_block_adts_with_scale_factors_by_bit_cost(
    adts: AdtsConfig,
    channel: AacScaleFactorChannel<'_>,
    pcm: &AudioBuffer,
    pcm_config: AacPcmLongBlockConfig,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<u8>, Error> {
    if adts.channels != 1 || pcm.channels != 1 {
        return Err(Error::InvalidInput(
            "AAC mono PCM encode requires one-channel ADTS and PCM",
        ));
    }
    let quantized = quantize_pcm_long_block(pcm, 0, pcm_config.start_frame, pcm_config.step)?;
    encode_quantized_mono_adts_with_scale_factors_by_bit_cost(
        adts,
        channel.config,
        &quantized,
        pcm_config.band_width,
        channel.scale_factors,
        scale_factor_table,
        spectral_tables,
    )
}

/// Encodes one mono AAC-LC ADTS frame from PCM with internally selected scale factors.
pub fn encode_pcm_mono_long_block_adts_with_selected_scale_factors(
    adts: AdtsConfig,
    channel: AacLongBlockConfig,
    pcm: &AudioBuffer,
    pcm_config: AacPcmLongBlockConfig,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<u8>, Error> {
    if adts.channels != 1 || pcm.channels != 1 {
        return Err(Error::InvalidInput(
            "AAC mono PCM encode requires one-channel ADTS and PCM",
        ));
    }
    let quantized = quantize_pcm_long_block(pcm, 0, pcm_config.start_frame, pcm_config.step)?;
    encode_quantized_mono_adts_with_selected_scale_factors(
        adts,
        channel,
        &quantized,
        pcm_config.band_width,
        scale_factor_table,
        spectral_tables,
    )
}

/// Encodes one mono AAC-LC ADTS frame from PCM with selected scale factors and bit-cost sections.
pub fn encode_pcm_mono_long_block_adts_with_selected_scale_factors_by_bit_cost(
    adts: AdtsConfig,
    channel: AacLongBlockConfig,
    pcm: &AudioBuffer,
    pcm_config: AacPcmLongBlockConfig,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<u8>, Error> {
    if adts.channels != 1 || pcm.channels != 1 {
        return Err(Error::InvalidInput(
            "AAC mono PCM encode requires one-channel ADTS and PCM",
        ));
    }
    let quantized = quantize_pcm_long_block(pcm, 0, pcm_config.start_frame, pcm_config.step)?;
    encode_quantized_mono_adts_with_selected_scale_factors_by_bit_cost(
        adts,
        channel,
        &quantized,
        pcm_config.band_width,
        scale_factor_table,
        spectral_tables,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn encode_pcm_mono_long_block_adts_with_offsets_and_selected_scale_factors_by_bit_cost(
    adts: AdtsConfig,
    channel: AacLongBlockConfig,
    pcm: &AudioBuffer,
    start_frame: usize,
    step: f32,
    offsets: &[usize],
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<u8>, Error> {
    if adts.channels != 1 || pcm.channels != 1 {
        return Err(Error::InvalidInput(
            "AAC mono PCM encode requires one-channel ADTS and PCM",
        ));
    }
    if offsets.len() < 2 {
        return Err(Error::InvalidInput("AAC scale-factor offsets are empty"));
    }
    let global_gain = aac_uniform_scale_factor_for_step(step)?;
    let channel = AacLongBlockConfig::new(global_gain, channel.max_sfb);
    let quantized = quantize_pcm_long_block(pcm, 0, start_frame, step)?;
    let scale_factors = vec![i16::from(global_gain); offsets.len() - 1];
    encode_quantized_mono_adts_with_offsets_and_scale_factors_by_bit_cost(
        adts,
        channel,
        &quantized,
        offsets,
        &scale_factors,
        scale_factor_table,
        spectral_tables,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn encode_pcm_mono_long_block_adts_with_offsets_and_scale_factors_by_bit_cost(
    adts: AdtsConfig,
    channel: AacScaleFactorChannel<'_>,
    pcm: &AudioBuffer,
    start_frame: usize,
    step: f32,
    offsets: &[usize],
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<u8>, Error> {
    if adts.channels != 1 || pcm.channels != 1 {
        return Err(Error::InvalidInput(
            "AAC mono PCM encode requires one-channel ADTS and PCM",
        ));
    }
    let quantized = quantize_pcm_long_block(pcm, 0, start_frame, step)?;
    encode_quantized_mono_adts_with_offsets_and_scale_factors_by_bit_cost(
        adts,
        channel.config,
        &quantized,
        offsets,
        channel.scale_factors,
        scale_factor_table,
        spectral_tables,
    )
}

pub fn encode_pcm_mono_long_block_adts_with_standard_spectral_offsets_and_scale_factors_by_bit_cost(
    adts: AdtsConfig,
    channel: AacScaleFactorChannel<'_>,
    pcm: &AudioBuffer,
    start_frame: usize,
    step: f32,
    offsets: &[usize],
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
) -> Result<Vec<u8>, Error> {
    if adts.channels != 1 || pcm.channels != 1 {
        return Err(Error::InvalidInput(
            "AAC mono PCM encode requires one-channel ADTS and PCM",
        ));
    }
    let quantized = quantize_pcm_long_block(pcm, 0, start_frame, step)?;
    encode_quantized_mono_adts_with_standard_spectral_offsets_and_scale_factors_by_bit_cost(
        adts,
        channel.config,
        &quantized,
        offsets,
        channel.scale_factors,
        scale_factor_table,
    )
}

pub fn encode_pcm_mono_long_block_adts_with_standard_spectral_offsets_and_selected_scale_factors_by_bit_cost(
    adts: AdtsConfig,
    channel: AacLongBlockConfig,
    pcm: &AudioBuffer,
    start_frame: usize,
    step: f32,
    offsets: &[usize],
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
) -> Result<Vec<u8>, Error> {
    encode_pcm_mono_long_block_adts_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_by_bit_cost(
        adts,
        channel,
        pcm,
        start_frame,
        step,
        offsets,
        0,
        scale_factor_table,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn encode_pcm_mono_long_block_adts_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_by_bit_cost(
    adts: AdtsConfig,
    channel: AacLongBlockConfig,
    pcm: &AudioBuffer,
    start_frame: usize,
    step: f32,
    offsets: &[usize],
    scale_factor_magnitude_bias: i16,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
) -> Result<Vec<u8>, Error> {
    if adts.channels != 1 || pcm.channels != 1 {
        return Err(Error::InvalidInput(
            "AAC mono PCM encode requires one-channel ADTS and PCM",
        ));
    }
    let quantized = quantize_pcm_long_block(pcm, 0, start_frame, step)?;
    encode_quantized_mono_adts_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_by_bit_cost(
        adts,
        channel,
        &quantized,
        offsets,
        scale_factor_magnitude_bias,
        scale_factor_table,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn encode_pcm_stereo_long_block_adts_with_offsets_and_scale_factors_by_bit_cost(
    adts: AdtsConfig,
    left: AacScaleFactorChannel<'_>,
    right: AacScaleFactorChannel<'_>,
    pcm: &AudioBuffer,
    start_frame: usize,
    step: f32,
    offsets: &[usize],
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<u8>, Error> {
    if adts.channels != 2 || pcm.channels != 2 {
        return Err(Error::InvalidInput(
            "AAC stereo PCM encode requires two-channel ADTS and PCM",
        ));
    }
    let left_quantized = quantize_pcm_long_block(pcm, 0, start_frame, step)?;
    let right_quantized = quantize_pcm_long_block(pcm, 1, start_frame, step)?;
    encode_quantized_stereo_adts_with_offsets_and_scale_factors_by_bit_cost(
        adts,
        AacQuantizedChannel::new(left.config, &left_quantized, left.scale_factors),
        AacQuantizedChannel::new(right.config, &right_quantized, right.scale_factors),
        offsets,
        scale_factor_table,
        spectral_tables,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn encode_pcm_stereo_long_block_adts_with_standard_spectral_offsets_and_scale_factors_by_bit_cost(
    adts: AdtsConfig,
    left: AacScaleFactorChannel<'_>,
    right: AacScaleFactorChannel<'_>,
    pcm: &AudioBuffer,
    start_frame: usize,
    step: f32,
    offsets: &[usize],
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
) -> Result<Vec<u8>, Error> {
    if adts.channels != 2 || pcm.channels != 2 {
        return Err(Error::InvalidInput(
            "AAC stereo PCM encode requires two-channel ADTS and PCM",
        ));
    }
    let left_quantized = quantize_pcm_long_block(pcm, 0, start_frame, step)?;
    let right_quantized = quantize_pcm_long_block(pcm, 1, start_frame, step)?;
    encode_quantized_stereo_adts_with_standard_spectral_offsets_and_scale_factors_by_bit_cost(
        adts,
        AacQuantizedChannel::new(left.config, &left_quantized, left.scale_factors),
        AacQuantizedChannel::new(right.config, &right_quantized, right.scale_factors),
        offsets,
        scale_factor_table,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn encode_pcm_stereo_long_block_adts_with_standard_spectral_offsets_and_selected_scale_factors_by_bit_cost(
    adts: AdtsConfig,
    left: AacLongBlockConfig,
    right: AacLongBlockConfig,
    pcm: &AudioBuffer,
    start_frame: usize,
    step: f32,
    offsets: &[usize],
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
) -> Result<Vec<u8>, Error> {
    encode_pcm_stereo_long_block_adts_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_by_bit_cost(
        adts,
        left,
        right,
        pcm,
        start_frame,
        step,
        offsets,
        0,
        scale_factor_table,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn encode_pcm_stereo_long_block_adts_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_by_bit_cost(
    adts: AdtsConfig,
    left: AacLongBlockConfig,
    right: AacLongBlockConfig,
    pcm: &AudioBuffer,
    start_frame: usize,
    step: f32,
    offsets: &[usize],
    scale_factor_magnitude_bias: i16,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
) -> Result<Vec<u8>, Error> {
    if adts.channels != 2 || pcm.channels != 2 {
        return Err(Error::InvalidInput(
            "AAC stereo PCM encode requires two-channel ADTS and PCM",
        ));
    }
    let left_quantized = quantize_pcm_long_block(pcm, 0, start_frame, step)?;
    let right_quantized = quantize_pcm_long_block(pcm, 1, start_frame, step)?;
    encode_quantized_stereo_adts_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_by_bit_cost(
        adts,
        AacQuantizedSpectrum::new(left, &left_quantized),
        AacQuantizedSpectrum::new(right, &right_quantized),
        offsets,
        scale_factor_magnitude_bias,
        scale_factor_table,
    )
}

/// Encodes one independent-stereo AAC-LC ADTS frame from PCM long analysis blocks.
pub fn encode_pcm_stereo_long_block_adts(
    adts: AdtsConfig,
    left: AacLongBlockConfig,
    right: AacLongBlockConfig,
    pcm: &AudioBuffer,
    pcm_config: AacPcmLongBlockConfig,
    tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<u8>, Error> {
    if adts.channels != 2 || pcm.channels != 2 {
        return Err(Error::InvalidInput(
            "AAC stereo PCM encode requires two-channel ADTS and PCM",
        ));
    }
    let left_quantized = quantize_pcm_long_block(pcm, 0, pcm_config.start_frame, pcm_config.step)?;
    let right_quantized = quantize_pcm_long_block(pcm, 1, pcm_config.start_frame, pcm_config.step)?;
    encode_quantized_stereo_adts(
        adts,
        left,
        &left_quantized,
        right,
        &right_quantized,
        pcm_config.band_width,
        tables,
    )
}

/// Encodes one independent-stereo AAC-LC ADTS frame from PCM using bit-cost section planning.
pub fn encode_pcm_stereo_long_block_adts_by_bit_cost(
    adts: AdtsConfig,
    left: AacLongBlockConfig,
    right: AacLongBlockConfig,
    pcm: &AudioBuffer,
    pcm_config: AacPcmLongBlockConfig,
    tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<u8>, Error> {
    if adts.channels != 2 || pcm.channels != 2 {
        return Err(Error::InvalidInput(
            "AAC stereo PCM encode requires two-channel ADTS and PCM",
        ));
    }
    let left_quantized = quantize_pcm_long_block(pcm, 0, pcm_config.start_frame, pcm_config.step)?;
    let right_quantized = quantize_pcm_long_block(pcm, 1, pcm_config.start_frame, pcm_config.step)?;
    encode_quantized_stereo_adts_by_bit_cost(
        adts,
        left,
        &left_quantized,
        right,
        &right_quantized,
        pcm_config.band_width,
        tables,
    )
}

/// Encodes one independent-stereo AAC-LC ADTS frame from PCM with scale-factor DPCM payloads.
pub fn encode_pcm_stereo_long_block_adts_with_scale_factors(
    adts: AdtsConfig,
    left: AacScaleFactorChannel<'_>,
    right: AacScaleFactorChannel<'_>,
    pcm: &AudioBuffer,
    pcm_config: AacPcmLongBlockConfig,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<u8>, Error> {
    if adts.channels != 2 || pcm.channels != 2 {
        return Err(Error::InvalidInput(
            "AAC stereo PCM encode requires two-channel ADTS and PCM",
        ));
    }
    let left_quantized = quantize_pcm_long_block(pcm, 0, pcm_config.start_frame, pcm_config.step)?;
    let right_quantized = quantize_pcm_long_block(pcm, 1, pcm_config.start_frame, pcm_config.step)?;
    encode_quantized_stereo_adts_with_scale_factors(
        adts,
        AacQuantizedChannel::new(left.config, &left_quantized, left.scale_factors),
        AacQuantizedChannel::new(right.config, &right_quantized, right.scale_factors),
        pcm_config.band_width,
        scale_factor_table,
        spectral_tables,
    )
}

/// Encodes one independent-stereo AAC-LC ADTS frame from PCM with scale factors and bit-cost sections.
pub fn encode_pcm_stereo_long_block_adts_with_scale_factors_by_bit_cost(
    adts: AdtsConfig,
    left: AacScaleFactorChannel<'_>,
    right: AacScaleFactorChannel<'_>,
    pcm: &AudioBuffer,
    pcm_config: AacPcmLongBlockConfig,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<u8>, Error> {
    if adts.channels != 2 || pcm.channels != 2 {
        return Err(Error::InvalidInput(
            "AAC stereo PCM encode requires two-channel ADTS and PCM",
        ));
    }
    let left_quantized = quantize_pcm_long_block(pcm, 0, pcm_config.start_frame, pcm_config.step)?;
    let right_quantized = quantize_pcm_long_block(pcm, 1, pcm_config.start_frame, pcm_config.step)?;
    encode_quantized_stereo_adts_with_scale_factors_by_bit_cost(
        adts,
        AacQuantizedChannel::new(left.config, &left_quantized, left.scale_factors),
        AacQuantizedChannel::new(right.config, &right_quantized, right.scale_factors),
        pcm_config.band_width,
        scale_factor_table,
        spectral_tables,
    )
}

/// Encodes one independent-stereo AAC-LC ADTS frame from PCM with internally selected scale factors.
pub fn encode_pcm_stereo_long_block_adts_with_selected_scale_factors(
    adts: AdtsConfig,
    left: AacLongBlockConfig,
    right: AacLongBlockConfig,
    pcm: &AudioBuffer,
    pcm_config: AacPcmLongBlockConfig,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<u8>, Error> {
    if adts.channels != 2 || pcm.channels != 2 {
        return Err(Error::InvalidInput(
            "AAC stereo PCM encode requires two-channel ADTS and PCM",
        ));
    }
    let left_quantized = quantize_pcm_long_block(pcm, 0, pcm_config.start_frame, pcm_config.step)?;
    let right_quantized = quantize_pcm_long_block(pcm, 1, pcm_config.start_frame, pcm_config.step)?;
    encode_quantized_stereo_adts_with_selected_scale_factors(
        adts,
        AacQuantizedSpectrum::new(left, &left_quantized),
        AacQuantizedSpectrum::new(right, &right_quantized),
        pcm_config.band_width,
        scale_factor_table,
        spectral_tables,
    )
}

/// Encodes one independent-stereo AAC-LC ADTS frame from PCM with selected scale factors and bit-cost sections.
pub fn encode_pcm_stereo_long_block_adts_with_selected_scale_factors_by_bit_cost(
    adts: AdtsConfig,
    left: AacLongBlockConfig,
    right: AacLongBlockConfig,
    pcm: &AudioBuffer,
    pcm_config: AacPcmLongBlockConfig,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<u8>, Error> {
    if adts.channels != 2 || pcm.channels != 2 {
        return Err(Error::InvalidInput(
            "AAC stereo PCM encode requires two-channel ADTS and PCM",
        ));
    }
    let left_quantized = quantize_pcm_long_block(pcm, 0, pcm_config.start_frame, pcm_config.step)?;
    let right_quantized = quantize_pcm_long_block(pcm, 1, pcm_config.start_frame, pcm_config.step)?;
    encode_quantized_stereo_adts_with_selected_scale_factors_by_bit_cost(
        adts,
        AacQuantizedSpectrum::new(left, &left_quantized),
        AacQuantizedSpectrum::new(right, &right_quantized),
        pcm_config.band_width,
        scale_factor_table,
        spectral_tables,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn encode_pcm_stereo_long_block_adts_with_offsets_and_selected_scale_factors_by_bit_cost(
    adts: AdtsConfig,
    left: AacLongBlockConfig,
    right: AacLongBlockConfig,
    pcm: &AudioBuffer,
    start_frame: usize,
    step: f32,
    offsets: &[usize],
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<u8>, Error> {
    if adts.channels != 2 || pcm.channels != 2 {
        return Err(Error::InvalidInput(
            "AAC stereo PCM encode requires two-channel ADTS and PCM",
        ));
    }
    if offsets.len() < 2 {
        return Err(Error::InvalidInput("AAC scale-factor offsets are empty"));
    }
    let global_gain = aac_uniform_scale_factor_for_step(step)?;
    let left = AacLongBlockConfig::new(global_gain, left.max_sfb);
    let right = AacLongBlockConfig::new(global_gain, right.max_sfb);
    let left_quantized = quantize_pcm_long_block(pcm, 0, start_frame, step)?;
    let right_quantized = quantize_pcm_long_block(pcm, 1, start_frame, step)?;
    let scale_factors = vec![i16::from(global_gain); offsets.len() - 1];
    encode_quantized_stereo_adts_with_offsets_and_scale_factors_by_bit_cost(
        adts,
        AacQuantizedChannel::new(left, &left_quantized, &scale_factors),
        AacQuantizedChannel::new(right, &right_quantized, &scale_factors),
        offsets,
        scale_factor_table,
        spectral_tables,
    )
}
