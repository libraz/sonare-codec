use super::*;

#[cfg(test)]
#[allow(clippy::module_inception)]
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

    /// The standard "128 x 4" floor1 postlist from libvorbis `floor_all.h`.
    const POSTLIST_128X4: [i32; 6] = [0, 128, 33, 8, 16, 70];

    /// Render the full floor curve (post-height domain) from the post heights by
    /// drawing lines between adjacent sorted posts, as floor1 decode does.
    fn render_floor(postlist: &[i32], heights: &[i32], n: usize) -> Vec<i32> {
        let posts = postlist.len();
        let mut order: Vec<usize> = (0..posts).collect();
        order.sort_by_key(|&i| postlist[i]);
        let mut out = vec![0i32; n];
        let mut lx = postlist[order[0]];
        let mut ly = heights[order[0]] & 0x7fff;
        for &cur in &order[1..] {
            let hx = postlist[cur];
            let hy = heights[cur] & 0x7fff;
            render_line0(n as i32, lx, hx, ly, hy, &mut out);
            lx = hx;
            ly = hy;
        }
        out
    }

    /// RMS error between a rendered floor and the quantized mask over `n` bins.
    fn floor_rms_error(floor: &[i32], logmask: &[f32]) -> f32 {
        let n = floor.len();
        let sse: f32 = (0..n)
            .map(|i| {
                let d = (floor[i] - vorbis_db_quant(logmask[i])) as f32;
                d * d
            })
            .sum();
        (sse / n as f32).sqrt()
    }

    /// A sloped masking curve falling from `-10` dB to `-90` dB across `n` bins,
    /// and an MDCT energy equal to it (so every bin is "audible" to the fit).
    fn sloped_mask(n: usize) -> (Vec<f32>, Vec<f32>) {
        let mask: Vec<f32> = (0..n)
            .map(|i| -10.0 - (i as f32 / n as f32) * 80.0)
            .collect();
        let mdct = mask.clone();
        (mdct, mask)
    }

    #[test]
    fn fit_tracks_a_sloped_masking_curve() {
        let n = 128;
        let (mdct, mask) = sloped_mask(n);
        let fitter = Floor1Fitter::new(POSTLIST_128X4.to_vec(), Floor1FitInfo::standard());
        let posts = fitter.fit(&mdct, &mask).expect("nonzero floor");

        let floor = render_floor(&POSTLIST_128X4, &posts, n);
        let rms = floor_rms_error(&floor, &mask);
        // The fit error stays well inside one floor1 "max_err" segment budget.
        assert!(rms < 60.0, "fitted floor strays from the mask: rms {rms}");
        // The floor must slope down with the mask (low-freq louder than high).
        assert!(
            (floor[4] & 0x7fff) > (floor[120] & 0x7fff),
            "floor should fall with the sloped mask"
        );
    }

    #[test]
    fn fit_is_flat_for_a_flat_mask() {
        let n = 128;
        let mask = vec![-40.0f32; n];
        let mdct = mask.clone();
        let fitter = Floor1Fitter::new(POSTLIST_128X4.to_vec(), Floor1FitInfo::standard());
        let posts = fitter.fit(&mdct, &mask).expect("nonzero floor");
        let floor = render_floor(&POSTLIST_128X4, &posts, n);
        let target = vorbis_db_quant(-40.0);
        for (i, &f) in floor.iter().enumerate() {
            assert!(
                ((f & 0x7fff) - target).abs() <= 2,
                "bin {i}: flat floor {} vs target {target}",
                f & 0x7fff
            );
        }
    }

    #[test]
    fn fit_declines_a_silent_spectrum() {
        let n = 128;
        // Below the dB-quant floor everywhere -> every post quantizes to zero.
        let mask = vec![-200.0f32; n];
        let mdct = mask.clone();
        let fitter = Floor1Fitter::new(POSTLIST_128X4.to_vec(), Floor1FitInfo::standard());
        assert!(fitter.fit(&mdct, &mask).is_none(), "silent floor is unused");
    }

    #[test]
    fn quantize_posts_to_mult_shifts_and_keeps_flag() {
        // mult 4 -> >>4; the 0x8000 declined flag survives.
        let posts = [1000, 64, 0x8000 | 800, 16];
        let q = quantize_posts_to_mult(&posts, 4);
        assert_eq!(q[0], 1000 >> 4);
        assert_eq!(q[1], 64 >> 4);
        assert_eq!(q[2], (800 >> 4) | 0x8000);
        assert_eq!(q[3], 16 >> 4);
        // mult 3 divides by 12.
        assert_eq!(quantize_posts_to_mult(&[120], 3)[0], 10);
    }

    #[test]
    fn full_floor_chain_round_trips_through_the_deviation_layer() {
        // fit -> quantize -> deviation-encode -> deviation-decode -> render, and
        // the decoded floor must still approximate the mask. Exercises the whole
        // encode-side floor1 chain end to end (entropy layer is lossless).
        let n = 128;
        let (mdct, mask) = sloped_mask(n);
        let fitter = Floor1Fitter::new(POSTLIST_128X4.to_vec(), Floor1FitInfo::standard());
        let fit = fitter.fit(&mdct, &mask).expect("nonzero floor");

        let quant_q = 64; // mult 4
        let q = quantize_posts_to_mult(&fit, 4);
        let (lo, hi) = low_high_neighbors(&POSTLIST_128X4);
        let mut post = q.clone();
        let out = encode_post_deviations(&POSTLIST_128X4, &mut post, &lo, &hi, quant_q);
        let decoded = decode_post_deviations(&POSTLIST_128X4, &out, &lo, &hi, quant_q);

        // Dequantize back to the post-height domain (mult 4 -> *16) and render.
        let dequant: Vec<i32> = decoded.iter().map(|&p| (p & 0x7fff) << 4).collect();
        let floor = render_floor(&POSTLIST_128X4, &dequant, n);
        let rms = floor_rms_error(&floor, &mask);
        assert!(
            rms < 80.0,
            "round-tripped floor strays from the mask: rms {rms}"
        );
    }
}
