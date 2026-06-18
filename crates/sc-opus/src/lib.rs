#![deny(unsafe_code)]
#![warn(clippy::all)]

use opus_decoder::{OpusDecoder as PacketDecoder, OpusError};
use sc_core::{detect, AudioBuffer, Decoder, Encoder, Error, Format};

use crate::opus_packet::CeltOpusEncoder;

mod allocation;
mod analysis;
mod band_split;
mod bands;
mod celt_frame;
mod celt_frontend;
mod cwrs;
mod encoder;
mod laplace;
mod mdct;
mod mode;
mod opus_packet;
mod pitch;
mod preemph;
mod quant_all_bands;
mod quant_band;
mod quant_bands;
mod range_coder;
mod rate;
mod tf;
mod theta;
mod vbr;
mod vq;

const OGG_CAPTURE: &[u8; 4] = b"OggS";
const OPUS_HEAD: &[u8; 8] = b"OpusHead";
const OPUS_TAGS: &[u8; 8] = b"OpusTags";
const OPUS_SAMPLE_RATE: u32 = 48_000;
/// The CELT frame-size shift the first-party encoder emits (20 ms = 960 samples).
const OPUS_CELT_LM: i32 = 3;
const OPUS_FRAME_SIZE: usize = 960;
/// Per-channel target bitrate for the first-party fullband CELT encoder.
const OPUS_BITRATE_PER_CHANNEL: i32 = 64_000;
/// PCM scale the CELT encoder works in (`CELT_SIG`, i16 full-scale). The public
/// API takes `[-1, 1]` float, so input is multiplied by this on the way in.
const CELT_SIG_SCALE: f32 = 32_768.0;
/// The CELT decoder's algorithmic delay at 48 kHz: the MDCT overlap (120
/// samples). Signalled as the OpusHead pre-skip so players discard the priming
/// samples and the decoded output aligns with the input.
const OPUS_CELT_PRE_SKIP: u16 = 120;
const OGG_OPUS_SERIAL: u32 = 0x5343_4f50;

#[derive(Default)]
pub struct OpusDecoder {
    pending: Vec<u8>,
}

impl OpusDecoder {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl Decoder for OpusDecoder {
    fn decode(&mut self, input: &[u8]) -> Result<AudioBuffer, Error> {
        decode(input)
    }

