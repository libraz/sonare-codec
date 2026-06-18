//! Opus range coder (entropy coder).
//!
//! Hand-ported to safe Rust from libopus `celt/entcode.c`, `celt/entenc.c`, and
//! `celt/entdec.c` (the Martin range coder used by CELT/SILK). Derivative work
//! of libopus (BSD-3-Clause); see `LICENSE-THIRDPARTY`.
//!
//! C relies on modular `u32` arithmetic throughout; the port uses `wrapping_*`
//! at exactly those sites so the bitstream stays bit-identical to libopus while
//! avoiding Rust's debug overflow panics.

// Foundational primitive for the Opus port; not yet wired into the live encoder.
#![allow(dead_code)]

const EC_SYM_BITS: u32 = 8;
const EC_CODE_BITS: u32 = 32;
const EC_SYM_MAX: u32 = (1 << EC_SYM_BITS) - 1; // 255
const EC_CODE_SHIFT: u32 = EC_CODE_BITS - EC_SYM_BITS - 1; // 23
const EC_CODE_TOP: u32 = 1u32 << (EC_CODE_BITS - 1); // 0x8000_0000
const EC_CODE_BOT: u32 = EC_CODE_TOP >> EC_SYM_BITS; // 0x0080_0000
const EC_CODE_EXTRA: u32 = (EC_CODE_BITS - 2) % EC_SYM_BITS + 1; // 7
const EC_UINT_BITS: u32 = 8;
const EC_WINDOW_SIZE: u32 = 32;

/// `ec_ilog`: number of significant bits in `v` (`ec_ilog(0) == 0`).
fn ec_ilog(v: u32) -> u32 {
    EC_CODE_BITS - v.leading_zeros()
}

/// Rounding table for the sub-bit fraction in `ec_tell_frac`.
const EC_TELL_FRAC_CORRECTION: [u32; 8] = [35733, 38967, 42495, 46340, 50535, 55109, 60097, 65535];

/// `ec_tell_frac`: bits used/consumed so far in eighth-of-a-bit (`BITRES = 3`)
/// resolution, shared by the encoder and decoder (both expose `nbits_total`
/// and `rng`). Bit-exact with libopus `entcode.c`.
fn ec_tell_frac_impl(nbits_total: i32, rng: u32) -> u32 {
    let nbits = (nbits_total as u32) << 3;
    let l = ec_ilog(rng);
    let r = rng >> (l - 16);
    let mut b = (r >> 12) - 8;
    if r > EC_TELL_FRAC_CORRECTION[b as usize] {
        b += 1;
    }
    let l = (l << 3) + b;
    nbits - l
}

/// Range encoder, equivalent to libopus `ec_enc`.
#[derive(Clone)]
pub struct RangeEncoder {
    buf: Vec<u8>,
    storage: u32,
    end_offs: u32,
    end_window: u32,
    nend_bits: i32,
    nbits_total: i32,
    offs: u32,
    rng: u32,
    val: u32,
    /// Number of outstanding carry-propagating symbols.
    ext: u32,
    /// Buffered output symbol awaiting carry propagation (`-1` = none).
    rem: i32,
    error: bool,
}

impl RangeEncoder {
    #[must_use]
    pub fn new(size: u32) -> Self {
        Self {
            buf: vec![0u8; size as usize],
            storage: size,
            end_offs: 0,
            end_window: 0,
            nend_bits: 0,
            nbits_total: (EC_CODE_BITS + 1) as i32,
            offs: 0,
            rng: EC_CODE_TOP,
            val: 0,
            ext: 0,
            rem: -1,
            error: false,
        }
    }

    #[must_use]
    pub fn is_error(&self) -> bool {
        self.error
    }

    /// Number of whole range-coder bytes written so far.
    #[must_use]
    pub fn range_bytes(&self) -> u32 {
        self.offs
    }

