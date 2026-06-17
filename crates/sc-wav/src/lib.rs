#![deny(unsafe_code)]
#![warn(clippy::all)]

use sc_core::{AudioBuffer, Decoder, Encoder, Error};

const RIFF_HEADER_LEN: usize = 12;
const CHUNK_HEADER_LEN: usize = 8;
const PCM_FORMAT: u16 = 1;
const FLOAT_FORMAT: u16 = 3;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WavSampleFormat {
    Pcm16,
    Pcm24,
    Float32,
}

impl WavSampleFormat {
    const fn audio_format(self) -> u16 {
        match self {
            Self::Pcm16 | Self::Pcm24 => PCM_FORMAT,
            Self::Float32 => FLOAT_FORMAT,
        }
    }

    const fn bits_per_sample(self) -> u16 {
        match self {
            Self::Pcm16 => 16,
            Self::Pcm24 => 24,
            Self::Float32 => 32,
        }
    }

    const fn bytes_per_sample(self) -> u16 {
        self.bits_per_sample() / 8
    }
}

#[derive(Default)]
pub struct WavDecoder;

impl WavDecoder {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Decoder for WavDecoder {
    fn decode(&mut self, input: &[u8]) -> Result<AudioBuffer, Error> {
        decode(input)
    }

    fn decode_stream(&mut self, chunk: &[u8]) -> Result<Option<AudioBuffer>, Error> {
        decode(chunk).map(Some)
    }
}

#[derive(Default)]
pub struct WavEncoder;

impl WavEncoder {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Encoder for WavEncoder {
    fn encode(&mut self, pcm: &AudioBuffer) -> Result<Vec<u8>, Error> {
        encode(pcm)
    }
}

#[derive(Clone, Copy, Debug)]
struct FormatChunk {
    audio_format: u16,
    channels: u16,
    sample_rate: u32,
    bits_per_sample: u16,
    block_align: u16,
}

pub fn decode(input: &[u8]) -> Result<AudioBuffer, Error> {
    if input.len() < RIFF_HEADER_LEN {
        return Err(Error::InvalidInput("WAV header is truncated"));
    }
    if input.get(0..4) != Some(b"RIFF") || input.get(8..12) != Some(b"WAVE") {
        return Err(Error::InvalidInput("missing RIFF/WAVE signature"));
    }
    let riff_size = read_u32_le(input, 4)? as usize;
    if riff_size < 4 {
        return Err(Error::InvalidInput("RIFF size is too small"));
    }
    let riff_end = riff_size
        .checked_add(8)
        .ok_or(Error::InvalidInput("RIFF size overflow"))?;
    if riff_end > input.len() {
        return Err(Error::InvalidInput("RIFF data is truncated"));
    }

    let mut cursor = RIFF_HEADER_LEN;
    let mut format = None;
    let mut data = None;

    while cursor < riff_end {
        let header_end = cursor
            .checked_add(CHUNK_HEADER_LEN)
            .ok_or(Error::InvalidInput("chunk header overflow"))?;
        let header = input
            .get(cursor..header_end)
            .ok_or(Error::InvalidInput("chunk header is truncated"))?;
        let chunk_id = header
            .get(0..4)
            .ok_or(Error::InvalidInput("chunk id is truncated"))?;
        let chunk_size = read_u32_le(header, 4)? as usize;
        let data_start = cursor
            .checked_add(CHUNK_HEADER_LEN)
            .ok_or(Error::InvalidInput("chunk offset overflow"))?;
        let data_end = data_start
            .checked_add(chunk_size)
            .ok_or(Error::InvalidInput("chunk size overflow"))?;
        if data_end > riff_end {
            return Err(Error::InvalidInput("chunk exceeds RIFF size"));
        }
        let chunk_data = input
            .get(data_start..data_end)
            .ok_or(Error::InvalidInput("chunk data is truncated"))?;

        match chunk_id {
            b"fmt " => format = Some(parse_format_chunk(chunk_data)?),
            b"data" => data = Some(chunk_data),
            _ => {}
        }

        let padded_size = chunk_size + (chunk_size % 2);
        cursor = data_start
            .checked_add(padded_size)
            .ok_or(Error::InvalidInput("chunk padding overflow"))?;
    }

    let format = format.ok_or(Error::InvalidInput("missing fmt chunk"))?;
    let data = data.ok_or(Error::InvalidInput("missing data chunk"))?;
    decode_samples(format, data)
}

pub fn encode(pcm: &AudioBuffer) -> Result<Vec<u8>, Error> {
    encode_as(pcm, WavSampleFormat::Pcm16)
}

pub fn encode_as(pcm: &AudioBuffer, sample_format: WavSampleFormat) -> Result<Vec<u8>, Error> {
    let channels = pcm.channels;
    if channels == 0 {
        return Err(Error::InvalidPcm("channel count must be non-zero"));
    }
    if pcm.sample_rate == 0 {
        return Err(Error::InvalidPcm("sample rate must be non-zero"));
    }
    if pcm.samples.len() % usize::from(channels) != 0 {
        return Err(Error::InvalidPcm(
            "interleaved sample count must be divisible by channels",
        ));
    }

    let bytes_per_sample = sample_format.bytes_per_sample();
    let block_align = channels
        .checked_mul(bytes_per_sample)
        .ok_or(Error::InvalidPcm("block align overflow"))?;
    let byte_rate = pcm
        .sample_rate
        .checked_mul(u32::from(block_align))
        .ok_or(Error::InvalidPcm("byte rate overflow"))?;
    let data_len = pcm
        .samples
        .len()
        .checked_mul(usize::from(bytes_per_sample))
        .ok_or(Error::InvalidPcm("data length overflow"))?;
    let data_len_u32 =
        u32::try_from(data_len).map_err(|_| Error::InvalidPcm("WAV data exceeds 4 GiB"))?;
    let riff_size = 4_u32
        .checked_add(8 + 16)
        .and_then(|n| n.checked_add(8))
        .and_then(|n| n.checked_add(data_len_u32))
        .ok_or(Error::InvalidPcm("RIFF size overflow"))?;

    let capacity = usize::try_from(riff_size)
        .ok()
        .and_then(|n| n.checked_add(8))
        .ok_or(Error::InvalidPcm("output size overflow"))?;
    let mut out = Vec::with_capacity(capacity);

    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&riff_size.to_le_bytes());
    out.extend_from_slice(b"WAVE");
    out.extend_from_slice(b"fmt ");
    out.extend_from_slice(&16_u32.to_le_bytes());
    out.extend_from_slice(&sample_format.audio_format().to_le_bytes());
    out.extend_from_slice(&channels.to_le_bytes());
    out.extend_from_slice(&pcm.sample_rate.to_le_bytes());
    out.extend_from_slice(&byte_rate.to_le_bytes());
    out.extend_from_slice(&block_align.to_le_bytes());
    out.extend_from_slice(&sample_format.bits_per_sample().to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len_u32.to_le_bytes());

