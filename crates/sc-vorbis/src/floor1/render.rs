use super::*;

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
