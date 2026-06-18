//! CELT encoder analysis.
//!
//! Hand-ported to safe Rust from the float build of libopus `celt/celt_encoder.c`
//! (`alloc_trim_analysis`, `stereo_analysis`): the encoder-only heuristics that
//! pick the allocation trim (the tilt of the bit-allocation curve) and decide
//! whether a stereo frame is better coded mid/side. Derivative work of libopus
//! (BSD-3-Clause); see `LICENSE-THIRDPARTY`.
//!
//! These produce integer/boolean decisions consumed by the bit allocator and the
//! stereo coder; the tests pin determinism and the documented monotonic
//! behaviour (e.g. spectral tilt lowers the trim).

// Consumed by the CELT encode entry point; the live encoder still ships via the
// Opus FFI path.
#![allow(dead_code)]

use crate::bands::celt_exp2;
use crate::quant_bands::celt_log2;

/// The per-band log-energy mean removed before quantisation (`eMeans`, libopus
/// `quant_bands.c`); the dynalloc noise floor adds it back.
#[allow(clippy::excessive_precision)]
#[rustfmt::skip]
const E_MEANS: [f32; 25] = [
    6.437500, 6.250000, 5.750000, 5.312500, 5.062500,
    4.812500, 4.500000, 4.375000, 4.875000, 4.687500,
    4.562500, 4.437500, 4.875000, 4.625000, 4.312500,
    4.500000, 4.375000, 4.625000, 4.750000, 4.437500,
    3.750000, 3.750000, 3.750000, 3.750000, 3.750000,
];

/// Bands above this index get no tonality leak boost (libopus `LEAK_BANDS`).
const LEAK_BANDS: usize = 19;

/// `celt_inner_prod`: the dot product of two equal-length slices.
fn celt_inner_prod(x: &[f32], y: &[f32], n: usize) -> f32 {
    let mut sum = 0.0f32;
    for (a, b) in x.iter().zip(y).take(n) {
        sum += a * b;
    }
    sum
}

/// The per-frame inputs `alloc_trim_analysis` reads from earlier analysis stages
/// (the tonality analysis is optional; pass `valid = false` when absent).
pub struct TrimAnalysis {
    pub tf_estimate: f32,
    pub surround_trim: f32,
    pub equiv_rate: i32,
    pub intensity: usize,
    pub analysis_valid: bool,
    pub tonality_slope: f32,
}

/// `alloc_trim_analysis`: computes the allocation trim index (0..=10) from the
/// spectral tilt, the stereo correlation and the rate. `stereo_saving` is the
/// running mid/side savings estimate, updated in place.
#[allow(clippy::too_many_arguments)]
pub fn alloc_trim_analysis(
    e_bands: &[i16],
    nb_e_bands: usize,
    x: &[f32],
    band_log_e: &[f32],
    end: usize,
    lm: i32,
    c: usize,
    n0: usize,
    stereo_saving: &mut f32,
    info: &TrimAnalysis,
) -> i32 {
    let mut trim = 5.0f32;
    // At low bitrate, reducing the trim helps.
    if info.equiv_rate < 64000 {
        trim = 4.0;
    } else if info.equiv_rate < 80000 {
        let frac = (info.equiv_rate - 64000) >> 10;
        trim = 4.0 + (1.0 / 16.0) * frac as f32;
    }

    if c == 2 {
        // Inter-channel correlation for low frequencies.
        let band = |i: usize| (e_bands[i] as usize) << lm;
        let blen = |i: usize| ((e_bands[i + 1] - e_bands[i]) as usize) << lm;
        let mut sum = 0.0f32;
        for i in 0..8 {
            sum += celt_inner_prod(&x[band(i)..], &x[n0 + band(i)..], blen(i));
        }
        sum *= 1.0 / 8.0;
        sum = 1.0f32.min(sum.abs());
        let mut min_xc = sum;
        for i in 8..info.intensity {
            let partial = celt_inner_prod(&x[band(i)..], &x[n0 + band(i)..], blen(i));
            min_xc = min_xc.min(partial.abs());
        }
        min_xc = 1.0f32.min(min_xc.abs());
        // Mid/side savings from the LF average and the min correlation.
        let log_xc = celt_log2(1.001 - sum * sum);
        let log_xc2 = (0.5 * log_xc).max(celt_log2(1.001 - min_xc * min_xc));
        trim += (-4.0f32).max(0.75 * log_xc);
        *stereo_saving = (*stereo_saving + 0.25).min(-0.5 * log_xc2);
    }

    // Estimate spectral tilt.
    let mut diff = 0.0f32;
    for ch in 0..c {
        for i in 0..end - 1 {
            diff += band_log_e[i + ch * nb_e_bands] * (2 + 2 * i as i32 - end as i32) as f32;
        }
    }
    diff /= (c * (end - 1)) as f32;
    trim -= (-2.0f32).max(2.0f32.min((diff + 1.0) / 6.0));
    trim -= info.surround_trim;
    trim -= 2.0 * info.tf_estimate;
    if info.analysis_valid {
        trim -= (-2.0f32).max(2.0f32.min(2.0 * (info.tonality_slope + 0.05)));
    }

    let trim_index = (0.5 + trim).floor() as i32;
    trim_index.clamp(0, 10)
}

