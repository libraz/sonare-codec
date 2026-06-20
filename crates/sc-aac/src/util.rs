use super::*;

pub(crate) fn non_empty_table<'a>(
    table: &'a [HuffmanEntry<AacSpectralPair>],
    name: &'static str,
) -> Result<&'a [HuffmanEntry<AacSpectralPair>], Error> {
    if table.is_empty() {
        return Err(Error::UnsupportedFeature(name));
    }
    Ok(table)
}

pub(crate) fn non_empty_magnitude_table<'a>(
    table: &'a [HuffmanEntry<AacSpectralMagnitudePair>],
    name: &'static str,
) -> Result<&'a [HuffmanEntry<AacSpectralMagnitudePair>], Error> {
    if table.is_empty() {
        return Err(Error::UnsupportedFeature(name));
    }
    Ok(table)
}

pub(crate) fn non_empty_quad_table<'a>(
    table: &'a [HuffmanEntry<AacSpectralMagnitudeQuad>],
    name: &'static str,
) -> Result<&'a [HuffmanEntry<AacSpectralMagnitudeQuad>], Error> {
    if table.is_empty() {
        return Err(Error::UnsupportedFeature(name));
    }
    Ok(table)
}

pub(crate) fn non_empty_signed_quad_table<'a>(
    table: &'a [HuffmanEntry<AacSpectralQuad>],
    name: &'static str,
) -> Result<&'a [HuffmanEntry<AacSpectralQuad>], Error> {
    if table.is_empty() {
        return Err(Error::UnsupportedFeature(name));
    }
    Ok(table)
}

pub(crate) fn validate_scale_factor_band_offsets(
    quantized: &[i32],
    offsets: &[usize],
) -> Result<(), Error> {
    if offsets.len() < 2 || offsets.first().copied() != Some(0) {
        return Err(Error::InvalidInput("invalid AAC scale-factor offsets"));
    }
    if offsets.last().copied() != Some(quantized.len()) {
        return Err(Error::InvalidInput(
            "AAC scale-factor offsets must cover the spectrum",
        ));
    }
    for band in offsets.windows(2) {
        if band[0] >= band[1] {
            return Err(Error::InvalidInput("invalid AAC scale-factor offsets"));
        }
    }
    Ok(())
}

pub(crate) fn offset_band_index(offsets: &[usize], offset: usize) -> Result<usize, Error> {
    offsets
        .iter()
        .position(|&candidate| candidate == offset)
        .ok_or(Error::InvalidInput(
            "AAC section boundary is not a scale-factor offset",
        ))
}

pub(crate) fn pcm_frame_starts(
    pcm: &AudioBuffer,
    first_start_frame: usize,
) -> Result<Vec<usize>, Error> {
    let frames = pcm.frames();
    if first_start_frame > frames {
        return Err(Error::InvalidInput(
            "AAC PCM start frame is past end of input",
        ));
    }
    let frame_count = frames
        .saturating_sub(first_start_frame)
        .div_ceil(1024)
        .max(1);
    (0..frame_count)
        .map(|index| {
            first_start_frame
                .checked_add(index * 1024)
                .ok_or(Error::InvalidInput("AAC PCM frame start overflows"))
        })
        .collect()
}

pub(crate) fn adts_stream_capacity(frame_count: usize) -> usize {
    frame_count.saturating_mul(1024)
}

pub(crate) fn aac_spectral_pair_magnitude(
    pair: AacSpectralPair,
) -> Result<AacSpectralMagnitudePair, Error> {
    AacSpectralMagnitudePair::try_from(pair)
}

pub(crate) fn aac_spectral_quad_magnitude(
    quad: AacSpectralQuad,
) -> Result<AacSpectralMagnitudeQuad, Error> {
    AacSpectralMagnitudeQuad::try_from(quad)
}

pub(crate) fn write_aac_escape_suffix(
    writer: &mut CoreBitWriter,
    magnitude: u16,
) -> Result<(), Error> {
    if magnitude < AAC_ESCAPE_MAGNITUDE {
        return Ok(());
    }

    let mut threshold = AAC_ESCAPE_MAGNITUDE;
    let mut suffix_len = 4_u8;
    while magnitude
        >= threshold
            .checked_mul(2)
            .ok_or(Error::InvalidInput("AAC escape threshold overflows"))?
    {
        writer.write_bits(1, 1)?;
        threshold = threshold
            .checked_mul(2)
            .ok_or(Error::InvalidInput("AAC escape threshold overflows"))?;
        suffix_len = suffix_len
            .checked_add(1)
            .ok_or(Error::InvalidInput("AAC escape suffix length overflows"))?;
    }
    writer.write_bits(0, 1)?;
    writer.write_bits(u32::from(magnitude - threshold), suffix_len)
}

