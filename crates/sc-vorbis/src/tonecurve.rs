//! Vorbis tone-curve level construction.
//!
//! Hand-ported to safe Rust from the per-band level-curve stage of
//! `setup_tone_curves` in libvorbis/aoTuV `lib/psy.c` (the part before the
//! octave bin-render): the six measured [`TONEMASKS`] curves are expanded to
//! [`P_LEVELS`] drive levels, given a centred boost/decay, normalised so the
//! driving amplitude is 0 dB, overlaid with the absolute threshold of hearing,
//! and the louder curves limited against the quieter ones. Derivative work of
//! libvorbis/aoTuV (BSD-3-Clause); see `LICENSE-THIRDPARTY`.
//!
//! The bin-render that resamples these `EHMER_MAX`-point curves onto MDCT bins
//! is a later stage; this module produces the resolution-independent curves it
//! consumes.

use crate::masking::{band_ath, P_BANDS, P_LEVELS, P_LEVEL_0};
use crate::psy::{attenuate_curve, from_oc, max_curve, min_curve, EHMER_MAX, EHMER_OFFSET};
use crate::tonemask::TONEMASKS;

/// One band's expanded tone curves, indexed `[level][ehmer point]`.
pub type LevelCurves = [[f32; EHMER_MAX]; P_LEVELS];

/// The drive level whose measured [`TONEMASKS`] curve seeds expanded level `j`.
/// Levels 0–2 all replicate the quietest measured curve (so the 30/40 dB slots
/// reuse the 50 dB data); levels 3–7 map straight onto measured curves 1–5.
fn source_level(j: usize) -> usize {
    j.saturating_sub(2)
}

/// Builds the expanded tone curves for every band, given the per-band tone
/// attenuation `curveatt_db`, the centre boost, and the centre decay rate.
///
/// Mirrors the first half of `setup_tone_curves`: replicate measured curves to
/// [`P_LEVELS`], apply the centred boost/decay, attenuate each level so its
/// driving amplitude sits at 0 dB, overlay the ATH, and limit each louder
/// curve against the one below it.
#[must_use]
pub fn tone_level_curves(
    curveatt_db: &[f32; P_BANDS],
    center_boost: f32,
    center_decay_rate: f32,
) -> Vec<LevelCurves> {
    let mut out = Vec::with_capacity(P_BANDS);

    for (band, &curveatt) in curveatt_db.iter().enumerate() {
        let ath = band_ath(band);
        let mut workc = [[0.0f32; EHMER_MAX]; P_LEVELS];

        // Replicate the measured curves into the expanded level slots, then
        // apply the centred boost/decay.
        for (j, level) in workc.iter_mut().enumerate() {
            let src = &TONEMASKS[band][source_level(j)];
            for (k, slot) in level.iter_mut().enumerate() {
                let dist = EHMER_OFFSET.abs_diff(k) as f32;
                let mut adj = center_boost + dist * center_decay_rate;
                // The decay may only ever push toward zero, never past it.
                if adj < 0.0 && center_boost > 0.0 {
                    adj = 0.0;
                }
                if adj > 0.0 && center_boost < 0.0 {
                    adj = 0.0;
                }
                *slot = src[k] + adj;
            }
        }

        // Normalise each level to a 0 dB driving amplitude and build the
        // ATH-overlaid limiting curves.
        let mut athc = [[0.0f32; EHMER_MAX]; P_LEVELS];
        for j in 0..P_LEVELS {
            let level_step = if j < 2 { 2 } else { j } as f32;
            attenuate_curve(
                &mut workc[j],
                curveatt + 100.0 - level_step * 10.0 - P_LEVEL_0,
            );
            athc[j] = ath;
            attenuate_curve(&mut athc[j], 100.0 - j as f32 * 10.0 - P_LEVEL_0);
            let (athc_j, workc_j) = (&mut athc[j], &workc[j]);
            max_curve(athc_j, workc_j);
        }

        // Limit the louder curves: a sound 20 dB down can only be in a 20-dB
        // lower playback range, so its curve is capped by the quieter one.
        for j in 1..P_LEVELS {
            let (lower, upper) = athc.split_at_mut(j);
            min_curve(&mut upper[0], &lower[j - 1]);
            min_curve(&mut workc[j], &upper[0]);
        }

        out.push(workc);
    }

    out
}

