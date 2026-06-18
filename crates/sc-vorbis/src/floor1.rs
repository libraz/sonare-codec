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

/// Least-squares line-fit accumulators for one minimal post division. Mirrors
/// libvorbis `lsfit_acc`: each spectral bin's quantized floor value is summed
/// into an `a` bucket (bins at/above the MDCT energy, weighted up in the fit) or
/// a `b` bucket (bins masked below it), so the line is pulled toward the audible
/// part of the spectrum.
#[derive(Clone, Copy, Default)]
struct LsfitAcc {
    x0: i32,
    x1: i32,
    xa: i32,
    ya: i32,
    x2a: i32,
    y2a: i32,
    xya: i32,
    an: i32,
    xb: i32,
    yb: i32,
    x2b: i32,
    y2b: i32,
    xyb: i32,
    bn: i32,
}

/// The floor1 encode-side fit tuning parameters (libvorbis `vorbis_info_floor1`
/// analysis fields). The standard library values across all sample rates are
/// `max_over = 60`, `max_under = 30`, `max_err = 500`, `two_fit_weight = 1.0`,
/// `two_fit_atten = 18.0`.
pub struct Floor1FitInfo {
    /// Per-bin overshoot the floor may have above the masking curve before the
    /// fit must split the segment (dB-quant units).
    pub max_over: f32,
    /// Per-bin undershoot allowed below the masking curve before a split.
    pub max_under: f32,
    /// Mean-squared-error ceiling that also forces a split.
    pub max_err: f32,
    /// How strongly audible (above-MDCT) bins are weighted in the line fit.
    pub two_fit_weight: f32,
    /// dB margin by which a bin counts as audible (`mdct + atten >= mask`).
    pub two_fit_atten: f32,
}

impl Floor1FitInfo {
    /// The standard libvorbis analysis parameters shared by every built-in
    /// floor1 setup.
    #[must_use]
    pub fn standard() -> Self {
        Self {
            max_over: 60.0,
            max_under: 30.0,
            max_err: 500.0,
            two_fit_weight: 1.0,
            two_fit_atten: 18.0,
        }
    }
}

/// Round half-to-even, matching C `rint` under the default rounding mode.
fn rint(x: f64) -> f64 {
    x.round_ties_even()
}

/// Combine the two candidate fit values for a post (libvorbis `post_Y`): use
/// whichever side is defined, or their midpoint when both are.
fn post_y(a: &[i32], b: &[i32], pos: usize) -> i32 {
    if a[pos] < 0 {
        b[pos]
    } else if b[pos] < 0 {
        a[pos]
    } else {
        (a[pos] + b[pos]) >> 1
    }
}

/// Accumulate the least-squares sums for the bins in `[x0, x1]` (libvorbis
/// `accumulate_fit`). Returns the number of audible (`a`-bucket) bins.
fn accumulate_fit(
    logmask: &[f32],
    logmdct: &[f32],
    x0: i32,
    mut x1: i32,
    n: i32,
    info: &Floor1FitInfo,
) -> LsfitAcc {
    let mut acc = LsfitAcc {
        x0,
        x1,
        ..LsfitAcc::default()
    };
    if x1 >= n {
        x1 = n - 1;
    }
    for i in x0..=x1 {
        let idx = i as usize;
        let quantized = vorbis_db_quant(logmask[idx]);
        if quantized != 0 {
            if logmdct[idx] + info.two_fit_atten >= logmask[idx] {
                acc.xa += i;
                acc.ya += quantized;
                acc.x2a += i * i;
                acc.y2a += quantized * quantized;
                acc.xya += i * quantized;
                acc.an += 1;
            } else {
                acc.xb += i;
                acc.yb += quantized;
                acc.x2b += i * i;
                acc.y2b += quantized * quantized;
                acc.xyb += i * quantized;
                acc.bn += 1;
            }
        }
    }
    acc
}

/// Least-squares fit a line across the accumulators `a` (libvorbis `fit_line`),
/// optionally pinned to existing endpoint values `y0`/`y1` (`< 0` means free).
/// Writes the fitted endpoints back into `y0`/`y1` and returns `true` when the
/// fit is degenerate (a flat zero line).
fn fit_line(a: &[LsfitAcc], y0: &mut i32, y1: &mut i32, info: &Floor1FitInfo) -> bool {
    let (mut xb, mut yb, mut x2b, mut y2b, mut xyb, mut bn) = (0f64, 0f64, 0f64, 0f64, 0f64, 0f64);
    let x0 = a[0].x0;
    let x1 = a[a.len() - 1].x1;

    for acc in a {
        let weight =
            (acc.bn + acc.an) as f64 * f64::from(info.two_fit_weight) / (acc.an as f64 + 1.0) + 1.0;
        xb += acc.xb as f64 + acc.xa as f64 * weight;
        yb += acc.yb as f64 + acc.ya as f64 * weight;
        x2b += acc.x2b as f64 + acc.x2a as f64 * weight;
        y2b += acc.y2b as f64 + acc.y2a as f64 * weight;
        xyb += acc.xyb as f64 + acc.xya as f64 * weight;
        bn += acc.bn as f64 + acc.an as f64 * weight;
    }
    let _ = y2b;

    if *y0 >= 0 {
        xb += f64::from(x0);
        yb += f64::from(*y0);
        x2b += f64::from(x0) * f64::from(x0);
        xyb += f64::from(*y0) * f64::from(x0);
        bn += 1.0;
    }
    if *y1 >= 0 {
        xb += f64::from(x1);
        yb += f64::from(*y1);
        x2b += f64::from(x1) * f64::from(x1);
        xyb += f64::from(*y1) * f64::from(x1);
        bn += 1.0;
    }

    let denom = bn * x2b - xb * xb;
    if denom > 0.0 {
        let a_coef = (yb * x2b - xyb * xb) / denom;
        let b_coef = (bn * xyb - xb * yb) / denom;
        *y0 = (rint(a_coef + b_coef * f64::from(x0)) as i32).clamp(0, 1023);
        *y1 = (rint(a_coef + b_coef * f64::from(x1)) as i32).clamp(0, 1023);
        false
    } else {
        *y0 = 0;
        *y1 = 0;
        true
    }
}

