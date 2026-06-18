#![deny(unsafe_code)]
#![warn(clippy::all)]

use sc_core::{
    apply_window, concat_packed_bits, lookup_huffman_code, mdct, pack_huffman_codes,
    pack_huffman_codes_with_len, pack_huffman_symbols_with_len, quantize_spectrum, sine_window,
    AudioBuffer, BitWriter as CoreBitWriter, Decoder, Encoder, Error, HuffmanCode, HuffmanEntry,
    PackedBits,
};

mod filterbank;

pub const MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT: usize = 21;
pub const MPEG1_LAYER3_PCM_STEP_CANDIDATES: &[f32] = &[
    0.0005,
    0.001,
    0.002,
    0.005,
    0.01,
    0.02,
    0.05,
    0.1,
    0.2,
    0.5,
    1.0,
    2.0,
    5.0,
    10.0,
    20.0,
    50.0,
    100.0,
    200.0,
    500.0,
    1_000.0,
    f32::MAX,
];

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Layer3PcmFrameStepSelection {
    pub step: f32,
    pub payload_bit_len: usize,
    pub frame_capacity_bits: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FrameHeader {
    pub version: MpegVersion,
    pub layer: Layer,
    pub protection_absent: bool,
    pub bitrate_kbps: u16,
    pub sample_rate: u32,
    pub padding: bool,
    pub channel_mode: ChannelMode,
}

impl FrameHeader {
    pub fn parse(input: &[u8]) -> Result<Self, Error> {
        if input.len() < 4 {
            return Err(Error::InvalidInput("truncated MP3 frame header"));
        }
        if input[0] != 0xff || (input[1] & 0xe0) != 0xe0 {
            return Err(Error::InvalidInput("missing MP3 frame sync"));
        }

        let version = match (input[1] >> 3) & 0x03 {
            0b00 => MpegVersion::Mpeg25,
            0b10 => MpegVersion::Mpeg2,
            0b11 => MpegVersion::Mpeg1,
            _ => return Err(Error::InvalidInput("reserved MPEG audio version")),
        };
        let layer = match (input[1] >> 1) & 0x03 {
            0b01 => Layer::Layer3,
            0b10 => Layer::Layer2,
            0b11 => Layer::Layer1,
            _ => return Err(Error::InvalidInput("reserved MPEG audio layer")),
        };
        let protection_absent = input[1] & 0x01 != 0;
        let bitrate_index = (input[2] >> 4) & 0x0f;
        let sample_rate_index = (input[2] >> 2) & 0x03;
        let padding = input[2] & 0x02 != 0;
        let channel_mode = match (input[3] >> 6) & 0x03 {
            0b00 => ChannelMode::Stereo,
            0b01 => ChannelMode::JointStereo,
            0b10 => ChannelMode::DualChannel,
            _ => ChannelMode::SingleChannel,
        };

        Ok(Self {
            version,
            layer,
            protection_absent,
            bitrate_kbps: bitrate_kbps(version, layer, bitrate_index)?,
            sample_rate: sample_rate(version, sample_rate_index)?,
            padding,
            channel_mode,
        })
    }

    #[must_use]
    pub fn frame_len(&self) -> usize {
        let padding = usize::from(self.padding);
        match self.layer {
            Layer::Layer1 => {
                ((12 * usize::from(self.bitrate_kbps) * 1000 / self.sample_rate as usize) + padding)
                    * 4
            }
            Layer::Layer2 => {
                144 * usize::from(self.bitrate_kbps) * 1000 / self.sample_rate as usize + padding
            }
            Layer::Layer3 => {
                let coefficient = if self.version == MpegVersion::Mpeg1 {
                    144
                } else {
                    72
                };
                coefficient * usize::from(self.bitrate_kbps) * 1000 / self.sample_rate as usize
                    + padding
            }
        }
    }

    #[must_use]
    pub fn samples_per_frame(&self) -> u16 {
        match (self.version, self.layer) {
            (_, Layer::Layer1) => 384,
            (_, Layer::Layer2) | (MpegVersion::Mpeg1, Layer::Layer3) => 1152,
            (_, Layer::Layer3) => 576,
        }
    }

    #[must_use]
    pub fn channel_count(&self) -> usize {
        if self.channel_mode == ChannelMode::SingleChannel {
            1
        } else {
            2
        }
    }

    #[must_use]
    pub fn layer3_granule_count(&self) -> usize {
        if self.version == MpegVersion::Mpeg1 {
            2
        } else {
            1
        }
    }

    #[must_use]
    pub fn layer3_side_info_len(&self) -> Option<usize> {
        if self.layer != Layer::Layer3 {
            return None;
        }
        Some(match (self.version, self.channel_mode) {
            (MpegVersion::Mpeg1, ChannelMode::SingleChannel) => 17,
            (MpegVersion::Mpeg1, _) => 32,
            (_, ChannelMode::SingleChannel) => 9,
            (_, _) => 17,
        })
    }

    pub fn to_bytes(self) -> Result<[u8; 4], Error> {
        let version_bits = match self.version {
            MpegVersion::Mpeg25 => 0b00,
            MpegVersion::Mpeg2 => 0b10,
            MpegVersion::Mpeg1 => 0b11,
        };
        let layer_bits = match self.layer {
            Layer::Layer1 => 0b11,
            Layer::Layer2 => 0b10,
            Layer::Layer3 => 0b01,
        };
        let bitrate_index = bitrate_index(self.version, self.layer, self.bitrate_kbps)?;
        let sample_rate_index = sample_rate_index(self.version, self.sample_rate)?;
        let channel_mode_bits = match self.channel_mode {
            ChannelMode::Stereo => 0b00,
            ChannelMode::JointStereo => 0b01,
            ChannelMode::DualChannel => 0b10,
            ChannelMode::SingleChannel => 0b11,
        };

        Ok([
            0xff,
            0xe0 | (version_bits << 3) | (layer_bits << 1) | u8::from(self.protection_absent),
            (bitrate_index << 4) | (sample_rate_index << 2) | (u8::from(self.padding) << 1),
            channel_mode_bits << 6,
        ])
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MpegVersion {
    Mpeg1,
    Mpeg2,
    Mpeg25,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Layer {
    Layer1,
    Layer2,
    Layer3,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChannelMode {
    Stereo,
    JointStereo,
    DualChannel,
    SingleChannel,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Layer3SideInfo {
    pub main_data_begin: u16,
    pub private_bits: u8,
    pub scfsi: [[bool; 4]; 2],
    pub granules: [[Layer3GranuleChannelInfo; 2]; 2],
}

impl Layer3SideInfo {
    #[must_use]
    pub fn silent(header: &FrameHeader) -> Self {
        let granule = Layer3GranuleChannelInfo::default();
        let mut info = Self {
            main_data_begin: 0,
            private_bits: 0,
            scfsi: [[false; 4]; 2],
            granules: [[granule; 2]; 2],
        };
        for granule_index in 0..header.layer3_granule_count() {
            for channel in 0..header.channel_count() {
                info.granules[granule_index][channel].global_gain = 210;
            }
        }
        info
    }

    pub fn pack(&self, header: &FrameHeader) -> Result<Vec<u8>, Error> {
        if header.layer != Layer::Layer3 {
            return Err(Error::UnsupportedFeature(
                "MP3 side info requires Layer III",
            ));
        }

        let channels = header.channel_count();
        let granules = header.layer3_granule_count();
        let mut writer = BitWriter::new();
        if header.version == MpegVersion::Mpeg1 {
            writer.write_bits(u32::from(self.main_data_begin), 9)?;
            writer.write_bits(
                u32::from(self.private_bits),
                if channels == 1 { 5 } else { 3 },
            )?;
            for channel in 0..channels {
                for band in 0..4 {
                    writer.write_bits(u32::from(self.scfsi[channel][band]), 1)?;
                }
            }
        } else {
            writer.write_bits(u32::from(self.main_data_begin), 8)?;
            writer.write_bits(
                u32::from(self.private_bits),
                if channels == 1 { 1 } else { 2 },
            )?;
        }

        for granule in 0..granules {
            for channel in 0..channels {
                self.granules[granule][channel].pack(&mut writer, header.version)?;
            }
        }

        let packed = writer.finish_byte_aligned();
        let expected_len = header
            .layer3_side_info_len()
            .ok_or(Error::UnsupportedFeature(
                "MP3 side info requires Layer III",
            ))?;
        if packed.len() != expected_len {
            return Err(Error::InvalidInput("MP3 side info length mismatch"));
        }
        Ok(packed)
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Layer3GranuleChannelInfo {
    pub part2_3_length: u16,
    pub big_values: u16,
    pub global_gain: u8,
    pub scalefac_compress: u16,
    pub window_switching: Option<Layer3WindowSwitching>,
    pub table_select: [u8; 3],
    pub region0_count: u8,
    pub region1_count: u8,
    pub preflag: bool,
    pub scalefac_scale: bool,
    pub count1table_select: bool,
}

impl Layer3GranuleChannelInfo {
    fn pack(&self, writer: &mut BitWriter, version: MpegVersion) -> Result<(), Error> {
        writer.write_bits(u32::from(self.part2_3_length), 12)?;
        writer.write_bits(u32::from(self.big_values), 9)?;
        writer.write_bits(u32::from(self.global_gain), 8)?;
        writer.write_bits(
            u32::from(self.scalefac_compress),
            if version == MpegVersion::Mpeg1 { 4 } else { 9 },
        )?;

        match self.window_switching {
            Some(window) => {
                writer.write_bits(1, 1)?;
                window.pack(writer)?;
            }
            None => {
                writer.write_bits(0, 1)?;
                for table in self.table_select {
                    writer.write_bits(u32::from(table), 5)?;
                }
                writer.write_bits(u32::from(self.region0_count), 4)?;
                writer.write_bits(u32::from(self.region1_count), 3)?;
            }
        }

        if version == MpegVersion::Mpeg1 {
            writer.write_bits(u32::from(self.preflag), 1)?;
        }
        writer.write_bits(u32::from(self.scalefac_scale), 1)?;
        writer.write_bits(u32::from(self.count1table_select), 1)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Layer3WindowSwitching {
    pub block_type: u8,
    pub mixed_block_flag: bool,
    pub table_select: [u8; 2],
    pub subblock_gain: [u8; 3],
}

impl Layer3WindowSwitching {
    fn pack(&self, writer: &mut BitWriter) -> Result<(), Error> {
        writer.write_bits(u32::from(self.block_type), 2)?;
        writer.write_bits(u32::from(self.mixed_block_flag), 1)?;
        for table in self.table_select {
            writer.write_bits(u32::from(table), 5)?;
        }
        for gain in self.subblock_gain {
            writer.write_bits(u32::from(gain), 3)?;
        }
        Ok(())
    }
}

#[derive(Default)]
pub struct Mp3Decoder;

impl Mp3Decoder {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Decoder for Mp3Decoder {
    fn decode(&mut self, input: &[u8]) -> Result<AudioBuffer, Error> {
        decode(input)
    }

    fn decode_stream(&mut self, chunk: &[u8]) -> Result<Option<AudioBuffer>, Error> {
        decode(chunk).map(Some)
    }
}

#[derive(Default)]
pub struct Mp3Encoder;

impl Mp3Encoder {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Encoder for Mp3Encoder {
    fn encode(&mut self, pcm: &AudioBuffer) -> Result<Vec<u8>, Error> {
        encode(pcm)
    }
}

pub fn decode(input: &[u8]) -> Result<AudioBuffer, Error> {
    decode_silent_layer3(input)
}

/// Encodes mono/stereo PCM as MPEG-1 Layer III frames.
///
/// Silent input routes through the compact zero-spectral frame scaffold.
/// Non-silent input currently uses the same experimental long-block scaffold
/// with an intentionally coarse quantizer, so production-quality psychoacoustic
/// modeling, standard Huffman tables, bit reservoir use, and VBR are still
/// incomplete.
pub fn encode(pcm: &AudioBuffer) -> Result<Vec<u8>, Error> {
    if pcm.channels != 1 && pcm.channels != 2 {
        return Err(Error::UnsupportedFeature(
            "MP3 encode currently supports mono/stereo only",
        ));
    }

    if pcm.samples.iter().any(|sample| *sample != 0.0) {
        encode_mpeg1_layer3_pcm_frames_with_auto_step_and_table_provider(
            pcm,
            MPEG1_LAYER3_PCM_STEP_CANDIDATES,
            mpeg1_layer3_standard_table_provider(),
        )
    } else {
        encode_mpeg1_layer3_pcm_frames_with_selected_scale_factors_and_table_provider(
            pcm,
            1.0,
            Layer3EntropyTableProvider::default(),
        )
    }
}

/// Encodes PCM through the experimental MPEG-1 Layer III long-block frame scaffold.
pub fn encode_mpeg1_layer3_pcm_frames_with_selected_scale_factors(
    pcm: &AudioBuffer,
    step: f32,
    tables: Layer3EntropyTables<'_>,
) -> Result<Vec<u8>, Error> {
    let header = mpeg1_layer3_header_for_pcm(pcm)?;
    encode_mpeg1_layer3_pcm_frames_with_header_and_selected_scale_factors(header, pcm, step, tables)
}

/// Encodes PCM through the experimental MPEG-1 Layer III frame scaffold using provider lookup.
pub fn encode_mpeg1_layer3_pcm_frames_with_selected_scale_factors_and_table_provider(
    pcm: &AudioBuffer,
    step: f32,
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<Vec<u8>, Error> {
    let header = mpeg1_layer3_header_for_pcm(pcm)?;
    encode_mpeg1_layer3_pcm_frames_with_header_and_selected_scale_factors_and_table_provider(
        header, pcm, step, provider,
    )
}

/// Encodes PCM through the frame scaffold, selecting one quantizer step per frame.
pub fn encode_mpeg1_layer3_pcm_frames_with_auto_step_and_table_provider(
    pcm: &AudioBuffer,
    candidates: &[f32],
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<Vec<u8>, Error> {
    let header = mpeg1_layer3_header_for_pcm(pcm)?;
    encode_mpeg1_layer3_pcm_frames_with_header_cbr_padding_and_table_provider(
        header, pcm, candidates, provider,
    )
}

/// Encodes PCM through the frame scaffold, selecting each frame within an
/// explicit Layer III payload bit budget.
pub fn encode_mpeg1_layer3_pcm_frames_with_max_payload_bits_and_table_provider(
    pcm: &AudioBuffer,
    candidates: &[f32],
    max_payload_bit_len: usize,
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<Vec<u8>, Error> {
    let header = mpeg1_layer3_header_for_pcm(pcm)?;
    encode_mpeg1_layer3_pcm_frames_with_header_and_max_payload_bits_and_table_provider(
        header,
        pcm,
        candidates,
        max_payload_bit_len,
        provider,
    )
}

/// Encodes PCM through the frame scaffold using a caller-selected Layer III bitrate.
///
/// This builds the MPEG header and derives each frame's main-data capacity from
/// that header, so callers can drive the existing per-frame step search from a
/// bitrate without duplicating header/capacity calculations.
pub fn encode_mpeg1_layer3_pcm_frames_with_bitrate_and_table_provider(
    pcm: &AudioBuffer,
    candidates: &[f32],
    bitrate_kbps: u16,
    padding: bool,
    crc_protected: bool,
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<Vec<u8>, Error> {
    let header = layer3_header_for_capacity(
        pcm.sample_rate,
        pcm.channels,
        bitrate_kbps,
        padding,
        crc_protected,
    )?;
    encode_mpeg1_layer3_pcm_frames_with_header_and_auto_step_and_table_provider(
        header, pcm, candidates, provider,
    )
}

/// Encodes PCM using a caller-selected Layer III bitrate and CBR padding schedule.
///
/// MPEG-1 Layer III CBR at rates such as 44.1kHz needs some frames to carry one
/// padding slot so the average frame length reaches the requested bitrate. This
/// helper derives each frame header from an integer slot accumulator instead of
/// forcing callers to precompute per-frame padding.
pub fn encode_mpeg1_layer3_pcm_frames_with_cbr_bitrate_and_table_provider(
    pcm: &AudioBuffer,
    candidates: &[f32],
    bitrate_kbps: u16,
    crc_protected: bool,
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<Vec<u8>, Error> {
    let base_header = layer3_header_for_capacity(
        pcm.sample_rate,
        pcm.channels,
        bitrate_kbps,
        false,
        crc_protected,
    )?;
    encode_mpeg1_layer3_pcm_frames_with_header_cbr_padding_and_table_provider(
        base_header,
        pcm,
        candidates,
        provider,
    )
}

/// Encodes PCM with an explicit MPEG-1 Layer III header through the frame scaffold.
pub fn encode_mpeg1_layer3_pcm_frames_with_header_and_selected_scale_factors(
    header: FrameHeader,
    pcm: &AudioBuffer,
    step: f32,
    tables: Layer3EntropyTables<'_>,
) -> Result<Vec<u8>, Error> {
    let frame_count = layer3_frame_count(header, pcm)?;
    let mut out = Vec::with_capacity(header.frame_len() * frame_count);
    for frame_index in 0..frame_count {
        let start_frame = frame_index
            .checked_mul(usize::from(header.samples_per_frame()))
            .ok_or(Error::InvalidInput("MP3 frame start overflows"))?;
        out.extend_from_slice(
            &assemble_mpeg1_layer3_pcm_frame_with_selected_scale_factors(
                header,
                pcm,
                start_frame,
                step,
                tables,
            )?,
        );
    }
    Ok(out)
}

/// Encodes PCM with an explicit MPEG-1 Layer III header and per-frame step search.
pub fn encode_mpeg1_layer3_pcm_frames_with_header_and_auto_step_and_table_provider(
    header: FrameHeader,
    pcm: &AudioBuffer,
    candidates: &[f32],
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<Vec<u8>, Error> {
    let frame_count = layer3_frame_count(header, pcm)?;
    let mut out = Vec::with_capacity(header.frame_len() * frame_count);
    for frame_index in 0..frame_count {
        let start_frame = frame_index
            .checked_mul(usize::from(header.samples_per_frame()))
            .ok_or(Error::InvalidInput("MP3 frame start overflows"))?;
        let step = select_mpeg1_layer3_pcm_frame_step_with_table_provider(
            header,
            pcm,
            start_frame,
            candidates,
            provider,
        )?;
        out.extend_from_slice(
            &assemble_mpeg1_layer3_pcm_frame_with_selected_scale_factors_and_table_provider(
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

/// Encodes PCM with an explicit MPEG-1 Layer III header and per-frame step
/// search constrained by a caller-provided payload bit budget.
pub fn encode_mpeg1_layer3_pcm_frames_with_header_and_max_payload_bits_and_table_provider(
    header: FrameHeader,
    pcm: &AudioBuffer,
    candidates: &[f32],
    max_payload_bit_len: usize,
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<Vec<u8>, Error> {
    let frame_count = layer3_frame_count(header, pcm)?;
    let mut out = Vec::with_capacity(header.frame_len() * frame_count);
    for frame_index in 0..frame_count {
        let start_frame = frame_index
            .checked_mul(usize::from(header.samples_per_frame()))
            .ok_or(Error::InvalidInput("MP3 frame start overflows"))?;
        let step = select_mpeg1_layer3_pcm_frame_step_with_max_payload_bits_and_table_provider(
            header,
            pcm,
            start_frame,
            candidates,
            max_payload_bit_len,
            provider,
        )?;
        out.extend_from_slice(
            &assemble_mpeg1_layer3_pcm_frame_with_selected_scale_factors_and_table_provider(
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

/// Encodes PCM with per-frame CBR padding derived from the header bitrate.
pub fn encode_mpeg1_layer3_pcm_frames_with_header_cbr_padding_and_table_provider(
    header: FrameHeader,
    pcm: &AudioBuffer,
    candidates: &[f32],
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<Vec<u8>, Error> {
    let frame_count = layer3_frame_count(header, pcm)?;
    let mut padding = Layer3PaddingSchedule::new(header)?;
    let mut out = Vec::with_capacity(header.frame_len() * frame_count + frame_count);
    for frame_index in 0..frame_count {
        let start_frame = frame_index
            .checked_mul(usize::from(header.samples_per_frame()))
            .ok_or(Error::InvalidInput("MP3 frame start overflows"))?;
        let frame_header = padding.next_header();
        let step = select_mpeg1_layer3_pcm_frame_step_with_table_provider(
            frame_header,
            pcm,
            start_frame,
            candidates,
            provider,
        )?;
        out.extend_from_slice(
            &assemble_mpeg1_layer3_pcm_frame_with_selected_scale_factors_and_table_provider(
                frame_header,
                pcm,
                start_frame,
                step,
                provider,
            )?,
        );
    }
    Ok(out)
}

/// MPEG-1 main-data backward pointer width: `main_data_begin` is 9 bits.
const MAX_MAIN_DATA_BEGIN: usize = 511;

/// One frame's reservoir-aware packing result, retained for the layout pass.
struct Layer3ReservoirFrame {
    header: FrameHeader,
    side_info: Layer3SideInfo,
    payload: Vec<u8>,
    capacity: usize,
    main_data_begin: usize,
}

/// Packs one Layer III frame at the finest quantizer step whose byte-padded
/// payload fits a main-data byte budget (frame capacity plus borrowed reservoir).
///
/// Unlike the single-frame step search, the budget may exceed one frame's own
/// capacity, so the per-step capacity guard is replaced by the supplied budget.
fn pack_mpeg1_layer3_reservoir_frame_with_table_provider(
    header: FrameHeader,
    pcm: &AudioBuffer,
    start_frame: usize,
    candidates: &[f32],
    budget_bytes: usize,
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<(Layer3SideInfo, PackedBits), Error> {
    if candidates.is_empty() {
        return Err(Error::InvalidInput(
            "MP3 quantizer step candidate list is empty",
        ));
    }
    let mut best: Option<(f32, Layer3SideInfo, PackedBits)> = None;
    for &step in candidates {
        if !step.is_finite() || step <= 0.0 {
            return Err(Error::InvalidInput(
                "MP3 quantizer step must be positive and finite",
            ));
        }
        let Ok((side_info, main_data)) = pack_mpeg1_layer3_pcm_frame_payloads_with_table_provider(
            header,
            pcm,
            start_frame,
            step,
            provider,
        ) else {
            continue;
        };
        if main_data.bytes.len() > budget_bytes {
            continue;
        }
        // Prefer the smallest fitting step (finest quantization, best quality).
        best = match best {
            Some((best_step, best_side_info, best_main_data)) if step >= best_step => {
                Some((best_step, best_side_info, best_main_data))
            }
            _ => Some((step, side_info, main_data)),
        };
    }
    best.map(|(_, side_info, main_data)| (side_info, main_data))
        .ok_or(Error::UnsupportedFeature("MP3 reservoir step search"))
}

/// Encodes PCM as constant-bitrate MPEG-1 Layer III using a bit reservoir.
///
/// A frame whose granules need more bits than its own main-data slot provides
/// borrows from the unused tail of earlier frames through `main_data_begin`, the
/// spec's backward byte pointer into the shared main-data stream (ISO/IEC
/// 11172-3 §2.4.1.7). Frames that quantize cheaply leave surplus slot bytes that
/// later, busier frames consume, so the average bitrate stays constant while
/// per-frame quality is no longer hard-capped at one frame's capacity.
pub fn encode_mpeg1_layer3_pcm_frames_with_reservoir_and_table_provider(
    pcm: &AudioBuffer,
    candidates: &[f32],
    bitrate_kbps: u16,
    crc_protected: bool,
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<Vec<u8>, Error> {
    let base_header = layer3_header_for_capacity(
        pcm.sample_rate,
        pcm.channels,
        bitrate_kbps,
        false,
        crc_protected,
    )?;
    let frame_count = layer3_frame_count(base_header, pcm)?;
    let mut padding = Layer3PaddingSchedule::new(base_header)?;

    // Pass 1: choose each frame's step, pack its payload, and record how far the
    // running reservoir lets its main data begin before its own slot.
    let mut frames: Vec<Layer3ReservoirFrame> = Vec::with_capacity(frame_count);
    let mut reservoir = 0_usize;
    for frame_index in 0..frame_count {
        let start_frame = frame_index
            .checked_mul(usize::from(base_header.samples_per_frame()))
            .ok_or(Error::InvalidInput("MP3 frame start overflows"))?;
        let frame_header = padding.next_header();
        let capacity = layer3_main_data_capacity_bytes(frame_header)?;
        let main_data_begin = reservoir.min(MAX_MAIN_DATA_BEGIN);
        let budget_bytes = capacity
            .checked_add(main_data_begin)
            .ok_or(Error::InvalidInput("MP3 reservoir budget overflows"))?;
        let (mut side_info, main_data) = pack_mpeg1_layer3_reservoir_frame_with_table_provider(
            frame_header,
            pcm,
            start_frame,
            candidates,
            budget_bytes,
            provider,
        )?;
        side_info.main_data_begin = u16::try_from(main_data_begin)
            .map_err(|_| Error::InvalidInput("MP3 main_data_begin exceeds field width"))?;
        let payload = main_data.bytes;
        // Surplus beyond the 511-byte pointer range can never be referenced, so
        // it is dropped here (those slot bytes stay zero padding in the file).
        reservoir = main_data_begin
            .checked_add(capacity)
            .ok_or(Error::InvalidInput("MP3 reservoir overflows"))?
            .checked_sub(payload.len())
            .ok_or(Error::InvalidInput("MP3 reservoir underflows"))?;
        frames.push(Layer3ReservoirFrame {
            header: frame_header,
            side_info,
            payload,
            capacity,
            main_data_begin,
        });
    }

    // Pass 2: lay each payload into the shared main-data stream at the byte its
    // `main_data_begin` resolves to, then slice the stream back into frame slots.
    let mut total_slots = 0_usize;
    for frame in &frames {
        total_slots = total_slots
            .checked_add(frame.capacity)
            .ok_or(Error::InvalidInput("MP3 total main-data size overflows"))?;
    }
    let mut stream = vec![0_u8; total_slots];
    let mut slot_start = 0_usize;
    for frame in &frames {
        let payload_start =
            slot_start
                .checked_sub(frame.main_data_begin)
                .ok_or(Error::InvalidInput(
                    "MP3 main_data_begin precedes stream start",
                ))?;
        let payload_end = payload_start
            .checked_add(frame.payload.len())
            .filter(|end| *end <= stream.len())
            .ok_or(Error::InvalidInput(
                "MP3 payload overflows main-data stream",
            ))?;
        stream[payload_start..payload_end].copy_from_slice(&frame.payload);
        slot_start = slot_start
            .checked_add(frame.capacity)
            .ok_or(Error::InvalidInput("MP3 main-data stream overflows"))?;
    }

    let mut out = Vec::with_capacity(total_slots + frames.len() * 64);
    let mut slot_start = 0_usize;
    for frame in &frames {
        let slot = &stream[slot_start..slot_start + frame.capacity];
        out.extend_from_slice(&assemble_layer3_frame(
            frame.header,
            &frame.side_info,
            slot,
        )?);
        slot_start += frame.capacity;
    }
    Ok(out)
}

/// Encodes PCM with an explicit MPEG-1 Layer III header using provider lookup.
pub fn encode_mpeg1_layer3_pcm_frames_with_header_and_selected_scale_factors_and_table_provider(
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
            &assemble_mpeg1_layer3_pcm_frame_with_selected_scale_factors_and_table_provider(
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

/// Selects the finest quantizer step that can be packed into one Layer III frame.
pub fn select_mpeg1_layer3_pcm_frame_step_with_table_provider(
    header: FrameHeader,
    pcm: &AudioBuffer,
    start_frame: usize,
    candidates: &[f32],
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<f32, Error> {
    Ok(
        select_mpeg1_layer3_pcm_frame_step_details_with_table_provider(
            header,
            pcm,
            start_frame,
            candidates,
            provider,
        )?
        .step,
    )
}

/// Selects the finest quantizer step within a caller-provided Layer III payload budget.
pub fn select_mpeg1_layer3_pcm_frame_step_with_max_payload_bits_and_table_provider(
    header: FrameHeader,
    pcm: &AudioBuffer,
    start_frame: usize,
    candidates: &[f32],
    max_payload_bit_len: usize,
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<f32, Error> {
    Ok(
        select_mpeg1_layer3_pcm_frame_step_details_with_max_payload_bits_and_table_provider(
            header,
            pcm,
            start_frame,
            candidates,
            max_payload_bit_len,
            provider,
        )?
        .step,
    )
}

/// Selects the finest quantizer step and reports the resulting frame payload cost.
pub fn select_mpeg1_layer3_pcm_frame_step_details_with_table_provider(
    header: FrameHeader,
    pcm: &AudioBuffer,
    start_frame: usize,
    candidates: &[f32],
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<Layer3PcmFrameStepSelection, Error> {
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
            selected = match selected {
                Some(previous)
                    if selection.step > previous.step
                        || (selection.step == previous.step
                            && selection.payload_bit_len <= previous.payload_bit_len) =>
                {
                    Some(previous)
                }
                _ => Some(selection),
            };
        }
    }
    selected.ok_or(Error::UnsupportedFeature("MP3 quantizer step search"))
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

fn limit_mpeg1_layer3_pcm_frame_step_selection(
    mut selection: Layer3PcmFrameStepSelection,
    max_payload_bit_len: usize,
) -> Option<Layer3PcmFrameStepSelection> {
    if selection.payload_bit_len > max_payload_bit_len {
        return None;
    }
    selection.frame_capacity_bits = max_payload_bit_len;
    Some(selection)
}

fn select_better_mpeg1_layer3_pcm_frame_step(
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

fn evaluate_mpeg1_layer3_pcm_frame_step_with_table_provider(
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
struct Layer3PaddingSchedule {
    header: FrameHeader,
    slot_remainder: u64,
    sample_rate: u64,
    accumulator: u64,
}

impl Layer3PaddingSchedule {
    fn new(mut header: FrameHeader) -> Result<Self, Error> {
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

    fn next_header(&mut self) -> FrameHeader {
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
fn channel_sample_or_zero(pcm: &AudioBuffer, channel: usize, frame: isize) -> f32 {
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
fn analysis_subband_hops(
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
const LONG_BLOCK_GRANULE_SAMPLES: usize = 18;

/// Applies Layer III odd-subband frequency inversion to one granule's samples.
///
/// The hybrid synthesis filterbank negates the odd-indexed time samples of every
/// odd subband; the encoder pre-applies the same inversion so the two cancel.
fn apply_frequency_inversion(subband: usize, samples: &mut [f32; LONG_BLOCK_GRANULE_SAMPLES]) {
    if subband % 2 == 1 {
        for sample in samples.iter_mut().skip(1).step_by(2) {
            *sample = -*sample;
        }
    }
}

/// Collects one granule's 18 subband samples for `subband`, newest hop last,
/// with the odd-subband frequency inversion applied.
fn long_block_granule_samples(
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
const ALIAS_REDUCTION_C: [f32; 8] = [
    -0.6, -0.535, -0.33, -0.185, -0.095, -0.041, -0.0142, -0.0037,
];

/// Applies the encoder-side (forward) alias-reduction butterflies in place.
///
/// The decoder rotates spectral lines across each subband boundary to cancel
/// aliasing introduced by the polyphase filterbank; the encoder applies the
/// inverse rotation so the cascade is transparent. Operates on the 576-line
/// subband-major long-block spectrum.
fn apply_alias_reduction(spectrum: &mut [f32]) {
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
/// Both mono and each stereo channel run through the real polyphase + hybrid
/// MDCT analysis (the channel index selects the PCM lane), so stereo
/// reconstructs through a real decoder the same way mono does.
pub fn quantize_pcm_long_block(
    pcm: &AudioBuffer,
    channel: usize,
    start_frame: usize,
    step: f32,
) -> Result<Vec<i32>, Error> {
    let spectrum = layer3_long_block_spectrum(pcm, channel, start_frame)?;
    let inverted: Vec<f32> = spectrum.into_iter().map(|line| -line).collect();
    quantize_spectrum(&inverted, step, 8191)
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Layer3SpectralRegions {
    pub big_values: u16,
    pub count1: u16,
    pub rzero: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Layer3BigValuePair {
    pub x: i16,
    pub y: i16,
}

impl Layer3BigValuePair {
    #[must_use]
    pub fn new(x: i16, y: i16) -> Self {
        Self { x, y }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Layer3BigValueMagnitude {
    pub x: u16,
    pub y: u16,
}

impl Layer3BigValueMagnitude {
    #[must_use]
    pub fn new(x: u16, y: u16) -> Self {
        Self { x, y }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Layer3BigValueTableSelection {
    pub table_select: u8,
    pub linbits: u8,
    pub max_magnitude: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Layer3BigValueRegionTableSelection {
    pub regions: [Layer3BigValueTableSelection; 3],
    pub region0_pairs: u16,
    pub region1_pairs: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Layer3Count1Quad {
    pub v: i8,
    pub w: i8,
    pub x: i8,
    pub y: i8,
}

impl Layer3Count1Quad {
    #[must_use]
    pub fn new(v: i8, w: i8, x: i8, y: i8) -> Self {
        Self { v, w, x, y }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Layer3Count1MagnitudeQuad {
    pub v: u8,
    pub w: u8,
    pub x: u8,
    pub y: u8,
}

impl Layer3Count1MagnitudeQuad {
    #[must_use]
    pub fn new(v: u8, w: u8, x: u8, y: u8) -> Self {
        Self { v, w, x, y }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Layer3Count1TableSelection {
    pub table_select: bool,
    pub max_nonzero_values: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Layer3ScaleFactorCompress {
    pub scalefac_compress: u16,
    pub slen1: u8,
    pub slen2: u8,
}

#[derive(Clone, Copy, Debug)]
pub struct Layer3EntropyTables<'a> {
    pub big_values: &'a [HuffmanEntry<Layer3BigValueMagnitude>],
    pub count1: &'a [HuffmanEntry<Layer3Count1MagnitudeQuad>],
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Layer3EntropyTableProvider<'a> {
    pub big_value_table_1: &'a [HuffmanEntry<Layer3BigValueMagnitude>],
    pub big_value_table_2: &'a [HuffmanEntry<Layer3BigValueMagnitude>],
    pub big_value_table_5: &'a [HuffmanEntry<Layer3BigValueMagnitude>],
    pub big_value_table_7: &'a [HuffmanEntry<Layer3BigValueMagnitude>],
    pub big_value_table_10: &'a [HuffmanEntry<Layer3BigValueMagnitude>],
    pub big_value_table_13: &'a [HuffmanEntry<Layer3BigValueMagnitude>],
    pub big_value_table_16: &'a [HuffmanEntry<Layer3BigValueMagnitude>],
    pub count1_table_0: &'a [HuffmanEntry<Layer3Count1MagnitudeQuad>],
    pub count1_table_1: &'a [HuffmanEntry<Layer3Count1MagnitudeQuad>],
}

impl<'a> Layer3EntropyTableProvider<'a> {
    pub fn big_value_table(
        self,
        selection: Layer3BigValueTableSelection,
    ) -> Result<&'a [HuffmanEntry<Layer3BigValueMagnitude>], Error> {
        let table = match selection.table_select {
            0 => &[],
            1 => self.big_value_table_1,
            2 => self.big_value_table_2,
            5 => self.big_value_table_5,
            7 => self.big_value_table_7,
            10 => self.big_value_table_10,
            13 => self.big_value_table_13,
            // Tables 16..=23 share the table-16 codeword tree (different linbits).
            16..=23 => self.big_value_table_16,
            _ => return Err(Error::UnsupportedFeature("MP3 big-values Huffman table")),
        };
        if selection.table_select != 0 && table.is_empty() {
            return Err(Error::UnsupportedFeature("MP3 big-values Huffman table"));
        }
        Ok(table)
    }

    pub fn count1_table(
        self,
        selection: Layer3Count1TableSelection,
    ) -> Result<&'a [HuffmanEntry<Layer3Count1MagnitudeQuad>], Error> {
        if selection.max_nonzero_values == 0 {
            return Ok(&[]);
        }

        let table = if selection.table_select {
            self.count1_table_1
        } else {
            self.count1_table_0
        };
        if table.is_empty() {
            return Err(Error::UnsupportedFeature("MP3 count1 Huffman table"));
        }
        Ok(table)
    }
}

const MPEG1_LAYER3_BIG_VALUE_TABLE_1: &[HuffmanEntry<Layer3BigValueMagnitude>] = &[
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 0, y: 0 },
        code: HuffmanCode { bits: 0b1, len: 1 },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 0, y: 1 },
        code: HuffmanCode {
            bits: 0b001,
            len: 3,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 1, y: 0 },
        code: HuffmanCode { bits: 0b01, len: 2 },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 1, y: 1 },
        code: HuffmanCode {
            bits: 0b000,
            len: 3,
        },
    },
];

const MPEG1_LAYER3_BIG_VALUE_TABLE_2: &[HuffmanEntry<Layer3BigValueMagnitude>] = &[
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 0, y: 0 },
        code: HuffmanCode { bits: 0b1, len: 1 },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 0, y: 1 },
        code: HuffmanCode {
            bits: 0b010,
            len: 3,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 0, y: 2 },
        code: HuffmanCode {
            bits: 0b000001,
            len: 6,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 1, y: 0 },
        code: HuffmanCode {
            bits: 0b011,
            len: 3,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 1, y: 1 },
        code: HuffmanCode {
            bits: 0b001,
            len: 3,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 1, y: 2 },
        code: HuffmanCode {
            bits: 0b00001,
            len: 5,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 2, y: 0 },
        code: HuffmanCode {
            bits: 0b00011,
            len: 5,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 2, y: 1 },
        code: HuffmanCode {
            bits: 0b00010,
            len: 5,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 2, y: 2 },
        code: HuffmanCode {
            bits: 0b000000,
            len: 6,
        },
    },
];

const MPEG1_LAYER3_COUNT1_TABLE_32: &[HuffmanEntry<Layer3Count1MagnitudeQuad>] = &[
    HuffmanEntry {
        symbol: Layer3Count1MagnitudeQuad {
            v: 0,
            w: 0,
            x: 0,
            y: 0,
        },
        code: HuffmanCode { bits: 0b1, len: 1 },
    },
    HuffmanEntry {
        symbol: Layer3Count1MagnitudeQuad {
            v: 0,
            w: 0,
            x: 0,
            y: 1,
        },
        code: HuffmanCode {
            bits: 0b0101,
            len: 4,
        },
    },
    HuffmanEntry {
        symbol: Layer3Count1MagnitudeQuad {
            v: 0,
            w: 0,
            x: 1,
            y: 0,
        },
        code: HuffmanCode {
            bits: 0b0100,
            len: 4,
        },
    },
    HuffmanEntry {
        symbol: Layer3Count1MagnitudeQuad {
            v: 0,
            w: 0,
            x: 1,
            y: 1,
        },
        code: HuffmanCode {
            bits: 0b0101,
            len: 5,
        },
    },
    HuffmanEntry {
        symbol: Layer3Count1MagnitudeQuad {
            v: 0,
            w: 1,
            x: 0,
            y: 0,
        },
        code: HuffmanCode {
            bits: 0b0110,
            len: 4,
        },
    },
    HuffmanEntry {
        symbol: Layer3Count1MagnitudeQuad {
            v: 0,
            w: 1,
            x: 0,
            y: 1,
        },
        code: HuffmanCode {
            bits: 0b0101,
            len: 6,
        },
    },
    HuffmanEntry {
        symbol: Layer3Count1MagnitudeQuad {
            v: 0,
            w: 1,
            x: 1,
            y: 0,
        },
        code: HuffmanCode {
            bits: 0b0100,
            len: 5,
        },
    },
    HuffmanEntry {
        symbol: Layer3Count1MagnitudeQuad {
            v: 0,
            w: 1,
            x: 1,
            y: 1,
        },
        code: HuffmanCode {
            bits: 0b0100,
            len: 6,
        },
    },
    HuffmanEntry {
        symbol: Layer3Count1MagnitudeQuad {
            v: 1,
            w: 0,
            x: 0,
            y: 0,
        },
        code: HuffmanCode {
            bits: 0b0111,
            len: 4,
        },
    },
    HuffmanEntry {
        symbol: Layer3Count1MagnitudeQuad {
            v: 1,
            w: 0,
            x: 0,
            y: 1,
        },
        code: HuffmanCode {
            bits: 0b0011,
            len: 5,
        },
    },
    HuffmanEntry {
        symbol: Layer3Count1MagnitudeQuad {
            v: 1,
            w: 0,
            x: 1,
            y: 0,
        },
        code: HuffmanCode {
            bits: 0b0110,
            len: 5,
        },
    },
    HuffmanEntry {
        symbol: Layer3Count1MagnitudeQuad {
            v: 1,
            w: 0,
            x: 1,
            y: 1,
        },
        code: HuffmanCode {
            bits: 0b0000,
            len: 6,
        },
    },
    HuffmanEntry {
        symbol: Layer3Count1MagnitudeQuad {
            v: 1,
            w: 1,
            x: 0,
            y: 0,
        },
        code: HuffmanCode {
            bits: 0b0111,
            len: 5,
        },
    },
    HuffmanEntry {
        symbol: Layer3Count1MagnitudeQuad {
            v: 1,
            w: 1,
            x: 0,
            y: 1,
        },
        code: HuffmanCode {
            bits: 0b0010,
            len: 6,
        },
    },
    HuffmanEntry {
        symbol: Layer3Count1MagnitudeQuad {
            v: 1,
            w: 1,
            x: 1,
            y: 0,
        },
        code: HuffmanCode {
            bits: 0b0011,
            len: 6,
        },
    },
    HuffmanEntry {
        symbol: Layer3Count1MagnitudeQuad {
            v: 1,
            w: 1,
            x: 1,
            y: 1,
        },
        code: HuffmanCode {
            bits: 0b0001,
            len: 6,
        },
    },
];

const MPEG1_LAYER3_COUNT1_TABLE_33: &[HuffmanEntry<Layer3Count1MagnitudeQuad>] = &[
    HuffmanEntry {
        symbol: Layer3Count1MagnitudeQuad {
            v: 0,
            w: 0,
            x: 0,
            y: 0,
        },
        code: HuffmanCode {
            bits: 0b1111,
            len: 4,
        },
    },
    HuffmanEntry {
        symbol: Layer3Count1MagnitudeQuad {
            v: 0,
            w: 0,
            x: 0,
            y: 1,
        },
        code: HuffmanCode {
            bits: 0b1110,
            len: 4,
        },
    },
    HuffmanEntry {
        symbol: Layer3Count1MagnitudeQuad {
            v: 0,
            w: 0,
            x: 1,
            y: 0,
        },
        code: HuffmanCode {
            bits: 0b1101,
            len: 4,
        },
    },
    HuffmanEntry {
        symbol: Layer3Count1MagnitudeQuad {
            v: 0,
            w: 0,
            x: 1,
            y: 1,
        },
        code: HuffmanCode {
            bits: 0b1100,
            len: 4,
        },
    },
    HuffmanEntry {
        symbol: Layer3Count1MagnitudeQuad {
            v: 0,
            w: 1,
            x: 0,
            y: 0,
        },
        code: HuffmanCode {
            bits: 0b1011,
            len: 4,
        },
    },
    HuffmanEntry {
        symbol: Layer3Count1MagnitudeQuad {
            v: 0,
            w: 1,
            x: 0,
            y: 1,
        },
        code: HuffmanCode {
            bits: 0b1010,
            len: 4,
        },
    },
    HuffmanEntry {
        symbol: Layer3Count1MagnitudeQuad {
            v: 0,
            w: 1,
            x: 1,
            y: 0,
        },
        code: HuffmanCode {
            bits: 0b111,
            len: 3,
        },
    },
    HuffmanEntry {
        symbol: Layer3Count1MagnitudeQuad {
            v: 0,
            w: 1,
            x: 1,
            y: 1,
        },
        code: HuffmanCode {
            bits: 0b110,
            len: 3,
        },
    },
    HuffmanEntry {
        symbol: Layer3Count1MagnitudeQuad {
            v: 1,
            w: 0,
            x: 0,
            y: 0,
        },
        code: HuffmanCode {
            bits: 0b1001,
            len: 4,
        },
    },
    HuffmanEntry {
        symbol: Layer3Count1MagnitudeQuad {
            v: 1,
            w: 0,
            x: 0,
            y: 1,
        },
        code: HuffmanCode {
            bits: 0b101,
            len: 3,
        },
    },
    HuffmanEntry {
        symbol: Layer3Count1MagnitudeQuad {
            v: 1,
            w: 0,
            x: 1,
            y: 0,
        },
        code: HuffmanCode {
            bits: 0b1000,
            len: 4,
        },
    },
    HuffmanEntry {
        symbol: Layer3Count1MagnitudeQuad {
            v: 1,
            w: 0,
            x: 1,
            y: 1,
        },
        code: HuffmanCode {
            bits: 0b100,
            len: 3,
        },
    },
    HuffmanEntry {
        symbol: Layer3Count1MagnitudeQuad {
            v: 1,
            w: 1,
            x: 0,
            y: 0,
        },
        code: HuffmanCode {
            bits: 0b0111,
            len: 4,
        },
    },
    HuffmanEntry {
        symbol: Layer3Count1MagnitudeQuad {
            v: 1,
            w: 1,
            x: 0,
            y: 1,
        },
        code: HuffmanCode {
            bits: 0b011,
            len: 3,
        },
    },
    HuffmanEntry {
        symbol: Layer3Count1MagnitudeQuad {
            v: 1,
            w: 1,
            x: 1,
            y: 0,
        },
        code: HuffmanCode { bits: 0b10, len: 2 },
    },
    HuffmanEntry {
        symbol: Layer3Count1MagnitudeQuad {
            v: 1,
            w: 1,
            x: 1,
            y: 1,
        },
        code: HuffmanCode { bits: 0b1, len: 1 },
    },
];

// MPEG-1 Layer III big-values Huffman code tables (ISO/IEC 11172-3 Annex B
// Table 3-B.7). These are normative ISO constants; the codeword/length pairs
// here were derived by walking the decode tree of the public-domain (Unlicense)
// PDMP3 decoder — the same neutral, non-copyleft source as the analysis window.
// Clean-room applies only to copyleft *encoders* (LAME et al.), which were not
// consulted. Tables 16..=23 share table 16's codewords with different linbits.
const MPEG1_LAYER3_BIG_VALUE_TABLE_5: &[HuffmanEntry<Layer3BigValueMagnitude>] = &[
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 0, y: 0 },
        code: HuffmanCode { bits: 0b1, len: 1 },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 0, y: 1 },
        code: HuffmanCode {
            bits: 0b010,
            len: 3,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 0, y: 2 },
        code: HuffmanCode {
            bits: 0b000110,
            len: 6,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 0, y: 3 },
        code: HuffmanCode {
            bits: 0b0000101,
            len: 7,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 1, y: 0 },
        code: HuffmanCode {
            bits: 0b011,
            len: 3,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 1, y: 1 },
        code: HuffmanCode {
            bits: 0b001,
            len: 3,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 1, y: 2 },
        code: HuffmanCode {
            bits: 0b000100,
            len: 6,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 1, y: 3 },
        code: HuffmanCode {
            bits: 0b0000100,
            len: 7,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 2, y: 0 },
        code: HuffmanCode {
            bits: 0b000111,
            len: 6,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 2, y: 1 },
        code: HuffmanCode {
            bits: 0b000101,
            len: 6,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 2, y: 2 },
        code: HuffmanCode {
            bits: 0b0000111,
            len: 7,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 2, y: 3 },
        code: HuffmanCode {
            bits: 0b00000001,
            len: 8,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 3, y: 0 },
        code: HuffmanCode {
            bits: 0b0000110,
            len: 7,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 3, y: 1 },
        code: HuffmanCode {
            bits: 0b000001,
            len: 6,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 3, y: 2 },
        code: HuffmanCode {
            bits: 0b0000001,
            len: 7,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 3, y: 3 },
        code: HuffmanCode {
            bits: 0b00000000,
            len: 8,
        },
    },
];

const MPEG1_LAYER3_BIG_VALUE_TABLE_7: &[HuffmanEntry<Layer3BigValueMagnitude>] = &[
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 0, y: 0 },
        code: HuffmanCode { bits: 0b1, len: 1 },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 0, y: 1 },
        code: HuffmanCode {
            bits: 0b010,
            len: 3,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 0, y: 2 },
        code: HuffmanCode {
            bits: 0b001010,
            len: 6,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 0, y: 3 },
        code: HuffmanCode {
            bits: 0b00010011,
            len: 8,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 0, y: 4 },
        code: HuffmanCode {
            bits: 0b00010000,
            len: 8,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 0, y: 5 },
        code: HuffmanCode {
            bits: 0b000001010,
            len: 9,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 1, y: 0 },
        code: HuffmanCode {
            bits: 0b011,
            len: 3,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 1, y: 1 },
        code: HuffmanCode {
            bits: 0b0011,
            len: 4,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 1, y: 2 },
        code: HuffmanCode {
            bits: 0b000111,
            len: 6,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 1, y: 3 },
        code: HuffmanCode {
            bits: 0b0001010,
            len: 7,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 1, y: 4 },
        code: HuffmanCode {
            bits: 0b0000101,
            len: 7,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 1, y: 5 },
        code: HuffmanCode {
            bits: 0b00000011,
            len: 8,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 2, y: 0 },
        code: HuffmanCode {
            bits: 0b001011,
            len: 6,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 2, y: 1 },
        code: HuffmanCode {
            bits: 0b00100,
            len: 5,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 2, y: 2 },
        code: HuffmanCode {
            bits: 0b0001101,
            len: 7,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 2, y: 3 },
        code: HuffmanCode {
            bits: 0b00010001,
            len: 8,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 2, y: 4 },
        code: HuffmanCode {
            bits: 0b00001000,
            len: 8,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 2, y: 5 },
        code: HuffmanCode {
            bits: 0b000000100,
            len: 9,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 3, y: 0 },
        code: HuffmanCode {
            bits: 0b0001100,
            len: 7,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 3, y: 1 },
        code: HuffmanCode {
            bits: 0b0001011,
            len: 7,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 3, y: 2 },
        code: HuffmanCode {
            bits: 0b00010010,
            len: 8,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 3, y: 3 },
        code: HuffmanCode {
            bits: 0b000001111,
            len: 9,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 3, y: 4 },
        code: HuffmanCode {
            bits: 0b000001011,
            len: 9,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 3, y: 5 },
        code: HuffmanCode {
            bits: 0b000000010,
            len: 9,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 4, y: 0 },
        code: HuffmanCode {
            bits: 0b0000111,
            len: 7,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 4, y: 1 },
        code: HuffmanCode {
            bits: 0b0000110,
            len: 7,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 4, y: 2 },
        code: HuffmanCode {
            bits: 0b00001001,
            len: 8,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 4, y: 3 },
        code: HuffmanCode {
            bits: 0b000001110,
            len: 9,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 4, y: 4 },
        code: HuffmanCode {
            bits: 0b000000011,
            len: 9,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 4, y: 5 },
        code: HuffmanCode {
            bits: 0b0000000001,
            len: 10,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 5, y: 0 },
        code: HuffmanCode {
            bits: 0b00000110,
            len: 8,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 5, y: 1 },
        code: HuffmanCode {
            bits: 0b00000100,
            len: 8,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 5, y: 2 },
        code: HuffmanCode {
            bits: 0b000000101,
            len: 9,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 5, y: 3 },
        code: HuffmanCode {
            bits: 0b0000000011,
            len: 10,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 5, y: 4 },
        code: HuffmanCode {
            bits: 0b0000000010,
            len: 10,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 5, y: 5 },
        code: HuffmanCode {
            bits: 0b0000000000,
            len: 10,
        },
    },
];

const MPEG1_LAYER3_BIG_VALUE_TABLE_10: &[HuffmanEntry<Layer3BigValueMagnitude>] = &[
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 0, y: 0 },
        code: HuffmanCode { bits: 0b1, len: 1 },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 0, y: 1 },
        code: HuffmanCode {
            bits: 0b010,
            len: 3,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 0, y: 2 },
        code: HuffmanCode {
            bits: 0b001010,
            len: 6,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 0, y: 3 },
        code: HuffmanCode {
            bits: 0b00010111,
            len: 8,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 0, y: 4 },
        code: HuffmanCode {
            bits: 0b000100011,
            len: 9,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 0, y: 5 },
        code: HuffmanCode {
            bits: 0b000011110,
            len: 9,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 0, y: 6 },
        code: HuffmanCode {
            bits: 0b000001100,
            len: 9,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 0, y: 7 },
        code: HuffmanCode {
            bits: 0b0000010001,
            len: 10,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 1, y: 0 },
        code: HuffmanCode {
            bits: 0b011,
            len: 3,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 1, y: 1 },
        code: HuffmanCode {
            bits: 0b0011,
            len: 4,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 1, y: 2 },
        code: HuffmanCode {
            bits: 0b001000,
            len: 6,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 1, y: 3 },
        code: HuffmanCode {
            bits: 0b0001100,
            len: 7,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 1, y: 4 },
        code: HuffmanCode {
            bits: 0b00010010,
            len: 8,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 1, y: 5 },
        code: HuffmanCode {
            bits: 0b000010101,
            len: 9,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 1, y: 6 },
        code: HuffmanCode {
            bits: 0b00001100,
            len: 8,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 1, y: 7 },
        code: HuffmanCode {
            bits: 0b00000111,
            len: 8,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 2, y: 0 },
        code: HuffmanCode {
            bits: 0b001011,
            len: 6,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 2, y: 1 },
        code: HuffmanCode {
            bits: 0b001001,
            len: 6,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 2, y: 2 },
        code: HuffmanCode {
            bits: 0b0001111,
            len: 7,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 2, y: 3 },
        code: HuffmanCode {
            bits: 0b00010101,
            len: 8,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 2, y: 4 },
        code: HuffmanCode {
            bits: 0b000100000,
            len: 9,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 2, y: 5 },
        code: HuffmanCode {
            bits: 0b0000101000,
            len: 10,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 2, y: 6 },
        code: HuffmanCode {
            bits: 0b000010011,
            len: 9,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 2, y: 7 },
        code: HuffmanCode {
            bits: 0b000000110,
            len: 9,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 3, y: 0 },
        code: HuffmanCode {
            bits: 0b0001110,
            len: 7,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 3, y: 1 },
        code: HuffmanCode {
            bits: 0b0001101,
            len: 7,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 3, y: 2 },
        code: HuffmanCode {
            bits: 0b00010110,
            len: 8,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 3, y: 3 },
        code: HuffmanCode {
            bits: 0b000100010,
            len: 9,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 3, y: 4 },
        code: HuffmanCode {
            bits: 0b0000101110,
            len: 10,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 3, y: 5 },
        code: HuffmanCode {
            bits: 0b0000010111,
            len: 10,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 3, y: 6 },
        code: HuffmanCode {
            bits: 0b000010010,
            len: 9,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 3, y: 7 },
        code: HuffmanCode {
            bits: 0b0000000111,
            len: 10,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 4, y: 0 },
        code: HuffmanCode {
            bits: 0b00010100,
            len: 8,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 4, y: 1 },
        code: HuffmanCode {
            bits: 0b00010011,
            len: 8,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 4, y: 2 },
        code: HuffmanCode {
            bits: 0b000100001,
            len: 9,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 4, y: 3 },
        code: HuffmanCode {
            bits: 0b0000101111,
            len: 10,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 4, y: 4 },
        code: HuffmanCode {
            bits: 0b0000011011,
            len: 10,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 4, y: 5 },
        code: HuffmanCode {
            bits: 0b0000010110,
            len: 10,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 4, y: 6 },
        code: HuffmanCode {
            bits: 0b0000001001,
            len: 10,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 4, y: 7 },
        code: HuffmanCode {
            bits: 0b0000000011,
            len: 10,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 5, y: 0 },
        code: HuffmanCode {
            bits: 0b000011111,
            len: 9,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 5, y: 1 },
        code: HuffmanCode {
            bits: 0b000010110,
            len: 9,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 5, y: 2 },
        code: HuffmanCode {
            bits: 0b0000101001,
            len: 10,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 5, y: 3 },
        code: HuffmanCode {
            bits: 0b0000011010,
            len: 10,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 5, y: 4 },
        code: HuffmanCode {
            bits: 0b00000010101,
            len: 11,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 5, y: 5 },
        code: HuffmanCode {
            bits: 0b00000010100,
            len: 11,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 5, y: 6 },
        code: HuffmanCode {
            bits: 0b0000000101,
            len: 10,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 5, y: 7 },
        code: HuffmanCode {
            bits: 0b00000000011,
            len: 11,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 6, y: 0 },
        code: HuffmanCode {
            bits: 0b00001110,
            len: 8,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 6, y: 1 },
        code: HuffmanCode {
            bits: 0b00001101,
            len: 8,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 6, y: 2 },
        code: HuffmanCode {
            bits: 0b000001010,
            len: 9,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 6, y: 3 },
        code: HuffmanCode {
            bits: 0b0000001011,
            len: 10,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 6, y: 4 },
        code: HuffmanCode {
            bits: 0b0000010000,
            len: 10,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 6, y: 5 },
        code: HuffmanCode {
            bits: 0b0000000110,
            len: 10,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 6, y: 6 },
        code: HuffmanCode {
            bits: 0b00000000101,
            len: 11,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 6, y: 7 },
        code: HuffmanCode {
            bits: 0b00000000001,
            len: 11,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 7, y: 0 },
        code: HuffmanCode {
            bits: 0b000001001,
            len: 9,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 7, y: 1 },
        code: HuffmanCode {
            bits: 0b00001000,
            len: 8,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 7, y: 2 },
        code: HuffmanCode {
            bits: 0b000000111,
            len: 9,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 7, y: 3 },
        code: HuffmanCode {
            bits: 0b0000001000,
            len: 10,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 7, y: 4 },
        code: HuffmanCode {
            bits: 0b0000000100,
            len: 10,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 7, y: 5 },
        code: HuffmanCode {
            bits: 0b00000000100,
            len: 11,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 7, y: 6 },
        code: HuffmanCode {
            bits: 0b00000000010,
            len: 11,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 7, y: 7 },
        code: HuffmanCode {
            bits: 0b00000000000,
            len: 11,
        },
    },
];

const MPEG1_LAYER3_BIG_VALUE_TABLE_13: &[HuffmanEntry<Layer3BigValueMagnitude>] = &[
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 0, y: 0 },
        code: HuffmanCode { bits: 0b1, len: 1 },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 0, y: 1 },
        code: HuffmanCode {
            bits: 0b0101,
            len: 4,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 0, y: 2 },
        code: HuffmanCode {
            bits: 0b001110,
            len: 6,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 0, y: 3 },
        code: HuffmanCode {
            bits: 0b0010101,
            len: 7,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 0, y: 4 },
        code: HuffmanCode {
            bits: 0b00100010,
            len: 8,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 0, y: 5 },
        code: HuffmanCode {
            bits: 0b000110011,
            len: 9,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 0, y: 6 },
        code: HuffmanCode {
            bits: 0b000101110,
            len: 9,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 0, y: 7 },
        code: HuffmanCode {
            bits: 0b0001000111,
            len: 10,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 0, y: 8 },
        code: HuffmanCode {
            bits: 0b000101010,
            len: 9,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 0, y: 9 },
        code: HuffmanCode {
            bits: 0b0000110100,
            len: 10,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 0, y: 10 },
        code: HuffmanCode {
            bits: 0b00001000100,
            len: 11,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 0, y: 11 },
        code: HuffmanCode {
            bits: 0b00000110100,
            len: 11,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 0, y: 12 },
        code: HuffmanCode {
            bits: 0b000001000011,
            len: 12,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 0, y: 13 },
        code: HuffmanCode {
            bits: 0b000000101100,
            len: 12,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 0, y: 14 },
        code: HuffmanCode {
            bits: 0b0000000101011,
            len: 13,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 0, y: 15 },
        code: HuffmanCode {
            bits: 0b0000000010011,
            len: 13,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 1, y: 0 },
        code: HuffmanCode {
            bits: 0b011,
            len: 3,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 1, y: 1 },
        code: HuffmanCode {
            bits: 0b0100,
            len: 4,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 1, y: 2 },
        code: HuffmanCode {
            bits: 0b001100,
            len: 6,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 1, y: 3 },
        code: HuffmanCode {
            bits: 0b0010011,
            len: 7,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 1, y: 4 },
        code: HuffmanCode {
            bits: 0b00011111,
            len: 8,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 1, y: 5 },
        code: HuffmanCode {
            bits: 0b00011010,
            len: 8,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 1, y: 6 },
        code: HuffmanCode {
            bits: 0b000101100,
            len: 9,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 1, y: 7 },
        code: HuffmanCode {
            bits: 0b000100001,
            len: 9,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 1, y: 8 },
        code: HuffmanCode {
            bits: 0b000011111,
            len: 9,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 1, y: 9 },
        code: HuffmanCode {
            bits: 0b000011000,
            len: 9,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 1, y: 10 },
        code: HuffmanCode {
            bits: 0b0000100000,
            len: 10,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 1, y: 11 },
        code: HuffmanCode {
            bits: 0b0000011000,
            len: 10,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 1, y: 12 },
        code: HuffmanCode {
            bits: 0b00000011111,
            len: 11,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 1, y: 13 },
        code: HuffmanCode {
            bits: 0b000000100011,
            len: 12,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 1, y: 14 },
        code: HuffmanCode {
            bits: 0b000000010110,
            len: 12,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 1, y: 15 },
        code: HuffmanCode {
            bits: 0b000000001110,
            len: 12,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 2, y: 0 },
        code: HuffmanCode {
            bits: 0b001111,
            len: 6,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 2, y: 1 },
        code: HuffmanCode {
            bits: 0b001101,
            len: 6,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 2, y: 2 },
        code: HuffmanCode {
            bits: 0b0010111,
            len: 7,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 2, y: 3 },
        code: HuffmanCode {
            bits: 0b00100100,
            len: 8,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 2, y: 4 },
        code: HuffmanCode {
            bits: 0b000111011,
            len: 9,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 2, y: 5 },
        code: HuffmanCode {
            bits: 0b000110001,
            len: 9,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 2, y: 6 },
        code: HuffmanCode {
            bits: 0b0001001101,
            len: 10,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 2, y: 7 },
        code: HuffmanCode {
            bits: 0b0001000001,
            len: 10,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 2, y: 8 },
        code: HuffmanCode {
            bits: 0b000011101,
            len: 9,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 2, y: 9 },
        code: HuffmanCode {
            bits: 0b0000101000,
            len: 10,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 2, y: 10 },
        code: HuffmanCode {
            bits: 0b0000011110,
            len: 10,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 2, y: 11 },
        code: HuffmanCode {
            bits: 0b00000101000,
            len: 11,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 2, y: 12 },
        code: HuffmanCode {
            bits: 0b00000011011,
            len: 11,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 2, y: 13 },
        code: HuffmanCode {
            bits: 0b000000100001,
            len: 12,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 2, y: 14 },
        code: HuffmanCode {
            bits: 0b0000000101010,
            len: 13,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 2, y: 15 },
        code: HuffmanCode {
            bits: 0b0000000010000,
            len: 13,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 3, y: 0 },
        code: HuffmanCode {
            bits: 0b0010110,
            len: 7,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 3, y: 1 },
        code: HuffmanCode {
            bits: 0b0010100,
            len: 7,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 3, y: 2 },
        code: HuffmanCode {
            bits: 0b00100101,
            len: 8,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 3, y: 3 },
        code: HuffmanCode {
            bits: 0b000111101,
            len: 9,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 3, y: 4 },
        code: HuffmanCode {
            bits: 0b000111000,
            len: 9,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 3, y: 5 },
        code: HuffmanCode {
            bits: 0b0001001111,
            len: 10,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 3, y: 6 },
        code: HuffmanCode {
            bits: 0b0001001001,
            len: 10,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 3, y: 7 },
        code: HuffmanCode {
            bits: 0b0001000000,
            len: 10,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 3, y: 8 },
        code: HuffmanCode {
            bits: 0b0000101011,
            len: 10,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 3, y: 9 },
        code: HuffmanCode {
            bits: 0b00001001100,
            len: 11,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 3, y: 10 },
        code: HuffmanCode {
            bits: 0b00000111000,
            len: 11,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 3, y: 11 },
        code: HuffmanCode {
            bits: 0b00000100101,
            len: 11,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 3, y: 12 },
        code: HuffmanCode {
            bits: 0b00000011010,
            len: 11,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 3, y: 13 },
        code: HuffmanCode {
            bits: 0b000000011111,
            len: 12,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 3, y: 14 },
        code: HuffmanCode {
            bits: 0b0000000011001,
            len: 13,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 3, y: 15 },
        code: HuffmanCode {
            bits: 0b0000000001110,
            len: 13,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 4, y: 0 },
        code: HuffmanCode {
            bits: 0b00100011,
            len: 8,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 4, y: 1 },
        code: HuffmanCode {
            bits: 0b0010000,
            len: 7,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 4, y: 2 },
        code: HuffmanCode {
            bits: 0b000111100,
            len: 9,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 4, y: 3 },
        code: HuffmanCode {
            bits: 0b000111001,
            len: 9,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 4, y: 4 },
        code: HuffmanCode {
            bits: 0b0001100001,
            len: 10,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 4, y: 5 },
        code: HuffmanCode {
            bits: 0b0001001011,
            len: 10,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 4, y: 6 },
        code: HuffmanCode {
            bits: 0b00001110010,
            len: 11,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 4, y: 7 },
        code: HuffmanCode {
            bits: 0b00001011011,
            len: 11,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 4, y: 8 },
        code: HuffmanCode {
            bits: 0b0000110110,
            len: 10,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 4, y: 9 },
        code: HuffmanCode {
            bits: 0b00001001001,
            len: 11,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 4, y: 10 },
        code: HuffmanCode {
            bits: 0b00000110111,
            len: 11,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 4, y: 11 },
        code: HuffmanCode {
            bits: 0b000000101001,
            len: 12,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 4, y: 12 },
        code: HuffmanCode {
            bits: 0b000000110000,
            len: 12,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 4, y: 13 },
        code: HuffmanCode {
            bits: 0b0000000110101,
            len: 13,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 4, y: 14 },
        code: HuffmanCode {
            bits: 0b0000000010111,
            len: 13,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 4, y: 15 },
        code: HuffmanCode {
            bits: 0b00000000011000,
            len: 14,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 5, y: 0 },
        code: HuffmanCode {
            bits: 0b000111010,
            len: 9,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 5, y: 1 },
        code: HuffmanCode {
            bits: 0b00011011,
            len: 8,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 5, y: 2 },
        code: HuffmanCode {
            bits: 0b000110010,
            len: 9,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 5, y: 3 },
        code: HuffmanCode {
            bits: 0b0001100000,
            len: 10,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 5, y: 4 },
        code: HuffmanCode {
            bits: 0b0001001100,
            len: 10,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 5, y: 5 },
        code: HuffmanCode {
            bits: 0b0001000110,
            len: 10,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 5, y: 6 },
        code: HuffmanCode {
            bits: 0b00001011101,
            len: 11,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 5, y: 7 },
        code: HuffmanCode {
            bits: 0b00001010100,
            len: 11,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 5, y: 8 },
        code: HuffmanCode {
            bits: 0b00001001101,
            len: 11,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 5, y: 9 },
        code: HuffmanCode {
            bits: 0b00000111010,
            len: 11,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 5, y: 10 },
        code: HuffmanCode {
            bits: 0b000001001111,
            len: 12,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 5, y: 11 },
        code: HuffmanCode {
            bits: 0b00000011101,
            len: 11,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 5, y: 12 },
        code: HuffmanCode {
            bits: 0b0000001001010,
            len: 13,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 5, y: 13 },
        code: HuffmanCode {
            bits: 0b0000000110001,
            len: 13,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 5, y: 14 },
        code: HuffmanCode {
            bits: 0b00000000101001,
            len: 14,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 5, y: 15 },
        code: HuffmanCode {
            bits: 0b00000000010001,
            len: 14,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 6, y: 0 },
        code: HuffmanCode {
            bits: 0b000101111,
            len: 9,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 6, y: 1 },
        code: HuffmanCode {
            bits: 0b000101101,
            len: 9,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 6, y: 2 },
        code: HuffmanCode {
            bits: 0b0001001110,
            len: 10,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 6, y: 3 },
        code: HuffmanCode {
            bits: 0b0001001010,
            len: 10,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 6, y: 4 },
        code: HuffmanCode {
            bits: 0b00001110011,
            len: 11,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 6, y: 5 },
        code: HuffmanCode {
            bits: 0b00001011110,
            len: 11,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 6, y: 6 },
        code: HuffmanCode {
            bits: 0b00001011010,
            len: 11,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 6, y: 7 },
        code: HuffmanCode {
            bits: 0b00001001111,
            len: 11,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 6, y: 8 },
        code: HuffmanCode {
            bits: 0b00001000101,
            len: 11,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 6, y: 9 },
        code: HuffmanCode {
            bits: 0b000001010011,
            len: 12,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 6, y: 10 },
        code: HuffmanCode {
            bits: 0b000001000111,
            len: 12,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 6, y: 11 },
        code: HuffmanCode {
            bits: 0b000000110010,
            len: 12,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 6, y: 12 },
        code: HuffmanCode {
            bits: 0b0000000111011,
            len: 13,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 6, y: 13 },
        code: HuffmanCode {
            bits: 0b0000000100110,
            len: 13,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 6, y: 14 },
        code: HuffmanCode {
            bits: 0b00000000100100,
            len: 14,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 6, y: 15 },
        code: HuffmanCode {
            bits: 0b00000000001111,
            len: 14,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 7, y: 0 },
        code: HuffmanCode {
            bits: 0b0001001000,
            len: 10,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 7, y: 1 },
        code: HuffmanCode {
            bits: 0b000100010,
            len: 9,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 7, y: 2 },
        code: HuffmanCode {
            bits: 0b0000111000,
            len: 10,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 7, y: 3 },
        code: HuffmanCode {
            bits: 0b00001011111,
            len: 11,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 7, y: 4 },
        code: HuffmanCode {
            bits: 0b00001011100,
            len: 11,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 7, y: 5 },
        code: HuffmanCode {
            bits: 0b00001010101,
            len: 11,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 7, y: 6 },
        code: HuffmanCode {
            bits: 0b000001011011,
            len: 12,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 7, y: 7 },
        code: HuffmanCode {
            bits: 0b000001011010,
            len: 12,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 7, y: 8 },
        code: HuffmanCode {
            bits: 0b000001010110,
            len: 12,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 7, y: 9 },
        code: HuffmanCode {
            bits: 0b000001001001,
            len: 12,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 7, y: 10 },
        code: HuffmanCode {
            bits: 0b0000001001101,
            len: 13,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 7, y: 11 },
        code: HuffmanCode {
            bits: 0b0000001000001,
            len: 13,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 7, y: 12 },
        code: HuffmanCode {
            bits: 0b0000000110011,
            len: 13,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 7, y: 13 },
        code: HuffmanCode {
            bits: 0b00000000101100,
            len: 14,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 7, y: 14 },
        code: HuffmanCode {
            bits: 0b0000000000101011,
            len: 16,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 7, y: 15 },
        code: HuffmanCode {
            bits: 0b0000000000101010,
            len: 16,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 8, y: 0 },
        code: HuffmanCode {
            bits: 0b000101011,
            len: 9,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 8, y: 1 },
        code: HuffmanCode {
            bits: 0b00010100,
            len: 8,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 8, y: 2 },
        code: HuffmanCode {
            bits: 0b000011110,
            len: 9,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 8, y: 3 },
        code: HuffmanCode {
            bits: 0b0000101100,
            len: 10,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 8, y: 4 },
        code: HuffmanCode {
            bits: 0b0000110111,
            len: 10,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 8, y: 5 },
        code: HuffmanCode {
            bits: 0b00001001110,
            len: 11,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 8, y: 6 },
        code: HuffmanCode {
            bits: 0b00001001000,
            len: 11,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 8, y: 7 },
        code: HuffmanCode {
            bits: 0b000001010111,
            len: 12,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 8, y: 8 },
        code: HuffmanCode {
            bits: 0b000001001110,
            len: 12,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 8, y: 9 },
        code: HuffmanCode {
            bits: 0b000000111101,
            len: 12,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 8, y: 10 },
        code: HuffmanCode {
            bits: 0b000000101110,
            len: 12,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 8, y: 11 },
        code: HuffmanCode {
            bits: 0b0000000110110,
            len: 13,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 8, y: 12 },
        code: HuffmanCode {
            bits: 0b0000000100101,
            len: 13,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 8, y: 13 },
        code: HuffmanCode {
            bits: 0b00000000011110,
            len: 14,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 8, y: 14 },
        code: HuffmanCode {
            bits: 0b000000000010100,
            len: 15,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 8, y: 15 },
        code: HuffmanCode {
            bits: 0b000000000010000,
            len: 15,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 9, y: 0 },
        code: HuffmanCode {
            bits: 0b0000110101,
            len: 10,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 9, y: 1 },
        code: HuffmanCode {
            bits: 0b000011001,
            len: 9,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 9, y: 2 },
        code: HuffmanCode {
            bits: 0b0000101001,
            len: 10,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 9, y: 3 },
        code: HuffmanCode {
            bits: 0b0000100101,
            len: 10,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 9, y: 4 },
        code: HuffmanCode {
            bits: 0b00000101100,
            len: 11,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 9, y: 5 },
        code: HuffmanCode {
            bits: 0b00000111011,
            len: 11,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 9, y: 6 },
        code: HuffmanCode {
            bits: 0b00000110110,
            len: 11,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 9, y: 7 },
        code: HuffmanCode {
            bits: 0b0000001010001,
            len: 13,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 9, y: 8 },
        code: HuffmanCode {
            bits: 0b000001000010,
            len: 12,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 9, y: 9 },
        code: HuffmanCode {
            bits: 0b0000001001100,
            len: 13,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 9, y: 10 },
        code: HuffmanCode {
            bits: 0b0000000111001,
            len: 13,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 9, y: 11 },
        code: HuffmanCode {
            bits: 0b00000000110110,
            len: 14,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 9, y: 12 },
        code: HuffmanCode {
            bits: 0b00000000100101,
            len: 14,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 9, y: 13 },
        code: HuffmanCode {
            bits: 0b00000000010010,
            len: 14,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 9, y: 14 },
        code: HuffmanCode {
            bits: 0b0000000000100111,
            len: 16,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 9, y: 15 },
        code: HuffmanCode {
            bits: 0b000000000001011,
            len: 15,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 10, y: 0 },
        code: HuffmanCode {
            bits: 0b0000100011,
            len: 10,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 10, y: 1 },
        code: HuffmanCode {
            bits: 0b0000100001,
            len: 10,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 10, y: 2 },
        code: HuffmanCode {
            bits: 0b0000011111,
            len: 10,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 10, y: 3 },
        code: HuffmanCode {
            bits: 0b00000111001,
            len: 11,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 10, y: 4 },
        code: HuffmanCode {
            bits: 0b00000101010,
            len: 11,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 10, y: 5 },
        code: HuffmanCode {
            bits: 0b000001010010,
            len: 12,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 10, y: 6 },
        code: HuffmanCode {
            bits: 0b000001001000,
            len: 12,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 10, y: 7 },
        code: HuffmanCode {
            bits: 0b0000001010000,
            len: 13,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 10, y: 8 },
        code: HuffmanCode {
            bits: 0b000000101111,
            len: 12,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 10, y: 9 },
        code: HuffmanCode {
            bits: 0b0000000111010,
            len: 13,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 10, y: 10 },
        code: HuffmanCode {
            bits: 0b00000000110111,
            len: 14,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 10, y: 11 },
        code: HuffmanCode {
            bits: 0b0000000010101,
            len: 13,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 10, y: 12 },
        code: HuffmanCode {
            bits: 0b00000000010110,
            len: 14,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 10, y: 13 },
        code: HuffmanCode {
            bits: 0b000000000011010,
            len: 15,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 10, y: 14 },
        code: HuffmanCode {
            bits: 0b0000000000100110,
            len: 16,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 10, y: 15 },
        code: HuffmanCode {
            bits: 0b00000000000010110,
            len: 17,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 11, y: 0 },
        code: HuffmanCode {
            bits: 0b00000110101,
            len: 11,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 11, y: 1 },
        code: HuffmanCode {
            bits: 0b0000011001,
            len: 10,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 11, y: 2 },
        code: HuffmanCode {
            bits: 0b0000010111,
            len: 10,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 11, y: 3 },
        code: HuffmanCode {
            bits: 0b00000100110,
            len: 11,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 11, y: 4 },
        code: HuffmanCode {
            bits: 0b000001000110,
            len: 12,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 11, y: 5 },
        code: HuffmanCode {
            bits: 0b000000111100,
            len: 12,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 11, y: 6 },
        code: HuffmanCode {
            bits: 0b000000110011,
            len: 12,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 11, y: 7 },
        code: HuffmanCode {
            bits: 0b000000100100,
            len: 12,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 11, y: 8 },
        code: HuffmanCode {
            bits: 0b0000000110111,
            len: 13,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 11, y: 9 },
        code: HuffmanCode {
            bits: 0b0000000011010,
            len: 13,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 11, y: 10 },
        code: HuffmanCode {
            bits: 0b0000000100010,
            len: 13,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 11, y: 11 },
        code: HuffmanCode {
            bits: 0b00000000010111,
            len: 14,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 11, y: 12 },
        code: HuffmanCode {
            bits: 0b000000000011011,
            len: 15,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 11, y: 13 },
        code: HuffmanCode {
            bits: 0b000000000001110,
            len: 15,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 11, y: 14 },
        code: HuffmanCode {
            bits: 0b000000000001001,
            len: 15,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 11, y: 15 },
        code: HuffmanCode {
            bits: 0b0000000000000111,
            len: 16,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 12, y: 0 },
        code: HuffmanCode {
            bits: 0b00000100010,
            len: 11,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 12, y: 1 },
        code: HuffmanCode {
            bits: 0b00000100000,
            len: 11,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 12, y: 2 },
        code: HuffmanCode {
            bits: 0b00000011100,
            len: 11,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 12, y: 3 },
        code: HuffmanCode {
            bits: 0b000000100111,
            len: 12,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 12, y: 4 },
        code: HuffmanCode {
            bits: 0b000000110001,
            len: 12,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 12, y: 5 },
        code: HuffmanCode {
            bits: 0b0000001001011,
            len: 13,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 12, y: 6 },
        code: HuffmanCode {
            bits: 0b000000011110,
            len: 12,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 12, y: 7 },
        code: HuffmanCode {
            bits: 0b0000000110100,
            len: 13,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 12, y: 8 },
        code: HuffmanCode {
            bits: 0b00000000110000,
            len: 14,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 12, y: 9 },
        code: HuffmanCode {
            bits: 0b00000000101000,
            len: 14,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 12, y: 10 },
        code: HuffmanCode {
            bits: 0b000000000110100,
            len: 15,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 12, y: 11 },
        code: HuffmanCode {
            bits: 0b000000000011100,
            len: 15,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 12, y: 12 },
        code: HuffmanCode {
            bits: 0b000000000010010,
            len: 15,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 12, y: 13 },
        code: HuffmanCode {
            bits: 0b0000000000010001,
            len: 16,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 12, y: 14 },
        code: HuffmanCode {
            bits: 0b0000000000001001,
            len: 16,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 12, y: 15 },
        code: HuffmanCode {
            bits: 0b0000000000000101,
            len: 16,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 13, y: 0 },
        code: HuffmanCode {
            bits: 0b000000101101,
            len: 12,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 13, y: 1 },
        code: HuffmanCode {
            bits: 0b00000010101,
            len: 11,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 13, y: 2 },
        code: HuffmanCode {
            bits: 0b000000100010,
            len: 12,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 13, y: 3 },
        code: HuffmanCode {
            bits: 0b0000001000000,
            len: 13,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 13, y: 4 },
        code: HuffmanCode {
            bits: 0b0000000111000,
            len: 13,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 13, y: 5 },
        code: HuffmanCode {
            bits: 0b0000000110010,
            len: 13,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 13, y: 6 },
        code: HuffmanCode {
            bits: 0b00000000110001,
            len: 14,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 13, y: 7 },
        code: HuffmanCode {
            bits: 0b00000000101101,
            len: 14,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 13, y: 8 },
        code: HuffmanCode {
            bits: 0b00000000011111,
            len: 14,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 13, y: 9 },
        code: HuffmanCode {
            bits: 0b00000000010011,
            len: 14,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 13, y: 10 },
        code: HuffmanCode {
            bits: 0b00000000001100,
            len: 14,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 13, y: 11 },
        code: HuffmanCode {
            bits: 0b000000000001111,
            len: 15,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 13, y: 12 },
        code: HuffmanCode {
            bits: 0b0000000000001010,
            len: 16,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 13, y: 13 },
        code: HuffmanCode {
            bits: 0b000000000000111,
            len: 15,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 13, y: 14 },
        code: HuffmanCode {
            bits: 0b0000000000000110,
            len: 16,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 13, y: 15 },
        code: HuffmanCode {
            bits: 0b0000000000000011,
            len: 16,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 14, y: 0 },
        code: HuffmanCode {
            bits: 0b0000000110000,
            len: 13,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 14, y: 1 },
        code: HuffmanCode {
            bits: 0b000000010111,
            len: 12,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 14, y: 2 },
        code: HuffmanCode {
            bits: 0b000000010100,
            len: 12,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 14, y: 3 },
        code: HuffmanCode {
            bits: 0b0000000100111,
            len: 13,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 14, y: 4 },
        code: HuffmanCode {
            bits: 0b0000000100100,
            len: 13,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 14, y: 5 },
        code: HuffmanCode {
            bits: 0b0000000100011,
            len: 13,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 14, y: 6 },
        code: HuffmanCode {
            bits: 0b000000000110101,
            len: 15,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 14, y: 7 },
        code: HuffmanCode {
            bits: 0b00000000010101,
            len: 14,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 14, y: 8 },
        code: HuffmanCode {
            bits: 0b00000000010000,
            len: 14,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 14, y: 9 },
        code: HuffmanCode {
            bits: 0b00000000000010111,
            len: 17,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 14, y: 10 },
        code: HuffmanCode {
            bits: 0b000000000001101,
            len: 15,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 14, y: 11 },
        code: HuffmanCode {
            bits: 0b000000000001010,
            len: 15,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 14, y: 12 },
        code: HuffmanCode {
            bits: 0b000000000000110,
            len: 15,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 14, y: 13 },
        code: HuffmanCode {
            bits: 0b00000000000000001,
            len: 17,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 14, y: 14 },
        code: HuffmanCode {
            bits: 0b0000000000000100,
            len: 16,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 14, y: 15 },
        code: HuffmanCode {
            bits: 0b0000000000000010,
            len: 16,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 15, y: 0 },
        code: HuffmanCode {
            bits: 0b000000010000,
            len: 12,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 15, y: 1 },
        code: HuffmanCode {
            bits: 0b000000001111,
            len: 12,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 15, y: 2 },
        code: HuffmanCode {
            bits: 0b0000000010001,
            len: 13,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 15, y: 3 },
        code: HuffmanCode {
            bits: 0b00000000011011,
            len: 14,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 15, y: 4 },
        code: HuffmanCode {
            bits: 0b00000000011001,
            len: 14,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 15, y: 5 },
        code: HuffmanCode {
            bits: 0b00000000010100,
            len: 14,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 15, y: 6 },
        code: HuffmanCode {
            bits: 0b000000000011101,
            len: 15,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 15, y: 7 },
        code: HuffmanCode {
            bits: 0b00000000001011,
            len: 14,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 15, y: 8 },
        code: HuffmanCode {
            bits: 0b000000000010001,
            len: 15,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 15, y: 9 },
        code: HuffmanCode {
            bits: 0b000000000001100,
            len: 15,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 15, y: 10 },
        code: HuffmanCode {
            bits: 0b0000000000010000,
            len: 16,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 15, y: 11 },
        code: HuffmanCode {
            bits: 0b0000000000001000,
            len: 16,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 15, y: 12 },
        code: HuffmanCode {
            bits: 0b0000000000000000001,
            len: 19,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 15, y: 13 },
        code: HuffmanCode {
            bits: 0b000000000000000001,
            len: 18,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 15, y: 14 },
        code: HuffmanCode {
            bits: 0b0000000000000000000,
            len: 19,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 15, y: 15 },
        code: HuffmanCode {
            bits: 0b0000000000000001,
            len: 16,
        },
    },
];

const MPEG1_LAYER3_BIG_VALUE_TABLE_16: &[HuffmanEntry<Layer3BigValueMagnitude>] = &[
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 0, y: 0 },
        code: HuffmanCode { bits: 0b1, len: 1 },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 0, y: 1 },
        code: HuffmanCode {
            bits: 0b0101,
            len: 4,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 0, y: 2 },
        code: HuffmanCode {
            bits: 0b001110,
            len: 6,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 0, y: 3 },
        code: HuffmanCode {
            bits: 0b00101100,
            len: 8,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 0, y: 4 },
        code: HuffmanCode {
            bits: 0b001001010,
            len: 9,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 0, y: 5 },
        code: HuffmanCode {
            bits: 0b000111111,
            len: 9,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 0, y: 6 },
        code: HuffmanCode {
            bits: 0b0001101110,
            len: 10,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 0, y: 7 },
        code: HuffmanCode {
            bits: 0b0001011101,
            len: 10,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 0, y: 8 },
        code: HuffmanCode {
            bits: 0b00010101100,
            len: 11,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 0, y: 9 },
        code: HuffmanCode {
            bits: 0b00010010101,
            len: 11,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 0, y: 10 },
        code: HuffmanCode {
            bits: 0b00010001010,
            len: 11,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 0, y: 11 },
        code: HuffmanCode {
            bits: 0b000011110010,
            len: 12,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 0, y: 12 },
        code: HuffmanCode {
            bits: 0b000011100001,
            len: 12,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 0, y: 13 },
        code: HuffmanCode {
            bits: 0b000011000011,
            len: 12,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 0, y: 14 },
        code: HuffmanCode {
            bits: 0b0000101111000,
            len: 13,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 0, y: 15 },
        code: HuffmanCode {
            bits: 0b000010001,
            len: 9,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 1, y: 0 },
        code: HuffmanCode {
            bits: 0b011,
            len: 3,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 1, y: 1 },
        code: HuffmanCode {
            bits: 0b0100,
            len: 4,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 1, y: 2 },
        code: HuffmanCode {
            bits: 0b001100,
            len: 6,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 1, y: 3 },
        code: HuffmanCode {
            bits: 0b0010100,
            len: 7,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 1, y: 4 },
        code: HuffmanCode {
            bits: 0b00100011,
            len: 8,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 1, y: 5 },
        code: HuffmanCode {
            bits: 0b000111110,
            len: 9,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 1, y: 6 },
        code: HuffmanCode {
            bits: 0b000110101,
            len: 9,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 1, y: 7 },
        code: HuffmanCode {
            bits: 0b000101111,
            len: 9,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 1, y: 8 },
        code: HuffmanCode {
            bits: 0b0001010011,
            len: 10,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 1, y: 9 },
        code: HuffmanCode {
            bits: 0b0001001011,
            len: 10,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 1, y: 10 },
        code: HuffmanCode {
            bits: 0b0001000100,
            len: 10,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 1, y: 11 },
        code: HuffmanCode {
            bits: 0b00001110111,
            len: 11,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 1, y: 12 },
        code: HuffmanCode {
            bits: 0b000011001001,
            len: 12,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 1, y: 13 },
        code: HuffmanCode {
            bits: 0b00001101011,
            len: 11,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 1, y: 14 },
        code: HuffmanCode {
            bits: 0b000011001111,
            len: 12,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 1, y: 15 },
        code: HuffmanCode {
            bits: 0b00001001,
            len: 8,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 2, y: 0 },
        code: HuffmanCode {
            bits: 0b001111,
            len: 6,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 2, y: 1 },
        code: HuffmanCode {
            bits: 0b001101,
            len: 6,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 2, y: 2 },
        code: HuffmanCode {
            bits: 0b0010111,
            len: 7,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 2, y: 3 },
        code: HuffmanCode {
            bits: 0b00100110,
            len: 8,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 2, y: 4 },
        code: HuffmanCode {
            bits: 0b001000011,
            len: 9,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 2, y: 5 },
        code: HuffmanCode {
            bits: 0b000111010,
            len: 9,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 2, y: 6 },
        code: HuffmanCode {
            bits: 0b0001100111,
            len: 10,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 2, y: 7 },
        code: HuffmanCode {
            bits: 0b0001011010,
            len: 10,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 2, y: 8 },
        code: HuffmanCode {
            bits: 0b00010100001,
            len: 11,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 2, y: 9 },
        code: HuffmanCode {
            bits: 0b0001001000,
            len: 10,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 2, y: 10 },
        code: HuffmanCode {
            bits: 0b00001111111,
            len: 11,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 2, y: 11 },
        code: HuffmanCode {
            bits: 0b00001110101,
            len: 11,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 2, y: 12 },
        code: HuffmanCode {
            bits: 0b00001101110,
            len: 11,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 2, y: 13 },
        code: HuffmanCode {
            bits: 0b000011010001,
            len: 12,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 2, y: 14 },
        code: HuffmanCode {
            bits: 0b000011001110,
            len: 12,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 2, y: 15 },
        code: HuffmanCode {
            bits: 0b000010000,
            len: 9,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 3, y: 0 },
        code: HuffmanCode {
            bits: 0b00101101,
            len: 8,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 3, y: 1 },
        code: HuffmanCode {
            bits: 0b0010101,
            len: 7,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 3, y: 2 },
        code: HuffmanCode {
            bits: 0b00100111,
            len: 8,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 3, y: 3 },
        code: HuffmanCode {
            bits: 0b001000101,
            len: 9,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 3, y: 4 },
        code: HuffmanCode {
            bits: 0b001000000,
            len: 9,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 3, y: 5 },
        code: HuffmanCode {
            bits: 0b0001110010,
            len: 10,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 3, y: 6 },
        code: HuffmanCode {
            bits: 0b0001100011,
            len: 10,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 3, y: 7 },
        code: HuffmanCode {
            bits: 0b0001010111,
            len: 10,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 3, y: 8 },
        code: HuffmanCode {
            bits: 0b00010011110,
            len: 11,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 3, y: 9 },
        code: HuffmanCode {
            bits: 0b00010001100,
            len: 11,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 3, y: 10 },
        code: HuffmanCode {
            bits: 0b000011111100,
            len: 12,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 3, y: 11 },
        code: HuffmanCode {
            bits: 0b000011010100,
            len: 12,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 3, y: 12 },
        code: HuffmanCode {
            bits: 0b000011000111,
            len: 12,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 3, y: 13 },
        code: HuffmanCode {
            bits: 0b0000110000011,
            len: 13,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 3, y: 14 },
        code: HuffmanCode {
            bits: 0b0000101101101,
            len: 13,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 3, y: 15 },
        code: HuffmanCode {
            bits: 0b0000011010,
            len: 10,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 4, y: 0 },
        code: HuffmanCode {
            bits: 0b001001011,
            len: 9,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 4, y: 1 },
        code: HuffmanCode {
            bits: 0b00100100,
            len: 8,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 4, y: 2 },
        code: HuffmanCode {
            bits: 0b001000100,
            len: 9,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 4, y: 3 },
        code: HuffmanCode {
            bits: 0b001000001,
            len: 9,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 4, y: 4 },
        code: HuffmanCode {
            bits: 0b0001110011,
            len: 10,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 4, y: 5 },
        code: HuffmanCode {
            bits: 0b0001100101,
            len: 10,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 4, y: 6 },
        code: HuffmanCode {
            bits: 0b00010110011,
            len: 11,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 4, y: 7 },
        code: HuffmanCode {
            bits: 0b00010100100,
            len: 11,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 4, y: 8 },
        code: HuffmanCode {
            bits: 0b00010011011,
            len: 11,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 4, y: 9 },
        code: HuffmanCode {
            bits: 0b000100001000,
            len: 12,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 4, y: 10 },
        code: HuffmanCode {
            bits: 0b000011110110,
            len: 12,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 4, y: 11 },
        code: HuffmanCode {
            bits: 0b000011100010,
            len: 12,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 4, y: 12 },
        code: HuffmanCode {
            bits: 0b0000110001011,
            len: 13,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 4, y: 13 },
        code: HuffmanCode {
            bits: 0b0000101111110,
            len: 13,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 4, y: 14 },
        code: HuffmanCode {
            bits: 0b0000101101010,
            len: 13,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 4, y: 15 },
        code: HuffmanCode {
            bits: 0b000001001,
            len: 9,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 5, y: 0 },
        code: HuffmanCode {
            bits: 0b001000010,
            len: 9,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 5, y: 1 },
        code: HuffmanCode {
            bits: 0b00011110,
            len: 8,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 5, y: 2 },
        code: HuffmanCode {
            bits: 0b000111011,
            len: 9,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 5, y: 3 },
        code: HuffmanCode {
            bits: 0b000111000,
            len: 9,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 5, y: 4 },
        code: HuffmanCode {
            bits: 0b0001100110,
            len: 10,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 5, y: 5 },
        code: HuffmanCode {
            bits: 0b00010111001,
            len: 11,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 5, y: 6 },
        code: HuffmanCode {
            bits: 0b00010101101,
            len: 11,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 5, y: 7 },
        code: HuffmanCode {
            bits: 0b000100001001,
            len: 12,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 5, y: 8 },
        code: HuffmanCode {
            bits: 0b00010001110,
            len: 11,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 5, y: 9 },
        code: HuffmanCode {
            bits: 0b000011111101,
            len: 12,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 5, y: 10 },
        code: HuffmanCode {
            bits: 0b000011101000,
            len: 12,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 5, y: 11 },
        code: HuffmanCode {
            bits: 0b0000110010000,
            len: 13,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 5, y: 12 },
        code: HuffmanCode {
            bits: 0b0000110000100,
            len: 13,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 5, y: 13 },
        code: HuffmanCode {
            bits: 0b0000101111010,
            len: 13,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 5, y: 14 },
        code: HuffmanCode {
            bits: 0b00000110111101,
            len: 14,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 5, y: 15 },
        code: HuffmanCode {
            bits: 0b0000010000,
            len: 10,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 6, y: 0 },
        code: HuffmanCode {
            bits: 0b0001101111,
            len: 10,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 6, y: 1 },
        code: HuffmanCode {
            bits: 0b000110110,
            len: 9,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 6, y: 2 },
        code: HuffmanCode {
            bits: 0b000110100,
            len: 9,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 6, y: 3 },
        code: HuffmanCode {
            bits: 0b0001100100,
            len: 10,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 6, y: 4 },
        code: HuffmanCode {
            bits: 0b00010111000,
            len: 11,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 6, y: 5 },
        code: HuffmanCode {
            bits: 0b00010110010,
            len: 11,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 6, y: 6 },
        code: HuffmanCode {
            bits: 0b00010100000,
            len: 11,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 6, y: 7 },
        code: HuffmanCode {
            bits: 0b00010000101,
            len: 11,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 6, y: 8 },
        code: HuffmanCode {
            bits: 0b000100000001,
            len: 12,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 6, y: 9 },
        code: HuffmanCode {
            bits: 0b000011110100,
            len: 12,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 6, y: 10 },
        code: HuffmanCode {
            bits: 0b000011100100,
            len: 12,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 6, y: 11 },
        code: HuffmanCode {
            bits: 0b000011011001,
            len: 12,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 6, y: 12 },
        code: HuffmanCode {
            bits: 0b0000110000001,
            len: 13,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 6, y: 13 },
        code: HuffmanCode {
            bits: 0b0000101101110,
            len: 13,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 6, y: 14 },
        code: HuffmanCode {
            bits: 0b00001011001011,
            len: 14,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 6, y: 15 },
        code: HuffmanCode {
            bits: 0b0000001010,
            len: 10,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 7, y: 0 },
        code: HuffmanCode {
            bits: 0b0001100010,
            len: 10,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 7, y: 1 },
        code: HuffmanCode {
            bits: 0b000110000,
            len: 9,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 7, y: 2 },
        code: HuffmanCode {
            bits: 0b0001011011,
            len: 10,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 7, y: 3 },
        code: HuffmanCode {
            bits: 0b0001011000,
            len: 10,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 7, y: 4 },
        code: HuffmanCode {
            bits: 0b00010100101,
            len: 11,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 7, y: 5 },
        code: HuffmanCode {
            bits: 0b00010011101,
            len: 11,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 7, y: 6 },
        code: HuffmanCode {
            bits: 0b00010010100,
            len: 11,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 7, y: 7 },
        code: HuffmanCode {
            bits: 0b000100000101,
            len: 12,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 7, y: 8 },
        code: HuffmanCode {
            bits: 0b000011111000,
            len: 12,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 7, y: 9 },
        code: HuffmanCode {
            bits: 0b0000110010111,
            len: 13,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 7, y: 10 },
        code: HuffmanCode {
            bits: 0b0000110001101,
            len: 13,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 7, y: 11 },
        code: HuffmanCode {
            bits: 0b0000101110100,
            len: 13,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 7, y: 12 },
        code: HuffmanCode {
            bits: 0b0000101111100,
            len: 13,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 7, y: 13 },
        code: HuffmanCode {
            bits: 0b000001101111001,
            len: 15,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 7, y: 14 },
        code: HuffmanCode {
            bits: 0b000001101110100,
            len: 15,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 7, y: 15 },
        code: HuffmanCode {
            bits: 0b0000001000,
            len: 10,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 8, y: 0 },
        code: HuffmanCode {
            bits: 0b0001010101,
            len: 10,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 8, y: 1 },
        code: HuffmanCode {
            bits: 0b0001010100,
            len: 10,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 8, y: 2 },
        code: HuffmanCode {
            bits: 0b0001010001,
            len: 10,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 8, y: 3 },
        code: HuffmanCode {
            bits: 0b00010011111,
            len: 11,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 8, y: 4 },
        code: HuffmanCode {
            bits: 0b00010011100,
            len: 11,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 8, y: 5 },
        code: HuffmanCode {
            bits: 0b00010001111,
            len: 11,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 8, y: 6 },
        code: HuffmanCode {
            bits: 0b000100000100,
            len: 12,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 8, y: 7 },
        code: HuffmanCode {
            bits: 0b000011111001,
            len: 12,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 8, y: 8 },
        code: HuffmanCode {
            bits: 0b0000110101011,
            len: 13,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 8, y: 9 },
        code: HuffmanCode {
            bits: 0b0000110010001,
            len: 13,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 8, y: 10 },
        code: HuffmanCode {
            bits: 0b0000110001000,
            len: 13,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 8, y: 11 },
        code: HuffmanCode {
            bits: 0b0000101111111,
            len: 13,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 8, y: 12 },
        code: HuffmanCode {
            bits: 0b00001011010111,
            len: 14,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 8, y: 13 },
        code: HuffmanCode {
            bits: 0b00001011001001,
            len: 14,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 8, y: 14 },
        code: HuffmanCode {
            bits: 0b00001011000100,
            len: 14,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 8, y: 15 },
        code: HuffmanCode {
            bits: 0b0000000111,
            len: 10,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 9, y: 0 },
        code: HuffmanCode {
            bits: 0b00010011010,
            len: 11,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 9, y: 1 },
        code: HuffmanCode {
            bits: 0b0001001100,
            len: 10,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 9, y: 2 },
        code: HuffmanCode {
            bits: 0b0001001001,
            len: 10,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 9, y: 3 },
        code: HuffmanCode {
            bits: 0b00010001101,
            len: 11,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 9, y: 4 },
        code: HuffmanCode {
            bits: 0b00010000011,
            len: 11,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 9, y: 5 },
        code: HuffmanCode {
            bits: 0b000100000000,
            len: 12,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 9, y: 6 },
        code: HuffmanCode {
            bits: 0b000011110101,
            len: 12,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 9, y: 7 },
        code: HuffmanCode {
            bits: 0b0000110101010,
            len: 13,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 9, y: 8 },
        code: HuffmanCode {
            bits: 0b0000110010110,
            len: 13,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 9, y: 9 },
        code: HuffmanCode {
            bits: 0b0000110001010,
            len: 13,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 9, y: 10 },
        code: HuffmanCode {
            bits: 0b0000110000000,
            len: 13,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 9, y: 11 },
        code: HuffmanCode {
            bits: 0b00001011011111,
            len: 14,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 9, y: 12 },
        code: HuffmanCode {
            bits: 0b0000101100111,
            len: 13,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 9, y: 13 },
        code: HuffmanCode {
            bits: 0b00001011000110,
            len: 14,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 9, y: 14 },
        code: HuffmanCode {
            bits: 0b0000101100000,
            len: 13,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 9, y: 15 },
        code: HuffmanCode {
            bits: 0b00000001011,
            len: 11,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 10, y: 0 },
        code: HuffmanCode {
            bits: 0b00010001011,
            len: 11,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 10, y: 1 },
        code: HuffmanCode {
            bits: 0b00010000001,
            len: 11,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 10, y: 2 },
        code: HuffmanCode {
            bits: 0b0001000011,
            len: 10,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 10, y: 3 },
        code: HuffmanCode {
            bits: 0b00001111101,
            len: 11,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 10, y: 4 },
        code: HuffmanCode {
            bits: 0b000011110111,
            len: 12,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 10, y: 5 },
        code: HuffmanCode {
            bits: 0b000011101001,
            len: 12,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 10, y: 6 },
        code: HuffmanCode {
            bits: 0b000011100101,
            len: 12,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 10, y: 7 },
        code: HuffmanCode {
            bits: 0b000011011011,
            len: 12,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 10, y: 8 },
        code: HuffmanCode {
            bits: 0b0000110001001,
            len: 13,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 10, y: 9 },
        code: HuffmanCode {
            bits: 0b00001011100111,
            len: 14,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 10, y: 10 },
        code: HuffmanCode {
            bits: 0b00001011100001,
            len: 14,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 10, y: 11 },
        code: HuffmanCode {
            bits: 0b00001011010000,
            len: 14,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 10, y: 12 },
        code: HuffmanCode {
            bits: 0b000001101110101,
            len: 15,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 10, y: 13 },
        code: HuffmanCode {
            bits: 0b000001101110010,
            len: 15,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 10, y: 14 },
        code: HuffmanCode {
            bits: 0b00000110110111,
            len: 14,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 10, y: 15 },
        code: HuffmanCode {
            bits: 0b0000000100,
            len: 10,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 11, y: 0 },
        code: HuffmanCode {
            bits: 0b000011110011,
            len: 12,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 11, y: 1 },
        code: HuffmanCode {
            bits: 0b00001111000,
            len: 11,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 11, y: 2 },
        code: HuffmanCode {
            bits: 0b00001110110,
            len: 11,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 11, y: 3 },
        code: HuffmanCode {
            bits: 0b00001110011,
            len: 11,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 11, y: 4 },
        code: HuffmanCode {
            bits: 0b000011100011,
            len: 12,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 11, y: 5 },
        code: HuffmanCode {
            bits: 0b000011011111,
            len: 12,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 11, y: 6 },
        code: HuffmanCode {
            bits: 0b0000110001100,
            len: 13,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 11, y: 7 },
        code: HuffmanCode {
            bits: 0b00001011101010,
            len: 14,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 11, y: 8 },
        code: HuffmanCode {
            bits: 0b00001011100110,
            len: 14,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 11, y: 9 },
        code: HuffmanCode {
            bits: 0b00001011100000,
            len: 14,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 11, y: 10 },
        code: HuffmanCode {
            bits: 0b00001011010001,
            len: 14,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 11, y: 11 },
        code: HuffmanCode {
            bits: 0b00001011001000,
            len: 14,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 11, y: 12 },
        code: HuffmanCode {
            bits: 0b00001011000010,
            len: 14,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 11, y: 13 },
        code: HuffmanCode {
            bits: 0b0000011011111,
            len: 13,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 11, y: 14 },
        code: HuffmanCode {
            bits: 0b00000110110100,
            len: 14,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 11, y: 15 },
        code: HuffmanCode {
            bits: 0b00000000110,
            len: 11,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 12, y: 0 },
        code: HuffmanCode {
            bits: 0b000011001010,
            len: 12,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 12, y: 1 },
        code: HuffmanCode {
            bits: 0b000011100000,
            len: 12,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 12, y: 2 },
        code: HuffmanCode {
            bits: 0b000011011110,
            len: 12,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 12, y: 3 },
        code: HuffmanCode {
            bits: 0b000011011010,
            len: 12,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 12, y: 4 },
        code: HuffmanCode {
            bits: 0b000011011000,
            len: 12,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 12, y: 5 },
        code: HuffmanCode {
            bits: 0b0000110000101,
            len: 13,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 12, y: 6 },
        code: HuffmanCode {
            bits: 0b0000110000010,
            len: 13,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 12, y: 7 },
        code: HuffmanCode {
            bits: 0b0000101111101,
            len: 13,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 12, y: 8 },
        code: HuffmanCode {
            bits: 0b0000101101100,
            len: 13,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 12, y: 9 },
        code: HuffmanCode {
            bits: 0b000001101111000,
            len: 15,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 12, y: 10 },
        code: HuffmanCode {
            bits: 0b00000110111011,
            len: 14,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 12, y: 11 },
        code: HuffmanCode {
            bits: 0b00001011000011,
            len: 14,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 12, y: 12 },
        code: HuffmanCode {
            bits: 0b00000110111000,
            len: 14,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 12, y: 13 },
        code: HuffmanCode {
            bits: 0b00000110110101,
            len: 14,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 12, y: 14 },
        code: HuffmanCode {
            bits: 0b0000011011000000,
            len: 16,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 12, y: 15 },
        code: HuffmanCode {
            bits: 0b00000000100,
            len: 11,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 13, y: 0 },
        code: HuffmanCode {
            bits: 0b00001011101011,
            len: 14,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 13, y: 1 },
        code: HuffmanCode {
            bits: 0b000011010011,
            len: 12,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 13, y: 2 },
        code: HuffmanCode {
            bits: 0b000011010010,
            len: 12,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 13, y: 3 },
        code: HuffmanCode {
            bits: 0b000011010000,
            len: 12,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 13, y: 4 },
        code: HuffmanCode {
            bits: 0b0000101110010,
            len: 13,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 13, y: 5 },
        code: HuffmanCode {
            bits: 0b0000101111011,
            len: 13,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 13, y: 6 },
        code: HuffmanCode {
            bits: 0b00001011011110,
            len: 14,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 13, y: 7 },
        code: HuffmanCode {
            bits: 0b00001011010011,
            len: 14,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 13, y: 8 },
        code: HuffmanCode {
            bits: 0b00001011001010,
            len: 14,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 13, y: 9 },
        code: HuffmanCode {
            bits: 0b0000011011000111,
            len: 16,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 13, y: 10 },
        code: HuffmanCode {
            bits: 0b000001101110011,
            len: 15,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 13, y: 11 },
        code: HuffmanCode {
            bits: 0b000001101101101,
            len: 15,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 13, y: 12 },
        code: HuffmanCode {
            bits: 0b000001101101100,
            len: 15,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 13, y: 13 },
        code: HuffmanCode {
            bits: 0b00000110110000011,
            len: 17,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 13, y: 14 },
        code: HuffmanCode {
            bits: 0b000001101100001,
            len: 15,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 13, y: 15 },
        code: HuffmanCode {
            bits: 0b00000000010,
            len: 11,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 14, y: 0 },
        code: HuffmanCode {
            bits: 0b0000101111001,
            len: 13,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 14, y: 1 },
        code: HuffmanCode {
            bits: 0b0000101110001,
            len: 13,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 14, y: 2 },
        code: HuffmanCode {
            bits: 0b00001100110,
            len: 11,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 14, y: 3 },
        code: HuffmanCode {
            bits: 0b000010111011,
            len: 12,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 14, y: 4 },
        code: HuffmanCode {
            bits: 0b00001011010110,
            len: 14,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 14, y: 5 },
        code: HuffmanCode {
            bits: 0b00001011010010,
            len: 14,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 14, y: 6 },
        code: HuffmanCode {
            bits: 0b0000101100110,
            len: 13,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 14, y: 7 },
        code: HuffmanCode {
            bits: 0b00001011000111,
            len: 14,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 14, y: 8 },
        code: HuffmanCode {
            bits: 0b00001011000101,
            len: 14,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 14, y: 9 },
        code: HuffmanCode {
            bits: 0b000001101100010,
            len: 15,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 14, y: 10 },
        code: HuffmanCode {
            bits: 0b0000011011000110,
            len: 16,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 14, y: 11 },
        code: HuffmanCode {
            bits: 0b000001101100111,
            len: 15,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 14, y: 12 },
        code: HuffmanCode {
            bits: 0b00000110110000010,
            len: 17,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 14, y: 13 },
        code: HuffmanCode {
            bits: 0b000001101100110,
            len: 15,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 14, y: 14 },
        code: HuffmanCode {
            bits: 0b00000110110010,
            len: 14,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 14, y: 15 },
        code: HuffmanCode {
            bits: 0b00000000000,
            len: 11,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 15, y: 0 },
        code: HuffmanCode {
            bits: 0b000001100,
            len: 9,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 15, y: 1 },
        code: HuffmanCode {
            bits: 0b00001010,
            len: 8,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 15, y: 2 },
        code: HuffmanCode {
            bits: 0b00000111,
            len: 8,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 15, y: 3 },
        code: HuffmanCode {
            bits: 0b000001011,
            len: 9,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 15, y: 4 },
        code: HuffmanCode {
            bits: 0b000001010,
            len: 9,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 15, y: 5 },
        code: HuffmanCode {
            bits: 0b0000010001,
            len: 10,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 15, y: 6 },
        code: HuffmanCode {
            bits: 0b0000001011,
            len: 10,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 15, y: 7 },
        code: HuffmanCode {
            bits: 0b0000001001,
            len: 10,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 15, y: 8 },
        code: HuffmanCode {
            bits: 0b00000001101,
            len: 11,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 15, y: 9 },
        code: HuffmanCode {
            bits: 0b00000001100,
            len: 11,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 15, y: 10 },
        code: HuffmanCode {
            bits: 0b00000001010,
            len: 11,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 15, y: 11 },
        code: HuffmanCode {
            bits: 0b00000000111,
            len: 11,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 15, y: 12 },
        code: HuffmanCode {
            bits: 0b00000000101,
            len: 11,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 15, y: 13 },
        code: HuffmanCode {
            bits: 0b00000000011,
            len: 11,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 15, y: 14 },
        code: HuffmanCode {
            bits: 0b00000000001,
            len: 11,
        },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 15, y: 15 },
        code: HuffmanCode {
            bits: 0b00000011,
            len: 8,
        },
    },
];

/// Returns the implemented MPEG-1 Layer III standard Huffman tables.
///
/// This provider currently exposes big-values tables 1/2/5/7/10/13 plus the
/// table-16 codeword tree used by escape-class tables 16..=23, and count1
/// tables 32/33. The remaining standard big-values tables are filled in
/// incrementally as the clean-room encoder grows.
#[must_use]
pub fn mpeg1_layer3_standard_table_provider() -> Layer3EntropyTableProvider<'static> {
    Layer3EntropyTableProvider {
        big_value_table_1: MPEG1_LAYER3_BIG_VALUE_TABLE_1,
        big_value_table_2: MPEG1_LAYER3_BIG_VALUE_TABLE_2,
        big_value_table_5: MPEG1_LAYER3_BIG_VALUE_TABLE_5,
        big_value_table_7: MPEG1_LAYER3_BIG_VALUE_TABLE_7,
        big_value_table_10: MPEG1_LAYER3_BIG_VALUE_TABLE_10,
        big_value_table_13: MPEG1_LAYER3_BIG_VALUE_TABLE_13,
        big_value_table_16: MPEG1_LAYER3_BIG_VALUE_TABLE_16,
        count1_table_0: MPEG1_LAYER3_COUNT1_TABLE_32,
        count1_table_1: MPEG1_LAYER3_COUNT1_TABLE_33,
    }
}

/// Returns the implemented MPEG-1 Layer III standard Huffman tables.
///
/// Kept for compatibility with earlier scaffold helpers. Prefer
/// [`mpeg1_layer3_standard_table_provider`] now that count1 tables are included.
#[must_use]
pub fn mpeg1_layer3_standard_big_value_table_provider() -> Layer3EntropyTableProvider<'static> {
    mpeg1_layer3_standard_table_provider()
}

const EXPERIMENTAL_COUNT1_TABLE_0: &[HuffmanEntry<Layer3Count1MagnitudeQuad>] = &[
    HuffmanEntry {
        symbol: Layer3Count1MagnitudeQuad {
            v: 0,
            w: 0,
            x: 0,
            y: 0,
        },
        code: HuffmanCode {
            bits: 0b0000,
            len: 4,
        },
    },
    HuffmanEntry {
        symbol: Layer3Count1MagnitudeQuad {
            v: 0,
            w: 0,
            x: 0,
            y: 1,
        },
        code: HuffmanCode {
            bits: 0b0001,
            len: 4,
        },
    },
    HuffmanEntry {
        symbol: Layer3Count1MagnitudeQuad {
            v: 0,
            w: 0,
            x: 1,
            y: 0,
        },
        code: HuffmanCode {
            bits: 0b0010,
            len: 4,
        },
    },
    HuffmanEntry {
        symbol: Layer3Count1MagnitudeQuad {
            v: 0,
            w: 0,
            x: 1,
            y: 1,
        },
        code: HuffmanCode {
            bits: 0b0011,
            len: 4,
        },
    },
    HuffmanEntry {
        symbol: Layer3Count1MagnitudeQuad {
            v: 0,
            w: 1,
            x: 0,
            y: 0,
        },
        code: HuffmanCode {
            bits: 0b0100,
            len: 4,
        },
    },
    HuffmanEntry {
        symbol: Layer3Count1MagnitudeQuad {
            v: 0,
            w: 1,
            x: 0,
            y: 1,
        },
        code: HuffmanCode {
            bits: 0b0101,
            len: 4,
        },
    },
    HuffmanEntry {
        symbol: Layer3Count1MagnitudeQuad {
            v: 0,
            w: 1,
            x: 1,
            y: 0,
        },
        code: HuffmanCode {
            bits: 0b0110,
            len: 4,
        },
    },
    HuffmanEntry {
        symbol: Layer3Count1MagnitudeQuad {
            v: 0,
            w: 1,
            x: 1,
            y: 1,
        },
        code: HuffmanCode {
            bits: 0b0111,
            len: 4,
        },
    },
    HuffmanEntry {
        symbol: Layer3Count1MagnitudeQuad {
            v: 1,
            w: 0,
            x: 0,
            y: 0,
        },
        code: HuffmanCode {
            bits: 0b1000,
            len: 4,
        },
    },
    HuffmanEntry {
        symbol: Layer3Count1MagnitudeQuad {
            v: 1,
            w: 0,
            x: 0,
            y: 1,
        },
        code: HuffmanCode {
            bits: 0b1001,
            len: 4,
        },
    },
    HuffmanEntry {
        symbol: Layer3Count1MagnitudeQuad {
            v: 1,
            w: 0,
            x: 1,
            y: 0,
        },
        code: HuffmanCode {
            bits: 0b1010,
            len: 4,
        },
    },
    HuffmanEntry {
        symbol: Layer3Count1MagnitudeQuad {
            v: 1,
            w: 0,
            x: 1,
            y: 1,
        },
        code: HuffmanCode {
            bits: 0b1011,
            len: 4,
        },
    },
    HuffmanEntry {
        symbol: Layer3Count1MagnitudeQuad {
            v: 1,
            w: 1,
            x: 0,
            y: 0,
        },
        code: HuffmanCode {
            bits: 0b1100,
            len: 4,
        },
    },
    HuffmanEntry {
        symbol: Layer3Count1MagnitudeQuad {
            v: 1,
            w: 1,
            x: 0,
            y: 1,
        },
        code: HuffmanCode {
            bits: 0b1101,
            len: 4,
        },
    },
    HuffmanEntry {
        symbol: Layer3Count1MagnitudeQuad {
            v: 1,
            w: 1,
            x: 1,
            y: 0,
        },
        code: HuffmanCode {
            bits: 0b1110,
            len: 4,
        },
    },
    HuffmanEntry {
        symbol: Layer3Count1MagnitudeQuad {
            v: 1,
            w: 1,
            x: 1,
            y: 1,
        },
        code: HuffmanCode {
            bits: 0b1111,
            len: 4,
        },
    },
];

/// Returns a minimal experimental provider that can encode zero/one magnitudes.
///
/// This is not the MPEG Layer III standard Huffman table set. It exists to
/// exercise the non-zero payload plumbing while the complete clean-room tables
/// and rate control are being implemented.
#[must_use]
pub fn experimental_unit_magnitude_table_provider() -> Layer3EntropyTableProvider<'static> {
    Layer3EntropyTableProvider {
        big_value_table_1: MPEG1_LAYER3_BIG_VALUE_TABLE_1,
        count1_table_0: EXPERIMENTAL_COUNT1_TABLE_0,
        ..Default::default()
    }
}

/// Converts the Layer III big-values region into pair symbols.
pub fn big_value_pairs(
    quantized: &[i32],
    regions: Layer3SpectralRegions,
) -> Result<Vec<Layer3BigValuePair>, Error> {
    let coeff_count = usize::from(regions.big_values)
        .checked_mul(2)
        .ok_or(Error::InvalidInput("MP3 big-values region is too large"))?;
    if coeff_count > quantized.len() {
        return Err(Error::InvalidInput(
            "MP3 big-values region exceeds spectrum length",
        ));
    }

    quantized[..coeff_count]
        .chunks_exact(2)
        .map(|pair| {
            Ok(Layer3BigValuePair::new(
                i16::try_from(pair[0]).map_err(|_| {
                    Error::InvalidInput("MP3 big-value coefficient exceeds i16 range")
                })?,
                i16::try_from(pair[1]).map_err(|_| {
                    Error::InvalidInput("MP3 big-value coefficient exceeds i16 range")
                })?,
            ))
        })
        .collect()
}

/// Maps an escape-class magnitude to the ISO `table_select` (16..=23) whose
/// fixed `linbits` covers it.
///
/// Big-values tables 16 through 23 share the table-16 Huffman codeword tree and
/// differ only in their fixed `linbits` widths (1, 2, 3, 4, 6, 8, 10, 13 in
/// ISO/IEC 11172-3 Annex B). The decoder derives `linbits` from `table_select`,
/// so the encoder must emit the table whose fixed width matches — not a free
/// `linbits` paired with `table_select` 16. This picks the smallest such table
/// that still represents `max_magnitude`.
fn escape_table_select_for_magnitude(max_magnitude: u16) -> Result<(u8, u8), Error> {
    const ESCAPE_TABLES: [(u8, u8); 8] = [
        (16, 1),
        (17, 2),
        (18, 3),
        (19, 4),
        (20, 6),
        (21, 8),
        (22, 10),
        (23, 13),
    ];
    let required = linbits_for_big_value_magnitude(max_magnitude)?;
    ESCAPE_TABLES
        .into_iter()
        .find(|&(_, linbits)| linbits >= required)
        .ok_or(Error::InvalidInput(
            "MP3 big-values magnitude exceeds table range",
        ))
}

/// Selects the smallest implemented Layer III big-values table class.
pub fn select_big_value_table(
    pairs: &[Layer3BigValuePair],
) -> Result<Layer3BigValueTableSelection, Error> {
    let max_magnitude = max_big_value_magnitude(pairs)?;

    let (table_select, linbits) = match max_magnitude {
        0 => (0, 0),
        1 => (1, 0),
        2..=3 => (5, 0),
        4..=5 => (7, 0),
        6..=7 => (10, 0),
        8..=15 => (13, 0),
        _ => escape_table_select_for_magnitude(max_magnitude)?,
    };

    Ok(Layer3BigValueTableSelection {
        table_select,
        linbits,
        max_magnitude,
    })
}

/// Selects the shortest available Layer III big-values table from a provider.
pub fn select_big_value_table_by_bit_cost(
    pairs: &[Layer3BigValuePair],
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<Layer3BigValueTableSelection, Error> {
    let max_magnitude = max_big_value_magnitude(pairs)?;
    if max_magnitude == 0 {
        return Ok(Layer3BigValueTableSelection {
            table_select: 0,
            linbits: 0,
            max_magnitude,
        });
    }

    let (escape_table_select, escape_linbits) = escape_table_select_for_magnitude(max_magnitude)?;
    let candidates = [
        (1, 0, provider.big_value_table_1),
        (2, 0, provider.big_value_table_2),
        (5, 0, provider.big_value_table_5),
        (7, 0, provider.big_value_table_7),
        (10, 0, provider.big_value_table_10),
        (13, 0, provider.big_value_table_13),
        (
            escape_table_select,
            escape_linbits,
            provider.big_value_table_16,
        ),
    ];
    let mut best: Option<(Layer3BigValueTableSelection, usize)> = None;
    for (table_select, linbits, table) in candidates {
        if table.is_empty() {
            continue;
        }
        let selection = Layer3BigValueTableSelection {
            table_select,
            linbits,
            max_magnitude,
        };
        let Ok(packed) = pack_big_value_pairs_with_selection(pairs, table, selection) else {
            continue;
        };
        if best
            .as_ref()
            .is_none_or(|(_, bit_len)| packed.bit_len < *bit_len)
        {
            best = Some((selection, packed.bit_len));
        }
    }

    best.map(|(selection, _)| selection)
        .ok_or(Error::UnsupportedFeature("MP3 big-values Huffman table"))
}

/// Applies one big-values Huffman table selection to Layer III side info.
pub fn apply_big_value_table_to_granule(
    granule: &mut Layer3GranuleChannelInfo,
    selection: Layer3BigValueTableSelection,
) {
    let table = if granule.big_values == 0 {
        0
    } else {
        selection.table_select
    };
    granule.table_select = [table, table, table];
}

/// Selects Layer III big-values Huffman table classes independently per region.
pub fn select_big_value_region_tables(
    pairs: &[Layer3BigValuePair],
    region0_pairs: usize,
    region1_pairs: usize,
) -> Result<Layer3BigValueRegionTableSelection, Error> {
    let region1_start = region0_pairs;
    let region2_start = region1_start
        .checked_add(region1_pairs)
        .ok_or(Error::InvalidInput("MP3 big-values region is too large"))?;
    if region2_start > pairs.len() {
        return Err(Error::InvalidInput(
            "MP3 big-values region exceeds spectrum length",
        ));
    }

    let region0 = &pairs[..region1_start];
    let region1 = &pairs[region1_start..region2_start];
    let region2 = &pairs[region2_start..];

    Ok(Layer3BigValueRegionTableSelection {
        regions: [
            select_big_value_table(region0)?,
            select_big_value_table(region1)?,
            select_big_value_table(region2)?,
        ],
        region0_pairs: u16::try_from(region0.len())
            .map_err(|_| Error::InvalidInput("MP3 region0 count exceeds side-info range"))?,
        region1_pairs: u16::try_from(region1.len())
            .map_err(|_| Error::InvalidInput("MP3 region1 count exceeds side-info range"))?,
    })
}

/// Selects Layer III big-values Huffman tables independently per region by bit cost.
pub fn select_big_value_region_tables_by_bit_cost(
    pairs: &[Layer3BigValuePair],
    region0_pairs: usize,
    region1_pairs: usize,
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<Layer3BigValueRegionTableSelection, Error> {
    let region1_start = region0_pairs;
    let region2_start = region1_start
        .checked_add(region1_pairs)
        .ok_or(Error::InvalidInput("MP3 big-values region is too large"))?;
    if region2_start > pairs.len() {
        return Err(Error::InvalidInput(
            "MP3 big-values region exceeds spectrum length",
        ));
    }

    let region0 = &pairs[..region1_start];
    let region1 = &pairs[region1_start..region2_start];
    let region2 = &pairs[region2_start..];

    Ok(Layer3BigValueRegionTableSelection {
        regions: [
            select_big_value_table_by_bit_cost(region0, provider)?,
            select_big_value_table_by_bit_cost(region1, provider)?,
            select_big_value_table_by_bit_cost(region2, provider)?,
        ],
        region0_pairs: u16::try_from(region0.len())
            .map_err(|_| Error::InvalidInput("MP3 region0 count exceeds side-info range"))?,
        region1_pairs: u16::try_from(region1.len())
            .map_err(|_| Error::InvalidInput("MP3 region1 count exceeds side-info range"))?,
    })
}

/// Applies region-specific big-values Huffman selections to Layer III side info.
pub fn apply_big_value_region_tables_to_granule(
    granule: &mut Layer3GranuleChannelInfo,
    selection: Layer3BigValueRegionTableSelection,
) -> Result<(), Error> {
    granule.table_select = [
        selection.regions[0].table_select,
        selection.regions[1].table_select,
        selection.regions[2].table_select,
    ];
    // `region0_count`/`region1_count` are the scalefactor-band region addresses
    // (set by `apply_spectral_regions_to_granule`), not pair counts. The pair
    // split carried by `selection` drives the bit packing and must match the
    // decoder's sfb-derived boundaries for those addresses; it is not copied
    // back into the side-info fields here.
    Ok(())
}

/// Packs Layer III big-values regions with provider-selected Huffman tables.
pub fn pack_big_value_pairs_with_region_tables_and_provider(
    pairs: &[Layer3BigValuePair],
    selection: Layer3BigValueRegionTableSelection,
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<PackedBits, Error> {
    let region1_start = usize::from(selection.region0_pairs);
    let region2_start = region1_start
        .checked_add(usize::from(selection.region1_pairs))
        .ok_or(Error::InvalidInput("MP3 big-values region is too large"))?;
    if region2_start > pairs.len() {
        return Err(Error::InvalidInput(
            "MP3 big-values region exceeds spectrum length",
        ));
    }

    let region0 = pack_big_value_pairs_with_selection(
        &pairs[..region1_start],
        provider.big_value_table(selection.regions[0])?,
        selection.regions[0],
    )?;
    let region1 = pack_big_value_pairs_with_selection(
        &pairs[region1_start..region2_start],
        provider.big_value_table(selection.regions[1])?,
        selection.regions[1],
    )?;
    let region2 = pack_big_value_pairs_with_selection(
        &pairs[region2_start..],
        provider.big_value_table(selection.regions[2])?,
        selection.regions[2],
    )?;

    concat_packed_bits(&[region0, region1, region2])
}

/// Converts the Layer III count1 region into quadruple symbols.
pub fn count1_quads(
    quantized: &[i32],
    regions: Layer3SpectralRegions,
) -> Result<Vec<Layer3Count1Quad>, Error> {
    let start = usize::from(regions.big_values)
        .checked_mul(2)
        .ok_or(Error::InvalidInput("MP3 big-values region is too large"))?;
    let coeff_count = usize::from(regions.count1)
        .checked_mul(4)
        .ok_or(Error::InvalidInput("MP3 count1 region is too large"))?;
    let end = start
        .checked_add(coeff_count)
        .ok_or(Error::InvalidInput("MP3 count1 region is too large"))?;
    if end > quantized.len() {
        return Err(Error::InvalidInput(
            "MP3 count1 region exceeds spectrum length",
        ));
    }

    quantized[start..end]
        .chunks_exact(4)
        .map(|quad| {
            for &coeff in quad {
                if coeff.abs() > 1 {
                    return Err(Error::InvalidInput(
                        "MP3 count1 coefficient exceeds unit magnitude",
                    ));
                }
            }
            Ok(Layer3Count1Quad::new(
                i8::try_from(quad[0])
                    .map_err(|_| Error::InvalidInput("MP3 count1 coefficient exceeds i8 range"))?,
                i8::try_from(quad[1])
                    .map_err(|_| Error::InvalidInput("MP3 count1 coefficient exceeds i8 range"))?,
                i8::try_from(quad[2])
                    .map_err(|_| Error::InvalidInput("MP3 count1 coefficient exceeds i8 range"))?,
                i8::try_from(quad[3])
                    .map_err(|_| Error::InvalidInput("MP3 count1 coefficient exceeds i8 range"))?,
            ))
        })
        .collect()
}

/// Selects a conservative Layer III count1 table class.
pub fn select_count1_table(
    quads: &[Layer3Count1Quad],
) -> Result<Layer3Count1TableSelection, Error> {
    let max_nonzero_values = max_count1_nonzero_values(quads)?;

    Ok(Layer3Count1TableSelection {
        table_select: max_nonzero_values >= 3,
        max_nonzero_values,
    })
}

/// Selects the shortest available Layer III count1 table from a provider.
pub fn select_count1_table_by_bit_cost(
    quads: &[Layer3Count1Quad],
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<Layer3Count1TableSelection, Error> {
    let max_nonzero_values = max_count1_nonzero_values(quads)?;
    if max_nonzero_values == 0 {
        return Ok(Layer3Count1TableSelection {
            table_select: false,
            max_nonzero_values,
        });
    }

    let candidates = [
        (false, provider.count1_table_0),
        (true, provider.count1_table_1),
    ];
    let mut best: Option<(Layer3Count1TableSelection, usize)> = None;
    for (table_select, table) in candidates {
        if table.is_empty() {
            continue;
        }
        let selection = Layer3Count1TableSelection {
            table_select,
            max_nonzero_values,
        };
        let Ok(packed) = pack_count1_quads_with_table_selection(quads, table, selection) else {
            continue;
        };
        if best
            .as_ref()
            .is_none_or(|(_, bit_len)| packed.bit_len < *bit_len)
        {
            best = Some((selection, packed.bit_len));
        }
    }

    best.map(|(selection, _)| selection)
        .ok_or(Error::UnsupportedFeature("MP3 count1 Huffman table"))
}

/// Applies one count1 Huffman table selection to Layer III side info.
pub fn apply_count1_table_to_granule(
    granule: &mut Layer3GranuleChannelInfo,
    selection: Layer3Count1TableSelection,
) {
    granule.count1table_select = selection.table_select;
}

/// Builds one Layer III granule/channel entropy payload from quantized spectrum.
pub fn pack_quantized_spectrum_for_granule(
    granule: &mut Layer3GranuleChannelInfo,
    quantized: &[i32],
    tables: Layer3EntropyTables<'_>,
) -> Result<PackedBits, Error> {
    let regions = plan_spectral_regions(quantized)?;
    apply_spectral_regions_to_granule(granule, regions)?;

    let big_value_pairs = big_value_pairs(quantized, regions)?;
    let big_value_selection = select_big_value_table(&big_value_pairs)?;
    apply_big_value_table_to_granule(granule, big_value_selection);

    let count1_quads = count1_quads(quantized, regions)?;
    let count1_selection = select_count1_table(&count1_quads)?;
    apply_count1_table_to_granule(granule, count1_selection);

    let big_values = pack_big_value_pairs_with_linbits(
        &big_value_pairs,
        tables.big_values,
        big_value_selection.linbits,
    )?;
    let count1 = pack_count1_quads_with_sign_bits(&count1_quads, tables.count1)?;
    pack_main_data_regions_for_granule(granule, big_values, count1)
}

/// Builds one Layer III granule/channel main-data payload with scale factors.
pub fn pack_quantized_spectrum_with_scale_factors_for_granule(
    granule: &mut Layer3GranuleChannelInfo,
    scale_factors: PackedBits,
    quantized: &[i32],
    tables: Layer3EntropyTables<'_>,
) -> Result<PackedBits, Error> {
    let regions = plan_spectral_regions(quantized)?;
    apply_spectral_regions_to_granule(granule, regions)?;

    let big_value_pairs = big_value_pairs(quantized, regions)?;
    let big_value_selection = select_big_value_table(&big_value_pairs)?;
    apply_big_value_table_to_granule(granule, big_value_selection);

    let count1_quads = count1_quads(quantized, regions)?;
    let count1_selection = select_count1_table(&count1_quads)?;
    apply_count1_table_to_granule(granule, count1_selection);

    let big_values = pack_big_value_pairs_with_linbits(
        &big_value_pairs,
        tables.big_values,
        big_value_selection.linbits,
    )?;
    let count1 = pack_count1_quads_with_sign_bits(&count1_quads, tables.count1)?;
    pack_main_data_parts_for_granule(granule, scale_factors, big_values, count1)
}

/// Builds one granule/channel entropy payload using table selection and provider lookup.
pub fn pack_quantized_spectrum_with_table_provider(
    granule: &mut Layer3GranuleChannelInfo,
    quantized: &[i32],
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<PackedBits, Error> {
    let regions = plan_spectral_regions(quantized)?;
    apply_spectral_regions_to_granule(granule, regions)?;

    let big_value_pairs = big_value_pairs(quantized, regions)?;
    let (region0_pairs, region1_pairs) = long_block_region_pair_split(
        granule.region0_count,
        granule.region1_count,
        big_value_pairs.len(),
    );
    let big_value_selection = select_big_value_region_tables_by_bit_cost(
        &big_value_pairs,
        region0_pairs,
        region1_pairs,
        provider,
    )?;
    apply_big_value_region_tables_to_granule(granule, big_value_selection)?;

    let count1_quads = count1_quads(quantized, regions)?;
    let count1_selection = select_count1_table_by_bit_cost(&count1_quads, provider)?;
    apply_count1_table_to_granule(granule, count1_selection);

    let big_values = pack_big_value_pairs_with_region_tables_and_provider(
        &big_value_pairs,
        big_value_selection,
        provider,
    )?;
    let count1 = pack_count1_quads_with_table_selection(
        &count1_quads,
        provider.count1_table(count1_selection)?,
        count1_selection,
    )?;
    pack_main_data_regions_for_granule(granule, big_values, count1)
}

/// Builds one granule/channel main-data payload with scale factors and provider lookup.
pub fn pack_quantized_spectrum_with_scale_factors_and_table_provider(
    granule: &mut Layer3GranuleChannelInfo,
    scale_factors: PackedBits,
    quantized: &[i32],
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<PackedBits, Error> {
    let regions = plan_spectral_regions(quantized)?;
    apply_spectral_regions_to_granule(granule, regions)?;

    let big_value_pairs = big_value_pairs(quantized, regions)?;
    let (region0_pairs, region1_pairs) = long_block_region_pair_split(
        granule.region0_count,
        granule.region1_count,
        big_value_pairs.len(),
    );
    let big_value_selection = select_big_value_region_tables_by_bit_cost(
        &big_value_pairs,
        region0_pairs,
        region1_pairs,
        provider,
    )?;
    apply_big_value_region_tables_to_granule(granule, big_value_selection)?;

    let count1_quads = count1_quads(quantized, regions)?;
    let count1_selection = select_count1_table_by_bit_cost(&count1_quads, provider)?;
    apply_count1_table_to_granule(granule, count1_selection);

    let big_values = pack_big_value_pairs_with_region_tables_and_provider(
        &big_value_pairs,
        big_value_selection,
        provider,
    )?;
    let count1 = pack_count1_quads_with_table_selection(
        &count1_quads,
        provider.count1_table(count1_selection)?,
        count1_selection,
    )?;
    pack_main_data_parts_for_granule(granule, scale_factors, big_values, count1)
}

/// Long-block scalefactor-band line boundaries shared by all MPEG-1 sample
/// rates. The full per-rate tables diverge only above index 6; the prefix used
/// to place the big-value region addresses (indices 0..=2) is rate-independent.
const LONG_SFB_LOW_BOUNDARIES: [usize; 7] = [0, 4, 8, 12, 16, 20, 24];

/// Maps the written `region_address1`/`region_address2` side-info fields to the
/// big-value region split in pairs, exactly as a spec decoder derives it.
///
/// The decoder reads `region0 = [0, sfb[ra1 + 1])`,
/// `region1 = [sfb[ra1 + 1], sfb[ra1 + ra2 + 2])`, and `region2` the remainder,
/// all in spectral-line units, then capped at the big-value count. The encoder
/// must split pairs at the same boundaries so the bitstream stays in sync.
fn long_block_region_pair_split(
    region0_count: u8,
    region1_count: u8,
    pair_count: usize,
) -> (usize, usize) {
    let r1_idx = usize::from(region0_count) + 1;
    let r2_idx = usize::from(region0_count) + usize::from(region1_count) + 2;
    let r1_start = LONG_SFB_LOW_BOUNDARIES
        .get(r1_idx)
        .copied()
        .unwrap_or(usize::MAX);
    let r2_start = LONG_SFB_LOW_BOUNDARIES
        .get(r2_idx)
        .copied()
        .unwrap_or(usize::MAX);
    let region0 = (r1_start / 2).min(pair_count);
    let region1 = (r2_start.saturating_sub(r1_start) / 2).min(pair_count - region0);
    (region0, region1)
}

/// Splits quantized Layer III spectral coefficients into entropy-coded regions.
pub fn plan_spectral_regions(quantized: &[i32]) -> Result<Layer3SpectralRegions, Error> {
    if quantized.is_empty() || quantized.len() > 576 {
        return Err(Error::InvalidInput(
            "invalid MP3 spectral coefficient count",
        ));
    }
    for &coeff in quantized {
        if coeff
            .checked_abs()
            .ok_or(Error::InvalidInput("MP3 spectral coefficient overflows"))?
            > 8191
        {
            return Err(Error::InvalidInput(
                "MP3 spectral coefficient exceeds supported range",
            ));
        }
    }

    let Some(last_nonzero) = quantized.iter().rposition(|coeff| *coeff != 0) else {
        return Ok(Layer3SpectralRegions {
            big_values: 0,
            count1: 0,
            rzero: u16::try_from(quantized.len())
                .map_err(|_| Error::InvalidInput("MP3 rzero region is too large"))?,
        });
    };

    let nonzero_end = last_nonzero + 1;
    let mut count1_start = nonzero_end;
    while count1_start >= 4 {
        let start = count1_start - 4;
        if quantized[start..count1_start]
            .iter()
            .all(|coeff| coeff.abs() <= 1)
        {
            count1_start = start;
        } else {
            break;
        }
    }

    let big_values = count1_start.div_ceil(2);
    let count1 = (nonzero_end - count1_start) / 4;
    Ok(Layer3SpectralRegions {
        big_values: u16::try_from(big_values)
            .map_err(|_| Error::InvalidInput("MP3 big_values region is too large"))?,
        count1: u16::try_from(count1)
            .map_err(|_| Error::InvalidInput("MP3 count1 region is too large"))?,
        rzero: u16::try_from(quantized.len() - nonzero_end)
            .map_err(|_| Error::InvalidInput("MP3 rzero region is too large"))?,
    })
}

/// Applies spectral region planning to a Layer III granule/channel side-info entry.
pub fn apply_spectral_regions_to_granule(
    granule: &mut Layer3GranuleChannelInfo,
    regions: Layer3SpectralRegions,
) -> Result<(), Error> {
    if regions.big_values > 288 {
        return Err(Error::InvalidInput(
            "MP3 big_values exceeds side-info range",
        ));
    }

    granule.big_values = regions.big_values;
    if regions.big_values == 0 {
        granule.table_select = [0; 3];
        granule.region0_count = 0;
        granule.region1_count = 0;
        granule.count1table_select = regions.count1 > 0;
        return Ok(());
    }

    granule.table_select = [1, 1, 0];
    // Fixed region addresses 0/0 place the region boundaries at the
    // rate-independent low scalefactor bands (lines 4 and 8). The big-value
    // packer splits pairs at the matching boundaries via
    // `long_block_region_pair_split`, keeping the encoder in sync with the
    // decoder's scalefactor-band interpretation of these fields.
    granule.region0_count = 0;
    granule.region1_count = 0;
    granule.count1table_select = regions.count1 > 0;
    Ok(())
}

/// Packs preselected MP3 Layer III main-data Huffman codewords.
pub fn pack_main_data_codewords(codes: &[HuffmanCode]) -> Result<Vec<u8>, Error> {
    pack_huffman_codes(codes)
}

/// Packs preselected MP3 Layer III main-data codewords and preserves bit length.
pub fn pack_main_data_codewords_with_len(codes: &[HuffmanCode]) -> Result<PackedBits, Error> {
    pack_huffman_codes_with_len(codes)
}

/// Sets `part2_3_length` from already-packed Layer III scale-factor/Huffman bits.
pub fn apply_part2_3_length_to_granule(
    granule: &mut Layer3GranuleChannelInfo,
    bit_len: usize,
) -> Result<(), Error> {
    granule.part2_3_length = u16::try_from(bit_len)
        .map_err(|_| Error::InvalidInput("MP3 part2_3_length exceeds side-info range"))?;
    Ok(())
}

/// Packs preselected Layer III main-data codewords and updates side-info length.
pub fn pack_main_data_codewords_for_granule(
    granule: &mut Layer3GranuleChannelInfo,
    codes: &[HuffmanCode],
) -> Result<PackedBits, Error> {
    let packed = pack_main_data_codewords_with_len(codes)?;
    apply_part2_3_length_to_granule(granule, packed.bit_len)?;
    Ok(packed)
}

/// Selects MPEG-1 Layer III long-block scale-factor bit widths.
pub fn select_mpeg1_layer3_long_scale_factor_compress(
    scale_factors: &[u8; MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT],
) -> Result<Layer3ScaleFactorCompress, Error> {
    let max_slen1_value = scale_factors[..11].iter().copied().max().unwrap_or(0);
    let max_slen2_value = scale_factors[11..].iter().copied().max().unwrap_or(0);

    for selection in MPEG1_LAYER3_SCALE_FACTOR_COMPRESS {
        if scale_factor_fits_width(max_slen1_value, selection.slen1)
            && scale_factor_fits_width(max_slen2_value, selection.slen2)
        {
            return Ok(selection);
        }
    }

    Err(Error::InvalidInput(
        "MP3 scale factor exceeds MPEG-1 Layer III compress range",
    ))
}

/// Applies MPEG-1 Layer III scale-factor compression metadata to side info.
pub fn apply_scale_factor_compress_to_granule(
    granule: &mut Layer3GranuleChannelInfo,
    selection: Layer3ScaleFactorCompress,
) {
    granule.scalefac_compress = selection.scalefac_compress;
}

/// Packs MPEG-1 Layer III long-block scale-factor values.
pub fn pack_mpeg1_layer3_long_scale_factors(
    scale_factors: &[u8; MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT],
    selection: Layer3ScaleFactorCompress,
) -> Result<PackedBits, Error> {
    if !MPEG1_LAYER3_SCALE_FACTOR_COMPRESS.contains(&selection) {
        return Err(Error::InvalidInput(
            "invalid MPEG-1 Layer III scalefac_compress selection",
        ));
    }

    let mut writer = CoreBitWriter::new();
    for &scale_factor in &scale_factors[..11] {
        write_mp3_scale_factor(&mut writer, scale_factor, selection.slen1)?;
    }
    for &scale_factor in &scale_factors[11..] {
        write_mp3_scale_factor(&mut writer, scale_factor, selection.slen2)?;
    }

    let bit_len = writer.bit_len();
    Ok(PackedBits {
        bytes: writer.finish_byte_aligned(),
        bit_len,
    })
}

/// Packs MPEG-1 Layer III long-block scale factors and updates side-info metadata.
pub fn pack_mpeg1_layer3_long_scale_factors_for_granule(
    granule: &mut Layer3GranuleChannelInfo,
    scale_factors: &[u8; MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT],
) -> Result<PackedBits, Error> {
    let selection = select_mpeg1_layer3_long_scale_factor_compress(scale_factors)?;
    apply_scale_factor_compress_to_granule(granule, selection);
    pack_mpeg1_layer3_long_scale_factors(scale_factors, selection)
}

/// Selects deterministic MPEG-1 Layer III long-block scale factors from coefficients.
pub fn select_mpeg1_layer3_long_scale_factors_for_quantized_spectrum(
    quantized: &[i32],
) -> Result<[u8; MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT], Error> {
    plan_spectral_regions(quantized)?;

    let mut band_max = [0_u16; MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT];
    for (index, &coefficient) in quantized.iter().enumerate() {
        let magnitude = coefficient
            .checked_abs()
            .ok_or(Error::InvalidInput("MP3 spectral coefficient overflows"))?;
        if magnitude > 8191 {
            return Err(Error::InvalidInput(
                "MP3 spectral coefficient exceeds supported range",
            ));
        }

        let band = index
            .checked_mul(MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT)
            .ok_or(Error::InvalidInput("MP3 scale-factor band index overflows"))?
            / quantized.len();
        band_max[band] = band_max[band].max(
            u16::try_from(magnitude)
                .map_err(|_| Error::InvalidInput("MP3 coefficient magnitude overflows"))?,
        );
    }

    let mut scale_factors = [0_u8; MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT];
    for (band, &max_magnitude) in band_max.iter().enumerate() {
        let raw = if max_magnitude == 0 {
            0
        } else {
            u16::BITS as u8 - max_magnitude.leading_zeros() as u8
        };
        let syntax_cap = if band < 11 { 15 } else { 7 };
        scale_factors[band] = raw.min(syntax_cap);
    }
    Ok(scale_factors)
}

/// Builds one MPEG-1 Layer III long-block main-data payload from scale factors and spectrum.
pub fn pack_mpeg1_layer3_long_quantized_spectrum_for_granule(
    granule: &mut Layer3GranuleChannelInfo,
    scale_factors: &[u8; MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT],
    quantized: &[i32],
    tables: Layer3EntropyTables<'_>,
) -> Result<PackedBits, Error> {
    let scale_factor_bits =
        pack_mpeg1_layer3_long_scale_factors_for_granule(granule, scale_factors)?;
    pack_quantized_spectrum_with_scale_factors_for_granule(
        granule,
        scale_factor_bits,
        quantized,
        tables,
    )
}

/// Builds one MPEG-1 Layer III long-block main-data payload using provider lookup.
pub fn pack_mpeg1_layer3_long_quantized_spectrum_with_table_provider(
    granule: &mut Layer3GranuleChannelInfo,
    scale_factors: &[u8; MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT],
    quantized: &[i32],
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<PackedBits, Error> {
    let scale_factor_bits =
        pack_mpeg1_layer3_long_scale_factors_for_granule(granule, scale_factors)?;
    pack_quantized_spectrum_with_scale_factors_and_table_provider(
        granule,
        scale_factor_bits,
        quantized,
        provider,
    )
}

/// Builds one MPEG-1 Layer III long-block payload with internally selected scale factors.
pub fn pack_mpeg1_layer3_long_quantized_spectrum_with_selected_scale_factors_for_granule(
    granule: &mut Layer3GranuleChannelInfo,
    quantized: &[i32],
    tables: Layer3EntropyTables<'_>,
) -> Result<PackedBits, Error> {
    let scale_factors = select_mpeg1_layer3_long_scale_factors_for_quantized_spectrum(quantized)?;
    pack_mpeg1_layer3_long_quantized_spectrum_for_granule(
        granule,
        &scale_factors,
        quantized,
        tables,
    )
}

/// Builds one MPEG-1 Layer III long-block payload with selected scale factors and provider lookup.
pub fn pack_mpeg1_layer3_long_quantized_spectrum_with_selected_scale_factors_and_table_provider(
    granule: &mut Layer3GranuleChannelInfo,
    quantized: &[i32],
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<PackedBits, Error> {
    let scale_factors = select_mpeg1_layer3_long_scale_factors_for_quantized_spectrum(quantized)?;
    pack_mpeg1_layer3_long_quantized_spectrum_with_table_provider(
        granule,
        &scale_factors,
        quantized,
        provider,
    )
}

/// Builds one MPEG-1 Layer III long-block payload from PCM analysis.
///
/// The quantizer `step` is folded entirely into `global_gain`
/// (see [`mpeg1_layer3_global_gain_for_step`]) and all scale factors are left at
/// zero, so the decoder's per-line requantization inverts the encoder's
/// quantization without per-band double scaling. An all-zero granule keeps the
/// ISO reference gain, preserving the canonical silent-frame encoding.
pub fn pack_mpeg1_layer3_pcm_long_block_with_calibrated_gain_for_granule(
    granule: &mut Layer3GranuleChannelInfo,
    pcm: &AudioBuffer,
    channel: usize,
    start_frame: usize,
    step: f32,
    tables: Layer3EntropyTables<'_>,
) -> Result<PackedBits, Error> {
    let quantized = quantize_pcm_long_block(pcm, channel, start_frame, step)?;
    let scale_factors = [0_u8; MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT];
    let packed = pack_mpeg1_layer3_long_quantized_spectrum_for_granule(
        granule,
        &scale_factors,
        &quantized,
        tables,
    )?;
    granule.global_gain = calibrated_global_gain_for_granule(&quantized, step);
    Ok(packed)
}

/// Builds one MPEG-1 Layer III long-block payload from PCM analysis using provider lookup.
///
/// Behaves like [`pack_mpeg1_layer3_pcm_long_block_with_calibrated_gain_for_granule`]
/// but resolves the entropy tables through a [`Layer3EntropyTableProvider`].
pub fn pack_mpeg1_layer3_pcm_long_block_with_calibrated_gain_and_table_provider(
    granule: &mut Layer3GranuleChannelInfo,
    pcm: &AudioBuffer,
    channel: usize,
    start_frame: usize,
    step: f32,
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<PackedBits, Error> {
    let quantized = quantize_pcm_long_block(pcm, channel, start_frame, step)?;
    let scale_factors = [0_u8; MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT];
    let packed = pack_mpeg1_layer3_long_quantized_spectrum_with_table_provider(
        granule,
        &scale_factors,
        &quantized,
        provider,
    )?;
    granule.global_gain = calibrated_global_gain_for_granule(&quantized, step);
    Ok(packed)
}

/// Picks the `global_gain` for a packed granule: the step-inverting value for a
/// granule that carries energy, or the ISO reference gain (210) for an all-zero
/// granule whose gain is acoustically irrelevant.
fn calibrated_global_gain_for_granule(quantized: &[i32], step: f32) -> u8 {
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

/// Assembles one MPEG-1 Layer III frame from PCM long-block payload scaffolding.
pub fn assemble_mpeg1_layer3_pcm_frame_with_selected_scale_factors(
    header: FrameHeader,
    pcm: &AudioBuffer,
    start_frame: usize,
    step: f32,
    tables: Layer3EntropyTables<'_>,
) -> Result<Vec<u8>, Error> {
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
            let payload = pack_mpeg1_layer3_pcm_long_block_with_calibrated_gain_for_granule(
                &mut side_info.granules[granule][channel],
                pcm,
                channel,
                granule_start,
                step,
                tables,
            )?;
            payloads.push(payload);
        }
    }
    assemble_layer3_frame_from_payloads(header, &side_info, &payloads)
}

/// Assembles one MPEG-1 Layer III frame from PCM long-block payloads using provider lookup.
pub fn assemble_mpeg1_layer3_pcm_frame_with_selected_scale_factors_and_table_provider(
    header: FrameHeader,
    pcm: &AudioBuffer,
    start_frame: usize,
    step: f32,
    provider: Layer3EntropyTableProvider<'_>,
) -> Result<Vec<u8>, Error> {
    let (side_info, main_data) = pack_mpeg1_layer3_pcm_frame_payloads_with_table_provider(
        header,
        pcm,
        start_frame,
        step,
        provider,
    )?;
    assemble_layer3_frame(header, &side_info, &main_data.bytes)
}

fn pack_mpeg1_layer3_pcm_frame_payloads_with_table_provider(
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
            let payload = pack_mpeg1_layer3_pcm_long_block_with_calibrated_gain_and_table_provider(
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

/// Concatenates Layer III big-values and count1 main-data bits.
pub fn pack_main_data_regions(
    big_values: PackedBits,
    count1: PackedBits,
) -> Result<PackedBits, Error> {
    concat_packed_bits(&[big_values, count1])
}

/// Concatenates Layer III scale-factor, big-values, and count1 main-data bits.
pub fn pack_main_data_parts(
    scale_factors: PackedBits,
    big_values: PackedBits,
    count1: PackedBits,
) -> Result<PackedBits, Error> {
    concat_packed_bits(&[scale_factors, big_values, count1])
}

/// Concatenates Layer III entropy regions and updates side-info length.
pub fn pack_main_data_regions_for_granule(
    granule: &mut Layer3GranuleChannelInfo,
    big_values: PackedBits,
    count1: PackedBits,
) -> Result<PackedBits, Error> {
    let packed = pack_main_data_regions(big_values, count1)?;
    apply_part2_3_length_to_granule(granule, packed.bit_len)?;
    Ok(packed)
}

/// Concatenates Layer III main-data parts and updates side-info length.
pub fn pack_main_data_parts_for_granule(
    granule: &mut Layer3GranuleChannelInfo,
    scale_factors: PackedBits,
    big_values: PackedBits,
    count1: PackedBits,
) -> Result<PackedBits, Error> {
    let packed = pack_main_data_parts(scale_factors, big_values, count1)?;
    apply_part2_3_length_to_granule(granule, packed.bit_len)?;
    Ok(packed)
}

/// Packs Layer III big-values pairs using a caller-supplied Huffman table.
pub fn pack_big_value_pairs_with_table(
    pairs: &[Layer3BigValuePair],
    table: &[HuffmanEntry<Layer3BigValuePair>],
) -> Result<PackedBits, Error> {
    pack_huffman_symbols_with_len(pairs, table)
}

/// Packs Layer III big-values pairs as magnitude codewords followed by sign bits.
pub fn pack_big_value_pairs_with_sign_bits(
    pairs: &[Layer3BigValuePair],
    table: &[HuffmanEntry<Layer3BigValueMagnitude>],
) -> Result<PackedBits, Error> {
    pack_big_value_pairs_with_linbits(pairs, table, 0)
}

/// Packs Layer III big-values pairs with optional escape-table linbits.
pub fn pack_big_value_pairs_with_linbits(
    pairs: &[Layer3BigValuePair],
    table: &[HuffmanEntry<Layer3BigValueMagnitude>],
    linbits: u8,
) -> Result<PackedBits, Error> {
    if linbits > 16 {
        return Err(Error::InvalidInput(
            "MP3 linbits width exceeds supported range",
        ));
    }

    let mut writer = CoreBitWriter::new();
    for pair in pairs {
        let x_magnitude = abs_i16_to_u16(pair.x)?;
        let y_magnitude = abs_i16_to_u16(pair.y)?;
        let table_magnitude = Layer3BigValueMagnitude::new(
            table_magnitude_with_linbits(x_magnitude, linbits)?,
            table_magnitude_with_linbits(y_magnitude, linbits)?,
        );
        let code = lookup_huffman_code(table, &table_magnitude)?;
        writer.write_bits(code.bits, code.len)?;
        // ISO/IEC 11172-3 emits each value's escape linbits immediately before
        // its sign, interleaved per value: linbits_x, sign_x, linbits_y, sign_y.
        // Grouping all linbits before all signs desyncs the decoder.
        write_mp3_linbits(&mut writer, x_magnitude, linbits)?;
        write_mp3_sign_bit(&mut writer, pair.x)?;
        write_mp3_linbits(&mut writer, y_magnitude, linbits)?;
        write_mp3_sign_bit(&mut writer, pair.y)?;
    }
    let bit_len = writer.bit_len();
    Ok(PackedBits {
        bytes: writer.finish_byte_aligned(),
        bit_len,
    })
}

fn pack_big_value_pairs_with_selection(
    pairs: &[Layer3BigValuePair],
    table: &[HuffmanEntry<Layer3BigValueMagnitude>],
    selection: Layer3BigValueTableSelection,
) -> Result<PackedBits, Error> {
    if selection.table_select == 0 {
        if max_big_value_magnitude(pairs)? != 0 {
            return Err(Error::InvalidInput(
                "MP3 table 0 requires zero big-values coefficients",
            ));
        }
        return Ok(PackedBits {
            bytes: Vec::new(),
            bit_len: 0,
        });
    }

    pack_big_value_pairs_with_linbits(pairs, table, selection.linbits)
}

/// Packs Layer III count1 quadruples using a caller-supplied Huffman table.
pub fn pack_count1_quads_with_table(
    quads: &[Layer3Count1Quad],
    table: &[HuffmanEntry<Layer3Count1Quad>],
) -> Result<PackedBits, Error> {
    pack_huffman_symbols_with_len(quads, table)
}

/// Packs Layer III count1 quadruples as magnitude codewords followed by sign bits.
pub fn pack_count1_quads_with_sign_bits(
    quads: &[Layer3Count1Quad],
    table: &[HuffmanEntry<Layer3Count1MagnitudeQuad>],
) -> Result<PackedBits, Error> {
    let mut writer = CoreBitWriter::new();
    for quad in quads {
        let magnitude = Layer3Count1MagnitudeQuad::new(
            count1_abs_to_u8(quad.v)?,
            count1_abs_to_u8(quad.w)?,
            count1_abs_to_u8(quad.x)?,
            count1_abs_to_u8(quad.y)?,
        );
        let code = lookup_huffman_code(table, &magnitude)?;
        writer.write_bits(code.bits, code.len)?;
        write_mp3_sign_bit(&mut writer, i16::from(quad.v))?;
        write_mp3_sign_bit(&mut writer, i16::from(quad.w))?;
        write_mp3_sign_bit(&mut writer, i16::from(quad.x))?;
        write_mp3_sign_bit(&mut writer, i16::from(quad.y))?;
    }
    let bit_len = writer.bit_len();
    Ok(PackedBits {
        bytes: writer.finish_byte_aligned(),
        bit_len,
    })
}

fn pack_count1_quads_with_table_selection(
    quads: &[Layer3Count1Quad],
    table: &[HuffmanEntry<Layer3Count1MagnitudeQuad>],
    selection: Layer3Count1TableSelection,
) -> Result<PackedBits, Error> {
    if selection.max_nonzero_values == 0 {
        return Ok(PackedBits {
            bytes: Vec::new(),
            bit_len: 0,
        });
    }

    pack_count1_quads_with_sign_bits(quads, table)
}

fn abs_i16_to_u16(value: i16) -> Result<u16, Error> {
    let magnitude = value
        .checked_abs()
        .ok_or(Error::InvalidInput("MP3 coefficient magnitude overflows"))?;
    u16::try_from(magnitude).map_err(|_| Error::InvalidInput("MP3 coefficient magnitude overflows"))
}

fn count1_abs_to_u8(value: i8) -> Result<u8, Error> {
    let magnitude = value.checked_abs().ok_or(Error::InvalidInput(
        "MP3 count1 coefficient magnitude overflows",
    ))?;
    if magnitude > 1 {
        return Err(Error::InvalidInput(
            "MP3 count1 coefficient exceeds unit magnitude",
        ));
    }
    u8::try_from(magnitude)
        .map_err(|_| Error::InvalidInput("MP3 count1 coefficient magnitude overflows"))
}

fn max_count1_nonzero_values(quads: &[Layer3Count1Quad]) -> Result<u8, Error> {
    let mut max_nonzero_values = 0_u8;
    for quad in quads {
        let values = [quad.v, quad.w, quad.x, quad.y];
        for value in values {
            count1_abs_to_u8(value)?;
        }
        let nonzero = values.iter().filter(|&&value| value != 0).count();
        max_nonzero_values = max_nonzero_values.max(
            u8::try_from(nonzero)
                .map_err(|_| Error::InvalidInput("MP3 count1 nonzero count overflows"))?,
        );
    }
    Ok(max_nonzero_values)
}

fn max_big_value_magnitude(pairs: &[Layer3BigValuePair]) -> Result<u16, Error> {
    let mut max_magnitude = 0_u16;
    for pair in pairs {
        max_magnitude = max_magnitude.max(abs_i16_to_u16(pair.x)?);
        max_magnitude = max_magnitude.max(abs_i16_to_u16(pair.y)?);
    }
    Ok(max_magnitude)
}

fn linbits_for_big_value_magnitude(max_magnitude: u16) -> Result<u8, Error> {
    if max_magnitude <= 15 {
        return Ok(0);
    }

    let extra = max_magnitude - 15;
    let linbits = (16 - extra.leading_zeros()) as u8;
    if linbits > 13 {
        return Err(Error::InvalidInput(
            "MP3 big-values magnitude exceeds table range",
        ));
    }
    Ok(linbits)
}

fn prepare_mpeg1_layer3_pcm_frame_side_info(
    header: FrameHeader,
    pcm: &AudioBuffer,
) -> Result<Layer3SideInfo, Error> {
    if header.version != MpegVersion::Mpeg1 || header.layer != Layer::Layer3 {
        return Err(Error::UnsupportedFeature(
            "MP3 PCM frame payload currently requires MPEG-1 Layer III",
        ));
    }
    if header.sample_rate != pcm.sample_rate {
        return Err(Error::InvalidInput(
            "MP3 header sample rate does not match PCM",
        ));
    }
    if header.channel_count() != usize::from(pcm.channels) {
        return Err(Error::InvalidInput(
            "MP3 header channel count does not match PCM",
        ));
    }

    Ok(Layer3SideInfo::silent(&header))
}

fn mpeg1_layer3_header_for_pcm(pcm: &AudioBuffer) -> Result<FrameHeader, Error> {
    if pcm.channels != 1 && pcm.channels != 2 {
        return Err(Error::UnsupportedFeature(
            "MP3 encode currently supports mono/stereo only",
        ));
    }

    let header = FrameHeader {
        version: MpegVersion::Mpeg1,
        layer: Layer::Layer3,
        protection_absent: true,
        bitrate_kbps: 128,
        sample_rate: pcm.sample_rate,
        padding: false,
        channel_mode: if pcm.channels == 1 {
            ChannelMode::SingleChannel
        } else {
            ChannelMode::Stereo
        },
    };
    header.to_bytes()?;
    Ok(header)
}

fn layer3_frame_count(header: FrameHeader, pcm: &AudioBuffer) -> Result<usize, Error> {
    prepare_mpeg1_layer3_pcm_frame_side_info(header, pcm)?;
    Ok(pcm
        .frames()
        .div_ceil(usize::from(header.samples_per_frame())))
}

const MPEG1_LAYER3_SCALE_FACTOR_COMPRESS: [Layer3ScaleFactorCompress; 16] = [
    Layer3ScaleFactorCompress {
        scalefac_compress: 0,
        slen1: 0,
        slen2: 0,
    },
    Layer3ScaleFactorCompress {
        scalefac_compress: 1,
        slen1: 0,
        slen2: 1,
    },
    Layer3ScaleFactorCompress {
        scalefac_compress: 2,
        slen1: 0,
        slen2: 2,
    },
    Layer3ScaleFactorCompress {
        scalefac_compress: 3,
        slen1: 0,
        slen2: 3,
    },
    Layer3ScaleFactorCompress {
        scalefac_compress: 4,
        slen1: 3,
        slen2: 0,
    },
    Layer3ScaleFactorCompress {
        scalefac_compress: 5,
        slen1: 1,
        slen2: 1,
    },
    Layer3ScaleFactorCompress {
        scalefac_compress: 6,
        slen1: 1,
        slen2: 2,
    },
    Layer3ScaleFactorCompress {
        scalefac_compress: 7,
        slen1: 1,
        slen2: 3,
    },
    Layer3ScaleFactorCompress {
        scalefac_compress: 8,
        slen1: 2,
        slen2: 1,
    },
    Layer3ScaleFactorCompress {
        scalefac_compress: 9,
        slen1: 2,
        slen2: 2,
    },
    Layer3ScaleFactorCompress {
        scalefac_compress: 10,
        slen1: 2,
        slen2: 3,
    },
    Layer3ScaleFactorCompress {
        scalefac_compress: 11,
        slen1: 3,
        slen2: 1,
    },
    Layer3ScaleFactorCompress {
        scalefac_compress: 12,
        slen1: 3,
        slen2: 2,
    },
    Layer3ScaleFactorCompress {
        scalefac_compress: 13,
        slen1: 3,
        slen2: 3,
    },
    Layer3ScaleFactorCompress {
        scalefac_compress: 14,
        slen1: 4,
        slen2: 2,
    },
    Layer3ScaleFactorCompress {
        scalefac_compress: 15,
        slen1: 4,
        slen2: 3,
    },
];

fn scale_factor_fits_width(scale_factor: u8, width: u8) -> bool {
    width < 8 && u16::from(scale_factor) < (1_u16 << width)
}

fn write_mp3_scale_factor(
    writer: &mut CoreBitWriter,
    scale_factor: u8,
    width: u8,
) -> Result<(), Error> {
    if !scale_factor_fits_width(scale_factor, width) {
        return Err(Error::InvalidInput("MP3 scale factor exceeds bit width"));
    }
    writer.write_bits(u32::from(scale_factor), width)
}

fn table_magnitude_with_linbits(magnitude: u16, linbits: u8) -> Result<u16, Error> {
    if linbits == 0 || magnitude < 15 {
        return Ok(magnitude);
    }

    let max_extra = (1_u32 << linbits) - 1;
    let extra = u32::from(magnitude - 15);
    if extra > max_extra {
        return Err(Error::InvalidInput("MP3 linbits value exceeds width"));
    }
    Ok(15)
}

fn write_mp3_linbits(writer: &mut CoreBitWriter, magnitude: u16, linbits: u8) -> Result<(), Error> {
    if linbits == 0 || magnitude < 15 {
        return Ok(());
    }
    writer.write_bits(u32::from(magnitude - 15), linbits)
}

fn write_mp3_sign_bit(writer: &mut CoreBitWriter, value: i16) -> Result<(), Error> {
    if value != 0 {
        writer.write_bits(u32::from(value < 0), 1)?;
    }
    Ok(())
}

fn decode_silent_layer3(input: &[u8]) -> Result<AudioBuffer, Error> {
    if input.is_empty() {
        return Err(Error::InvalidInput("MP3 stream has no frames"));
    }

    let mut remaining = input;
    let mut sample_rate = None;
    let mut channels = None;
    let mut frame_count = 0_usize;
    while !remaining.is_empty() {
        let header = FrameHeader::parse(remaining)?;
        if header.layer != Layer::Layer3 || header.version != MpegVersion::Mpeg1 {
            return Err(Error::UnsupportedFeature(
                "MP3 decode currently supports sonare silent MPEG-1 Layer III only",
            ));
        }

        let frame_len = header.frame_len();
        if remaining.len() < frame_len {
            return Err(Error::InvalidInput("truncated MP3 frame"));
        }

        let side_info_len = header
            .layer3_side_info_len()
            .ok_or(Error::UnsupportedFeature(
                "MP3 side info requires Layer III",
            ))?;
        let crc_len = if header.protection_absent { 0 } else { 2 };
        let side_info_start = 4_usize + crc_len;
        let side_info_end = side_info_start
            .checked_add(side_info_len)
            .ok_or(Error::InvalidInput("MP3 side info offset overflow"))?;
        if side_info_end > frame_len {
            return Err(Error::InvalidInput("invalid MP3 frame side info"));
        }

        let expected_side_info = Layer3SideInfo::silent(&header).pack(&header)?;
        let frame = &remaining[..frame_len];
        if frame[side_info_start..side_info_end] != expected_side_info
            || frame[side_info_end..].iter().any(|byte| *byte != 0)
        {
            return Err(Error::UnsupportedFeature(
                "MP3 decode currently supports sonare silent MPEG-1 Layer III only",
            ));
        }

        match (sample_rate, channels) {
            (Some(sample_rate), Some(channels))
                if sample_rate != header.sample_rate || channels != header.channel_count() =>
            {
                return Err(Error::UnsupportedFeature(
                    "MP3 parameter changes within stream",
                ));
            }
            (Some(_), Some(_)) => {}
            (None, None) => {
                sample_rate = Some(header.sample_rate);
                channels = Some(header.channel_count());
            }
            _ => return Err(Error::InvalidInput("inconsistent MP3 decoder state")),
        }

        frame_count = frame_count
            .checked_add(1)
            .ok_or(Error::InvalidInput("too many MP3 frames"))?;
        remaining = &remaining[frame_len..];
    }

    let sample_rate = sample_rate.ok_or(Error::InvalidInput("MP3 stream has no frames"))?;
    let channels = channels.ok_or(Error::InvalidInput("MP3 stream has no frames"))?;
    let sample_count = frame_count
        .checked_mul(1152)
        .and_then(|frames| frames.checked_mul(channels))
        .ok_or(Error::InvalidInput("decoded MP3 PCM is too large"))?;
    AudioBuffer::new(
        sample_rate,
        u16::try_from(channels).map_err(|_| Error::InvalidInput("too many MP3 channels"))?,
        vec![0.0; sample_count],
    )
}

/// Computes the MPEG audio CRC16 over header/side-information bits after the
/// sync word, using polynomial 0x8005 and initial value 0xffff.
#[must_use]
pub fn crc16_mpeg_audio(bytes: &[u8]) -> u16 {
    let mut crc = 0xffff_u16;
    for &byte in bytes {
        crc ^= u16::from(byte) << 8;
        for _ in 0..8 {
            if crc & 0x8000 != 0 {
                crc = (crc << 1) ^ 0x8005;
            } else {
                crc <<= 1;
            }
        }
    }
    crc
}

#[derive(Clone, Debug, Default)]
pub struct BitWriter {
    out: Vec<u8>,
    bit_pos: u8,
}

impl BitWriter {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn bit_len(&self) -> usize {
        self.out.len() * 8
            - usize::from(if self.bit_pos == 0 {
                0
            } else {
                8 - self.bit_pos
            })
    }

    pub fn write_bits(&mut self, value: u32, count: u8) -> Result<(), Error> {
        if count > 32 {
            return Err(Error::InvalidInput(
                "cannot write more than 32 bits at once",
            ));
        }
        if count < 32 && value >= (1_u32 << count) {
            return Err(Error::InvalidInput("bit value exceeds width"));
        }

        for shift in (0..count).rev() {
            if self.bit_pos == 0 {
                self.out.push(0);
            }
            let bit = ((value >> shift) & 1) as u8;
            let byte = self
                .out
                .last_mut()
                .ok_or(Error::InvalidInput("bit writer has no current byte"))?;
            *byte |= bit << (7 - self.bit_pos);
            self.bit_pos = (self.bit_pos + 1) % 8;
        }
        Ok(())
    }

    pub fn write_bytes(&mut self, bytes: &[u8]) -> Result<(), Error> {
        if self.bit_pos == 0 {
            self.out.extend_from_slice(bytes);
            return Ok(());
        }

        for &byte in bytes {
            self.write_bits(u32::from(byte), 8)?;
        }
        Ok(())
    }

    #[must_use]
    pub fn finish_byte_aligned(self) -> Vec<u8> {
        self.out
    }
}

fn bitrate_kbps(version: MpegVersion, layer: Layer, index: u8) -> Result<u16, Error> {
    if index == 0 || index == 15 {
        return Err(Error::InvalidInput("invalid MP3 bitrate index"));
    }
    let table = match (version, layer) {
        (MpegVersion::Mpeg1, Layer::Layer1) => [
            0, 32, 64, 96, 128, 160, 192, 224, 256, 288, 320, 352, 384, 416, 448, 0,
        ],
        (MpegVersion::Mpeg1, Layer::Layer2) => [
            0, 32, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320, 384, 0,
        ],
        (MpegVersion::Mpeg1, Layer::Layer3) => [
            0, 32, 40, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320, 0,
        ],
        (_, Layer::Layer1) => [
            0, 32, 48, 56, 64, 80, 96, 112, 128, 144, 160, 176, 192, 224, 256, 0,
        ],
        (_, Layer::Layer2 | Layer::Layer3) => [
            0, 8, 16, 24, 32, 40, 48, 56, 64, 80, 96, 112, 128, 144, 160, 0,
        ],
    };
    Ok(table[usize::from(index)])
}

fn bitrate_index(version: MpegVersion, layer: Layer, bitrate_kbps: u16) -> Result<u8, Error> {
    for index in 1..15 {
        if self::bitrate_kbps(version, layer, index)? == bitrate_kbps {
            return Ok(index);
        }
    }
    Err(Error::UnsupportedFeature("MP3 bitrate"))
}

fn sample_rate(version: MpegVersion, index: u8) -> Result<u32, Error> {
    let base = match index {
        0 => 44_100,
        1 => 48_000,
        2 => 32_000,
        _ => return Err(Error::InvalidInput("reserved MP3 sample-rate index")),
    };
    Ok(match version {
        MpegVersion::Mpeg1 => base,
        MpegVersion::Mpeg2 => base / 2,
        MpegVersion::Mpeg25 => base / 4,
    })
}

fn sample_rate_index(version: MpegVersion, target_sample_rate: u32) -> Result<u8, Error> {
    for index in 0..3 {
        if sample_rate(version, index)? == target_sample_rate {
            return Ok(index);
        }
    }
    Err(Error::UnsupportedFeature("MP3 sample rate"))
}

#[cfg(test)]
mod tests {
    use super::filterbank;
    use super::{
        apply_alias_reduction, apply_big_value_table_to_granule, apply_count1_table_to_granule,
        apply_frequency_inversion, apply_part2_3_length_to_granule,
        apply_scale_factor_compress_to_granule, apply_spectral_regions_to_granule,
        assemble_layer3_frame, assemble_layer3_frame_from_payloads,
        assemble_mpeg1_layer3_pcm_frame_with_selected_scale_factors,
        assemble_mpeg1_layer3_pcm_frame_with_selected_scale_factors_and_table_provider,
        big_value_pairs, count1_quads, crc16_mpeg_audio, decode, encode,
        encode_mpeg1_layer3_pcm_frames_with_auto_step_and_table_provider,
        encode_mpeg1_layer3_pcm_frames_with_bitrate_and_table_provider,
        encode_mpeg1_layer3_pcm_frames_with_cbr_bitrate_and_table_provider,
        encode_mpeg1_layer3_pcm_frames_with_header_and_auto_step_and_table_provider,
        encode_mpeg1_layer3_pcm_frames_with_header_and_max_payload_bits_and_table_provider,
        encode_mpeg1_layer3_pcm_frames_with_header_and_selected_scale_factors,
        encode_mpeg1_layer3_pcm_frames_with_header_and_selected_scale_factors_and_table_provider,
        encode_mpeg1_layer3_pcm_frames_with_max_payload_bits_and_table_provider,
        encode_mpeg1_layer3_pcm_frames_with_reservoir_and_table_provider,
        encode_mpeg1_layer3_pcm_frames_with_selected_scale_factors,
        encode_mpeg1_layer3_pcm_frames_with_selected_scale_factors_and_table_provider,
        experimental_unit_magnitude_table_provider, layer3_analysis_subband_block,
        layer3_header_for_capacity, layer3_long_block_spectrum, layer3_main_data_capacity_bits,
        layer3_main_data_capacity_bytes, mdct_long_block, mpeg1_layer3_global_gain_for_step,
        mpeg1_layer3_standard_big_value_table_provider, mpeg1_layer3_standard_table_provider,
        pack_big_value_pairs_with_linbits, pack_big_value_pairs_with_region_tables_and_provider,
        pack_big_value_pairs_with_sign_bits, pack_big_value_pairs_with_table,
        pack_count1_quads_with_sign_bits, pack_count1_quads_with_table,
        pack_layer3_main_data_payloads, pack_main_data_codewords,
        pack_main_data_codewords_for_granule, pack_main_data_codewords_with_len,
        pack_main_data_parts, pack_main_data_parts_for_granule, pack_main_data_regions,
        pack_main_data_regions_for_granule, pack_mpeg1_layer3_long_quantized_spectrum_for_granule,
        pack_mpeg1_layer3_long_quantized_spectrum_with_selected_scale_factors_and_table_provider,
        pack_mpeg1_layer3_long_quantized_spectrum_with_selected_scale_factors_for_granule,
        pack_mpeg1_layer3_long_quantized_spectrum_with_table_provider,
        pack_mpeg1_layer3_long_scale_factors, pack_mpeg1_layer3_long_scale_factors_for_granule,
        pack_mpeg1_layer3_pcm_long_block_with_calibrated_gain_and_table_provider,
        pack_mpeg1_layer3_pcm_long_block_with_calibrated_gain_for_granule,
        pack_quantized_spectrum_for_granule,
        pack_quantized_spectrum_with_scale_factors_and_table_provider,
        pack_quantized_spectrum_with_scale_factors_for_granule,
        pack_quantized_spectrum_with_table_provider, plan_spectral_regions, quantize_long_block,
        quantize_pcm_long_block, select_big_value_region_tables,
        select_big_value_region_tables_by_bit_cost, select_big_value_table,
        select_big_value_table_by_bit_cost, select_count1_table, select_count1_table_by_bit_cost,
        select_mpeg1_layer3_long_scale_factor_compress,
        select_mpeg1_layer3_long_scale_factors_for_quantized_spectrum,
        select_mpeg1_layer3_pcm_frame_step_details_with_max_payload_bits_and_table_provider,
        select_mpeg1_layer3_pcm_frame_step_details_with_table_provider,
        select_mpeg1_layer3_pcm_frame_step_with_max_payload_bits_and_table_provider,
        select_mpeg1_layer3_pcm_frame_step_with_table_provider, BitWriter, ChannelMode,
        FrameHeader, Layer, Layer3BigValueMagnitude, Layer3BigValuePair,
        Layer3BigValueRegionTableSelection, Layer3BigValueTableSelection,
        Layer3Count1MagnitudeQuad, Layer3Count1Quad, Layer3Count1TableSelection,
        Layer3EntropyTableProvider, Layer3EntropyTables, Layer3GranuleChannelInfo,
        Layer3PcmFrameStepSelection, Layer3ScaleFactorCompress, Layer3SideInfo,
        Layer3SpectralRegions, Layer3WindowSwitching, MpegVersion, ALIAS_REDUCTION_C,
        LONG_BLOCK_GRANULE_SAMPLES, MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT,
        MPEG1_LAYER3_PCM_STEP_CANDIDATES,
    };
    use sc_core::{
        detect, quantize_spectrum, AudioBuffer, BitReader, Error, Format, HuffmanCode,
        HuffmanEntry, PackedBits,
    };

    /// Inverse of the `sc-core` (unnormalized) MDCT used by `mdct_long_block`:
    /// `x[m] = (2/N) sum_k X[k] cos[(pi/N)(m + 0.5 + N/2)(k + 0.5)]`, N = 18.
    fn ctrl_imdct_36(lines: &[f32]) -> [f32; 36] {
        let n = LONG_BLOCK_GRANULE_SAMPLES;
        let mut out = [0.0_f32; 36];
        for (m, o) in out.iter_mut().enumerate() {
            let mut acc = 0.0_f64;
            for (k, &x) in lines.iter().enumerate() {
                let angle = std::f64::consts::PI / n as f64
                    * (m as f64 + 0.5 + n as f64 / 2.0)
                    * (k as f64 + 0.5);
                acc += f64::from(x) * angle.cos();
            }
            *o = (2.0 / n as f64 * acc) as f32;
        }
        out
    }

    fn ctrl_sine_window_36() -> [f32; 36] {
        let mut w = [0.0_f32; 36];
        for (i, wi) in w.iter_mut().enumerate() {
            *wi = (std::f32::consts::PI / 36.0 * (i as f32 + 0.5)).sin();
        }
        w
    }

    /// Decoder-side alias reduction: the exact inverse of `apply_alias_reduction`.
    fn ctrl_alias_reduce(spectrum: &mut [f32]) {
        for boundary in 0..(filterbank::SUBBANDS - 1) {
            let upper_base =
                boundary * LONG_BLOCK_GRANULE_SAMPLES + (LONG_BLOCK_GRANULE_SAMPLES - 1);
            let lower_base = (boundary + 1) * LONG_BLOCK_GRANULE_SAMPLES;
            for (i, &c) in ALIAS_REDUCTION_C.iter().enumerate() {
                let cs = 1.0 / (1.0 + c * c).sqrt();
                let ca = c / (1.0 + c * c).sqrt();
                let upper = upper_base - i;
                let lower = lower_base + i;
                let a = spectrum[upper];
                let b = spectrum[lower];
                spectrum[upper] = a * cs - b * ca;
                spectrum[lower] = b * cs + a * ca;
            }
        }
    }

    /// ISO/IEC 11172-3 polyphase synthesis filterbank (decoder), used only as a
    /// controlled oracle so the full encoder chain can be inverted in-process.
    struct CtrlSynth {
        v: Vec<f32>,
    }

    impl CtrlSynth {
        fn new() -> Self {
            Self { v: vec![0.0; 1024] }
        }

        fn step(&mut self, s: &[f32; filterbank::SUBBANDS]) -> [f32; filterbank::SUBBANDS] {
            self.v.rotate_right(64);
            for i in 0..64 {
                let mut acc = 0.0_f32;
                for (k, sk) in s.iter().enumerate() {
                    let angle =
                        (16.0 + i as f64) * (2.0 * k as f64 + 1.0) * std::f64::consts::PI / 64.0;
                    acc += angle.cos() as f32 * *sk;
                }
                self.v[i] = acc;
            }
            let mut u = [0.0_f32; filterbank::WINDOW_LEN];
            for i in 0..8 {
                for j in 0..32 {
                    u[i * 64 + j] = self.v[i * 128 + j];
                    u[i * 64 + 32 + j] = self.v[i * 128 + 96 + j];
                }
            }
            let mut out = [0.0_f32; filterbank::SUBBANDS];
            for (j, oj) in out.iter_mut().enumerate() {
                let mut acc = 0.0_f32;
                for i in 0..16 {
                    acc += u[j + 32 * i] * filterbank::SYNTHESIS_WINDOW_D[j + 32 * i];
                }
                *oj = acc;
            }
            out
        }
    }

    fn ctrl_corr(a: &[f32], b: &[f32]) -> f64 {
        let n = a.len().min(b.len());
        if n == 0 {
            return 0.0;
        }
        let ma = a[..n].iter().map(|x| f64::from(*x)).sum::<f64>() / n as f64;
        let mb = b[..n].iter().map(|x| f64::from(*x)).sum::<f64>() / n as f64;
        let (mut num, mut da, mut db) = (0.0_f64, 0.0_f64, 0.0_f64);
        for i in 0..n {
            let x = f64::from(a[i]) - ma;
            let y = f64::from(b[i]) - mb;
            num += x * y;
            da += x * x;
            db += y * y;
        }
        if da == 0.0 || db == 0.0 {
            0.0
        } else {
            num / (da.sqrt() * db.sqrt())
        }
    }

    /// Runs the full Layer III long-block encoder chain
    /// (`layer3_long_block_spectrum`) and inverts it with a controlled
    /// spec-complete decoder. If the encoder is the exact inverse of the standard
    /// decoder, this reconstructs the input sweep; a low correlation localizes the
    /// bug inside our encoder chain rather than in Symphonia's conventions.
    #[test]
    fn controlled_full_chain_reconstructs_sweep() {
        let sample_rate = 44_100_u32;
        let total = 22_050_usize;
        let input: Vec<f32> = (0..total)
            .map(|i| {
                let t = i as f32 / sample_rate as f32;
                let f = 300.0 + 5_700.0 * (i as f32 / total as f32);
                0.5 * (std::f32::consts::TAU * f * t).sin()
            })
            .collect();
        let pcm = AudioBuffer::new(sample_rate, 1, input.clone()).unwrap();

        let granules = total / 576;
        let win = ctrl_sine_window_36();
        let mut prev_tail = vec![[0.0_f32; LONG_BLOCK_GRANULE_SAMPLES]; filterbank::SUBBANDS];
        let mut synth = CtrlSynth::new();
        let mut out = Vec::<f32>::with_capacity(granules * 576);

        for g in 0..granules {
            let mut spectrum = layer3_long_block_spectrum(&pcm, 0, g * 576).unwrap();
            ctrl_alias_reduce(&mut spectrum);

            let mut hops = [[0.0_f32; filterbank::SUBBANDS]; LONG_BLOCK_GRANULE_SAMPLES];
            for sb in 0..filterbank::SUBBANDS {
                let lines = &spectrum[sb * 18..sb * 18 + 18];
                let im = ctrl_imdct_36(lines);
                let mut cur = [0.0_f32; LONG_BLOCK_GRANULE_SAMPLES];
                for i in 0..LONG_BLOCK_GRANULE_SAMPLES {
                    cur[i] = im[i] * win[i] + prev_tail[sb][i];
                    prev_tail[sb][i] = im[i + 18] * win[i + 18];
                }
                apply_frequency_inversion(sb, &mut cur);
                for h in 0..LONG_BLOCK_GRANULE_SAMPLES {
                    hops[h][sb] = cur[h];
                }
            }
            for hop in &hops {
                out.extend_from_slice(&synth.step(hop));
            }
        }

        // Lag-scan to absorb the filterbank + overlap reconstruction delay.
        let seg = 8_192_usize;
        let ref_start = 6_000_usize;
        let reference = &input[ref_start..ref_start + seg];
        let mut best = (0_usize, f64::NEG_INFINITY);
        for d in 0..2_000_usize {
            let start = ref_start + d;
            if start + seg > out.len() {
                break;
            }
            let c = ctrl_corr(reference, &out[start..start + seg]);
            if c > best.1 {
                best = (d, c);
            }
        }
        let aligned = &out[ref_start + best.0..ref_start + best.0 + seg];
        let in_rms = (reference.iter().map(|x| x * x).sum::<f32>() / seg as f32).sqrt();
        let out_rms = (aligned.iter().map(|x| x * x).sum::<f32>() / seg as f32).sqrt();
        println!(
            "controlled full chain: delay={} corr={:.4} ratio={:.4}",
            best.0,
            best.1,
            out_rms / in_rms
        );
        assert!(
            best.1 > 0.9,
            "encoder chain is not the inverse of the standard decoder: corr={:.4}",
            best.1
        );
    }

    /// Like `controlled_full_chain_reconstructs_sweep`, but routes the spectrum
    /// through the real quantizer and the calibrated `global_gain` requantization
    /// (`xr = sign * |is|^(4/3) * 2^((gg-210)/4)`, zero scalefactors) before
    /// decoding. If this still reconstructs, the spectral *values* the decoder
    /// should receive are correct, so any end-to-end failure is in the bitstream
    /// packing/side-info, not the DSP or the gain calibration.
    #[test]
    fn controlled_chain_survives_quantization() {
        let sample_rate = 44_100_u32;
        let total = 22_050_usize;
        let input: Vec<f32> = (0..total)
            .map(|i| {
                let t = i as f32 / sample_rate as f32;
                let f = 300.0 + 5_700.0 * (i as f32 / total as f32);
                0.5 * (std::f32::consts::TAU * f * t).sin()
            })
            .collect();
        let pcm = AudioBuffer::new(sample_rate, 1, input.clone()).unwrap();

        let step = 0.05_f32;
        let gg = mpeg1_layer3_global_gain_for_step(step);
        let gain = 2.0_f32.powf((f32::from(gg) - 210.0) / 4.0);

        let granules = total / 576;
        let win = ctrl_sine_window_36();
        let mut prev_tail = vec![[0.0_f32; LONG_BLOCK_GRANULE_SAMPLES]; filterbank::SUBBANDS];
        let mut synth = CtrlSynth::new();
        let mut out = Vec::<f32>::with_capacity(granules * 576);

        for g in 0..granules {
            let spectrum = layer3_long_block_spectrum(&pcm, 0, g * 576).unwrap();
            let is = quantize_spectrum(&spectrum, step, 8191).unwrap();
            // Calibrated requantization, exactly as a spec decoder would apply it.
            let mut xr: Vec<f32> = is
                .iter()
                .map(|&q| {
                    let mag = (q.unsigned_abs() as f32).powf(4.0 / 3.0) * gain;
                    if q < 0 {
                        -mag
                    } else {
                        mag
                    }
                })
                .collect();
            ctrl_alias_reduce(&mut xr);

            let mut hops = [[0.0_f32; filterbank::SUBBANDS]; LONG_BLOCK_GRANULE_SAMPLES];
            for sb in 0..filterbank::SUBBANDS {
                let lines = &xr[sb * 18..sb * 18 + 18];
                let im = ctrl_imdct_36(lines);
                let mut cur = [0.0_f32; LONG_BLOCK_GRANULE_SAMPLES];
                for i in 0..LONG_BLOCK_GRANULE_SAMPLES {
                    cur[i] = im[i] * win[i] + prev_tail[sb][i];
                    prev_tail[sb][i] = im[i + 18] * win[i + 18];
                }
                apply_frequency_inversion(sb, &mut cur);
                for h in 0..LONG_BLOCK_GRANULE_SAMPLES {
                    hops[h][sb] = cur[h];
                }
            }
            for hop in &hops {
                out.extend_from_slice(&synth.step(hop));
            }
        }

        let seg = 8_192_usize;
        let ref_start = 6_000_usize;
        let reference = &input[ref_start..ref_start + seg];
        let mut best = (0_usize, f64::NEG_INFINITY);
        for d in 0..2_000_usize {
            let start = ref_start + d;
            if start + seg > out.len() {
                break;
            }
            let c = ctrl_corr(reference, &out[start..start + seg]);
            if c > best.1 {
                best = (d, c);
            }
        }
        let aligned = &out[ref_start + best.0..ref_start + best.0 + seg];
        let in_rms = (reference.iter().map(|x| x * x).sum::<f32>() / seg as f32).sqrt();
        let out_rms = (aligned.iter().map(|x| x * x).sum::<f32>() / seg as f32).sqrt();
        println!(
            "controlled chain (quantized): delay={} corr={:.4} ratio={:.4}",
            best.0,
            best.1,
            out_rms / in_rms
        );
        assert!(
            best.1 > 0.9,
            "quantize+requant path lost the signal: corr={:.4}",
            best.1
        );
    }

    #[ignore = "diagnostic: report auto-selected quantizer step and bit usage"]
    #[test]
    fn diagnostic_reports_step_and_bit_usage() {
        let header = FrameHeader::parse(&[0xff, 0xfb, 0x90, 0xc0]).unwrap();
        let sample_rate = 44_100_u32;
        let samples: Vec<f32> = (0..22_050)
            .map(|i| {
                0.5 * (std::f32::consts::TAU * 2_000.0 * (i as f32 / sample_rate as f32)).sin()
            })
            .collect();
        let pcm = AudioBuffer::new(sample_rate, 1, samples).unwrap();
        let provider = mpeg1_layer3_standard_table_provider();
        let start = usize::from(header.samples_per_frame());
        for &step in MPEG1_LAYER3_PCM_STEP_CANDIDATES {
            let q = quantize_pcm_long_block(&pcm, 0, start, step);
            let max_is = q
                .as_ref()
                .map(|v| v.iter().map(|x| x.unsigned_abs()).max().unwrap_or(0))
                .unwrap_or(0);
            let pack = q.as_ref().ok().map(|quantized| {
                let mut g = Layer3GranuleChannelInfo::default();
                pack_quantized_spectrum_with_table_provider(&mut g, quantized, provider)
                    .map(|p| p.bit_len)
                    .map_err(|e| format!("{e:?}"))
            });
            println!(
                "  step={step:>9} quant_ok={} max_is={max_is} pack={pack:?}",
                q.is_ok(),
            );
        }
        let sel = select_mpeg1_layer3_pcm_frame_step_details_with_table_provider(
            header,
            &pcm,
            start,
            MPEG1_LAYER3_PCM_STEP_CANDIDATES,
            provider,
        )
        .unwrap();
        println!(
            "selected step={} payload_bits={}",
            sel.step, sel.payload_bit_len
        );
    }

    #[test]
    fn parses_mpeg1_layer3_header() {
        let header = FrameHeader::parse(&[0xff, 0xfb, 0x90, 0x64]).unwrap();

        assert_eq!(header.version, MpegVersion::Mpeg1);
        assert_eq!(header.layer, Layer::Layer3);
        assert!(header.protection_absent);
        assert_eq!(header.bitrate_kbps, 128);
        assert_eq!(header.sample_rate, 44_100);
        assert!(!header.padding);
        assert_eq!(header.channel_mode, ChannelMode::JointStereo);
        assert_eq!(header.samples_per_frame(), 1152);
        assert_eq!(header.frame_len(), 417);
        assert_eq!(header.channel_count(), 2);
        assert_eq!(header.layer3_granule_count(), 2);
        assert_eq!(header.layer3_side_info_len(), Some(32));
        assert_eq!(layer3_main_data_capacity_bytes(header).unwrap(), 381);
        assert_eq!(layer3_main_data_capacity_bits(header).unwrap(), 3048);
    }

    #[test]
    fn parses_mpeg2_layer3_padded_header() {
        let header = FrameHeader::parse(&[0xff, 0xf3, 0x82, 0xc0]).unwrap();

        assert_eq!(header.version, MpegVersion::Mpeg2);
        assert_eq!(header.layer, Layer::Layer3);
        assert_eq!(header.bitrate_kbps, 64);
        assert_eq!(header.sample_rate, 22_050);
        assert!(header.padding);
        assert_eq!(header.channel_mode, ChannelMode::SingleChannel);
        assert_eq!(header.samples_per_frame(), 576);
        assert_eq!(header.frame_len(), 209);
        assert_eq!(header.channel_count(), 1);
        assert_eq!(header.layer3_granule_count(), 1);
        assert_eq!(header.layer3_side_info_len(), Some(9));
        assert_eq!(layer3_main_data_capacity_bytes(header).unwrap(), 196);
        assert_eq!(layer3_main_data_capacity_bits(header).unwrap(), 1568);
    }

    #[test]
    fn builds_layer3_capacity_headers() {
        let mono = layer3_header_for_capacity(44_100, 1, 128, false, false).unwrap();
        let stereo = layer3_header_for_capacity(44_100, 2, 128, false, false).unwrap();
        let mpeg2 = layer3_header_for_capacity(22_050, 1, 64, true, false).unwrap();

        assert_eq!(mono.version, MpegVersion::Mpeg1);
        assert_eq!(mono.channel_mode, ChannelMode::SingleChannel);
        assert_eq!(layer3_main_data_capacity_bytes(mono).unwrap(), 396);
        assert_eq!(layer3_main_data_capacity_bits(mono).unwrap(), 3168);
        assert_eq!(stereo.version, MpegVersion::Mpeg1);
        assert_eq!(stereo.channel_mode, ChannelMode::Stereo);
        assert_eq!(layer3_main_data_capacity_bytes(stereo).unwrap(), 381);
        assert_eq!(mpeg2.version, MpegVersion::Mpeg2);
        assert_eq!(layer3_main_data_capacity_bytes(mpeg2).unwrap(), 196);
        assert!(layer3_header_for_capacity(44_100, 3, 128, false, false).is_err());
        assert!(layer3_header_for_capacity(44_100, 1, 123, false, false).is_err());
    }

    #[test]
    fn serializes_header_roundtrip() {
        let header = FrameHeader {
            version: MpegVersion::Mpeg1,
            layer: Layer::Layer3,
            protection_absent: true,
            bitrate_kbps: 128,
            sample_rate: 44_100,
            padding: false,
            channel_mode: ChannelMode::JointStereo,
        };

        let bytes = header.to_bytes().unwrap();

        assert_eq!(FrameHeader::parse(&bytes).unwrap(), header);
    }

    #[test]
    fn rejects_reserved_header_fields() {
        let err = FrameHeader::parse(&[0xff, 0xfb, 0x00, 0x00]).unwrap_err();
        assert!(matches!(
            err,
            Error::InvalidInput("invalid MP3 bitrate index")
        ));

        let err = FrameHeader::parse(&[0xff, 0xfb, 0x9c, 0x00]).unwrap_err();
        assert!(matches!(
            err,
            Error::InvalidInput("reserved MP3 sample-rate index")
        ));
    }

    #[test]
    fn rejects_unsupported_serialized_values() {
        let header = FrameHeader {
            version: MpegVersion::Mpeg1,
            layer: Layer::Layer3,
            protection_absent: true,
            bitrate_kbps: 123,
            sample_rate: 44_100,
            padding: false,
            channel_mode: ChannelMode::Stereo,
        };

        let err = header.to_bytes().unwrap_err();

        assert!(matches!(err, Error::UnsupportedFeature("MP3 bitrate")));
    }

    #[test]
    fn bit_writer_writes_msb_first_and_pads_last_byte() {
        let mut writer = BitWriter::new();

        writer.write_bits(0b101, 3).unwrap();
        writer.write_bits(0b10, 2).unwrap();

        assert_eq!(writer.bit_len(), 5);
        assert_eq!(writer.finish_byte_aligned(), &[0b1011_0000]);
    }

    #[test]
    fn bit_writer_writes_bytes_across_unaligned_position() {
        let mut writer = BitWriter::new();

        writer.write_bits(0b1, 1).unwrap();
        writer.write_bytes(&[0b0101_0101]).unwrap();

        assert_eq!(writer.bit_len(), 9);
        assert_eq!(writer.finish_byte_aligned(), &[0b1010_1010, 0b1000_0000]);
    }

    #[test]
    fn crc16_mpeg_audio_is_stable_for_known_header_bits() {
        assert_eq!(crc16_mpeg_audio(&[]), 0xffff);
        assert_eq!(crc16_mpeg_audio(&[0xfb, 0x90, 0x64]), 0xe30d);
    }

    #[test]
    fn packs_mpeg1_stereo_silent_side_info() {
        let header = FrameHeader::parse(&[0xff, 0xfb, 0x90, 0x00]).unwrap();
        let side_info = Layer3SideInfo::silent(&header);

        let packed = side_info.pack(&header).unwrap();

        assert_eq!(packed.len(), 32);
        assert_eq!(&packed[..4], &[0x00, 0x00, 0x00, 0x00]);
        assert!(packed.iter().any(|byte| *byte != 0));
    }

    #[test]
    fn packs_mpeg2_mono_silent_side_info() {
        let header = FrameHeader::parse(&[0xff, 0xf3, 0x80, 0xc0]).unwrap();
        let side_info = Layer3SideInfo::silent(&header);

        let packed = side_info.pack(&header).unwrap();

        assert_eq!(packed.len(), 9);
        assert_eq!(&packed[..3], &[0x00, 0x00, 0x00]);
        assert!(packed.iter().any(|byte| *byte != 0));
    }

    #[test]
    fn packs_window_switching_side_info() {
        let header = FrameHeader::parse(&[0xff, 0xfb, 0x90, 0xc0]).unwrap();
        let mut side_info = Layer3SideInfo::silent(&header);
        side_info.granules[0][0] = Layer3GranuleChannelInfo {
            part2_3_length: 3,
            big_values: 2,
            global_gain: 210,
            scalefac_compress: 5,
            window_switching: Some(Layer3WindowSwitching {
                block_type: 2,
                mixed_block_flag: true,
                table_select: [1, 2],
                subblock_gain: [3, 4, 5],
            }),
            table_select: [0; 3],
            region0_count: 0,
            region1_count: 0,
            preflag: true,
            scalefac_scale: true,
            count1table_select: true,
        };

        let packed = side_info.pack(&header).unwrap();

        assert_eq!(packed.len(), 17);
        assert_ne!(
            packed,
            Layer3SideInfo::silent(&header).pack(&header).unwrap()
        );
    }

    #[test]
    fn rejects_side_info_for_non_layer3() {
        let header = FrameHeader {
            version: MpegVersion::Mpeg1,
            layer: Layer::Layer2,
            protection_absent: true,
            bitrate_kbps: 128,
            sample_rate: 44_100,
            padding: false,
            channel_mode: ChannelMode::Stereo,
        };

        let err = Layer3SideInfo::silent(&header).pack(&header).unwrap_err();

        assert!(matches!(
            err,
            Error::UnsupportedFeature("MP3 side info requires Layer III")
        ));
    }

    #[test]
    fn assembles_layer3_frame_without_crc() {
        let header = FrameHeader::parse(&[0xff, 0xfb, 0x90, 0x00]).unwrap();
        let side_info = Layer3SideInfo::silent(&header);
        let main_data = [0xaa, 0xbb, 0xcc];

        let frame = assemble_layer3_frame(header, &side_info, &main_data).unwrap();

        assert_eq!(frame.len(), header.frame_len());
        assert_eq!(&frame[..4], &header.to_bytes().unwrap());
        assert_eq!(
            &frame[4..4 + header.layer3_side_info_len().unwrap()],
            side_info.pack(&header).unwrap()
        );
        assert_eq!(
            &frame[4 + header.layer3_side_info_len().unwrap()
                ..4 + header.layer3_side_info_len().unwrap() + main_data.len()],
            main_data
        );
        assert!(
            frame[4 + header.layer3_side_info_len().unwrap() + main_data.len()..]
                .iter()
                .all(|byte| *byte == 0)
        );
    }

    #[test]
    fn assembles_layer3_frame_with_crc() {
        let mut header = FrameHeader::parse(&[0xff, 0xfa, 0x90, 0xc0]).unwrap();
        header.protection_absent = false;
        let side_info = Layer3SideInfo::silent(&header);

        let frame = assemble_layer3_frame(header, &side_info, &[]).unwrap();
        let expected_crc = {
            let mut crc_input = Vec::new();
            crc_input.extend_from_slice(&header.to_bytes().unwrap()[1..]);
            crc_input.extend_from_slice(&side_info.pack(&header).unwrap());
            crc16_mpeg_audio(&crc_input)
        };

        assert_eq!(frame.len(), header.frame_len());
        assert_eq!(&frame[..4], &header.to_bytes().unwrap());
        assert_eq!(&frame[4..6], &expected_crc.to_be_bytes());
        assert_eq!(
            &frame[6..6 + header.layer3_side_info_len().unwrap()],
            side_info.pack(&header).unwrap()
        );
    }

    #[test]
    fn assembles_layer3_frame_from_granule_payloads() {
        let header = FrameHeader::parse(&[0xff, 0xfb, 0x90, 0x00]).unwrap();
        let side_info = Layer3SideInfo::silent(&header);
        let payloads = [
            PackedBits {
                bytes: vec![0b1000_0000],
                bit_len: 1,
            },
            PackedBits {
                bytes: vec![0b0100_0000],
                bit_len: 2,
            },
            PackedBits {
                bytes: vec![0b1110_0000],
                bit_len: 3,
            },
            PackedBits {
                bytes: vec![],
                bit_len: 0,
            },
        ];

        let packed = pack_layer3_main_data_payloads(&header, &payloads).unwrap();
        assert_eq!(
            packed,
            PackedBits {
                bytes: vec![0b1011_1100],
                bit_len: 6,
            }
        );

        let frame = assemble_layer3_frame_from_payloads(header, &side_info, &payloads).unwrap();
        let main_data_start = 4 + header.layer3_side_info_len().unwrap();
        assert_eq!(frame[main_data_start], 0b1011_1100);
        assert!(frame[main_data_start + 1..].iter().all(|byte| *byte == 0));
    }

    #[test]
    fn assembles_mpeg1_layer3_pcm_frame_with_selected_scale_factors() {
        let header = FrameHeader {
            version: MpegVersion::Mpeg1,
            layer: Layer::Layer3,
            protection_absent: true,
            bitrate_kbps: 128,
            sample_rate: 44_100,
            padding: false,
            channel_mode: ChannelMode::Stereo,
        };
        let pcm = AudioBuffer::new(
            44_100,
            2,
            vec![0.0; usize::from(header.samples_per_frame()) * 2],
        )
        .unwrap();
        let expected =
            assemble_layer3_frame(header, &Layer3SideInfo::silent(&header), &[]).unwrap();

        let frame = assemble_mpeg1_layer3_pcm_frame_with_selected_scale_factors(
            header,
            &pcm,
            0,
            1.0,
            Layer3EntropyTables {
                big_values: &[],
                count1: &[],
            },
        )
        .unwrap();
        let provider_frame =
            assemble_mpeg1_layer3_pcm_frame_with_selected_scale_factors_and_table_provider(
                header,
                &pcm,
                0,
                1.0,
                Layer3EntropyTableProvider::default(),
            )
            .unwrap();

        assert_eq!(frame, expected);
        assert_eq!(provider_frame, expected);
    }

    #[test]
    fn rejects_mpeg1_layer3_pcm_frame_shape_mismatch() {
        let header = FrameHeader {
            version: MpegVersion::Mpeg1,
            layer: Layer::Layer3,
            protection_absent: true,
            bitrate_kbps: 128,
            sample_rate: 44_100,
            padding: false,
            channel_mode: ChannelMode::Stereo,
        };
        let pcm = AudioBuffer::new(
            48_000,
            2,
            vec![0.0; usize::from(header.samples_per_frame()) * 2],
        )
        .unwrap();

        let err = assemble_mpeg1_layer3_pcm_frame_with_selected_scale_factors(
            header,
            &pcm,
            0,
            1.0,
            Layer3EntropyTables {
                big_values: &[],
                count1: &[],
            },
        )
        .unwrap_err();
        assert!(matches!(
            err,
            Error::InvalidInput("MP3 header sample rate does not match PCM")
        ));

        let non_mpeg1 = FrameHeader {
            version: MpegVersion::Mpeg2,
            ..header
        };
        let err = assemble_mpeg1_layer3_pcm_frame_with_selected_scale_factors_and_table_provider(
            non_mpeg1,
            &pcm,
            0,
            1.0,
            Layer3EntropyTableProvider::default(),
        )
        .unwrap_err();
        assert!(matches!(
            err,
            Error::UnsupportedFeature("MP3 PCM frame payload currently requires MPEG-1 Layer III")
        ));
    }

    #[test]
    fn rejects_layer3_payload_count_mismatch() {
        let header = FrameHeader::parse(&[0xff, 0xfb, 0x90, 0x00]).unwrap();
        let err = pack_layer3_main_data_payloads(&header, &[]).unwrap_err();
        assert!(matches!(
            err,
            Error::InvalidInput("MP3 main data payload count does not match header")
        ));

        let non_layer3 = FrameHeader {
            layer: Layer::Layer2,
            ..header
        };
        let err = pack_layer3_main_data_payloads(&non_layer3, &[]).unwrap_err();
        assert!(matches!(
            err,
            Error::UnsupportedFeature("MP3 main data requires Layer III")
        ));
    }

    #[test]
    fn rejects_main_data_that_exceeds_frame_capacity() {
        let header = FrameHeader::parse(&[0xff, 0xfb, 0x10, 0xc0]).unwrap();
        let side_info = Layer3SideInfo::silent(&header);
        let main_data = vec![0xff; header.frame_len()];

        let err = assemble_layer3_frame(header, &side_info, &main_data).unwrap_err();

        assert!(matches!(
            err,
            Error::InvalidInput("MP3 main data exceeds frame capacity")
        ));
    }

    #[test]
    fn computes_long_block_mdct_for_layer3_analysis() {
        let mut samples = [0.0_f32; 36];
        samples[0] = 1.0;

        let coeffs = mdct_long_block(&samples).unwrap();

        assert_eq!(coeffs.len(), 18);
        assert!(coeffs.iter().any(|coeff| coeff.abs() > 0.0));
        assert_eq!(mdct_long_block(&[0.0; 36]).unwrap(), vec![0.0; 18]);
    }

    #[test]
    fn builds_layer3_analysis_subband_blocks() {
        let pcm = AudioBuffer::new(
            44_100,
            1,
            (0..2304)
                .map(|sample| ((sample as f32) * 0.01).sin() * 0.25)
                .collect(),
        )
        .unwrap();

        let low = layer3_analysis_subband_block(&pcm, 0, 0, 0).unwrap();
        let high = layer3_analysis_subband_block(&pcm, 0, 0, 31).unwrap();
        let padded = layer3_analysis_subband_block(&pcm, 0, 4096, 0).unwrap();

        assert_eq!(low.len(), 36);
        assert_eq!(high.len(), 36);
        assert_ne!(low, high);
        assert!(low.iter().any(|sample| sample.abs() > 0.0));
        assert_eq!(padded, [0.0; 36]);
        assert!(layer3_analysis_subband_block(&pcm, 0, 0, 32).is_err());
    }

    #[test]
    fn analysis_filterbank_localizes_tones_by_subband() {
        let sample_rate = 44_100.0_f32;
        // Subband `b` covers the band centered near (b + 0.5) * sample_rate / 64.
        for band in [0_usize, 5, 16, 28] {
            let freq = (band as f32 + 0.5) * sample_rate / 64.0;
            let pcm = AudioBuffer::new(
                44_100,
                1,
                (0..4096)
                    .map(|n| {
                        (2.0 * core::f32::consts::PI * freq * (n as f32) / sample_rate).sin() * 0.5
                    })
                    .collect(),
            )
            .unwrap();

            // Analyse a granule whose 512-sample window is fully populated.
            let energy = |subband: usize| -> f32 {
                layer3_analysis_subband_block(&pcm, 0, 1152, subband)
                    .unwrap()
                    .iter()
                    .map(|s| s * s)
                    .sum()
            };
            let peak = (0..32)
                .max_by(|a, b| energy(*a).partial_cmp(&energy(*b)).unwrap())
                .unwrap();
            assert_eq!(
                peak, band,
                "tone at {freq} Hz should peak in subband {band}"
            );
        }
    }

    #[test]
    fn frequency_inversion_is_scoped_and_self_inverse() {
        let original: [f32; LONG_BLOCK_GRANULE_SAMPLES] = core::array::from_fn(|i| i as f32 + 1.0);

        // Even subbands are untouched.
        let mut even = original;
        apply_frequency_inversion(0, &mut even);
        assert_eq!(even, original);

        // Odd subbands negate odd-indexed samples only.
        let mut odd = original;
        apply_frequency_inversion(1, &mut odd);
        for (i, (got, base)) in odd.iter().zip(original.iter()).enumerate() {
            if i % 2 == 1 {
                assert_eq!(*got, -*base);
            } else {
                assert_eq!(*got, *base);
            }
        }

        // Applying the inversion twice restores the input.
        apply_frequency_inversion(1, &mut odd);
        assert_eq!(odd, original);
    }

    #[test]
    fn alias_reduction_inverts_the_decoder_rotation() {
        let mut spectrum: Vec<f32> = (0..576).map(|i| ((i * 7) % 13) as f32 - 6.0).collect();
        let original = spectrum.clone();

        apply_alias_reduction(&mut spectrum);
        assert_ne!(
            spectrum, original,
            "alias reduction should change the spectrum"
        );

        // The decoder applies the forward rotation; it must undo the encoder's.
        for boundary in 0..(filterbank::SUBBANDS - 1) {
            let upper_base =
                boundary * LONG_BLOCK_GRANULE_SAMPLES + (LONG_BLOCK_GRANULE_SAMPLES - 1);
            let lower_base = (boundary + 1) * LONG_BLOCK_GRANULE_SAMPLES;
            for (i, &c) in ALIAS_REDUCTION_C.iter().enumerate() {
                let cs = 1.0 / (1.0 + c * c).sqrt();
                let ca = c / (1.0 + c * c).sqrt();
                let upper = upper_base - i;
                let lower = lower_base + i;
                let a = spectrum[upper];
                let b = spectrum[lower];
                spectrum[upper] = a * cs - b * ca;
                spectrum[lower] = b * cs + a * ca;
            }
        }

        for (got, base) in spectrum.iter().zip(original.iter()) {
            assert!(
                (got - base).abs() < 1e-5,
                "rotation pair should be transparent"
            );
        }
    }

    #[test]
    fn long_block_spectrum_shape_and_silence() {
        let silent = AudioBuffer::new(44_100, 1, vec![0.0; 2304]).unwrap();
        let spectrum = layer3_long_block_spectrum(&silent, 0, 0).unwrap();
        assert_eq!(spectrum.len(), 576);
        assert!(spectrum.iter().all(|line| *line == 0.0));

        let tone = AudioBuffer::new(
            44_100,
            1,
            (0..4096).map(|n| (n as f32 * 0.05).sin() * 0.4).collect(),
        )
        .unwrap();
        let spectrum = layer3_long_block_spectrum(&tone, 0, 1152).unwrap();
        assert_eq!(spectrum.len(), 576);
        assert!(spectrum.iter().any(|line| line.abs() > 0.0));
    }

    #[test]
    fn quantizes_long_block_for_layer3_analysis() {
        let mut samples = [0.0_f32; 36];
        samples[0] = 1.0;

        let quantized = quantize_long_block(&samples, 0.001).unwrap();

        assert_eq!(quantized.len(), 18);
        assert!(quantized.iter().any(|coeff| *coeff != 0));
        assert_eq!(quantize_long_block(&[0.0; 36], 1.0).unwrap(), vec![0; 18]);
        assert!(quantize_long_block(&samples, 0.0).is_err());
    }

    #[test]
    fn quantizes_pcm_long_block_for_layer3_analysis() {
        let pcm = AudioBuffer::new(44_100, 2, vec![1.0, -1.0, 0.0, 0.0]).unwrap();

        let left = quantize_pcm_long_block(&pcm, 0, 0, 0.001).unwrap();
        let right = quantize_pcm_long_block(&pcm, 1, 0, 0.001).unwrap();
        let padded = quantize_pcm_long_block(&pcm, 0, 10, 1.0).unwrap();

        assert_eq!(left.len(), 576);
        assert_eq!(right.len(), 576);
        assert_ne!(left, right);
        assert_eq!(padded, vec![0; 576]);
        assert!(quantize_pcm_long_block(&pcm, 2, 0, 1.0).is_err());
    }

    #[test]
    fn quantizes_mono_and_stereo_with_polyphase_filterbank() {
        let mono = AudioBuffer::new(
            44_100,
            1,
            (0..2304)
                .map(|sample| ((sample as f32) * 0.017).sin() * 0.35)
                .collect(),
        )
        .unwrap();
        let mono_spectrum = layer3_long_block_spectrum(&mono, 0, 576).unwrap();
        let inverted: Vec<f32> = mono_spectrum.into_iter().map(|line| -line).collect();
        let expected_mono = quantize_spectrum(&inverted, 0.01, 8191).unwrap();

        assert_eq!(
            quantize_pcm_long_block(&mono, 0, 576, 0.01).unwrap(),
            expected_mono
        );

        // Each stereo channel runs through the same real polyphase + hybrid MDCT
        // analysis as mono, selected by the channel index.
        let stereo = AudioBuffer::new(
            44_100,
            2,
            (0..2304)
                .flat_map(|sample| {
                    [
                        ((sample as f32) * 0.013).sin() * 0.30,
                        ((sample as f32) * 0.021).cos() * 0.20,
                    ]
                })
                .collect(),
        )
        .unwrap();
        for channel in 0..2 {
            let spectrum = layer3_long_block_spectrum(&stereo, channel, 576).unwrap();
            let inverted: Vec<f32> = spectrum.into_iter().map(|line| -line).collect();
            let expected = quantize_spectrum(&inverted, 0.01, 8191).unwrap();
            assert_eq!(
                quantize_pcm_long_block(&stereo, channel, 576, 0.01).unwrap(),
                expected
            );
        }
        // The two channels carry distinct signals, so their spectra differ.
        assert_ne!(
            quantize_pcm_long_block(&stereo, 0, 576, 0.01).unwrap(),
            quantize_pcm_long_block(&stereo, 1, 576, 0.01).unwrap(),
        );
    }

    #[test]
    fn plans_layer3_spectral_regions() {
        let all_zero = plan_spectral_regions(&[0; 18]).unwrap();
        assert_eq!(
            all_zero,
            Layer3SpectralRegions {
                big_values: 0,
                count1: 0,
                rzero: 18,
            }
        );

        let mixed = plan_spectral_regions(&[3, -2, 0, 0, 1, -1, 0, 1, 0, 0]).unwrap();
        assert_eq!(
            mixed,
            Layer3SpectralRegions {
                big_values: 2,
                count1: 1,
                rzero: 2,
            }
        );

        let count1_only = plan_spectral_regions(&[1, -1, 0, 1, 0, 0, 0, 0]).unwrap();
        assert_eq!(
            count1_only,
            Layer3SpectralRegions {
                big_values: 0,
                count1: 1,
                rzero: 4,
            }
        );
        assert!(plan_spectral_regions(&[]).is_err());
        assert!(plan_spectral_regions(&[8192]).is_err());
    }

    #[test]
    fn extracts_layer3_big_value_pairs() {
        let quantized = [3, -2, 0, 0, 1, -1, 0, 1, 0, 0];
        let regions = plan_spectral_regions(&quantized).unwrap();

        assert_eq!(
            big_value_pairs(&quantized, regions).unwrap(),
            vec![
                Layer3BigValuePair::new(3, -2),
                Layer3BigValuePair::new(0, 0),
            ]
        );
        assert_eq!(
            big_value_pairs(
                &[0, 0, 0, 0],
                Layer3SpectralRegions {
                    big_values: 0,
                    count1: 0,
                    rzero: 4,
                },
            )
            .unwrap(),
            Vec::<Layer3BigValuePair>::new()
        );
        assert!(big_value_pairs(
            &[1, 2],
            Layer3SpectralRegions {
                big_values: 2,
                count1: 0,
                rzero: 0,
            },
        )
        .is_err());
    }

    #[test]
    fn selects_layer3_big_value_table_class() {
        assert_eq!(
            select_big_value_table(&[]).unwrap(),
            Layer3BigValueTableSelection {
                table_select: 0,
                linbits: 0,
                max_magnitude: 0,
            }
        );
        assert_eq!(
            select_big_value_table(&[Layer3BigValuePair::new(1, -1)]).unwrap(),
            Layer3BigValueTableSelection {
                table_select: 1,
                linbits: 0,
                max_magnitude: 1,
            }
        );
        assert_eq!(
            select_big_value_table(&[Layer3BigValuePair::new(3, -2)]).unwrap(),
            Layer3BigValueTableSelection {
                table_select: 5,
                linbits: 0,
                max_magnitude: 3,
            }
        );
        // Tables 16..=23 share table 16's codewords but carry fixed linbits
        // widths; the decoder reads linbits from table_select, so magnitude 18
        // (needs linbits 2) must emit table 17, not table 16 with a free width.
        assert_eq!(
            select_big_value_table(&[Layer3BigValuePair::new(18, -15)]).unwrap(),
            Layer3BigValueTableSelection {
                table_select: 17,
                linbits: 2,
                max_magnitude: 18,
            }
        );
        assert_eq!(
            select_big_value_table(&[Layer3BigValuePair::new(8191, 0)]).unwrap(),
            Layer3BigValueTableSelection {
                table_select: 23,
                linbits: 13,
                max_magnitude: 8191,
            }
        );

        let mut granule = Layer3GranuleChannelInfo {
            big_values: 4,
            ..Default::default()
        };
        apply_big_value_table_to_granule(
            &mut granule,
            Layer3BigValueTableSelection {
                table_select: 16,
                linbits: 4,
                max_magnitude: 20,
            },
        );
        assert_eq!(granule.table_select, [16, 16, 16]);

        granule.big_values = 0;
        apply_big_value_table_to_granule(
            &mut granule,
            Layer3BigValueTableSelection {
                table_select: 1,
                linbits: 0,
                max_magnitude: 1,
            },
        );
        assert_eq!(granule.table_select, [0, 0, 0]);
    }

    #[test]
    fn selects_layer3_big_value_tables_per_region() {
        let pairs = [
            Layer3BigValuePair::new(1, 0),
            Layer3BigValuePair::new(0, -1),
            Layer3BigValuePair::new(3, -2),
            Layer3BigValuePair::new(5, 4),
        ];

        assert_eq!(
            select_big_value_region_tables(&pairs, 2, 1).unwrap(),
            Layer3BigValueRegionTableSelection {
                regions: [
                    Layer3BigValueTableSelection {
                        table_select: 1,
                        linbits: 0,
                        max_magnitude: 1,
                    },
                    Layer3BigValueTableSelection {
                        table_select: 5,
                        linbits: 0,
                        max_magnitude: 3,
                    },
                    Layer3BigValueTableSelection {
                        table_select: 7,
                        linbits: 0,
                        max_magnitude: 5,
                    },
                ],
                region0_pairs: 2,
                region1_pairs: 1,
            }
        );

        let err = select_big_value_region_tables(&pairs, 3, 2).unwrap_err();
        assert!(matches!(
            err,
            Error::InvalidInput("MP3 big-values region exceeds spectrum length")
        ));
    }

    #[test]
    fn selects_layer3_big_value_table_by_bit_cost() {
        let pairs = [Layer3BigValuePair::new(1, 0)];
        let table_1 = [HuffmanEntry {
            symbol: Layer3BigValueMagnitude::new(1, 0),
            code: HuffmanCode::new(0b1111, 4).unwrap(),
        }];
        let table_5 = [HuffmanEntry {
            symbol: Layer3BigValueMagnitude::new(1, 0),
            code: HuffmanCode::new(0b0, 1).unwrap(),
        }];

        assert_eq!(
            select_big_value_table_by_bit_cost(
                &pairs,
                Layer3EntropyTableProvider {
                    big_value_table_1: &table_1,
                    big_value_table_5: &table_5,
                    ..Default::default()
                },
            )
            .unwrap(),
            Layer3BigValueTableSelection {
                table_select: 5,
                linbits: 0,
                max_magnitude: 1,
            }
        );
        assert_eq!(
            select_big_value_table_by_bit_cost(
                &[Layer3BigValuePair::new(0, 0)],
                Default::default()
            )
            .unwrap(),
            Layer3BigValueTableSelection {
                table_select: 0,
                linbits: 0,
                max_magnitude: 0,
            }
        );
        let err = select_big_value_table_by_bit_cost(&pairs, Default::default()).unwrap_err();
        assert!(matches!(
            err,
            Error::UnsupportedFeature("MP3 big-values Huffman table")
        ));
    }

    #[test]
    fn extracts_layer3_count1_quads() {
        let quantized = [3, -2, 0, 0, 1, -1, 0, 1, 0, 0];
        let regions = plan_spectral_regions(&quantized).unwrap();

        assert_eq!(
            count1_quads(&quantized, regions).unwrap(),
            vec![Layer3Count1Quad::new(1, -1, 0, 1)]
        );
        assert_eq!(
            count1_quads(
                &[0, 0, 0, 0],
                Layer3SpectralRegions {
                    big_values: 0,
                    count1: 0,
                    rzero: 4,
                },
            )
            .unwrap(),
            Vec::<Layer3Count1Quad>::new()
        );
        assert!(count1_quads(
            &[1, 2, 0, 0],
            Layer3SpectralRegions {
                big_values: 0,
                count1: 1,
                rzero: 0,
            },
        )
        .is_err());
        assert!(count1_quads(
            &[1, 0],
            Layer3SpectralRegions {
                big_values: 0,
                count1: 1,
                rzero: 0,
            },
        )
        .is_err());
    }

    #[test]
    fn selects_layer3_count1_table_class() {
        assert_eq!(
            select_count1_table(&[]).unwrap(),
            Layer3Count1TableSelection {
                table_select: false,
                max_nonzero_values: 0,
            }
        );
        assert_eq!(
            select_count1_table(&[Layer3Count1Quad::new(1, 0, -1, 0)]).unwrap(),
            Layer3Count1TableSelection {
                table_select: false,
                max_nonzero_values: 2,
            }
        );
        assert_eq!(
            select_count1_table(&[
                Layer3Count1Quad::new(1, -1, 0, 1),
                Layer3Count1Quad::new(0, 0, 0, 0),
            ])
            .unwrap(),
            Layer3Count1TableSelection {
                table_select: true,
                max_nonzero_values: 3,
            }
        );
        assert!(select_count1_table(&[Layer3Count1Quad::new(2, 0, 0, 0)]).is_err());

        let mut granule = Layer3GranuleChannelInfo::default();
        apply_count1_table_to_granule(
            &mut granule,
            Layer3Count1TableSelection {
                table_select: true,
                max_nonzero_values: 4,
            },
        );
        assert!(granule.count1table_select);
    }

    #[test]
    fn selects_layer3_count1_table_by_bit_cost() {
        let quads = [Layer3Count1Quad::new(1, -1, 0, 1)];
        let table_0 = [HuffmanEntry {
            symbol: Layer3Count1MagnitudeQuad::new(1, 1, 0, 1),
            code: HuffmanCode::new(0b1111, 4).unwrap(),
        }];
        let table_1 = [HuffmanEntry {
            symbol: Layer3Count1MagnitudeQuad::new(1, 1, 0, 1),
            code: HuffmanCode::new(0b0, 1).unwrap(),
        }];

        assert_eq!(
            select_count1_table_by_bit_cost(
                &quads,
                Layer3EntropyTableProvider {
                    count1_table_0: &table_0,
                    count1_table_1: &table_1,
                    ..Default::default()
                },
            )
            .unwrap(),
            Layer3Count1TableSelection {
                table_select: true,
                max_nonzero_values: 3,
            }
        );
        assert_eq!(
            select_count1_table_by_bit_cost(
                &[Layer3Count1Quad::new(0, 0, 0, 0)],
                Default::default()
            )
            .unwrap(),
            Layer3Count1TableSelection {
                table_select: false,
                max_nonzero_values: 0,
            }
        );
        let err = select_count1_table_by_bit_cost(&quads, Default::default()).unwrap_err();
        assert!(matches!(
            err,
            Error::UnsupportedFeature("MP3 count1 Huffman table")
        ));
    }

    #[test]
    fn applies_spectral_regions_to_side_info_granule() {
        let header = FrameHeader::parse(&[0xff, 0xfb, 0x90, 0xc0]).unwrap();
        let mut side_info = Layer3SideInfo::silent(&header);
        let silent = side_info.pack(&header).unwrap();

        apply_spectral_regions_to_granule(
            &mut side_info.granules[0][0],
            Layer3SpectralRegions {
                big_values: 9,
                count1: 2,
                rzero: 12,
            },
        )
        .unwrap();

        let granule = side_info.granules[0][0];
        assert_eq!(granule.big_values, 9);
        assert_eq!(granule.table_select, [1, 1, 0]);
        // Region addresses are fixed at the rate-independent low scalefactor
        // bands so the packer's pair split matches the decoder's interpretation.
        assert_eq!(granule.region0_count, 0);
        assert_eq!(granule.region1_count, 0);
        assert!(granule.count1table_select);
        assert_ne!(side_info.pack(&header).unwrap(), silent);

        let mut empty = Layer3GranuleChannelInfo::default();
        apply_spectral_regions_to_granule(
            &mut empty,
            Layer3SpectralRegions {
                big_values: 0,
                count1: 0,
                rzero: 18,
            },
        )
        .unwrap();
        assert_eq!(empty.table_select, [0; 3]);
        assert!(!empty.count1table_select);

        let err = apply_spectral_regions_to_granule(
            &mut empty,
            Layer3SpectralRegions {
                big_values: 289,
                count1: 0,
                rzero: 0,
            },
        )
        .unwrap_err();
        assert!(matches!(
            err,
            Error::InvalidInput("MP3 big_values exceeds side-info range")
        ));
    }

    #[test]
    fn packs_mp3_main_data_codewords() {
        let codes = [
            HuffmanCode::new(0b11, 2).unwrap(),
            HuffmanCode::new(0b001, 3).unwrap(),
            HuffmanCode::new(0b0, 1).unwrap(),
        ];

        assert_eq!(pack_main_data_codewords(&codes).unwrap(), &[0b1100_1000]);
        assert_eq!(
            pack_main_data_codewords_with_len(&codes).unwrap(),
            PackedBits {
                bytes: vec![0b1100_1000],
                bit_len: 6,
            }
        );

        let mut granule = Layer3GranuleChannelInfo::default();
        let packed = pack_main_data_codewords_for_granule(&mut granule, &codes).unwrap();
        assert_eq!(packed.bit_len, 6);
        assert_eq!(granule.part2_3_length, 6);

        let err =
            apply_part2_3_length_to_granule(&mut granule, usize::from(u16::MAX) + 1).unwrap_err();
        assert!(matches!(
            err,
            Error::InvalidInput("MP3 part2_3_length exceeds side-info range")
        ));
    }

    #[test]
    fn packs_mp3_main_data_regions_for_granule() {
        let big_values = PackedBits {
            bytes: vec![0b1010_0000],
            bit_len: 3,
        };
        let count1 = PackedBits {
            bytes: vec![0b1100_0000],
            bit_len: 2,
        };

        assert_eq!(
            pack_main_data_regions(big_values.clone(), count1.clone()).unwrap(),
            PackedBits {
                bytes: vec![0b1011_1000],
                bit_len: 5,
            }
        );

        let mut granule = Layer3GranuleChannelInfo::default();
        let packed = pack_main_data_regions_for_granule(&mut granule, big_values, count1).unwrap();
        assert_eq!(packed.bit_len, 5);
        assert_eq!(granule.part2_3_length, 5);
        assert!(pack_main_data_regions(
            PackedBits {
                bytes: vec![0],
                bit_len: 9,
            },
            PackedBits {
                bytes: vec![],
                bit_len: 0,
            },
        )
        .is_err());
    }

    #[test]
    fn packs_mp3_main_data_parts_for_granule() {
        let scale_factors = PackedBits {
            bytes: vec![0b1100_0000],
            bit_len: 2,
        };
        let big_values = PackedBits {
            bytes: vec![0b1010_0000],
            bit_len: 3,
        };
        let count1 = PackedBits {
            bytes: vec![0b0100_0000],
            bit_len: 2,
        };

        assert_eq!(
            pack_main_data_parts(scale_factors.clone(), big_values.clone(), count1.clone())
                .unwrap(),
            PackedBits {
                bytes: vec![0b1110_1010],
                bit_len: 7,
            }
        );

        let mut granule = Layer3GranuleChannelInfo::default();
        let packed =
            pack_main_data_parts_for_granule(&mut granule, scale_factors, big_values, count1)
                .unwrap();
        assert_eq!(packed.bit_len, 7);
        assert_eq!(granule.part2_3_length, 7);
        assert!(pack_main_data_parts(
            PackedBits {
                bytes: vec![0],
                bit_len: 9,
            },
            PackedBits {
                bytes: vec![],
                bit_len: 0,
            },
            PackedBits {
                bytes: vec![],
                bit_len: 0,
            },
        )
        .is_err());
    }

    #[test]
    fn packs_mpeg1_layer3_long_scale_factors_for_granule() {
        let mut scale_factors = [0_u8; MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT];
        scale_factors[0] = 3;
        scale_factors[10] = 2;
        scale_factors[11] = 1;
        scale_factors[20] = 1;

        let selection = select_mpeg1_layer3_long_scale_factor_compress(&scale_factors).unwrap();
        assert_eq!(
            selection,
            Layer3ScaleFactorCompress {
                scalefac_compress: 8,
                slen1: 2,
                slen2: 1,
            }
        );
        assert_eq!(
            pack_mpeg1_layer3_long_scale_factors(&scale_factors, selection).unwrap(),
            PackedBits {
                bytes: vec![0b1100_0000, 0b0000_0000, 0b0000_1010, 0b0000_0001],
                bit_len: 32,
            }
        );

        let mut granule = Layer3GranuleChannelInfo::default();
        let packed =
            pack_mpeg1_layer3_long_scale_factors_for_granule(&mut granule, &scale_factors).unwrap();
        assert_eq!(packed.bit_len, 32);
        assert_eq!(granule.scalefac_compress, 8);

        apply_scale_factor_compress_to_granule(
            &mut granule,
            Layer3ScaleFactorCompress {
                scalefac_compress: 15,
                slen1: 4,
                slen2: 3,
            },
        );
        assert_eq!(granule.scalefac_compress, 15);
    }

    #[test]
    fn packs_zero_width_mpeg1_layer3_long_scale_factors() {
        let scale_factors = [0_u8; MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT];
        let mut granule = Layer3GranuleChannelInfo::default();

        let packed =
            pack_mpeg1_layer3_long_scale_factors_for_granule(&mut granule, &scale_factors).unwrap();

        assert_eq!(
            packed,
            PackedBits {
                bytes: vec![],
                bit_len: 0,
            }
        );
        assert_eq!(granule.scalefac_compress, 0);
    }

    #[test]
    fn rejects_unrepresentable_mpeg1_layer3_long_scale_factors() {
        let mut scale_factors = [0_u8; MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT];
        scale_factors[11] = 8;

        assert!(select_mpeg1_layer3_long_scale_factor_compress(&scale_factors).is_err());
        assert!(pack_mpeg1_layer3_long_scale_factors(
            &scale_factors,
            Layer3ScaleFactorCompress {
                scalefac_compress: 8,
                slen1: 2,
                slen2: 1,
            },
        )
        .is_err());
        assert!(pack_mpeg1_layer3_long_scale_factors(
            &[0_u8; MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT],
            Layer3ScaleFactorCompress {
                scalefac_compress: 16,
                slen1: 4,
                slen2: 4,
            },
        )
        .is_err());
    }

    #[test]
    fn selects_mpeg1_layer3_long_scale_factors_from_quantized_spectrum() {
        let mut quantized = [0_i32; 42];
        quantized[0] = 1;
        quantized[20] = 15;
        quantized[22] = 7;
        quantized[40] = 8191;

        let scale_factors =
            select_mpeg1_layer3_long_scale_factors_for_quantized_spectrum(&quantized).unwrap();

        let mut expected = [0_u8; MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT];
        expected[0] = 1;
        expected[10] = 4;
        expected[11] = 3;
        expected[20] = 7;
        assert_eq!(scale_factors, expected);
        assert!(select_mpeg1_layer3_long_scale_factors_for_quantized_spectrum(&[]).is_err());
        assert!(select_mpeg1_layer3_long_scale_factors_for_quantized_spectrum(&[8192]).is_err());
        assert!(
            select_mpeg1_layer3_long_scale_factors_for_quantized_spectrum(&[i32::MIN]).is_err()
        );
    }

    #[test]
    fn packs_mpeg1_layer3_long_quantized_spectrum_for_granule() {
        let big_value_table = [
            HuffmanEntry {
                symbol: Layer3BigValueMagnitude::new(3, 2),
                code: HuffmanCode::new(0b10, 2).unwrap(),
            },
            HuffmanEntry {
                symbol: Layer3BigValueMagnitude::new(0, 0),
                code: HuffmanCode::new(0b0, 1).unwrap(),
            },
        ];
        let count1_table = [HuffmanEntry {
            symbol: Layer3Count1MagnitudeQuad::new(1, 1, 0, 1),
            code: HuffmanCode::new(0b11, 2).unwrap(),
        }];
        let mut scale_factors = [0_u8; MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT];
        scale_factors[0] = 3;
        scale_factors[10] = 2;
        scale_factors[11] = 1;
        scale_factors[20] = 1;
        let quantized = [3, -2, 0, 0, 1, -1, 0, 1, 0, 0];
        let mut granule = Layer3GranuleChannelInfo::default();

        let packed = pack_mpeg1_layer3_long_quantized_spectrum_for_granule(
            &mut granule,
            &scale_factors,
            &quantized,
            Layer3EntropyTables {
                big_values: &big_value_table,
                count1: &count1_table,
            },
        )
        .unwrap();

        assert_eq!(
            packed,
            PackedBits {
                bytes: vec![
                    0b1100_0000,
                    0b0000_0000,
                    0b0000_1010,
                    0b0000_0001,
                    0b1001_0110,
                    0b1000_0000,
                ],
                bit_len: 42,
            }
        );
        assert_eq!(granule.scalefac_compress, 8);
        assert_eq!(granule.big_values, 2);
        assert_eq!(granule.table_select, [5, 5, 5]);
        assert!(granule.count1table_select);
        assert_eq!(granule.part2_3_length, 42);
    }

    #[test]
    fn packs_mpeg1_layer3_long_quantized_spectrum_with_table_provider() {
        let big_value_table_5 = [
            HuffmanEntry {
                symbol: Layer3BigValueMagnitude::new(3, 2),
                code: HuffmanCode::new(0b10, 2).unwrap(),
            },
            HuffmanEntry {
                symbol: Layer3BigValueMagnitude::new(0, 0),
                code: HuffmanCode::new(0b0, 1).unwrap(),
            },
        ];
        let count1_table_1 = [HuffmanEntry {
            symbol: Layer3Count1MagnitudeQuad::new(1, 1, 0, 1),
            code: HuffmanCode::new(0b11, 2).unwrap(),
        }];
        let mut scale_factors = [0_u8; MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT];
        scale_factors[11] = 1;
        let quantized = [3, -2, 0, 0, 1, -1, 0, 1, 0, 0];
        let mut granule = Layer3GranuleChannelInfo::default();

        let packed = pack_mpeg1_layer3_long_quantized_spectrum_with_table_provider(
            &mut granule,
            &scale_factors,
            &quantized,
            Layer3EntropyTableProvider {
                big_value_table_5: &big_value_table_5,
                count1_table_1: &count1_table_1,
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(
            packed,
            PackedBits {
                bytes: vec![0b1000_0000, 0b0010_0101, 0b1010_0000],
                bit_len: 20,
            }
        );
        assert_eq!(granule.scalefac_compress, 1);
        assert_eq!(granule.table_select, [5, 0, 0]);
        assert!(granule.count1table_select);
        assert_eq!(granule.part2_3_length, 20);
    }

    #[test]
    fn packs_mpeg1_layer3_long_quantized_spectrum_with_selected_scale_factors() {
        let big_value_table = [
            HuffmanEntry {
                symbol: Layer3BigValueMagnitude::new(3, 2),
                code: HuffmanCode::new(0b10, 2).unwrap(),
            },
            HuffmanEntry {
                symbol: Layer3BigValueMagnitude::new(0, 0),
                code: HuffmanCode::new(0b0, 1).unwrap(),
            },
        ];
        let count1_table = [HuffmanEntry {
            symbol: Layer3Count1MagnitudeQuad::new(1, 1, 0, 1),
            code: HuffmanCode::new(0b11, 2).unwrap(),
        }];
        let quantized = [3, -2, 0, 0, 1, -1, 0, 1, 0, 0];
        let scale_factors =
            select_mpeg1_layer3_long_scale_factors_for_quantized_spectrum(&quantized).unwrap();
        let mut manual_granule = Layer3GranuleChannelInfo::default();
        let manual = pack_mpeg1_layer3_long_quantized_spectrum_for_granule(
            &mut manual_granule,
            &scale_factors,
            &quantized,
            Layer3EntropyTables {
                big_values: &big_value_table,
                count1: &count1_table,
            },
        )
        .unwrap();

        let mut selected_granule = Layer3GranuleChannelInfo::default();
        let selected =
            pack_mpeg1_layer3_long_quantized_spectrum_with_selected_scale_factors_for_granule(
                &mut selected_granule,
                &quantized,
                Layer3EntropyTables {
                    big_values: &big_value_table,
                    count1: &count1_table,
                },
            )
            .unwrap();

        assert_eq!(selected, manual);
        assert_eq!(
            selected_granule.scalefac_compress,
            manual_granule.scalefac_compress
        );
        assert_eq!(
            selected_granule.part2_3_length,
            manual_granule.part2_3_length
        );
    }

    #[test]
    fn packs_mpeg1_layer3_long_quantized_spectrum_with_selected_scale_factors_and_provider() {
        let big_value_table_5 = [
            HuffmanEntry {
                symbol: Layer3BigValueMagnitude::new(3, 2),
                code: HuffmanCode::new(0b10, 2).unwrap(),
            },
            HuffmanEntry {
                symbol: Layer3BigValueMagnitude::new(0, 0),
                code: HuffmanCode::new(0b0, 1).unwrap(),
            },
        ];
        let count1_table_1 = [HuffmanEntry {
            symbol: Layer3Count1MagnitudeQuad::new(1, 1, 0, 1),
            code: HuffmanCode::new(0b11, 2).unwrap(),
        }];
        let quantized = [3, -2, 0, 0, 1, -1, 0, 1, 0, 0];
        let scale_factors =
            select_mpeg1_layer3_long_scale_factors_for_quantized_spectrum(&quantized).unwrap();
        let provider = Layer3EntropyTableProvider {
            big_value_table_5: &big_value_table_5,
            count1_table_1: &count1_table_1,
            ..Default::default()
        };
        let mut manual_granule = Layer3GranuleChannelInfo::default();
        let manual = pack_mpeg1_layer3_long_quantized_spectrum_with_table_provider(
            &mut manual_granule,
            &scale_factors,
            &quantized,
            provider,
        )
        .unwrap();

        let mut selected_granule = Layer3GranuleChannelInfo::default();
        let selected =
            pack_mpeg1_layer3_long_quantized_spectrum_with_selected_scale_factors_and_table_provider(
                &mut selected_granule,
                &quantized,
                provider,
            )
            .unwrap();

        assert_eq!(selected, manual);
        assert_eq!(
            selected_granule.scalefac_compress,
            manual_granule.scalefac_compress
        );
        assert_eq!(
            selected_granule.part2_3_length,
            manual_granule.part2_3_length
        );
    }

    #[test]
    fn packs_mpeg1_layer3_pcm_long_block_with_selected_scale_factors() {
        let pcm = AudioBuffer::new(44_100, 1, vec![0.0; 36]).unwrap();
        let quantized = quantize_pcm_long_block(&pcm, 0, 0, 1.0).unwrap();
        let tables = Layer3EntropyTables {
            big_values: &[],
            count1: &[],
        };
        let mut manual_granule = Layer3GranuleChannelInfo::default();
        let manual =
            pack_mpeg1_layer3_long_quantized_spectrum_with_selected_scale_factors_for_granule(
                &mut manual_granule,
                &quantized,
                tables,
            )
            .unwrap();

        let mut pcm_granule = Layer3GranuleChannelInfo::default();
        let packed = pack_mpeg1_layer3_pcm_long_block_with_calibrated_gain_for_granule(
            &mut pcm_granule,
            &pcm,
            0,
            0,
            1.0,
            tables,
        )
        .unwrap();

        assert_eq!(packed, manual);
        assert_eq!(packed.bit_len, 0);
        assert_eq!(
            pcm_granule.scalefac_compress,
            manual_granule.scalefac_compress
        );
        assert_eq!(pcm_granule.part2_3_length, manual_granule.part2_3_length);
    }

    #[test]
    fn packs_mpeg1_layer3_pcm_long_block_with_selected_scale_factors_and_provider() {
        let pcm = AudioBuffer::new(44_100, 1, vec![0.0; 36]).unwrap();
        let quantized = quantize_pcm_long_block(&pcm, 0, 0, 1.0).unwrap();
        let provider = Layer3EntropyTableProvider::default();
        let mut manual_granule = Layer3GranuleChannelInfo::default();
        let manual =
            pack_mpeg1_layer3_long_quantized_spectrum_with_selected_scale_factors_and_table_provider(
                &mut manual_granule,
                &quantized,
                provider,
            )
            .unwrap();

        let mut pcm_granule = Layer3GranuleChannelInfo::default();
        let packed = pack_mpeg1_layer3_pcm_long_block_with_calibrated_gain_and_table_provider(
            &mut pcm_granule,
            &pcm,
            0,
            0,
            1.0,
            provider,
        )
        .unwrap();

        assert_eq!(packed, manual);
        assert_eq!(pcm_granule.big_values, 0);
        assert_eq!(pcm_granule.part2_3_length, manual_granule.part2_3_length);
        assert!(
            pack_mpeg1_layer3_pcm_long_block_with_calibrated_gain_and_table_provider(
                &mut Layer3GranuleChannelInfo::default(),
                &pcm,
                1,
                0,
                1.0,
                provider,
            )
            .is_err()
        );
    }

    #[test]
    fn packs_quantized_spectrum_for_granule() {
        let big_value_table = [
            HuffmanEntry {
                symbol: Layer3BigValueMagnitude::new(3, 2),
                code: HuffmanCode::new(0b10, 2).unwrap(),
            },
            HuffmanEntry {
                symbol: Layer3BigValueMagnitude::new(0, 0),
                code: HuffmanCode::new(0b0, 1).unwrap(),
            },
        ];
        let count1_table = [HuffmanEntry {
            symbol: Layer3Count1MagnitudeQuad::new(1, 1, 0, 1),
            code: HuffmanCode::new(0b11, 2).unwrap(),
        }];
        let quantized = [3, -2, 0, 0, 1, -1, 0, 1, 0, 0];
        let mut granule = Layer3GranuleChannelInfo::default();

        let packed = pack_quantized_spectrum_for_granule(
            &mut granule,
            &quantized,
            Layer3EntropyTables {
                big_values: &big_value_table,
                count1: &count1_table,
            },
        )
        .unwrap();

        assert_eq!(
            packed,
            PackedBits {
                bytes: vec![0b1001_0110, 0b1000_0000],
                bit_len: 10,
            }
        );
        assert_eq!(granule.big_values, 2);
        assert_eq!(granule.table_select, [5, 5, 5]);
        assert!(granule.count1table_select);
        assert_eq!(granule.part2_3_length, 10);
    }

    #[test]
    fn packs_quantized_spectrum_with_scale_factors_for_granule() {
        let big_value_table = [
            HuffmanEntry {
                symbol: Layer3BigValueMagnitude::new(3, 2),
                code: HuffmanCode::new(0b10, 2).unwrap(),
            },
            HuffmanEntry {
                symbol: Layer3BigValueMagnitude::new(0, 0),
                code: HuffmanCode::new(0b0, 1).unwrap(),
            },
        ];
        let count1_table = [HuffmanEntry {
            symbol: Layer3Count1MagnitudeQuad::new(1, 1, 0, 1),
            code: HuffmanCode::new(0b11, 2).unwrap(),
        }];
        let scale_factors = PackedBits {
            bytes: vec![0b1100_0000],
            bit_len: 2,
        };
        let quantized = [3, -2, 0, 0, 1, -1, 0, 1, 0, 0];
        let mut granule = Layer3GranuleChannelInfo::default();

        let packed = pack_quantized_spectrum_with_scale_factors_for_granule(
            &mut granule,
            scale_factors,
            &quantized,
            Layer3EntropyTables {
                big_values: &big_value_table,
                count1: &count1_table,
            },
        )
        .unwrap();

        assert_eq!(
            packed,
            PackedBits {
                bytes: vec![0b1110_0101, 0b1010_0000],
                bit_len: 12,
            }
        );
        assert_eq!(granule.big_values, 2);
        assert_eq!(granule.table_select, [5, 5, 5]);
        assert!(granule.count1table_select);
        assert_eq!(granule.part2_3_length, 12);
    }

    #[test]
    fn packs_quantized_spectrum_with_table_provider() {
        let big_value_table_5 = [
            HuffmanEntry {
                symbol: Layer3BigValueMagnitude::new(3, 2),
                code: HuffmanCode::new(0b10, 2).unwrap(),
            },
            HuffmanEntry {
                symbol: Layer3BigValueMagnitude::new(0, 0),
                code: HuffmanCode::new(0b0, 1).unwrap(),
            },
        ];
        let count1_table_1 = [HuffmanEntry {
            symbol: Layer3Count1MagnitudeQuad::new(1, 1, 0, 1),
            code: HuffmanCode::new(0b11, 2).unwrap(),
        }];
        let quantized = [3, -2, 0, 0, 1, -1, 0, 1, 0, 0];
        let mut granule = Layer3GranuleChannelInfo::default();

        let packed = pack_quantized_spectrum_with_table_provider(
            &mut granule,
            &quantized,
            Layer3EntropyTableProvider {
                big_value_table_5: &big_value_table_5,
                count1_table_1: &count1_table_1,
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(
            packed,
            PackedBits {
                bytes: vec![0b1001_0110, 0b1000_0000],
                bit_len: 10,
            }
        );
        assert_eq!(granule.table_select, [5, 0, 0]);
        assert!(granule.count1table_select);

        let err = pack_quantized_spectrum_with_table_provider(
            &mut Layer3GranuleChannelInfo::default(),
            &quantized,
            Layer3EntropyTableProvider::default(),
        )
        .unwrap_err();
        assert!(matches!(
            err,
            Error::UnsupportedFeature("MP3 big-values Huffman table")
        ));
    }

    #[test]
    fn table_provider_selects_big_value_tables_per_region() {
        let big_value_table_1 = [HuffmanEntry {
            symbol: Layer3BigValueMagnitude::new(1, 0),
            code: HuffmanCode::new(0b0, 1).unwrap(),
        }];
        let big_value_table_5 = [HuffmanEntry {
            symbol: Layer3BigValueMagnitude::new(3, 2),
            code: HuffmanCode::new(0b10, 2).unwrap(),
        }];
        let big_value_table_7 = [HuffmanEntry {
            symbol: Layer3BigValueMagnitude::new(5, 4),
            code: HuffmanCode::new(0b11, 2).unwrap(),
        }];
        // The big-value regions split at the fixed scalefactor-band boundaries
        // into 2 + 2 + remainder pairs, so lay out one homogeneous value per
        // region to exercise distinct per-region table selection.
        let mut quantized = Vec::new();
        for _ in 0..2 {
            quantized.extend_from_slice(&[1, 0]);
        }
        for _ in 0..2 {
            quantized.extend_from_slice(&[3, -2]);
        }
        for _ in 0..2 {
            quantized.extend_from_slice(&[5, 4]);
        }
        let mut granule = Layer3GranuleChannelInfo::default();

        let packed = pack_quantized_spectrum_with_table_provider(
            &mut granule,
            &quantized,
            Layer3EntropyTableProvider {
                big_value_table_1: &big_value_table_1,
                big_value_table_5: &big_value_table_5,
                big_value_table_7: &big_value_table_7,
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(granule.big_values, 6);
        assert_eq!(granule.region0_count, 0);
        assert_eq!(granule.region1_count, 0);
        assert_eq!(granule.table_select, [1, 5, 7]);
        assert!(!granule.count1table_select);
        // region0: 2x[1,0] = 2*(1 code + 1 sign); region1: 2x[3,-2] =
        // 2*(2 code + 2 signs); region2: 2x[5,4] = 2*(2 code + 2 signs).
        assert_eq!(granule.part2_3_length, 20);
        assert_eq!(packed.bit_len, 20);
    }

    #[test]
    fn table_provider_prefers_shorter_available_big_value_table() {
        let big_value_table_1 = [HuffmanEntry {
            symbol: Layer3BigValueMagnitude::new(1, 0),
            code: HuffmanCode::new(0b1111, 4).unwrap(),
        }];
        let big_value_table_5 = [
            HuffmanEntry {
                symbol: Layer3BigValueMagnitude::new(1, 0),
                code: HuffmanCode::new(0b0, 1).unwrap(),
            },
            HuffmanEntry {
                symbol: Layer3BigValueMagnitude::new(2, 0),
                code: HuffmanCode::new(0b10, 2).unwrap(),
            },
        ];
        let mut quantized = Vec::new();
        for _ in 0..7 {
            quantized.extend_from_slice(&[1, 0]);
        }
        quantized.extend_from_slice(&[2, 0]);
        let mut granule = Layer3GranuleChannelInfo::default();
        let pairs =
            big_value_pairs(&quantized, plan_spectral_regions(&quantized).unwrap()).unwrap();

        assert_eq!(
            select_big_value_region_tables_by_bit_cost(
                &pairs,
                7,
                1,
                Layer3EntropyTableProvider {
                    big_value_table_1: &big_value_table_1,
                    big_value_table_5: &big_value_table_5,
                    ..Default::default()
                },
            )
            .unwrap()
            .regions
            .map(|selection| selection.table_select),
            [5, 5, 0]
        );

        let packed = pack_quantized_spectrum_with_table_provider(
            &mut granule,
            &quantized,
            Layer3EntropyTableProvider {
                big_value_table_1: &big_value_table_1,
                big_value_table_5: &big_value_table_5,
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(granule.big_values, 8);
        // With the fixed 2 + 2 + remainder split the `[2,0]` pair falls in
        // region2, so every region needs table 5 (table 1 cannot code it).
        assert_eq!(granule.table_select, [5, 5, 5]);
        assert_eq!(granule.part2_3_length, 17);
        assert_eq!(packed.bit_len, 17);
    }

    #[test]
    fn table_provider_prefers_shorter_available_count1_table() {
        let count1_table_0 = [HuffmanEntry {
            symbol: Layer3Count1MagnitudeQuad::new(1, 1, 0, 1),
            code: HuffmanCode::new(0b1111, 4).unwrap(),
        }];
        let count1_table_1 = [HuffmanEntry {
            symbol: Layer3Count1MagnitudeQuad::new(1, 1, 0, 1),
            code: HuffmanCode::new(0b0, 1).unwrap(),
        }];
        let quantized = [1, -1, 0, 1, 0, 0];
        let mut granule = Layer3GranuleChannelInfo::default();

        let packed = pack_quantized_spectrum_with_table_provider(
            &mut granule,
            &quantized,
            Layer3EntropyTableProvider {
                count1_table_0: &count1_table_0,
                count1_table_1: &count1_table_1,
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(granule.big_values, 0);
        assert_eq!(granule.table_select, [0, 0, 0]);
        assert!(granule.count1table_select);
        assert_eq!(granule.part2_3_length, 4);
        assert_eq!(packed.bit_len, 4);
    }

    #[test]
    fn experimental_unit_provider_packs_nonzero_big_values_and_count1() {
        let provider = experimental_unit_magnitude_table_provider();
        let big_value_pairs = [
            Layer3BigValuePair::new(1, -1),
            Layer3BigValuePair::new(0, 0),
        ];

        let big_value_selection =
            select_big_value_region_tables_by_bit_cost(&big_value_pairs, 1, 0, provider).unwrap();
        let big_value_bits = pack_big_value_pairs_with_region_tables_and_provider(
            &big_value_pairs,
            big_value_selection,
            provider,
        )
        .unwrap();

        assert_eq!(big_value_selection.regions[0].table_select, 1);
        assert_eq!(big_value_selection.regions[1].table_select, 0);
        assert_eq!(big_value_selection.regions[2].table_select, 0);
        assert_eq!(big_value_bits.bit_len, 5);

        let count1_quads = [Layer3Count1Quad::new(1, 0, -1, 1)];
        let count1_selection = select_count1_table_by_bit_cost(&count1_quads, provider).unwrap();
        let count1_bits = pack_count1_quads_with_sign_bits(
            &count1_quads,
            provider.count1_table(count1_selection).unwrap(),
        )
        .unwrap();

        assert!(!count1_selection.table_select);
        assert_eq!(count1_bits.bit_len, 7);
    }

    #[test]
    fn standard_provider_packs_table_1_and_count1_codewords() {
        let provider = mpeg1_layer3_standard_table_provider();
        let pairs = [
            Layer3BigValuePair::new(0, 0),
            Layer3BigValuePair::new(0, 1),
            Layer3BigValuePair::new(-1, 0),
            Layer3BigValuePair::new(1, -1),
        ];
        let selection = select_big_value_region_tables_by_bit_cost(&pairs, 4, 0, provider).unwrap();
        let packed =
            pack_big_value_pairs_with_region_tables_and_provider(&pairs, selection, provider)
                .unwrap();

        assert_eq!(selection.regions[0].table_select, 1);
        assert_eq!(packed.bit_len, 13);
        assert_eq!(packed.bytes, [0b1001_0011, 0b0000_1000]);

        let sparse_count1 = [Layer3Count1Quad::new(1, 0, 0, 0)];
        let sparse_selection = select_count1_table_by_bit_cost(&sparse_count1, provider).unwrap();
        let sparse_packed = pack_count1_quads_with_sign_bits(
            &sparse_count1,
            provider.count1_table(sparse_selection).unwrap(),
        )
        .unwrap();
        assert!(!sparse_selection.table_select);
        assert_eq!(sparse_packed.bit_len, 5);
        assert_eq!(sparse_packed.bytes, [0b0111_0000]);

        let dense_count1 = [Layer3Count1Quad::new(1, 1, 1, 1)];
        let dense_selection = select_count1_table_by_bit_cost(&dense_count1, provider).unwrap();
        let dense_packed = pack_count1_quads_with_sign_bits(
            &dense_count1,
            provider.count1_table(dense_selection).unwrap(),
        )
        .unwrap();
        assert!(dense_selection.table_select);
        assert_eq!(dense_packed.bit_len, 5);
        assert_eq!(dense_packed.bytes, [0b1000_0000]);
    }

    #[test]
    fn standard_provider_packs_table_2_big_value_codewords() {
        let provider = mpeg1_layer3_standard_table_provider();
        let pairs = [
            Layer3BigValuePair::new(2, 0),
            Layer3BigValuePair::new(0, -2),
            Layer3BigValuePair::new(-2, 2),
        ];

        let selection = select_big_value_region_tables_by_bit_cost(&pairs, 3, 0, provider).unwrap();
        let packed =
            pack_big_value_pairs_with_region_tables_and_provider(&pairs, selection, provider)
                .unwrap();

        assert_eq!(selection.regions[0].table_select, 2);
        assert_eq!(selection.regions[0].linbits, 0);
        assert_eq!(selection.regions[0].max_magnitude, 2);
        assert_eq!(packed.bit_len, 5 + 1 + 6 + 1 + 6 + 2);
        assert_eq!(packed.bytes, [0b0001_1000, 0b0001_1000, 0b0001_0000]);
    }

    #[test]
    fn standard_provider_packs_count1_only_quantized_spectrum() {
        let provider = mpeg1_layer3_standard_table_provider();
        let mut granule = Layer3GranuleChannelInfo::default();

        let packed =
            pack_quantized_spectrum_with_table_provider(&mut granule, &[1, 1, 1, 1], provider)
                .unwrap();

        assert_eq!(granule.big_values, 0);
        assert!(granule.count1table_select);
        assert_eq!(granule.part2_3_length, 5);
        assert_eq!(packed.bit_len, 5);
        assert_eq!(packed.bytes, [0b1000_0000]);
    }

    #[test]
    fn standard_big_value_provider_alias_includes_count1_tables() {
        let provider = mpeg1_layer3_standard_big_value_table_provider();
        let selection =
            select_count1_table_by_bit_cost(&[Layer3Count1Quad::new(1, 1, 1, 1)], provider)
                .unwrap();

        assert!(selection.table_select);
        assert!(provider.count1_table(selection).is_ok());
    }

    #[test]
    fn packs_quantized_spectrum_with_scale_factors_and_table_provider() {
        let big_value_table_5 = [
            HuffmanEntry {
                symbol: Layer3BigValueMagnitude::new(3, 2),
                code: HuffmanCode::new(0b10, 2).unwrap(),
            },
            HuffmanEntry {
                symbol: Layer3BigValueMagnitude::new(0, 0),
                code: HuffmanCode::new(0b0, 1).unwrap(),
            },
        ];
        let count1_table_1 = [HuffmanEntry {
            symbol: Layer3Count1MagnitudeQuad::new(1, 1, 0, 1),
            code: HuffmanCode::new(0b11, 2).unwrap(),
        }];
        let scale_factors = PackedBits {
            bytes: vec![0b1000_0000],
            bit_len: 1,
        };
        let quantized = [3, -2, 0, 0, 1, -1, 0, 1, 0, 0];
        let mut granule = Layer3GranuleChannelInfo::default();

        let packed = pack_quantized_spectrum_with_scale_factors_and_table_provider(
            &mut granule,
            scale_factors,
            &quantized,
            Layer3EntropyTableProvider {
                big_value_table_5: &big_value_table_5,
                count1_table_1: &count1_table_1,
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(
            packed,
            PackedBits {
                bytes: vec![0b1100_1011, 0b0100_0000],
                bit_len: 11,
            }
        );
        assert_eq!(granule.part2_3_length, 11);
        assert_eq!(granule.table_select, [5, 0, 0]);
        assert!(granule.count1table_select);
    }

    #[test]
    fn packs_mp3_big_value_pairs_from_table() {
        let table = [
            HuffmanEntry {
                symbol: Layer3BigValuePair::new(0, 0),
                code: HuffmanCode::new(0b0, 1).unwrap(),
            },
            HuffmanEntry {
                symbol: Layer3BigValuePair::new(2, -1),
                code: HuffmanCode::new(0b10, 2).unwrap(),
            },
            HuffmanEntry {
                symbol: Layer3BigValuePair::new(-3, 1),
                code: HuffmanCode::new(0b111, 3).unwrap(),
            },
        ];
        let pairs = [
            Layer3BigValuePair::new(2, -1),
            Layer3BigValuePair::new(0, 0),
            Layer3BigValuePair::new(-3, 1),
        ];

        assert_eq!(
            pack_big_value_pairs_with_table(&pairs, &table).unwrap(),
            PackedBits {
                bytes: vec![0b1001_1100],
                bit_len: 6,
            }
        );
        assert!(pack_big_value_pairs_with_table(&[Layer3BigValuePair::new(4, 4)], &table).is_err());
    }

    #[test]
    fn packs_mp3_big_value_pairs_with_sign_bits() {
        let table = [
            HuffmanEntry {
                symbol: Layer3BigValueMagnitude::new(0, 0),
                code: HuffmanCode::new(0b0, 1).unwrap(),
            },
            HuffmanEntry {
                symbol: Layer3BigValueMagnitude::new(2, 1),
                code: HuffmanCode::new(0b10, 2).unwrap(),
            },
            HuffmanEntry {
                symbol: Layer3BigValueMagnitude::new(3, 1),
                code: HuffmanCode::new(0b111, 3).unwrap(),
            },
        ];
        let pairs = [
            Layer3BigValuePair::new(2, -1),
            Layer3BigValuePair::new(0, 0),
            Layer3BigValuePair::new(-3, 1),
        ];

        assert_eq!(
            pack_big_value_pairs_with_sign_bits(&pairs, &table).unwrap(),
            PackedBits {
                bytes: vec![0b1001_0111, 0b1000_0000],
                bit_len: 10,
            }
        );
        assert!(
            pack_big_value_pairs_with_sign_bits(&[Layer3BigValuePair::new(4, 4)], &table).is_err()
        );
    }

    #[test]
    fn packs_mp3_big_value_pairs_with_linbits() {
        let table = [
            HuffmanEntry {
                symbol: Layer3BigValueMagnitude::new(15, 15),
                code: HuffmanCode::new(0b10, 2).unwrap(),
            },
            HuffmanEntry {
                symbol: Layer3BigValueMagnitude::new(1, 15),
                code: HuffmanCode::new(0b111, 3).unwrap(),
            },
        ];
        let pairs = [
            Layer3BigValuePair::new(18, -15),
            Layer3BigValuePair::new(-1, 16),
        ];

        // Escape linbits and signs interleave per value: code, linbits_x,
        // sign_x, linbits_y, sign_y. Pair (18,-15): `10` `0011` `0` `0000` `1`;
        // pair (-1,16): `111` `1` `0001` `0`.
        assert_eq!(
            pack_big_value_pairs_with_linbits(&pairs, &table, 4).unwrap(),
            PackedBits {
                bytes: vec![0b1000_1100, 0b0001_1111, 0b0001_0000],
                bit_len: 21,
            }
        );
        assert!(
            pack_big_value_pairs_with_linbits(&[Layer3BigValuePair::new(32, 0)], &table, 4)
                .is_err()
        );
        assert!(pack_big_value_pairs_with_linbits(&pairs, &table, 17).is_err());
    }

    #[test]
    fn packs_mp3_count1_quads_from_table() {
        let table = [
            HuffmanEntry {
                symbol: Layer3Count1Quad::new(0, 0, 0, 0),
                code: HuffmanCode::new(0b0, 1).unwrap(),
            },
            HuffmanEntry {
                symbol: Layer3Count1Quad::new(1, -1, 0, 1),
                code: HuffmanCode::new(0b10, 2).unwrap(),
            },
            HuffmanEntry {
                symbol: Layer3Count1Quad::new(-1, 0, 1, 0),
                code: HuffmanCode::new(0b111, 3).unwrap(),
            },
        ];
        let quads = [
            Layer3Count1Quad::new(1, -1, 0, 1),
            Layer3Count1Quad::new(0, 0, 0, 0),
            Layer3Count1Quad::new(-1, 0, 1, 0),
        ];

        assert_eq!(
            pack_count1_quads_with_table(&quads, &table).unwrap(),
            PackedBits {
                bytes: vec![0b1001_1100],
                bit_len: 6,
            }
        );
        assert!(
            pack_count1_quads_with_table(&[Layer3Count1Quad::new(1, 1, 1, 1)], &table).is_err()
        );
    }

    #[test]
    fn packs_mp3_count1_quads_with_sign_bits() {
        let table = [
            HuffmanEntry {
                symbol: Layer3Count1MagnitudeQuad::new(0, 0, 0, 0),
                code: HuffmanCode::new(0b0, 1).unwrap(),
            },
            HuffmanEntry {
                symbol: Layer3Count1MagnitudeQuad::new(1, 1, 0, 1),
                code: HuffmanCode::new(0b10, 2).unwrap(),
            },
            HuffmanEntry {
                symbol: Layer3Count1MagnitudeQuad::new(1, 0, 1, 0),
                code: HuffmanCode::new(0b111, 3).unwrap(),
            },
        ];
        let quads = [
            Layer3Count1Quad::new(1, -1, 0, 1),
            Layer3Count1Quad::new(0, 0, 0, 0),
            Layer3Count1Quad::new(-1, 0, 1, 0),
        ];

        assert_eq!(
            pack_count1_quads_with_sign_bits(&quads, &table).unwrap(),
            PackedBits {
                bytes: vec![0b1001_0011, 0b1100_0000],
                bit_len: 11,
            }
        );
        assert!(
            pack_count1_quads_with_sign_bits(&[Layer3Count1Quad::new(2, 0, 0, 0)], &table).is_err()
        );
    }

    #[test]
    fn encodes_silent_mono_pcm_as_layer3_frames() {
        let pcm = AudioBuffer::new(44_100, 1, vec![0.0; 1152]).unwrap();

        let mp3 = encode(&pcm).unwrap();
        let header = FrameHeader::parse(&mp3[..4]).unwrap();

        assert_eq!(detect(&mp3), Some(Format::Mp3));
        assert_eq!(header.version, MpegVersion::Mpeg1);
        assert_eq!(header.layer, Layer::Layer3);
        assert_eq!(header.bitrate_kbps, 128);
        assert_eq!(header.sample_rate, 44_100);
        assert_eq!(header.channel_mode, ChannelMode::SingleChannel);
        assert_eq!(mp3.len(), header.frame_len());
    }

    #[test]
    fn encodes_silent_stereo_pcm_as_multiple_layer3_frames() {
        let pcm = AudioBuffer::new(48_000, 2, vec![0.0; 1153 * 2]).unwrap();

        let mp3 = encode(&pcm).unwrap();
        let header = FrameHeader::parse(&mp3[..4]).unwrap();

        assert_eq!(header.sample_rate, 48_000);
        assert_eq!(header.channel_mode, ChannelMode::Stereo);
        assert_eq!(mp3.len(), header.frame_len() * 2);
        assert_eq!(
            FrameHeader::parse(&mp3[header.frame_len()..header.frame_len() + 4]).unwrap(),
            header
        );
    }

    #[test]
    fn encodes_silent_pcm_with_experimental_frame_scaffold() {
        let pcm = AudioBuffer::new(44_100, 2, vec![0.0; 1153 * 2]).unwrap();
        let expected = encode(&pcm).unwrap();

        let table_encoded = encode_mpeg1_layer3_pcm_frames_with_selected_scale_factors(
            &pcm,
            1.0,
            Layer3EntropyTables {
                big_values: &[],
                count1: &[],
            },
        )
        .unwrap();
        let provider_encoded =
            encode_mpeg1_layer3_pcm_frames_with_selected_scale_factors_and_table_provider(
                &pcm,
                1.0,
                Layer3EntropyTableProvider::default(),
            )
            .unwrap();

        assert_eq!(table_encoded, expected);
        assert_eq!(provider_encoded, expected);
    }

    #[test]
    fn encodes_silent_pcm_with_explicit_experimental_header() {
        let header = FrameHeader {
            version: MpegVersion::Mpeg1,
            layer: Layer::Layer3,
            protection_absent: true,
            bitrate_kbps: 128,
            sample_rate: 44_100,
            padding: false,
            channel_mode: ChannelMode::SingleChannel,
        };
        let pcm = AudioBuffer::new(44_100, 1, vec![0.0; 1153]).unwrap();
        let expected = encode(&pcm).unwrap();

        let encoded = encode_mpeg1_layer3_pcm_frames_with_header_and_selected_scale_factors(
            header,
            &pcm,
            1.0,
            Layer3EntropyTables {
                big_values: &[],
                count1: &[],
            },
        )
        .unwrap();
        let provider_encoded =
            encode_mpeg1_layer3_pcm_frames_with_header_and_selected_scale_factors_and_table_provider(
                header,
                &pcm,
                1.0,
                Layer3EntropyTableProvider::default(),
            )
            .unwrap();

        assert_eq!(encoded, expected);
        assert_eq!(provider_encoded, expected);

        let stereo_header = FrameHeader {
            channel_mode: ChannelMode::Stereo,
            ..header
        };
        assert!(
            encode_mpeg1_layer3_pcm_frames_with_header_and_selected_scale_factors(
                stereo_header,
                &pcm,
                1.0,
                Layer3EntropyTables {
                    big_values: &[],
                    count1: &[],
                },
            )
            .is_err()
        );
    }

    #[test]
    fn selects_pcm_frame_step_for_standard_nonzero_payload() {
        let pcm = AudioBuffer::new(
            44_100,
            1,
            (0..1152)
                .map(|sample| ((sample as f32) * 0.01).sin() * 0.25)
                .collect(),
        )
        .unwrap();
        let header = FrameHeader {
            version: MpegVersion::Mpeg1,
            layer: Layer::Layer3,
            protection_absent: true,
            bitrate_kbps: 128,
            sample_rate: 44_100,
            padding: false,
            channel_mode: ChannelMode::SingleChannel,
        };
        let provider = mpeg1_layer3_standard_table_provider();

        let step = select_mpeg1_layer3_pcm_frame_step_with_table_provider(
            header,
            &pcm,
            0,
            MPEG1_LAYER3_PCM_STEP_CANDIDATES,
            provider,
        )
        .unwrap();
        let reversed_candidates = MPEG1_LAYER3_PCM_STEP_CANDIDATES
            .iter()
            .copied()
            .rev()
            .collect::<Vec<_>>();
        let details: Layer3PcmFrameStepSelection =
            select_mpeg1_layer3_pcm_frame_step_details_with_table_provider(
                header,
                &pcm,
                0,
                &reversed_candidates,
                provider,
            )
            .unwrap();
        let auto = encode_mpeg1_layer3_pcm_frames_with_auto_step_and_table_provider(
            &pcm,
            MPEG1_LAYER3_PCM_STEP_CANDIDATES,
            provider,
        )
        .unwrap();
        let selected =
            encode_mpeg1_layer3_pcm_frames_with_selected_scale_factors_and_table_provider(
                &pcm, step, provider,
            )
            .unwrap();
        let zero_payload =
            encode_mpeg1_layer3_pcm_frames_with_selected_scale_factors_and_table_provider(
                &pcm,
                f32::MAX,
                Layer3EntropyTableProvider::default(),
            )
            .unwrap();

        assert!(step < f32::MAX);
        assert_eq!(details.step, step);
        assert!(details.payload_bit_len > 0);
        assert!(details.payload_bit_len <= details.frame_capacity_bits);
        assert_eq!(auto, selected);
        assert_ne!(auto, zero_payload);
    }

    #[test]
    fn selects_pcm_frame_step_with_max_payload_bits() {
        let pcm = AudioBuffer::new(
            44_100,
            1,
            (0..1152)
                .map(|sample| ((sample as f32) * 0.01).sin() * 0.25)
                .collect(),
        )
        .unwrap();
        let header = FrameHeader {
            version: MpegVersion::Mpeg1,
            layer: Layer::Layer3,
            protection_absent: true,
            bitrate_kbps: 128,
            sample_rate: 44_100,
            padding: false,
            channel_mode: ChannelMode::SingleChannel,
        };
        let provider = mpeg1_layer3_standard_table_provider();
        let unconstrained = select_mpeg1_layer3_pcm_frame_step_details_with_table_provider(
            header,
            &pcm,
            0,
            MPEG1_LAYER3_PCM_STEP_CANDIDATES,
            provider,
        )
        .unwrap();
        let positive_payload_selections = MPEG1_LAYER3_PCM_STEP_CANDIDATES
            .iter()
            .copied()
            .filter_map(|candidate| {
                select_mpeg1_layer3_pcm_frame_step_details_with_table_provider(
                    header,
                    &pcm,
                    0,
                    &[candidate],
                    provider,
                )
                .ok()
            })
            .filter(|selection| selection.payload_bit_len > 0)
            .collect::<Vec<_>>();
        let budget = positive_payload_selections
            .iter()
            .filter(|selection| selection.step > unconstrained.step)
            .map(|selection| selection.payload_bit_len)
            .min()
            .expect("at least one coarser positive-payload MP3 step candidate");
        let min_positive_budget = positive_payload_selections
            .iter()
            .map(|selection| selection.payload_bit_len)
            .min()
            .unwrap();
        let positive_payload_candidates = positive_payload_selections
            .iter()
            .map(|selection| selection.step)
            .collect::<Vec<_>>();

        let step = select_mpeg1_layer3_pcm_frame_step_with_max_payload_bits_and_table_provider(
            header,
            &pcm,
            0,
            MPEG1_LAYER3_PCM_STEP_CANDIDATES,
            budget,
            provider,
        )
        .unwrap();
        let details =
            select_mpeg1_layer3_pcm_frame_step_details_with_max_payload_bits_and_table_provider(
                header,
                &pcm,
                0,
                MPEG1_LAYER3_PCM_STEP_CANDIDATES,
                budget,
                provider,
            )
            .unwrap();

        assert_eq!(step, details.step);
        assert!(details.step > unconstrained.step);
        assert_eq!(details.frame_capacity_bits, budget);
        assert!(details.payload_bit_len <= budget);
        assert!(
            select_mpeg1_layer3_pcm_frame_step_details_with_max_payload_bits_and_table_provider(
                header,
                &pcm,
                0,
                &positive_payload_candidates,
                min_positive_budget - 1,
                provider,
            )
            .is_err()
        );
    }

    #[test]
    fn encodes_pcm_frames_with_max_payload_bits() {
        let pcm = AudioBuffer::new(
            44_100,
            1,
            (0..2304)
                .map(|sample| ((sample as f32) * 0.01).sin() * 0.25)
                .collect(),
        )
        .unwrap();
        let header = FrameHeader {
            version: MpegVersion::Mpeg1,
            layer: Layer::Layer3,
            protection_absent: true,
            bitrate_kbps: 128,
            sample_rate: 44_100,
            padding: false,
            channel_mode: ChannelMode::SingleChannel,
        };
        let provider = mpeg1_layer3_standard_table_provider();
        let first_frame = select_mpeg1_layer3_pcm_frame_step_details_with_table_provider(
            header,
            &pcm,
            0,
            MPEG1_LAYER3_PCM_STEP_CANDIDATES,
            provider,
        )
        .unwrap();
        let budget = first_frame.payload_bit_len;
        let step = select_mpeg1_layer3_pcm_frame_step_with_max_payload_bits_and_table_provider(
            header,
            &pcm,
            0,
            MPEG1_LAYER3_PCM_STEP_CANDIDATES,
            budget,
            provider,
        )
        .unwrap();

        let budgeted = encode_mpeg1_layer3_pcm_frames_with_max_payload_bits_and_table_provider(
            &pcm,
            MPEG1_LAYER3_PCM_STEP_CANDIDATES,
            budget,
            provider,
        )
        .unwrap();
        let budgeted_with_header =
            encode_mpeg1_layer3_pcm_frames_with_header_and_max_payload_bits_and_table_provider(
                header,
                &pcm,
                MPEG1_LAYER3_PCM_STEP_CANDIDATES,
                budget,
                provider,
            )
            .unwrap();
        let selected =
            encode_mpeg1_layer3_pcm_frames_with_selected_scale_factors_and_table_provider(
                &pcm, step, provider,
            )
            .unwrap();

        assert_eq!(budgeted, budgeted_with_header);
        // The budget path and the explicit-step path agree on the first frame,
        // since `step` is the budgeted step selected for frame 0. Later frames
        // carry distinct spectra and may select a different per-frame step.
        let frame_len = header.frame_len();
        assert_eq!(budgeted[..frame_len], selected[..frame_len]);
        assert_eq!(budgeted.len(), selected.len());
        assert_eq!(budgeted.len(), header.frame_len() * 2);
        assert!(
            encode_mpeg1_layer3_pcm_frames_with_max_payload_bits_and_table_provider(
                &pcm,
                MPEG1_LAYER3_PCM_STEP_CANDIDATES,
                0,
                provider,
            )
            .is_err()
        );
    }

    #[test]
    fn encodes_pcm_frames_with_bitrate_helper() {
        let pcm = AudioBuffer::new(
            44_100,
            1,
            (0..1152)
                .map(|sample| ((sample as f32) * 0.01).sin() * 0.25)
                .collect(),
        )
        .unwrap();
        let provider = mpeg1_layer3_standard_table_provider();
        let header = layer3_header_for_capacity(44_100, 1, 96, false, false).unwrap();

        let encoded = encode_mpeg1_layer3_pcm_frames_with_bitrate_and_table_provider(
            &pcm,
            MPEG1_LAYER3_PCM_STEP_CANDIDATES,
            96,
            false,
            false,
            provider,
        )
        .unwrap();
        let explicit = encode_mpeg1_layer3_pcm_frames_with_header_and_auto_step_and_table_provider(
            header,
            &pcm,
            MPEG1_LAYER3_PCM_STEP_CANDIDATES,
            provider,
        )
        .unwrap();
        let parsed = FrameHeader::parse(&encoded[..4]).unwrap();

        assert_eq!(encoded, explicit);
        assert_eq!(parsed, header);
        assert_eq!(parsed.bitrate_kbps, 96);
        assert_eq!(encoded.len(), header.frame_len());
        assert!(
            encode_mpeg1_layer3_pcm_frames_with_bitrate_and_table_provider(
                &pcm,
                MPEG1_LAYER3_PCM_STEP_CANDIDATES,
                123,
                false,
                false,
                provider,
            )
            .is_err()
        );
    }

    #[test]
    fn encodes_pcm_frames_with_cbr_bitrate_padding_schedule() {
        let pcm = AudioBuffer::new(
            44_100,
            1,
            (0..(1152 * 3))
                .map(|sample| ((sample as f32) * 0.01).sin() * 0.25)
                .collect(),
        )
        .unwrap();
        let provider = mpeg1_layer3_standard_table_provider();
        let unpadded_header = layer3_header_for_capacity(44_100, 1, 128, false, false).unwrap();
        let padded_header = layer3_header_for_capacity(44_100, 1, 128, true, false).unwrap();

        let cbr = encode_mpeg1_layer3_pcm_frames_with_cbr_bitrate_and_table_provider(
            &pcm,
            MPEG1_LAYER3_PCM_STEP_CANDIDATES,
            128,
            false,
            provider,
        )
        .unwrap();
        let fixed = encode_mpeg1_layer3_pcm_frames_with_bitrate_and_table_provider(
            &pcm,
            MPEG1_LAYER3_PCM_STEP_CANDIDATES,
            128,
            false,
            false,
            provider,
        )
        .unwrap();

        let first = FrameHeader::parse(&cbr[..4]).unwrap();
        let second_offset = first.frame_len();
        let second = FrameHeader::parse(&cbr[second_offset..second_offset + 4]).unwrap();
        let third_offset = second_offset + second.frame_len();
        let third = FrameHeader::parse(&cbr[third_offset..third_offset + 4]).unwrap();

        assert_eq!(first, unpadded_header);
        assert_eq!(second, padded_header);
        assert_eq!(third, padded_header);
        assert_eq!(
            cbr.len(),
            unpadded_header.frame_len() + 2 * padded_header.frame_len()
        );
        assert_eq!(fixed.len(), 3 * unpadded_header.frame_len());
        assert!(cbr.len() > fixed.len());
    }

    #[test]
    fn reservoir_encode_borrows_main_data_across_frames() {
        // Alternate broadband (expensive to quantize) and near-silent (cheap)
        // frames so the shared main-data stream builds a reservoir that later
        // frames reference: main_data_begin must climb above zero somewhere.
        let frames = 8_usize;
        let samples_per_frame = 1152_usize;
        let mut samples = Vec::with_capacity(frames * samples_per_frame);
        for frame in 0..frames {
            let loud = frame % 2 == 0;
            for n in 0..samples_per_frame {
                let t = n as f32;
                let value = if loud {
                    0.3 * ((t * 0.043).sin()
                        + (t * 0.131).sin()
                        + (t * 0.277).sin()
                        + (t * 0.611).sin())
                } else {
                    0.02 * (t * 0.05).sin()
                };
                samples.push(value);
            }
        }
        let pcm = AudioBuffer::new(44_100, 1, samples).unwrap();
        let provider = mpeg1_layer3_standard_table_provider();
        let stream = encode_mpeg1_layer3_pcm_frames_with_reservoir_and_table_provider(
            &pcm,
            MPEG1_LAYER3_PCM_STEP_CANDIDATES,
            128,
            false,
            provider,
        )
        .unwrap();

        // Walk the stream: every frame parses, the buffer is consumed exactly,
        // and main_data_begin (the first 9 side-info bits) exceeds zero on at
        // least one frame, proving cross-frame borrowing.
        let mut offset = 0_usize;
        let mut frame_count = 0_usize;
        let mut max_main_data_begin = 0_u32;
        while offset < stream.len() {
            let header = FrameHeader::parse(&stream[offset..offset + 4]).unwrap();
            let mut reader = BitReader::new(&stream[offset + 4..]);
            let main_data_begin = reader.read_bits(9).unwrap();
            max_main_data_begin = max_main_data_begin.max(main_data_begin);
            offset += header.frame_len();
            frame_count += 1;
        }
        assert_eq!(offset, stream.len(), "frames did not tile the stream");
        assert_eq!(frame_count, frames);
        assert!(
            max_main_data_begin > 0,
            "reservoir never used: main_data_begin stayed zero"
        );
        // The MPEG-1 main_data_begin pointer is 9 bits wide.
        assert!(max_main_data_begin <= 511);
    }

    #[test]
    fn decodes_own_silent_layer3_frames() {
        let pcm = AudioBuffer::new(44_100, 2, vec![0.0; 1153 * 2]).unwrap();
        let mp3 = encode(&pcm).unwrap();

        let decoded = decode(&mp3).unwrap();

        assert_eq!(decoded.sample_rate, 44_100);
        assert_eq!(decoded.channels, 2);
        assert_eq!(decoded.samples.len(), 1152 * 2 * 2);
        assert!(decoded.samples.iter().all(|sample| *sample == 0.0));
    }

    #[test]
    fn rejects_unknown_layer3_payload_for_decode() {
        let pcm = AudioBuffer::new(44_100, 1, vec![0.0; 1152]).unwrap();
        let mut mp3 = encode(&pcm).unwrap();
        *mp3.last_mut().unwrap() = 1;

        let err = decode(&mp3).unwrap_err();

        assert!(matches!(
            err,
            Error::UnsupportedFeature(
                "MP3 decode currently supports sonare silent MPEG-1 Layer III only"
            )
        ));
    }

    #[test]
    fn encodes_non_silent_pcm_as_layer3_scaffold() {
        let pcm = AudioBuffer::new(44_100, 1, vec![0.0, 0.25]).unwrap();
        let zero_payload =
            encode_mpeg1_layer3_pcm_frames_with_selected_scale_factors_and_table_provider(
                &pcm,
                f32::MAX,
                Layer3EntropyTableProvider::default(),
            )
            .unwrap();

        let mp3 = encode(&pcm).unwrap();
        let header = FrameHeader::parse(&mp3[..4]).unwrap();

        assert_eq!(detect(&mp3), Some(Format::Mp3));
        assert_eq!(header.version, MpegVersion::Mpeg1);
        assert_eq!(header.layer, Layer::Layer3);
        assert_eq!(header.channel_mode, ChannelMode::SingleChannel);
        assert_eq!(mp3.len(), header.frame_len());
        assert_ne!(mp3, zero_payload);
    }

    #[test]
    fn decodes_explicit_zero_payload_scaffold_as_zero_pcm() {
        let pcm = AudioBuffer::new(44_100, 1, vec![0.0, 0.25]).unwrap();
        let mp3 = encode_mpeg1_layer3_pcm_frames_with_selected_scale_factors_and_table_provider(
            &pcm,
            f32::MAX,
            Layer3EntropyTableProvider::default(),
        )
        .unwrap();

        let decoded = decode(&mp3).unwrap();

        assert_eq!(decoded.sample_rate, 44_100);
        assert_eq!(decoded.channels, 1);
        assert_eq!(decoded.samples.len(), 1152);
        assert!(decoded.samples.iter().all(|sample| *sample == 0.0));
    }

    #[test]
    fn rejects_unsupported_encode_shape() {
        let pcm = AudioBuffer::new(44_100, 3, vec![0.0; 3]).unwrap();

        let err = encode(&pcm).unwrap_err();

        assert!(matches!(
            err,
            Error::UnsupportedFeature("MP3 encode currently supports mono/stereo only")
        ));

        let pcm = AudioBuffer::new(22_050, 1, vec![0.0; 576]).unwrap();
        let err = encode(&pcm).unwrap_err();

        assert!(matches!(err, Error::UnsupportedFeature("MP3 sample rate")));
    }

    /// ISO/IEC 11172-3 §2.4.3.4 long-block requantization with zero scale
    /// factors and zero preflag: `xr = sign(is)·|is|^(4/3)·2^((global_gain−210)/4)`.
    fn requantize_long_line(is: i32, global_gain: u8) -> f32 {
        let sign = if is < 0 { -1.0 } else { 1.0 };
        let magnitude = (is.unsigned_abs() as f32).powf(4.0 / 3.0);
        let gain = 2.0_f32.powf(0.25 * (f32::from(global_gain) - 210.0));
        sign * magnitude * gain
    }

    #[test]
    fn global_gain_for_step_inverts_the_quantizer_step() {
        // At step == 1 the gain is the ISO reference value, and each octave of
        // step shifts the gain by 16/3 quarter-dB units.
        assert_eq!(mpeg1_layer3_global_gain_for_step(1.0), 210);
        assert_eq!(mpeg1_layer3_global_gain_for_step(2.0), 215);
        assert_eq!(mpeg1_layer3_global_gain_for_step(0.5), 205);
        // Degenerate steps fall back to the reference gain instead of panicking.
        assert_eq!(mpeg1_layer3_global_gain_for_step(0.0), 210);
        assert_eq!(mpeg1_layer3_global_gain_for_step(-1.0), 210);
        assert_eq!(mpeg1_layer3_global_gain_for_step(f32::NAN), 210);
        // The gain stays inside the 8-bit syntax range for extreme steps.
        assert_eq!(mpeg1_layer3_global_gain_for_step(f32::MIN_POSITIVE), 0);
        assert_eq!(mpeg1_layer3_global_gain_for_step(1.0e30), 255);
    }

    #[test]
    fn calibrated_gain_requantizes_the_long_block_spectrum() {
        // A non-periodic frequency sweep exercises every scale-factor band. The
        // encoder quantizes the (sign-inverted) spectrum, so the ISO
        // requantization with the calibrated gain and zero scale factors must
        // reconstruct that same signal within quantization noise (positive
        // SNR), and finer steps must not regress.
        let samples: Vec<f32> = (0..2304)
            .map(|n| {
                let t = n as f32 / 44_100.0;
                let f = 200.0 + 6000.0 * t;
                0.6 * (std::f32::consts::TAU * f * t).sin()
            })
            .collect();
        let pcm = AudioBuffer::new(44_100, 1, samples).unwrap();
        let spectrum = layer3_long_block_spectrum(&pcm, 0, 576).unwrap();

        let mut previous_snr = f64::NEG_INFINITY;
        for &step in &[1.0_f32, 0.25, 0.05] {
            let global_gain = mpeg1_layer3_global_gain_for_step(step);
            let quantized = quantize_pcm_long_block(&pcm, 0, 576, step).unwrap();

            let mut signal = 0.0_f64;
            let mut noise = 0.0_f64;
            for (&line, &is) in spectrum.iter().zip(quantized.iter()) {
                // The encoder quantizes the negated spectrum.
                let reference = f64::from(-line);
                let reconstructed = f64::from(requantize_long_line(is, global_gain));
                signal += reference * reference;
                let error = reconstructed - reference;
                noise += error * error;
            }

            let snr = 10.0 * (signal / noise.max(1.0e-30)).log10();
            assert!(
                snr > 10.0,
                "step {step} reconstruction SNR too low: {snr} dB"
            );
            assert!(
                snr >= previous_snr - 0.5,
                "finer step {step} regressed SNR: {snr} dB vs {previous_snr} dB"
            );
            previous_snr = snr;
        }
    }
}
