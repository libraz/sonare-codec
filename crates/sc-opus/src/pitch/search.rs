use super::*;

/// `celt_inner_prod`: the dot product `Σ_{j<len} x[j]·y[j]`.
pub(crate) fn inner_prod(x: &[f32], y: &[f32], len: usize) -> f32 {
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
pub(crate) fn find_best_pitch(
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
pub(crate) fn celt_autocorr(x: &[f32], ac: &mut [f32], n: usize, lag: usize) {
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
pub(crate) fn celt_lpc(lpc: &mut [f32], ac: &[f32], p: usize) {
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
pub(crate) fn celt_fir5(x: &mut [f32], num: &[f32; 5], nn: usize) {
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