/// Walk the integer floor line `(x0,y0)-(x1,y1)` and decide whether its error
/// against the masking curve exceeds the fit bounds (libvorbis `inspect_error`).
/// Returns `true` when the segment must be split.
fn inspect_error(
    x0: i32,
    x1: i32,
    y0: i32,
    y1: i32,
    logmask: &[f32],
    logmdct: &[f32],
    info: &Floor1FitInfo,
) -> bool {
    let dy = y1 - y0;
    let adx = x1 - x0;
    let mut ady = dy.abs();
    let base = dy / adx;
    let sy = if dy < 0 { base - 1 } else { base + 1 };
    let mut x = x0;
    let mut y = y0;
    let mut err = 0;
    ady -= (base * adx).abs();

    let mut val = vorbis_db_quant(logmask[x as usize]);
    let mut mse = (y - val) * (y - val);
    let mut n = 1;
    if logmdct[x as usize] + info.two_fit_atten >= logmask[x as usize] {
        if (y as f32 + info.max_over) < val as f32 {
            return true;
        }
        if (y as f32 - info.max_under) > val as f32 {
            return true;
        }
    }

    x += 1;
    while x < x1 {
        err += ady;
        if err >= adx {
            err -= adx;
            y += sy;
        } else {
            y += base;
        }
        val = vorbis_db_quant(logmask[x as usize]);
        mse += (y - val) * (y - val);
        n += 1;
        if logmdct[x as usize] + info.two_fit_atten >= logmask[x as usize] && val != 0 {
            if (y as f32 + info.max_over) < val as f32 {
                return true;
            }
            if (y as f32 - info.max_under) > val as f32 {
                return true;
            }
        }
        x += 1;
    }

    if info.max_over * info.max_over / n as f32 > info.max_err {
        return false;
    }
    if info.max_under * info.max_under / n as f32 > info.max_err {
        return false;
    }
    if (mse / n) as f32 > info.max_err {
        return true;
    }
    false
}

/// The encode-side floor1 curve fitter: precomputes the post sort/neighbor
/// indices once, then [`fit`](Self::fit)s a masking curve into floor posts.
///
/// Hand-ported from libvorbis `floor1_look` (index build) and `floor1_fit` (the
/// greedy progressive-splitting line fit). Derivative work of libvorbis/aoTuV
/// (BSD-3-Clause); see `LICENSE-THIRDPARTY`.
pub struct Floor1Fitter {
    postlist: Vec<i32>,
    n: i32,
    forward_index: Vec<usize>,
    reverse_index: Vec<usize>,
    sorted_index: Vec<i32>,
    loneighbor: Vec<usize>,
    hineighbor: Vec<usize>,
    info: Floor1FitInfo,
}

impl Floor1Fitter {
    /// Build the fitter for a floor1 `postlist` (post x-positions, first two the
    /// `0`/`n` endpoints) and analysis `info`. `n = postlist[1]` is the spectral
    /// bin count the masking curves must supply.
    #[must_use]
    pub fn new(postlist: Vec<i32>, info: Floor1FitInfo) -> Self {
        let n = postlist[1];
        let posts = postlist.len();
        // Sort post indices by position (libvorbis qsort + icomp); positions are
        // distinct, so the order is well-defined.
        let mut order: Vec<usize> = (0..posts).collect();
        order.sort_by_key(|&i| postlist[i]);
        let forward_index = order;
        let mut reverse_index = vec![0usize; posts];
        for (sorted_pos, &range) in forward_index.iter().enumerate() {
            reverse_index[range] = sorted_pos;
        }
        let sorted_index: Vec<i32> = forward_index.iter().map(|&i| postlist[i]).collect();
        let (loneighbor, hineighbor) = low_high_neighbors(&postlist);
        Self {
            postlist,
            n,
            forward_index,
            reverse_index,
            sorted_index,
            loneighbor,
            hineighbor,
            info,
        }
    }

