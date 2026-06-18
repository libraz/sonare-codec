//! CELT variable-bitrate (VBR) rate control.
//!
//! Hand-ported to safe Rust from the float build of libopus `celt/celt_encoder.c`
//! (`compute_vbr` and the per-frame byte-budget block of `celt_encode_with_ec`).
//! Derivative work of libopus (BSD-3-Clause); see `LICENSE-THIRDPARTY`.
//!
//! The target is carried in eighth-bits per frame (`<< BITRES`). [`vbr_rate`]
//! turns a nominal bitrate into the per-frame target, [`base_target`] removes the
//! fixed per-frame overhead, [`compute_vbr`] shapes the target from the encoder's
//! analysis (dynalloc boost, transient/tonality boosts, the masking floor, the
//! temporal-VBR nudge and the hard "no more than double" clamp), and
//! [`vbr_choose_bytes`] converts the shaped target into a packet byte count.
//!
//! Scope: the non-hybrid, non-surround CELT path. The constrained-VBR reservoir
//! drift (cross-frame state) lives in [`VbrState`]; the SILK/hybrid target tweaks
//! are not modelled here. The optional psychoacoustic analysis is passed in via
//! [`VbrAnalysis`].

// Consumed by the CELT encode entry point; the live encoder still ships via the
// Opus FFI path.
#![allow(dead_code)]

use crate::theta::BITRES;

/// `vbr_rate`: the per-frame target in eighth-bits for a nominal `bitrate_bps`
/// at `sample_rate` Hz and `frame_size` samples (libopus
/// `vbr_rate = (bitrate*frame_size + den/2) / den`, `den = Fs >> BITRES`).
#[must_use]
pub fn vbr_rate(bitrate_bps: i32, sample_rate: i32, frame_size: i32) -> i32 {
    let den = (sample_rate >> BITRES) as i64;
    let num = bitrate_bps as i64 * frame_size as i64 + den / 2;
    (num / den) as i32
}

/// `base_target`: the per-frame target with the fixed overhead removed (the
/// non-hybrid CELT case, `vbr_rate - ((40*C + 20) << BITRES)`).
#[must_use]
pub fn base_target(vbr_rate: i32, channels: i32) -> i32 {
    vbr_rate - ((40 * channels + 20) << BITRES)
}

/// The optional psychoacoustic analysis inputs to [`compute_vbr`]. When the
/// analysis is invalid the encoder passes `None` and the tonality/activity
/// adjustments are skipped, exactly as libopus gates them on `analysis->valid`.
pub struct VbrAnalysis {
    /// Tonality estimate in `[0, 1]`.
    pub tonality: f32,
    /// Activity estimate in `[0, 1]`.
    pub activity: f32,
}

/// Inputs to [`compute_vbr`] (the float, non-hybrid, non-surround path).
pub struct VbrInput<'a> {
    /// The mode's band boundaries (`nb_e_bands + 1` entries).
    pub e_bands: &'a [i16],
    /// The number of energy bands.
    pub nb_e_bands: usize,
    /// The per-frame target with overhead removed (see [`base_target`]).
    pub base_target: i32,
    /// The frame-size shift `LM`.
    pub lm: i32,
    /// The coded channel count (`1` or `2`).
    pub channels: i32,
    /// The current stereo intensity band.
    pub intensity: i32,
    /// The previous frame's coded band count (`0` until the first frame).
    pub last_coded_bands: usize,
    /// The stereo-saving estimate (eighth-bit scale, `<= 1`).
    pub stereo_saving: f32,
    /// The total dynalloc boost (eighth-bits).
    pub tot_boost: i32,
    /// The transient estimate from `transient_analysis`.
    pub tf_estimate: f32,
    /// Whether the pitch changed materially this frame.
    pub pitch_change: bool,
    /// The masking depth from `dynalloc_analysis`.
    pub max_depth: f32,
    /// Whether this is the LFE channel.
    pub lfe: bool,
    /// Whether constrained VBR is active (damps the shaping toward base).
    pub constrained_vbr: bool,
    /// The temporal-VBR strength.
    pub temporal_vbr: f32,
    /// The CBR-equivalent rate used by the temporal-VBR nudge.
    pub equiv_rate: i32,
    /// The psychoacoustic analysis, or `None` when invalid.
    pub analysis: Option<VbrAnalysis>,
}

