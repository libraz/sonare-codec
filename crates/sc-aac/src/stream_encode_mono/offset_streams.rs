use super::*;

#[allow(clippy::too_many_arguments)]
pub fn select_aac_lc_mono_pcm_stream_frame_details_with_offsets_and_selected_scale_factors_and_max_frame_len_by_bit_cost(
    adts: AdtsConfig,
    channel: AacLongBlockConfig,
    pcm: &AudioBuffer,
    offsets: &[usize],
    candidates: &[f32],
    max_frame_len_bytes: usize,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<AacPcmFrameStepSelection>, Error> {
    if adts.channels != 1 || pcm.channels != 1 {
        return Err(Error::InvalidInput(
            "AAC mono PCM frame detail selection requires one-channel ADTS and PCM",
        ));
    }

    pcm_frame_starts(pcm, 0)?
        .into_iter()
        .map(|start_frame| {
            select_aac_lc_mono_pcm_frame_step_details_with_offsets_and_max_frame_len_by_bit_cost(
                adts,
                channel,
                pcm,
                start_frame,
                offsets,
                candidates,
                max_frame_len_bytes,
                scale_factor_table,
                spectral_tables,
            )
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
pub fn select_aac_lc_mono_pcm_stream_frame_details_with_offsets_and_selected_scale_factors_and_bitrate_by_bit_cost(
    adts: AdtsConfig,
    channel: AacLongBlockConfig,
    pcm: &AudioBuffer,
    offsets: &[usize],
    candidates: &[f32],
    target_bitrate_bps: u32,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<AacPcmFrameStepSelection>, Error> {
    let max_frame_len_bytes =
        aac_lc_adts_max_frame_len_for_bitrate(adts.sample_rate, target_bitrate_bps)?;
    select_aac_lc_mono_pcm_stream_frame_details_with_offsets_and_selected_scale_factors_and_max_frame_len_by_bit_cost(
        adts,
        channel,
        pcm,
        offsets,
        candidates,
        max_frame_len_bytes,
        scale_factor_table,
        spectral_tables,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn encode_pcm_mono_long_block_adts_stream_with_offsets_and_selected_scale_factors_and_bitrate_by_bit_cost(
    adts: AdtsConfig,
    channel: AacLongBlockConfig,
    pcm: &AudioBuffer,
    offsets: &[usize],
    candidates: &[f32],
    target_bitrate_bps: u32,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<u8>, Error> {
    let max_frame_len_bytes =
        aac_lc_adts_max_frame_len_for_bitrate(adts.sample_rate, target_bitrate_bps)?;
    encode_pcm_mono_long_block_adts_stream_with_offsets_and_selected_scale_factors_and_max_frame_len_by_bit_cost(
        adts,
        channel,
        pcm,
        offsets,
        candidates,
        max_frame_len_bytes,
        scale_factor_table,
        spectral_tables,
    )
}

pub fn encode_pcm_mono_long_block_adts_stream_with_offsets_and_scale_factors_by_bit_cost(
    adts: AdtsConfig,
    channel: AacScaleFactorChannel<'_>,
    pcm: &AudioBuffer,
    offsets: &[usize],
    candidates: &[f32],
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<u8>, Error> {
    if adts.channels != 1 || pcm.channels != 1 {
        return Err(Error::InvalidInput(
            "AAC mono PCM encode requires one-channel ADTS and PCM",
        ));
    }

    let starts = pcm_frame_starts(pcm, 0)?;
    let mut out = Vec::with_capacity(adts_stream_capacity(starts.len()));
    for start_frame in starts {
        let step = select_aac_lc_mono_pcm_frame_step_with_offsets_and_scale_factors_by_bit_cost(
            adts,
            channel,
            pcm,
            start_frame,
            offsets,
            candidates,
            scale_factor_table,
            spectral_tables,
        )?;
        out.extend_from_slice(
            &encode_pcm_mono_long_block_adts_with_offsets_and_scale_factors_by_bit_cost(
                adts,
                channel,
                pcm,
                start_frame,
                step,
                offsets,
                scale_factor_table,
                spectral_tables,
            )?,
        );
    }
    Ok(out)
}

#[allow(clippy::too_many_arguments)]
pub fn encode_pcm_mono_long_block_adts_stream_with_offsets_and_scale_factors_and_max_frame_len_by_bit_cost(
    adts: AdtsConfig,
    channel: AacScaleFactorChannel<'_>,
    pcm: &AudioBuffer,
    offsets: &[usize],
    candidates: &[f32],
    max_frame_len_bytes: usize,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<u8>, Error> {
    if adts.channels != 1 || pcm.channels != 1 {
        return Err(Error::InvalidInput(
            "AAC mono PCM encode requires one-channel ADTS and PCM",
        ));
    }

    let starts = pcm_frame_starts(pcm, 0)?;
    let mut out = Vec::with_capacity(adts_stream_capacity(starts.len()));
    for start_frame in starts {
        let step =
            select_aac_lc_mono_pcm_frame_step_with_offsets_and_scale_factors_and_max_frame_len_by_bit_cost(
                adts,
                channel,
                pcm,
                start_frame,
                offsets,
                candidates,
                max_frame_len_bytes,
                scale_factor_table,
                spectral_tables,
            )?;
        out.extend_from_slice(
            &encode_pcm_mono_long_block_adts_with_offsets_and_scale_factors_by_bit_cost(
                adts,
                channel,
                pcm,
                start_frame,
                step,
                offsets,
                scale_factor_table,
                spectral_tables,
            )?,
        );
    }
    Ok(out)
}

#[allow(clippy::too_many_arguments)]
pub fn encode_pcm_mono_long_block_adts_stream_with_offsets_and_scale_factors_and_bitrate_by_bit_cost(
    adts: AdtsConfig,
    channel: AacScaleFactorChannel<'_>,
    pcm: &AudioBuffer,
    offsets: &[usize],
    candidates: &[f32],
    target_bitrate_bps: u32,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<u8>, Error> {
    let max_frame_len_bytes =
        aac_lc_adts_max_frame_len_for_bitrate(adts.sample_rate, target_bitrate_bps)?;
    encode_pcm_mono_long_block_adts_stream_with_offsets_and_scale_factors_and_max_frame_len_by_bit_cost(
        adts,
        channel,
        pcm,
        offsets,
        candidates,
        max_frame_len_bytes,
        scale_factor_table,
        spectral_tables,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn encode_pcm_stereo_long_block_adts_stream_with_offsets_and_scale_factors_by_bit_cost(
    adts: AdtsConfig,
    left: AacScaleFactorChannel<'_>,
    right: AacScaleFactorChannel<'_>,
    pcm: &AudioBuffer,
    offsets: &[usize],
    candidates: &[f32],
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<u8>, Error> {
    if adts.channels != 2 || pcm.channels != 2 {
        return Err(Error::InvalidInput(
            "AAC stereo PCM encode requires two-channel ADTS and PCM",
        ));
    }

    let starts = pcm_frame_starts(pcm, 0)?;
    let mut out = Vec::with_capacity(adts_stream_capacity(starts.len()));
    for start_frame in starts {
        let step = select_aac_lc_stereo_pcm_frame_step_with_offsets_and_scale_factors_by_bit_cost(
            adts,
            left,
            right,
            pcm,
            start_frame,
            offsets,
            candidates,
            scale_factor_table,
            spectral_tables,
        )?;
        out.extend_from_slice(
            &encode_pcm_stereo_long_block_adts_with_offsets_and_scale_factors_by_bit_cost(
                adts,
                left,
                right,
                pcm,
                start_frame,
                step,
                offsets,
                scale_factor_table,
                spectral_tables,
            )?,
        );
    }
    Ok(out)
}

#[allow(clippy::too_many_arguments)]
pub fn encode_pcm_stereo_long_block_adts_stream_with_offsets_and_selected_scale_factors_by_bit_cost(
    adts: AdtsConfig,
    left: AacLongBlockConfig,
    right: AacLongBlockConfig,
    pcm: &AudioBuffer,
    offsets: &[usize],
    candidates: &[f32],
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<u8>, Error> {
    if adts.channels != 2 || pcm.channels != 2 {
        return Err(Error::InvalidInput(
            "AAC stereo PCM encode requires two-channel ADTS and PCM",
        ));
    }

    let starts = pcm_frame_starts(pcm, 0)?;
    let mut out = Vec::with_capacity(adts_stream_capacity(starts.len()));
    for start_frame in starts {
        let step = select_aac_lc_stereo_pcm_frame_step_with_offsets_by_bit_cost(
            adts,
            left,
            right,
            pcm,
            start_frame,
            offsets,
            candidates,
            scale_factor_table,
            spectral_tables,
        )?;
        out.extend_from_slice(
            &encode_pcm_stereo_long_block_adts_with_offsets_and_selected_scale_factors_by_bit_cost(
                adts,
                left,
                right,
                pcm,
                start_frame,
                step,
                offsets,
                scale_factor_table,
                spectral_tables,
            )?,
        );
    }
    Ok(out)
}

#[allow(clippy::too_many_arguments)]
pub fn encode_pcm_stereo_long_block_adts_stream_with_offsets_and_selected_scale_factors_and_max_frame_len_by_bit_cost(
    adts: AdtsConfig,
    left: AacLongBlockConfig,
    right: AacLongBlockConfig,
    pcm: &AudioBuffer,
    offsets: &[usize],
    candidates: &[f32],
    max_frame_len_bytes: usize,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<u8>, Error> {
    if adts.channels != 2 || pcm.channels != 2 {
        return Err(Error::InvalidInput(
            "AAC stereo PCM encode requires two-channel ADTS and PCM",
        ));
    }

    let starts = pcm_frame_starts(pcm, 0)?;
    let mut out = Vec::with_capacity(adts_stream_capacity(starts.len()));
    for start_frame in starts {
        let step = select_aac_lc_stereo_pcm_frame_step_with_offsets_and_max_frame_len_by_bit_cost(
            adts,
            left,
            right,
            pcm,
            start_frame,
            offsets,
            candidates,
            max_frame_len_bytes,
            scale_factor_table,
            spectral_tables,
        )?;
        out.extend_from_slice(
            &encode_pcm_stereo_long_block_adts_with_offsets_and_selected_scale_factors_by_bit_cost(
                adts,
                left,
                right,
                pcm,
                start_frame,
                step,
                offsets,
                scale_factor_table,
                spectral_tables,
            )?,
        );
    }
    Ok(out)
}

#[allow(clippy::too_many_arguments)]
pub fn select_aac_lc_stereo_pcm_stream_frame_details_with_offsets_and_selected_scale_factors_and_max_frame_len_by_bit_cost(
    adts: AdtsConfig,
    left: AacLongBlockConfig,
    right: AacLongBlockConfig,
    pcm: &AudioBuffer,
    offsets: &[usize],
    candidates: &[f32],
    max_frame_len_bytes: usize,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<AacPcmFrameStepSelection>, Error> {
    if adts.channels != 2 || pcm.channels != 2 {
        return Err(Error::InvalidInput(
            "AAC stereo PCM frame detail selection requires two-channel ADTS and PCM",
        ));
    }

    pcm_frame_starts(pcm, 0)?
        .into_iter()
        .map(|start_frame| {
            select_aac_lc_stereo_pcm_frame_step_details_with_offsets_and_max_frame_len_by_bit_cost(
                adts,
                left,
                right,
                pcm,
                start_frame,
                offsets,
                candidates,
                max_frame_len_bytes,
                scale_factor_table,
                spectral_tables,
            )
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
pub fn select_aac_lc_stereo_pcm_stream_frame_details_with_offsets_and_selected_scale_factors_and_bitrate_by_bit_cost(
    adts: AdtsConfig,
    left: AacLongBlockConfig,
    right: AacLongBlockConfig,
    pcm: &AudioBuffer,
    offsets: &[usize],
    candidates: &[f32],
    target_bitrate_bps: u32,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<AacPcmFrameStepSelection>, Error> {
    let max_frame_len_bytes =
        aac_lc_adts_max_frame_len_for_bitrate(adts.sample_rate, target_bitrate_bps)?;
    select_aac_lc_stereo_pcm_stream_frame_details_with_offsets_and_selected_scale_factors_and_max_frame_len_by_bit_cost(
        adts,
        left,
        right,
        pcm,
        offsets,
        candidates,
        max_frame_len_bytes,
        scale_factor_table,
        spectral_tables,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn encode_pcm_stereo_long_block_adts_stream_with_offsets_and_selected_scale_factors_and_bitrate_by_bit_cost(
    adts: AdtsConfig,
    left: AacLongBlockConfig,
    right: AacLongBlockConfig,
    pcm: &AudioBuffer,
    offsets: &[usize],
    candidates: &[f32],
    target_bitrate_bps: u32,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<u8>, Error> {
    let max_frame_len_bytes =
        aac_lc_adts_max_frame_len_for_bitrate(adts.sample_rate, target_bitrate_bps)?;
    encode_pcm_stereo_long_block_adts_stream_with_offsets_and_selected_scale_factors_and_max_frame_len_by_bit_cost(
        adts,
        left,
        right,
        pcm,
        offsets,
        candidates,
        max_frame_len_bytes,
        scale_factor_table,
        spectral_tables,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn encode_pcm_stereo_long_block_adts_stream_with_offsets_and_scale_factors_and_max_frame_len_by_bit_cost(
    adts: AdtsConfig,
    left: AacScaleFactorChannel<'_>,
    right: AacScaleFactorChannel<'_>,
    pcm: &AudioBuffer,
    offsets: &[usize],
    candidates: &[f32],
    max_frame_len_bytes: usize,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<u8>, Error> {
    if adts.channels != 2 || pcm.channels != 2 {
        return Err(Error::InvalidInput(
            "AAC stereo PCM encode requires two-channel ADTS and PCM",
        ));
    }

    let starts = pcm_frame_starts(pcm, 0)?;
    let mut out = Vec::with_capacity(adts_stream_capacity(starts.len()));
    for start_frame in starts {
        let step =
            select_aac_lc_stereo_pcm_frame_step_with_offsets_and_scale_factors_and_max_frame_len_by_bit_cost(
                adts,
                left,
                right,
                pcm,
                start_frame,
                offsets,
                candidates,
                max_frame_len_bytes,
                scale_factor_table,
                spectral_tables,
            )?;
        out.extend_from_slice(
            &encode_pcm_stereo_long_block_adts_with_offsets_and_scale_factors_by_bit_cost(
                adts,
                left,
                right,
                pcm,
                start_frame,
                step,
                offsets,
                scale_factor_table,
                spectral_tables,
            )?,
        );
    }
    Ok(out)
}

#[allow(clippy::too_many_arguments)]
pub fn encode_pcm_stereo_long_block_adts_stream_with_offsets_and_scale_factors_and_bitrate_by_bit_cost(
    adts: AdtsConfig,
    left: AacScaleFactorChannel<'_>,
    right: AacScaleFactorChannel<'_>,
    pcm: &AudioBuffer,
    offsets: &[usize],
    candidates: &[f32],
    target_bitrate_bps: u32,
    scale_factor_table: &[HuffmanEntry<AacScaleFactorDelta>],
    spectral_tables: AacSpectralMagnitudeTables<'_>,
) -> Result<Vec<u8>, Error> {
    let max_frame_len_bytes =
        aac_lc_adts_max_frame_len_for_bitrate(adts.sample_rate, target_bitrate_bps)?;
    encode_pcm_stereo_long_block_adts_stream_with_offsets_and_scale_factors_and_max_frame_len_by_bit_cost(
        adts,
        left,
        right,
        pcm,
        offsets,
        candidates,
        max_frame_len_bytes,
        scale_factor_table,
        spectral_tables,
    )
}
