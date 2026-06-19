//! Vorbis single-channel block codec (analysis + entropy round-trip).
//!
//! Combines the analysis pipeline ([`analyze_block`](crate::block::analyze_block))
//! with the floor1 and residue entropy layers to encode one channel's MDCT
//! block into a self-contained bitstream payload and decode it back to a
//! reconstructed spectrum. This proves the pure-Rust path emits a decodable
//! stream end to end; it uses a compact, self-consistent codebook set rather
//! than the standard libvorbis books (which a spec-compatible stream needs).
//! Derivative work of libvorbis/aoTuV (BSD-3-Clause); see `LICENSE-THIRDPARTY`.

// Self-contained per-block codec superseded by `encoder`; retained as a tested
// reference and exercised only by its own round-trip tests.
#![allow(dead_code)]

use crate::analysis::PsyAnalysis;
use crate::block::{analyze_block, FLOOR_MULT};
use crate::codebook::{Codebook, VqBook};
use crate::floor1::{
    decode_post_deviations, encode_post_deviations, low_high_neighbors, Floor1Class, Floor1Encoding,
};
use crate::floor_render::render_floor1;
use crate::oggpack::{BitReader, BitWriter};
use crate::residue::ResidueConfig;

/// floor1 post-value ceiling that pairs with [`FLOOR_MULT`].
const QUANT_Q: i32 = 64;

/// A per-block, single-channel Vorbis codec for one blocksize/sample-rate.
pub struct BlockCodec {
    psy: PsyAnalysis,
    fitter: crate::floor1::Floor1Fitter,
    postlist: Vec<i32>,
    floor: Floor1Encoding,
    residue: ResidueConfig,
    n: usize,
}

/// A complete uniform Huffman book: `1 << len` entries, every codeword `len`
/// bits.
fn complete_book(len: u8) -> Codebook {
    Codebook::new(vec![len; 1usize << len]).expect("complete book")
}

impl BlockCodec {
    /// Builds the codec for 128 MDCT bins at `rate` Hz using the standard
    /// "128 × 4" floor postlist and a 128-level scalar residue book spanning
    /// `[-16, 15.75]`.
    #[must_use]
    pub fn new_128(rate: u32) -> Self {
        let n = 128;
        let postlist = vec![0, 128, 33, 8, 16, 70];
        let psy = PsyAnalysis::new(n, rate);
        let fitter = crate::floor1::Floor1Fitter::new(
            postlist.clone(),
            crate::floor1::Floor1FitInfo::standard(),
        );

        // One floor partition class spanning the four interior posts: subclass
        // 0 codes the literal zero, subclass 1 a full value book.
        let floor = Floor1Encoding {
            quant_q: QUANT_Q,
            partition_class: vec![0],
            classes: vec![Floor1Class {
                dim: 4,
                subs: 1,
                book: 1,              // 16-entry phrase book (cval in 0..2^4)
                subbook: vec![-1, 0], // subclass 1 -> value book at index 0
            }],
            books: vec![complete_book(6), complete_book(4)],
        };

        // Scalar residue book: 128 levels of width 0.25 covering [-16, 15.75],
        // wide enough for sharp peaks and fine enough to keep the floor.
        let quantlist: Vec<f32> = (0..128).map(|i| i as f32).collect();
        let residue_book = VqBook::new(complete_book(7), 1, -16.0, 0.25, false, quantlist);
        let residue = ResidueConfig {
            begin: 0,
            end: 128,
            grouping: 16,
            partitions: 1,
            partitions_per_word: 2,
            stages: 1,
            secondstages: vec![1],
            partbooks: vec![vec![Some(residue_book)]],
            classmetric1: vec![0],
            classmetric2: vec![-1],
            phrasebook: Codebook::new(vec![1]).expect("single-entry phrasebook"),
        };

        Self {
            psy,
            fitter,
            postlist,
            floor,
            residue,
            n,
        }
    }

    /// MDCT bins per block.
    #[must_use]
    pub fn n(&self) -> usize {
        self.n
    }