/// The outcome of [`transient_analysis`].
pub struct TransientResult {
    /// Whether the frame should use short (transient) MDCT blocks.
    pub is_transient: bool,
    /// VBR boost estimate fed to the trim/rate logic.
    pub tf_estimate: f32,
    /// The channel that drove the decision (the most transient one).
    pub tf_chan: usize,
    /// A low-bitrate "weak transient" that is handled without short blocks.
    pub weak_transient: bool,
}

/// `6*64/x` lookup, trained to minimise the harmonic-mean error (libopus).
#[rustfmt::skip]
const TRANSIENT_INV_TABLE: [u8; 128] = [
    255, 255, 156, 110, 86, 70, 59, 51, 45, 40, 37, 33, 31, 28, 26, 25,
    23, 22, 21, 20, 19, 18, 17, 16, 16, 15, 15, 14, 13, 13, 12, 12,
    12, 12, 11, 11, 11, 10, 10, 10, 9, 9, 9, 9, 9, 9, 8, 8,
    8, 8, 8, 7, 7, 7, 7, 7, 7, 6, 6, 6, 6, 6, 6, 6,
    6, 6, 6, 6, 6, 6, 6, 6, 6, 5, 5, 5, 5, 5, 5, 5,
    5, 5, 5, 5, 5, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4,
    4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 3, 3,
    3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 2,
];

