use super::*;

#[rustfmt::skip]
pub(crate) const MPEG1_LAYER3_BIG_VALUE_TABLE_1: &[HuffmanEntry<Layer3BigValueMagnitude>] = &[
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 0, y: 0 },
        code: HuffmanCode { bits: 0b1, len: 1 },
    },
    HuffmanEntry { symbol: Layer3BigValueMagnitude { x: 0, y: 1 }, code: HuffmanCode { bits: 0b001, len: 3 } },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 1, y: 0 },
        code: HuffmanCode { bits: 0b01, len: 2 },
    },
    HuffmanEntry { symbol: Layer3BigValueMagnitude { x: 1, y: 1 }, code: HuffmanCode { bits: 0b000, len: 3 } },
];

#[rustfmt::skip]
pub(crate) const MPEG1_LAYER3_BIG_VALUE_TABLE_2: &[HuffmanEntry<Layer3BigValueMagnitude>] = &[
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 0, y: 0 },
        code: HuffmanCode { bits: 0b1, len: 1 },
    },
    HuffmanEntry { symbol: Layer3BigValueMagnitude { x: 0, y: 1 }, code: HuffmanCode { bits: 0b010, len: 3 } },
    HuffmanEntry { symbol: Layer3BigValueMagnitude { x: 0, y: 2 }, code: HuffmanCode { bits: 0b000001, len: 6 } },
    HuffmanEntry { symbol: Layer3BigValueMagnitude { x: 1, y: 0 }, code: HuffmanCode { bits: 0b011, len: 3 } },
    HuffmanEntry { symbol: Layer3BigValueMagnitude { x: 1, y: 1 }, code: HuffmanCode { bits: 0b001, len: 3 } },
    HuffmanEntry { symbol: Layer3BigValueMagnitude { x: 1, y: 2 }, code: HuffmanCode { bits: 0b00001, len: 5 } },
    HuffmanEntry { symbol: Layer3BigValueMagnitude { x: 2, y: 0 }, code: HuffmanCode { bits: 0b00011, len: 5 } },
    HuffmanEntry { symbol: Layer3BigValueMagnitude { x: 2, y: 1 }, code: HuffmanCode { bits: 0b00010, len: 5 } },
    HuffmanEntry { symbol: Layer3BigValueMagnitude { x: 2, y: 2 }, code: HuffmanCode { bits: 0b000000, len: 6 } },
];

#[rustfmt::skip]
pub(crate) const MPEG1_LAYER3_BIG_VALUE_TABLE_3: &[HuffmanEntry<Layer3BigValueMagnitude>] = &[
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 0, y: 0 },
        code: HuffmanCode { bits: 0b11, len: 2 },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 0, y: 1 },
        code: HuffmanCode { bits: 0b10, len: 2 },
    },
    HuffmanEntry { symbol: Layer3BigValueMagnitude { x: 0, y: 2 }, code: HuffmanCode { bits: 0b000001, len: 6 } },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 1, y: 0 },
        code: HuffmanCode { bits: 0b01, len: 2 },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 1, y: 1 },
        code: HuffmanCode { bits: 0b1, len: 2 },
    },
    HuffmanEntry { symbol: Layer3BigValueMagnitude { x: 1, y: 2 }, code: HuffmanCode { bits: 0b00001, len: 5 } },
    HuffmanEntry { symbol: Layer3BigValueMagnitude { x: 2, y: 0 }, code: HuffmanCode { bits: 0b00011, len: 5 } },
    HuffmanEntry { symbol: Layer3BigValueMagnitude { x: 2, y: 1 }, code: HuffmanCode { bits: 0b00010, len: 5 } },
    HuffmanEntry { symbol: Layer3BigValueMagnitude { x: 2, y: 2 }, code: HuffmanCode { bits: 0b000000, len: 6 } },
];