    /// Encodes one `2n`-sample PCM block into a bitstream payload. Returns
    /// `None` for the wrong length or a silent block (no floor to code).
    #[must_use]
    pub fn encode(&self, pcm: &[f32]) -> Option<Vec<u8>> {
        let block = analyze_block(&self.psy, &self.fitter, &self.postlist, pcm)?;

        let mut w = BitWriter::new();

        // Floor: encode the per-post deviations, then pack the cascade.
        let (lo, hi) = low_high_neighbors(&self.postlist);
        let mut post = block.posts.clone();
        let dev = encode_post_deviations(&self.postlist, &mut post, &lo, &hi, QUANT_Q);
        self.floor.pack(&dev, &mut w);

        // Residue: code the whitened spectrum.
        self.residue.encode(&[block.residue], &mut w);

        Some(w.into_bytes())
    }

    /// Decodes a payload back into the reconstructed spectrum (`residue *
    /// floor`). Returns `None` if the floor packet marks an absent floor.
    #[must_use]
    pub fn decode(&self, bytes: &[u8]) -> Option<Vec<f32>> {
        let mut r = BitReader::new(bytes);

        let dev = self.floor.unpack(&mut r)?;
        let (lo, hi) = low_high_neighbors(&self.postlist);
        let posts = decode_post_deviations(&self.postlist, &dev, &lo, &hi, QUANT_Q);
        let floor = render_floor1(&self.postlist, &posts, FLOOR_MULT, self.n);

        let residue = self.residue.decode(1, &mut r);
        let spectrum: Vec<f32> = residue[0]
            .iter()
            .zip(&floor)
            .map(|(&res, &flr)| res * flr)
            .collect();
        Some(spectrum)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tone_block(amp: f32, bin: usize) -> Vec<f32> {
        let n = 128;
        let rate = 48_000.0f32;
        let freq = bin as f32 * rate / (2.0 * n as f32);
        (0..2 * n)
            .map(|i| amp * (2.0 * std::f32::consts::PI * freq * i as f32 / rate).sin())
            .collect()
    }

    /// Correlation between two equal-length spectra.
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
    fn encode_produces_a_nonempty_packet() {
        let codec = BlockCodec::new_128(48_000);
        let packet = codec.encode(&tone_block(0.6, 20)).expect("packet");
        assert!(!packet.is_empty(), "empty packet");
    }

    #[test]
    fn silent_and_short_blocks_do_not_encode() {
        let codec = BlockCodec::new_128(48_000);
        assert!(codec.encode(&vec![0.0; 256]).is_none(), "silence encoded");
        assert!(
            codec.encode(&vec![0.0; 100]).is_none(),
            "short block encoded"
        );
    }

    #[test]
    fn round_trip_reconstructs_the_spectrum() {
        // Encode a tone, decode the payload, and confirm the reconstructed
        // spectrum tracks the original MDCT (lossy: correlation, not exact).
        let codec = BlockCodec::new_128(48_000);
        let pcm = tone_block(0.6, 20);
        let (mdct, _) = codec.psy.mdct_analysis(&pcm).expect("analysis");

        let packet = codec.encode(&pcm).expect("encode");
        let spectrum = codec.decode(&packet).expect("decode");

        assert_eq!(spectrum.len(), 128);
        let corr = correlation(&mdct, &spectrum);
        assert!(corr > 0.7, "reconstruction correlation too low: {corr}");
    }

    #[test]
    fn round_trip_is_stable_across_tones() {
        let codec = BlockCodec::new_128(48_000);
        for &bin in &[8usize, 16, 24, 40] {
            let pcm = tone_block(0.5, bin);
            let (mdct, _) = codec.psy.mdct_analysis(&pcm).expect("analysis");
            let packet = codec.encode(&pcm).expect("encode");
            let spectrum = codec.decode(&packet).expect("decode");
            let corr = correlation(&mdct, &spectrum);
            assert!(corr > 0.6, "bin {bin}: correlation {corr}");
        }
    }
}
