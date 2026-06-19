//! Vorbis psychoacoustic octave-line map.
//!
//! Hand-ported to safe Rust from the octave-coordinate setup in `_vp_psy_init`
//! (libvorbis/aoTuV `lib/psy.c`): the per-bin mapping from MDCT bin index to a
//! fine octave-line coordinate that `seed_loop`/[`seed_curve`] stamp tone
//! curves into. Derivative work of libvorbis/aoTuV (BSD-3-Clause); see
//! `LICENSE-THIRDPARTY`.
//!
//! The line grid has `eighth_octave_lines` lines per eighth octave; `shiftoc`
//! is the right-shift that collapses an octave-line position back to a
//! psychoacoustic band index. The integer rounding follows the C verbatim
//! (truncation, or `+0.5` truncation where libvorbis rounds), computed in
//! `f64` so the truncation boundaries are stable.
//!
//! [`seed_curve`]: crate::seed::seed_curve

// `band_of` and `seed_pos` are accessors exercised by this module's tests,
// not the encoder.
#![allow(dead_code)]

use crate::masking::P_BANDS;

/// `toOC` evaluated in `f64` (the closed form from `scales.h`).
fn to_oc64(n: f64) -> f64 {
    n.ln() * std::f64::consts::LOG2_E - 5.965_784
}

/// The bin→octave-line mapping for one blocksize/sample-rate, plus the
/// constants `seed_loop` threads into [`seed_curve`](crate::seed::seed_curve).
pub struct PsyOctaveMap {
    /// Lines per eighth octave (the seed stride `linesper`).
    pub eighth_octave_lines: i32,
    /// Right-shift from an octave-line position to a band index.
    pub shiftoc: i32,
    /// Octave-line coordinate of the first line (subtracted to index the seed
    /// buffer).
    pub firstoc: i32,
    /// Length of the octave-line seed buffer.
    pub total_octave_lines: i32,
    /// Per-bin octave-line coordinate (`octave[i]`), length `n`.
    pub octave: Vec<i32>,
}

impl PsyOctaveMap {
    /// Builds the map for `n` MDCT bins at `rate` Hz with `eighth_octave_lines`
    /// lines per eighth octave. Returns an empty map for degenerate inputs.
    #[must_use]
    pub fn new(n: usize, rate: u32, eighth_octave_lines: i32) -> Self {
        let empty = || PsyOctaveMap {
            eighth_octave_lines,
            shiftoc: 0,
            firstoc: 0,
            total_octave_lines: 0,
            octave: Vec::new(),
        };
        if n == 0 || rate == 0 || eighth_octave_lines < 1 {
            return empty();
        }

        let nf = n as f64;
        let rate_f = f64::from(rate);

        // shiftoc = rint(log2(eighth_octave_lines * 8)) - 1.
        let shiftoc = (f64::from(eighth_octave_lines) * 8.0)
            .log2()
            .round_ties_even() as i32
            - 1;
        let shift = (shiftoc + 1).clamp(0, 30);
        let scale = f64::from(1i32 << shift);

        // firstoc and maxoc bound the octave-line coordinate range.
        let firstoc =
            (to_oc64(0.25 * rate_f * 0.5 / nf) * scale - f64::from(eighth_octave_lines)) as i32;
        let maxoc = (to_oc64((nf + 0.25) * rate_f * 0.5 / nf) * scale + 0.5) as i32;
        let total_octave_lines = maxoc - firstoc + 1;

        let octave = (0..n)
            .map(|i| (to_oc64((i as f64 + 0.25) * 0.5 * rate_f / nf) * scale + 0.5) as i32)
            .collect();

        PsyOctaveMap {
            eighth_octave_lines,
            shiftoc,
            firstoc,
            total_octave_lines,
            octave,
        }
    }

    /// The psychoacoustic band index for bin `i` (`octave[i] >> shiftoc`,
    /// clamped to a valid band), as `seed_loop` selects the curve set.
    #[must_use]
    pub fn band_of(&self, i: usize) -> usize {
        let oc = self.octave[i] >> self.shiftoc;
        oc.clamp(0, P_BANDS as i32 - 1) as usize
    }

    /// The octave-line seed position for bin `i` (`octave[i] - firstoc`), the
    /// `oc` argument [`seed_curve`](crate::seed::seed_curve) expects.
    #[must_use]
    pub fn seed_pos(&self, i: usize) -> i32 {
        self.octave[i] - self.firstoc
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn degenerate_inputs_yield_an_empty_map() {
        assert!(PsyOctaveMap::new(0, 48_000, 8).octave.is_empty());
        assert!(PsyOctaveMap::new(1024, 0, 8).octave.is_empty());
        assert!(PsyOctaveMap::new(1024, 48_000, 0).octave.is_empty());
    }

    #[test]
    fn standard_shiftoc_for_eight_lines_per_eighth() {
        // log2(8*8) = 6, so shiftoc = 5 and the line scale is 1<<6 = 64.
        let map = PsyOctaveMap::new(1024, 48_000, 8);
        assert_eq!(map.shiftoc, 5);
    }

    #[test]
    fn octave_coordinate_is_monotonic() {
        let map = PsyOctaveMap::new(1024, 48_000, 8);
        assert_eq!(map.octave.len(), 1024);
        for w in map.octave.windows(2) {
            assert!(w[1] >= w[0], "octave map dipped: {} -> {}", w[0], w[1]);
        }
    }

    #[test]
    fn first_seed_position_is_about_one_eighth_octave() {
        // firstoc and octave[0] share the same toOC argument, so their
        // difference is the +0.5 rounding plus eighth_octave_lines.
        let lines = 8;
        let map = PsyOctaveMap::new(1024, 48_000, lines);
        assert_eq!(map.seed_pos(0), lines);
    }

    #[test]
    fn every_seed_position_fits_the_octave_buffer() {
        // seed_loop passes total_octave_lines as the seed buffer length, so
        // every bin's position must land inside it.
        for &(n, rate) in &[(128usize, 48_000u32), (1024, 44_100), (2048, 48_000)] {
            let map = PsyOctaveMap::new(n, rate, 8);
            assert!(map.total_octave_lines > 0);
            for i in 0..n {
                let pos = map.seed_pos(i);
                assert!(
                    (0..map.total_octave_lines).contains(&pos),
                    "n={n} rate={rate} bin {i}: pos {pos} / {}",
                    map.total_octave_lines
                );
            }
        }
    }

    #[test]
    fn band_index_stays_in_range_and_rises() {
        let map = PsyOctaveMap::new(1024, 48_000, 8);
        let mut last = 0;
        for i in 0..1024 {
            let b = map.band_of(i);
            assert!(b < P_BANDS);
            assert!(b >= last, "band index dipped at {i}");
            last = b;
        }
    }
}
