//! Vorbis single-channel block analysis.
//!
//! Wires the hand-ported analysis stages into the per-block encode pipeline of
//! `mapping0.c` (the analysis half): window + MDCT, the masking model, the
//! floor1 fit, the floor synthesis, and the residue division. The output is the
//! fitted floor posts plus the whitened residue an entropy stage would code.
//! Derivative work of libvorbis/aoTuV (BSD-3-Clause); see `LICENSE-THIRDPARTY`.
//!
//! This is the analysis pipeline only; packing the posts and residue into a
//! bitstream packet (and the Ogg framing) is a separate stage.

// Assembles the analysis stages; the live encoder still ships via FFI.
#![allow(dead_code)]

use crate::analysis::PsyAnalysis;
use crate::floor1::{quantize_posts_to_mult, Floor1Fitter};
use crate::floor_render::{render_floor1, spectral_residue};

/// The floor1 post multiplier paired with a `quant_q` of 64 (`mult * quant_q`
/// spans the 256-entry dB lookup).
pub const FLOOR_MULT: i32 = 4;

/// One block's analysis result: the transform, the masking curve, the fitted
/// floor, and the residue the spectrum reduces to once the floor is divided
/// out.
pub struct BlockAnalysis {
    /// MDCT coefficients (`n`).
    pub mdct: Vec<f32>,
    /// Log-magnitude spectrum (`n`).
    pub logmdct: Vec<f32>,
    /// The masking curve (`n`).
    pub logmask: Vec<f32>,
    /// Fitted floor posts, mult-quantized (one per postlist entry).
    pub posts: Vec<i32>,
    /// The synthesized linear floor curve (`n`).
    pub floor: Vec<f32>,
    /// The residue `mdct / floor` the entropy stage codes (`n`).
    pub residue: Vec<f32>,
}

/// Runs the analysis pipeline for one `2n`-sample PCM block of a single channel.
///
/// `psy` and `fitter` must be built for the same `n`, and `postlist` must be the
/// fitter's post positions. Returns `None` if the block is the wrong length or
/// the floor quantizes away (a silent block leaves no floor to code).
#[must_use]
pub fn analyze_block(
    psy: &PsyAnalysis,
    fitter: &Floor1Fitter,
    postlist: &[i32],
    pcm: &[f32],
) -> Option<BlockAnalysis> {
    let (mdct, logmdct) = psy.mdct_analysis(pcm)?;
    let logmask = psy.logmask(&logmdct);
    if logmask.is_empty() {
        return None;
    }

    let fit = fitter.fit(&logmdct, &logmask)?;
    let posts = quantize_posts_to_mult(&fit, FLOOR_MULT);
    let floor = render_floor1(postlist, &posts, FLOOR_MULT, psy.n());
    let residue = spectral_residue(&mdct, &floor);

    Some(BlockAnalysis {
        mdct,
        logmdct,
        logmask,
        posts,
        floor,
        residue,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::floor1::Floor1FitInfo;

    /// The standard "128 × 4" floor1 postlist (libvorbis `floor_all.h`).
    const POSTLIST: [i32; 6] = [0, 128, 33, 8, 16, 70];

    fn fitter() -> Floor1Fitter {
        Floor1Fitter::new(POSTLIST.to_vec(), Floor1FitInfo::standard())
    }

    /// A 256-sample (`n = 128`) PCM block holding a single low-frequency tone.
    fn tone_block(bin: usize, amp: f32) -> Vec<f32> {
        let n = 128;
        let rate = 48_000.0f32;
        let freq = bin as f32 * rate / (2.0 * n as f32);
        (0..2 * n)
            .map(|i| amp * (2.0 * std::f32::consts::PI * freq * i as f32 / rate).sin())
            .collect()
    }

    #[test]
    fn rejects_a_wrong_length_block() {
        let psy = PsyAnalysis::new(128, 48_000);
        assert!(analyze_block(&psy, &fitter(), &POSTLIST, &[0.0; 100]).is_none());
    }

    #[test]
    fn a_silent_block_has_no_floor() {
        let psy = PsyAnalysis::new(128, 48_000);
        // True silence quantizes the whole floor away.
        assert!(analyze_block(&psy, &fitter(), &POSTLIST, &vec![0.0; 256]).is_none());
    }

    #[test]
    fn residue_times_floor_reconstructs_the_spectrum() {
        let psy = PsyAnalysis::new(128, 48_000);
        let block = analyze_block(&psy, &fitter(), &POSTLIST, &tone_block(20, 0.6)).expect("block");
        assert_eq!(block.floor.len(), 128);
        // The floor is a strictly positive magnitude curve.
        assert!(block.floor.iter().all(|&f| f > 0.0), "floor not positive");
        // residue * floor recovers the MDCT exactly (the division is exact).
        for i in 0..128 {
            let recon = block.residue[i] * block.floor[i];
            assert!(
                (recon - block.mdct[i]).abs() < 1e-4,
                "bin {i}: {recon} vs {}",
                block.mdct[i]
            );
        }
    }

    #[test]
    fn the_floor_tracks_the_spectral_envelope() {
        // A strong low-frequency tone: the floor must sit higher in the loud
        // low band than in the quiet high band.
        let psy = PsyAnalysis::new(128, 48_000);
        let block = analyze_block(&psy, &fitter(), &POSTLIST, &tone_block(16, 0.8)).expect("block");
        let low = block.floor[16];
        let high = block.floor[120];
        assert!(
            low > high,
            "floor did not track the envelope: {low} <= {high}"
        );
    }

    #[test]
    fn dividing_out_the_floor_whitens_the_spectrum() {
        // The residue's dynamic range (in dB) is markedly smaller than the raw
        // MDCT's: that flattening is the point of the floor.
        let psy = PsyAnalysis::new(128, 48_000);
        let block = analyze_block(&psy, &fitter(), &POSTLIST, &tone_block(24, 0.7)).expect("block");

        let db_range = |xs: &[f32]| {
            let mags: Vec<f32> = xs.iter().map(|x| x.abs()).filter(|&m| m > 1e-9).collect();
            let max = mags.iter().copied().fold(0.0f32, f32::max);
            let min = mags.iter().copied().fold(f32::INFINITY, f32::min);
            20.0 * (max / min).log10()
        };

        let mdct_range = db_range(&block.mdct);
        let residue_range = db_range(&block.residue);
        assert!(
            residue_range < mdct_range,
            "floor did not whiten: residue {residue_range} dB vs mdct {mdct_range} dB",
        );
    }
}
