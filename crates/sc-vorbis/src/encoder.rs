#![allow(unused_imports)]
//! Pure-Rust Vorbis encoder producing a standard, decoder-compatible stream.
//!
//! Unlike [`crate::stream`] (a self-contained proof format), this emits a real
//! Ogg Vorbis bitstream: the codebooks, floor1, residue, mapping and mode are
//! serialized into a spec setup header ([`crate::setup`]), and each audio packet
//! carries the spec framing (audio bit, mode, per-channel floor, then residue)
//! coded with the *same* codebook objects. A standard decoder (Symphonia, the
//! library's decode path) therefore decodes it. Derivative work of
//! libvorbis/aoTuV (BSD-3-Clause) via its components; see `LICENSE-THIRDPARTY`.
//!
//! Scope: two block sizes with switching. Steady content uses the long block
//! (2048-sample window), which resolves tones into few bins so most of the
//! spectrum is empty; a transient switches that grid slot to a group of short
//! blocks (256-sample window) bracketed by transition windows, localizing the
//! attack so it does not pre-echo. The long and short coders share one set of
//! codebooks (a floor and residue config per block size). A strongly-correlated
//! stereo pair is square-polar coupled (the angle channel's residue collapses to
//! zero and skips); decorrelated channels are coded independently.
//!
//! The two residue value books (a fine `0.25`-step book and a wide coarse book
//! for a cascade second stage) are built per stream: a first analysis pass
//! histograms the entries each cascade stage would code, Huffman books are
//! fitted to those distributions (so common near-zero values get short
//! codewords), and they are serialized into the setup header. The books change
//! only codeword *lengths*, not the quantization grid, so the reconstruction is
//! identical to flat books — only the bitrate drops. Empty residue partitions
//! are skipped, quiet ones use the fine book alone, and loud (tonal-peak)
//! partitions add the coarse stage so the concentrated peak does not clip.

use sc_core::{AudioBuffer, Error};

use crate::analysis::PsyAnalysis;
use crate::block::{analyze_block_windowed, FLOOR_MULT};
use crate::codebook::{huffman_lengths, Codebook, VqBook};
use crate::floor1::{
    encode_post_deviations, low_high_neighbors, Floor1Class, Floor1Encoding, Floor1FitInfo,
    Floor1Fitter,
};
use crate::header::{pack_comment_header, pack_identification_header};
use crate::ogg_mux::mux_vorbis;
use crate::oggpack::BitWriter;
use crate::residue::ResidueConfig;
use crate::setup::{
    float32_pack, Floor1Setup, Mapping0Setup, ModeSetup, ResidueSetup, SetupConfig, StaticCodebook,
};
use crate::window::{vorbis_window, vorbis_window_lr};

/// Logical-stream serial number.
mod config;
pub use config::*;
mod residue_books;
pub use residue_books::*;
mod schedule;
pub use schedule::*;
mod encode_impl;
pub use encode_impl::*;
mod api;
pub use api::*;
mod tests;
