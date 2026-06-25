use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AacCodebook {
    Zero,
    SignedPairs1,
    SignedPairs5,
    SignedPairs6,
    UnsignedPairs7,
    UnsignedPairs8,
    UnsignedPairs9,
    UnsignedPairs10,
    Escape,
}

impl AacCodebook {
    #[must_use]
    pub fn id(self) -> u8 {
        match self {
            Self::Zero => 0,
            Self::SignedPairs1 => 1,
            Self::SignedPairs5 => 5,
            Self::SignedPairs6 => 6,
            Self::UnsignedPairs7 => 7,
            Self::UnsignedPairs8 => 8,
            Self::UnsignedPairs9 => 9,
            Self::UnsignedPairs10 => 10,
            Self::Escape => 11,
        }
    }

    /// Whether this spectral Huffman codebook embeds each coefficient's sign in
    /// the codeword (ISO/IEC 14496-3, Table 4.A.2).
    ///
    /// Signed codebooks (1/2 quads, 5/6 pairs) carry the sign inside the
    /// codeword and MUST NOT be followed by explicit sign bits. Unsigned
    /// codebooks (3/4 quads, 7–11 pairs) emit one sign bit per nonzero
    /// coefficient after the codeword. Mixing the two — packing a signed
    /// codebook through the sign-bit packer — yields a non-conformant,
    /// undecodable section.
    #[must_use]
    pub fn embeds_sign(self) -> bool {
        matches!(
            self,
            Self::SignedPairs1 | Self::SignedPairs5 | Self::SignedPairs6
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AacSection {
    pub start: usize,
    pub end: usize,
    pub codebook: AacCodebook,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AacQuadSection {
    pub start: usize,
    pub end: usize,
    pub codebook_id: u8,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AacSpectralSection {
    pub start: usize,
    pub end: usize,
    pub codebook_id: u8,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct AacMagnitudeSection<'a> {
    pub(crate) start: usize,
    pub(crate) end: usize,
    pub(crate) codebook_id: u8,
    pub(crate) table: &'a [HuffmanEntry<AacSpectralMagnitudePair>],
}

impl AacMagnitudeSection<'_> {
    pub(crate) fn is_zero(self) -> bool {
        self.codebook_id == AacCodebook::Zero.id()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AacSpectralPair {
    pub x: i16,
    pub y: i16,
}

impl AacSpectralPair {
    #[must_use]
    pub fn new(x: i16, y: i16) -> Self {
        Self { x, y }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AacSpectralMagnitudePair {
    pub x: u16,
    pub y: u16,
}

impl AacSpectralMagnitudePair {
    #[must_use]
    pub fn new(x: u16, y: u16) -> Self {
        Self { x, y }
    }
}

impl TryFrom<AacSpectralPair> for AacSpectralMagnitudePair {
    type Error = Error;

    fn try_from(pair: AacSpectralPair) -> Result<Self, Self::Error> {
        Ok(Self::new(
            pair.x
                .checked_abs()
                .and_then(|value| u16::try_from(value).ok())
                .ok_or(Error::InvalidInput("AAC spectral pair x overflows"))?,
            pair.y
                .checked_abs()
                .and_then(|value| u16::try_from(value).ok())
                .ok_or(Error::InvalidInput("AAC spectral pair y overflows"))?,
        ))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AacSpectralQuad {
    pub v: i16,
    pub w: i16,
    pub x: i16,
    pub y: i16,
}

impl AacSpectralQuad {
    #[must_use]
    pub fn new(v: i16, w: i16, x: i16, y: i16) -> Self {
        Self { v, w, x, y }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AacSpectralMagnitudeQuad {
    pub v: u16,
    pub w: u16,
    pub x: u16,
    pub y: u16,
}

impl AacSpectralMagnitudeQuad {
    #[must_use]
    pub fn new(v: u16, w: u16, x: u16, y: u16) -> Self {
        Self { v, w, x, y }
    }
}

impl TryFrom<AacSpectralQuad> for AacSpectralMagnitudeQuad {
    type Error = Error;

    fn try_from(quad: AacSpectralQuad) -> Result<Self, Self::Error> {
        Ok(Self::new(
            quad.v
                .checked_abs()
                .and_then(|value| u16::try_from(value).ok())
                .ok_or(Error::InvalidInput("AAC spectral quad v overflows"))?,
            quad.w
                .checked_abs()
                .and_then(|value| u16::try_from(value).ok())
                .ok_or(Error::InvalidInput("AAC spectral quad w overflows"))?,
            quad.x
                .checked_abs()
                .and_then(|value| u16::try_from(value).ok())
                .ok_or(Error::InvalidInput("AAC spectral quad x overflows"))?,
            quad.y
                .checked_abs()
                .and_then(|value| u16::try_from(value).ok())
                .ok_or(Error::InvalidInput("AAC spectral quad y overflows"))?,
        ))
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct AacSpectralTables<'a> {
    pub signed_pairs1: &'a [HuffmanEntry<AacSpectralPair>],
    pub signed_pairs5: &'a [HuffmanEntry<AacSpectralPair>],
    pub signed_pairs6: &'a [HuffmanEntry<AacSpectralPair>],
    pub escape: &'a [HuffmanEntry<AacSpectralPair>],
}

impl<'a> AacSpectralTables<'a> {
    pub(crate) fn table_for(
        self,
        codebook: AacCodebook,
    ) -> Result<&'a [HuffmanEntry<AacSpectralPair>], Error> {
        match codebook {
            AacCodebook::Zero => Ok(&[]),
            AacCodebook::SignedPairs1 => non_empty_table(self.signed_pairs1, "AAC codebook 1"),
            AacCodebook::SignedPairs5 => non_empty_table(self.signed_pairs5, "AAC codebook 5"),
            AacCodebook::SignedPairs6 => non_empty_table(self.signed_pairs6, "AAC codebook 6"),
            AacCodebook::UnsignedPairs7
            | AacCodebook::UnsignedPairs8
            | AacCodebook::UnsignedPairs9
            | AacCodebook::UnsignedPairs10 => Err(Error::UnsupportedFeature(
                "AAC unsigned-pairs codebooks 7/8/9/10 require magnitude tables",
            )),
            AacCodebook::Escape => non_empty_table(self.escape, "AAC escape codebook"),
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct AacSpectralMagnitudeTables<'a> {
    pub pairs1: &'a [HuffmanEntry<AacSpectralMagnitudePair>],
    pub pairs5: &'a [HuffmanEntry<AacSpectralMagnitudePair>],
    pub pairs6: &'a [HuffmanEntry<AacSpectralMagnitudePair>],
    pub escape: &'a [HuffmanEntry<AacSpectralMagnitudePair>],
}

impl<'a> AacSpectralMagnitudeTables<'a> {
    pub(crate) fn table_for(
        self,
        codebook: AacCodebook,
    ) -> Result<&'a [HuffmanEntry<AacSpectralMagnitudePair>], Error> {
        match codebook {
            AacCodebook::Zero => Ok(&[]),
            AacCodebook::SignedPairs1 => {
                non_empty_magnitude_table(self.pairs1, "AAC magnitude codebook 1")
            }
            AacCodebook::SignedPairs5 => {
                non_empty_magnitude_table(self.pairs5, "AAC magnitude codebook 5")
            }
            AacCodebook::SignedPairs6 => {
                non_empty_magnitude_table(self.pairs6, "AAC magnitude codebook 6")
            }
            AacCodebook::UnsignedPairs7 => Ok(aac_unsigned_pairs7_table()),
            AacCodebook::UnsignedPairs8 => Ok(aac_unsigned_pairs8_table()),
            AacCodebook::UnsignedPairs9 => Ok(aac_unsigned_pairs9_table()),
            AacCodebook::UnsignedPairs10 => Ok(aac_unsigned_pairs10_table()),
            AacCodebook::Escape => {
                non_empty_magnitude_table(self.escape, "AAC magnitude escape codebook")
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct AacSpectralMagnitudeQuadTables<'a> {
    pub quads1: &'a [HuffmanEntry<AacSpectralMagnitudeQuad>],
    pub quads2: &'a [HuffmanEntry<AacSpectralMagnitudeQuad>],
    pub quads3: &'a [HuffmanEntry<AacSpectralMagnitudeQuad>],
    pub quads4: &'a [HuffmanEntry<AacSpectralMagnitudeQuad>],
}

impl<'a> AacSpectralMagnitudeQuadTables<'a> {
    pub(crate) fn table_for_codebook_id(
        self,
        codebook_id: u8,
    ) -> Result<&'a [HuffmanEntry<AacSpectralMagnitudeQuad>], Error> {
        match codebook_id {
            1 => non_empty_quad_table(self.quads1, "AAC quad codebook 1"),
            2 => non_empty_quad_table(self.quads2, "AAC quad codebook 2"),
            3 => non_empty_quad_table(self.quads3, "AAC quad codebook 3"),
            4 => non_empty_quad_table(self.quads4, "AAC quad codebook 4"),
            _ => Err(Error::InvalidInput("AAC quad codebook id must be 1..=4")),
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct AacSpectralQuadTables<'a> {
    pub quads1: &'a [HuffmanEntry<AacSpectralQuad>],
    pub quads2: &'a [HuffmanEntry<AacSpectralQuad>],
}

impl<'a> AacSpectralQuadTables<'a> {
    pub(crate) fn table_for_codebook_id(
        self,
        codebook_id: u8,
    ) -> Result<&'a [HuffmanEntry<AacSpectralQuad>], Error> {
        match codebook_id {
            1 => non_empty_signed_quad_table(self.quads1, "AAC signed quad codebook 1"),
            2 => non_empty_signed_quad_table(self.quads2, "AAC signed quad codebook 2"),
            _ => Err(Error::InvalidInput(
                "AAC signed quad codebook id must be 1..=2",
            )),
        }
    }
}

pub(crate) fn quad_codebook_table_candidates(
    tables: AacSpectralMagnitudeQuadTables<'_>,
) -> [(u8, &[HuffmanEntry<AacSpectralMagnitudeQuad>]); 4] {
    [
        (1, tables.quads1),
        (2, tables.quads2),
        (3, tables.quads3),
        (4, tables.quads4),
    ]
}

pub(crate) fn signed_quad_codebook_table_candidates(
    tables: AacSpectralQuadTables<'_>,
) -> [(u8, &[HuffmanEntry<AacSpectralQuad>]); 2] {
    [(1, tables.quads1), (2, tables.quads2)]
}
