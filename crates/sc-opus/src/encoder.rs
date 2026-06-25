//! High-level CELT encoder driver: PCM frames in, CELT packets out.
//!
//! Assembles the hand-ported bricks into the stage order of libopus
//! `celt/celt_encoder.c` (`celt_encode_with_ec`) for the non-hybrid CELT path:
//! pre-emphasis + pitch pre-filter -> transient analysis -> forward MDCT ->
//! dynamic allocation -> VBR/CBR rate control -> range coding. Derivative work
//! of libopus (BSD-3-Clause); see `LICENSE-THIRDPARTY`.
//!
//! Scope: 48 kHz, mono or stereo, full-band CELT. The byte budget is either the
//! CBR per-frame average or the shaped VBR target ([`crate::vbr`]); the
//! constrained-VBR reservoir drift and the psychoacoustic tonality analysis are
//! not modelled (the analysis is passed as `None`).

// Consumed by the public Opus encode entry point; the live encoder still ships
// via the Opus FFI path.
#![allow(dead_code)]

use crate::analysis::{dynalloc_analysis, transient_analysis, DynallocResult, TransientResult};
use crate::celt_frame::{encode_celt_frame, CeltEncoderState, FrameParams, SPREAD_NORMAL};
use crate::celt_frontend::{frontend_preprocess, frontend_transform, FrontendState};
use crate::mode::{celt_mode_48k, CeltMode};
use crate::vbr::{base_target, compute_vbr, vbr_rate, VbrChoose, VbrInput, VbrState};

const SAMPLE_RATE: i32 = 48_000;
/// The least-significant-bit depth the dynalloc noise floor assumes.
const LSB_DEPTH: i32 = 24;
/// The hard CELT packet ceiling in bytes (510 kb/s at 2.5 ms).
const MAX_PACKET_BYTES: i32 = 1275;

/// A self-contained CELT encoder: PCM frames in, CELT packets out, with VBR or
/// CBR byte budgeting. Carries the front-end and range-coder state across frames.
pub struct CeltEncoder {
    mode: CeltMode,
    frontend: FrontendState,
    enc_state: CeltEncoderState,
    channels: usize,
    lm: i32,
    end: usize,
    complexity: i32,
    bitrate_bps: i32,
    vbr: bool,
    constrained_vbr: bool,
    /// Cross-frame constrained-VBR reservoir/drift state.
    vbr_state: VbrState,
    /// Previous frame's band log-energies — the `band_log_e2` dynalloc needs.
    band_log_e2: Vec<f32>,
    intensity: i32,
    last_coded_bands: usize,
}

impl CeltEncoder {
    /// Create an encoder for `channels` (1 or 2) at frame-size shift `lm`
    /// (`frame_size = short_mdct_size << lm`), targeting `bitrate_bps`. `vbr`
    /// selects variable-rate budgeting; otherwise the budget is the CBR average.
    #[must_use]
    pub fn new(channels: usize, lm: i32, bitrate_bps: i32, vbr: bool) -> Self {
        let mode = celt_mode_48k();
        let nb = mode.nb_e_bands;
        let frontend = FrontendState::new(channels, mode.overlap);
        let enc_state = CeltEncoderState::new(channels, nb);
        Self {
            mode,
            frontend,
            enc_state,
            channels,
            lm,
            end: nb,
            complexity: 5,
            bitrate_bps,
            vbr,
            constrained_vbr: false,
            vbr_state: VbrState::new(),
            band_log_e2: vec![-28.0; channels * nb],
            intensity: nb as i32,
            last_coded_bands: 0,
        }
    }

    /// Enable constrained VBR: the per-frame budget is shaped toward `base_target`
    /// and a cross-frame reservoir ([`VbrState`]) corrects drift so the long-run
    /// average rate tracks `bitrate_bps`. No effect unless `vbr` was set.
    #[must_use]
    pub fn with_constrained_vbr(mut self, constrained: bool) -> Self {
        self.constrained_vbr = constrained;
        self
    }

    /// The frame size in samples this encoder consumes per [`Self::encode`].
    #[must_use]
    pub fn frame_size(&self) -> usize {
        self.mode.short_mdct_size << self.lm
    }