    /// `ec_tell`: bits used so far (rounded up to the next whole bit).
    #[must_use]
    pub fn ec_tell(&self) -> i32 {
        self.nbits_total - ec_ilog(self.rng) as i32
    }

    /// `ec_tell_frac`: bits used so far in eighth-of-a-bit resolution.
    #[must_use]
    pub fn ec_tell_frac(&self) -> u32 {
        ec_tell_frac_impl(self.nbits_total, self.rng)
    }

    /// The current range register (libopus `enc->rng`); CELT carries this across
    /// frames as the PVQ noise seed.
    #[must_use]
    pub fn rng(&self) -> u32 {
        self.rng
    }

    /// Total buffer capacity expressed in bits (`storage * 8`).
    #[must_use]
    pub fn storage_bits(&self) -> u32 {
        self.storage * 8
    }

    fn write_byte(&mut self, value: u32) -> bool {
        if self.offs + self.end_offs >= self.storage {
            return true;
        }
        self.buf[self.offs as usize] = value as u8;
        self.offs += 1;
        false
    }

    fn write_byte_at_end(&mut self, value: u32) -> bool {
        if self.offs + self.end_offs >= self.storage {
            return true;
        }
        self.end_offs += 1;
        self.buf[(self.storage - self.end_offs) as usize] = value as u8;
        false
    }

    fn carry_out(&mut self, c: i32) {
        if c as u32 != EC_SYM_MAX {
            let carry = c >> EC_SYM_BITS;
            if self.rem >= 0 {
                let byte = (self.rem + carry) as u32;
                self.error |= self.write_byte(byte);
            }
            if self.ext > 0 {
                let sym = (EC_SYM_MAX.wrapping_add(carry as u32)) & EC_SYM_MAX;
                loop {
                    self.error |= self.write_byte(sym);
                    self.ext -= 1;
                    if self.ext == 0 {
                        break;
                    }
                }
            }
            self.rem = c & EC_SYM_MAX as i32;
        } else {
            self.ext += 1;
        }
    }

    fn normalize(&mut self) {
        while self.rng <= EC_CODE_BOT {
            self.carry_out((self.val >> EC_CODE_SHIFT) as i32);
            self.val = self.val.wrapping_shl(EC_SYM_BITS) & (EC_CODE_TOP - 1);
            self.rng = self.rng.wrapping_shl(EC_SYM_BITS);
            self.nbits_total += EC_SYM_BITS as i32;
        }
    }

    /// Encodes a symbol with cumulative frequencies `[fl, fh)` of total `ft`.
    pub fn encode(&mut self, fl: u32, fh: u32, ft: u32) {
        let r = self.rng / ft;
        if fl > 0 {
            self.val = self
                .val
                .wrapping_add(self.rng.wrapping_sub(r.wrapping_mul(ft - fl)));
            self.rng = r.wrapping_mul(fh - fl);
        } else {
            self.rng = self.rng.wrapping_sub(r.wrapping_mul(ft - fh));
        }
        self.normalize();
    }

    /// Encodes a symbol with total frequency `1 << bits`.
    pub fn encode_bin(&mut self, fl: u32, fh: u32, bits: u32) {
        let r = self.rng >> bits;
        if fl > 0 {
            self.val = self
                .val
                .wrapping_add(self.rng.wrapping_sub(r.wrapping_mul((1u32 << bits) - fl)));
            self.rng = r.wrapping_mul(fh - fl);
        } else {
            self.rng = self.rng.wrapping_sub(r.wrapping_mul((1u32 << bits) - fh));
        }
        self.normalize();
    }

    /// Encodes one bit with probability of a one equal to `1/(1<<logp)`.
    pub fn enc_bit_logp(&mut self, val: bool, logp: u32) {
        let r = self.rng;
        let l = self.val;
        let s = r >> logp;
        let r = r - s;
        if val {
            self.val = l.wrapping_add(r);
        }
        self.rng = if val { s } else { r };
        self.normalize();
    }