#[rustfmt::skip]
pub(crate) const MPEG1_LAYER3_COUNT1_TABLE_32: &[HuffmanEntry<Layer3Count1MagnitudeQuad>] = &[
    HuffmanEntry {
        symbol: Layer3Count1MagnitudeQuad {
            v: 0,
            w: 0,
            x: 0,
            y: 0,
        },
        code: HuffmanCode { bits: 0b1, len: 1 },
    },
    HuffmanEntry {
        symbol: Layer3Count1MagnitudeQuad {
            v: 0,
            w: 0,
            x: 0,
            y: 1,
        },
        code: HuffmanCode {
            bits: 0b0101,
            len: 4,
        },
    },
    HuffmanEntry {
        symbol: Layer3Count1MagnitudeQuad {
            v: 0,
            w: 0,
            x: 1,
            y: 0,
        },
        code: HuffmanCode {
            bits: 0b0100,
            len: 4,
        },
    },
    HuffmanEntry {
        symbol: Layer3Count1MagnitudeQuad {
            v: 0,
            w: 0,
            x: 1,
            y: 1,
        },
        code: HuffmanCode {
            bits: 0b0101,
            len: 5,
        },
    },
    HuffmanEntry {
        symbol: Layer3Count1MagnitudeQuad {
            v: 0,
            w: 1,
            x: 0,
            y: 0,
        },
        code: HuffmanCode {
            bits: 0b0110,
            len: 4,
        },
    },
    HuffmanEntry {
        symbol: Layer3Count1MagnitudeQuad {
            v: 0,
            w: 1,
            x: 0,
            y: 1,
        },
        code: HuffmanCode {
            bits: 0b0101,
            len: 6,
        },
    },
    HuffmanEntry {
        symbol: Layer3Count1MagnitudeQuad {
            v: 0,
            w: 1,
            x: 1,
            y: 0,
        },
        code: HuffmanCode {
            bits: 0b0100,
            len: 5,
        },
    },
    HuffmanEntry {
        symbol: Layer3Count1MagnitudeQuad {
            v: 0,
            w: 1,
            x: 1,
            y: 1,
        },
        code: HuffmanCode {
            bits: 0b0100,
            len: 6,
        },
    },
    HuffmanEntry {
        symbol: Layer3Count1MagnitudeQuad {
            v: 1,
            w: 0,
            x: 0,
            y: 0,
        },
        code: HuffmanCode {
            bits: 0b0111,
            len: 4,
        },
    },
    HuffmanEntry {
        symbol: Layer3Count1MagnitudeQuad {
            v: 1,
            w: 0,
            x: 0,
            y: 1,
        },
        code: HuffmanCode {
            bits: 0b0011,
            len: 5,
        },
    },
    HuffmanEntry {
        symbol: Layer3Count1MagnitudeQuad {
            v: 1,
            w: 0,
            x: 1,
            y: 0,
        },
        code: HuffmanCode {
            bits: 0b0110,
            len: 5,
        },
    },
    HuffmanEntry {
        symbol: Layer3Count1MagnitudeQuad {
            v: 1,
            w: 0,
            x: 1,
            y: 1,
        },
        code: HuffmanCode {
            bits: 0b0000,
            len: 6,
        },
    },
    HuffmanEntry {
        symbol: Layer3Count1MagnitudeQuad {
            v: 1,
            w: 1,
            x: 0,
            y: 0,
        },
        code: HuffmanCode {
            bits: 0b0111,
            len: 5,
        },
    },
    HuffmanEntry {
        symbol: Layer3Count1MagnitudeQuad {
            v: 1,
            w: 1,
            x: 0,
            y: 1,
        },
        code: HuffmanCode {
            bits: 0b0010,
            len: 6,
        },
    },
    HuffmanEntry {
        symbol: Layer3Count1MagnitudeQuad {
            v: 1,
            w: 1,
            x: 1,
            y: 0,
        },
        code: HuffmanCode {
            bits: 0b0011,
            len: 6,
        },
    },
    HuffmanEntry {
        symbol: Layer3Count1MagnitudeQuad {
            v: 1,
            w: 1,
            x: 1,
            y: 1,
        },
        code: HuffmanCode {
            bits: 0b0001,
            len: 6,
        },
    },
];

