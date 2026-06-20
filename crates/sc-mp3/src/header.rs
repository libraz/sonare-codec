use super::*;

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
    pub(crate) fn pack(&self, writer: &mut BitWriter, version: MpegVersion) -> Result<(), Error> {
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
    pub(crate) fn pack(&self, writer: &mut BitWriter) -> Result<(), Error> {
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
