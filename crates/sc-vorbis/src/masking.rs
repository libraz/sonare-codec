//! Vorbis absolute threshold of hearing (ATH) and its per-band compositing.
//!
//! Hand-ported to safe Rust from libvorbis/aoTuV `lib/masking.h` (the `ATH`
//! table — the v6+ "Aoyumi" set the encoder actually uses) and the per-band
//! ATH reduction at the top of `setup_tone_curves` in `lib/psy.c`. The ATH is
//! the quiet-threshold curve the tone-masking setup overlays so low-level
//! masking curves do not fall to −∞. Derivative work of libvorbis/aoTuV
//! (BSD-3-Clause); see `LICENSE-THIRDPARTY`.

// Feeds the (not-yet-landed) tone-curve setup; the live encoder still ships via
// FFI until the analysis stages are wired in.
#![allow(dead_code)]

use crate::psy::{from_oc, EHMER_MAX};

/// Number of quarter-octave entries in the ATH table (`MAX_ATH`).
pub const MAX_ATH: usize = 88;

/// Number of psychoacoustic bands (`P_BANDS`, 62.5 Hz to 16 kHz).
pub const P_BANDS: usize = 17;

/// Number of drive levels the tone curves are expanded to (`P_LEVELS`,
/// 30 dB to 100 dB).
pub const P_LEVELS: usize = 8;

/// dB of the quietest expanded drive level (`P_LEVEL_0`).
pub const P_LEVEL_0: f32 = 30.0;

/// Absolute threshold of hearing in dB, sampled every quarter octave from
/// ~15 Hz upward (the v6+ "Aoyumi" set; libvorbis `ATH`). Values are negative
/// dB relative to the loudest representable tone (Vorbis 0 dB ≈ 100 dB SPL).
pub const ATH: [f32; MAX_ATH] = [
    -31.0, -33.0, -35.0, -37.0, -39.0, -41.0, -43.0, -45.0, // 15 Hz
    -47.0, -49.0, -51.0, -53.0, -55.0, -57.0, -59.0, -61.0, // 31 Hz
    -63.0, -65.0, -67.0, -69.0, -71.0, -73.0, -75.0, -77.0, // 63 Hz
    -79.0, -81.0, -83.0, -84.0, -85.0, -86.0, -87.0, -88.0, // 125 Hz
    -89.0, -90.0, -91.0, -92.0, -93.0, -94.0, -95.0, -96.0, // 250 Hz
    -96.0, -97.0, -97.0, -97.0, -98.0, -98.0, -98.0, -99.0, // 500 Hz
    -98.0, -97.0, -97.0, -98.0, -99.0, -100.0, -101.0, -101.0, // 1 kHz
    -102.0, -103.0, -104.0, -105.0, -106.0, -106.0, -107.0, -107.0, // 2 kHz
    -105.0, -104.0, -103.0, -102.0, -101.0, -99.0, -98.0, -97.0, // 4 kHz
    -96.0, -95.0, -95.0, -96.0, -97.0, -97.0, -93.0, -89.0, // 8 kHz
    -80.0, -70.0, -50.0, -40.0, -30.0, -26.0, -22.0, -18.0, // 16 kHz
];

/// Composites the ATH for psychoacoustic band `band` (`0..P_BANDS`) into an
/// `EHMER_MAX`-point curve, as the first stage of `setup_tone_curves` does.
///
/// Each Ehmer point `j` takes the minimum of the four ATH entries starting at
/// `j + band*4` — a half-band's setting must hold over the whole band, and it
/// is safer to mask too little, so the quietest (lowest-dB) threshold wins.
/// Indices past the table saturate on its last entry.
#[must_use]
pub fn band_ath(band: usize) -> [f32; EHMER_MAX] {
    let ath_offset = band * 4;
    let mut out = [0.0f32; EHMER_MAX];
    for (j, slot) in out.iter_mut().enumerate() {
        let mut min = 999.0f32;
        for k in 0..4 {
            let idx = j + k + ath_offset;
            let v = if idx < MAX_ATH {
                ATH[idx]
            } else {
                ATH[MAX_ATH - 1]
            };
            if v < min {
                min = v;
            }
        }
        *slot = min;
    }
    out
}