    /// The reconstructed band log-energies after the most recent frame (the
    /// encoder's inter-prediction base); exposed for round-trip verification.
    #[must_use]
    pub fn reconstructed_energies(&self) -> &[f32] {
        &self.enc_state.old_band_e
    }

    /// Encode one interleaved PCM frame (`frame_size * channels` samples) into a
    /// CELT packet of exactly the chosen byte budget. Returns an error if the
    /// range coder overflows that budget.
    pub fn encode(&mut self, pcm: &[f32]) -> Result<Vec<u8>, sc_core::Error> {
        let nb = self.mode.nb_e_bands;
        let n = self.frame_size();
        let overlap = self.mode.overlap;
        let c = self.channels;
        let cc = self.channels;

        // The approximate per-frame budget (used for the analysis thresholds
        // before the exact VBR budget is known).
        let approx_bytes =
            ((i64::from(self.bitrate_bps) * n as i64) / (i64::from(SAMPLE_RATE) * 8)).max(2) as i32;

        // Phase 1: pre-emphasis + pitch pre-filter (time domain).
        let pf_enabled = self.complexity >= 5 && approx_bytes > 12 * c as i32;
        let (input, pf) = frontend_preprocess(
            &self.mode,
            pcm,
            self.lm,
            cc,
            1,
            false,
            0,
            pf_enabled,
            approx_bytes,
            &mut self.frontend,
        );

        // Transient decision on the pre-processed buffer (before the MDCT).
        let tr = transient_analysis(&input, n + overlap, cc, false);

        // Phase 2: forward MDCT + band energies.
        let fe = frontend_transform(
            &self.mode,
            &input,
            self.lm,
            c,
            cc,
            self.end,
            tr.is_transient,
            1,
        );

        // Dynamic allocation -> per-band boosts, total boost, masking depth.
        let mut offsets = vec![0i32; nb];
        let mut importance = vec![0i32; nb];
        let mut spread_weight = vec![0i32; nb];
        let surround = vec![0.0f32; nb];
        let dyn_res = dynalloc_analysis(
            &fe.band_log_e,
            &self.band_log_e2,
            nb,
            0,
            self.end,
            c,
            &mut offsets,
            LSB_DEPTH,
            self.mode.log_n,
            tr.is_transient,
            self.vbr,
            false,
            self.mode.e_bands,
            self.lm,
            approx_bytes,
            false,
            &surround,
            None,
            &mut importance,
            &mut spread_weight,
        );

        // Rate control -> packet byte budget.
        let nb_bytes = self.choose_bytes(n, c, &tr, &dyn_res);

        // Range-code the frame.
        let params = FrameParams {
            start: 0,
            end: self.end,
            lm: self.lm,
            c,
            is_transient: tr.is_transient,
            spread: SPREAD_NORMAL,
            tf_select: 0,
            alloc_trim: 5,
            intensity: self.intensity,
            dual_stereo: 0,
            complexity: self.complexity,
            disable_inv: false,
        };
        let mut band_log_e = fe.band_log_e.clone();
        let mut x = fe.x.clone();
        let mut tf_res = vec![0i32; nb];
        let pf_opt = if pf.pf_on { Some(&pf) } else { None };
        let bytes = encode_celt_frame(
            &self.mode,
            &params,
            &mut band_log_e,
            &fe.band_e,
            &mut x,
            &mut tf_res,
            &mut offsets,
            nb_bytes,
            pf_opt,
            &mut self.enc_state,
        )?;

        // Carry this frame's energies for the next frame's dynalloc.
        self.band_log_e2.copy_from_slice(&fe.band_log_e);
        Ok(bytes)
    }

