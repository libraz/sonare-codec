//! CELT recursive band quantization.
//!
//! Hand-ported to safe Rust from libopus `celt/bands.c` (`quant_band_n1`,
//! `quant_partition`, `quant_band`, `quant_band_stereo`): the recursive coder
//! that splits a band by its mid/side angle (via [`crate::band_split`]),
//! recombines / time-divides for the transient resolution, reorders for the
//! Hadamard transform, and finally quantizes each leaf with the PVQ pulse coder
//! (via [`crate::vq`]) using the pulse budget from [`crate::rate`]. Derivative
//! work of libopus (BSD-3-Clause); see `LICENSE-THIRDPARTY`.
//!
//! libopus shares one function between the encoder and decoder via an `encode`
//! flag and a tagged entropy-coder pointer; this port mirrors that with the
//! [`Coder`] enum so the (deeply recursive) control flow is written once and the
//! encode/decode sides stay structurally identical — which is what guarantees
//! the decoder reconstructs exactly what the encoder produced.

// Consumed by the CELT `quant_all_bands` stage; the live encoder still ships via
// the Opus FFI path.
#![allow(dead_code)]

use crate::band_split::{compute_theta_decode, compute_theta_encode, BandCtx};
use crate::bands::{
    celt_lcg_rand, deinterleave_hadamard, haar1, interleave_hadamard, stereo_merge,
};
use crate::range_coder::{RangeDecoder, RangeEncoder};
use crate::rate::{bits2pulses, get_pulses, pulses2bits, PulseCache};
use crate::theta::BITRES;
use crate::vq::{alg_quant, alg_unquant, renormalise_vector};

/// The entropy coder a band-quantization pass is driven by: either an encoder
/// (which reads the band content `X`/`Y`) or a decoder (which writes it).
pub enum Coder<'a, 'b> {
    Enc(&'a mut RangeEncoder),
    Dec(&'a mut RangeDecoder<'b>),
}

impl Coder<'_, '_> {
    fn is_encode(&self) -> bool {
        matches!(self, Coder::Enc(_))
    }
}

/// The slice of `CELTMode` plus mutable allocator state that the band quantizer
/// reads. Mirrors the parts of `band_ctx` (and the mode) that the recursion
/// touches; `remaining_bits` and `seed` are updated as it runs.
pub struct QuantCtx<'a> {
    pub cache: &'a PulseCache,
    pub e_bands: &'a [i16],
    pub log_n: &'a [i16],
    pub nb_e_bands: usize,
    pub i: usize,
    pub intensity: usize,
    pub spread: i32,
    pub tf_change: i32,
    pub remaining_bits: i32,
    pub seed: u32,
    pub resynth: bool,
    pub band_e: &'a [f32],
    pub theta_round: i32,
    pub avoid_split_noise: bool,
    pub disable_inv: bool,
}

impl QuantCtx<'_> {
    /// Builds the [`BandCtx`] that [`crate::band_split`] reads for the current
    /// band, snapshotting `remaining_bits`.
    fn theta_ctx(&self) -> BandCtx<'_> {
        BandCtx {
            log_n: i32::from(self.log_n[self.i]),
            i: self.i,
            intensity: self.intensity,
            band_e: self.band_e,
            nb_e_bands: self.nb_e_bands,
            remaining_bits: self.remaining_bits,
            theta_round: self.theta_round,
            avoid_split_noise: self.avoid_split_noise,
            disable_inv: self.disable_inv,
        }
    }
}

/// Codes a single-sample band: just the sign bit (plus the fine bits handled by
/// the caller). `quant_band_n1` in libopus.
fn quant_band_n1(
    ctx: &mut QuantCtx,
    coder: &mut Coder,
    x: &mut [f32],
    y: Option<&mut [f32]>,
    mut b: i32,
    lowband_out: Option<&mut [f32]>,
) -> u32 {
    quant_n1_channel(ctx, coder, x, &mut b);
    if let Some(yc) = y {
        quant_n1_channel(ctx, coder, yc, &mut b);
    }
    if let Some(lo) = lowband_out {
        lo[0] = x[0];
    }
    1
}

