use super::*;

#[cfg(feature = "aac")]
pub fn aac_standard_id_selected_scale_factor_global_gain(channels: u16) -> Result<u8, Error> {
    match channels {
        1 => Ok(AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_MONO_GLOBAL_GAIN),
        2 => Ok(AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_STEREO_GLOBAL_GAIN),
        _ => Err(Error::InvalidInput(
            "AAC standard-id selected-scale-factor global gain requires mono or stereo",
        )),
    }
}

#[cfg(feature = "aac")]
pub fn aac_standard_id_selected_scale_factor_magnitude_bias() -> i16 {
    AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_MAGNITUDE_BIAS
}

#[cfg(feature = "aac")]
pub fn aac_standard_id_selected_scale_factor_balanced_max_quantized_abs(
    channels: u16,
) -> Result<u32, Error> {
    Ok(aac_standard_id_selected_scale_factor_balance_profile(channels)?.max_quantized_abs)
}

#[cfg(feature = "aac")]
pub fn aac_standard_id_selected_scale_factor_balance_profile(
    channels: u16,
) -> Result<AacStandardIdSelectedScaleFactorBalanceProfile, Error> {
    match channels {
        1 => Ok(AacStandardIdSelectedScaleFactorBalanceProfile {
            recommended_global_gain: AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_MONO_GLOBAL_GAIN,
            global_gain_deltas: AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_MONO_BALANCED_GAIN_DELTAS,
            magnitude_biases: AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_MONO_BALANCED_MAGNITUDE_BIASES,
            selected_global_gain: AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_MONO_BALANCED_GLOBAL_GAIN,
            selected_magnitude_bias:
                AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_MONO_BALANCED_MAGNITUDE_BIAS,
            max_quantized_abs:
                AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_MONO_BALANCED_MAX_QUANTIZED_ABS,
        }),
        2 => Ok(AacStandardIdSelectedScaleFactorBalanceProfile {
            recommended_global_gain: AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_STEREO_GLOBAL_GAIN,
            global_gain_deltas: AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_STEREO_BALANCED_GAIN_DELTAS,
            magnitude_biases:
                AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_STEREO_BALANCED_MAGNITUDE_BIASES,
            selected_global_gain: AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_STEREO_BALANCED_GLOBAL_GAIN,
            selected_magnitude_bias:
                AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_STEREO_BALANCED_MAGNITUDE_BIAS,
            max_quantized_abs:
                AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_STEREO_BALANCED_MAX_QUANTIZED_ABS,
        }),
        _ => Err(Error::InvalidInput(
            "AAC standard-id selected-scale-factor balanced profile requires mono or stereo",
        )),
    }
}

#[cfg(feature = "aac")]
pub fn aac_standard_id_selected_scale_factor_balanced_parameters(
    channels: u16,
) -> Result<(u8, i16, u32), Error> {
    let profile = aac_standard_id_selected_scale_factor_balance_profile(channels)?;
    Ok((
        profile.selected_global_gain,
        profile.selected_magnitude_bias,
        profile.max_quantized_abs,
    ))
}

#[cfg(feature = "aac")]
pub fn aac_standard_id_selected_scale_factor_parameters(channels: u16) -> Result<(u8, i16), Error> {
    Ok((
        aac_standard_id_selected_scale_factor_global_gain(channels)?,
        aac_standard_id_selected_scale_factor_magnitude_bias(),
    ))
}

#[cfg(feature = "aac")]
pub fn encode_aac_adts_with_recommended_standard_spectral_offsets_and_selected_scale_factors_and_bitrate(
    pcm: &AudioBuffer,
    target_bitrate_bps: u32,
) -> Result<Vec<u8>, Error> {
    let (global_gain, scale_factor_magnitude_bias) =
        aac_standard_id_selected_scale_factor_parameters(pcm.channels)?;
    encode_aac_adts_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_bitrate(
        pcm,
        target_bitrate_bps,
        global_gain,
        scale_factor_magnitude_bias,
    )
}

