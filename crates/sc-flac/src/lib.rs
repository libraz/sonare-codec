#![deny(unsafe_code)]
#![warn(clippy::all)]

use md5::{Digest, Md5};
use sc_core::{AudioBuffer, BitReader, Decoder, Encoder, Error};

const FLAC_MARKER: &[u8; 4] = b"fLaC";
const METADATA_HEADER_LEN: usize = 4;
const STREAMINFO_BLOCK_TYPE: u8 = 0;
const STREAMINFO_LEN: usize = 34;
const ENCODE_BITS_PER_SAMPLE: u8 = 16;
const ENCODE_BLOCK_SIZE: usize = 4096;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StreamInfo {
    pub min_block_size: u16,
    pub max_block_size: u16,
    pub min_frame_size: u32,
    pub max_frame_size: u32,
    pub sample_rate: u32,
    pub channels: u8,
    pub bits_per_sample: u8,
    pub total_samples: u64,
    pub md5: [u8; 16],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BlockingStrategy {
    FixedBlockSize,
    VariableBlockSize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChannelAssignment {
    Independent(u8),
    LeftSide,
    RightSide,
    MidSide,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FrameHeader {
    pub blocking_strategy: BlockingStrategy,
    pub block_size: u32,
    pub sample_rate: u32,
    pub channel_assignment: ChannelAssignment,
    pub bits_per_sample: u8,
    pub frame_or_sample_number: u64,
    pub header_len: usize,
}

#[derive(Default)]
pub struct FlacDecoder {
    pending: Vec<u8>,
}

impl FlacDecoder {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl Decoder for FlacDecoder {
    fn decode(&mut self, input: &[u8]) -> Result<AudioBuffer, Error> {
        decode(input)
    }

    fn decode_stream(&mut self, chunk: &[u8]) -> Result<Option<AudioBuffer>, Error> {
        if chunk.is_empty() && self.pending.is_empty() {
            return Ok(None);
        }
        self.pending.extend_from_slice(chunk);
        match decode(&self.pending) {
            Ok(buffer) => {
                self.pending.clear();
                Ok(Some(buffer))
            }
            Err(err) if is_incomplete_stream_error(&err) => Ok(None),
            Err(err) => Err(err),
        }
    }
}

#[derive(Default)]
pub struct FlacEncoder;

impl FlacEncoder {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Encoder for FlacEncoder {
    fn encode(&mut self, pcm: &AudioBuffer) -> Result<Vec<u8>, Error> {
        encode(pcm)
    }
}

pub fn decode(input: &[u8]) -> Result<AudioBuffer, Error> {
    let (stream_info, audio_start) = parse_metadata(input)?;
    let mut cursor = audio_start;
    let mut samples = Vec::new();
    let mut md5_input = Vec::new();
    let mut decoded_frames = 0_usize;
    let mut decoded_frame_count = 0_u64;

    while cursor < input.len() {
        let frame_input = input
            .get(cursor..)
            .ok_or(Error::InvalidInput("FLAC audio data is truncated"))?;
        let decoded = decode_frame(frame_input, &stream_info)?;
        if decoded.header.sample_rate != stream_info.sample_rate {
            return Err(Error::InvalidInput("FLAC frame sample rate mismatch"));
        }
        if decoded.header.channel_assignment.channels() != stream_info.channels {
            return Err(Error::InvalidInput("FLAC frame channel count mismatch"));
        }
        let decoded_frames_after_frame = decoded_frames
            .checked_add(decoded.frames)
            .ok_or(Error::InvalidInput("FLAC decoded sample count overflow"))?;
        let is_declared_last_frame = stream_info.total_samples != 0
            && u64::try_from(decoded_frames_after_frame)
                .map_err(|_| Error::InvalidInput("FLAC decoded sample count overflow"))?
                == stream_info.total_samples;
        if decoded.header.block_size > u32::from(stream_info.max_block_size) {
            return Err(Error::InvalidInput(
                "FLAC frame block size outside STREAMINFO range",
            ));
        }
        if decoded.header.block_size < u32::from(stream_info.min_block_size)
            && !is_declared_last_frame
        {
            return Err(Error::InvalidInput(
                "FLAC non-final frame block size below STREAMINFO minimum",
            ));
        }
        validate_frame_size(decoded.bytes_read, &stream_info)?;
        let expected_coded_number = match decoded.header.blocking_strategy {
            BlockingStrategy::FixedBlockSize => decoded_frame_count,
            BlockingStrategy::VariableBlockSize => u64::try_from(decoded_frames)
                .map_err(|_| Error::InvalidInput("FLAC decoded sample count overflow"))?,
        };
        if decoded.header.frame_or_sample_number != expected_coded_number {
            return Err(Error::InvalidInput("FLAC frame/sample number mismatch"));
        }

        decoded_frames = decoded_frames_after_frame;
        decoded_frame_count = decoded_frame_count
            .checked_add(1)
            .ok_or(Error::InvalidInput("FLAC decoded frame count overflow"))?;
        samples.extend(decoded.samples);
        md5_input.extend(decoded.md5_input);
        cursor = cursor
            .checked_add(decoded.bytes_read)
            .ok_or(Error::InvalidInput("FLAC frame cursor overflow"))?;
    }

    if decoded_frames == 0 {
        return Err(Error::InvalidInput("FLAC stream has no audio frames"));
    }
    if stream_info.total_samples != 0 && stream_info.total_samples != decoded_frames as u64 {
        return Err(Error::InvalidInput("FLAC decoded sample count mismatch"));
    }
    if stream_info.md5 != [0; 16] {
        let actual_md5: [u8; 16] = Md5::digest(&md5_input).into();
        if actual_md5 != stream_info.md5 {
            return Err(Error::InvalidInput("FLAC STREAMINFO MD5 mismatch"));
        }
    }

    AudioBuffer::new(
        stream_info.sample_rate,
        u16::from(stream_info.channels),
        samples,
    )
}

pub fn encode(pcm: &AudioBuffer) -> Result<Vec<u8>, Error> {
    if pcm.is_empty() {
        return Err(Error::InvalidPcm("FLAC encode requires at least one frame"));
    }
    if pcm.sample_rate == 0 || pcm.sample_rate > 0x000f_ffff {
        return Err(Error::InvalidPcm("FLAC sample rate is out of range"));
    }
    let channels = usize::from(pcm.channels);
    if channels == 0 || channels > 8 {
        return Err(Error::InvalidPcm("FLAC supports 1 to 8 channels"));
    }

    let total_frames = pcm.frames();
    let total_frames_u64 =
        u64::try_from(total_frames).map_err(|_| Error::InvalidPcm("FLAC input is too large"))?;
    if total_frames_u64 > 0x0000_000f_ffff_ffff {
        return Err(Error::InvalidPcm("FLAC total sample count is out of range"));
    }

    let mut pcm_i16 = Vec::with_capacity(pcm.samples.len());
    let mut md5_input = Vec::with_capacity(pcm.samples.len() * 2);
    for &sample in &pcm.samples {
        let quantized = quantize_i16(sample);
        pcm_i16.push(quantized);
        md5_input.extend_from_slice(&quantized.to_le_bytes());
    }
    let md5: [u8; 16] = Md5::digest(&md5_input).into();

    let block_sizes = encode_block_sizes(total_frames);
    let encoded_min_block_size = *block_sizes
        .iter()
        .min()
        .ok_or(Error::InvalidPcm("FLAC encode has no blocks"))?;
    let encoded_max_block_size = *block_sizes
        .iter()
        .max()
        .ok_or(Error::InvalidPcm("FLAC encode has no blocks"))?;
    let fixed_blocking = encoded_min_block_size == encoded_max_block_size;
    let min_block_size = u16::try_from(
        *block_sizes
            .iter()
            .min()
            .ok_or(Error::InvalidPcm("FLAC encode has no blocks"))?,
    )
    .map(|block_size| block_size.max(16))
    .map_err(|_| Error::InvalidPcm("FLAC block size is out of range"))?;
    let max_block_size = u16::try_from(
        *block_sizes
            .iter()
            .max()
            .ok_or(Error::InvalidPcm("FLAC encode has no blocks"))?,
    )
    .map(|block_size| block_size.max(16))
    .map_err(|_| Error::InvalidPcm("FLAC block size is out of range"))?;

    let mut frames = Vec::with_capacity(block_sizes.len());
    let mut sample_offset = 0_usize;
    for (frame_index, &block_size) in block_sizes.iter().enumerate() {
        let coded_number = if fixed_blocking {
            frame_index
        } else {
            sample_offset
        };
        let frame = encode_frame(
            &pcm_i16,
            channels,
            sample_offset,
            block_size,
            coded_number,
            fixed_blocking,
        )?;
        frames.push(frame);
        sample_offset = sample_offset
            .checked_add(block_size)
            .ok_or(Error::InvalidPcm("FLAC sample offset overflow"))?;
    }
    let min_frame_size = frames
        .iter()
        .map(Vec::len)
        .min()
        .ok_or(Error::InvalidPcm("FLAC encode has no frames"))?;
    let max_frame_size = frames
        .iter()
        .map(Vec::len)
        .max()
        .ok_or(Error::InvalidPcm("FLAC encode has no frames"))?;

    let mut out = Vec::new();
    out.extend_from_slice(FLAC_MARKER);
    out.push(0x80 | STREAMINFO_BLOCK_TYPE);
    out.extend_from_slice(&(STREAMINFO_LEN as u32).to_be_bytes()[1..4]);
    out.extend_from_slice(&min_block_size.to_be_bytes());
    out.extend_from_slice(&max_block_size.to_be_bytes());
    out.extend_from_slice(
        &u32::try_from(min_frame_size)
            .map_err(|_| Error::InvalidPcm("FLAC frame size is out of range"))?
            .to_be_bytes()[1..4],
    );
    out.extend_from_slice(
        &u32::try_from(max_frame_size)
            .map_err(|_| Error::InvalidPcm("FLAC frame size is out of range"))?
            .to_be_bytes()[1..4],
    );
    let packed = (u64::from(pcm.sample_rate) << 44)
        | (u64::from(pcm.channels - 1) << 41)
        | (u64::from(ENCODE_BITS_PER_SAMPLE - 1) << 36)
        | u64::try_from(total_frames)
            .map_err(|_| Error::InvalidPcm("FLAC total sample count is out of range"))?;
    out.extend_from_slice(&packed.to_be_bytes());
    out.extend_from_slice(&md5);
    for frame in frames {
        out.extend_from_slice(&frame);
    }

    Ok(out)
}

fn is_incomplete_stream_error(err: &Error) -> bool {
    match err {
        Error::InvalidInput(reason) => {
            reason.contains("truncated") || *reason == "FLAC stream has no audio frames"
        }
        Error::UnsupportedFormat | Error::InvalidPcm(_) | Error::UnsupportedFeature(_) => false,
    }
}

struct DecodedFrame {
    header: FrameHeader,
    frames: usize,
    samples: Vec<f32>,
    md5_input: Vec<u8>,
    bytes_read: usize,
}

fn decode_frame(input: &[u8], stream_info: &StreamInfo) -> Result<DecodedFrame, Error> {
    let frame = parse_frame_header(input, stream_info)?;
    let decoded_channels = frame.channel_assignment.channels();
    let frame_body = input
        .get(frame.header_len..)
        .ok_or(Error::InvalidInput("FLAC frame body is truncated"))?;
    let mut reader = BitReader::new(frame_body);
    let block_size = usize::try_from(frame.block_size)
        .map_err(|_| Error::InvalidInput("FLAC block size is too large"))?;
    let mut channel_samples = Vec::with_capacity(usize::from(decoded_channels));
    for channel_index in 0..decoded_channels {
        channel_samples.push(decode_subframe_from_reader(
            &mut reader,
            block_size,
            frame
                .channel_assignment
                .bits_per_sample_for_channel(frame.bits_per_sample, channel_index)?,
        )?);
    }
    let channel_samples = decorrelate_channels(frame.channel_assignment, channel_samples)?;

    let mut samples = Vec::with_capacity(block_size * usize::from(decoded_channels));
    let md5_sample_bytes = usize::from(frame.bits_per_sample).div_ceil(8);
    let mut md5_input = Vec::with_capacity(samples.capacity() * md5_sample_bytes);
    for frame_index in 0..block_size {
        for channel in &channel_samples {
            let sample = *channel
                .get(frame_index)
                .ok_or(Error::InvalidInput("FLAC channel sample is missing"))?;
            append_md5_sample(&mut md5_input, sample, frame.bits_per_sample)?;
            samples.push(normalize_signed_sample(sample, frame.bits_per_sample)?);
        }
    }

    let bytes_read = frame
        .header_len
        .checked_add(reader.byte_pos())
        .and_then(|value| value.checked_add(2))
        .ok_or(Error::InvalidInput("FLAC frame size overflow"))?;
    input
        .get(bytes_read - 2..bytes_read)
        .ok_or(Error::InvalidInput("FLAC frame footer is truncated"))?;
    let expected_crc = read_u16_be(input, bytes_read - 2)?;
    let actual_crc = crc16(
        input
            .get(..bytes_read - 2)
            .ok_or(Error::InvalidInput("FLAC frame is truncated"))?,
    );
    if actual_crc != expected_crc {
        return Err(Error::InvalidInput("FLAC frame footer CRC mismatch"));
    }

    Ok(DecodedFrame {
        header: frame,
        frames: block_size,
        samples,
        md5_input,
        bytes_read,
    })
}

fn append_md5_sample(out: &mut Vec<u8>, sample: i32, bits_per_sample: u8) -> Result<(), Error> {
    if bits_per_sample == 0 || bits_per_sample > 32 {
        return Err(Error::InvalidInput("unsupported FLAC sample width"));
    }
    let bytes = usize::from(bits_per_sample).div_ceil(8);
    out.extend_from_slice(&sample.to_le_bytes()[..bytes]);
    Ok(())
}

fn validate_frame_size(bytes_read: usize, stream_info: &StreamInfo) -> Result<(), Error> {
    let bytes_read = u32::try_from(bytes_read)
        .map_err(|_| Error::InvalidInput("FLAC frame size is too large"))?;
    if stream_info.min_frame_size != 0 && bytes_read < stream_info.min_frame_size {
        return Err(Error::InvalidInput(
            "FLAC frame size below STREAMINFO minimum",
        ));
    }
    if stream_info.max_frame_size != 0 && bytes_read > stream_info.max_frame_size {
        return Err(Error::InvalidInput(
            "FLAC frame size above STREAMINFO maximum",
        ));
    }
    Ok(())
}

fn encode_block_sizes(total_frames: usize) -> Vec<usize> {
    let mut remaining = total_frames;
    let mut block_sizes = Vec::new();
    while remaining > 0 {
        let block_size = remaining.min(ENCODE_BLOCK_SIZE);
        block_sizes.push(block_size);
        remaining -= block_size;
    }
    block_sizes
}

fn encode_frame(
    pcm_i16: &[i16],
    channels: usize,
    sample_offset: usize,
    block_size: usize,
    coded_number: usize,
    fixed_blocking: bool,
) -> Result<Vec<u8>, Error> {
    if channels == 2 {
        let candidates = vec![
            encode_frame_with_channels(
                channels,
                block_size,
                coded_number,
                fixed_blocking,
                1,
                &stereo_independent_channels(pcm_i16, sample_offset, block_size)?,
            )?,
            encode_frame_with_channels(
                channels,
                block_size,
                coded_number,
                fixed_blocking,
                8,
                &left_side_channels(pcm_i16, sample_offset, block_size)?,
            )?,
            encode_frame_with_channels(
                channels,
                block_size,
                coded_number,
                fixed_blocking,
                9,
                &right_side_channels(pcm_i16, sample_offset, block_size)?,
            )?,
            encode_frame_with_channels(
                channels,
                block_size,
                coded_number,
                fixed_blocking,
                10,
                &mid_side_channels(pcm_i16, sample_offset, block_size)?,
            )?,
        ];
        return candidates
            .into_iter()
            .min_by_key(Vec::len)
            .ok_or(Error::InvalidPcm("FLAC encode has no frame candidates"));
    }

    let mut encoded_channels = Vec::with_capacity(channels);
    for channel in 0..channels {
        encoded_channels.push(EncodedChannel {
            bits_per_sample: ENCODE_BITS_PER_SAMPLE,
            samples: collect_channel_samples(
                pcm_i16,
                channels,
                sample_offset,
                block_size,
                channel,
            )?,
        });
    }
    encode_frame_with_channels(
        channels,
        block_size,
        coded_number,
        fixed_blocking,
        u8::try_from(channels - 1)
            .map_err(|_| Error::InvalidPcm("FLAC channel assignment is out of range"))?,
        &encoded_channels,
    )
}

struct EncodedChannel {
    bits_per_sample: u8,
    samples: Vec<i32>,
}

fn encode_frame_with_channels(
    channels: usize,
    block_size: usize,
    coded_number: usize,
    fixed_blocking: bool,
    channel_assignment_code: u8,
    encoded_channels: &[EncodedChannel],
) -> Result<Vec<u8>, Error> {
    if block_size == 0 || block_size > usize::from(u16::MAX) {
        return Err(Error::InvalidPcm("FLAC block size is out of range"));
    }
    if encoded_channels.len() != channels {
        return Err(Error::InvalidPcm("FLAC encoded channel count mismatch"));
    }
    let mut frame = Vec::new();
    let sync_second = if fixed_blocking { 0xf8 } else { 0xf9 };
    frame.extend_from_slice(&[
        0xff,
        sync_second,
        0x70,
        (channel_assignment_code << 4) | 0x08,
    ]);
    frame
        .extend_from_slice(&utf8_coded_number(u64::try_from(coded_number).map_err(
            |_| Error::InvalidPcm("FLAC frame/sample number is out of range"),
        )?)?);
    frame.extend_from_slice(
        &u16::try_from(block_size - 1)
            .map_err(|_| Error::InvalidPcm("FLAC block size is out of range"))?
            .to_be_bytes(),
    );
    frame.push(crc8(&frame));

    let mut writer = FlacBitWriter::new();
    for channel in encoded_channels {
        write_best_subframe(&mut writer, &channel.samples, channel.bits_per_sample)?;
    }
    frame.extend_from_slice(&writer.finish());
    frame.extend_from_slice(&crc16(&frame).to_be_bytes());
    Ok(frame)
}

fn collect_channel_samples(
    pcm_i16: &[i16],
    channels: usize,
    sample_offset: usize,
    block_size: usize,
    channel: usize,
) -> Result<Vec<i32>, Error> {
    let mut channel_samples = Vec::with_capacity(block_size);
    for frame_index in 0..block_size {
        let sample_index = sample_offset
            .checked_add(frame_index)
            .and_then(|frame| frame.checked_mul(channels))
            .and_then(|base| base.checked_add(channel))
            .ok_or(Error::InvalidPcm("FLAC sample index overflow"))?;
        let sample = *pcm_i16
            .get(sample_index)
            .ok_or(Error::InvalidPcm("FLAC sample is missing"))?;
        channel_samples.push(i32::from(sample));
    }
    Ok(channel_samples)
}

fn stereo_independent_channels(
    pcm_i16: &[i16],
    sample_offset: usize,
    block_size: usize,
) -> Result<[EncodedChannel; 2], Error> {
    Ok([
        EncodedChannel {
            bits_per_sample: ENCODE_BITS_PER_SAMPLE,
            samples: collect_channel_samples(pcm_i16, 2, sample_offset, block_size, 0)?,
        },
        EncodedChannel {
            bits_per_sample: ENCODE_BITS_PER_SAMPLE,
            samples: collect_channel_samples(pcm_i16, 2, sample_offset, block_size, 1)?,
        },
    ])
}

fn left_side_channels(
    pcm_i16: &[i16],
    sample_offset: usize,
    block_size: usize,
) -> Result<[EncodedChannel; 2], Error> {
    let left = collect_channel_samples(pcm_i16, 2, sample_offset, block_size, 0)?;
    let right = collect_channel_samples(pcm_i16, 2, sample_offset, block_size, 1)?;
    let side = left
        .iter()
        .zip(&right)
        .map(|(&left, &right)| left - right)
        .collect::<Vec<_>>();
    Ok([
        EncodedChannel {
            bits_per_sample: ENCODE_BITS_PER_SAMPLE,
            samples: left,
        },
        EncodedChannel {
            bits_per_sample: ENCODE_BITS_PER_SAMPLE + 1,
            samples: side,
        },
    ])
}

fn right_side_channels(
    pcm_i16: &[i16],
    sample_offset: usize,
    block_size: usize,
) -> Result<[EncodedChannel; 2], Error> {
    let left = collect_channel_samples(pcm_i16, 2, sample_offset, block_size, 0)?;
    let right = collect_channel_samples(pcm_i16, 2, sample_offset, block_size, 1)?;
    let side = left
        .iter()
        .zip(&right)
        .map(|(&left, &right)| left - right)
        .collect::<Vec<_>>();
    Ok([
        EncodedChannel {
            bits_per_sample: ENCODE_BITS_PER_SAMPLE + 1,
            samples: side,
        },
        EncodedChannel {
            bits_per_sample: ENCODE_BITS_PER_SAMPLE,
            samples: right,
        },
    ])
}

fn mid_side_channels(
    pcm_i16: &[i16],
    sample_offset: usize,
    block_size: usize,
) -> Result<[EncodedChannel; 2], Error> {
    let left = collect_channel_samples(pcm_i16, 2, sample_offset, block_size, 0)?;
    let right = collect_channel_samples(pcm_i16, 2, sample_offset, block_size, 1)?;
    let mid = left
        .iter()
        .zip(&right)
        .map(|(&left, &right)| (left + right) >> 1)
        .collect::<Vec<_>>();
    let side = left
        .iter()
        .zip(&right)
        .map(|(&left, &right)| left - right)
        .collect::<Vec<_>>();
    Ok([
        EncodedChannel {
            bits_per_sample: ENCODE_BITS_PER_SAMPLE,
            samples: mid,
        },
        EncodedChannel {
            bits_per_sample: ENCODE_BITS_PER_SAMPLE + 1,
            samples: side,
        },
    ])
}

fn write_best_subframe(
    writer: &mut FlacBitWriter,
    samples: &[i32],
    bits_per_sample: u8,
) -> Result<(), Error> {
    if let Some(&first_sample) = samples.first() {
        if samples.iter().all(|&sample| sample == first_sample) {
            write_constant_subframe(writer, first_sample, bits_per_sample);
            return Ok(());
        }
    }

    let Some(candidate) = best_fixed_rice(samples, bits_per_sample) else {
        write_verbatim_subframe(writer, samples, bits_per_sample);
        return Ok(());
    };

    let fixed_bits = fixed_rice_bits(
        samples.len(),
        candidate.order,
        &candidate.residuals,
        candidate.rice_parameter,
        bits_per_sample,
    )?;
    let verbatim_bits = 8_usize
        .checked_add(
            samples
                .len()
                .checked_mul(usize::from(bits_per_sample))
                .ok_or(Error::InvalidPcm("FLAC verbatim subframe size overflow"))?,
        )
        .ok_or(Error::InvalidPcm("FLAC verbatim subframe size overflow"))?;

    if fixed_bits < verbatim_bits {
        write_fixed_rice_subframe(writer, samples, &candidate, bits_per_sample);
    } else {
        write_verbatim_subframe(writer, samples, bits_per_sample);
    }
    Ok(())
}

fn write_constant_subframe(writer: &mut FlacBitWriter, sample: i32, bits_per_sample: u8) {
    writer.write_bits(0, 1);
    writer.write_bits(0, 6);
    writer.write_bits(0, 1);
    writer.write_signed_bits(sample, bits_per_sample);
}

fn write_verbatim_subframe(writer: &mut FlacBitWriter, samples: &[i32], bits_per_sample: u8) {
    writer.write_bits(0, 1);
    writer.write_bits(1, 6);
    writer.write_bits(0, 1);
    for &sample in samples {
        writer.write_signed_bits(sample, bits_per_sample);
    }
}

fn write_fixed_rice_subframe(
    writer: &mut FlacBitWriter,
    samples: &[i32],
    candidate: &FixedRiceCandidate,
    bits_per_sample: u8,
) {
    writer.write_bits(0, 1);
    writer.write_bits(u32::from(8 + candidate.order), 6);
    writer.write_bits(0, 1);
    for &sample in &samples[..usize::from(candidate.order)] {
        writer.write_signed_bits(sample, bits_per_sample);
    }
    writer.write_bits(0, 2);
    writer.write_bits(0, 4);
    writer.write_bits(u32::from(candidate.rice_parameter), 4);
    for &residual in &candidate.residuals {
        writer.write_rice_signed(residual, candidate.rice_parameter);
    }
}

struct FixedRiceCandidate {
    order: u8,
    rice_parameter: u8,
    residuals: Vec<i32>,
}

fn best_fixed_rice(samples: &[i32], bits_per_sample: u8) -> Option<FixedRiceCandidate> {
    if samples.len() < 2 {
        return None;
    }

    let mut best: Option<(usize, FixedRiceCandidate)> = None;
    for order in 1..=4 {
        if samples.len() <= usize::from(order) {
            continue;
        }
        let residuals = fixed_residuals(samples, order)?;
        for rice_parameter in 0..=14 {
            let Ok(bits) = fixed_rice_bits(
                samples.len(),
                order,
                &residuals,
                rice_parameter,
                bits_per_sample,
            ) else {
                continue;
            };
            if best
                .as_ref()
                .map(|(best_bits, _)| bits < *best_bits)
                .unwrap_or(true)
            {
                best = Some((
                    bits,
                    FixedRiceCandidate {
                        order,
                        rice_parameter,
                        residuals: residuals.clone(),
                    },
                ));
            }
        }
    }

    best.map(|(_, candidate)| candidate)
}

fn fixed_residuals(samples: &[i32], order: u8) -> Option<Vec<i32>> {
    let order = usize::from(order);
    if order == 0 || order > 4 || samples.len() <= order {
        return None;
    }
    let mut residuals = Vec::with_capacity(samples.len() - order);
    for index in order..samples.len() {
        let predicted = match order {
            1 => samples[index - 1],
            2 => 2 * samples[index - 1] - samples[index - 2],
            3 => 3 * samples[index - 1] - 3 * samples[index - 2] + samples[index - 3],
            4 => {
                4 * samples[index - 1] - 6 * samples[index - 2] + 4 * samples[index - 3]
                    - samples[index - 4]
            }
            _ => return None,
        };
        residuals.push(samples[index] - predicted);
    }
    Some(residuals)
}

fn fixed_rice_bits(
    samples_len: usize,
    order: u8,
    residuals: &[i32],
    rice_parameter: u8,
    bits_per_sample: u8,
) -> Result<usize, Error> {
    let order = usize::from(order);
    if order == 0 || order > 4 || residuals.len() + order != samples_len {
        return Err(Error::InvalidPcm("FLAC residual count mismatch"));
    }
    let mut bits = 8_usize
        .checked_add(
            order
                .checked_mul(usize::from(bits_per_sample))
                .ok_or(Error::InvalidPcm("FLAC fixed subframe size overflow"))?,
        )
        .and_then(|value| value.checked_add(2 + 4 + 4))
        .ok_or(Error::InvalidPcm("FLAC fixed subframe size overflow"))?;
    for &residual in residuals {
        let folded = folded_rice_value(residual);
        let quotient = folded >> rice_parameter;
        bits = bits
            .checked_add(
                usize::try_from(quotient)
                    .map_err(|_| Error::InvalidPcm("FLAC Rice residual is too large"))?,
            )
            .and_then(|value| value.checked_add(1 + usize::from(rice_parameter)))
            .ok_or(Error::InvalidPcm("FLAC fixed subframe size overflow"))?;
    }
    Ok(bits)
}

fn folded_rice_value(value: i32) -> u32 {
    if value >= 0 {
        (value as u32) << 1
    } else {
        ((-value as u32) << 1) - 1
    }
}

fn utf8_coded_number(value: u64) -> Result<Vec<u8>, Error> {
    if value <= 0x7f {
        return Ok(vec![value as u8]);
    }
    if value <= 0x7ff {
        return Ok(vec![
            0xc0 | ((value >> 6) as u8),
            0x80 | ((value & 0x3f) as u8),
        ]);
    }
    if value <= 0xffff {
        return Ok(vec![
            0xe0 | ((value >> 12) as u8),
            0x80 | (((value >> 6) & 0x3f) as u8),
            0x80 | ((value & 0x3f) as u8),
        ]);
    }
    if value <= 0x1f_ffff {
        return Ok(vec![
            0xf0 | ((value >> 18) as u8),
            0x80 | (((value >> 12) & 0x3f) as u8),
            0x80 | (((value >> 6) & 0x3f) as u8),
            0x80 | ((value & 0x3f) as u8),
        ]);
    }
    if value <= 0x03ff_ffff {
        return Ok(vec![
            0xf8 | ((value >> 24) as u8),
            0x80 | (((value >> 18) & 0x3f) as u8),
            0x80 | (((value >> 12) & 0x3f) as u8),
            0x80 | (((value >> 6) & 0x3f) as u8),
            0x80 | ((value & 0x3f) as u8),
        ]);
    }
    if value <= 0x7fff_ffff {
        return Ok(vec![
            0xfc | ((value >> 30) as u8),
            0x80 | (((value >> 24) & 0x3f) as u8),
            0x80 | (((value >> 18) & 0x3f) as u8),
            0x80 | (((value >> 12) & 0x3f) as u8),
            0x80 | (((value >> 6) & 0x3f) as u8),
            0x80 | ((value & 0x3f) as u8),
        ]);
    }
    if value <= 0x000f_ffff_ffff {
        return Ok(vec![
            0xfe,
            0x80 | (((value >> 30) & 0x3f) as u8),
            0x80 | (((value >> 24) & 0x3f) as u8),
            0x80 | (((value >> 18) & 0x3f) as u8),
            0x80 | (((value >> 12) & 0x3f) as u8),
            0x80 | (((value >> 6) & 0x3f) as u8),
            0x80 | ((value & 0x3f) as u8),
        ]);
    }
    Err(Error::InvalidPcm(
        "FLAC coded frame/sample number is out of range",
    ))
}

fn quantize_i16(sample: f32) -> i16 {
    let sample = sample.clamp(-1.0, 1.0);
    if sample <= -1.0 {
        i16::MIN
    } else {
        (sample * f32::from(i16::MAX)).round() as i16
    }
}

struct FlacBitWriter {
    bytes: Vec<u8>,
    bit_pos: usize,
}

impl FlacBitWriter {
    fn new() -> Self {
        Self {
            bytes: Vec::new(),
            bit_pos: 0,
        }
    }

    fn write_bits(&mut self, value: u32, count: u8) {
        for bit_index in (0..count).rev() {
            self.write_bit(((value >> bit_index) & 1) as u8);
        }
    }

    fn write_signed_bits(&mut self, value: i32, count: u8) {
        let mask = if count == 32 {
            u32::MAX
        } else {
            (1_u32 << count) - 1
        };
        self.write_bits((value as u32) & mask, count);
    }

    fn write_rice_signed(&mut self, value: i32, rice_parameter: u8) {
        let folded = folded_rice_value(value);
        let quotient = folded >> rice_parameter;
        for _ in 0..quotient {
            self.write_bit(0);
        }
        self.write_bit(1);
        if rice_parameter > 0 {
            self.write_bits(folded & ((1_u32 << rice_parameter) - 1), rice_parameter);
        }
    }

    fn finish(self) -> Vec<u8> {
        self.bytes
    }

    fn write_bit(&mut self, bit: u8) {
        if self.bit_pos % 8 == 0 {
            self.bytes.push(0);
        }
        let byte_index = self.bit_pos / 8;
        let bit_index = 7 - (self.bit_pos % 8);
        self.bytes[byte_index] |= bit << bit_index;
        self.bit_pos += 1;
    }
}

impl ChannelAssignment {
    const fn channels(self) -> u8 {
        match self {
            Self::Independent(channels) => channels,
            Self::LeftSide | Self::RightSide | Self::MidSide => 2,
        }
    }

    fn bits_per_sample_for_channel(
        self,
        bits_per_sample: u8,
        channel_index: u8,
    ) -> Result<u8, Error> {
        match self {
            Self::Independent(_) => Ok(bits_per_sample),
            Self::LeftSide => {
                if channel_index == 1 {
                    bits_per_sample.checked_add(1).ok_or(Error::InvalidInput(
                        "FLAC side channel sample width overflow",
                    ))
                } else {
                    Ok(bits_per_sample)
                }
            }
            Self::RightSide => {
                if channel_index == 0 {
                    bits_per_sample.checked_add(1).ok_or(Error::InvalidInput(
                        "FLAC side channel sample width overflow",
                    ))
                } else {
                    Ok(bits_per_sample)
                }
            }
            Self::MidSide => {
                if channel_index == 1 {
                    bits_per_sample.checked_add(1).ok_or(Error::InvalidInput(
                        "FLAC side channel sample width overflow",
                    ))
                } else {
                    Ok(bits_per_sample)
                }
            }
        }
    }
}

pub fn parse_frame_header(input: &[u8], stream_info: &StreamInfo) -> Result<FrameHeader, Error> {
    if input.len() < 4 {
        return Err(Error::InvalidInput("FLAC frame header is truncated"));
    }
    let first = *input
        .first()
        .ok_or(Error::InvalidInput("FLAC frame header is truncated"))?;
    let second = *input
        .get(1)
        .ok_or(Error::InvalidInput("FLAC frame header is truncated"))?;
    if first != 0xff || (second & 0xfc) != 0xf8 {
        return Err(Error::InvalidInput("missing FLAC frame sync"));
    }
    if second & 0x02 != 0 {
        return Err(Error::InvalidInput("invalid FLAC reserved frame bit"));
    }

    let blocking_strategy = if second & 0x01 == 0 {
        BlockingStrategy::FixedBlockSize
    } else {
        BlockingStrategy::VariableBlockSize
    };
    let third = *input
        .get(2)
        .ok_or(Error::InvalidInput("FLAC frame header is truncated"))?;
    let fourth = *input
        .get(3)
        .ok_or(Error::InvalidInput("FLAC frame header is truncated"))?;
    let block_size_code = third >> 4;
    let sample_rate_code = third & 0x0f;
    let channel_assignment_code = fourth >> 4;
    let sample_size_code = (fourth >> 1) & 0x07;
    if fourth & 0x01 != 0 {
        return Err(Error::InvalidInput("invalid FLAC reserved sample-size bit"));
    }

    let (frame_or_sample_number, mut cursor) = read_utf8_uint(input, 4)?;
    let block_size = decode_block_size(block_size_code, input, &mut cursor)?;
    let sample_rate = decode_sample_rate(sample_rate_code, input, &mut cursor, stream_info)?;
    let channel_assignment = decode_channel_assignment(channel_assignment_code)?;
    let bits_per_sample = decode_bits_per_sample(sample_size_code, stream_info)?;

    let actual_crc = *input
        .get(cursor)
        .ok_or(Error::InvalidInput("FLAC frame header CRC is truncated"))?;
    let expected_crc = crc8(
        input
            .get(..cursor)
            .ok_or(Error::InvalidInput("FLAC frame header is truncated"))?,
    );
    if actual_crc != expected_crc {
        return Err(Error::InvalidInput("FLAC frame header CRC mismatch"));
    }
    cursor += 1;

    Ok(FrameHeader {
        blocking_strategy,
        block_size,
        sample_rate,
        channel_assignment,
        bits_per_sample,
        frame_or_sample_number,
        header_len: cursor,
    })
}

pub fn decode_subframe(
    input: &[u8],
    block_size: usize,
    bits_per_sample: u8,
) -> Result<(Vec<i32>, usize), Error> {
    let mut reader = BitReader::new(input);
    let samples = decode_subframe_from_reader(&mut reader, block_size, bits_per_sample)?;
    Ok((samples, reader.byte_pos()))
}

fn decode_subframe_from_reader(
    reader: &mut BitReader<'_>,
    block_size: usize,
    bits_per_sample: u8,
) -> Result<Vec<i32>, Error> {
    if reader.read_bool()? {
        return Err(Error::InvalidInput(
            "invalid FLAC subframe zero padding bit",
        ));
    }
    let subframe_type = reader.read_bits(6)? as u8;
    let wasted_bits_per_sample = if reader.read_bool()? {
        read_wasted_bits_per_sample(reader)?
    } else {
        0
    };
    if wasted_bits_per_sample >= bits_per_sample {
        return Err(Error::InvalidInput("invalid FLAC wasted bits-per-sample"));
    }
    let effective_bits_per_sample = bits_per_sample - wasted_bits_per_sample;

    let mut samples = match subframe_type {
        0 => decode_constant_subframe(reader, block_size, effective_bits_per_sample)?,
        1 => decode_verbatim_subframe(reader, block_size, effective_bits_per_sample)?,
        8..=12 => decode_fixed_subframe(
            reader,
            block_size,
            effective_bits_per_sample,
            subframe_type - 8,
        )?,
        32..=63 => decode_lpc_subframe(
            reader,
            block_size,
            effective_bits_per_sample,
            subframe_type - 31,
        )?,
        _ => return Err(Error::UnsupportedFeature("FLAC subframe type")),
    };

    if wasted_bits_per_sample > 0 {
        for sample in &mut samples {
            *sample <<= wasted_bits_per_sample;
        }
    }

    Ok(samples)
}

pub fn parse_streaminfo(input: &[u8]) -> Result<StreamInfo, Error> {
    parse_metadata(input).map(|(stream_info, _)| stream_info)
}

fn parse_metadata(input: &[u8]) -> Result<(StreamInfo, usize), Error> {
    if input.get(0..4) != Some(FLAC_MARKER) {
        return Err(Error::InvalidInput("missing FLAC marker"));
    }

    let mut cursor = FLAC_MARKER.len();
    let mut stream_info = None;
    loop {
        let header_end = cursor
            .checked_add(METADATA_HEADER_LEN)
            .ok_or(Error::InvalidInput("FLAC metadata header overflow"))?;
        let header = input
            .get(cursor..header_end)
            .ok_or(Error::InvalidInput("FLAC metadata header is truncated"))?;
        let is_last = header[0] & 0x80 != 0;
        let block_type = header[0] & 0x7f;
        let block_len = read_u24_be(header, 1)? as usize;
        let data_start = header_end;
        let data_end = data_start
            .checked_add(block_len)
            .ok_or(Error::InvalidInput("FLAC metadata block overflow"))?;
        let block = input
            .get(data_start..data_end)
            .ok_or(Error::InvalidInput("FLAC metadata block is truncated"))?;

        if block_type == STREAMINFO_BLOCK_TYPE {
            stream_info = Some(parse_streaminfo_block(block)?);
        }
        cursor = data_end;
        if is_last {
            break;
        }
    }

    Ok((
        stream_info.ok_or(Error::InvalidInput("missing FLAC STREAMINFO block"))?,
        cursor,
    ))
}

fn decode_constant_subframe(
    reader: &mut BitReader<'_>,
    block_size: usize,
    bits_per_sample: u8,
) -> Result<Vec<i32>, Error> {
    let sample = reader.read_signed_bits(bits_per_sample)?;
    Ok(vec![sample; block_size])
}

fn decode_verbatim_subframe(
    reader: &mut BitReader<'_>,
    block_size: usize,
    bits_per_sample: u8,
) -> Result<Vec<i32>, Error> {
    let mut samples = Vec::with_capacity(block_size);
    for _ in 0..block_size {
        samples.push(reader.read_signed_bits(bits_per_sample)?);
    }
    Ok(samples)
}

fn decorrelate_channels(
    assignment: ChannelAssignment,
    mut channels: Vec<Vec<i32>>,
) -> Result<Vec<Vec<i32>>, Error> {
    match assignment {
        ChannelAssignment::Independent(_) => Ok(channels),
        ChannelAssignment::LeftSide => {
            let side = channels
                .pop()
                .ok_or(Error::InvalidInput("missing FLAC side channel"))?;
            let left = channels
                .pop()
                .ok_or(Error::InvalidInput("missing FLAC left channel"))?;
            if left.len() != side.len() {
                return Err(Error::InvalidInput("FLAC channel length mismatch"));
            }
            let mut right = Vec::with_capacity(left.len());
            for (&left_sample, &side_sample) in left.iter().zip(&side) {
                right.push(
                    left_sample
                        .checked_sub(side_sample)
                        .ok_or(Error::InvalidInput("FLAC left-side sample overflow"))?,
                );
            }
            Ok(vec![left, right])
        }
        ChannelAssignment::RightSide => {
            let right = channels
                .pop()
                .ok_or(Error::InvalidInput("missing FLAC right channel"))?;
            let side = channels
                .pop()
                .ok_or(Error::InvalidInput("missing FLAC side channel"))?;
            if right.len() != side.len() {
                return Err(Error::InvalidInput("FLAC channel length mismatch"));
            }
            let mut left = Vec::with_capacity(right.len());
            for (&side_sample, &right_sample) in side.iter().zip(&right) {
                left.push(
                    right_sample
                        .checked_add(side_sample)
                        .ok_or(Error::InvalidInput("FLAC right-side sample overflow"))?,
                );
            }
            Ok(vec![left, right])
        }
        ChannelAssignment::MidSide => {
            let side = channels
                .pop()
                .ok_or(Error::InvalidInput("missing FLAC side channel"))?;
            let mid = channels
                .pop()
                .ok_or(Error::InvalidInput("missing FLAC mid channel"))?;
            if mid.len() != side.len() {
                return Err(Error::InvalidInput("FLAC channel length mismatch"));
            }
            let mut left = Vec::with_capacity(mid.len());
            let mut right = Vec::with_capacity(mid.len());
            for (&mid_sample, &side_sample) in mid.iter().zip(&side) {
                let mid_adjusted = i64::from(mid_sample)
                    .checked_mul(2)
                    .and_then(|value| value.checked_add(i64::from(side_sample & 1)))
                    .ok_or(Error::InvalidInput("FLAC mid-side sample overflow"))?;
                let side_sample = i64::from(side_sample);
                left.push(
                    i32::try_from((mid_adjusted + side_sample) / 2)
                        .map_err(|_| Error::InvalidInput("FLAC mid-side sample overflow"))?,
                );
                right.push(
                    i32::try_from((mid_adjusted - side_sample) / 2)
                        .map_err(|_| Error::InvalidInput("FLAC mid-side sample overflow"))?,
                );
            }
            Ok(vec![left, right])
        }
    }
}

fn decode_fixed_subframe(
    reader: &mut BitReader<'_>,
    block_size: usize,
    bits_per_sample: u8,
    predictor_order: u8,
) -> Result<Vec<i32>, Error> {
    let predictor_order = usize::from(predictor_order);
    if predictor_order > 4 || predictor_order > block_size {
        return Err(Error::InvalidInput("invalid FLAC fixed predictor order"));
    }

    let mut samples = Vec::with_capacity(block_size);
    for _ in 0..predictor_order {
        samples.push(reader.read_signed_bits(bits_per_sample)?);
    }

    let residuals = decode_residual(reader, block_size, predictor_order)?;
    for residual in residuals {
        let predicted = fixed_prediction(&samples, predictor_order)?;
        samples.push(
            predicted
                .checked_add(residual)
                .ok_or(Error::InvalidInput("FLAC fixed predictor sample overflow"))?,
        );
    }

    Ok(samples)
}

fn decode_lpc_subframe(
    reader: &mut BitReader<'_>,
    block_size: usize,
    bits_per_sample: u8,
    predictor_order: u8,
) -> Result<Vec<i32>, Error> {
    let predictor_order = usize::from(predictor_order);
    if predictor_order == 0 || predictor_order > 32 || predictor_order > block_size {
        return Err(Error::InvalidInput("invalid FLAC LPC predictor order"));
    }

    let mut samples = Vec::with_capacity(block_size);
    for _ in 0..predictor_order {
        samples.push(reader.read_signed_bits(bits_per_sample)?);
    }

    let coefficient_precision = reader.read_bits(4)? as u8 + 1;
    if coefficient_precision == 16 {
        return Err(Error::InvalidInput(
            "invalid FLAC LPC coefficient precision",
        ));
    }
    let quantization_shift = reader.read_signed_bits(5)?;
    if quantization_shift < 0 {
        return Err(Error::UnsupportedFeature(
            "negative FLAC LPC quantization shift",
        ));
    }
    let quantization_shift = u32::try_from(quantization_shift)
        .map_err(|_| Error::InvalidInput("invalid FLAC LPC quantization shift"))?;

    let mut coefficients = Vec::with_capacity(predictor_order);
    for _ in 0..predictor_order {
        coefficients.push(reader.read_signed_bits(coefficient_precision)?);
    }

    let residuals = decode_residual(reader, block_size, predictor_order)?;
    for residual in residuals {
        let predicted = lpc_prediction(&samples, &coefficients, quantization_shift)?;
        samples.push(
            predicted
                .checked_add(residual)
                .ok_or(Error::InvalidInput("FLAC LPC sample overflow"))?,
        );
    }

    Ok(samples)
}

fn decode_residual(
    reader: &mut BitReader<'_>,
    block_size: usize,
    predictor_order: usize,
) -> Result<Vec<i32>, Error> {
    let coding_method = reader.read_bits(2)? as u8;
    let rice_parameter_bits = match coding_method {
        0 => 4,
        1 => 5,
        _ => return Err(Error::UnsupportedFeature("FLAC residual coding method")),
    };
    let escape_parameter = (1_u8 << rice_parameter_bits) - 1;
    let partition_order = reader.read_bits(4)? as usize;
    let partitions = 1_usize
        .checked_shl(
            u32::try_from(partition_order)
                .map_err(|_| Error::InvalidInput("FLAC partition order is too large"))?,
        )
        .ok_or(Error::InvalidInput("FLAC partition count overflow"))?;
    if partitions == 0 || block_size % partitions != 0 {
        return Err(Error::InvalidInput("invalid FLAC residual partition order"));
    }

    let partition_samples = block_size / partitions;
    let mut residuals = Vec::with_capacity(block_size.saturating_sub(predictor_order));
    for partition in 0..partitions {
        let mut samples_in_partition = partition_samples;
        if partition == 0 {
            if predictor_order > samples_in_partition {
                return Err(Error::InvalidInput(
                    "invalid FLAC predictor order for partition",
                ));
            }
            samples_in_partition -= predictor_order;
        }

        let rice_parameter = reader.read_bits(rice_parameter_bits)? as u8;
        if rice_parameter == escape_parameter {
            let raw_bits = reader.read_bits(5)? as u8;
            for _ in 0..samples_in_partition {
                residuals.push(reader.read_signed_bits(raw_bits)?);
            }
        } else {
            for _ in 0..samples_in_partition {
                residuals.push(read_rice_signed(reader, rice_parameter)?);
            }
        }
    }

    Ok(residuals)
}

fn read_rice_signed(reader: &mut BitReader<'_>, rice_parameter: u8) -> Result<i32, Error> {
    let quotient = reader.read_unary_zeros()?;
    let remainder = if rice_parameter == 0 {
        0
    } else {
        reader.read_bits(rice_parameter)?
    };
    let unsigned = (quotient << rice_parameter) | remainder;
    if unsigned & 1 == 0 {
        Ok((unsigned >> 1) as i32)
    } else {
        Ok(-(((unsigned >> 1) as i32) + 1))
    }
}

fn fixed_prediction(samples: &[i32], predictor_order: usize) -> Result<i32, Error> {
    let len = samples.len();
    let sample = match predictor_order {
        0 => 0_i64,
        1 => samples
            .get(len.wrapping_sub(1))
            .copied()
            .ok_or(Error::InvalidInput("missing FLAC fixed predictor history"))?
            .into(),
        2 => {
            let s1 = predictor_history(samples, len, 1)?;
            let s2 = predictor_history(samples, len, 2)?;
            2 * s1 - s2
        }
        3 => {
            let s1 = predictor_history(samples, len, 1)?;
            let s2 = predictor_history(samples, len, 2)?;
            let s3 = predictor_history(samples, len, 3)?;
            3 * s1 - 3 * s2 + s3
        }
        4 => {
            let s1 = predictor_history(samples, len, 1)?;
            let s2 = predictor_history(samples, len, 2)?;
            let s3 = predictor_history(samples, len, 3)?;
            let s4 = predictor_history(samples, len, 4)?;
            4 * s1 - 6 * s2 + 4 * s3 - s4
        }
        _ => return Err(Error::InvalidInput("invalid FLAC fixed predictor order")),
    };

    i32::try_from(sample).map_err(|_| Error::InvalidInput("FLAC fixed predictor overflow"))
}

fn predictor_history(samples: &[i32], len: usize, back: usize) -> Result<i64, Error> {
    samples
        .get(len.wrapping_sub(back))
        .copied()
        .map(i64::from)
        .ok_or(Error::InvalidInput("missing FLAC fixed predictor history"))
}

fn lpc_prediction(
    samples: &[i32],
    coefficients: &[i32],
    quantization_shift: u32,
) -> Result<i32, Error> {
    let len = samples.len();
    let mut sum = 0_i64;
    for (index, &coefficient) in coefficients.iter().enumerate() {
        let sample = samples
            .get(len.wrapping_sub(index + 1))
            .copied()
            .ok_or(Error::InvalidInput("missing FLAC LPC predictor history"))?;
        sum = sum
            .checked_add(i64::from(coefficient) * i64::from(sample))
            .ok_or(Error::InvalidInput("FLAC LPC prediction overflow"))?;
    }

    i32::try_from(sum >> quantization_shift)
        .map_err(|_| Error::InvalidInput("FLAC LPC prediction overflow"))
}

fn read_wasted_bits_per_sample(reader: &mut BitReader<'_>) -> Result<u8, Error> {
    let mut count = 1_u8;
    while !reader.read_bool()? {
        count = count
            .checked_add(1)
            .ok_or(Error::InvalidInput("FLAC wasted bit count overflow"))?;
    }
    Ok(count)
}

fn read_utf8_uint(input: &[u8], offset: usize) -> Result<(u64, usize), Error> {
    let first = *input
        .get(offset)
        .ok_or(Error::InvalidInput("FLAC UTF-8 integer is truncated"))?;
    if first & 0x80 == 0 {
        return Ok((u64::from(first), offset + 1));
    }

    let (extra_bytes, mut value) = if first & 0xe0 == 0xc0 {
        (1, u64::from(first & 0x1f))
    } else if first & 0xf0 == 0xe0 {
        (2, u64::from(first & 0x0f))
    } else if first & 0xf8 == 0xf0 {
        (3, u64::from(first & 0x07))
    } else if first & 0xfc == 0xf8 {
        (4, u64::from(first & 0x03))
    } else if first & 0xfe == 0xfc {
        (5, u64::from(first & 0x01))
    } else if first == 0xfe {
        (6, 0)
    } else {
        return Err(Error::InvalidInput("invalid FLAC UTF-8 integer"));
    };

    let mut cursor = offset + 1;
    for _ in 0..extra_bytes {
        let byte = *input
            .get(cursor)
            .ok_or(Error::InvalidInput("FLAC UTF-8 integer is truncated"))?;
        if byte & 0xc0 != 0x80 {
            return Err(Error::InvalidInput("invalid FLAC UTF-8 continuation byte"));
        }
        value = (value << 6) | u64::from(byte & 0x3f);
        cursor += 1;
    }

    Ok((value, cursor))
}

fn decode_block_size(code: u8, input: &[u8], cursor: &mut usize) -> Result<u32, Error> {
    match code {
        0 => Err(Error::InvalidInput("reserved FLAC block-size code")),
        1 => Ok(192),
        2..=5 => Ok(576_u32 << (code - 2)),
        6 => {
            let value = u32::from(*input.get(*cursor).ok_or(Error::InvalidInput(
                "FLAC block-size extension is truncated",
            ))?) + 1;
            *cursor += 1;
            Ok(value)
        }
        7 => {
            let value = u32::from(read_u16_be(input, *cursor)?) + 1;
            *cursor += 2;
            Ok(value)
        }
        8..=15 => Ok(256_u32 << (code - 8)),
        _ => Err(Error::InvalidInput("invalid FLAC block-size code")),
    }
}

fn decode_sample_rate(
    code: u8,
    input: &[u8],
    cursor: &mut usize,
    stream_info: &StreamInfo,
) -> Result<u32, Error> {
    let sample_rate = match code {
        0 => stream_info.sample_rate,
        1 => 88_200,
        2 => 176_400,
        3 => 192_000,
        4 => 8_000,
        5 => 16_000,
        6 => 22_050,
        7 => 24_000,
        8 => 32_000,
        9 => 44_100,
        10 => 48_000,
        11 => 96_000,
        12 => {
            let value = u32::from(*input.get(*cursor).ok_or(Error::InvalidInput(
                "FLAC sample-rate extension is truncated",
            ))?) * 1000;
            *cursor += 1;
            value
        }
        13 => {
            let value = u32::from(read_u16_be(input, *cursor)?);
            *cursor += 2;
            value
        }
        14 => {
            let value = u32::from(read_u16_be(input, *cursor)?) * 10;
            *cursor += 2;
            value
        }
        _ => return Err(Error::InvalidInput("reserved FLAC sample-rate code")),
    };

    if sample_rate == 0 {
        return Err(Error::InvalidInput("FLAC frame sample rate is zero"));
    }
    Ok(sample_rate)
}

fn decode_channel_assignment(code: u8) -> Result<ChannelAssignment, Error> {
    match code {
        0..=7 => Ok(ChannelAssignment::Independent(code + 1)),
        8 => Ok(ChannelAssignment::LeftSide),
        9 => Ok(ChannelAssignment::RightSide),
        10 => Ok(ChannelAssignment::MidSide),
        _ => Err(Error::InvalidInput("reserved FLAC channel assignment")),
    }
}

fn decode_bits_per_sample(code: u8, stream_info: &StreamInfo) -> Result<u8, Error> {
    let bits_per_sample = match code {
        0 => stream_info.bits_per_sample,
        1 => 8,
        2 => 12,
        4 => 16,
        5 => 20,
        6 => 24,
        7 => 32,
        _ => return Err(Error::InvalidInput("reserved FLAC sample-size code")),
    };
    Ok(bits_per_sample)
}

fn normalize_signed_sample(sample: i32, bits_per_sample: u8) -> Result<f32, Error> {
    if bits_per_sample == 0 || bits_per_sample > 32 {
        return Err(Error::InvalidInput("unsupported FLAC sample width"));
    }

    let negative_denominator = (1_i64 << (bits_per_sample - 1)) as f32;
    let positive_denominator = ((1_i64 << (bits_per_sample - 1)) - 1) as f32;
    if sample < 0 {
        Ok((sample as f32 / negative_denominator).clamp(-1.0, 1.0))
    } else {
        Ok((sample as f32 / positive_denominator).clamp(-1.0, 1.0))
    }
}

fn crc8(input: &[u8]) -> u8 {
    let mut crc = 0_u8;
    for &byte in input {
        crc ^= byte;
        for _ in 0..8 {
            crc = if crc & 0x80 != 0 {
                (crc << 1) ^ 0x07
            } else {
                crc << 1
            };
        }
    }
    crc
}

fn crc16(input: &[u8]) -> u16 {
    let mut crc = 0_u16;
    for &byte in input {
        crc ^= u16::from(byte) << 8;
        for _ in 0..8 {
            crc = if crc & 0x8000 != 0 {
                (crc << 1) ^ 0x8005
            } else {
                crc << 1
            };
        }
    }
    crc
}

fn parse_streaminfo_block(block: &[u8]) -> Result<StreamInfo, Error> {
    if block.len() != STREAMINFO_LEN {
        return Err(Error::InvalidInput("invalid FLAC STREAMINFO length"));
    }

    let min_block_size = read_u16_be(block, 0)?;
    let max_block_size = read_u16_be(block, 2)?;
    if min_block_size == 0 || max_block_size == 0 || min_block_size > max_block_size {
        return Err(Error::InvalidInput("invalid FLAC block size range"));
    }

    let min_frame_size = read_u24_be(block, 4)?;
    let max_frame_size = read_u24_be(block, 7)?;
    if max_frame_size != 0 && min_frame_size > max_frame_size {
        return Err(Error::InvalidInput("invalid FLAC frame size range"));
    }

    let packed = read_u64_be(block, 10)?;
    let sample_rate = ((packed >> 44) & 0x000f_ffff) as u32;
    let channels = (((packed >> 41) & 0x07) as u8) + 1;
    let bits_per_sample = (((packed >> 36) & 0x1f) as u8) + 1;
    let total_samples = packed & 0x0000_000f_ffff_ffff;
    if sample_rate == 0 {
        return Err(Error::InvalidInput("FLAC sample rate is zero"));
    }

    let md5 = block
        .get(18..34)
        .ok_or(Error::InvalidInput("FLAC STREAMINFO md5 is truncated"))?
        .try_into()
        .map_err(|_| Error::InvalidInput("FLAC STREAMINFO md5 size mismatch"))?;

    Ok(StreamInfo {
        min_block_size,
        max_block_size,
        min_frame_size,
        max_frame_size,
        sample_rate,
        channels,
        bits_per_sample,
        total_samples,
        md5,
    })
}

fn read_u16_be(input: &[u8], offset: usize) -> Result<u16, Error> {
    Ok(u16::from_be_bytes(read_array::<2>(input, offset)?))
}

fn read_u24_be(input: &[u8], offset: usize) -> Result<u32, Error> {
    let bytes = read_array::<3>(input, offset)?;
    Ok(u32::from_be_bytes([0, bytes[0], bytes[1], bytes[2]]))
}

fn read_u64_be(input: &[u8], offset: usize) -> Result<u64, Error> {
    Ok(u64::from_be_bytes(read_array::<8>(input, offset)?))
}

fn read_array<const N: usize>(input: &[u8], offset: usize) -> Result<[u8; N], Error> {
    let end = offset
        .checked_add(N)
        .ok_or(Error::InvalidInput("read offset overflow"))?;
    input
        .get(offset..end)
        .ok_or(Error::InvalidInput("read is truncated"))?
        .try_into()
        .map_err(|_| Error::InvalidInput("read size mismatch"))
}

#[cfg(test)]
mod tests {
    use super::{
        crc16, crc8, decode, decode_subframe, parse_frame_header, parse_streaminfo,
        BlockingStrategy, ChannelAssignment, FlacDecoder, FlacEncoder, StreamInfo,
    };
    use sc_core::{AudioBuffer, Decoder, Encoder};

    #[test]
    fn parses_streaminfo() {
        let flac = flac_with_streaminfo(StreamInfoFixture {
            min_block_size: 4096,
            max_block_size: 4096,
            min_frame_size: 0,
            max_frame_size: 0,
            sample_rate: 48_000,
            channels: 2,
            bits_per_sample: 16,
            total_samples: 1234,
        });
        let info = parse_streaminfo(&flac).unwrap();

        assert_eq!(info.min_block_size, 4096);
        assert_eq!(info.max_block_size, 4096);
        assert_eq!(info.sample_rate, 48_000);
        assert_eq!(info.channels, 2);
        assert_eq!(info.bits_per_sample, 16);
        assert_eq!(info.total_samples, 1234);
    }

    #[test]
    fn rejects_missing_streaminfo() {
        assert!(parse_streaminfo(b"fLaC\x81\0\0\0").is_err());
    }

    #[test]
    fn parses_frame_header() {
        let info = test_streaminfo();
        let header =
            parse_frame_header(&frame_header([0xff, 0xf8, 0x1a, 0x18, 0x00]), &info).unwrap();

        assert_eq!(header.blocking_strategy, BlockingStrategy::FixedBlockSize);
        assert_eq!(header.block_size, 192);
        assert_eq!(header.sample_rate, 48_000);
        assert_eq!(header.channel_assignment, ChannelAssignment::Independent(2));
        assert_eq!(header.bits_per_sample, 16);
        assert_eq!(header.frame_or_sample_number, 0);
        assert_eq!(header.header_len, 6);
    }

    #[test]
    fn decodes_constant_subframe() {
        let (samples, bytes_read) = decode_subframe(&[0x00, 0x12, 0x34], 4, 16).unwrap();

        assert_eq!(samples, vec![0x1234; 4]);
        assert_eq!(bytes_read, 3);
    }

    #[test]
    fn decodes_verbatim_subframe() {
        let (samples, bytes_read) =
            decode_subframe(&[0x02, 0x00, 0x01, 0xff, 0xff], 2, 16).unwrap();

        assert_eq!(samples, vec![1, -1]);
        assert_eq!(bytes_read, 5);
    }

    #[test]
    fn decodes_fixed_subframe_with_rice_residual() {
        let subframe = fixed_order_one_subframe();
        let (samples, _bytes_read) = decode_subframe(&subframe, 4, 16).unwrap();

        assert_eq!(samples, vec![10, 11, 13, 16]);
    }

    #[test]
    fn decodes_lpc_subframe_with_rice_residual() {
        let subframe = lpc_order_one_subframe();
        let (samples, _bytes_read) = decode_subframe(&subframe, 4, 16).unwrap();

        assert_eq!(samples, vec![10, 11, 13, 16]);
    }

    #[test]
    fn decodes_single_constant_frame() {
        let mut flac = flac_with_streaminfo(StreamInfoFixture {
            min_block_size: 192,
            max_block_size: 192,
            min_frame_size: 0,
            max_frame_size: 0,
            sample_rate: 48_000,
            channels: 1,
            bits_per_sample: 16,
            total_samples: 192,
        });
        flac.extend_from_slice(&flac_frame(
            &[0xff, 0xf8, 0x1a, 0x08, 0x00],
            &[0x00, 0x40, 0x00],
        ));

        let decoded = decode(&flac).unwrap();

        assert_eq!(decoded.sample_rate, 48_000);
        assert_eq!(decoded.channels, 1);
        assert_eq!(decoded.frames(), 192);
        assert_eq!(decoded.samples[0], 16_384.0 / 32_767.0);
    }

    #[test]
    fn encodes_verbatim_flac_roundtrip() {
        let pcm = AudioBuffer::new(
            48_000,
            2,
            vec![-1.0, 1.0, -0.5, 0.5, 0.0, 0.25, 0.75, -0.25, 0.125, -0.125],
        )
        .unwrap();

        let flac = super::encode(&pcm).unwrap();
        let decoded = decode(&flac).unwrap();

        assert_eq!(decoded.sample_rate, 48_000);
        assert_eq!(decoded.channels, 2);
        assert_eq!(decoded.frames(), 5);
        assert_pcm_close(&decoded.samples, &pcm.samples, 1.0 / 32_767.0);
    }

    #[test]
    fn encoder_uses_fixed_rice_subframe_for_smooth_pcm() {
        let samples = (0..128)
            .map(|sample| sample as f32 / 32_767.0)
            .collect::<Vec<_>>();
        let pcm = AudioBuffer::new(48_000, 1, samples).unwrap();

        let flac = super::encode(&pcm).unwrap();
        let info = parse_streaminfo(&flac).unwrap();
        let header = parse_frame_header(&flac[42..], &info).unwrap();
        let subframe_type = flac[42 + header.header_len] >> 1;
        let decoded = decode(&flac).unwrap();

        assert_eq!(subframe_type, 10);
        assert_eq!(decoded.frames(), 128);
        assert_pcm_close(&decoded.samples, &pcm.samples, 1.0 / 32_767.0);
    }

    #[test]
    fn encoder_uses_constant_subframe_for_constant_pcm() {
        let pcm = AudioBuffer::new(48_000, 1, vec![0.25; 64]).unwrap();

        let flac = super::encode(&pcm).unwrap();
        let info = parse_streaminfo(&flac).unwrap();
        let header = parse_frame_header(&flac[42..], &info).unwrap();
        let subframe_type = flac[42 + header.header_len] >> 1;
        let decoded = decode(&flac).unwrap();

        assert_eq!(subframe_type, 0);
        assert_eq!(decoded.frames(), 64);
        assert_pcm_close(&decoded.samples, &pcm.samples, 1.0 / 32_767.0);
    }

    #[test]
    fn encoder_can_choose_fixed_predictor_orders_two_through_four() {
        assert_encoded_fixed_order((0..128).map(|sample| sample * 64).collect::<Vec<_>>(), 2);
        assert_encoded_fixed_order(
            (0..96)
                .map(|sample| {
                    let centered = sample - 48;
                    centered * centered * 8
                })
                .collect::<Vec<_>>(),
            3,
        );
        assert_encoded_fixed_order(
            (0..48)
                .map(|sample| {
                    let centered = sample - 24;
                    centered * centered * centered
                })
                .collect::<Vec<_>>(),
            4,
        );
    }

    #[test]
    fn encoder_can_choose_stereo_decorrelation() {
        let mut samples = Vec::new();
        for sample in 0..128 {
            let value = sample as f32 / 32_767.0;
            samples.push(value);
            samples.push(value);
        }
        let pcm = AudioBuffer::new(48_000, 2, samples).unwrap();

        let flac = super::encode(&pcm).unwrap();
        let info = parse_streaminfo(&flac).unwrap();
        let header = parse_frame_header(&flac[42..], &info).unwrap();
        let decoded = decode(&flac).unwrap();

        assert!(matches!(
            header.channel_assignment,
            ChannelAssignment::LeftSide | ChannelAssignment::RightSide | ChannelAssignment::MidSide
        ));
        assert_eq!(decoded.frames(), 128);
        assert_pcm_close(&decoded.samples, &pcm.samples, 1.0 / 32_767.0);
    }

    #[test]
    fn encoder_trait_encodes_flac() {
        let pcm = AudioBuffer::new(44_100, 1, vec![0.0, 0.25, -0.25]).unwrap();
        let mut encoder = FlacEncoder::new();
        let flac = encoder.encode(&pcm).unwrap();

        let decoded = decode(&flac).unwrap();

        assert_eq!(decoded.sample_rate, 44_100);
        assert_eq!(decoded.channels, 1);
        assert_pcm_close(&decoded.samples, &pcm.samples, 1.0 / 32_767.0);
    }

    #[test]
    fn rejects_empty_flac_encode() {
        let pcm = AudioBuffer::new(48_000, 1, Vec::new()).unwrap();

        assert!(super::encode(&pcm).is_err());
    }

    #[test]
    fn decodes_32_bit_constant_frame() {
        let mut flac = flac_with_streaminfo(StreamInfoFixture {
            min_block_size: 2,
            max_block_size: 2,
            min_frame_size: 0,
            max_frame_size: 0,
            sample_rate: 48_000,
            channels: 1,
            bits_per_sample: 32,
            total_samples: 2,
        });
        let mut writer = BitWriter::new();
        writer.write_constant_subframe(1_073_741_824, 32);
        flac.extend_from_slice(&flac_frame(
            &[0xff, 0xf8, 0x6a, 0x0e, 0x00, 0x01],
            &writer.finish(),
        ));

        let decoded = decode(&flac).unwrap();

        assert_eq!(decoded.channels, 1);
        assert_eq!(decoded.frames(), 2);
        assert_eq!(
            decoded.samples,
            vec![
                1_073_741_824.0 / 2_147_483_647.0,
                1_073_741_824.0 / 2_147_483_647.0,
            ]
        );
    }

    #[test]
    fn stream_decode_buffers_until_complete_stream() {
        let mut flac = flac_with_streaminfo(StreamInfoFixture {
            min_block_size: 192,
            max_block_size: 192,
            min_frame_size: 0,
            max_frame_size: 0,
            sample_rate: 48_000,
            channels: 1,
            bits_per_sample: 16,
            total_samples: 192,
        });
        flac.extend_from_slice(&flac_frame(
            &[0xff, 0xf8, 0x1a, 0x08, 0x00],
            &[0x00, 0x40, 0x00],
        ));
        let split = flac.len() - 2;
        let mut decoder = FlacDecoder::new();

        assert!(decoder.decode_stream(&flac[..split]).unwrap().is_none());
        let decoded = decoder
            .decode_stream(&flac[split..])
            .unwrap()
            .expect("complete stream should decode");

        assert_eq!(decoded.frames(), 192);
        assert_eq!(decoded.samples[0], 16_384.0 / 32_767.0);
    }

    #[test]
    fn rejects_bad_frame_header_crc() {
        let info = test_streaminfo();
        let mut header = frame_header([0xff, 0xf8, 0x1a, 0x18, 0x00]);
        let last = header.len() - 1;
        header[last] ^= 0x01;

        assert!(parse_frame_header(&header, &info).is_err());
    }

    #[test]
    fn parses_seven_byte_coded_sample_number() {
        let mut flac = flac_with_streaminfo(StreamInfoFixture {
            min_block_size: 2,
            max_block_size: 2,
            min_frame_size: 0,
            max_frame_size: 0,
            sample_rate: 48_000,
            channels: 1,
            bits_per_sample: 16,
            total_samples: 0,
        });
        let sample_number = 0x000f_ffff_ffff_u64;
        let mut header_without_crc = vec![0xff, 0xf9, 0x6a, 0x08];
        header_without_crc.extend_from_slice(&utf8_coded_number(sample_number));
        header_without_crc.push(0x01);
        flac.extend_from_slice(&flac_frame(&header_without_crc, &[0x00, 0x00, 0x00]));
        let stream_info = parse_streaminfo(&flac).unwrap();
        let header = parse_frame_header(&flac[42..], &stream_info).unwrap();

        assert_eq!(
            header.blocking_strategy,
            BlockingStrategy::VariableBlockSize
        );
        assert_eq!(header.frame_or_sample_number, sample_number);
    }

    #[test]
    fn decodes_multiple_constant_frames() {
        let mut flac = flac_with_streaminfo(StreamInfoFixture {
            min_block_size: 2,
            max_block_size: 2,
            min_frame_size: 0,
            max_frame_size: 0,
            sample_rate: 48_000,
            channels: 1,
            bits_per_sample: 16,
            total_samples: 4,
        });
        flac.extend_from_slice(&single_channel_constant_frame(0, 10));
        flac.extend_from_slice(&single_channel_constant_frame(1, 20));

        let decoded = decode(&flac).unwrap();

        assert_eq!(decoded.sample_rate, 48_000);
        assert_eq!(decoded.channels, 1);
        assert_eq!(decoded.frames(), 4);
        assert_eq!(
            decoded.samples,
            vec![
                10.0 / 32_767.0,
                10.0 / 32_767.0,
                20.0 / 32_767.0,
                20.0 / 32_767.0,
            ]
        );
    }

    #[test]
    fn validates_streaminfo_md5_when_present() {
        let mut flac = flac_with_streaminfo_and_md5(
            StreamInfoFixture {
                min_block_size: 2,
                max_block_size: 2,
                min_frame_size: 0,
                max_frame_size: 0,
                sample_rate: 48_000,
                channels: 1,
                bits_per_sample: 16,
                total_samples: 2,
            },
            [
                0x8e, 0x20, 0xe9, 0x73, 0x99, 0x77, 0xbd, 0x6e, 0x89, 0x1e, 0xd7, 0x2b, 0x1a, 0x2a,
                0xde, 0xa0,
            ],
        );
        flac.extend_from_slice(&single_channel_constant_frame(0, 10));

        let decoded = decode(&flac).unwrap();

        assert_eq!(decoded.frames(), 2);
    }

    #[test]
    fn rejects_streaminfo_md5_mismatch() {
        let mut flac = flac_with_streaminfo_and_md5(
            StreamInfoFixture {
                min_block_size: 2,
                max_block_size: 2,
                min_frame_size: 0,
                max_frame_size: 0,
                sample_rate: 48_000,
                channels: 1,
                bits_per_sample: 16,
                total_samples: 2,
            },
            [0xff; 16],
        );
        flac.extend_from_slice(&single_channel_constant_frame(0, 10));

        assert!(decode(&flac).is_err());
    }

    #[test]
    fn rejects_non_monotonic_frame_numbers() {
        let mut flac = flac_with_streaminfo(StreamInfoFixture {
            min_block_size: 2,
            max_block_size: 2,
            min_frame_size: 0,
            max_frame_size: 0,
            sample_rate: 48_000,
            channels: 1,
            bits_per_sample: 16,
            total_samples: 4,
        });
        flac.extend_from_slice(&single_channel_constant_frame(0, 10));
        flac.extend_from_slice(&single_channel_constant_frame(2, 20));

        assert!(decode(&flac).is_err());
    }

    #[test]
    fn rejects_non_final_frame_block_size_below_streaminfo_minimum() {
        let mut flac = flac_with_streaminfo(StreamInfoFixture {
            min_block_size: 4,
            max_block_size: 4,
            min_frame_size: 0,
            max_frame_size: 0,
            sample_rate: 48_000,
            channels: 1,
            bits_per_sample: 16,
            total_samples: 4,
        });
        flac.extend_from_slice(&single_channel_constant_frame(0, 10));
        flac.extend_from_slice(&single_channel_constant_frame(1, 20));

        assert!(decode(&flac).is_err());
    }

    #[test]
    fn allows_final_frame_block_size_below_streaminfo_minimum() {
        let mut flac = flac_with_streaminfo(StreamInfoFixture {
            min_block_size: 4,
            max_block_size: 4,
            min_frame_size: 0,
            max_frame_size: 0,
            sample_rate: 48_000,
            channels: 1,
            bits_per_sample: 16,
            total_samples: 2,
        });
        flac.extend_from_slice(&single_channel_constant_frame(0, 10));

        let decoded = decode(&flac).unwrap();

        assert_eq!(decoded.frames(), 2);
    }

    #[test]
    fn rejects_frame_size_below_streaminfo_minimum() {
        let mut flac = flac_with_streaminfo(StreamInfoFixture {
            min_block_size: 2,
            max_block_size: 2,
            min_frame_size: 13,
            max_frame_size: 0,
            sample_rate: 48_000,
            channels: 1,
            bits_per_sample: 16,
            total_samples: 2,
        });
        flac.extend_from_slice(&single_channel_constant_frame(0, 10));

        assert!(decode(&flac).is_err());
    }

    #[test]
    fn rejects_frame_size_above_streaminfo_maximum() {
        let mut flac = flac_with_streaminfo(StreamInfoFixture {
            min_block_size: 2,
            max_block_size: 2,
            min_frame_size: 0,
            max_frame_size: 7,
            sample_rate: 48_000,
            channels: 1,
            bits_per_sample: 16,
            total_samples: 2,
        });
        flac.extend_from_slice(&single_channel_constant_frame(0, 10));

        assert!(decode(&flac).is_err());
    }

    #[test]
    fn rejects_total_sample_count_mismatch() {
        let mut flac = flac_with_streaminfo(StreamInfoFixture {
            min_block_size: 2,
            max_block_size: 2,
            min_frame_size: 0,
            max_frame_size: 0,
            sample_rate: 48_000,
            channels: 1,
            bits_per_sample: 16,
            total_samples: 3,
        });
        flac.extend_from_slice(&single_channel_constant_frame(0, 10));

        assert!(decode(&flac).is_err());
    }

    #[test]
    fn decodes_single_fixed_frame() {
        let mut flac = flac_with_streaminfo(StreamInfoFixture {
            min_block_size: 4,
            max_block_size: 4,
            min_frame_size: 0,
            max_frame_size: 0,
            sample_rate: 48_000,
            channels: 1,
            bits_per_sample: 16,
            total_samples: 4,
        });
        flac.extend_from_slice(&flac_frame(
            &[0xff, 0xf8, 0x6a, 0x08, 0x00, 0x03],
            &fixed_order_one_subframe(),
        ));

        let decoded = decode(&flac).unwrap();

        assert_eq!(decoded.sample_rate, 48_000);
        assert_eq!(decoded.channels, 1);
        assert_eq!(decoded.frames(), 4);
        assert_eq!(
            decoded.samples,
            vec![
                10.0 / 32_767.0,
                11.0 / 32_767.0,
                13.0 / 32_767.0,
                16.0 / 32_767.0,
            ]
        );
    }

    #[test]
    fn decodes_single_lpc_frame() {
        let mut flac = flac_with_streaminfo(StreamInfoFixture {
            min_block_size: 4,
            max_block_size: 4,
            min_frame_size: 0,
            max_frame_size: 0,
            sample_rate: 48_000,
            channels: 1,
            bits_per_sample: 16,
            total_samples: 4,
        });
        flac.extend_from_slice(&flac_frame(
            &[0xff, 0xf8, 0x6a, 0x08, 0x00, 0x03],
            &lpc_order_one_subframe(),
        ));

        let decoded = decode(&flac).unwrap();

        assert_eq!(decoded.sample_rate, 48_000);
        assert_eq!(decoded.channels, 1);
        assert_eq!(decoded.frames(), 4);
        assert_eq!(
            decoded.samples,
            vec![
                10.0 / 32_767.0,
                11.0 / 32_767.0,
                13.0 / 32_767.0,
                16.0 / 32_767.0,
            ]
        );
    }

    #[test]
    fn rejects_bad_frame_footer_crc() {
        let mut flac = flac_with_streaminfo(StreamInfoFixture {
            min_block_size: 2,
            max_block_size: 2,
            min_frame_size: 0,
            max_frame_size: 0,
            sample_rate: 48_000,
            channels: 1,
            bits_per_sample: 16,
            total_samples: 2,
        });
        flac.extend_from_slice(&single_channel_constant_frame(0, 10));
        let last = flac.len() - 1;
        flac[last] ^= 0x01;

        assert!(decode(&flac).is_err());
    }

    #[test]
    fn decodes_left_side_stereo_frame() {
        let decoded = decode(&stereo_constant_flac(0x88, 20, 5, 16, 17)).unwrap();

        assert_eq!(decoded.channels, 2);
        assert_eq!(decoded.frames(), 2);
        assert_eq!(
            decoded.samples,
            vec![
                20.0 / 32_767.0,
                15.0 / 32_767.0,
                20.0 / 32_767.0,
                15.0 / 32_767.0,
            ]
        );
    }

    #[test]
    fn decodes_right_side_stereo_frame() {
        let decoded = decode(&stereo_constant_flac(0x98, 5, 15, 17, 16)).unwrap();

        assert_eq!(decoded.channels, 2);
        assert_eq!(decoded.frames(), 2);
        assert_eq!(
            decoded.samples,
            vec![
                20.0 / 32_767.0,
                15.0 / 32_767.0,
                20.0 / 32_767.0,
                15.0 / 32_767.0,
            ]
        );
    }

    #[test]
    fn decodes_mid_side_stereo_frame() {
        let decoded = decode(&stereo_constant_flac(0xa8, 17, 6, 16, 17)).unwrap();

        assert_eq!(decoded.channels, 2);
        assert_eq!(decoded.frames(), 2);
        assert_eq!(
            decoded.samples,
            vec![
                20.0 / 32_767.0,
                14.0 / 32_767.0,
                20.0 / 32_767.0,
                14.0 / 32_767.0,
            ]
        );
    }

    struct StreamInfoFixture {
        min_block_size: u16,
        max_block_size: u16,
        min_frame_size: u32,
        max_frame_size: u32,
        sample_rate: u32,
        channels: u8,
        bits_per_sample: u8,
        total_samples: u64,
    }

    fn flac_with_streaminfo(fixture: StreamInfoFixture) -> Vec<u8> {
        flac_with_streaminfo_and_md5(fixture, [0; 16])
    }

    fn flac_with_streaminfo_and_md5(fixture: StreamInfoFixture, md5: [u8; 16]) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(b"fLaC");
        out.push(0x80);
        out.extend_from_slice(&34_u32.to_be_bytes()[1..4]);
        out.extend_from_slice(&fixture.min_block_size.to_be_bytes());
        out.extend_from_slice(&fixture.max_block_size.to_be_bytes());
        out.extend_from_slice(&fixture.min_frame_size.to_be_bytes()[1..4]);
        out.extend_from_slice(&fixture.max_frame_size.to_be_bytes()[1..4]);

        let packed = (u64::from(fixture.sample_rate) << 44)
            | (u64::from(fixture.channels - 1) << 41)
            | (u64::from(fixture.bits_per_sample - 1) << 36)
            | fixture.total_samples;
        out.extend_from_slice(&packed.to_be_bytes());
        out.extend_from_slice(&md5);
        out
    }

    fn test_streaminfo() -> StreamInfo {
        parse_streaminfo(&flac_with_streaminfo(StreamInfoFixture {
            min_block_size: 192,
            max_block_size: 4096,
            min_frame_size: 0,
            max_frame_size: 0,
            sample_rate: 48_000,
            channels: 2,
            bits_per_sample: 16,
            total_samples: 0,
        }))
        .unwrap()
    }

    fn fixed_order_one_subframe() -> Vec<u8> {
        let mut writer = BitWriter::new();
        writer.write_bits(0, 1);
        writer.write_bits(9, 6);
        writer.write_bits(0, 1);
        writer.write_signed_bits(10, 16);
        writer.write_bits(0, 2);
        writer.write_bits(0, 4);
        writer.write_bits(2, 4);
        writer.write_rice_signed(1, 2);
        writer.write_rice_signed(2, 2);
        writer.write_rice_signed(3, 2);
        writer.finish()
    }

    fn lpc_order_one_subframe() -> Vec<u8> {
        let mut writer = BitWriter::new();
        writer.write_bits(0, 1);
        writer.write_bits(32, 6);
        writer.write_bits(0, 1);
        writer.write_signed_bits(10, 16);
        writer.write_bits(3, 4);
        writer.write_signed_bits(0, 5);
        writer.write_signed_bits(1, 4);
        writer.write_bits(0, 2);
        writer.write_bits(0, 4);
        writer.write_bits(2, 4);
        writer.write_rice_signed(1, 2);
        writer.write_rice_signed(2, 2);
        writer.write_rice_signed(3, 2);
        writer.finish()
    }

    fn stereo_constant_flac(
        channel_assignment_and_sample_size: u8,
        first_sample: i32,
        second_sample: i32,
        first_bits_per_sample: u8,
        second_bits_per_sample: u8,
    ) -> Vec<u8> {
        let mut flac = flac_with_streaminfo(StreamInfoFixture {
            min_block_size: 2,
            max_block_size: 2,
            min_frame_size: 0,
            max_frame_size: 0,
            sample_rate: 48_000,
            channels: 2,
            bits_per_sample: 16,
            total_samples: 2,
        });
        let header_without_crc = [
            0xff,
            0xf8,
            0x6a,
            channel_assignment_and_sample_size,
            0x00,
            0x01,
        ];
        let mut writer = BitWriter::new();
        writer.write_constant_subframe(first_sample, first_bits_per_sample);
        writer.write_constant_subframe(second_sample, second_bits_per_sample);
        flac.extend_from_slice(&flac_frame(&header_without_crc, &writer.finish()));
        flac
    }

    fn single_channel_constant_frame(frame_number: u8, sample: i32) -> Vec<u8> {
        let mut writer = BitWriter::new();
        writer.write_constant_subframe(sample, 16);
        flac_frame(
            &[0xff, 0xf8, 0x6a, 0x08, frame_number, 0x01],
            &writer.finish(),
        )
    }

    fn frame_header<const N: usize>(header_without_crc: [u8; N]) -> Vec<u8> {
        let mut header = header_without_crc.to_vec();
        header.push(crc8(&header));
        header
    }

    fn flac_frame(header_without_crc: &[u8], subframes: &[u8]) -> Vec<u8> {
        let mut frame = header_without_crc.to_vec();
        frame.push(crc8(&frame));
        frame.extend_from_slice(subframes);
        frame.extend_from_slice(&crc16(&frame).to_be_bytes());
        frame
    }

    fn utf8_coded_number(value: u64) -> Vec<u8> {
        assert!(value <= 0x000f_ffff_ffff);
        if value <= 0x7f {
            return vec![value as u8];
        }
        if value <= 0x7ff {
            return vec![0xc0 | ((value >> 6) as u8), 0x80 | ((value & 0x3f) as u8)];
        }
        if value <= 0xffff {
            return vec![
                0xe0 | ((value >> 12) as u8),
                0x80 | (((value >> 6) & 0x3f) as u8),
                0x80 | ((value & 0x3f) as u8),
            ];
        }
        if value <= 0x1f_ffff {
            return vec![
                0xf0 | ((value >> 18) as u8),
                0x80 | (((value >> 12) & 0x3f) as u8),
                0x80 | (((value >> 6) & 0x3f) as u8),
                0x80 | ((value & 0x3f) as u8),
            ];
        }
        if value <= 0x03ff_ffff {
            return vec![
                0xf8 | ((value >> 24) as u8),
                0x80 | (((value >> 18) & 0x3f) as u8),
                0x80 | (((value >> 12) & 0x3f) as u8),
                0x80 | (((value >> 6) & 0x3f) as u8),
                0x80 | ((value & 0x3f) as u8),
            ];
        }
        if value <= 0x7fff_ffff {
            return vec![
                0xfc | ((value >> 30) as u8),
                0x80 | (((value >> 24) & 0x3f) as u8),
                0x80 | (((value >> 18) & 0x3f) as u8),
                0x80 | (((value >> 12) & 0x3f) as u8),
                0x80 | (((value >> 6) & 0x3f) as u8),
                0x80 | ((value & 0x3f) as u8),
            ];
        }
        vec![
            0xfe,
            0x80 | (((value >> 30) & 0x3f) as u8),
            0x80 | (((value >> 24) & 0x3f) as u8),
            0x80 | (((value >> 18) & 0x3f) as u8),
            0x80 | (((value >> 12) & 0x3f) as u8),
            0x80 | (((value >> 6) & 0x3f) as u8),
            0x80 | ((value & 0x3f) as u8),
        ]
    }

    fn assert_pcm_close(actual: &[f32], expected: &[f32], epsilon: f32) {
        assert_eq!(actual.len(), expected.len());
        for (&actual, &expected) in actual.iter().zip(expected) {
            assert!(
                (actual - expected).abs() <= epsilon,
                "sample mismatch: actual={actual}, expected={expected}, epsilon={epsilon}"
            );
        }
    }

    fn assert_encoded_fixed_order(samples: Vec<i32>, expected_order: u8) {
        let pcm_samples = samples
            .iter()
            .map(|&sample| sample as f32 / 32_767.0)
            .collect::<Vec<_>>();
        let pcm = AudioBuffer::new(48_000, 1, pcm_samples).unwrap();

        let flac = super::encode(&pcm).unwrap();
        let info = parse_streaminfo(&flac).unwrap();
        let header = parse_frame_header(&flac[42..], &info).unwrap();
        let subframe_type = flac[42 + header.header_len] >> 1;
        let decoded = decode(&flac).unwrap();

        assert_eq!(subframe_type, 8 + expected_order);
        assert_eq!(decoded.frames(), samples.len());
        assert_pcm_close(&decoded.samples, &pcm.samples, 1.0 / 32_767.0);
    }

    struct BitWriter {
        bytes: Vec<u8>,
        bit_pos: usize,
    }

    impl BitWriter {
        fn new() -> Self {
            Self {
                bytes: Vec::new(),
                bit_pos: 0,
            }
        }

        fn write_bits(&mut self, value: u32, count: u8) {
            for bit_index in (0..count).rev() {
                let bit = ((value >> bit_index) & 1) as u8;
                self.write_bit(bit);
            }
        }

        fn write_signed_bits(&mut self, value: i32, count: u8) {
            let mask = if count == 32 {
                u32::MAX
            } else {
                (1_u32 << count) - 1
            };
            self.write_bits((value as u32) & mask, count);
        }

        fn write_rice_signed(&mut self, value: i32, rice_parameter: u8) {
            let folded = if value >= 0 {
                (value as u32) << 1
            } else {
                ((-value as u32) << 1) - 1
            };
            let quotient = folded >> rice_parameter;
            for _ in 0..quotient {
                self.write_bit(0);
            }
            self.write_bit(1);
            if rice_parameter > 0 {
                self.write_bits(folded & ((1_u32 << rice_parameter) - 1), rice_parameter);
            }
        }

        fn write_constant_subframe(&mut self, sample: i32, bits_per_sample: u8) {
            self.write_bits(0, 1);
            self.write_bits(0, 6);
            self.write_bits(0, 1);
            self.write_signed_bits(sample, bits_per_sample);
        }

        fn finish(self) -> Vec<u8> {
            self.bytes
        }

        fn write_bit(&mut self, bit: u8) {
            if self.bit_pos % 8 == 0 {
                self.bytes.push(0);
            }
            let byte_index = self.bit_pos / 8;
            let bit_index = 7 - (self.bit_pos % 8);
            self.bytes[byte_index] |= bit << bit_index;
            self.bit_pos += 1;
        }
    }
}
