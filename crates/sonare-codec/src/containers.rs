use super::*;

#[cfg(feature = "aac")]
pub fn aac_standard_selected_scale_factor_frame_details_with_bitrate(
    pcm: &AudioBuffer,
    target_bitrate_bps: u32,
    global_gain: u8,
) -> Result<Vec<AacPcmFrameStepSelection>, Error> {
    aac_standard_selected_scale_factor_frame_details_with_magnitude_bias_and_bitrate(
        pcm,
        target_bitrate_bps,
        global_gain,
        0,
    )
}

#[cfg(feature = "aac")]
pub fn encode_aac_adts_with_standard_spectral_offsets_and_selected_scale_factors_and_bitrate(
    pcm: &AudioBuffer,
    target_bitrate_bps: u32,
    global_gain: u8,
) -> Result<Vec<u8>, Error> {
    encode_aac_adts_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_bitrate(
        pcm,
        target_bitrate_bps,
        global_gain,
        0,
    )
}

#[cfg(feature = "aac")]
pub fn encode_m4a_with_bitrate(
    pcm: &AudioBuffer,
    target_bitrate_bps: u32,
) -> Result<Vec<u8>, Error> {
    mux_aac_adts_as_m4a(&encode_aac_adts_with_bitrate(pcm, target_bitrate_bps)?)
}

#[cfg(feature = "aac")]
pub fn encode_m4a_with_selected_scale_factors_and_bitrate(
    pcm: &AudioBuffer,
    target_bitrate_bps: u32,
) -> Result<Vec<u8>, Error> {
    mux_aac_adts_as_m4a(&encode_aac_adts_with_selected_scale_factors_and_bitrate(
        pcm,
        target_bitrate_bps,
    )?)
}

#[cfg(feature = "aac")]
pub fn encode_m4a_with_standard_spectral_offsets_and_bitrate(
    pcm: &AudioBuffer,
    target_bitrate_bps: u32,
    global_gain: u8,
) -> Result<Vec<u8>, Error> {
    mux_aac_adts_as_m4a(&encode_aac_adts_with_standard_spectral_offsets_and_bitrate(
        pcm,
        target_bitrate_bps,
        global_gain,
    )?)
}

#[cfg(feature = "aac")]
pub fn encode_m4a_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_bitrate(
    pcm: &AudioBuffer,
    target_bitrate_bps: u32,
    global_gain: u8,
    scale_factor_magnitude_bias: i16,
) -> Result<Vec<u8>, Error> {
    mux_aac_adts_as_m4a(
        &encode_aac_adts_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_bitrate(
            pcm,
            target_bitrate_bps,
            global_gain,
            scale_factor_magnitude_bias,
        )?,
    )
}

#[cfg(feature = "aac")]
pub fn encode_m4a_with_recommended_standard_spectral_offsets_and_selected_scale_factors_and_bitrate(
    pcm: &AudioBuffer,
    target_bitrate_bps: u32,
) -> Result<Vec<u8>, Error> {
    mux_aac_adts_as_m4a(
        &encode_aac_adts_with_recommended_standard_spectral_offsets_and_selected_scale_factors_and_bitrate(
            pcm,
            target_bitrate_bps,
        )?,
    )
}

#[cfg(feature = "aac")]
pub fn encode_m4a_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_max_quantized_abs_and_bitrate(
    pcm: &AudioBuffer,
    target_bitrate_bps: u32,
    global_gain: u8,
    scale_factor_magnitude_bias: i16,
    max_quantized_abs: u32,
) -> Result<Vec<u8>, Error> {
    mux_aac_adts_as_m4a(
        &encode_aac_adts_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_max_quantized_abs_and_bitrate(
            pcm,
            target_bitrate_bps,
            global_gain,
            scale_factor_magnitude_bias,
            max_quantized_abs,
        )?,
    )
}

#[cfg(feature = "aac")]
pub fn encode_m4a_with_recommended_standard_spectral_offsets_and_selected_scale_factors_max_quantized_abs_and_bitrate(
    pcm: &AudioBuffer,
    target_bitrate_bps: u32,
    max_quantized_abs: u32,
) -> Result<Vec<u8>, Error> {
    mux_aac_adts_as_m4a(
        &encode_aac_adts_with_recommended_standard_spectral_offsets_and_selected_scale_factors_max_quantized_abs_and_bitrate(
            pcm,
            target_bitrate_bps,
            max_quantized_abs,
        )?,
    )
}