/// Codes the sign of one channel of an `N == 1` band.
fn quant_n1_channel(ctx: &mut QuantCtx, coder: &mut Coder, ch: &mut [f32], b: &mut i32) {
    let mut sign = 0u32;
    if ctx.remaining_bits >= 1 << BITRES {
        match coder {
            Coder::Enc(ec) => {
                sign = u32::from(ch[0] < 0.0);
                ec.enc_bits(sign, 1);
            }
            Coder::Dec(dec) => {
                sign = dec.dec_bits(1);
            }
        }
        ctx.remaining_bits -= 1 << BITRES;
        *b -= 1 << BITRES;
    }
    if ctx.resynth {
        ch[0] = if sign != 0 { -1.0 } else { 1.0 };
    }
}

/// Encodes/decodes a mono partition, recursively splitting the band in two and
/// transmitting the energy split until it bottoms out in a PVQ leaf.
/// `quant_partition` in libopus.
#[allow(clippy::too_many_arguments)]
fn quant_partition(
    ctx: &mut QuantCtx,
    coder: &mut Coder,
    x: &mut [f32],
    mut n: i32,
    mut b: i32,
    mut big_b: i32,
    lowband: Option<&[f32]>,
    mut lm: i32,
    gain: f32,
    mut fill: i32,
) -> u32 {
    let b0 = big_b;
    let i = ctx.i;
    let spread = ctx.spread;

    // If we need ~1.5 more bits than we can produce, split the band in two.
    let cache_off = ctx.cache.index[((lm + 1) as usize) * ctx.nb_e_bands + i];
    let want_split = lm != -1 && n > 2 && cache_off >= 0 && {
        let c = &ctx.cache.bits[cache_off as usize..];
        b > i32::from(c[c[0] as usize]) + 12
    };

    if want_split {
        n >>= 1;
        let (x_lo, x_hi) = x.split_at_mut(n as usize);
        lm -= 1;
        if big_b == 1 {
            fill = (fill & 1) | (fill << 1);
        }
        big_b = (big_b + 1) >> 1;

        let mut split_fill = fill;
        let sctx = {
            let bctx = ctx.theta_ctx();
            match coder {
                Coder::Enc(ec) => compute_theta_encode(
                    &bctx,
                    ec,
                    x_lo,
                    x_hi,
                    n,
                    big_b,
                    b0,
                    lm,
                    false,
                    &mut b,
                    &mut split_fill,
                ),
                Coder::Dec(dec) => compute_theta_decode(
                    &bctx,
                    dec,
                    n,
                    big_b,
                    b0,
                    lm,
                    false,
                    &mut b,
                    &mut split_fill,
                ),
            }
        };
        fill = split_fill;
        let mid = sctx.imid as f32 / 32768.0;
        let side = sctx.iside as f32 / 32768.0;
        let itheta = sctx.itheta;
        let mut delta = sctx.delta;

        // Give more bits to low-energy MDCTs than they would otherwise deserve.
        if b0 > 1 && (itheta & 0x3fff) != 0 {
            if itheta > 8192 {
                // Rough approximation for pre-echo masking.
                delta -= delta >> (4 - lm);
            } else {
                // A forward-masking slope of 1.5 dB per 10 ms.
                delta = 0.min(delta + ((n << BITRES) >> (5 - lm)));
            }
        }
        let mut mbits = 0.max(b.min((b - delta) / 2));
        let mut sbits = b - mbits;
        ctx.remaining_bits -= sctx.qalloc;

        let (low_lo, low_hi) = match lowband {
            Some(l) => (Some(&l[..n as usize]), Some(&l[n as usize..])),
            None => (None, None),
        };

        let mut rebalance = ctx.remaining_bits;
        let cm;
        if mbits >= sbits {
            let cm0 = quant_partition(
                ctx,
                coder,
                x_lo,
                n,
                mbits,
                big_b,
                low_lo,
                lm,
                gain * mid,
                fill,
            );
            rebalance = mbits - (rebalance - ctx.remaining_bits);
            if rebalance > 3 << BITRES && itheta != 0 {
                sbits += rebalance - (3 << BITRES);
            }
            let cm1 = quant_partition(
                ctx,
                coder,
                x_hi,
                n,
                sbits,
                big_b,
                low_hi,
                lm,
                gain * side,
                fill >> big_b,
            );
            cm = cm0 | cm1.wrapping_shl((b0 >> 1) as u32);
        } else {
            let cm1 = quant_partition(
                ctx,
                coder,
                x_hi,
                n,
                sbits,
                big_b,
                low_hi,
                lm,
                gain * side,
                fill >> big_b,
            );
            rebalance = sbits - (rebalance - ctx.remaining_bits);
            if rebalance > 3 << BITRES && itheta != 16384 {
                mbits += rebalance - (3 << BITRES);
            }
            let cm0 = quant_partition(
                ctx,
                coder,
                x_lo,
                n,
                mbits,
                big_b,
                low_lo,
                lm,
                gain * mid,
                fill,
            );
            cm = cm1.wrapping_shl((b0 >> 1) as u32) | cm0;
        }
        return cm;
    }

    // Basic no-split case: turn the bit budget into a pulse count and quantize.
    let mut q = bits2pulses(ctx.cache, ctx.nb_e_bands, i, lm, b);
    let mut curr_bits = pulses2bits(ctx.cache, ctx.nb_e_bands, i, lm, q);
    ctx.remaining_bits -= curr_bits;
    // Ensure we can never bust the budget.
    while ctx.remaining_bits < 0 && q > 0 {
        ctx.remaining_bits += curr_bits;
        q -= 1;
        curr_bits = pulses2bits(ctx.cache, ctx.nb_e_bands, i, lm, q);
        ctx.remaining_bits -= curr_bits;
    }

    if q != 0 {
        let big_k = get_pulses(q);
        match coder {
            Coder::Enc(ec) => alg_quant(
                x,
                n as usize,
                big_k,
                spread,
                big_b as usize,
                ec,
                gain,
                ctx.resynth,
            ),
            Coder::Dec(dec) => alg_unquant(x, n as usize, big_k, spread, big_b as usize, dec, gain),
        }
    } else {
        // No pulse: fill the band with folded spectrum or noise.
        let mut cm = 0u32;
        if ctx.resynth {
            let cm_mask = (1u32 << big_b) - 1;
            fill &= cm_mask as i32;
            if fill == 0 {
                x[..n as usize].fill(0.0);
            } else {
                match lowband {
                    None => {
                        for v in x.iter_mut().take(n as usize) {
                            ctx.seed = celt_lcg_rand(ctx.seed);
                            *v = ((ctx.seed as i32) >> 20) as f32;
                        }
                        cm = cm_mask;
                    }
                    Some(l) => {
                        for j in 0..n as usize {
                            ctx.seed = celt_lcg_rand(ctx.seed);
                            // About 48 dB below the normal folding level.
                            let tmp = if ctx.seed & 0x8000 != 0 {
                                1.0 / 256.0
                            } else {
                                -1.0 / 256.0
                            };
                            x[j] = l[j] + tmp;
                        }
                        cm = fill as u32;
                    }
                }
                renormalise_vector(x, n as usize, gain);
            }
        }
        cm
    }
}

