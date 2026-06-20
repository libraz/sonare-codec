#![allow(unused_imports)]
#![deny(unsafe_code)]
#![warn(clippy::all)]

use md5::{Digest, Md5};
use sc_core::{AudioBuffer, BitReader, Decoder, Encoder, Error};

const FLAC_MARKER: &[u8; 4] = b"fLaC";
const METADATA_HEADER_LEN: usize = 4;
const STREAMINFO_BLOCK_TYPE: u8 = 0;
const STREAMINFO_LEN: usize = 34;
const ENCODE_BITS_PER_SAMPLE: u8 = 16;
const ENCODE_BLOCK_SIZE: usize = 4096;

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
