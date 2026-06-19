//! Vorbis tone-seed application.
//!
//! Hand-ported to safe Rust from `seed_curve` in libvorbis/aoTuV `lib/psy.c`
//! (the portable, non-SSE path): stamps one spectral peak's tone-masking curve
//! into the octave-line seed buffer, choosing the drive-level curve by the
//! peak's amplitude and taking the running maximum so louder maskers win.
//! Derivative work of libvorbis/aoTuV (BSD-3-Clause); see `LICENSE-THIRDPARTY`.
//!
//! The seed buffer is indexed in eighth-octave lines: `seed_loop` drives
//! `seed_curve` per spectral peak, `seed_chase` resolves overlapping seeds, and
//! `max_seeds` scatters the result back onto the per-bin floor.

use crate::masking::{P_BANDS, P_LEVELS, P_LEVEL_0};
use crate::octave::PsyOctaveMap;
use crate::psy::EHMER_OFFSET;
use crate::tonecurve::ToneBinCurve;

/// Stamps the tone curve for a peak of amplitude `amp` (dB) into `seed`.
///
/// `curves` are the [`P_LEVELS`] bin-rendered curves for the peak's band; the
/// drive level is chosen from `amp + dboffset`. `oc` is the peak's octave-line
/// position and `linesper` the eighth-octave line spacing. Each curve point is
/// written `linesper` lines apart, as `amp + curve_value`, keeping the larger
/// of the existing and new seed (so the loudest contributor at each line wins).
pub fn seed_curve(
    seed: &mut [f32],
    curves: &[ToneBinCurve],
    amp: f32,
    oc: i32,
    linesper: i32,
    dboffset: f32,
) {
    let n = seed.len() as i32;

    // Pick the drive-level curve bracketing this amplitude.
    let choice = ((amp + dboffset - P_LEVEL_0) * 0.1) as i32;
    let choice = choice.clamp(0, P_LEVELS as i32 - 1) as usize;
    let Some(post) = curves.get(choice) else {
        return;
    };

    let lo = post.lo;
    let post1 = post.hi;
    let mut seedptr = oc + (lo - EHMER_OFFSET as i32) * linesper - (linesper >> 1);

    let mut i = lo;
    while i < post1 {
        if seedptr > 0 && seedptr < n {
            if let Some(&cv) = post.curve.get(i as usize) {
                let lin = amp + cv;
                let slot = &mut seed[seedptr as usize];
                if *slot < lin {
                    *slot = lin;
                }
            }
        }
        seedptr += linesper;
        if seedptr >= n {
            break;
        }
        i += 1;
    }
}

/// Seeds the whole spectrum (`seed_loop`): for each run of bins sharing an
/// octave-line coordinate it finds the peak amplitude, and where that peak
/// rises within 6 dB of the floor `flr` it stamps the band's tone curve into
/// `seed` via [`seed_curve`].
///
/// `f` is the per-bin amplitude (dB) and `flr` the per-bin floor (dB), both of
/// length `map.octave.len()`. `curves` are the per-band bin-rendered curves
/// (`[band][level]`). `seed` must be `map.total_octave_lines` long. `specmax`
/// is the spectrum's peak amplitude; `max_curve_db` the model's reference
/// (105 dB in the stock setups). The call is a no-op on mismatched lengths.
pub fn seed_loop(
    map: &PsyOctaveMap,
    curves: &[Vec<ToneBinCurve>],
    f: &[f32],
    flr: &[f32],
    seed: &mut [f32],
    specmax: f32,
    max_curve_db: f32,
) {
    let n = map.octave.len();
    if f.len() != n || flr.len() != n || curves.len() < P_BANDS {
        return;
    }
    let dboffset = max_curve_db - specmax;

    let mut i = 0;
    while i < n {
        let mut max = f[i];
        let oc = map.octave[i];
        // Collapse the run of bins that map to this octave line, keeping the
        // loudest as the masker amplitude.
        while i + 1 < n && map.octave[i + 1] == oc {
            i += 1;
            if f[i] > max {
                max = f[i];
            }
        }

        if max + 6.0 > flr[i] {
            let band = (oc >> map.shiftoc).clamp(0, P_BANDS as i32 - 1) as usize;
            // seed_curve takes its line count from seed.len(), which the caller
            // sizes to total_octave_lines.
            seed_curve(
                seed,
                &curves[band],
                max,
                map.octave[i] - map.firstoc,
                map.eighth_octave_lines,
                dboffset,
            );
        }
        i += 1;
    }
}

