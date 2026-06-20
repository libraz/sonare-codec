use super::*;

/// FFT length the perceptual model analyses for one long granule. The granule's
/// 576 samples are centred in this window, zero-padded past the stream edges.
pub(crate) const MPEG1_LAYER3_PSY_FFT_LEN: usize = 1024;

/// Builds one MPEG-1 Layer III long-block payload from PCM analysis, shaping the
/// quantization noise per scale-factor band with the psychoacoustic model.
///
/// Like
/// [`pack_mpeg1_layer3_pcm_long_block_with_calibrated_gain_and_table_provider`],
/// but instead of zeroing the scale factors it runs the psychoacoustic model
/// over a Hann-windowed FFT block centred on the granule and allocates per-band
/// scale factors that drive quantization noise under the masking threshold. The
/// decoder reverses the per-band gain via the transmitted scale factors and
/// `scalefac_scale`, so reconstruction stays consistent with the calibrated
/// global gain.
pub fn pack_mpeg1_layer3_pcm_long_block_with_perceptual_scale_factors_and_table_provider(
    granule: &mut Layer3GranuleChannelInfo,
    pcm: &AudioBuffer,
    channel: usize,
    start_frame: usize,
    step: f32,
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<PackedBits, Error> {
    pack_mpeg1_layer3_pcm_long_block_with_perceptual_scalefac_scale_and_table_provider(
        granule,
        pcm,
        channel,
        start_frame,
        step,
        false,
        provider,
    )
}

/// Builds one perceptual long-block payload with caller-selected
/// `scalefac_scale`.
pub fn pack_mpeg1_layer3_pcm_long_block_with_perceptual_scalefac_scale_and_table_provider(
    granule: &mut Layer3GranuleChannelInfo,
    pcm: &AudioBuffer,
    channel: usize,
    start_frame: usize,
    step: f32,
    scalefac_scale: bool,
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<PackedBits, Error> {
    let quantizer_spectrum = layer3_perceptual_quantizer_spectrum(pcm, channel, start_frame)?;
    let scale_factors = select_centered_mpeg1_layer3_psychoacoustic_long_scale_factors(
        pcm,
        channel,
        start_frame,
        &quantizer_spectrum,
        step,
        scalefac_scale,
    )?;
    let quantized = quantize_mpeg1_layer3_long_spectrum_with_scalefactors(
        &quantizer_spectrum,
        step,
        &scale_factors,
        scalefac_scale,
        pcm.sample_rate,
    )?;
    granule.scalefac_scale = scalefac_scale;
    let packed = pack_mpeg1_layer3_long_quantized_spectrum_with_table_provider(
        granule,
        &scale_factors,
        &quantized,
        provider,
    )?;
    granule.global_gain = calibrated_global_gain_for_granule(&quantized, step);
    Ok(packed)
}

/// Builds one perceptual long-block payload with an allowed-noise multiplier.
pub fn pack_mpeg1_layer3_pcm_long_block_with_perceptual_allowed_noise_scale_and_table_provider(
    granule: &mut Layer3GranuleChannelInfo,
    pcm: &AudioBuffer,
    channel: usize,
    start_frame: usize,
    step: f32,
    allowed_noise_scale: f64,
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<PackedBits, Error> {
    let quantizer_spectrum = layer3_perceptual_quantizer_spectrum(pcm, channel, start_frame)?;
    let scalefac_scale = false;
    let pcm_window = centered_mpeg1_layer3_psychoacoustic_pcm_window(
        pcm,
        channel,
        start_frame,
        quantizer_spectrum.len(),
    );
    let scale_factors =
        psychoacoustic::perceptual_long_block_scalefactors_with_allowed_noise_scale(
            &quantizer_spectrum,
            &pcm_window,
            step,
            scalefac_scale,
            pcm.sample_rate,
            allowed_noise_scale,
        )?;
    let quantized = quantize_mpeg1_layer3_long_spectrum_with_scalefactors(
        &quantizer_spectrum,
        step,
        &scale_factors,
        scalefac_scale,
        pcm.sample_rate,
    )?;
    granule.scalefac_scale = scalefac_scale;
    let packed = pack_mpeg1_layer3_long_quantized_spectrum_with_table_provider(
        granule,
        &scale_factors,
        &quantized,
        provider,
    )?;
    granule.global_gain = calibrated_global_gain_for_granule(&quantized, step);
    Ok(packed)
}

pub(crate) fn mpeg1_layer3_scale_factor_syntax_cap(band: usize) -> u8 {
    if band < 11 {
        15
    } else {
        7
    }
}

pub(crate) fn apply_mpeg1_layer3_scale_factor_band_bias(
    scale_factors: &mut [u8; MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT],
    band_bias: Layer3ScaleFactorBandBias,
) -> Result<(), Error> {
    if band_bias.band_start > band_bias.band_end
        || band_bias.band_end > MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT
    {
        return Err(Error::InvalidInput(
            "MP3 scale-factor band range is invalid",
        ));
    }
    for (band, scale_factor) in scale_factors
        .iter_mut()
        .enumerate()
        .take(band_bias.band_end)
        .skip(band_bias.band_start)
    {
        let cap = i16::from(mpeg1_layer3_scale_factor_syntax_cap(band));
        let adjusted = (i16::from(*scale_factor) + i16::from(band_bias.bias)).clamp(0, cap);
        *scale_factor = adjusted as u8;
    }
    Ok(())
}

pub(crate) fn pack_mpeg1_layer3_pcm_long_block_with_perceptual_scale_factor_band_bias_and_table_provider(
    granule: &mut Layer3GranuleChannelInfo,
    pcm: &AudioBuffer,
    channel: usize,
    start_frame: usize,
    step: f32,
    band_bias: Layer3ScaleFactorBandBias,
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<PackedBits, Error> {
    let quantizer_spectrum = layer3_perceptual_quantizer_spectrum(pcm, channel, start_frame)?;
    let scalefac_scale = false;
    let mut scale_factors = select_centered_mpeg1_layer3_psychoacoustic_long_scale_factors(
        pcm,
        channel,
        start_frame,
        &quantizer_spectrum,
        step,
        scalefac_scale,
    )?;
    apply_mpeg1_layer3_scale_factor_band_bias(&mut scale_factors, band_bias)?;
    let quantized = quantize_mpeg1_layer3_long_spectrum_with_scalefactors(
        &quantizer_spectrum,
        step,
        &scale_factors,
        scalefac_scale,
        pcm.sample_rate,
    )?;
    granule.scalefac_scale = scalefac_scale;
    let packed = pack_mpeg1_layer3_long_quantized_spectrum_with_table_provider(
        granule,
        &scale_factors,
        &quantized,
        provider,
    )?;
    granule.global_gain = calibrated_global_gain_for_granule(&quantized, step);
    Ok(packed)
}

pub(crate) fn apply_mpeg1_layer3_quantized_band_gain(
    quantized: &mut [i32],
    sample_rate: u32,
    band_gain: Layer3QuantizedBandGain,
) -> Result<(), Error> {
    if band_gain.band_start > band_gain.band_end
        || band_gain.band_end > MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT
        || !band_gain.gain.is_finite()
    {
        return Err(Error::InvalidInput("MP3 quantized band gain is invalid"));
    }
    for band in band_gain.band_start..band_gain.band_end {
        let (start, end) = mpeg1_layer3_long_scalefactor_band_range(band, sample_rate)?;
        for line in start..end.min(quantized.len()) {
            let adjusted = ((quantized[line] as f32) * band_gain.gain).round();
            if adjusted.abs() > 8191.0 {
                return Err(Error::InvalidInput(
                    "MP3 quantized band gain exceeds coefficient bound",
                ));
            }
            quantized[line] = adjusted as i32;
        }
    }
    Ok(())
}

pub(crate) fn pack_mpeg1_layer3_pcm_long_block_with_perceptual_quantized_band_gain_and_table_provider(
    granule: &mut Layer3GranuleChannelInfo,
    pcm: &AudioBuffer,
    channel: usize,
    start_frame: usize,
    step: f32,
    band_gain: Layer3QuantizedBandGain,
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<PackedBits, Error> {
    let quantizer_spectrum = layer3_perceptual_quantizer_spectrum(pcm, channel, start_frame)?;
    let scalefac_scale = false;
    let scale_factors = select_centered_mpeg1_layer3_psychoacoustic_long_scale_factors(
        pcm,
        channel,
        start_frame,
        &quantizer_spectrum,
        step,
        scalefac_scale,
    )?;
    let mut quantized = quantize_mpeg1_layer3_long_spectrum_with_scalefactors(
        &quantizer_spectrum,
        step,
        &scale_factors,
        scalefac_scale,
        pcm.sample_rate,
    )?;
    apply_mpeg1_layer3_quantized_band_gain(&mut quantized, pcm.sample_rate, band_gain)?;
    granule.scalefac_scale = scalefac_scale;
    let packed = pack_mpeg1_layer3_long_quantized_spectrum_with_table_provider(
        granule,
        &scale_factors,
        &quantized,
        provider,
    )?;
    granule.global_gain = calibrated_global_gain_for_granule(&quantized, step);
    Ok(packed)
}

pub(crate) fn biased_layer3_global_gain(global_gain: u8, bias: i16) -> u8 {
    (i16::from(global_gain) + bias).clamp(0, 255) as u8
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn pack_mpeg1_layer3_pcm_long_block_with_perceptual_quantized_band_gain_and_global_gain_bias_and_table_provider(
    granule: &mut Layer3GranuleChannelInfo,
    pcm: &AudioBuffer,
    channel: usize,
    start_frame: usize,
    step: f32,
    band_gain: Layer3QuantizedBandGain,
    global_gain_bias: i16,
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<PackedBits, Error> {
    let packed =
        pack_mpeg1_layer3_pcm_long_block_with_perceptual_quantized_band_gain_and_table_provider(
            granule,
            pcm,
            channel,
            start_frame,
            step,
            band_gain,
            provider,
        )?;
    granule.global_gain = biased_layer3_global_gain(granule.global_gain, global_gain_bias);
    Ok(packed)
}

pub(crate) fn requantize_mpeg1_layer3_long_line_with_scalefactors(
    quantized: i32,
    line: usize,
    global_gain: u8,
    scale_factors: &[u8; MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT],
    scalefac_scale: bool,
    sample_rate: u32,
) -> Result<f32, Error> {
    let magnitude = (quantized.unsigned_abs() as f32).powf(4.0 / 3.0);
    let sign = if quantized < 0 { -1.0 } else { 1.0 };
    let gain = 2.0_f32.powf(0.25 * (f32::from(global_gain) - 210.0));
    let index = mpeg1_layer3_long_scalefactor_band_index(sample_rate)?;
    let multiplier = if scalefac_scale { 1.0 } else { 0.5 };
    let attenuation = match index[1..=MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT]
        .iter()
        .position(|&boundary| line < usize::from(boundary))
    {
        Some(band) => 2.0_f32.powf(-multiplier * f32::from(scale_factors[band])),
        None => 1.0,
    };
    Ok(sign * magnitude * gain * attenuation)
}

pub(crate) fn mpeg1_layer3_long_perceptual_distortion_with_scalefactors(
    spectrum: &[f32],
    quantized: &[i32],
    global_gain: u8,
    scale_factors: &[u8; MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT],
    scalefac_scale: bool,
    sample_rate: u32,
    allowed_noise: &[f64; MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT],
) -> Result<f64, Error> {
    if spectrum.len() != quantized.len() {
        return Err(Error::InvalidInput(
            "MP3 perceptual distortion inputs must match",
        ));
    }

    let mut distortion = 0.0_f64;
    for (band, &allowed) in allowed_noise.iter().enumerate() {
        let (start, end) = mpeg1_layer3_long_scalefactor_band_range(band, sample_rate)?;
        let mut noise = 0.0_f64;
        for line in start..end.min(spectrum.len()) {
            let reconstructed = requantize_mpeg1_layer3_long_line_with_scalefactors(
                quantized[line],
                line,
                global_gain,
                scale_factors,
                scalefac_scale,
                sample_rate,
            )?;
            let error = f64::from(reconstructed - spectrum[line]);
            noise += error * error;
        }
        if allowed.is_finite() {
            distortion += noise / allowed.max(1.0e-12);
        }
    }
    Ok(distortion)
}

pub(crate) fn mpeg1_layer3_scale_factor_complexity(
    scale_factors: &[u8; MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT],
) -> (usize, u32) {
    (
        scale_factors
            .iter()
            .filter(|&&scale_factor| scale_factor != 0)
            .count(),
        scale_factors
            .iter()
            .map(|&scale_factor| u32::from(scale_factor))
            .sum(),
    )
}

pub(crate) fn mpeg1_layer3_quality_guard_candidate_is_better(
    previous: &Layer3QualityGuardPerceptualCandidate,
    candidate: &Layer3QualityGuardPerceptualCandidate,
) -> bool {
    const DISTORTION_TIE_EPSILON: f64 = 1.0e-9;
    if candidate.distortion + DISTORTION_TIE_EPSILON < previous.distortion {
        return true;
    }
    if (candidate.distortion - previous.distortion).abs() <= DISTORTION_TIE_EPSILON {
        return mpeg1_layer3_scale_factor_complexity(&candidate.scale_factors)
            < mpeg1_layer3_scale_factor_complexity(&previous.scale_factors);
    }
    false
}

pub(crate) fn select_mpeg1_layer3_quality_guard_perceptual_candidate(
    spectrum: &[f32],
    scale_factors: &[u8; MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT],
    step: f32,
    scalefac_scale: bool,
    sample_rate: u32,
    allowed_noise: &[f64; MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT],
) -> Result<Layer3QualityGuardPerceptualCandidate, Error> {
    let mut selected: Option<Layer3QualityGuardPerceptualCandidate> = None;
    let mut last_error: Option<Error> = None;
    for candidate_scale_factors in [*scale_factors] {
        let perceptual_quantized = match quantize_mpeg1_layer3_long_spectrum_with_scalefactors(
            spectrum,
            step,
            &candidate_scale_factors,
            scalefac_scale,
            sample_rate,
        ) {
            Ok(quantized) => quantized,
            Err(err) => {
                last_error = Some(err);
                continue;
            }
        };
        let perceptual_global_gain =
            calibrated_global_gain_for_granule(&perceptual_quantized, step);
        let perceptual_distortion = mpeg1_layer3_long_perceptual_distortion_with_scalefactors(
            spectrum,
            &perceptual_quantized,
            perceptual_global_gain,
            &candidate_scale_factors,
            scalefac_scale,
            sample_rate,
            allowed_noise,
        )?;
        let candidate = Layer3QualityGuardPerceptualCandidate {
            scale_factors: candidate_scale_factors,
            quantized: perceptual_quantized,
            scalefac_scale,
            global_gain: perceptual_global_gain,
            distortion: perceptual_distortion,
        };
        selected = match selected {
            Some(previous)
                if !mpeg1_layer3_quality_guard_candidate_is_better(&previous, &candidate) =>
            {
                Some(previous)
            }
            _ => Some(candidate),
        };
    }
    selected.ok_or_else(|| {
        last_error.unwrap_or(Error::UnsupportedFeature(
            "MP3 quality guard perceptual candidate",
        ))
    })
}

pub(crate) fn centered_mpeg1_layer3_psychoacoustic_pcm_window(
    pcm: &AudioBuffer,
    channel: usize,
    start_frame: usize,
    spectrum_len: usize,
) -> Vec<f64> {
    let offset = (MPEG1_LAYER3_PSY_FFT_LEN.saturating_sub(spectrum_len)) / 2;
    let window_start = start_frame as isize - offset as isize;
    (0..MPEG1_LAYER3_PSY_FFT_LEN)
        .map(|n| {
            f64::from(channel_sample_or_zero(
                pcm,
                channel,
                window_start + n as isize,
            ))
        })
        .collect()
}

pub(crate) fn pack_mpeg1_layer3_pcm_long_block_with_perceptual_quality_guard_and_table_provider(
    granule: &mut Layer3GranuleChannelInfo,
    pcm: &AudioBuffer,
    channel: usize,
    start_frame: usize,
    step: f32,
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<Layer3QualityGuardGranulePayload, Error> {
    let spectrum = layer3_perceptual_quantizer_spectrum(pcm, channel, start_frame)?;
    let pcm_window =
        centered_mpeg1_layer3_psychoacoustic_pcm_window(pcm, channel, start_frame, spectrum.len());
    let allowed_noise = psychoacoustic::perceptual_long_block_allowed_noise(
        &spectrum,
        &pcm_window,
        pcm.sample_rate,
    )?;
    let zero_scale_factors = [0_u8; MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT];
    let calibrated_quantized = quantize_mpeg1_layer3_long_spectrum_with_scalefactors(
        &spectrum,
        step,
        &zero_scale_factors,
        false,
        pcm.sample_rate,
    )?;
    let calibrated_global_gain = calibrated_global_gain_for_granule(&calibrated_quantized, step);
    let calibrated_distortion = mpeg1_layer3_long_perceptual_distortion_with_scalefactors(
        &spectrum,
        &calibrated_quantized,
        calibrated_global_gain,
        &zero_scale_factors,
        false,
        pcm.sample_rate,
        &allowed_noise,
    )?;

    let perceptual_candidate: Result<Layer3QualityGuardPerceptualCandidate, Error> = (|| {
        let perceptual_scalefac_scale = false;
        let perceptual_scale_factors =
            select_centered_mpeg1_layer3_psychoacoustic_long_scale_factors(
                pcm,
                channel,
                start_frame,
                &spectrum,
                step,
                perceptual_scalefac_scale,
            )?;
        select_mpeg1_layer3_quality_guard_perceptual_candidate(
            &spectrum,
            &perceptual_scale_factors,
            step,
            perceptual_scalefac_scale,
            pcm.sample_rate,
            &allowed_noise,
        )
    })();

    let (quality_guard_compared_granules, quality_guard_distortion_delta) =
        match perceptual_candidate.as_ref() {
            Ok(candidate) => (1, calibrated_distortion - candidate.distortion),
            Err(_) => (0, 0.0),
        };
    let used_perceptual = perceptual_candidate.is_ok();
    let (scale_factors, quantized, scalefac_scale, global_gain) =
        if let (true, Ok(candidate)) = (used_perceptual, perceptual_candidate) {
            (
                candidate.scale_factors,
                candidate.quantized,
                candidate.scalefac_scale,
                candidate.global_gain,
            )
        } else {
            (
                zero_scale_factors,
                calibrated_quantized,
                false,
                calibrated_global_gain,
            )
        };

    granule.scalefac_scale = scalefac_scale;
    let packed = pack_mpeg1_layer3_long_quantized_spectrum_with_table_provider(
        granule,
        &scale_factors,
        &quantized,
        provider,
    )?;
    granule.global_gain = global_gain;
    Ok(Layer3QualityGuardGranulePayload {
        bits: packed,
        used_perceptual,
        compared_granules: quality_guard_compared_granules,
        distortion_delta: quality_guard_distortion_delta,
    })
}

pub(crate) fn select_centered_mpeg1_layer3_psychoacoustic_long_scale_factors(
    pcm: &AudioBuffer,
    channel: usize,
    start_frame: usize,
    mdct_spectrum: &[f32],
    step: f32,
    scalefac_scale: bool,
) -> Result<[u8; MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT], Error> {
    // Centre the analysis FFT on the granule, zero-padding past stream edges.
    let pcm_window = centered_mpeg1_layer3_psychoacoustic_pcm_window(
        pcm,
        channel,
        start_frame,
        mdct_spectrum.len(),
    );

    psychoacoustic::perceptual_long_block_scalefactors(
        mdct_spectrum,
        &pcm_window,
        step,
        scalefac_scale,
        pcm.sample_rate,
    )
}

pub(crate) fn count_mpeg1_layer3_pcm_frame_perceptual_nonzero_scale_factors(
    header: FrameHeader,
    pcm: &AudioBuffer,
    start_frame: usize,
    step: f32,
) -> Result<usize, Error> {
    let mut count = 0usize;
    for granule in 0..header.layer3_granule_count() {
        let granule_start = start_frame
            .checked_add(
                granule
                    .checked_mul(576)
                    .ok_or(Error::InvalidInput("MP3 granule start frame overflows"))?,
            )
            .ok_or(Error::InvalidInput("MP3 granule start frame overflows"))?;
        for channel in 0..header.channel_count() {
            let quantizer_spectrum =
                layer3_perceptual_quantizer_spectrum(pcm, channel, granule_start)?;
            let scale_factors = select_centered_mpeg1_layer3_psychoacoustic_long_scale_factors(
                pcm,
                channel,
                granule_start,
                &quantizer_spectrum,
                step,
                false,
            )?;
            count += scale_factors
                .iter()
                .filter(|&&scale_factor| scale_factor != 0)
                .count();
        }
    }
    Ok(count)
}

/// Picks the `global_gain` for a packed granule: the step-inverting value for a
/// granule that carries energy, or the ISO reference gain (210) for an all-zero
/// granule whose gain is acoustically irrelevant.
pub(crate) fn calibrated_global_gain_for_granule(quantized: &[i32], step: f32) -> u8 {
    if quantized.iter().all(|&line| line == 0) || !step.is_finite() || step <= 0.0 {
        return 210;
    }
    // `mpeg1_layer3_global_gain_for_step` inverts only the quantizer step. The
    // decoder's hybrid IMDCT carries no 2/N normalization, so its 18-point
    // inverse reconstructs N/2 = 9 times the encoded magnitude. Lower the gain
    // by 4*log2(9) to make the full encode/decode chain unity. Rounding once
    // over the combined expression keeps the residual gain error minimal.
    let imdct_gain_offset = 4.0 * 9.0_f32.log2();
    let raw = (210.0 + (16.0 / 3.0) * step.log2() - imdct_gain_offset).round();
    raw.clamp(0.0, 255.0) as u8
}