    /// Pick the packet byte budget: the CBR per-frame average, or the shaped VBR
    /// target converted to bytes (with the constrained-VBR reservoir drift folded
    /// in across frames).
    fn choose_bytes(
        &mut self,
        n: usize,
        c: usize,
        tr: &TransientResult,
        dyn_res: &DynallocResult,
    ) -> usize {
        if !self.vbr {
            let cbr = (i64::from(self.bitrate_bps) * n as i64) / (i64::from(SAMPLE_RATE) * 8);
            return cbr.clamp(2, i64::from(MAX_PACKET_BYTES)) as usize;
        }
        let lm_diff = self.mode.max_lm - self.lm;
        let vr = vbr_rate(self.bitrate_bps, SAMPLE_RATE, n as i32);
        // Constrained VBR nudges base_target by the drift accumulated so far.
        let mut base = base_target(vr, c as i32);
        if self.constrained_vbr {
            base += self.vbr_state.base_target_offset(lm_diff);
        }
        let target = compute_vbr(&VbrInput {
            e_bands: self.mode.e_bands,
            nb_e_bands: self.mode.nb_e_bands,
            base_target: base,
            lm: self.lm,
            channels: c as i32,
            intensity: self.intensity,
            last_coded_bands: self.last_coded_bands,
            stereo_saving: 0.0,
            tot_boost: dyn_res.tot_boost,
            tf_estimate: tr.tf_estimate,
            pitch_change: false,
            max_depth: dyn_res.max_depth,
            lfe: false,
            constrained_vbr: self.constrained_vbr,
            temporal_vbr: 0.0,
            equiv_rate: self.bitrate_bps,
            analysis: None,
        });
        // The budget is chosen before range-coding, so no bits are spent yet
        // (tell_frac = 0). The reservoir/drift updates run inside choose_bytes.
        self.vbr_state
            .choose_bytes(&VbrChoose {
                target,
                tell_frac: 0,
                total_boost: dyn_res.tot_boost,
                vbr_rate: vr,
                nb_compressed_bytes: MAX_PACKET_BYTES,
                lm: self.lm,
                lm_diff,
                constrained_vbr: self.constrained_vbr,
                silence: false,
            })
            .max(2) as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::celt_frame::{decode_celt_frame, CeltDecoderState};

    /// A deterministic harmonic + noise PCM frame, interleaved for `channels`.
    fn make_pcm(n: usize, channels: usize, salt: u32) -> Vec<f32> {
        let mut s = salt.wrapping_add(1);
        let mut rng = || {
            s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (s >> 9) as f32 / (1u32 << 23) as f32 - 0.5
        };
        let mut pcm = vec![0.0f32; n * channels];
        for i in 0..n {
            let phase = i as f32 / 96.0 * std::f32::consts::TAU;
            let tone: f32 = (1..=6).map(|h| (phase * h as f32).sin() / h as f32).sum();
            for ch in 0..channels {
                pcm[i * channels + ch] = 3000.0 * (tone + 0.05 * rng());
            }
        }
        pcm
    }

    /// A broadband, spectrally flat PCM frame (noise + a mild tone). Its small
    /// dynalloc boost keeps the VBR byte budget close to the nominal rate, so the
    /// rate-tracking property is not swamped by signal-driven boosts.
    fn make_broadband(n: usize, channels: usize, salt: u32) -> Vec<f32> {
        let mut s = salt.wrapping_add(1);
        let mut rng = || {
            s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (s >> 9) as f32 / (1u32 << 23) as f32 - 0.5
        };
        let mut pcm = vec![0.0f32; n * channels];
        for i in 0..n {
            let tone = (i as f32 / 70.0 * std::f32::consts::TAU).sin();
            for ch in 0..channels {
                pcm[i * channels + ch] = 1500.0 * rng() + 400.0 * tone;
            }
        }
        pcm
    }

    #[test]
    fn cbr_packet_size_is_exact() {
        // 20 ms at 64 kb/s => 64000*0.02/8 = 160 bytes per frame.
        let mut enc = CeltEncoder::new(1, 3, 64_000, false);
        let n = enc.frame_size();
        let pcm = make_pcm(n, 1, 1);
        let bytes = enc.encode(&pcm).expect("encode");
        assert_eq!(bytes.len(), 160, "CBR frame should be exactly 160 bytes");
    }

    #[test]
    fn vbr_packet_size_tracks_bitrate() {
        let n = celt_mode_48k().short_mdct_size << 3;
        let mut lo = CeltEncoder::new(1, 3, 32_000, true);
        let mut hi = CeltEncoder::new(1, 3, 96_000, true);
        let pcm = make_broadband(n, 1, 7);
        // Prime to steady state: the first frame's dynalloc is dominated by the
        // cold-start band_log_e2 = -28 dB history, which inflates the boost.
        for _ in 0..3 {
            lo.encode(&pcm).expect("encode");
            hi.encode(&pcm).expect("encode");
        }
        let lo_bytes = lo.encode(&pcm).expect("encode").len();
        let hi_bytes = hi.encode(&pcm).expect("encode").len();
        assert!(
            hi_bytes > lo_bytes,
            "higher bitrate must allocate more bytes: {lo_bytes} vs {hi_bytes}"
        );
        // Both stay within the packet ceiling and above the 2-byte minimum.
        assert!(
            (8..=MAX_PACKET_BYTES as usize).contains(&lo_bytes),
            "32k VBR size {lo_bytes}"
        );
        assert!(
            (8..=MAX_PACKET_BYTES as usize).contains(&hi_bytes),
            "96k VBR size {hi_bytes}"
        );
    }

    #[test]
    fn encoded_frames_decode_and_energies_round_trip() {
        // Drive several frames through the full pipeline; each packet must decode
        // and the decoder's reconstructed energies must match the encoder's.
        let mut enc = CeltEncoder::new(1, 3, 96_000, true);
        let n = enc.frame_size();
        let mut dec_state = CeltDecoderState::new(1, enc.end);
        for f in 0..4 {
            let pcm = make_pcm(n, 1, 100 + f);
            let bytes = enc.encode(&pcm).expect("encode");
            let total_bits = (bytes.len() as i32) * 8;
            let dec = decode_celt_frame(
                &enc.mode,
                &bytes,
                0,
                enc.end,
                enc.lm,
                1,
                enc.complexity,
                false,
                &mut dec_state,
            );
            assert!(dec.x.iter().all(|v| v.is_finite()), "frame {f} not finite");
            assert!(total_bits > 0);
            // The shared energy-quantiser contract: encoder and decoder agree.
            assert_eq!(
                enc.reconstructed_energies(),
                dec.old_band_e.as_slice(),
                "frame {f} energies diverged"
            );
        }
    }

    #[test]
    fn constrained_vbr_packets_decode_and_track_rate() {
        // Constrained VBR drives the reservoir across frames; every packet must
        // still decode, and the long-run average should stay near the nominal
        // rate (it must not run away high after the cold-start inflation).
        let mut enc = CeltEncoder::new(1, 3, 48_000, true).with_constrained_vbr(true);
        let n = enc.frame_size();
        let mut dec_state = CeltDecoderState::new(1, enc.end);
        let mut total = 0usize;
        let frames = 20;
        for f in 0..frames {
            let pcm = make_broadband(n, 1, 200 + f);
            let bytes = enc.encode(&pcm).expect("encode");
            assert!(
                (2..=MAX_PACKET_BYTES as usize).contains(&bytes.len()),
                "frame {f} size {} out of range",
                bytes.len()
            );
            let dec = decode_celt_frame(
                &enc.mode,
                &bytes,
                0,
                enc.end,
                enc.lm,
                1,
                enc.complexity,
                false,
                &mut dec_state,
            );
            assert!(dec.x.iter().all(|v| v.is_finite()), "frame {f} not finite");
            total += bytes.len();
        }
        // 20 ms frames at 48 kb/s => 120 bytes/frame nominal. The reservoir keeps
        // the average bounded rather than letting it shoot to the ceiling.
        let avg = total as f32 / frames as f32;
        assert!(
            (40.0..=300.0).contains(&avg),
            "constrained-VBR average {avg} B/frame drifted out of range"
        );
    }

    #[test]
    fn stereo_pipeline_produces_decodable_packets() {
        let mut enc = CeltEncoder::new(2, 3, 128_000, true);
        let n = enc.frame_size();
        let mut dec_state = CeltDecoderState::new(2, enc.end);
        let pcm = make_pcm(n, 2, 3);
        let bytes = enc.encode(&pcm).expect("encode");
        let dec = decode_celt_frame(
            &enc.mode,
            &bytes,
            0,
            enc.end,
            enc.lm,
            2,
            enc.complexity,
            false,
            &mut dec_state,
        );
        assert!(dec.x.iter().all(|v| v.is_finite()));
        assert_eq!(enc.reconstructed_energies(), dec.old_band_e.as_slice());
    }
}