    for sample in &pcm.samples {
        match sample_format {
            WavSampleFormat::Pcm16 => out.extend_from_slice(&quantize_i16(*sample).to_le_bytes()),
            WavSampleFormat::Pcm24 => {
                out.extend_from_slice(&quantize_i24(*sample).to_le_bytes()[0..3]);
            }
            WavSampleFormat::Float32 => {
                out.extend_from_slice(&sample.clamp(-1.0, 1.0).to_le_bytes());
            }
        }
    }

    Ok(out)
}

fn parse_format_chunk(chunk: &[u8]) -> Result<FormatChunk, Error> {
    if chunk.len() < 16 {
        return Err(Error::InvalidInput("fmt chunk is truncated"));
    }

    let audio_format = read_u16_le(chunk, 0)?;
    let channels = read_u16_le(chunk, 2)?;
    let sample_rate = read_u32_le(chunk, 4)?;
    let block_align = read_u16_le(chunk, 12)?;
    let bits_per_sample = read_u16_le(chunk, 14)?;

    if channels == 0 {
        return Err(Error::InvalidInput("WAV channel count is zero"));
    }
    if sample_rate == 0 {
        return Err(Error::InvalidInput("WAV sample rate is zero"));
    }

    Ok(FormatChunk {
        audio_format,
        channels,
        sample_rate,
        bits_per_sample,
        block_align,
    })
}

fn decode_samples(format: FormatChunk, data: &[u8]) -> Result<AudioBuffer, Error> {
    if format.block_align == 0 || data.len() % usize::from(format.block_align) != 0 {
        return Err(Error::InvalidInput("WAV data does not align to frames"));
    }

    let bytes_per_sample = match (format.audio_format, format.bits_per_sample) {
        (PCM_FORMAT, 8) => 1,
        (PCM_FORMAT, 16) => 2,
        (PCM_FORMAT, 24) => 3,
        (PCM_FORMAT, 32) | (FLOAT_FORMAT, 32) => 4,
        _ => return Err(Error::UnsupportedFeature("WAV sample format")),
    };
    let expected_align = usize::from(format.channels)
        .checked_mul(bytes_per_sample)
        .ok_or(Error::InvalidInput("WAV block align overflow"))?;
    if usize::from(format.block_align) != expected_align {
        return Err(Error::InvalidInput("WAV block align does not match format"));
    }

    let sample_count = data.len() / bytes_per_sample;
    let mut samples = Vec::with_capacity(sample_count);
    let mut cursor = 0;
    while cursor < data.len() {
        let sample = match (format.audio_format, format.bits_per_sample) {
            (PCM_FORMAT, 8) => normalize_u8(
                *data
                    .get(cursor)
                    .ok_or(Error::InvalidInput("u8 sample is truncated"))?,
            ),
            (PCM_FORMAT, 16) => normalize_i16(read_i16_le(data, cursor)?),
            (PCM_FORMAT, 24) => normalize_i24(read_i24_le(data, cursor)?),
            (PCM_FORMAT, 32) => normalize_i32(read_i32_le(data, cursor)?),
            (FLOAT_FORMAT, 32) => f32::from_le_bytes(read_array::<4>(data, cursor)?),
            _ => return Err(Error::UnsupportedFeature("WAV sample format")),
        };
        samples.push(sample.clamp(-1.0, 1.0));
        cursor += bytes_per_sample;
    }

    AudioBuffer::new(format.sample_rate, format.channels, samples)
}

fn quantize_i16(sample: f32) -> i16 {
    let sample = sample.clamp(-1.0, 1.0);
    if sample <= -1.0 {
        i16::MIN
    } else {
        (sample * f32::from(i16::MAX)).round() as i16
    }
}

fn quantize_i24(sample: f32) -> i32 {
    let sample = sample.clamp(-1.0, 1.0);
    if sample <= -1.0 {
        -8_388_608
    } else {
        (sample * 8_388_607.0).round() as i32
    }
}

fn normalize_u8(sample: u8) -> f32 {
    (f32::from(sample) - 128.0) / 128.0
}

fn normalize_i16(sample: i16) -> f32 {
    if sample < 0 {
        f32::from(sample) / 32_768.0
    } else {
        f32::from(sample) / f32::from(i16::MAX)
    }
}

fn normalize_i24(sample: i32) -> f32 {
    if sample < 0 {
        sample as f32 / 8_388_608.0
    } else {
        sample as f32 / 8_388_607.0
    }
}

fn normalize_i32(sample: i32) -> f32 {
    if sample < 0 {
        (f64::from(sample) / 2_147_483_648.0) as f32
    } else {
        (f64::from(sample) / 2_147_483_647.0) as f32
    }
}

fn read_u16_le(input: &[u8], offset: usize) -> Result<u16, Error> {
    Ok(u16::from_le_bytes(read_array::<2>(input, offset)?))
}

fn read_i16_le(input: &[u8], offset: usize) -> Result<i16, Error> {
    Ok(i16::from_le_bytes(read_array::<2>(input, offset)?))
}

fn read_u32_le(input: &[u8], offset: usize) -> Result<u32, Error> {
    Ok(u32::from_le_bytes(read_array::<4>(input, offset)?))
}

fn read_i32_le(input: &[u8], offset: usize) -> Result<i32, Error> {
    Ok(i32::from_le_bytes(read_array::<4>(input, offset)?))
}

fn read_i24_le(input: &[u8], offset: usize) -> Result<i32, Error> {
    let bytes = read_array::<3>(input, offset)?;
    let sign = if bytes[2] & 0x80 == 0 { 0x00 } else { 0xff };
    Ok(i32::from_le_bytes([bytes[0], bytes[1], bytes[2], sign]))
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
    use super::{decode, encode, encode_as, WavSampleFormat};
    use sc_core::AudioBuffer;

    #[test]
    fn roundtrips_pcm16_wav_shape() {
        let pcm =
            AudioBuffer::new(48_000, 2, vec![0.0, 0.0, 0.5, -0.5, 1.0, -1.0, 0.25, -0.25]).unwrap();

        let encoded = encode(&pcm).unwrap();
        let decoded = decode(&encoded).unwrap();
        let encoded_again = encode(&decoded).unwrap();

        assert_eq!(decoded.sample_rate, 48_000);
        assert_eq!(decoded.channels, 2);
        assert_eq!(decoded.samples.len(), pcm.samples.len());
        assert_eq!(encoded_again, encoded);
    }

    #[test]
    fn rejects_truncated_header() {
        assert!(decode(b"RIFF").is_err());
    }

    #[test]
    fn rejects_truncated_declared_riff_size() {
        let mut wav = encode(&AudioBuffer::new(48_000, 1, vec![0.0]).unwrap()).unwrap();
        wav[4..8].copy_from_slice(&999_u32.to_le_bytes());

        assert!(decode(&wav).is_err());
    }

    #[test]
    fn encodes_pcm24_and_float32() {
        let pcm = AudioBuffer::new(48_000, 1, vec![-1.0, 0.0, 1.0]).unwrap();
        let pcm24 = encode_as(&pcm, WavSampleFormat::Pcm24).unwrap();
        let float32 = encode_as(&pcm, WavSampleFormat::Float32).unwrap();

        assert_eq!(decode(&pcm24).unwrap().samples.len(), 3);
        assert_eq!(decode(&float32).unwrap().samples, pcm.samples);
    }
}
