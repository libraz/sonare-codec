#![allow(unused_imports)]
#![deny(unsafe_code)]
#![warn(clippy::all)]

use sc_core::{
    apply_window, concat_packed_bits, lookup_huffman_code, mdct, pack_huffman_codes,
    pack_huffman_codes_with_len, pack_huffman_symbols_with_len, quantize_spectrum, sine_window,
    AudioBuffer, BitWriter as CoreBitWriter, Decoder, Encoder, Error, HuffmanCode, HuffmanEntry,
    PackedBits,
};
use std::sync::OnceLock;

mod filterbank;
pub mod psychoacoustic;

mod scalefactor;
pub use scalefactor::*;
mod header;
pub use header::*;
mod api;
pub use api::*;
mod reservoir;
pub use reservoir::*;
mod frame_select;
pub use frame_select::*;
mod analysis_quant;
pub use analysis_quant::*;
mod huffman_types;
pub use huffman_types::*;
mod huffman_tables_low;
pub use huffman_tables_low::*;
mod huffman_tables_mid;
pub use huffman_tables_mid::*;
mod huffman_tables_high;
pub use huffman_tables_high::*;
mod huffman_pack;
pub use huffman_pack::*;
mod perceptual_pack;
pub use perceptual_pack::*;
mod frame_assembly;
pub use frame_assembly::*;
mod bitstream;
pub use bitstream::*;
mod tests;