/// `transient_analysis`: decides whether a frame contains a transient (so the
/// encoder should switch to short MDCT blocks), and produces the VBR `tf_estimate`.
///
/// Hand-ported to safe Rust from the float build of libopus `celt/celt_encoder.c`
/// (`transient_analysis`). Derivative work of libopus (BSD-3-Clause); see
/// `LICENSE-THIRDPARTY`. `input` holds `c_channels` regions of `len` time-domain
/// samples. The detector high-pass filters each channel, builds a forward
/// (post-echo) and backward (pre-echo) masking envelope, and compares the frame
/// energy to the harmonic mean of the envelope — a temporal noise-to-mask ratio.
pub fn transient_analysis(
    input: &[f32],
    len: usize,
    c_channels: usize,
    allow_weak_transients: bool,
) -> TransientResult {
    const EPSILON: f32 = 1e-15;
    // Forward masking decay: 6.7 dB/ms normally, 3.3 dB/ms when weak transients
    // are allowed (low bitrate, where coding transients can collapse energy).
    let forward_decay = if allow_weak_transients {
        0.03125f32
    } else {
        0.0625f32
    };
    let len2 = len / 2;
    let mut tmp = vec![0.0f32; len];
    let mut mask_metric: i64 = 0;
    let mut tf_chan = 0usize;

    for c in 0..c_channels {
        // High-pass filter: (1 - 2 z^-1 + z^-2) / (1 - z^-1 + 0.5 z^-2).
        let (mut mem0, mut mem1) = (0.0f32, 0.0f32);
        for i in 0..len {
            let x = input[i + c * len];
            let y = mem0 + x;
            mem0 = mem1 + y - 2.0 * x;
            mem1 = x - 0.5 * y;
            tmp[i] = y;
        }
        // The first samples are unreliable (memory not propagated yet).
        for v in tmp.iter_mut().take(12.min(len)) {
            *v = 0.0;
        }

        // Forward pass: post-echo threshold, grouping samples by two.
        let mut mean = 0.0f32;
        mem0 = 0.0;
        for i in 0..len2 {
            let x2 = tmp[2 * i] * tmp[2 * i] + tmp[2 * i + 1] * tmp[2 * i + 1];
            mean += x2;
            tmp[i] = mem0 + forward_decay * (x2 - mem0);
            mem0 = tmp[i];
        }

        // Backward pass: pre-echo threshold (backward masking, 13.9 dB/ms).
        mem0 = 0.0;
        let mut max_e = 0.0f32;
        for i in (0..len2).rev() {
            tmp[i] = mem0 + 0.125 * (tmp[i] - mem0);
            mem0 = tmp[i];
            max_e = max_e.max(mem0);
        }

        // Frame energy as the geometric mean of the energy and half the max.
        let mean = (mean * max_e * 0.5 * len2 as f32).sqrt();
        // Inverse mean energy (the fixed-point Q-shifts are identities in float).
        let norm = len2 as f32 / (EPSILON + mean);
        // Harmonic mean over the reliable, smooth interior (every 4th sample).
        let mut unmask: i64 = 0;
        let mut i = 12;
        while i + 5 < len2 {
            let id = (64.0 * norm * (tmp[i] + EPSILON)).floor().clamp(0.0, 127.0) as usize;
            unmask += i64::from(TRANSIENT_INV_TABLE[id]);
            i += 4;
        }
        if len2 > 17 {
            // Normalise: compensate the 1/4 sampling and the factor of 6 in the table.
            let unmask = 64 * unmask * 4 / (6 * (len2 as i64 - 17));
            if unmask > mask_metric {
                tf_chan = c;
                mask_metric = unmask;
            }
        }
    }

    let mut is_transient = mask_metric > 200;
    let mut weak_transient = false;
    if allow_weak_transients && is_transient && mask_metric < 600 {
        is_transient = false;
        weak_transient = true;
    }
    // Arbitrary metrics for VBR boost.
    let tf_max = 0.0f32.max((27.0 * mask_metric as f32).sqrt() - 42.0);
    let tf_estimate = 0.0f32.max(0.0069 * 163.0f32.min(tf_max) - 0.139).sqrt();

    TransientResult {
        is_transient,
        tf_estimate,
        tf_chan,
        weak_transient,
    }
}

/// `stereo_analysis`: returns `true` when the frame is better coded as separate
/// left/right ("dual stereo") rather than joint mid/side, comparing the L1 norm
/// of the two representations.
pub fn stereo_analysis(e_bands: &[i16], x: &[f32], lm: i32, n0: usize) -> bool {
    let mut sum_lr = 1e-15f32;
    let mut sum_ms = 1e-15f32;
    // L1 norm models the entropy of the L/R vs M/S signal.
    for i in 0..13 {
        for j in (e_bands[i] as usize) << lm..(e_bands[i + 1] as usize) << lm {
            let l = x[j];
            let r = x[n0 + j];
            sum_lr += l.abs() + r.abs();
            sum_ms += (l + r).abs() + (l - r).abs();
        }
    }
    sum_ms *= core::f32::consts::FRAC_1_SQRT_2; // 0.707107
                                                // thetas: per-band overhead; we don't need them for lower bands at LM<=1.
    let mut thetas = 13;
    if lm <= 1 {
        thetas -= 8;
    }
    let big_n = ((e_bands[13] as usize) << (lm + 1)) as f32;
    (big_n + thetas as f32) * sum_ms > big_n * sum_lr
}

