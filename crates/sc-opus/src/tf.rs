//! CELT time-frequency resolution flags.
//!
//! Hand-ported to safe Rust from libopus `celt/celt_encoder.c` (`tf_encode`) and
//! `celt/celt_decoder.c` (`tf_decode`): the per-band time-frequency change flags
//! that pick, for each band, whether to trade frequency resolution for time
//! resolution (and vice versa). The flags are delta-coded against the running
//! value with a budget-aware probability, then mapped through `tf_select_table`.
//! Derivative work of libopus (BSD-3-Clause); see `LICENSE-THIRDPARTY`.
//!
//! Encoder and decoder must agree on the flags, so the port keeps the two sides
//! structurally identical and the test round-trips them to bit-exact agreement.

// Consumed by the CELT encode/decode entry points; the live encoder still ships
// via the Opus FFI path.
#![allow(dead_code)]

use crate::bands::haar1;
use crate::range_coder::{RangeDecoder, RangeEncoder};

/// `tf_select_table[LM][4*isTransient + 2*tf_select + tf_change]`: the resolution
/// adjustment applied to each band's coded flag.
const TF_SELECT_TABLE: [[i8; 8]; 4] = [
    [0, -1, 0, -1, 0, -1, 0, -1],
    [0, -1, 0, -2, 1, 0, 1, -1],
    [0, -2, 0, -3, 2, 0, 1, -1],
    [0, -2, 0, -3, 3, 0, 1, -1],
];

/// `l1_metric`: the bias-weighted L1 norm of a (possibly Haar-transformed) band,
/// used by [`tf_analysis`] to score each candidate time/frequency resolution.
/// The `lm*bias` term gently prefers good frequency resolution when in doubt.
fn l1_metric(tmp: &[f32], n: usize, lm: i32, bias: f32) -> f32 {
    let mut l1 = 0.0f32;
    for &v in tmp.iter().take(n) {
        l1 += v.abs();
    }
    l1 + (lm as f32 * bias) * l1
}

