use super::*;

/// Quantize floor1 post heights from the `0..=1023` dB-quant domain down to the
/// multiplier range (`mult` 1..=4 → 256/128/86/64 levels), preserving the
/// `0x8000` declined flag. Mirrors the quantize prologue of libvorbis
/// `floor1_encode`; feed the result to [`encode_post_deviations`].
#[must_use]
pub fn quantize_posts_to_mult(post: &[i32], mult: i32) -> Vec<i32> {
    post.iter()
        .map(|&p| {
            let val = p & 0x7fff;
            let q = match mult {
                1 => val >> 2,
                2 => val >> 3,
                3 => val / 12,
                _ => val >> 4,
            };
            q | (p & 0x8000)
        })
        .collect()
}

/// One floor1 partition class: how many posts it spans (`dim`), how many
/// cascade subclass bits select among its `subbook`s, the phrase `book` that
/// codes the cascade value (used only when `subs > 0`), and the per-subclass
/// `subbook` indices (`-1` meaning "this subclass codes the literal value 0").
pub struct Floor1Class {
    pub dim: usize,
    pub subs: u32,
    pub book: usize,
    pub subbook: Vec<i32>,
}

/// The floor1 codebook configuration needed to entropy-code the per-post
/// deviation values: the post-value quantization ceiling `quant_q`, the class
/// assigned to each partition, the class table, and the codebook pool the
/// class/subbook indices point into.
///
/// Hand-ported from the cascade pack/unpack of libvorbis `floor1_encode` /
/// `floor1_inverse1`. Derivative work of libvorbis/aoTuV (BSD-3-Clause); see
/// `LICENSE-THIRDPARTY`.
pub struct Floor1Encoding {
    pub quant_q: i32,
    pub partition_class: Vec<usize>,
    pub classes: Vec<Floor1Class>,
    pub books: Vec<Codebook>,
}

impl Floor1Encoding {
    /// Number of floor posts this configuration codes: the two endpoints plus
    /// the dimension of every partition's class.
    #[must_use]
    pub fn posts(&self) -> usize {
        2 + self
            .partition_class
            .iter()
            .map(|&c| self.classes[c].dim)
            .sum::<usize>()
    }

    /// Bit width of an endpoint post value: `ilog(quant_q - 1)`.
    pub(crate) fn endpoint_bits(&self) -> u32 {
        ov_ilog((self.quant_q - 1) as u32) as u32
    }

    /// Packs the per-post deviation values `out` (from [`encode_post_deviations`],
    /// length [`posts`](Self::posts)) into `w`: the nontrivial-floor flag, the
    /// two endpoint posts, then each partition's cascade phrase plus subbook
    /// values. Mirrors the pack stage of libvorbis `floor1_encode`.
    pub fn pack(&self, out: &[i32], w: &mut BitWriter) {
        let bits = self.endpoint_bits();
        // Mark a nontrivial (present) floor.
        w.write(1, 1);
        w.write(out[0] as u32, bits);
        w.write(out[1] as u32, bits);

        let mut j = 2usize;
        for &class_idx in &self.partition_class {
            let class = &self.classes[class_idx];
            let cdim = class.dim;
            let csubbits = class.subs;
            let csub = 1usize << csubbits;
            let mut bookas = [0usize; 8];

            // First-stage cascade value: choose, per post, the lowest subclass
            // whose book can represent the value, and pack those choices.
            if csubbits > 0 {
                let mut maxval = [0i32; 8];
                for (k, slot) in maxval.iter_mut().enumerate().take(csub) {
                    let booknum = class.subbook[k];
                    *slot = if booknum < 0 {
                        1
                    } else {
                        self.books[booknum as usize].entries() as i32
                    };
                }
                let mut cval = 0u32;
                let mut cshift = 0u32;
                for k in 0..cdim {
                    let val = out[j + k];
                    for (l, &mv) in maxval.iter().enumerate().take(csub) {
                        if val < mv {
                            bookas[k] = l;
                            break;
                        }
                    }
                    cval |= (bookas[k] as u32) << cshift;
                    cshift += csubbits;
                }
                self.books[class.book].encode(cval as usize, w);
            }

            // Second stage: code each post value through its chosen subbook.
            for (k, &b) in bookas.iter().enumerate().take(cdim) {
                let book = class.subbook[b];
                if book >= 0 {
                    let cb = &self.books[book as usize];
                    // Guard out-of-range values exactly as the C does.
                    if (out[j + k] as usize) < cb.entries() {
                        cb.encode(out[j + k] as usize, w);
                    }
                }
            }
            j += cdim;
        }
    }

