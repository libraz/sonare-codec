//! CELT bit allocation.
//!
//! Hand-ported to safe Rust from libopus `celt/rate.c` (`clt_compute_allocation`,
//! `interp_bits2pulses`) and `celt/celt.c` (`init_caps`): given the per-band
//! allocation table, the dynalloc offsets and the allocation trim, this decides
//! how many PVQ bits (`pulses`) and fine-energy bits (`ebits`) each band gets,
//! which bands are skipped (`coded_bands`), and the intensity / dual-stereo
//! parameters — coding the skip / intensity / dual-stereo flags into the range
//! coder. Derivative work of libopus (BSD-3-Clause); see `LICENSE-THIRDPARTY`.
//!
//! The allocation drives the whole CELT bitstream, so encoder and decoder must
//! reach the identical split. The port keeps one code path for both sides (the
//! [`Coder`] enum), and the tests round-trip encode → decode and pin every
//! output (`pulses` / `ebits` / `fine_priority` / `coded_bands` / `intensity` /
//! `dual_stereo` / `balance`) to bit-exact agreement.

// Consumed by the CELT encode/decode entry points; the live encoder still ships
// via the Opus FFI path.
#![allow(dead_code)]

use crate::quant_band::Coder;
use crate::range_coder::{RangeDecoder, RangeEncoder};
use crate::theta::BITRES;

const ALLOC_STEPS: i32 = 6;
const MAX_FINE_BITS: i32 = 8;
const FINE_OFFSET: i32 = 21;

/// `LOG2_FRAC_TABLE`: the cost (in eighths of a bit) of coding a uniform value
/// in `[0, i)`, used to reserve bits for the intensity parameter.
const LOG2_FRAC_TABLE: [i32; 24] = [
    0, 8, 13, 16, 19, 21, 23, 24, 26, 27, 28, 29, 30, 31, 32, 32, 33, 34, 34, 35, 36, 36, 37, 37,
];

/// The mode tables the allocation reads.
pub struct AllocInput<'a> {
    pub e_bands: &'a [i16],
    pub log_n: &'a [i16],
    pub alloc_vectors: &'a [u8],
    pub nb_alloc_vectors: usize,
    pub nb_e_bands: usize,
    pub start: usize,
    pub end: usize,
    pub c: i32,
    pub lm: i32,
}

/// `init_caps`: turns the packed per-band rate caps into the actual cap (in
/// eighth-bits) for each band at this `LM` / channel count `C`.
#[must_use]
pub fn init_caps(caps: &[u8], e_bands: &[i16], nb_e_bands: usize, lm: i32, c: i32) -> Vec<i32> {
    (0..nb_e_bands)
        .map(|i| {
            let n = (i32::from(e_bands[i + 1]) - i32::from(e_bands[i])) << lm;
            let row = nb_e_bands * (2 * lm as usize + (c as usize - 1));
            ((i32::from(caps[row + i]) + 64) * c * n) >> 2
        })
        .collect()
}