#[rustfmt::skip]
pub(crate) const MPEG1_LAYER3_COUNT1_TABLE_33: &[HuffmanEntry<Layer3Count1MagnitudeQuad>] = &[
    HuffmanEntry {
        symbol: Layer3Count1MagnitudeQuad {
            v: 0,
            w: 0,
            x: 0,
            y: 0,
        },
        code: HuffmanCode {
            bits: 0b1111,
            len: 4,
        },
    },
    HuffmanEntry {
        symbol: Layer3Count1MagnitudeQuad {
            v: 0,
            w: 0,
            x: 0,
            y: 1,
        },
        code: HuffmanCode {
            bits: 0b1110,
            len: 4,
        },
    },
    HuffmanEntry {
        symbol: Layer3Count1MagnitudeQuad {
            v: 0,
            w: 0,
            x: 1,
            y: 0,
        },
        code: HuffmanCode {
            bits: 0b1101,
            len: 4,
        },
    },
    HuffmanEntry {
        symbol: Layer3Count1MagnitudeQuad {
            v: 0,
            w: 0,
            x: 1,
            y: 1,
        },
        code: HuffmanCode {
            bits: 0b1100,
            len: 4,
        },
    },
    HuffmanEntry {
        symbol: Layer3Count1MagnitudeQuad {
            v: 0,
            w: 1,
            x: 0,
            y: 0,
        },
        code: HuffmanCode {
            bits: 0b1011,
            len: 4,
        },
    },
    HuffmanEntry {
        symbol: Layer3Count1MagnitudeQuad {
            v: 0,
            w: 1,
            x: 0,
            y: 1,
        },
        code: HuffmanCode {
            bits: 0b1010,
            len: 4,
        },
    },
    HuffmanEntry {
        symbol: Layer3Count1MagnitudeQuad {
            v: 0,
            w: 1,
            x: 1,
            y: 0,
        },
        code: HuffmanCode {
            bits: 0b111,
            len: 3,
        },
    },
    HuffmanEntry {
        symbol: Layer3Count1MagnitudeQuad {
            v: 0,
            w: 1,
            x: 1,
            y: 1,
        },
        code: HuffmanCode {
            bits: 0b110,
            len: 3,
        },
    },
    HuffmanEntry {
        symbol: Layer3Count1MagnitudeQuad {
            v: 1,
            w: 0,
            x: 0,
            y: 0,
        },
        code: HuffmanCode {
            bits: 0b1001,
            len: 4,
        },
    },
    HuffmanEntry {
        symbol: Layer3Count1MagnitudeQuad {
            v: 1,
            w: 0,
            x: 0,
            y: 1,
        },
        code: HuffmanCode {
            bits: 0b101,
            len: 3,
        },
    },
    HuffmanEntry {
        symbol: Layer3Count1MagnitudeQuad {
            v: 1,
            w: 0,
            x: 1,
            y: 0,
        },
        code: HuffmanCode {
            bits: 0b1000,
            len: 4,
        },
    },
    HuffmanEntry {
        symbol: Layer3Count1MagnitudeQuad {
            v: 1,
            w: 0,
            x: 1,
            y: 1,
        },
        code: HuffmanCode {
            bits: 0b100,
            len: 3,
        },
    },
    HuffmanEntry {
        symbol: Layer3Count1MagnitudeQuad {
            v: 1,
            w: 1,
            x: 0,
            y: 0,
        },
        code: HuffmanCode {
            bits: 0b0111,
            len: 4,
        },
    },
    HuffmanEntry {
        symbol: Layer3Count1MagnitudeQuad {
            v: 1,
            w: 1,
            x: 0,
            y: 1,
        },
        code: HuffmanCode {
            bits: 0b011,
            len: 3,
        },
    },
    HuffmanEntry {
        symbol: Layer3Count1MagnitudeQuad {
            v: 1,
            w: 1,
            x: 1,
            y: 0,
        },
        code: HuffmanCode { bits: 0b10, len: 2 },
    },
    HuffmanEntry {
        symbol: Layer3Count1MagnitudeQuad {
            v: 1,
            w: 1,
            x: 1,
            y: 1,
        },
        code: HuffmanCode { bits: 0b1, len: 1 },
    },
];

// MPEG-1 Layer III big-values Huffman code tables (ISO/IEC 11172-3 Annex B
// Table 3-B.7). These are normative ISO constants; the codeword/length pairs
// here were derived by walking the decode tree of the public-domain (Unlicense)
// PDMP3 decoder — the same neutral, non-copyleft source as the analysis window.
// Clean-room applies only to copyleft *encoders* (LAME et al.), which were not
// consulted. Tables 16..=23 share table 16's codewords with different linbits.
#[rustfmt::skip]
pub(crate) const MPEG1_LAYER3_BIG_VALUE_TABLE_5: &[HuffmanEntry<Layer3BigValueMagnitude>] = &[
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 0, y: 0 },
        code: HuffmanCode { bits: 0b1, len: 1 },
    },
    HuffmanEntry { symbol: Layer3BigValueMagnitude { x: 0, y: 1 }, code: HuffmanCode { bits: 0b010, len: 3 } },
    HuffmanEntry { symbol: Layer3BigValueMagnitude { x: 0, y: 2 }, code: HuffmanCode { bits: 0b000110, len: 6 } },
    HuffmanEntry { symbol: Layer3BigValueMagnitude { x: 0, y: 3 }, code: HuffmanCode { bits: 0b0000101, len: 7 } },
    HuffmanEntry { symbol: Layer3BigValueMagnitude { x: 1, y: 0 }, code: HuffmanCode { bits: 0b011, len: 3 } },
    HuffmanEntry { symbol: Layer3BigValueMagnitude { x: 1, y: 1 }, code: HuffmanCode { bits: 0b001, len: 3 } },
    HuffmanEntry { symbol: Layer3BigValueMagnitude { x: 1, y: 2 }, code: HuffmanCode { bits: 0b000100, len: 6 } },
    HuffmanEntry { symbol: Layer3BigValueMagnitude { x: 1, y: 3 }, code: HuffmanCode { bits: 0b0000100, len: 7 } },
    HuffmanEntry { symbol: Layer3BigValueMagnitude { x: 2, y: 0 }, code: HuffmanCode { bits: 0b000111, len: 6 } },
    HuffmanEntry { symbol: Layer3BigValueMagnitude { x: 2, y: 1 }, code: HuffmanCode { bits: 0b000101, len: 6 } },
    HuffmanEntry { symbol: Layer3BigValueMagnitude { x: 2, y: 2 }, code: HuffmanCode { bits: 0b0000111, len: 7 } },
    HuffmanEntry { symbol: Layer3BigValueMagnitude { x: 2, y: 3 }, code: HuffmanCode { bits: 0b00000001, len: 8 } },
    HuffmanEntry { symbol: Layer3BigValueMagnitude { x: 3, y: 0 }, code: HuffmanCode { bits: 0b0000110, len: 7 } },
    HuffmanEntry { symbol: Layer3BigValueMagnitude { x: 3, y: 1 }, code: HuffmanCode { bits: 0b000001, len: 6 } },
    HuffmanEntry { symbol: Layer3BigValueMagnitude { x: 3, y: 2 }, code: HuffmanCode { bits: 0b0000001, len: 7 } },
    HuffmanEntry { symbol: Layer3BigValueMagnitude { x: 3, y: 3 }, code: HuffmanCode { bits: 0b00000000, len: 8 } },
];

