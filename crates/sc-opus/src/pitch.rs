//! CELT pitch analysis and post-filter (comb filter).
//!
//! Hand-ported to safe Rust from the float build of libopus: the comb filter
//! from `celt/celt.c` (`comb_filter` / `comb_filter_const_c`) and the pitch
//! estimator from `celt/pitch.c` (`celt_pitch_xcorr`, `find_best_pitch`,
//! `pitch_search`, `pitch_downsample`, `remove_doubling`, plus the
//! `_celt_autocorr` / `_celt_lpc` / `celt_fir5` it depends on). Derivative work
//! of libopus (BSD-3-Clause); see `LICENSE-THIRDPARTY`.
//!
//! The comb filter reinforces a periodic (pitched) component by adding delayed
//! copies of the signal at the pitch period `T`, weighted by a 3-tap kernel
//! selected by `tapset` and scaled by the post-filter gain. [`comb_filter`]
//! handles the gain/period *transition* at a frame boundary by cross-fading the
//! old filter `(T0, g0, tapset0)` into the new one `(T1, g1, tapset1)` over the
//! overlap window; the steady-state body is [`comb_filter_const`].
//!
//! The analysis side runs at half rate: [`pitch_downsample`] decimates the
//! input by two and LPC-whitens it, [`pitch_search`] finds the lag, and
//! [`remove_doubling`] corrects octave errors and reports the post-filter gain.
//!
//! Indexing note: the C reads `x[-T-2 .. -T+2]`, i.e. history *before* the first
//! output sample. Safe Rust can't index negatively, so the buffer is passed whole
//! with an explicit `head` offset and the routines read `x[head + i - T + k]`;
//! callers guarantee `head >= T + 2` so the history is in bounds.

// Consumed by the CELT prefilter (encoder) and matches the decoder post-filter;
// the live encoder still ships via the Opus FFI path.
#![allow(dead_code)]

use crate::range_coder::{RangeDecoder, RangeEncoder};

/// `COMBFILTER_MINPERIOD`: the shortest pitch period the comb filter accepts.
pub const COMBFILTER_MINPERIOD: usize = 15;
/// `COMBFILTER_MAXPERIOD`: the longest pitch period (history the buffer must
/// carry ahead of the first output sample).
pub const COMBFILTER_MAXPERIOD: usize = 1024;

/// The three 3-tap post-filter kernels (`gains[tapset][tap]`), as the exact Q15
/// constants from libopus expressed over 32768 so the float arithmetic matches.
const COMB_GAINS: [[f32; 3]; 3] = [
    [10048.0 / 32768.0, 7112.0 / 32768.0, 4248.0 / 32768.0],
    [15200.0 / 32768.0, 8784.0 / 32768.0, 0.0],
    [26208.0 / 32768.0, 3280.0 / 32768.0, 0.0],
];