/// `encode_dynalloc_boost`: codes the per-band dynamic-allocation boosts.
///
/// Hand-ported to safe Rust from libopus `celt/celt_encoder.c` (the dynalloc
/// loop in `celt_encode_with_ec`). Derivative work of libopus (BSD-3-Clause);
/// see `LICENSE-THIRDPARTY`. On input `offsets[i]` is the desired boost *count*
/// from `dynalloc_analysis`; on output it is the granted boost in eighth-bits.
/// `total_bits` is the frame's bit budget (pre-`BITRES`). Returns `total_boost`.
#[allow(clippy::too_many_arguments)]
pub fn encode_dynalloc_boost(
    enc: &mut RangeEncoder,
    e_bands: &[i16],
    start: usize,
    end: usize,
    c: i32,
    lm: i32,
    cap: &[i32],
    total_bits: i32,
    offsets: &mut [i32],
) -> i32 {
    let total_bits = total_bits << BITRES;
    let mut dynalloc_logp = 6i32;
    let mut total_boost = 0i32;
    let mut tell = enc.ec_tell_frac() as i32;
    for i in start..end {
        let width = (c * (i32::from(e_bands[i + 1]) - i32::from(e_bands[i]))) << lm;
        // quanta: 6 bits, but no more than 1 bit/sample and no less than 1/8.
        let quanta = (width << BITRES).min((6 << BITRES).max(width));
        let mut dynalloc_loop_logp = dynalloc_logp;
        let mut boost = 0i32;
        let mut j = 0i32;
        while tell + (dynalloc_loop_logp << BITRES) < total_bits - total_boost && boost < cap[i] {
            let flag = j < offsets[i];
            enc.enc_bit_logp(flag, dynalloc_loop_logp as u32);
            tell = enc.ec_tell_frac() as i32;
            if !flag {
                break;
            }
            boost += quanta;
            total_boost += quanta;
            dynalloc_loop_logp = 1;
            j += 1;
        }
        if j > 0 {
            dynalloc_logp = 2.max(dynalloc_logp - 1);
        }
        offsets[i] = boost;
    }
    total_boost
}

/// `decode_dynalloc_boost`: the decoder side of [`encode_dynalloc_boost`],
/// reading each band's granted boost (eighth-bits) into `offsets`.
///
/// Ported from libopus `celt/celt_decoder.c` (the dynalloc loop in
/// `celt_decode_with_ec`). The encoder's `total_bits - total_boost` guard equals
/// this loop's decremented `total_bits`, so the two stay bit-exactly in step.
#[allow(clippy::too_many_arguments)]
pub fn decode_dynalloc_boost(
    dec: &mut RangeDecoder,
    e_bands: &[i16],
    start: usize,
    end: usize,
    c: i32,
    lm: i32,
    cap: &[i32],
    total_bits: i32,
    offsets: &mut [i32],
) {
    let mut total_bits = total_bits << BITRES;
    let mut dynalloc_logp = 6i32;
    let mut tell = dec.ec_tell_frac() as i32;
    for i in start..end {
        let width = (c * (i32::from(e_bands[i + 1]) - i32::from(e_bands[i]))) << lm;
        let quanta = (width << BITRES).min((6 << BITRES).max(width));
        let mut dynalloc_loop_logp = dynalloc_logp;
        let mut boost = 0i32;
        while tell + (dynalloc_loop_logp << BITRES) < total_bits && boost < cap[i] {
            let flag = dec.dec_bit_logp(dynalloc_loop_logp as u32);
            tell = dec.ec_tell_frac() as i32;
            if !flag {
                break;
            }
            boost += quanta;
            total_bits -= quanta;
            dynalloc_loop_logp = 1;
        }
        offsets[i] = boost;
        if boost > 0 {
            dynalloc_logp = 2.max(dynalloc_logp - 1);
        }
    }
}

/// The result of [`clt_compute_allocation`].
pub struct Allocation {
    pub coded_bands: usize,
    pub balance: i32,
    pub intensity: i32,
    pub dual_stereo: i32,
}

