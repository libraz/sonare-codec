//! Vorbis Huffman codebook entropy coding.
//!
//! Hand-ported to safe Rust from libvorbis/aoTuV `lib/sharedbook.c`
//! (`_make_words`, `ov_ilog`) and `lib/codebook.c` (`vorbis_book_encode`): the
//! canonical codeword construction the Vorbis I spec (§3.2.1) defines from a
//! list of codeword lengths, plus the encode/decode of an entry through the
//! Ogg bit packer. Derivative work of libvorbis/aoTuV (BSD-3-Clause); see
//! `LICENSE-THIRDPARTY`.
//!
//! Codewords are assigned "lowest value first" and then bit-reversed, because
//! the Ogg packer is LSb-first; encoder and decoder both rebuild them from the
//! same length list, so the construction only has to be deterministic and
//! spec-conformant (it is verified against the spec's worked example).

use crate::oggpack::{BitReader, BitWriter};

/// `ov_ilog`: number of significant bits in `v` (`ov_ilog(0) == 0`).
#[must_use]
pub fn ov_ilog(mut v: u32) -> i32 {
    let mut ret = 0;
    while v != 0 {
        ret += 1;
        v >>= 1;
    }
    ret
}

/// Builds the LSb-first codewords for a list of codeword `lengths` (length 0
/// marks an unused entry). Returns `None` if the lengths describe an over- or
/// under-populated tree, as the spec requires.
///
/// This is `_make_words` with `sparsecount == 0`: the result has one slot per
/// input entry, in order.
#[must_use]
pub fn make_words(lengths: &[u8]) -> Option<Vec<u32>> {
    let n = lengths.len();
    let mut marker = [0u32; 33];
    let mut r = vec![0u32; n];

    for (i, &len) in lengths.iter().enumerate() {
        let length = usize::from(len);
        if length == 0 {
            continue;
        }
        let mut entry = marker[length];

        // The lengths must not specify an overpopulated tree.
        if length < 32 && (entry >> length) != 0 {
            return None;
        }
        r[i] = entry;

        // Claim this node: walk the shorter markers, jumping branches where a
        // marker already points along our path.
        let mut j = length;
        while j > 0 {
            if marker[j] & 1 != 0 {
                if j == 1 {
                    marker[1] = marker[1].wrapping_add(1);
                } else {
                    marker[j] = marker[j - 1].wrapping_shl(1);
                }
                break;
            }
            marker[j] = marker[j].wrapping_add(1);
            j -= 1;
        }

        // Prune: re-dangle the longer markers from our newly taken node.
        for j in (length + 1)..33 {
            if (marker[j] >> 1) == entry {
                entry = marker[j];
                marker[j] = marker[j - 1].wrapping_shl(1);
            } else {
                break;
            }
        }
    }

    // Reject any underpopulated tree, shielding the single-entry retcon
    // (one codeword '0' of length 1, which is legally underpopulated).
    let count = lengths.iter().filter(|&&l| l > 0).count();
    if !(count == 1 && marker[2] == 2) {
        for (i, &m) in marker.iter().enumerate().skip(1) {
            if m & (0xffff_ffffu32 >> (32 - i)) != 0 {
                return None;
            }
        }
    }

    // Bit-reverse each codeword for the LSb-first packer.
    let mut out = vec![0u32; n];
    for (i, &len) in lengths.iter().enumerate() {
        let mut temp = 0u32;
        for j in 0..u32::from(len) {
            temp <<= 1;
            temp |= (r[i] >> j) & 1;
        }
        out[i] = temp;
    }
    Some(out)
}