/// A tone curve resampled onto a band's MDCT bin grid. `curve` holds the
/// `EHMER_MAX` masking values; `lo`/`hi` are the fencepost indices bounding the
/// audible (> −200 dB) span, as the decoder-side application expects.
#[derive(Clone)]
pub struct ToneBinCurve {
    pub lo: i32,
    pub hi: i32,
    pub curve: [f32; EHMER_MAX],
}

/// Renders one source curve into the brute-force bin buffer, taking the running
/// minimum so any subsampling aliasing yields a safe (pessimistic) masking
/// value. `oc_center` is the octave centre the curve is positioned at.
fn render_pass(brute: &mut [f32], src: &[f32; EHMER_MAX], oc_center: f32, bin_hz: f32) {
    let nn = brute.len() as i32;
    let mut l: i32 = 0;
    for (j, &value) in src.iter().enumerate() {
        let pos = j as f32 * 0.125 + oc_center;
        let mut lo_bin = (from_oc(pos - 2.0625) / bin_hz) as i32;
        let mut hi_bin = (from_oc(pos - 1.9375) / bin_hz) as i32 + 1;
        lo_bin = lo_bin.clamp(0, nn);
        if lo_bin < l {
            l = lo_bin;
        }
        hi_bin = hi_bin.clamp(0, nn);
        while l < hi_bin && l < nn {
            let slot = &mut brute[l as usize];
            if *slot > value {
                *slot = value;
            }
            l += 1;
        }
    }
    // Beyond the curve's last measured point, extend with its tail value.
    let tail = src[EHMER_MAX - 1];
    while l < nn {
        let slot = &mut brute[l as usize];
        if *slot > tail {
            *slot = tail;
        }
        l += 1;
    }
}

/// Resamples the resolution-independent [`tone_level_curves`] onto the MDCT bin
/// grid (`n` bins, `bin_hz` Hz per bin) — the second half of
/// `setup_tone_curves`. For each band it composites every octave curve that can
/// land in the band's bins (plus a paranoid look at the next half octave),
/// pulls the per-bin minimum back into Ehmer coordinates, and marks the audible
/// span with fencepost indices. Returns `[band][level]` curves, or an empty
/// vector if `n` or `bin_hz` is non-positive.
#[must_use]
pub fn tone_bin_curves(
    level_curves: &[LevelCurves],
    n: usize,
    bin_hz: f32,
) -> Vec<Vec<ToneBinCurve>> {
    if n == 0 || bin_hz <= 0.0 || !bin_hz.is_finite() || level_curves.len() < P_BANDS {
        return Vec::new();
    }
    let nn = n as i32;
    let mut out = Vec::with_capacity(P_BANDS);

    for i in 0..P_BANDS {
        // Which octave curves can contribute to this band's bins?
        let bin = (from_oc(i as f32 * 0.5) / bin_hz).floor() as i32;
        let mut lo_curve = (to_oc_x2_ceil(bin as f32 * bin_hz + 1.0)) as i32;
        let mut hi_curve = (to_oc_x2_floor((bin + 1) as f32 * bin_hz)) as i32;
        if lo_curve > i as i32 {
            lo_curve = i as i32;
        }
        if lo_curve < 0 {
            lo_curve = 0;
        }
        if hi_curve >= P_BANDS as i32 {
            hi_curve = P_BANDS as i32 - 1;
        }

        let mut band_levels = Vec::with_capacity(P_LEVELS);
        for m in 0..P_LEVELS {
            let mut brute = vec![999.0f32; n];

            for k in lo_curve..=hi_curve {
                let k = k as usize;
                render_pass(&mut brute, &level_curves[k][m], k as f32 * 0.5, bin_hz);
            }

            // Paranoia: stay valid up to the next half octave, positioning the
            // next band's curve at this band's octave centre.
            if i + 1 < P_BANDS {
                render_pass(&mut brute, &level_curves[i + 1][m], i as f32 * 0.5, bin_hz);
            }

            // Pull the rendered bins back into Ehmer coordinates.
            let mut curve = [0.0f32; EHMER_MAX];
            for (j, slot) in curve.iter_mut().enumerate() {
                let pos = j as f32 * 0.125 + i as f32 * 0.5 - 2.0;
                let b = (from_oc(pos) / bin_hz) as i32;
                *slot = if b < 0 || b >= nn {
                    -999.0
                } else {
                    brute[b as usize]
                };
            }

            // Fenceposts: first and last points carrying audible masking.
            let mut lo = EHMER_OFFSET as i32;
            for (j, &v) in curve.iter().enumerate().take(EHMER_OFFSET) {
                if v > -200.0 {
                    lo = j as i32;
                    break;
                }
            }
            let mut hi = EHMER_OFFSET as i32 + 1;
            for j in ((EHMER_OFFSET + 2)..EHMER_MAX).rev() {
                if curve[j] > -200.0 {
                    hi = j as i32;
                    break;
                }
            }

            band_levels.push(ToneBinCurve { lo, hi, curve });
        }
        out.push(band_levels);
    }

    out
}

