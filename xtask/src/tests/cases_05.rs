    #[test]
    fn mp3_quality_guard_proxy_tracks_mono_fine_step_gap() {
        let pcm = readiness_pcm(44_100, 1).unwrap();
        let profiles =
            sonare_codec::select_mpeg1_layer3_first_frame_quality_guarded_candidate_profile_with_table_provider(
                &pcm,
                sonare_codec::MPEG1_LAYER3_PCM_STEP_CANDIDATES,
                128,
                false,
                sonare_codec::mpeg1_layer3_standard_table_provider(),
            )
            .unwrap();
        let high_payload_fine = profiles
            .iter()
            .find(|profile| profile.step == 0.0005)
            .copied()
            .unwrap();
        let neutral_fine = profiles
            .iter()
            .find(|profile| profile.step == 0.2)
            .copied()
            .unwrap();
        let active = profiles
            .iter()
            .find(|profile| profile.step == 1.0)
            .copied()
            .unwrap();
        let positive_proxy = profiles
            .iter()
            .find(|profile| profile.quality_guard_distortion_delta > 0.0)
            .copied()
            .unwrap();

        eprintln!(
            "MP3 quality guard mono proxy: high_payload_fine={high_payload_fine:?}, neutral_fine={neutral_fine:?}, active={active:?}, positive_proxy={positive_proxy:?}, profiles={profiles:?}"
        );
        assert!(high_payload_fine.quality_guard_compared_granules > 0);
        assert!(neutral_fine.quality_guard_compared_granules > 0);
        assert!(active.quality_guard_compared_granules > 0);
        assert!(high_payload_fine.quality_guard_distortion_delta.is_finite());
        assert!(neutral_fine.quality_guard_distortion_delta.is_finite());
        assert!(active.quality_guard_distortion_delta.is_finite());
        assert!(
            high_payload_fine.payload_bit_len > high_payload_fine.frame_capacity_bits / 2,
            "very fine candidate should expose the high-payload zero-scale-factor region: high_payload_fine={high_payload_fine:?}"
        );
        assert_eq!(high_payload_fine.quality_guard_distortion_delta, 0.0);
        assert_eq!(neutral_fine.quality_guard_distortion_delta, 0.0);
        assert!(
            active.quality_guard_distortion_delta < 0.0,
            "active mono candidate should expose the current guard proxy mismatch: active={active:?}"
        );
        assert_eq!(
            active.perceptual_granules,
            active.quality_guard_compared_granules
        );
        assert_eq!(active.calibrated_granules, 0);
        assert!(
            active.step >= 1.0 && active.payload_bit_len < active.frame_capacity_bits / 20,
            "active quality-guard candidate should remain in the low-payload mono region: active={active:?}"
        );
        assert!(
            positive_proxy.step >= 2.0
                && positive_proxy.payload_bit_len < positive_proxy.frame_capacity_bits / 20,
            "positive guard proxy region should remain coarse and low-payload: positive_proxy={positive_proxy:?}"
        );
    }

    #[test]
    fn mp3_mono_full_fixed_step_oracle_profile_tracks_production_quality_region_when_ffmpeg_is_available(
    ) {
        let Some(ffmpeg) = std::env::var_os("SONARE_FFMPEG") else {
            return;
        };
        let pcm = readiness_pcm(44_100, 1).unwrap();
        let out_dir = std::env::temp_dir().join(format!(
            "sonare-codec-mp3-mono-full-fixed-step-oracle-profile-{}-{}",
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
        let production_path = out_dir.join("mp3-full-step-production.mp3");
        std::fs::write(&production_path, production).unwrap();
        run_ffmpeg_clean_acceptance(&ffmpeg, &production_path).unwrap();
        let production_decoded =
            run_ffmpeg_decode_f32le(&ffmpeg, &production_path, pcm.sample_rate, pcm.channels)
                .unwrap();
        let production_quality =
            validate_lossy_oracle_pcm_quality(&pcm.samples, &production_decoded).unwrap();

        let mut accepted = Vec::new();
        let mut rejected = Vec::new();
        for step in sonare_codec::MPEG1_LAYER3_PCM_STEP_CANDIDATES
            .iter()
            .copied()
            .chain([1.5_f32])
        {
            let profile =
                match sonare_codec::select_mpeg1_layer3_first_frame_perceptual_candidate_profile_with_table_provider(
                    &pcm,
                    &[step],
                    128,
                    false,
                    sonare_codec::mpeg1_layer3_standard_table_provider(),
                ) {
                    Ok(profiles) => profiles[0],
                    Err(err) => {
                        rejected.push((step, format!("profile failed: {err}")));
                        continue;
                    }
                };
            if profile.step != step {
                rejected.push((step, format!("profile step mismatch: {:?}", profile.step)));
                continue;
            }
            let encoded = match sonare_codec::encode_mpeg1_layer3_pcm_frames_with_perceptual_scale_factors_and_table_provider(
                &pcm,
                step,
                sonare_codec::mpeg1_layer3_standard_table_provider(),
            ) {
                Ok(encoded) => encoded,
                Err(err) => {
                    rejected.push((step, format!("encode failed: {err}")));
                    continue;
                }
            };
            let path = out_dir.join(format!("mp3-full-step-perceptual-{step:.6}.mp3"));
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
                "MP3 mono full fixed-step oracle step={step}: quality={quality:?}, best_offset={best_offset}, profile={profile:?}, production={production_quality:?}"
            );
            if quality.best_correlation >= 0.20 {
                accepted.push((step, quality, best_offset, profile));
            } else {
                rejected.push((
                    step,
                    format!(
                        "quality rejected: decoded_rms={:.4}, best_correlation={:.3}, best_offset={best_offset}, payload_bits={}",
                        quality.decoded_rms, quality.best_correlation, profile.payload_bit_len
                    ),
                ));
            }
        }

        let best = accepted
            .iter()
            .copied()
            .max_by(|(_, left, _, _), (_, right, _, _)| {
                left.best_correlation
                    .partial_cmp(&right.best_correlation)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap();
        eprintln!(
            "MP3 mono full fixed-step oracle summary: best={best:?}, accepted={accepted:?}, rejected={rejected:?}, production={production_quality:?}"
        );
        assert_eq!(
            best.0, 2.0,
            "full fixed-step oracle should keep exposing step=2.0 as the best self-contained mono perceptual candidate: best={best:?}, accepted={accepted:?}"
        );
        assert_eq!(
            best.2, 0,
            "best fixed-step oracle candidate should remain sample-aligned rather than a lag artifact: best={best:?}"
        );
        assert!(
            best.1.best_correlation <= production_quality.best_correlation + 0.001,
            "full fixed-step oracle should not expose a material unpromoted mono candidate above low-band gain production: best={best:?}, production={production_quality:?}, accepted={accepted:?}, rejected={rejected:?}"
        );
        assert!(
            production_quality.best_correlation > best.1.best_correlation + 0.02,
            "production low-band gain reservoir should exceed the best fixed-step mono quality region: best={best:?}, production={production_quality:?}, accepted={accepted:?}, rejected={rejected:?}"
        );
        let near_production = accepted
            .iter()
            .find(|(step, _, _, _)| *step == 1.5)
            .copied()
            .unwrap();
        assert!(
            near_production.1.best_correlation + 0.001 < production_quality.best_correlation,
            "near-production step=1.5 should remain below the selected low-band gain production region: near={near_production:?}, production={production_quality:?}, accepted={accepted:?}"
        );
        assert!(
            best.3.payload_bit_len < best.3.frame_capacity_bits / 20,
            "best unpromoted fixed-step candidate should remain in the sparse payload region that current production selector does not explicitly prefer: best={best:?}"
        );
        assert!(
            accepted.iter().any(|(step, _, _, profile)| {
                *step <= 0.2 && profile.payload_bit_len > profile.frame_capacity_bits / 10
            }),
            "accepted fine-step candidates should still spend more first-frame payload than production-active steps: accepted={accepted:?}"
        );
        assert!(
            accepted.iter().any(|(step, quality, _, _)| {
                *step <= 0.2
                    && quality.best_correlation + 0.02 < production_quality.best_correlation
            }),
            "fine-step fixed candidates should continue exposing the quality proxy gap: accepted={accepted:?}, production={production_quality:?}"
        );

        std::fs::remove_dir_all(&out_dir).unwrap();
    }

    fn mp3_read_bits(bytes: &[u8], bit_offset: usize, bit_len: usize) -> Result<u32, String> {
        let mut value = 0_u32;
        for bit in 0..bit_len {
            let absolute = bit_offset
                .checked_add(bit)
                .ok_or_else(|| "MP3 bit offset overflows".to_owned())?;
            let byte = *bytes
                .get(absolute / 8)
                .ok_or_else(|| "MP3 bit read extends past stream".to_owned())?;
            value = (value << 1) | u32::from((byte >> (7 - absolute % 8)) & 1);
        }
        Ok(value)
    }

    fn mp3_write_bits(
        bytes: &mut [u8],
        bit_offset: usize,
        bit_len: usize,
        value: u32,
    ) -> Result<(), String> {
        for bit in 0..bit_len {
            let absolute = bit_offset
                .checked_add(bit)
                .ok_or_else(|| "MP3 bit offset overflows".to_owned())?;
            let byte = bytes
                .get_mut(absolute / 8)
                .ok_or_else(|| "MP3 bit write extends past stream".to_owned())?;
            let shift = 7 - absolute % 8;
            let source_shift = bit_len - 1 - bit;
            let mask = 1_u8 << shift;
            if ((value >> source_shift) & 1) == 0 {
                *byte &= !mask;
            } else {
                *byte |= mask;
            }
        }
        Ok(())
    }

    fn mp3_skip_layer3_granule_channel_side_info(
        bytes: &[u8],
        mut bit_offset: usize,
    ) -> Result<usize, String> {
        bit_offset += 12 + 9 + 8 + 4;
        mp3_read_bits(bytes, bit_offset, 1)?;
        bit_offset += 1;
        bit_offset += 15;
        Ok(bit_offset + 1 + 1 + 1)
    }

    fn mp3_with_global_gain_bias(bytes: &[u8], bias: i16) -> Result<Vec<u8>, String> {
        let mut patched = bytes.to_vec();
        let mut frame_offset = 0_usize;
        while frame_offset < patched.len() {
            let header = sonare_codec::FrameHeader::parse(&patched[frame_offset..])
                .map_err(|err| format!("MP3 global-gain patch failed to parse frame: {err}"))?;
            if header.layer != sonare_codec::Layer::Layer3
                || header.version != sonare_codec::MpegVersion::Mpeg1
            {
                return Err("MP3 global-gain patch supports MPEG-1 Layer III only".to_owned());
            }
            let side_info_len = header
                .layer3_side_info_len()
                .ok_or_else(|| "MP3 global-gain patch missing side-info length".to_owned())?;
            let frame_len = header.frame_len();
            if frame_offset + frame_len > patched.len() || frame_len < 4 + side_info_len {
                return Err("MP3 global-gain patch frame extends past stream".to_owned());
            }

            let channels = header.channel_count();
            let mut bit_offset = (frame_offset + 4) * 8 + 9 + if channels == 1 { 5 } else { 3 };
            bit_offset += channels * 4;
            for _granule in 0..header.layer3_granule_count() {
                for _channel in 0..channels {
                    let global_gain_offset = bit_offset + 12 + 9;
                    let global_gain = mp3_read_bits(&patched, global_gain_offset, 8)? as i16;
                    let biased = (global_gain + bias).clamp(0, 255) as u32;
                    mp3_write_bits(&mut patched, global_gain_offset, 8, biased)?;
                    bit_offset = mp3_skip_layer3_granule_channel_side_info(&patched, bit_offset)?;
                }
            }

            frame_offset += frame_len;
        }
        Ok(patched)
    }

    #[test]
    fn mp3_global_gain_bias_sweep_tracks_loudness_without_correlation_recovery_when_ffmpeg_is_available(
    ) {
        let Some(ffmpeg) = std::env::var_os("SONARE_FFMPEG") else {
            return;
        };
        let pcm = readiness_pcm(44_100, 1).unwrap();
        let production = sonare_codec::encode_with_mode(
            sonare_codec::Format::Mp3,
            &pcm,
            sonare_codec::EncodeMode::ProductionOnly,
        )
        .unwrap();
        let out_dir = std::env::temp_dir().join(format!(
            "sonare-codec-mp3-global-gain-bias-sweep-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&out_dir).unwrap();

        let mut results = Vec::new();
        for bias in [-4_i16, -2, 0, 2, 4] {
            let encoded = mp3_with_global_gain_bias(&production, bias).unwrap();
            let path = out_dir.join(format!("mp3-global-gain-bias-{bias}.mp3"));
            std::fs::write(&path, encoded).unwrap();
            run_ffmpeg_clean_acceptance(&ffmpeg, &path).unwrap();
            let decoded =
                run_ffmpeg_decode_f32le(&ffmpeg, &path, pcm.sample_rate, pcm.channels).unwrap();
            let quality = validate_lossy_oracle_pcm_quality(&pcm.samples, &decoded).unwrap();
            eprintln!(
                "MP3 global-gain bias sweep bias={bias}: decoded_rms={:.4}, best_correlation={:.3}",
                quality.decoded_rms, quality.best_correlation
            );
            results.push((bias, quality));
        }

        let baseline = results
            .iter()
            .find_map(|(bias, quality)| (*bias == 0).then_some(*quality))
            .unwrap();
        let best = results
            .iter()
            .copied()
            .max_by(|(_, left), (_, right)| {
                left.best_correlation
                    .partial_cmp(&right.best_correlation)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap();
        assert!(
            best.1.best_correlation <= baseline.best_correlation + 0.001,
            "global-gain bias should not hide the mono correlation proxy gap: best={best:?}, baseline={baseline:?}, results={results:?}"
        );
        let negative = results
            .iter()
            .find_map(|(bias, quality)| (*bias == -2).then_some(*quality))
            .unwrap();
        let positive = results
            .iter()
            .find_map(|(bias, quality)| (*bias == 2).then_some(*quality))
            .unwrap();
        assert!(negative.decoded_rms < baseline.decoded_rms);
        assert!(positive.decoded_rms > baseline.decoded_rms);

        std::fs::remove_dir_all(&out_dir).unwrap();
    }

    #[test]
    fn mp3_band_local_scale_factor_bias_sweep_tracks_fine_step_proxy_gap_when_ffmpeg_is_available()
    {
        let Some(ffmpeg) = std::env::var_os("SONARE_FFMPEG") else {
            return;
        };
        let pcm = readiness_pcm(44_100, 1).unwrap();
        let out_dir = std::env::temp_dir().join(format!(
            "sonare-codec-mp3-band-scale-factor-bias-sweep-{}-{}",
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
        let production_path = out_dir.join("mp3-band-bias-production.mp3");
        std::fs::write(&production_path, production).unwrap();
        run_ffmpeg_clean_acceptance(&ffmpeg, &production_path).unwrap();
        let production_decoded =
            run_ffmpeg_decode_f32le(&ffmpeg, &production_path, pcm.sample_rate, pcm.channels)
                .unwrap();
        let production_quality =
            validate_lossy_oracle_pcm_quality(&pcm.samples, &production_decoded).unwrap();

        let candidates = [
            ("baseline", 0_usize, 21_usize, 0_i8),
            ("low-plus", 0, 7, 2),
            ("mid-plus", 7, 14, 2),
            ("high-plus", 14, 21, 2),
            ("low-minus", 0, 7, -2),
            ("mid-minus", 7, 14, -2),
            ("high-minus", 14, 21, -2),
        ];
        let mut results = Vec::new();
        for (label, band_start, band_end, bias) in candidates {
            let encoded = sonare_codec::encode_mpeg1_layer3_pcm_frames_with_perceptual_scale_factor_band_bias_and_table_provider(
                &pcm,
                0.2,
                sonare_codec::Layer3ScaleFactorBandBias {
                    band_start,
                    band_end,
                    bias,
                },
                sonare_codec::mpeg1_layer3_standard_table_provider(),
            )
            .unwrap();
            let path = out_dir.join(format!("mp3-band-bias-{label}.mp3"));
            std::fs::write(&path, encoded).unwrap();
            run_ffmpeg_clean_acceptance(&ffmpeg, &path).unwrap();
            let decoded =
                run_ffmpeg_decode_f32le(&ffmpeg, &path, pcm.sample_rate, pcm.channels).unwrap();
            let quality = validate_lossy_oracle_pcm_quality(&pcm.samples, &decoded).unwrap();
            eprintln!(
                "MP3 band-local scale-factor bias {label}: bands={band_start}..{band_end}, bias={bias}, decoded_rms={:.4}, best_correlation={:.3}, production={production_quality:?}",
                quality.decoded_rms, quality.best_correlation
            );
            results.push((label, quality));
        }

        let baseline = results
            .iter()
            .find_map(|(label, quality)| (*label == "baseline").then_some(*quality))
            .unwrap();
        let best = results
            .iter()
            .copied()
            .max_by(|(_, left), (_, right)| {
                left.best_correlation
                    .partial_cmp(&right.best_correlation)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap();
        assert!(
            best.1.best_correlation + 0.02 < production_quality.best_correlation,
            "band-local fine-step bias should not be mistaken for production recovery yet: best={best:?}, production={production_quality:?}, results={results:?}"
        );
        assert!(
            results
                .iter()
                .any(|(label, quality)| *label != "baseline"
                    && quality.best_correlation < baseline.best_correlation - 0.01),
            "at least one band-local perturbation should expose a sensitive scale-factor region: baseline={baseline:?}, results={results:?}"
        );

        std::fs::remove_dir_all(&out_dir).unwrap();
    }

    #[test]
    fn mp3_quantized_band_gain_sweep_tracks_low_band_shape_gap_when_ffmpeg_is_available() {
        let Some(ffmpeg) = std::env::var_os("SONARE_FFMPEG") else {
            return;
        };
        let pcm = readiness_pcm(44_100, 1).unwrap();
        let out_dir = std::env::temp_dir().join(format!(
            "sonare-codec-mp3-quantized-band-gain-sweep-{}-{}",
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
        let production_path = out_dir.join("mp3-quantized-gain-production.mp3");
        std::fs::write(&production_path, production).unwrap();
        run_ffmpeg_clean_acceptance(&ffmpeg, &production_path).unwrap();
        let production_decoded =
            run_ffmpeg_decode_f32le(&ffmpeg, &production_path, pcm.sample_rate, pcm.channels)
                .unwrap();
        let production_quality =
            validate_lossy_oracle_pcm_quality(&pcm.samples, &production_decoded).unwrap();

        let candidates = [
            ("baseline", 0_usize, 21_usize, 1.0_f32),
            ("low-half", 0, 7, 0.5),
            ("low-boost", 0, 7, 1.5),
            ("low-invert", 0, 7, -1.0),
            ("mid-half", 7, 14, 0.5),
            ("mid-boost", 7, 14, 1.5),
            ("mid-invert", 7, 14, -1.0),
            ("high-half", 14, 21, 0.5),
            ("high-boost", 14, 21, 1.5),
            ("high-invert", 14, 21, -1.0),
        ];
        let mut results = Vec::new();
        for (label, band_start, band_end, gain) in candidates {
            let encoded = sonare_codec::encode_mpeg1_layer3_pcm_frames_with_perceptual_quantized_band_gain_and_table_provider(
                &pcm,
                0.2,
                sonare_codec::Layer3QuantizedBandGain {
                    band_start,
                    band_end,
                    gain,
                },
                sonare_codec::mpeg1_layer3_standard_table_provider(),
            )
            .unwrap();
            let path = out_dir.join(format!("mp3-quantized-gain-{label}.mp3"));
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
                "MP3 quantized band gain {label}: bands={band_start}..{band_end}, gain={gain:.2}, decoded_rms={:.4}, best_correlation={:.3}, best_offset={best_offset}, production={production_quality:?}",
                quality.decoded_rms, quality.best_correlation
            );
            results.push((label, quality, best_offset));
        }

        let baseline = results
            .iter()
            .find_map(|(label, quality, offset)| {
                (*label == "baseline").then_some((*quality, *offset))
            })
            .unwrap();
        let best = results
            .iter()
            .copied()
            .max_by(|(_, left, _), (_, right, _)| {
                left.best_correlation
                    .partial_cmp(&right.best_correlation)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap();
        assert!(
            best.1.best_correlation + 0.01 < production_quality.best_correlation,
            "quantized band gain should not be mistaken for production recovery yet: best={best:?}, production={production_quality:?}, results={results:?}"
        );
        assert!(
            results
                .iter()
                .any(|(label, quality, _)| *label != "baseline"
                    && quality.best_correlation + 0.01 < baseline.0.best_correlation),
            "at least one quantized band gain should expose low-band spectral-shape sensitivity: baseline={baseline:?}, results={results:?}"
        );
        assert!(
            results.iter().all(|(_, _, offset)| *offset == baseline.1),
            "quantized band gain should expose a spectral-shape gap, not a best-lag shift: baseline={baseline:?}, results={results:?}"
        );

        std::fs::remove_dir_all(&out_dir).unwrap();
    }

    #[test]
    fn mp3_production_region_band_local_sweep_exposes_low_gain_loudness_tradeoff_when_ffmpeg_is_available(
    ) {
        let Some(ffmpeg) = std::env::var_os("SONARE_FFMPEG") else {
            return;
        };
        let pcm = readiness_pcm(44_100, 1).unwrap();
        let out_dir = std::env::temp_dir().join(format!(
            "sonare-codec-mp3-production-region-band-local-sweep-{}-{}",
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
        let production_path = out_dir.join("mp3-production-region-production.mp3");
        std::fs::write(&production_path, production).unwrap();
        run_ffmpeg_clean_acceptance(&ffmpeg, &production_path).unwrap();
        let production_decoded =
            run_ffmpeg_decode_f32le(&ffmpeg, &production_path, pcm.sample_rate, pcm.channels)
                .unwrap();
        let production_quality =
            validate_lossy_oracle_pcm_quality(&pcm.samples, &production_decoded).unwrap();

        let baseline = sonare_codec::encode_mpeg1_layer3_pcm_frames_with_perceptual_scale_factors_and_table_provider(
            &pcm,
            2.0,
            sonare_codec::mpeg1_layer3_standard_table_provider(),
        )
        .unwrap();
        let baseline_path = out_dir.join("mp3-production-region-step2-baseline.mp3");
        std::fs::write(&baseline_path, baseline).unwrap();
        run_ffmpeg_clean_acceptance(&ffmpeg, &baseline_path).unwrap();
        let baseline_decoded =
            run_ffmpeg_decode_f32le(&ffmpeg, &baseline_path, pcm.sample_rate, pcm.channels)
                .unwrap();
        let baseline_quality =
            validate_lossy_oracle_pcm_quality(&pcm.samples, &baseline_decoded).unwrap();

        let mut results = vec![("baseline", baseline_quality, 0usize, 21usize, "none")];
        for (label, band_start, band_end, bias) in [
            ("sf-low-plus1", 0_usize, 7_usize, 1_i8),
            ("sf-low-plus2", 0, 7, 2),
            ("sf-low-minus1", 0, 7, -1),
            ("sf-mid-plus1", 7, 14, 1),
            ("sf-high-plus1", 14, 21, 1),
        ] {
            let encoded = sonare_codec::encode_mpeg1_layer3_pcm_frames_with_perceptual_scale_factor_band_bias_and_table_provider(
                &pcm,
                2.0,
                sonare_codec::Layer3ScaleFactorBandBias {
                    band_start,
                    band_end,
                    bias,
                },
                sonare_codec::mpeg1_layer3_standard_table_provider(),
            )
            .unwrap();
            let path = out_dir.join(format!("mp3-production-region-{label}.mp3"));
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
                "MP3 production-region scale-factor band bias {label}: bands={band_start}..{band_end}, bias={bias}, decoded_rms={:.4}, best_correlation={:.3}, best_offset={best_offset}, production={production_quality:?}, baseline={baseline_quality:?}",
                quality.decoded_rms, quality.best_correlation
            );
            results.push((label, quality, band_start, band_end, "sf"));
        }
        for (label, band_start, band_end, gain) in [
            ("q-low-half", 0_usize, 7_usize, 0.5_f32),
            ("q-low-boost125", 0, 7, 1.25),
            ("q-low-boost150", 0, 7, 1.5),
            ("q-mid-boost125", 7, 14, 1.25),
            ("q-high-boost125", 14, 21, 1.25),
        ] {
            let encoded = sonare_codec::encode_mpeg1_layer3_pcm_frames_with_perceptual_quantized_band_gain_and_table_provider(
                &pcm,
                2.0,
                sonare_codec::Layer3QuantizedBandGain {
                    band_start,
                    band_end,
                    gain,
                },
                sonare_codec::mpeg1_layer3_standard_table_provider(),
            )
            .unwrap();
            let path = out_dir.join(format!("mp3-production-region-{label}.mp3"));
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
                "MP3 production-region quantized band gain {label}: bands={band_start}..{band_end}, gain={gain:.2}, decoded_rms={:.4}, best_correlation={:.3}, best_offset={best_offset}, production={production_quality:?}, baseline={baseline_quality:?}",
                quality.decoded_rms, quality.best_correlation
            );
            results.push((label, quality, band_start, band_end, "q"));
        }

        let best = results
            .iter()
            .copied()
            .max_by(|(_, left, _, _, _), (_, right, _, _, _)| {
                left.best_correlation
                    .partial_cmp(&right.best_correlation)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap();
        assert_eq!(
            best.0, "q-low-boost150",
            "step=2.0 production-region sweep should keep exposing low-band quantized gain as the only correlation-improving perturbation: best={best:?}, production={production_quality:?}, results={results:?}"
        );
        assert!(
            best.1.best_correlation > baseline_quality.best_correlation + 0.02
                && production_quality.best_correlation > baseline_quality.best_correlation + 0.02,
            "low-band quantized gain and production should both improve over the self-contained baseline: best={best:?}, baseline={baseline_quality:?}, production={production_quality:?}"
        );
        assert!(
            best.1.decoded_rms >= production_quality.decoded_rms * 1.9,
            "low-band quantized gain without global gain bias should remain blocked from direct production promotion by loudness overshoot: best={best:?}, production={production_quality:?}"
        );
        assert!(
            (best.1.best_correlation - production_quality.best_correlation).abs() <= 0.002,
            "production should keep the low-band quantized gain correlation while correcting loudness with global gain bias: best={best:?}, production={production_quality:?}"
        );
        assert!(
            results.iter().any(|(label, quality, _, _, _)| {
                *label != "baseline"
                    && quality.best_correlation + 0.01 < baseline_quality.best_correlation
            }),
            "band-local perturbations should continue exposing sensitive production-region support: baseline={baseline_quality:?}, results={results:?}"
        );

        std::fs::remove_dir_all(&out_dir).unwrap();
    }

    #[test]
    fn mp3_low_band_gain_global_gain_bias_sweep_finds_loudness_matched_promotion_candidate_when_ffmpeg_is_available(
    ) {
        let Some(ffmpeg) = std::env::var_os("SONARE_FFMPEG") else {
            return;
        };
        let pcm = readiness_pcm(44_100, 1).unwrap();
        let out_dir = std::env::temp_dir().join(format!(
            "sonare-codec-mp3-low-band-gain-global-gain-bias-sweep-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&out_dir).unwrap();

        let baseline = sonare_codec::encode_mpeg1_layer3_pcm_frames_with_entropy_targeted_perceptual_reservoir_and_table_provider(
            &pcm,
            sonare_codec::mpeg1_layer3_production_pcm_step_candidates(pcm.channels).unwrap(),
            128,
            false,
            0,
            sonare_codec::mpeg1_layer3_standard_table_provider(),
        )
        .unwrap();
        let production_path = out_dir.join("mp3-low-band-gain-baseline.mp3");
        std::fs::write(&production_path, &baseline).unwrap();
        run_ffmpeg_clean_acceptance(&ffmpeg, &production_path).unwrap();
        let baseline_decoded =
            run_ffmpeg_decode_f32le(&ffmpeg, &production_path, pcm.sample_rate, pcm.channels)
                .unwrap();
        let baseline_quality =
            validate_lossy_oracle_pcm_quality(&pcm.samples, &baseline_decoded).unwrap();

        let mut results = Vec::new();
        for bias in [-8_i16, -6, -4, -2, 0, 2] {
            let encoded = sonare_codec::encode_mpeg1_layer3_pcm_frames_with_perceptual_quantized_band_gain_and_global_gain_bias_and_table_provider(
                &pcm,
                2.0,
                sonare_codec::Layer3QuantizedBandGain {
                    band_start: 0,
                    band_end: 7,
                    gain: 1.5,
                },
                bias,
                sonare_codec::mpeg1_layer3_standard_table_provider(),
            )
            .unwrap();
            let path = out_dir.join(format!("mp3-low-band-gain-global-gain-bias-{bias}.mp3"));
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
            let rms_ratio = quality.decoded_rms / baseline_quality.decoded_rms;
            eprintln!(
                "MP3 low-band gain + global-gain bias sweep bias={bias}: decoded_rms={:.4}, rms_ratio={rms_ratio:.3}, best_correlation={:.3}, best_offset={best_offset}, baseline={baseline_quality:?}",
                quality.decoded_rms, quality.best_correlation
            );
            results.push((bias, quality, rms_ratio, best_offset));
        }

        let best_correlation = results
            .iter()
            .copied()
            .max_by(|(_, left, _, _), (_, right, _, _)| {
                left.best_correlation
                    .partial_cmp(&right.best_correlation)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap();
        let best_loudness_matched = results
            .iter()
            .copied()
            .filter(|(_, _, rms_ratio, _)| (0.80..=1.20).contains(rms_ratio))
            .max_by(|(_, left, _, _), (_, right, _, _)| {
                left.best_correlation
                    .partial_cmp(&right.best_correlation)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap();

        assert_eq!(
            best_correlation.0, 0,
            "unbiased low-band gain should remain the best correlation but over-loud: best={best_correlation:?}, baseline={baseline_quality:?}, results={results:?}"
        );
        assert!(
            best_correlation.1.decoded_rms > baseline_quality.decoded_rms * 2.0,
            "best correlation candidate should remain blocked by loudness overshoot: best={best_correlation:?}, baseline={baseline_quality:?}"
        );
        assert_eq!(
            best_loudness_matched.0, -4,
            "global gain correction should identify the loudness-matched low-band gain candidate: loudness_matched={best_loudness_matched:?}, baseline={baseline_quality:?}, results={results:?}"
        );
        assert!(
            (0.95..=1.10).contains(&best_loudness_matched.2)
                && best_loudness_matched.1.best_correlation
                    > baseline_quality.best_correlation + 0.02,
            "loudness-matched low-band gain should preserve the correlation boost and stay near baseline RMS: loudness_matched={best_loudness_matched:?}, baseline={baseline_quality:?}, results={results:?}"
        );
        assert!(
            results.iter().all(|(_, _, _, offset)| *offset == 0),
            "global-gain corrected low-band gain should remain sample-aligned, not a lag artifact: results={results:?}"
        );

        std::fs::remove_dir_all(&out_dir).unwrap();
    }

