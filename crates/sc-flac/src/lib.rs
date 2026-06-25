#![allow(unused_imports)]
#![deny(unsafe_code)]
#![warn(clippy::all)]

use md5::{Digest, Md5};
use sc_core::{AudioBuffer, BitReader, Decoder, Encoder, Error};

const FLAC_MARKER: &[u8; 4] = b"fLaC";
const METADATA_HEADER_LEN: usize = 4;
const STREAMINFO_BLOCK_TYPE: u8 = 0;
const STREAMINFO_LEN: usize = 34;
const ENCODE_BLOCK_SIZE: usize = 4096;

/// Integer sample width used when encoding PCM into FLAC.
///
/// FLAC is an integer codec, so float PCM is quantized to this depth. 24-bit
/// preserves roughly 256× finer detail than 16-bit and is the default for the
/// high-level umbrella `encode`; 16-bit produces smaller files for material that
/// originated at CD depth. For bit-exact float round-trips use WAV float instead.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FlacBitDepth {
    Bits16,
    Bits24,
}

impl FlacBitDepth {
    pub(crate) const fn bits(self) -> u8 {
        match self {
            Self::Bits16 => 16,
            Self::Bits24 => 24,
        }
    }

    /// Returns the low nibble of the FLAC frame-header byte that carries the
    /// sample-size code for this depth (the upper nibble is the channel
    /// assignment). ISO/FLAC sample-size codes: `0b100` → 16-bit, `0b110` →
    /// 24-bit, each shifted left one bit above the reserved bit.
    pub(crate) const fn frame_sample_size_nibble(self) -> u8 {
        match self {
            Self::Bits16 => 0b1000,
            Self::Bits24 => 0b1100,
        }
    }
}

mod types;
pub use types::*;
mod decode;
pub use decode::*;
mod encode;
pub use encode::*;
mod bitwriter;
pub use bitwriter::*;
mod frame_header;
pub use frame_header::*;
mod metadata;
pub use metadata::*;
mod tests;
