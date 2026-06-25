//! Opus packet framing (RFC 6716 §3.1) for the first-party CELT-only path.
//!
//! Wraps a raw CELT frame from [`crate::encoder::CeltEncoder`] in an Opus packet:
//! a one-byte TOC (table of contents) followed by the frame. Only the CELT-only,
//! fullband, single-frame (code 0) case is produced here — the configuration the
//! CELT encoder emits. The TOC layout is RFC 6716 §3.1: the top five bits are the
//! configuration number, bit 2 is the stereo flag, and the low two bits are the
//! frame-count code.

// Consumed by the public Opus encode entry point; the live encoder still ships
// via the Opus FFI path.
#![allow(dead_code)]

use crate::encoder::CeltEncoder;

/// The Opus configuration number for CELT-only fullband at 2.5 ms (RFC 6716
/// Table 2). Configs 28..=31 cover 2.5/5/10/20 ms, i.e. `base + LM`.
const CELT_FULLBAND_BASE_CONFIG: u8 = 28;

/// The largest frame-size shift the CELT-only configs encode (20 ms = config 31).
const MAX_CELT_LM: i32 = 3;

/// Build the one-byte Opus TOC for a CELT-only fullband frame of size-shift `lm`
/// (`0..=3` → 2.5/5/10/20 ms), `stereo`, as a single frame (code 0).
#[must_use]
pub fn celt_fullband_toc(lm: i32, stereo: bool) -> u8 {
    debug_assert!((0..=MAX_CELT_LM).contains(&lm), "LM out of CELT range");
    let config = CELT_FULLBAND_BASE_CONFIG + lm as u8; // 28..=31
    (config << 3) | (u8::from(stereo) << 2) // frame-count code 0 in the low bits
}

/// The decoded fields of an Opus TOC byte (RFC 6716 §3.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Toc {
    /// The configuration number (0..=31).
    pub config: u8,
    /// Whether the frame is stereo.
    pub stereo: bool,
    /// The frame-count code (0..=3).
    pub code: u8,
}

/// Parse an Opus TOC byte into its fields.
#[must_use]
pub fn parse_toc(byte: u8) -> Toc {
    Toc {
        config: byte >> 3,
        stereo: byte & 0x04 != 0,
        code: byte & 0x03,
    }
}

/// Frame a single CELT frame as a CELT-only fullband Opus packet (TOC + frame).
#[must_use]
pub fn celt_opus_packet(lm: i32, stereo: bool, frame: &[u8]) -> Vec<u8> {
    let mut packet = Vec::with_capacity(1 + frame.len());
    packet.push(celt_fullband_toc(lm, stereo));
    packet.extend_from_slice(frame);
    packet
}

/// A CELT-only Opus encoder: PCM frames in, self-delimited Opus packets out. It
/// drives [`CeltEncoder`] and prefixes each frame with the matching TOC byte, so
/// the output decodes through any conformant Opus decoder.
pub struct CeltOpusEncoder {
    inner: CeltEncoder,
    stereo: bool,
    lm: i32,
}

impl CeltOpusEncoder {
    /// Create a CELT-only Opus encoder for `channels` (1 or 2) at frame-size
    /// shift `lm`, targeting `bitrate_bps`. `vbr` selects variable-rate budgeting.
    #[must_use]
    pub fn new(channels: usize, lm: i32, bitrate_bps: i32, vbr: bool) -> Self {
        Self {
            inner: CeltEncoder::new(channels, lm, bitrate_bps, vbr),
            stereo: channels == 2,
            lm,
        }
    }

    /// Enable constrained VBR (see [`CeltEncoder::with_constrained_vbr`]).
    #[must_use]
    pub fn with_constrained_vbr(mut self, constrained: bool) -> Self {
        self.inner = self.inner.with_constrained_vbr(constrained);
        self
    }

    /// The frame size in samples this encoder consumes per [`Self::encode_packet`].
    #[must_use]
    pub fn frame_size(&self) -> usize {
        self.inner.frame_size()
    }

