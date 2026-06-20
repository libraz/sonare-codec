use super::*;

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

pub(crate) fn decode_subframe_from_reader(
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