#[cfg(feature = "aac")]
pub fn encode_aac_adts_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_max_quantized_abs_and_bitrate(
    pcm: &AudioBuffer,
    target_bitrate_bps: u32,
    global_gain: u8,
    scale_factor_magnitude_bias: i16,
    max_quantized_abs: u32,
) -> Result<Vec<u8>, Error> {
    let channels = u8::try_from(pcm.channels)
        .map_err(|_| Error::InvalidInput("AAC channel count is unsupported"))?;
    let adts = AdtsConfig::aac_lc(pcm.sample_rate, channels);
    let offsets = aac_lc_long_window_scale_factor_band_offsets(pcm.sample_rate)
        .ok_or(Error::UnsupportedFeature("AAC-LC sample rate"))?;
    let channel_config = AacLongBlockConfig::new(
        global_gain,
        u8::try_from(offsets.len() - 1)
            .map_err(|_| Error::InvalidInput("AAC scale-factor band count exceeds u8"))?,
    );
    let scale_factor_table = aac_scale_factor_delta_table();

    match pcm.channels {
        1 => encode_pcm_mono_long_block_adts_stream_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_max_quantized_abs_and_bitrate_by_bit_cost(
            adts,
            channel_config,
            pcm,
            0,
            offsets,
            scale_factor_magnitude_bias,
            AAC_STANDARD_ID_PCM_STEP_CANDIDATES,
            max_quantized_abs,
            target_bitrate_bps,
            &scale_factor_table,
        ),
        2 => encode_pcm_stereo_long_block_adts_stream_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_max_quantized_abs_and_bitrate_by_bit_cost(
            adts,
            channel_config,
            channel_config,
            pcm,
            0,
            offsets,
            scale_factor_magnitude_bias,
            AAC_STANDARD_ID_PCM_STEP_CANDIDATES,
            max_quantized_abs,
            target_bitrate_bps,
            &scale_factor_table,
        ),
        _ => Err(Error::InvalidInput(
            "AAC standard spectral-offset selected-scale-factor max-quantized-abs bitrate encode requires mono or stereo PCM",
        )),
    }
}

#[cfg(feature = "aac")]
pub fn encode_aac_adts_with_recommended_standard_spectral_offsets_and_selected_scale_factors_max_quantized_abs_and_bitrate(
    pcm: &AudioBuffer,
    target_bitrate_bps: u32,
    max_quantized_abs: u32,
) -> Result<Vec<u8>, Error> {
    let (global_gain, scale_factor_magnitude_bias) =
        aac_standard_id_selected_scale_factor_parameters(pcm.channels)?;
    encode_aac_adts_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_max_quantized_abs_and_bitrate(
        pcm,
        target_bitrate_bps,
        global_gain,
        scale_factor_magnitude_bias,
        max_quantized_abs,
    )
}

#[cfg(feature = "aac")]
pub fn encode_aac_adts_with_balanced_standard_spectral_offsets_and_selected_scale_factors_and_bitrate(
    pcm: &AudioBuffer,
    target_bitrate_bps: u32,
) -> Result<Vec<u8>, Error> {
    let (global_gain, scale_factor_magnitude_bias, max_quantized_abs) =
        aac_standard_id_selected_scale_factor_balanced_parameters(pcm.channels)?;
    encode_aac_adts_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_max_quantized_abs_and_bitrate(
        pcm,
        target_bitrate_bps,
        global_gain,
        scale_factor_magnitude_bias,
        max_quantized_abs,
    )
}

#[cfg(feature = "aac")]
pub fn aac_standard_selected_scale_factor_frame_details_with_magnitude_bias_and_bitrate(
    pcm: &AudioBuffer,
    target_bitrate_bps: u32,
    global_gain: u8,
    scale_factor_magnitude_bias: i16,
) -> Result<Vec<AacPcmFrameStepSelection>, Error> {
    let channels = u8::try_from(pcm.channels)
        .map_err(|_| Error::InvalidInput("AAC channel count is unsupported"))?;
    let adts = AdtsConfig::aac_lc(pcm.sample_rate, channels);
    let offsets = aac_lc_long_window_scale_factor_band_offsets(pcm.sample_rate)
        .ok_or(Error::UnsupportedFeature("AAC-LC sample rate"))?;
    let channel_config = AacLongBlockConfig::new(
        global_gain,
        u8::try_from(offsets.len() - 1)
            .map_err(|_| Error::InvalidInput("AAC scale-factor band count exceeds u8"))?,
    );
    let scale_factor_table = aac_scale_factor_delta_table();

    match pcm.channels {
        1 => select_aac_lc_mono_pcm_stream_frame_details_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_bitrate_by_bit_cost(
            adts,
            channel_config,
            pcm,
            0,
            offsets,
            scale_factor_magnitude_bias,
            AAC_STANDARD_ID_PCM_STEP_CANDIDATES,
            target_bitrate_bps,
            &scale_factor_table,
        ),
        2 => select_aac_lc_stereo_pcm_stream_frame_details_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_bitrate_by_bit_cost(
            adts,
            channel_config,
            channel_config,
            pcm,
            0,
            offsets,
            scale_factor_magnitude_bias,
            AAC_STANDARD_ID_PCM_STEP_CANDIDATES,
            target_bitrate_bps,
            &scale_factor_table,
        ),
        _ => Err(Error::InvalidInput(
            "AAC standard selected-scale-factor frame details require mono or stereo PCM",
        )),
    }
}