/// Canonical Huffman codeword lengths for a list of symbol `freqs`.
///
/// Symbols with a non-zero frequency get a leaf depth; zero-frequency symbols
/// get length 0 (unused). The result is a complete prefix-code length list
/// ([`make_words`] accepts it) — a single used symbol yields the length-1
/// retcon, and equal frequencies yield a balanced tree. Ties break by symbol
/// index, so the output is deterministic.
#[must_use]
pub fn huffman_lengths(freqs: &[u64]) -> Vec<u8> {
    use std::cmp::Reverse;
    use std::collections::BinaryHeap;

    let n = freqs.len();
    let mut lengths = vec![0u8; n];
    let used: Vec<usize> = (0..n).filter(|&i| freqs[i] > 0).collect();
    if used.is_empty() {
        return lengths;
    }
    if used.len() == 1 {
        lengths[used[0]] = 1; // single-entry retcon: one codeword '0'
        return lengths;
    }

    // Node arena: leaves first, then the internal nodes the merges create.
    // `symbol[node] == usize::MAX` marks an internal node.
    let mut weight: Vec<u64> = Vec::new();
    let mut left: Vec<usize> = Vec::new();
    let mut right: Vec<usize> = Vec::new();
    let mut symbol: Vec<usize> = Vec::new();
    // Min-heap keyed by (weight, node id) so ties break deterministically.
    let mut heap: BinaryHeap<Reverse<(u64, usize)>> = BinaryHeap::new();
    for &s in &used {
        let id = weight.len();
        weight.push(freqs[s]);
        left.push(usize::MAX);
        right.push(usize::MAX);
        symbol.push(s);
        heap.push(Reverse((freqs[s], id)));
    }

    // Repeatedly merge the two lightest nodes.
    while heap.len() > 1 {
        let (Some(Reverse((w1, a))), Some(Reverse((w2, b)))) = (heap.pop(), heap.pop()) else {
            break;
        };
        let id = weight.len();
        weight.push(w1.saturating_add(w2));
        left.push(a);
        right.push(b);
        symbol.push(usize::MAX);
        heap.push(Reverse((w1.saturating_add(w2), id)));
    }

    let Some(Reverse((_, root))) = heap.pop() else {
        return lengths; // unreachable: used.len() >= 2 leaves one root
    };

    // Walk the tree, recording each leaf's depth as its codeword length.
    let mut stack = vec![(root, 0u32)];
    while let Some((node, depth)) = stack.pop() {
        if symbol[node] != usize::MAX {
            lengths[symbol[node]] = depth.clamp(1, 255) as u8;
        } else {
            stack.push((left[node], depth + 1));
            stack.push((right[node], depth + 1));
        }
    }
    lengths
}

/// A Vorbis Huffman codebook: per-entry codeword lengths and the LSb-first
/// codewords derived from them.
pub struct Codebook {
    lengths: Vec<u8>,
    codes: Vec<u32>,
}

impl Codebook {
    /// Builds a codebook from a codeword-length list, or `None` if the lengths
    /// do not form a valid (neither over- nor under-populated) tree.
    #[must_use]
    pub fn new(lengths: Vec<u8>) -> Option<Self> {
        let codes = make_words(&lengths)?;
        Some(Self { lengths, codes })
    }

    /// Number of entries in the book.
    #[must_use]
    pub fn entries(&self) -> usize {
        self.lengths.len()
    }

    /// Codeword length of entry `i` (0 marks an unused entry).
    #[must_use]
    pub fn length(&self, i: usize) -> u8 {
        self.lengths[i]
    }

    /// Encodes entry `a` into `w`, returning the number of bits written.
    /// Mirrors `vorbis_book_encode`: out-of-range entries write nothing.
    pub fn encode(&self, a: usize, w: &mut BitWriter) -> u32 {
        if a >= self.lengths.len() {
            return 0;
        }
        let bits = u32::from(self.lengths[a]);
        w.write(self.codes[a], bits);
        bits
    }

    /// Decodes one entry from `r`, or `None` on an invalid/over-long codeword.
    #[must_use]
    pub fn decode(&self, r: &mut BitReader) -> Option<usize> {
        let mut acc = 0u32;
        for len in 1..=32u32 {
            acc |= r.read(1) << (len - 1);
            for (entry, &l) in self.lengths.iter().enumerate() {
                if u32::from(l) == len && self.codes[entry] == acc {
                    return Some(entry);
                }
            }
        }
        None
    }
}

/// `_book_maptype1_quantvals`: the largest `vals` such that
/// `vals.pow(dim) <= entries < (vals+1).pow(dim)` — the per-scalar value count
/// of a maptype-1 lattice codebook. Computed with the integer refinement loop
/// libvorbis uses (the `pow` estimate is never trusted for bitstream sync).
#[must_use]
pub fn maptype1_quantvals(entries: usize, dim: usize) -> usize {
    if entries < 1 {
        return 0;
    }
    let entries = entries as i64;
    // Integer initial guess; the loop below corrects it either direction.
    let mut vals = (entries as f64).powf(1.0 / dim as f64).floor() as i64;
    if vals < 1 {
        vals = 1;
    }
    loop {
        let mut acc: i64 = 1;
        let mut acc1: i64 = 1;
        let mut i = 0;
        while i < dim {
            if entries / vals < acc {
                break;
            }
            acc *= vals;
            acc1 = if i64::MAX / (vals + 1) < acc1 {
                i64::MAX
            } else {
                acc1 * (vals + 1)
            };
            i += 1;
        }
        if i >= dim && acc <= entries && acc1 > entries {
            return vals as usize;
        } else if i < dim || acc > entries {
            vals -= 1;
        } else {
            vals += 1;
        }
    }
}