/// `compute_vbr`: shape the per-frame target (eighth-bits) from the encoder's
/// analysis. A faithful port of the float, non-hybrid, non-surround path.
#[must_use]
pub fn compute_vbr(inp: &VbrInput) -> i32 {
    let nb = inp.nb_e_bands;
    let eb = inp.e_bands;
    let lm = inp.lm;
    let c = inp.channels;

    let coded_bands = if inp.last_coded_bands != 0 {
        inp.last_coded_bands
    } else {
        nb
    };
    let mut coded_bins = (i32::from(eb[coded_bands])) << lm;
    if c == 2 {
        let b = (inp.intensity.min(coded_bands as i32)).max(0) as usize;
        coded_bins += (i32::from(eb[b])) << lm;
    }

    let mut target = inp.base_target;

    // Lower the rate on low-activity frames.
    if let Some(a) = &inp.analysis {
        if a.activity < 0.4 {
            target -= ((coded_bins << BITRES) as f32 * (0.4 - a.activity)) as i32;
        }
    }

    // Stereo savings: spend fewer bits when the signal is near-mono.
    if c == 2 {
        let coded_stereo_bands = (inp.intensity.min(coded_bands as i32)).max(0) as usize;
        let coded_stereo_dof =
            ((i32::from(eb[coded_stereo_bands])) << lm) - coded_stereo_bands as i32;
        let max_frac = 0.8 * coded_stereo_dof as f32 / coded_bins as f32;
        let stereo_saving = inp.stereo_saving.min(1.0);
        let a = max_frac * target as f32;
        let b = (stereo_saving - 0.1) * ((coded_stereo_dof << BITRES) as f32);
        target -= a.min(b) as i32;
    }

    // Dynalloc boost, relative to the average boost it calibrates against.
    target += inp.tot_boost - (19 << lm);

    // Transient boost (relative to the average transient level).
    let tf_calibration = 0.044f32;
    target += ((inp.tf_estimate - tf_calibration) * target as f32) as i32;

    // Tonality boost.
    if let Some(a) = &inp.analysis {
        if !inp.lfe {
            let tonal = 0.0f32.max(a.tonality - 0.15) - 0.12;
            let mut tonal_target = target + ((coded_bins << BITRES) as f32 * 1.2 * tonal) as i32;
            if inp.pitch_change {
                tonal_target += ((coded_bins << BITRES) as f32 * 0.8) as i32;
            }
            target = tonal_target;
        }
    }

    // Masking floor: never drop below what the loudest band needs.
    let bins = (i32::from(eb[nb - 2])) << lm;
    let mut floor_depth = (((c * bins) << BITRES) as f32 * inp.max_depth) as i32;
    floor_depth = floor_depth.max(target >> 2);
    target = target.min(floor_depth);

    // Constrained VBR damps the shaping back toward the base target.
    if inp.constrained_vbr {
        target = inp.base_target + (0.67 * (target - inp.base_target) as f32) as i32;
    }

    // Temporal VBR: a small boost on near-steady frames at lower rates.
    if inp.tf_estimate < 0.2 {
        let amount = 0.000_003_1f32 * 0.max(32000.min(96000 - inp.equiv_rate)) as f32;
        let tvbr_factor = inp.temporal_vbr * amount;
        target += (tvbr_factor * target as f32) as i32;
    }

    // Never more than double the base target.
    target.min(2 * inp.base_target)
}

/// `vbr_choose_bytes`: convert the shaped `target` (eighth-bits) into a packet
/// byte count, given the bits used so far (`tell_frac`, eighth-bits), the total
/// dynalloc boost and the hard packet ceiling. Returns the chosen byte budget.
///
/// This is the unconstrained core of the byte-budget block (it omits the
/// constrained-VBR reservoir drift, which needs cross-frame state).
#[must_use]
pub fn vbr_choose_bytes(
    target: i32,
    tell_frac: i32,
    total_boost: i32,
    nb_compressed_bytes: i32,
    lm: i32,
    silence: bool,
) -> i32 {
    if silence {
        return 2;
    }
    // 510 kb/s ceiling: the allocator can't use more anyway.
    let nb_compressed_bytes = nb_compressed_bytes.min(1275 >> (3 - lm));
    // Add the space already spent, then round to bytes.
    let target = target + tell_frac;
    let min_allowed = ((tell_frac + total_boost + (1 << (BITRES + 3)) - 1) >> (BITRES + 3)) + 2;
    let nb = (target + (1 << (BITRES + 2))) >> (BITRES + 3);
    nb.max(min_allowed).min(nb_compressed_bytes)
}

