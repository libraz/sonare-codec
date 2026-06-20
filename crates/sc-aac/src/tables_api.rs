use super::*;

/// Returns a minimal experimental AAC spectral table set for zero/one pairs.
///
/// This is not the AAC-LC standard Huffman table set. It exists to exercise
/// non-zero section and sign-bit payload plumbing while the full clean-room
/// codebooks and rate control are being implemented.
#[must_use]
pub fn experimental_unit_magnitude_spectral_tables() -> AacSpectralMagnitudeTables<'static> {
    AacSpectralMagnitudeTables {
        pairs1: EXPERIMENTAL_AAC_PAIRS1_TABLE,
        ..Default::default()
    }
}

/// Returns the unit-magnitude codebook-6 fixture used by AAC section-planner workbenches.
///
/// This is intentionally small and is not a replacement for the full standard
/// signed-pairs codebook 6 table.
#[must_use]
pub fn aac_unit_codebook6_spectral_tables() -> AacSpectralMagnitudeTables<'static> {
    AacSpectralMagnitudeTables {
        pairs6: AAC_UNIT_PAIRS6_TABLE,
        ..Default::default()
    }
}

/// Returns the unit-magnitude quad fixture used by AAC section-planner workbenches.
///
/// This keeps the package-visible quad planner on a core-owned table fixture
/// while the complete standard quad codebooks 1-4 are still pending.
#[must_use]
pub fn aac_unit_quad_spectral_tables() -> AacSpectralMagnitudeQuadTables<'static> {
    AacSpectralMagnitudeQuadTables {
        quads1: AAC_UNIT_QUADS1_TABLE,
        quads3: AAC_UNIT_QUADS3_TABLE,
        ..Default::default()
    }
}

/// Returns the implemented standard AAC unsigned quad codebook set.
///
/// Codebooks 3 and 4 use magnitude-keyed quadruples followed by sign bits for
/// non-zero coefficients. Direct signed quad codebooks 1 and 2 are intentionally
/// kept separate until the direct signed-quad path is added.
#[must_use]
pub fn aac_lc_standard_unsigned_quad_tables() -> AacSpectralMagnitudeQuadTables<'static> {
    AacSpectralMagnitudeQuadTables {
        quads3: aac_unsigned_quads3_table(),
        quads4: aac_unsigned_quads4_table(),
        ..Default::default()
    }
}

/// Returns the implemented standard AAC direct signed quad codebook set.
///
/// Codebooks 1 and 2 carry signs in their Huffman symbols; they must not be
/// packed through magnitude tables with appended sign bits.
#[must_use]
pub fn aac_lc_standard_signed_quad_tables() -> AacSpectralQuadTables<'static> {
    AacSpectralQuadTables {
        quads1: aac_signed_quads1_table(),
        quads2: aac_signed_quads2_table(),
    }
}

/// Returns the implemented standard AAC direct signed-pair codebook set.
///
/// Codebooks 5 and 6 carry signs in their Huffman symbols; they must not be
/// packed through magnitude tables with appended sign bits.
#[must_use]
pub fn aac_lc_standard_signed_pair_tables() -> AacSpectralTables<'static> {
    AacSpectralTables {
        signed_pairs5: aac_signed_pairs5_table(),
        signed_pairs6: aac_signed_pairs6_table(),
        ..Default::default()
    }
}

/// Returns a compatibility table set for callers of the current AAC production helper.
///
/// The offset-based production path enables the standard codebook-7 zero/one
/// subset internally so the public magnitude-table struct can remain semver
/// compatible while the full table surface is still being designed.
#[must_use]
pub fn aac_unsigned_pairs7_unit_magnitude_spectral_tables() -> AacSpectralMagnitudeTables<'static> {
    AacSpectralMagnitudeTables::default()
}