    /// The number of floor posts (endpoints included).
    #[must_use]
    pub fn posts(&self) -> usize {
        self.postlist.len()
    }

    /// Fit floor posts to the per-bin masking curve `logmask` (the threshold to
    /// stay above), guided by the per-bin MDCT energy `logmdct` (both in the
    /// dB-quant domain, length `n`). Returns the post heights in `0..=1023` with
    /// declined posts flagged `0x8000`, ready for [`quantize_posts_to_mult`].
    /// Returns `None` when the whole curve quantizes to zero (floor unused).
    #[must_use]
    pub fn fit(&self, logmdct: &[f32], logmask: &[f32]) -> Option<Vec<i32>> {
        let posts = self.posts();
        let info = &self.info;

        // One line-fit accumulator per minimal division between sorted posts.
        let mut fits = vec![LsfitAcc::default(); posts.saturating_sub(1)];
        let mut nonzero = 0i32;
        for (i, slot) in fits.iter_mut().enumerate() {
            *slot = accumulate_fit(
                logmask,
                logmdct,
                self.sorted_index[i],
                self.sorted_index[i + 1],
                self.n,
                info,
            );
            nonzero += slot.an;
        }
        if nonzero == 0 {
            return None;
        }

        let mut value_a = vec![-200i32; posts];
        let mut value_b = vec![-200i32; posts];
        let mut loneighbor = vec![0usize; posts];
        let mut hineighbor = vec![1usize; posts];
        let mut memo = vec![-1i32; posts];

        // Fit the implicit base line across every division first.
        let mut y0 = -200;
        let mut y1 = -200;
        fit_line(&fits, &mut y0, &mut y1, info);
        value_a[0] = y0;
        value_b[0] = y0;
        value_a[1] = y1;
        value_b[1] = y1;

        // Greedy progressive splitting: visit posts in range order, splitting a
        // segment whenever its line exceeds the error bounds.
        for i in 2..posts {
            let sortpos = self.reverse_index[i];
            let ln = loneighbor[sortpos];
            let hn = hineighbor[sortpos];

            if memo[ln] == hn as i32 {
                continue;
            }
            let lsortpos = self.reverse_index[ln];
            let hsortpos = self.reverse_index[hn];
            memo[ln] = hn as i32;

            let lx = self.postlist[ln];
            let hx = self.postlist[hn];
            let ly = post_y(&value_a, &value_b, ln);
            let hy = post_y(&value_a, &value_b, hn);

            if !inspect_error(lx, hx, ly, hy, logmask, logmdct, info) {
                value_a[i] = -200;
                value_b[i] = -200;
                continue;
            }

            // Split: refit the two halves around the new post.
            let mut ly0 = -200;
            let mut ly1 = -200;
            let mut hy0 = -200;
            let mut hy1 = -200;
            let ret0 = fit_line(&fits[lsortpos..sortpos], &mut ly0, &mut ly1, info);
            let ret1 = fit_line(&fits[sortpos..hsortpos], &mut hy0, &mut hy1, info);
            if ret0 {
                ly0 = ly;
                ly1 = hy0;
            }
            if ret1 {
                hy0 = ly1;
                hy1 = hy;
            }

            if ret0 && ret1 {
                value_a[i] = -200;
                value_b[i] = -200;
            } else {
                value_b[ln] = ly0;
                if ln == 0 {
                    value_a[ln] = ly0;
                }
                value_a[i] = ly1;
                value_b[i] = hy0;
                value_a[hn] = hy1;
                if hn == 1 {
                    value_b[hn] = hy1;
                }
                if ly1 >= 0 || hy0 >= 0 {
                    // Re-point the dynamic neighbors to the freshly used post.
                    // The index walk (with an early break on the first
                    // non-matching neighbor) mirrors the C and reads clearer than
                    // an iterator here.
                    #[allow(clippy::needless_range_loop)]
                    for j in (0..sortpos).rev() {
                        if hineighbor[j] == hn {
                            hineighbor[j] = i;
                        } else {
                            break;
                        }
                    }
                    #[allow(clippy::needless_range_loop)]
                    for j in sortpos + 1..posts {
                        if loneighbor[j] == ln {
                            loneighbor[j] = i;
                        } else {
                            break;
                        }
                    }
                }
            }
        }

        // Emit posts, declining any whose value the curve interpolation already
        // predicts (they cost no bits and the decoder reconstructs them).
        let mut output = vec![0i32; posts];
        output[0] = post_y(&value_a, &value_b, 0);
        output[1] = post_y(&value_a, &value_b, 1);
        for i in 2..posts {
            let ln = self.loneighbor[i - 2];
            let hn = self.hineighbor[i - 2];
            let predicted = render_point(
                self.postlist[ln],
                self.postlist[hn],
                output[ln],
                output[hn],
                self.postlist[i],
            );
            let vx = post_y(&value_a, &value_b, i);
            if vx >= 0 && predicted != vx {
                output[i] = vx;
            } else {
                output[i] = predicted | 0x8000;
            }
        }
        Some(output)
    }
}

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
