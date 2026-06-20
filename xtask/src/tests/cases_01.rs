    use super::{
        aac_standard_candidate_is_at_least_as_good, best_normalized_correlation,
        best_normalized_correlation_with_offset, compatibility_lossy_encode_diagnostics,
        mp3_perceptual_bit_allocation_targets_by_frame, production_lossy_min_correlation,
        readiness_pcm, required_qa_tool_in_list, rms, run_ffmpeg_acceptance,
        run_ffmpeg_clean_acceptance, run_ffmpeg_decode_f32le,
        validate_aac_standard_id_mixed_workbench,
        validate_aac_standard_id_production_correlation_gap, validate_adts_frame_budget,
        validate_diagnostic_quality_floor, validate_lossy_oracle_pcm_quality,
        validate_mp3_perceptual_reservoir_production_correlation_gap,
        verify_aac_default_production_budget, verify_diagnostic_lossy_encode_readiness,
        verify_mp3_default_production_budget, verify_mp3_production_reservoir,
        verify_production_lossy_oracle_acceptance, AacStandardDiagnosticCandidate,
        LossyOraclePcmQuality, ProductionArtifactKind, AAC_PRODUCTION_MIN_CORRELATION,
        MP3_PRODUCTION_MONO_MIN_CORRELATION, MP3_PRODUCTION_STEREO_MIN_CORRELATION,
    };
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn parses_required_qa_tool_list() {
        assert!(required_qa_tool_in_list(
            "nextest,audit machete;semver-checks",
            "nextest"
        ));
        assert!(required_qa_tool_in_list(
            "nextest,audit machete;semver-checks",
            "audit"
        ));
        assert!(required_qa_tool_in_list(
            "nextest,audit machete;semver-checks",
            "machete"
        ));
        assert!(required_qa_tool_in_list(
            "nextest,audit machete;semver-checks",
            "semver-checks"
        ));
        assert!(!required_qa_tool_in_list(
            "nextest,audit machete;semver-checks",
            "miri"
        ));
    }

    #[test]
    fn required_qa_tool_all_matches_every_tool() {
        assert!(required_qa_tool_in_list("all", "nextest"));
        assert!(required_qa_tool_in_list("nextest,all", "llvm-cov"));
    }

    #[test]
    fn lossy_oracle_quality_allows_delayed_correlated_pcm() {
        let expected = (0..256)
            .map(|sample| ((sample as f32) * 0.05).sin() * 0.25)
            .collect::<Vec<_>>();
        let mut decoded = vec![0.0; 31];
        decoded.extend(expected.iter().map(|sample| sample * 0.9));
        decoded.extend([0.0; 17]);

        let quality = validate_lossy_oracle_pcm_quality(&expected, &decoded).unwrap();
        assert!(quality.decoded_rms > 0.0);
        assert!(quality.best_correlation > 0.99);
    }

    #[test]
    fn production_lossy_min_correlation_matches_release_floors() {
        assert_eq!(
            production_lossy_min_correlation(ProductionArtifactKind::Mp3, 1).unwrap(),
            MP3_PRODUCTION_MONO_MIN_CORRELATION
        );
        assert_eq!(
            production_lossy_min_correlation(ProductionArtifactKind::Mp3, 2).unwrap(),
            MP3_PRODUCTION_STEREO_MIN_CORRELATION
        );
        assert_eq!(
            production_lossy_min_correlation(ProductionArtifactKind::Aac, 1).unwrap(),
            AAC_PRODUCTION_MIN_CORRELATION
        );
        assert_eq!(
            production_lossy_min_correlation(ProductionArtifactKind::M4a, 2).unwrap(),
            AAC_PRODUCTION_MIN_CORRELATION
        );

        let err = production_lossy_min_correlation(ProductionArtifactKind::Mp3, 3).unwrap_err();
        assert!(err.contains("mono/stereo only"));
    }

    #[test]
    fn aac_standard_id_production_gap_is_release_gated() {
        let standard_id = LossyOraclePcmQuality {
            decoded_rms: 0.1709,
            best_correlation: 0.550,
        };
        let production = LossyOraclePcmQuality {
            decoded_rms: 0.7004,
            best_correlation: 0.762,
        };
        validate_aac_standard_id_production_correlation_gap(
            "AAC standard-id mono",
            standard_id,
            production,
        )
        .unwrap();
        validate_aac_standard_id_production_correlation_gap(
            "AAC balanced standard-id mono",
            LossyOraclePcmQuality {
                decoded_rms: 0.1901,
                best_correlation: 0.553,
            },
            production,
        )
        .unwrap();

        let regressed = LossyOraclePcmQuality {
            decoded_rms: 0.1709,
            best_correlation: 0.490,
        };
        let err = validate_aac_standard_id_production_correlation_gap(
            "AAC standard-id mono",
            regressed,
            production,
        )
        .unwrap_err();
        assert!(err.contains("correlation gap to production exceeded diagnostic limit"));
        let err = validate_aac_standard_id_production_correlation_gap(
            "AAC balanced standard-id mono",
            regressed,
            production,
        )
        .unwrap_err();
        assert!(err.contains("AAC balanced standard-id mono"));
    }

    #[test]
    fn aac_standard_id_rms_control_advantage_is_release_gated() {
        let standard_id = LossyOraclePcmQuality {
            decoded_rms: 0.1709,
            best_correlation: 0.550,
        };
        let production = LossyOraclePcmQuality {
            decoded_rms: 0.7004,
            best_correlation: 0.762,
        };
        super::validate_aac_standard_id_rms_control_advantage(
            "AAC standard-id mono",
            standard_id,
            production,
            0.1750,
        )
        .unwrap();

        let regressed = LossyOraclePcmQuality {
            decoded_rms: 0.9100,
            best_correlation: 0.570,
        };
        let err = super::validate_aac_standard_id_rms_control_advantage(
            "AAC standard-id mono",
            regressed,
            production,
            0.1750,
        )
        .unwrap_err();
        assert!(err.contains("RMS control regressed behind production"));
    }

    #[test]
    fn aac_standard_id_frame_selection_comparison_reports_budget_deltas() {
        let production = [
            sonare_codec::AacPcmFrameStepSelection {
                step: 0.2,
                frame_len: 300,
                frame_capacity_bytes: 372,
            },
            sonare_codec::AacPcmFrameStepSelection {
                step: 0.1,
                frame_len: 240,
                frame_capacity_bytes: 372,
            },
        ];
        let standard_id = [
            sonare_codec::AacPcmFrameStepSelection {
                step: 0.15,
                frame_len: 280,
                frame_capacity_bytes: 372,
            },
            sonare_codec::AacPcmFrameStepSelection {
                step: 0.075,
                frame_len: 260,
                frame_capacity_bytes: 372,
            },
        ];

        let comparison =
            super::compare_aac_frame_selection_details(&production, &standard_id).unwrap();

        assert_eq!(comparison.frames, 2);
        assert_eq!(comparison.production_max_frame_len, 300);
        assert_eq!(comparison.standard_id_max_frame_len, 280);
        assert_eq!(comparison.max_frame_len_delta, -20);
        assert_eq!(comparison.production_min_budget_slack, 72);
        assert_eq!(comparison.standard_id_min_budget_slack, 92);
        assert_eq!(comparison.min_budget_slack_delta, 20);
        assert!((comparison.max_step_delta + 0.05).abs() < 1.0e-6);
    }

    #[test]
    fn aac_standard_id_frame_selection_comparison_rejects_shape_mismatch() {
        let production = [sonare_codec::AacPcmFrameStepSelection {
            step: 0.2,
            frame_len: 300,
            frame_capacity_bytes: 372,
        }];
        let standard_id = [
            sonare_codec::AacPcmFrameStepSelection {
                step: 0.15,
                frame_len: 280,
                frame_capacity_bytes: 372,
            },
            sonare_codec::AacPcmFrameStepSelection {
                step: 0.075,
                frame_len: 260,
                frame_capacity_bytes: 372,
            },
        ];

        let err =
            super::compare_aac_frame_selection_details(&production, &standard_id).unwrap_err();

        assert!(err.contains("frame count diverged"));
    }

    #[test]
    fn aac_standard_id_candidate_set_comparison_tracks_promotion_blocker() {
        let mono = readiness_pcm(44_100, 1).unwrap();
        let stereo = super::aac_standard_surface_stereo_pcm(&mono).unwrap();

        let mono_recommended =
            super::compare_aac_standard_id_to_production_frame_selection(&mono).unwrap();
        let mono_production_step =
            super::compare_aac_standard_id_candidate_set_to_production_frame_selection(
                &mono,
                sonare_codec::AAC_LC_PCM_STEP_CANDIDATES,
            )
            .unwrap();
        let stereo_recommended =
            super::compare_aac_standard_id_to_production_frame_selection(&stereo).unwrap();
        let stereo_production_step =
            super::compare_aac_standard_id_candidate_set_to_production_frame_selection(
                &stereo,
                sonare_codec::AAC_LC_PCM_STEP_CANDIDATES,
            )
            .unwrap();

        eprintln!(
            "AAC standard-id candidate-set blocker: mono recommended={mono_recommended:?}, mono production-step={mono_production_step:?}, stereo recommended={stereo_recommended:?}, stereo production-step={stereo_production_step:?}"
        );
        assert!(mono_recommended.max_frame_len_delta > 0);
        assert!(stereo_recommended.max_frame_len_delta > 0);
        assert!(mono_production_step.max_frame_len_delta <= mono_recommended.max_frame_len_delta);
        assert!(
            stereo_production_step.max_frame_len_delta <= stereo_recommended.max_frame_len_delta
        );
    }

    #[test]
    fn aac_standard_id_scale_factor_profile_tracks_balanced_production_gap() {
        let mono = readiness_pcm(44_100, 1).unwrap();
        let stereo = super::aac_standard_surface_stereo_pcm(&mono).unwrap();

        for (label, pcm) in [("mono", mono), ("stereo", stereo)] {
            let bitrate = sonare_codec::aac_lc_default_production_bitrate_bps(
                u8::try_from(pcm.channels).unwrap(),
            )
            .unwrap();
            let production_details =
                sonare_codec::aac_selected_scale_factor_frame_details_with_bitrate(&pcm, bitrate)
                    .unwrap();
            let balanced_details =
                sonare_codec::aac_balanced_standard_selected_scale_factor_frame_details_with_bitrate(
                    &pcm,
                    bitrate,
                )
                .unwrap();
            let production_profile = super::aac_selected_scale_factor_profile_for_frame_selection(
                &pcm,
                &production_details,
                180,
                0,
            )
            .unwrap();
            let (balanced_global_gain, balanced_magnitude_bias, _) =
                sonare_codec::aac_standard_id_selected_scale_factor_balanced_parameters(
                    pcm.channels,
                )
                .unwrap();
            let balanced_profile = super::aac_selected_scale_factor_profile_for_frame_selection(
                &pcm,
                &balanced_details,
                balanced_global_gain,
                balanced_magnitude_bias,
            )
            .unwrap();

            eprintln!(
                "AAC standard-id scale-factor profile {label}: production={production_profile:?}, balanced={balanced_profile:?}"
            );
            assert_eq!(production_profile.frames, balanced_profile.frames);
            assert_eq!(production_profile.channels, balanced_profile.channels);
            assert_eq!(production_profile.bands, balanced_profile.bands);
            assert!(production_profile.raised_bands > 0);
            assert!(balanced_profile.raised_bands > 0);
            assert!(
                production_profile.mean_delta > balanced_profile.mean_delta,
                "{label} balanced profile should expose reduced scale-factor pressure: production={production_profile:?}, balanced={balanced_profile:?}"
            );
        }
    }

    #[test]
    fn aac_standard_id_scale_factor_pressure_recovery_sweep_keeps_default_promotion_blocked_when_ffmpeg_is_available(
    ) {
        let Some(ffmpeg) = std::env::var_os("SONARE_FFMPEG") else {
            eprintln!("skipping AAC scale-factor pressure recovery sweep: set SONARE_FFMPEG=/path/to/ffmpeg");
            return;
        };
        let mono = readiness_pcm(44_100, 1).unwrap();
        let stereo = super::aac_standard_surface_stereo_pcm(&mono).unwrap();
        let out_dir = std::env::temp_dir().join(format!(
            "sonare-codec-aac-scale-factor-pressure-recovery-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&out_dir).unwrap();

        let cases: [(
            &str,
            sonare_codec::AudioBuffer,
            &[super::AacScaleFactorPressureRecoveryCandidate],
        ); 2] = [
            (
                "mono",
                mono,
                &[
                    super::AacScaleFactorPressureRecoveryCandidate {
                        restored_bias: 6,
                        restored_bands_per_channel: 4,
                    },
                    super::AacScaleFactorPressureRecoveryCandidate {
                        restored_bias: 4,
                        restored_bands_per_channel: 8,
                    },
                    super::AacScaleFactorPressureRecoveryCandidate {
                        restored_bias: 2,
                        restored_bands_per_channel: 12,
                    },
                    super::AacScaleFactorPressureRecoveryCandidate {
                        restored_bias: 0,
                        restored_bands_per_channel: 16,
                    },
                ],
            ),
            (
                "stereo",
                stereo,
                &[
                    super::AacScaleFactorPressureRecoveryCandidate {
                        restored_bias: 3,
                        restored_bands_per_channel: 4,
                    },
                    super::AacScaleFactorPressureRecoveryCandidate {
                        restored_bias: 2,
                        restored_bands_per_channel: 8,
                    },
                    super::AacScaleFactorPressureRecoveryCandidate {
                        restored_bias: 1,
                        restored_bands_per_channel: 12,
                    },
                    super::AacScaleFactorPressureRecoveryCandidate {
                        restored_bias: 0,
                        restored_bands_per_channel: 16,
                    },
                ],
            ),
        ];

        for (label, pcm, candidates) in cases {
            let bitrate = sonare_codec::aac_lc_default_production_bitrate_bps(
                u8::try_from(pcm.channels).unwrap(),
            )
            .unwrap();
            let production_adts = sonare_codec::encode_with_mode(
                sonare_codec::Format::Aac,
                &pcm,
                sonare_codec::EncodeMode::ProductionOnly,
            )
            .unwrap();
            let production_path =
                out_dir.join(format!("aac-scale-factor-pressure-{label}-production.aac"));
            std::fs::write(&production_path, production_adts).unwrap();
            run_ffmpeg_clean_acceptance(&ffmpeg, &production_path).unwrap();
            let production_decoded =
                run_ffmpeg_decode_f32le(&ffmpeg, &production_path, pcm.sample_rate, pcm.channels)
                    .unwrap();
            let production_quality =
                validate_lossy_oracle_pcm_quality(&pcm.samples, &production_decoded).unwrap();

            let (balanced_global_gain, balanced_magnitude_bias, balanced_max_quantized_abs) =
                sonare_codec::aac_standard_id_selected_scale_factor_balanced_parameters(
                    pcm.channels,
                )
                .unwrap();
            let balanced_details =
                sonare_codec::aac_standard_selected_scale_factor_frame_details_with_magnitude_bias_max_quantized_abs_and_bitrate(
                    &pcm,
                    bitrate,
                    balanced_global_gain,
                    balanced_magnitude_bias,
                    balanced_max_quantized_abs,
                )
                .unwrap();
            let balanced_profile = super::aac_selected_scale_factor_profile_for_frame_selection(
                &pcm,
                &balanced_details,
                balanced_global_gain,
                balanced_magnitude_bias,
            )
            .unwrap();
            let balanced_breakdown = super::aac_standard_id_payload_breakdown_for_frame_selection(
                &pcm,
                &balanced_details,
            )
            .unwrap();

            let mut recoveries = Vec::new();
            for candidate in candidates {
                let (adts, profile) =
                    super::encode_aac_standard_id_pressure_recovered_stream_for_frame_selection(
                        &pcm,
                        &balanced_details,
                        balanced_global_gain,
                        balanced_magnitude_bias,
                        *candidate,
                    )
                    .unwrap();
                let path = out_dir.join(format!(
                    "aac-scale-factor-pressure-{label}-bias-{}-bands-{}.aac",
                    candidate.restored_bias, candidate.restored_bands_per_channel
                ));
                std::fs::write(&path, adts).unwrap();
                run_ffmpeg_clean_acceptance(&ffmpeg, &path).unwrap();
                let decoded =
                    run_ffmpeg_decode_f32le(&ffmpeg, &path, pcm.sample_rate, pcm.channels).unwrap();
                let quality = match validate_lossy_oracle_pcm_quality(&pcm.samples, &decoded) {
                    Ok(quality) => quality,
                    Err(err) => {
                        eprintln!(
                            "AAC scale-factor pressure recovery {label}: candidate={candidate:?}, quality rejected: {err}, profile={profile:?}"
                        );
                        continue;
                    }
                };
                let correlation_gap =
                    production_quality.best_correlation - quality.best_correlation;
                let rms_ratio =
                    quality.decoded_rms / production_quality.decoded_rms.max(f64::EPSILON);
                eprintln!(
                    "AAC scale-factor pressure recovery {label}: candidate={candidate:?}, correlation_gap={correlation_gap:.3}, rms_ratio={rms_ratio:.3}, profile={profile:?}, quality={quality:?}, balanced_breakdown={balanced_breakdown:?}"
                );
                recoveries.push(super::AacScaleFactorPressureRecovery {
                    candidate: *candidate,
                    profile,
                    quality,
                });
            }

            assert!(
                recoveries.iter().all(|recovery| {
                    recovery.profile.mean_delta > balanced_profile.mean_delta
                        && recovery.profile.raised_bands >= balanced_profile.raised_bands
                }),
                "{label} pressure recovery sweep did not increase scale-factor pressure: balanced={balanced_profile:?}, recoveries={recoveries:?}"
            );
            let promotable = recoveries
                .iter()
                .filter(|recovery| {
                    production_quality.best_correlation - recovery.quality.best_correlation <= 0.09
                        && recovery.quality.decoded_rms
                            / production_quality.decoded_rms.max(f64::EPSILON)
                            >= 0.50
                })
                .collect::<Vec<_>>();
            assert!(
                promotable.is_empty(),
                "{label} scale-factor pressure recovery found a default-promotion candidate: promotable={promotable:?}, production={production_quality:?}, balanced_profile={balanced_profile:?}, balanced_breakdown={balanced_breakdown:?}"
            );
        }

        std::fs::remove_dir_all(&out_dir).unwrap();
    }

    #[test]
    fn aac_standard_id_quantizer_step_sweep_tracks_max_abs_quality_tradeoff_when_ffmpeg_is_available(
    ) {
        let Some(ffmpeg) = std::env::var_os("SONARE_FFMPEG") else {
            eprintln!("skipping AAC quantizer step sweep: set SONARE_FFMPEG=/path/to/ffmpeg");
            return;
        };
        let mono = readiness_pcm(44_100, 1).unwrap();
        let stereo = super::aac_standard_surface_stereo_pcm(&mono).unwrap();
        let out_dir = std::env::temp_dir().join(format!(
            "sonare-codec-aac-quantizer-step-sweep-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&out_dir).unwrap();

        for (label, pcm) in [("mono", mono), ("stereo", stereo)] {
            let bitrate = sonare_codec::aac_lc_default_production_bitrate_bps(
                u8::try_from(pcm.channels).unwrap(),
            )
            .unwrap();
            let frame_budget =
                sonare_codec::aac_lc_adts_max_frame_len_for_bitrate(pcm.sample_rate, bitrate)
                    .unwrap();
            let production_adts = sonare_codec::encode_with_mode(
                sonare_codec::Format::Aac,
                &pcm,
                sonare_codec::EncodeMode::ProductionOnly,
            )
            .unwrap();
            let production_path =
                out_dir.join(format!("aac-quantizer-step-{label}-production.aac"));
            std::fs::write(&production_path, production_adts).unwrap();
            run_ffmpeg_clean_acceptance(&ffmpeg, &production_path).unwrap();
            let production_decoded =
                run_ffmpeg_decode_f32le(&ffmpeg, &production_path, pcm.sample_rate, pcm.channels)
                    .unwrap();
            let production_quality =
                validate_lossy_oracle_pcm_quality(&pcm.samples, &production_decoded).unwrap();

            let (balanced_global_gain, balanced_magnitude_bias, balanced_max_quantized_abs) =
                sonare_codec::aac_standard_id_selected_scale_factor_balanced_parameters(
                    pcm.channels,
                )
                .unwrap();
            let balanced_details =
                sonare_codec::aac_standard_selected_scale_factor_frame_details_with_magnitude_bias_max_quantized_abs_and_bitrate(
                    &pcm,
                    bitrate,
                    balanced_global_gain,
                    balanced_magnitude_bias,
                    balanced_max_quantized_abs,
                )
                .unwrap();
            let mut sweep_results = Vec::new();
            for step_scale in [0.95_f32, 0.90, 0.80, 0.70, 0.60, 0.50] {
                let scaled_details =
                    super::aac_scaled_frame_selection_steps(&balanced_details, step_scale).unwrap();
                let max_quantized_abs =
                    super::aac_max_quantized_abs_for_frame_selection(&pcm, &scaled_details)
                        .unwrap();
                let (adts, profile) =
                    super::encode_aac_standard_id_pressure_recovered_stream_for_frame_selection(
                        &pcm,
                        &scaled_details,
                        balanced_global_gain,
                        balanced_magnitude_bias,
                        super::AacScaleFactorPressureRecoveryCandidate {
                            restored_bias: balanced_magnitude_bias,
                            restored_bands_per_channel: 0,
                        },
                    )
                    .unwrap();
                let max_frame_len = super::max_adts_frame_len(&adts).unwrap();
                let path = out_dir.join(format!(
                    "aac-quantizer-step-{label}-scale-{step_scale:.2}.aac"
                ));
                std::fs::write(&path, adts).unwrap();
                run_ffmpeg_clean_acceptance(&ffmpeg, &path).unwrap();
                let decoded =
                    run_ffmpeg_decode_f32le(&ffmpeg, &path, pcm.sample_rate, pcm.channels).unwrap();
                let quality = match validate_lossy_oracle_pcm_quality(&pcm.samples, &decoded) {
                    Ok(quality) => quality,
                    Err(err) => {
                        eprintln!(
                            "AAC quantizer step sweep {label}: step_scale={step_scale:.2}, quality rejected: {err}, max_abs={max_quantized_abs}, max_frame_len={max_frame_len}, profile={profile:?}"
                        );
                        continue;
                    }
                };
                let constrained = max_quantized_abs
                    <= i32::try_from(balanced_max_quantized_abs).unwrap()
                    && max_frame_len <= frame_budget;
                let correlation_gap =
                    production_quality.best_correlation - quality.best_correlation;
                let rms_ratio =
                    quality.decoded_rms / production_quality.decoded_rms.max(f64::EPSILON);
                eprintln!(
                    "AAC quantizer step sweep {label}: step_scale={step_scale:.2}, constrained={constrained}, max_abs={max_quantized_abs}/{balanced_max_quantized_abs}, max_frame_len={max_frame_len}/{frame_budget}, correlation_gap={correlation_gap:.3}, rms_ratio={rms_ratio:.3}, profile={profile:?}, quality={quality:?}"
                );
                sweep_results.push(super::AacQuantizerStepSweepResult {
                    step_scale,
                    max_quantized_abs,
                    max_frame_len,
                    profile,
                    quality,
                });
            }

            let constrained_promotable = sweep_results
                .iter()
                .filter(|result| {
                    result.max_quantized_abs <= i32::try_from(balanced_max_quantized_abs).unwrap()
                        && result.max_frame_len <= frame_budget
                        && production_quality.best_correlation - result.quality.best_correlation
                            <= 0.09
                        && result.quality.decoded_rms
                            / production_quality.decoded_rms.max(f64::EPSILON)
                            >= 0.50
                })
                .collect::<Vec<_>>();
            assert!(
                constrained_promotable.is_empty(),
                "{label} quantizer step sweep found a constrained default-promotion candidate: promotable={constrained_promotable:?}, production={production_quality:?}, balanced_max_abs={balanced_max_quantized_abs}, frame_budget={frame_budget}"
            );
            assert!(
                sweep_results.iter().any(|result| result.max_quantized_abs
                    > i32::try_from(balanced_max_quantized_abs).unwrap()
                    || result.max_frame_len > frame_budget),
                "{label} quantizer step sweep should expose max_abs or frame-budget pressure when moving finer: results={sweep_results:?}"
            );
        }

        std::fs::remove_dir_all(&out_dir).unwrap();
    }

    #[test]
    fn aac_standard_id_payload_breakdown_identifies_spectral_cost() {
        let mono = readiness_pcm(44_100, 1).unwrap();
        let stereo = super::aac_standard_surface_stereo_pcm(&mono).unwrap();
        let mono_details =
            super::aac_standard_selected_scale_factor_frame_details_with_candidates_and_bitrate(
                &mono,
                sonare_codec::aac_lc_default_production_bitrate_bps(1).unwrap(),
                sonare_codec::AAC_STANDARD_ID_PCM_STEP_CANDIDATES,
            )
            .unwrap();
        let stereo_details =
            super::aac_standard_selected_scale_factor_frame_details_with_candidates_and_bitrate(
                &stereo,
                sonare_codec::aac_lc_default_production_bitrate_bps(2).unwrap(),
                sonare_codec::AAC_STANDARD_ID_PCM_STEP_CANDIDATES,
            )
            .unwrap();

        let mono_breakdown =
            super::aac_standard_id_payload_breakdown_for_frame_selection(&mono, &mono_details)
                .unwrap();
        let stereo_breakdown =
            super::aac_standard_id_payload_breakdown_for_frame_selection(&stereo, &stereo_details)
                .unwrap();

        eprintln!(
            "AAC standard-id payload breakdown: mono={mono_breakdown:?}, stereo={stereo_breakdown:?}"
        );
        assert_eq!(mono_breakdown.frames, mono_details.len());
        assert_eq!(stereo_breakdown.frames, stereo_details.len());
        assert_eq!(mono_breakdown.channels, 1);
        assert_eq!(stereo_breakdown.channels, 2);
        assert!(mono_breakdown.sections > 0);
        assert!(stereo_breakdown.sections > mono_breakdown.sections);
        assert!(mono_breakdown.spectral_bits > mono_breakdown.scale_factor_bits);
        assert!(stereo_breakdown.spectral_bits > stereo_breakdown.scale_factor_bits);
        assert!(mono_breakdown.escape_spectral_bits > 0);
        assert!(stereo_breakdown.escape_spectral_bits > mono_breakdown.escape_spectral_bits);
        let mono_dominant = mono_breakdown
            .dominant_spectral_section
            .expect("mono dominant spectral section");
        let stereo_dominant = stereo_breakdown
            .dominant_spectral_section
            .expect("stereo dominant spectral section");
        let mono_dominant_escape = mono_breakdown
            .dominant_escape_section
            .expect("mono dominant escape section");
        let stereo_dominant_escape = stereo_breakdown
            .dominant_escape_section
            .expect("stereo dominant escape section");
        assert_ne!(mono_dominant.codebook_id, 0);
        assert_ne!(stereo_dominant.codebook_id, 0);
        assert_eq!(mono_dominant_escape.codebook_id, 11);
        assert_eq!(stereo_dominant_escape.codebook_id, 11);
        assert!(mono_dominant.spectral_bits > mono_breakdown.scale_factor_bits);
        assert!(stereo_dominant.spectral_bits > stereo_breakdown.scale_factor_bits);
        assert!(mono_dominant_escape.max_abs >= 13);
        assert!(stereo_dominant_escape.max_abs >= 13);
        assert!(mono_dominant
            .best_alternative_spectral_bits
            .is_some_and(|bit_len| bit_len >= mono_dominant.spectral_bits));
        assert!(stereo_dominant
            .best_alternative_spectral_bits
            .is_some_and(|bit_len| bit_len >= stereo_dominant.spectral_bits));
        assert!(mono_dominant_escape
            .best_alternative_spectral_bits
            .is_none());
        assert!(stereo_dominant_escape
            .best_alternative_spectral_bits
            .is_none());
        assert!(mono_breakdown.total_bits() > 0);
        assert!(stereo_breakdown.total_bits() > mono_breakdown.total_bits());
    }

    #[test]
    fn aac_standard_id_max_quantized_abs_selection_can_suppress_escape() {
        let mono = readiness_pcm(44_100, 1).unwrap();
        let stereo = super::aac_standard_surface_stereo_pcm(&mono).unwrap();
        let mono_bitrate = sonare_codec::aac_lc_default_production_bitrate_bps(1).unwrap();
        let stereo_bitrate = sonare_codec::aac_lc_default_production_bitrate_bps(2).unwrap();
        let mono_baseline =
            super::aac_standard_selected_scale_factor_frame_details_with_candidates_and_bitrate(
                &mono,
                mono_bitrate,
                sonare_codec::AAC_STANDARD_ID_PCM_STEP_CANDIDATES,
            )
            .unwrap();
        let stereo_baseline =
            super::aac_standard_selected_scale_factor_frame_details_with_candidates_and_bitrate(
                &stereo,
                stereo_bitrate,
                sonare_codec::AAC_STANDARD_ID_PCM_STEP_CANDIDATES,
            )
            .unwrap();
        let mono_limited =
            sonare_codec::aac_recommended_standard_selected_scale_factor_frame_details_with_max_quantized_abs_and_bitrate(
                &mono,
                mono_bitrate,
                12,
            )
            .unwrap();
        let stereo_limited =
            sonare_codec::aac_recommended_standard_selected_scale_factor_frame_details_with_max_quantized_abs_and_bitrate(
                &stereo,
                stereo_bitrate,
                12,
            )
            .unwrap();

        let mono_baseline_breakdown =
            super::aac_standard_id_payload_breakdown_for_frame_selection(&mono, &mono_baseline)
                .unwrap();
        let stereo_baseline_breakdown =
            super::aac_standard_id_payload_breakdown_for_frame_selection(&stereo, &stereo_baseline)
                .unwrap();
        let mono_limited_breakdown =
            super::aac_standard_id_payload_breakdown_for_frame_selection(&mono, &mono_limited)
                .unwrap();
        let stereo_limited_breakdown =
            super::aac_standard_id_payload_breakdown_for_frame_selection(&stereo, &stereo_limited)
                .unwrap();

        eprintln!(
            "AAC standard-id max-abs escape suppression: mono baseline={mono_baseline_breakdown:?}, mono limited={mono_limited_breakdown:?}, stereo baseline={stereo_baseline_breakdown:?}, stereo limited={stereo_limited_breakdown:?}"
        );
        assert!(mono_baseline_breakdown.escape_sections > 0);
        assert!(stereo_baseline_breakdown.escape_sections > 0);
        assert!(mono_limited_breakdown.escape_sections < mono_baseline_breakdown.escape_sections);
        assert!(
            stereo_limited_breakdown.escape_sections < stereo_baseline_breakdown.escape_sections
        );
        assert!(
            mono_limited_breakdown.escape_spectral_bits
                < mono_baseline_breakdown.escape_spectral_bits
        );
        assert!(
            stereo_limited_breakdown.escape_spectral_bits
                < stereo_baseline_breakdown.escape_spectral_bits
        );
        assert!(mono_limited_breakdown.max_abs <= 12);
        assert!(stereo_limited_breakdown.max_abs <= 12);
        assert!(mono_limited
            .iter()
            .zip(mono_baseline.iter())
            .any(|(limited, baseline)| limited.step > baseline.step));
        assert!(stereo_limited
            .iter()
            .zip(stereo_baseline.iter())
            .any(|(limited, baseline)| limited.step > baseline.step));
    }

    #[test]
    fn aac_standard_id_quality_control_profile_tracks_balanced_constraints() {
        let mono = readiness_pcm(44_100, 1).unwrap();
        let stereo = super::aac_standard_surface_stereo_pcm(&mono).unwrap();

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
            let baseline_details =
                sonare_codec::aac_recommended_standard_selected_scale_factor_frame_details_with_bitrate(
                    &pcm,
                    bitrate,
                )
                .unwrap();
            let balanced_details =
                sonare_codec::aac_balanced_standard_selected_scale_factor_frame_details_with_bitrate(
                    &pcm,
                    bitrate,
                )
                .unwrap();
            let baseline_breakdown = super::aac_standard_id_payload_breakdown_for_frame_selection(
                &pcm,
                &baseline_details,
            )
            .unwrap();
            let balanced_profile =
                sonare_codec::aac_balanced_standard_id_quality_control_profile_with_bitrate(
                    &pcm, bitrate,
                )
                .unwrap();
            let balanced_profile_from_details =
                sonare_codec::aac_balanced_standard_id_quality_control_profile_for_frame_details(
                    &pcm,
                    &balanced_details,
                )
                .unwrap();

            eprintln!(
                "AAC standard-id balanced quality-control profile {label}: baseline={baseline_breakdown:?}, balanced={balanced_profile:?}"
            );
            assert_eq!(balanced_profile, balanced_profile_from_details);
            assert_eq!(balanced_profile.frames, balanced_details.len());
            assert_eq!(balanced_profile.channels, usize::from(pcm.channels));
            assert!(balanced_profile.min_frame_budget_slack >= 0);
            assert!(balanced_profile.max_frame_len > 0);
            assert!(balanced_profile.max_abs < baseline_breakdown.max_abs);
            assert!(
                balanced_profile.escape_spectral_bits < baseline_breakdown.escape_spectral_bits
            );
            assert!(
                balanced_profile.max_abs
                    <= i32::try_from(balanced_profile.max_quantized_abs_limit).unwrap()
            );
            assert!(balanced_profile.total_bits > balanced_profile.spectral_bits);
            assert!(
                balanced_profile.raised_scale_factor_bands <= balanced_profile.scale_factor_bands
            );
        }
    }

