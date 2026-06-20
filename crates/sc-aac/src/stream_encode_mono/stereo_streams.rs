use super::*;

/// Encodes an independent-stereo AAC-LC ADTS stream from PCM using 1024-frame hops.
pub fn encode_pcm_stereo_long_block_adts_stream(
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

    let mut out = Vec::new();
    for start_frame in pcm_frame_starts(pcm, pcm_config.start_frame)? {
        out.extend_from_slice(&encode_pcm_stereo_long_block_adts(
            adts,
            left,
            right,
            pcm,
            AacPcmLongBlockConfig::new(start_frame, pcm_config.step, pcm_config.band_width),
            tables,
        )?);
    }
    Ok(out)
}

/// Encodes an independent-stereo AAC-LC ADTS stream from PCM using bit-cost section planning.
pub fn encode_pcm_stereo_long_block_adts_stream_by_bit_cost(
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

    let mut out = Vec::new();
    for start_frame in pcm_frame_starts(pcm, pcm_config.start_frame)? {
        out.extend_from_slice(&encode_pcm_stereo_long_block_adts_by_bit_cost(
            adts,
            left,
            right,
            pcm,
            AacPcmLongBlockConfig::new(start_frame, pcm_config.step, pcm_config.band_width),
            tables,
        )?);
    }
    Ok(out)
}

/// Encodes an independent-stereo AAC-LC ADTS stream from PCM with per-frame scale-factor payloads.
pub fn encode_pcm_stereo_long_block_adts_stream_with_scale_factors(
    adts: AdtsConfig,
    left: AacScaleFactorSequence<'_>,
    right: AacScaleFactorSequence<'_>,
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

    let starts = pcm_frame_starts(pcm, pcm_config.start_frame)?;
    if starts.len() != left.scale_factors_by_frame.len()
        || starts.len() != right.scale_factors_by_frame.len()
    {
        return Err(Error::InvalidInput(
            "AAC scale-factor frame count does not match PCM frame count",
        ));
    }

    let mut out = Vec::new();
    for (frame_index, start_frame) in starts.into_iter().enumerate() {
        out.extend_from_slice(&encode_pcm_stereo_long_block_adts_with_scale_factors(
            adts,
            left.channel_for_frame(frame_index)?,
            right.channel_for_frame(frame_index)?,
            pcm,
            AacPcmLongBlockConfig::new(start_frame, pcm_config.step, pcm_config.band_width),
            scale_factor_table,
            spectral_tables,
        )?);
    }
    Ok(out)
}

/// Encodes an independent-stereo AAC-LC ADTS stream from PCM with per-frame scale factors and bit-cost sections.
pub fn encode_pcm_stereo_long_block_adts_stream_with_scale_factors_by_bit_cost(
    adts: AdtsConfig,
    left: AacScaleFactorSequence<'_>,
    right: AacScaleFactorSequence<'_>,
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

    let starts = pcm_frame_starts(pcm, pcm_config.start_frame)?;
    if starts.len() != left.scale_factors_by_frame.len()
        || starts.len() != right.scale_factors_by_frame.len()
    {
        return Err(Error::InvalidInput(
            "AAC scale-factor frame count does not match PCM frame count",
        ));
    }

    let mut out = Vec::new();
    for (frame_index, start_frame) in starts.into_iter().enumerate() {
        out.extend_from_slice(
            &encode_pcm_stereo_long_block_adts_with_scale_factors_by_bit_cost(
                adts,
                left.channel_for_frame(frame_index)?,
                right.channel_for_frame(frame_index)?,
                pcm,
                AacPcmLongBlockConfig::new(start_frame, pcm_config.step, pcm_config.band_width),
                scale_factor_table,
                spectral_tables,
            )?,
        );
    }
    Ok(out)
}

#[allow(clippy::too_many_arguments)]
pub fn encode_pcm_stereo_long_block_adts_stream_with_standard_spectral_offsets_and_scale_factors_by_bit_cost(
    adts: AdtsConfig,
    left: AacScaleFactorSequence<'_>,
    right: AacScaleFactorSequence<'_>,
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

    let starts = pcm_frame_starts(pcm, start_frame)?;
    if starts.len() != left.scale_factors_by_frame.len()
        || starts.len() != right.scale_factors_by_frame.len()
    {
        return Err(Error::InvalidInput(
            "AAC scale-factor frame count does not match PCM frame count",
        ));
    }

    let mut out = Vec::new();
    for (frame_index, start_frame) in starts.into_iter().enumerate() {
        out.extend_from_slice(
            &encode_pcm_stereo_long_block_adts_with_standard_spectral_offsets_and_scale_factors_by_bit_cost(
                adts,
                left.channel_for_frame(frame_index)?,
                right.channel_for_frame(frame_index)?,
                pcm,
                start_frame,
                step,
                offsets,
                scale_factor_table,
            )?,
        );
    }
    Ok(out)
}

