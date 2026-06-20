use super::*;

pub(crate) fn comb_filter_const(
    y: &mut [f32],
    x: &[f32],
    head: usize,
    t: usize,
    g10: f32,
    g11: f32,
    g12: f32,
) {
    let mut x4 = x[head - t - 2];
    let mut x3 = x[head - t - 1];
    let mut x2 = x[head - t];
    let mut x1 = x[head - t + 1];
    for (i, yi) in y.iter_mut().enumerate() {
        let x0 = x[head + i - t + 2];
        *yi = x[head + i] + g10 * x2 + g11 * (x1 + x3) + g12 * (x0 + x4);
        x4 = x3;
        x3 = x2;
        x2 = x1;
        x1 = x0;
    }
}

/// `comb_filter`: apply the pitch post-filter for `n` samples, cross-fading the
/// previous filter `(t0, g0, tapset0)` into the new one `(t1, g1, tapset1)` over
/// the first `overlap` samples (weighted by `window`²), then running the
/// steady-state filter for the rest.
///
/// `y` receives `n` outputs; `x` is the input buffer with `head` the index of
/// output sample 0 (history at `head - max(t0, t1) - 2 ..` must be valid).
#[allow(clippy::too_many_arguments)]
pub fn comb_filter(
    y: &mut [f32],
    x: &[f32],
    head: usize,
    t0: usize,
    t1: usize,
    n: usize,
    g0: f32,
    g1: f32,
    tapset0: usize,
    tapset1: usize,
    window: &[f32],
    overlap: usize,
) {
    if g0 == 0.0 && g1 == 0.0 {
        // No filtering: copy the input straight through.
        y[..n].copy_from_slice(&x[head..head + n]);
        return;
    }
    // A zero gain leaves the period unset; clamp it so we don't read garbage.
    let t0 = t0.max(COMBFILTER_MINPERIOD);
    let t1 = t1.max(COMBFILTER_MINPERIOD);
    let ga = COMB_GAINS[tapset0];
    let gb = COMB_GAINS[tapset1];
    let (g00, g01, g02) = (g0 * ga[0], g0 * ga[1], g0 * ga[2]);
    let (g10, g11, g12) = (g1 * gb[0], g1 * gb[1], g1 * gb[2]);

    let mut x1 = x[head - t1 + 1];
    let mut x2 = x[head - t1];
    let mut x3 = x[head - t1 - 1];
    let mut x4 = x[head - t1 - 2];

    // If the filter didn't change, the overlap cross-fade is a no-op.
    let overlap = if g0 == g1 && t0 == t1 && tapset0 == tapset1 {
        0
    } else {
        overlap
    };

    for i in 0..overlap {
        let x0 = x[head + i - t1 + 2];
        let f = window[i] * window[i];
        let inv = 1.0 - f;
        y[i] = x[head + i]
            + inv * g00 * x[head + i - t0]
            + inv * g01 * (x[head + i - t0 + 1] + x[head + i - t0 - 1])
            + inv * g02 * (x[head + i - t0 + 2] + x[head + i - t0 - 2])
            + f * g10 * x2
            + f * g11 * (x1 + x3)
            + f * g12 * (x0 + x4);
        x4 = x3;
        x3 = x2;
        x2 = x1;
        x1 = x0;
    }

    if g1 == 0.0 {
        // The new filter is off: copy the remaining input straight through.
        y[overlap..n].copy_from_slice(&x[head + overlap..head + n]);
        return;
    }

    comb_filter_const(&mut y[overlap..n], x, head + overlap, t1, g10, g11, g12);
}