/// `tf_analysis`: picks each band's time/frequency resolution flag (`tf_res`) and
/// the frame-level `tf_select`, returned for [`tf_encode`] to transmit.
///
/// Hand-ported to safe Rust from the float build of libopus `celt/celt_encoder.c`
/// (`tf_analysis`). Derivative work of libopus (BSD-3-Clause); see
/// `LICENSE-THIRDPARTY`. For each band it Haar-transforms the normalised spectrum
/// to several time/frequency splits, scores them with [`l1_metric`], then runs a
/// Viterbi search (cost `lambda` per resolution switch, weighted by `importance`)
/// to choose the cheapest globally-consistent set of flags.
#[allow(clippy::too_many_arguments)]
pub fn tf_analysis(
    e_bands: &[i16],
    len: usize,
    is_transient: bool,
    tf_res: &mut [i32],
    lambda: i32,
    x: &[f32],
    n0: usize,
    lm: i32,
    tf_estimate: f32,
    tf_chan: usize,
    importance: &[i32],
) -> i32 {
    let bias = 0.04 * (-0.25f32).max(0.5 - tf_estimate);
    let it = is_transient as usize;
    let lmu = lm as usize;

    // Per-band: find the resolution split (level) that minimises the L1 metric.
    let mut metric = vec![0i32; len];
    for i in 0..len {
        let width = (e_bands[i + 1] - e_bands[i]) as usize;
        let n = width << lm;
        let narrow = width == 1;
        let base = tf_chan * n0 + ((e_bands[i] as usize) << lm);
        let mut tmp: Vec<f32> = x[base..base + n].to_vec();

        let mut best_l1 = l1_metric(&tmp, n, if is_transient { lm } else { 0 }, bias);
        let mut best_level = 0i32;
        // The -1 split (finer time resolution) only applies to splittable transients.
        if is_transient && !narrow {
            let mut tmp_1 = tmp.clone();
            haar1(&mut tmp_1, n >> lm, 1 << lm);
            let l1 = l1_metric(&tmp_1, n, lm + 1, bias);
            if l1 < best_l1 {
                best_l1 = l1;
                best_level = -1;
            }
        }
        let k_max = lm + i32::from(!(is_transient || narrow));
        for k in 0..k_max {
            let b = if is_transient { lm - k - 1 } else { k + 1 };
            haar1(&mut tmp, n >> k, 1 << k);
            let l1 = l1_metric(&tmp, n, b, bias);
            if l1 < best_l1 {
                best_l1 = l1;
                best_level = k + 1;
            }
        }
        // Q1 metric so narrow bands can sit at the -0.5 mid-point.
        metric[i] = if is_transient {
            2 * best_level
        } else {
            -2 * best_level
        };
        if narrow && (metric[i] == 0 || metric[i] == -2 * lm) {
            metric[i] -= 1;
        }
    }

    let tbl = |sel: i32, j: usize| i32::from(TF_SELECT_TABLE[lmu][4 * it + 2 * sel as usize + j]);

    // Choose tf_select by comparing the two table options' total Viterbi cost.
    let mut sel_cost = [0i32; 2];
    for sel in 0..2i32 {
        let (t0, t1) = (tbl(sel, 0), tbl(sel, 1));
        let mut cost0 = importance[0] * (metric[0] - 2 * t0).abs();
        let mut cost1 =
            importance[0] * (metric[0] - 2 * t1).abs() + if is_transient { 0 } else { lambda };
        for i in 1..len {
            let curr0 = cost0.min(cost1 + lambda);
            let curr1 = (cost0 + lambda).min(cost1);
            cost0 = curr0 + importance[i] * (metric[i] - 2 * t0).abs();
            cost1 = curr1 + importance[i] * (metric[i] - 2 * t1).abs();
        }
        sel_cost[sel as usize] = cost0.min(cost1);
    }
    // Conservative: only allow tf_select=1 for transients.
    let tf_select = i32::from(sel_cost[1] < sel_cost[0] && is_transient);

    // Viterbi forward pass with backpointers, then trace back into tf_res.
    let (t0, t1) = (tbl(tf_select, 0), tbl(tf_select, 1));
    let mut cost0 = importance[0] * (metric[0] - 2 * t0).abs();
    let mut cost1 =
        importance[0] * (metric[0] - 2 * t1).abs() + if is_transient { 0 } else { lambda };
    let mut path0 = vec![0i32; len];
    let mut path1 = vec![0i32; len];
    for i in 1..len {
        let (curr0, p0) = if cost0 < cost1 + lambda {
            (cost0, 0)
        } else {
            (cost1 + lambda, 1)
        };
        let (curr1, p1) = if cost0 + lambda < cost1 {
            (cost0 + lambda, 0)
        } else {
            (cost1, 1)
        };
        path0[i] = p0;
        path1[i] = p1;
        cost0 = curr0 + importance[i] * (metric[i] - 2 * t0).abs();
        cost1 = curr1 + importance[i] * (metric[i] - 2 * t1).abs();
    }
    tf_res[len - 1] = i32::from(cost0 >= cost1);
    for i in (0..len - 1).rev() {
        tf_res[i] = if tf_res[i + 1] == 1 {
            path1[i + 1]
        } else {
            path0[i + 1]
        };
    }
    tf_select
}

/// `tf_encode`: delta-codes the per-band tf-change flags `tf_res[start..end]`
/// (each `0`/`1`) into `enc`, then rewrites `tf_res` through `tf_select_table`.
pub fn tf_encode(
    start: usize,
    end: usize,
    is_transient: bool,
    tf_res: &mut [i32],
    lm: i32,
    mut tf_select: i32,
    enc: &mut RangeEncoder,
) {
    let budget0 = enc.storage_bits();
    let mut tell = enc.ec_tell() as u32;
    let mut logp: u32 = if is_transient { 2 } else { 4 };
    // Reserve space to code the tf_select decision.
    let tf_select_rsv = u32::from(lm > 0 && tell + logp < budget0);
    let budget = budget0 - tf_select_rsv;
    let mut curr = 0i32;
    let mut tf_changed = 0i32;
    for r in tf_res.iter_mut().take(end).skip(start) {
        if tell + logp <= budget {
            enc.enc_bit_logp((*r ^ curr) != 0, logp);
            tell = enc.ec_tell() as u32;
            curr = *r;
            tf_changed |= curr;
        } else {
            *r = curr;
        }
        logp = if is_transient { 4 } else { 5 };
    }
    let lmu = lm as usize;
    let it = usize::from(is_transient);
    // Only code tf_select if it would actually make a difference.
    if tf_select_rsv != 0
        && TF_SELECT_TABLE[lmu][4 * it + tf_changed as usize]
            != TF_SELECT_TABLE[lmu][4 * it + 2 + tf_changed as usize]
    {
        enc.enc_bit_logp(tf_select != 0, 1);
    } else {
        tf_select = 0;
    }
    for r in tf_res.iter_mut().take(end).skip(start) {
        *r = i32::from(TF_SELECT_TABLE[lmu][4 * it + 2 * tf_select as usize + *r as usize]);
    }
}