/// Inputs to [`VbrState::choose_bytes`] (the full, constrained-aware byte-budget
/// block of `celt_encode_with_ec`).
pub struct VbrChoose {
    /// The shaped per-frame target (eighth-bits, from [`compute_vbr`]).
    pub target: i32,
    /// The bits already spent this frame (`ec_tell_frac`, eighth-bits).
    pub tell_frac: i32,
    /// The total dynalloc boost (eighth-bits).
    pub total_boost: i32,
    /// The raw per-frame rate (see [`vbr_rate`]); the drift measures against it.
    pub vbr_rate: i32,
    /// The packet ceiling in bytes before the `1275 >> (3 - lm)` cap.
    pub nb_compressed_bytes: i32,
    /// The frame-size shift `LM`.
    pub lm: i32,
    /// `mode.max_lm - lm`; scales the offset fed back into `base_target`.
    pub lm_diff: i32,
    /// Whether constrained VBR is active (enables the reservoir correction).
    pub constrained_vbr: bool,
    /// Whether this frame is silent (the drift is frozen, the bitres refills).
    pub silence: bool,
}

/// Cross-frame constrained-VBR reservoir state (libopus `st->vbr_reservoir /
/// vbr_drift / vbr_offset / vbr_count`). The reservoir tracks the cumulative
/// over/under-shoot against the nominal rate; the drift is a slow leaky-integrator
/// correction whose negated value ([`Self::base_target_offset`]) nudges the next
/// frame's `base_target` so constrained VBR holds its average rate over time.
#[derive(Debug, Clone, Default)]
pub struct VbrState {
    reservoir: i32,
    drift: i32,
    offset: i32,
    count: i32,
}

impl VbrState {
    /// A fresh reservoir (all counters zero), matching the encoder reset state.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The offset to fold into `base_target` before [`compute_vbr`] this frame
    /// (`base_target += vbr_offset >> lm_diff`). Zero until constrained VBR has
    /// accumulated drift.
    #[must_use]
    pub fn base_target_offset(&self, lm_diff: i32) -> i32 {
        self.offset >> lm_diff
    }

    /// Run the full byte-budget step: convert the shaped `target` to a packet
    /// byte count, then update the reservoir/drift so future frames correct any
    /// accumulated drift. For unconstrained VBR this matches [`vbr_choose_bytes`]
    /// exactly (the reservoir/drift updates are gated on `constrained_vbr`); the
    /// `vbr_count` counter still advances so the drift smoothing warms up.
    pub fn choose_bytes(&mut self, c: &VbrChoose) -> i32 {
        let nb_compressed = c.nb_compressed_bytes.min(1275 >> (3 - c.lm));

        // The bits spent so far are added; round the shaped target to bytes.
        let target_frac = c.target + c.tell_frac;
        let min_allowed =
            ((c.tell_frac + c.total_boost + (1 << (BITRES + 3)) - 1) >> (BITRES + 3)) + 2;
        let mut nb = (target_frac + (1 << (BITRES + 2))) >> (BITRES + 3);
        nb = nb.max(min_allowed).min(nb_compressed);

        // How much we missed the nominal rate by (uses the shaped target+tell),
        // then snap `target` to the bytes we actually chose.
        let mut delta = target_frac - c.vbr_rate;
        let mut target = nb << (BITRES + 3);

        if c.silence {
            nb = 2;
            target = (2 * 8) << BITRES;
            delta = 0;
        }

        // Leaky-integrator gain: 1/(count+20) while warming up, then a floor.
        let alpha = if self.count < 970 {
            self.count += 1;
            1.0f32 / (self.count + 20) as f32
        } else {
            0.001f32
        };

        if c.constrained_vbr {
            // Bits used in excess of what we're allowed accumulate here.
            self.reservoir += target - c.vbr_rate;
            // Smooth the per-frame miss into a slow drift; the next frame's
            // base_target is nudged by its negation.
            let err = (delta * (1 << c.lm_diff)) - self.offset - self.drift;
            self.drift += (alpha * err as f32) as i32;
            self.offset = -self.drift;
            if self.reservoir < 0 {
                // Under the floor: spend a little more and reset the reservoir.
                let adjust = (-self.reservoir) / (8 << BITRES);
                if !c.silence {
                    nb += adjust;
                }
                self.reservoir = 0;
            }
        }

        nb_compressed.min(nb)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // A small, representative band table (the 48 kHz CELT boundaries).
    const E_BANDS: [i16; 22] = [
        0, 1, 2, 3, 4, 5, 6, 7, 8, 10, 12, 14, 16, 20, 24, 28, 34, 40, 48, 60, 78, 100,
    ];

    fn input(base: i32) -> VbrInput<'static> {
        VbrInput {
            e_bands: &E_BANDS,
            nb_e_bands: 21,
            base_target: base,
            lm: 3,
            channels: 1,
            intensity: 0,
            last_coded_bands: 0,
            stereo_saving: 0.0,
            tot_boost: 0,
            tf_estimate: 0.044, // the calibration point -> no transient boost
            pitch_change: false,
            max_depth: 1000.0, // large floor so it does not clamp
            lfe: false,
            constrained_vbr: false,
            temporal_vbr: 0.0,
            equiv_rate: 64000,
            analysis: None,
        }
    }

