use super::*;
use std::cell::RefCell;

/// Identifies the PCM buffer a cached spectrum was derived from.
///
/// The quantizer step search re-derives the same MDCT spectrum for every
/// candidate step of a granule; memoizing it on `(channel, start_frame)` removes
/// that redundancy. The identity guards against serving a spectrum computed from
/// a different buffer: the cache is cleared whenever the active buffer changes.
#[derive(Clone, Copy, PartialEq, Eq)]
struct CachedPcmIdentity {
    ptr: usize,
    len: usize,
    sample_rate: u32,
    channels: u16,
    fingerprint: u64,
}

fn cached_pcm_identity(pcm: &AudioBuffer) -> CachedPcmIdentity {
    let samples = &pcm.samples;
    let len = samples.len();
    // Cheap content fingerprint so an unrelated buffer reusing a freed
    // allocation's address+length cannot alias a stale entry.
    let fingerprint = if len == 0 {
        0
    } else {
        u64::from(samples[0].to_bits())
            ^ u64::from(samples[len / 2].to_bits()).rotate_left(21)
            ^ u64::from(samples[len - 1].to_bits()).rotate_left(42)
            ^ (len as u64)
    };
    CachedPcmIdentity {
        ptr: samples.as_ptr() as usize,
        len,
        sample_rate: pcm.sample_rate,
        channels: pcm.channels,
        fingerprint,
    }
}

/// Distinguishes the two spectrum flavours sharing the cache keyspace.
const SPECTRUM_KIND_LONG_BLOCK: u8 = 0;
const SPECTRUM_KIND_PERCEPTUAL: u8 = 1;

/// Bounded LRU; one frame's step search touches at most a handful of
/// `(granule, channel)` spectra, so a small cap keeps every candidate a hit
/// while bounding memory across a whole stream.
const SPECTRUM_CACHE_CAP: usize = 8;

struct SpectrumCache {
    pcm: Option<CachedPcmIdentity>,
    entries: Vec<((u8, usize, usize), Vec<f32>)>,
}

thread_local! {
    static SPECTRUM_CACHE: RefCell<SpectrumCache> =
        const { RefCell::new(SpectrumCache { pcm: None, entries: Vec::new() }) };
}

/// Returns the spectrum for `(kind, channel, start_frame)`, computing it via
/// `compute` on a miss. Bit-identical to calling `compute` directly: only
/// recomputation is elided, never the values.
fn cached_spectrum<F>(
    kind: u8,
    pcm: &AudioBuffer,
    channel: usize,
    start_frame: usize,
    compute: F,
) -> Result<Vec<f32>, Error>
where
    F: FnOnce() -> Result<Vec<f32>, Error>,
{
    let id = cached_pcm_identity(pcm);
    let key = (kind, channel, start_frame);

    let hit = SPECTRUM_CACHE.with(|cell| {
        let mut cache = cell.borrow_mut();
        if cache.pcm != Some(id) {
            cache.pcm = Some(id);
            cache.entries.clear();
            return None;
        }
        if let Some(pos) = cache.entries.iter().position(|(k, _)| *k == key) {
            // Promote to most-recently-used.
            let entry = cache.entries.remove(pos);
            let value = entry.1.clone();
            cache.entries.push(entry);
            Some(value)
        } else {
            None
        }
    });
    if let Some(value) = hit {
        return Ok(value);
    }

    let value = compute()?;
    SPECTRUM_CACHE.with(|cell| {
        let mut cache = cell.borrow_mut();
        // Only insert if the buffer is still the one we keyed against; a nested
        // compute() could have cleared the cache for a different buffer.
        if cache.pcm == Some(id) {
            if cache.entries.len() >= SPECTRUM_CACHE_CAP {
                cache.entries.remove(0);
            }
            cache.entries.push((key, value.clone()));
        }
    });
    Ok(value)
}

/// Per-band allowed quantization noise for one long granule. Step-invariant: it
/// derives only from the granule's MDCT spectrum and analysis FFT, so the step
/// search can reuse it for every candidate instead of re-running the FFT and
/// masking model each time.
type CachedAllowedNoise = [f64; MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT];

const ALLOWED_NOISE_CACHE_CAP: usize = 8;

struct AllowedNoiseCache {
    pcm: Option<CachedPcmIdentity>,
    entries: Vec<((usize, usize), CachedAllowedNoise)>,
}

thread_local! {
    static ALLOWED_NOISE_CACHE: RefCell<AllowedNoiseCache> =
        const { RefCell::new(AllowedNoiseCache { pcm: None, entries: Vec::new() }) };
}