    /// Encodes symbol `s` against an inverse-CDF table with `ftb`-bit total.
    pub fn enc_icdf(&mut self, s: usize, icdf: &[u8], ftb: u32) {
        let r = self.rng >> ftb;
        if s > 0 {
            let prev = u32::from(icdf[s - 1]);
            self.val = self
                .val
                .wrapping_add(self.rng.wrapping_sub(r.wrapping_mul(prev)));
            self.rng = r.wrapping_mul(prev - u32::from(icdf[s]));
        } else {
            self.rng = self.rng.wrapping_sub(r.wrapping_mul(u32::from(icdf[s])));
        }
        self.normalize();
    }

    /// Encodes a raw unsigned integer in `[0, ft)`.
    pub fn enc_uint(&mut self, fl: u32, ft: u32) {
        debug_assert!(ft > 1);
        let ft = ft - 1;
        let ftb = ec_ilog(ft);
        if ftb > EC_UINT_BITS {
            let ftb = ftb - EC_UINT_BITS;
            let ftn = (ft >> ftb) + 1;
            let fln = fl >> ftb;
            self.encode(fln, fln + 1, ftn);
            self.enc_bits(fl & ((1u32 << ftb) - 1), ftb);
        } else {
            self.encode(fl, fl + 1, ft + 1);
        }
    }

    /// Encodes `bits` raw bits, buffered at the end of the stream.
    pub fn enc_bits(&mut self, fl: u32, bits: u32) {
        let mut window = self.end_window;
        let mut used = self.nend_bits;
        debug_assert!(bits > 0);
        if used as u32 + bits > EC_WINDOW_SIZE {
            loop {
                self.error |= self.write_byte_at_end(window & EC_SYM_MAX);
                window >>= EC_SYM_BITS;
                used -= EC_SYM_BITS as i32;
                if used < EC_SYM_BITS as i32 {
                    break;
                }
            }
        }
        window |= fl << used;
        used += bits as i32;
        self.end_window = window;
        self.nend_bits = used;
        self.nbits_total += bits as i32;
    }

    /// Finalizes the stream and returns the encoded buffer.
    pub fn done(mut self) -> Vec<u8> {
        let mut l: i32 = EC_CODE_BITS as i32 - ec_ilog(self.rng) as i32;
        let mut msk: u32 = (EC_CODE_TOP - 1) >> l;
        let mut end: u32 = self.val.wrapping_add(msk) & !msk;
        if (end | msk) >= self.val.wrapping_add(self.rng) {
            l += 1;
            msk >>= 1;
            end = self.val.wrapping_add(msk) & !msk;
        }
        while l > 0 {
            self.carry_out((end >> EC_CODE_SHIFT) as i32);
            end = end.wrapping_shl(EC_SYM_BITS) & (EC_CODE_TOP - 1);
            l -= EC_SYM_BITS as i32;
        }
        if self.rem >= 0 || self.ext > 0 {
            self.carry_out(0);
        }
        let mut window = self.end_window;
        let mut used = self.nend_bits;
        while used >= EC_SYM_BITS as i32 {
            self.error |= self.write_byte_at_end(window & EC_SYM_MAX);
            window >>= EC_SYM_BITS;
            used -= EC_SYM_BITS as i32;
        }
        if !self.error {
            let clear_from = self.offs as usize;
            let clear_to = (self.storage - self.end_offs) as usize;
            for byte in &mut self.buf[clear_from..clear_to] {
                *byte = 0;
            }
            if used > 0 {
                if self.end_offs >= self.storage {
                    self.error = true;
                } else {
                    l = -l;
                    if self.offs + self.end_offs >= self.storage && l < used {
                        window &= (1u32 << l) - 1;
                        self.error = true;
                    }
                    let idx = (self.storage - self.end_offs - 1) as usize;
                    self.buf[idx] |= window as u8;
                }
            }
        }
        self.buf
    }
}