#[rustfmt::skip]
pub(crate) const MPEG1_LAYER3_BIG_VALUE_TABLE_6: &[HuffmanEntry<Layer3BigValueMagnitude>] = &[
    HuffmanEntry { symbol: Layer3BigValueMagnitude { x: 0, y: 0 }, code: HuffmanCode { bits: 0b111, len: 3 } },
    HuffmanEntry { symbol: Layer3BigValueMagnitude { x: 0, y: 1 }, code: HuffmanCode { bits: 0b011, len: 3 } },
    HuffmanEntry { symbol: Layer3BigValueMagnitude { x: 0, y: 2 }, code: HuffmanCode { bits: 0b00101, len: 5 } },
    HuffmanEntry { symbol: Layer3BigValueMagnitude { x: 0, y: 3 }, code: HuffmanCode { bits: 0b0000001, len: 7 } },
    HuffmanEntry { symbol: Layer3BigValueMagnitude { x: 1, y: 0 }, code: HuffmanCode { bits: 0b110, len: 3 } },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 1, y: 1 },
        code: HuffmanCode { bits: 0b10, len: 2 },
    },
    HuffmanEntry { symbol: Layer3BigValueMagnitude { x: 1, y: 2 }, code: HuffmanCode { bits: 0b0011, len: 4 } },
    HuffmanEntry { symbol: Layer3BigValueMagnitude { x: 1, y: 3 }, code: HuffmanCode { bits: 0b00010, len: 5 } },
    HuffmanEntry { symbol: Layer3BigValueMagnitude { x: 2, y: 0 }, code: HuffmanCode { bits: 0b0101, len: 4 } },
    HuffmanEntry { symbol: Layer3BigValueMagnitude { x: 2, y: 1 }, code: HuffmanCode { bits: 0b0100, len: 4 } },
    HuffmanEntry { symbol: Layer3BigValueMagnitude { x: 2, y: 2 }, code: HuffmanCode { bits: 0b00100, len: 5 } },
    HuffmanEntry { symbol: Layer3BigValueMagnitude { x: 2, y: 3 }, code: HuffmanCode { bits: 0b000001, len: 6 } },
    HuffmanEntry { symbol: Layer3BigValueMagnitude { x: 3, y: 0 }, code: HuffmanCode { bits: 0b000011, len: 6 } },
    HuffmanEntry { symbol: Layer3BigValueMagnitude { x: 3, y: 1 }, code: HuffmanCode { bits: 0b00011, len: 5 } },
    HuffmanEntry { symbol: Layer3BigValueMagnitude { x: 3, y: 2 }, code: HuffmanCode { bits: 0b000010, len: 6 } },
    HuffmanEntry { symbol: Layer3BigValueMagnitude { x: 3, y: 3 }, code: HuffmanCode { bits: 0b0000000, len: 7 } },
];

