use super::*;

pub fn decode(input: &[u8]) -> Result<AudioBuffer, Error> {
    let (stream_info, audio_start) = parse_metadata(input)?;
    let mut cursor = audio_start;
    // The STREAMINFO total_samples field is attacker-controlled (up to 2^36-1),
    // so reserving `total_samples * channels` directly is a decompression bomb.
    // Bound the speculative reservation against the actual remaining input: even
    // maximally compressible frames cannot decode to more than this many samples,
    // and `samples.extend()` grows the buffer if the estimate is short.
    const MIN_FLAC_FRAME_BYTES: usize = 6;
    let declared = usize::try_from(stream_info.total_samples)
        .ok()
        .and_then(|frames| frames.checked_mul(usize::from(stream_info.channels)));
    let remaining = input.len().saturating_sub(audio_start);
    let max_block = usize::from(stream_info.max_block_size).max(1);
    let capacity_bound = (remaining / MIN_FLAC_FRAME_BYTES)
        .saturating_add(1)
        .saturating_mul(max_block)
        .saturating_mul(usize::from(stream_info.channels));
    let sample_capacity = declared.unwrap_or(capacity_bound).min(capacity_bound);
    let mut samples = Vec::with_capacity(sample_capacity);
    let mut md5 = Md5::new();
    let mut decoded_frames = 0_usize;
    let mut decoded_frame_count = 0_u64;

    while cursor < input.len() {
        let frame_input = input
            .get(cursor..)
            .ok_or(Error::InvalidInput("FLAC audio data is truncated"))?;
        let decoded = decode_frame(frame_input, &stream_info, &mut md5)?;
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
        let actual_md5: [u8; 16] = md5.finalize().into();
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
    let mut md5 = Md5::new();
    for &sample in &pcm.samples {
        let quantized = quantize_i16(sample);
        pcm_i16.push(quantized);
        md5.update(quantized.to_le_bytes());
    }
    let md5: [u8; 16] = md5.finalize().into();

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

pub(crate) fn is_incomplete_stream_error(err: &Error) -> bool {
    match err {
        Error::Incomplete => true,
        Error::InvalidInput(reason) => {
            reason.contains("truncated") || *reason == "FLAC stream has no audio frames"
        }
        Error::UnsupportedFormat | Error::InvalidPcm(_) | Error::UnsupportedFeature(_) => false,
    }
}

pub(crate) struct DecodedFrame {
    pub(crate) header: FrameHeader,
    pub(crate) frames: usize,
    pub(crate) samples: Vec<f32>,
    pub(crate) bytes_read: usize,
}

pub(crate) fn decode_frame(
    input: &[u8],
    stream_info: &StreamInfo,
    md5: &mut Md5,
) -> Result<DecodedFrame, Error> {
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
    for frame_index in 0..block_size {
        for channel in &channel_samples {
            let sample = *channel
                .get(frame_index)
                .ok_or(Error::InvalidInput("FLAC channel sample is missing"))?;
            update_md5_sample(md5, sample, frame.bits_per_sample)?;
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
        bytes_read,
    })
}

pub(crate) fn update_md5_sample(
    md5: &mut Md5,
    sample: i32,
    bits_per_sample: u8,
) -> Result<(), Error> {
    if bits_per_sample == 0 || bits_per_sample > 32 {
        return Err(Error::InvalidInput("unsupported FLAC sample width"));
    }
    let bytes = usize::from(bits_per_sample).div_ceil(8);
    md5.update(&sample.to_le_bytes()[..bytes]);
    Ok(())
}

pub(crate) fn validate_frame_size(
    bytes_read: usize,
    stream_info: &StreamInfo,
) -> Result<(), Error> {
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

pub(crate) fn encode_block_sizes(total_frames: usize) -> Vec<usize> {
    let mut remaining = total_frames;
    let mut block_sizes = Vec::new();
    while remaining > 0 {
        let block_size = remaining.min(ENCODE_BLOCK_SIZE);
        block_sizes.push(block_size);
        remaining -= block_size;
    }
    block_sizes
}
