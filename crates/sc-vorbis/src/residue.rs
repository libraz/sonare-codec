//! Vorbis residue (type 0/1) partition coding.
//!
//! Hand-ported to safe Rust from libvorbis/aoTuV `lib/res0.c`: the partition
//! classifier (`_01class`), the multi-stage forward coder (`_01forward` /
//! `_encodepart`) and its inverse (`_01inverse` with `vorbis_book_decodevs_add`),
//! plus the phrase-word `decodemap` from residue setup. Derivative work of
//! libvorbis/aoTuV (BSD-3-Clause); see `LICENSE-THIRDPARTY`.
//!
//! The residue input is treated as `f32` throughout: the VQ cascade picks each
//! partition's nearest lattice vector, subtracts it (the residual the next
//! stage refines), and the decoder adds the same vectors back. libvorbis runs
//! the encoder's nearest-match in the integer domain; residue is lossy and is
//! never compared bit-exactly against another decoder, so the float cascade is
//! a faithful, self-consistent equivalent.

use crate::codebook::{Codebook, VqBook};
use crate::oggpack::{BitReader, BitWriter};

/// A Vorbis residue 0/1 configuration: the coded spectral window
/// `[begin, end)`, the partition `grouping` (samples per partition), the number
/// of partition types, the cascade `stages`, the per-type active-stage bitmask
/// `secondstages`, the classification metrics, and the codebooks.
pub struct ResidueConfig {
    pub begin: usize,
    pub end: usize,
    pub grouping: usize,
    pub partitions: usize,
    pub partitions_per_word: usize,
    pub stages: usize,
    /// Per partition type: bitmask of stages that code residual values.
    pub secondstages: Vec<u32>,
    /// Per partition type, per stage: the VQ book (or `None` if inactive).
    pub partbooks: Vec<Vec<Option<VqBook>>>,
    pub classmetric1: Vec<i32>,
    pub classmetric2: Vec<i32>,
    /// Scalar book coding the mixed-radix phrase word over partition types.
    pub phrasebook: Codebook,
}

impl ResidueConfig {
    /// Number of partitions in the coded window (`(end - begin) / grouping`).
    #[must_use]
    pub fn partition_count(&self) -> usize {
        (self.end - self.begin) / self.grouping
    }

    /// Size of the phrase-word entry space: `partitions ^ partitions_per_word`.
    fn phrase_space(&self) -> usize {
        self.partitions.pow(self.partitions_per_word as u32)
    }

    /// `_01class`: assigns each partition of each channel a partition type from
    /// the magnitude/energy metrics.
    fn classify(&self, channels: &[Vec<f32>]) -> Vec<Vec<usize>> {
        let partvals = self.partition_count();
        let scale = 100.0 / self.grouping as f32;
        let mut types = vec![vec![0usize; partvals]; channels.len()];
        for (j, channel) in channels.iter().enumerate() {
            for i in 0..partvals {
                let offset = i * self.grouping + self.begin;
                let mut max = 0.0f32;
                let mut ent = 0.0f32;
                for &v in &channel[offset..offset + self.grouping] {
                    let a = v.abs();
                    if a > max {
                        max = a;
                    }
                    ent += a;
                }
                ent *= scale;

                let mut k = 0;
                while k < self.partitions - 1 {
                    let cm2 = self.classmetric2[k];
                    if max <= self.classmetric1[k] as f32 && (cm2 < 0 || ent < cm2 as f32) {
                        break;
                    }
                    k += 1;
                }
                types[j][i] = k;
            }
        }
        types
    }

    /// Decomposes a phrase entry into its `partitions_per_word` partition types
    /// (base-`partitions`, most-significant first), mirroring `decodemap`.
    fn decode_map(&self, entry: usize) -> Vec<usize> {
        let dim = self.partitions_per_word;
        let mut out = vec![0usize; dim];
        let mut val = entry;
        let mut mult = self.phrase_space() / self.partitions;
        for slot in out.iter_mut() {
            let deco = val / mult;
            val -= deco * mult;
            mult /= self.partitions;
            *slot = deco;
        }
        out
    }

