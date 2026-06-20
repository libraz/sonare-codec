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
mod encode_aac;
pub use encode_aac::*;
mod aac_tables;
pub use aac_tables::*;
mod aac_diagnostics;
pub use aac_diagnostics::*;
mod mp3_diagnostics;
pub use mp3_diagnostics::*;
mod util;
pub use util::*;
mod tests;