#[allow(clippy::too_many_arguments)]
pub fn encode_pcm_stereo_long_block_adts_stream_with_standard_spectral_offsets_and_scale_factors_and_max_frame_len_by_bit_cost(
    adts: AdtsConfig,
    left: AacScaleFactorSequence<'_>,
    right: AacScaleFactorSequence<'_>,
    pcm: &AudioBuffer,
    start_frame: usize,
    offsets: &[usize],
    candidates: &[f32],
    max_frame_len_bytes: usize,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
) -> Result<Vec<u8>, Error> {
    let selections =
        select_aac_lc_stereo_pcm_stream_frame_details_with_standard_spectral_offsets_and_scale_factors_and_max_frame_len_by_bit_cost(
            adts,
            left,
            right,
            pcm,
            start_frame,
            offsets,
            candidates,
            max_frame_len_bytes,
            scale_factor_table,
        )?;
    let starts = pcm_frame_starts(pcm, start_frame)?;
    let mut out = Vec::new();
    for (frame_index, (start_frame, selection)) in
        starts.into_iter().zip(selections.into_iter()).enumerate()
    {
        let left_channel = left.channel_for_frame(frame_index)?;
        let right_channel = right.channel_for_frame(frame_index)?;
        out.extend_from_slice(
            &encode_pcm_stereo_long_block_adts_with_standard_spectral_offsets_and_scale_factors_by_bit_cost(
                adts,
                left_channel,
                right_channel,
                pcm,
                start_frame,
                selection.step,
                offsets,
                scale_factor_table,
            )?,
        );
    }
    Ok(out)
}