/// Resolves overlapping seeds into a single masking line in place
/// (`seed_chase`, non-SSE path): a linear-time stack walk keeps only the seeds
/// that still matter, then sweeps them out so each holds until the next
/// relevant seed (or `linesper + 1` lines, whichever comes first).
pub fn seed_chase(seeds: &mut [f32], linesper: i32) {
    let n = seeds.len();
    let mut posstack: Vec<i32> = Vec::with_capacity(n);
    let mut ampstack: Vec<f32> = Vec::with_capacity(n);

    for (i, &amp) in seeds.iter().enumerate() {
        let ii = i as i32;
        if posstack.len() < 2 {
            posstack.push(ii);
            ampstack.push(amp);
            continue;
        }
        loop {
            let top = ampstack.len() - 1;
            if amp < ampstack[top] {
                posstack.push(ii);
                ampstack.push(amp);
                break;
            }
            // A taller-or-equal seed within `linesper` of the previous two
            // fully overlaps the top one, making it irrelevant: pop and retry.
            if ii < posstack[top] + linesper
                && posstack.len() > 1
                && ampstack[top] <= ampstack[top - 1]
                && ii < posstack[top - 1] + linesper
            {
                posstack.pop();
                ampstack.pop();
                continue;
            }
            posstack.push(ii);
            ampstack.push(amp);
            break;
        }
    }

    // Sweep the surviving seeds straight through the buffer.
    let stack = posstack.len();
    let mut pos = 0usize;
    for i in 0..stack {
        let endpos = if i < stack - 1 && ampstack[i + 1] > ampstack[i] {
            i64::from(posstack[i + 1])
        } else {
            // The +1 keeps line 0 from being dropped in short frames.
            i64::from(posstack[i]) + i64::from(linesper) + 1
        };
        let endpos = endpos.clamp(0, n as i64) as usize;
        while pos < endpos {
            seeds[pos] = ampstack[i];
            pos += 1;
        }
    }
}

/// The "no seed here" sentinel in the octave-line seed buffer (`NEGINF`).
const NEGINF: f32 = -9999.0;

/// Reads octave-line `idx` of `seed`, treating out-of-range as [`NEGINF`].
/// On valid maps the walk never leaves the buffer; this only hardens the edges.
fn seed_at(seed: &[f32], idx: i32) -> f32 {
    if idx >= 0 && (idx as usize) < seed.len() {
        seed[idx as usize]
    } else {
        NEGINF
    }
}

