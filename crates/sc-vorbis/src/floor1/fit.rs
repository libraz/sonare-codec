use super::*;

/// Least-squares line-fit accumulators for one minimal post division. Mirrors
/// libvorbis `lsfit_acc`: each spectral bin's quantized floor value is summed
/// into an `a` bucket (bins at/above the MDCT energy, weighted up in the fit) or
/// a `b` bucket (bins masked below it), so the line is pulled toward the audible
/// part of the spectrum.
#[derive(Clone, Copy, Default)]
pub(crate) struct LsfitAcc {
    pub(crate) x0: i32,
    pub(crate) x1: i32,
    pub(crate) xa: i32,
    pub(crate) ya: i32,
    pub(crate) x2a: i32,
    pub(crate) y2a: i32,
    pub(crate) xya: i32,
    pub(crate) an: i32,
    pub(crate) xb: i32,
    pub(crate) yb: i32,
    pub(crate) x2b: i32,
    pub(crate) y2b: i32,
    pub(crate) xyb: i32,
    pub(crate) bn: i32,
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
pub(crate) fn rint(x: f64) -> f64 {
    x.round_ties_even()
}

/// Combine the two candidate fit values for a post (libvorbis `post_Y`): use
/// whichever side is defined, or their midpoint when both are.
pub(crate) fn post_y(a: &[i32], b: &[i32], pos: usize) -> i32 {
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
pub(crate) fn accumulate_fit(
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
pub(crate) fn fit_line(a: &[LsfitAcc], y0: &mut i32, y1: &mut i32, info: &Floor1FitInfo) -> bool {
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
pub(crate) fn inspect_error(
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
