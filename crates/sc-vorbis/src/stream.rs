//! Pure-Rust single-channel Vorbis stream round-trip.
//!
//! Composes the finished bricks into a whole-signal codec: the three header
//! packets ([`crate::header`] + a setup placeholder), per-block entropy coding
//! ([`crate::block_codec::BlockCodec`]), and Ogg framing ([`crate::ogg_mux`]),
//! with 50%-overlapping MDCT windows and time-domain-aliasing-cancellation
//! overlap-add on decode. This is the first end-to-end pure-Rust PCM -> Ogg
//! bytes -> PCM path.
//!
//! It is a *self-contained* format: the decoder rebuilds the codec configuration
//! from [`BlockCodec`] rather than parsing the setup header, so the stream is not
//! yet byte-compatible with a standard Vorbis decoder (that needs the standard
//! libvorbis codebooks). The framing, headers, and overlap-add are real; only
//! the codebooks are placeholder. Derivative work of libvorbis/aoTuV
//! (BSD-3-Clause) via its components; see `LICENSE-THIRDPARTY`.

// Self-contained Ogg Vorbis stream superseded by `encoder`; retained as a tested
// reference and exercised only by its own round-trip tests.
#![allow(dead_code)]

use crate::block_codec::BlockCodec;
use crate::header::{pack_comment_header, pack_identification_header};
use crate::mdct::imdct;
use crate::ogg_mux::{demux, mux_vorbis};
use crate::window::vorbis_window;

/// Fixed logical-stream serial for the single stream we emit.
const STREAM_SERIAL: u32 = 0x736f_6e61; // "sona"

/// A single-channel Vorbis stream codec for one blocksize/sample-rate.
pub struct VorbisStream {
    codec: BlockCodec,
    rate: u32,
    n: usize,
}

impl VorbisStream {
    /// Builds the stream codec for 128-bin (256-sample) blocks at `rate` Hz.
    #[must_use]
    pub fn new_128(rate: u32) -> Self {
        Self {
            codec: BlockCodec::new_128(rate),
            rate,
            n: 128,
        }
    }

    /// Encodes one channel of PCM into a complete Ogg Vorbis byte stream.
    ///
    /// The signal is padded by half a block on each side so the first and last
    /// real samples have an overlap partner, then coded in 50%-overlapping
    /// blocks. A block that quantizes to silence is emitted as an empty packet.
    #[must_use]
    pub fn encode(&self, pcm: &[f32]) -> Vec<u8> {
        let n = self.n;
        let m = 2 * n;
        let half = n;

        // Front pad by half a block; pad the tail to a whole number of hops with
        // at least a half block of trailing zeros for the last overlap.
        let mut padded = vec![0.0f32; half];
        padded.extend_from_slice(pcm);
        let needed = pcm.len() + 2 * half;
        let padded_len = needed.div_ceil(half) * half;
        padded.resize(padded_len, 0.0);

        let mut audio: Vec<(Vec<u8>, u64)> = Vec::new();
        let mut pos = 0usize;
        let mut granule = 0u64;
        while pos + m <= padded.len() {
            let frame = &padded[pos..pos + m];
            granule += half as u64;
            // None (silent block) -> empty packet sentinel.
            let packet = self.codec.encode(frame).unwrap_or_default();
            audio.push((packet, granule));
            pos += half;
        }

        let id = pack_identification_header(1, self.rate, 0, 0, 0, m as u32, m as u32);
        let comment = pack_comment_header(b"sonare-codec", &[]);
        // Placeholder setup header: the self-contained decoder reconstructs the
        // codec config from BlockCodec rather than parsing this.
        let setup = b"\x05vorbis".to_vec();

        mux_vorbis(STREAM_SERIAL, &id, &comment, &setup, &audio)
    }