#[rustfmt::skip]
pub(crate) const MPEG1_LAYER3_BIG_VALUE_TABLE_7: &[HuffmanEntry<Layer3BigValueMagnitude>] = &[
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 0, y: 0 },
        code: HuffmanCode { bits: 0b1, len: 1 },
    },
    HuffmanEntry { symbol: Layer3BigValueMagnitude { x: 0, y: 1 }, code: HuffmanCode { bits: 0b010, len: 3 } },
    HuffmanEntry { symbol: Layer3BigValueMagnitude { x: 0, y: 2 }, code: HuffmanCode { bits: 0b001010, len: 6 } },
    HuffmanEntry { symbol: Layer3BigValueMagnitude { x: 0, y: 3 }, code: HuffmanCode { bits: 0b00010011, len: 8 } },
    HuffmanEntry { symbol: Layer3BigValueMagnitude { x: 0, y: 4 }, code: HuffmanCode { bits: 0b00010000, len: 8 } },
    HuffmanEntry { symbol: Layer3BigValueMagnitude { x: 0, y: 5 }, code: HuffmanCode { bits: 0b000001010, len: 9 } },
    HuffmanEntry { symbol: Layer3BigValueMagnitude { x: 1, y: 0 }, code: HuffmanCode { bits: 0b011, len: 3 } },
    HuffmanEntry { symbol: Layer3BigValueMagnitude { x: 1, y: 1 }, code: HuffmanCode { bits: 0b0011, len: 4 } },
    HuffmanEntry { symbol: Layer3BigValueMagnitude { x: 1, y: 2 }, code: HuffmanCode { bits: 0b000111, len: 6 } },
    HuffmanEntry { symbol: Layer3BigValueMagnitude { x: 1, y: 3 }, code: HuffmanCode { bits: 0b0001010, len: 7 } },
    HuffmanEntry { symbol: Layer3BigValueMagnitude { x: 1, y: 4 }, code: HuffmanCode { bits: 0b0000101, len: 7 } },
    HuffmanEntry { symbol: Layer3BigValueMagnitude { x: 1, y: 5 }, code: HuffmanCode { bits: 0b00000011, len: 8 } },
    HuffmanEntry { symbol: Layer3BigValueMagnitude { x: 2, y: 0 }, code: HuffmanCode { bits: 0b001011, len: 6 } },
    HuffmanEntry { symbol: Layer3BigValueMagnitude { x: 2, y: 1 }, code: HuffmanCode { bits: 0b00100, len: 5 } },
    HuffmanEntry { symbol: Layer3BigValueMagnitude { x: 2, y: 2 }, code: HuffmanCode { bits: 0b0001101, len: 7 } },
    HuffmanEntry { symbol: Layer3BigValueMagnitude { x: 2, y: 3 }, code: HuffmanCode { bits: 0b00010001, len: 8 } },
    HuffmanEntry { symbol: Layer3BigValueMagnitude { x: 2, y: 4 }, code: HuffmanCode { bits: 0b00001000, len: 8 } },
    HuffmanEntry { symbol: Layer3BigValueMagnitude { x: 2, y: 5 }, code: HuffmanCode { bits: 0b000000100, len: 9 } },
    HuffmanEntry { symbol: Layer3BigValueMagnitude { x: 3, y: 0 }, code: HuffmanCode { bits: 0b0001100, len: 7 } },
    HuffmanEntry { symbol: Layer3BigValueMagnitude { x: 3, y: 1 }, code: HuffmanCode { bits: 0b0001011, len: 7 } },
    HuffmanEntry { symbol: Layer3BigValueMagnitude { x: 3, y: 2 }, code: HuffmanCode { bits: 0b00010010, len: 8 } },
    HuffmanEntry { symbol: Layer3BigValueMagnitude { x: 3, y: 3 }, code: HuffmanCode { bits: 0b000001111, len: 9 } },
    HuffmanEntry { symbol: Layer3BigValueMagnitude { x: 3, y: 4 }, code: HuffmanCode { bits: 0b000001011, len: 9 } },
    HuffmanEntry { symbol: Layer3BigValueMagnitude { x: 3, y: 5 }, code: HuffmanCode { bits: 0b000000010, len: 9 } },
    HuffmanEntry { symbol: Layer3BigValueMagnitude { x: 4, y: 0 }, code: HuffmanCode { bits: 0b0000111, len: 7 } },
    HuffmanEntry { symbol: Layer3BigValueMagnitude { x: 4, y: 1 }, code: HuffmanCode { bits: 0b0000110, len: 7 } },
    HuffmanEntry { symbol: Layer3BigValueMagnitude { x: 4, y: 2 }, code: HuffmanCode { bits: 0b00001001, len: 8 } },
    HuffmanEntry { symbol: Layer3BigValueMagnitude { x: 4, y: 3 }, code: HuffmanCode { bits: 0b000001110, len: 9 } },
    HuffmanEntry { symbol: Layer3BigValueMagnitude { x: 4, y: 4 }, code: HuffmanCode { bits: 0b000000011, len: 9 } },
    HuffmanEntry { symbol: Layer3BigValueMagnitude { x: 4, y: 5 }, code: HuffmanCode { bits: 0b0000000001, len: 10 } },
    HuffmanEntry { symbol: Layer3BigValueMagnitude { x: 5, y: 0 }, code: HuffmanCode { bits: 0b00000110, len: 8 } },
    HuffmanEntry { symbol: Layer3BigValueMagnitude { x: 5, y: 1 }, code: HuffmanCode { bits: 0b00000100, len: 8 } },
    HuffmanEntry { symbol: Layer3BigValueMagnitude { x: 5, y: 2 }, code: HuffmanCode { bits: 0b000000101, len: 9 } },
    HuffmanEntry { symbol: Layer3BigValueMagnitude { x: 5, y: 3 }, code: HuffmanCode { bits: 0b0000000011, len: 10 } },
    HuffmanEntry { symbol: Layer3BigValueMagnitude { x: 5, y: 4 }, code: HuffmanCode { bits: 0b0000000010, len: 10 } },
    HuffmanEntry { symbol: Layer3BigValueMagnitude { x: 5, y: 5 }, code: HuffmanCode { bits: 0b0000000000, len: 10 } },
];

