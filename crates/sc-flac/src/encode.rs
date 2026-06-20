use super::*;

pub(crate) fn encode_frame(
    pcm_i16: &[i16],
    channels: usize,
    sample_offset: usize,
    block_size: usize,
    coded_number: usize,
    fixed_blocking: bool,
) -> Result<Vec<u8>, Error> {
    if channels == 2 {
        let left = collect_channel_samples(pcm_i16, 2, sample_offset, block_size, 0)?;
        let right = collect_channel_samples(pcm_i16, 2, sample_offset, block_size, 1)?;
        let side = stereo_side_samples(&left, &right);
        let mid = stereo_mid_samples(&left, &right);
        let mut best = encode_frame_with_channels(
            channels,
            block_size,
            coded_number,
            fixed_blocking,
            1,
            &stereo_independent_channels(&left, &right),
        )?;
        for (channel_assignment_code, encoded_channels) in [
            (8, left_side_channels(&left, &side)),
            (9, right_side_channels(&side, &right)),
            (10, mid_side_channels(&mid, &side)),
        ] {
            let candidate = encode_frame_with_channels(
                channels,
                block_size,
                coded_number,
                fixed_blocking,
                channel_assignment_code,
                &encoded_channels,
            )?;
            if candidate.len() < best.len() {
                best = candidate;
            }
        }
        return Ok(best);
    }

    let mut channel_samples = Vec::with_capacity(channels);
    for channel in 0..channels {
        channel_samples.push(collect_channel_samples(
            pcm_i16,
            channels,
            sample_offset,
            block_size,
            channel,
        )?);
    }
    let encoded_channels = channel_samples
        .iter()
        .map(|samples| EncodedChannel {
            bits_per_sample: ENCODE_BITS_PER_SAMPLE,
            samples,
        })
        .collect::<Vec<_>>();
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

pub(crate) struct EncodedChannel<'a> {
    bits_per_sample: u8,
    samples: &'a [i32],
}

pub(crate) fn encode_frame_with_channels(
    channels: usize,
    block_size: usize,
    coded_number: usize,
    fixed_blocking: bool,
    channel_assignment_code: u8,
    encoded_channels: &[EncodedChannel<'_>],
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
        write_best_subframe(&mut writer, channel.samples, channel.bits_per_sample)?;
    }
    frame.extend_from_slice(&writer.finish());
    frame.extend_from_slice(&crc16(&frame).to_be_bytes());
    Ok(frame)
}

pub(crate) fn collect_channel_samples(
    pcm_i16: &[i16],
    channels: usize,
    sample_offset: usize,
    block_size: usize,
    channel: usize,
) -> Result<Vec<i32>, Error> {
    if channels == 0 || channel >= channels {
        return Err(Error::InvalidPcm("FLAC channel index is out of range"));
    }
    let start = sample_offset
        .checked_mul(channels)
        .and_then(|base| base.checked_add(channel))
        .ok_or(Error::InvalidPcm("FLAC sample index overflow"))?;
    if block_size > 0 {
        let last = start
            .checked_add(
                (block_size - 1)
                    .checked_mul(channels)
                    .ok_or(Error::InvalidPcm("FLAC sample index overflow"))?,
            )
            .ok_or(Error::InvalidPcm("FLAC sample index overflow"))?;
        if last >= pcm_i16.len() {
            return Err(Error::InvalidPcm("FLAC sample is missing"));
        }
    }

    Ok(pcm_i16[start..]
        .iter()
        .step_by(channels)
        .take(block_size)
        .map(|&sample| i32::from(sample))
        .collect())
}

pub(crate) fn stereo_independent_channels<'a>(
    left: &'a [i32],
    right: &'a [i32],
) -> [EncodedChannel<'a>; 2] {
    [
        EncodedChannel {
            bits_per_sample: ENCODE_BITS_PER_SAMPLE,
            samples: left,
        },
        EncodedChannel {
            bits_per_sample: ENCODE_BITS_PER_SAMPLE,
            samples: right,
        },
    ]
}

