use super::*;

/// Encodes interleaved PCM as WAV.
#[cfg(feature = "wav")]
pub fn encode_wav(pcm: &AudioBuffer) -> Result<Vec<u8>, Error> {
    encode_wav_impl(pcm)
}

#[cfg(feature = "flac")]
pub fn encode_flac(pcm: &AudioBuffer) -> Result<Vec<u8>, Error> {
    encode_flac_impl(pcm)
}

#[cfg(feature = "mp3")]
pub fn encode_mp3(pcm: &AudioBuffer) -> Result<Vec<u8>, Error> {
    encode_mp3_impl(pcm)
}

/// Encodes mono/stereo PCM as MP3 (Layer III) targeting a constant bitrate.
///
/// Unlike the default [`encode`](crate::encode)`(Format::Mp3, ..)` path (fixed
/// 128 kbit/s MPEG-1 / 64 kbit/s MPEG-2 LSF), this runs the per-frame perceptual
/// quantizer-step search so the step adapts to `bitrate_kbps`. The value must be
/// a Layer III bitrate valid for the stream's MPEG version and sample rate.
///
/// Output is long-block-only and not sample-accurate (no Xing/LAME gapless
/// header), matching the default MP3 encode path's caveats.
#[cfg(feature = "mp3")]
pub fn encode_mp3_with_bitrate(pcm: &AudioBuffer, bitrate_kbps: u16) -> Result<Vec<u8>, Error> {
    sc_mp3::encode_with_bitrate(pcm, bitrate_kbps)
}

#[cfg(feature = "vorbis")]
pub fn encode_vorbis(pcm: &AudioBuffer) -> Result<Vec<u8>, Error> {
    encode_vorbis_impl(pcm)
}

#[cfg(feature = "opus")]
pub fn encode_opus(pcm: &AudioBuffer) -> Result<Vec<u8>, Error> {
    encode_opus_impl(pcm)
}

#[cfg(feature = "aac")]
pub fn encode_aac(pcm: &AudioBuffer) -> Result<Vec<u8>, Error> {
    encode_aac_impl(pcm)
}

/// Encodes mono/stereo PCM as an AAC-LC ADTS stream targeting `target_bitrate_bps`.
///
/// Two behaviors are worth noting for callers:
///
/// - **Frame granularity / length.** AAC-LC codes audio in fixed 1024-sample
///   blocks, so the encoder pads the final block with silence. The decoded
///   stream is therefore a whole multiple of 1024 samples per channel and may be
///   up to 1023 samples longer than the input. This path is not sample-accurate;
///   for sample-exact round-trips use a lossless format (WAV/FLAC).
/// - **Very low target bitrates.** `target_bitrate_bps` is enforced as a
///   per-frame byte budget. If the budget is so small that no quantizer step in
///   the search produces a frame that fits (e.g. ~17 kbps at 44.1 kHz, where the
///   budget is only tens of bytes per frame), the encoder degrades gracefully:
///   it falls back to the coarsest quantizer step — the smallest frame the
///   candidate list can produce — so the stream stays valid and decodable even
///   though individual frames exceed the requested budget. The realized bitrate
///   is therefore higher than the target at such extremes. Only a budget below
///   the ADTS header size itself is rejected outright.
#[cfg(feature = "aac")]
pub fn encode_aac_adts_with_bitrate(
    pcm: &AudioBuffer,
    target_bitrate_bps: u32,
) -> Result<Vec<u8>, Error> {
    let channels = u8::try_from(pcm.channels)
        .map_err(|_| Error::InvalidInput("AAC channel count is unsupported"))?;
    let adts = AdtsConfig::aac_lc(pcm.sample_rate, channels);
    let offsets = aac_lc_long_window_scale_factor_band_offsets(pcm.sample_rate)
        .ok_or(Error::UnsupportedFeature("AAC-LC sample rate"))?;
    // Derive the global gain / scale factors from the quantizer step chosen by
    // the bit-cost search rather than pinning them to a fixed value: a fixed
    // scale factor that does not match the step the spectrum was quantized on
    // shifts the decoded level by orders of magnitude. The selected-scale-factor
    // path recomputes the gain from each candidate step, so the declared scale
    // factors always match the grid.
    let channel_config = AacLongBlockConfig::new(
        180,
        u8::try_from(offsets.len() - 1)
            .map_err(|_| Error::InvalidInput("AAC scale-factor band count exceeds u8"))?,
    );
    let scale_factor_table = aac_scale_factor_delta_table();
    let spectral_tables = aac_lc_standard_spectral_tables();

    match pcm.channels {
        1 => encode_pcm_mono_long_block_adts_stream_with_offsets_and_selected_scale_factors_and_bitrate_by_bit_cost(
            adts,
            channel_config,
            pcm,
            offsets,
            AAC_LC_PCM_STEP_CANDIDATES,
            target_bitrate_bps,
            &scale_factor_table,
            spectral_tables,
        ),
        2 => encode_pcm_stereo_long_block_adts_stream_with_offsets_and_selected_scale_factors_and_bitrate_by_bit_cost(
            adts,
            channel_config,
            channel_config,
            pcm,
            offsets,
            AAC_LC_PCM_STEP_CANDIDATES,
            target_bitrate_bps,
            &scale_factor_table,
            spectral_tables,
        ),
        _ => Err(Error::InvalidInput(
            "AAC bitrate encode requires mono or stereo PCM",
        )),
    }
}