    /// Encode one interleaved PCM frame into a complete CELT-only Opus packet.
    /// Returns an error if the range coder overflows the frame byte budget.
    pub fn encode_packet(&mut self, pcm: &[f32]) -> Result<Vec<u8>, sc_core::Error> {
        let frame = self.inner.encode(pcm)?;
        Ok(celt_opus_packet(self.lm, self.stereo, &frame))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use opus_decoder::OpusDecoder as PacketDecoder;

    #[test]
    fn toc_matches_the_rfc_config_table() {
        // CELT-only fullband 20 ms, mono: config 31 -> 31<<3 = 0xF8.
        assert_eq!(celt_fullband_toc(3, false), 0xF8);
        // The same, stereo: bit 2 set -> 0xFC.
        assert_eq!(celt_fullband_toc(3, true), 0xFC);
        // 2.5 ms mono: config 28 -> 0xE0.
        assert_eq!(celt_fullband_toc(0, false), 0xE0);
        // 10 ms mono: config 30 -> 0xF0.
        assert_eq!(celt_fullband_toc(2, false), 0xF0);
    }

    #[test]
    fn toc_round_trips_through_the_parser() {
        for lm in 0..=3 {
            for stereo in [false, true] {
                let toc = parse_toc(celt_fullband_toc(lm, stereo));
                assert_eq!(toc.config, 28 + lm as u8);
                assert_eq!(toc.stereo, stereo);
                assert_eq!(toc.code, 0, "single-frame packets use code 0");
            }
        }
    }

    #[test]
    fn packet_prefixes_the_toc_byte() {
        let frame = [1u8, 2, 3, 4];
        let packet = celt_opus_packet(3, false, &frame);
        assert_eq!(packet[0], 0xF8);
        assert_eq!(&packet[1..], &frame);
    }

    /// A deterministic harmonic + noise PCM frame, interleaved for `channels`.
    /// Amplitude is i16-scale (`±32768`): our CELT encoder operates on the same
    /// `CELT_SIG` domain the decoder reconstructs before its `/32768` to float.
    fn make_pcm(n: usize, channels: usize, salt: u32) -> Vec<f32> {
        let mut s = salt.wrapping_add(1);
        let mut rng = || {
            s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (s >> 9) as f32 / (1u32 << 23) as f32 - 0.5
        };
        let mut pcm = vec![0.0f32; n * channels];
        for i in 0..n {
            let phase = i as f32 / 96.0 * std::f32::consts::TAU;
            let tone: f32 = (1..=6).map(|h| (phase * h as f32).sin() / h as f32).sum();
            for ch in 0..channels {
                pcm[i * channels + ch] = 3000.0 * (tone + 0.05 * rng());
            }
        }
        pcm
    }

    /// Decode one of our CELT-only packets through the independent pure-Rust Opus
    /// decoder: it must parse the TOC, return a full 20 ms frame, and produce
    /// finite audio. This is the end-to-end conformance check of the framing.
    #[test]
    fn celt_packet_decodes_through_an_independent_opus_decoder() {
        let channels = 1;
        let mut enc = CeltOpusEncoder::new(channels, 3, 96_000, true);
        let n = enc.frame_size();
        let mut dec = PacketDecoder::new(48_000, channels).expect("decoder");
        let mut pcm = vec![0.0f32; dec.max_frame_size_per_channel() * channels];

        // Prime past the cold-start frame, then decode a steady-state packet.
        let mut decoded_samples = 0;
        let mut input = Vec::new();
        for f in 0..4 {
            input = make_pcm(n, channels, 10 + f);
            let packet = enc.encode_packet(&input).expect("encode");
            // The TOC must announce CELT-only fullband 20 ms, single frame.
            assert_eq!(parse_toc(packet[0]).config, 31);
            decoded_samples = dec
                .decode_float(&packet, &mut pcm, false)
                .expect("decode our CELT packet");
        }
        assert_eq!(decoded_samples, n, "a 20 ms frame is 960 samples");
        assert!(
            pcm[..decoded_samples].iter().all(|v| v.is_finite()),
            "decoded PCM must be finite"
        );
        // Lossy fidelity, in energy only (never bit-exact vs another decoder):
        // the decoder outputs float = CELT_SIG / 32768, so the reconstructed RMS
        // must land within a broad band of the input RMS scaled the same way.
        let dec_rms = rms(&pcm[..decoded_samples]);
        let in_rms = rms(&input) / 32768.0;
        let ratio = dec_rms / in_rms;
        assert!(
            (0.1..=10.0).contains(&ratio),
            "decoded RMS {dec_rms} vs scaled input RMS {in_rms} (ratio {ratio})"
        );
    }

    fn rms(x: &[f32]) -> f32 {
        (x.iter().map(|v| v * v).sum::<f32>() / x.len().max(1) as f32).sqrt()
    }

    #[test]
    fn stereo_celt_packet_decodes_through_an_independent_opus_decoder() {
        let channels = 2;
        let mut enc = CeltOpusEncoder::new(channels, 3, 128_000, true);
        let n = enc.frame_size();
        let mut dec = PacketDecoder::new(48_000, channels).expect("decoder");
        let mut pcm = vec![0.0f32; dec.max_frame_size_per_channel() * channels];

        let mut decoded_samples = 0;
        for f in 0..4 {
            let packet = enc
                .encode_packet(&make_pcm(n, channels, 20 + f))
                .expect("encode");
            assert!(parse_toc(packet[0]).stereo);
            decoded_samples = dec
                .decode_float(&packet, &mut pcm, false)
                .expect("decode our stereo CELT packet");
        }
        assert_eq!(decoded_samples, n);
        assert!(pcm[..decoded_samples * channels]
            .iter()
            .all(|v| v.is_finite()));
    }
}