#[cfg(feature = "aac")]
pub fn encode_m4a_with_balanced_standard_spectral_offsets_and_selected_scale_factors_and_bitrate(
    pcm: &AudioBuffer,
    target_bitrate_bps: u32,
) -> Result<Vec<u8>, Error> {
    mux_aac_adts_as_m4a(
        &encode_aac_adts_with_balanced_standard_spectral_offsets_and_selected_scale_factors_and_bitrate(
            pcm,
            target_bitrate_bps,
        )?,
    )
}

#[cfg(feature = "aac")]
pub fn encode_m4a_with_standard_spectral_offsets_and_selected_scale_factors_and_bitrate(
    pcm: &AudioBuffer,
    target_bitrate_bps: u32,
    global_gain: u8,
) -> Result<Vec<u8>, Error> {
    encode_m4a_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_bitrate(
        pcm,
        target_bitrate_bps,
        global_gain,
        0,
    )
}

#[cfg(feature = "aac")]
pub fn frame_aac_adts(config: AdtsConfig, access_unit: &[u8]) -> Result<Vec<u8>, Error> {
    sc_aac::frame_adts(config, access_unit)
}

#[cfg(feature = "aac")]
pub fn frame_aac_adts_stream<'a>(
    config: AdtsConfig,
    access_units: impl IntoIterator<Item = &'a [u8]>,
) -> Result<Vec<u8>, Error> {
    sc_aac::frame_adts_stream(config, access_units)
}

#[cfg(feature = "aac")]
pub fn mux_aac_adts_as_m4a(adts: &[u8]) -> Result<Vec<u8>, Error> {
    sc_aac::mux_adts_as_m4a(adts)
}

#[cfg(feature = "aac")]
pub fn demux_m4a_as_aac_adts(input: &[u8]) -> Result<Vec<u8>, Error> {
    sc_aac::demux_m4a_as_adts(input)
}

#[cfg(feature = "aac")]
pub(crate) fn constant_aac_scale_factors_by_frame(
    pcm: &AudioBuffer,
    scale_factor: i16,
    band_count: usize,
) -> Vec<Vec<i16>> {
    let frame_count = pcm.samples.len().div_ceil(usize::from(pcm.channels) * 1024);
    (0..frame_count)
        .map(|_| vec![scale_factor; band_count])
        .collect()
}

#[cfg(feature = "wav")]
pub(crate) fn encode_wav_impl(pcm: &AudioBuffer) -> Result<Vec<u8>, Error> {
    sc_wav::encode(pcm)
}

#[cfg(not(feature = "wav"))]
pub(crate) fn encode_wav_impl(_pcm: &AudioBuffer) -> Result<Vec<u8>, Error> {
    Err(Error::UnsupportedFormat)
}

#[cfg(feature = "flac")]
pub(crate) fn encode_flac_impl(pcm: &AudioBuffer) -> Result<Vec<u8>, Error> {
    sc_flac::encode(pcm)
}

#[cfg(not(feature = "flac"))]
pub(crate) fn encode_flac_impl(_pcm: &AudioBuffer) -> Result<Vec<u8>, Error> {
    Err(Error::UnsupportedFormat)
}

#[cfg(feature = "mp3")]
pub(crate) fn encode_mp3_impl(pcm: &AudioBuffer) -> Result<Vec<u8>, Error> {
    sc_mp3::encode(pcm)
}

#[cfg(not(feature = "mp3"))]
pub(crate) fn encode_mp3_impl(_pcm: &AudioBuffer) -> Result<Vec<u8>, Error> {
    Err(Error::UnsupportedFormat)
}

#[cfg(feature = "vorbis")]
pub(crate) fn encode_vorbis_impl(pcm: &AudioBuffer) -> Result<Vec<u8>, Error> {
    sc_vorbis::encode(pcm)
}

#[cfg(not(feature = "vorbis"))]
pub(crate) fn encode_vorbis_impl(_pcm: &AudioBuffer) -> Result<Vec<u8>, Error> {
    Err(Error::UnsupportedFormat)
}

#[cfg(feature = "opus")]
pub(crate) fn encode_opus_impl(pcm: &AudioBuffer) -> Result<Vec<u8>, Error> {
    sc_opus::encode(pcm)
}

#[cfg(not(feature = "opus"))]
pub(crate) fn encode_opus_impl(_pcm: &AudioBuffer) -> Result<Vec<u8>, Error> {
    Err(Error::UnsupportedFormat)
}

#[cfg(feature = "aac")]
pub(crate) fn encode_aac_impl(pcm: &AudioBuffer) -> Result<Vec<u8>, Error> {
    sc_aac::encode(pcm)
}

#[cfg(not(feature = "aac"))]
pub(crate) fn encode_aac_impl(_pcm: &AudioBuffer) -> Result<Vec<u8>, Error> {
    Err(Error::UnsupportedFormat)
}
