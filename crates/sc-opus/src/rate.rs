//! CELT pulse-cache and bit/pulse conversion.
//!
//! Hand-ported to safe Rust from libopus `celt/rate.c` and `celt/rate.h`:
//! `log2_frac` (the fractional base-2 logarithm), `get_required_bits` (the
//! per-pulse-count bit costs derived from the CWRS codebook sizes),
//! `compute_pulse_cache` (the per-band pulse-cost cache a mode precomputes), and
//! the `get_pulses` / `bits2pulses` / `pulses2bits` conversions that the band
//! quantizer uses to turn a bit budget into a pulse count. Derivative work of
//! libopus (BSD-3-Clause); see `LICENSE-THIRDPARTY`.
//!
//! These cost tables drive the CELT bit allocation, so they must be
//! bit-identical to libopus. The tests regenerate the cache for the standard
//! 48 kHz mode band layout and pin it against the static `cache_index50` /
//! `cache_bits50` tables shipped by libopus.

// Consumed by the CELT band-quantization and allocation stages; the live
// encoder still ships via the Opus FFI path.
#![allow(dead_code)]

use crate::cwrs::v;
use crate::theta::BITRES;

const MAX_PSEUDO: i32 = 40;
const LOG_MAX_PSEUDO: usize = 6;
const CELT_MAX_PULSES: i32 = 128;
const MAX_FINE_BITS: i32 = 8;
const FINE_OFFSET: i32 = 21;
const QTHETA_OFFSET: i32 = 4;
const QTHETA_OFFSET_TWOPHASE: i32 = 16;

/// `get_pulses`: maps a pseudo-pulse index to the actual pulse count it codes.
#[must_use]
pub fn get_pulses(i: i32) -> i32 {
    if i < 8 {
        i
    } else {
        (8 + (i & 7)) << ((i >> 3) - 1)
    }
}

/// `fits_in32`: whether the PVQ codebook size `V(n, k)` fits in a 32-bit
/// unsigned integer (`n` and `k` are each limited to 15 bits).
fn fits_in32(n: i32, k: i32) -> bool {
    const MAX_N: [i16; 15] = [
        32767, 32767, 32767, 1476, 283, 109, 60, 40, 29, 24, 20, 18, 16, 14, 13,
    ];
    const MAX_K: [i16; 15] = [
        32767, 32767, 32767, 32767, 1172, 238, 95, 53, 36, 27, 22, 18, 16, 15, 13,
    ];
    if n >= 14 {
        if k >= 14 {
            false
        } else {
            n <= i32::from(MAX_N[k as usize])
        }
    } else {
        k <= i32::from(MAX_K[n as usize])
    }
}

/// `log2_frac`: `log2(val)` with `frac` bits of fractional precision (rounded
/// up). Defined for `val > 0`; the maximum overestimation is ~0.0625 bits.
#[must_use]
pub fn log2_frac(mut val: u32, frac: i32) -> i32 {
    let l = 32 - val.leading_zeros() as i32; // EC_ILOG(val)
    if val & (val - 1) != 0 {
        // Renormalise to ~16 significant bits, rounding up.
        if l > 16 {
            val = ((val - 1) >> (l - 16)) + 1;
        } else {
            val <<= 16 - l;
        }
        let mut l = (l - 1) << frac;
        let mut frac = frac;
        // One iteration is always needed: the rounding above may have bumped
        // the integer part of the logarithm.
        loop {
            let b = (val >> 16) as i32;
            l += b << frac;
            val = (val + b as u32) >> (b as u32);
            val = (val * val + 0x7FFF) >> 15;
            if frac == 0 {
                break;
            }
            frac -= 1;
        }
        l + i32::from(val > 0x8000)
    } else {
        // Exact powers of two require no rounding.
        (l - 1) << frac
    }
}

/// `get_required_bits`: bit cost (in eighths, here `frac` fractional bits) of
/// coding `k` pulses in a size-`n` band, for `k` in `0..=maxk`.
fn get_required_bits(n: i32, maxk: i32, frac: i32) -> Vec<i32> {
    let mut bits = vec![0i32; (maxk + 1) as usize];
    for k in 1..=maxk {
        bits[k as usize] = log2_frac(v(n as u32, k as u32), frac);
    }
    bits
}

