    #[test]
    fn mp3_entropy_targeted_perceptual_reservoir_candidate_passes_ffmpeg_oracle_when_available() {
        let Some(ffmpeg) = std::env::var_os("SONARE_FFMPEG") else {
            return;
        };
        let pcm = readiness_pcm(44_100, 1).unwrap();
        let out_dir = std::env::temp_dir().join(format!(
            "sonare-codec-mp3-entropy-targeted-reservoir-quality-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&out_dir).unwrap();

        let baseline =
            sonare_codec::encode_mpeg1_layer3_pcm_frames_with_perceptual_reservoir_and_table_provider(
                &pcm,
                super::MP3_PERCEPTUAL_DIAGNOSTIC_STEP_CANDIDATES,
                128,
                false,
                sonare_codec::mpeg1_layer3_standard_table_provider(),
            )
            .unwrap();
        let candidate = sonare_codec::encode_mpeg1_layer3_pcm_frames_with_entropy_targeted_perceptual_reservoir_and_table_provider(
            &pcm,
            super::MP3_PERCEPTUAL_DIAGNOSTIC_STEP_CANDIDATES,
            128,
            false,
            0,
            sonare_codec::mpeg1_layer3_standard_table_provider(),
        )
        .unwrap();
        let candidate_details = sonare_codec::select_mpeg1_layer3_entropy_targeted_perceptual_reservoir_frame_details_with_table_provider(
            &pcm,
            super::MP3_PERCEPTUAL_DIAGNOSTIC_STEP_CANDIDATES,
            128,
            false,
            0,
            sonare_codec::mpeg1_layer3_standard_table_provider(),
        )
        .unwrap();

        let entropy_target_bits = candidate_details
            .iter()
            .map(|detail| detail.entropy_target_bits)
            .sum::<usize>();
        let capacity_bits = candidate_details
            .iter()
            .map(|detail| detail.frame_capacity_bytes * 8)
            .sum::<usize>();
        assert_eq!(
            entropy_target_bits, capacity_bits,
            "entropy-targeted reservoir should distribute the full frame capacity"
        );
        assert!(
            candidate_details
                .iter()
                .any(|detail| detail.used_entropy_target_budget),
            "entropy-targeted reservoir did not exercise its entropy budget path"
        );

        let baseline_path = out_dir.join("mp3-perceptual-reservoir-baseline.mp3");
        std::fs::write(&baseline_path, baseline).unwrap();
        run_ffmpeg_acceptance(&ffmpeg, &baseline_path).unwrap();
        let baseline_decoded =
            run_ffmpeg_decode_f32le(&ffmpeg, &baseline_path, pcm.sample_rate, pcm.channels)
                .unwrap();
        let baseline_quality =
            validate_lossy_oracle_pcm_quality(&pcm.samples, &baseline_decoded).unwrap();

        let candidate_path = out_dir.join("mp3-entropy-targeted-perceptual-reservoir.mp3");
        std::fs::write(&candidate_path, candidate).unwrap();
        run_ffmpeg_acceptance(&ffmpeg, &candidate_path).unwrap();
        let candidate_decoded =
            run_ffmpeg_decode_f32le(&ffmpeg, &candidate_path, pcm.sample_rate, pcm.channels)
                .unwrap();
        let candidate_quality =
            validate_lossy_oracle_pcm_quality(&pcm.samples, &candidate_decoded).unwrap();
        std::fs::remove_dir_all(&out_dir).unwrap();

        validate_diagnostic_quality_floor(
            "MP3 entropy-targeted perceptual reservoir diagnostic",
            candidate_quality,
            super::MP3_PERCEPTUAL_DIAGNOSTIC_MIN_DECODED_RMS,
            super::MP3_PERCEPTUAL_DIAGNOSTIC_MIN_CORRELATION,
        )
        .unwrap();
        assert!(
            candidate_quality.best_correlation + 0.05 >= baseline_quality.best_correlation,
            "entropy-targeted reservoir regressed below perceptual reservoir baseline: candidate={candidate_quality:?}, baseline={baseline_quality:?}"
        );
        eprintln!(
            "MP3 entropy-targeted reservoir quality: candidate={candidate_quality:?}, baseline={baseline_quality:?}"
        );
    }

    #[test]
    fn mp3_entropy_target_floor_sweep_keeps_current_production_choice_when_ffmpeg_is_available() {
        let Some(ffmpeg) = std::env::var_os("SONARE_FFMPEG") else {
            return;
        };
        let out_dir = std::env::temp_dir().join(format!(
            "sonare-codec-mp3-entropy-target-floor-sweep-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&out_dir).unwrap();

        for channels in [1, 2] {
            let pcm = readiness_pcm(44_100, channels).unwrap();
            let mut baseline_quality = None;
            let mut best_quality = None;
            let mut best_min_bits = 0usize;

            for min_bits in [0usize, 64, 128, 256, 512] {
                let encoded = sonare_codec::encode_mpeg1_layer3_pcm_frames_with_entropy_targeted_perceptual_reservoir_and_table_provider(
                    &pcm,
                    sonare_codec::MPEG1_LAYER3_PCM_STEP_CANDIDATES,
                    128,
                    false,
                    min_bits,
                    sonare_codec::mpeg1_layer3_standard_table_provider(),
                )
                .unwrap();
                let details = sonare_codec::select_mpeg1_layer3_entropy_targeted_perceptual_reservoir_frame_details_with_table_provider(
                    &pcm,
                    sonare_codec::MPEG1_LAYER3_PCM_STEP_CANDIDATES,
                    128,
                    false,
                    min_bits,
                    sonare_codec::mpeg1_layer3_standard_table_provider(),
                )
                .unwrap();
                let entropy_target_bits = details
                    .iter()
                    .map(|detail| detail.entropy_target_bits)
                    .sum::<usize>();
                let capacity_bits = details
                    .iter()
                    .map(|detail| detail.frame_capacity_bytes * 8)
                    .sum::<usize>();
                assert_eq!(entropy_target_bits, capacity_bits);
                assert!(details
                    .iter()
                    .any(|detail| detail.used_entropy_target_budget));

                let path = out_dir.join(format!(
                    "mp3-entropy-target-floor-{}ch-{min_bits}.mp3",
                    channels
                ));
                std::fs::write(&path, encoded).unwrap();
                super::run_ffmpeg_acceptance(&ffmpeg, &path).unwrap();
                let decoded =
                    super::run_ffmpeg_decode_f32le(&ffmpeg, &path, pcm.sample_rate, pcm.channels)
                        .unwrap();
                let quality =
                    super::validate_lossy_oracle_pcm_quality(&pcm.samples, &decoded).unwrap();
                eprintln!(
                    "MP3 entropy target floor sweep {channels}ch min_bits={min_bits}: decoded_rms={:.4}, best_correlation={:.3}",
                    quality.decoded_rms,
                    quality.best_correlation
                );

                if min_bits == 0 {
                    baseline_quality = Some(quality);
                }
                if best_quality.is_none_or(|best: LossyOraclePcmQuality| {
                    quality.best_correlation > best.best_correlation
                        || ((quality.best_correlation - best.best_correlation).abs() <= 0.001
                            && quality.decoded_rms > best.decoded_rms)
                }) {
                    best_quality = Some(quality);
                    best_min_bits = min_bits;
                }
            }

            let baseline_quality = baseline_quality.unwrap();
            let best_quality = best_quality.unwrap();
            assert!(
                baseline_quality.best_correlation + 0.001 >= best_quality.best_correlation,
                "{channels}ch entropy target floor sweep found better min_bits={best_min_bits}: baseline={baseline_quality:?}, best={best_quality:?}"
            );
            assert_eq!(
                best_min_bits, 0,
                "{channels}ch entropy target floor sweep should keep current production min_bits while correlation is tied"
            );
        }

        std::fs::remove_dir_all(&out_dir).unwrap();
    }

    #[test]
    fn mp3_entropy_target_candidate_floor_sweep_tracks_mono_quality_tradeoff_when_ffmpeg_is_available(
    ) {
        let Some(ffmpeg) = std::env::var_os("SONARE_FFMPEG") else {
            return;
        };
        let out_dir = std::env::temp_dir().join(format!(
            "sonare-codec-mp3-entropy-target-candidate-floor-sweep-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&out_dir).unwrap();

        let pcm = readiness_pcm(44_100, 1).unwrap();
        let fine_only = [0.0005_f32];
        let fine_encoded = sonare_codec::encode_mpeg1_layer3_pcm_frames_with_entropy_targeted_perceptual_reservoir_and_table_provider(
            &pcm,
            &fine_only,
            128,
            false,
            0,
            sonare_codec::mpeg1_layer3_standard_table_provider(),
        )
        .unwrap();
        let fine_details = sonare_codec::select_mpeg1_layer3_entropy_targeted_perceptual_reservoir_frame_details_with_table_provider(
            &pcm,
            &fine_only,
            128,
            false,
            0,
            sonare_codec::mpeg1_layer3_standard_table_provider(),
        )
        .unwrap();
        let fine_path = out_dir.join("mp3-entropy-target-candidate-floor-fine-only.mp3");
        std::fs::write(&fine_path, fine_encoded).unwrap();
        super::run_ffmpeg_acceptance(&ffmpeg, &fine_path).unwrap();
        let fine_decoded =
            super::run_ffmpeg_decode_f32le(&ffmpeg, &fine_path, pcm.sample_rate, pcm.channels)
                .unwrap();
        let fine_quality =
            super::validate_lossy_oracle_pcm_quality(&pcm.samples, &fine_decoded).unwrap();
        let fine_max_payload = fine_details
            .iter()
            .map(|detail| detail.payload_bit_len)
            .max()
            .unwrap_or(0);
        eprintln!(
            "MP3 entropy target candidate floor sweep fine-only: max_payload_bits={fine_max_payload}, decoded_rms={:.4}, best_correlation={:.3}",
            fine_quality.decoded_rms,
            fine_quality.best_correlation
        );

        let mut best_quality = None;
        let mut best_selected_step = 0.0_f32;
        for min_step in [0.0005_f32, 0.001, 0.002, 0.005, 0.01, 0.1, 1.0, 2.0] {
            let candidates: Vec<f32> = sonare_codec::MPEG1_LAYER3_PCM_STEP_CANDIDATES
                .iter()
                .copied()
                .filter(|step| *step >= min_step)
                .collect();
            let encoded = sonare_codec::encode_mpeg1_layer3_pcm_frames_with_entropy_targeted_perceptual_reservoir_and_table_provider(
                &pcm,
                &candidates,
                128,
                false,
                0,
                sonare_codec::mpeg1_layer3_standard_table_provider(),
            )
            .unwrap();
            let details = sonare_codec::select_mpeg1_layer3_entropy_targeted_perceptual_reservoir_frame_details_with_table_provider(
                &pcm,
                &candidates,
                128,
                false,
                0,
                sonare_codec::mpeg1_layer3_standard_table_provider(),
            )
            .unwrap();
            let max_payload = details
                .iter()
                .map(|detail| detail.payload_bit_len)
                .max()
                .unwrap_or(0);
            let selected_min_step = details
                .iter()
                .map(|detail| detail.step)
                .fold(f32::INFINITY, f32::min);
            let path = out_dir.join(format!(
                "mp3-entropy-target-candidate-floor-{min_step:.4}.mp3"
            ));
            std::fs::write(&path, encoded).unwrap();
            super::run_ffmpeg_acceptance(&ffmpeg, &path).unwrap();
            let decoded =
                super::run_ffmpeg_decode_f32le(&ffmpeg, &path, pcm.sample_rate, pcm.channels)
                    .unwrap();
            let quality = super::validate_lossy_oracle_pcm_quality(&pcm.samples, &decoded).unwrap();
            eprintln!(
                "MP3 entropy target candidate floor sweep min_step={min_step}: selected_min_step={selected_min_step}, max_payload_bits={max_payload}, decoded_rms={:.4}, best_correlation={:.3}",
                quality.decoded_rms,
                quality.best_correlation
            );
            if best_quality.is_none_or(|best: LossyOraclePcmQuality| {
                quality.best_correlation > best.best_correlation
                    || ((quality.best_correlation - best.best_correlation).abs() <= 0.001
                        && quality.decoded_rms > best.decoded_rms)
            }) {
                best_quality = Some(quality);
                best_selected_step = selected_min_step;
            }
        }

        let best_quality = best_quality.unwrap();
        assert_eq!(best_selected_step, 2.0);
        assert!(
            best_quality.best_correlation >= 0.38,
            "mono candidate floor sweep should promote the richer nonzero-scale-factor quality region: best_selected_step={best_selected_step}, best={best_quality:?}"
        );
        assert!(
            fine_max_payload > 2_000,
            "fine-only candidate should demonstrate the high-payload zero-scale-factor region: payload={fine_max_payload}"
        );
        assert!(
            fine_quality.best_correlation + 0.05 < best_quality.best_correlation,
            "fine-only candidate should remain below the active scale-factor quality region: fine={fine_quality:?}, best={best_quality:?}"
        );

        std::fs::remove_dir_all(&out_dir).unwrap();
    }

    #[test]
    fn mp3_entropy_target_utilization_exposes_mono_rate_control_gap() {
        fn utilization(
            channels: u16,
        ) -> (
            Vec<sonare_codec::Layer3EntropyTargetedReservoirFrameSelection>,
            sonare_codec::Layer3EntropyTargetUtilizationProfile,
        ) {
            let pcm = readiness_pcm(44_100, channels).unwrap();
            let details = sonare_codec::select_mpeg1_layer3_entropy_targeted_perceptual_reservoir_frame_details_with_table_provider(
                &pcm,
                sonare_codec::MPEG1_LAYER3_PCM_STEP_CANDIDATES,
                128,
                false,
                0,
                sonare_codec::mpeg1_layer3_standard_table_provider(),
            )
            .unwrap();
            let profile = sonare_codec::mpeg1_layer3_entropy_target_utilization_profile(&details);
            let selected_profile =
                sonare_codec::select_mpeg1_layer3_entropy_target_utilization_profile_with_table_provider(
                    &pcm,
                    sonare_codec::MPEG1_LAYER3_PCM_STEP_CANDIDATES,
                    128,
                    false,
                    0,
                    sonare_codec::mpeg1_layer3_standard_table_provider(),
                )
                .unwrap();
            assert_eq!(profile, selected_profile);
            for detail in &details {
                if !detail.used_entropy_target_budget {
                    continue;
                }
                let entropy_budget_bits = detail
                    .entropy_target_bits
                    .saturating_add(7)
                    .checked_div(8)
                    .unwrap_or(0)
                    .clamp(1, detail.frame_capacity_bytes + detail.main_data_begin)
                    * 8;
                assert!(detail.payload_bit_len <= entropy_budget_bits);
            }
            (details, profile)
        }

        let (mono_details, mono_profile) = utilization(1);
        let (stereo_details, stereo_profile) = utilization(2);

        assert!(mono_details
            .iter()
            .all(|detail| detail.perceptual_granules > 0 && detail.calibrated_granules == 0));
        assert!(stereo_details
            .iter()
            .all(|detail| detail.perceptual_granules > 0 && detail.calibrated_granules == 0));
        assert!(
            mono_profile.utilization < 0.10,
            "mono entropy target path unexpectedly started using most of its budget; revisit rate-control gap assumptions: profile={mono_profile:?}, details={mono_details:?}"
        );
        assert!(
            stereo_profile.utilization > 0.50,
            "stereo entropy target path should remain substantially budget-active: profile={stereo_profile:?}, details={stereo_details:?}"
        );
        assert!(
            mono_profile.max_entropy_budget_slack_bits
                > stereo_profile.max_entropy_budget_slack_bits,
            "mono should expose the larger scale-factor/rate-control slack: mono={mono_profile:?}, stereo={stereo_profile:?}"
        );
        eprintln!(
            "MP3 entropy target utilization gap: mono_profile={mono_profile:?}, stereo_profile={stereo_profile:?}"
        );
    }

    #[test]
    fn mp3_first_frame_candidate_profile_explains_mono_rate_control_gap() {
        fn profile(
            channels: u16,
        ) -> (
            Vec<sonare_codec::Layer3PerceptualCandidateProfile>,
            Vec<sonare_codec::Layer3EntropyTargetedReservoirFrameSelection>,
        ) {
            let pcm = readiness_pcm(44_100, channels).unwrap();
            let candidate_profile =
                sonare_codec::select_mpeg1_layer3_first_frame_perceptual_candidate_profile_with_table_provider(
                    &pcm,
                    sonare_codec::MPEG1_LAYER3_PCM_STEP_CANDIDATES,
                    128,
                    false,
                    sonare_codec::mpeg1_layer3_standard_table_provider(),
                )
                .unwrap();
            let details =
                sonare_codec::select_mpeg1_layer3_entropy_targeted_perceptual_reservoir_frame_details_with_table_provider(
                    &pcm,
                    sonare_codec::MPEG1_LAYER3_PCM_STEP_CANDIDATES,
                    128,
                    false,
                    0,
                    sonare_codec::mpeg1_layer3_standard_table_provider(),
                )
                .unwrap();
            (candidate_profile, details)
        }

        let (mono_profile, mono_details) = profile(1);
        let (stereo_profile, stereo_details) = profile(2);

        let mono_first_active = mono_profile
            .iter()
            .find(|profile| profile.nonzero_scale_factors > 0)
            .copied()
            .unwrap();
        let mono_largest_zero_payload = mono_profile
            .iter()
            .filter(|profile| profile.nonzero_scale_factors == 0)
            .map(|profile| profile.payload_bit_len)
            .max()
            .unwrap_or(0);
        let stereo_largest_zero_payload = stereo_profile
            .iter()
            .filter(|profile| profile.nonzero_scale_factors == 0)
            .map(|profile| profile.payload_bit_len)
            .max()
            .unwrap_or(0);
        let mono_max_payload = mono_details
            .iter()
            .map(|detail| detail.payload_bit_len)
            .max()
            .unwrap_or(0);
        let stereo_max_payload = stereo_details
            .iter()
            .map(|detail| detail.payload_bit_len)
            .max()
            .unwrap_or(0);
        let mono_capacity_bits = mono_details
            .iter()
            .map(|detail| detail.frame_capacity_bytes * 8)
            .max()
            .unwrap_or(0);
        let stereo_capacity_bits = stereo_details
            .iter()
            .map(|detail| detail.frame_capacity_bytes * 8)
            .max()
            .unwrap_or(0);

        eprintln!(
            "MP3 first-frame candidate profile mono: first_active={mono_first_active:?}, largest_zero_payload={mono_largest_zero_payload}, production_max_payload={mono_max_payload}, capacity_bits={mono_capacity_bits}, details={mono_details:?}"
        );
        eprintln!(
            "MP3 first-frame candidate profile stereo: largest_zero_payload={stereo_largest_zero_payload}, production_max_payload={stereo_max_payload}, capacity_bits={stereo_capacity_bits}, details={stereo_details:?}"
        );

        assert_eq!(
            mono_first_active.step, 1.0,
            "mono active scale-factor region should still start at the coarse entropy-targeted step: profile={mono_profile:?}"
        );
        assert!(
            mono_largest_zero_payload > mono_capacity_bits / 2,
            "mono zero-scale-factor fine candidates should still demonstrate high payload but poor quality pressure: zero_payload={mono_largest_zero_payload}, capacity={mono_capacity_bits}, profile={mono_profile:?}"
        );
        assert!(
            mono_first_active.payload_bit_len < mono_capacity_bits / 20,
            "mono active candidate should expose the low-payload rate-control gap: active={mono_first_active:?}, capacity={mono_capacity_bits}"
        );
        assert!(
            mono_max_payload <= mono_first_active.payload_bit_len * 2,
            "mono entropy-targeted candidate selection should remain tied to the low-payload active region: max_payload={mono_max_payload}, first_active={mono_first_active:?}"
        );
        assert!(
            stereo_largest_zero_payload > stereo_capacity_bits / 2,
            "stereo zero-scale-factor fine candidates should remain budget-active unlike mono's quality-limited fine region: zero_payload={stereo_largest_zero_payload}, capacity={stereo_capacity_bits}, profile={stereo_profile:?}"
        );
        assert!(
            stereo_max_payload > stereo_capacity_bits / 2,
            "stereo production should continue using substantial payload budget: max_payload={stereo_max_payload}, capacity={stereo_capacity_bits}"
        );
    }

    #[test]
    fn mp3_low_band_spectral_shape_profile_tracks_mono_proxy_gap() {
        let pcm = readiness_pcm(44_100, 1).unwrap();
        let perceptual_profiles =
            sonare_codec::select_mpeg1_layer3_first_frame_perceptual_candidate_profile_with_table_provider(
                &pcm,
                sonare_codec::MPEG1_LAYER3_PCM_STEP_CANDIDATES,
                128,
                false,
                sonare_codec::mpeg1_layer3_standard_table_provider(),
            )
            .unwrap();
        let shape_profiles =
            sonare_codec::select_mpeg1_layer3_first_frame_low_band_spectral_shape_candidate_profile_with_table_provider(
                &pcm,
                sonare_codec::MPEG1_LAYER3_PCM_STEP_CANDIDATES,
                128,
                false,
                sonare_codec::mpeg1_layer3_standard_table_provider(),
            )
            .unwrap();
        let band_shape_profiles =
            sonare_codec::select_mpeg1_layer3_first_frame_band_spectral_shape_candidate_profile_with_table_provider(
                &pcm,
                sonare_codec::MPEG1_LAYER3_PCM_STEP_CANDIDATES,
                128,
                false,
                sonare_codec::mpeg1_layer3_standard_table_provider(),
            )
            .unwrap();
        let fine = shape_profiles
            .iter()
            .find(|profile| profile.step == 0.2)
            .copied()
            .unwrap();
        let very_fine = shape_profiles
            .iter()
            .find(|profile| profile.step == 0.0005)
            .copied()
            .unwrap();
        let first_active = perceptual_profiles
            .iter()
            .find(|profile| profile.nonzero_scale_factors > 0)
            .copied()
            .unwrap();
        let active_shape = shape_profiles
            .iter()
            .find(|profile| profile.step == first_active.step)
            .copied()
            .unwrap();
        let production_region = shape_profiles
            .iter()
            .find(|profile| profile.step == 2.0)
            .copied()
            .unwrap();

        eprintln!(
            "MP3 low-band spectral shape profile: very_fine={very_fine:?}, fine={fine:?}, first_active={first_active:?}, active_shape={active_shape:?}, production_region={production_region:?}, band_profile_rows={}, all={shape_profiles:?}",
            band_shape_profiles.len()
        );

        assert!(
            shape_profiles.iter().all(|profile| {
                profile.low_band_abs_sum <= profile.total_abs_sum
                    && profile.low_band_nonzero_lines <= profile.total_nonzero_lines
            }),
            "low-band profile should be internally bounded: {shape_profiles:?}"
        );
        assert_eq!(
            band_shape_profiles.len(),
            sonare_codec::MPEG1_LAYER3_PCM_STEP_CANDIDATES.len()
                * sonare_codec::MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT
        );
        assert!(
            band_shape_profiles.iter().all(|profile| {
                profile.band < sonare_codec::MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT
                    && profile.band_start <= profile.band_end
                    && profile.band_abs_sum <= profile.total_abs_sum
                    && profile.band_nonzero_lines <= profile.total_nonzero_lines
            }),
            "band spectral shape profile should be internally bounded: {band_shape_profiles:?}"
        );
        let fine_band_low_abs: u64 = band_shape_profiles
            .iter()
            .filter(|profile| profile.step == fine.step && profile.band < 7)
            .map(|profile| profile.band_abs_sum)
            .sum();
        let fine_band_low_nonzero: usize = band_shape_profiles
            .iter()
            .filter(|profile| profile.step == fine.step && profile.band < 7)
            .map(|profile| profile.band_nonzero_lines)
            .sum();
        assert_eq!(fine_band_low_abs, fine.low_band_abs_sum);
        assert_eq!(fine_band_low_nonzero, fine.low_band_nonzero_lines);
        assert!(
            very_fine.payload_bit_len > active_shape.payload_bit_len * 10,
            "very fine candidate should expose high bit growth outside the active scale-factor region: very_fine={very_fine:?}, active_shape={active_shape:?}"
        );
        assert!(
            fine.low_band_abs_sum > production_region.low_band_abs_sum,
            "fine-step candidate should carry more low-band quantized magnitude while still failing the FFmpeg quality proxy: fine={fine:?}, production_region={production_region:?}"
        );
        assert!(
            active_shape.low_band_nonzero_lines > 0
                && production_region.low_band_nonzero_lines > 0,
            "active/production-region candidates should keep low-band spectral support: active={active_shape:?}, production={production_region:?}"
        );
        assert!(
            first_active.step >= 1.0
                && production_region.low_band_abs_sum < fine.low_band_abs_sum
                && production_region.low_band_nonzero_lines <= fine.low_band_nonzero_lines,
            "mono production-region proxy should remain coarse with less low-band spectral magnitude than the quality-gap fine step: first_active={first_active:?}, fine={fine:?}, production={production_region:?}"
        );
    }

    #[test]
    fn mp3_low_band_shape_oracle_sweep_keeps_shape_proxy_below_production_when_ffmpeg_is_available()
    {
        let Some(ffmpeg) = std::env::var_os("SONARE_FFMPEG") else {
            return;
        };
        let pcm = readiness_pcm(44_100, 1).unwrap();
        let out_dir = std::env::temp_dir().join(format!(
            "sonare-codec-mp3-low-band-shape-oracle-sweep-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&out_dir).unwrap();

        let production = sonare_codec::encode_with_mode(
            sonare_codec::Format::Mp3,
            &pcm,
            sonare_codec::EncodeMode::ProductionOnly,
        )
        .unwrap();
        let production_path = out_dir.join("mp3-low-band-shape-production.mp3");
        std::fs::write(&production_path, production).unwrap();
        run_ffmpeg_clean_acceptance(&ffmpeg, &production_path).unwrap();
        let production_decoded =
            run_ffmpeg_decode_f32le(&ffmpeg, &production_path, pcm.sample_rate, pcm.channels)
                .unwrap();
        let production_quality =
            validate_lossy_oracle_pcm_quality(&pcm.samples, &production_decoded).unwrap();

        let probe_steps = [0.0005_f32, 0.001, 0.01, 0.2, 1.0, 2.0, 5.0, 10.0];
        let shape_profiles =
            sonare_codec::select_mpeg1_layer3_first_frame_low_band_spectral_shape_candidate_profile_with_table_provider(
                &pcm,
                &probe_steps,
                128,
                false,
                sonare_codec::mpeg1_layer3_standard_table_provider(),
            )
            .unwrap();
        let mut results = Vec::new();
        for profile in shape_profiles {
            let encoded = match sonare_codec::encode_mpeg1_layer3_pcm_frames_with_perceptual_scale_factors_and_table_provider(
                &pcm,
                profile.step,
                sonare_codec::mpeg1_layer3_standard_table_provider(),
            ) {
                Ok(encoded) => encoded,
                Err(err) => {
                    eprintln!(
                        "MP3 low-band shape oracle step={}: encode failed: {err}",
                        profile.step
                    );
                    continue;
                }
            };
            let path = out_dir.join(format!("mp3-low-band-shape-{:.6}.mp3", profile.step));
            std::fs::write(&path, encoded).unwrap();
            run_ffmpeg_clean_acceptance(&ffmpeg, &path).unwrap();
            let decoded =
                run_ffmpeg_decode_f32le(&ffmpeg, &path, pcm.sample_rate, pcm.channels).unwrap();
            let (best_correlation, best_offset) =
                best_normalized_correlation_with_offset(&pcm.samples, &decoded).unwrap();
            let quality = LossyOraclePcmQuality {
                decoded_rms: rms(&decoded),
                best_correlation,
            };
            eprintln!(
                "MP3 low-band shape oracle step={}: profile={profile:?}, quality={quality:?}, best_offset={best_offset}, production={production_quality:?}",
                profile.step
            );
            results.push((profile, quality, best_offset));
        }

        let best_quality = results
            .iter()
            .copied()
            .max_by(|(_, left, _), (_, right, _)| {
                left.best_correlation
                    .partial_cmp(&right.best_correlation)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap();
        let max_low_abs = results
            .iter()
            .copied()
            .max_by_key(|(profile, _, _)| profile.low_band_abs_sum)
            .unwrap();
        let max_payload = results
            .iter()
            .copied()
            .max_by_key(|(profile, _, _)| profile.payload_bit_len)
            .unwrap();
        let max_loudness = results
            .iter()
            .copied()
            .max_by(|(_, left, _), (_, right, _)| {
                left.decoded_rms
                    .partial_cmp(&right.decoded_rms)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap();

        assert_eq!(
            best_quality.0.step, 2.0,
            "low-band shape oracle should keep step=2.0 as the best tested mono fixed-step region: best={best_quality:?}, results={results:?}"
        );
        assert!(
            production_quality.best_correlation > best_quality.1.best_correlation + 0.02,
            "production low-band gain reservoir should exceed the best self-contained low-band shape region: best={best_quality:?}, production={production_quality:?}, results={results:?}"
        );
        assert_eq!(
            max_low_abs.0.step, 0.0005,
            "very fine candidate should expose the maximum low-band magnitude: max_low_abs={max_low_abs:?}, results={results:?}"
        );
        assert_eq!(
            max_payload.0.step, 0.0005,
            "very fine candidate should expose the maximum first-frame payload: max_payload={max_payload:?}, results={results:?}"
        );
        assert!(
            max_low_abs.1.best_correlation + 0.02 < production_quality.best_correlation
                && max_payload.1.best_correlation + 0.02 < production_quality.best_correlation,
            "shape-only or payload-only proxy should not be promoted over current production: max_low_abs={max_low_abs:?}, max_payload={max_payload:?}, production={production_quality:?}"
        );
        assert!(
            max_loudness.0.step != best_quality.0.step
                && max_loudness.1.best_correlation + 0.005 < best_quality.1.best_correlation,
            "loudness-only proxy should not be promoted over the best correlation region: max_loudness={max_loudness:?}, best={best_quality:?}, production={production_quality:?}"
        );
        assert!(
            results.iter().all(|(_, _, offset)| *offset == 0),
            "low-band shape oracle should expose a spectral-shape gap, not lag correction: results={results:?}"
        );

        std::fs::remove_dir_all(&out_dir).unwrap();
    }

    #[test]
    fn mp3_mono_fixed_step_scale_factor_path_sweep_tracks_quality_proxy_gap_when_ffmpeg_is_available(
    ) {
        let Some(ffmpeg) = std::env::var_os("SONARE_FFMPEG") else {
            return;
        };
        let pcm = readiness_pcm(44_100, 1).unwrap();
        let out_dir = std::env::temp_dir().join(format!(
            "sonare-codec-mp3-mono-fixed-step-scale-factor-sweep-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&out_dir).unwrap();

        let production = sonare_codec::encode_with_mode(
            sonare_codec::Format::Mp3,
            &pcm,
            sonare_codec::EncodeMode::ProductionOnly,
        )
        .unwrap();
        let production_path = out_dir.join("mp3-mono-fixed-step-production.mp3");
        std::fs::write(&production_path, production).unwrap();
        run_ffmpeg_clean_acceptance(&ffmpeg, &production_path).unwrap();
        let production_decoded =
            run_ffmpeg_decode_f32le(&ffmpeg, &production_path, pcm.sample_rate, pcm.channels)
                .unwrap();
        let production_quality =
            validate_lossy_oracle_pcm_quality(&pcm.samples, &production_decoded).unwrap();

        let mut selected_results = Vec::new();
        let mut perceptual_results = Vec::new();
        let mut scalefac_scale_results = Vec::new();
        let mut allowed_noise_scale_results = Vec::new();
        for step in [0.2_f32, 0.5, 1.0, 2.0] {
            let selected_quality = match sonare_codec::encode_mpeg1_layer3_pcm_frames_with_selected_scale_factors_and_table_provider(
                &pcm,
                step,
                sonare_codec::mpeg1_layer3_standard_table_provider(),
            ) {
                Ok(selected) => {
                    let selected_path =
                        out_dir.join(format!("mp3-mono-fixed-step-selected-{step:.1}.mp3"));
                    std::fs::write(&selected_path, selected).unwrap();
                    run_ffmpeg_clean_acceptance(&ffmpeg, &selected_path).unwrap();
                    let selected_decoded = run_ffmpeg_decode_f32le(
                        &ffmpeg,
                        &selected_path,
                        pcm.sample_rate,
                        pcm.channels,
                    )
                    .unwrap();
                    match validate_lossy_oracle_pcm_quality(&pcm.samples, &selected_decoded) {
                        Ok(quality) => {
                            selected_results.push((step, quality));
                            Some(quality)
                        }
                        Err(err) => {
                            eprintln!(
                                "MP3 mono fixed-step selected path step={step}: quality rejected: {err}"
                            );
                            None
                        }
                    }
                }
                Err(err) => {
                    eprintln!("MP3 mono fixed-step selected path step={step}: encode failed: {err}");
                    None
                }
            };

            let perceptual = match sonare_codec::encode_mpeg1_layer3_pcm_frames_with_perceptual_scale_factors_and_table_provider(
                &pcm,
                step,
                sonare_codec::mpeg1_layer3_standard_table_provider(),
            ) {
                Ok(encoded) => encoded,
                Err(err) => {
                    eprintln!("MP3 mono fixed-step perceptual path step={step}: encode failed: {err}");
                    continue;
                }
            };
            let perceptual_path =
                out_dir.join(format!("mp3-mono-fixed-step-perceptual-{step:.1}.mp3"));
            std::fs::write(&perceptual_path, perceptual).unwrap();
            run_ffmpeg_clean_acceptance(&ffmpeg, &perceptual_path).unwrap();
            let perceptual_decoded =
                run_ffmpeg_decode_f32le(&ffmpeg, &perceptual_path, pcm.sample_rate, pcm.channels)
                    .unwrap();
            let perceptual_quality =
                match validate_lossy_oracle_pcm_quality(&pcm.samples, &perceptual_decoded) {
                    Ok(quality) => quality,
                    Err(err) => {
                        eprintln!(
                            "MP3 mono fixed-step perceptual path step={step}: rejected: {err}"
                        );
                        continue;
                    }
                };
            perceptual_results.push((step, perceptual_quality));

            let scalefac_scale = match sonare_codec::encode_mpeg1_layer3_pcm_frames_with_perceptual_scalefac_scale_and_table_provider(
                &pcm,
                step,
                true,
                sonare_codec::mpeg1_layer3_standard_table_provider(),
            ) {
                Ok(encoded) => encoded,
                Err(err) => {
                    eprintln!("MP3 mono fixed-step scalefac_scale path step={step}: encode failed: {err}");
                    continue;
                }
            };
            let scalefac_scale_path =
                out_dir.join(format!("mp3-mono-fixed-step-scalefac-scale-{step:.1}.mp3"));
            std::fs::write(&scalefac_scale_path, scalefac_scale).unwrap();
            run_ffmpeg_clean_acceptance(&ffmpeg, &scalefac_scale_path).unwrap();
            let scalefac_scale_decoded = run_ffmpeg_decode_f32le(
                &ffmpeg,
                &scalefac_scale_path,
                pcm.sample_rate,
                pcm.channels,
            )
            .unwrap();
            let scalefac_scale_quality =
                match validate_lossy_oracle_pcm_quality(&pcm.samples, &scalefac_scale_decoded) {
                    Ok(quality) => quality,
                    Err(err) => {
                        eprintln!(
                            "MP3 mono fixed-step scalefac_scale path step={step}: rejected: {err}"
                        );
                        continue;
                    }
                };
            scalefac_scale_results.push((step, scalefac_scale_quality));

            eprintln!(
                "MP3 mono fixed-step scale-factor sweep step={step}: selected={selected_quality:?}, perceptual={perceptual_quality:?}, scalefac_scale={scalefac_scale_quality:?}, production={production_quality:?}"
            );
        }
        for (step, allowed_noise_scale) in [(1.0_f32, 0.5_f64), (2.0, 0.5), (2.0, 0.25)] {
            let encoded = match sonare_codec::encode_mpeg1_layer3_pcm_frames_with_perceptual_allowed_noise_scale_and_table_provider(
                &pcm,
                step,
                allowed_noise_scale,
                sonare_codec::mpeg1_layer3_standard_table_provider(),
            ) {
                Ok(encoded) => encoded,
                Err(err) => {
                    eprintln!("MP3 mono allowed-noise scale path step={step} scale={allowed_noise_scale}: encode failed: {err}");
                    continue;
                }
            };
            let path = out_dir.join(format!(
                "mp3-mono-fixed-step-allowed-noise-{step:.1}-{allowed_noise_scale:.2}.mp3"
            ));
            std::fs::write(&path, encoded).unwrap();
            run_ffmpeg_clean_acceptance(&ffmpeg, &path).unwrap();
            let decoded =
                run_ffmpeg_decode_f32le(&ffmpeg, &path, pcm.sample_rate, pcm.channels).unwrap();
            let quality = match validate_lossy_oracle_pcm_quality(&pcm.samples, &decoded) {
                Ok(quality) => quality,
                Err(err) => {
                    eprintln!(
                        "MP3 mono allowed-noise scale path step={step} scale={allowed_noise_scale}: rejected: {err}"
                    );
                    continue;
                }
            };
            eprintln!(
                "MP3 mono allowed-noise scale path step={step} scale={allowed_noise_scale}: quality={quality:?}, production={production_quality:?}"
            );
            allowed_noise_scale_results.push((step, allowed_noise_scale, quality));
        }

        assert!(
            perceptual_results
                .iter()
                .any(|(step, quality)| *step <= 0.2
                    && quality.best_correlation + 0.02 < production_quality.best_correlation),
            "fine-step perceptual path should still expose the mono quality-proxy gap: perceptual={perceptual_results:?}, production={production_quality:?}"
        );
        let best_perceptual = perceptual_results
            .iter()
            .copied()
            .max_by(|(_, left), (_, right)| {
                left.best_correlation
                    .partial_cmp(&right.best_correlation)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap();
        assert!(
            production_quality.best_correlation > best_perceptual.1.best_correlation + 0.02,
            "production low-band gain reservoir should exceed the best fixed-step perceptual quality region: best={best_perceptual:?}, production={production_quality:?}"
        );
        assert!(
            selected_results
                .iter()
                .any(|(step, quality)| *step <= 0.2
                    && quality.best_correlation + 0.02 < production_quality.best_correlation),
            "selected scale-factor fine steps should also remain below production quality: selected={selected_results:?}, production={production_quality:?}"
        );
        assert!(
            !scalefac_scale_results.is_empty(),
            "scalefac_scale diagnostic should produce at least one accepted candidate"
        );
        let best_scalefac_scale = scalefac_scale_results
            .iter()
            .copied()
            .max_by(|(_, left), (_, right)| {
                left.best_correlation
                    .partial_cmp(&right.best_correlation)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap();
        assert!(
            best_scalefac_scale.1.best_correlation <= production_quality.best_correlation + 0.02,
            "scalefac_scale=true should be promoted only if it materially beats current production: best={best_scalefac_scale:?}, all={scalefac_scale_results:?}, production={production_quality:?}"
        );
        assert!(
            !allowed_noise_scale_results.is_empty(),
            "allowed-noise scale diagnostic should produce at least one accepted candidate"
        );
        let best_allowed_noise_scale = allowed_noise_scale_results
            .iter()
            .copied()
            .max_by(|(_, _, left), (_, _, right)| {
                left.best_correlation
                    .partial_cmp(&right.best_correlation)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap();
        assert!(
            best_allowed_noise_scale.2.best_correlation <= production_quality.best_correlation + 0.02,
            "allowed-noise scale should be promoted only if it materially beats current production: best={best_allowed_noise_scale:?}, all={allowed_noise_scale_results:?}, production={production_quality:?}"
        );

        std::fs::remove_dir_all(&out_dir).unwrap();
    }

