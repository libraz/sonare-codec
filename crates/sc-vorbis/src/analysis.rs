//! Vorbis psychoacoustic analysis driver.
//!
//! Ties the hand-ported psy primitives into the tone-masking pipeline of
//! `_vp_tonemask` (libvorbis/aoTuV `lib/psy.c`, non-SSE path): the per-block
//! state (octave map, bin-rendered tone curves, per-bin ATH) is precomputed
//! once per blocksize/sample-rate, then [`PsyAnalysis::tonemask`] turns a
//! log-magnitude spectrum into a tone-masking curve. Derivative work of
//! libvorbis/aoTuV (BSD-3-Clause); see `LICENSE-THIRDPARTY`.
//!
//! The noise-floor mask ([`bark_noise_hybridmp`](crate::noise)) and the final
//! tone/noise mix ([`offset_and_mix`](crate::noise)) are combined onto this
//! tone mask by the caller; the AoTuV noise-companding refinement is a separate
//! stage.

// Drives the analysis stages; the live encoder still ships via FFI.
#![allow(dead_code)]

use crate::masking::{build_bin_ath, P_BANDS};
use crate::mdct::mdct_forward;
use crate::noise::{bark_noise_hybridmp, build_bark_windows, offset_and_mix};
use crate::octave::PsyOctaveMap;
use crate::psy::to_db;
use crate::seed::{max_seeds, seed_loop};
use crate::tonecurve::{tone_bin_curves, tone_level_curves, ToneBinCurve};
use crate::window::vorbis_window;

/// The "no seed" sentinel matching libvorbis `NEGINF`.
const NEGINF: f32 = -9999.0;

/// Standard psy parameters (the `_psy_info_template` / stock 48 kHz values).
const ATH_ADJATT: f32 = -140.0;
const ATH_MAXATT: f32 = -140.0;
const MAX_CURVE_DB: f32 = 105.0;
const TONE_ABS_LIMIT: f32 = -40.0;
const EIGHTH_OCTAVE_LINES: i32 = 8;
/// Tone master attenuation (`tone_masteratt[0]`, template value).
const TONEATT: f32 = 0.0;
/// Per-bin noise offset (`noiseoffset`, template value).
const NOISEOFFSET: f32 = -1.0;
/// Noise suppression ceiling (`noisemaxsupp`, template value).
const NOISEMAXSUPP: f32 = 0.0;
/// Bark noise-window widths and minimum bin widths.
const NOISEWINDOW_BARK: f32 = 0.5;
const NOISEWINDOW_MIN: i32 = 1;
/// Offset the first noise regression pass adds before clamping (`140` dB).
const NOISE_OFFSET_DB: f32 = 140.0;

/// Precomputed psychoacoustic state for one blocksize and sample rate.
pub struct PsyAnalysis {
    n: usize,
    octave_map: PsyOctaveMap,
    tone_curves: Vec<Vec<ToneBinCurve>>,
    bin_ath: Vec<f32>,
    bark: Vec<i32>,
    noiseoffset: Vec<f32>,
}

impl PsyAnalysis {
    /// Builds the analysis state for `n` MDCT bins at `rate` Hz.
    #[must_use]
    pub fn new(n: usize, rate: u32) -> Self {
        let octave_map = PsyOctaveMap::new(n, rate, EIGHTH_OCTAVE_LINES);
        // Stock tone curves: no per-band attenuation, no centre boost/decay.
        let level = tone_level_curves(&[0.0; P_BANDS], 0.0, 0.0);
        let bin_hz = if n == 0 {
            0.0
        } else {
            rate as f32 * 0.5 / n as f32
        };
        let tone_curves = tone_bin_curves(&level, n, bin_hz);
        let bin_ath = build_bin_ath(n, rate);
        let bark = build_bark_windows(
            n,
            rate,
            NOISEWINDOW_BARK,
            NOISEWINDOW_BARK,
            NOISEWINDOW_MIN,
            NOISEWINDOW_MIN,
        );
        let noiseoffset = vec![NOISEOFFSET; n];
        Self {
            n,
            octave_map,
            tone_curves,
            bin_ath,
            bark,
            noiseoffset,
        }
    }

    /// Number of MDCT bins this state was built for.
    #[must_use]
    pub fn n(&self) -> usize {
        self.n
    }

