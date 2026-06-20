    #[test]
    fn aac_standard_id_balanced_gain_bias_sweep_tracks_loudness_ceiling_when_ffmpeg_is_available() {
        let Some(ffmpeg) = std::env::var_os("SONARE_FFMPEG") else {
            eprintln!(
                "skipping AAC balanced gain/bias loudness sweep: set SONARE_FFMPEG=/path/to/ffmpeg"
            );
            return;
        };
        let mono = readiness_pcm(44_100, 1).unwrap();
        let stereo = super::aac_standard_surface_stereo_pcm(&mono).unwrap();
        let out_dir = std::env::temp_dir().join(format!(
            "sonare-codec-aac-balanced-gain-bias-sweep-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&out_dir).unwrap();

        let cases: [(&str, sonare_codec::AudioBuffer, u32, f64); 2] = [
            (
                "mono",
                mono,
                sonare_codec::aac_lc_default_production_bitrate_bps(1).unwrap(),
                0.45,
            ),
            (
                "stereo",
                stereo,
                sonare_codec::aac_lc_default_production_bitrate_bps(2).unwrap(),
                0.50,
            ),
        ];
        for (label, pcm, bitrate, min_correlation) in cases {
            let baseline_details =
                sonare_codec::aac_recommended_standard_selected_scale_factor_frame_details_with_bitrate(
                    &pcm,
                    bitrate,
                )
                .unwrap();
            let baseline_breakdown = super::aac_standard_id_payload_breakdown_for_frame_selection(
                &pcm,
                &baseline_details,
            )
            .unwrap();
            let baseline_adts =
                sonare_codec::encode_aac_adts_with_recommended_standard_spectral_offsets_and_selected_scale_factors_and_bitrate(
                    &pcm,
                    bitrate,
                )
                .unwrap();
            let baseline_path = out_dir.join(format!(
                "aac-standard-id-balanced-sweep-{label}-baseline.aac"
            ));
            std::fs::write(&baseline_path, baseline_adts).unwrap();
            let baseline_decoded =
                run_ffmpeg_decode_f32le(&ffmpeg, &baseline_path, pcm.sample_rate, pcm.channels)
                    .unwrap();
            let baseline_quality =
                validate_lossy_oracle_pcm_quality(&pcm.samples, &baseline_decoded).unwrap();
            let balance_profile =
                sonare_codec::aac_standard_id_selected_scale_factor_balance_profile(pcm.channels)
                    .unwrap();
            let quality_control_candidates =
                sonare_codec::aac_standard_id_quality_control_candidates_for_balance_profile_with_bitrate(
                    &pcm,
                    bitrate,
                )
                .unwrap();
            assert!(
                quality_control_candidates.iter().any(|candidate| {
                    candidate.global_gain == balance_profile.selected_global_gain
                        && candidate.scale_factor_magnitude_bias
                            == balance_profile.selected_magnitude_bias
                        && candidate.max_quantized_abs == balance_profile.max_quantized_abs
                }),
                "{label} balanced quality-control candidates did not include selected profile candidate: candidates={quality_control_candidates:?}, profile={balance_profile:?}"
            );
            let mut best: Option<(
                u8,
                i16,
                LossyOraclePcmQuality,
                sonare_codec::AacStandardIdQualityControlProfile,
            )> = None;

            for candidate in quality_control_candidates {
                let global_gain = candidate.global_gain;
                let magnitude_bias = candidate.scale_factor_magnitude_bias;
                let max_quantized_abs = candidate.max_quantized_abs;
                let profile = candidate.profile;
                if profile.max_abs >= baseline_breakdown.max_abs
                    || profile.escape_spectral_bits >= baseline_breakdown.escape_spectral_bits
                {
                    continue;
                }

                let adts = sonare_codec::encode_aac_adts_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_max_quantized_abs_and_bitrate(
                    &pcm,
                    bitrate,
                    global_gain,
                    magnitude_bias,
                    max_quantized_abs,
                )
                .unwrap();
                let path = out_dir.join(format!(
                    "aac-standard-id-balanced-sweep-{label}-gain-{global_gain}-bias-{magnitude_bias}.aac"
                ));
                std::fs::write(&path, adts).unwrap();
                run_ffmpeg_clean_acceptance(&ffmpeg, &path).unwrap();
                let decoded =
                    run_ffmpeg_decode_f32le(&ffmpeg, &path, pcm.sample_rate, pcm.channels).unwrap();
                let quality = validate_lossy_oracle_pcm_quality(&pcm.samples, &decoded).unwrap();
                let rms_ratio =
                    quality.decoded_rms / baseline_quality.decoded_rms.max(f64::EPSILON);
                eprintln!(
                    "AAC standard-id balanced sweep {label}: gain={global_gain}, bias={magnitude_bias}, rms_ratio={rms_ratio:.3}, quality={quality:?}, profile={profile:?}"
                );

                if quality.best_correlation < min_correlation
                    || quality.best_correlation + 0.10 < baseline_quality.best_correlation
                    || quality.decoded_rms < baseline_quality.decoded_rms * 0.35
                {
                    continue;
                }

                let candidate = (global_gain, magnitude_bias, quality, profile);
                best = match best {
                    Some(previous)
                        if (previous.2.decoded_rms - baseline_quality.decoded_rms).abs()
                            <= (candidate.2.decoded_rms - baseline_quality.decoded_rms).abs() =>
                    {
                        Some(previous)
                    }
                    _ => Some(candidate),
                };
            }

            let (global_gain, magnitude_bias, quality, profile) = best.unwrap_or_else(|| {
                panic!(
                    "{label} balanced gain/bias sweep found no quality-gated escape reduction: baseline_quality={baseline_quality:?}, baseline_breakdown={baseline_breakdown:?}"
                )
            });
            assert!(profile.max_abs < baseline_breakdown.max_abs);
            assert!(profile.escape_spectral_bits < baseline_breakdown.escape_spectral_bits);
            assert!(quality.decoded_rms >= baseline_quality.decoded_rms * 0.35);
            let expected_balanced_parameters =
                sonare_codec::aac_standard_id_selected_scale_factor_balanced_parameters(
                    pcm.channels,
                )
                .unwrap();
            assert_eq!(
                (global_gain, magnitude_bias, profile.max_quantized_abs_limit),
                expected_balanced_parameters
            );
            eprintln!(
                "AAC standard-id balanced gain/bias best {label}: gain={global_gain}, bias={magnitude_bias}, baseline_quality={baseline_quality:?}, best_quality={quality:?}, baseline_breakdown={baseline_breakdown:?}, best_profile={profile:?}"
            );
        }

        std::fs::remove_dir_all(&out_dir).unwrap();
    }

    #[test]
    fn aac_standard_id_quality_control_candidate_distribution_keeps_default_promotion_blocked_when_ffmpeg_is_available(
    ) {
        let Some(ffmpeg) = std::env::var_os("SONARE_FFMPEG") else {
            eprintln!(
                "skipping AAC quality-control candidate distribution: set SONARE_FFMPEG=/path/to/ffmpeg"
            );
            return;
        };
        let mono = readiness_pcm(44_100, 1).unwrap();
        let stereo = super::aac_standard_surface_stereo_pcm(&mono).unwrap();
        let out_dir = std::env::temp_dir().join(format!(
            "sonare-codec-aac-qc-candidate-distribution-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&out_dir).unwrap();

        for (label, pcm, bitrate) in [
            (
                "mono",
                mono,
                sonare_codec::aac_lc_default_production_bitrate_bps(1).unwrap(),
            ),
            (
                "stereo",
                stereo,
                sonare_codec::aac_lc_default_production_bitrate_bps(2).unwrap(),
            ),
        ] {
            let production_adts = sonare_codec::encode_with_mode(
                sonare_codec::Format::Aac,
                &pcm,
                sonare_codec::EncodeMode::ProductionOnly,
            )
            .unwrap();
            let production_path = out_dir.join(format!(
                "aac-qc-candidate-distribution-{label}-production.aac"
            ));
            std::fs::write(&production_path, production_adts).unwrap();
            run_ffmpeg_clean_acceptance(&ffmpeg, &production_path).unwrap();
            let production_decoded =
                run_ffmpeg_decode_f32le(&ffmpeg, &production_path, pcm.sample_rate, pcm.channels)
                    .unwrap();
            let production_quality =
                validate_lossy_oracle_pcm_quality(&pcm.samples, &production_decoded).unwrap();

            let balance_profile =
                sonare_codec::aac_standard_id_selected_scale_factor_balance_profile(pcm.channels)
                    .unwrap();
            let candidates =
                sonare_codec::aac_standard_id_quality_control_candidates_for_balance_profile_with_bitrate(
                    &pcm,
                    bitrate,
                )
                .unwrap();
            let mut results = Vec::new();
            for candidate in candidates {
                let adts = sonare_codec::encode_aac_adts_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_max_quantized_abs_and_bitrate(
                    &pcm,
                    bitrate,
                    candidate.global_gain,
                    candidate.scale_factor_magnitude_bias,
                    candidate.max_quantized_abs,
                )
                .unwrap();
                let path = out_dir.join(format!(
                    "aac-qc-candidate-distribution-{label}-gain-{}-bias-{}.aac",
                    candidate.global_gain, candidate.scale_factor_magnitude_bias
                ));
                std::fs::write(&path, adts).unwrap();
                run_ffmpeg_clean_acceptance(&ffmpeg, &path).unwrap();
                let decoded =
                    run_ffmpeg_decode_f32le(&ffmpeg, &path, pcm.sample_rate, pcm.channels).unwrap();
                let quality = validate_lossy_oracle_pcm_quality(&pcm.samples, &decoded).unwrap();
                let correlation_gap =
                    production_quality.best_correlation - quality.best_correlation;
                let rms_ratio =
                    quality.decoded_rms / production_quality.decoded_rms.max(f64::EPSILON);
                eprintln!(
                    "AAC QC candidate distribution {label}: gain={}, bias={}, rms_ratio={rms_ratio:.3}, correlation_gap={correlation_gap:.3}, quality={quality:?}, profile={:?}",
                    candidate.global_gain,
                    candidate.scale_factor_magnitude_bias,
                    candidate.profile
                );
                results.push((candidate, quality, correlation_gap, rms_ratio));
            }

            let selected = results
                .iter()
                .find(|(candidate, _, _, _)| {
                    candidate.global_gain == balance_profile.selected_global_gain
                        && candidate.scale_factor_magnitude_bias
                            == balance_profile.selected_magnitude_bias
                        && candidate.max_quantized_abs == balance_profile.max_quantized_abs
                })
                .copied()
                .unwrap_or_else(|| {
                    panic!(
                        "{label} QC distribution did not include selected balance profile: profile={balance_profile:?}, results={results:?}"
                    )
                });
            let best_correlation = results
                .iter()
                .copied()
                .max_by(|(_, left, _, _), (_, right, _, _)| {
                    left.best_correlation
                        .partial_cmp(&right.best_correlation)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .unwrap();
            let best_loudness = results
                .iter()
                .copied()
                .max_by(|(_, left, _, _), (_, right, _, _)| {
                    left.decoded_rms
                        .partial_cmp(&right.decoded_rms)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .unwrap();
            let closest_default_promotion = results
                .iter()
                .copied()
                .min_by(|(_, _, left_gap, left_rms), (_, _, right_gap, right_rms)| {
                    let left_score = left_gap.max(0.0) + (0.50 - left_rms).max(0.0);
                    let right_score = right_gap.max(0.0) + (0.50 - right_rms).max(0.0);
                    left_score
                        .partial_cmp(&right_score)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .unwrap();

            assert!(
                results.iter().all(|(_, _, correlation_gap, rms_ratio)| {
                    *correlation_gap > 0.09 || *rms_ratio < 0.50
                }),
                "{label} QC candidate distribution found a default-promotion candidate: production={production_quality:?}, results={results:?}"
            );
            assert!(
                best_correlation.2 > 0.09 || best_correlation.3 < 0.50,
                "{label} best-correlation QC candidate now meets default-promotion gates: best={best_correlation:?}, production={production_quality:?}"
            );
            assert!(
                best_loudness.2 > 0.09 || best_loudness.3 < 0.50,
                "{label} best-loudness QC candidate now meets default-promotion gates: best={best_loudness:?}, production={production_quality:?}"
            );
            assert!(
                selected.2 > 0.09 || selected.3 < 0.50,
                "{label} selected balanced QC candidate unexpectedly meets default-promotion gates: selected={selected:?}, production={production_quality:?}"
            );
            eprintln!(
                "AAC QC candidate distribution summary {label}: selected={selected:?}, best_correlation={best_correlation:?}, best_loudness={best_loudness:?}, closest_default_promotion={closest_default_promotion:?}, production={production_quality:?}"
            );
        }

        std::fs::remove_dir_all(&out_dir).unwrap();
    }

    #[test]
    fn mp3_perceptual_reservoir_production_gap_is_release_gated() {
        let reservoir = LossyOraclePcmQuality {
            decoded_rms: 0.9290,
            best_correlation: 0.572,
        };
        let production = LossyOraclePcmQuality {
            decoded_rms: 0.9290,
            best_correlation: 0.572,
        };
        validate_mp3_perceptual_reservoir_production_correlation_gap(
            "MP3 perceptual reservoir stereo",
            reservoir,
            production,
        )
        .unwrap();

        let regressed = LossyOraclePcmQuality {
            decoded_rms: 0.8403,
            best_correlation: 0.450,
        };
        let err = validate_mp3_perceptual_reservoir_production_correlation_gap(
            "MP3 perceptual reservoir stereo",
            regressed,
            production,
        )
        .unwrap_err();
        assert!(err.contains("correlation gap to production exceeded diagnostic limit"));
    }

    #[test]
    fn mp3_perceptual_diagnostic_reports_candidate_profile() {
        let pcm = readiness_pcm(44_100, 1).unwrap();
        let summary = super::mp3_perceptual_diagnostic_summary(
            &pcm,
            super::MP3_PERCEPTUAL_DIAGNOSTIC_STEP_CANDIDATES,
        )
        .unwrap();

        assert!(summary.contains("first_frame_candidate_profile=["));
        assert!(summary.contains("0.0005:2552b,0/42,max0"));
        assert!(summary.contains("first_nonzero_scale_factor_step=1"));
        assert!(summary.contains("1:43b,1/42,max2"));
    }

    #[test]
    fn aac_standard_candidate_tiebreak_prefers_expected_rms() {
        let selected = sonare_codec::AacPcmFrameStepSelection {
            step: 0.005,
            frame_len: 171,
            frame_capacity_bytes: 372,
        };
        let quiet = AacStandardDiagnosticCandidate {
            global_gain: 112,
            selected,
            encoded: Vec::new(),
            quality: LossyOraclePcmQuality {
                decoded_rms: 0.0107,
                best_correlation: 0.550,
            },
        };
        let matched = AacStandardDiagnosticCandidate {
            global_gain: 128,
            selected,
            encoded: Vec::new(),
            quality: LossyOraclePcmQuality {
                decoded_rms: 0.1709,
                best_correlation: 0.550,
            },
        };

        assert!(!aac_standard_candidate_is_at_least_as_good(
            &quiet, &matched, 0.1750
        ));
        assert!(aac_standard_candidate_is_at_least_as_good(
            &matched, &quiet, 0.1750
        ));
    }

    #[test]
    fn aac_selected_scale_factor_gain_sweep_prefers_rms_controlled_candidate() {
        let controlled = LossyOraclePcmQuality {
            decoded_rms: 0.2014,
            best_correlation: 0.548,
        };
        let over_amplified = LossyOraclePcmQuality {
            decoded_rms: 1.6111,
            best_correlation: 0.548,
        };

        assert!(!super::lossy_oracle_quality_is_at_least_as_good(
            &over_amplified,
            &controlled,
            0.1750
        ));
        assert!(super::lossy_oracle_quality_is_at_least_as_good(
            &controlled,
            &over_amplified,
            0.1750
        ));

        let stereo_controlled = LossyOraclePcmQuality {
            decoded_rms: 0.1030,
            best_correlation: 0.601,
        };
        let stereo_over_amplified = LossyOraclePcmQuality {
            decoded_rms: 1.6473,
            best_correlation: 0.601,
        };

        assert!(!super::lossy_oracle_quality_is_at_least_as_good(
            &stereo_over_amplified,
            &stereo_controlled,
            0.1468
        ));
        assert!(super::lossy_oracle_quality_is_at_least_as_good(
            &stereo_controlled,
            &stereo_over_amplified,
            0.1468
        ));
    }

    #[test]
    fn aac_selected_scale_factor_bias_sweep_keeps_fixed_like_candidates() {
        assert!(super::AAC_STANDARD_DIAGNOSTIC_GLOBAL_GAIN_CANDIDATES
            .contains(&super::AAC_STANDARD_DIAGNOSTIC_SELECTED_SURFACE_GLOBAL_GAIN));
        assert!(
            super::AAC_STANDARD_HIGH_LEVEL_SELECTED_SURFACE_GLOBAL_GAIN_CANDIDATES
                .contains(&super::AAC_STANDARD_DIAGNOSTIC_SELECTED_SURFACE_GLOBAL_GAIN)
        );
        assert!(
            super::AAC_STANDARD_HIGH_LEVEL_SELECTED_SURFACE_GLOBAL_GAIN_CANDIDATES.contains(&126)
        );
        assert!(
            super::AAC_STANDARD_HIGH_LEVEL_SELECTED_SURFACE_GLOBAL_GAIN_CANDIDATES.contains(&130)
        );
        assert!(
            super::AAC_STANDARD_HIGH_LEVEL_FIXED_SURFACE_GLOBAL_GAIN_CANDIDATES
                .contains(&super::AAC_STANDARD_DIAGNOSTIC_SELECTED_SURFACE_GLOBAL_GAIN)
        );
        assert!(
            super::AAC_STANDARD_HIGH_LEVEL_SELECTED_SURFACE_MAGNITUDE_BIAS_CANDIDATES
                .contains(&super::AAC_STANDARD_DIAGNOSTIC_SELECTED_SURFACE_MAGNITUDE_BIAS)
        );
        assert!(
            super::AAC_STANDARD_HIGH_LEVEL_SELECTED_SURFACE_MAGNITUDE_BIAS_CANDIDATES.contains(&12)
        );
        assert!(
            !super::AAC_STANDARD_HIGH_LEVEL_SELECTED_SURFACE_MAGNITUDE_BIAS_CANDIDATES.contains(&0)
        );
        assert!(
            !super::AAC_STANDARD_HIGH_LEVEL_SELECTED_SURFACE_MAGNITUDE_BIAS_CANDIDATES
                .contains(&20)
        );

        let low_bias_mono = LossyOraclePcmQuality {
            decoded_rms: 0.1693,
            best_correlation: 0.548,
        };
        let fixed_like_mono = LossyOraclePcmQuality {
            decoded_rms: 0.1709,
            best_correlation: 0.550,
        };
        assert!(super::lossy_oracle_quality_is_at_least_as_good(
            &fixed_like_mono,
            &low_bias_mono,
            0.1750
        ));
        assert!(!super::lossy_oracle_quality_is_at_least_as_good(
            &low_bias_mono,
            &fixed_like_mono,
            0.1750
        ));

        let low_bias_stereo = LossyOraclePcmQuality {
            decoded_rms: 0.2059,
            best_correlation: 0.602,
        };
        let fixed_like_stereo = LossyOraclePcmQuality {
            decoded_rms: 0.1743,
            best_correlation: 0.607,
        };
        assert!(super::lossy_oracle_quality_is_at_least_as_good(
            &fixed_like_stereo,
            &low_bias_stereo,
            0.1468
        ));
        assert!(!super::lossy_oracle_quality_is_at_least_as_good(
            &low_bias_stereo,
            &fixed_like_stereo,
            0.1468
        ));
    }

    #[test]
    fn lossy_oracle_quality_rejects_silent_pcm() {
        let expected = (0..256)
            .map(|sample| ((sample as f32) * 0.05).sin() * 0.25)
            .collect::<Vec<_>>();
        let err = validate_lossy_oracle_pcm_quality(&expected, &[0.0; 256]).unwrap_err();
        assert!(err.contains("effectively silent"));
    }

    #[test]
    fn lossy_oracle_quality_rejects_excessively_amplified_pcm() {
        let expected = (0..256)
            .map(|sample| ((sample as f32) * 0.05).sin() * 0.25)
            .collect::<Vec<_>>();
        let decoded = expected
            .iter()
            .map(|sample| sample * 64.0)
            .collect::<Vec<_>>();

        let err = validate_lossy_oracle_pcm_quality(&expected, &decoded).unwrap_err();
        assert!(err.contains("excessively amplified"));
    }

    #[test]
    fn lossy_oracle_quality_rejects_uncorrelated_pcm() {
        let expected = (0..256)
            .map(|sample| ((sample as f32) * 0.05).sin() * 0.25)
            .collect::<Vec<_>>();
        let decoded = (0..256)
            .map(|sample| ((sample as f32) * 0.31).cos() * 0.25)
            .collect::<Vec<_>>();

        let err = validate_lossy_oracle_pcm_quality(&expected, &decoded).unwrap_err();
        assert!(err.contains("does not correlate"));
    }

    #[test]
    fn diagnostic_quality_floor_rejects_known_regressions() {
        let passing = LossyOraclePcmQuality {
            decoded_rms: 0.1460,
            best_correlation: 0.384,
        };
        validate_diagnostic_quality_floor("MP3 diagnostic", passing, 0.10, 0.30).unwrap();

        let quiet = LossyOraclePcmQuality {
            decoded_rms: 0.0107,
            best_correlation: 0.550,
        };
        let err =
            validate_diagnostic_quality_floor("AAC diagnostic", quiet, 0.10, 0.50).unwrap_err();
        assert!(err.contains("decoded RMS regressed"));

        let decorrelated = LossyOraclePcmQuality {
            decoded_rms: 0.1709,
            best_correlation: 0.016,
        };
        let err = validate_diagnostic_quality_floor("MP3 diagnostic", decorrelated, 0.10, 0.30)
            .unwrap_err();
        assert!(err.contains("correlation regressed"));
    }

    #[test]
    fn adts_frame_budget_rejects_oversized_diagnostic_frame() {
        validate_adts_frame_budget("AAC diagnostic", 171, 372, 128_000).unwrap();

        let err = validate_adts_frame_budget("AAC diagnostic", 373, 372, 128_000).unwrap_err();
        assert!(err.contains("ADTS frame budget failed"));
    }

    #[test]
    fn aac_standard_id_mixed_workbench_is_publish_readiness_gated() {
        validate_aac_standard_id_mixed_workbench().unwrap();
    }

    #[test]
    fn correlation_search_handles_decoder_delay() {
        let expected = (0..128)
            .map(|sample| ((sample as f32) * 0.1).sin())
            .collect::<Vec<_>>();
        let mut decoded = vec![0.0; 64];
        decoded.extend_from_slice(&expected);

        assert!(best_normalized_correlation(&expected, &decoded).unwrap() > 0.99);
    }

    #[test]
    fn compatibility_lossy_scaffolds_are_not_publish_ready_when_ffmpeg_is_available() {
        let Some(ffmpeg) = std::env::var_os("SONARE_FFMPEG") else {
            return;
        };
        let samples = (0..2304)
            .map(|sample| ((sample as f32) * 0.01).sin() * 0.25)
            .collect::<Vec<_>>();
        let pcm = sonare_codec::AudioBuffer::new(44_100, 1, samples).unwrap();

        let diagnostics = compatibility_lossy_encode_diagnostics(&ffmpeg, &pcm).unwrap();

        assert_eq!(diagnostics.len(), 7);
        assert!(
            diagnostics.iter().any(|diagnostic| diagnostic
                .contains("MP3 compatibility scaffold passes current oracle")
                || diagnostic.contains("MP3 compatibility scaffold cannot be promoted")),
            "{diagnostics:?}"
        );
        assert!(
            diagnostics.iter().any(|diagnostic| diagnostic
                .contains("AAC-LC compatibility scaffold passes current oracle")),
            "{diagnostics:?}"
        );
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.contains("MP3 standard-table scaffold")),
            "{diagnostics:?}"
        );
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.contains("MP3 perceptual-scale-factor scaffold")),
            "{diagnostics:?}"
        );
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.contains("MP3 perceptual reservoir scaffold")),
            "{diagnostics:?}"
        );
        assert!(
            diagnostics.iter().any(|diagnostic| diagnostic
                .contains("AAC-LC experimental nonzero scaffold is still not production-gated")),
            "{diagnostics:?}"
        );
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.contains("AAC-LC standard-table scaffold")),
            "{diagnostics:?}"
        );
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.contains("best_correlation")),
            "{diagnostics:?}"
        );
    }

    #[test]
    fn diagnostic_lossy_readiness_passes_when_ffmpeg_is_available() {
        if std::env::var_os("SONARE_FFMPEG").is_none() {
            return;
        }

        verify_diagnostic_lossy_encode_readiness().unwrap();
    }

    #[test]
    fn mp3_stereo_production_artifact_passes_oracle_when_ffmpeg_is_available() {
        let Some(ffmpeg) = std::env::var_os("SONARE_FFMPEG") else {
            return;
        };
        let pcm = readiness_pcm(32_000, 2).unwrap();
        let encoded = sonare_codec::encode_with_mode(
            sonare_codec::Format::Mp3,
            &pcm,
            sonare_codec::EncodeMode::ProductionOnly,
        )
        .unwrap();
        let artifacts = [(
            "MP3 32kHz stereo",
            ProductionArtifactKind::Mp3,
            pcm,
            encoded,
        )];

        verify_production_lossy_oracle_acceptance(ffmpeg, &artifacts).unwrap();
    }

    #[test]
    fn mp3_stereo_perceptual_reservoir_candidate_catches_up_with_production_when_ffmpeg_is_available(
    ) {
        let Some(ffmpeg) = std::env::var_os("SONARE_FFMPEG") else {
            return;
        };
        let pcm = readiness_pcm(44_100, 2).unwrap();
        let out_dir = std::env::temp_dir().join(format!(
            "sonare-codec-stereo-perceptual-reservoir-diagnostic-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&out_dir).unwrap();

        let calibrated_details =
            sonare_codec::select_mpeg1_layer3_reservoir_frame_details_with_table_provider(
                &pcm,
                sonare_codec::MPEG1_LAYER3_PCM_STEP_CANDIDATES,
                128,
                false,
                sonare_codec::mpeg1_layer3_standard_table_provider(),
            )
            .unwrap();
        let perceptual_details =
            sonare_codec::select_mpeg1_layer3_perceptual_reservoir_frame_details_with_table_provider(
                &pcm,
                super::MP3_PERCEPTUAL_DIAGNOSTIC_STEP_CANDIDATES,
                128,
                false,
                sonare_codec::mpeg1_layer3_standard_table_provider(),
            )
            .unwrap();
        let guarded_details = sonare_codec::select_mpeg1_layer3_quality_guarded_perceptual_reservoir_frame_details_with_table_provider(
            &pcm,
            super::MP3_PERCEPTUAL_DIAGNOSTIC_STEP_CANDIDATES,
            128,
            false,
            sonare_codec::mpeg1_layer3_standard_table_provider(),
        )
        .unwrap();
        assert!(calibrated_details
            .iter()
            .all(|detail| { detail.perceptual_granules == 0 && detail.calibrated_granules == 4 }));
        assert!(perceptual_details
            .iter()
            .all(|detail| { detail.perceptual_granules == 4 && detail.calibrated_granules == 0 }));
        assert!(guarded_details
            .iter()
            .all(|detail| { detail.perceptual_granules + detail.calibrated_granules == 4 }));
        assert!(guarded_details
            .iter()
            .all(|detail| { detail.quality_guard_compared_granules == 4 }));
        assert!(
            guarded_details
                .iter()
                .all(|detail| detail.quality_guard_distortion_delta.is_finite()),
            "quality guard reported a non-finite encoder-side distortion delta"
        );
        let calibrated_max_payload = calibrated_details
            .iter()
            .map(|detail| detail.payload_bit_len)
            .max()
            .unwrap_or(0);
        let perceptual_max_payload = perceptual_details
            .iter()
            .map(|detail| detail.payload_bit_len)
            .max()
            .unwrap_or(0);
        let perceptual_min_step = perceptual_details
            .iter()
            .map(|detail| detail.step)
            .fold(f32::INFINITY, f32::min);
        let guarded_max_payload = guarded_details
            .iter()
            .map(|detail| detail.payload_bit_len)
            .max()
            .unwrap_or(0);

        let candidate_quality =
            super::mp3_perceptual_reservoir_nonzero_encode_diagnostic(&ffmpeg, &pcm, &out_dir)
                .unwrap();
        let guarded =
            sonare_codec::encode_mpeg1_layer3_pcm_frames_with_quality_guarded_perceptual_reservoir_and_table_provider(
                &pcm,
                super::MP3_PERCEPTUAL_DIAGNOSTIC_STEP_CANDIDATES,
                128,
                false,
                sonare_codec::mpeg1_layer3_standard_table_provider(),
            )
            .unwrap();
        let guarded_path = out_dir.join("mp3-stereo-guarded-perceptual-reservoir.mp3");
        std::fs::write(&guarded_path, guarded).unwrap();
        super::run_ffmpeg_acceptance(&ffmpeg, &guarded_path).unwrap();
        let guarded_decoded =
            super::run_ffmpeg_decode_f32le(&ffmpeg, &guarded_path, pcm.sample_rate, pcm.channels)
                .unwrap();
        let guarded_quality =
            super::validate_lossy_oracle_pcm_quality(&pcm.samples, &guarded_decoded).unwrap();
        let production = sonare_codec::encode_with_mode(
            sonare_codec::Format::Mp3,
            &pcm,
            sonare_codec::EncodeMode::ProductionOnly,
        )
        .unwrap();
        let production_path = out_dir.join("mp3-stereo-production.mp3");
        std::fs::write(&production_path, production).unwrap();
        super::run_ffmpeg_acceptance(&ffmpeg, &production_path).unwrap();
        let production_decoded = super::run_ffmpeg_decode_f32le(
            &ffmpeg,
            &production_path,
            pcm.sample_rate,
            pcm.channels,
        )
        .unwrap();
        let production_quality =
            super::validate_lossy_oracle_pcm_quality(&pcm.samples, &production_decoded).unwrap();
        std::fs::remove_dir_all(&out_dir).unwrap();

        assert!(
            candidate_quality.best_correlation >= 0.49,
            "stereo perceptual reservoir should pass the tightened basic oracle before production re-evaluation: {candidate_quality:?}"
        );
        assert!(
            perceptual_details
                .iter()
                .any(|detail| detail.main_data_begin > 0),
            "stereo perceptual reservoir should exercise reservoir borrowing"
        );
        assert!(
            perceptual_max_payload <= calibrated_max_payload,
            "stereo perceptual reservoir is not being held back by payload size: perceptual={perceptual_max_payload}, calibrated={calibrated_max_payload}"
        );
        assert!(
            guarded_details
                .iter()
                .any(|detail| detail.main_data_begin > 0),
            "quality-guarded stereo perceptual reservoir should exercise reservoir borrowing"
        );
        assert!(
            guarded_max_payload <= calibrated_max_payload,
            "quality-guarded stereo perceptual reservoir should stay within the calibrated payload envelope: guarded={guarded_max_payload}, calibrated={calibrated_max_payload}"
        );
        assert!(
            perceptual_min_step <= 1.0,
            "stereo perceptual reservoir did not select an active fine step: min_step={perceptual_min_step}"
        );
        assert!(
            guarded_quality.best_correlation + 0.01 >= production_quality.best_correlation,
            "quality-guarded stereo perceptual reservoir regressed production quality: guarded={guarded_quality:?}, production={production_quality:?}"
        );
        assert!(
            candidate_quality.best_correlation + 0.001 >= production_quality.best_correlation,
            "stereo perceptual reservoir should now match the production bridge: candidate={candidate_quality:?}, production={production_quality:?}"
        );
        assert!(
            production_quality.best_correlation + 0.001 >= candidate_quality.best_correlation,
            "stereo perceptual reservoir unexpectedly exceeded the production bridge enough to require floor re-tuning: candidate={candidate_quality:?}, production={production_quality:?}"
        );
    }