#[rustfmt::skip]
pub(crate) const MPEG1_LAYER3_BIG_VALUE_TABLE_8: &[HuffmanEntry<Layer3BigValueMagnitude>] = &[
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 0, y: 0 },
        code: HuffmanCode { bits: 0x3, len: 2 },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 0, y: 1 },
        code: HuffmanCode { bits: 0x4, len: 3 },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 0, y: 2 },
        code: HuffmanCode { bits: 0x6, len: 6 },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 0, y: 3 },
        code: HuffmanCode { bits: 0x12, len: 8 },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 0, y: 4 },
        code: HuffmanCode { bits: 0xc, len: 8 },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 0, y: 5 },
        code: HuffmanCode { bits: 0x5, len: 9 },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 1, y: 0 },
        code: HuffmanCode { bits: 0x5, len: 3 },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 1, y: 1 },
        code: HuffmanCode { bits: 0x1, len: 2 },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 1, y: 2 },
        code: HuffmanCode { bits: 0x2, len: 4 },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 1, y: 3 },
        code: HuffmanCode { bits: 0x10, len: 8 },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 1, y: 4 },
        code: HuffmanCode { bits: 0x9, len: 8 },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 1, y: 5 },
        code: HuffmanCode { bits: 0x3, len: 8 },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 2, y: 0 },
        code: HuffmanCode { bits: 0x7, len: 6 },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 2, y: 1 },
        code: HuffmanCode { bits: 0x3, len: 4 },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 2, y: 2 },
        code: HuffmanCode { bits: 0x5, len: 6 },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 2, y: 3 },
        code: HuffmanCode { bits: 0xe, len: 8 },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 2, y: 4 },
        code: HuffmanCode { bits: 0x7, len: 8 },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 2, y: 5 },
        code: HuffmanCode { bits: 0x3, len: 9 },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 3, y: 0 },
        code: HuffmanCode { bits: 0x13, len: 8 },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 3, y: 1 },
        code: HuffmanCode { bits: 0x11, len: 8 },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 3, y: 2 },
        code: HuffmanCode { bits: 0xf, len: 8 },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 3, y: 3 },
        code: HuffmanCode { bits: 0xd, len: 9 },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 3, y: 4 },
        code: HuffmanCode { bits: 0xa, len: 9 },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 3, y: 5 },
        code: HuffmanCode { bits: 0x4, len: 10 },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 4, y: 0 },
        code: HuffmanCode { bits: 0xd, len: 8 },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 4, y: 1 },
        code: HuffmanCode { bits: 0x5, len: 7 },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 4, y: 2 },
        code: HuffmanCode { bits: 0x8, len: 8 },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 4, y: 3 },
        code: HuffmanCode { bits: 0xb, len: 9 },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 4, y: 4 },
        code: HuffmanCode { bits: 0x5, len: 10 },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 4, y: 5 },
        code: HuffmanCode { bits: 0x1, len: 10 },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 5, y: 0 },
        code: HuffmanCode { bits: 0xc, len: 9 },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 5, y: 1 },
        code: HuffmanCode { bits: 0x4, len: 8 },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 5, y: 2 },
        code: HuffmanCode { bits: 0x4, len: 9 },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 5, y: 3 },
        code: HuffmanCode { bits: 0x1, len: 9 },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 5, y: 4 },
        code: HuffmanCode { bits: 0x1, len: 11 },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 5, y: 5 },
        code: HuffmanCode { bits: 0x0, len: 11 },
    },
];