#[cfg(feature = "aac")]
pub fn aac_recommended_standard_selected_scale_factor_frame_details_with_bitrate(
    pcm: &AudioBuffer,
    target_bitrate_bps: u32,
) -> Result<Vec<AacPcmFrameStepSelection>, Error> {
    let (global_gain, scale_factor_magnitude_bias) =
        aac_standard_id_selected_scale_factor_parameters(pcm.channels)?;
    aac_standard_selected_scale_factor_frame_details_with_magnitude_bias_and_bitrate(
        pcm,
        target_bitrate_bps,
        global_gain,
        scale_factor_magnitude_bias,
    )
}

#[cfg(feature = "aac")]
pub fn aac_standard_selected_scale_factor_frame_details_with_magnitude_bias_max_quantized_abs_and_bitrate(
    pcm: &AudioBuffer,
    target_bitrate_bps: u32,
    global_gain: u8,
    scale_factor_magnitude_bias: i16,
    max_quantized_abs: u32,
) -> Result<Vec<AacPcmFrameStepSelection>, Error> {
    let channels = u8::try_from(pcm.channels)
        .map_err(|_| Error::InvalidInput("AAC standard frame details require u8 channels"))?;
    let adts = AdtsConfig::aac_lc(pcm.sample_rate, channels);
    let offsets = aac_lc_long_window_scale_factor_band_offsets(pcm.sample_rate).ok_or(
        Error::UnsupportedFeature("AAC-LC scale-factor offsets for sample rate"),
    )?;
    let channel_config = AacLongBlockConfig::new(
        global_gain,
        u8::try_from(offsets.len() - 1)
            .map_err(|_| Error::InvalidInput("AAC scale-factor band count exceeds u8"))?,
    );
    let scale_factor_table = aac_scale_factor_delta_table();
    match pcm.channels {
        1 => select_aac_lc_mono_pcm_stream_frame_details_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_max_quantized_abs_and_bitrate_by_bit_cost(
            adts,
            channel_config,
            pcm,
            0,
            offsets,
            scale_factor_magnitude_bias,
            AAC_STANDARD_ID_PCM_STEP_CANDIDATES,
            max_quantized_abs,
            target_bitrate_bps,
            &scale_factor_table,
        ),
        2 => select_aac_lc_stereo_pcm_stream_frame_details_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_max_quantized_abs_and_bitrate_by_bit_cost(
            adts,
            channel_config,
            channel_config,
            pcm,
            0,
            offsets,
            scale_factor_magnitude_bias,
            AAC_STANDARD_ID_PCM_STEP_CANDIDATES,
            max_quantized_abs,
            target_bitrate_bps,
            &scale_factor_table,
        ),
        _ => Err(Error::InvalidInput(
            "AAC standard selected-scale-factor frame details require mono or stereo PCM",
        )),
    }
}

#[cfg(feature = "aac")]
pub fn aac_recommended_standard_selected_scale_factor_frame_details_with_max_quantized_abs_and_bitrate(
    pcm: &AudioBuffer,
    target_bitrate_bps: u32,
    max_quantized_abs: u32,
) -> Result<Vec<AacPcmFrameStepSelection>, Error> {
    let (global_gain, scale_factor_magnitude_bias) =
        aac_standard_id_selected_scale_factor_parameters(pcm.channels)?;
    aac_standard_selected_scale_factor_frame_details_with_magnitude_bias_max_quantized_abs_and_bitrate(
        pcm,
        target_bitrate_bps,
        global_gain,
        scale_factor_magnitude_bias,
        max_quantized_abs,
    )
}

#[cfg(feature = "aac")]
pub fn aac_balanced_standard_selected_scale_factor_frame_details_with_bitrate(
    pcm: &AudioBuffer,
    target_bitrate_bps: u32,
) -> Result<Vec<AacPcmFrameStepSelection>, Error> {
    let (global_gain, scale_factor_magnitude_bias, max_quantized_abs) =
        aac_standard_id_selected_scale_factor_balanced_parameters(pcm.channels)?;
    aac_standard_selected_scale_factor_frame_details_with_magnitude_bias_max_quantized_abs_and_bitrate(
        pcm,
        target_bitrate_bps,
        global_gain,
        scale_factor_magnitude_bias,
        max_quantized_abs,
    )
}