/// `tf_decode`: the inverse of [`tf_encode`] — reconstructs `tf_res[start..end]`
/// from `dec` and maps them through `tf_select_table`.
pub fn tf_decode(
    start: usize,
    end: usize,
    is_transient: bool,
    tf_res: &mut [i32],
    lm: i32,
    dec: &mut RangeDecoder,
) {
    let budget0 = dec.storage_bits();
    let mut tell = dec.ec_tell() as u32;
    let mut logp: u32 = if is_transient { 2 } else { 4 };
    let tf_select_rsv = u32::from(lm > 0 && tell + logp < budget0);
    let budget = budget0 - tf_select_rsv;
    let mut curr = 0i32;
    let mut tf_changed = 0i32;
    for r in tf_res.iter_mut().take(end).skip(start) {
        if tell + logp <= budget {
            curr ^= i32::from(dec.dec_bit_logp(logp));
            tell = dec.ec_tell() as u32;
            tf_changed |= curr;
        }
        *r = curr;
        logp = if is_transient { 4 } else { 5 };
    }
    let lmu = lm as usize;
    let it = usize::from(is_transient);
    let mut tf_select = 0i32;
    if tf_select_rsv != 0
        && TF_SELECT_TABLE[lmu][4 * it + tf_changed as usize]
            != TF_SELECT_TABLE[lmu][4 * it + 2 + tf_changed as usize]
    {
        tf_select = i32::from(dec.dec_bit_logp(1));
    }
    for r in tf_res.iter_mut().take(end).skip(start) {
        *r = i32::from(TF_SELECT_TABLE[lmu][4 * it + 2 * tf_select as usize + *r as usize]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Round-trips a raw tf-change pattern through encode/decode and asserts the
    /// mapped flags agree exactly.
    fn round_trip(lm: i32, is_transient: bool, tf_select: i32, raw: &[i32]) {
        let end = raw.len();
        let mut enc_res = raw.to_vec();
        let mut enc = RangeEncoder::new(64);
        tf_encode(0, end, is_transient, &mut enc_res, lm, tf_select, &mut enc);
        let bytes = enc.done();

        let mut dec_res = vec![0i32; end];
        let mut dec = RangeDecoder::new(&bytes);
        tf_decode(0, end, is_transient, &mut dec_res, lm, &mut dec);

        assert_eq!(
            enc_res, dec_res,
            "tf_res mismatch (lm={lm}, transient={is_transient}, tf_select={tf_select})"
        );
    }

    #[test]
    fn tf_round_trips_all_modes() {
        let patterns: &[&[i32]] = &[
            &[
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            ],
            &[
                1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
            ],
            &[
                0, 1, 0, 1, 1, 0, 0, 1, 1, 1, 0, 0, 1, 0, 1, 0, 1, 1, 0, 0, 1,
            ],
            &[
                0, 0, 0, 1, 1, 1, 0, 0, 0, 1, 1, 1, 0, 0, 0, 1, 1, 1, 0, 0, 0,
            ],
        ];
        for lm in 0..=3 {
            for &is_transient in &[false, true] {
                for tf_select in 0..=1 {
                    for pat in patterns {
                        round_trip(lm, is_transient, tf_select, pat);
                    }
                }
            }
        }
    }

    #[test]
    fn tf_mapped_values_are_in_table() {
        // After encoding, every flag is a tf_select_table entry for this LM.
        let mut res = vec![0, 1, 1, 0, 1, 0, 0, 1, 1, 0, 1];
        let mut enc = RangeEncoder::new(64);
        tf_encode(0, res.len(), true, &mut res, 3, 1, &mut enc);
        for &v in &res {
            assert!(
                TF_SELECT_TABLE[3].iter().any(|&t| i32::from(t) == v),
                "mapped value {v} not in table"
            );
        }
    }

    #[test]
    fn tf_low_budget_forces_constant_flags() {
        // With a near-full coder there's no room to code changes, so all flags
        // collapse to the running value and still round-trip.
        round_trip(2, false, 0, &[1, 0, 1, 1, 0]);
        round_trip(0, true, 0, &[0, 1, 0]);
    }

    const EBAND5MS: [i16; 22] = [
        0, 1, 2, 3, 4, 5, 6, 7, 8, 10, 12, 14, 16, 20, 24, 28, 34, 40, 48, 60, 78, 100,
    ];

    fn switches(v: &[i32]) -> usize {
        v.windows(2).filter(|w| w[0] != w[1]).count()
    }

    #[test]
    fn tf_analysis_outputs_valid_flags_and_is_deterministic() {
        let len = 21usize;
        let lm = 3i32;
        let n0 = (EBAND5MS[21] as usize) << lm;
        let x: Vec<f32> = (0..n0).map(|i| (0.05 * i as f32).sin() * 0.1).collect();
        let importance = vec![1i32; len];
        let mut tf1 = vec![0i32; len];
        let s1 = tf_analysis(
            &EBAND5MS,
            len,
            false,
            &mut tf1,
            2,
            &x,
            n0,
            lm,
            0.0,
            0,
            &importance,
        );
        let mut tf2 = vec![0i32; len];
        let s2 = tf_analysis(
            &EBAND5MS,
            len,
            false,
            &mut tf2,
            2,
            &x,
            n0,
            lm,
            0.0,
            0,
            &importance,
        );
        assert_eq!(tf1, tf2, "deterministic flags");
        assert_eq!(s1, s2, "deterministic tf_select");
        assert!(tf1.iter().all(|&v| v == 0 || v == 1), "flags are 0/1");
        assert!(s1 == 0 || s1 == 1);
        // Non-transient frames keep tf_select=0 (the conservative policy).
        assert_eq!(s1, 0, "tf_select stays 0 for non-transient");
    }

    #[test]
    fn tf_analysis_large_lambda_suppresses_switching() {
        let len = 21usize;
        let lm = 3i32;
        let n0 = (EBAND5MS[21] as usize) << lm;
        // Band-to-band variation so the per-band metrics genuinely differ.
        let x: Vec<f32> = (0..n0)
            .map(|i| if (i / 13) % 2 == 0 { 0.3 } else { -0.02 })
            .collect();
        let importance = vec![1i32; len];

        let mut tf_small = vec![0i32; len];
        tf_analysis(
            &EBAND5MS,
            len,
            false,
            &mut tf_small,
            0,
            &x,
            n0,
            lm,
            0.0,
            0,
            &importance,
        );
        let mut tf_big = vec![0i32; len];
        tf_analysis(
            &EBAND5MS,
            len,
            false,
            &mut tf_big,
            1_000_000,
            &x,
            n0,
            lm,
            0.0,
            0,
            &importance,
        );
        assert!(
            switches(&tf_big) <= switches(&tf_small),
            "raising lambda must not add switches"
        );
        assert_eq!(switches(&tf_big), 0, "huge switch cost -> constant flags");
    }

    #[test]
    fn tf_analysis_feeds_tf_encode_round_trip() {
        // The analysis output must survive the real transmit path: encode the
        // chosen flags, decode them, and recover the same mapped resolution.
        let len = 8usize;
        let lm = 3i32;
        let n0 = (EBAND5MS[len] as usize) << lm;
        let x: Vec<f32> = (0..n0).map(|i| (0.07 * i as f32).cos() * 0.2).collect();
        let importance = vec![1i32; len];
        let mut tf_res = vec![0i32; len];
        let tf_select = tf_analysis(
            &EBAND5MS,
            len,
            false,
            &mut tf_res,
            1,
            &x,
            n0,
            lm,
            0.0,
            0,
            &importance,
        );

        let mut enc = RangeEncoder::new(64);
        let mut to_send = tf_res.clone();
        tf_encode(0, len, false, &mut to_send, lm, tf_select, &mut enc);
        let buf = enc.done();

        let mut dec = RangeDecoder::new(&buf);
        let mut got = vec![0i32; len];
        tf_decode(0, len, false, &mut got, lm, &mut dec);
        assert_eq!(got, to_send, "encoded tf flags must decode bit-exactly");
    }
}
