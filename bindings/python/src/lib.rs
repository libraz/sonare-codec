#![allow(unused_imports)]
#![deny(unsafe_code)]
#![warn(clippy::all)]

use pyo3::prelude::*;

mod types;
pub use types::*;
mod decode;
pub use decode::*;
mod encode_basic;
pub use encode_basic::*;
mod aac_encode;
pub use aac_encode::*;
mod aac_tables;
pub use aac_tables::*;
mod aac_diagnostics;
pub use aac_diagnostics::*;
mod mp3_diagnostics;
pub use mp3_diagnostics::*;
mod module;
pub use module::*;
mod util;
pub use util::*;