/// `median_of_5`: the median of five samples via the libopus comparison network.
fn median_of_5(x: &[f32]) -> f32 {
    let t2 = x[2];
    let (mut t0, mut t1) = if x[0] > x[1] {
        (x[1], x[0])
    } else {
        (x[0], x[1])
    };
    let (mut t3, mut t4) = if x[3] > x[4] {
        (x[4], x[3])
    } else {
        (x[3], x[4])
    };
    if t0 > t3 {
        core::mem::swap(&mut t0, &mut t3);
        core::mem::swap(&mut t1, &mut t4);
    }
    if t2 > t1 {
        if t1 < t3 {
            t2.min(t3)
        } else {
            t4.min(t1)
        }
    } else if t2 < t3 {
        t1.min(t3)
    } else {
        t2.min(t4)
    }
}

/// `median_of_3`: the median of three samples.
fn median_of_3(x: &[f32]) -> f32 {
    let (t0, t1) = if x[0] > x[1] {
        (x[1], x[0])
    } else {
        (x[0], x[1])
    };
    let t2 = x[2];
    if t1 < t2 {
        t1
    } else if t0 < t2 {
        t2
    } else {
        t0
    }
}

/// The per-band outputs and the frame depth that [`dynalloc_analysis`] produces.
pub struct DynallocResult {
    /// Maximum coding depth across the frame (drives the bit-allocation cap).
    pub max_depth: f32,
    /// Total dynalloc boost in eighth-bits.
    pub tot_boost: i32,
}

