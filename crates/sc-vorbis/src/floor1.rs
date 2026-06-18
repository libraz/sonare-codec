//! Vorbis floor1 curve primitives.
//!
//! Hand-ported to safe Rust from libvorbis/aoTuV `lib/floor1.c`: the integer
//! line rasterizer (`render_point` / `render_line0`) that turns floor posts into
//! a per-bin floor curve, and the dB quantizer (`vorbis_dBquant`) used when
//! fitting that curve. Derivative work of libvorbis/aoTuV (BSD-3-Clause); see
//! `LICENSE-THIRDPARTY`.

// Consumed by the floor1 fit/encode stage; the live encoder still ships via FFI.
#![allow(dead_code)]

use crate::codebook::{ov_ilog, Codebook};
use crate::oggpack::{BitReader, BitWriter};

/// Interpolates the integer floor value of the line `(x0,y0)-(x1,y1)` at `x`.
///
/// The high bit of `y0`/`y1` is a post "used" flag in libvorbis and is masked
/// off here exactly as in the C.
#[must_use]
pub fn render_point(x0: i32, x1: i32, y0: i32, y1: i32, x: i32) -> i32 {
    let y0 = y0 & 0x7fff;
    let y1 = y1 & 0x7fff;
    let dy = y1 - y0;
    let adx = x1 - x0;
    let ady = dy.abs();
    let err = ady * (x - x0);
    let off = err / adx;
    if dy < 0 {
        y0 - off
    } else {
        y0 + off
    }
}

/// Rasterizes the line `(x0,y0)-(x1,y1)` into `d[x0..min(n, x1)]` as integers.
///
/// This is the integer DDA libvorbis uses to build the log-domain floor mask.
pub fn render_line0(n: i32, x0: i32, x1: i32, y0: i32, y1: i32, d: &mut [i32]) {
    let dy = y1 - y0;
    let adx = x1 - x0;
    let mut ady = dy.abs();
    let base = dy / adx;
    let sy = if dy < 0 { base - 1 } else { base + 1 };
    let mut x = x0;
    let mut y = y0;
    let mut err = 0;

    ady -= (base * adx).abs();

    let n = n.min(x1);

    if x < n {
        d[x as usize] = y;
    }
    x += 1;
    while x < n {
        err += ady;
        if err >= adx {
            err -= adx;
            y += sy;
        } else {
            y += base;
        }
        d[x as usize] = y;
        x += 1;
    }
}

/// Quantizes a linear floor magnitude to libvorbis's dB index in `0..=1023`.
#[must_use]
pub fn vorbis_db_quant(x: f32) -> i32 {
    let i = (x * 7.3142857 + 1023.5) as i32;
    i.clamp(0, 1023)
}

/// Computes the static low/high interpolation neighbors for every floor post
/// past the two endpoints, from the post x-positions `postlist`.
///
/// Ported from the neighbor-discovery loop in libvorbis `floor1_look`. Returns
/// `(loneighbor, hineighbor)`, each indexed by `post_index - 2`.
#[must_use]
pub fn low_high_neighbors(postlist: &[i32]) -> (Vec<usize>, Vec<usize>) {
    let n = postlist.len();
    let look_n = postlist[1]; // the rightmost post position
    let count = n.saturating_sub(2);
    let mut lo_n = vec![0usize; count];
    let mut hi_n = vec![0usize; count];
    for i in 0..count {
        let mut lo = 0usize;
        let mut hi = 1usize;
        let mut lx = 0i32;
        let mut hx = look_n;
        let currentx = postlist[i + 2];
        for (j, &x) in postlist.iter().enumerate().take(i + 2) {
            if x > lx && x < currentx {
                lo = j;
                lx = x;
            }
            if x < hx && x > currentx {
                hi = j;
                hx = x;
            }
        }
        lo_n[i] = lo;
        hi_n[i] = hi;
    }
    (lo_n, hi_n)
}

/// Encodes the floor1 post heights `post` (quantized to `quant_q`) into the
/// per-post deviation values written to the bitstream.
///
/// Ported from the prediction/wrap loop of libvorbis `floor1_encode`. `post` is
/// modified in place exactly as the C does (declined posts gain the `0x8000`
/// flag; used neighbors are masked back to `0x7fff`). The codeword entropy
/// coding of the returned values is a separate layer.
#[must_use]
pub fn encode_post_deviations(
    postlist: &[i32],
    post: &mut [i32],
    loneighbor: &[usize],
    hineighbor: &[usize],
    quant_q: i32,
) -> Vec<i32> {
    let posts = postlist.len();
    let mut out = vec![0i32; posts];
    out[0] = post[0];
    out[1] = post[1];

    for i in 2..posts {
        let ln = loneighbor[i - 2];
        let hn = hineighbor[i - 2];
        let predicted = render_point(postlist[ln], postlist[hn], post[ln], post[hn], postlist[i]);

        if (post[i] & 0x8000) != 0 || predicted == post[i] {
            // Roundoff jitter in interpolation, or an explicitly declined post.
            post[i] = predicted | 0x8000;
            out[i] = 0;
        } else {
            let headroom = (quant_q - predicted).min(predicted);
            let mut val = post[i] - predicted;
            // Wrap the signed deviation into [0, range) while preserving the
            // (roughly Gaussian) probability ordering.
            if val < 0 {
                if val < -headroom {
                    val = headroom - val - 1;
                } else {
                    val = -1 - (val << 1);
                }
            } else if val >= headroom {
                val += headroom;
            } else {
                val <<= 1;
            }
            out[i] = val;
            post[ln] &= 0x7fff;
            post[hn] &= 0x7fff;
        }
    }
    out
}