/// `ceil(toOC(x) * 2)`, guarding the `ln` against a non-positive argument.
fn to_oc_x2_ceil(x: f32) -> f32 {
    if x <= 0.0 {
        return f32::NEG_INFINITY;
    }
    (crate::psy::to_oc(x) * 2.0).ceil()
}

/// `floor(toOC(x) * 2)`, guarding the `ln` against a non-positive argument.
fn to_oc_x2_floor(x: f32) -> f32 {
    if x <= 0.0 {
        return f32::NEG_INFINITY;
    }
    (crate::psy::to_oc(x) * 2.0).floor()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// With zero boost/decay and zero per-band attenuation the construction
    /// reduces to clean integer arithmetic on the measured curves.
    const ZERO_ATT: [f32; P_BANDS] = [0.0; P_BANDS];

    #[test]
    fn produces_one_curve_set_per_band() {
        let curves = tone_level_curves(&ZERO_ATT, 0.0, 0.0);
        assert_eq!(curves.len(), P_BANDS);
    }

    #[test]
    fn quietest_level_is_the_attenuated_measured_curve() {
        // Level 0 is only attenuated (it is never limited), and with no boost
        // the attenuation is +100 - 2*10 - 30 = +50 dB over measured curve 0.
        let curves = tone_level_curves(&ZERO_ATT, 0.0, 0.0);
        for band in 0..P_BANDS {
            for k in 0..EHMER_MAX {
                let expect = TONEMASKS[band][0][k] + 50.0;
                assert_eq!(curves[band][0][k], expect, "band {band} k {k}");
            }
        }
    }

    #[test]
    fn no_mask_regions_stay_far_below_audibility() {
        // The -999 sentinel survives every stage (it is far lower than any ATH
        // overlay), shifted only by the per-level attenuation.
        let curves = tone_level_curves(&ZERO_ATT, 0.0, 0.0);
        // Band 0, level 0, point 33 is a -999 sentinel; attenuation is +50.
        assert_eq!(curves[0][0][33], -949.0);
        // Levels 1 and 2 also seed from measured curve 0, so the same sentinel
        // survives their limiting stage unchanged (it sits far below any ATH).
        assert_eq!(curves[0][2][33], -949.0);
    }

    #[test]
    fn centre_boost_lifts_the_unlimited_level_by_its_amount() {
        // Level 0 is never limited, so a centre boost shows up undiluted at the
        // driving point (distance 0 -> adjustment == boost).
        let base = tone_level_curves(&ZERO_ATT, 0.0, 0.0);
        let boosted = tone_level_curves(&ZERO_ATT, 3.0, 0.0);
        let k = EHMER_OFFSET;
        for band in 0..P_BANDS {
            assert!(
                (boosted[band][0][k] - base[band][0][k] - 3.0).abs() < 1e-4,
                "band {band}",
            );
        }
    }

    #[test]
    fn every_value_is_finite_and_bounded_by_the_quietest_overlay() {
        let curves = tone_level_curves(&ZERO_ATT, 0.0, 0.0);
        for band in &curves {
            for level in band {
                for &v in level {
                    assert!(v.is_finite(), "non-finite {v}");
                    // No curve rises above the loudest plausible overlay value.
                    assert!(v < 100.0, "implausibly loud {v}");
                }
            }
        }
    }

    /// A short-block grid: 48 kHz, 256-sample block -> 128 bins, 187.5 Hz/bin.
    fn short_block_bins() -> Vec<Vec<ToneBinCurve>> {
        let level = tone_level_curves(&ZERO_ATT, 0.0, 0.0);
        tone_bin_curves(&level, 128, 24_000.0 / 128.0)
    }

    #[test]
    fn bin_render_has_one_curve_per_band_and_level() {
        let curves = short_block_bins();
        assert_eq!(curves.len(), P_BANDS);
        for band in &curves {
            assert_eq!(band.len(), P_LEVELS);
        }
    }

    #[test]
    fn bin_render_rejects_degenerate_grids() {
        let level = tone_level_curves(&ZERO_ATT, 0.0, 0.0);
        assert!(tone_bin_curves(&level, 0, 187.5).is_empty());
        assert!(tone_bin_curves(&level, 128, 0.0).is_empty());
        assert!(tone_bin_curves(&level, 128, -1.0).is_empty());
    }

    #[test]
    fn fenceposts_bound_the_audible_span() {
        for band in &short_block_bins() {
            for c in band {
                // Indices stay within their defined ranges.
                assert!((0..=EHMER_OFFSET as i32).contains(&c.lo), "lo {}", c.lo);
                assert!(
                    (EHMER_OFFSET as i32 + 1..EHMER_MAX as i32).contains(&c.hi),
                    "hi {}",
                    c.hi
                );
                // Points before the low fencepost are inaudible by definition.
                for j in 0..c.lo as usize {
                    assert!(c.curve[j] <= -200.0, "audible below lo at {j}");
                }
                // The fencepost itself carries audible masking when one exists.
                if c.lo < EHMER_OFFSET as i32 {
                    assert!(c.curve[c.lo as usize] > -200.0);
                }
            }
        }
    }

    /// A long-block grid: 48 kHz, 2048-sample block -> 1024 bins, 23.4 Hz/bin.
    /// Coarse grids cannot resolve the lowest bands (a sub-bin tone yields the
    /// empty curve, signalled by the default fenceposts); a long block resolves
    /// the bands from a few hundred Hz upward.
    fn long_block_bins() -> Vec<Vec<ToneBinCurve>> {
        let level = tone_level_curves(&ZERO_ATT, 0.0, 0.0);
        tone_bin_curves(&level, 1024, 24_000.0 / 1024.0)
    }

    #[test]
    fn resolved_bands_render_audible_masking_content() {
        // Bands from ~260 Hz up (index 5 onward) resolve at this grid, so every
        // level must carry a real masking onset, not the empty-curve sentinel.
        let curves = long_block_bins();
        for (b, band) in curves.iter().enumerate().skip(5) {
            for (m, c) in band.iter().enumerate() {
                assert!(c.lo < EHMER_OFFSET as i32, "band {b} level {m} empty");
                assert!(c.curve.iter().any(|&v| v > -200.0), "band {b} level {m}");
            }
        }
    }

    #[test]
    fn all_rendered_values_stay_finite() {
        for grid in [short_block_bins(), long_block_bins()] {
            for band in &grid {
                for c in band {
                    assert!(c.curve.iter().all(|v| v.is_finite()));
                }
            }
        }
    }
}
