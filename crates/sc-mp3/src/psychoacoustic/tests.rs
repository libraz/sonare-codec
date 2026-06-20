use super::*;

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() <= tol
    }

    #[test]
    fn hann_window_is_symmetric_and_zero_at_the_start() {
        let window = hann_window(1024).unwrap();
        assert_eq!(window.len(), 1024);
        assert!(approx(window[0], 0.0, 1.0e-12));
        // A periodic Hann peaks at the midpoint.
        assert!(approx(window[512], 1.0, 1.0e-9));
        // Symmetric about the midpoint (n and N-n match).
        for n in 1..512 {
            assert!(approx(window[n], window[1024 - n], 1.0e-9));
        }
    }

    #[test]
    fn hann_window_rejects_zero_length() {
        assert!(hann_window(0).is_err());
    }

    #[test]
    fn forward_dft_localizes_a_pure_tone() {
        // A cosine at exactly bin 8 of a 64-point DFT must concentrate all energy
        // in bin 8 (and its conjugate, which the half-spectrum drops).
        let n = 64usize;
        let bin = 8usize;
        let signal: Vec<f64> = (0..n)
            .map(|t| (std::f64::consts::TAU * bin as f64 * t as f64 / n as f64).cos())
            .collect();
        let spectrum = power_spectrum(&signal).unwrap();
        assert_eq!(spectrum.len(), n / 2 + 1);
        let peak = spectrum
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .map(|(index, _)| index)
            .unwrap();
        assert_eq!(peak, bin);
        // Off-peak bins carry negligible energy relative to the peak.
        let peak_energy = spectrum[bin];
        for (index, &energy) in spectrum.iter().enumerate() {
            if index != bin {
                assert!(energy < peak_energy * 1.0e-6);
            }
        }
    }

    #[test]
    fn forward_dft_rejects_empty_input() {
        assert!(forward_dft_half(&[]).is_err());
        assert!(power_spectrum(&[]).is_err());
    }

    #[test]
    fn fft_matches_the_naive_dft_on_a_multitone_signal() {
        // The radix-2 FFT (power-of-two path) must agree with the reference DFT
        // bin for bin. Use a deterministic multi-tone-plus-ramp signal at the
        // psychoacoustic FFT length so leakage exercises every bin.
        let n = 1024usize;
        let signal: Vec<f64> = (0..n)
            .map(|t| {
                let x = t as f64;
                0.7 * (std::f64::consts::TAU * 30.0 * x / n as f64).sin()
                    + 0.4 * (std::f64::consts::TAU * 137.5 * x / n as f64).cos()
                    + 0.05 * (x / n as f64)
            })
            .collect();
        let fast = forward_dft_half(&signal).unwrap();
        let reference = forward_dft_half_naive(&signal).unwrap();
        assert_eq!(fast.len(), reference.len());
        for (f, r) in fast.iter().zip(reference.iter()) {
            assert!(approx(f.re, r.re, 1.0e-7), "re mismatch: {f:?} vs {r:?}");
            assert!(approx(f.im, r.im, 1.0e-7), "im mismatch: {f:?} vs {r:?}");
        }
    }

    #[test]
    fn fft_and_naive_paths_agree_for_a_non_power_of_two_length() {
        // Length 96 is not a power of two, so forward_dft_half falls back to the
        // naive DFT; the two entry points must return identical results.
        let n = 96usize;
        let signal: Vec<f64> = (0..n)
            .map(|t| (std::f64::consts::TAU * 7.0 * t as f64 / n as f64).cos())
            .collect();
        let viafront = forward_dft_half(&signal).unwrap();
        let reference = forward_dft_half_naive(&signal).unwrap();
        assert_eq!(viafront.len(), reference.len());
        for (a, b) in viafront.iter().zip(reference.iter()) {
            assert!(approx(a.re, b.re, 1.0e-9));
            assert!(approx(a.im, b.im, 1.0e-9));
        }
    }

    #[test]
    fn power_of_two_predicate_is_correct() {
        for &p in &[1usize, 2, 4, 8, 16, 1024, 4096] {
            assert!(is_power_of_two(p));
        }
        for &q in &[0usize, 3, 6, 96, 1000, 1023] {
            assert!(!is_power_of_two(q));
        }
    }

    #[test]
    fn complex_bin_reports_energy_magnitude_and_phase() {
        let bin = ComplexBin { re: 3.0, im: 4.0 };
        assert!(approx(bin.energy(), 25.0, 1.0e-12));
        assert!(approx(bin.magnitude(), 5.0, 1.0e-12));
        assert!(approx(bin.phase(), 4.0_f64.atan2(3.0), 1.0e-12));
    }

    #[test]
    fn bark_scale_tracks_known_anchors() {
        // The bark scale is ~0 at DC, ~8.5 near 1 kHz, and monotone increasing.
        assert!(approx(bark(0.0), 0.0, 1.0e-9));
        assert!(approx(bark(1000.0), 8.5, 0.6));
        assert!(bark(2000.0) > bark(1000.0));
        assert!(bark(8000.0) > bark(4000.0));
    }

    #[test]
    fn absolute_threshold_dips_in_the_most_sensitive_band() {
        // Hearing is most sensitive around 3–4 kHz, where the ATH is near its
        // minimum, and rises steeply at both extremes.
        let mid = absolute_threshold_db(3500.0);
        assert!(mid < absolute_threshold_db(200.0));
        assert!(mid < absolute_threshold_db(15000.0));
        assert!(mid < 5.0);
    }

    #[test]
    fn spreading_function_peaks_at_the_masker() {
        // The spreading function maxes out just above the masker and falls off on
        // both sides; the low-bark skirt drops faster than the high-bark skirt.
        let at_masker = spreading_db(10.0, 10.0);
        assert!(spreading_db(10.0, 12.0) < at_masker);
        assert!(spreading_db(10.0, 8.0) < at_masker);
        // Asymmetry: two barks below the masker is attenuated more than two above.
        assert!(spreading_db(10.0, 8.0) < spreading_db(10.0, 12.0));
    }

    #[test]
    fn tonality_separates_tones_from_noise() {
        // A perfectly flat spectrum is maximally noise-like (tonality 0).
        assert!(approx(spectral_flatness_tonality(&[1.0; 64]), 0.0, 1.0e-9));
        // A lone, well-isolated spectral spike is maximally tonal (clamps to 1),
        // and a tone with realistic −40 dB sidelobes still reads as strongly tonal
        // and well above a noise-like spectrum.
        let mut isolated = [1.0e-9_f64; 64];
        isolated[8] = 1.0;
        assert!(spectral_flatness_tonality(&isolated) > 0.99);
        let mut leaky = [1.0e-4_f64; 64];
        leaky[8] = 1.0;
        let leaky_tonality = spectral_flatness_tonality(&leaky);
        assert!(leaky_tonality > 0.3);
        assert!(leaky_tonality > spectral_flatness_tonality(&[1.0; 64]));
        // An empty spectrum is treated as noise-like rather than panicking.
        assert!(approx(spectral_flatness_tonality(&[]), 0.0, 1.0e-12));
    }

    #[test]
    fn masking_threshold_peaks_under_a_tone_and_decays_with_bark() {
        // A single tonal masker at bin 20: the masked threshold should peak at the
        // masker and fall off monotonically with bark distance on the high side.
        let bins = 64usize;
        let bark: Vec<f64> = (0..bins).map(|k| k as f64 * 0.25).collect();
        let mut energy = vec![0.0_f64; bins];
        energy[20] = 1.0;
        let threshold = spread_masking_threshold(&energy, &bark, 1.0).unwrap();

        let peak = threshold
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .map(|(index, _)| index)
            .unwrap();
        assert_eq!(peak, 20);
        for j in 21..bins - 1 {
            assert!(threshold[j] >= threshold[j + 1]);
        }
        // The tonal masker demands an 18 dB signal-to-mask ratio, so its own bin's
        // threshold sits ~18 dB below the masker energy (the spreading peak is ~0 dB).
        let smr_db = 10.0 * (energy[20] / threshold[20]).log10();
        assert!(approx(smr_db, 18.0, 1.0));
    }

    #[test]
    fn masking_threshold_rejects_mismatched_lengths() {
        assert!(spread_masking_threshold(&[1.0, 2.0], &[0.0], 0.5).is_err());
    }

    #[test]
    fn windowed_tonality_resolves_tone_and_noise_regions() {
        // Low half: a clean tone embedded in near-silence (tonal). High half: a
        // flat noise floor (noise-like). The per-bin index must separate them.
        let bins = 128usize;
        let mut energy = vec![1.0e-9_f64; bins];
        energy[16] = 1.0; // isolated tone in the low region
        for e in energy.iter_mut().take(bins).skip(bins / 2) {
            *e = 1.0; // flat noise in the high region
        }
        let tonality = windowed_tonality(&energy, 17).unwrap();
        assert_eq!(tonality.len(), bins);
        // The window over the tone reads strongly tonal; the flat region reads
        // fully noise-like.
        assert!(tonality[16] > 0.6, "tone bin tonality {}", tonality[16]);
        assert!(
            tonality[bins - 8] < 1.0e-6,
            "noise bin tonality {}",
            tonality[bins - 8]
        );
        // Empty input yields empty output; zero window width is rejected.
        assert!(windowed_tonality(&[], 17).unwrap().is_empty());
        assert!(windowed_tonality(&energy, 0).is_err());
    }

    #[test]
    fn per_bin_masking_generalizes_the_constant_tonality_case() {
        // A constant per-bin tonality must reproduce the scalar spread function.
        let bins = 48usize;
        let bark: Vec<f64> = (0..bins).map(|k| k as f64 * 0.3).collect();
        let mut energy = vec![0.01_f64; bins];
        energy[10] = 1.0;
        energy[30] = 0.5;
        let constant = 0.4_f64;

        let scalar = spread_masking_threshold(&energy, &bark, constant).unwrap();
        let per_bin =
            spread_masking_threshold_per_bin(&energy, &bark, &vec![constant; bins]).unwrap();
        assert_eq!(scalar.len(), per_bin.len());
        for (a, b) in scalar.iter().zip(per_bin.iter()) {
            assert!(approx(*a, *b, 1.0e-12), "{a} vs {b}");
        }
    }

    #[test]
    fn per_bin_masking_applies_each_maskers_own_smr() {
        // Two equal-energy maskers far apart in bark: one fully tonal (18 dB SMR),
        // one fully noise-like (6 dB SMR). Each bin's own threshold reflects its
        // own ratio, so the tonal masker sits ~12 dB further below its energy.
        let bark = [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0];
        let mut energy = [0.0_f64; 10];
        energy[2] = 1.0; // tonal masker
        energy[7] = 1.0; // noise-like masker
        let mut tonality = [0.0_f64; 10];
        tonality[2] = 1.0;
        tonality[7] = 0.0;
        let threshold = spread_masking_threshold_per_bin(&energy, &bark, &tonality).unwrap();

        let tonal_smr_db = 10.0 * (energy[2] / threshold[2]).log10();
        let noise_smr_db = 10.0 * (energy[7] / threshold[7]).log10();
        // The maskers are far enough apart that cross-spread is negligible, so
        // each bin's SMR is dominated by its own masker.
        assert!(approx(tonal_smr_db, 18.0, 1.5), "tonal SMR {tonal_smr_db}");
        assert!(approx(noise_smr_db, 6.0, 1.5), "noise SMR {noise_smr_db}");
        assert!(tonal_smr_db > noise_smr_db);
    }

    #[test]
    fn per_bin_masking_rejects_mismatched_lengths() {
        assert!(spread_masking_threshold_per_bin(&[1.0, 2.0], &[0.0, 1.0], &[0.5]).is_err());
    }

    #[test]
    fn perceptual_entropy_is_zero_when_signal_stays_under_threshold() {
        // A bin well below its threshold (√(e/thr) rounds to 0) costs 0 bits, and
        // pure silence costs exactly 0.
        let threshold = vec![1.0_f64; 32];
        let masked = vec![0.1_f64; 32]; // √0.1 ≈ 0.32 → round 0 → 0 bits
        assert!(perceptual_entropy(&masked, &threshold).unwrap() < 1.0e-9);
        assert!(approx(
            perceptual_entropy(&vec![0.0; 32], &threshold).unwrap(),
            0.0,
            1.0e-12
        ));
    }

    #[test]
    fn perceptual_entropy_grows_with_signal_to_threshold_ratio() {
        // Raising the signal above the masking threshold raises the bit demand.
        let threshold = vec![1.0_f64; 32];
        let quiet = vec![4.0_f64; 32];
        let loud = vec![400.0_f64; 32];
        let pe_quiet = perceptual_entropy(&quiet, &threshold).unwrap();
        let pe_loud = perceptual_entropy(&loud, &threshold).unwrap();
        assert!(pe_quiet > 0.0);
        assert!(
            pe_loud > pe_quiet,
            "louder signal must demand more bits: {pe_loud} vs {pe_quiet}"
        );
        // A single audible bin demands log2(2·round(√(e/thr)) + 1) bits; e/thr = 4
        // gives round(2) = 2 → log2(5) ≈ 2.32.
        let one = perceptual_entropy(&[4.0], &[1.0]).unwrap();
        assert!(approx(one, 5.0_f64.log2(), 1.0e-9));
    }

    #[test]
    fn perceptual_entropy_rejects_mismatched_lengths() {
        assert!(perceptual_entropy(&[1.0, 2.0], &[1.0]).is_err());
    }

    #[test]
    fn bin_barks_increase_with_frequency() {
        let barks = bin_barks(513, 44_100, 1024).unwrap();
        assert_eq!(barks.len(), 513);
        assert!(approx(barks[0], 0.0, 1.0e-9));
        for pair in barks.windows(2) {
            assert!(pair[1] >= pair[0]);
        }
        assert!(bin_barks(0, 0, 1024).is_err());
    }

    #[test]
    fn allowed_noise_applies_the_mask_ratio_to_mdct_energy() {
        // A uniform FFT threshold/signal ratio means every covered band's allowed
        // noise equals ratio * the band's MDCT signal energy, independent of the
        // FFT vs MDCT normalization — the dimensionless ratio cancels it.
        let fft_len = 1024usize;
        let bins = fft_len / 2 + 1;
        let ratio = 0.1_f64;
        let fft_energy = vec![1.0_f64; bins];
        let fft_threshold = vec![ratio; bins];

        let mut mdct = vec![0.0_f32; 576];
        mdct[2] = 3.0; // band 0 (lines 0..4): energy 9
        mdct[50] = 4.0; // band 9 (lines 44..52): energy 16

        let allowed =
            perceptual_band_allowed_noise(&mdct, &fft_energy, &fft_threshold, 44_100, fft_len)
                .unwrap();

        assert!(approx(allowed[0], ratio * 9.0, 1.0e-9));
        assert!(approx(allowed[9], ratio * 16.0, 1.0e-9));
        // A band with no MDCT energy collapses to the floor, not zero.
        assert!(approx(allowed[2], MIN_ALLOWED_NOISE, 1.0e-15));

        // Doubling the masking threshold doubles the allowed noise.
        let louder_threshold = vec![ratio * 2.0; bins];
        let louder =
            perceptual_band_allowed_noise(&mdct, &fft_energy, &louder_threshold, 44_100, fft_len)
                .unwrap();
        assert!(approx(louder[0], 2.0 * allowed[0], 1.0e-9));
    }

    #[test]
    fn allowed_noise_rejects_mismatched_or_empty_inputs() {
        let mdct = vec![1.0_f32; 576];
        assert!(perceptual_band_allowed_noise(&mdct, &[1.0, 2.0], &[1.0], 44_100, 1024).is_err());
        assert!(perceptual_band_allowed_noise(&[], &[1.0], &[1.0], 44_100, 1024).is_err());
    }

    /// Recomputes the requantization-noise energy in one band for a scale-factor
    /// set, mirroring the allocator's internal measurement.
    fn band_noise(
        spectrum: &[f32],
        scale_factors: &[u8; crate::MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT],
        band: usize,
        step: f32,
    ) -> f64 {
        let quantized = crate::quantize_mpeg1_layer3_long_spectrum_with_scalefactors(
            spectrum,
            step,
            scale_factors,
            false,
            44_100,
        )
        .unwrap();
        let gain = 2.0_f64
            .powf(0.25 * (f64::from(crate::mpeg1_layer3_global_gain_for_step(step)) - 210.0));
        let attenuation = 2.0_f64.powf(-0.5 * f64::from(scale_factors[band]));
        let (start, end) = crate::mpeg1_layer3_long_scalefactor_band_range(band, 44_100).unwrap();
        let mut noise = 0.0_f64;
        for line in start..end {
            let is = quantized[line];
            let sign = if is < 0 { -1.0 } else { 1.0 };
            let reconstructed =
                (is.unsigned_abs() as f64).powf(4.0 / 3.0) * gain * attenuation * sign;
            let error = f64::from(spectrum[line]) - reconstructed;
            noise += error * error;
        }
        noise
    }

    #[test]
    fn allocation_leaves_loose_targets_at_zero() {
        let spectrum: Vec<f32> = (0..576)
            .map(|l| 0.3 * (-(l as f32) / 150.0).exp())
            .collect();
        let allowed = [f64::INFINITY; crate::MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT];
        let scale_factors =
            allocate_long_block_scalefactors(&spectrum, &allowed, 0.05, false, 44_100).unwrap();
        assert_eq!(
            scale_factors,
            [0_u8; crate::MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT]
        );
    }

    #[test]
    fn allocation_drives_noise_below_a_tight_band_target() {
        // Only band 0 carries energy; the rest are silent.
        let mut spectrum = vec![0.0_f32; 576];
        for line in spectrum.iter_mut().take(4) {
            *line = 0.5;
        }
        let zero = [0_u8; crate::MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT];
        let noise_at_zero = band_noise(&spectrum, &zero, 0, 0.05);
        assert!(
            noise_at_zero > 0.0,
            "quantization must introduce some noise"
        );

        // Demand band 0's noise be cut to 30%; leave every other band unconstrained.
        let mut allowed = [f64::INFINITY; crate::MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT];
        allowed[0] = noise_at_zero * 0.3;
        let scale_factors =
            allocate_long_block_scalefactors(&spectrum, &allowed, 0.05, false, 44_100).unwrap();

        assert!(
            scale_factors[0] > 0,
            "the loud band's scale factor must rise"
        );
        for &sf in &scale_factors[1..] {
            assert_eq!(sf, 0, "silent bands must stay at zero");
        }
        let noise_final = band_noise(&spectrum, &scale_factors, 0, 0.05);
        assert!(
            noise_final <= allowed[0],
            "allocation did not meet the target: {noise_final} > {}",
            allowed[0]
        );
        assert!(
            noise_final < noise_at_zero,
            "amplification must reduce band noise"
        );
    }

    #[test]
    fn allocation_rejects_nonpositive_step() {
        let spectrum = vec![0.1_f32; 576];
        let allowed = [1.0_f64; crate::MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT];
        assert!(allocate_long_block_scalefactors(&spectrum, &allowed, 0.0, false, 44_100).is_err());
    }

    #[test]
    fn allowed_noise_is_finite_for_a_fully_silent_granule() {
        // A silent FFT span and silent MDCT band must not produce 0 · ∞ = NaN.
        let mdct = vec![0.0_f32; 576];
        let bins = 1024 / 2 + 1;
        let allowed =
            perceptual_band_allowed_noise(&mdct, &vec![0.0; bins], &vec![0.0; bins], 44_100, 1024)
                .unwrap();
        for &value in &allowed {
            assert!(
                value.is_finite(),
                "silent granule produced a non-finite target"
            );
        }
    }

    #[test]
    fn driver_produces_valid_scalefactors_for_a_tone() {
        // A 1 kHz tone through the full driver yields in-range scale factors.
        let fft_len = 1024usize;
        let pcm_window: Vec<f64> = (0..fft_len)
            .map(|n| 0.5 * (std::f64::consts::TAU * 1000.0 * n as f64 / 44_100.0).sin())
            .collect();
        // A decaying low-frequency MDCT spectrum to allocate against.
        let mdct: Vec<f32> = (0..576)
            .map(|l| 0.3 * (-(l as f32) / 120.0).exp())
            .collect();
        let scale_factors =
            perceptual_long_block_scalefactors(&mdct, &pcm_window, 0.05, false, 44_100).unwrap();
        for (band, &sf) in scale_factors.iter().enumerate() {
            let cap = if band < 11 { 15 } else { 7 };
            assert!(sf <= cap, "band {band} scale factor {sf} exceeds cap {cap}");
        }
    }

    #[test]
    fn driver_leaves_a_silent_granule_at_zero() {
        let scale_factors = perceptual_long_block_scalefactors(
            &[0.0_f32; 576],
            &[0.0_f64; 1024],
            0.05,
            false,
            44_100,
        )
        .unwrap();
        assert_eq!(
            scale_factors,
            [0_u8; crate::MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT]
        );
    }

    #[test]
    fn steady_tone_is_not_a_transient() {
        // A continuous tone has near-uniform segment energy, so the attack ratio
        // stays close to 1 and the block is not flagged.
        let pcm: Vec<f64> = (0..1152)
            .map(|n| 0.5 * (std::f64::consts::TAU * 1000.0 * n as f64 / 44_100.0).sin())
            .collect();
        let ratio = transient_attack_ratio(&pcm, TRANSIENT_SEGMENTS).unwrap();
        assert!(ratio < TRANSIENT_RATIO_THRESHOLD, "steady ratio {ratio}");
        assert!(!is_transient_block(&pcm).unwrap());
    }

    #[test]
    fn sudden_onset_is_a_transient() {
        // Silence for the first half of the block, then a loud burst: the running
        // mean of the preceding (near-silent) segments is tiny, so the onset
        // segment's ratio is large and the block is flagged.
        let mut pcm = vec![0.0_f64; 1152];
        for (n, sample) in pcm.iter_mut().enumerate().skip(640) {
            *sample = 0.8 * (std::f64::consts::TAU * 2000.0 * n as f64 / 44_100.0).sin();
        }
        let ratio = transient_attack_ratio(&pcm, TRANSIENT_SEGMENTS).unwrap();
        assert!(ratio > TRANSIENT_RATIO_THRESHOLD, "onset ratio {ratio}");
        assert!(is_transient_block(&pcm).unwrap());
    }

    #[test]
    fn silence_is_not_a_transient() {
        // A fully silent block must not be flagged (no division blow-up).
        let pcm = vec![0.0_f64; 1152];
        let ratio = transient_attack_ratio(&pcm, TRANSIENT_SEGMENTS).unwrap();
        assert!(approx(ratio, 1.0, 1.0e-9));
        assert!(!is_transient_block(&pcm).unwrap());
    }

    #[test]
    fn transient_analysis_rejects_empty_or_zero_segments() {
        assert!(transient_attack_ratio(&[], TRANSIENT_SEGMENTS).is_err());
        assert!(transient_attack_ratio(&[1.0, 2.0, 3.0], 0).is_err());
    }

    #[test]
    fn segment_energies_partition_the_block() {
        // The per-segment energies must sum to the block's total energy, with one
        // segment per equal contiguous span.
        let pcm: Vec<f64> = (0..96).map(|n| (n as f64 - 48.0) / 48.0).collect();
        let energies = segment_energies(&pcm, 8).unwrap();
        assert_eq!(energies.len(), 8);
        let total: f64 = pcm.iter().map(|s| s * s).sum();
        let summed: f64 = energies.iter().sum();
        assert!(approx(total, summed, 1.0e-9));
    }

    #[test]
    fn bit_distribution_favors_higher_entropy_granules_and_sums_exactly() {
        // Granule 0 demands three times the entropy of granule 1, so after the
        // shared floor it receives three times the remaining budget.
        let targets = distribute_bits_by_perceptual_entropy(&[3.0, 1.0], 800, 100).unwrap();
        assert_eq!(targets, vec![550, 250]);
        assert_eq!(targets.iter().sum::<usize>(), 800);
        assert!(targets[0] > targets[1]);
    }

    #[test]
    fn bit_distribution_is_exact_with_awkward_rounding() {
        // Equal demand over a budget that does not divide evenly still sums to the
        // exact total via largest-remainder rounding.
        let targets = distribute_bits_by_perceptual_entropy(&[1.0, 1.0, 1.0], 1001, 0).unwrap();
        assert_eq!(targets.iter().sum::<usize>(), 1001);
        // Each granule is within one bit of an even share.
        for &t in &targets {
            assert!(t == 333 || t == 334);
        }
    }

    #[test]
    fn bit_distribution_splits_evenly_without_demand() {
        // No perceptual demand: the budget is shared evenly above the floor.
        let targets =
            distribute_bits_by_perceptual_entropy(&[0.0, 0.0, 0.0, 0.0], 1000, 100).unwrap();
        assert_eq!(targets, vec![250, 250, 250, 250]);
    }

    #[test]
    fn bit_distribution_handles_floors_exceeding_the_budget() {
        // Floors that cannot be met collapse to an even split of the whole budget.
        let targets = distribute_bits_by_perceptual_entropy(&[5.0, 1.0], 150, 100).unwrap();
        assert_eq!(targets, vec![75, 75]);
        assert_eq!(targets.iter().sum::<usize>(), 150);
    }

    #[test]
    fn bit_distribution_validates_inputs() {
        assert!(distribute_bits_by_perceptual_entropy(&[], 100, 10)
            .unwrap()
            .is_empty());
        assert!(distribute_bits_by_perceptual_entropy(&[1.0, -1.0], 100, 10).is_err());
        assert!(distribute_bits_by_perceptual_entropy(&[1.0, f64::NAN], 100, 10).is_err());
    }

    #[test]
    fn mid_side_transform_preserves_energy() {
        // Orthonormal transform: mid² + side² equals left² + right² sample-wise.
        let left = [0.3, -0.7, 0.1, 0.9, -0.2];
        let right = [0.5, 0.2, -0.4, 0.6, 0.8];
        let (mid, side) = mid_side_transform(&left, &right).unwrap();
        for i in 0..left.len() {
            let lr = left[i] * left[i] + right[i] * right[i];
            let ms = mid[i] * mid[i] + side[i] * side[i];
            assert!(approx(lr, ms, 1.0e-12), "energy mismatch at {i}");
        }
        assert!(mid_side_transform(&[1.0, 2.0], &[1.0]).is_err());
    }

    #[test]
    fn identical_channels_choose_mid_side() {
        // Mono-like content: left == right, so the side channel is silent and the
        // fraction is 0 — mid/side is selected.
        let signal: Vec<f64> = (0..256)
            .map(|n| 0.5 * (std::f64::consts::TAU * 440.0 * n as f64 / 44_100.0).sin())
            .collect();
        let fraction = side_energy_fraction(&signal, &signal).unwrap();
        assert!(approx(fraction, 0.0, 1.0e-12));
        assert!(should_use_mid_side(&signal, &signal).unwrap());
    }

    #[test]
    fn anticorrelated_channels_stay_left_right() {
        // Anti-correlated content: right = −left, so the mid channel is silent and
        // all energy is in the side — the fraction is 1 and L/R is kept.
        let left: Vec<f64> = (0..256)
            .map(|n| 0.5 * (std::f64::consts::TAU * 440.0 * n as f64 / 44_100.0).sin())
            .collect();
        let right: Vec<f64> = left.iter().map(|&s| -s).collect();
        let fraction = side_energy_fraction(&left, &right).unwrap();
        assert!(approx(fraction, 1.0, 1.0e-12));
        assert!(!should_use_mid_side(&left, &right).unwrap());
    }

    #[test]
    fn silent_stereo_is_safe_and_defaults_to_mid_side() {
        let zero = [0.0_f64; 64];
        assert!(approx(
            side_energy_fraction(&zero, &zero).unwrap(),
            0.0,
            1.0e-15
        ));
        assert!(should_use_mid_side(&zero, &zero).unwrap());
    }

    #[test]
    fn analyze_long_block_matches_the_individual_paths() {
        // The aggregator must produce exactly what the standalone allowed-noise
        // path produces for the same inputs, and a sensible entropy/transient.
        let fft_len = 1024usize;
        let pcm_window: Vec<f64> = (0..fft_len)
            .map(|n| 0.5 * (std::f64::consts::TAU * 1000.0 * n as f64 / 44_100.0).sin())
            .collect();
        let mdct: Vec<f32> = (0..576)
            .map(|l| 0.3 * (-(l as f32) / 120.0).exp())
            .collect();

        let analysis = analyze_long_block(&mdct, &pcm_window, 44_100).unwrap();
        let reference = perceptual_long_block_allowed_noise(&mdct, &pcm_window, 44_100).unwrap();
        assert_eq!(analysis.allowed_noise, reference);

        // A steady tone is not transient and demands a positive number of bits.
        assert!(!analysis.transient);
        assert!(analysis.perceptual_entropy > 0.0);
        assert!(analysis.perceptual_entropy.is_finite());
    }

    #[test]
    fn analyze_long_block_flags_a_silent_granule_cheaply() {
        let analysis = analyze_long_block(&[0.0_f32; 576], &[0.0_f64; 1024], 44_100).unwrap();
        assert!(approx(analysis.perceptual_entropy, 0.0, 1.0e-12));
        assert!(!analysis.transient);
    }

    #[test]
    fn analysis_rejects_non_finite_input_instead_of_panicking() {
        // Non-finite PCM or MDCT must be reported as an error, not silently
        // propagated as NaN through the model (and never panic).
        let mut pcm = vec![0.1_f64; 1024];
        pcm[10] = f64::NAN;
        let mdct = vec![0.2_f32; 576];
        assert!(analyze_long_block(&mdct, &pcm, 44_100).is_err());

        let pcm_ok = vec![0.1_f64; 1024];
        let mut mdct_bad = vec![0.2_f32; 576];
        mdct_bad[3] = f32::INFINITY;
        assert!(analyze_long_block(&mdct_bad, &pcm_ok, 44_100).is_err());

        // The transient path guards the same way.
        let mut burst = vec![0.0_f64; 1152];
        burst[600] = f64::INFINITY;
        assert!(transient_attack_ratio(&burst, TRANSIENT_SEGMENTS).is_err());
        assert!(is_transient_block(&burst).is_err());
    }
}