    /// Histograms the codebook entries [`pack`](Self::pack) would code for the
    /// deviation values `out`, into `counts` keyed by global book index
    /// (`counts[book][entry] += 1`). This mirrors `pack`'s cascade classification
    /// exactly but counts instead of writing, so the per-book entry distribution
    /// can drive an adaptive Huffman fit. The classification depends only on each
    /// book's *entry count* (not its codeword lengths), so histogramming with the
    /// construction-time books and then coding with length-fitted books of the
    /// same size produces a bit-identical stream — only the codeword lengths, and
    /// thus the size, change.
    pub fn histogram(&self, out: &[i32], counts: &mut [Vec<u64>]) {
        let mut j = 2usize;
        for &class_idx in &self.partition_class {
            let class = &self.classes[class_idx];
            let cdim = class.dim;
            let csubbits = class.subs;
            let csub = 1usize << csubbits;
            let mut bookas = [0usize; 8];

            if csubbits > 0 {
                let mut maxval = [0i32; 8];
                for (k, slot) in maxval.iter_mut().enumerate().take(csub) {
                    let booknum = class.subbook[k];
                    *slot = if booknum < 0 {
                        1
                    } else {
                        self.books[booknum as usize].entries() as i32
                    };
                }
                let mut cval = 0u32;
                let mut cshift = 0u32;
                for k in 0..cdim {
                    let val = out[j + k];
                    for (l, &mv) in maxval.iter().enumerate().take(csub) {
                        if val < mv {
                            bookas[k] = l;
                            break;
                        }
                    }
                    cval |= (bookas[k] as u32) << cshift;
                    cshift += csubbits;
                }
                if let Some(book) = counts.get_mut(class.book) {
                    if let Some(slot) = book.get_mut(cval as usize) {
                        *slot += 1;
                    }
                }
            }

            for (k, &b) in bookas.iter().enumerate().take(cdim) {
                let book = class.subbook[b];
                if book >= 0 {
                    let cb = &self.books[book as usize];
                    let entry = out[j + k] as usize;
                    if entry < cb.entries() {
                        if let Some(hist) = counts.get_mut(book as usize) {
                            if let Some(slot) = hist.get_mut(entry) {
                                *slot += 1;
                            }
                        }
                    }
                }
            }
            j += cdim;
        }
    }

    /// Unpacks the deviation values written by [`pack`](Self::pack) from `r`.
    /// Returns `None` when the nontrivial-floor flag is clear (floor unused) or
    /// the stream ends mid-codeword. Mirrors the unpack stage of libvorbis
    /// `floor1_inverse1`; feed the result to [`decode_post_deviations`].
    #[must_use]
    pub fn unpack(&self, r: &mut BitReader) -> Option<Vec<i32>> {
        if r.read(1) != 1 {
            return None;
        }
        let bits = self.endpoint_bits();
        let posts = self.posts();
        let mut out = vec![0i32; posts];
        out[0] = r.read(bits) as i32;
        out[1] = r.read(bits) as i32;

        let mut j = 2usize;
        for &class_idx in &self.partition_class {
            let class = &self.classes[class_idx];
            let cdim = class.dim;
            let csubbits = class.subs;
            let csub = 1u32 << csubbits;

            let mut cval = 0u32;
            if csubbits > 0 {
                cval = self.books[class.book].decode(r)? as u32;
            }
            for k in 0..cdim {
                let book = class.subbook[(cval & (csub - 1)) as usize];
                cval >>= csubbits;
                out[j + k] = if book >= 0 {
                    self.books[book as usize].decode(r)? as i32
                } else {
                    0
                };
            }
            j += cdim;
        }
        Some(out)
    }
}