    /// Windows and transforms one `2n`-sample PCM block into its `n` MDCT
    /// coefficients and their log-magnitude spectrum (`logmdct[i] = todB`).
    ///
    /// This is the analysis front-end: the `logmdct` it returns drives both the
    /// masking model ([`tonemask`](Self::tonemask)) and the floor fit. Returns
    /// `None` unless `pcm` is exactly `2 * n` samples.
    #[must_use]
    pub fn mdct_analysis(&self, pcm: &[f32]) -> Option<(Vec<f32>, Vec<f32>)> {
        if self.n == 0 || pcm.len() != 2 * self.n {
            return None;
        }
        let window = vorbis_window(pcm.len());
        let windowed: Vec<f32> = pcm.iter().zip(&window).map(|(&s, &w)| s * w).collect();
        let mdct = mdct_forward(&windowed);
        let logmdct: Vec<f32> = mdct.iter().map(|&c| to_db(c)).collect();
        Some((mdct, logmdct))
    }

    /// Computes the tone-masking curve for the log-magnitude spectrum `logfft`
    /// (`_vp_tonemask`): floor the mask at the ATH lifted to sit a fixed
    /// attenuation below the spectrum peak, then scatter each spectral peak's
    /// tone curve on top. Returns an empty vector on a size mismatch.
    #[must_use]
    pub fn tonemask(&self, logfft: &[f32]) -> Vec<f32> {
        if logfft.len() != self.n || self.tone_curves.len() < P_BANDS || self.n == 0 {
            return Vec::new();
        }

        let specmax = logfft.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        // The ATH floats a fixed attenuation below the local peak, never below
        // the absolute floor.
        let att = (specmax + ATH_ADJATT).max(ATH_MAXATT);

        let mut mask: Vec<f32> = self.bin_ath.iter().map(|a| a + att).collect();
        let mut seed = vec![NEGINF; self.octave_map.total_octave_lines.max(0) as usize];

        seed_loop(
            &self.octave_map,
            &self.tone_curves,
            logfft,
            &mask,
            &mut seed,
            specmax,
            MAX_CURVE_DB,
        );
        max_seeds(&self.octave_map, &mut seed, &mut mask, TONE_ABS_LIMIT);
        mask
    }