    /// `_encodepart`: codes one partition's samples through `book`, subtracting
    /// each chosen lattice vector so the next stage refines the residual.
    fn encode_part(book: &VqBook, part: &mut [f32], w: &mut BitWriter) {
        let dim = book.dim();
        let step = part.len() / dim;
        for s in 0..step {
            let sub = &mut part[s * dim..s * dim + dim];
            if let Some(entry) = book.best_entry(sub) {
                book.encode(sub, w);
                let value = book.value_vector(entry);
                for (p, v) in sub.iter_mut().zip(&value) {
                    *p -= v;
                }
            }
        }
    }

    /// Decodes one partition's VQ vectors and adds them to the output slice,
    /// mirroring the subvector loop in `encode_part`.
    fn decode_part(book: &VqBook, part: &mut [f32], r: &mut BitReader) -> Option<()> {
        let dim = book.dim();
        let step = part.len() / dim;
        for s in 0..step {
            let value = book.decode(r)?;
            for (o, v) in part[s * dim..s * dim + dim].iter_mut().zip(&value) {
                *o += v;
            }
        }
        Some(())
    }

    /// `_01forward`: classifies, then codes phrase words and interleaved
    /// residual values across all stages into `w`.
    pub fn encode(&self, channels: &[Vec<f32>], w: &mut BitWriter) {
        let partvals = self.partition_count();
        let ppw = self.partitions_per_word;
        let types = self.classify(channels);
        // Working copy the cascade subtracts from across stages.
        let mut work: Vec<Vec<f32>> = channels.to_vec();

        for s in 0..self.stages {
            let mut i = 0;
            while i < partvals {
                // Stage 0 writes the partition phrase word for each channel.
                if s == 0 {
                    for ch_types in &types {
                        let mut val = ch_types[i];
                        for k in 1..ppw {
                            val *= self.partitions;
                            if i + k < partvals {
                                val += ch_types[i + k];
                            }
                        }
                        if val < self.phrasebook.entries() {
                            self.phrasebook.encode(val, w);
                        }
                    }
                }

                // Then the residual values for this phrase word, interleaved.
                let mut k = 0;
                while k < ppw && i < partvals {
                    let offset = i * self.grouping + self.begin;
                    for (j, ch_types) in types.iter().enumerate() {
                        let t = ch_types[i];
                        if self.secondstages[t] & (1 << s) != 0 {
                            if let Some(book) = &self.partbooks[t][s] {
                                Self::encode_part(
                                    book,
                                    &mut work[j][offset..offset + self.grouping],
                                    w,
                                );
                            }
                        }
                    }
                    k += 1;
                    i += 1;
                }
            }
        }
    }