/// A Vorbis maptype-1 VQ value codebook: a scalar Huffman `book` whose entries
/// index a lattice of value vectors generated algorithmically from a short
/// `quantlist`. Each entry's dim-vector is the mixed-radix-`quantvals`
/// decomposition of its index, mapped through `quantlist` and the
/// `mindel`/`delta`/`sequence_p` dequantization.
///
/// Hand-ported from libvorbis `sharedbook.c` (`_book_unquantize` maptype 1) and
/// the squared-error search of `res0.c` (`local_book_besterror`). Derivative
/// work of libvorbis/aoTuV (BSD-3-Clause); see `LICENSE-THIRDPARTY`.
pub struct VqBook {
    book: Codebook,
    dim: usize,
    quantvals: usize,
    mindel: f32,
    delta: f32,
    sequence_p: bool,
    quantlist: Vec<f32>,
}

impl VqBook {
    /// Builds a maptype-1 VQ book. `quantlist` holds the `quantvals` per-scalar
    /// quant values; `quantvals` is derived from the book's entry count and
    /// `dim` exactly as libvorbis does.
    #[must_use]
    pub fn new(
        book: Codebook,
        dim: usize,
        mindel: f32,
        delta: f32,
        sequence_p: bool,
        quantlist: Vec<f32>,
    ) -> Self {
        let quantvals = maptype1_quantvals(book.entries(), dim);
        Self {
            book,
            dim,
            quantvals,
            mindel,
            delta,
            sequence_p,
            quantlist,
        }
    }

    /// Vector dimension of each entry.
    #[must_use]
    pub fn dim(&self) -> usize {
        self.dim
    }

    /// The dequantized value vector for `entry`, generated algorithmically by
    /// counting each column through the quant vector (`_book_unquantize`).
    #[must_use]
    pub fn value_vector(&self, entry: usize) -> Vec<f32> {
        let mut out = vec![0.0f32; self.dim];
        let mut last = 0.0f32;
        let mut indexdiv = 1usize;
        for slot in out.iter_mut() {
            let index = (entry / indexdiv) % self.quantvals;
            let val = self.quantlist[index].abs() * self.delta + self.mindel + last;
            if self.sequence_p {
                last = val;
            }
            *slot = val;
            indexdiv *= self.quantvals;
        }
        out
    }

    /// Nearest used entry to `target` by squared error — the always-correct
    /// path of `local_book_besterror`. Returns `None` only for an empty book.
    #[must_use]
    pub fn best_entry(&self, target: &[f32]) -> Option<usize> {
        let mut best: Option<(f32, usize)> = None;
        for e in 0..self.book.entries() {
            if self.book.length(e) == 0 {
                continue;
            }
            let err: f32 = self
                .value_vector(e)
                .iter()
                .zip(target)
                .map(|(a, b)| (a - b) * (a - b))
                .sum();
            if best.is_none_or(|(be, _)| err < be) {
                best = Some((err, e));
            }
        }
        best.map(|(_, e)| e)
    }

    /// Quantizes `target` to its nearest entry and writes that entry's codeword
    /// to `w`, returning the chosen entry (or `None` for an empty book).
    pub fn encode(&self, target: &[f32], w: &mut BitWriter) -> Option<usize> {
        let entry = self.best_entry(target)?;
        self.book.encode(entry, w);
        Some(entry)
    }

