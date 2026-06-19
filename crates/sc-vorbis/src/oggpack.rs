//! Ogg LSB-first bit packing.
//!
//! Hand-ported to safe Rust from libogg `src/bitwise.c` — specifically the
//! little-endian `oggpack_*` variant that Vorbis uses to assemble packet
//! payloads. Derivative work of libogg (BSD-3-Clause); see `LICENSE-THIRDPARTY`.
//!
//! The bit order matches libogg exactly: the first bit written occupies the
//! least-significant bit of the first output byte, so a port that round-trips
//! against [`BitReader`] also reproduces the on-the-wire layout libvorbis emits.

// `BitReader` and the writer alignment helpers are the decode-direction
// counterparts, exercised by every module's round-trip tests.
#![allow(dead_code)]

/// Returns a mask keeping the low `bits` bits, for `bits` in `0..=32`.
fn low_mask(bits: u32) -> u64 {
    debug_assert!(bits <= 32);
    if bits == 0 {
        0
    } else {
        (1u64 << bits) - 1
    }
}

/// LSB-first bit writer equivalent to libogg's `oggpack_buffer` write side.
#[derive(Clone, Default)]
pub struct BitWriter {
    /// Backing storage; bytes at and beyond `endbyte` may be staging area.
    bytes: Vec<u8>,
    /// Index of the byte currently being filled.
    endbyte: usize,
    /// Bits already consumed in `bytes[endbyte]`, in `0..8`.
    endbit: u32,
}

impl BitWriter {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends the low `bits` bits of `value`, LSB first. `bits` must be `<= 32`.
    pub fn write(&mut self, value: u32, bits: u32) {
        debug_assert!(bits <= 32);
        if bits == 0 {
            return;
        }
        let value = u64::from(value) & low_mask(bits);

        // The store touches `endbyte ..= endbyte + 4`; keep those bytes present
        // and zero-initialized so the high-byte assignments below are in range.
        if self.bytes.len() < self.endbyte + 5 {
            self.bytes.resize(self.endbyte + 5, 0);
        }

        let endbit = self.endbit;
        let total = bits + endbit;
        let i = self.endbyte;

        self.bytes[i] |= (value << endbit) as u8;
        if total >= 8 {
            self.bytes[i + 1] = (value >> (8 - endbit)) as u8;
            if total >= 16 {
                self.bytes[i + 2] = (value >> (16 - endbit)) as u8;
                if total >= 24 {
                    self.bytes[i + 3] = (value >> (24 - endbit)) as u8;
                    if total >= 32 {
                        self.bytes[i + 4] = if endbit != 0 {
                            (value >> (32 - endbit)) as u8
                        } else {
                            0
                        };
                    }
                }
            }
        }

        self.endbyte += (total >> 3) as usize;
        self.endbit = total & 7;
    }

    /// Pads with zero bits up to the next byte boundary.
    pub fn align(&mut self) {
        let pad = 8 - self.endbit;
        if pad < 8 {
            self.write(0, pad);
        }
    }

    /// Total number of bits written.
    #[must_use]
    pub fn bit_len(&self) -> usize {
        self.endbyte * 8 + self.endbit as usize
    }

    /// Number of bytes the written bits occupy (rounding up a partial byte).
    #[must_use]
    pub fn byte_len(&self) -> usize {
        self.endbyte + usize::from(self.endbit > 0)
    }

    /// Consumes the writer and returns the packed bytes.
    #[must_use]
    pub fn into_bytes(mut self) -> Vec<u8> {
        let len = self.byte_len();
        self.bytes.truncate(len);
        self.bytes
    }
}

/// LSB-first bit reader equivalent to libogg's `oggpack_read`.
///
/// Bytes past the end of the input read as zero, so callers stay responsible
/// for not over-reading; this mirrors how the port verifies round trips.
pub struct BitReader<'a> {
    data: &'a [u8],
    endbyte: usize,
    endbit: u32,
}

impl<'a> BitReader<'a> {
    #[must_use]
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            endbyte: 0,
            endbit: 0,
        }
    }

    fn byte(&self, index: usize) -> u64 {
        u64::from(self.data.get(index).copied().unwrap_or(0))
    }

    /// Reads `bits` bits (LSB first) and advances. `bits` must be `<= 32`.
    pub fn read(&mut self, bits: u32) -> u32 {
        debug_assert!(bits <= 32);
        if bits == 0 {
            return 0;
        }
        let endbit = self.endbit;
        let i = self.endbyte;

        let mut ret = self.byte(i) >> endbit;
        if bits > 8 - endbit {
            ret |= self.byte(i + 1) << (8 - endbit);
            if bits > 16 - endbit {
                ret |= self.byte(i + 2) << (16 - endbit);
                if bits > 24 - endbit {
                    ret |= self.byte(i + 3) << (24 - endbit);
                    if bits > 32 - endbit && endbit != 0 {
                        ret |= self.byte(i + 4) << (32 - endbit);
                    }
                }
            }
        }

        let total = endbit + bits;
        self.endbyte += (total >> 3) as usize;
        self.endbit = total & 7;
        (ret & low_mask(bits)) as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_lsb_first_byte_layout() {
        let mut w = BitWriter::new();
        // 0b101 (=5) then 0b11 (=3): first bit is the LSB of byte 0.
        w.write(0b101, 3);
        w.write(0b11, 2);
        assert_eq!(w.bit_len(), 5);
        assert_eq!(w.byte_len(), 1);
        // byte0 = 3<<3 | 5 = 0b11101 = 0x1D
        assert_eq!(w.into_bytes(), vec![0x1D]);
    }

    #[test]
    fn round_trips_mixed_widths() {
        let fields = [
            (0u32, 1u32),
            (1, 1),
            (5, 3),
            (0xABCD, 16),
            (0, 5),
            (0x3F, 6),
        ];
        let mut w = BitWriter::new();
        for &(value, bits) in &fields {
            w.write(value, bits);
        }
        let bytes = w.into_bytes();

        let mut r = BitReader::new(&bytes);
        for &(value, bits) in &fields {
            assert_eq!(r.read(bits), value, "field {value:#x}/{bits}");
        }
    }

    #[test]
    fn round_trips_full_width_values_at_offsets() {
        for offset in 0..8u32 {
            let mut w = BitWriter::new();
            if offset > 0 {
                w.write(0, offset);
            }
            w.write(0xDEAD_BEEF, 32);
            w.write(0x7, 3);
            let bytes = w.into_bytes();

            let mut r = BitReader::new(&bytes);
            if offset > 0 {
                assert_eq!(r.read(offset), 0);
            }
            assert_eq!(r.read(32), 0xDEAD_BEEF, "offset {offset}");
            assert_eq!(r.read(3), 0x7);
        }
    }

    #[test]
    fn align_pads_to_byte_boundary() {
        let mut w = BitWriter::new();
        w.write(0b1, 1);
        w.align();
        assert_eq!(w.bit_len(), 8);
        w.write(0xFF, 8);
        assert_eq!(w.into_bytes(), vec![0x01, 0xFF]);
    }
}