/// Range decoder, equivalent to libopus `ec_dec`.
pub struct RangeDecoder<'a> {
    buf: &'a [u8],
    storage: u32,
    end_offs: u32,
    end_window: u32,
    nend_bits: i32,
    nbits_total: i32,
    offs: u32,
    rng: u32,
    val: u32,
    /// Saved normalization factor from the last `decode`.
    ext: u32,
    rem: i32,
    error: bool,
}

impl<'a> RangeDecoder<'a> {
    #[must_use]
    pub fn new(buf: &'a [u8]) -> Self {
        let storage = buf.len() as u32;
        let mut this = Self {
            buf,
            storage,
            end_offs: 0,
            end_window: 0,
            nend_bits: 0,
            nbits_total: (EC_CODE_BITS + 1
                - ((EC_CODE_BITS - EC_CODE_EXTRA) / EC_SYM_BITS) * EC_SYM_BITS)
                as i32,
            offs: 0,
            rng: 1u32 << EC_CODE_EXTRA,
            val: 0,
            ext: 0,
            rem: 0,
            error: false,
        };
        this.rem = this.read_byte();
        this.val = this.rng - 1 - (this.rem as u32 >> (EC_SYM_BITS - EC_CODE_EXTRA));
        this.normalize();
        this
    }

    /// `ec_tell`: bits consumed so far (rounded up to the next whole bit).
    #[must_use]
    pub fn ec_tell(&self) -> i32 {
        self.nbits_total - ec_ilog(self.rng) as i32
    }

    /// `ec_tell_frac`: bits consumed so far in eighth-of-a-bit resolution.
    #[must_use]
    pub fn ec_tell_frac(&self) -> u32 {
        ec_tell_frac_impl(self.nbits_total, self.rng)
    }

    /// The current range register (libopus `dec->rng`); CELT carries this across
    /// frames as the PVQ noise seed, mirroring the encoder.
    #[must_use]
    pub fn rng(&self) -> u32 {
        self.rng
    }

    /// Total capacity of the backing buffer in bytes (`ec_dec.storage`).
    #[must_use]
    pub fn storage(&self) -> u32 {
        self.storage
    }

    /// Total buffer capacity expressed in bits (`storage * 8`).
    #[must_use]
    pub fn storage_bits(&self) -> u32 {
        self.storage * 8
    }

    fn read_byte(&mut self) -> i32 {
        if self.offs < self.storage {
            let b = self.buf[self.offs as usize];
            self.offs += 1;
            i32::from(b)
        } else {
            0
        }
    }

    fn read_byte_from_end(&mut self) -> i32 {
        if self.end_offs < self.storage {
            self.end_offs += 1;
            i32::from(self.buf[(self.storage - self.end_offs) as usize])
        } else {
            0
        }
    }

    fn normalize(&mut self) {
        while self.rng <= EC_CODE_BOT {
            self.nbits_total += EC_SYM_BITS as i32;
            self.rng = self.rng.wrapping_shl(EC_SYM_BITS);
            let sym0 = self.rem;
            self.rem = self.read_byte();
            let sym = ((sym0 << EC_SYM_BITS | self.rem) >> (EC_SYM_BITS - EC_CODE_EXTRA)) as u32;
            self.val = (self
                .val
                .wrapping_shl(EC_SYM_BITS)
                .wrapping_add(EC_SYM_MAX & !sym))
                & (EC_CODE_TOP - 1);
        }
    }

    /// Returns the cumulative frequency of the next symbol given total `ft`.
    pub fn decode(&mut self, ft: u32) -> u32 {
        self.ext = self.rng / ft;
        let s = self.val / self.ext;
        ft - (s + 1).min(ft)
    }

    /// As [`decode`](Self::decode) with total frequency `1 << bits`.
    pub fn decode_bin(&mut self, bits: u32) -> u32 {
        self.ext = self.rng >> bits;
        let s = self.val / self.ext;
        (1u32 << bits) - (s + 1).min(1u32 << bits)
    }