/// Returns the allowed-noise target for `(channel, start_frame)`, computing it
/// via `compute` on a miss. Bit-identical to calling `compute` directly.
pub(crate) fn cached_perceptual_long_block_allowed_noise<F>(
    pcm: &AudioBuffer,
    channel: usize,
    start_frame: usize,
    compute: F,
) -> Result<CachedAllowedNoise, Error>
where
    F: FnOnce() -> Result<CachedAllowedNoise, Error>,
{
    let id = cached_pcm_identity(pcm);
    let key = (channel, start_frame);

    let hit = ALLOWED_NOISE_CACHE.with(|cell| {
        let mut cache = cell.borrow_mut();
        if cache.pcm != Some(id) {
            cache.pcm = Some(id);
            cache.entries.clear();
            return None;
        }
        if let Some(pos) = cache.entries.iter().position(|(k, _)| *k == key) {
            let entry = cache.entries.remove(pos);
            let value = entry.1;
            cache.entries.push(entry);
            Some(value)
        } else {
            None
        }
    });
    if let Some(value) = hit {
        return Ok(value);
    }

    let value = compute()?;
    ALLOWED_NOISE_CACHE.with(|cell| {
        let mut cache = cell.borrow_mut();
        if cache.pcm == Some(id) {
            if cache.entries.len() >= ALLOWED_NOISE_CACHE_CAP {
                cache.entries.remove(0);
            }
            cache.entries.push((key, value));
        }
    });
    Ok(value)
}

pub(crate) fn mpeg1_layer3_granule_perceptual_entropy(
    pcm: &AudioBuffer,
    channel: usize,
    granule_start: usize,
) -> Result<f64, Error> {
    let pcm_window =
        centered_mpeg1_layer3_psychoacoustic_pcm_window(pcm, channel, granule_start, 576);
    let window = mpeg1_layer3_psychoacoustic_window();
    let windowed: Vec<f64> = pcm_window
        .iter()
        .zip(window.iter())
        .map(|(&sample, &scale)| sample * scale)
        .collect();
    let energy = psychoacoustic::power_spectrum(&windowed)?;
    let tonality = psychoacoustic::windowed_tonality(&energy, 17)?;
    let barks = psychoacoustic::bin_barks(energy.len(), pcm.sample_rate, MPEG1_LAYER3_PSY_FFT_LEN)?;
    let threshold = psychoacoustic::spread_masking_threshold_per_bin(&energy, &barks, &tonality)?;
    psychoacoustic::perceptual_entropy(&energy, &threshold)
}

fn mpeg1_layer3_psychoacoustic_window() -> &'static [f64] {
    static WINDOW: std::sync::OnceLock<Vec<f64>> = std::sync::OnceLock::new();
    WINDOW.get_or_init(|| {
        psychoacoustic::hann_window(MPEG1_LAYER3_PSY_FFT_LEN)
            .expect("MP3 psychoacoustic FFT length is non-zero")
    })
}

/// Computes perceptual-entropy weighted target bits for each granule/channel.
///
/// The total target is the same CBR main-data capacity used by the Layer III
/// frame builder for the stream. This does not change encoding decisions yet;
/// it exposes the psychoacoustic rate-control signal that a later reservoir
/// selector can use to spend bits where the current PCM demands them.
pub fn select_mpeg1_layer3_perceptual_bit_allocation_with_bitrate(
    pcm: &AudioBuffer,
    bitrate_kbps: u16,
    crc_protected: bool,
    min_bits_per_granule_channel: usize,
) -> Result<Vec<Layer3PerceptualBitAllocation>, Error> {
    let base_header = layer3_header_for_capacity(
        pcm.sample_rate,
        pcm.channels,
        bitrate_kbps,
        false,
        crc_protected,
    )?;
    let frame_count = layer3_frame_count(base_header, pcm)?;
    let mut padding = Layer3PaddingSchedule::new(base_header)?;
    let samples_per_frame = usize::from(base_header.samples_per_frame());
    let allocation_count = frame_count
        .checked_mul(base_header.layer3_granule_count())
        .and_then(|count| count.checked_mul(base_header.channel_count()))
        .ok_or(Error::InvalidInput(
            "MP3 perceptual bit allocation count overflows",
        ))?;
    let mut entropies = Vec::with_capacity(allocation_count);
    let mut positions = Vec::with_capacity(allocation_count);
    let mut total_capacity_bits = 0usize;

    for frame_index in 0..frame_count {
        let frame_header = padding.next_header();
        total_capacity_bits = total_capacity_bits
            .checked_add(layer3_main_data_capacity_bits(frame_header)?)
            .ok_or(Error::InvalidInput(
                "MP3 perceptual bit allocation budget overflows",
            ))?;
        let frame_start = frame_index
            .checked_mul(samples_per_frame)
            .ok_or(Error::InvalidInput("MP3 frame start overflows"))?;
        for granule in 0..base_header.layer3_granule_count() {
            let granule_start = frame_start
                .checked_add(granule * 576)
                .ok_or(Error::InvalidInput("MP3 granule start overflows"))?;
            for channel in 0..base_header.channel_count() {
                entropies.push(mpeg1_layer3_granule_perceptual_entropy(
                    pcm,
                    channel,
                    granule_start,
                )?);
                positions.push((frame_index, granule, channel));
            }
        }
    }

    let targets = psychoacoustic::distribute_bits_by_perceptual_entropy(
        &entropies,
        total_capacity_bits,
        min_bits_per_granule_channel,
    )?;
    Ok(positions
        .into_iter()
        .zip(entropies)
        .zip(targets)
        .map(
            |(((frame_index, granule, channel), perceptual_entropy), target_bits)| {
                Layer3PerceptualBitAllocation {
                    frame_index,
                    granule,
                    channel,
                    perceptual_entropy,
                    target_bits,
                }
            },
        )
        .collect())
}