#[cfg(feature = "aac")]
pub fn aac_standard_selected_scale_factor_profile_for_frame_details_with_magnitude_bias(
    pcm: &AudioBuffer,
    details: &[AacPcmFrameStepSelection],
    global_gain: u8,
    scale_factor_magnitude_bias: i16,
) -> Result<AacSelectedScaleFactorProfile, Error> {
    let offsets = aac_lc_long_window_scale_factor_band_offsets(pcm.sample_rate)
        .ok_or(Error::UnsupportedFeature("AAC-LC sample rate"))?;
    let mut bands = 0usize;
    let mut raised_bands = 0usize;
    let mut max_delta = 0i16;
    let mut delta_sum = 0i64;

    for (frame_index, detail) in details.iter().enumerate() {
        let start_frame = frame_index
            .checked_mul(1024)
            .ok_or(Error::InvalidInput("AAC frame index overflows"))?;
        for channel in 0..usize::from(pcm.channels) {
            let quantized = quantize_pcm_long_block(pcm, channel, start_frame, detail.step)?;
            let scale_factors =
                select_scale_factors_for_quantized_bands_by_offsets_with_magnitude_bias(
                    &quantized,
                    offsets,
                    i16::from(global_gain),
                    scale_factor_magnitude_bias,
                )?;
            for scale_factor in scale_factors {
                let delta = scale_factor - i16::from(global_gain);
                bands += 1;
                raised_bands += usize::from(delta > 0);
                max_delta = max_delta.max(delta);
                delta_sum += i64::from(delta);
            }
        }
    }

    if bands == 0 {
        return Err(Error::InvalidInput(
            "AAC scale-factor profile requires at least one band",
        ));
    }

    Ok(AacSelectedScaleFactorProfile {
        frames: details.len(),
        channels: usize::from(pcm.channels),
        bands,
        raised_bands,
        max_delta,
        mean_delta: delta_sum as f64 / bands as f64,
    })
}

#[cfg(feature = "aac")]
pub fn aac_recommended_standard_selected_scale_factor_profile_for_frame_details(
    pcm: &AudioBuffer,
    details: &[AacPcmFrameStepSelection],
) -> Result<AacSelectedScaleFactorProfile, Error> {
    let (global_gain, scale_factor_magnitude_bias) =
        aac_standard_id_selected_scale_factor_parameters(pcm.channels)?;
    aac_standard_selected_scale_factor_profile_for_frame_details_with_magnitude_bias(
        pcm,
        details,
        global_gain,
        scale_factor_magnitude_bias,
    )
}

#[cfg(feature = "aac")]
pub fn aac_balanced_standard_selected_scale_factor_profile_for_frame_details(
    pcm: &AudioBuffer,
    details: &[AacPcmFrameStepSelection],
) -> Result<AacSelectedScaleFactorProfile, Error> {
    let profile = aac_standard_id_selected_scale_factor_balance_profile(pcm.channels)?;
    aac_standard_selected_scale_factor_profile_for_frame_details_with_magnitude_bias(
        pcm,
        details,
        profile.selected_global_gain,
        profile.selected_magnitude_bias,
    )
}

#[cfg(feature = "aac")]
pub(crate) fn aac_scale_factor_band_index(
    offsets: &[usize],
    offset: usize,
) -> Result<usize, Error> {
    offsets
        .iter()
        .position(|band_offset| *band_offset == offset)
        .ok_or(Error::InvalidInput(
            "AAC scale-factor band offset not found",
        ))
}

#[cfg(feature = "aac")]
pub(crate) fn aac_spectral_pairs_for_i32_slice(
    quantized: &[i32],
) -> Result<Vec<AacSpectralPair>, Error> {
    if quantized.len() % 2 != 0 {
        return Err(Error::InvalidInput(
            "AAC spectral pair slice length must be even",
        ));
    }
    quantized
        .chunks_exact(2)
        .map(|pair| {
            Ok(AacSpectralPair::new(
                i16::try_from(pair[0])
                    .map_err(|_| Error::InvalidInput("AAC spectral pair x exceeds i16"))?,
                i16::try_from(pair[1])
                    .map_err(|_| Error::InvalidInput("AAC spectral pair y exceeds i16"))?,
            ))
        })
        .collect()
}

#[cfg(feature = "aac")]
pub(crate) fn aac_spectral_quads_for_i32_slice(
    quantized: &[i32],
) -> Result<Vec<AacSpectralQuad>, Error> {
    if quantized.len() % 4 != 0 {
        return Err(Error::InvalidInput(
            "AAC spectral quad slice length must be divisible by four",
        ));
    }
    quantized
        .chunks_exact(4)
        .map(|quad| {
            Ok(AacSpectralQuad::new(
                i16::try_from(quad[0])
                    .map_err(|_| Error::InvalidInput("AAC spectral quad v exceeds i16"))?,
                i16::try_from(quad[1])
                    .map_err(|_| Error::InvalidInput("AAC spectral quad w exceeds i16"))?,
                i16::try_from(quad[2])
                    .map_err(|_| Error::InvalidInput("AAC spectral quad x exceeds i16"))?,
                i16::try_from(quad[3])
                    .map_err(|_| Error::InvalidInput("AAC spectral quad y exceeds i16"))?,
            ))
        })
        .collect()
}