/// Interpolates the quarter-octave [`ATH`] table onto the `n` MDCT bins for a
/// given `rate` (the `p->ath` loop of `_vp_psy_init`), returning the per-bin
/// quiet threshold lifted by `+100` dB (the model's reference level).
///
/// Each ATH entry covers an eighth-octave span (`fromOC((i+1)*0.125 - 2)`
/// converted to a bin index); values are linearly interpolated within a span,
/// and bins past the table are linearly extrapolated from the last slope.
#[must_use]
pub fn build_bin_ath(n: usize, rate: u32) -> Vec<f32> {
    let mut ath = vec![0.0f32; n];
    if n == 0 || rate == 0 {
        return ath;
    }
    let nn = n as i32;
    let scale = 2.0 * n as f32 / rate as f32;

    let mut j: i32 = 0;
    for i in 0..MAX_ATH - 1 {
        let endpos = (from_oc((i as f32 + 1.0) * 0.125 - 2.0) * scale).round_ties_even() as i32;
        let mut base = ATH[i];
        if j < endpos {
            let delta = (ATH[i + 1] - base) / (endpos - j) as f32;
            while j < endpos && j < nn {
                ath[j as usize] = base + 100.0;
                base += delta;
                j += 1;
            }
        }
    }

    // Extrapolate the remaining high bins along the last segment's slope.
    if j >= 2 && (j as usize) < n {
        let mut cs = ath[j as usize - 1];
        let ds = ath[j as usize - 1] - ath[j as usize - 2];
        for slot in ath.iter_mut().skip(j as usize) {
            *slot = cs;
            cs += ds;
        }
    }
    ath
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ath_table_has_expected_extent() {
        assert_eq!(ATH.len(), MAX_ATH);
        // The set runs from a quiet-but-audible low band down to the very
        // sensitive midrange and back up toward the inaudible top octave.
        assert_eq!(ATH[0], -31.0);
        assert_eq!(ATH[MAX_ATH - 1], -18.0);
        // The most sensitive region (lowest threshold) is around 2 kHz.
        let min = ATH.iter().copied().fold(f32::INFINITY, f32::min);
        assert_eq!(min, -107.0);
    }

    #[test]
    fn band_ath_is_the_windowed_minimum() {
        // Directly recompute the 4-wide windowed min for a mid band.
        let band = 5;
        let composed = band_ath(band);
        for (j, &got) in composed.iter().enumerate() {
            let mut expect = f32::INFINITY;
            for k in 0..4 {
                let idx = j + k + band * 4;
                let v = if idx < MAX_ATH {
                    ATH[idx]
                } else {
                    ATH[MAX_ATH - 1]
                };
                expect = expect.min(v);
            }
            assert_eq!(got, expect, "j={j}");
        }
    }

    #[test]
    fn band_ath_is_finite_for_every_band() {
        for band in 0..P_BANDS {
            for &v in band_ath(band).iter() {
                assert!(v.is_finite(), "band {band} produced {v}");
                // A composited threshold never exceeds the loudest ATH entry.
                assert!(v <= -18.0, "band {band} value {v} above table max");
            }
        }
    }

    #[test]
    fn bin_ath_is_finite_and_starts_near_the_table_floor() {
        let ath = build_bin_ath(1024, 48_000);
        assert_eq!(ath.len(), 1024);
        assert!(ath.iter().all(|v| v.is_finite()), "non-finite bin ATH");
        // The first bin tracks the table's first entry lifted by +100 dB.
        assert!((ath[0] - (ATH[0] + 100.0)).abs() < 2.0, "bin0 = {}", ath[0]);
    }

    #[test]
    fn bin_ath_dips_in_the_sensitive_midrange() {
        // The ear is most sensitive (lowest threshold) in the low kHz; the
        // interpolated curve must dip there relative to the very low bins.
        let ath = build_bin_ath(1024, 48_000);
        let low = ath[2];
        let mid_min = ath[20..400].iter().copied().fold(f32::INFINITY, f32::min);
        assert!(
            mid_min < low,
            "midrange not more sensitive: {mid_min} vs {low}"
        );
    }

    #[test]
    fn bin_ath_degenerate_inputs_are_empty_or_zero() {
        assert!(build_bin_ath(0, 48_000).is_empty());
        assert_eq!(build_bin_ath(8, 0), vec![0.0; 8]);
    }

    #[test]
    fn high_band_tail_saturates_on_the_last_entry() {
        // Top band offset is 64; once j*1 + 64 reaches the table end every
        // window index is past it, so the tail collapses onto the final value.
        let top = band_ath(P_BANDS - 1);
        for (j, &v) in top.iter().enumerate() {
            if j + 64 >= MAX_ATH {
                assert_eq!(v, ATH[MAX_ATH - 1], "tail point j={j}");
            }
        }
    }
}