/// Selects the finest quantizer step and reports the payload cost relative to a
/// caller-provided bit budget.
pub fn select_mpeg1_layer3_pcm_frame_step_details_with_max_payload_bits_and_table_provider(
    header: FrameHeader,
    pcm: &AudioBuffer,
    start_frame: usize,
    candidates: &[f32],
    max_payload_bit_len: usize,
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<Layer3PcmFrameStepSelection, Error> {
    if max_payload_bit_len == 0 {
        return Err(Error::InvalidInput(
            "MP3 max payload bit length must be greater than zero",
        ));
    }
    if candidates.is_empty() {
        return Err(Error::InvalidInput(
            "MP3 quantizer step candidate list is empty",
        ));
    }
    let mut selected: Option<Layer3PcmFrameStepSelection> = None;
    for &step in candidates {
        if !step.is_finite() || step <= 0.0 {
            return Err(Error::InvalidInput(
                "MP3 quantizer step must be positive and finite",
            ));
        }
        if let Ok(selection) = evaluate_mpeg1_layer3_pcm_frame_step_with_table_provider(
            header,
            pcm,
            start_frame,
            step,
            provider,
        ) {
            let Some(selection) =
                limit_mpeg1_layer3_pcm_frame_step_selection(selection, max_payload_bit_len)
            else {
                continue;
            };
            selected = select_better_mpeg1_layer3_pcm_frame_step(selected, selection);
        }
    }
    selected.ok_or(Error::UnsupportedFeature("MP3 quantizer step search"))
}

/// Selects a perceptual-path quantizer step relative to a payload bit budget.
pub fn select_mpeg1_layer3_pcm_frame_perceptual_step_details_with_max_payload_bits_and_table_provider(
    header: FrameHeader,
    pcm: &AudioBuffer,
    start_frame: usize,
    candidates: &[f32],
    max_payload_bit_len: usize,
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<Layer3PcmFrameStepSelection, Error> {
    if max_payload_bit_len == 0 {
        return Err(Error::InvalidInput(
            "MP3 max payload bit length must be greater than zero",
        ));
    }
    if candidates.is_empty() {
        return Err(Error::InvalidInput(
            "MP3 quantizer step candidate list is empty",
        ));
    }
    let mut selected: Option<Layer3PcmFrameStepSelection> = None;
    for &step in candidates {
        if !step.is_finite() || step <= 0.0 {
            return Err(Error::InvalidInput(
                "MP3 quantizer step must be positive and finite",
            ));
        }
        if let Ok(selection) = evaluate_mpeg1_layer3_pcm_frame_perceptual_step_with_table_provider(
            header,
            pcm,
            start_frame,
            step,
            provider,
        ) {
            let Some(selection) =
                limit_mpeg1_layer3_pcm_frame_step_selection(selection, max_payload_bit_len)
            else {
                continue;
            };
            selected = select_better_mpeg1_layer3_pcm_frame_step(selected, selection);
        }
    }
    selected.ok_or(Error::UnsupportedFeature(
        "MP3 perceptual quantizer step search",
    ))
}

pub(crate) fn limit_mpeg1_layer3_pcm_frame_step_selection(
    mut selection: Layer3PcmFrameStepSelection,
    max_payload_bit_len: usize,
) -> Option<Layer3PcmFrameStepSelection> {
    if selection.payload_bit_len > max_payload_bit_len {
        return None;
    }
    selection.frame_capacity_bits = max_payload_bit_len;
    Some(selection)
}

pub(crate) fn select_better_mpeg1_layer3_pcm_frame_step(
    selected: Option<Layer3PcmFrameStepSelection>,
    selection: Layer3PcmFrameStepSelection,
) -> Option<Layer3PcmFrameStepSelection> {
    match selected {
        Some(previous)
            if selection.step > previous.step
                || (selection.step == previous.step
                    && selection.payload_bit_len <= previous.payload_bit_len) =>
        {
            Some(previous)
        }
        _ => Some(selection),
    }
}

pub(crate) fn evaluate_mpeg1_layer3_pcm_frame_step_with_table_provider(
    header: FrameHeader,
    pcm: &AudioBuffer,
    start_frame: usize,
    step: f32,
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<Layer3PcmFrameStepSelection, Error> {
    let (_side_info, main_data) = pack_mpeg1_layer3_pcm_frame_payloads_with_table_provider(
        header,
        pcm,
        start_frame,
        step,
        provider,
    )?;
    let frame_capacity_bytes = layer3_main_data_capacity_bytes(header)?;
    if main_data.bytes.len() > frame_capacity_bytes {
        return Err(Error::InvalidInput("MP3 main data exceeds frame capacity"));
    }

    Ok(Layer3PcmFrameStepSelection {
        step,
        payload_bit_len: main_data.bit_len,
        frame_capacity_bits: frame_capacity_bytes
            .checked_mul(8)
            .ok_or(Error::InvalidInput("MP3 frame capacity overflows"))?,
    })
}

pub(crate) fn evaluate_mpeg1_layer3_pcm_frame_perceptual_step_with_table_provider(
    header: FrameHeader,
    pcm: &AudioBuffer,
    start_frame: usize,
    step: f32,
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<Layer3PcmFrameStepSelection, Error> {
    let (_side_info, main_data) =
        pack_mpeg1_layer3_pcm_frame_perceptual_payloads_with_table_provider(
            header,
            pcm,
            start_frame,
            step,
            provider,
        )?;
    let frame_capacity_bytes = layer3_main_data_capacity_bytes(header)?;
    if main_data.bytes.len() > frame_capacity_bytes {
        return Err(Error::InvalidInput("MP3 main data exceeds frame capacity"));
    }

    Ok(Layer3PcmFrameStepSelection {
        step,
        payload_bit_len: main_data.bit_len,
        frame_capacity_bits: frame_capacity_bytes
            .checked_mul(8)
            .ok_or(Error::InvalidInput("MP3 frame capacity overflows"))?,
    })
}

/// Returns the Layer III main-data payload capacity for one frame.
pub fn layer3_main_data_capacity_bytes(header: FrameHeader) -> Result<usize, Error> {
    if header.layer != Layer::Layer3 {
        return Err(Error::UnsupportedFeature(
            "MP3 frame assembly requires Layer III",
        ));
    }
    let side_info_len = header
        .layer3_side_info_len()
        .ok_or(Error::UnsupportedFeature(
            "MP3 side info requires Layer III",
        ))?;
    let crc_len = if header.protection_absent { 0 } else { 2 };
    let fixed_len = 4_usize
        .checked_add(crc_len)
        .and_then(|len| len.checked_add(side_info_len))
        .ok_or(Error::InvalidInput("MP3 frame length overflow"))?;
    header
        .frame_len()
        .checked_sub(fixed_len)
        .ok_or(Error::InvalidInput("MP3 frame length overflow"))
}

/// Builds a Layer III header for capacity and frame-budget calculations.
///
/// `channels` accepts mono (`1`) or stereo (`2`). `crc_protected` follows the
/// user-facing meaning and is converted to the MPEG header's `protection_absent`
/// bit.
pub fn layer3_header_for_capacity(
    sample_rate: u32,
    channels: u16,
    bitrate_kbps: u16,
    padding: bool,
    crc_protected: bool,
) -> Result<FrameHeader, Error> {
    let version = match sample_rate {
        32_000 | 44_100 | 48_000 => MpegVersion::Mpeg1,
        16_000 | 22_050 | 24_000 => MpegVersion::Mpeg2,
        8_000 | 11_025 | 12_000 => MpegVersion::Mpeg25,
        _ => return Err(Error::UnsupportedFeature("MP3 Layer III sample rate")),
    };
    let channel_mode = match channels {
        1 => ChannelMode::SingleChannel,
        2 => ChannelMode::Stereo,
        _ => return Err(Error::UnsupportedFeature("MP3 Layer III channel count")),
    };
    let header = FrameHeader {
        version,
        layer: Layer::Layer3,
        protection_absent: !crc_protected,
        bitrate_kbps,
        sample_rate,
        padding,
        channel_mode,
    };
    header.to_bytes()?;
    Ok(header)
}

/// Returns the Layer III main-data payload capacity in bits for one frame.
pub fn layer3_main_data_capacity_bits(header: FrameHeader) -> Result<usize, Error> {
    layer3_main_data_capacity_bytes(header)?
        .checked_mul(8)
        .ok_or(Error::InvalidInput("MP3 frame capacity overflows"))
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Layer3PaddingSchedule {
    pub(crate) header: FrameHeader,
    pub(crate) slot_remainder: u64,
    pub(crate) sample_rate: u64,
    pub(crate) accumulator: u64,
}

impl Layer3PaddingSchedule {
    pub(crate) fn new(mut header: FrameHeader) -> Result<Self, Error> {
        if header.layer != Layer::Layer3 {
            return Err(Error::UnsupportedFeature(
                "MP3 padding schedule requires Layer III",
            ));
        }
        header.padding = false;
        let coefficient = if header.version == MpegVersion::Mpeg1 {
            144_u64
        } else {
            72_u64
        };
        let sample_rate = u64::from(header.sample_rate);
        let slots = coefficient
            .checked_mul(u64::from(header.bitrate_kbps))
            .and_then(|value| value.checked_mul(1000))
            .ok_or(Error::InvalidInput("MP3 frame length overflow"))?;
        Ok(Self {
            header,
            slot_remainder: slots % sample_rate,
            sample_rate,
            accumulator: 0,
        })
    }

    pub(crate) fn next_header(&mut self) -> FrameHeader {
        let mut header = self.header;
        self.accumulator += self.slot_remainder;
        if self.accumulator >= self.sample_rate {
            self.accumulator -= self.sample_rate;
            header.padding = true;
        }
        header
    }
}

pub fn assemble_layer3_frame(
    header: FrameHeader,
    side_info: &Layer3SideInfo,
    main_data: &[u8],
) -> Result<Vec<u8>, Error> {
    if header.layer != Layer::Layer3 {
        return Err(Error::UnsupportedFeature(
            "MP3 frame assembly requires Layer III",
        ));
    }

    let header_bytes = header.to_bytes()?;
    let side_info = side_info.pack(&header)?;
    let frame_len = header.frame_len();
    let main_data_capacity = layer3_main_data_capacity_bytes(header)?;
    if main_data.len() > main_data_capacity {
        return Err(Error::InvalidInput("MP3 main data exceeds frame capacity"));
    }

    let mut frame = Vec::with_capacity(frame_len);
    frame.extend_from_slice(&header_bytes);
    if !header.protection_absent {
        let mut crc_input = Vec::with_capacity(3 + side_info.len());
        crc_input.extend_from_slice(&header_bytes[1..]);
        crc_input.extend_from_slice(&side_info);
        frame.extend_from_slice(&crc16_mpeg_audio(&crc_input).to_be_bytes());
    }
    frame.extend_from_slice(&side_info);
    frame.extend_from_slice(main_data);
    frame.resize(frame_len, 0);
    Ok(frame)
}

/// Concatenates granule/channel payloads in Layer III main-data order.
pub fn pack_layer3_main_data_payloads(
    header: &FrameHeader,
    payloads: &[PackedBits],
) -> Result<PackedBits, Error> {
    if header.layer != Layer::Layer3 {
        return Err(Error::UnsupportedFeature(
            "MP3 main data requires Layer III",
        ));
    }

    let expected = header
        .layer3_granule_count()
        .checked_mul(header.channel_count())
        .ok_or(Error::InvalidInput("MP3 main data payload count overflow"))?;
    if payloads.len() != expected {
        return Err(Error::InvalidInput(
            "MP3 main data payload count does not match header",
        ));
    }

    concat_packed_bits(payloads)
}

/// Assembles one Layer III frame from granule/channel payloads.
pub fn assemble_layer3_frame_from_payloads(
    header: FrameHeader,
    side_info: &Layer3SideInfo,
    payloads: &[PackedBits],
) -> Result<Vec<u8>, Error> {
    let main_data = pack_layer3_main_data_payloads(&header, payloads)?;
    assemble_layer3_frame(header, side_info, &main_data.bytes)
}

/// Runs the Layer III long-block analysis window and MDCT for one subband.
pub fn mdct_long_block(samples: &[f32; 36]) -> Result<Vec<f32>, Error> {
    let window = sine_window(36)?;
    mdct(&apply_window(samples, &window)?)
}

/// Runs Layer III long-block analysis and scalar spectral quantization.
pub fn quantize_long_block(samples: &[f32; 36], step: f32) -> Result<Vec<i32>, Error> {
    quantize_spectrum(&mdct_long_block(samples)?, step, 8191)
}

/// Reads one channel sample at a (possibly negative) frame index.
///
/// Returns `0.0` for indices before the start of the buffer or past its end, as
/// required by the analysis filterbank, which slides a 512-sample window over
/// the input and zero-pads outside it.
pub(crate) fn channel_sample_or_zero(pcm: &AudioBuffer, channel: usize, frame: isize) -> f32 {
    if frame < 0 {
        return 0.0;
    }
    let channels = usize::from(pcm.channels);
    (frame as usize)
        .checked_mul(channels)
        .and_then(|base| base.checked_add(channel))
        .and_then(|index| pcm.samples.get(index))
        .copied()
        .unwrap_or(0.0)
}

/// Runs the 32-band polyphase analysis filterbank over 36 consecutive hops.
///
/// Returns the subband samples as `out[hop][subband]`, where hop `h` analyses
/// the 32-sample block ending at frame `start_frame + h * 32 + 31`.
pub(crate) fn analysis_subband_hops(
    pcm: &AudioBuffer,
    channel: usize,
    start_frame: usize,
) -> Result<[[f32; filterbank::SUBBANDS]; 36], Error> {
    if channel >= usize::from(pcm.channels) {
        return Err(Error::InvalidPcm("channel index out of range"));
    }

    let mut hops = [[0.0_f32; filterbank::SUBBANDS]; 36];
    let mut window = [0.0_f32; filterbank::WINDOW_LEN];
    for (hop, out) in hops.iter_mut().enumerate() {
        let newest = start_frame
            .checked_add(
                hop.checked_mul(32)
                    .and_then(|offset| offset.checked_add(31))
                    .ok_or(Error::InvalidInput("MP3 analysis hop start overflows"))?,
            )
            .ok_or(Error::InvalidInput("MP3 analysis hop start overflows"))?;
        let newest = isize::try_from(newest)
            .map_err(|_| Error::InvalidInput("MP3 analysis hop start overflows"))?;
        for (offset, slot) in window.iter_mut().enumerate() {
            *slot = channel_sample_or_zero(pcm, channel, newest - offset as isize);
        }
        *out = filterbank::analysis_hop(&window);
    }
    Ok(hops)
}

/// Builds a 36-sample approximation of one Layer III analysis subband.
///
/// This is a standards-shaped placeholder for the full 32-band polyphase
/// analysis filterbank. It separates PCM into 32 cosine-modulated bands before
/// the hybrid MDCT stage, which is closer to Layer III than directly MDCT'ing
/// adjacent PCM windows.
pub fn layer3_analysis_subband_block(
    pcm: &AudioBuffer,
    channel: usize,
    start_frame: usize,
    subband: usize,
) -> Result<[f32; 36], Error> {
    if subband >= 32 {
        return Err(Error::InvalidInput("MP3 subband index exceeds 31"));
    }

    let mut out = [0.0_f32; 36];
    for (slot, sample) in out.iter_mut().enumerate() {
        let slot_start = start_frame
            .checked_add(
                slot.checked_mul(32)
                    .ok_or(Error::InvalidInput("MP3 analysis slot start overflows"))?,
            )
            .ok_or(Error::InvalidInput("MP3 analysis slot start overflows"))?;
        let pcm_window = pcm.channel_block(channel, slot_start, 32)?;
        let mut value = 0.0_f32;
        for (tap, pcm_sample) in pcm_window.iter().enumerate() {
            let phase =
                core::f32::consts::PI / 32.0 * ((tap as f32) + 0.5) * ((subband as f32) + 0.5);
            value += *pcm_sample * phase.cos();
        }
        *sample = value * -0.25;
    }
    Ok(out)
}

/// Number of subband samples a long block contributes per granule.
pub(crate) const LONG_BLOCK_GRANULE_SAMPLES: usize = 18;

/// Applies Layer III odd-subband frequency inversion to one granule's samples.
///
/// The hybrid synthesis filterbank negates the odd-indexed time samples of every
/// odd subband; the encoder pre-applies the same inversion so the two cancel.
pub(crate) fn apply_frequency_inversion(
    subband: usize,
    samples: &mut [f32; LONG_BLOCK_GRANULE_SAMPLES],
) {
    if subband % 2 == 1 {
        for sample in samples.iter_mut().skip(1).step_by(2) {
            *sample = -*sample;
        }
    }
}

/// Collects one granule's 18 subband samples for `subband`, newest hop last,
/// with the odd-subband frequency inversion applied.
pub(crate) fn long_block_granule_samples(
    hops: &[[f32; filterbank::SUBBANDS]; 36],
    subband: usize,
) -> [f32; LONG_BLOCK_GRANULE_SAMPLES] {
    let mut samples = [0.0_f32; LONG_BLOCK_GRANULE_SAMPLES];
    for (slot, hop) in samples.iter_mut().zip(hops.iter()) {
        *slot = hop[subband];
    }
    apply_frequency_inversion(subband, &mut samples);
    samples
}

/// ISO/IEC 11172-3 alias-reduction coefficients `c[i]`.
pub(crate) const ALIAS_REDUCTION_C: [f32; 8] = [
    -0.6, -0.535, -0.33, -0.185, -0.095, -0.041, -0.0142, -0.0037,
];

/// Applies the encoder-side (forward) alias-reduction butterflies in place.
///
/// The decoder rotates spectral lines across each subband boundary to cancel
/// aliasing introduced by the polyphase filterbank; the encoder applies the
/// inverse rotation so the cascade is transparent. Operates on the 576-line
/// subband-major long-block spectrum.
pub(crate) fn apply_alias_reduction(spectrum: &mut [f32]) {
    for boundary in 0..(filterbank::SUBBANDS - 1) {
        let upper_base = boundary * LONG_BLOCK_GRANULE_SAMPLES + (LONG_BLOCK_GRANULE_SAMPLES - 1);
        let lower_base = (boundary + 1) * LONG_BLOCK_GRANULE_SAMPLES;
        for (i, &c) in ALIAS_REDUCTION_C.iter().enumerate() {
            let cs = 1.0 / (1.0 + c * c).sqrt();
            let ca = c / (1.0 + c * c).sqrt();
            let upper = upper_base - i;
            let lower = lower_base + i;
            let a = spectrum[upper];
            let b = spectrum[lower];
            // Inverse of the decoder rotation `(a*cs - b*ca, b*cs + a*ca)`.
            spectrum[upper] = a * cs + b * ca;
            spectrum[lower] = b * cs - a * ca;
        }
    }
}

/// Computes the 576 long-block MDCT spectral lines for one granule.
///
/// Each subband forms a 36-sample MDCT block from the previous granule's 18
/// subband samples followed by the current granule's 18, matching the 50%
/// overlap the decoder reconstructs with overlap-add. Encoder-side alias
/// reduction is then applied across subband boundaries.
pub fn layer3_long_block_spectrum(
    pcm: &AudioBuffer,
    channel: usize,
    granule_start: usize,
) -> Result<Vec<f32>, Error> {
    cached_spectrum(
        SPECTRUM_KIND_LONG_BLOCK,
        pcm,
        channel,
        granule_start,
        || layer3_long_block_spectrum_uncached(pcm, channel, granule_start),
    )
}

fn layer3_long_block_spectrum_uncached(
    pcm: &AudioBuffer,
    channel: usize,
    granule_start: usize,
) -> Result<Vec<f32>, Error> {
    let current = analysis_subband_hops(pcm, channel, granule_start)?;
    let previous = match granule_start.checked_sub(576) {
        Some(prev_start) => Some(analysis_subband_hops(pcm, channel, prev_start)?),
        None => None,
    };

    let mut spectrum = Vec::with_capacity(576);
    let mut block = [0.0_f32; 36];
    for subband in 0_usize..filterbank::SUBBANDS {
        let current_samples = long_block_granule_samples(&current, subband);
        let previous_samples = previous
            .as_ref()
            .map(|hops| long_block_granule_samples(hops, subband))
            .unwrap_or([0.0_f32; LONG_BLOCK_GRANULE_SAMPLES]);

        block[..LONG_BLOCK_GRANULE_SAMPLES].copy_from_slice(&previous_samples);
        block[LONG_BLOCK_GRANULE_SAMPLES..].copy_from_slice(&current_samples);
        spectrum.extend(mdct_long_block(&block)?);
    }
    apply_alias_reduction(&mut spectrum);
    Ok(spectrum)
}

/// Extracts one PCM channel and quantizes one Layer III long granule.
///
/// Mono uses the real polyphase + hybrid MDCT workbench. Stereo remains on the
/// older cosine-modulated subband scaffold because that is the path currently
/// accepted by the FFmpeg-backed readiness oracle; promoting stereo to the real
/// polyphase path is tracked separately.
pub fn quantize_pcm_long_block(
    pcm: &AudioBuffer,
    channel: usize,
    start_frame: usize,
    step: f32,
) -> Result<Vec<i32>, Error> {
    if pcm.channels == 2 {
        let mut quantized = Vec::with_capacity(576);
        for subband in 0..32 {
            let block = layer3_analysis_subband_block(pcm, channel, start_frame, subband)?;
            quantized.extend(quantize_long_block(&block, step)?);
        }
        return Ok(quantized);
    }

    let spectrum = layer3_long_block_spectrum(pcm, channel, start_frame)?;
    let inverted: Vec<f32> = spectrum.into_iter().map(|line| -line).collect();
    quantize_spectrum(&inverted, step, 8191)
}

pub(crate) fn layer3_perceptual_quantizer_spectrum(
    pcm: &AudioBuffer,
    channel: usize,
    start_frame: usize,
) -> Result<Vec<f32>, Error> {
    cached_spectrum(SPECTRUM_KIND_PERCEPTUAL, pcm, channel, start_frame, || {
        layer3_perceptual_quantizer_spectrum_uncached(pcm, channel, start_frame)
    })
}

fn layer3_perceptual_quantizer_spectrum_uncached(
    pcm: &AudioBuffer,
    channel: usize,
    start_frame: usize,
) -> Result<Vec<f32>, Error> {
    if pcm.channels == 2 {
        let mut spectrum = Vec::with_capacity(576);
        for subband in 0..32 {
            let block = layer3_analysis_subband_block(pcm, channel, start_frame, subband)?;
            spectrum.extend(mdct_long_block(&block)?);
        }
        return Ok(spectrum);
    }

    let spectrum = layer3_long_block_spectrum(pcm, channel, start_frame)?;
    Ok(spectrum.into_iter().map(|line| -line).collect())
}

/// Per-band scale-factor gain applied to a long-block line before rounding.
///
/// The decoder attenuates band `sfb` by `2^(-0.5·(1+scalefac_scale)·sf)`
/// (ISO/IEC 11172-3 §2.4.3.4). The chain's power-law quantizer raises the
/// magnitude to the 3/4 power, so the encoder must pre-amplify the pre-rounded
/// quantizer value by `2^(0.375·(1+scalefac_scale)·sf)` for the decoder's
/// requantization to reconstruct the line exactly.
pub(crate) fn long_block_scalefactor_quantizer_gain(scale_factor: u8, scalefac_scale: bool) -> f32 {
    let multiplier = if scalefac_scale { 1.0 } else { 0.5 };
    2.0_f32.powf(0.75 * multiplier * f32::from(scale_factor))
}

/// Quantizes a long-block spectrum (576 lines) with per-band scale-factor
/// noise shaping.
///
/// Each line is quantized with the ISO power law `is = nint(|xr|^0.75 / step)`
/// and pre-amplified by its band's scale-factor gain
/// ([`long_block_scalefactor_quantizer_gain`]) so the decoder's per-band
/// attenuation reconstructs `xr`. Lines in the residual highest band (no
/// transmitted scale factor) and any trailing lines are quantized flat. The
/// caller is responsible for any sign convention; this quantizes the spectrum
/// as supplied.
pub fn quantize_mpeg1_layer3_long_spectrum_with_scalefactors(
    spectrum: &[f32],
    step: f32,
    scale_factors: &[u8; MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT],
    scalefac_scale: bool,
    sample_rate: u32,
) -> Result<Vec<i32>, Error> {
    let magnitudes = layer3_long_spectrum_quantizer_magnitudes(spectrum)?;
    quantize_mpeg1_layer3_long_spectrum_with_scalefactors_and_magnitudes(
        spectrum,
        &magnitudes,
        step,
        scale_factors,
        scalefac_scale,
        sample_rate,
    )
}

/// Power-law magnitudes `|xr|^0.75` for one long granule.
///
/// These are independent of the quantizer step and scale factors, so the
/// noise-control loop can compute them once and reuse them across every pass and
/// candidate step instead of re-running `powf` per line per pass. Errors on a
/// non-finite line, matching the per-line check of the direct quantizer.
pub(crate) fn layer3_long_spectrum_quantizer_magnitudes(
    spectrum: &[f32],
) -> Result<Vec<f32>, Error> {
    let mut magnitudes = Vec::with_capacity(spectrum.len());
    for &coeff in spectrum {
        if !coeff.is_finite() {
            return Err(Error::InvalidInput("spectral coefficient must be finite"));
        }
        magnitudes.push(coeff.abs().powf(0.75));
    }
    Ok(magnitudes)
}

/// Quantizes a long-block spectrum from precomputed `|xr|^0.75` magnitudes.
///
/// `magnitudes[line]` must equal `spectrum[line].abs().powf(0.75)`; the spectrum
/// is still consulted for each line's sign. Bit-identical to
/// [`quantize_mpeg1_layer3_long_spectrum_with_scalefactors`].
pub(crate) fn quantize_mpeg1_layer3_long_spectrum_with_scalefactors_and_magnitudes(
    spectrum: &[f32],
    magnitudes: &[f32],
    step: f32,
    scale_factors: &[u8; MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT],
    scalefac_scale: bool,
    sample_rate: u32,
) -> Result<Vec<i32>, Error> {
    if !step.is_finite() || step <= 0.0 {
        return Err(Error::InvalidInput("quantization step must be positive"));
    }
    let index = mpeg1_layer3_long_scalefactor_band_index(sample_rate)?;

    let mut out = Vec::with_capacity(spectrum.len());
    for (line, &coeff) in spectrum.iter().enumerate() {
        let magnitude_pow = match magnitudes.get(line) {
            Some(&value) => value,
            None => return Err(Error::InvalidInput("quantizer magnitude line missing")),
        };
        // Locate the transmitted scale-factor band for this line; the residual
        // band beyond index[21] (and any padding past 576) shapes flat.
        let gain = match index[1..=MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT]
            .iter()
            .position(|&boundary| line < usize::from(boundary))
        {
            Some(band) => {
                long_block_scalefactor_quantizer_gain(scale_factors[band], scalefac_scale)
            }
            None => 1.0,
        };

        let magnitude = (magnitude_pow / step * gain).round();
        if magnitude > 8191.0 {
            return Err(Error::InvalidInput(
                "quantized spectral coefficient exceeds bound",
            ));
        }
        let quantized = magnitude as i32;
        out.push(if coeff.is_sign_negative() {
            -quantized
        } else {
            quantized
        });
    }
    Ok(out)
}

/// Selects psychoacoustic MPEG-1 Layer III long-block scale factors from PCM.
///
/// This low-level helper builds the same sign-inverted hybrid MDCT spectrum
/// that mono [`quantize_pcm_long_block`] uses, analyzes a zero-padded PCM span
/// with the clean-room psychoacoustic model, and returns scale factors suitable
/// for [`quantize_mpeg1_layer3_long_spectrum_with_scalefactors`]. Production
/// encode still uses calibrated global gain until the matching bit-budget loop
/// is validated.
pub fn select_mpeg1_layer3_psychoacoustic_long_scale_factors(
    pcm: &AudioBuffer,
    channel: usize,
    start_frame: usize,
    step: f32,
    scalefac_scale: bool,
    fft_len: usize,
) -> Result<[u8; MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT], Error> {
    if fft_len == 0 {
        return Err(Error::InvalidInput(
            "MP3 psychoacoustic FFT length must be non-zero",
        ));
    }

    let spectrum = layer3_long_block_spectrum(pcm, channel, start_frame)?;
    let inverted: Vec<f32> = spectrum.into_iter().map(|line| -line).collect();
    let pcm_window: Vec<f64> = pcm
        .channel_block(channel, start_frame, fft_len)?
        .into_iter()
        .map(f64::from)
        .collect();

    psychoacoustic::perceptual_long_block_scalefactors(
        &inverted,
        &pcm_window,
        step,
        scalefac_scale,
        pcm.sample_rate,
    )
}

/// Computes the `global_gain` that inverts a given quantizer `step`.
///
/// The decoder requantizes a long-block line as
/// `sign · |is|^(4/3) · 2^((global_gain − 210)/4)` (ISO/IEC 11172-3 §2.4.3.4,
/// scale factors and preflag zero), while the encoder forms
/// `is = round(|coeff|^(3/4) / step)`. Substituting the latter into the former
/// reconstructs `coeff` exactly when `2^((global_gain − 210)/4) = step^(4/3)`,
/// i.e. `global_gain = 210 + (16/3)·log2(step)`. The result is rounded to the
/// nearest 8-bit value and clamped to the syntax range `[0, 255]`; degenerate
/// steps fall back to the ISO reference gain of 210.
#[must_use]
pub fn mpeg1_layer3_global_gain_for_step(step: f32) -> u8 {
    if !step.is_finite() || step <= 0.0 {
        return 210;
    }
    let raw = (210.0 + (16.0 / 3.0) * step.log2()).round();
    raw.clamp(0.0, 255.0) as u8
}