    /// Decodes one entry and returns its value vector.
    #[must_use]
    pub fn decode(&self, r: &mut BitReader) -> Option<Vec<f32>> {
        let entry = self.book.decode(r)?;
        Some(self.value_vector(entry))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Reverses the low `len` bits of `code` (undoes the LSb bit-reversal).
    fn reverse_bits(code: u32, len: u32) -> u32 {
        let mut out = 0;
        for j in 0..len {
            out = (out << 1) | ((code >> j) & 1);
        }
        out
    }

    #[test]
    fn ov_ilog_matches_reference() {
        assert_eq!(ov_ilog(0), 0);
        assert_eq!(ov_ilog(1), 1);
        assert_eq!(ov_ilog(7), 3);
        assert_eq!(ov_ilog(8), 4);
    }

    #[test]
    fn make_words_matches_spec_example() {
        // Vorbis I spec §3.2.1 worked example: these lengths produce the
        // MSb-first codewords 00, 0100, 0101, 0110, 0111, 10, 110, 111.
        let lengths = [2u8, 4, 4, 4, 4, 2, 3, 3];
        let expected_msb = [0u32, 4, 5, 6, 7, 2, 6, 7];
        let codes = make_words(&lengths).expect("valid tree");
        for i in 0..lengths.len() {
            assert_eq!(
                reverse_bits(codes[i], u32::from(lengths[i])),
                expected_msb[i],
                "entry {i}"
            );
        }
    }

    #[test]
    fn round_trips_entries_through_packer() {
        let book = Codebook::new(vec![2, 4, 4, 4, 4, 2, 3, 3]).expect("book");
        let seq = [0usize, 5, 7, 1, 2, 6, 3, 4, 0, 5, 5, 7, 6, 1];

        let mut w = BitWriter::new();
        for &e in &seq {
            book.encode(e, &mut w);
        }
        let bytes = w.into_bytes();

        let mut rd = BitReader::new(&bytes);
        for &e in &seq {
            assert_eq!(book.decode(&mut rd), Some(e));
        }
    }

    #[test]
    fn round_trips_unbalanced_book() {
        // Lengths [1,2,3,3] form a valid complete tree.
        let book = Codebook::new(vec![1, 2, 3, 3]).expect("book");
        let seq = [0usize, 0, 1, 3, 2, 1, 0, 3, 2, 0];
        let mut w = BitWriter::new();
        for &e in &seq {
            book.encode(e, &mut w);
        }
        let bytes = w.into_bytes();
        let mut rd = BitReader::new(&bytes);
        for &e in &seq {
            assert_eq!(book.decode(&mut rd), Some(e));
        }
    }

    #[test]
    fn single_entry_book_is_allowed() {
        // The retconned single-entry codebook: one codeword '0' of length 1.
        let book = Codebook::new(vec![1]).expect("single-entry book");
        let mut w = BitWriter::new();
        book.encode(0, &mut w);
        let bytes = w.into_bytes();
        let mut rd = BitReader::new(&bytes);
        assert_eq!(book.decode(&mut rd), Some(0));
    }

    #[test]
    fn huffman_lengths_form_a_valid_tree() {
        // A skewed distribution: make_words must accept the lengths, and the
        // most frequent symbol must get the shortest codeword.
        let freqs = [1000u64, 500, 200, 90, 40, 20, 9, 1];
        let lengths = huffman_lengths(&freqs);
        assert!(make_words(&lengths).is_some(), "lengths form a valid tree");
        let min_len = *lengths.iter().min().expect("non-empty");
        assert_eq!(lengths[0], min_len, "the most frequent symbol is shortest");
        assert!(
            lengths[7] >= lengths[0],
            "the rarest symbol is no shorter than the commonest"
        );
    }

    #[test]
    fn huffman_lengths_handle_degenerate_inputs() {
        // No used symbols -> all length 0.
        assert_eq!(huffman_lengths(&[0, 0, 0]), vec![0, 0, 0]);
        // One used symbol -> the length-1 retcon, accepted by make_words.
        let one = huffman_lengths(&[0, 5, 0]);
        assert_eq!(one, vec![0, 1, 0]);
        assert!(make_words(&one).is_some());
        // Equal frequencies over a power-of-two count -> a balanced tree.
        let balanced = huffman_lengths(&[1, 1, 1, 1]);
        assert_eq!(balanced, vec![2, 2, 2, 2]);
    }

    #[test]
    fn huffman_lengths_round_trip_through_the_codebook() {
        let freqs = [800u64, 300, 120, 60, 25, 12, 6, 3, 1, 1];
        let lengths = huffman_lengths(&freqs);
        let book = Codebook::new(lengths).expect("valid Huffman book");
        let seq = [0usize, 0, 1, 0, 2, 0, 9, 1, 3, 0, 0, 5];
        let mut w = BitWriter::new();
        for &e in &seq {
            book.encode(e, &mut w);
        }
        let bytes = w.into_bytes();
        let mut rd = BitReader::new(&bytes);
        for &e in &seq {
            assert_eq!(book.decode(&mut rd), Some(e));
        }
    }

    #[test]
    fn rejects_overpopulated_tree() {
        // Three length-1 codewords cannot coexist (only two length-1 leaves).
        assert!(make_words(&[1, 1, 1]).is_none());
    }

    #[test]
    fn rejects_underpopulated_tree() {
        // Three length-2 codewords leave one length-2 leaf unclaimed.
        assert!(make_words(&[2, 2, 2]).is_none());
    }

    #[test]
    fn maptype1_quantvals_matches_integer_property() {
        // vals must be the greatest with vals^dim <= entries < (vals+1)^dim.
        for &(entries, dim) in &[
            (8usize, 3usize),
            (256, 4),
            (256, 2),
            (100, 2),
            (9, 2),
            (10, 2),
            (1, 4),
            (1000, 3),
        ] {
            let vals = maptype1_quantvals(entries, dim);
            let lo = (vals as u128).pow(dim as u32);
            let hi = (vals as u128 + 1).pow(dim as u32);
            assert!(
                lo <= entries as u128 && (entries as u128) < hi,
                "entries={entries} dim={dim} vals={vals}"
            );
        }
    }

    #[test]
    fn value_vector_matches_hand_computed_lattice() {
        // dim=2, quantvals=3 (9 lattice points), quantlist [0,1,2], unit scale.
        let book = Codebook::new(vec![1u8, 2, 3, 4, 5, 6, 7, 8, 8]).expect("9-entry book");
        let vq = VqBook::new(book, 2, 0.0, 1.0, false, vec![0.0, 1.0, 2.0]);
        // entry j decomposes to digits (j%3, (j/3)%3) -> [q[d0], q[d1]].
        assert_eq!(vq.value_vector(0), vec![0.0, 0.0]);
        assert_eq!(vq.value_vector(5), vec![2.0, 1.0]); // 5 -> (2,1)
        assert_eq!(vq.value_vector(7), vec![1.0, 2.0]); // 7 -> (1,2)

        // sequence_p accumulates each column onto the previous.
        let book = Codebook::new(vec![1u8, 2, 3, 4, 5, 6, 7, 8, 8]).expect("9-entry book");
        let seq = VqBook::new(book, 2, 0.0, 1.0, true, vec![0.0, 1.0, 2.0]);
        assert_eq!(seq.value_vector(5), vec![2.0, 3.0]); // [2, 2+1]
    }

    #[test]
    fn vq_round_trips_lattice_vectors() {
        // Complete 8-entry book, quantvals=2 (digits {0,1}), dim=3.
        for &seq in &[false, true] {
            let book = Codebook::new(vec![3u8; 8]).expect("complete book");
            let vq = VqBook::new(book, 3, 0.0, 1.0, seq, vec![0.0, 1.0]);
            // Encoding each exact lattice vector recovers its own entry, and the
            // decoded vector is bit-identical.
            let mut w = BitWriter::new();
            let mut entries = Vec::new();
            for e in 0..8 {
                let v = vq.value_vector(e);
                let chosen = vq.encode(&v, &mut w).expect("entry");
                assert_eq!(chosen, e, "seq={seq} entry {e}");
                entries.push(e);
            }
            let bytes = w.into_bytes();
            let mut r = BitReader::new(&bytes);
            for &e in &entries {
                assert_eq!(vq.decode(&mut r), Some(vq.value_vector(e)), "seq={seq}");
            }
        }
    }

    #[test]
    fn vq_best_entry_snaps_noisy_vector_to_nearest() {
        let book = Codebook::new(vec![3u8; 8]).expect("complete book");
        let vq = VqBook::new(book, 3, 0.0, 1.0, false, vec![0.0, 1.0]);
        // A vector near lattice point (1,0,1) = entry 0b101 = 5.
        let near = [0.9f32, 0.1, 1.1];
        assert_eq!(vq.best_entry(&near), Some(5));
    }

    #[test]
    fn skips_unused_entries() {
        // Length-0 entries are unused; the rest must still form a valid tree.
        let lengths = [2u8, 0, 2, 2, 2];
        let codes = make_words(&lengths).expect("valid with gaps");
        assert_eq!(codes[1], 0, "unused entry has no codeword");
        let book = Codebook::new(lengths.to_vec()).expect("book");
        let seq = [0usize, 2, 3, 4, 2, 0, 4, 3];
        let mut w = BitWriter::new();
        for &e in &seq {
            book.encode(e, &mut w);
        }
        let bytes = w.into_bytes();
        let mut rd = BitReader::new(&bytes);
        for &e in &seq {
            assert_eq!(book.decode(&mut rd), Some(e));
        }
    }
}