/// The per-band pulse-cost cache a CELT mode precomputes: `index[LM*nbEBands+band]`
/// points into `bits`, where `bits[off]` is the maximum pulse index and
/// `bits[off+q]` the cost of `q` pulses. `caps[((LM*2)+(C-1))*nbEBands+band]` is
/// the per-band rate cap (the maximum bits a band reliably uses).
pub struct PulseCache {
    pub index: Vec<i16>,
    pub bits: Vec<u8>,
    pub caps: Vec<u8>,
}

/// `compute_pulse_cache`: builds the shared pulse-cost cache (`bits` / `index`)
/// and the per-band rate `caps` for a band layout `e_bands` (length
/// `nb_e_bands + 1`), the per-band `log_n` table, and a maximum time split `lm`.
#[must_use]
pub fn compute_pulse_cache(
    e_bands: &[i16],
    log_n: &[i16],
    nb_e_bands: usize,
    lm: i32,
) -> PulseCache {
    let rows = (lm + 2) as usize;
    let mut index = vec![-1i16; nb_e_bands * rows];
    let mut entry_n: Vec<i32> = Vec::new();
    let mut entry_k: Vec<i32> = Vec::new();
    let mut entry_i: Vec<i32> = Vec::new();
    let mut curr = 0i32;

    // Scan for all unique band sizes, sharing a cache slot between bands of
    // equal size.
    let band_size = |k: usize, n: usize| -> i32 {
        ((i32::from(e_bands[n + 1]) - i32::from(e_bands[n])) << k) >> 1
    };
    for i in 0..=(lm + 1) as usize {
        for j in 0..nb_e_bands {
            let n = band_size(i, j);
            index[i * nb_e_bands + j] = -1;
            // Find an earlier band of the same size and reuse its slot.
            for k in 0..=i {
                let n_max = if k != i { nb_e_bands } else { j };
                for nn in 0..n_max {
                    if n == band_size(k, nn) {
                        index[i * nb_e_bands + j] = index[k * nb_e_bands + nn];
                        break;
                    }
                }
            }
            if index[i * nb_e_bands + j] == -1 && n != 0 {
                let mut big_k = 0i32;
                while fits_in32(n, get_pulses(big_k + 1)) && big_k < MAX_PSEUDO {
                    big_k += 1;
                }
                entry_n.push(n);
                entry_k.push(big_k);
                index[i * nb_e_bands + j] = curr as i16;
                entry_i.push(curr);
                curr += big_k + 1;
            }
        }
    }

    let mut bits = vec![0u8; curr as usize];
    for idx in 0..entry_n.len() {
        let off = entry_i[idx] as usize;
        let big_k = entry_k[idx];
        let tmp = get_required_bits(entry_n[idx], get_pulses(big_k), BITRES);
        for j in 1..=big_k {
            bits[off + j as usize] = (tmp[get_pulses(j) as usize] - 1) as u8;
        }
        bits[off] = big_k as u8;
    }

    let _ = CELT_MAX_PULSES;

    // The maximum rate per band at which we reliably use all bits we ask for.
    let mut caps = Vec::with_capacity((lm as usize + 1) * 2 * nb_e_bands);
    for i in 0..=lm as usize {
        for cc in 1..=2i32 {
            for j in 0..nb_e_bands {
                let n0_full = i32::from(e_bands[j + 1]) - i32::from(e_bands[j]);
                let mut max_bits;
                if n0_full << i == 1 {
                    // N=1 bands only have a sign bit and fine bits.
                    max_bits = (cc * (1 + MAX_FINE_BITS)) << BITRES;
                } else {
                    let mut n0 = n0_full;
                    let mut lm0 = 0i32;
                    // Even-sized bands bigger than N=2 can be split once more.
                    if n0 > 2 {
                        n0 >>= 1;
                        lm0 -= 1;
                    } else if n0 <= 1 {
                        lm0 = (i as i32).min(1);
                        n0 <<= lm0;
                    }
                    // Cost of the lowest-level PVQ of a fully split band.
                    let pcache_off = index[((lm0 + 1) as usize) * nb_e_bands + j] as usize;
                    let pcache = &bits[pcache_off..];
                    max_bits = i32::from(pcache[pcache[0] as usize]) + 1;
                    // Add the cost of coding the regular (mono) splits.
                    let mut n = n0;
                    for k in 0..(i as i32 - lm0) {
                        max_bits <<= 1;
                        let offset =
                            ((i32::from(log_n[j]) + ((lm0 + k) << BITRES)) >> 1) - QTHETA_OFFSET;
                        // Average measured theta cost ~ 459/512 of qb.
                        let num = 459 * ((2 * n - 1) * offset + max_bits);
                        let den = ((2 * n - 1) << 9) - 459;
                        let qb = ((num + (den >> 1)) / den).min(57);
                        max_bits += qb;
                        n <<= 1;
                    }
                    // Add the cost of a stereo split, if necessary.
                    if cc == 2 {
                        max_bits <<= 1;
                        let offset = ((i32::from(log_n[j]) + ((i as i32) << BITRES)) >> 1)
                            - if n == 2 {
                                QTHETA_OFFSET_TWOPHASE
                            } else {
                                QTHETA_OFFSET
                            };
                        let ndof = 2 * n - 1 - i32::from(n == 2);
                        // Average theta cost with the step PDF ~ 487/512 of qb.
                        let p = if n == 2 { 512 } else { 487 };
                        let num = p * (max_bits + ndof * offset);
                        let den = (ndof << 9) - p;
                        let qb = ((num + (den >> 1)) / den).min(if n == 2 { 64 } else { 61 });
                        max_bits += qb;
                    }
                    // Add the fine bits (compensating for the stereo DoF).
                    let ndof = cc * n + i32::from(cc == 2 && n > 2);
                    let mut offset =
                        ((i32::from(log_n[j]) + ((i as i32) << BITRES)) >> 1) - FINE_OFFSET;
                    if n == 2 {
                        offset += 1 << BITRES >> 2;
                    }
                    let num = max_bits + ndof * offset;
                    let den = (ndof - 1) << BITRES;
                    let qb = ((num + (den >> 1)) / den).min(MAX_FINE_BITS);
                    max_bits += (cc * qb) << BITRES;
                }
                max_bits = (4 * max_bits / (cc * (n0_full << i))) - 64;
                caps.push(max_bits as u8);
            }
        }
    }

    PulseCache { index, bits, caps }
}