    /// Advances past a symbol with cumulative frequencies `[fl, fh)`, total `ft`.
    pub fn dec_update(&mut self, fl: u32, fh: u32, ft: u32) {
        let s = self.ext.wrapping_mul(ft - fh);
        self.val = self.val.wrapping_sub(s);
        self.rng = if fl > 0 {
            self.ext.wrapping_mul(fh - fl)
        } else {
            self.rng - s
        };
        self.normalize();
    }

    /// Decodes one bit with probability of a one equal to `1/(1<<logp)`.
    pub fn dec_bit_logp(&mut self, logp: u32) -> bool {
        let r = self.rng;
        let d = self.val;
        let s = r >> logp;
        let ret = d < s;
        if !ret {
            self.val = d - s;
        }
        self.rng = if ret { s } else { r - s };
        self.normalize();
        ret
    }

    /// Decodes a symbol against an inverse-CDF table with `ftb`-bit total.
    pub fn dec_icdf(&mut self, icdf: &[u8], ftb: u32) -> usize {
        let mut s = self.rng;
        let d = self.val;
        let r = s >> ftb;
        let mut ret: isize = -1;
        let mut t;
        loop {
            t = s;
            ret += 1;
            s = r.wrapping_mul(u32::from(icdf[ret as usize]));
            if d >= s {
                break;
            }
        }
        self.val = d - s;
        self.rng = t - s;
        self.normalize();
        ret as usize
    }

    /// Decodes a raw unsigned integer in `[0, ft)`.
    pub fn dec_uint(&mut self, ft: u32) -> u32 {
        debug_assert!(ft > 1);
        let ft = ft - 1;
        let ftb = ec_ilog(ft);
        if ftb > EC_UINT_BITS {
            let ftb = ftb - EC_UINT_BITS;
            let ftn = (ft >> ftb) + 1;
            let s = self.decode(ftn);
            self.dec_update(s, s + 1, ftn);
            let t = (s << ftb) | self.dec_bits(ftb);
            if t <= ft {
                t
            } else {
                self.error = true;
                ft
            }
        } else {
            let ft = ft + 1;
            let s = self.decode(ft);
            self.dec_update(s, s + 1, ft);
            s
        }
    }

