#![allow(unused_imports)]
#![deny(unsafe_code)]
#![warn(clippy::all)]

use sc_core::{
    apply_window, concat_packed_bits, mdct, pack_huffman_codes, pack_huffman_codes_with_len,
    pack_huffman_symbols_with_len, quantize_spectrum, sine_window, write_packed_bits, AudioBuffer,
    BitWriter as CoreBitWriter, Decoder, Encoder, Error, HuffmanCode, HuffmanEntry, PackedBits,
};
use std::sync::OnceLock;

mod config;
pub use config::*;
mod adts;
pub use adts::*;
mod spectral_types;
pub use spectral_types::*;
mod tables_quads;
pub use tables_quads::*;
mod tables_pairs;
pub use tables_pairs::*;
mod tables_api;
pub use tables_api::*;
mod section_plan;
pub use section_plan::*;
mod pack_sections;
pub use pack_sections::*;
mod raw_blocks;
pub use raw_blocks::*;
mod frame_encode;
pub use frame_encode::*;
mod stream_encode_mono;
pub use stream_encode_mono::*;
mod step_select_mono;
pub use step_select_mono::*;
mod step_select_stereo;
pub use step_select_stereo::*;
mod auto_step;
pub use auto_step::*;
mod util;
pub use util::*;
mod tests;
