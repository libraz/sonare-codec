#![allow(unused_imports)]
//! Vorbis floor1 curve primitives.
//!
//! Hand-ported to safe Rust from libvorbis/aoTuV `lib/floor1.c`: the integer
//! line rasterizer (`render_point` / `render_line0`) that turns floor posts into
//! a per-bin floor curve, and the dB quantizer (`vorbis_dBquant`) used when
//! fitting that curve. Derivative work of libvorbis/aoTuV (BSD-3-Clause); see
//! `LICENSE-THIRDPARTY`.

// The floor1 reader and line rasterizer helpers are the decode-direction
// counterparts, exercised by this module's round-trip tests rather than the encoder.
#![allow(dead_code)]

use crate::codebook::{ov_ilog, Codebook};
use crate::oggpack::{BitReader, BitWriter};

/// Interpolates the integer floor value of the line `(x0,y0)-(x1,y1)` at `x`.
///
/// The high bit of `y0`/`y1` is a post "used" flag in libvorbis and is masked
/// off here exactly as in the C.
mod render;
pub use render::*;
mod fit;
pub use fit::*;
mod encode;
pub use encode::*;
mod tests;