    /// Decodes `bits` raw bits from the end of the stream.
    pub fn dec_bits(&mut self, bits: u32) -> u32 {
        let mut window = self.end_window;
        let mut available = self.nend_bits;
        if (available as u32) < bits {
            loop {
                window |= (self.read_byte_from_end() as u32) << available;
                available += EC_SYM_BITS as i32;
                if available > (EC_WINDOW_SIZE - EC_SYM_BITS) as i32 {
                    break;
                }
            }
        }
        let ret = window & ((1u32 << bits) - 1);
        window >>= bits;
        available -= bits as i32;
        self.end_window = window;
        self.nend_bits = available;
        self.nbits_total += bits as i32;
        ret
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Cumulative-frequency model: symbol -> (fl, fh); total `FT`.
    const FT: u32 = 8;
    const MODEL: [(u32, u32); 4] = [(0, 3), (3, 4), (4, 6), (6, 8)];

    fn symbol_for(fs: u32) -> usize {
        MODEL
            .iter()
            .position(|&(fl, fh)| fl <= fs && fs < fh)
            .unwrap()
    }

    #[test]
    fn round_trips_frequency_symbols() {
        let symbols = [0usize, 2, 3, 1, 0, 0, 3, 2, 1, 2, 3, 0];
        let mut enc = RangeEncoder::new(64);
        for &s in &symbols {
            let (fl, fh) = MODEL[s];
            enc.encode(fl, fh, FT);
        }
        let bytes = enc.done();

        let mut dec = RangeDecoder::new(&bytes);
        for &s in &symbols {
            let fs = dec.decode(FT);
            let got = symbol_for(fs);
            let (fl, fh) = MODEL[got];
            dec.dec_update(fl, fh, FT);
            assert_eq!(got, s);
        }
    }

    #[test]
    fn ec_tell_frac_fresh_encoder_is_one_bit() {
        // A fresh encoder has used exactly one bit: 8 eighths.
        let enc = RangeEncoder::new(64);
        assert_eq!(enc.ec_tell_frac(), 8);
        assert_eq!(enc.ec_tell(), 1);
    }

    #[test]
    fn ec_tell_frac_matches_between_encoder_and_decoder() {
        // The fractional tell is symmetric: at every step the decoder has
        // consumed exactly as many eighth-bits as the encoder produced.
        let symbols = [0usize, 2, 3, 1, 0, 3, 2, 1, 2, 3];
        let mut enc = RangeEncoder::new(64);
        let mut enc_tells = Vec::new();
        for &s in &symbols {
            let (fl, fh) = MODEL[s];
            enc.encode(fl, fh, FT);
            enc_tells.push(enc.ec_tell_frac());
            // ec_tell is the ceiling of ec_tell_frac in whole bits.
            assert_eq!(enc.ec_tell(), ((enc.ec_tell_frac() + 7) >> 3) as i32);
        }
        let bytes = enc.done();

        let mut dec = RangeDecoder::new(&bytes);
        for (&s, &expected) in symbols.iter().zip(&enc_tells) {
            let fs = dec.decode(FT);
            let (fl, fh) = MODEL[symbol_for(fs)];
            dec.dec_update(fl, fh, FT);
            assert_eq!(dec.ec_tell_frac(), expected);
            let _ = s;
        }
    }

    #[test]
    fn round_trips_bit_logp() {
        let bits = [true, false, false, true, true, false, true, false, false];
        let logp = 3;
        let mut enc = RangeEncoder::new(64);
        for &b in &bits {
            enc.enc_bit_logp(b, logp);
        }
        let bytes = enc.done();

        let mut dec = RangeDecoder::new(&bytes);
        for &b in &bits {
            assert_eq!(dec.dec_bit_logp(logp), b);
        }
    }

    #[test]
    fn round_trips_icdf() {
        // 3 symbols, ftb = 4 (ft = 16), freqs [8,4,4] -> icdf [8,4,0].
        let icdf = [8u8, 4, 0];
        let ftb = 4;
        let symbols = [0usize, 1, 2, 2, 0, 1, 0, 2, 1];
        let mut enc = RangeEncoder::new(64);
        for &s in &symbols {
            enc.enc_icdf(s, &icdf, ftb);
        }
        let bytes = enc.done();

        let mut dec = RangeDecoder::new(&bytes);
        for &s in &symbols {
            assert_eq!(dec.dec_icdf(&icdf, ftb), s);
        }
    }

    #[test]
    fn round_trips_uint_and_raw_bits() {
        let values: [(u32, u32); 5] = [
            (12_345, 65_536),
            (7, 8),
            (1_000_000, 1_048_576),
            (0, 2),
            (255, 256),
        ];
        let raw: [(u32, u32); 3] = [(0b1011, 4), (0x3FF, 10), (1, 1)];

        let mut enc = RangeEncoder::new(128);
        for &(v, ft) in &values {
            enc.enc_uint(v, ft);
        }
        for &(v, b) in &raw {
            enc.enc_bits(v, b);
        }
        let bytes = enc.done();

        let mut dec = RangeDecoder::new(&bytes);
        for &(v, ft) in &values {
            assert_eq!(dec.dec_uint(ft), v);
        }
        for &(v, b) in &raw {
            assert_eq!(dec.dec_bits(b), v);
        }
    }

    #[test]
    fn ec_ilog_matches_reference() {
        assert_eq!(ec_ilog(0), 0);
        assert_eq!(ec_ilog(1), 1);
        assert_eq!(ec_ilog(2), 2);
        assert_eq!(ec_ilog(255), 8);
        assert_eq!(ec_ilog(256), 9);
        assert_eq!(ec_ilog(0xFFFF_FFFF), 32);
    }
}