#[rustfmt::skip]
pub(crate) const MPEG1_LAYER3_BIG_VALUE_TABLE_9: &[HuffmanEntry<Layer3BigValueMagnitude>] = &[
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 0, y: 0 },
        code: HuffmanCode { bits: 0x7, len: 3 },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 0, y: 1 },
        code: HuffmanCode { bits: 0x5, len: 3 },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 0, y: 2 },
        code: HuffmanCode { bits: 0x9, len: 5 },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 0, y: 3 },
        code: HuffmanCode { bits: 0xe, len: 6 },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 0, y: 4 },
        code: HuffmanCode { bits: 0xf, len: 8 },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 0, y: 5 },
        code: HuffmanCode { bits: 0x7, len: 9 },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 1, y: 0 },
        code: HuffmanCode { bits: 0x6, len: 3 },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 1, y: 1 },
        code: HuffmanCode { bits: 0x4, len: 3 },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 1, y: 2 },
        code: HuffmanCode { bits: 0x5, len: 4 },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 1, y: 3 },
        code: HuffmanCode { bits: 0x5, len: 5 },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 1, y: 4 },
        code: HuffmanCode { bits: 0x6, len: 6 },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 1, y: 5 },
        code: HuffmanCode { bits: 0x7, len: 8 },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 2, y: 0 },
        code: HuffmanCode { bits: 0x7, len: 4 },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 2, y: 1 },
        code: HuffmanCode { bits: 0x6, len: 4 },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 2, y: 2 },
        code: HuffmanCode { bits: 0x8, len: 5 },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 2, y: 3 },
        code: HuffmanCode { bits: 0x8, len: 6 },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 2, y: 4 },
        code: HuffmanCode { bits: 0x8, len: 7 },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 2, y: 5 },
        code: HuffmanCode { bits: 0x5, len: 8 },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 3, y: 0 },
        code: HuffmanCode { bits: 0xf, len: 6 },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 3, y: 1 },
        code: HuffmanCode { bits: 0x6, len: 5 },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 3, y: 2 },
        code: HuffmanCode { bits: 0x9, len: 6 },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 3, y: 3 },
        code: HuffmanCode { bits: 0xa, len: 7 },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 3, y: 4 },
        code: HuffmanCode { bits: 0x5, len: 7 },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 3, y: 5 },
        code: HuffmanCode { bits: 0x1, len: 8 },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 4, y: 0 },
        code: HuffmanCode { bits: 0xb, len: 7 },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 4, y: 1 },
        code: HuffmanCode { bits: 0x7, len: 6 },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 4, y: 2 },
        code: HuffmanCode { bits: 0x9, len: 7 },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 4, y: 3 },
        code: HuffmanCode { bits: 0x6, len: 7 },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 4, y: 4 },
        code: HuffmanCode { bits: 0x4, len: 8 },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 4, y: 5 },
        code: HuffmanCode { bits: 0x1, len: 9 },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 5, y: 0 },
        code: HuffmanCode { bits: 0xe, len: 8 },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 5, y: 1 },
        code: HuffmanCode { bits: 0x4, len: 7 },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 5, y: 2 },
        code: HuffmanCode { bits: 0x6, len: 8 },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 5, y: 3 },
        code: HuffmanCode { bits: 0x2, len: 8 },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 5, y: 4 },
        code: HuffmanCode { bits: 0x6, len: 9 },
    },
    HuffmanEntry {
        symbol: Layer3BigValueMagnitude { x: 5, y: 5 },
        code: HuffmanCode { bits: 0x0, len: 9 },
    },
];

#[rustfmt::skip]
pub(crate) const MPEG1_LAYER3_BIG_VALUE_TABLE_11_CODES: [u32; 64] = [
    0x0003, 0x0004, 0x000a, 0x0018, 0x0022, 0x0021, 0x0015, 0x000f, 0x0005, 0x0003, 0x0004, 0x000a,
    0x0020, 0x0011, 0x000b, 0x000a, 0x000b, 0x0007, 0x000d, 0x0012, 0x001e, 0x001f, 0x0014, 0x0005,
    0x0019, 0x000b, 0x0013, 0x003b, 0x001b, 0x0012, 0x000c, 0x0005, 0x0023, 0x0021, 0x001f, 0x003a,
    0x001e, 0x0010, 0x0007, 0x0005, 0x001c, 0x001a, 0x0020, 0x0013, 0x0011, 0x000f, 0x0008, 0x000e,
    0x000e, 0x000c, 0x0009, 0x000d, 0x000e, 0x0009, 0x0004, 0x0001, 0x000b, 0x0004, 0x0006, 0x0006,
    0x0006, 0x0003, 0x0002, 0x0000,
];