/// `comb_filter_const`: the steady-state comb filter for a fixed period `t` and
/// 3-tap gains `(g10, g11, g12)`. Writes `y.len()` outputs; reads `x` from
/// `head - t - 2` to `head + y.len() - t + 1`.
fn comb_filter_const(
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

/// `celt_inner_prod`: the dot product `Σ_{j<len} x[j]·y[j]`.
fn inner_prod(x: &[f32], y: &[f32], len: usize) -> f32 {
    let mut sum = 0.0f32;
    for j in 0..len {
        sum += x[j] * y[j];
    }
    sum
}

/// `celt_pitch_xcorr`: the cross-correlation `xcorr[i] = Σ_{j<len} x[j]·y[i+j]`
/// for each lag `i` in `0..max_pitch`.
///
/// Hand-ported to safe Rust from libopus `celt/pitch.c` (the unrolled SIMD
/// kernel collapses to this straight double loop in the reference path). `y`
/// must hold at least `len + max_pitch - 1` samples.
pub fn celt_pitch_xcorr(x: &[f32], y: &[f32], xcorr: &mut [f32], len: usize, max_pitch: usize) {
    for (i, slot) in xcorr.iter_mut().enumerate().take(max_pitch) {
        *slot = inner_prod(x, &y[i..], len);
    }
}

/// `find_best_pitch`: pick the two lags maximising the normalised correlation
/// `xcorr[i]² / Syy_i`, where `Syy_i` is the running energy of the `len`-sample
/// window of `y` at lag `i`. Hand-ported from libopus `celt/pitch.c` (float
/// build). `y` must hold `len + max_pitch` samples. The `1e-12` scale on the
/// correlation mirrors the C, keeping the squared term within `f32` range.
fn find_best_pitch(
    xcorr: &[f32],
    y: &[f32],
    len: usize,
    max_pitch: usize,
    best_pitch: &mut [usize; 2],
) {
    let mut syy = 1.0f32;
    let mut best_num = [-1.0f32; 2];
    let mut best_den = [0.0f32; 2];
    best_pitch[0] = 0;
    best_pitch[1] = 1;
    for &v in &y[..len] {
        syy += v * v;
    }
    for (i, &xc) in xcorr.iter().enumerate().take(max_pitch) {
        if xc > 0.0 {
            let xcorr16 = xc * 1e-12;
            let num = xcorr16 * xcorr16;
            if num * best_den[1] > best_num[1] * syy {
                if num * best_den[0] > best_num[0] * syy {
                    best_num[1] = best_num[0];
                    best_den[1] = best_den[0];
                    best_pitch[1] = best_pitch[0];
                    best_num[0] = num;
                    best_den[0] = syy;
                    best_pitch[0] = i;
                } else {
                    best_num[1] = num;
                    best_den[1] = syy;
                    best_pitch[1] = i;
                }
            }
        }
        syy += y[i + len] * y[i + len] - y[i] * y[i];
        syy = syy.max(1.0);
    }
}

/// `pitch_search`: estimate the pitch lag of the half-rate signal `x_lp` within
/// `y`, returning the lag in full-rate samples (twice the half-rate offset, so it
/// feeds the full-rate comb filter directly).
///
/// Hand-ported from libopus `celt/pitch.c`. `len` and `max_pitch` are full-rate
/// counts (the signals are at half rate, so the working lengths are `len >> 1`
/// etc.). A coarse 4× decimated search narrows the range, a 2× decimated search
/// refines it, and a parabolic pseudo-interpolation nudges the final lag.
/// `x_lp` must hold `len >> 1` samples and `y` at least `(len + max_pitch) >> 1`.
#[must_use]
pub fn pitch_search(x_lp: &[f32], y: &[f32], len: usize, max_pitch: usize) -> usize {
    let lag = len + max_pitch;
    let len4 = len >> 2;
    let lag4 = lag >> 2;

    // Downsample both signals by 2 again for the coarse pass.
    let x_lp4: Vec<f32> = (0..len4).map(|j| x_lp[2 * j]).collect();
    let y_lp4: Vec<f32> = (0..lag4).map(|j| y[2 * j]).collect();

    let mut best_pitch = [0usize; 2];
    let mut coarse = vec![0.0f32; max_pitch >> 2];
    celt_pitch_xcorr(&x_lp4, &y_lp4, &mut coarse, len4, max_pitch >> 2);
    find_best_pitch(&coarse, &y_lp4, len4, max_pitch >> 2, &mut best_pitch);

    // Finer 2× search, but only near the two coarse candidates.
    let mut xcorr = vec![0.0f32; max_pitch >> 1];
    for (i, slot) in xcorr.iter_mut().enumerate() {
        let d0 = (i as i32 - 2 * best_pitch[0] as i32).abs();
        let d1 = (i as i32 - 2 * best_pitch[1] as i32).abs();
        if d0 > 2 && d1 > 2 {
            continue;
        }
        *slot = inner_prod(x_lp, &y[i..], len >> 1).max(-1.0);
    }
    find_best_pitch(&xcorr, y, len >> 1, max_pitch >> 1, &mut best_pitch);

    // Parabolic pseudo-interpolation around the winning lag.
    let offset = if best_pitch[0] > 0 && best_pitch[0] < (max_pitch >> 1) - 1 {
        let a = xcorr[best_pitch[0] - 1];
        let b = xcorr[best_pitch[0]];
        let c = xcorr[best_pitch[0] + 1];
        if (c - a) > 0.7 * (b - a) {
            1
        } else if (a - c) > 0.7 * (b - c) {
            -1
        } else {
            0
        }
    } else {
        0
    };

    (2 * best_pitch[0] as i32 - offset).max(0) as usize
}

/// `_celt_autocorr` (no-window float path): the autocorrelation
/// `ac[k] = Σ_{i} x[i]·x[i+k]` for lags `0..=lag`. `n` is the sample count and
/// must exceed `lag`. (The C splits this into a fast `celt_pitch_xcorr` block
/// plus a tail; the straight double loop is the same result.)
fn celt_autocorr(x: &[f32], ac: &mut [f32], n: usize, lag: usize) {
    for (k, slot) in ac.iter_mut().enumerate().take(lag + 1) {
        let mut d = 0.0f32;
        for i in k..n {
            d += x[i] * x[i - k];
        }
        *slot = d;
    }
}

/// `_celt_lpc` (float path): Levinson-Durbin recursion turning the `p + 1`
/// autocorrelation values `ac` into `p` LPC coefficients. The coefficients
/// define the whitening filter `A(z) = 1 + Σ lpc[i] z^{-(i+1)}`; the recursion
/// bails out once the residual error drops 30 dB below `ac[0]`.
fn celt_lpc(lpc: &mut [f32], ac: &[f32], p: usize) {
    for v in lpc[..p].iter_mut() {
        *v = 0.0;
    }
    let mut error = ac[0];
    if ac[0] == 0.0 {
        return;
    }
    for i in 0..p {
        // This iteration's reflection coefficient.
        let mut rr = 0.0f32;
        for j in 0..i {
            rr += lpc[j] * ac[i - j];
        }
        rr += ac[i + 1];
        let r = -rr / error;
        lpc[i] = r;
        // Update the lower-order coefficients in mirror pairs.
        for j in 0..(i + 1) >> 1 {
            let tmp1 = lpc[j];
            let tmp2 = lpc[i - 1 - j];
            lpc[j] = tmp1 + r * tmp2;
            lpc[i - 1 - j] = tmp2 + r * tmp1;
        }
        error -= r * r * error;
        // Stop once we have 30 dB of prediction gain.
        if error < 0.001 * ac[0] {
            break;
        }
    }
}

/// `celt_fir5` (float path): the in-place 5-tap FIR `x[i] += Σ_k num[k]·x[i-1-k]`
/// using the past *inputs* as filter memory (zero history at the start).
fn celt_fir5(x: &mut [f32], num: &[f32; 5], nn: usize) {
    let (mut m0, mut m1, mut m2, mut m3, mut m4) = (0.0f32, 0.0f32, 0.0f32, 0.0f32, 0.0f32);
    for xi in x.iter_mut().take(nn) {
        let sum = *xi + num[0] * m0 + num[1] * m1 + num[2] * m2 + num[3] * m3 + num[4] * m4;
        m4 = m3;
        m3 = m2;
        m2 = m1;
        m1 = m0;
        m0 = *xi;
        *xi = sum;
    }
}

/// `pitch_downsample`: decimate the input by two (averaging a 3-tap low-pass)
/// and LPC-whiten the result, producing the half-rate analysis signal the pitch
/// search runs on. `x` holds `channels` slices of at least `len` samples each;
/// `x_lp` receives `len >> 1` whitened samples. Stereo inputs are summed.
///
/// Hand-ported from libopus `celt/pitch.c` (float build).
pub fn pitch_downsample(x: &[&[f32]], x_lp: &mut [f32], len: usize, channels: usize) {
    let half = len >> 1;
    // Stride-2 decimation with a 3-tap low-pass; the cross-offset reads
    // (2i-1, 2i, 2i+1) read clearest with an explicit index.
    #[allow(clippy::needless_range_loop)]
    for i in 1..half {
        x_lp[i] = 0.5 * (0.5 * (x[0][2 * i - 1] + x[0][2 * i + 1]) + x[0][2 * i]);
    }
    x_lp[0] = 0.5 * (0.5 * x[0][1] + x[0][0]);
    if channels == 2 {
        #[allow(clippy::needless_range_loop)]
        for i in 1..half {
            x_lp[i] += 0.5 * (0.5 * (x[1][2 * i - 1] + x[1][2 * i + 1]) + x[1][2 * i]);
        }
        x_lp[0] += 0.5 * (0.5 * x[1][1] + x[1][0]);
    }

    let mut ac = [0.0f32; 5];
    celt_autocorr(&x_lp[..half], &mut ac, half, 4);
    // Noise floor at -40 dB, then a light lag window to tame the LPC.
    ac[0] *= 1.0001;
    for (i, slot) in ac.iter_mut().enumerate().skip(1) {
        let w = 0.008 * i as f32;
        *slot -= *slot * w * w;
    }

    let mut lpc = [0.0f32; 4];
    celt_lpc(&mut lpc, &ac, 4);
    // Bandwidth-expand the LPC (0.9^k chirp).
    let mut tmp = 1.0f32;
    for v in lpc.iter_mut() {
        tmp *= 0.9;
        *v *= tmp;
    }
    // Add a zero at 0.8 to flatten the response a little further.
    let c1 = 0.8f32;
    let lpc2 = [
        lpc[0] + 0.8,
        lpc[1] + c1 * lpc[0],
        lpc[2] + c1 * lpc[1],
        lpc[3] + c1 * lpc[2],
        c1 * lpc[3],
    ];
    celt_fir5(&mut x_lp[..half], &lpc2, half);
}

/// `compute_pitch_gain` (float path): the normalised correlation
/// `xy / sqrt(1 + xx·yy)`.
fn compute_pitch_gain(xy: f32, xx: f32, yy: f32) -> f32 {
    xy / (1.0 + xx * yy).sqrt()
}

/// `dual_inner_prod`: the two dot products `Σ x·y01` and `Σ x·y02` in one pass.
fn dual_inner_prod(x: &[f32], y01: &[f32], y02: &[f32], n: usize) -> (f32, f32) {
    let mut a = 0.0f32;
    let mut b = 0.0f32;
    for i in 0..n {
        a += x[i] * y01[i];
        b += x[i] * y02[i];
    }
    (a, b)
}

/// `remove_doubling`: refine a pitch estimate by checking for stronger
/// correlation at sub-multiples `T/k` of the candidate period, correcting the
/// common octave (pitch-doubling) error, and report the post-filter gain.
///
/// Operates at half rate: `maxperiod`, `minperiod`, `n` and `prev_period` are
/// full-rate counts (halved internally), `x` is the half-rate buffer with
/// `maxperiod >> 1` samples of history ahead of the analysis window (so it must
/// hold `(maxperiod >> 1) + (n >> 1)` samples). `t0` carries the full-rate lag
/// in and the corrected full-rate lag out. Returns the pitch gain in `[0, 1]`.
///
/// Hand-ported from libopus `celt/pitch.c` (float build).
pub fn remove_doubling(
    x: &[f32],
    maxperiod: usize,
    minperiod: usize,
    n: usize,
    t0: &mut i32,
    prev_period: i32,
    prev_gain: f32,
) -> f32 {
    const SECOND_CHECK: [i32; 16] = [0, 0, 3, 2, 3, 2, 5, 2, 3, 2, 3, 2, 5, 2, 3, 2];

    let minperiod0 = minperiod as i32;
    let maxperiod = (maxperiod / 2) as i32;
    let minperiod = (minperiod / 2) as i32;
    let prev_period = prev_period / 2;
    let nn = n / 2;
    let head = maxperiod as usize; // index of analysis sample 0 (C does `x += maxperiod`)

    // Clamp the incoming (halved) lag into range.
    let t0i = {
        let h = *t0 / 2;
        if h >= maxperiod {
            maxperiod - 1
        } else {
            h
        }
    };
    let mut t = t0i;

    let (xx, xy) = dual_inner_prod(&x[head..], &x[head..], &x[head - t0i as usize..], nn);
    let mut yy_lookup = vec![0.0f32; maxperiod as usize + 1];
    yy_lookup[0] = xx;
    let mut yy = xx;
    for i in 1..=maxperiod as usize {
        yy = yy + x[head - i] * x[head - i] - x[head + nn - i] * x[head + nn - i];
        yy_lookup[i] = yy.max(0.0);
    }
    let yy = yy_lookup[t0i as usize];

    let mut best_xy = xy;
    let mut best_yy = yy;
    let g0 = compute_pitch_gain(xy, xx, yy);
    let mut g = g0;

    // Look for a stronger pitch at T/k for k = 2..=15.
    for k in 2..=15i32 {
        let t1 = (2 * t0i + k) / (2 * k);
        if t1 < minperiod {
            break;
        }
        // A second candidate period to corroborate T1.
        let t1b = if k == 2 {
            if t1 + t0i > maxperiod {
                t0i
            } else {
                t0i + t1
            }
        } else {
            (2 * SECOND_CHECK[k as usize] * t0i + k) / (2 * k)
        };
        let (xya, xyb) = dual_inner_prod(
            &x[head..],
            &x[head - t1 as usize..],
            &x[head - t1b as usize..],
            nn,
        );
        let xy1 = 0.5 * (xya + xyb);
        let yy1 = 0.5 * (yy_lookup[t1 as usize] + yy_lookup[t1b as usize]);
        let g1 = compute_pitch_gain(xy1, xx, yy1);

        // Carry a bias toward the previous frame's period if T1 is close to it.
        let cont = if (t1 - prev_period).abs() <= 1 {
            prev_gain
        } else if (t1 - prev_period).abs() <= 2 && 5 * k * k < t0i {
            0.5 * prev_gain
        } else {
            0.0
        };
        // Bias against very short periods to avoid short-term false positives.
        let mut thresh = (0.7 * g0 - cont).max(0.3);
        if t1 < 3 * minperiod {
            thresh = (0.85 * g0 - cont).max(0.4);
        } else if t1 < 2 * minperiod {
            thresh = (0.9 * g0 - cont).max(0.5);
        }
        if g1 > thresh {
            best_xy = xy1;
            best_yy = yy1;
            t = t1;
            g = g1;
        }
    }

    best_xy = best_xy.max(0.0);
    let pg_raw = if best_yy <= best_xy {
        1.0
    } else {
        best_xy / (best_yy + 1.0)
    };

    // Parabolic refinement: nudge the lag by ±1 toward the correlation peak.
    let mut xcorr = [0.0f32; 3];
    for (k, slot) in xcorr.iter_mut().enumerate() {
        let lag = (t + k as i32 - 1) as usize;
        *slot = inner_prod(&x[head..], &x[head - lag..], nn);
    }
    let offset = if (xcorr[2] - xcorr[0]) > 0.7 * (xcorr[1] - xcorr[0]) {
        1
    } else if (xcorr[0] - xcorr[2]) > 0.7 * (xcorr[1] - xcorr[2]) {
        -1
    } else {
        0
    };

    let pg = pg_raw.min(g);
    let t0_new = (2 * t + offset).max(minperiod0);
    *t0 = t0_new;
    pg
}

/// The post-filter decision for one frame, as produced by [`run_prefilter`] and
/// serialised by [`encode_postfilter`].
pub struct PostfilterParams {
    /// Whether the post-filter is enabled for this frame.
    pub pf_on: bool,
    /// The pitch period in full-rate samples (`COMBFILTER_MINPERIOD ..= MAXPERIOD-2`).
    pub pitch_index: i32,
    /// The (quantised) post-filter gain.
    pub gain: f32,
    /// The 3-bit quantised gain index (`0..=7`).
    pub qg: i32,
    /// The selected tapset (`0..=2`).
    pub tapset: usize,
}

/// Per-encoder prefilter state carried across frames: the comb-filter history,
/// the overlap memory, and the previous frame's period/gain/tapset.
pub struct PrefilterState {
    /// Per-channel comb-filter history (`cc * COMBFILTER_MAXPERIOD`).
    pub prefilter_mem: Vec<f32>,
    /// Per-channel overlap memory feeding the next frame's window (`cc * overlap`).
    pub in_mem: Vec<f32>,
    /// Previous frame's pitch period (full-rate samples).
    pub prefilter_period: i32,
    /// Previous frame's post-filter gain.
    pub prefilter_gain: f32,
    /// Previous frame's tapset.
    pub prefilter_tapset: usize,
}

impl PrefilterState {
    /// A zeroed state, matching a freshly reset encoder.
    #[must_use]
    pub fn new(cc: usize, overlap: usize) -> Self {
        Self {
            prefilter_mem: vec![0.0; cc * COMBFILTER_MAXPERIOD],
            in_mem: vec![0.0; cc * overlap],
            prefilter_period: 0,
            prefilter_gain: 0.0,
            prefilter_tapset: 0,
        }
    }
}

/// `run_prefilter`: estimate the pitch, decide whether to enable the
/// post-filter, comb-filter the pre-emphasised input in place, and report the
/// post-filter parameters for the bitstream.
///
/// `in_buf` holds `cc` channel planes of `n + overlap` samples; on entry the
/// frame occupies `[overlap .. overlap + n]` (the overlap prefix is rewritten
/// here from `state.in_mem`). `window` is the overlap window. `new_tapset` is
/// this frame's tapset decision, `enabled` gates the whole search, and
/// `nb_available_bytes` drives the enable threshold. The comb history and
/// overlap memory in `state` are updated for the next frame.
///
/// Hand-ported from libopus `celt/celt_encoder.c` (`run_prefilter`, float build).
#[allow(clippy::too_many_arguments)]
pub fn run_prefilter(
    in_buf: &mut [f32],
    n: usize,
    cc: usize,
    overlap: usize,
    short_mdct_size: usize,
    window: &[f32],
    new_tapset: usize,
    enabled: bool,
    nb_available_bytes: i32,
    state: &mut PrefilterState,
) -> PostfilterParams {
    let stride = n + overlap;
    let mp = COMBFILTER_MAXPERIOD;

    // pre[c] = [comb history | this frame's samples].
    let mut pre = vec![0.0f32; cc * (n + mp)];
    for c in 0..cc {
        let pbase = c * (n + mp);
        pre[pbase..pbase + mp].copy_from_slice(&state.prefilter_mem[c * mp..c * mp + mp]);
        let fbase = c * stride + overlap;
        pre[pbase + mp..pbase + mp + n].copy_from_slice(&in_buf[fbase..fbase + n]);
    }

    let (pitch_index, mut gain1) = if enabled {
        let mut pitch_buf = vec![0.0f32; (mp + n) >> 1];
        let chans: Vec<&[f32]> = (0..cc)
            .map(|c| &pre[c * (n + mp)..c * (n + mp) + n + mp])
            .collect();
        pitch_downsample(&chans, &mut pitch_buf, mp + n, cc);
        // Skip the last 1.5 octaves: too many short-term false positives.
        let lag = pitch_search(
            &pitch_buf[mp >> 1..],
            &pitch_buf,
            n,
            mp - 3 * COMBFILTER_MINPERIOD,
        );
        let mut pi = mp as i32 - lag as i32;
        let g = remove_doubling(
            &pitch_buf,
            mp,
            COMBFILTER_MINPERIOD,
            n,
            &mut pi,
            state.prefilter_period,
            state.prefilter_gain,
        );
        pi = pi.min(mp as i32 - 2);
        (pi, 0.7 * g)
    } else {
        (COMBFILTER_MINPERIOD as i32, 0.0)
    };

    // Gain threshold for enabling the post-filter, adjusted for rate/continuity.
    let mut pf_threshold = 0.2f32;
    if (pitch_index - state.prefilter_period).abs() * 10 > pitch_index {
        pf_threshold += 0.2;
    }
    if nb_available_bytes < 25 {
        pf_threshold += 0.1;
    }
    if nb_available_bytes < 35 {
        pf_threshold += 0.1;
    }
    if state.prefilter_gain > 0.4 {
        pf_threshold -= 0.1;
    }
    if state.prefilter_gain > 0.55 {
        pf_threshold -= 0.1;
    }
    pf_threshold = pf_threshold.max(0.2);

    let (pf_on, qg) = if gain1 < pf_threshold {
        gain1 = 0.0;
        (false, 0)
    } else {
        // Snap to the previous gain when close, to avoid needless transitions.
        if (gain1 - state.prefilter_gain).abs() < 0.1 {
            gain1 = state.prefilter_gain;
        }
        let q = ((0.5 + gain1 * 32.0 / 3.0).floor() as i32 - 1).clamp(0, 7);
        gain1 = 0.09375 * (q + 1) as f32;
        (true, q)
    };

    // Apply the comb pre-filter in place; carry the overlap and comb histories.
    let offset = short_mdct_size - overlap;
    state.prefilter_period = state.prefilter_period.max(COMBFILTER_MINPERIOD as i32);
    let pp = state.prefilter_period as usize;
    let pg = state.prefilter_gain;
    let pt = state.prefilter_tapset;
    let t1 = pitch_index as usize;
    for c in 0..cc {
        let pbase = c * (n + mp);
        let fbase = c * stride;
        // Restore the (previously comb-filtered) overlap prefix from in_mem.
        in_buf[fbase..fbase + overlap]
            .copy_from_slice(&state.in_mem[c * overlap..c * overlap + overlap]);
        let pchan = &pre[pbase..pbase + n + mp];
        if offset != 0 {
            comb_filter(
                &mut in_buf[fbase + overlap..],
                pchan,
                mp,
                pp,
                pp,
                offset,
                -pg,
                -pg,
                pt,
                pt,
                &[],
                0,
            );
        }
        comb_filter(
            &mut in_buf[fbase + overlap + offset..],
            pchan,
            mp + offset,
            pp,
            t1,
            n - offset,
            -pg,
            -gain1,
            pt,
            new_tapset,
            window,
            overlap,
        );
        // Save this frame's filtered tail for the next frame's window.
        state.in_mem[c * overlap..c * overlap + overlap]
            .copy_from_slice(&in_buf[fbase + n..fbase + n + overlap]);
        // Slide the comb history forward by one frame.
        if n > mp {
            state.prefilter_mem[c * mp..c * mp + mp]
                .copy_from_slice(&pre[pbase + n..pbase + n + mp]);
        } else {
            state
                .prefilter_mem
                .copy_within(c * mp + n..c * mp + mp, c * mp);
            state.prefilter_mem[c * mp + mp - n..c * mp + mp]
                .copy_from_slice(&pre[pbase + mp..pbase + mp + n]);
        }
    }

    // Carry the chosen period/gain/tapset to the next frame (libopus stores
    // these on the encoder state after the post-filter block).
    state.prefilter_period = pitch_index;
    state.prefilter_gain = gain1;
    state.prefilter_tapset = new_tapset;

    PostfilterParams {
        pf_on,
        pitch_index,
        gain: gain1,
        qg,
        tapset: new_tapset,
    }
}

/// `tapset_icdf`: the inverse CDF for the three post-filter tapsets.
const TAPSET_ICDF: [u8; 3] = [2, 1, 0];

/// Write the post-filter section of the CELT frame header.
///
/// Mirrors the `pf_on` branch of libopus `celt_encode_with_ec`: with the filter
/// off it writes a single "off" flag when there is room; with it on it writes
/// the "on" flag, the pitch (octave + in-octave bits), the 3-bit quantised gain,
/// and the tapset. The section only exists at `start == 0`.
pub fn encode_postfilter(
    enc: &mut RangeEncoder,
    pf: &PostfilterParams,
    start: usize,
    total_bits: i32,
) {
    if start != 0 {
        return;
    }
    if !pf.pf_on {
        if enc.ec_tell() + 16 <= total_bits {
            enc.enc_bit_logp(false, 1);
        }
        return;
    }
    enc.enc_bit_logp(true, 1);
    let pitch1 = pf.pitch_index + 1;
    let octave = 27 - (pitch1 as u32).leading_zeros() as i32;
    enc.enc_uint(octave as u32, 6);
    enc.enc_bits((pitch1 - (16 << octave)) as u32, (4 + octave) as u32);
    enc.enc_bits(pf.qg as u32, 3);
    enc.enc_icdf(pf.tapset, &TAPSET_ICDF, 2);
}

/// Read the post-filter section written by [`encode_postfilter`]. Returns the
/// decoded parameters when the filter is on, or `None` when it is off or the
/// section is absent (`start != 0` or no room), matching the decoder's gating.
#[must_use]
pub fn decode_postfilter(
    dec: &mut RangeDecoder,
    start: usize,
    total_bits: i32,
) -> Option<PostfilterParams> {
    if start != 0 || dec.ec_tell() + 16 > total_bits || !dec.dec_bit_logp(1) {
        return None;
    }
    let octave = dec.dec_uint(6) as i32;
    let pitch_index = (16 << octave) + dec.dec_bits((4 + octave) as u32) as i32 - 1;
    let qg = dec.dec_bits(3) as i32;
    // The tapset is only present when there are still bits to spare.
    let tapset = if dec.ec_tell() + 2 <= total_bits {
        dec.dec_icdf(&TAPSET_ICDF, 2)
    } else {
        0
    };
    Some(PostfilterParams {
        pf_on: true,
        pitch_index,
        gain: 0.09375 * (qg + 1) as f32,
        qg,
        tapset,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A buffer of `len` samples preceded by `COMBFILTER_MAXPERIOD` history, both
    /// produced by `f`. Returns `(buf, head)` with `head == COMBFILTER_MAXPERIOD`.
    fn buf_with_history(len: usize, f: impl Fn(usize) -> f32) -> (Vec<f32>, usize) {
        let head = COMBFILTER_MAXPERIOD;
        let mut buf = vec![0.0f32; head + len];
        for (i, v) in buf.iter_mut().enumerate() {
            // Index relative to the first output sample (can be negative history).
            *v = f(i.wrapping_sub(head));
        }
        (buf, head)
    }

    #[test]
    fn zero_gain_is_identity() {
        let (x, head) = buf_with_history(64, |i| (i as f32 * 0.3).sin());
        let win = vec![0.0f32; 0];
        let mut y = vec![999.0f32; 64];
        comb_filter(&mut y, &x, head, 40, 40, 64, 0.0, 0.0, 0, 0, &win, 0);
        assert_eq!(&y[..], &x[head..head + 64]);
    }

    #[test]
    fn dc_steady_state_matches_closed_form() {
        // Equal old/new filter params skip the overlap, so the whole frame runs
        // the constant filter. For a DC input every delayed tap equals 1, giving
        // y = 1 + g*(k0 + 2*k1 + 2*k2) exactly.
        let n = 50;
        let (x, head) = buf_with_history(n, |_| 1.0);
        let win = vec![0.0f32; 0];
        for (tapset, k) in COMB_GAINS.iter().enumerate() {
            let g = 0.5f32;
            let mut y = vec![0.0f32; n];
            comb_filter(&mut y, &x, head, 30, 30, n, g, g, tapset, tapset, &win, 0);
            let expected = 1.0 + g * (k[0] + 2.0 * k[1] + 2.0 * k[2]);
            for &yi in &y {
                assert!(
                    (yi - expected).abs() < 1e-6,
                    "tapset {tapset}: {yi} vs {expected}"
                );
            }
        }
    }

    #[test]
    fn reinforces_a_periodic_component() {
        // A tone whose period equals the comb period should be amplified: each
        // delayed tap lands in phase, so output energy exceeds input energy.
        let period = 32usize;
        let n = 256;
        let (x, head) = buf_with_history(n, |i| {
            (i as f32 / period as f32 * std::f32::consts::TAU).sin()
        });
        let win = vec![0.0f32; 0];
        let mut y = vec![0.0f32; n];
        comb_filter(&mut y, &x, head, period, period, n, 0.8, 0.8, 0, 0, &win, 0);
        let e_in: f32 = x[head..head + n].iter().map(|v| v * v).sum();
        let e_out: f32 = y.iter().map(|v| v * v).sum();
        assert!(
            e_out > 1.3 * e_in,
            "periodic energy not reinforced: {e_out} vs {e_in}"
        );
    }

    #[test]
    fn overlap_cross_fade_is_continuous_and_deterministic() {
        // With differing old/new gains the first `overlap` samples blend the two
        // filters; the result must be reproducible and reduce to the constant
        // filter once past the overlap.
        let n = 120;
        let overlap = 24;
        let (x, head) = buf_with_history(n, |i| {
            (i as f32 * 0.17).cos() + 0.5 * (i as f32 * 0.4).sin()
        });
        // A monotone-ish power-complementary window stand-in: sin ramp.
        let win: Vec<f32> = (0..overlap)
            .map(|i| ((i as f32 + 0.5) / overlap as f32 * std::f32::consts::FRAC_PI_2).sin())
            .collect();

        let mut y1 = vec![0.0f32; n];
        comb_filter(&mut y1, &x, head, 40, 48, n, 0.2, 0.7, 0, 1, &win, overlap);
        let mut y2 = vec![0.0f32; n];
        comb_filter(&mut y2, &x, head, 40, 48, n, 0.2, 0.7, 0, 1, &win, overlap);
        assert_eq!(y1, y2, "comb_filter is not deterministic");

        // Past the overlap the output equals the pure new-filter result.
        let gb = COMB_GAINS[1];
        let (g10, g11, g12) = (0.7 * gb[0], 0.7 * gb[1], 0.7 * gb[2]);
        let mut steady = vec![0.0f32; n - overlap];
        comb_filter_const(&mut steady, &x, head + overlap, 48, g10, g11, g12);
        assert_eq!(
            &y1[overlap..],
            &steady[..],
            "body diverges from constant filter"
        );
    }

    #[test]
    fn pitch_xcorr_matches_direct_dot_products() {
        let len = 40;
        let max_pitch = 24;
        let x: Vec<f32> = (0..len).map(|i| (i as f32 * 0.31).sin()).collect();
        let y: Vec<f32> = (0..len + max_pitch)
            .map(|i| (i as f32 * 0.17).cos() - 0.3 * i as f32 * 0.01)
            .collect();
        let mut xcorr = vec![0.0f32; max_pitch];
        celt_pitch_xcorr(&x, &y, &mut xcorr, len, max_pitch);
        for i in 0..max_pitch {
            let want: f32 = (0..len).map(|j| x[j] * y[i + j]).sum();
            assert!(
                (xcorr[i] - want).abs() < 1e-3,
                "lag {i}: {} vs {want}",
                xcorr[i]
            );
        }
    }

    #[test]
    fn find_best_pitch_picks_the_normalised_peak() {
        let len = 20;
        let max_pitch = 16;
        // Flat energy in y so the decision is driven purely by the correlation.
        let y = vec![1.0f32; len + max_pitch];
        let mut xcorr = vec![0.0f32; max_pitch];
        xcorr[5] = 100.0;
        xcorr[11] = 40.0;
        let mut best = [0usize; 2];
        find_best_pitch(&xcorr, &y, len, max_pitch, &mut best);
        assert_eq!(best[0], 5, "strongest lag");
        assert_eq!(best[1], 11, "second strongest lag");
    }

    #[test]
    fn pitch_search_recovers_a_known_lag() {
        // Full-rate counts; the signals are at half rate. A half-rate sinusoid
        // whose period exceeds the search range has a single in-range match, at
        // the offset where the current frame `x_lp` was lifted from `y`. The
        // returned lag is full-rate, i.e. twice the half-rate offset.
        let len = 256usize;
        let max_pitch = 200usize;
        let half_period = 90.0f32;
        let y_len = (len + max_pitch) >> 1;
        let y: Vec<f32> = (0..y_len + 4)
            .map(|i| (i as f32 / half_period * std::f32::consts::TAU).sin())
            .collect();
        let half_lag = 15usize; // half-rate offset the frame is taken from
        let frame = len >> 1;
        let x_lp: Vec<f32> = (0..frame).map(|k| y[half_lag + k]).collect();

        let pitch = pitch_search(&x_lp, &y, len, max_pitch);
        let expected_full = 2 * half_lag;
        assert!(
            (pitch as i32 - expected_full as i32).abs() <= 2,
            "recovered full-rate lag {pitch} not within 2 of {expected_full}"
        );
    }

    #[test]
    fn autocorr_matches_definition() {
        let n = 50;
        let lag = 4;
        let x: Vec<f32> = (0..n).map(|i| (i as f32 * 0.23).sin() + 0.1).collect();
        let mut ac = [0.0f32; 5];
        celt_autocorr(&x, &mut ac, n, lag);
        for k in 0..=lag {
            let want: f32 = (k..n).map(|i| x[i] * x[i - k]).sum();
            assert!((ac[k] - want).abs() < 1e-3, "lag {k}: {} vs {want}", ac[k]);
        }
        // Lag 0 is the energy and dominates.
        assert!(ac[0] >= ac[1].abs());
    }

    #[test]
    fn lpc_recovers_a_first_order_predictor() {
        // The autocorrelation of an AR(1) process is r[k] = rho^k. Levinson-Durbin
        // must then recover the whitening filter A(z) = 1 - rho z^-1, i.e.
        // lpc[0] = -rho and the higher orders ~0.
        let rho = 0.8f32;
        let ac: Vec<f32> = (0..=4).map(|k| rho.powi(k)).collect();
        let mut lpc = [0.0f32; 4];
        celt_lpc(&mut lpc, &ac, 4);
        assert!(
            (lpc[0] + rho).abs() < 1e-4,
            "lpc[0] = {} (want {})",
            lpc[0],
            -rho
        );
        for (i, &c) in lpc.iter().enumerate().skip(1) {
            assert!(c.abs() < 1e-4, "lpc[{i}] = {c} not ~0");
        }
    }

    #[test]
    fn fir5_impulse_response_is_the_taps() {
        // Feeding a unit impulse through the 5-tap FIR yields [1, num0..num4].
        let num = [0.5f32, -0.25, 0.125, 0.1, -0.05];
        let mut x = vec![0.0f32; 8];
        x[0] = 1.0;
        celt_fir5(&mut x, &num, 8);
        let want = [1.0, num[0], num[1], num[2], num[3], num[4], 0.0, 0.0];
        for (i, (&g, &w)) in x.iter().zip(&want).enumerate() {
            assert!((g - w).abs() < 1e-6, "tap {i}: {g} vs {w}");
        }
    }

    #[test]
    fn pitch_downsample_halves_length_and_whitens() {
        // A smooth, strongly low-pass signal has high lag-1 correlation; after
        // decimation + LPC whitening the normalised lag-1 correlation must drop.
        let len = 512;
        let raw: Vec<f32> = (0..len)
            .map(|i| (i as f32 * 0.03).sin() + 0.5 * (i as f32 * 0.012).sin())
            .collect();
        let half = len >> 1;

        // Plain decimation (the pre-whitening reference): lag-1 correlation.
        let dec: Vec<f32> = (0..half).map(|i| raw[2 * i]).collect();
        let norm_lag1 = |s: &[f32]| {
            let e: f32 = s.iter().map(|v| v * v).sum();
            let c: f32 = (1..s.len()).map(|i| s[i] * s[i - 1]).sum();
            c / e.max(1e-9)
        };
        let before = norm_lag1(&dec);

        let mut x_lp = vec![0.0f32; half];
        let chans: [&[f32]; 1] = [&raw];
        pitch_downsample(&chans, &mut x_lp, len, 1);
        let after = norm_lag1(&x_lp);

        assert!(
            after.abs() < before.abs(),
            "whitening did not reduce lag-1 correlation: {after} vs {before}"
        );
    }

    #[test]
    fn remove_doubling_corrects_an_octave_error() {
        // A pure tone at full-rate period 30 (half-rate 15) correlates equally at
        // every multiple of its period. Seeded with the doubled lag (60), the
        // search must pull the estimate back to the fundamental (~30).
        let maxperiod = 256usize;
        let minperiod = COMBFILTER_MINPERIOD;
        let n = 256usize;
        let half_period = 15.0f32; // half-rate samples per cycle
        let head = maxperiod / 2; // 128
        let buf_len = head + n / 2; // 256
                                    // x[head + j] is analysis sample j; negative-index history is the tone too.
        let x: Vec<f32> = (0..buf_len)
            .map(|i| {
                let t = i as f32 - head as f32;
                (t / half_period * std::f32::consts::TAU).sin()
            })
            .collect();

        let mut t0 = 60i32; // doubled (octave-too-low) estimate
        let pg = remove_doubling(&x, maxperiod, minperiod, n, &mut t0, 0, 0.0);
        assert!(
            (t0 - 30).abs() <= 2,
            "octave not corrected: full-rate lag {t0} (want ~30)"
        );
        assert!(pg > 0.5, "pitch gain too low for a pure tone: {pg}");
    }

    /// Build a `cc`-plane `in` buffer (`cc * (n + overlap)`) whose frame region
    /// is a harmonically rich tone of full-rate period `period` at PCM scale (a
    /// pure sine would be annihilated by the LPC whitening; CELT runs on
    /// broadband, pulse-like signals). The overlap prefix is left at zero.
    fn periodic_in(n: usize, overlap: usize, cc: usize, period: f32) -> Vec<f32> {
        let stride = n + overlap;
        let mut buf = vec![0.0f32; cc * stride];
        for c in 0..cc {
            for i in 0..n {
                let phase = i as f32 / period * std::f32::consts::TAU;
                // A sawtooth-like sum of harmonics keeps a strong pitch pulse in
                // the LPC residual.
                let s: f32 = (1..=8).map(|h| (phase * h as f32).sin() / h as f32).sum();
                buf[c * stride + overlap + i] = 3000.0 * s;
            }
        }
        buf
    }

    #[test]
    fn run_prefilter_detects_a_periodic_frame() {
        // A strongly periodic frame should enable the post-filter with a pitch
        // near the true period and a valid quantised gain.
        let n = 480usize;
        let overlap = 120usize;
        let short = 120usize;
        let period = 64.0f32;
        let window: Vec<f32> = (0..overlap)
            .map(|i| ((i as f32 + 0.5) / overlap as f32 * std::f32::consts::FRAC_PI_2).sin())
            .collect();
        let mut in_buf = periodic_in(n, overlap, 1, period);
        let mut st = PrefilterState::new(1, overlap);
        let pf = run_prefilter(
            &mut in_buf,
            n,
            1,
            overlap,
            short,
            &window,
            0,
            true,
            100,
            &mut st,
        );

        assert!(pf.pf_on, "post-filter should engage on a periodic frame");
        // The detected period should be a (sub-)multiple of the true period.
        let r = (pf.pitch_index as f32 / period).round();
        assert!(
            r >= 1.0 && (pf.pitch_index as f32 - r * period).abs() < 8.0,
            "pitch {} not near a multiple of {period}",
            pf.pitch_index
        );
        assert!((0..=7).contains(&pf.qg), "qg out of range: {}", pf.qg);
        assert!(
            (pf.gain - 0.09375 * (pf.qg + 1) as f32).abs() < 1e-6,
            "gain not the dequantised qg"
        );
        // State carried for the next frame.
        assert_eq!(st.prefilter_period, pf.pitch_index);
        assert_eq!(st.prefilter_gain, pf.gain);
    }

    #[test]
    fn run_prefilter_disables_on_silence_and_is_a_passthrough() {
        // A silent frame has no pitch: the filter stays off and (gain 0) the
        // frame passes through unchanged.
        let n = 480usize;
        let overlap = 120usize;
        let short = 120usize;
        let window = vec![0.0f32; overlap];
        let mut in_buf = vec![0.0f32; n + overlap];
        // A deterministic non-zero frame so "unchanged" is meaningful.
        for i in 0..n {
            in_buf[overlap + i] = ((i * 7 % 13) as f32 - 6.0) * 0.01;
        }
        let original = in_buf.clone();
        let mut st = PrefilterState::new(1, overlap);
        let pf = run_prefilter(
            &mut in_buf,
            n,
            1,
            overlap,
            short,
            &window,
            0,
            true,
            100,
            &mut st,
        );
        assert!(
            !pf.pf_on,
            "post-filter should be off for an aperiodic frame"
        );
        assert_eq!(pf.gain, 0.0);
        // gain0 (prev) and gain1 are both 0 on the first frame -> copy.
        assert_eq!(
            &in_buf[overlap..overlap + n],
            &original[overlap..overlap + n],
            "zero-gain prefilter must be a passthrough"
        );
    }

    #[test]
    fn postfilter_params_round_trip_bit_exact() {
        // Encode a populated post-filter header and decode it back unchanged.
        let total_bits = 1000i32;
        for (pitch_index, qg, tapset) in [(80, 5, 0usize), (15, 0, 2), (1000, 7, 1)] {
            let pf = PostfilterParams {
                pf_on: true,
                pitch_index,
                gain: 0.09375 * (qg + 1) as f32,
                qg,
                tapset,
            };
            let mut enc = RangeEncoder::new(64);
            enc.enc_bit_logp(false, 15); // a leading silence flag, as in a real frame
            encode_postfilter(&mut enc, &pf, 0, total_bits);
            let bytes = enc.done();

            let mut dec = RangeDecoder::new(&bytes);
            assert!(!dec.dec_bit_logp(15));
            let got = decode_postfilter(&mut dec, 0, total_bits).expect("post-filter on");
            assert_eq!(got.pitch_index, pitch_index, "pitch mismatch");
            assert_eq!(got.qg, qg, "qg mismatch");
            assert_eq!(got.tapset, tapset, "tapset mismatch");
            assert!((got.gain - pf.gain).abs() < 1e-9, "gain mismatch");
        }
    }

    #[test]
    fn postfilter_off_round_trips() {
        let total_bits = 1000i32;
        let pf = PostfilterParams {
            pf_on: false,
            pitch_index: COMBFILTER_MINPERIOD as i32,
            gain: 0.0,
            qg: 0,
            tapset: 0,
        };
        let mut enc = RangeEncoder::new(64);
        enc.enc_bit_logp(false, 15);
        encode_postfilter(&mut enc, &pf, 0, total_bits);
        let bytes = enc.done();

        let mut dec = RangeDecoder::new(&bytes);
        assert!(!dec.dec_bit_logp(15));
        assert!(
            decode_postfilter(&mut dec, 0, total_bits).is_none(),
            "off header must decode to None"
        );
    }
}
