#![allow(unused_imports)]
#![deny(unsafe_code)]
#![warn(clippy::all)]

pub use sc_core::{
    compare_pcm, compare_pcm_with_tolerance, detect, AudioBuffer, Decoder, Encoder, Error, Format,
    HuffmanCode, HuffmanEntry, PackedBits, PcmDiff, PcmTolerance,
};

/// Granular MP3/AAC encoder primitives. Most callers want the crate-root
/// `encode`/`decode` functions instead; this module gathers the large unstable
/// surface (quantizers, Huffman packers, section planners, step selectors, table
/// accessors, and their config/result types) in one documented place.
pub mod low_level;
// Re-export the low-level surface at the crate root so existing paths keep
// working, but hide it from the crate-root rustdoc: the primitives are documented
// once, under `low_level`, instead of burying the high-level entry points.
#[doc(hidden)]
pub use low_level::*;

/// Decodes supported audio bytes into interleaved PCM.
mod codec;
pub use codec::*;
mod encode;
pub use encode::*;
// AAC profile/breakdown diagnostics are low-level: reachable under `low_level`
// rather than at the crate root.
pub(crate) mod aac_breakdown;
pub(crate) mod aac_profiles;
mod containers;
pub use containers::*;
/// Helpers shared by the wasm and python bindings (format-name routing,
/// encode-by-name, container detection, AAC scaffolding).
pub mod bindings_support;
mod tests;
