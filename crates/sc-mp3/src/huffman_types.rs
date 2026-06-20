use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Layer3SpectralRegions {
    pub big_values: u16,
    pub count1: u16,
    pub rzero: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Layer3BigValuePair {
    pub x: i16,
    pub y: i16,
}

impl Layer3BigValuePair {
    #[must_use]
    pub fn new(x: i16, y: i16) -> Self {
        Self { x, y }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Layer3BigValueMagnitude {
    pub x: u16,
    pub y: u16,
}

impl Layer3BigValueMagnitude {
    #[must_use]
    pub fn new(x: u16, y: u16) -> Self {
        Self { x, y }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Layer3BigValueTableSelection {
    pub table_select: u8,
    pub linbits: u8,
    pub max_magnitude: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Layer3BigValueRegionTableSelection {
    pub regions: [Layer3BigValueTableSelection; 3],
    pub region0_pairs: u16,
    pub region1_pairs: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Layer3Count1Quad {
    pub v: i8,
    pub w: i8,
    pub x: i8,
    pub y: i8,
}

impl Layer3Count1Quad {
    #[must_use]
    pub fn new(v: i8, w: i8, x: i8, y: i8) -> Self {
        Self { v, w, x, y }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Layer3Count1MagnitudeQuad {
    pub v: u8,
    pub w: u8,
    pub x: u8,
    pub y: u8,
}

impl Layer3Count1MagnitudeQuad {
    #[must_use]
    pub fn new(v: u8, w: u8, x: u8, y: u8) -> Self {
        Self { v, w, x, y }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Layer3Count1TableSelection {
    pub table_select: bool,
    pub max_nonzero_values: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Layer3ScaleFactorCompress {
    pub scalefac_compress: u16,
    pub slen1: u8,
    pub slen2: u8,
}

/// MPEG-2 LSF (ISO/IEC 13818-3 §2.4.3.2) long-block scale-factor partition.
///
/// In the low-sampling-frequency extension the 21 transmitted long-block scale
/// factors are split into four contiguous groups whose sizes and per-group bit
/// widths (`slen`) are jointly encoded in the 9-bit `scalefac_compress` field.
/// This encoder generates only the two `preflag == 0` partition branches —
/// group sizes `[6, 5, 5, 5]` (`scalefac_compress < 400`) and `[6, 5, 7, 3]`
/// (`400 <= scalefac_compress < 500`); the `preflag` and intensity-stereo
/// branches are never produced.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Mpeg2Layer3LsfScaleFactorCompress {
    pub scalefac_compress: u16,
    /// Group sizes in scale-factor bands; the four entries sum to 21.
    pub group_sizes: [u8; 4],
    /// Per-group scale-factor bit widths.
    pub slen: [u8; 4],
}

#[derive(Clone, Copy, Debug)]
pub struct Layer3EntropyTables<'a> {
    pub big_values: &'a [HuffmanEntry<Layer3BigValueMagnitude>],
    pub count1: &'a [HuffmanEntry<Layer3Count1MagnitudeQuad>],
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Layer3EntropyTableProvider<'a> {
    pub big_value_table_1: &'a [HuffmanEntry<Layer3BigValueMagnitude>],
    pub big_value_table_2: &'a [HuffmanEntry<Layer3BigValueMagnitude>],
    pub big_value_table_3: &'a [HuffmanEntry<Layer3BigValueMagnitude>],
    pub big_value_table_5: &'a [HuffmanEntry<Layer3BigValueMagnitude>],
    pub big_value_table_6: &'a [HuffmanEntry<Layer3BigValueMagnitude>],
    pub big_value_table_7: &'a [HuffmanEntry<Layer3BigValueMagnitude>],
    pub big_value_table_8: &'a [HuffmanEntry<Layer3BigValueMagnitude>],
    pub big_value_table_9: &'a [HuffmanEntry<Layer3BigValueMagnitude>],
    pub big_value_table_10: &'a [HuffmanEntry<Layer3BigValueMagnitude>],
    pub big_value_table_11: &'a [HuffmanEntry<Layer3BigValueMagnitude>],
    pub big_value_table_12: &'a [HuffmanEntry<Layer3BigValueMagnitude>],
    pub big_value_table_13: &'a [HuffmanEntry<Layer3BigValueMagnitude>],
    pub big_value_table_15: &'a [HuffmanEntry<Layer3BigValueMagnitude>],
    pub big_value_table_16: &'a [HuffmanEntry<Layer3BigValueMagnitude>],
    pub big_value_table_24: &'a [HuffmanEntry<Layer3BigValueMagnitude>],
    pub count1_table_0: &'a [HuffmanEntry<Layer3Count1MagnitudeQuad>],
    pub count1_table_1: &'a [HuffmanEntry<Layer3Count1MagnitudeQuad>],
}

pub const MPEG1_LAYER3_STANDARD_BIG_VALUE_TABLE_SELECTS: &[u8] = &[
    1, 2, 3, 5, 6, 7, 8, 9, 10, 11, 12, 13, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28,
    29, 30, 31,
];
pub const MPEG1_LAYER3_MISSING_STANDARD_BIG_VALUE_TABLE_SELECTS: &[u8] = &[];
pub const MPEG1_LAYER3_STANDARD_COUNT1_TABLE_SELECTS: &[bool] = &[false, true];

impl<'a> Layer3EntropyTableProvider<'a> {
    pub fn big_value_table(
        self,
        selection: Layer3BigValueTableSelection,
    ) -> Result<&'a [HuffmanEntry<Layer3BigValueMagnitude>], Error> {
        let table = match selection.table_select {
            0 => &[],
            1 => self.big_value_table_1,
            2 => self.big_value_table_2,
            3 => self.big_value_table_3,
            5 => self.big_value_table_5,
            6 => self.big_value_table_6,
            7 => self.big_value_table_7,
            8 => self.big_value_table_8,
            9 => self.big_value_table_9,
            10 => self.big_value_table_10,
            11 => self.big_value_table_11,
            12 => self.big_value_table_12,
            13 => self.big_value_table_13,
            15 => self.big_value_table_15,
            // Tables 16..=23 share the table-16 codeword tree (different linbits).
            16..=23 => self.big_value_table_16,
            // Tables 24..=31 share the table-24 codeword tree (different linbits).
            24..=31 => self.big_value_table_24,
            _ => return Err(Error::UnsupportedFeature("MP3 big-values Huffman table")),
        };
        if selection.table_select != 0 && table.is_empty() {
            return Err(Error::UnsupportedFeature("MP3 big-values Huffman table"));
        }
        Ok(table)
    }

    pub fn count1_table(
        self,
        selection: Layer3Count1TableSelection,
    ) -> Result<&'a [HuffmanEntry<Layer3Count1MagnitudeQuad>], Error> {
        if selection.max_nonzero_values == 0 {
            return Ok(&[]);
        }

        let table = if selection.table_select {
            self.count1_table_1
        } else {
            self.count1_table_0
        };
        if table.is_empty() {
            return Err(Error::UnsupportedFeature("MP3 count1 Huffman table"));
        }
        Ok(table)
    }
}
