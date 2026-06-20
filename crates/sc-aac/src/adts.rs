use super::*;

pub(crate) fn encode_pcm_long_block_scaffold(
    config: AdtsConfig,
    pcm: &AudioBuffer,
) -> Result<Vec<u8>, Error> {
    if config.channels == 1 {
        if let Some(offsets) = aac_lc_long_window_scale_factor_band_offsets(config.sample_rate) {
            let channel_config = AacLongBlockConfig::new(
                180,
                u8::try_from(offsets.len() - 1)
                    .map_err(|_| Error::InvalidInput("AAC scale-factor band count exceeds u8"))?,
            );
            let scale_factor_table = aac_scale_factor_delta_table();
            return encode_pcm_mono_long_block_adts_stream_with_offsets_and_selected_scale_factors_and_bitrate_by_bit_cost(
                config,
                channel_config,
                pcm,
                offsets,
                AAC_LC_PCM_STEP_CANDIDATES,
                aac_lc_default_production_bitrate_bps(config.channels)?,
                &scale_factor_table,
                aac_unsigned_pairs7_unit_magnitude_spectral_tables(),
            );
        }
    }
    if config.channels == 2 {
        if let Some(offsets) = aac_lc_long_window_scale_factor_band_offsets(config.sample_rate) {
            let channel_config = AacLongBlockConfig::new(
                180,
                u8::try_from(offsets.len() - 1)
                    .map_err(|_| Error::InvalidInput("AAC scale-factor band count exceeds u8"))?,
            );
            let scale_factor_table = aac_scale_factor_delta_table();
            return encode_pcm_stereo_long_block_adts_stream_with_offsets_and_selected_scale_factors_and_bitrate_by_bit_cost(
                config,
                channel_config,
                channel_config,
                pcm,
                offsets,
                AAC_LC_PCM_STEP_CANDIDATES,
                aac_lc_default_production_bitrate_bps(config.channels)?,
                &scale_factor_table,
                aac_unsigned_pairs7_unit_magnitude_spectral_tables(),
            );
        }
    }

    let channel = AacLongBlockConfig::new(0, 1);
    let pcm_config = AacPcmLongBlockConfig::new(0, f32::MAX, 1024);
    match config.channels {
        1 => encode_pcm_mono_long_block_adts_stream_with_selected_scale_factors_by_bit_cost(
            config,
            channel,
            pcm,
            pcm_config,
            &[],
            AacSpectralMagnitudeTables::default(),
        ),
        2 => encode_pcm_stereo_long_block_adts_stream_with_selected_scale_factors_by_bit_cost(
            config,
            channel,
            channel,
            pcm,
            pcm_config,
            &[],
            AacSpectralMagnitudeTables::default(),
        ),
        _ => Err(Error::UnsupportedFeature(
            "AAC-LC encode currently supports mono/stereo only",
        )),
    }
}

/// Wraps one raw AAC access unit in an ADTS frame.
pub fn frame_adts(config: AdtsConfig, access_unit: &[u8]) -> Result<Vec<u8>, Error> {
    let sample_rate_index = sample_rate_index(config.sample_rate)?;
    if config.channels == 0 {
        return Err(Error::UnsupportedFeature(
            "AAC program config elements are not supported",
        ));
    }
    if config.channels > 7 {
        return Err(Error::InvalidInput("AAC ADTS channel count exceeds 7"));
    }

    let frame_len = access_unit
        .len()
        .checked_add(7)
        .ok_or(Error::InvalidInput("AAC ADTS frame is too large"))?;
    if frame_len > AAC_ADTS_MAX_FRAME_LEN {
        return Err(Error::InvalidInput("AAC ADTS frame exceeds 13-bit length"));
    }

    let profile = config.profile.adts_profile();
    let channels = config.channels;
    let mut out = Vec::with_capacity(frame_len);
    out.push(0xff);
    out.push(0xf1);
    out.push((profile << 6) | (sample_rate_index << 2) | (channels >> 2));
    out.push(((channels & 0x03) << 6) | (((frame_len >> 11) & 0x03) as u8));
    out.push(((frame_len >> 3) & 0xff) as u8);
    out.push((((frame_len & 0x07) << 5) as u8) | 0x1f);
    out.push(0xfc);
    out.extend_from_slice(access_unit);
    Ok(out)
}

/// Wraps raw AAC access units in consecutive ADTS frames.
pub fn frame_adts_stream<'a>(
    config: AdtsConfig,
    access_units: impl IntoIterator<Item = &'a [u8]>,
) -> Result<Vec<u8>, Error> {
    let mut out = Vec::new();
    for access_unit in access_units {
        out.extend_from_slice(&frame_adts(config, access_unit)?);
    }
    Ok(out)
}

/// Wraps AAC ADTS frames in a minimal M4A container.
pub fn mux_adts_as_m4a(adts: &[u8]) -> Result<Vec<u8>, Error> {
    sc_mp4::mux_aac(adts)
}

/// Demuxes a locally supported M4A container back into AAC ADTS frames.
pub fn demux_m4a_as_adts(input: &[u8]) -> Result<Vec<u8>, Error> {
    sc_mp4::demux_aac(input)
}

/// Runs the AAC-LC long-block analysis window and MDCT for one channel.
pub fn mdct_long_block(samples: &[f32; 2048]) -> Result<Vec<f32>, Error> {
    let window = sine_window(2048)?;
    mdct(&apply_window(samples, &window)?)
}

/// Runs AAC-LC long-block analysis and scalar spectral quantization.
pub fn quantize_long_block(samples: &[f32; 2048], step: f32) -> Result<Vec<i32>, Error> {
    quantize_spectrum(&mdct_long_block(samples)?, step, 8191)
}

/// Extracts one PCM channel and quantizes one AAC-LC long analysis block.
pub fn quantize_pcm_long_block(
    pcm: &AudioBuffer,
    channel: usize,
    start_frame: usize,
    step: f32,
) -> Result<Vec<i32>, Error> {
    let block = fixed_block::<2048>(&pcm.channel_block(channel, start_frame, 2048)?)?;
    quantize_long_block(&block, step)
}
