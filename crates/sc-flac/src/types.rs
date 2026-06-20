use super::*;

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