    #[test]
    fn vbr_rate_matches_the_closed_form() {
        // 20 ms at 64 kb/s = 1280 bits = 160 bytes = 10240 eighth-bits.
        assert_eq!(vbr_rate(64_000, 48_000, 960), 10_240);
        // The byte budget that implies.
        assert_eq!(vbr_rate(64_000, 48_000, 960) >> (BITRES + 3), 160);
        // Higher bitrate -> proportionally more.
        assert_eq!(vbr_rate(128_000, 48_000, 960), 20_480);
    }

    #[test]
    fn base_target_removes_the_fixed_overhead() {
        let vr = vbr_rate(64_000, 48_000, 960);
        assert_eq!(base_target(vr, 1), vr - (60 << BITRES));
        assert_eq!(base_target(vr, 2), vr - (100 << BITRES));
    }

    #[test]
    fn compute_vbr_is_monotonic_in_base_target() {
        let lo = compute_vbr(&input(8_000));
        let hi = compute_vbr(&input(9_000));
        assert!(hi > lo, "target should rise with base_target: {lo} -> {hi}");
    }

    #[test]
    fn dynalloc_boost_raises_the_target() {
        let plain = compute_vbr(&input(8_000));
        let mut boosted = input(8_000);
        boosted.tot_boost = 400;
        let boosted = compute_vbr(&boosted);
        assert!(
            boosted > plain,
            "tot_boost should raise target: {plain} -> {boosted}"
        );
    }

    #[test]
    fn masking_floor_caps_the_target() {
        // A tiny max_depth makes floor_depth collapse to target>>2, capping target.
        let mut inp = input(8_000);
        inp.max_depth = 0.0;
        let capped = compute_vbr(&inp);
        let uncapped = compute_vbr(&input(8_000));
        assert!(
            capped < uncapped,
            "small max_depth must cap: {uncapped} -> {capped}"
        );
        // With max_depth 0, floor_depth = max(0, target>>2) so target == base>>2-ish.
        assert!(capped <= (8_000 >> 2) + 1);
    }

    #[test]
    fn target_never_exceeds_double_base() {
        let mut inp = input(8_000);
        inp.tot_boost = 100_000; // absurd boost
        inp.tf_estimate = 0.9; // strong transient boost too
        let target = compute_vbr(&inp);
        assert!(target <= 2 * 8_000, "target {target} exceeded 2*base");
    }

    #[test]
    fn transient_boost_above_calibration_adds_bits() {
        let calm = compute_vbr(&input(8_000)); // tf == calibration
        let mut spiky = input(8_000);
        spiky.tf_estimate = 0.3;
        let spiky = compute_vbr(&spiky);
        assert!(spiky > calm, "transient should add bits: {calm} -> {spiky}");
    }

