#![allow(unused_imports)]
//! CELT pitch analysis and post-filter (comb filter).
//!
//! Hand-ported to safe Rust from the float build of libopus: the comb filter
//! from `celt/celt.c` (`comb_filter` / `comb_filter_const_c`) and the pitch
//! estimator from `celt/pitch.c` (`celt_pitch_xcorr`, `find_best_pitch`,
//! `pitch_search`, `pitch_downsample`, `remove_doubling`, plus the
//! `_celt_autocorr` / `_celt_lpc` / `celt_fir5` it depends on). Derivative work
//! of libopus (BSD-3-Clause); see `LICENSE-THIRDPARTY`.
//!
//! The comb filter reinforces a periodic (pitched) component by adding delayed
//! copies of the signal at the pitch period `T`, weighted by a 3-tap kernel
//! selected by `tapset` and scaled by the post-filter gain. [`comb_filter`]
//! handles the gain/period *transition* at a frame boundary by cross-fading the
//! old filter `(T0, g0, tapset0)` into the new one `(T1, g1, tapset1)` over the
//! overlap window; the steady-state body is [`comb_filter_const`].
//!
//! The analysis side runs at half rate: [`pitch_downsample`] decimates the
//! input by two and LPC-whitens it, [`pitch_search`] finds the lag, and
//! [`remove_doubling`] corrects octave errors and reports the post-filter gain.
//!
//! Indexing note: the C reads `x[-T-2 .. -T+2]`, i.e. history *before* the first
//! output sample. Safe Rust can't index negatively, so the buffer is passed whole
//! with an explicit `head` offset and the routines read `x[head + i - T + k]`;
//! callers guarantee `head >= T + 2` so the history is in bounds.

// Consumed by the CELT prefilter (encoder) and matches the decoder post-filter;
// the live encoder still ships via the Opus FFI path.
#![allow(dead_code)]

use crate::range_coder::{RangeDecoder, RangeEncoder};

/// `COMBFILTER_MINPERIOD`: the shortest pitch period the comb filter accepts.
pub const COMBFILTER_MINPERIOD: usize = 15;
/// `COMBFILTER_MAXPERIOD`: the longest pitch period (history the buffer must
/// carry ahead of the first output sample).
pub const COMBFILTER_MAXPERIOD: usize = 1024;

/// The three 3-tap post-filter kernels (`gains[tapset][tap]`), as the exact Q15
/// constants from libopus expressed over 32768 so the float arithmetic matches.
const COMB_GAINS: [[f32; 3]; 3] = [
    [10048.0 / 32768.0, 7112.0 / 32768.0, 4248.0 / 32768.0],
    [15200.0 / 32768.0, 8784.0 / 32768.0, 0.0],
    [26208.0 / 32768.0, 3280.0 / 32768.0, 0.0],
];

/// `comb_filter_const`: the steady-state comb filter for a fixed period `t` and
/// 3-tap gains `(g10, g11, g12)`. Writes `y.len()` outputs; reads `x` from
/// `head - t - 2` to `head + y.len() - t + 1`.
mod comb;
pub use comb::*;
mod search;
pub use search::*;
mod prefilter;
pub use prefilter::*;
mod postfilter;
pub use postfilter::*;
mod tests;
