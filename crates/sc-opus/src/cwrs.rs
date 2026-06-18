//! Opus CELT PVQ pulse coding (CWRS).
//!
//! Hand-ported to safe Rust from libopus `celt/cwrs.c`: `icwrs`/`encode_pulses`
//! (index a pulse vector and range-code it) and `cwrsi`/`decode_pulses` (the
//! inverse). Derivative work of libopus (BSD-3-Clause); see `LICENSE-THIRDPARTY`.
//!
//! libopus ships the combinatorial `U(N,K)` counts as a large static table
//! (`CELT_PVQ_U_DATA`); since `U` is symmetric and defined by the recurrence
//! `U(n,k) = U(n-1,k) + U(n,k-1) + U(n-1,k-1)` (with `U(0,0)=1`, otherwise 0 on
//! an axis), this port computes the values from that recurrence instead of
//! copying the table, and the tests pin the result to the documented closed
//! forms so it stays bit-identical to libopus.

#![allow(dead_code)]

use crate::range_coder::{RangeDecoder, RangeEncoder};

/// `CELT_PVQ_U(n, k)`: number of length-`n` pulse vectors whose entries before
/// the last sign choice index to `k`. Symmetric in its arguments.
fn u(n: u32, k: u32) -> u32 {
    let k = k as usize;
    // Rolling Levinson of the 2-D recurrence; row 0 is U(0, j).
    let mut prev = vec![0u64; k + 1];
    prev[0] = 1;
    if n == 0 {
        return prev[k] as u32;
    }
    let mut cur = vec![0u64; k + 1];
    for _ in 1..=n {
        cur[0] = 0;
        for j in 1..=k {
            cur[j] = prev[j] + cur[j - 1] + prev[j - 1];
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    prev[k] as u32
}

/// `CELT_PVQ_V(n, k)`: number of PVQ codewords for a size-`n` band with `k`
/// pulses.
pub(crate) fn v(n: u32, k: u32) -> u32 {
    u(n, k) + u(n, k + 1)
}

/// Indexes a pulse vector `y` (length `n >= 2`, L1 norm `k`) into `[0, V(n,k))`.
fn icwrs(n: usize, y: &[i32]) -> u32 {
    debug_assert!(n >= 2);
    let mut j = n - 1;
    let mut i = u32::from(y[j] < 0);
    let mut k = y[j].unsigned_abs();
    loop {
        j -= 1;
        i = i.wrapping_add(u((n - j) as u32, k));
        k += y[j].unsigned_abs();
        if y[j] < 0 {
            i = i.wrapping_add(u((n - j) as u32, k + 1));
        }
        if j == 0 {
            break;
        }
    }
    i
}

/// Range-codes the pulse vector `y` of size `n` with `k` pulses.
pub fn encode_pulses(y: &[i32], n: usize, k: u32, enc: &mut RangeEncoder) {
    debug_assert!(k > 0);
    enc.enc_uint(icwrs(n, y), v(n as u32, k));
}

/// Reconstructs the pulse vector for index `i` into `y`; returns `sum(y^2)`.
fn cwrsi(n0: usize, k0: u32, i0: u32, y: &mut [i32]) -> f32 {
    let mut n = n0;
    let mut k = k0;
    let mut i = i0;
    let mut yy = 0.0f32;
    let mut idx = 0usize;

    while n > 2 {
        if k >= n as u32 {
            // Lots of pulses.
            let mut p = u(n as u32, k + 1);
            let s = -i32::from(i >= p);
            i = i.wrapping_sub(p & s as u32);
            let prev_k = k;
            let q = u(n as u32, n as u32);
            if q > i {
                k = n as u32;
                loop {
                    k -= 1;
                    p = u(k, n as u32);
                    if p <= i {
                        break;
                    }
                }
            } else {
                loop {
                    p = u(n as u32, k);
                    if p > i {
                        k -= 1;
                    } else {
                        break;
                    }
                }
            }
            i -= p;
            let val = (prev_k as i32 - k as i32 + s) ^ s;
            y[idx] = val;
            idx += 1;
            yy += (val * val) as f32;
        } else {
            // Lots of dimensions.
            let p = u(k, n as u32);
            let q = u(k + 1, n as u32);
            if p <= i && i < q {
                i -= p;
                y[idx] = 0;
                idx += 1;
            } else {
                let s = -i32::from(i >= q);
                i = i.wrapping_sub(q & s as u32);
                let prev_k = k;
                let mut p2;
                loop {
                    k -= 1;
                    p2 = u(k, n as u32);
                    if p2 <= i {
                        break;
                    }
                }
                i -= p2;
                let val = (prev_k as i32 - k as i32 + s) ^ s;
                y[idx] = val;
                idx += 1;
                yy += (val * val) as f32;
            }
        }
        n -= 1;
    }

    // n == 2
    let p = 2 * k + 1;
    let s = -i32::from(i >= p);
    i = i.wrapping_sub(p & s as u32);
    let prev_k = k;
    k = (i + 1) >> 1;
    if k != 0 {
        i -= 2 * k - 1;
    }
    let val = (prev_k as i32 - k as i32 + s) ^ s;
    y[idx] = val;
    idx += 1;
    yy += (val * val) as f32;

    // n == 1
    let s = -(i as i32);
    let val = (k as i32 + s) ^ s;
    y[idx] = val;
    yy += (val * val) as f32;

    yy
}

/// Range-decodes a pulse vector of size `n` with `k` pulses into `y`.
pub fn decode_pulses(y: &mut [i32], n: usize, k: u32, dec: &mut RangeDecoder) -> f32 {
    let i = dec.dec_uint(v(n as u32, k));
    cwrsi(n, k, i, y)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn u_matches_closed_forms() {
        // Closed forms have intermediate negative terms for small k, so evaluate
        // them in signed 64-bit and compare against the unsigned recurrence.
        for k in 1..40i64 {
            assert_eq!(i64::from(u(1, k as u32)), 1, "U(1,{k})");
            assert_eq!(i64::from(u(2, k as u32)), 2 * k - 1, "U(2,{k})");
            assert_eq!(i64::from(u(3, k as u32)), (2 * k - 2) * k + 1, "U(3,{k})");
            assert_eq!(
                i64::from(u(4, k as u32)),
                (((4 * k - 6) * k + 8) * k - 3) / 3,
                "U(4,{k})"
            );
            assert_eq!(
                i64::from(u(5, k as u32)),
                ((((2 * k - 4) * k + 10) * k - 8) * k + 3) / 3,
                "U(5,{k})"
            );
        }
    }

    #[test]
    fn u_is_symmetric() {
        for n in 0..20u32 {
            for k in 0..20u32 {
                assert_eq!(u(n, k), u(k, n), "U({n},{k})");
            }
        }
    }

    #[test]
    fn v_matches_closed_forms() {
        for k in 1..30u32 {
            assert_eq!(v(2, k), 4 * k, "V(2,{k})");
            assert_eq!(v(3, k), 4 * k * k + 2, "V(3,{k})");
        }
    }

    #[test]
    fn round_trips_pulse_vectors() {
        let cases: &[(usize, u32, &[i32])] = &[
            (4, 3, &[1, -1, 0, 1]),
            (4, 3, &[3, 0, 0, 0]),
            (4, 3, &[0, 0, -3, 0]),
            (5, 4, &[2, -1, 0, 1, 0]),
            (5, 4, &[0, 0, 4, 0, 0]),
            (5, 4, &[-1, -1, -1, -1, 0]),
            (2, 5, &[3, -2]),
            (2, 5, &[0, -5]),
            (3, 6, &[2, -2, 2]),
            (8, 3, &[1, 0, -1, 0, 1, 0, 0, 0]),
            (6, 1, &[0, 0, 0, -1, 0, 0]),
        ];

        for &(n, k, y) in cases {
            assert_eq!(
                y.iter().map(|v| v.unsigned_abs()).sum::<u32>(),
                k,
                "test vector L1 norm must equal k"
            );

            let mut enc = RangeEncoder::new(64);
            encode_pulses(y, n, k, &mut enc);
            let bytes = enc.done();

            let mut dec = RangeDecoder::new(&bytes);
            let mut decoded = vec![0i32; n];
            let yy = decode_pulses(&mut decoded, n, k, &mut dec);

            assert_eq!(decoded, y, "n={n} k={k}");
            let expected_yy: f32 = y.iter().map(|&v| (v * v) as f32).sum();
            assert!((yy - expected_yy).abs() < 1e-3, "yy mismatch n={n} k={k}");
        }
    }
}