#[cfg(feature = "aac")]
pub fn encode_aac_adts_with_selected_scale_factors_and_bitrate(
    pcm: &AudioBuffer,
    target_bitrate_bps: u32,
) -> Result<Vec<u8>, Error> {
    let channels = u8::try_from(pcm.channels)
        .map_err(|_| Error::InvalidInput("AAC channel count is unsupported"))?;
    let adts = AdtsConfig::aac_lc(pcm.sample_rate, channels);
    let offsets = aac_lc_long_window_scale_factor_band_offsets(pcm.sample_rate)
        .ok_or(Error::UnsupportedFeature("AAC-LC sample rate"))?;
    let channel_config = AacLongBlockConfig::new(
        180,
        u8::try_from(offsets.len() - 1)
            .map_err(|_| Error::InvalidInput("AAC scale-factor band count exceeds u8"))?,
    );
    let scale_factor_table = aac_scale_factor_delta_table();
    let spectral_tables = aac_lc_standard_spectral_tables();

    match pcm.channels {
        1 => encode_pcm_mono_long_block_adts_stream_with_offsets_and_selected_scale_factors_and_bitrate_by_bit_cost(
            adts,
            channel_config,
            pcm,
            offsets,
            AAC_LC_PCM_STEP_CANDIDATES,
            target_bitrate_bps,
            &scale_factor_table,
            spectral_tables,
        ),
        2 => encode_pcm_stereo_long_block_adts_stream_with_offsets_and_selected_scale_factors_and_bitrate_by_bit_cost(
            adts,
            channel_config,
            channel_config,
            pcm,
            offsets,
            AAC_LC_PCM_STEP_CANDIDATES,
            target_bitrate_bps,
            &scale_factor_table,
            spectral_tables,
        ),
        _ => Err(Error::InvalidInput(
            "AAC selected-scale-factor bitrate encode requires mono or stereo PCM",
        )),
    }
}

#[cfg(feature = "aac")]
pub fn aac_selected_scale_factor_frame_details_with_bitrate(
    pcm: &AudioBuffer,
    target_bitrate_bps: u32,
) -> Result<Vec<AacPcmFrameStepSelection>, Error> {
    let channels = u8::try_from(pcm.channels)
        .map_err(|_| Error::InvalidInput("AAC channel count is unsupported"))?;
    let adts = AdtsConfig::aac_lc(pcm.sample_rate, channels);
    let offsets = aac_lc_long_window_scale_factor_band_offsets(pcm.sample_rate)
        .ok_or(Error::UnsupportedFeature("AAC-LC sample rate"))?;
    let channel_config = AacLongBlockConfig::new(
        180,
        u8::try_from(offsets.len() - 1)
            .map_err(|_| Error::InvalidInput("AAC scale-factor band count exceeds u8"))?,
    );
    let scale_factor_table = aac_scale_factor_delta_table();
    let spectral_tables = aac_lc_standard_spectral_tables();

    match pcm.channels {
        1 => select_aac_lc_mono_pcm_stream_frame_details_with_offsets_and_selected_scale_factors_and_bitrate_by_bit_cost(
            adts,
            channel_config,
            pcm,
            offsets,
            AAC_LC_PCM_STEP_CANDIDATES,
            target_bitrate_bps,
            &scale_factor_table,
            spectral_tables,
        ),
        2 => select_aac_lc_stereo_pcm_stream_frame_details_with_offsets_and_selected_scale_factors_and_bitrate_by_bit_cost(
            adts,
            channel_config,
            channel_config,
            pcm,
            offsets,
            AAC_LC_PCM_STEP_CANDIDATES,
            target_bitrate_bps,
            &scale_factor_table,
            spectral_tables,
        ),
        _ => Err(Error::InvalidInput(
            "AAC selected-scale-factor bitrate frame details require mono or stereo PCM",
        )),
    }
}

#[cfg(feature = "aac")]
pub fn encode_aac_adts_with_standard_spectral_offsets_and_bitrate(
    pcm: &AudioBuffer,
    target_bitrate_bps: u32,
    global_gain: u8,
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
    let scale_factors_by_frame = constant_aac_scale_factors_by_frame(
        pcm,
        i16::from(channel_config.global_gain),
        offsets.len() - 1,
    );
    let scale_factor_refs = scale_factors_by_frame
        .iter()
        .map(Vec::as_slice)
        .collect::<Vec<_>>();
    let scale_factor_table = aac_scale_factor_delta_table();

    match pcm.channels {
        1 => encode_pcm_mono_long_block_adts_stream_with_standard_spectral_offsets_and_scale_factors_and_bitrate_by_bit_cost(
            adts,
            AacScaleFactorSequence::new(channel_config, &scale_factor_refs),
            pcm,
            0,
            offsets,
            AAC_STANDARD_ID_PCM_STEP_CANDIDATES,
            target_bitrate_bps,
            &scale_factor_table,
        ),
        2 => encode_pcm_stereo_long_block_adts_stream_with_standard_spectral_offsets_and_scale_factors_and_bitrate_by_bit_cost(
            adts,
            AacScaleFactorSequence::new(channel_config, &scale_factor_refs),
            AacScaleFactorSequence::new(channel_config, &scale_factor_refs),
            pcm,
            0,
            offsets,
            AAC_STANDARD_ID_PCM_STEP_CANDIDATES,
            target_bitrate_bps,
            &scale_factor_table,
        ),
        _ => Err(Error::InvalidInput(
            "AAC standard spectral-offset bitrate encode requires mono or stereo PCM",
        )),
    }
}

#[cfg(feature = "aac")]
pub fn encode_aac_adts_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_bitrate(
    pcm: &AudioBuffer,
    target_bitrate_bps: u32,
    global_gain: u8,
    scale_factor_magnitude_bias: i16,
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
        1 => encode_pcm_mono_long_block_adts_stream_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_bitrate_by_bit_cost(
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
        2 => encode_pcm_stereo_long_block_adts_stream_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_bitrate_by_bit_cost(
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
            "AAC standard spectral-offset selected-scale-factor bitrate encode requires mono or stereo PCM",
        )),
    }
}