#[allow(clippy::too_many_arguments)]
pub fn encode_pcm_stereo_long_block_adts_stream_with_standard_spectral_offsets_and_scale_factors_and_bitrate_by_bit_cost(
    adts: AdtsConfig,
    left: AacScaleFactorSequence<'_>,
    right: AacScaleFactorSequence<'_>,
    pcm: &AudioBuffer,
    start_frame: usize,
    offsets: &[usize],
    candidates: &[f32],
    target_bitrate_bps: u32,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
) -> Result<Vec<u8>, Error> {
    let max_frame_len_bytes =
        aac_lc_adts_max_frame_len_for_bitrate(adts.sample_rate, target_bitrate_bps)?;
    encode_pcm_stereo_long_block_adts_stream_with_standard_spectral_offsets_and_scale_factors_and_max_frame_len_by_bit_cost(
        adts,
        left,
        right,
        pcm,
        start_frame,
        offsets,
        candidates,
        max_frame_len_bytes,
        scale_factor_table,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn encode_pcm_stereo_long_block_adts_stream_with_standard_spectral_offsets_and_selected_scale_factors_by_bit_cost(
    adts: AdtsConfig,
    left: AacLongBlockConfig,
    right: AacLongBlockConfig,
    pcm: &AudioBuffer,
    start_frame: usize,
    step: f32,
    offsets: &[usize],
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
) -> Result<Vec<u8>, Error> {
    encode_pcm_stereo_long_block_adts_stream_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_by_bit_cost(
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
pub fn encode_pcm_stereo_long_block_adts_stream_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_by_bit_cost(
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

    let mut out = Vec::new();
    for start_frame in pcm_frame_starts(pcm, start_frame)? {
        out.extend_from_slice(
            &encode_pcm_stereo_long_block_adts_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_by_bit_cost(
                adts,
                left,
                right,
                pcm,
                start_frame,
                step,
                offsets,
                scale_factor_magnitude_bias,
                scale_factor_table,
            )?,
        );
    }
    Ok(out)
}

#[allow(clippy::too_many_arguments)]
pub fn encode_pcm_stereo_long_block_adts_stream_with_standard_spectral_offsets_and_selected_scale_factors_and_max_frame_len_by_bit_cost(
    adts: AdtsConfig,
    left: AacLongBlockConfig,
    right: AacLongBlockConfig,
    pcm: &AudioBuffer,
    start_frame: usize,
    offsets: &[usize],
    candidates: &[f32],
    max_frame_len_bytes: usize,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
) -> Result<Vec<u8>, Error> {
    encode_pcm_stereo_long_block_adts_stream_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_max_frame_len_by_bit_cost(
        adts,
        left,
        right,
        pcm,
        start_frame,
        offsets,
        0,
        candidates,
        max_frame_len_bytes,
        scale_factor_table,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn encode_pcm_stereo_long_block_adts_stream_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_max_frame_len_by_bit_cost(
    adts: AdtsConfig,
    left: AacLongBlockConfig,
    right: AacLongBlockConfig,
    pcm: &AudioBuffer,
    start_frame: usize,
    offsets: &[usize],
    scale_factor_magnitude_bias: i16,
    candidates: &[f32],
    max_frame_len_bytes: usize,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
) -> Result<Vec<u8>, Error> {
    let selections =
        select_aac_lc_stereo_pcm_stream_frame_details_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_max_frame_len_by_bit_cost(
            adts,
            left,
            right,
            pcm,
            start_frame,
            offsets,
            scale_factor_magnitude_bias,
            candidates,
            max_frame_len_bytes,
            scale_factor_table,
        )?;
    let starts = pcm_frame_starts(pcm, start_frame)?;
    let mut out = Vec::new();
    for (start_frame, selection) in starts.into_iter().zip(selections.into_iter()) {
        out.extend_from_slice(
            &encode_pcm_stereo_long_block_adts_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_by_bit_cost(
                adts,
                left,
                right,
                pcm,
                start_frame,
                selection.step,
                offsets,
                scale_factor_magnitude_bias,
                scale_factor_table,
            )?,
        );
    }
    Ok(out)
}

#[allow(clippy::too_many_arguments)]
pub fn encode_pcm_stereo_long_block_adts_stream_with_standard_spectral_offsets_and_selected_scale_factors_and_bitrate_by_bit_cost(
    adts: AdtsConfig,
    left: AacLongBlockConfig,
    right: AacLongBlockConfig,
    pcm: &AudioBuffer,
    start_frame: usize,
    offsets: &[usize],
    candidates: &[f32],
    target_bitrate_bps: u32,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
) -> Result<Vec<u8>, Error> {
    encode_pcm_stereo_long_block_adts_stream_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_bitrate_by_bit_cost(
        adts,
        left,
        right,
        pcm,
        start_frame,
        offsets,
        0,
        candidates,
        target_bitrate_bps,
        scale_factor_table,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn encode_pcm_stereo_long_block_adts_stream_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_bitrate_by_bit_cost(
    adts: AdtsConfig,
    left: AacLongBlockConfig,
    right: AacLongBlockConfig,
    pcm: &AudioBuffer,
    start_frame: usize,
    offsets: &[usize],
    scale_factor_magnitude_bias: i16,
    candidates: &[f32],
    target_bitrate_bps: u32,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
) -> Result<Vec<u8>, Error> {
    let max_frame_len_bytes =
        aac_lc_adts_max_frame_len_for_bitrate(adts.sample_rate, target_bitrate_bps)?;
    encode_pcm_stereo_long_block_adts_stream_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_max_frame_len_by_bit_cost(
        adts,
        left,
        right,
        pcm,
        start_frame,
        offsets,
        scale_factor_magnitude_bias,
        candidates,
        max_frame_len_bytes,
        scale_factor_table,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn encode_pcm_stereo_long_block_adts_stream_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_max_quantized_abs_and_bitrate_by_bit_cost(
    adts: AdtsConfig,
    left: AacLongBlockConfig,
    right: AacLongBlockConfig,
    pcm: &AudioBuffer,
    start_frame: usize,
    offsets: &[usize],
    scale_factor_magnitude_bias: i16,
    candidates: &[f32],
    max_quantized_abs: u32,
    target_bitrate_bps: u32,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
) -> Result<Vec<u8>, Error> {
    let selections =
        select_aac_lc_stereo_pcm_stream_frame_details_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_max_quantized_abs_and_bitrate_by_bit_cost(
            adts,
            left,
            right,
            pcm,
            start_frame,
            offsets,
            scale_factor_magnitude_bias,
            candidates,
            max_quantized_abs,
            target_bitrate_bps,
            scale_factor_table,
        )?;
    let starts = pcm_frame_starts(pcm, start_frame)?;
    let mut out = Vec::new();
    for (start_frame, selection) in starts.into_iter().zip(selections.into_iter()) {
        out.extend_from_slice(
            &encode_pcm_stereo_long_block_adts_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_by_bit_cost(
                adts,
                left,
                right,
                pcm,
                start_frame,
                selection.step,
                offsets,
                scale_factor_magnitude_bias,
                scale_factor_table,
            )?,
        );
    }
    Ok(out)
}

/// Encodes an independent-stereo AAC-LC ADTS stream from PCM with internally selected scale factors.
pub fn encode_pcm_stereo_long_block_adts_stream_with_selected_scale_factors(
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

    let mut out = Vec::new();
    for start_frame in pcm_frame_starts(pcm, pcm_config.start_frame)? {
        out.extend_from_slice(
            &encode_pcm_stereo_long_block_adts_with_selected_scale_factors(
                adts,
                left,
                right,
                pcm,
                AacPcmLongBlockConfig::new(start_frame, pcm_config.step, pcm_config.band_width),
                scale_factor_table,
                spectral_tables,
            )?,
        );
    }
    Ok(out)
}

/// Encodes an independent-stereo AAC-LC ADTS stream from PCM with selected scale factors and bit-cost sections.
pub fn encode_pcm_stereo_long_block_adts_stream_with_selected_scale_factors_by_bit_cost(
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

    let mut out = Vec::new();
    for start_frame in pcm_frame_starts(pcm, pcm_config.start_frame)? {
        out.extend_from_slice(
            &encode_pcm_stereo_long_block_adts_with_selected_scale_factors_by_bit_cost(
                adts,
                left,
                right,
                pcm,
                AacPcmLongBlockConfig::new(start_frame, pcm_config.step, pcm_config.band_width),
                scale_factor_table,
                spectral_tables,
            )?,
        );
    }
    Ok(out)
}
