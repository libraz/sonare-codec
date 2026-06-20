use super::*;

pub(crate) const MPEG1_LAYER3_SCALE_FACTOR_COMPRESS: [Layer3ScaleFactorCompress; 16] = [
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

pub(crate) fn scale_factor_fits_width(scale_factor: u8, width: u8) -> bool {
    width < 8 && u16::from(scale_factor) < (1_u16 << width)
}

pub(crate) fn write_mp3_scale_factor(
    writer: &mut CoreBitWriter,
    scale_factor: u8,
    width: u8,
) -> Result<(), Error> {
    if !scale_factor_fits_width(scale_factor, width) {
        return Err(Error::InvalidInput("MP3 scale factor exceeds bit width"));
    }
    writer.write_bits(u32::from(scale_factor), width)
}

pub(crate) fn table_magnitude_with_linbits(magnitude: u16, linbits: u8) -> Result<u16, Error> {
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

pub(crate) fn write_mp3_linbits(
    writer: &mut CoreBitWriter,
    magnitude: u16,
    linbits: u8,
) -> Result<(), Error> {
    if linbits == 0 || magnitude < 15 {
        return Ok(());
    }
    writer.write_bits(u32::from(magnitude - 15), linbits)
}

pub(crate) fn write_mp3_sign_bit(writer: &mut CoreBitWriter, value: i16) -> Result<(), Error> {
    if value != 0 {
        writer.write_bits(u32::from(value < 0), 1)?;
    }
    Ok(())
}

pub(crate) fn decode_silent_layer3(input: &[u8]) -> Result<AudioBuffer, Error> {
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

pub(crate) fn bitrate_kbps(version: MpegVersion, layer: Layer, index: u8) -> Result<u16, Error> {
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

pub(crate) fn bitrate_index(
    version: MpegVersion,
    layer: Layer,
    bitrate_kbps: u16,
) -> Result<u8, Error> {
    for index in 1..15 {
        if self::bitrate_kbps(version, layer, index)? == bitrate_kbps {
            return Ok(index);
        }
    }
    Err(Error::UnsupportedFeature("MP3 bitrate"))
}

pub(crate) fn sample_rate(version: MpegVersion, index: u8) -> Result<u32, Error> {
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

pub(crate) fn sample_rate_index(
    version: MpegVersion,
    target_sample_rate: u32,
) -> Result<u8, Error> {
    for index in 0..3 {
        if sample_rate(version, index)? == target_sample_rate {
            return Ok(index);
        }
    }
    Err(Error::UnsupportedFeature("MP3 sample rate"))
}
