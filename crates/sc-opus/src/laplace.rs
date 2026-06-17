//! Opus CELT Laplace coder for energy deltas.
//!
//! Hand-ported to safe Rust from libopus `celt/laplace.c`, sitting on top of the
//! range coder in [`crate::range_coder`]. Derivative work of libopus
//! (BSD-3-Clause); see `LICENSE-THIRDPARTY`.
//!
//! The encoder clamps values that fall outside the representable range and
//! writes the clamped value back through its `value` argument, exactly as the C
//! does, so a round trip recovers the clamped value rather than the original.

#![allow(dead_code)]

use crate::range_coder::{RangeDecoder, RangeEncoder};

const LAPLACE_LOG_MINP: u32 = 0;
const LAPLACE_MINP: u32 = 1 << LAPLACE_LOG_MINP;
const LAPLACE_NMIN: u32 = 16;

/// `ec_laplace_get_freq1`: decaying-part frequency. `decay` is positive and at
/// most 11456.
fn get_freq1(fs0: u32, decay: i32) -> u32 {
    let ft = 32768 - LAPLACE_MINP * (2 * LAPLACE_NMIN) - fs0;
    (ft.wrapping_mul((16384 - decay) as u32)) >> 15
}

/// Encodes a signed energy delta, clamping and writing back through `value`.
pub fn ec_laplace_encode(enc: &mut RangeEncoder, value: &mut i32, mut fs: u32, decay: i32) {
    let mut fl: u32 = 0;
    let mut val = *value;
    if val != 0 {
        let s = -i32::from(val < 0); // 0 or -1
        val = (val + s) ^ s; // abs(val)
        fl = fs;
        fs = get_freq1(fs, decay);
        // Search the decaying part of the PDF.
        let mut i = 1;
        while fs > 0 && i < val {
            fs *= 2;
            fl += fs + 2 * LAPLACE_MINP;
            fs = (fs.wrapping_mul(decay as u32)) >> 15;
            i += 1;
        }
        // Everything beyond that has probability LAPLACE_MINP.
        if fs == 0 {
            let ndi_max = (32768 - fl + LAPLACE_MINP - 1) >> LAPLACE_LOG_MINP;
            let ndi_max = (ndi_max as i32 - s) >> 1;
            let di = (val - i).min(ndi_max - 1);
            fl += ((2 * di + 1 + s) as u32) * LAPLACE_MINP;
            fs = LAPLACE_MINP.min(32768 - fl);
            *value = (i + di + s) ^ s;
        } else {
            fs += LAPLACE_MINP;
            fl += fs & !(s as u32);
        }
        debug_assert!(fl + fs <= 32768);
        debug_assert!(fs > 0);
    }
    enc.encode_bin(fl, fl + fs, 15);
}

/// Decodes a signed energy delta encoded by [`ec_laplace_encode`].
pub fn ec_laplace_decode(dec: &mut RangeDecoder, mut fs: u32, decay: i32) -> i32 {
    let mut val = 0i32;
    let mut fl: u32 = 0;
    let fm = dec.decode_bin(15);
    if fm >= fs {
        val += 1;
        fl = fs;
        fs = get_freq1(fs, decay) + LAPLACE_MINP;
        // Search the decaying part of the PDF.
        while fs > LAPLACE_MINP && fm >= fl + 2 * fs {
            fs *= 2;
            fl += fs;
            fs = ((fs - 2 * LAPLACE_MINP).wrapping_mul(decay as u32)) >> 15;
            fs += LAPLACE_MINP;
            val += 1;
        }
        // Everything beyond that has probability LAPLACE_MINP.
        if fs <= LAPLACE_MINP {
            let di = ((fm - fl) >> (LAPLACE_LOG_MINP + 1)) as i32;
            val += di;
            fl += (2 * di as u32) * LAPLACE_MINP;
        }
        if fm < fl + fs {
            val = -val;
        } else {
            fl += fs;
        }
    }
    dec.dec_update(fl, (fl + fs).min(32768), 32768);
    val
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_energy_deltas() {
        let params = [
            (8000u32, 4000i32),
            (16000, 1000),
            (2000, 8000),
            (24000, 200),
        ];
        let values = [
            0i32, 1, -1, 2, -3, 5, -8, 13, -21, 34, -55, 200, -200, 5000, -5000,
        ];

        for &(fs, decay) in &params {
            let mut enc = RangeEncoder::new(512);
            let mut clamped = Vec::new();
            for &v in &values {
                let mut vv = v;
                ec_laplace_encode(&mut enc, &mut vv, fs, decay);
                clamped.push(vv);
            }
            let bytes = enc.done();

            let mut dec = RangeDecoder::new(&bytes);
            for &expected in &clamped {
                assert_eq!(
                    ec_laplace_decode(&mut dec, fs, decay),
                    expected,
                    "fs={fs} decay={decay}"
                );
            }
        }
    }

    #[test]
    fn zero_stays_zero() {
        let mut enc = RangeEncoder::new(64);
        let mut v = 0;
        ec_laplace_encode(&mut enc, &mut v, 12000, 3000);
        assert_eq!(v, 0);
        let bytes = enc.done();
        let mut dec = RangeDecoder::new(&bytes);
        assert_eq!(ec_laplace_decode(&mut dec, 12000, 3000), 0);
    }
}
