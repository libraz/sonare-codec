use super::*;

/// Builds one MPEG-1 Layer III short-block (block_type 2) main-data payload from
/// PCM with calibrated gain and flat scale factors.
///
/// The short counterpart of
/// [`pack_mpeg1_layer3_pcm_long_block_with_calibrated_gain_for_granule`]: the
/// quantizer `step` is folded entirely into `global_gain`
/// ([`calibrated_short_global_gain_for_granule`]) and the scale factors and
/// `subblock_gain` are left at zero, so the decoder's per-line short
/// requantization inverts the encoder's quantization without per-band scaling.
pub fn pack_mpeg1_layer3_pcm_short_block_with_calibrated_gain_for_granule(
    granule: &mut Layer3GranuleChannelInfo,
    pcm: &AudioBuffer,
    channel: usize,
    start_frame: usize,
    step: f32,
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<PackedBits, Error> {
    let quantized = quantize_pcm_short_block(pcm, channel, start_frame, step)?;
    let scale_factors = [[0_u8; LAYER3_SHORT_WINDOWS]; LAYER3_SHORT_SCALE_FACTOR_BANDS];
    let subblock_gain = [0_u8; LAYER3_SHORT_WINDOWS];
    granule.scalefac_scale = false;
    let packed = pack_mpeg1_layer3_short_quantized_spectrum_with_table_provider(
        granule,
        &scale_factors,
        &subblock_gain,
        &quantized,
        provider,
    )?;
    granule.global_gain = calibrated_short_global_gain_for_granule(&quantized, step);
    Ok(packed)
}

/// Packs every granule/channel of one frame as a short block, returning the
/// frame side info and concatenated main data.
fn pack_mpeg1_layer3_pcm_frame_all_short_payloads_with_table_provider(
    header: FrameHeader,
    pcm: &AudioBuffer,
    start_frame: usize,
    step: f32,
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<(Layer3SideInfo, PackedBits), Error> {
    let mut side_info = prepare_mpeg1_layer3_pcm_frame_side_info(header, pcm)?;
    let mut payloads = Vec::with_capacity(header.layer3_granule_count() * header.channel_count());
    for granule in 0..header.layer3_granule_count() {
        let granule_start = start_frame
            .checked_add(
                granule
                    .checked_mul(576)
                    .ok_or(Error::InvalidInput("MP3 granule start frame overflows"))?,
            )
            .ok_or(Error::InvalidInput("MP3 granule start frame overflows"))?;
        for channel in 0..header.channel_count() {
            let payload = pack_mpeg1_layer3_pcm_short_block_with_calibrated_gain_for_granule(
                &mut side_info.granules[granule][channel],
                pcm,
                channel,
                granule_start,
                step,
                provider,
            )?;
            payloads.push(payload);
        }
    }
    let main_data = pack_layer3_main_data_payloads(&header, &payloads)?;
    Ok((side_info, main_data))
}

/// Assembles one self-contained MPEG-1 Layer III frame coding every granule as a
/// short block.
pub fn assemble_mpeg1_layer3_pcm_frame_all_short_with_table_provider(
    header: FrameHeader,
    pcm: &AudioBuffer,
    start_frame: usize,
    step: f32,
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<Vec<u8>, Error> {
    let (side_info, main_data) =
        pack_mpeg1_layer3_pcm_frame_all_short_payloads_with_table_provider(
            header,
            pcm,
            start_frame,
            step,
            provider,
        )?;
    assemble_layer3_frame(header, &side_info, &main_data.bytes)
}

/// Encodes PCM as MPEG-1 Layer III frames coding every granule as a short
/// block, with an explicit header.
///
/// This is an experimental block-switching path used to validate the
/// short-block analysis, quantization, and Huffman packing end to end against a
/// reference decoder. The production `encode()` route stays on the long-block
/// path until block switching is gated by the readiness oracle.
pub fn encode_mpeg1_layer3_pcm_frames_with_header_all_short_and_table_provider(
    header: FrameHeader,
    pcm: &AudioBuffer,
    step: f32,
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<Vec<u8>, Error> {
    let frame_count = layer3_frame_count(header, pcm)?;
    let mut out = Vec::with_capacity(header.frame_len() * frame_count);
    for frame_index in 0..frame_count {
        let start_frame = frame_index
            .checked_mul(usize::from(header.samples_per_frame()))
            .ok_or(Error::InvalidInput("MP3 frame start overflows"))?;
        out.extend_from_slice(
            &assemble_mpeg1_layer3_pcm_frame_all_short_with_table_provider(
                header,
                pcm,
                start_frame,
                step,
                provider,
            )?,
        );
    }
    Ok(out)
}

/// Encodes PCM as all-short MPEG-1 Layer III frames, deriving the header from
/// the PCM format.
pub fn encode_mpeg1_layer3_pcm_frames_all_short_and_table_provider(
    pcm: &AudioBuffer,
    step: f32,
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<Vec<u8>, Error> {
    let header = mpeg1_layer3_header_for_pcm(pcm)?;
    encode_mpeg1_layer3_pcm_frames_with_header_all_short_and_table_provider(
        header, pcm, step, provider,
    )
}

/// Encodes PCM as all-long calibrated MPEG-1 Layer III frames, deriving the
/// header from the PCM format.
///
/// Shares the calibrated-gain, self-contained-frame pipeline of
/// [`encode_mpeg1_layer3_pcm_frames_with_block_switching_and_table_provider`]
/// but codes every granule as a long block. Used as the apples-to-apples
/// baseline for measuring the pre-echo that block switching removes.
pub fn encode_mpeg1_layer3_pcm_frames_all_long_calibrated_and_table_provider(
    pcm: &AudioBuffer,
    step: f32,
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<Vec<u8>, Error> {
    let header = mpeg1_layer3_header_for_pcm(pcm)?;
    let frame_count = layer3_frame_count(header, pcm)?;
    let granule_count = header.layer3_granule_count();
    let samples_per_frame = usize::from(header.samples_per_frame());

    let mut out = Vec::with_capacity(header.frame_len() * frame_count);
    for frame_index in 0..frame_count {
        let start_frame = frame_index
            .checked_mul(samples_per_frame)
            .ok_or(Error::InvalidInput("MP3 frame start overflows"))?;
        let mut side_info = prepare_mpeg1_layer3_pcm_frame_side_info(header, pcm)?;
        let mut payloads = Vec::with_capacity(granule_count * header.channel_count());
        for granule in 0..granule_count {
            let granule_start = start_frame
                .checked_add(
                    granule
                        .checked_mul(576)
                        .ok_or(Error::InvalidInput("MP3 granule start frame overflows"))?,
                )
                .ok_or(Error::InvalidInput("MP3 granule start frame overflows"))?;
            for channel in 0..header.channel_count() {
                let payload = pack_mpeg1_layer3_pcm_block_for_granule(
                    &mut side_info.granules[granule][channel],
                    pcm,
                    channel,
                    granule_start,
                    step,
                    Layer3BlockType::Long,
                    provider,
                )?;
                payloads.push(payload);
            }
        }
        let main_data = pack_layer3_main_data_payloads(&header, &payloads)?;
        out.extend_from_slice(&assemble_layer3_frame(
            header,
            &side_info,
            &main_data.bytes,
        )?);
    }
    Ok(out)
}

/// Builds one MPEG-1 Layer III transition-block (start/stop) main-data payload
/// from PCM with calibrated gain and flat scale factors.
pub fn pack_mpeg1_layer3_pcm_transition_block_with_calibrated_gain_for_granule(
    granule: &mut Layer3GranuleChannelInfo,
    pcm: &AudioBuffer,
    channel: usize,
    start_frame: usize,
    step: f32,
    block_type: Layer3BlockType,
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<PackedBits, Error> {
    let quantized = quantize_pcm_transition_block(pcm, channel, start_frame, step, block_type)?;
    let scale_factors = [0_u8; MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT];
    granule.scalefac_scale = false;
    let packed = pack_mpeg1_layer3_transition_quantized_spectrum_with_table_provider(
        granule,
        &scale_factors,
        block_type,
        &quantized,
        pcm.sample_rate,
        provider,
    )?;
    granule.global_gain = calibrated_global_gain_for_granule(&quantized, step);
    Ok(packed)
}

/// Packs one granule/channel with the block type chosen by the schedule.
fn pack_mpeg1_layer3_pcm_block_for_granule(
    granule: &mut Layer3GranuleChannelInfo,
    pcm: &AudioBuffer,
    channel: usize,
    start_frame: usize,
    step: f32,
    block_type: Layer3BlockType,
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<PackedBits, Error> {
    match block_type {
        Layer3BlockType::Long => {
            pack_mpeg1_layer3_pcm_long_block_with_calibrated_gain_and_table_provider(
                granule,
                pcm,
                channel,
                start_frame,
                step,
                provider,
            )
        }
        Layer3BlockType::Short => {
            pack_mpeg1_layer3_pcm_short_block_with_calibrated_gain_for_granule(
                granule,
                pcm,
                channel,
                start_frame,
                step,
                provider,
            )
        }
        Layer3BlockType::Start | Layer3BlockType::Stop => {
            pack_mpeg1_layer3_pcm_transition_block_with_calibrated_gain_for_granule(
                granule,
                pcm,
                channel,
                start_frame,
                step,
                block_type,
                provider,
            )
        }
    }
}

/// Builds the per-granule block-type schedule for a whole stream from the
/// transient content of each 576-sample granule (analysed on channel 0).
fn layer3_block_switching_schedule(
    pcm: &AudioBuffer,
    total_granules: usize,
) -> Result<Vec<Layer3BlockType>, Error> {
    let mut transient = Vec::with_capacity(total_granules);
    for granule in 0..total_granules {
        let granule_start = granule
            .checked_mul(576)
            .ok_or(Error::InvalidInput("MP3 granule start frame overflows"))?;
        let samples: Vec<f32> = (0..576)
            .map(|i| channel_sample_or_zero(pcm, 0, (granule_start + i) as isize))
            .collect();
        transient.push(layer3_granule_is_transient(&samples));
    }
    Ok(build_layer3_block_schedule(&transient))
}

/// Encodes PCM as MPEG-1 Layer III frames with long/short block switching.
///
/// A transient detector marks each granule, [`build_layer3_block_schedule`]
/// brackets every short run with start/stop transition blocks, and each granule
/// is packed as the scheduled block type. This is the experimental
/// block-switching path validated end to end against a reference decoder; the
/// production `encode()` route stays on the long-block path until block
/// switching is gated by the readiness oracle.
pub fn encode_mpeg1_layer3_pcm_frames_with_block_switching_and_table_provider(
    pcm: &AudioBuffer,
    step: f32,
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<Vec<u8>, Error> {
    let header = mpeg1_layer3_header_for_pcm(pcm)?;
    let frame_count = layer3_frame_count(header, pcm)?;
    let granule_count = header.layer3_granule_count();
    let samples_per_frame = usize::from(header.samples_per_frame());
    let total_granules = frame_count
        .checked_mul(granule_count)
        .ok_or(Error::InvalidInput("MP3 granule count overflows"))?;
    let schedule = layer3_block_switching_schedule(pcm, total_granules)?;

    let mut out = Vec::with_capacity(header.frame_len() * frame_count);
    for frame_index in 0..frame_count {
        let start_frame = frame_index
            .checked_mul(samples_per_frame)
            .ok_or(Error::InvalidInput("MP3 frame start overflows"))?;
        let mut side_info = prepare_mpeg1_layer3_pcm_frame_side_info(header, pcm)?;
        let mut payloads = Vec::with_capacity(granule_count * header.channel_count());
        for granule in 0..granule_count {
            let global_granule = frame_index * granule_count + granule;
            let block_type = schedule
                .get(global_granule)
                .copied()
                .unwrap_or(Layer3BlockType::Long);
            let granule_start = start_frame
                .checked_add(
                    granule
                        .checked_mul(576)
                        .ok_or(Error::InvalidInput("MP3 granule start frame overflows"))?,
                )
                .ok_or(Error::InvalidInput("MP3 granule start frame overflows"))?;
            for channel in 0..header.channel_count() {
                let payload = pack_mpeg1_layer3_pcm_block_for_granule(
                    &mut side_info.granules[granule][channel],
                    pcm,
                    channel,
                    granule_start,
                    step,
                    block_type,
                    provider,
                )?;
                payloads.push(payload);
            }
        }
        let main_data = pack_layer3_main_data_payloads(&header, &payloads)?;
        out.extend_from_slice(&assemble_layer3_frame(
            header,
            &side_info,
            &main_data.bytes,
        )?);
    }
    Ok(out)
}