/// Returns the standard AAC-LC spectral table set currently implemented.
///
/// The unsigned-pairs codebooks 7/8/9/10 are provided implicitly by
/// [`AacSpectralMagnitudeTables::table_for`]. This table set adds the standard
/// escape codebook 11 so bitrate/step search can keep larger magnitudes instead
/// of forcing them out of range. Signed/quad codebooks remain pending.
#[must_use]
pub fn aac_lc_standard_spectral_tables() -> AacSpectralMagnitudeTables<'static> {
    AacSpectralMagnitudeTables {
        escape: aac_escape_table(),
        ..Default::default()
    }
}

/// Returns the standard AAC signed-pairs codebook 5 for values -4..=4.
#[must_use]
pub fn aac_signed_pairs5_table() -> &'static [HuffmanEntry<AacSpectralPair>] {
    AAC_SIGNED_PAIRS5_TABLE
        .get_or_init(|| {
            AAC_SIGNED_PAIRS5_CODES
                .iter()
                .zip(AAC_SIGNED_PAIRS5_LENS)
                .enumerate()
                .map(|(index, (&bits, len))| HuffmanEntry {
                    symbol: AacSpectralPair {
                        x: (index / 9) as i16 - 4,
                        y: (index % 9) as i16 - 4,
                    },
                    code: HuffmanCode { bits, len },
                })
                .collect()
        })
        .as_slice()
}

/// Returns the standard AAC signed-pairs codebook 6 for values -4..=4.
#[must_use]
pub fn aac_signed_pairs6_table() -> &'static [HuffmanEntry<AacSpectralPair>] {
    AAC_SIGNED_PAIRS6_TABLE
        .get_or_init(|| {
            AAC_SIGNED_PAIRS6_CODES
                .iter()
                .zip(AAC_SIGNED_PAIRS6_LENS)
                .enumerate()
                .map(|(index, (&bits, len))| HuffmanEntry {
                    symbol: AacSpectralPair {
                        x: (index / 9) as i16 - 4,
                        y: (index % 9) as i16 - 4,
                    },
                    code: HuffmanCode { bits, len },
                })
                .collect()
        })
        .as_slice()
}

/// Returns the standard AAC signed-quad codebook 1 for values -1..=1.
#[must_use]
pub fn aac_signed_quads1_table() -> &'static [HuffmanEntry<AacSpectralQuad>] {
    AAC_SIGNED_QUADS1_TABLE
        .get_or_init(|| {
            AAC_SIGNED_QUADS1_CODES
                .iter()
                .zip(AAC_SIGNED_QUADS1_LENS)
                .enumerate()
                .map(|(index, (&bits, len))| HuffmanEntry {
                    symbol: AacSpectralQuad {
                        v: (index / 27) as i16 - 1,
                        w: ((index / 9) % 3) as i16 - 1,
                        x: ((index / 3) % 3) as i16 - 1,
                        y: (index % 3) as i16 - 1,
                    },
                    code: HuffmanCode { bits, len },
                })
                .collect()
        })
        .as_slice()
}

/// Returns the standard AAC signed-quad codebook 2 for values -1..=1.
#[must_use]
pub fn aac_signed_quads2_table() -> &'static [HuffmanEntry<AacSpectralQuad>] {
    AAC_SIGNED_QUADS2_TABLE
        .get_or_init(|| {
            AAC_SIGNED_QUADS2_CODES
                .iter()
                .zip(AAC_SIGNED_QUADS2_LENS)
                .enumerate()
                .map(|(index, (&bits, len))| HuffmanEntry {
                    symbol: AacSpectralQuad {
                        v: (index / 27) as i16 - 1,
                        w: ((index / 9) % 3) as i16 - 1,
                        x: ((index / 3) % 3) as i16 - 1,
                        y: (index % 3) as i16 - 1,
                    },
                    code: HuffmanCode { bits, len },
                })
                .collect()
        })
        .as_slice()
}