pub(crate) fn write_aac_sign_bit(writer: &mut CoreBitWriter, value: i16) -> Result<(), Error> {
    if value != 0 {
        writer.write_bits(u32::from(value < 0), 1)?;
    }
    Ok(())
}

pub(crate) fn sample_rate_index(sample_rate: u32) -> Result<u8, Error> {
    const SAMPLE_RATES: [u32; 13] = [
        96_000, 88_200, 64_000, 48_000, 44_100, 32_000, 24_000, 22_050, 16_000, 12_000, 11_025,
        8_000, 7_350,
    ];
    SAMPLE_RATES
        .iter()
        .position(|&rate| rate == sample_rate)
        .and_then(|index| u8::try_from(index).ok())
        .ok_or(Error::UnsupportedFeature("AAC sample rate"))
}

pub(crate) fn decode_silent_adts(input: &[u8]) -> Result<AudioBuffer, Error> {
    let mut remaining = input;
    let mut sample_rate = None;
    let mut channels = None;
    let mut frame_count = 0_usize;

    while !remaining.is_empty() {
        let frame = parse_adts_frame(remaining)?;
        let config = AdtsConfig {
            profile: AacProfile::LowComplexity,
            sample_rate: frame.sample_rate,
            channels: frame.channels,
        };
        if frame.profile != AacProfile::LowComplexity || !is_locally_supported_zero_payload(&frame)?
        {
            return Err(Error::UnsupportedFeature(
                "AAC decode currently supports sonare silent AAC-LC ADTS only",
            ));
        }

        match (sample_rate, channels) {
            (Some(sample_rate), Some(channels))
                if sample_rate != frame.sample_rate || channels != frame.channels =>
            {
                return Err(Error::UnsupportedFeature(
                    "AAC ADTS parameter changes within stream",
                ));
            }
            (None, None) => {
                sample_rate = Some(config.sample_rate);
                channels = Some(config.channels);
            }
            _ => return Err(Error::InvalidInput("inconsistent AAC decoder state")),
        }

        frame_count = frame_count
            .checked_add(1)
            .ok_or(Error::InvalidInput("too many AAC ADTS frames"))?;
        remaining = &remaining[frame.frame_len..];
    }

    let sample_rate = sample_rate.ok_or(Error::InvalidInput("AAC stream has no frames"))?;
    let channels = channels.ok_or(Error::InvalidInput("AAC stream has no frames"))?;
    let sample_count = frame_count
        .checked_mul(1024)
        .and_then(|frames| frames.checked_mul(usize::from(channels)))
        .ok_or(Error::InvalidInput("decoded AAC PCM is too large"))?;
    AudioBuffer::new(sample_rate, u16::from(channels), vec![0.0; sample_count])
}

pub(crate) fn is_locally_supported_zero_payload(
    frame: &ParsedAdtsFrame<'_>,
) -> Result<bool, Error> {
    Ok(
        frame.payload == encode_silent_raw_data_block(frame.channels)?
            || frame.payload == encode_zero_spectral_long_block_raw_data_block(frame.channels)?,
    )
}

pub(crate) struct ParsedAdtsFrame<'a> {
    pub(crate) profile: AacProfile,
    pub(crate) sample_rate: u32,
    pub(crate) channels: u8,
    pub(crate) frame_len: usize,
    pub(crate) payload: &'a [u8],
}

pub(crate) fn parse_adts_frame(input: &[u8]) -> Result<ParsedAdtsFrame<'_>, Error> {
    if input.len() < 7 {
        return Err(Error::InvalidInput("truncated AAC ADTS header"));
    }
    if input[0] != 0xff || input[1] & 0xf0 != 0xf0 {
        return Err(Error::InvalidInput("missing AAC ADTS sync word"));
    }

    let protection_absent = input[1] & 0x01 != 0;
    let header_len = if protection_absent { 7 } else { 9 };
    let profile = match (input[2] >> 6) & 0x03 {
        0 => AacProfile::Main,
        1 => AacProfile::LowComplexity,
        2 => AacProfile::ScalableSampleRate,
        _ => return Err(Error::UnsupportedFeature("AAC LTP profile")),
    };
    let sample_rate_index = (input[2] >> 2) & 0x0f;
    let channels = ((input[2] & 0x01) << 2) | ((input[3] >> 6) & 0x03);
    let frame_len = (usize::from(input[3] & 0x03) << 11)
        | (usize::from(input[4]) << 3)
        | usize::from(input[5] >> 5);

    if channels == 0 {
        return Err(Error::UnsupportedFeature(
            "AAC program config elements are not supported",
        ));
    }
    if frame_len < header_len {
        return Err(Error::InvalidInput("invalid AAC ADTS frame length"));
    }
    if input.len() < frame_len {
        return Err(Error::InvalidInput("truncated AAC ADTS frame"));
    }

    Ok(ParsedAdtsFrame {
        profile,
        sample_rate: sample_rate_from_index(sample_rate_index)?,
        channels,
        frame_len,
        payload: &input[header_len..frame_len],
    })
}

