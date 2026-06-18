//! The static CELT mode for 48 kHz (the only mode Opus uses).
//!
//! Hand-ported to safe Rust from the precomputed tables in libopus
//! `celt/static_modes_float.h` / `celt/modes.c` (`mode48000_960_120`).
//! Derivative work of libopus (BSD-3-Clause); see `LICENSE-THIRDPARTY`.
//!
//! [`CeltMode`] bundles the band layout, the allocation table, the preemphasis
//! coefficients, the overlap window and the per-`LM` pulse caches that the CELT
//! encoder and decoder share. The fixed tables live here as `const`s; the window
//! and the caches (derived, not tabulated) are built once by [`celt_mode_48k`].

// Consumed by the CELT encode/decode entry points; the live encoder still ships
// via the Opus FFI path.
#![allow(dead_code)]

use crate::mdct::compute_window;
use crate::rate::{compute_pulse_cache, PulseCache};

/// `eBands`: band edges in units of the shortest MDCT bin (`eband5ms`).
pub const E_BANDS: [i16; 22] = [
    0, 1, 2, 3, 4, 5, 6, 7, 8, 10, 12, 14, 16, 20, 24, 28, 34, 40, 48, 60, 78, 100,
];

/// `logN`: the log2 of each band's bin count in Q-bits (`logN400`).
pub const LOG_N: [i16; 21] = [
    0, 0, 0, 0, 0, 0, 0, 0, 8, 8, 8, 8, 16, 16, 16, 21, 21, 24, 29, 34, 36,
];

/// `allocVectors`: the 11 x 21 `band_allocation` table (bits/band per quality row).
#[rustfmt::skip]
pub const BAND_ALLOCATION: [u8; 11 * 21] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    90, 80, 75, 69, 63, 56, 49, 40, 34, 29, 20, 18, 10, 0, 0, 0, 0, 0, 0, 0, 0,
    110, 100, 90, 84, 78, 71, 65, 58, 51, 45, 39, 32, 26, 20, 12, 0, 0, 0, 0, 0, 0,
    118, 110, 103, 93, 86, 80, 75, 70, 65, 59, 53, 47, 40, 31, 23, 15, 4, 0, 0, 0, 0,
    126, 119, 112, 104, 95, 89, 83, 78, 72, 66, 60, 54, 47, 39, 32, 25, 17, 12, 1, 0, 0,
    134, 127, 120, 114, 103, 97, 91, 85, 78, 72, 66, 60, 54, 47, 41, 35, 29, 23, 16, 10, 1,
    144, 137, 130, 124, 113, 107, 101, 95, 88, 82, 76, 70, 64, 57, 51, 45, 39, 33, 26, 15, 1,
    152, 145, 138, 132, 123, 117, 111, 105, 98, 92, 86, 80, 74, 67, 61, 55, 49, 43, 36, 20, 1,
    162, 155, 148, 142, 133, 127, 121, 115, 108, 102, 96, 90, 84, 77, 71, 65, 59, 53, 46, 30, 1,
    172, 165, 158, 152, 143, 137, 131, 125, 118, 112, 106, 100, 94, 87, 81, 75, 69, 63, 56, 45, 20,
    200, 200, 200, 200, 200, 200, 200, 200, 198, 193, 188, 183, 178, 173, 168, 163, 158, 153, 148, 129, 104,
];

/// Number of energy bands (`nbEBands`).
pub const NB_E_BANDS: usize = 21;
/// The shortest MDCT's frame size (`shortMdctSize`).
pub const SHORT_MDCT_SIZE: usize = 120;
/// Number of short MDCTs in a long frame (`nbShortMdcts`).
pub const NB_SHORT_MDCTS: usize = 8;
/// The largest `LM` (log2 of `nbShortMdcts`).
pub const MAX_LM: i32 = 3;
/// Overlap (and window) length (`overlap`).
pub const OVERLAP: usize = 120;
/// Sample rate of this mode.
pub const SAMPLE_RATE: u32 = 48_000;
/// Pre-emphasis coefficient (`mode->preemph[0]`, `QCONST16(0.8500061f, 15)`).
pub const PREEMPH_COEF: f32 = 0.850_006_1;

/// The shared CELT mode: fixed tables plus the derived window and pulse caches.
pub struct CeltMode {
    pub sample_rate: u32,
    pub overlap: usize,
    pub nb_e_bands: usize,
    pub eff_e_bands: usize,
    pub short_mdct_size: usize,
    pub nb_short_mdcts: usize,
    pub max_lm: i32,
    pub e_bands: &'static [i16],
    pub log_n: &'static [i16],
    pub alloc_vectors: &'static [u8],
    pub nb_alloc_vectors: usize,
    pub preemph_coef: f32,
    pub window: Vec<f32>,
    /// One pulse cache per `LM` in `0..=max_lm`.
    pub cache: Vec<PulseCache>,
}

/// `celt_mode_48k`: build the standard 48 kHz CELT mode, deriving the overlap
/// window and the per-`LM` pulse caches from the fixed band tables.
#[must_use]
pub fn celt_mode_48k() -> CeltMode {
    let cache = (0..=MAX_LM)
        .map(|lm| compute_pulse_cache(&E_BANDS, &LOG_N, NB_E_BANDS, lm))
        .collect();
    CeltMode {
        sample_rate: SAMPLE_RATE,
        overlap: OVERLAP,
        nb_e_bands: NB_E_BANDS,
        eff_e_bands: NB_E_BANDS,
        short_mdct_size: SHORT_MDCT_SIZE,
        nb_short_mdcts: NB_SHORT_MDCTS,
        max_lm: MAX_LM,
        e_bands: &E_BANDS,
        log_n: &LOG_N,
        alloc_vectors: &BAND_ALLOCATION,
        nb_alloc_vectors: 11,
        preemph_coef: PREEMPH_COEF,
        window: compute_window(OVERLAP),
        cache,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_tables_are_consistent() {
        let m = celt_mode_48k();
        assert_eq!(m.e_bands.len(), m.nb_e_bands + 1);
        assert_eq!(m.log_n.len(), m.nb_e_bands);
        assert_eq!(m.alloc_vectors.len(), m.nb_alloc_vectors * m.nb_e_bands);
        // The long frame spans nbShortMdcts short blocks.
        assert_eq!(m.short_mdct_size * m.nb_short_mdcts, 960);
        assert_eq!(1usize << m.max_lm, m.nb_short_mdcts);
        // The top band edge (in shortest-MDCT bins) is 100 for this mode.
        assert_eq!(*m.e_bands.last().unwrap(), 100);
    }

    #[test]
    fn mode_window_is_power_complementary() {
        let m = celt_mode_48k();
        assert_eq!(m.window.len(), m.overlap);
        for i in 0..m.overlap {
            let s = m.window[i] * m.window[i]
                + m.window[m.overlap - 1 - i] * m.window[m.overlap - 1 - i];
            assert!((s - 1.0).abs() < 1e-5, "window not PR at {i}");
        }
    }

    #[test]
    fn mode_has_one_cache_per_lm() {
        let m = celt_mode_48k();
        assert_eq!(m.cache.len(), (m.max_lm + 1) as usize);
        // Every band has a cache index entry at each LM.
        for c in &m.cache {
            assert!(!c.index.is_empty());
            assert!(!c.caps.is_empty());
        }
    }

    #[test]
    fn mode_preemph_coef_is_085() {
        let m = celt_mode_48k();
        assert!((m.preemph_coef - 0.85).abs() < 1e-4);
    }
}
