    #[test]
    fn aac_standard_id_max_quantized_abs_candidate_passes_ffmpeg_oracle_when_available() {
        let Some(ffmpeg) = std::env::var_os("SONARE_FFMPEG") else {
            eprintln!(
                "skipping AAC max-quantized-abs quality gate: set SONARE_FFMPEG=/path/to/ffmpeg"
            );
            return;
        };
        let mono = readiness_pcm(44_100, 1).unwrap();
        let stereo = super::aac_standard_surface_stereo_pcm(&mono).unwrap();
        let out_dir = std::env::temp_dir().join(format!(
            "sonare-codec-aac-max-abs-test-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&out_dir).unwrap();

        for (label, pcm, bitrate, min_correlation) in [
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
        ] {
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
            let baseline_path = out_dir.join(format!("aac-standard-id-{label}-baseline.aac"));
            std::fs::write(&baseline_path, baseline_adts).unwrap();
            let baseline_decoded =
                run_ffmpeg_decode_f32le(&ffmpeg, &baseline_path, pcm.sample_rate, pcm.channels)
                    .unwrap();
            let baseline_quality =
                validate_lossy_oracle_pcm_quality(&pcm.samples, &baseline_decoded).unwrap();

            let max_quantized_abs = 2047;
            let limited_details =
                sonare_codec::aac_recommended_standard_selected_scale_factor_frame_details_with_max_quantized_abs_and_bitrate(
                    &pcm,
                    bitrate,
                    max_quantized_abs,
                )
                .unwrap();
            let limited_breakdown = super::aac_standard_id_payload_breakdown_for_frame_selection(
                &pcm,
                &limited_details,
            )
            .unwrap();
            let limited_adts =
                sonare_codec::encode_aac_adts_with_recommended_standard_spectral_offsets_and_selected_scale_factors_max_quantized_abs_and_bitrate(
                    &pcm,
                    bitrate,
                    max_quantized_abs,
                )
                .unwrap();
            let limited_m4a =
                sonare_codec::encode_m4a_with_recommended_standard_spectral_offsets_and_selected_scale_factors_max_quantized_abs_and_bitrate(
                    &pcm,
                    bitrate,
                    max_quantized_abs,
                )
                .unwrap();
            let limited_path = out_dir.join(format!(
                "aac-standard-id-{label}-max-abs-{max_quantized_abs}.aac"
            ));
            let limited_m4a_path = out_dir.join(format!(
                "aac-standard-id-{label}-max-abs-{max_quantized_abs}.m4a"
            ));
            std::fs::write(&limited_path, limited_adts).unwrap();
            std::fs::write(&limited_m4a_path, limited_m4a).unwrap();
            run_ffmpeg_clean_acceptance(&ffmpeg, &limited_path).unwrap();
            run_ffmpeg_clean_acceptance(&ffmpeg, &limited_m4a_path).unwrap();
            let limited_decoded =
                run_ffmpeg_decode_f32le(&ffmpeg, &limited_path, pcm.sample_rate, pcm.channels)
                    .unwrap();
            let limited_m4a_decoded =
                run_ffmpeg_decode_f32le(&ffmpeg, &limited_m4a_path, pcm.sample_rate, pcm.channels)
                    .unwrap();
            let limited_quality =
                validate_lossy_oracle_pcm_quality(&pcm.samples, &limited_decoded).unwrap();
            let limited_m4a_quality =
                validate_lossy_oracle_pcm_quality(&pcm.samples, &limited_m4a_decoded).unwrap();
            assert!(
                limited_quality.best_correlation >= min_correlation,
                "{label} max-abs candidate correlation below floor: limited={limited_quality:?}"
            );
            assert!(
                limited_quality.best_correlation + 0.10 >= baseline_quality.best_correlation,
                "{label} max-abs candidate regressed too far from baseline: limited={limited_quality:?}, baseline={baseline_quality:?}"
            );
            assert!(
                limited_quality.decoded_rms >= baseline_quality.decoded_rms * 0.10,
                "{label} max-abs candidate RMS collapsed too far: limited={limited_quality:?}, baseline={baseline_quality:?}"
            );
            assert!(
                limited_m4a_quality.best_correlation + f64::EPSILON
                    >= limited_quality.best_correlation,
                "{label} max-abs M4A lagged ADTS: m4a={limited_m4a_quality:?}, adts={limited_quality:?}"
            );
            assert!(limited_breakdown.max_abs <= i32::try_from(max_quantized_abs).unwrap());
            assert!(
                limited_breakdown.escape_spectral_bits < baseline_breakdown.escape_spectral_bits
            );
            eprintln!(
                "AAC standard-id max-abs {label}: max_abs_limit={max_quantized_abs}, baseline={baseline_quality:?}, limited={limited_quality:?}, limited_m4a={limited_m4a_quality:?}, baseline_breakdown={baseline_breakdown:?}, limited_breakdown={limited_breakdown:?}"
            );
        }

        std::fs::remove_dir_all(&out_dir).unwrap();
    }

    #[test]
    fn aac_standard_id_max_quantized_abs_ladder_finds_rms_balanced_candidate_when_available() {
        let Some(ffmpeg) = std::env::var_os("SONARE_FFMPEG") else {
            eprintln!(
                "skipping AAC max-quantized-abs ladder quality gate: set SONARE_FFMPEG=/path/to/ffmpeg"
            );
            return;
        };
        let mono = readiness_pcm(44_100, 1).unwrap();
        let stereo = super::aac_standard_surface_stereo_pcm(&mono).unwrap();
        let out_dir = std::env::temp_dir().join(format!(
            "sonare-codec-aac-max-abs-ladder-test-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&out_dir).unwrap();

        for (label, pcm, bitrate, min_correlation) in [
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
        ] {
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
            let baseline_path =
                out_dir.join(format!("aac-standard-id-{label}-ladder-baseline.aac"));
            std::fs::write(&baseline_path, baseline_adts).unwrap();
            let baseline_decoded =
                run_ffmpeg_decode_f32le(&ffmpeg, &baseline_path, pcm.sample_rate, pcm.channels)
                    .unwrap();
            let baseline_quality =
                validate_lossy_oracle_pcm_quality(&pcm.samples, &baseline_decoded).unwrap();

            let mut balanced = None;
            for max_quantized_abs in [5631_u32, 5119, 4095, 3071, 2047] {
                let details =
                    sonare_codec::aac_recommended_standard_selected_scale_factor_frame_details_with_max_quantized_abs_and_bitrate(
                        &pcm,
                        bitrate,
                        max_quantized_abs,
                    )
                    .unwrap();
                let breakdown =
                    super::aac_standard_id_payload_breakdown_for_frame_selection(&pcm, &details)
                        .unwrap();
                let adts =
                    sonare_codec::encode_aac_adts_with_recommended_standard_spectral_offsets_and_selected_scale_factors_max_quantized_abs_and_bitrate(
                        &pcm,
                        bitrate,
                        max_quantized_abs,
                    )
                    .unwrap();
                let path = out_dir.join(format!(
                    "aac-standard-id-{label}-ladder-{max_quantized_abs}.aac"
                ));
                std::fs::write(&path, adts).unwrap();
                run_ffmpeg_clean_acceptance(&ffmpeg, &path).unwrap();
                let decoded =
                    run_ffmpeg_decode_f32le(&ffmpeg, &path, pcm.sample_rate, pcm.channels).unwrap();
                let quality = validate_lossy_oracle_pcm_quality(&pcm.samples, &decoded).unwrap();
                let rms_ratio =
                    quality.decoded_rms / baseline_quality.decoded_rms.max(f64::EPSILON);
                eprintln!(
                    "AAC standard-id max-abs ladder {label}: limit={max_quantized_abs}, rms_ratio={rms_ratio:.3}, quality={quality:?}, breakdown={breakdown:?}"
                );

                if breakdown.escape_spectral_bits < baseline_breakdown.escape_spectral_bits
                    && breakdown.max_abs < baseline_breakdown.max_abs
                    && quality.best_correlation >= min_correlation
                    && quality.best_correlation + 0.10 >= baseline_quality.best_correlation
                    && quality.decoded_rms >= baseline_quality.decoded_rms * 0.35
                {
                    balanced = Some((max_quantized_abs, quality, breakdown));
                }
            }

            let (limit, quality, breakdown) = balanced.unwrap_or_else(|| {
                panic!(
                    "{label} max-abs ladder found no RMS-balanced escape reduction: baseline_quality={baseline_quality:?}, baseline_breakdown={baseline_breakdown:?}"
                )
            });
            assert!(limit < u32::try_from(baseline_breakdown.max_abs).unwrap());
            assert!(breakdown.max_abs < baseline_breakdown.max_abs);
            assert!(breakdown.escape_spectral_bits < baseline_breakdown.escape_spectral_bits);
            assert!(quality.decoded_rms >= baseline_quality.decoded_rms * 0.35);
            eprintln!(
                "AAC standard-id max-abs balanced {label}: limit={limit}, baseline_quality={baseline_quality:?}, balanced_quality={quality:?}, baseline_breakdown={baseline_breakdown:?}, balanced_breakdown={breakdown:?}"
            );
        }

        std::fs::remove_dir_all(&out_dir).unwrap();
    }

    #[test]
    fn aac_standard_id_balanced_surface_passes_release_guard_when_ffmpeg_is_available() {
        let Some(ffmpeg) = std::env::var_os("SONARE_FFMPEG") else {
            eprintln!(
                "skipping AAC balanced standard-id release gate: set SONARE_FFMPEG=/path/to/ffmpeg"
            );
            return;
        };
        let mono = readiness_pcm(44_100, 1).unwrap();
        let stereo = super::aac_standard_surface_stereo_pcm(&mono).unwrap();
        let out_dir = std::env::temp_dir().join(format!(
            "sonare-codec-aac-balanced-surface-test-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&out_dir).unwrap();

        for (label, pcm, bitrate, min_correlation) in [
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
        ] {
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
            let baseline_path =
                out_dir.join(format!("aac-standard-id-balanced-{label}-baseline.aac"));
            std::fs::write(&baseline_path, baseline_adts).unwrap();
            let baseline_decoded =
                run_ffmpeg_decode_f32le(&ffmpeg, &baseline_path, pcm.sample_rate, pcm.channels)
                    .unwrap();
            let baseline_quality =
                validate_lossy_oracle_pcm_quality(&pcm.samples, &baseline_decoded).unwrap();

            let (balanced_quality, balanced_breakdown) =
                super::validate_aac_standard_id_balanced_surface(
                    super::AacStandardIdBalancedSurfaceCheck {
                        ffmpeg: &ffmpeg,
                        label: &format!("AAC-LC standard-id balanced {label}"),
                        expected_pcm: &pcm,
                        bitrate,
                        baseline_quality,
                        min_correlation,
                        out_dir: &out_dir,
                        file_stem: &format!("aac-standard-id-balanced-{label}"),
                    },
                )
                .unwrap();

            assert!(balanced_breakdown.max_abs < baseline_breakdown.max_abs);
            assert!(
                balanced_breakdown.escape_spectral_bits < baseline_breakdown.escape_spectral_bits
            );
            assert!(balanced_quality.decoded_rms >= baseline_quality.decoded_rms * 0.35);
        }

        std::fs::remove_dir_all(&out_dir).unwrap();
    }

    #[test]
    fn aac_standard_id_balanced_surface_tracks_default_promotion_gap_when_ffmpeg_is_available() {
        let Some(ffmpeg) = std::env::var_os("SONARE_FFMPEG") else {
            eprintln!(
                "skipping AAC balanced promotion-gap gate: set SONARE_FFMPEG=/path/to/ffmpeg"
            );
            return;
        };
        let mono = readiness_pcm(44_100, 1).unwrap();
        let stereo = super::aac_standard_surface_stereo_pcm(&mono).unwrap();
        let out_dir = std::env::temp_dir().join(format!(
            "sonare-codec-aac-balanced-promotion-gap-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&out_dir).unwrap();

        for (label, pcm, bitrate, min_correlation) in [
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
        ] {
            let baseline_adts =
                sonare_codec::encode_aac_adts_with_recommended_standard_spectral_offsets_and_selected_scale_factors_and_bitrate(
                    &pcm,
                    bitrate,
                )
                .unwrap();
            let baseline_path =
                out_dir.join(format!("aac-standard-id-balanced-gap-{label}-baseline.aac"));
            std::fs::write(&baseline_path, baseline_adts).unwrap();
            let baseline_decoded =
                run_ffmpeg_decode_f32le(&ffmpeg, &baseline_path, pcm.sample_rate, pcm.channels)
                    .unwrap();
            let baseline_quality =
                validate_lossy_oracle_pcm_quality(&pcm.samples, &baseline_decoded).unwrap();

            let (balanced_quality, balanced_breakdown) =
                super::validate_aac_standard_id_balanced_surface(
                    super::AacStandardIdBalancedSurfaceCheck {
                        ffmpeg: &ffmpeg,
                        label: &format!("AAC-LC standard-id balanced promotion gap {label}"),
                        expected_pcm: &pcm,
                        bitrate,
                        baseline_quality,
                        min_correlation,
                        out_dir: &out_dir,
                        file_stem: &format!("aac-standard-id-balanced-gap-{label}"),
                    },
                )
                .unwrap();

            let production_adts = sonare_codec::encode_with_mode(
                sonare_codec::Format::Aac,
                &pcm,
                sonare_codec::EncodeMode::ProductionOnly,
            )
            .unwrap();
            let production_path = out_dir.join(format!(
                "aac-standard-id-balanced-gap-{label}-production.aac"
            ));
            std::fs::write(&production_path, production_adts).unwrap();
            run_ffmpeg_clean_acceptance(&ffmpeg, &production_path).unwrap();
            let production_decoded =
                run_ffmpeg_decode_f32le(&ffmpeg, &production_path, pcm.sample_rate, pcm.channels)
                    .unwrap();
            let production_quality =
                validate_lossy_oracle_pcm_quality(&pcm.samples, &production_decoded).unwrap();
            let correlation_gap =
                production_quality.best_correlation - balanced_quality.best_correlation;
            let rms_ratio =
                balanced_quality.decoded_rms / production_quality.decoded_rms.max(f64::EPSILON);

            assert!(
                correlation_gap >= 0.09,
                "{label} balanced standard-id path is close enough to production to revisit default promotion: balanced={balanced_quality:?}, production={production_quality:?}, gap={correlation_gap:.3}"
            );
            assert!(
                rms_ratio <= 0.30,
                "{label} balanced standard-id path no longer exposes the production loudness gap: balanced={balanced_quality:?}, production={production_quality:?}, rms_ratio={rms_ratio:.3}"
            );
            eprintln!(
                "AAC standard-id balanced default-promotion gap {label}: balanced={balanced_quality:?}, production={production_quality:?}, correlation_gap={correlation_gap:.3}, rms_ratio={rms_ratio:.3}, balanced_breakdown={balanced_breakdown:?}"
            );
        }

        std::fs::remove_dir_all(&out_dir).unwrap();
    }

    #[test]
    fn aac_standard_id_loudness_recovery_sweep_keeps_default_promotion_blocked_when_ffmpeg_is_available(
    ) {
        let Some(ffmpeg) = std::env::var_os("SONARE_FFMPEG") else {
            eprintln!("skipping AAC loudness recovery sweep: set SONARE_FFMPEG=/path/to/ffmpeg");
            return;
        };
        let mono = readiness_pcm(44_100, 1).unwrap();
        let stereo = super::aac_standard_surface_stereo_pcm(&mono).unwrap();
        let out_dir = std::env::temp_dir().join(format!(
            "sonare-codec-aac-loudness-recovery-sweep-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&out_dir).unwrap();

        let cases: [(&str, sonare_codec::AudioBuffer, u32); 2] = [
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
        ];

        for (label, pcm, bitrate) in cases {
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
            let production_adts = sonare_codec::encode_with_mode(
                sonare_codec::Format::Aac,
                &pcm,
                sonare_codec::EncodeMode::ProductionOnly,
            )
            .unwrap();
            let production_path =
                out_dir.join(format!("aac-standard-id-loudness-{label}-production.aac"));
            std::fs::write(&production_path, production_adts).unwrap();
            run_ffmpeg_clean_acceptance(&ffmpeg, &production_path).unwrap();
            let production_decoded =
                run_ffmpeg_decode_f32le(&ffmpeg, &production_path, pcm.sample_rate, pcm.channels)
                    .unwrap();
            let production_quality =
                validate_lossy_oracle_pcm_quality(&pcm.samples, &production_decoded).unwrap();
            let candidates = super::aac_loudness_recovery_candidates(pcm.channels).unwrap();
            assert_eq!(
                candidates.first().copied(),
                Some(super::aac_balanced_profile_selected_candidate(pcm.channels).unwrap())
            );

            let mut best: Option<(
                u8,
                i16,
                u32,
                LossyOraclePcmQuality,
                super::AacStandardIdPayloadBreakdown,
            )> = None;
            let mut promotable = Vec::new();
            for &(global_gain, magnitude_bias, max_quantized_abs) in &candidates {
                let details = match sonare_codec::aac_standard_selected_scale_factor_frame_details_with_magnitude_bias_max_quantized_abs_and_bitrate(
                    &pcm,
                    bitrate,
                    global_gain,
                    magnitude_bias,
                    max_quantized_abs,
                ) {
                    Ok(details) => details,
                    Err(err) => {
                        eprintln!(
                            "AAC standard-id loudness recovery {label}: gain={global_gain}, bias={magnitude_bias}, max_abs={max_quantized_abs}, details failed: {err}"
                        );
                        continue;
                    }
                };
                let breakdown =
                    super::aac_standard_id_payload_breakdown_for_frame_selection(&pcm, &details)
                        .unwrap();
                if breakdown.max_abs > i32::try_from(max_quantized_abs).unwrap()
                    || breakdown.escape_spectral_bits >= baseline_breakdown.escape_spectral_bits
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
                    "aac-standard-id-loudness-{label}-gain-{global_gain}-bias-{magnitude_bias}-maxabs-{max_quantized_abs}.aac"
                ));
                std::fs::write(&path, adts).unwrap();
                run_ffmpeg_clean_acceptance(&ffmpeg, &path).unwrap();
                let decoded =
                    run_ffmpeg_decode_f32le(&ffmpeg, &path, pcm.sample_rate, pcm.channels).unwrap();
                let quality = match validate_lossy_oracle_pcm_quality(&pcm.samples, &decoded) {
                    Ok(quality) => quality,
                    Err(err) => {
                        eprintln!(
                            "AAC standard-id loudness recovery {label}: gain={global_gain}, bias={magnitude_bias}, max_abs={max_quantized_abs}, quality rejected: {err}, breakdown={breakdown:?}"
                        );
                        continue;
                    }
                };
                let correlation_gap =
                    production_quality.best_correlation - quality.best_correlation;
                let rms_ratio =
                    quality.decoded_rms / production_quality.decoded_rms.max(f64::EPSILON);
                eprintln!(
                    "AAC standard-id loudness recovery {label}: gain={global_gain}, bias={magnitude_bias}, max_abs={max_quantized_abs}, correlation_gap={correlation_gap:.3}, rms_ratio={rms_ratio:.3}, quality={quality:?}, breakdown={breakdown:?}"
                );
                if correlation_gap <= 0.09 && rms_ratio >= 0.50 {
                    promotable.push((global_gain, magnitude_bias, max_quantized_abs, quality));
                }

                let candidate = (
                    global_gain,
                    magnitude_bias,
                    max_quantized_abs,
                    quality,
                    breakdown,
                );
                best = match best {
                    Some(previous)
                        if (production_quality.best_correlation - previous.3.best_correlation)
                            .abs()
                            <= (production_quality.best_correlation
                                - candidate.3.best_correlation)
                                .abs() =>
                    {
                        Some(previous)
                    }
                    _ => Some(candidate),
                };
            }

            let best = best.unwrap();
            assert!(
                promotable.is_empty(),
                "{label} loudness recovery sweep found a default-promotion candidate: promotable={promotable:?}, production={production_quality:?}, baseline_breakdown={baseline_breakdown:?}"
            );
            eprintln!(
                "AAC standard-id loudness recovery best {label}: best={best:?}, production={production_quality:?}, baseline_breakdown={baseline_breakdown:?}"
            );
        }

        std::fs::remove_dir_all(&out_dir).unwrap();
    }

    #[test]
    fn aac_standard_id_aggressive_max_abs_candidate_tracks_correlation_rms_tradeoff_when_ffmpeg_is_available(
    ) {
        let Some(ffmpeg) = std::env::var_os("SONARE_FFMPEG") else {
            eprintln!(
                "skipping AAC aggressive max-abs tradeoff gate: set SONARE_FFMPEG=/path/to/ffmpeg"
            );
            return;
        };
        let mono = readiness_pcm(44_100, 1).unwrap();
        let stereo = super::aac_standard_surface_stereo_pcm(&mono).unwrap();
        let out_dir = std::env::temp_dir().join(format!(
            "sonare-codec-aac-aggressive-max-abs-tradeoff-{}-{}",
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
            let balanced_adts =
                sonare_codec::encode_aac_adts_with_balanced_standard_spectral_offsets_and_selected_scale_factors_and_bitrate(
                    &pcm,
                    bitrate,
                )
                .unwrap();
            let balanced_path = out_dir.join(format!("aac-standard-id-{label}-balanced.aac"));
            std::fs::write(&balanced_path, balanced_adts).unwrap();
            let balanced_decoded =
                run_ffmpeg_decode_f32le(&ffmpeg, &balanced_path, pcm.sample_rate, pcm.channels)
                    .unwrap();
            let balanced_quality =
                validate_lossy_oracle_pcm_quality(&pcm.samples, &balanced_decoded).unwrap();
            let balanced_details =
                sonare_codec::aac_balanced_standard_selected_scale_factor_frame_details_with_bitrate(
                    &pcm,
                    bitrate,
                )
                .unwrap();
            let balanced_breakdown = super::aac_standard_id_payload_breakdown_for_frame_selection(
                &pcm,
                &balanced_details,
            )
            .unwrap();

            let aggressive_max_abs = 2047;
            let aggressive_adts =
                sonare_codec::encode_aac_adts_with_recommended_standard_spectral_offsets_and_selected_scale_factors_max_quantized_abs_and_bitrate(
                    &pcm,
                    bitrate,
                    aggressive_max_abs,
                )
                .unwrap();
            let aggressive_path = out_dir.join(format!(
                "aac-standard-id-{label}-aggressive-{aggressive_max_abs}.aac"
            ));
            std::fs::write(&aggressive_path, aggressive_adts).unwrap();
            run_ffmpeg_clean_acceptance(&ffmpeg, &aggressive_path).unwrap();
            let aggressive_decoded =
                run_ffmpeg_decode_f32le(&ffmpeg, &aggressive_path, pcm.sample_rate, pcm.channels)
                    .unwrap();
            let aggressive_quality =
                validate_lossy_oracle_pcm_quality(&pcm.samples, &aggressive_decoded).unwrap();
            let aggressive_details =
                sonare_codec::aac_recommended_standard_selected_scale_factor_frame_details_with_max_quantized_abs_and_bitrate(
                    &pcm,
                    bitrate,
                    aggressive_max_abs,
                )
                .unwrap();
            let aggressive_breakdown =
                super::aac_standard_id_payload_breakdown_for_frame_selection(
                    &pcm,
                    &aggressive_details,
                )
                .unwrap();

            assert!(
                aggressive_quality.best_correlation + 0.06 >= balanced_quality.best_correlation,
                "{label} aggressive max-abs candidate should remain a near-correlation tradeoff candidate: aggressive={aggressive_quality:?}, balanced={balanced_quality:?}"
            );
            assert!(
                aggressive_quality.decoded_rms < balanced_quality.decoded_rms * 0.25,
                "{label} aggressive max-abs candidate no longer exposes the RMS tradeoff: aggressive={aggressive_quality:?}, balanced={balanced_quality:?}"
            );
            assert!(
                aggressive_breakdown.escape_spectral_bits
                    <= balanced_breakdown.escape_spectral_bits + balanced_breakdown.escape_spectral_bits / 8,
                "{label} aggressive max-abs candidate should keep escape pressure in the same diagnostic region: aggressive={aggressive_breakdown:?}, balanced={balanced_breakdown:?}"
            );
            eprintln!(
                "AAC standard-id aggressive max-abs tradeoff {label}: aggressive={aggressive_quality:?}, balanced={balanced_quality:?}, aggressive_breakdown={aggressive_breakdown:?}, balanced_breakdown={balanced_breakdown:?}"
            );
        }

        std::fs::remove_dir_all(&out_dir).unwrap();
    }

    #[test]
    fn aac_standard_id_aggressive_max_abs_gain_bias_sweep_tracks_balanced_promotion_when_ffmpeg_is_available(
    ) {
        let Some(ffmpeg) = std::env::var_os("SONARE_FFMPEG") else {
            eprintln!(
                "skipping AAC aggressive max-abs gain/bias sweep: set SONARE_FFMPEG=/path/to/ffmpeg"
            );
            return;
        };
        let mono = readiness_pcm(44_100, 1).unwrap();
        let stereo = super::aac_standard_surface_stereo_pcm(&mono).unwrap();
        let out_dir = std::env::temp_dir().join(format!(
            "sonare-codec-aac-aggressive-max-abs-gain-bias-sweep-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&out_dir).unwrap();

        let cases: [(&str, sonare_codec::AudioBuffer, u32); 2] = [
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
        ];
        for (label, pcm, bitrate) in cases {
            let baseline_adts =
                sonare_codec::encode_aac_adts_with_recommended_standard_spectral_offsets_and_selected_scale_factors_and_bitrate(
                    &pcm,
                    bitrate,
                )
                .unwrap();
            let baseline_path = out_dir.join(format!(
                "aac-standard-id-aggressive-sweep-{label}-baseline.aac"
            ));
            std::fs::write(&baseline_path, baseline_adts).unwrap();
            let baseline_decoded =
                run_ffmpeg_decode_f32le(&ffmpeg, &baseline_path, pcm.sample_rate, pcm.channels)
                    .unwrap();
            let baseline_quality =
                validate_lossy_oracle_pcm_quality(&pcm.samples, &baseline_decoded).unwrap();

            let balanced_adts =
                sonare_codec::encode_aac_adts_with_balanced_standard_spectral_offsets_and_selected_scale_factors_and_bitrate(
                    &pcm,
                    bitrate,
                )
                .unwrap();
            let balanced_path = out_dir.join(format!(
                "aac-standard-id-aggressive-sweep-{label}-balanced.aac"
            ));
            std::fs::write(&balanced_path, balanced_adts).unwrap();
            let balanced_decoded =
                run_ffmpeg_decode_f32le(&ffmpeg, &balanced_path, pcm.sample_rate, pcm.channels)
                    .unwrap();
            let balanced_quality =
                validate_lossy_oracle_pcm_quality(&pcm.samples, &balanced_decoded).unwrap();
            let balanced_details =
                sonare_codec::aac_balanced_standard_selected_scale_factor_frame_details_with_bitrate(
                    &pcm,
                    bitrate,
                )
                .unwrap();
            let balanced_breakdown = super::aac_standard_id_payload_breakdown_for_frame_selection(
                &pcm,
                &balanced_details,
            )
            .unwrap();
            let balance_profile =
                sonare_codec::aac_standard_id_selected_scale_factor_balance_profile(pcm.channels)
                    .unwrap();
            let (gain_deltas, magnitude_biases, max_quantized_abs_candidates) =
                super::aac_aggressive_gain_bias_candidates(pcm.channels).unwrap();
            assert_eq!(
                (
                    balance_profile
                        .recommended_global_gain
                        .saturating_add(gain_deltas[0]),
                    magnitude_biases[0],
                    max_quantized_abs_candidates[0],
                ),
                (
                    balance_profile.selected_global_gain,
                    balance_profile.selected_magnitude_bias,
                    balance_profile.max_quantized_abs,
                )
            );
            let mut best: Option<(
                u8,
                i16,
                u32,
                LossyOraclePcmQuality,
                super::AacStandardIdPayloadBreakdown,
            )> = None;

            for &gain_delta in &gain_deltas {
                let global_gain = balance_profile
                    .recommended_global_gain
                    .saturating_add(gain_delta);
                for &magnitude_bias in &magnitude_biases {
                    for &max_quantized_abs in &max_quantized_abs_candidates {
                        let details = match sonare_codec::aac_standard_selected_scale_factor_frame_details_with_magnitude_bias_max_quantized_abs_and_bitrate(
                            &pcm,
                            bitrate,
                            global_gain,
                            magnitude_bias,
                            max_quantized_abs,
                        ) {
                            Ok(details) => details,
                            Err(err) => {
                                eprintln!(
                                    "AAC standard-id aggressive sweep {label}: gain={global_gain}, bias={magnitude_bias}, max_abs={max_quantized_abs}, details failed: {err}"
                                );
                                continue;
                            }
                        };
                        let breakdown =
                            super::aac_standard_id_payload_breakdown_for_frame_selection(
                                &pcm, &details,
                            )
                            .unwrap();
                        if breakdown.max_abs > i32::try_from(max_quantized_abs).unwrap()
                            || breakdown.escape_spectral_bits
                                >= balanced_breakdown.escape_spectral_bits
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
                            "aac-standard-id-aggressive-sweep-{label}-gain-{global_gain}-bias-{magnitude_bias}-maxabs-{max_quantized_abs}.aac"
                        ));
                        std::fs::write(&path, adts).unwrap();
                        run_ffmpeg_clean_acceptance(&ffmpeg, &path).unwrap();
                        let decoded =
                            run_ffmpeg_decode_f32le(&ffmpeg, &path, pcm.sample_rate, pcm.channels)
                                .unwrap();
                        let quality =
                            validate_lossy_oracle_pcm_quality(&pcm.samples, &decoded).unwrap();
                        let rms_ratio =
                            quality.decoded_rms / balanced_quality.decoded_rms.max(f64::EPSILON);
                        eprintln!(
                            "AAC standard-id aggressive sweep {label}: gain={global_gain}, bias={magnitude_bias}, max_abs={max_quantized_abs}, rms_ratio_vs_balanced={rms_ratio:.3}, quality={quality:?}, breakdown={breakdown:?}"
                        );

                        if quality.best_correlation <= balanced_quality.best_correlation
                            || quality.decoded_rms < balanced_quality.decoded_rms * 0.80
                            || breakdown.escape_spectral_bits
                                >= balanced_breakdown.escape_spectral_bits
                        {
                            continue;
                        }

                        let candidate = (
                            global_gain,
                            magnitude_bias,
                            max_quantized_abs,
                            quality,
                            breakdown,
                        );
                        best = match best {
                            Some(previous)
                                if (previous.3.decoded_rms - baseline_quality.decoded_rms)
                                    .abs()
                                    <= (candidate.3.decoded_rms - baseline_quality.decoded_rms)
                                        .abs() =>
                            {
                                Some(previous)
                            }
                            _ => Some(candidate),
                        };
                    }
                }
            }

            let expected_balanced_parameters =
                sonare_codec::aac_standard_id_selected_scale_factor_balanced_parameters(
                    pcm.channels,
                )
                .unwrap();
            if let Some(best) = best {
                assert_eq!(
                    (best.0, best.1, best.2),
                    expected_balanced_parameters,
                    "{label} aggressive sweep found a better balanced parameter set: best={best:?}, current={expected_balanced_parameters:?}, baseline_quality={baseline_quality:?}, balanced_quality={balanced_quality:?}, balanced_breakdown={balanced_breakdown:?}"
                );
                assert!(
                    best.3.best_correlation > balanced_quality.best_correlation
                        && best.3.decoded_rms >= balanced_quality.decoded_rms * 0.80
                        && best.4.escape_spectral_bits < balanced_breakdown.escape_spectral_bits
                );
                eprintln!(
                    "AAC standard-id aggressive sweep promotion {label}: best={best:?}, baseline_quality={baseline_quality:?}, balanced_quality={balanced_quality:?}, balanced_breakdown={balanced_breakdown:?}"
                );
            } else {
                eprintln!(
                    "AAC standard-id aggressive sweep current-balanced {label}: current={expected_balanced_parameters:?}, baseline_quality={baseline_quality:?}, balanced_quality={balanced_quality:?}, balanced_breakdown={balanced_breakdown:?}"
                );
            }
        }

        std::fs::remove_dir_all(&out_dir).unwrap();
    }