/// Returns the standard AAC unsigned-quad codebook 3 for values 0..=2.
#[must_use]
pub fn aac_unsigned_quads3_table() -> &'static [HuffmanEntry<AacSpectralMagnitudeQuad>] {
    AAC_UNSIGNED_QUADS3_TABLE
        .get_or_init(|| {
            AAC_UNSIGNED_QUADS3_CODES
                .iter()
                .zip(AAC_UNSIGNED_QUADS3_LENS)
                .enumerate()
                .map(|(index, (&bits, len))| HuffmanEntry {
                    symbol: AacSpectralMagnitudeQuad {
                        v: (index / 27) as u16,
                        w: ((index / 9) % 3) as u16,
                        x: ((index / 3) % 3) as u16,
                        y: (index % 3) as u16,
                    },
                    code: HuffmanCode { bits, len },
                })
                .collect()
        })
        .as_slice()
}

/// Returns the standard AAC unsigned-quad codebook 4 for values 0..=2.
#[must_use]
pub fn aac_unsigned_quads4_table() -> &'static [HuffmanEntry<AacSpectralMagnitudeQuad>] {
    AAC_UNSIGNED_QUADS4_TABLE
        .get_or_init(|| {
            AAC_UNSIGNED_QUADS4_CODES
                .iter()
                .zip(AAC_UNSIGNED_QUADS4_LENS)
                .enumerate()
                .map(|(index, (&bits, len))| HuffmanEntry {
                    symbol: AacSpectralMagnitudeQuad {
                        v: (index / 27) as u16,
                        w: ((index / 9) % 3) as u16,
                        x: ((index / 3) % 3) as u16,
                        y: (index % 3) as u16,
                    },
                    code: HuffmanCode { bits, len },
                })
                .collect()
        })
        .as_slice()
}

/// Returns the standard AAC unsigned-pairs codebook 7 for magnitudes 0..=7.
#[must_use]
pub fn aac_unsigned_pairs7_table() -> &'static [HuffmanEntry<AacSpectralMagnitudePair>] {
    AAC_UNSIGNED_PAIRS7_TABLE
        .get_or_init(|| {
            AAC_UNSIGNED_PAIRS7_CODES
                .iter()
                .zip(AAC_UNSIGNED_PAIRS7_LENS)
                .enumerate()
                .map(|(index, (&bits, len))| HuffmanEntry {
                    symbol: AacSpectralMagnitudePair {
                        x: (index / 8) as u16,
                        y: (index % 8) as u16,
                    },
                    code: HuffmanCode { bits, len },
                })
                .collect()
        })
        .as_slice()
}

/// Returns the standard AAC unsigned-pairs codebook 8 for magnitudes 0..=7.
#[must_use]
pub fn aac_unsigned_pairs8_table() -> &'static [HuffmanEntry<AacSpectralMagnitudePair>] {
    AAC_UNSIGNED_PAIRS8_TABLE
        .get_or_init(|| {
            AAC_UNSIGNED_PAIRS8_CODES
                .iter()
                .zip(AAC_UNSIGNED_PAIRS8_LENS)
                .enumerate()
                .map(|(index, (&bits, len))| HuffmanEntry {
                    symbol: AacSpectralMagnitudePair {
                        x: (index / 8) as u16,
                        y: (index % 8) as u16,
                    },
                    code: HuffmanCode { bits, len },
                })
                .collect()
        })
        .as_slice()
}

/// Returns the standard AAC unsigned-pairs codebook 9 for magnitudes 0..=12.
#[must_use]
pub fn aac_unsigned_pairs9_table() -> &'static [HuffmanEntry<AacSpectralMagnitudePair>] {
    AAC_UNSIGNED_PAIRS9_TABLE
        .get_or_init(|| {
            AAC_UNSIGNED_PAIRS9_CODES
                .iter()
                .zip(AAC_UNSIGNED_PAIRS9_LENS)
                .enumerate()
                .map(|(index, (&bits, len))| HuffmanEntry {
                    symbol: AacSpectralMagnitudePair {
                        x: (index / 13) as u16,
                        y: (index % 13) as u16,
                    },
                    code: HuffmanCode { bits, len },
                })
                .collect()
        })
        .as_slice()
}

