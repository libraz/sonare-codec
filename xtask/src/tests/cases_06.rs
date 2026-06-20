    #[test]
    fn mp3_low_band_gain_global_gain_bias_entropy_reservoir_preserves_mono_oracle_gain_when_ffmpeg_is_available(
    ) {
        let Some(ffmpeg) = std::env::var_os("SONARE_FFMPEG") else {
            return;
        };
        let out_dir = std::env::temp_dir().join(format!(
            "sonare-codec-mp3-low-band-gain-global-gain-bias-reservoir-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&out_dir).unwrap();

        for sample_rate in [32_000, 44_100, 48_000] {
            let pcm = readiness_pcm(sample_rate, 1).unwrap();
            let baseline = sonare_codec::encode_mpeg1_layer3_pcm_frames_with_entropy_targeted_perceptual_reservoir_and_table_provider(
                &pcm,
                sonare_codec::mpeg1_layer3_production_pcm_step_candidates(pcm.channels).unwrap(),
                128,
                false,
                0,
                sonare_codec::mpeg1_layer3_standard_table_provider(),
            )
            .unwrap();
            let production_path = out_dir.join(format!(
                "mp3-low-band-gain-reservoir-baseline-{sample_rate}.mp3"
            ));
            std::fs::write(&production_path, baseline).unwrap();
            run_ffmpeg_clean_acceptance(&ffmpeg, &production_path).unwrap();
            let baseline_decoded =
                run_ffmpeg_decode_f32le(&ffmpeg, &production_path, pcm.sample_rate, pcm.channels)
                    .unwrap();
            let baseline_quality =
                validate_lossy_oracle_pcm_quality(&pcm.samples, &baseline_decoded).unwrap();

            let reservoir = sonare_codec::encode_mpeg1_layer3_pcm_frames_with_entropy_targeted_perceptual_quantized_band_gain_and_global_gain_bias_reservoir_and_table_provider(
                &pcm,
                &[2.0],
                128,
                false,
                0,
                sonare_codec::Layer3QuantizedBandGain {
                    band_start: 0,
                    band_end: 7,
                    gain: 1.5,
                },
                -4,
                sonare_codec::mpeg1_layer3_standard_table_provider(),
            )
            .unwrap();
            let reservoir_path = out_dir.join(format!(
                "mp3-low-band-gain-global-gain-bias-reservoir-{sample_rate}.mp3"
            ));
            std::fs::write(&reservoir_path, reservoir).unwrap();
            run_ffmpeg_clean_acceptance(&ffmpeg, &reservoir_path).unwrap();
            let reservoir_decoded =
                run_ffmpeg_decode_f32le(&ffmpeg, &reservoir_path, pcm.sample_rate, pcm.channels)
                    .unwrap();
            let (reservoir_best_correlation, reservoir_best_offset) =
                best_normalized_correlation_with_offset(&pcm.samples, &reservoir_decoded).unwrap();
            let reservoir_quality = LossyOraclePcmQuality {
                decoded_rms: rms(&reservoir_decoded),
                best_correlation: reservoir_best_correlation,
            };
            let rms_ratio = reservoir_quality.decoded_rms / baseline_quality.decoded_rms;
            eprintln!(
                "MP3 low-band gain + global-gain bias entropy reservoir {sample_rate}Hz: decoded_rms={:.4}, rms_ratio={rms_ratio:.3}, best_correlation={:.3}, best_offset={reservoir_best_offset}, baseline={baseline_quality:?}",
                reservoir_quality.decoded_rms, reservoir_quality.best_correlation
            );

            assert_eq!(
                reservoir_best_offset, 0,
                "reservoir low-band gain candidate should stay sample-aligned, not win through a lag artifact"
            );
            assert!(
                (0.95..=1.10).contains(&rms_ratio),
                "reservoir low-band gain candidate should remain loudness-matched with the old entropy-targeted baseline: reservoir={reservoir_quality:?}, baseline={baseline_quality:?}, rms_ratio={rms_ratio}"
            );
            assert!(
                reservoir_quality.best_correlation > baseline_quality.best_correlation + 0.02,
                "reservoir low-band gain candidate should preserve the mono oracle correlation gain over the old entropy-targeted baseline: reservoir={reservoir_quality:?}, baseline={baseline_quality:?}"
            );
        }

        std::fs::remove_dir_all(&out_dir).unwrap();
    }

    #[test]
    fn mp3_reservoir_quality_bridge_sweep_keeps_entropy_targeted_production_when_ffmpeg_is_available(
    ) {
        let Some(ffmpeg) = std::env::var_os("SONARE_FFMPEG") else {
            return;
        };
        let out_dir = std::env::temp_dir().join(format!(
            "sonare-codec-mp3-reservoir-quality-bridge-sweep-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&out_dir).unwrap();

        for channels in [1, 2] {
            let pcm = readiness_pcm(44_100, channels).unwrap();
            let production_candidates =
                sonare_codec::mpeg1_layer3_production_pcm_step_candidates(pcm.channels).unwrap();
            let calibrated =
                sonare_codec::encode_mpeg1_layer3_pcm_frames_with_reservoir_and_table_provider(
                    &pcm,
                    sonare_codec::MPEG1_LAYER3_PCM_STEP_CANDIDATES,
                    128,
                    false,
                    sonare_codec::mpeg1_layer3_standard_table_provider(),
                )
                .unwrap();
            let perceptual = sonare_codec::encode_mpeg1_layer3_pcm_frames_with_perceptual_reservoir_and_table_provider(
                &pcm,
                sonare_codec::MPEG1_LAYER3_PCM_STEP_CANDIDATES,
                128,
                false,
                sonare_codec::mpeg1_layer3_standard_table_provider(),
            )
            .unwrap();
            let quality_guarded = sonare_codec::encode_mpeg1_layer3_pcm_frames_with_quality_guarded_perceptual_reservoir_and_table_provider(
                &pcm,
                sonare_codec::MPEG1_LAYER3_PCM_STEP_CANDIDATES,
                128,
                false,
                sonare_codec::mpeg1_layer3_standard_table_provider(),
            )
            .unwrap();
            let entropy_targeted = sonare_codec::encode_mpeg1_layer3_pcm_frames_with_entropy_targeted_perceptual_reservoir_and_table_provider(
                &pcm,
                production_candidates,
                128,
                false,
                0,
                sonare_codec::mpeg1_layer3_standard_table_provider(),
            )
            .unwrap();
            let mono_low_band_gain = if channels == 1 {
                Some(
                    sonare_codec::encode_mpeg1_layer3_pcm_frames_with_entropy_targeted_perceptual_quantized_band_gain_and_global_gain_bias_reservoir_and_table_provider(
                        &pcm,
                        &[2.0],
                        128,
                        false,
                        0,
                        sonare_codec::Layer3QuantizedBandGain {
                            band_start: 0,
                            band_end: 7,
                            gain: 1.5,
                        },
                        -4,
                        sonare_codec::mpeg1_layer3_standard_table_provider(),
                    )
                    .unwrap(),
                )
            } else {
                None
            };
            let production = sonare_codec::encode_with_mode(
                sonare_codec::Format::Mp3,
                &pcm,
                sonare_codec::EncodeMode::ProductionOnly,
            )
            .unwrap();
            if channels == 1 {
                assert_eq!(
                    production,
                    mono_low_band_gain.clone().unwrap(),
                    "mono MP3 production should remain byte-for-byte tied to the low-band gain/global-gain-bias entropy reservoir bridge"
                );
            } else {
                assert_eq!(
                    production, entropy_targeted,
                    "{channels}ch MP3 production should remain byte-for-byte tied to the entropy-targeted reservoir bridge"
                );
            }

            let guarded_details = sonare_codec::select_mpeg1_layer3_quality_guarded_perceptual_reservoir_frame_details_with_table_provider(
                &pcm,
                sonare_codec::MPEG1_LAYER3_PCM_STEP_CANDIDATES,
                128,
                false,
                sonare_codec::mpeg1_layer3_standard_table_provider(),
            )
            .unwrap();
            assert!(guarded_details
                .iter()
                .any(|detail| detail.quality_guard_compared_granules > 0));
            let guarded_perceptual_granules: usize = guarded_details
                .iter()
                .map(|detail| detail.perceptual_granules)
                .sum();
            let guarded_calibrated_granules: usize = guarded_details
                .iter()
                .map(|detail| detail.calibrated_granules)
                .sum();
            let guarded_compared_granules: usize = guarded_details
                .iter()
                .map(|detail| detail.quality_guard_compared_granules)
                .sum();
            let guarded_distortion_delta: f64 = guarded_details
                .iter()
                .map(|detail| detail.quality_guard_distortion_delta)
                .sum();
            let guarded_min_step = guarded_details
                .iter()
                .map(|detail| detail.step)
                .fold(f32::INFINITY, f32::min);
            let guarded_max_step = guarded_details
                .iter()
                .map(|detail| detail.step)
                .fold(0.0_f32, f32::max);
            let guarded_max_payload = guarded_details
                .iter()
                .map(|detail| detail.payload_bit_len)
                .max()
                .unwrap_or(0);
            eprintln!(
                "MP3 reservoir quality bridge {channels}ch guard: step_range={guarded_min_step:.3}..{guarded_max_step:.3}, max_payload_bits={guarded_max_payload}, perceptual_granules={guarded_perceptual_granules}, calibrated_granules={guarded_calibrated_granules}, compared_granules={guarded_compared_granules}, distortion_delta={guarded_distortion_delta:.3}"
            );
            if channels == 1 {
                assert!(
                    guarded_perceptual_granules > 0,
                    "mono quality guard stopped exercising the perceptual allocation path"
                );
                assert!(
                    guarded_min_step >= 1.0,
                    "mono quality guard should prefer the active scale-factor step range: min_step={guarded_min_step}"
                );
            }
            let entropy_details = sonare_codec::select_mpeg1_layer3_entropy_targeted_perceptual_reservoir_frame_details_with_table_provider(
                &pcm,
                production_candidates,
                128,
                false,
                0,
                sonare_codec::mpeg1_layer3_standard_table_provider(),
            )
            .unwrap();
            assert!(entropy_details
                .iter()
                .any(|detail| detail.used_entropy_target_budget));
            let entropy_min_step = entropy_details
                .iter()
                .map(|detail| detail.step)
                .fold(f32::INFINITY, f32::min);
            let entropy_max_step = entropy_details
                .iter()
                .map(|detail| detail.step)
                .fold(0.0_f32, f32::max);
            let entropy_max_payload = entropy_details
                .iter()
                .map(|detail| detail.payload_bit_len)
                .max()
                .unwrap_or(0);
            eprintln!(
                "MP3 reservoir quality bridge {channels}ch entropy-targeted: step_range={entropy_min_step:.3}..{entropy_max_step:.3}, max_payload_bits={entropy_max_payload}"
            );

            let mut encoded_candidates = vec![
                ("calibrated", calibrated),
                ("perceptual", perceptual),
                ("quality_guarded", quality_guarded),
                ("entropy_targeted", entropy_targeted),
                ("production", production),
            ];
            if let Some(encoded) = mono_low_band_gain {
                encoded_candidates.push(("mono_low_band_gain", encoded));
            }

            let mut qualities = Vec::new();
            for (kind, encoded) in encoded_candidates {
                let path = out_dir.join(format!("mp3-quality-bridge-{channels}ch-{kind}.mp3"));
                std::fs::write(&path, encoded).unwrap();
                run_ffmpeg_clean_acceptance(&ffmpeg, &path).unwrap();
                let decoded =
                    run_ffmpeg_decode_f32le(&ffmpeg, &path, pcm.sample_rate, pcm.channels).unwrap();
                let quality = validate_lossy_oracle_pcm_quality(&pcm.samples, &decoded).unwrap();
                eprintln!(
                    "MP3 reservoir quality bridge {channels}ch {kind}: decoded_rms={:.4}, best_correlation={:.3}",
                    quality.decoded_rms,
                    quality.best_correlation
                );
                qualities.push((kind, quality));
            }

            let production_quality = qualities
                .iter()
                .find_map(|(kind, quality)| (*kind == "production").then_some(*quality))
                .unwrap();
            let calibrated_quality = qualities
                .iter()
                .find_map(|(kind, quality)| (*kind == "calibrated").then_some(*quality))
                .unwrap();
            let guarded_quality = qualities
                .iter()
                .find_map(|(kind, quality)| (*kind == "quality_guarded").then_some(*quality))
                .unwrap();
            if channels == 1 {
                let mono_low_band_gain_quality = qualities
                    .iter()
                    .find_map(|(kind, quality)| (*kind == "mono_low_band_gain").then_some(*quality))
                    .unwrap();
                assert!(
                    guarded_quality.best_correlation
                        >= calibrated_quality.best_correlation + 0.015,
                    "mono quality-guarded stream selection should improve over calibrated after active scale-factor filtering: guarded={guarded_quality:?}, calibrated={calibrated_quality:?}"
                );
                assert!(
                    production_quality.best_correlation
                        >= mono_low_band_gain_quality.best_correlation - 0.001
                        && production_quality.best_correlation
                            > guarded_quality.best_correlation + 0.02,
                    "mono production should use the low-band gain bridge and improve over the older guarded path: production={production_quality:?}, low_band={mono_low_band_gain_quality:?}, guarded={guarded_quality:?}"
                );
            }
            let best = qualities
                .iter()
                .copied()
                .max_by(|(_, left), (_, right)| {
                    left.best_correlation
                        .partial_cmp(&right.best_correlation)
                        .unwrap_or(std::cmp::Ordering::Equal)
                        .then_with(|| {
                            left.decoded_rms
                                .partial_cmp(&right.decoded_rms)
                                .unwrap_or(std::cmp::Ordering::Equal)
                        })
                })
                .unwrap();
            assert!(
                production_quality.best_correlation + 0.001 >= best.1.best_correlation,
                "{channels}ch MP3 reservoir bridge found a better non-production candidate {best:?}; promote or retune production"
            );
        }

        std::fs::remove_dir_all(&out_dir).unwrap();
    }

    #[test]
    fn mp3_production_artifacts_respect_default_frame_budget() {
        for (sample_rate, channels) in [(44_100, 1), (44_100, 2)] {
            let pcm = readiness_pcm(sample_rate, channels).unwrap();
            let encoded = sonare_codec::encode_with_mode(
                sonare_codec::Format::Mp3,
                &pcm,
                sonare_codec::EncodeMode::ProductionOnly,
            )
            .unwrap();
            let label = if channels == 1 {
                "MP3 44.1kHz mono"
            } else {
                "MP3 44.1kHz stereo"
            };

            verify_mp3_default_production_budget(
                label,
                ProductionArtifactKind::Mp3,
                &pcm,
                &encoded,
            )
            .unwrap();
        }
    }

    #[test]
    fn mp3_production_entropy_targets_match_public_bit_allocation() {
        for channels in [1, 2] {
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
            let frame_targets =
                mp3_perceptual_bit_allocation_targets_by_frame("MP3 allocation", &pcm, &details)
                    .unwrap();

            assert_eq!(frame_targets.len(), details.len());
            for (target_bits, detail) in frame_targets.iter().zip(details.iter()) {
                assert_eq!(*target_bits, detail.entropy_target_bits);
                if detail.used_entropy_target_budget {
                    let entropy_budget_bits = detail
                        .entropy_target_bits
                        .saturating_add(7)
                        .checked_div(8)
                        .unwrap_or(0)
                        .clamp(1, detail.frame_capacity_bytes + detail.main_data_begin)
                        * 8;
                    assert!(detail.payload_bit_len <= entropy_budget_bits);
                }
            }
            assert!(details
                .iter()
                .any(|detail| detail.used_entropy_target_budget));
        }
    }

    #[test]
    fn mp3_production_artifacts_pass_focused_ffmpeg_quality_gate() {
        let Some(ffmpeg) = std::env::var_os("SONARE_FFMPEG") else {
            eprintln!("skipping MP3 production quality gate: set SONARE_FFMPEG=/path/to/ffmpeg");
            return;
        };
        let out_dir = std::env::temp_dir().join(format!(
            "sonare-codec-mp3-production-quality-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&out_dir).unwrap();

        for (sample_rate, channels) in [(44_100, 1), (44_100, 2)] {
            let pcm = readiness_pcm(sample_rate, channels).unwrap();
            let encoded = sonare_codec::encode_with_mode(
                sonare_codec::Format::Mp3,
                &pcm,
                sonare_codec::EncodeMode::ProductionOnly,
            )
            .unwrap();
            let label = if channels == 1 {
                "MP3 44.1kHz mono"
            } else {
                "MP3 44.1kHz stereo"
            };

            verify_mp3_default_production_budget(
                label,
                ProductionArtifactKind::Mp3,
                &pcm,
                &encoded,
            )
            .unwrap();
            let artifact_path = out_dir.join(format!("mp3-production-quality-{}ch.mp3", channels));
            std::fs::write(&artifact_path, &encoded).unwrap();
            run_ffmpeg_clean_acceptance(&ffmpeg, &artifact_path).unwrap();
            let decoded =
                run_ffmpeg_decode_f32le(&ffmpeg, &artifact_path, sample_rate, channels).unwrap();
            let quality = validate_lossy_oracle_pcm_quality(&pcm.samples, &decoded).unwrap();
            let min_correlation =
                production_lossy_min_correlation(ProductionArtifactKind::Mp3, channels).unwrap();
            assert!(
                quality.best_correlation >= min_correlation,
                "{label} production quality regressed below floor {min_correlation}: {quality:?}"
            );
            eprintln!(
                "{label} production quality: decoded_rms={:.4}, best_correlation={:.3}",
                quality.decoded_rms, quality.best_correlation
            );
        }
    }

    #[test]
    fn mp3_default_frame_budget_rejects_truncated_frame() {
        let pcm = readiness_pcm(44_100, 1).unwrap();
        let encoded = sonare_codec::encode_with_mode(
            sonare_codec::Format::Mp3,
            &pcm,
            sonare_codec::EncodeMode::ProductionOnly,
        )
        .unwrap();
        let err = verify_mp3_default_production_budget(
            "MP3 truncated",
            ProductionArtifactKind::Mp3,
            &pcm,
            &encoded[..encoded.len() - 1],
        )
        .unwrap_err();

        assert!(err.contains("extends past stream length"));
    }

    #[test]
    fn mp3_production_reservoir_check_rejects_self_contained_perceptual_stream() {
        let pcm = readiness_pcm(44_100, 1).unwrap();
        let perceptual =
            sonare_codec::encode_mpeg1_layer3_pcm_frames_with_perceptual_active_cbr_bitrate_and_table_provider(
                &pcm,
                sonare_codec::MPEG1_LAYER3_PCM_STEP_CANDIDATES,
                128,
                false,
                sonare_codec::mpeg1_layer3_standard_table_provider(),
            )
            .unwrap();
        let err = verify_mp3_production_reservoir("MP3 perceptual diagnostic", &pcm, &perceptual)
            .unwrap_err();

        assert!(
            err.contains("never used main_data_begin")
                || err.contains("does not match selector detail")
                || err
                    .contains("did not match the low-band gain/global-gain-bias reservoir profile"),
            "unexpected MP3 production reservoir rejection: {err}"
        );
    }

    #[test]
    fn aac_production_artifacts_respect_default_bitrate_budget() {
        for (sample_rate, channels) in [(44_100, 1), (44_100, 2)] {
            let pcm = readiness_pcm(sample_rate, channels).unwrap();
            let adts = sonare_codec::encode_with_mode(
                sonare_codec::Format::Aac,
                &pcm,
                sonare_codec::EncodeMode::ProductionOnly,
            )
            .unwrap();
            let label = if channels == 1 {
                "AAC-LC 44.1kHz mono"
            } else {
                "AAC-LC 44.1kHz stereo"
            };

            verify_aac_default_production_budget(label, ProductionArtifactKind::Aac, &pcm, &adts)
                .unwrap();

            let m4a = sonare_codec::mux_aac_adts_as_m4a(&adts).unwrap();
            verify_aac_default_production_budget(label, ProductionArtifactKind::M4a, &pcm, &m4a)
                .unwrap();
        }
    }

    #[test]
    fn aac_production_artifacts_pass_focused_ffmpeg_quality_gate() {
        let Some(ffmpeg) = std::env::var_os("SONARE_FFMPEG") else {
            eprintln!("skipping AAC production quality gate: set SONARE_FFMPEG=/path/to/ffmpeg");
            return;
        };
        let out_dir = std::env::temp_dir().join(format!(
            "sonare-codec-aac-production-quality-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&out_dir).unwrap();

        for (sample_rate, channels) in [(44_100, 1), (44_100, 2)] {
            let pcm = readiness_pcm(sample_rate, channels).unwrap();
            let adts = sonare_codec::encode_with_mode(
                sonare_codec::Format::Aac,
                &pcm,
                sonare_codec::EncodeMode::ProductionOnly,
            )
            .unwrap();
            let m4a = sonare_codec::mux_aac_adts_as_m4a(&adts).unwrap();
            let label = if channels == 1 {
                "AAC-LC 44.1kHz mono"
            } else {
                "AAC-LC 44.1kHz stereo"
            };

            for (kind, bytes, extension) in [
                (ProductionArtifactKind::Aac, adts.as_slice(), "aac"),
                (ProductionArtifactKind::M4a, m4a.as_slice(), "m4a"),
            ] {
                verify_aac_default_production_budget(label, kind, &pcm, bytes).unwrap();
                let artifact_path =
                    out_dir.join(format!("aac-production-quality-{}ch.{extension}", channels));
                std::fs::write(&artifact_path, bytes).unwrap();
                run_ffmpeg_clean_acceptance(&ffmpeg, &artifact_path).unwrap();
                let decoded =
                    run_ffmpeg_decode_f32le(&ffmpeg, &artifact_path, sample_rate, channels)
                        .unwrap();
                let quality = validate_lossy_oracle_pcm_quality(&pcm.samples, &decoded).unwrap();
                let min_correlation = production_lossy_min_correlation(kind, channels).unwrap();
                assert!(
                    quality.best_correlation >= min_correlation,
                    "{label} {kind:?} production quality regressed below floor {min_correlation}: {quality:?}"
                );
                eprintln!(
                    "{label} {kind:?} production quality: decoded_rms={:.4}, best_correlation={:.3}",
                    quality.decoded_rms, quality.best_correlation
                );
            }
        }
    }

    #[test]
    fn aac_default_bitrate_budget_rejects_malformed_adts() {
        let pcm = readiness_pcm(44_100, 1).unwrap();
        let err = verify_aac_default_production_budget(
            "AAC-LC malformed",
            ProductionArtifactKind::Aac,
            &pcm,
            &[0xff, 0xf1, 0x50],
        )
        .unwrap_err();

        assert!(err.contains("ADTS stream has no complete frames"));
    }