/// Encodes/decodes a mono band: handles the transient recombine / time-division
/// and Hadamard reorder around the recursive [`quant_partition`].
/// `quant_band` in libopus.
#[allow(clippy::too_many_arguments)]
pub fn quant_band(
    ctx: &mut QuantCtx,
    coder: &mut Coder,
    x: &mut [f32],
    n: i32,
    b: i32,
    mut big_b: i32,
    lowband: Option<&[f32]>,
    lm: i32,
    lowband_out: Option<&mut [f32]>,
    gain: f32,
    _lowband_scratch: Option<&mut [f32]>,
    mut fill: i32,
) -> u32 {
    const BIT_INTERLEAVE: [u8; 16] = [0, 1, 1, 1, 2, 3, 3, 3, 2, 3, 3, 3, 2, 3, 3, 3];
    const BIT_DEINTERLEAVE: [u8; 16] = [
        0x00, 0x03, 0x0C, 0x0F, 0x30, 0x33, 0x3C, 0x3F, 0xC0, 0xC3, 0xCC, 0xCF, 0xF0, 0xF3, 0xFC,
        0xFF,
    ];

    let n0 = n;
    let b0_orig = big_b;
    let mut time_divide = 0;
    let mut recombine = 0;
    let mut tf_change = ctx.tf_change;
    let encode = coder.is_encode();
    let long_blocks = b0_orig == 1;

    let mut n_b = n / big_b;

    if n == 1 {
        return quant_band_n1(ctx, coder, x, None, b, lowband_out);
    }

    if tf_change > 0 {
        recombine = tf_change;
    }

    // A mutable working copy of the folding reference, made only when a
    // transform below would mutate it (libopus uses caller `lowband_scratch`).
    let need_copy =
        lowband.is_some() && (recombine != 0 || ((n_b & 1) == 0 && tf_change < 0) || b0_orig > 1);
    let mut lb_work: Option<Vec<f32>> = need_copy.then(|| lowband.unwrap()[..n as usize].to_vec());

    // Band recombining to increase frequency resolution.
    for k in 0..recombine {
        if encode {
            haar1(x, (n >> k) as usize, 1 << k);
        }
        if let Some(lb) = lb_work.as_deref_mut() {
            haar1(lb, (n >> k) as usize, 1 << k);
        }
        fill = i32::from(BIT_INTERLEAVE[(fill & 0xF) as usize])
            | i32::from(BIT_INTERLEAVE[(fill >> 4) as usize & 0xF]) << 2;
    }
    big_b >>= recombine;
    n_b <<= recombine;

    // Increasing the time resolution.
    while (n_b & 1) == 0 && tf_change < 0 {
        if encode {
            haar1(x, n_b as usize, big_b as usize);
        }
        if let Some(lb) = lb_work.as_deref_mut() {
            haar1(lb, n_b as usize, big_b as usize);
        }
        fill |= fill << big_b;
        big_b <<= 1;
        n_b >>= 1;
        time_divide += 1;
        tf_change += 1;
    }
    let b0 = big_b;
    let n_b0 = n_b;

    // Reorganize the samples in time order instead of frequency order.
    if b0 > 1 {
        if encode {
            deinterleave_hadamard(
                x,
                (n_b >> recombine) as usize,
                (b0 << recombine) as usize,
                long_blocks,
            );
        }
        if let Some(lb) = lb_work.as_deref_mut() {
            deinterleave_hadamard(
                lb,
                (n_b >> recombine) as usize,
                (b0 << recombine) as usize,
                long_blocks,
            );
        }
    }

    let lowband_ref = lb_work.as_deref().or(lowband);
    let mut cm = quant_partition(ctx, coder, x, n, b, big_b, lowband_ref, lm, gain, fill);

    // Used by the decoder and by the resynthesis-enabled encoder.
    if ctx.resynth {
        // Undo the time-order reorganization.
        if b0 > 1 {
            interleave_hadamard(
                x,
                (n_b0 >> recombine) as usize,
                (b0 << recombine) as usize,
                long_blocks,
            );
        }

        // Undo the time-frequency changes.
        let mut n_b = n_b0;
        let mut big_b = b0;
        for _ in 0..time_divide {
            big_b >>= 1;
            n_b <<= 1;
            cm |= cm >> big_b;
            haar1(x, n_b as usize, big_b as usize);
        }
        for k in 0..recombine {
            cm = u32::from(BIT_DEINTERLEAVE[(cm & 0xF) as usize]);
            haar1(x, (n0 >> k) as usize, 1 << k);
        }
        big_b <<= recombine;

        // Scale output for later folding.
        if let Some(lo) = lowband_out {
            let nn = (n0 as f32).sqrt();
            for j in 0..n0 as usize {
                lo[j] = nn * x[j];
            }
        }
        cm &= (1u32 << big_b) - 1;
    }
    cm
}