/// Resolves a band's cache slice for `(band, lm)`, or `None` if the band has no
/// pulses cached (an `N == 0` band, marked `-1` in `index`).
fn cache_slice(cache: &PulseCache, nb_e_bands: usize, band: usize, lm: i32) -> Option<&[u8]> {
    let off = cache.index[((lm + 1) as usize) * nb_e_bands + band];
    if off < 0 {
        None
    } else {
        Some(&cache.bits[off as usize..])
    }
}

/// `bits2pulses`: the pulse index whose coded cost is closest to a `bits` budget
/// (in eighths) for band `band` at time split `lm`.
#[must_use]
pub fn bits2pulses(cache: &PulseCache, nb_e_bands: usize, band: usize, lm: i32, bits: i32) -> i32 {
    let Some(c) = cache_slice(cache, nb_e_bands, band, lm) else {
        return 0;
    };
    let mut lo = 0i32;
    let mut hi = i32::from(c[0]);
    let bits = bits - 1;
    for _ in 0..LOG_MAX_PSEUDO {
        let mid = (lo + hi + 1) >> 1;
        if i32::from(c[mid as usize]) >= bits {
            hi = mid;
        } else {
            lo = mid;
        }
    }
    let lo_cost = if lo == 0 {
        -1
    } else {
        i32::from(c[lo as usize])
    };
    if bits - lo_cost <= i32::from(c[hi as usize]) - bits {
        lo
    } else {
        hi
    }
}

