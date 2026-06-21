#![allow(unused_imports)]
#![deny(unsafe_code)]
#![warn(clippy::all)]

use wasm_bindgen::prelude::wasm_bindgen;

mod types;
pub use types::*;
mod decode;
pub use decode::*;
mod encode_basic;
pub use encode_basic::*;
// The AAC-specific JS surface (encode params, codebook tables, diagnostics) maps
// onto the umbrella's `aac`-gated API, so it only exists when that feature is on.
#[cfg(feature = "aac")]
mod encode_aac;
#[cfg(feature = "aac")]
pub use encode_aac::*;
#[cfg(feature = "aac")]
mod aac_tables;
#[cfg(feature = "aac")]
pub use aac_tables::*;
#[cfg(feature = "aac")]
mod aac_diagnostics;
#[cfg(feature = "aac")]
pub use aac_diagnostics::*;
mod mp3_diagnostics;
pub use mp3_diagnostics::*;
mod util;
pub use util::*;
mod tests;