/// Reconstructs floor1 post heights from the per-post deviation values `out`.
///
/// Ported from the unwrap loop of libvorbis `floor1_inverse1`; the inverse of
/// [`encode_post_deviations`]. Returns the reconstructed posts (declined posts
/// carry the `0x8000` flag, as in the decoder).
#[must_use]
pub fn decode_post_deviations(
    postlist: &[i32],
    out: &[i32],
    loneighbor: &[usize],
    hineighbor: &[usize],
    quant_q: i32,
) -> Vec<i32> {
    let posts = postlist.len();
    let mut fit = vec![0i32; posts];
    fit[0] = out[0];
    fit[1] = out[1];

    for i in 2..posts {
        let ln = loneighbor[i - 2];
        let hn = hineighbor[i - 2];
        let predicted = render_point(postlist[ln], postlist[hn], fit[ln], fit[hn], postlist[i]);
        let hiroom = quant_q - predicted;
        let loroom = predicted;
        let room = hiroom.min(loroom) << 1;
        let mut val = out[i];

        if val != 0 {
            if val >= room {
                if hiroom > loroom {
                    val -= loroom;
                } else {
                    val = -1 - (val - hiroom);
                }
            } else if val & 1 != 0 {
                val = -((val + 1) >> 1);
            } else {
                val >>= 1;
            }
            fit[i] = (val + predicted) & 0x7fff;
            fit[ln] &= 0x7fff;
            fit[hn] &= 0x7fff;
        } else {
            fit[i] = predicted | 0x8000;
        }
    }
    fit
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
    fn endpoint_bits(&self) -> u32 {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_point_hits_endpoints() {
        assert_eq!(render_point(0, 128, 10, 200, 0), 10);
        assert_eq!(render_point(0, 128, 10, 200, 128), 200);
        // Decreasing line.
        assert_eq!(render_point(5, 50, 300, 100, 5), 300);
        assert_eq!(render_point(5, 50, 300, 100, 50), 100);
    }

    #[test]
    fn render_point_midpoint() {
        // Halfway along a 0..100 / 0..200 line is ~100.
        assert_eq!(render_point(0, 100, 0, 200, 50), 100);
    }

    #[test]
    fn render_line0_matches_render_point_within_one() {
        let cases = [
            (0, 128, 10, 200),
            (0, 100, 200, 5),
            (3, 64, 0, 63),
            (0, 200, 512, 1),
        ];
        for &(x0, x1, y0, y1) in &cases {
            let mut d = vec![-9999; (x1 + 1) as usize];
            render_line0(x1 + 1, x0, x1, y0, y1, &mut d);
            assert_eq!(d[x0 as usize], y0 & 0x7fff, "start value");
            for x in x0..x1 {
                let direct = render_point(x0, x1, y0, y1, x);
                assert!(
                    (d[x as usize] - direct).abs() <= 1,
                    "x={x}: dda={} direct={direct}",
                    d[x as usize]
                );
            }
        }
    }

    #[test]
    fn render_line0_respects_n_cap() {
        let mut d = vec![-1; 200];
        // n caps the fill below x1.
        render_line0(50, 0, 128, 10, 200, &mut d);
        assert_eq!(d[0], 10, "start value");
        assert_ne!(d[49], -1, "within n must be filled");
        assert_eq!(d[50], -1, "beyond n must be untouched");
        assert_eq!(d[127], -1, "beyond n must be untouched");
    }

    #[test]
    fn db_quant_clamps() {
        assert_eq!(vorbis_db_quant(0.0), 1023);
        assert_eq!(vorbis_db_quant(-1000.0), 0);
        assert_eq!(vorbis_db_quant(1000.0), 1023);
        // Mid-range monotonic.
        assert!(vorbis_db_quant(-50.0) < vorbis_db_quant(-40.0));
    }

    #[test]
    fn neighbors_match_spec_layout() {
        // A typical floor1 postlist: endpoints 0,n then bisected positions.
        let postlist = [0, 128, 64, 32, 96, 16, 48, 80, 112];
        let (lo, hi) = low_high_neighbors(&postlist);
        // Post 2 (x=64): nearest below is 0 (idx0), above is 128 (idx1).
        assert_eq!((lo[0], hi[0]), (0, 1));
        // Post 3 (x=32): below 0(idx0), above 64(idx2).
        assert_eq!((lo[1], hi[1]), (0, 2));
        // Post 4 (x=96): below 64(idx2), above 128(idx1).
        assert_eq!((lo[2], hi[2]), (2, 1));
        // Post 6 (x=48): below 32(idx3), above 64(idx2).
        assert_eq!((lo[4], hi[4]), (3, 2));
    }

    fn deviation_roundtrip(postlist: &[i32], heights: &[i32], quant_q: i32) {
        let (lo, hi) = low_high_neighbors(postlist);
        let mut post = heights.to_vec();
        let out = encode_post_deviations(postlist, &mut post, &lo, &hi, quant_q);
        let fit = decode_post_deviations(postlist, &out, &lo, &hi, quant_q);
        for i in 0..postlist.len() {
            assert_eq!(
                fit[i] & 0x7fff,
                heights[i] & 0x7fff,
                "post {i}: fit={} orig={}",
                fit[i] & 0x7fff,
                heights[i] & 0x7fff,
            );
        }
    }

    #[test]
    fn post_deviations_round_trip() {
        let postlist = [0, 128, 64, 32, 96, 16, 48, 80, 112];
        // Heights within [0, quant_q); the predictor handles the interpolation.
        deviation_roundtrip(&postlist, &[120, 110, 115, 100, 130, 90, 105, 125, 95], 256);
        deviation_roundtrip(&postlist, &[10, 240, 200, 30, 150, 5, 80, 180, 220], 256);
        // mult=4 -> quant_q=64.
        deviation_roundtrip(&postlist, &[40, 20, 30, 50, 10, 55, 35, 15, 45], 64);
        // Flat floor: every post predicted exactly (out[i]==0 path).
        deviation_roundtrip(&postlist, &[100; 9], 256);
    }

    /// Complete uniform Huffman book: `1 << len` entries, every codeword `len`
    /// bits, which `make_words` accepts as a fully populated tree.
    fn complete_book(len: u8) -> Codebook {
        let entries = 1usize << len;
        Codebook::new(vec![len; entries]).expect("complete book")
    }

    /// A two-partition floor1 config over the 9-post layout used above, with
    /// `quant_q = 64`. Each class has one cascade subclass bit: subclass 0 codes
    /// the literal 0 (subbook -1), subclass 1 a full 64-entry value book.
    fn cascade_encoding() -> Floor1Encoding {
        Floor1Encoding {
            quant_q: 64,
            // 7 non-endpoint posts split as 3 + 4.
            partition_class: vec![0, 1],
            classes: vec![
                Floor1Class {
                    dim: 3,
                    subs: 1,
                    book: 1, // 8-entry phrase book covers cval in 0..2^3.
                    subbook: vec![-1, 0],
                },
                Floor1Class {
                    dim: 4,
                    subs: 1,
                    book: 2, // 16-entry phrase book covers cval in 0..2^4.
                    subbook: vec![-1, 0],
                },
            ],
            // books[0]=value book (64), books[1]=phrase(8), books[2]=phrase(16).
            books: vec![complete_book(6), complete_book(3), complete_book(4)],
        }
    }

    fn cascade_roundtrip(postlist: &[i32], heights: &[i32]) {
        let enc = cascade_encoding();
        let (lo, hi) = low_high_neighbors(postlist);
        let mut post = heights.to_vec();
        let out = encode_post_deviations(postlist, &mut post, &lo, &hi, enc.quant_q);

        let mut w = BitWriter::new();
        enc.pack(&out, &mut w);
        let bytes = w.into_bytes();

        let mut r = BitReader::new(&bytes);
        let decoded_out = enc.unpack(&mut r).expect("present floor");
        // The cascade layer must reproduce the deviation values bit-exactly...
        assert_eq!(decoded_out, out, "cascade deviations");
        // ...and the full pipeline must reconstruct the post heights.
        let fit = decode_post_deviations(postlist, &decoded_out, &lo, &hi, enc.quant_q);
        for i in 0..postlist.len() {
            assert_eq!(
                fit[i] & 0x7fff,
                heights[i] & 0x7fff,
                "post {i}: fit={} orig={}",
                fit[i] & 0x7fff,
                heights[i] & 0x7fff,
            );
        }
    }

    #[test]
    fn cascade_pack_unpack_round_trips() {
        let postlist = [0, 128, 64, 32, 96, 16, 48, 80, 112];
        cascade_roundtrip(&postlist, &[40, 20, 30, 50, 10, 55, 35, 15, 45]);
        cascade_roundtrip(&postlist, &[5, 60, 33, 12, 48, 3, 27, 52, 40]);
        // Flat floor exercises the subbook -1 (literal zero) path on every post.
        cascade_roundtrip(&postlist, &[32; 9]);
    }

    #[test]
    fn unpack_rejects_absent_floor() {
        // A lone 0 flag marks an unused floor for this frame.
        let mut w = BitWriter::new();
        w.write(0, 1);
        let bytes = w.into_bytes();
        let enc = cascade_encoding();
        let mut r = BitReader::new(&bytes);
        assert!(enc.unpack(&mut r).is_none());
    }

    #[test]
    fn posts_counts_endpoints_plus_partitions() {
        // 2 endpoints + class dims 3 + 4.
        assert_eq!(cascade_encoding().posts(), 9);
    }
}