/// `dynalloc_analysis`: per-band dynamic bit-allocation boosts, the band
/// `importance` weights (used by [`tf_analysis`] and rate control) and the
/// `spread_weight` masking weights, plus the frame's maximum coding depth.
///
/// Hand-ported to safe Rust from the float build of libopus `celt/celt_encoder.c`
/// (`dynalloc_analysis`). Derivative work of libopus (BSD-3-Clause); see
/// `LICENSE-THIRDPARTY`. It builds a noise floor and a simple spreading mask from
/// the band energies, follows the per-band energy with a median-filtered
/// envelope, and converts the excess over the follower into per-band boosts,
/// bounded so dynalloc can never bust the frame budget.
#[allow(clippy::too_many_arguments)]
pub fn dynalloc_analysis(
    band_log_e: &[f32],
    band_log_e2: &[f32],
    nb_e_bands: usize,
    start: usize,
    end: usize,
    c_channels: usize,
    offsets: &mut [i32],
    lsb_depth: i32,
    log_n: &[i16],
    is_transient: bool,
    vbr: bool,
    constrained_vbr: bool,
    e_bands: &[i16],
    lm: i32,
    effective_bytes: i32,
    lfe: bool,
    surround_dynalloc: &[f32],
    analysis_leak_boost: Option<&[f32]>,
    importance: &mut [i32],
    spread_weight: &mut [i32],
) -> DynallocResult {
    const BITRES: i32 = 3;
    let c = c_channels;
    let mut tot_boost = 0i32;
    let mut max_depth = -31.9f32;

    for v in offsets.iter_mut().take(nb_e_bands) {
        *v = 0;
    }

    // Noise floor: eMeans, depth, band width and the preemphasis tilt (~bark^2).
    let mut noise_floor = vec![0.0f32; nb_e_bands];
    for i in 0..end {
        noise_floor[i] = 0.0625 * f32::from(log_n[i]) + 0.5 + (9 - lsb_depth) as f32 - E_MEANS[i]
            + 0.0062 * ((i + 5) * (i + 5)) as f32;
    }
    for ch in 0..c {
        for i in 0..end {
            max_depth = max_depth.max(band_log_e[ch * nb_e_bands + i] - noise_floor[i]);
        }
    }

    // Simple masking model -> per-band spreading weight.
    {
        let mut mask = vec![0.0f32; nb_e_bands];
        for i in 0..end {
            mask[i] = band_log_e[i] - noise_floor[i];
        }
        if c == 2 {
            for i in 0..end {
                mask[i] = mask[i].max(band_log_e[nb_e_bands + i] - noise_floor[i]);
            }
        }
        let sig: Vec<f32> = mask[..end].to_vec();
        for i in 1..end {
            mask[i] = mask[i].max(mask[i - 1] - 2.0);
        }
        for i in (0..end - 1).rev() {
            mask[i] = mask[i].max(mask[i + 1] - 3.0);
        }
        for i in 0..end {
            // SMR: at most 72 dB below the peak, never below the noise floor.
            let smr = sig[i] - 0.0f32.max(max_depth - 12.0).max(mask[i]);
            let shift = (-(0.5 + smr).floor() as i32).clamp(0, 5);
            spread_weight[i] = 32 >> shift;
        }
    }

    if effective_bytes > 50 && lm >= 1 && !lfe {
        let mut follower = vec![0.0f32; c * nb_e_bands];
        for ch in 0..c {
            let base = ch * nb_e_bands;
            let mut last = 0usize;
            follower[base] = band_log_e2[base];
            for i in 1..end {
                if band_log_e2[base + i] > band_log_e2[base + i - 1] + 0.5 {
                    last = i;
                }
                follower[base + i] = (follower[base + i - 1] + 1.5).min(band_log_e2[base + i]);
            }
            for i in (0..last).rev() {
                follower[base + i] = follower[base + i]
                    .min((follower[base + i + 1] + 2.0).min(band_log_e2[base + i]));
            }
            // Median filter so dynalloc doesn't fire on noise; offset is the
            // conservativeness knob.
            let offset = 1.0f32;
            for i in 2..end - 2 {
                follower[base + i] =
                    follower[base + i].max(median_of_5(&band_log_e2[base + i - 2..]) - offset);
            }
            let tmp = median_of_3(&band_log_e2[base..]) - offset;
            follower[base] = follower[base].max(tmp);
            follower[base + 1] = follower[base + 1].max(tmp);
            let tmp = median_of_3(&band_log_e2[base + end - 3..]) - offset;
            follower[base + end - 2] = follower[base + end - 2].max(tmp);
            follower[base + end - 1] = follower[base + end - 1].max(tmp);
            for i in 0..end {
                follower[base + i] = follower[base + i].max(noise_floor[i]);
            }
        }
        if c == 2 {
            for i in start..end {
                // 24 dB cross-talk between channels, then the masked excess.
                follower[nb_e_bands + i] = follower[nb_e_bands + i].max(follower[i] - 4.0);
                follower[i] = follower[i].max(follower[nb_e_bands + i] - 4.0);
                follower[i] = 0.5
                    * (0.0f32.max(band_log_e[i] - follower[i])
                        + 0.0f32.max(band_log_e[nb_e_bands + i] - follower[nb_e_bands + i]));
            }
        } else {
            for i in start..end {
                follower[i] = 0.0f32.max(band_log_e[i] - follower[i]);
            }
        }
        for (f, &s) in follower[start..end]
            .iter_mut()
            .zip(&surround_dynalloc[start..end])
        {
            *f = f.max(s);
        }
        for i in start..end {
            importance[i] = (0.5 + 13.0 * celt_exp2(follower[i].min(4.0))).floor() as i32;
        }
        // For non-transient CBR/CVBR frames, halve the dynalloc contribution.
        if (!vbr || constrained_vbr) && !is_transient {
            for f in follower[start..end].iter_mut() {
                *f *= 0.5;
            }
        }
        for (i, f) in follower.iter_mut().enumerate().take(end).skip(start) {
            if i < 8 {
                *f *= 2.0;
            }
            if i >= 12 {
                *f *= 0.5;
            }
        }
        if let Some(leak) = analysis_leak_boost {
            for i in start..LEAK_BANDS.min(end) {
                follower[i] += (1.0 / 64.0) * leak[i];
            }
        }
        for i in start..end {
            follower[i] = follower[i].min(4.0);
            let width = (c as i32 * (e_bands[i + 1] - e_bands[i]) as i32) << lm;
            let (boost, boost_bits) = if width < 6 {
                let boost = follower[i] as i32;
                (boost, (boost * width) << BITRES)
            } else if width > 48 {
                let boost = (follower[i] * 8.0) as i32;
                (boost, ((boost * width) << BITRES) / 8)
            } else {
                let boost = (follower[i] * width as f32 / 6.0) as i32;
                (boost, (boost * 6) << BITRES)
            };
            // For CBR / non-transient CVBR, cap dynalloc at 2/3 of the bits.
            if (!vbr || (constrained_vbr && !is_transient))
                && (((tot_boost + boost_bits) >> BITRES) >> 3) > 2 * effective_bytes / 3
            {
                let cap = (2 * effective_bytes / 3) << BITRES << 3;
                offsets[i] = cap - tot_boost;
                tot_boost = cap;
                break;
            }
            offsets[i] = boost;
            tot_boost += boost_bits;
        }
    } else {
        for v in importance.iter_mut().take(end).skip(start) {
            *v = 13;
        }
    }

    DynallocResult {
        max_depth,
        tot_boost,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EBAND5MS: [i16; 22] = [
        0, 1, 2, 3, 4, 5, 6, 7, 8, 10, 12, 14, 16, 20, 24, 28, 34, 40, 48, 60, 78, 100,
    ];
    const NB_E_BANDS: usize = 21;
    const LOG_N: [i16; 21] = [
        0, 0, 0, 0, 0, 0, 0, 0, 8, 8, 8, 8, 16, 16, 16, 21, 21, 24, 29, 34, 36,
    ];

    #[allow(clippy::too_many_arguments)]
    fn run_dynalloc(
        ble: &[f32],
        ble2: &[f32],
        c: usize,
        effective_bytes: i32,
        vbr: bool,
    ) -> (DynallocResult, Vec<i32>, Vec<i32>, Vec<i32>) {
        let mut offsets = vec![0i32; NB_E_BANDS];
        let mut importance = vec![0i32; NB_E_BANDS];
        let mut spread = vec![0i32; NB_E_BANDS];
        let surround = vec![0.0f32; NB_E_BANDS];
        let res = dynalloc_analysis(
            ble,
            ble2,
            NB_E_BANDS,
            0,
            NB_E_BANDS,
            c,
            &mut offsets,
            24,
            &LOG_N,
            false,
            vbr,
            false,
            &EBAND5MS,
            3,
            effective_bytes,
            false,
            &surround,
            None,
            &mut importance,
            &mut spread,
        );
        (res, offsets, importance, spread)
    }

    #[test]
    fn dynalloc_boosts_tonal_peak_not_flat_bands() {
        // A single band well above its neighbours should attract a dynalloc boost
        // and a higher importance; a flat band gets neither.
        let mut ble = vec![0.0f32; NB_E_BANDS];
        ble[15] = 10.0;
        let (_res, offsets, importance, spread) = run_dynalloc(&ble, &ble, 1, 200, true);
        assert!(offsets[15] > 0, "tonal peak not boosted: {}", offsets[15]);
        assert_eq!(offsets[3], 0, "flat band wrongly boosted");
        assert!(
            importance[15] > importance[3],
            "peak importance {} should exceed flat {}",
            importance[15],
            importance[3]
        );
        // spread_weight is always 32 >> shift with shift in 0..=5.
        for &w in &spread {
            assert!(
                [1, 2, 4, 8, 16, 32].contains(&w),
                "spread_weight {w} not a 32>>shift value"
            );
        }
    }

    #[test]
    fn dynalloc_small_frame_uses_flat_importance() {
        // effective_bytes <= 50 disables dynalloc: importance is the flat 13,
        // no offsets, no boost.
        let ble = vec![1.0f32; NB_E_BANDS];
        let (res, offsets, importance, _spread) = run_dynalloc(&ble, &ble, 1, 40, true);
        assert!(importance.iter().all(|&v| v == 13));
        assert!(offsets.iter().all(|&v| v == 0));
        assert_eq!(res.tot_boost, 0);
    }

    #[test]
    fn dynalloc_is_deterministic() {
        let ble: Vec<f32> = (0..NB_E_BANDS)
            .map(|i| (i as f32 * 0.37).sin() * 3.0)
            .collect();
        let (r1, o1, i1, s1) = run_dynalloc(&ble, &ble, 1, 200, true);
        let (r2, o2, i2, s2) = run_dynalloc(&ble, &ble, 1, 200, true);
        assert_eq!(o1, o2);
        assert_eq!(i1, i2);
        assert_eq!(s1, s2);
        assert_eq!(r1.tot_boost, r2.tot_boost);
        assert_eq!(r1.max_depth.to_bits(), r2.max_depth.to_bits());
    }

    fn info(equiv_rate: i32) -> TrimAnalysis {
        TrimAnalysis {
            tf_estimate: 0.0,
            surround_trim: 0.0,
            equiv_rate,
            intensity: NB_E_BANDS,
            analysis_valid: false,
            tonality_slope: 0.0,
        }
    }

    #[test]
    fn trim_is_clamped_and_deterministic() {
        let n0 = 8 * 120;
        let x = vec![0.01f32; n0];
        let ble = vec![1.0f32; NB_E_BANDS];
        let mut ss = 0.0;
        let t1 = alloc_trim_analysis(
            &EBAND5MS,
            NB_E_BANDS,
            &x,
            &ble,
            NB_E_BANDS,
            3,
            1,
            n0,
            &mut ss,
            &info(96000),
        );
        let mut ss2 = 0.0;
        let t2 = alloc_trim_analysis(
            &EBAND5MS,
            NB_E_BANDS,
            &x,
            &ble,
            NB_E_BANDS,
            3,
            1,
            n0,
            &mut ss2,
            &info(96000),
        );
        assert_eq!(t1, t2, "deterministic");
        assert!((0..=10).contains(&t1), "trim {t1} out of range");
    }

    #[test]
    fn low_rate_lowers_base_trim() {
        let n0 = 8 * 120;
        let x = vec![0.0f32; n0];
        let ble = vec![0.0f32; NB_E_BANDS];
        let mut ss = 0.0;
        // Flat spectrum, no tilt: trim equals the rate-dependent base.
        let hi = alloc_trim_analysis(
            &EBAND5MS,
            NB_E_BANDS,
            &x,
            &ble,
            NB_E_BANDS,
            3,
            1,
            n0,
            &mut ss,
            &info(96000),
        );
        let mut ss2 = 0.0;
        let lo = alloc_trim_analysis(
            &EBAND5MS,
            NB_E_BANDS,
            &x,
            &ble,
            NB_E_BANDS,
            3,
            1,
            n0,
            &mut ss2,
            &info(32000),
        );
        assert!(hi >= lo, "low rate {lo} should not exceed high rate {hi}");
        assert_eq!(hi, 5, "flat 96k base trim");
        assert_eq!(lo, 4, "flat 32k base trim");
    }

    #[test]
    fn rising_spectrum_lowers_trim_vs_falling() {
        let n0 = 8 * 120;
        let x = vec![0.0f32; n0];
        // Falling energy (more LF): positive tilt term -> lower trim.
        let falling: Vec<f32> = (0..NB_E_BANDS).map(|i| 4.0 - 0.3 * i as f32).collect();
        let rising: Vec<f32> = (0..NB_E_BANDS).map(|i| 0.3 * i as f32).collect();
        let mut ss = 0.0;
        let t_fall = alloc_trim_analysis(
            &EBAND5MS,
            NB_E_BANDS,
            &x,
            &falling,
            NB_E_BANDS,
            3,
            1,
            n0,
            &mut ss,
            &info(96000),
        );
        let mut ss2 = 0.0;
        let t_rise = alloc_trim_analysis(
            &EBAND5MS,
            NB_E_BANDS,
            &x,
            &rising,
            NB_E_BANDS,
            3,
            1,
            n0,
            &mut ss2,
            &info(96000),
        );
        // An LF-heavy (falling) spectrum tilts the allocation toward LF: higher
        // trim than an HF-heavy (rising) one.
        assert!(
            t_fall >= t_rise,
            "falling spectrum trim {t_fall} should be >= rising {t_rise}"
        );
    }

    #[test]
    fn transient_analysis_flags_onset_not_steady_tone() {
        let len = 480usize;
        // A steady sinusoid: smooth envelope, no transient.
        let steady: Vec<f32> = (0..len).map(|i| (0.2 * i as f32).sin() * 0.3).collect();
        let r_steady = transient_analysis(&steady, len, 1, false);
        assert!(!r_steady.is_transient, "steady tone flagged transient");

        // Silence then a sudden loud burst: a strong transient.
        let mut onset = vec![0.0f32; len];
        for (i, v) in onset.iter_mut().enumerate().skip(len / 2) {
            *v = ((0.7 * i as f32).sin()) * 0.9;
        }
        let r_onset = transient_analysis(&onset, len, 1, false);
        assert!(r_onset.is_transient, "sharp onset not flagged transient");
        assert!(r_onset.tf_estimate >= 0.0);
    }

    #[test]
    fn transient_analysis_is_deterministic() {
        let len = 480usize;
        let sig: Vec<f32> = (0..len)
            .map(|i| (0.13 * i as f32).sin() * if i > 300 { 0.9 } else { 0.05 })
            .collect();
        let a = transient_analysis(&sig, len, 1, false);
        let b = transient_analysis(&sig, len, 1, false);
        assert_eq!(a.is_transient, b.is_transient);
        assert_eq!(a.tf_chan, b.tf_chan);
        assert_eq!(a.tf_estimate.to_bits(), b.tf_estimate.to_bits());
    }

    #[test]
    fn transient_analysis_weak_transient_path() {
        // allow_weak_transients reclassifies a moderate transient as "weak"
        // (no short blocks) instead of a full transient. Build a mild onset and
        // confirm the weak path is at least reachable and self-consistent.
        let len = 480usize;
        let mut sig = vec![0.0f32; len];
        for (i, v) in sig.iter_mut().enumerate().skip(len / 2) {
            *v = (0.5 * i as f32).sin() * 0.25;
        }
        let r = transient_analysis(&sig, len, 1, true);
        // Either a clean classification or the weak-transient flag, never both.
        assert!(!(r.is_transient && r.weak_transient));
    }

    #[test]
    fn stereo_analysis_picks_ms_for_mono_and_lr_for_panned() {
        let n0 = 8 * 120;
        let span = (EBAND5MS[13] as usize) << 3;
        // Identical channels (L==R): side is zero -> joint M/S (returns false).
        let mut x = vec![0.0f32; 2 * n0];
        for j in 0..span {
            x[j] = ((j as f32) * 0.1).sin();
            x[n0 + j] = x[j];
        }
        assert!(!stereo_analysis(&EBAND5MS, &x, 3, n0), "mono -> joint M/S");

        // Hard-panned (R==0): M/S would waste bits -> dual L/R (returns true).
        let mut x2 = vec![0.0f32; 2 * n0];
        for (j, v) in x2.iter_mut().enumerate().take(span) {
            *v = ((j as f32) * 0.1).sin();
        }
        assert!(stereo_analysis(&EBAND5MS, &x2, 3, n0), "panned -> dual L/R");
    }
}
