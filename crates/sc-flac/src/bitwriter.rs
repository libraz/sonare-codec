use super::*;

pub(crate) struct FlacBitWriter {
    pub(crate) bytes: Vec<u8>,
    pub(crate) bit_pos: usize,
}

impl FlacBitWriter {
    pub(crate) fn with_capacity(capacity: usize) -> Self {
        Self {
            bytes: Vec::with_capacity(capacity),
            bit_pos: 0,
        }
    }

    pub(crate) fn write_bits(&mut self, value: u32, count: u8) {
        for bit_index in (0..count).rev() {
            self.write_bit(((value >> bit_index) & 1) as u8);
        }
    }

    pub(crate) fn write_signed_bits(&mut self, value: i32, count: u8) {
        let mask = if count == 32 {
            u32::MAX
        } else {
            (1_u32 << count) - 1
        };
        self.write_bits((value as u32) & mask, count);
    }

    pub(crate) fn write_rice_signed(&mut self, value: i32, rice_parameter: u8) {
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

    pub(crate) fn finish(self) -> Vec<u8> {
        self.bytes
    }

    pub(crate) fn write_bit(&mut self, bit: u8) {
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
    pub(crate) const fn channels(self) -> u8 {
        match self {
            Self::Independent(channels) => channels,
            Self::LeftSide | Self::RightSide | Self::MidSide => 2,
        }
    }

    pub(crate) fn bits_per_sample_for_channel(
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