/// `pulses2bits`: the coded cost (in eighths) of `pulses` pulses for band
/// `band` at time split `lm`.
#[must_use]
pub fn pulses2bits(
    cache: &PulseCache,
    nb_e_bands: usize,
    band: usize,
    lm: i32,
    pulses: i32,
) -> i32 {
    if pulses == 0 {
        return 0;
    }
    match cache_slice(cache, nb_e_bands, band, lm) {
        Some(c) => i32::from(c[pulses as usize]) + 1,
        None => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The standard 48 kHz / 20 ms mode band layout (`eband5ms`).
    const EBAND5MS: [i16; 22] = [
        0, 1, 2, 3, 4, 5, 6, 7, 8, 10, 12, 14, 16, 20, 24, 28, 34, 40, 48, 60, 78, 100,
    ];
    const NB_E_BANDS: usize = 21;
    const LOGN400: [i16; 21] = [
        0, 0, 0, 0, 0, 0, 0, 0, 8, 8, 8, 8, 16, 16, 16, 21, 21, 24, 29, 34, 36,
    ];

    /// libopus `cache_caps50` for the standard mode (`(LM+1)*2*nbEBands = 168`).
    const CACHE_CAPS50: [u8; 168] = [
        224, 224, 224, 224, 224, 224, 224, 224, 160, 160, 160, 160, 185, 185, 185, 178, 178, 168,
        134, 61, 37, 224, 224, 224, 224, 224, 224, 224, 224, 240, 240, 240, 240, 207, 207, 207,
        198, 198, 183, 144, 66, 40, 160, 160, 160, 160, 160, 160, 160, 160, 185, 185, 185, 185,
        193, 193, 193, 183, 183, 172, 138, 64, 38, 240, 240, 240, 240, 240, 240, 240, 240, 207,
        207, 207, 207, 204, 204, 204, 193, 193, 180, 143, 66, 40, 185, 185, 185, 185, 185, 185,
        185, 185, 193, 193, 193, 193, 193, 193, 193, 183, 183, 172, 138, 65, 39, 207, 207, 207,
        207, 207, 207, 207, 207, 204, 204, 204, 204, 201, 201, 201, 188, 188, 176, 141, 66, 40,
        193, 193, 193, 193, 193, 193, 193, 193, 193, 193, 193, 193, 194, 194, 194, 184, 184, 173,
        139, 65, 39, 204, 204, 204, 204, 204, 204, 204, 204, 201, 201, 201, 201, 198, 198, 198,
        187, 187, 175, 140, 66, 40,
    ];

    /// libopus `cache_index50` for the standard mode (`LM = 3`, so 5 rows).
    const CACHE_INDEX50: [i16; 105] = [
        -1, -1, -1, -1, -1, -1, -1, -1, 0, 0, 0, 0, 41, 41, 41, 82, 82, 123, 164, 200, 222, 0, 0,
        0, 0, 0, 0, 0, 0, 41, 41, 41, 41, 123, 123, 123, 164, 164, 240, 266, 283, 295, 41, 41, 41,
        41, 41, 41, 41, 41, 123, 123, 123, 123, 240, 240, 240, 266, 266, 305, 318, 328, 336, 123,
        123, 123, 123, 123, 123, 123, 123, 240, 240, 240, 240, 305, 305, 305, 318, 318, 343, 351,
        358, 364, 240, 240, 240, 240, 240, 240, 240, 240, 305, 305, 305, 305, 343, 343, 343, 351,
        351, 370, 376, 382, 387,
    ];

    /// libopus `cache_bits50` for the standard mode.
    const CACHE_BITS50: [u8; 392] = [
        40, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7,
        7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 40, 15, 23, 28, 31, 34, 36, 38, 39, 41, 42, 43, 44, 45,
        46, 47, 47, 49, 50, 51, 52, 53, 54, 55, 55, 57, 58, 59, 60, 61, 62, 63, 63, 65, 66, 67, 68,
        69, 70, 71, 71, 40, 20, 33, 41, 48, 53, 57, 61, 64, 66, 69, 71, 73, 75, 76, 78, 80, 82, 85,
        87, 89, 91, 92, 94, 96, 98, 101, 103, 105, 107, 108, 110, 112, 114, 117, 119, 121, 123,
        124, 126, 128, 40, 23, 39, 51, 60, 67, 73, 79, 83, 87, 91, 94, 97, 100, 102, 105, 107, 111,
        115, 118, 121, 124, 126, 129, 131, 135, 139, 142, 145, 148, 150, 153, 155, 159, 163, 166,
        169, 172, 174, 177, 179, 35, 28, 49, 65, 78, 89, 99, 107, 114, 120, 126, 132, 136, 141,
        145, 149, 153, 159, 165, 171, 176, 180, 185, 189, 192, 199, 205, 211, 216, 220, 225, 229,
        232, 239, 245, 251, 21, 33, 58, 79, 97, 112, 125, 137, 148, 157, 166, 174, 182, 189, 195,
        201, 207, 217, 227, 235, 243, 251, 17, 35, 63, 86, 106, 123, 139, 152, 165, 177, 187, 197,
        206, 214, 222, 230, 237, 250, 25, 31, 55, 75, 91, 105, 117, 128, 138, 146, 154, 161, 168,
        174, 180, 185, 190, 200, 208, 215, 222, 229, 235, 240, 245, 255, 16, 36, 65, 89, 110, 128,
        144, 159, 173, 185, 196, 207, 217, 226, 234, 242, 250, 11, 41, 74, 103, 128, 151, 172, 191,
        209, 225, 241, 255, 9, 43, 79, 110, 138, 163, 186, 207, 227, 246, 12, 39, 71, 99, 123, 144,
        164, 182, 198, 214, 228, 241, 253, 9, 44, 81, 113, 142, 168, 192, 214, 235, 255, 7, 49, 90,
        127, 160, 191, 220, 247, 6, 51, 95, 134, 170, 203, 234, 7, 47, 87, 123, 155, 184, 212, 237,
        6, 52, 97, 137, 174, 208, 240, 5, 57, 106, 151, 192, 231, 5, 59, 111, 158, 202, 243, 5, 55,
        103, 147, 187, 224, 5, 60, 113, 161, 206, 248, 4, 65, 122, 175, 224, 4, 67, 127, 182, 234,
    ];

    #[test]
    fn get_pulses_matches_closed_form() {
        assert_eq!(get_pulses(0), 0);
        assert_eq!(get_pulses(7), 7);
        assert_eq!(get_pulses(8), 8);
        assert_eq!(get_pulses(9), 9);
        // i>=8: (8 + (i&7)) << ((i>>3)-1).
        assert_eq!(get_pulses(16), 16);
        assert_eq!(get_pulses(24), 32);
    }

    #[test]
    fn log2_frac_exact_on_powers_of_two() {
        // For val = 2^p, log2_frac(val, frac) == p << frac exactly.
        for p in 0..20 {
            assert_eq!(log2_frac(1 << p, 3), p << 3, "2^{p}");
        }
    }

    #[test]
    fn log2_frac_overestimates_within_tolerance() {
        // Documented bound: never below the true value, never more than ~0.063
        // bits above it (in eighths, ~0.5).
        for &val in &[3u32, 5, 100, 1000, 65535, 1 << 20, u32::MAX] {
            let approx = log2_frac(val, 3) as f64 / 8.0;
            let exact = (val as f64).log2();
            assert!(approx >= exact - 1e-9, "log2_frac({val}) under-estimates");
            // ~0.0625-bit algorithm error plus the 1/8-bit output quantization.
            assert!(approx <= exact + 0.19, "log2_frac({val}) over by too much");
        }
    }

    #[test]
    fn pulse_cache_matches_libopus_static_tables() {
        // The generated cache must be bit-identical to the tables libopus ships.
        let cache = compute_pulse_cache(&EBAND5MS, &LOGN400, NB_E_BANDS, 3);
        assert_eq!(cache.index.len(), CACHE_INDEX50.len(), "index length");
        assert_eq!(cache.bits.len(), CACHE_BITS50.len(), "bits length");
        assert_eq!(cache.index, CACHE_INDEX50, "cache index table");
        assert_eq!(cache.bits, CACHE_BITS50, "cache bits table");
    }

    #[test]
    fn pulse_cache_caps_match_libopus_static_table() {
        // The generated rate caps must equal libopus' cache_caps50 exactly.
        let cache = compute_pulse_cache(&EBAND5MS, &LOGN400, NB_E_BANDS, 3);
        assert_eq!(cache.caps.len(), CACHE_CAPS50.len(), "caps length");
        assert_eq!(cache.caps, CACHE_CAPS50, "cache caps table");
    }

    #[test]
    fn bits2pulses_and_pulses2bits_are_consistent() {
        let cache = compute_pulse_cache(&EBAND5MS, &LOGN400, NB_E_BANDS, 3);
        // For a mid band at LM=3, more bits never yields fewer pulses, and the
        // cost of the chosen pulse count never exceeds the budget by much.
        let band = 17;
        let lm = 3;
        let mut prev = 0;
        for bits in (8..400).step_by(8) {
            let q = bits2pulses(&cache, NB_E_BANDS, band, lm, bits);
            assert!(q >= prev, "pulses shrank as budget grew");
            prev = q;
            // pulses2bits inverts the cost the search read for the chosen q.
            let cost = pulses2bits(&cache, NB_E_BANDS, band, lm, q);
            assert!(cost >= 0);
        }
        // Round-trip a pulse count through its own cost (band 17 caps at K=4).
        for q in 1..=4 {
            let cost = pulses2bits(&cache, NB_E_BANDS, band, lm, q);
            assert_eq!(
                bits2pulses(&cache, NB_E_BANDS, band, lm, cost),
                q,
                "pulses2bits/bits2pulses round trip at q={q}"
            );
        }
    }

    #[test]
    fn pulses2bits_zero_is_free() {
        let cache = compute_pulse_cache(&EBAND5MS, &LOGN400, NB_E_BANDS, 3);
        for band in 8..NB_E_BANDS {
            assert_eq!(pulses2bits(&cache, NB_E_BANDS, band, 3, 0), 0);
        }
    }
}
