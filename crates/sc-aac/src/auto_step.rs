use super::*;

/// Encodes a mono AAC-LC ADTS stream with per-frame quantizer step search.
pub fn encode_pcm_mono_long_block_adts_stream_with_auto_step_by_bit_cost(
    adts: AdtsConfig,
    channel: AacLongBlockConfig,
    pcm: &AudioBuffer,
    search: AacPcmStepSearchConfig<'_>,
) -> Result<Vec<u8>, Error> {
    if adts.channels != 1 || pcm.channels != 1 {
        return Err(Error::InvalidInput(
            "AAC mono PCM encode requires one-channel ADTS and PCM",
        ));
    }

    let starts = pcm_frame_starts(pcm, search.start_frame)?;
    let mut out = Vec::with_capacity(adts_stream_capacity(starts.len()));
    for frame_start in starts {
        let step = select_aac_lc_mono_pcm_frame_step_by_bit_cost(
            adts,
            channel,
            pcm,
            AacPcmStepSearchConfig::new(
                frame_start,
                search.band_width,
                search.candidates,
                search.scale_factor_table,
                search.spectral_tables,
            ),
        )?;
        out.extend_from_slice(
            &encode_pcm_mono_long_block_adts_with_selected_scale_factors_by_bit_cost(
                adts,
                channel,
                pcm,
                AacPcmLongBlockConfig::new(frame_start, step, search.band_width),
                search.scale_factor_table,
                search.spectral_tables,
            )?,
        );
    }
    Ok(out)
}

pub fn encode_pcm_mono_long_block_adts_stream_with_offsets_and_auto_step_by_bit_cost(
    adts: AdtsConfig,
    channel: AacLongBlockConfig,
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
    for frame_start in starts {
        let step = select_aac_lc_mono_pcm_frame_step_with_offsets_by_bit_cost(
            adts,
            channel,
            pcm,
            frame_start,
            offsets,
            candidates,
            scale_factor_table,
            spectral_tables,
        )?;
        out.extend_from_slice(
            &encode_pcm_mono_long_block_adts_with_offsets_and_selected_scale_factors_by_bit_cost(
                adts,
                channel,
                pcm,
                frame_start,
                step,
                offsets,
                scale_factor_table,
                spectral_tables,
            )?,
        );
    }
    Ok(out)
}

/// Encodes a stereo AAC-LC ADTS stream with per-frame quantizer step search.
pub fn encode_pcm_stereo_long_block_adts_stream_with_auto_step_by_bit_cost(
    adts: AdtsConfig,
    left: AacLongBlockConfig,
    right: AacLongBlockConfig,
    pcm: &AudioBuffer,
    search: AacPcmStepSearchConfig<'_>,
) -> Result<Vec<u8>, Error> {
    if adts.channels != 2 || pcm.channels != 2 {
        return Err(Error::InvalidInput(
            "AAC stereo PCM encode requires two-channel ADTS and PCM",
        ));
    }

    let starts = pcm_frame_starts(pcm, search.start_frame)?;
    let mut out = Vec::with_capacity(adts_stream_capacity(starts.len()));
    for frame_start in starts {
        let step = select_aac_lc_stereo_pcm_frame_step_by_bit_cost(
            adts,
            left,
            right,
            pcm,
            AacPcmStepSearchConfig::new(
                frame_start,
                search.band_width,
                search.candidates,
                search.scale_factor_table,
                search.spectral_tables,
            ),
        )?;
        out.extend_from_slice(
            &encode_pcm_stereo_long_block_adts_with_selected_scale_factors_by_bit_cost(
                adts,
                left,
                right,
                pcm,
                AacPcmLongBlockConfig::new(frame_start, step, search.band_width),
                search.scale_factor_table,
                search.spectral_tables,
            )?,
        );
    }
    Ok(out)
}