/// Encodes/decodes a stereo band: codes the mid/side angle, then quantizes the
/// mid and side via [`quant_band`] (with an `N == 2` orthogonal special case).
/// `quant_band_stereo` in libopus.
#[allow(clippy::too_many_arguments)]
pub fn quant_band_stereo(
    ctx: &mut QuantCtx,
    coder: &mut Coder,
    x: &mut [f32],
    y: &mut [f32],
    n: i32,
    mut b: i32,
    big_b: i32,
    lowband: Option<&[f32]>,
    lm: i32,
    lowband_out: Option<&mut [f32]>,
    lowband_scratch: Option<&mut [f32]>,
    mut fill: i32,
) -> u32 {
    if n == 1 {
        return quant_band_n1(ctx, coder, x, Some(y), b, lowband_out);
    }

    let orig_fill = fill;

    let sctx = {
        let bctx = ctx.theta_ctx();
        match coder {
            Coder::Enc(ec) => compute_theta_encode(
                &bctx, ec, x, y, n, big_b, big_b, lm, true, &mut b, &mut fill,
            ),
            Coder::Dec(dec) => {
                compute_theta_decode(&bctx, dec, n, big_b, big_b, lm, true, &mut b, &mut fill)
            }
        }
    };
    let inv = sctx.inv;
    let mid = sctx.imid as f32 / 32768.0;
    let side = sctx.iside as f32 / 32768.0;
    let itheta = sctx.itheta;
    let delta = sctx.delta;
    let qalloc = sctx.qalloc;

    let mut cm;
    if n == 2 {
        // Special case for N=2: mid and side are orthogonal, so the side needs
        // only a sign bit.
        let mut sbits = 0;
        if itheta != 0 && itheta != 16384 {
            sbits = 1 << BITRES;
        }
        let mbits = b - sbits;
        let c = itheta > 8192;
        ctx.remaining_bits -= qalloc + sbits;

        // Read the side sign from the original (pre-quantization) pair.
        let mut sign = 0u32;
        if sbits != 0 {
            match coder {
                Coder::Enc(ec) => {
                    let (x2_0, x2_1, y2_0, y2_1) = if c {
                        (y[0], y[1], x[0], x[1])
                    } else {
                        (x[0], x[1], y[0], y[1])
                    };
                    sign = u32::from(x2_0 * y2_1 - x2_1 * y2_0 < 0.0);
                    ec.enc_bits(sign, 1);
                }
                Coder::Dec(dec) => {
                    sign = dec.dec_bits(1);
                }
            }
        }
        let signf = 1.0 - 2.0 * sign as f32;

        // Quantize the "x2" channel (the larger of mid/side).
        cm = if c {
            quant_band(
                ctx,
                coder,
                y,
                n,
                mbits,
                big_b,
                lowband,
                lm,
                lowband_out,
                1.0,
                lowband_scratch,
                orig_fill,
            )
        } else {
            quant_band(
                ctx,
                coder,
                x,
                n,
                mbits,
                big_b,
                lowband,
                lm,
                lowband_out,
                1.0,
                lowband_scratch,
                orig_fill,
            )
        };
        // y2 = orthogonal complement of the resynthesized x2.
        if c {
            x[0] = -signf * y[1];
            x[1] = signf * y[0];
        } else {
            y[0] = -signf * x[1];
            y[1] = signf * x[0];
        }
        if ctx.resynth {
            x[0] *= mid;
            x[1] *= mid;
            y[0] *= side;
            y[1] *= side;
            let t0 = x[0];
            x[0] = t0 - y[0];
            y[0] += t0;
            let t1 = x[1];
            x[1] = t1 - y[1];
            y[1] += t1;
        }
    } else {
        // "Normal" split code.
        let mut mbits = 0.max(b.min((b - delta) / 2));
        let mut sbits = b - mbits;
        ctx.remaining_bits -= qalloc;

        let mut rebalance = ctx.remaining_bits;
        if mbits >= sbits {
            // We do not scale the mid here: folding needs the normalized mid.
            cm = quant_band(
                ctx,
                coder,
                x,
                n,
                mbits,
                big_b,
                lowband,
                lm,
                lowband_out,
                1.0,
                lowband_scratch,
                fill,
            );
            rebalance = mbits - (rebalance - ctx.remaining_bits);
            if rebalance > 3 << BITRES && itheta != 0 {
                sbits += rebalance - (3 << BITRES);
            }
            // The high bits of fill are zero for a stereo split, so the side is
            // never folded.
            cm |= quant_band(
                ctx,
                coder,
                y,
                n,
                sbits,
                big_b,
                None,
                lm,
                None,
                side,
                None,
                fill >> big_b,
            );
        } else {
            cm = quant_band(
                ctx,
                coder,
                y,
                n,
                sbits,
                big_b,
                None,
                lm,
                None,
                side,
                None,
                fill >> big_b,
            );
            rebalance = sbits - (rebalance - ctx.remaining_bits);
            if rebalance > 3 << BITRES && itheta != 16384 {
                mbits += rebalance - (3 << BITRES);
            }
            cm |= quant_band(
                ctx,
                coder,
                x,
                n,
                mbits,
                big_b,
                lowband,
                lm,
                lowband_out,
                1.0,
                lowband_scratch,
                fill,
            );
        }
    }

    if ctx.resynth {
        if n != 2 {
            stereo_merge(x, y, mid, n as usize);
        }
        if inv {
            for v in y.iter_mut().take(n as usize) {
                *v = -*v;
            }
        }
    }
    cm
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rate::compute_pulse_cache;

    const EBAND5MS: [i16; 22] = [
        0, 1, 2, 3, 4, 5, 6, 7, 8, 10, 12, 14, 16, 20, 24, 28, 34, 40, 48, 60, 78, 100,
    ];
    const LOGN400: [i16; 21] = [
        0, 0, 0, 0, 0, 0, 0, 0, 8, 8, 8, 8, 16, 16, 16, 21, 21, 24, 29, 34, 36,
    ];
    const NB_E_BANDS: usize = 21;

    fn make_ctx<'a>(
        cache: &'a PulseCache,
        band_e: &'a [f32],
        i: usize,
        tf_change: i32,
        intensity: usize,
    ) -> QuantCtx<'a> {
        QuantCtx {
            cache,
            e_bands: &EBAND5MS,
            log_n: &LOGN400,
            nb_e_bands: NB_E_BANDS,
            i,
            intensity,
            spread: 2,
            tf_change,
            remaining_bits: 2000,
            seed: 42,
            resynth: true,
            band_e,
            theta_round: 0,
            avoid_split_noise: false,
            disable_inv: false,
        }
    }

    fn band_n(i: usize, lm: i32) -> usize {
        ((EBAND5MS[i + 1] - EBAND5MS[i]) as usize) << lm
    }

    /// Round-trips a mono band through encode/decode and asserts the decoder
    /// reconstructs exactly the encoder's resynthesized spectrum.
    fn round_trip_mono(i: usize, lm: i32, b: i32, tf_change: i32, seed: f32) {
        let cache = compute_pulse_cache(&EBAND5MS, &LOGN400, NB_E_BANDS, lm);
        let n = band_n(i, lm);
        let band_e = vec![1.0f32; NB_E_BANDS * 2];
        let big_b = 1 << lm;

        let mut x_enc: Vec<f32> = (0..n)
            .map(|j| ((j as f32 + 1.0) * seed).sin() - 0.2 * (j as f32 * 0.3).cos())
            .collect();
        // Normalise the band to unit energy (as the normalised spectrum is).
        let norm = x_enc.iter().map(|v| v * v).sum::<f32>().sqrt();
        for v in &mut x_enc {
            *v /= norm;
        }

        let mut ctx = make_ctx(&cache, &band_e, i, tf_change, 8);
        let mut enc = RangeEncoder::new(512);
        let mut coder = Coder::Enc(&mut enc);
        let cm_e = quant_band(
            &mut ctx,
            &mut coder,
            &mut x_enc,
            n as i32,
            b,
            big_b,
            None,
            lm,
            None,
            1.0,
            None,
            (1 << big_b) - 1,
        );
        let rem_e = ctx.remaining_bits;
        let seed_e = ctx.seed;
        let bytes = enc.done();

        let mut x_dec = vec![0.0f32; n];
        let mut ctx_d = make_ctx(&cache, &band_e, i, tf_change, 8);
        let mut dec = RangeDecoder::new(&bytes);
        let mut coder_d = Coder::Dec(&mut dec);
        let cm_d = quant_band(
            &mut ctx_d,
            &mut coder_d,
            &mut x_dec,
            n as i32,
            b,
            big_b,
            None,
            lm,
            None,
            1.0,
            None,
            (1 << big_b) - 1,
        );

        assert_eq!(
            cm_e, cm_d,
            "collapse mask (band {i}, lm {lm}, tf {tf_change})"
        );
        assert_eq!(rem_e, ctx_d.remaining_bits, "remaining bits");
        assert_eq!(seed_e, ctx_d.seed, "seed");
        for j in 0..n {
            assert_eq!(
                x_enc[j].to_bits(),
                x_dec[j].to_bits(),
                "bin {j} (band {i}, lm {lm}, tf {tf_change}): enc={} dec={}",
                x_enc[j],
                x_dec[j],
            );
        }
    }

    #[test]
    fn mono_band_round_trips_plain() {
        // Long block, no transient reorder: exercises the split + PVQ leaf path.
        round_trip_mono(17, 3, 240, 0, 0.37);
        round_trip_mono(15, 3, 160, 0, 0.81);
        round_trip_mono(20, 3, 400, 0, 0.13);
    }

    #[test]
    fn mono_band_round_trips_low_budget() {
        // A tiny budget drives the no-pulse fold/noise fill path.
        round_trip_mono(18, 3, 16, 0, 0.55);
    }

    #[test]
    fn mono_band_round_trips_recombine() {
        // tf_change > 0 exercises band recombining (haar1 + bit interleave).
        round_trip_mono(19, 3, 300, 1, 0.42);
        round_trip_mono(19, 3, 300, 2, 0.66);
    }

    #[test]
    fn mono_band_round_trips_time_divide() {
        // tf_change < 0 exercises the time-resolution increase.
        round_trip_mono(20, 3, 360, -1, 0.29);
    }

    /// Round-trips a stereo band and asserts both channels reconstruct exactly.
    fn round_trip_stereo(i: usize, lm: i32, b: i32, seed: f32) {
        let cache = compute_pulse_cache(&EBAND5MS, &LOGN400, NB_E_BANDS, lm);
        let n = band_n(i, lm);
        // Distinct per-channel energies so intensity stereo has something to do.
        let mut band_e = vec![1.0f32; NB_E_BANDS * 2];
        band_e[i] = 1.3;
        band_e[i + NB_E_BANDS] = 0.7;
        let big_b = 1 << lm;

        let mk = |s: f32, ph: f32| -> Vec<f32> {
            let mut v: Vec<f32> = (0..n).map(|j| ((j as f32 + 1.0) * s + ph).sin()).collect();
            let norm = v.iter().map(|a| a * a).sum::<f32>().sqrt();
            for a in &mut v {
                *a /= norm;
            }
            v
        };
        let mut x_enc = mk(seed, 0.0);
        let mut y_enc = mk(seed * 1.7, 0.9);

        let mut ctx = make_ctx(&cache, &band_e, i, 0, NB_E_BANDS);
        let mut enc = RangeEncoder::new(512);
        let mut coder = Coder::Enc(&mut enc);
        let cm_e = quant_band_stereo(
            &mut ctx,
            &mut coder,
            &mut x_enc,
            &mut y_enc,
            n as i32,
            b,
            big_b,
            None,
            lm,
            None,
            None,
            (1 << big_b) - 1,
        );
        let rem_e = ctx.remaining_bits;
        let bytes = enc.done();

        let mut x_dec = vec![0.0f32; n];
        let mut y_dec = vec![0.0f32; n];
        let mut ctx_d = make_ctx(&cache, &band_e, i, 0, NB_E_BANDS);
        let mut dec = RangeDecoder::new(&bytes);
        let mut coder_d = Coder::Dec(&mut dec);
        let cm_d = quant_band_stereo(
            &mut ctx_d,
            &mut coder_d,
            &mut x_dec,
            &mut y_dec,
            n as i32,
            b,
            big_b,
            None,
            lm,
            None,
            None,
            (1 << big_b) - 1,
        );

        assert_eq!(cm_e, cm_d, "collapse mask (stereo band {i})");
        assert_eq!(rem_e, ctx_d.remaining_bits, "remaining bits");
        for j in 0..n {
            assert_eq!(x_enc[j].to_bits(), x_dec[j].to_bits(), "X bin {j} band {i}");
            assert_eq!(y_enc[j].to_bits(), y_dec[j].to_bits(), "Y bin {j} band {i}");
        }
    }

    #[test]
    fn stereo_band_round_trips() {
        round_trip_stereo(17, 3, 300, 0.37);
        round_trip_stereo(20, 3, 500, 0.61);
    }

    #[test]
    fn stereo_n2_band_round_trips() {
        // N=2 stereo uses the orthogonal single-sign-bit special case
        // (band 1 has width 1, so lm=1 gives N = 1<<1 = 2).
        round_trip_stereo(1, 1, 120, 0.44);
    }
}