    fn decode_stream(&mut self, chunk: &[u8]) -> Result<Option<AudioBuffer>, Error> {
        if chunk.is_empty() && self.pending.is_empty() {
            return Ok(None);
        }
        self.pending.extend_from_slice(chunk);
        if looks_like_incomplete_ogg_opus_prefix(&self.pending) {
            return Ok(None);
        }

        match decode(&self.pending) {
            Ok(pcm) => {
                self.pending.clear();
                Ok(Some(pcm))
            }
            Err(err) if is_incomplete_ogg_stream_error(&err) => Ok(None),
            Err(err) => Err(err),
        }
    }
}

#[derive(Default)]
pub struct OpusEncoder;

impl OpusEncoder {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Encoder for OpusEncoder {
    fn encode(&mut self, pcm: &AudioBuffer) -> Result<Vec<u8>, Error> {
        encode(pcm)
    }
}

pub fn decode(input: &[u8]) -> Result<AudioBuffer, Error> {
    if detect(input) != Some(Format::Opus) {
        return Err(Error::UnsupportedFormat);
    }

    let ogg = parse_ogg_opus(input)?;
    let mut decoder =
        PacketDecoder::new(OPUS_SAMPLE_RATE, usize::from(ogg.channels)).map_err(map_opus_error)?;
    let mut samples = Vec::new();
    let mut skip = usize::from(ogg.pre_skip) * usize::from(ogg.channels);
    let mut scratch =
        vec![0.0_f32; decoder.max_frame_size_per_channel() * usize::from(ogg.channels)];

    for packet in ogg.audio_packets {
        let decoded = decoder
            .decode_float(&packet, &mut scratch, false)
            .map_err(map_opus_error)?;
        let written = decoded * usize::from(ogg.channels);
        let start = skip.min(written);
        skip -= start;
        samples.extend_from_slice(&scratch[start..written]);
    }

    AudioBuffer::new(OPUS_SAMPLE_RATE, u16::from(ogg.channels), samples)
}

/// Encode interleaved `[-1, 1]` float PCM into an Ogg Opus stream using the
/// first-party, pure-Rust CELT encoder (no C dependency, so this also works on
/// the wasm target). The stream is CELT-only fullband at 20 ms per frame.
pub fn encode(pcm: &AudioBuffer) -> Result<Vec<u8>, Error> {
    if pcm.sample_rate != OPUS_SAMPLE_RATE {
        return Err(Error::UnsupportedFeature(
            "Opus encode currently requires 48000 Hz PCM",
        ));
    }
    let channel_count = match pcm.channels {
        1 | 2 => usize::from(pcm.channels),
        _ => {
            return Err(Error::UnsupportedFeature(
                "only mono and stereo Opus encode are supported",
            ))
        }
    };

    let bitrate = OPUS_BITRATE_PER_CHANNEL * channel_count as i32;
    let mut encoder = CeltOpusEncoder::new(channel_count, OPUS_CELT_LM, bitrate, true);
    let frame_samples = encoder.frame_size() * channel_count;

    // The CELT encoder works in the i16-scale CELT_SIG domain; lift the public
    // [-1, 1] float into it (and zero-pad the final partial frame).
    let mut packets = Vec::new();
    let mut frame = vec![0.0_f32; frame_samples];
    for source in pcm.samples.chunks(frame_samples) {
        frame.fill(0.0);
        for (dst, &src) in frame.iter_mut().zip(source) {
            *dst = src * CELT_SIG_SCALE;
        }
        packets.push(encoder.encode_packet(&frame));
    }
    if packets.is_empty() {
        // No input: emit a single silent frame so the stream carries audio.
        packets.push(encoder.encode_packet(&frame));
    }

    Ok(mux_ogg_opus(pcm.channels, packets))
}

#[derive(Debug, PartialEq, Eq)]
struct OpusHead {
    channels: u8,
    pre_skip: u16,
}

#[derive(Debug)]
struct OggOpus {
    channels: u8,
    pre_skip: u16,
    audio_packets: Vec<Vec<u8>>,
}

fn parse_ogg_opus(input: &[u8]) -> Result<OggOpus, Error> {
    let pages = parse_ogg_pages(input)?;
    let packets = collect_packets(&pages)?;
    let (head_packet, remaining) = packets
        .split_first()
        .ok_or(Error::InvalidInput("Ogg Opus stream is missing OpusHead"))?;
    let head = parse_opus_head(head_packet)?;
    let (tags_packet, audio_packets) = remaining
        .split_first()
        .ok_or(Error::InvalidInput("Ogg Opus stream is missing OpusTags"))?;
    if tags_packet.get(..OPUS_TAGS.len()) != Some(OPUS_TAGS) {
        return Err(Error::InvalidInput("Ogg Opus stream is missing OpusTags"));
    }
    Ok(OggOpus {
        channels: head.channels,
        pre_skip: head.pre_skip,
        audio_packets: audio_packets.to_vec(),
    })
}

fn parse_opus_head(packet: &[u8]) -> Result<OpusHead, Error> {
    if packet.get(..OPUS_HEAD.len()) != Some(OPUS_HEAD) {
        return Err(Error::InvalidInput("Ogg Opus stream is missing OpusHead"));
    }
    if packet.len() < 19 {
        return Err(Error::InvalidInput("OpusHead packet is truncated"));
    }
    let version = packet[8];
    if version & 0xf0 != 0 {
        return Err(Error::UnsupportedFeature("unsupported OpusHead version"));
    }
    let channels = packet[9];
    if !matches!(channels, 1 | 2) {
        return Err(Error::UnsupportedFeature(
            "only mono and stereo Opus streams are supported",
        ));
    }
    let pre_skip = u16::from_le_bytes([packet[10], packet[11]]);
    let mapping_family = packet[18];
    if mapping_family != 0 {
        return Err(Error::UnsupportedFeature(
            "only Opus channel mapping family 0 is supported",
        ));
    }
    Ok(OpusHead { channels, pre_skip })
}

#[derive(Debug)]
struct OggPage {
    continued: bool,
    packets: Vec<Vec<u8>>,
    last_packet_complete: bool,
}

fn parse_ogg_pages(mut input: &[u8]) -> Result<Vec<OggPage>, Error> {
    let mut pages = Vec::new();
    while !input.is_empty() {
        if input.len() < 27 {
            return Err(Error::InvalidInput("Ogg page header is truncated"));
        }
        if input.get(..4) != Some(OGG_CAPTURE) {
            return Err(Error::InvalidInput("Ogg capture pattern is missing"));
        }
        if input[4] != 0 {
            return Err(Error::UnsupportedFeature(
                "unsupported Ogg bitstream version",
            ));
        }

        let header_type = input[5];
        let segment_count = usize::from(input[26]);
        let segment_table_end = 27 + segment_count;
        if input.len() < segment_table_end {
            return Err(Error::InvalidInput("Ogg segment table is truncated"));
        }
        let laces = &input[27..segment_table_end];
        let payload_len = laces
            .iter()
            .try_fold(0usize, |sum, &lace| sum.checked_add(usize::from(lace)))
            .ok_or(Error::InvalidInput("Ogg payload length overflows"))?;
        let page_end = segment_table_end
            .checked_add(payload_len)
            .ok_or(Error::InvalidInput("Ogg page length overflows"))?;
        if input.len() < page_end {
            return Err(Error::InvalidInput("Ogg page payload is truncated"));
        }

        let payload = &input[segment_table_end..page_end];
        let mut offset = 0usize;
        let mut packets = Vec::new();
        let mut packet = Vec::new();
        let mut last_packet_complete = true;
        for &lace in laces {
            let lace = usize::from(lace);
            packet.extend_from_slice(&payload[offset..offset + lace]);
            offset += lace;
            last_packet_complete = lace < 255;
            if last_packet_complete {
                packets.push(std::mem::take(&mut packet));
            }
        }
        if !packet.is_empty() {
            packets.push(packet);
        }

        pages.push(OggPage {
            continued: header_type & 0x01 != 0,
            packets,
            last_packet_complete,
        });
        input = &input[page_end..];
    }

    Ok(pages)
}

fn collect_packets(pages: &[OggPage]) -> Result<Vec<Vec<u8>>, Error> {
    let mut packets = Vec::new();
    let mut pending = Vec::new();

    for page in pages {
        let mut start_index = 0usize;
        if page.continued {
            let Some(first) = page.packets.first() else {
                return Err(Error::InvalidInput("continued Ogg page has no packet data"));
            };
            if pending.is_empty() {
                return Err(Error::InvalidInput(
                    "Ogg continuation has no previous packet",
                ));
            }
            pending.extend_from_slice(first);
            start_index = 1;
            if page.packets.len() > 1 || page.last_packet_complete {
                packets.push(std::mem::take(&mut pending));
            }
        } else if !pending.is_empty() {
            return Err(Error::InvalidInput("Ogg packet continuation is missing"));
        }

        for packet in &page.packets[start_index..] {
            packets.push(packet.clone());
        }

        if !page.last_packet_complete {
            if pending.is_empty() {
                let Some(last) = packets.pop() else {
                    return Err(Error::InvalidInput("incomplete Ogg packet is missing data"));
                };
                pending = last;
            }
        } else if !pending.is_empty() {
            return Err(Error::InvalidInput("Ogg continuation did not finish"));
        }
    }

    if !pending.is_empty() {
        return Err(Error::InvalidInput(
            "Ogg stream ends inside a continued packet",
        ));
    }

    Ok(packets)
}

fn mux_ogg_opus(channels: u16, audio_packets: Vec<Vec<u8>>) -> Vec<u8> {
    let mut stream = Vec::new();
    let mut sequence = 0_u32;
    let channels = u8::try_from(channels).expect("validated Opus channel count");

    let mut head = Vec::with_capacity(19);
    head.extend_from_slice(OPUS_HEAD);
    head.push(1);
    head.push(channels);
    head.extend_from_slice(&OPUS_CELT_PRE_SKIP.to_le_bytes());
    head.extend_from_slice(&OPUS_SAMPLE_RATE.to_le_bytes());
    head.extend_from_slice(&0_i16.to_le_bytes());
    head.push(0);
    push_ogg_page(&mut stream, &[head], 0x02, 0, &mut sequence);

    let vendor = b"sonare-codec";
    let mut tags = Vec::with_capacity(16 + vendor.len());
    tags.extend_from_slice(OPUS_TAGS);
    tags.extend_from_slice(&(vendor.len() as u32).to_le_bytes());
    tags.extend_from_slice(vendor);
    tags.extend_from_slice(&0_u32.to_le_bytes());
    push_ogg_page(&mut stream, &[tags], 0x00, 0, &mut sequence);

    let mut granule_position = 0_u64;
    let packet_count = audio_packets.len();
    for (index, packet) in audio_packets.into_iter().enumerate() {
        granule_position += OPUS_FRAME_SIZE as u64;
        let header_type = if index + 1 == packet_count {
            0x04
        } else {
            0x00
        };
        push_ogg_page(
            &mut stream,
            &[packet],
            header_type,
            granule_position,
            &mut sequence,
        );
    }

    stream
}

fn push_ogg_page(
    stream: &mut Vec<u8>,
    packets: &[Vec<u8>],
    header_type: u8,
    granule_position: u64,
    sequence: &mut u32,
) {
    let mut laces = Vec::new();
    let mut payload = Vec::new();
    for packet in packets {
        let mut remaining = packet.len();
        while remaining >= 255 {
            laces.push(255);
            remaining -= 255;
        }
        laces.push(u8::try_from(remaining).expect("lace value"));
        payload.extend_from_slice(packet);
    }

    let mut page = Vec::with_capacity(27 + laces.len() + payload.len());
    page.extend_from_slice(OGG_CAPTURE);
    page.push(0);
    page.push(header_type);
    page.extend_from_slice(&granule_position.to_le_bytes());
    page.extend_from_slice(&OGG_OPUS_SERIAL.to_le_bytes());
    page.extend_from_slice(&sequence.to_le_bytes());
    page.extend_from_slice(&0_u32.to_le_bytes());
    page.push(u8::try_from(laces.len()).expect("Ogg page segment count"));
    page.extend_from_slice(&laces);
    page.extend_from_slice(&payload);

    let crc = ogg_crc(&page);
    page[22..26].copy_from_slice(&crc.to_le_bytes());
    *sequence += 1;
    stream.extend_from_slice(&page);
}

fn ogg_crc(bytes: &[u8]) -> u32 {
    let mut crc = 0_u32;
    for &byte in bytes {
        crc ^= u32::from(byte) << 24;
        for _ in 0..8 {
            crc = if crc & 0x8000_0000 != 0 {
                (crc << 1) ^ 0x04c1_1db7
            } else {
                crc << 1
            };
        }
    }
    crc
}

fn map_opus_error(error: OpusError) -> Error {
    match error {
        OpusError::InvalidPacket => Error::InvalidInput("invalid Opus packet"),
        OpusError::InternalError => Error::InvalidInput("Opus decoder internal error"),
        OpusError::BufferTooSmall => Error::InvalidInput("Opus decode buffer is too small"),
        OpusError::InvalidArgument(_) => Error::InvalidInput("invalid Opus decoder argument"),
    }
}

fn looks_like_incomplete_ogg_opus_prefix(input: &[u8]) -> bool {
    if OGG_CAPTURE.starts_with(input) {
        return true;
    }
    if !input.starts_with(OGG_CAPTURE) || contains(input, OPUS_HEAD) {
        return false;
    }

    match parse_ogg_pages(input).and_then(|pages| collect_packets(&pages)) {
        Ok(packets) => packets.is_empty(),
        Err(err) => is_incomplete_ogg_stream_error(&err),
    }
}

fn is_incomplete_ogg_stream_error(error: &Error) -> bool {
    matches!(
        error,
        Error::InvalidInput(reason)
            if reason.contains("truncated") || reason.contains("ends inside a continued packet")
    )
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    !needle.is_empty()
        && haystack
            .windows(needle.len())
            .any(|window| window == needle)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opus_head(channels: u8, mapping_family: u8) -> [u8; 19] {
        let mut packet = [0; 19];
        packet[..OPUS_HEAD.len()].copy_from_slice(OPUS_HEAD);
        packet[8] = 1;
        packet[9] = channels;
        packet[10..12].copy_from_slice(&312u16.to_le_bytes());
        packet[12..16].copy_from_slice(&OPUS_SAMPLE_RATE.to_le_bytes());
        packet[18] = mapping_family;
        packet
    }

    #[test]
    fn parses_supported_opus_head() {
        let mut packet = opus_head(2, 0);
        let head = parse_opus_head(&packet).expect("head");
        assert_eq!(
            head,
            OpusHead {
                channels: 2,
                pre_skip: 312
            }
        );

        packet[9] = 1;
        let head = parse_opus_head(&packet).expect("mono head");
        assert_eq!(head.channels, 1);
    }

    #[test]
    fn rejects_unsupported_mapping_family() {
        let mut packet = opus_head(2, 1);
        let err = parse_opus_head(&packet).expect_err("mapping family");
        assert!(matches!(err, Error::UnsupportedFeature(_)));

        packet[18] = 0;
        packet[9] = 3;
        let err = parse_opus_head(&packet).expect_err("channels");
        assert!(matches!(err, Error::UnsupportedFeature(_)));
    }

    #[test]
    fn rejects_truncated_ogg_page() {
        let err = parse_ogg_pages(b"OggS").expect_err("truncated");
        assert!(matches!(err, Error::InvalidInput(_)));
    }

    #[test]
    fn encodes_ogg_opus_stream() {
        let pcm = sine_pcm(OPUS_SAMPLE_RATE, 1, 4800, 440.0);
        let encoded = encode(&pcm).expect("encode");

        assert_eq!(&encoded[..4], OGG_CAPTURE);
        assert_eq!(detect(&encoded), Some(Format::Opus));

        let decoded = decode(&encoded).expect("decode");
        assert_eq!(decoded.sample_rate, OPUS_SAMPLE_RATE);
        assert_eq!(decoded.channels, 1);
        assert!(!decoded.samples.is_empty());
        assert!(decoded.samples.iter().any(|sample| sample.abs() > 0.0001));
    }

    #[test]
    fn encodes_stereo_ogg_opus_stream() {
        let pcm = sine_pcm(OPUS_SAMPLE_RATE, 2, 4800, 440.0);
        let encoded = encode(&pcm).expect("encode");
        let decoded = decode(&encoded).expect("decode");

        assert_eq!(decoded.sample_rate, OPUS_SAMPLE_RATE);
        assert_eq!(decoded.channels, 2);
    }

    #[test]
    fn public_roundtrip_reproduces_the_input_signal() {
        // A clean multi-tone the perceptual coder should track closely. Drive
        // enough frames to reach steady state past the cold-start frame.
        let frames = 20;
        let n = OPUS_FRAME_SIZE * frames;
        let mut samples = Vec::with_capacity(n);
        for i in 0..n {
            let t = i as f32 / OPUS_SAMPLE_RATE as f32;
            let tau = std::f32::consts::TAU;
            let value = 0.4 * (tau * 440.0 * t).sin() + 0.2 * (tau * 880.0 * t).sin();
            samples.push(value);
        }
        let pcm = AudioBuffer::new(OPUS_SAMPLE_RATE, 1, samples.clone()).expect("pcm");

        let encoded = encode(&pcm).expect("encode");
        let decoded = decode(&encoded).expect("decode");

        // The codec adds algorithmic delay, so align by the lag that maximises the
        // normalised cross-correlation, then assert the signal is reproduced (a
        // perceptual coder need not be waveform-exact, but a clean tone correlates
        // strongly). This is oracle-free: we compare our own input to our output.
        let (lag, corr) = best_alignment(&samples, &decoded.samples, 1200);
        assert!(
            corr > 0.7,
            "roundtrip correlation {corr} at lag {lag} is too low — the encoder is \
             not reproducing the input"
        );
        // The OpusHead pre-skip discards the CELT priming delay, so the decoded
        // output aligns with the input at (near) zero lag.
        assert!(
            lag <= 8,
            "pre-skip should compensate the codec delay; residual lag {lag}"
        );
    }

    /// Find the delay (in samples) that maximises the normalised cross-correlation
    /// of `output` against a stable mid-signal window of `input`, returning that
    /// lag and the correlation in `[-1, 1]`.
    fn best_alignment(input: &[f32], output: &[f32], max_lag: usize) -> (usize, f32) {
        let w_start = 4_000usize;
        let w_len = 8_000usize.min(input.len().saturating_sub(w_start));
        let win = &input[w_start..w_start + w_len];
        let in_energy: f32 = win.iter().map(|v| v * v).sum();

        let mut best = (0usize, -1.0f32);
        for lag in 0..max_lag {
            let start = w_start + lag;
            if start + w_len > output.len() {
                break;
            }
            let seg = &output[start..start + w_len];
            let dot: f32 = win.iter().zip(seg).map(|(a, b)| a * b).sum();
            let out_energy: f32 = seg.iter().map(|v| v * v).sum();
            let corr = dot / (in_energy * out_energy).sqrt().max(1e-12);
            if corr > best.1 {
                best = (lag, corr);
            }
        }
        best
    }

    #[test]
    fn rejects_unsupported_opus_encode_pcm_shape() {
        let pcm = AudioBuffer::new(44_100, 1, vec![0.0; 1024]).expect("pcm");
        let err = encode(&pcm).expect_err("sample rate");
        assert!(matches!(err, Error::UnsupportedFeature(_)));

        let pcm = AudioBuffer {
            sample_rate: OPUS_SAMPLE_RATE,
            channels: 3,
            samples: vec![0.0; 2880],
        };
        let err = encode(&pcm).expect_err("channels");
        assert!(matches!(err, Error::UnsupportedFeature(_)));
    }

    #[test]
    fn stream_decoder_buffers_incomplete_ogg_opus_prefix() {
        let mut decoder = OpusDecoder::new();
        assert!(decoder.decode_stream(b"O").expect("prefix").is_none());
        assert!(decoder.decode_stream(b"ggS").expect("capture").is_none());
        assert!(decoder
            .decode_stream(&[0; 27])
            .expect("partial page")
            .is_none());
    }

    #[test]
    fn decodes_ffmpeg_generated_ogg_opus_when_available() {
        let Ok(ffmpeg) = std::env::var("SONARE_FFMPEG") else {
            return;
        };
        let path = std::env::temp_dir().join(format!(
            "sonare-codec-opus-smoke-{}-{}.opus",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));

        let status = std::process::Command::new(ffmpeg)
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-f",
                "lavfi",
                "-i",
                "sine=frequency=440:duration=0.05:sample_rate=48000",
                "-ac",
                "1",
                "-c:a",
                "libopus",
                "-y",
            ])
            .arg(&path)
            .status()
            .expect("run ffmpeg");
        assert!(status.success(), "ffmpeg failed with {status}");

