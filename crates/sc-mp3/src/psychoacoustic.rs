#![allow(unused_imports)]
//! Psychoacoustic model primitives for MPEG-1 Layer III encoding.
//!
//! These are clean-room building blocks implemented from the public literature
//! (ISO/IEC 11172-3 Annex D Psychoacoustic Model 2, the Davis Pan tutorial, and
//! Painter & Spanias, "Perceptual Coding of Digital Audio"). Rather than copying
//! the spec's sample-rate-specific partition tables, the masking math is derived
//! from closed-form psychoacoustic functions — the Zwicker bark scale, the
//! Terhardt absolute threshold of hearing, and the Schroeder spreading function
//! — which are evaluated at runtime for the FFT bin frequencies.
//!
//! The half-spectrum transform uses an iterative radix-2 Cooley–Tukey FFT for
//! power-of-two lengths (the Layer III psychoacoustic FFT is 1024 points) and
//! falls back to a direct DFT otherwise. The direct DFT is retained as a
//! reference and cross-checked against the FFT in the tests.

use sc_core::Error;

/// Builds a periodic Hann (raised-cosine) analysis window of the given length.
///
/// The window is `0.5 · (1 − cos(2π·n / N))`, the standard window for the Layer
/// III psychoacoustic FFT. Returns an error for a zero-length request.
mod fft;
pub use fft::*;
mod masking;
pub use masking::*;
mod scalefactors;
pub use scalefactors::*;
mod stereo;
pub use stereo::*;
mod tests;