    /// Computes the full per-bin masking curve `logmask` for `logmdct`: the
    /// tone mask ([`tonemask`](Self::tonemask)) combined with the Bark-window
    /// noise floor and mixed so the louder of the two masks each bin.
    ///
    /// This is the base masking model (the AoTuV noise-companding refinement is
    /// not yet applied). Returns an empty vector on a size mismatch.
    #[must_use]
    pub fn logmask(&self, logmdct: &[f32]) -> Vec<f32> {
        if logmdct.len() != self.n || self.bark.len() != self.n || self.n == 0 {
            return Vec::new();
        }
        let tone = self.tonemask(logmdct);
        if tone.is_empty() {
            return Vec::new();
        }

        // First-pass Bark regression: the local noise floor of the spectrum.
        let mut noise = vec![0.0f32; self.n];
        bark_noise_hybridmp(&self.bark, logmdct, &mut noise, NOISE_OFFSET_DB, -1);

        let mut logmask = vec![0.0f32; self.n];
        offset_and_mix(
            &noise,
            &tone,
            &self.noiseoffset,
            TONEATT,
            NOISEMAXSUPP,
            &mut logmask,
        );
        logmask
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A quiet spectrum with one strong tonal peak at `peak_bin`.
    fn tonal_spectrum(n: usize, peak_bin: usize, floor: f32, peak: f32) -> Vec<f32> {
        let mut s = vec![floor; n];
        s[peak_bin] = peak;
        s
    }

    #[test]
    fn tonemask_has_one_value_per_bin() {
        let psy = PsyAnalysis::new(1024, 48_000);
        let mask = psy.tonemask(&vec![-80.0; 1024]);
        assert_eq!(mask.len(), 1024);
        assert!(mask.iter().all(|v| v.is_finite()));
    }

    #[test]
    fn tonemask_rejects_a_size_mismatch() {
        let psy = PsyAnalysis::new(1024, 48_000);
        assert!(psy.tonemask(&vec![-80.0; 512]).is_empty());
    }

    #[test]
    fn a_tonal_peak_raises_the_local_mask() {
        // A strong peak in the midrange must mask its neighbourhood louder than
        // a far-away quiet region.
        let n = 1024;
        let psy = PsyAnalysis::new(n, 48_000);
        let spectrum = tonal_spectrum(n, 300, -120.0, 0.0);
        let mask = psy.tonemask(&spectrum);

        let near = mask[305];
        let far = mask[900];
        assert!(
            near > far,
            "peak did not raise its neighbourhood: {near} <= {far}"
        );
    }

    #[test]
    fn tonemask_is_deterministic() {
        let psy = PsyAnalysis::new(512, 44_100);
        let spectrum = tonal_spectrum(512, 100, -110.0, -10.0);
        assert_eq!(psy.tonemask(&spectrum), psy.tonemask(&spectrum));
    }

    #[test]
    fn mdct_analysis_rejects_a_wrong_length_block() {
        let psy = PsyAnalysis::new(512, 48_000);
        assert!(psy.mdct_analysis(&vec![0.0; 512]).is_none()); // needs 1024
        assert!(psy.mdct_analysis(&vec![0.0; 1024]).is_some());
    }

    #[test]
    fn mdct_analysis_peaks_at_a_tone_frequency() {
        // A pure sinusoid concentrates its MDCT energy near its own bin, so the
        // log spectrum peaks there and sits far lower in a distant region.
        let n = 1024;
        let rate = 48_000.0f32;
        let psy = PsyAnalysis::new(n, 48_000);
        let bin = 200;
        let freq = bin as f32 * rate / (2.0 * n as f32);
        let pcm: Vec<f32> = (0..2 * n)
            .map(|i| (2.0 * std::f32::consts::PI * freq * i as f32 / rate).sin())
            .collect();

        let (mdct, logmdct) = psy.mdct_analysis(&pcm).expect("analysis");
        assert_eq!(mdct.len(), n);
        assert_eq!(logmdct.len(), n);
        // logmdct is the dB magnitude of each coefficient.
        for (i, (&c, &l)) in mdct.iter().zip(&logmdct).enumerate() {
            assert!((l - to_db(c)).abs() < 1e-3, "bin {i}");
        }
        // The tone bin dominates a faraway bin by a wide dB margin.
        assert!(logmdct[bin] > logmdct[700] + 20.0, "tone not concentrated");
    }

    #[test]
    fn silence_analyses_to_a_very_low_spectrum() {
        let n = 256;
        let psy = PsyAnalysis::new(n, 44_100);
        let (_, logmdct) = psy.mdct_analysis(&vec![0.0; 2 * n]).expect("analysis");
        // Pure silence has no spectral energy.
        assert!(logmdct.iter().all(|&l| l < -200.0), "silence not quiet");
    }

    #[test]
    fn logmask_never_falls_below_the_tone_mask() {
        // The mix takes the louder of tone and noise, so logmask >= tonemask.
        let n = 1024;
        let psy = PsyAnalysis::new(n, 48_000);
        let spectrum = tonal_spectrum(n, 250, -110.0, -10.0);
        let tone = psy.tonemask(&spectrum);
        let logmask = psy.logmask(&spectrum);
        assert_eq!(logmask.len(), n);
        for i in 0..n {
            assert!(
                logmask[i] >= tone[i] - 1e-3,
                "bin {i}: {} < {}",
                logmask[i],
                tone[i]
            );
        }
    }

    #[test]
    fn logmask_is_finite_and_size_checked() {
        let psy = PsyAnalysis::new(512, 44_100);
        let spectrum = tonal_spectrum(512, 80, -100.0, -20.0);
        let logmask = psy.logmask(&spectrum);
        assert!(logmask.iter().all(|v| v.is_finite()));
        assert!(psy.logmask(&vec![-80.0; 256]).is_empty());
    }

    #[test]
    fn logmask_runs_on_a_real_mdct_block() {
        // End-to-end: PCM -> MDCT -> logmdct -> logmask, all finite.
        let n = 1024;
        let rate = 48_000.0f32;
        let psy = PsyAnalysis::new(n, 48_000);
        let freq = 250.0 * rate / (2.0 * n as f32);
        let pcm: Vec<f32> = (0..2 * n)
            .map(|i| 0.5 * (2.0 * std::f32::consts::PI * freq * i as f32 / rate).sin())
            .collect();
        let (_, logmdct) = psy.mdct_analysis(&pcm).expect("analysis");
        let logmask = psy.logmask(&logmdct);
        assert_eq!(logmask.len(), n);
        assert!(logmask.iter().all(|v| v.is_finite()), "non-finite logmask");
    }

    #[test]
    fn a_louder_peak_masks_at_least_as_much() {
        let n = 1024;
        let psy = PsyAnalysis::new(n, 48_000);
        let quiet = psy.tonemask(&tonal_spectrum(n, 300, -120.0, -40.0));
        let loud = psy.tonemask(&tonal_spectrum(n, 300, -120.0, 0.0));
        // Around the peak the louder tone never masks less.
        for i in 295..315 {
            assert!(
                loud[i] >= quiet[i] - 1e-3,
                "bin {i}: {} < {}",
                loud[i],
                quiet[i]
            );
        }
    }
}