#[rustfmt::skip]
pub(crate) const MPEG1_LAYER3_BIG_VALUE_TABLE_11_BITS: [u8; 64] = [
    2, 3, 5, 7, 8, 9, 8, 9, 3, 3, 4, 6, 8, 8, 7, 8, 5, 5, 6, 7, 8, 9, 8, 8, 7, 6, 7, 9, 8, 10, 8,
    9, 8, 8, 8, 9, 9, 10, 9, 10, 8, 8, 9, 10, 10, 11, 10, 11, 8, 7, 7, 8, 9, 10, 10, 10, 8, 7, 8,
    9, 10, 10, 10, 10,
];

#[rustfmt::skip]
pub(crate) const MPEG1_LAYER3_BIG_VALUE_TABLE_12_CODES: [u32; 64] = [
    0x0009, 0x0006, 0x0010, 0x0021, 0x0029, 0x0027, 0x0026, 0x001a, 0x0007, 0x0005, 0x0006, 0x0009,
    0x0017, 0x0010, 0x001a, 0x000b, 0x0011, 0x0007, 0x000b, 0x000e, 0x0015, 0x001e, 0x000a, 0x0007,
    0x0011, 0x000a, 0x000f, 0x000c, 0x0012, 0x001c, 0x000e, 0x0005, 0x0020, 0x000d, 0x0016, 0x0013,
    0x0012, 0x0010, 0x0009, 0x0005, 0x0028, 0x0011, 0x001f, 0x001d, 0x0011, 0x000d, 0x0004, 0x0002,
    0x001b, 0x000c, 0x000b, 0x000f, 0x000a, 0x0007, 0x0004, 0x0001, 0x001b, 0x000c, 0x0008, 0x000c,
    0x0006, 0x0003, 0x0001, 0x0000,
];

#[rustfmt::skip]
pub(crate) const MPEG1_LAYER3_BIG_VALUE_TABLE_12_BITS: [u8; 64] = [
    4, 3, 5, 7, 8, 9, 9, 9, 3, 3, 4, 5, 7, 7, 8, 8, 5, 4, 5, 6, 7, 8, 7, 8, 6, 5, 6, 6, 7, 8, 8, 8,
    7, 6, 7, 7, 8, 8, 8, 9, 8, 7, 8, 8, 8, 9, 8, 9, 8, 7, 7, 8, 8, 9, 9, 10, 9, 8, 8, 9, 9, 9, 9,
    10,
];

pub(crate) static MPEG1_LAYER3_BIG_VALUE_TABLE_11: OnceLock<
    Vec<HuffmanEntry<Layer3BigValueMagnitude>>,
> = OnceLock::new();
pub(crate) static MPEG1_LAYER3_BIG_VALUE_TABLE_12: OnceLock<
    Vec<HuffmanEntry<Layer3BigValueMagnitude>>,
> = OnceLock::new();

pub(crate) fn build_mpeg1_layer3_big_value_table(
    wrap: u8,
    codes: &[u32],
    bits: &[u8],
) -> Vec<HuffmanEntry<Layer3BigValueMagnitude>> {
    assert_eq!(codes.len(), bits.len());
    codes
        .iter()
        .zip(bits.iter())
        .enumerate()
        .map(|(index, (&code, &len))| HuffmanEntry {
            symbol: Layer3BigValueMagnitude {
                x: u16::try_from(index / usize::from(wrap)).expect("MP3 table x index fits u16"),
                y: u16::try_from(index % usize::from(wrap)).expect("MP3 table y index fits u16"),
            },
            code: HuffmanCode { bits: code, len },
        })
        .collect()
}

pub(crate) fn mpeg1_layer3_big_value_table_11() -> &'static [HuffmanEntry<Layer3BigValueMagnitude>]
{
    MPEG1_LAYER3_BIG_VALUE_TABLE_11
        .get_or_init(|| {
            build_mpeg1_layer3_big_value_table(
                8,
                &MPEG1_LAYER3_BIG_VALUE_TABLE_11_CODES,
                &MPEG1_LAYER3_BIG_VALUE_TABLE_11_BITS,
            )
        })
        .as_slice()
}

pub(crate) fn mpeg1_layer3_big_value_table_12() -> &'static [HuffmanEntry<Layer3BigValueMagnitude>]
{
    MPEG1_LAYER3_BIG_VALUE_TABLE_12
        .get_or_init(|| {
            build_mpeg1_layer3_big_value_table(
                8,
                &MPEG1_LAYER3_BIG_VALUE_TABLE_12_CODES,
                &MPEG1_LAYER3_BIG_VALUE_TABLE_12_BITS,
            )
        })
        .as_slice()
}