/// Returns the standard AAC unsigned-pairs codebook 10 for magnitudes 0..=12.
#[must_use]
pub fn aac_unsigned_pairs10_table() -> &'static [HuffmanEntry<AacSpectralMagnitudePair>] {
    AAC_UNSIGNED_PAIRS10_TABLE
        .get_or_init(|| {
            AAC_UNSIGNED_PAIRS10_CODES
                .iter()
                .zip(AAC_UNSIGNED_PAIRS10_LENS)
                .enumerate()
                .map(|(index, (&bits, len))| HuffmanEntry {
                    symbol: AacSpectralMagnitudePair {
                        x: (index / 13) as u16,
                        y: (index % 13) as u16,
                    },
                    code: HuffmanCode { bits, len },
                })
                .collect()
        })
        .as_slice()
}

/// Returns the standard AAC escape codebook 11 for magnitudes 0..=16.
///
/// Magnitude 16 is the escape sentinel; actual magnitudes above 16 are packed
/// by appending escape suffix bits after the Huffman codeword.
#[must_use]
pub fn aac_escape_table() -> &'static [HuffmanEntry<AacSpectralMagnitudePair>] {
    AAC_ESCAPE_TABLE
        .get_or_init(|| {
            AAC_ESCAPE_CODES
                .iter()
                .zip(AAC_ESCAPE_LENS)
                .enumerate()
                .map(|(index, (&bits, len))| HuffmanEntry {
                    symbol: AacSpectralMagnitudePair {
                        x: (index / 17) as u16,
                        y: (index % 17) as u16,
                    },
                    code: HuffmanCode { bits, len },
                })
                .collect()
        })
        .as_slice()
}

/// Returns the standard AAC unsigned-pairs codebook 7 entries for magnitudes 0/1.
///
/// This compatibility helper exposes the compact subset older diagnostics used;
/// new code should prefer `aac_unsigned_pairs7_table()`.
#[must_use]
pub fn aac_unsigned_pairs7_unit_magnitude_table(
) -> &'static [HuffmanEntry<AacSpectralMagnitudePair>] {
    AAC_UNSIGNED_PAIRS7_UNIT_MAGNITUDE_TABLE
}

/// Returns a minimal experimental AAC scale-factor delta table.
///
/// This is not the AAC-LC standard scale-factor Huffman table. It exists to
/// keep older tests deterministic; new production-shaped paths should use
/// `aac_scale_factor_delta_table()`.
#[must_use]
pub fn experimental_aac_scale_factor_delta_table() -> Vec<HuffmanEntry<AacScaleFactorDelta>> {
    (-16..=16)
        .enumerate()
        .map(|(index, delta)| HuffmanEntry {
            symbol: AacScaleFactorDelta::new(delta),
            code: HuffmanCode {
                bits: index as u32,
                len: 6,
            },
        })
        .collect()
}

/// Returns the standard AAC scale-factor Huffman table for DPCM deltas -60..=60.
#[must_use]
pub fn aac_scale_factor_delta_table() -> Vec<HuffmanEntry<AacScaleFactorDelta>> {
    AAC_SCALE_FACTOR_CODEBOOK_CODES
        .iter()
        .zip(AAC_SCALE_FACTOR_CODEBOOK_LENS)
        .enumerate()
        .map(|(index, (&bits, len))| HuffmanEntry {
            symbol: AacScaleFactorDelta::new(index as i16 - 60),
            code: HuffmanCode { bits, len },
        })
        .collect()
}

/// Returns the standard AAC scale-factor codebook entry for a zero DPCM delta.
///
/// Prefer `aac_scale_factor_delta_table()` when non-zero deltas are possible.
#[must_use]
pub fn aac_scale_factor_delta_zero_table() -> &'static [HuffmanEntry<AacScaleFactorDelta>] {
    AAC_SCALE_FACTOR_DELTA_ZERO_TABLE
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AacScaleFactorDelta {
    pub delta: i16,
}

impl AacScaleFactorDelta {
    #[must_use]
    pub fn new(delta: i16) -> Self {
        Self { delta }
    }
}