    /// `_01inverse`: reconstructs each channel's residue by adding the decoded
    /// lattice vectors. Returns one buffer of length `end` per channel; a
    /// truncated packet simply stops (the filled prefix is returned).
    #[must_use]
    pub fn decode(&self, channels: usize, r: &mut BitReader) -> Vec<Vec<f32>> {
        let mut out = vec![vec![0.0f32; self.end]; channels];
        let partvals = self.partition_count();
        if partvals == 0 {
            return out;
        }
        let ppw = self.partitions_per_word;
        let partwords = partvals.div_ceil(ppw);
        // Per channel, per phrase word: the decoded partition types.
        let mut partword = vec![vec![Vec::<usize>::new(); partwords]; channels];

        'stages: for s in 0..self.stages {
            let mut i = 0;
            let mut l = 0;
            while i < partvals {
                if s == 0 {
                    for word in partword.iter_mut().take(channels) {
                        let Some(temp) = self.phrasebook.decode(r) else {
                            break 'stages;
                        };
                        if temp >= self.phrase_space() {
                            break 'stages;
                        }
                        word[l] = self.decode_map(temp);
                    }
                }

                let mut k = 0;
                while k < ppw && i < partvals {
                    let offset = self.begin + i * self.grouping;
                    for (j, word) in partword.iter().enumerate().take(channels) {
                        let t = word[l][k];
                        if self.secondstages[t] & (1 << s) != 0 {
                            if let Some(book) = &self.partbooks[t][s] {
                                let Some(()) = Self::decode_part(
                                    book,
                                    &mut out[j][offset..offset + self.grouping],
                                    r,
                                ) else {
                                    break 'stages;
                                };
                            }
                        }
                    }
                    k += 1;
                    i += 1;
                }
                l += 1;
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Complete uniform Huffman book: `1 << len` entries, all `len` bits.
    fn complete_book(len: u8) -> Codebook {
        Codebook::new(vec![len; 1usize << len]).expect("complete book")
    }

    /// A maptype-1 VQ book over digits `{0,1}`, the given `dim`, unit scale.
    fn lattice_book(dim: usize) -> VqBook {
        let book = complete_book(dim as u8); // 2^dim lattice points
        VqBook::new(book, dim, 0.0, 1.0, false, vec![0.0, 1.0])
    }

    /// Two partition types, one stage, phrase word of 2 partitions.
    fn single_stage_config() -> ResidueConfig {
        ResidueConfig {
            begin: 0,
            end: 16,
            grouping: 4,
            partitions: 2,
            partitions_per_word: 2,
            stages: 1,
            secondstages: vec![1, 1],
            partbooks: vec![vec![Some(lattice_book(2))], vec![Some(lattice_book(2))]],
            classmetric1: vec![1, 0],
            classmetric2: vec![-1, -1],
            phrasebook: complete_book(2), // 4 = 2^2 phrase entries
        }
    }

    /// A signal whose every 2-sample subvector is an exact lattice point, so the
    /// single-stage cascade reproduces it with zero residual.
    fn lattice_signal(frames: usize) -> Vec<f32> {
        (0..frames).map(|i| (i % 2) as f32).collect()
    }

    #[test]
    fn classify_picks_type_by_metric() {
        let cfg = single_stage_config();
        // Partition 0: all 1.0 -> max=1<=cm1[0]=1 -> type 0.
        // Partition 1: a 2.0 -> max=2>cm1[0] -> falls through to type 1.
        let mut data = vec![1.0; 16];
        data[4] = 2.0;
        let types = cfg.classify(&[data]);
        assert_eq!(types[0][0], 0, "low-magnitude partition");
        assert_eq!(types[0][1], 1, "high-magnitude partition");
    }

    #[test]
    fn single_stage_round_trips_on_lattice() {
        let cfg = single_stage_config();
        let channels = vec![lattice_signal(16), lattice_signal(16)];
        let mut w = BitWriter::new();
        cfg.encode(&channels, &mut w);
        let bytes = w.into_bytes();

        let mut r = BitReader::new(&bytes);
        let decoded = cfg.decode(2, &mut r);
        // On-lattice input is reproduced exactly through classify + phrase
        // framing + VQ cascade.
        for j in 0..2 {
            assert_eq!(decoded[j], channels[j], "channel {j}");
        }
    }

    #[test]
    fn two_stages_round_trip_with_zero_residual() {
        // Single partition type (partitions==1) so the phrase word is always 0;
        // a 1-entry phrasebook codes it. Stage 0 nails the on-lattice signal,
        // stage 1 sees zero residual and adds the zero lattice point.
        let cfg = ResidueConfig {
            begin: 0,
            end: 16,
            grouping: 4,
            partitions: 1,
            partitions_per_word: 2,
            stages: 2,
            secondstages: vec![0b11],
            partbooks: vec![vec![Some(lattice_book(2)), Some(lattice_book(2))]],
            classmetric1: vec![0],
            classmetric2: vec![-1],
            phrasebook: Codebook::new(vec![1]).expect("single-entry phrasebook"),
        };
        let channels = vec![lattice_signal(16)];
        let mut w = BitWriter::new();
        cfg.encode(&channels, &mut w);
        let bytes = w.into_bytes();

        let mut r = BitReader::new(&bytes);
        let decoded = cfg.decode(1, &mut r);
        assert_eq!(decoded[0], channels[0]);
    }

    #[test]
    fn decode_map_round_trips_phrase_words() {
        let cfg = single_stage_config(); // partitions=2, ppw=2
                                         // Entry = t0*2 + t1; decode_map recovers (t0, t1).
        assert_eq!(cfg.decode_map(0), vec![0, 0]);
        assert_eq!(cfg.decode_map(1), vec![0, 1]);
        assert_eq!(cfg.decode_map(2), vec![1, 0]);
        assert_eq!(cfg.decode_map(3), vec![1, 1]);
    }
}