pub(crate) fn left_side_channels<'a>(left: &'a [i32], side: &'a [i32]) -> [EncodedChannel<'a>; 2] {
    [
        EncodedChannel {
            bits_per_sample: ENCODE_BITS_PER_SAMPLE,
            samples: left,
        },
        EncodedChannel {
            bits_per_sample: ENCODE_BITS_PER_SAMPLE + 1,
            samples: side,
        },
    ]
}

pub(crate) fn right_side_channels<'a>(
    side: &'a [i32],
    right: &'a [i32],
) -> [EncodedChannel<'a>; 2] {
    [
        EncodedChannel {
            bits_per_sample: ENCODE_BITS_PER_SAMPLE + 1,
            samples: side,
        },
        EncodedChannel {
            bits_per_sample: ENCODE_BITS_PER_SAMPLE,
            samples: right,
        },
    ]
}

pub(crate) fn mid_side_channels<'a>(mid: &'a [i32], side: &'a [i32]) -> [EncodedChannel<'a>; 2] {
    [
        EncodedChannel {
            bits_per_sample: ENCODE_BITS_PER_SAMPLE,
            samples: mid,
        },
        EncodedChannel {
            bits_per_sample: ENCODE_BITS_PER_SAMPLE + 1,
            samples: side,
        },
    ]
}

pub(crate) fn stereo_side_samples(left: &[i32], right: &[i32]) -> Vec<i32> {
    left.iter()
        .zip(right)
        .map(|(&left, &right)| left - right)
        .collect()
}

pub(crate) fn stereo_mid_samples(left: &[i32], right: &[i32]) -> Vec<i32> {
    left.iter()
        .zip(right)
        .map(|(&left, &right)| (left + right) >> 1)
        .collect()
}

pub(crate) fn write_best_subframe(
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

pub(crate) fn write_constant_subframe(
    writer: &mut FlacBitWriter,
    sample: i32,
    bits_per_sample: u8,
) {
    writer.write_bits(0, 1);
    writer.write_bits(0, 6);
    writer.write_bits(0, 1);
    writer.write_signed_bits(sample, bits_per_sample);
}

pub(crate) fn write_verbatim_subframe(
    writer: &mut FlacBitWriter,
    samples: &[i32],
    bits_per_sample: u8,
) {
    writer.write_bits(0, 1);
    writer.write_bits(1, 6);
    writer.write_bits(0, 1);
    for &sample in samples {
        writer.write_signed_bits(sample, bits_per_sample);
    }
}

pub(crate) fn write_fixed_rice_subframe(
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

pub(crate) struct FixedRiceCandidate {
    pub(crate) order: u8,
    pub(crate) rice_parameter: u8,
    pub(crate) residuals: Vec<i32>,
}

pub(crate) fn best_fixed_rice(samples: &[i32], bits_per_sample: u8) -> Option<FixedRiceCandidate> {
    if samples.len() < 2 {
        return None;
    }

    let mut best: Option<(usize, FixedRiceCandidate)> = None;
    for order in 1..=4 {
        if samples.len() <= usize::from(order) {
            continue;
        }
        let residuals = fixed_residuals(samples, order)?;
        let mut best_rice_parameter = None;
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
            if best_rice_parameter
                .as_ref()
                .map(|(order_best_bits, _)| bits >= *order_best_bits)
                .unwrap_or(false)
            {
                continue;
            }
            best_rice_parameter = Some((bits, rice_parameter));
        }

        if let Some((bits, rice_parameter)) = best_rice_parameter {
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
                        residuals,
                    },
                ));
            }
        }
    }

    best.map(|(_, candidate)| candidate)
}

pub(crate) fn fixed_residuals(samples: &[i32], order: u8) -> Option<Vec<i32>> {
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

pub(crate) fn fixed_rice_bits(
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

pub(crate) fn folded_rice_value(value: i32) -> u32 {
    if value >= 0 {
        (value as u32) << 1
    } else {
        ((-value as u32) << 1) - 1
    }
}

pub(crate) fn utf8_coded_number(value: u64) -> Result<Vec<u8>, Error> {
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

pub(crate) fn quantize_i16(sample: f32) -> i16 {
    let sample = sample.clamp(-1.0, 1.0);
    if sample <= -1.0 {
        i16::MIN
    } else {
        (sample * f32::from(i16::MAX)).round() as i16
    }
}
