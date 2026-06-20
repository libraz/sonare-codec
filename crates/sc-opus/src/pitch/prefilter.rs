use super::*;

/// `compute_pitch_gain` (float path): the normalised correlation
/// `xy / sqrt(1 + xx·yy)`.
pub(crate) fn compute_pitch_gain(xy: f32, xx: f32, yy: f32) -> f32 {
    xy / (1.0 + xx * yy).sqrt()
}

/// `dual_inner_prod`: the two dot products `Σ x·y01` and `Σ x·y02` in one pass.
pub(crate) fn dual_inner_prod(x: &[f32], y01: &[f32], y02: &[f32], n: usize) -> (f32, f32) {
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