    #[test]
    fn choose_bytes_stays_within_bounds() {
        let total_boost = 0;
        let nb_compressed = 160;
        // A modest target rounds to roughly target/64 bytes.
        let nb = vbr_choose_bytes(8_000, 200, total_boost, nb_compressed, 3, false);
        assert!(nb >= 2 && nb <= nb_compressed, "bytes {nb} out of bounds");
        // A huge target clamps to the packet ceiling.
        let nb_big = vbr_choose_bytes(1_000_000, 200, total_boost, nb_compressed, 3, false);
        assert_eq!(nb_big, nb_compressed, "must clamp to the packet ceiling");
        // Silence forces the minimal 2-byte frame.
        assert_eq!(vbr_choose_bytes(8_000, 200, 0, nb_compressed, 3, true), 2);
    }

    #[test]
    fn choose_bytes_is_monotonic_until_the_ceiling() {
        let a = vbr_choose_bytes(4_000, 100, 0, 200, 3, false);
        let b = vbr_choose_bytes(6_000, 100, 0, 200, 3, false);
        assert!(
            b >= a,
            "more target should not give fewer bytes: {a} -> {b}"
        );
    }

    fn choose(
        st: &mut VbrState,
        target: i32,
        vbr_rate: i32,
        constrained: bool,
        silence: bool,
    ) -> i32 {
        st.choose_bytes(&VbrChoose {
            target,
            tell_frac: 100,
            total_boost: 0,
            vbr_rate,
            nb_compressed_bytes: 1275,
            lm: 3,
            lm_diff: 0,
            constrained_vbr: constrained,
            silence,
        })
    }

    #[test]
    fn unconstrained_state_matches_the_free_function() {
        // With constrained_vbr off, the stateful path must equal the core,
        // for any target — the reservoir/drift updates are gated out.
        let mut st = VbrState::new();
        for &target in &[1_000, 4_000, 8_000, 50_000, 1_000_000] {
            let stateful = choose(&mut st, target, 8_000, false, false);
            let core = vbr_choose_bytes(target, 100, 0, 1275, 3, false);
            assert_eq!(stateful, core, "target {target}");
        }
        // No constrained updates means no drift accumulates.
        assert_eq!(st.base_target_offset(0), 0);
    }

    #[test]
    fn silence_forces_two_bytes_and_freezes_drift() {
        let mut st = VbrState::new();
        let nb = choose(&mut st, 80_000, 8_000, true, true);
        assert_eq!(nb, 2, "silence must code a 2-byte frame");
        // delta is zeroed on silence, so no offset is injected this frame.
        assert_eq!(st.base_target_offset(0), 0);
    }

    #[test]
    fn constrained_undershoot_refills_from_the_reservoir() {
        // A target far below the nominal rate drives the reservoir negative, and
        // the next-frame correction must hand back extra bytes versus the same
        // run without the constrained reservoir logic.
        let mut con = VbrState::new();
        let mut unc = VbrState::new();
        // Prime both with a few lean frames so the reservoir goes negative.
        let mut con_last = 0;
        let mut unc_last = 0;
        for _ in 0..8 {
            con_last = choose(&mut con, 2_000, 40_000, true, false);
            unc_last = choose(&mut unc, 2_000, 40_000, false, false);
        }
        assert!(
            con_last >= unc_last,
            "constrained refill must not under-allocate vs unconstrained: \
             {con_last} vs {unc_last}"
        );
        // The drift integrator has moved off zero and feeds back a base-target
        // offset for the following frame.
        assert_ne!(
            con.base_target_offset(0),
            0,
            "constrained drift should accumulate an offset"
        );
    }

    #[test]
    fn drift_offset_scales_down_with_lm_diff() {
        let mut st = VbrState::new();
        for _ in 0..8 {
            choose(&mut st, 2_000, 40_000, true, false);
        }
        let full = st.base_target_offset(0);
        let halved = st.base_target_offset(1);
        // offset >> 1 vs offset >> 0 (the offset here is negative, so compare via
        // magnitude through the arithmetic shift).
        assert_eq!(
            halved,
            full >> 1,
            "lm_diff must arithmetic-shift the offset"
        );
    }
}
