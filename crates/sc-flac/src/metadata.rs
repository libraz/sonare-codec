use super::*;

pub(crate) fn parse_metadata(input: &[u8]) -> Result<(StreamInfo, usize), Error> {
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

pub(crate) fn decode_constant_subframe(
    reader: &mut BitReader<'_>,
    block_size: usize,
    bits_per_sample: u8,
) -> Result<Vec<i32>, Error> {
    let sample = reader.read_signed_bits(bits_per_sample)?;
    Ok(vec![sample; block_size])
}

pub(crate) fn decode_verbatim_subframe(
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

pub(crate) fn decorrelate_channels(
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

pub(crate) fn decode_fixed_subframe(
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

pub(crate) fn decode_lpc_subframe(
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

pub(crate) fn decode_residual(
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

pub(crate) fn read_rice_signed(
    reader: &mut BitReader<'_>,
    rice_parameter: u8,
) -> Result<i32, Error> {
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

pub(crate) fn fixed_prediction(samples: &[i32], predictor_order: usize) -> Result<i32, Error> {
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

pub(crate) fn predictor_history(samples: &[i32], len: usize, back: usize) -> Result<i64, Error> {
    samples
        .get(len.wrapping_sub(back))
        .copied()
        .map(i64::from)
        .ok_or(Error::InvalidInput("missing FLAC fixed predictor history"))
}

pub(crate) fn lpc_prediction(
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

pub(crate) fn read_wasted_bits_per_sample(reader: &mut BitReader<'_>) -> Result<u8, Error> {
    let mut count = 1_u8;
    while !reader.read_bool()? {
        count = count
            .checked_add(1)
            .ok_or(Error::InvalidInput("FLAC wasted bit count overflow"))?;
    }
    Ok(count)
}

pub(crate) fn read_utf8_uint(input: &[u8], offset: usize) -> Result<(u64, usize), Error> {
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

pub(crate) fn decode_block_size(code: u8, input: &[u8], cursor: &mut usize) -> Result<u32, Error> {
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

pub(crate) fn decode_sample_rate(
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

pub(crate) fn decode_channel_assignment(code: u8) -> Result<ChannelAssignment, Error> {
    match code {
        0..=7 => Ok(ChannelAssignment::Independent(code + 1)),
        8 => Ok(ChannelAssignment::LeftSide),
        9 => Ok(ChannelAssignment::RightSide),
        10 => Ok(ChannelAssignment::MidSide),
        _ => Err(Error::InvalidInput("reserved FLAC channel assignment")),
    }
}

pub(crate) fn decode_bits_per_sample(code: u8, stream_info: &StreamInfo) -> Result<u8, Error> {
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

pub(crate) fn normalize_signed_sample(sample: i32, bits_per_sample: u8) -> Result<f32, Error> {
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

pub(crate) fn crc8(input: &[u8]) -> u8 {
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

pub(crate) fn crc16(input: &[u8]) -> u16 {
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

pub(crate) fn parse_streaminfo_block(block: &[u8]) -> Result<StreamInfo, Error> {
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

pub(crate) fn read_u16_be(input: &[u8], offset: usize) -> Result<u16, Error> {
    Ok(u16::from_be_bytes(read_array::<2>(input, offset)?))
}

pub(crate) fn read_u24_be(input: &[u8], offset: usize) -> Result<u32, Error> {
    let bytes = read_array::<3>(input, offset)?;
    Ok(u32::from_be_bytes([0, bytes[0], bytes[1], bytes[2]]))
}

pub(crate) fn read_u64_be(input: &[u8], offset: usize) -> Result<u64, Error> {
    Ok(u64::from_be_bytes(read_array::<8>(input, offset)?))
}

pub(crate) fn read_array<const N: usize>(input: &[u8], offset: usize) -> Result<[u8; N], Error> {
    let end = offset
        .checked_add(N)
        .ok_or(Error::InvalidInput("read offset overflow"))?;
    input
        .get(offset..end)
        .ok_or(Error::InvalidInput("read is truncated"))?
        .try_into()
        .map_err(|_| Error::InvalidInput("read size mismatch"))
}