/// Scatters the chased octave-line seed onto the per-bin floor (`max_seeds`,
/// non-SSE path): chases the seed, then for each MDCT bin takes the minimum
/// seed across the octave lines it spans (capped at `tone_abs_limit`) and
/// raises `flr` to it. `flr` is never lowered. `seed` is
/// `map.total_octave_lines` long and is consumed (chased) in place; `flr` is
/// per MDCT bin. The call is a no-op on mismatched lengths.
pub fn max_seeds(map: &PsyOctaveMap, seed: &mut [f32], flr: &mut [f32], tone_abs_limit: f32) {
    let lines = map.total_octave_lines;
    let bins = map.octave.len();
    if seed.len() as i32 != lines || flr.len() != bins || bins == 0 {
        return;
    }

    seed_chase(seed, map.eighth_octave_lines);

    let mut linpos = 0usize;
    let mut pos = map.octave[0] - map.firstoc - (map.eighth_octave_lines >> 1);

    while linpos + 1 < bins {
        let mut min_v = seed_at(seed, pos);
        // The octave line halfway to the next bin bounds this bin's seed span.
        let mut end = ((map.octave[linpos] + map.octave[linpos + 1]) >> 1) - map.firstoc;
        if min_v > tone_abs_limit {
            min_v = tone_abs_limit;
        }
        while pos < end {
            pos += 1;
            let s = seed_at(seed, pos);
            if (s > NEGINF && s < min_v) || min_v == NEGINF {
                min_v = s;
            }
        }

        end = pos + map.firstoc;
        while linpos < bins && map.octave[linpos] <= end {
            if flr[linpos] < min_v {
                flr[linpos] = min_v;
            }
            linpos += 1;
        }
    }

    // Any remaining bins take the last octave line's seed.
    let min_v = seed_at(seed, lines - 1);
    while linpos < bins {
        if flr[linpos] < min_v {
            flr[linpos] = min_v;
        }
        linpos += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::psy::EHMER_MAX;
    use crate::tonecurve::{tone_bin_curves, tone_level_curves};

    /// A flat tone curve spanning the whole Ehmer range at a fixed dB level.
    fn flat_curve(level_db: f32) -> ToneBinCurve {
        ToneBinCurve {
            lo: 0,
            hi: EHMER_MAX as i32,
            curve: [level_db; EHMER_MAX],
        }
    }

    /// Eight identical levels, so the amplitude choice never changes the curve.
    fn flat_levels(level_db: f32) -> Vec<ToneBinCurve> {
        (0..P_LEVELS).map(|_| flat_curve(level_db)).collect()
    }

    #[test]
    fn writes_strided_seed_values_taking_the_maximum() {
        // linesper 1, oc chosen so the first written line is index 5.
        let curves = flat_levels(-50.0);
        let mut seed = vec![-999.0f32; 64];
        // seedptr0 = oc + (0 - 16)*1 - 0 = oc - 16; pick oc = 21 -> start 5.
        seed_curve(&mut seed, &curves, 0.0, 21, 1, 0.0);
        // The 56-point curve stamps lines 5..=60; the rest stay untouched.
        for (idx, &v) in seed.iter().enumerate() {
            if (5..5 + EHMER_MAX).contains(&idx) {
                assert_eq!(v, -50.0, "line {idx}");
            } else {
                assert_eq!(v, -999.0, "untouched line {idx}");
            }
        }
    }

    #[test]
    fn keeps_a_louder_existing_seed() {
        let curves = flat_levels(-50.0);
        let mut seed = vec![0.0f32; 64]; // already louder than amp+curve = -50
        seed_curve(&mut seed, &curves, 0.0, 21, 1, 0.0);
        assert!(seed.iter().all(|&v| v == 0.0), "louder seed overwritten");
    }

    #[test]
    fn amplitude_offsets_the_written_level() {
        let curves = flat_levels(-50.0);
        let mut seed = vec![-999.0f32; 64];
        seed_curve(&mut seed, &curves, 30.0, 21, 1, 0.0);
        // amp 30 + curve -50 = -20.
        assert_eq!(seed[5], -20.0);
    }

    #[test]
    fn drive_level_choice_saturates_both_ends() {
        // Distinct per-level curves; the quietest amp picks level 0, the
        // loudest picks the last level.
        let curves: Vec<ToneBinCurve> = (0..P_LEVELS).map(|lv| flat_curve(-(lv as f32))).collect();
        let mut quiet = vec![-999.0f32; 64];
        seed_curve(&mut quiet, &curves, -100.0, 21, 1, 0.0);
        // level 0 curve == -0.0, so amp + 0 = -100.
        assert_eq!(quiet[5], -100.0);

        let mut loud = vec![-999.0f32; 64];
        seed_curve(&mut loud, &curves, 200.0, 21, 1, 0.0);
        // last level curve == -(P_LEVELS-1); amp + that.
        assert_eq!(loud[5], 200.0 - (P_LEVELS as f32 - 1.0));
    }

    #[test]
    fn line_zero_and_negative_positions_are_skipped() {
        // With oc small the early lines land at <= 0 and must be left alone.
        let curves = flat_levels(-50.0);
        let mut seed = vec![-999.0f32; 64];
        // seedptr0 = 16 - 16 = 0 -> the first line (index 0) is skipped (>0 only).
        seed_curve(&mut seed, &curves, 0.0, 16, 1, 0.0);
        assert_eq!(seed[0], -999.0, "line 0 must not be written");
        assert_eq!(seed[1], -50.0, "line 1 is the first writable line");
    }

    #[test]
    fn never_writes_out_of_bounds() {
        // A large oc pushes most lines past the buffer; the call must stay safe
        // and simply stop.
        let curves = flat_levels(-50.0);
        let mut seed = vec![-999.0f32; 8];
        seed_curve(&mut seed, &curves, 0.0, 1000, 1, 0.0);
        // Nothing within [0,8) is reachable from oc 1000, so all untouched.
        assert!(seed.iter().all(|&v| v == -999.0));
    }

    /// A realistic long-block setup with the stock 48 kHz curves.
    fn loop_fixture() -> (PsyOctaveMap, Vec<Vec<ToneBinCurve>>, usize) {
        let n = 1024;
        let rate = 48_000u32;
        let map = PsyOctaveMap::new(n, rate, 8);
        let level = tone_level_curves(&[0.0; P_BANDS], 0.0, 0.0);
        let curves = tone_bin_curves(&level, n, rate as f32 * 0.5 / n as f32);
        (map, curves, n)
    }

    #[test]
    fn a_loud_peak_seeds_the_buffer() {
        let (map, curves, n) = loop_fixture();
        // One loud bin well above an otherwise low floor.
        let mut f = vec![-100.0f32; n];
        f[400] = 40.0;
        let flr = vec![-80.0f32; n];
        let mut seed = vec![-999.0f32; map.total_octave_lines as usize];

        seed_loop(&map, &curves, &f, &flr, &mut seed, 40.0, 105.0);
        assert!(
            seed.iter().any(|&v| v > -999.0),
            "loud peak left the seed buffer empty",
        );
    }

    #[test]
    fn a_spectrum_below_the_floor_seeds_nothing() {
        let (map, curves, n) = loop_fixture();
        // Every bin sits far below its floor, so nothing clears the +6 dB gate.
        let f = vec![-200.0f32; n];
        let flr = vec![0.0f32; n];
        let mut seed = vec![-999.0f32; map.total_octave_lines as usize];

        seed_loop(&map, &curves, &f, &flr, &mut seed, -200.0, 105.0);
        assert!(
            seed.iter().all(|&v| v == -999.0),
            "inaudible spectrum seeded"
        );
    }

    #[test]
    fn mismatched_lengths_are_a_no_op() {
        let (map, curves, n) = loop_fixture();
        let f = vec![0.0f32; n];
        let flr = vec![-100.0f32; n - 1]; // wrong length
        let mut seed = vec![-999.0f32; map.total_octave_lines as usize];
        seed_loop(&map, &curves, &f, &flr, &mut seed, 0.0, 105.0);
        assert!(seed.iter().all(|&v| v == -999.0));
    }

    #[test]
    fn louder_peaks_seed_at_least_as_high() {
        // Raising the masker amplitude can only raise the seeded values.
        let (map, curves, n) = loop_fixture();
        let flr = vec![-80.0f32; n];

        let mut quiet_f = vec![-100.0f32; n];
        quiet_f[400] = 10.0;
        let mut quiet = vec![-999.0f32; map.total_octave_lines as usize];
        seed_loop(&map, &curves, &quiet_f, &flr, &mut quiet, 10.0, 105.0);

        let mut loud_f = vec![-100.0f32; n];
        loud_f[400] = 40.0;
        let mut loud = vec![-999.0f32; map.total_octave_lines as usize];
        seed_loop(&map, &curves, &loud_f, &flr, &mut loud, 40.0, 105.0);

        for (q, l) in quiet.iter().zip(&loud) {
            assert!(l >= q, "louder peak seeded lower: {l} < {q}");
        }
    }

    #[test]
    fn chase_leaves_a_flat_buffer_flat() {
        let mut seeds = vec![-42.0f32; 32];
        seed_chase(&mut seeds, 4);
        assert!(seeds.iter().all(|&v| v == -42.0), "flat buffer changed");
    }

    #[test]
    fn chase_preserves_the_global_maximum() {
        let original = [
            -90.0f32, -80.0, -70.0, -30.0, -85.0, -88.0, -60.0, -95.0, -50.0, -99.0,
        ];
        let mut seeds = original.to_vec();
        seed_chase(&mut seeds, 3);
        let in_max = original.iter().copied().fold(f32::MIN, f32::max);
        let out_max = seeds.iter().copied().fold(f32::MIN, f32::max);
        assert_eq!(out_max, in_max, "global peak lost");
    }

    #[test]
    fn chase_only_emits_original_seed_values() {
        let original = [
            -90.0f32, -80.0, -70.0, -30.0, -85.0, -88.0, -60.0, -95.0, -50.0, -99.0, -40.0, -77.0,
        ];
        let inputs: std::collections::HashSet<u32> = original.iter().map(|v| v.to_bits()).collect();
        let mut seeds = original.to_vec();
        seed_chase(&mut seeds, 3);
        for &v in &seeds {
            assert!(inputs.contains(&v.to_bits()), "fabricated value {v}");
        }
    }

    #[test]
    fn chase_spreads_a_lone_peak_forward() {
        // A single peak in a quiet field is held for a contiguous run no longer
        // than linesper + 1 lines (its exact start depends on the quiet collapse
        // ahead of it).
        let linesper = 4;
        let mut seeds = vec![-999.0f32; 32];
        seeds[10] = 0.0;
        seed_chase(&mut seeds, linesper);

        let zeros: Vec<usize> = seeds
            .iter()
            .enumerate()
            .filter(|(_, &v)| v == 0.0)
            .map(|(i, _)| i)
            .collect();
        assert!(!zeros.is_empty(), "peak vanished");
        assert!(
            zeros.len() <= linesper as usize + 1,
            "peak overran: {} lines",
            zeros.len()
        );
        for w in zeros.windows(2) {
            assert_eq!(w[1], w[0] + 1, "peak run not contiguous");
        }
    }

    #[test]
    fn chase_is_safe_on_tiny_buffers() {
        for len in 0..3usize {
            let mut seeds = vec![-10.0f32; len];
            seed_chase(&mut seeds, 4);
            assert!(seeds.iter().all(|&v| v == -10.0));
        }
    }

    const TONE_ABS_LIMIT: f32 = -40.0;

    #[test]
    fn max_seeds_raises_a_low_floor_to_an_uncapped_seed() {
        let map = PsyOctaveMap::new(1024, 48_000, 8);
        // A uniform seed below the absolute limit scatters through unchanged.
        let mut seed = vec![-50.0f32; map.total_octave_lines as usize];
        let mut flr = vec![-80.0f32; 1024];
        max_seeds(&map, &mut seed, &mut flr, TONE_ABS_LIMIT);
        assert!(flr.iter().all(|&v| v == -50.0), "floor not raised to seed");
    }

    #[test]
    fn max_seeds_caps_at_the_absolute_limit() {
        let map = PsyOctaveMap::new(1024, 48_000, 8);
        // A seed above the limit is clamped before it reaches the floor.
        let mut seed = vec![-10.0f32; map.total_octave_lines as usize];
        let mut flr = vec![-80.0f32; 1024];
        max_seeds(&map, &mut seed, &mut flr, TONE_ABS_LIMIT);
        assert!(
            flr.iter().all(|&v| v == TONE_ABS_LIMIT),
            "limit not applied"
        );
    }

    #[test]
    fn max_seeds_only_raises_the_floor() {
        let map = PsyOctaveMap::new(1024, 48_000, 8);
        let mut seed = vec![-60.0f32; map.total_octave_lines as usize];
        // Floor already louder than the seed in places; those must not drop.
        let original: Vec<f32> = (0..1024)
            .map(|i| if i % 2 == 0 { -30.0 } else { -90.0 })
            .collect();
        let mut flr = original.clone();
        max_seeds(&map, &mut seed, &mut flr, TONE_ABS_LIMIT);
        for (i, (&o, &v)) in original.iter().zip(&flr).enumerate() {
            assert!(v >= o, "floor lowered at {i}: {v} < {o}");
        }
        // The already-loud even bins are unchanged (seed -60 < -30).
        assert_eq!(flr[0], -30.0);
        // The quiet odd bins rise to the seed.
        assert_eq!(flr[1], -60.0);
    }

    #[test]
    fn max_seeds_with_an_empty_seed_leaves_the_floor_alone() {
        let map = PsyOctaveMap::new(1024, 48_000, 8);
        let mut seed = vec![NEGINF; map.total_octave_lines as usize];
        let mut flr = vec![-80.0f32; 1024];
        max_seeds(&map, &mut seed, &mut flr, TONE_ABS_LIMIT);
        assert!(
            flr.iter().all(|&v| v == -80.0),
            "empty seed altered the floor"
        );
    }

    #[test]
    fn max_seeds_is_a_no_op_on_length_mismatch() {
        let map = PsyOctaveMap::new(1024, 48_000, 8);
        let mut seed = vec![-50.0f32; map.total_octave_lines as usize];
        let mut flr = vec![-80.0f32; 512]; // wrong bin count
        max_seeds(&map, &mut seed, &mut flr, TONE_ABS_LIMIT);
        assert!(flr.iter().all(|&v| v == -80.0));
    }
}