/// `interp_bits2pulses`: interpolates between the two allocation rows, decides
/// which bands to skip, and splits each band's budget into fine-energy
/// (`ebits`) and PVQ (`pulses`) bits. Returns `(coded_bands, balance)`.
#[allow(clippy::too_many_arguments)]
fn interp_bits2pulses(
    inp: &AllocInput,
    coder: &mut Coder,
    skip_start: i32,
    bits1: &[i32],
    bits2: &[i32],
    thresh: &[i32],
    cap: &[i32],
    mut total: i32,
    skip_rsv: i32,
    intensity: &mut i32,
    mut intensity_rsv: i32,
    dual_stereo: &mut i32,
    mut dual_stereo_rsv: i32,
    pulses: &mut [i32],
    ebits: &mut [i32],
    fine_priority: &mut [i32],
    prev: i32,
    signal_bandwidth: i32,
) -> (usize, i32) {
    let c = inp.c;
    let lm = inp.lm;
    let start = inp.start as i32;
    let end = inp.end as i32;
    let e_bands = inp.e_bands;
    let alloc_floor = c << BITRES;
    let stereo = i32::from(c > 1);
    let log_m = lm << BITRES;

    // Bisect for the interpolation point that just fits the budget.
    let mut lo = 0i32;
    let mut hi = 1 << ALLOC_STEPS;
    for _ in 0..ALLOC_STEPS {
        let mid = (lo + hi) >> 1;
        let mut psum = 0i32;
        let mut done = false;
        let mut j = end;
        while j > start {
            j -= 1;
            let ju = j as usize;
            let tmp = bits1[ju] + ((mid * bits2[ju]) >> ALLOC_STEPS);
            if tmp >= thresh[ju] || done {
                done = true;
                psum += tmp.min(cap[ju]);
            } else if tmp >= alloc_floor {
                psum += alloc_floor;
            }
        }
        if psum > total {
            hi = mid;
        } else {
            lo = mid;
        }
    }

    let mut psum = 0i32;
    let mut done = false;
    let mut j = end;
    while j > start {
        j -= 1;
        let ju = j as usize;
        let mut tmp = bits1[ju] + ((lo * bits2[ju]) >> ALLOC_STEPS);
        if tmp < thresh[ju] && !done {
            tmp = if tmp >= alloc_floor { alloc_floor } else { 0 };
        } else {
            done = true;
        }
        tmp = tmp.min(cap[ju]);
        pulses[ju] = tmp;
        psum += tmp;
    }

    // Decide which bands to skip, working back from the end.
    let mut coded_bands = end;
    loop {
        let j = coded_bands - 1;
        let ju = j as usize;
        if j <= skip_start {
            total += skip_rsv;
            break;
        }
        let mut left = total - psum;
        let denom = i32::from(e_bands[coded_bands as usize]) - i32::from(e_bands[start as usize]);
        let percoeff = left / denom;
        left -= denom * percoeff;
        let rem = (left - (i32::from(e_bands[ju]) - i32::from(e_bands[start as usize]))).max(0);
        let band_width = i32::from(e_bands[coded_bands as usize]) - i32::from(e_bands[ju]);
        let mut band_bits = pulses[ju] + percoeff * band_width + rem;
        if band_bits >= thresh[ju].max(alloc_floor + (1 << BITRES)) {
            let skip = match coder {
                Coder::Enc(ec) => {
                    let depth_threshold = if coded_bands > 17 {
                        if j < prev {
                            7
                        } else {
                            9
                        }
                    } else {
                        0
                    };
                    let do_skip = coded_bands <= start + 2
                        || (band_bits > (((depth_threshold * band_width) << lm << BITRES) >> 4)
                            && j <= signal_bandwidth);
                    ec.enc_bit_logp(do_skip, 1);
                    do_skip
                }
                Coder::Dec(dec) => dec.dec_bit_logp(1),
            };
            if skip {
                break;
            }
            psum += 1 << BITRES;
            band_bits -= 1 << BITRES;
        }
        psum -= pulses[ju] + intensity_rsv;
        if intensity_rsv > 0 {
            intensity_rsv = LOG2_FRAC_TABLE[(j - start) as usize];
        }
        psum += intensity_rsv;
        if band_bits >= alloc_floor {
            psum += alloc_floor;
            pulses[ju] = alloc_floor;
        } else {
            pulses[ju] = 0;
        }
        coded_bands -= 1;
    }
    debug_assert!(coded_bands > start);

    // Code the intensity and dual-stereo parameters.
    if intensity_rsv > 0 {
        match coder {
            Coder::Enc(ec) => {
                *intensity = (*intensity).min(coded_bands);
                ec.enc_uint(
                    (*intensity - start) as u32,
                    (coded_bands + 1 - start) as u32,
                );
            }
            Coder::Dec(dec) => {
                *intensity = start + dec.dec_uint((coded_bands + 1 - start) as u32) as i32;
            }
        }
    } else {
        *intensity = 0;
    }
    if *intensity <= start {
        total += dual_stereo_rsv;
        dual_stereo_rsv = 0;
    }
    if dual_stereo_rsv > 0 {
        match coder {
            Coder::Enc(ec) => ec.enc_bit_logp(*dual_stereo != 0, 1),
            Coder::Dec(dec) => *dual_stereo = i32::from(dec.dec_bit_logp(1)),
        }
    } else {
        *dual_stereo = 0;
    }

    // Allocate the remaining bits.
    let mut left = total - psum;
    let denom = i32::from(e_bands[coded_bands as usize]) - i32::from(e_bands[start as usize]);
    let percoeff = left / denom;
    left -= denom * percoeff;
    for j in start..coded_bands {
        let ju = j as usize;
        pulses[ju] += percoeff * (i32::from(e_bands[ju + 1]) - i32::from(e_bands[ju]));
    }
    for j in start..coded_bands {
        let ju = j as usize;
        let tmp = left.min(i32::from(e_bands[ju + 1]) - i32::from(e_bands[ju]));
        pulses[ju] += tmp;
        left -= tmp;
    }

    let mut balance = 0i32;
    for j in start..coded_bands {
        let ju = j as usize;
        let n0 = i32::from(e_bands[ju + 1]) - i32::from(e_bands[ju]);
        let n = n0 << lm;
        let bit = pulses[ju] + balance;
        let mut excess;

        if n > 1 {
            excess = (bit - cap[ju]).max(0);
            pulses[ju] = bit - excess;

            // Compensate for the extra DoF in stereo.
            let den = c * n + i32::from(c == 2 && n > 2 && *dual_stereo == 0 && j < *intensity);
            let nc_log_n = den * (i32::from(inp.log_n[ju]) + log_m);
            let mut offset = (nc_log_n >> 1) - den * FINE_OFFSET;
            if n == 2 {
                offset += (den << BITRES) >> 2;
            }
            // Offsets for the second and third fine-energy bits.
            if pulses[ju] + offset < (den * 2) << BITRES {
                offset += nc_log_n >> 2;
            } else if pulses[ju] + offset < (den * 3) << BITRES {
                offset += nc_log_n >> 3;
            }
            ebits[ju] = (pulses[ju] + offset + (den << (BITRES - 1))).max(0);
            ebits[ju] = (ebits[ju] / den) >> BITRES;
            // Make sure not to bust.
            if c * ebits[ju] > (pulses[ju] >> BITRES) {
                ebits[ju] = pulses[ju] >> stereo >> BITRES;
            }
            ebits[ju] = ebits[ju].min(MAX_FINE_BITS);
            // If we rounded down or capped, mark for the final fine pass.
            fine_priority[ju] = i32::from(ebits[ju] * (den << BITRES) >= pulses[ju] + offset);
            // The rest goes to PVQ.
            pulses[ju] -= (c * ebits[ju]) << BITRES;
        } else {
            // N=1: all bits to fine energy except a single sign bit.
            excess = (bit - (c << BITRES)).max(0);
            pulses[ju] = bit - excess;
            ebits[ju] = 0;
            fine_priority[ju] = 1;
        }

        // Re-balance here (fine energy can't use quant_all_bands rebalancing).
        if excess > 0 {
            let extra_fine = (excess >> (stereo + BITRES)).min(MAX_FINE_BITS - ebits[ju]);
            ebits[ju] += extra_fine;
            let extra_bits = (extra_fine * c) << BITRES;
            fine_priority[ju] = i32::from(extra_bits >= excess - balance);
            excess -= extra_bits;
        }
        balance = excess;
    }
    let final_balance = balance;

    // Skipped bands use all their bits for fine energy.
    for j in coded_bands..end {
        let ju = j as usize;
        ebits[ju] = pulses[ju] >> stereo >> BITRES;
        pulses[ju] = 0;
        fine_priority[ju] = i32::from(ebits[ju] < 1);
    }

    (coded_bands as usize, final_balance)
}