    /// Decodes an Ogg Vorbis byte stream produced by [`encode`](Self::encode)
    /// back into one channel of PCM. Returns `None` on a malformed stream.
    ///
    /// The leading half block of priming pad is dropped, so sample `i` of the
    /// result aligns with sample `i` of the encoded signal (edge samples near the
    /// very start and end are approximate, lacking a full overlap partner).
    #[must_use]
    pub fn decode(&self, bytes: &[u8]) -> Option<Vec<f32>> {
        let n = self.n;
        let m = 2 * n;
        let half = n;

        let packets = demux(bytes)?;
        if packets.len() < 3 {
            return None;
        }
        let audio = &packets[3..];

        let window = vorbis_window(m);
        let mut recon = vec![0.0f32; audio.len() * half + m];

        let mut pos = 0usize;
        for packet in audio {
            let spectrum = if packet.is_empty() {
                vec![0.0f32; n]
            } else {
                self.codec.decode(packet)?
            };
            if spectrum.len() != n {
                return None;
            }
            let time = imdct(&spectrum);
            for (j, (&t, &wnd)) in time.iter().zip(&window).enumerate() {
                recon[pos + j] += t * wnd;
            }
            pos += half;
        }

        // Drop the front half-block of priming pad.
        Some(recon.split_off(half))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine(rate: u32, freq: f32, len: usize) -> Vec<f32> {
        (0..len)
            .map(|i| 0.6 * (2.0 * std::f32::consts::PI * freq * i as f32 / rate as f32).sin())
            .collect()
    }

    fn correlation(a: &[f32], b: &[f32]) -> f32 {
        let dot: f32 = a.iter().zip(b).map(|(&x, &y)| x * y).sum();
        let na: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let nb: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        if na == 0.0 || nb == 0.0 {
            0.0
        } else {
            dot / (na * nb)
        }
    }

    #[test]
    fn encodes_a_real_ogg_stream() {
        let stream = VorbisStream::new_128(48_000);
        let bytes = stream.encode(&sine(48_000, 1000.0, 2048));
        // The stream begins with the Ogg capture pattern and carries the three
        // Vorbis headers plus audio packets.
        assert_eq!(&bytes[..4], b"OggS");
        let packets = demux(&bytes).expect("demux");
        assert!(packets.len() > 3, "no audio packets emitted");
        assert_eq!(packets[0][0], 0x01, "id header type");
        assert_eq!(packets[1][0], 0x03, "comment header type");
        assert_eq!(packets[2][0], 0x05, "setup header type");
    }

    #[test]
    fn pcm_round_trips_through_the_full_stream() {
        // Encode a tone to a full Ogg stream and decode it back, all pure-Rust.
        let stream = VorbisStream::new_128(48_000);
        let pcm = sine(48_000, 1500.0, 4096);
        let bytes = stream.encode(&pcm);
        let decoded = stream.decode(&bytes).expect("decode");

        // Compare the aligned interior (skip a block at each edge: those samples
        // lack a full overlap partner, the standard MDCT edge caveat).
        let m = 256;
        let count = pcm.len().min(decoded.len());
        assert!(count > 4 * m, "decoded too short");
        let lo = m;
        let hi = count - m;
        let corr = correlation(&pcm[lo..hi], &decoded[lo..hi]);
        assert!(corr > 0.5, "round-trip correlation too low: {corr}");
    }

    #[test]
    fn round_trips_multiple_frequencies() {
        let stream = VorbisStream::new_128(44_100);
        for &freq in &[440.0f32, 1000.0, 3000.0] {
            let pcm = sine(44_100, freq, 3000);
            let bytes = stream.encode(&pcm);
            let decoded = stream.decode(&bytes).expect("decode");
            let m = 256;
            let count = pcm.len().min(decoded.len());
            let corr = correlation(&pcm[m..count - m], &decoded[m..count - m]);
            assert!(corr > 0.4, "freq {freq}: correlation {corr}");
        }
    }

    #[test]
    fn decode_rejects_a_non_stream() {
        let stream = VorbisStream::new_128(48_000);
        assert!(stream.decode(b"not ogg").is_none());
    }

    #[test]
    fn silent_input_round_trips_to_near_silence() {
        let stream = VorbisStream::new_128(48_000);
        let pcm = vec![0.0f32; 2048];
        let bytes = stream.encode(&pcm);
        let decoded = stream.decode(&bytes).expect("decode");
        let energy: f32 = decoded.iter().map(|x| x * x).sum();
        assert!(energy < 1e-3, "silence decoded with energy {energy}");
    }
}