pub(crate) fn sample_rate_from_index(index: u8) -> Result<u32, Error> {
    const SAMPLE_RATES: [u32; 13] = [
        96_000, 88_200, 64_000, 48_000, 44_100, 32_000, 24_000, 22_050, 16_000, 12_000, 11_025,
        8_000, 7_350,
    ];
    SAMPLE_RATES
        .get(usize::from(index))
        .copied()
        .ok_or(Error::InvalidInput("invalid AAC sample-rate index"))
}

pub(crate) fn fixed_block<const N: usize>(samples: &[f32]) -> Result<[f32; N], Error> {
    samples
        .try_into()
        .map_err(|_| Error::InvalidInput("analysis block length mismatch"))
}

pub(crate) fn classify_aac_codebook(quantized: &[i32]) -> Result<AacCodebook, Error> {
    let max_abs = quantized
        .iter()
        .map(|coeff| coeff.checked_abs())
        .collect::<Option<Vec<_>>>()
        .ok_or(Error::InvalidInput("AAC spectral coefficient overflows"))?
        .into_iter()
        .max()
        .unwrap_or(0);
    Ok(match max_abs {
        0 => AacCodebook::Zero,
        1..=7 => AacCodebook::UnsignedPairs7,
        8..=12 => AacCodebook::UnsignedPairs9,
        13..=8191 => AacCodebook::Escape,
        _ => {
            return Err(Error::InvalidInput(
                "AAC spectral coefficient exceeds supported codebook range",
            ));
        }
    })
}

pub(crate) fn encode_silent_raw_data_block(channels: u8) -> Result<Vec<u8>, Error> {
    let mut writer = BitWriter::new();
    match channels {
        1 => {
            writer.write_bits(0, 3)?;
            writer.write_bits(0, 4)?;
            write_silent_individual_channel_stream(&mut writer)?;
        }
        2 => {
            writer.write_bits(1, 3)?;
            writer.write_bits(0, 4)?;
            writer.write_bits(0, 1)?;
            write_silent_individual_channel_stream(&mut writer)?;
            write_silent_individual_channel_stream(&mut writer)?;
        }
        _ => {
            return Err(Error::UnsupportedFeature(
                "AAC-LC encode currently supports mono/stereo only",
            ));
        }
    }
    writer.write_bits(7, 3)?;
    Ok(writer.finish_byte_aligned())
}

pub(crate) fn encode_zero_spectral_long_block_raw_data_block(
    channels: u8,
) -> Result<Vec<u8>, Error> {
    let sections = [AacSection {
        start: 0,
        end: 1024,
        codebook: AacCodebook::Zero,
    }];
    let payload = pack_section_data_with_len(&sections, 1024)?;
    let channel = AacLongBlockConfig::new(0, 1);
    match channels {
        1 => pack_single_channel_raw_data_block(channel, &payload),
        2 => pack_channel_pair_raw_data_block(channel, &payload, channel, &payload),
        _ => Err(Error::UnsupportedFeature(
            "AAC-LC encode currently supports mono/stereo only",
        )),
    }
}

pub(crate) fn write_silent_individual_channel_stream(writer: &mut BitWriter) -> Result<(), Error> {
    writer.write_bits(0, 8)?;
    writer.write_bits(0, 1)?;
    writer.write_bits(0, 2)?;
    writer.write_bits(0, 1)?;
    writer.write_bits(0, 6)?;
    writer.write_bits(0, 1)?;
    writer.write_bits(0, 1)?;
    writer.write_bits(0, 1)?;
    writer.write_bits(0, 1)
}

#[derive(Default)]
pub(crate) struct BitWriter {
    pub(crate) out: Vec<u8>,
    pub(crate) bit_pos: u8,
}

impl BitWriter {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn write_bits(&mut self, value: u32, count: u8) -> Result<(), Error> {
        if count > 32 {
            return Err(Error::InvalidInput(
                "cannot write more than 32 bits at once",
            ));
        }
        if count < 32 && value >= (1_u32 << count) {
            return Err(Error::InvalidInput("bit value exceeds width"));
        }

        for bit_index in (0..count).rev() {
            if self.bit_pos == 0 {
                self.out.push(0);
            }
            let bit = ((value >> bit_index) & 1) as u8;
            let byte = self
                .out
                .last_mut()
                .ok_or(Error::InvalidInput("bit writer has no current byte"))?;
            *byte |= bit << (7 - self.bit_pos);
            self.bit_pos = (self.bit_pos + 1) % 8;
        }
        Ok(())
    }

    pub(crate) fn finish_byte_aligned(self) -> Vec<u8> {
        self.out
    }
}