/// `clt_compute_allocation`: the top-level CELT allocator. Fills `pulses`,
/// `ebits`, `fine_priority` (length `nb_e_bands`) and returns the coded-band
/// count plus the residual balance and the intensity / dual-stereo parameters.
#[allow(clippy::too_many_arguments)]
pub fn clt_compute_allocation(
    inp: &AllocInput,
    coder: &mut Coder,
    offsets: &[i32],
    cap: &[i32],
    alloc_trim: i32,
    mut intensity: i32,
    mut dual_stereo: i32,
    mut total: i32,
    pulses: &mut [i32],
    ebits: &mut [i32],
    fine_priority: &mut [i32],
    prev: i32,
    signal_bandwidth: i32,
) -> Allocation {
    let c = inp.c;
    let lm = inp.lm;
    let start = inp.start;
    let end = inp.end;
    let len = inp.nb_e_bands;
    let e_bands = inp.e_bands;

    total = total.max(0);
    let mut skip_start = start as i32;
    // Reserve a bit to signal the end of manually skipped bands.
    let skip_rsv = if total >= 1 << BITRES { 1 << BITRES } else { 0 };
    total -= skip_rsv;
    // Reserve bits for the intensity and dual-stereo parameters.
    let mut intensity_rsv = 0i32;
    let mut dual_stereo_rsv = 0i32;
    if c == 2 {
        intensity_rsv = LOG2_FRAC_TABLE[end - start];
        if intensity_rsv > total {
            intensity_rsv = 0;
        } else {
            total -= intensity_rsv;
            dual_stereo_rsv = if total >= 1 << BITRES { 1 << BITRES } else { 0 };
            total -= dual_stereo_rsv;
        }
    }

    let mut bits1 = vec![0i32; len];
    let mut bits2 = vec![0i32; len];
    let mut thresh = vec![0i32; len];
    let mut trim_offset = vec![0i32; len];

    for j in start..end {
        let n0 = i32::from(e_bands[j + 1]) - i32::from(e_bands[j]);
        // Below this threshold we never allocate PVQ bits.
        thresh[j] = (c << BITRES).max(((3 * n0) << lm << BITRES) >> 4);
        // Tilt of the allocation curve.
        trim_offset[j] =
            (c * n0 * (alloc_trim - 5 - lm) * (end as i32 - j as i32 - 1) * (1 << (lm + BITRES)))
                >> 6;
        // Less resolution to single-coefficient bands.
        if (n0 << lm) == 1 {
            trim_offset[j] -= c << BITRES;
        }
    }

    // Bisect over the allocation rows for the best fit.
    let mut lo = 1i32;
    let mut hi = inp.nb_alloc_vectors as i32 - 1;
    loop {
        let mut done = false;
        let mut psum = 0i32;
        let mid = (lo + hi) >> 1;
        let mut j = end;
        while j > start {
            j -= 1;
            let n = i32::from(e_bands[j + 1]) - i32::from(e_bands[j]);
            let mut bitsj =
                ((c * n * i32::from(inp.alloc_vectors[mid as usize * len + j])) << lm) >> 2;
            if bitsj > 0 {
                bitsj = (bitsj + trim_offset[j]).max(0);
            }
            bitsj += offsets[j];
            if bitsj >= thresh[j] || done {
                done = true;
                psum += bitsj.min(cap[j]);
            } else if bitsj >= c << BITRES {
                psum += c << BITRES;
            }
        }
        if psum > total {
            hi = mid - 1;
        } else {
            lo = mid + 1;
        }
        if lo > hi {
            break;
        }
    }
    hi = lo;
    lo -= 1;

    for j in start..end {
        let n = i32::from(e_bands[j + 1]) - i32::from(e_bands[j]);
        let mut bits1j = ((c * n * i32::from(inp.alloc_vectors[lo as usize * len + j])) << lm) >> 2;
        let mut bits2j = if hi >= inp.nb_alloc_vectors as i32 {
            cap[j]
        } else {
            ((c * n * i32::from(inp.alloc_vectors[hi as usize * len + j])) << lm) >> 2
        };
        if bits1j > 0 {
            bits1j = (bits1j + trim_offset[j]).max(0);
        }
        if bits2j > 0 {
            bits2j = (bits2j + trim_offset[j]).max(0);
        }
        if lo > 0 {
            bits1j += offsets[j];
        }
        bits2j += offsets[j];
        if offsets[j] > 0 {
            skip_start = j as i32;
        }
        bits2j = (bits2j - bits1j).max(0);
        bits1[j] = bits1j;
        bits2[j] = bits2j;
    }

    let (coded_bands, balance) = interp_bits2pulses(
        inp,
        coder,
        skip_start,
        &bits1,
        &bits2,
        &thresh,
        cap,
        total,
        skip_rsv,
        &mut intensity,
        intensity_rsv,
        &mut dual_stereo,
        dual_stereo_rsv,
        pulses,
        ebits,
        fine_priority,
        prev,
        signal_bandwidth,
    );

    Allocation {
        coded_bands,
        balance,
        intensity,
        dual_stereo,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::range_coder::{RangeDecoder, RangeEncoder};
    use crate::rate::compute_pulse_cache;

    const EBAND5MS: [i16; 22] = [
        0, 1, 2, 3, 4, 5, 6, 7, 8, 10, 12, 14, 16, 20, 24, 28, 34, 40, 48, 60, 78, 100,
    ];
    const LOGN400: [i16; 21] = [
        0, 0, 0, 0, 0, 0, 0, 0, 8, 8, 8, 8, 16, 16, 16, 21, 21, 24, 29, 34, 36,
    ];
    const NB_E_BANDS: usize = 21;
    /// The standard 48 kHz mode `band_allocation` (11 rows x 21 bands).
    const BAND_ALLOCATION: [u8; 11 * 21] = [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, //
        90, 80, 75, 69, 63, 56, 49, 40, 34, 29, 20, 18, 10, 0, 0, 0, 0, 0, 0, 0, 0, //
        110, 100, 90, 84, 78, 71, 65, 58, 51, 45, 39, 32, 26, 20, 12, 0, 0, 0, 0, 0, 0, //
        118, 110, 103, 93, 86, 80, 75, 70, 65, 59, 53, 47, 40, 31, 23, 15, 4, 0, 0, 0, 0, //
        126, 119, 112, 104, 95, 89, 83, 78, 72, 66, 60, 54, 47, 39, 32, 25, 17, 12, 1, 0,
        0, //
        134, 127, 120, 114, 103, 97, 91, 85, 78, 72, 66, 60, 54, 47, 41, 35, 29, 23, 16, 10,
        1, //
        144, 137, 130, 124, 113, 107, 101, 95, 88, 82, 76, 70, 64, 57, 51, 45, 39, 33, 26, 15,
        1, //
        152, 145, 138, 132, 123, 117, 111, 105, 98, 92, 86, 80, 74, 67, 61, 55, 49, 43, 36, 20,
        1, //
        162, 155, 148, 142, 133, 127, 121, 115, 108, 102, 96, 90, 84, 77, 71, 65, 59, 53, 46, 30,
        1, //
        172, 165, 158, 152, 143, 137, 131, 125, 118, 112, 106, 100, 94, 87, 81, 75, 69, 63, 56, 45,
        20, //
        200, 200, 200, 200, 200, 200, 200, 200, 198, 193, 188, 183, 178, 173, 168, 163, 158, 153,
        148, 129, 104,
    ];

    fn make_input(c: i32, lm: i32) -> (Vec<i32>,) {
        let cache = compute_pulse_cache(&EBAND5MS, &LOGN400, NB_E_BANDS, lm);
        (init_caps(&cache.caps, &EBAND5MS, NB_E_BANDS, lm, c),)
    }

    /// Runs encode then decode of the allocation and asserts they agree exactly.
    fn round_trip(c: i32, lm: i32, total: i32, alloc_trim: i32, intensity_in: i32) {
        let (cap,) = make_input(c, lm);
        let inp = AllocInput {
            e_bands: &EBAND5MS,
            log_n: &LOGN400,
            alloc_vectors: &BAND_ALLOCATION,
            nb_alloc_vectors: 11,
            nb_e_bands: NB_E_BANDS,
            start: 0,
            end: NB_E_BANDS,
            c,
            lm,
        };
        let offsets = vec![0i32; NB_E_BANDS];
        let signal_bandwidth = (NB_E_BANDS - 1) as i32;

        let mut p_e = vec![0i32; NB_E_BANDS];
        let mut eb_e = vec![0i32; NB_E_BANDS];
        let mut fp_e = vec![0i32; NB_E_BANDS];
        let mut enc = RangeEncoder::new(512);
        let mut coder = Coder::Enc(&mut enc);
        let alloc_e = clt_compute_allocation(
            &inp,
            &mut coder,
            &offsets,
            &cap,
            alloc_trim,
            intensity_in,
            0,
            total,
            &mut p_e,
            &mut eb_e,
            &mut fp_e,
            0,
            signal_bandwidth,
        );
        let bytes = enc.done();

        let mut p_d = vec![0i32; NB_E_BANDS];
        let mut eb_d = vec![0i32; NB_E_BANDS];
        let mut fp_d = vec![0i32; NB_E_BANDS];
        let mut dec = RangeDecoder::new(&bytes);
        let mut coder_d = Coder::Dec(&mut dec);
        let alloc_d = clt_compute_allocation(
            &inp,
            &mut coder_d,
            &offsets,
            &cap,
            alloc_trim,
            intensity_in,
            0,
            total,
            &mut p_d,
            &mut eb_d,
            &mut fp_d,
            0,
            signal_bandwidth,
        );

        assert_eq!(alloc_e.coded_bands, alloc_d.coded_bands, "coded_bands");
        assert_eq!(alloc_e.balance, alloc_d.balance, "balance");
        assert_eq!(alloc_e.intensity, alloc_d.intensity, "intensity");
        assert_eq!(alloc_e.dual_stereo, alloc_d.dual_stereo, "dual_stereo");
        assert_eq!(p_e, p_d, "pulses");
        assert_eq!(eb_e, eb_d, "ebits");
        assert_eq!(fp_e, fp_d, "fine_priority");
        // Allocation respects the per-band caps and stays non-negative.
        for j in 0..alloc_e.coded_bands {
            assert!(p_e[j] >= 0, "pulses[{j}] negative");
            assert!(eb_e[j] >= 0 && eb_e[j] <= MAX_FINE_BITS, "ebits[{j}] range");
        }
    }

    #[test]
    fn mono_allocation_round_trips() {
        for &total in &[800, 2000, 4000, 8000] {
            round_trip(1, 3, total, 5, 0);
        }
    }

    #[test]
    fn stereo_allocation_round_trips() {
        for &total in &[2000, 5000, 9000] {
            round_trip(2, 3, total, 5, NB_E_BANDS as i32);
        }
    }

    #[test]
    fn allocation_trim_extremes_round_trip() {
        round_trip(1, 3, 4000, 0, 0);
        round_trip(1, 3, 4000, 10, 0);
        round_trip(2, 3, 6000, 2, 10);
        round_trip(2, 3, 6000, 8, 18);
    }

    #[test]
    fn allocation_short_block_round_trips() {
        round_trip(1, 0, 1500, 5, 0);
        round_trip(2, 1, 3000, 5, NB_E_BANDS as i32);
    }

    #[test]
    fn total_bits_grow_pulses() {
        // More bits never reduce the total PVQ allocation.
        let (cap,) = make_input(1, 3);
        let inp = AllocInput {
            e_bands: &EBAND5MS,
            log_n: &LOGN400,
            alloc_vectors: &BAND_ALLOCATION,
            nb_alloc_vectors: 11,
            nb_e_bands: NB_E_BANDS,
            start: 0,
            end: NB_E_BANDS,
            c: 1,
            lm: 3,
        };
        let offsets = vec![0i32; NB_E_BANDS];
        let sum = |total: i32| -> i32 {
            let mut p = vec![0i32; NB_E_BANDS];
            let mut eb = vec![0i32; NB_E_BANDS];
            let mut fp = vec![0i32; NB_E_BANDS];
            let mut enc = RangeEncoder::new(512);
            let mut coder = Coder::Enc(&mut enc);
            clt_compute_allocation(
                &inp, &mut coder, &offsets, &cap, 5, 0, 0, total, &mut p, &mut eb, &mut fp, 0, 20,
            );
            p.iter().sum()
        };
        assert!(sum(4000) >= sum(1500), "pulses should grow with budget");
    }

    /// Encodes a desired-boost vector, decodes it, and returns
    /// (granted-by-encoder, granted-by-decoder, total_boost).
    fn boost_round_trip(
        c: i32,
        lm: i32,
        total_bits: i32,
        desired: &[i32],
    ) -> (Vec<i32>, Vec<i32>, i32) {
        let (cap,) = make_input(c, lm);
        let mut enc = RangeEncoder::new(512);
        let mut off_enc = desired.to_vec();
        let tot = encode_dynalloc_boost(
            &mut enc,
            &EBAND5MS,
            0,
            NB_E_BANDS,
            c,
            lm,
            &cap,
            total_bits,
            &mut off_enc,
        );
        let bytes = enc.done();
        let mut dec = RangeDecoder::new(&bytes);
        let mut off_dec = vec![0i32; NB_E_BANDS];
        decode_dynalloc_boost(
            &mut dec,
            &EBAND5MS,
            0,
            NB_E_BANDS,
            c,
            lm,
            &cap,
            total_bits,
            &mut off_dec,
        );
        (off_enc, off_dec, tot)
    }

    #[test]
    fn dynalloc_boost_round_trips_bit_exact() {
        let desired = [
            0, 3, 0, 0, 5, 0, 0, 2, 0, 1, 0, 0, 4, 0, 0, 0, 2, 0, 0, 0, 1,
        ];
        let (off_enc, off_dec, tot) = boost_round_trip(1, 3, 2000, &desired);
        assert_eq!(off_enc, off_dec, "encoder/decoder boosts must agree");
        // total_boost is exactly the sum of the granted per-band boosts.
        assert_eq!(tot, off_enc.iter().sum::<i32>());
        // At a generous budget at least one requested boost is granted.
        assert!(off_enc.iter().any(|&b| b > 0), "no boost granted");
    }

    #[test]
    fn dynalloc_boost_budget_limits_grant() {
        // A tiny budget grants no (or fewer) boosts than a large one, and still
        // round-trips exactly.
        let desired = [
            6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6,
        ];
        let (small_e, small_d, small_tot) = boost_round_trip(1, 3, 60, &desired);
        let (big_e, big_d, big_tot) = boost_round_trip(1, 3, 4000, &desired);
        assert_eq!(small_e, small_d);
        assert_eq!(big_e, big_d);
        assert!(
            small_tot <= big_tot,
            "small budget {small_tot} granted more than large {big_tot}"
        );
    }

    #[test]
    fn dynalloc_boost_zero_desired_grants_nothing() {
        let desired = [0i32; NB_E_BANDS];
        let (off_enc, off_dec, tot) = boost_round_trip(2, 2, 3000, &desired);
        assert_eq!(off_enc, off_dec);
        assert_eq!(tot, 0);
        assert!(off_enc.iter().all(|&b| b == 0));
    }
}