        let bytes = std::fs::read(&path).expect("read opus");
        let _ = std::fs::remove_file(&path);
        let decoded = decode(&bytes).expect("decode opus");
        assert_eq!(decoded.sample_rate, OPUS_SAMPLE_RATE);
        assert_eq!(decoded.channels, 1);
        assert!(!decoded.samples.is_empty());
        assert!(decoded.samples.iter().any(|sample| sample.abs() > 0.0001));

        let mut stream_decoder = OpusDecoder::new();
        let chunk_len = (bytes.len() / 3).max(1);
        let mut streamed = None;
        for chunk in bytes.chunks(chunk_len) {
            if let Some(pcm) = stream_decoder.decode_stream(chunk).expect("stream decode") {
                assert!(streamed.is_none(), "stream decoder emitted more than once");
                streamed = Some(pcm);
            }
        }
        let streamed = streamed.expect("streamed opus decode");
        assert_eq!(streamed.sample_rate, decoded.sample_rate);
        assert_eq!(streamed.channels, decoded.channels);
        assert_eq!(streamed.samples.len(), decoded.samples.len());
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn ffmpeg_accepts_encoded_ogg_opus_when_available() {
        let Ok(ffmpeg) = std::env::var("SONARE_FFMPEG") else {
            return;
        };
        let pcm = sine_pcm(OPUS_SAMPLE_RATE, 1, 4800, 440.0);
        let encoded = encode(&pcm).expect("encode");
        let path = std::env::temp_dir().join(format!(
            "sonare-codec-opus-encode-smoke-{}-{}.opus",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));
        std::fs::write(&path, encoded).expect("write opus");

        let status = std::process::Command::new(ffmpeg)
            .args(["-hide_banner", "-loglevel", "error", "-i"])
            .arg(&path)
            .args(["-f", "null", "-"])
            .status()
            .expect("run ffmpeg");
        let _ = std::fs::remove_file(&path);
        assert!(status.success(), "ffmpeg failed with {status}");
    }

    fn sine_pcm(sample_rate: u32, channels: u16, frames: usize, freq: f32) -> AudioBuffer {
        let mut samples = Vec::with_capacity(frames * usize::from(channels));
        for frame in 0..frames {
            let t = frame as f32 / sample_rate as f32;
            let value = (2.0 * std::f32::consts::PI * freq * t).sin() * 0.5;
            for _ in 0..channels {
                samples.push(value);
            }
        }
        AudioBuffer::new(sample_rate, channels, samples).expect("pcm")
    }
}
