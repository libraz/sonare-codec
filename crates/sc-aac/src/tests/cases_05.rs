    #[test]
    fn encodes_pcm_stereo_long_block_as_adts() {
        let pcm = AudioBuffer::new(44_100, 2, vec![0.0; 4096]).unwrap();

        let adts = encode_pcm_stereo_long_block_adts(
            AdtsConfig::aac_lc(44_100, 2),
            AacLongBlockConfig::new(0, 1),
            AacLongBlockConfig::new(0, 1),
            &pcm,
            AacPcmLongBlockConfig::new(0, 1.0, 1024),
            AacSpectralMagnitudeTables::default(),
        )
        .unwrap();

        assert_eq!(
            adts,
            [
                0xff, 0xf1, 0x50, 0x80, 0x02, 0x3f, 0xfc, 0x20, 0x00, 0x00, 0x40, 0x10, 0x00, 0x00,
                0x80, 0x23, 0x80,
            ]
        );
        assert!(encode_pcm_stereo_long_block_adts(
            AdtsConfig::aac_lc(44_100, 2),
            AacLongBlockConfig::new(0, 1),
            AacLongBlockConfig::new(0, 1),
            &AudioBuffer::new(44_100, 1, vec![0.0; 2048]).unwrap(),
            AacPcmLongBlockConfig::new(0, 1.0, 1024),
            AacSpectralMagnitudeTables::default(),
        )
        .is_err());
    }

    #[test]
    fn encodes_pcm_stereo_long_block_with_scale_factors_as_adts() {
        let pcm = AudioBuffer::new(44_100, 2, vec![0.0; 4096]).unwrap();

        let adts = encode_pcm_stereo_long_block_adts_with_scale_factors(
            AdtsConfig::aac_lc(44_100, 2),
            AacScaleFactorChannel::new(AacLongBlockConfig::new(0, 1), &[0]),
            AacScaleFactorChannel::new(AacLongBlockConfig::new(0, 1), &[0]),
            &pcm,
            AacPcmLongBlockConfig::new(0, 1.0, 1024),
            &[],
            AacSpectralMagnitudeTables::default(),
        )
        .unwrap();

        assert_eq!(
            adts,
            [
                0xff, 0xf1, 0x50, 0x80, 0x02, 0x3f, 0xfc, 0x20, 0x00, 0x00, 0x40, 0x10, 0x00, 0x00,
                0x80, 0x23, 0x80,
            ]
        );
        assert!(encode_pcm_stereo_long_block_adts_with_scale_factors(
            AdtsConfig::aac_lc(44_100, 2),
            AacScaleFactorChannel::new(AacLongBlockConfig::new(0, 1), &[0]),
            AacScaleFactorChannel::new(AacLongBlockConfig::new(0, 1), &[0]),
            &AudioBuffer::new(44_100, 1, vec![0.0; 2048]).unwrap(),
            AacPcmLongBlockConfig::new(0, 1.0, 1024),
            &[],
            AacSpectralMagnitudeTables::default(),
        )
        .is_err());
    }

    #[test]
    fn encodes_pcm_stereo_long_block_with_selected_scale_factors_as_adts() {
        let pcm = AudioBuffer::new(44_100, 2, vec![0.0; 4096]).unwrap();

        let selected = encode_pcm_stereo_long_block_adts_with_selected_scale_factors(
            AdtsConfig::aac_lc(44_100, 2),
            AacLongBlockConfig::new(0, 1),
            AacLongBlockConfig::new(0, 1),
            &pcm,
            AacPcmLongBlockConfig::new(0, 1.0, 1024),
            &[],
            AacSpectralMagnitudeTables::default(),
        )
        .unwrap();
        let manual = encode_pcm_stereo_long_block_adts_with_scale_factors(
            AdtsConfig::aac_lc(44_100, 2),
            AacScaleFactorChannel::new(AacLongBlockConfig::new(0, 1), &[0]),
            AacScaleFactorChannel::new(AacLongBlockConfig::new(0, 1), &[0]),
            &pcm,
            AacPcmLongBlockConfig::new(0, 1.0, 1024),
            &[],
            AacSpectralMagnitudeTables::default(),
        )
        .unwrap();

        assert_eq!(selected, manual);
        assert!(
            encode_pcm_stereo_long_block_adts_with_selected_scale_factors(
                AdtsConfig::aac_lc(44_100, 2),
                AacLongBlockConfig::new(0, 1),
                AacLongBlockConfig::new(0, 1),
                &AudioBuffer::new(44_100, 1, vec![0.0; 2048]).unwrap(),
                AacPcmLongBlockConfig::new(0, 1.0, 1024),
                &[],
                AacSpectralMagnitudeTables::default(),
            )
            .is_err()
        );
    }

    #[test]
    fn encodes_pcm_stereo_long_block_with_bit_cost_sections_as_adts() {
        let pcm = AudioBuffer::new(44_100, 2, vec![0.0; 4096]).unwrap();

        let encoded = encode_pcm_stereo_long_block_adts_by_bit_cost(
            AdtsConfig::aac_lc(44_100, 2),
            AacLongBlockConfig::new(0, 1),
            AacLongBlockConfig::new(0, 1),
            &pcm,
            AacPcmLongBlockConfig::new(0, 1.0, 1024),
            AacSpectralMagnitudeTables::default(),
        )
        .unwrap();
        let with_scale_factors = encode_pcm_stereo_long_block_adts_with_scale_factors_by_bit_cost(
            AdtsConfig::aac_lc(44_100, 2),
            AacScaleFactorChannel::new(AacLongBlockConfig::new(0, 1), &[0]),
            AacScaleFactorChannel::new(AacLongBlockConfig::new(0, 1), &[0]),
            &pcm,
            AacPcmLongBlockConfig::new(0, 1.0, 1024),
            &[],
            AacSpectralMagnitudeTables::default(),
        )
        .unwrap();
        let selected = encode_pcm_stereo_long_block_adts_with_selected_scale_factors_by_bit_cost(
            AdtsConfig::aac_lc(44_100, 2),
            AacLongBlockConfig::new(0, 1),
            AacLongBlockConfig::new(0, 1),
            &pcm,
            AacPcmLongBlockConfig::new(0, 1.0, 1024),
            &[],
            AacSpectralMagnitudeTables::default(),
        )
        .unwrap();

        assert_eq!(
            encoded,
            [
                0xff, 0xf1, 0x50, 0x80, 0x02, 0x3f, 0xfc, 0x20, 0x00, 0x00, 0x40, 0x10, 0x00, 0x00,
                0x80, 0x23, 0x80,
            ]
        );
        assert_eq!(with_scale_factors, encoded);
        assert_eq!(selected, with_scale_factors);
        assert!(encode_pcm_stereo_long_block_adts_by_bit_cost(
            AdtsConfig::aac_lc(44_100, 2),
            AacLongBlockConfig::new(0, 1),
            AacLongBlockConfig::new(0, 1),
            &AudioBuffer::new(44_100, 1, vec![0.0; 2048]).unwrap(),
            AacPcmLongBlockConfig::new(0, 1.0, 1024),
            AacSpectralMagnitudeTables::default(),
        )
        .is_err());
    }

    #[test]
    fn encodes_pcm_mono_long_block_adts_stream() {
        let pcm = AudioBuffer::new(44_100, 1, vec![0.0; 2048]).unwrap();
        let frame = [
            0xff, 0xf1, 0x50, 0x40, 0x01, 0xbf, 0xfc, 0x00, 0x00, 0x00, 0x80, 0x23, 0x80,
        ];

        let adts = encode_pcm_mono_long_block_adts_stream(
            AdtsConfig::aac_lc(44_100, 1),
            AacLongBlockConfig::new(0, 1),
            &pcm,
            AacPcmLongBlockConfig::new(0, 1.0, 1024),
            AacSpectralMagnitudeTables::default(),
        )
        .unwrap();

        assert_eq!(adts, [frame, frame].concat());
        assert!(encode_pcm_mono_long_block_adts_stream(
            AdtsConfig::aac_lc(44_100, 1),
            AacLongBlockConfig::new(0, 1),
            &pcm,
            AacPcmLongBlockConfig::new(4096, 1.0, 1024),
            AacSpectralMagnitudeTables::default(),
        )
        .is_err());
    }

    #[test]
    fn encodes_pcm_mono_long_block_adts_stream_with_scale_factors() {
        let pcm = AudioBuffer::new(44_100, 1, vec![0.0; 2048]).unwrap();
        let frame = [
            0xff, 0xf1, 0x50, 0x40, 0x01, 0xbf, 0xfc, 0x00, 0x00, 0x00, 0x80, 0x23, 0x80,
        ];
        let scale_factors_by_frame: [&[i16]; 2] = [&[0], &[0]];

        let adts = encode_pcm_mono_long_block_adts_stream_with_scale_factors(
            AdtsConfig::aac_lc(44_100, 1),
            AacScaleFactorSequence::new(AacLongBlockConfig::new(0, 1), &scale_factors_by_frame),
            &pcm,
            AacPcmLongBlockConfig::new(0, 1.0, 1024),
            &[],
            AacSpectralMagnitudeTables::default(),
        )
        .unwrap();

        assert_eq!(adts, [frame, frame].concat());
        assert!(encode_pcm_mono_long_block_adts_stream_with_scale_factors(
            AdtsConfig::aac_lc(44_100, 1),
            AacScaleFactorSequence::new(AacLongBlockConfig::new(0, 1), &[&[0]]),
            &pcm,
            AacPcmLongBlockConfig::new(0, 1.0, 1024),
            &[],
            AacSpectralMagnitudeTables::default(),
        )
        .is_err());
    }

    #[test]
    fn encodes_pcm_mono_long_block_adts_stream_with_selected_scale_factors() {
        let pcm = AudioBuffer::new(44_100, 1, vec![0.0; 2048]).unwrap();
        let frame = [
            0xff, 0xf1, 0x50, 0x40, 0x01, 0xbf, 0xfc, 0x00, 0x00, 0x00, 0x80, 0x23, 0x80,
        ];

        let adts = encode_pcm_mono_long_block_adts_stream_with_selected_scale_factors(
            AdtsConfig::aac_lc(44_100, 1),
            AacLongBlockConfig::new(0, 1),
            &pcm,
            AacPcmLongBlockConfig::new(0, 1.0, 1024),
            &[],
            AacSpectralMagnitudeTables::default(),
        )
        .unwrap();

        assert_eq!(adts, [frame, frame].concat());
        assert!(
            encode_pcm_mono_long_block_adts_stream_with_selected_scale_factors(
                AdtsConfig::aac_lc(44_100, 1),
                AacLongBlockConfig::new(0, 1),
                &pcm,
                AacPcmLongBlockConfig::new(4096, 1.0, 1024),
                &[],
                AacSpectralMagnitudeTables::default(),
            )
            .is_err()
        );
    }

    #[test]
    fn encodes_pcm_mono_long_block_adts_stream_with_bit_cost_sections() {
        let pcm = AudioBuffer::new(44_100, 1, vec![0.0; 2048]).unwrap();
        let frame = [
            0xff, 0xf1, 0x50, 0x40, 0x01, 0xbf, 0xfc, 0x00, 0x00, 0x00, 0x80, 0x23, 0x80,
        ];
        let scale_factors_by_frame: [&[i16]; 2] = [&[0], &[0]];

        let encoded = encode_pcm_mono_long_block_adts_stream_by_bit_cost(
            AdtsConfig::aac_lc(44_100, 1),
            AacLongBlockConfig::new(0, 1),
            &pcm,
            AacPcmLongBlockConfig::new(0, 1.0, 1024),
            AacSpectralMagnitudeTables::default(),
        )
        .unwrap();
        let with_scale_factors =
            encode_pcm_mono_long_block_adts_stream_with_scale_factors_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 1),
                AacScaleFactorSequence::new(AacLongBlockConfig::new(0, 1), &scale_factors_by_frame),
                &pcm,
                AacPcmLongBlockConfig::new(0, 1.0, 1024),
                &[],
                AacSpectralMagnitudeTables::default(),
            )
            .unwrap();
        let selected =
            encode_pcm_mono_long_block_adts_stream_with_selected_scale_factors_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 1),
                AacLongBlockConfig::new(0, 1),
                &pcm,
                AacPcmLongBlockConfig::new(0, 1.0, 1024),
                &[],
                AacSpectralMagnitudeTables::default(),
            )
            .unwrap();

        assert_eq!(encoded, [frame, frame].concat());
        assert_eq!(with_scale_factors, encoded);
        assert_eq!(selected, with_scale_factors);
        assert!(
            encode_pcm_mono_long_block_adts_stream_with_scale_factors_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 1),
                AacScaleFactorSequence::new(AacLongBlockConfig::new(0, 1), &[&[0]]),
                &pcm,
                AacPcmLongBlockConfig::new(0, 1.0, 1024),
                &[],
                AacSpectralMagnitudeTables::default(),
            )
            .is_err()
        );
    }

    #[test]
    fn selects_mono_pcm_frame_step_for_experimental_nonzero_payload() {
        let pcm = AudioBuffer::new(
            44_100,
            1,
            (0..2048)
                .map(|sample| ((sample as f32) * 0.01).sin() * 0.25)
                .collect(),
        )
        .unwrap();
        let channel = AacLongBlockConfig::new(0, 1);
        let scale_factor_table = experimental_aac_scale_factor_delta_table();
        let spectral_tables = experimental_unit_magnitude_spectral_tables();

        let step = select_aac_lc_mono_pcm_frame_step_by_bit_cost(
            AdtsConfig::aac_lc(44_100, 1),
            channel,
            &pcm,
            AacPcmStepSearchConfig::new(
                0,
                1024,
                AAC_LC_PCM_STEP_CANDIDATES,
                &scale_factor_table,
                spectral_tables,
            ),
        )
        .unwrap();
        let reversed_candidates = AAC_LC_PCM_STEP_CANDIDATES
            .iter()
            .copied()
            .rev()
            .collect::<Vec<_>>();
        let details: AacPcmFrameStepSelection =
            select_aac_lc_mono_pcm_frame_step_details_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 1),
                channel,
                &pcm,
                AacPcmStepSearchConfig::new(
                    0,
                    1024,
                    &reversed_candidates,
                    &scale_factor_table,
                    spectral_tables,
                ),
            )
            .unwrap();
        let auto = encode_pcm_mono_long_block_adts_stream_with_auto_step_by_bit_cost(
            AdtsConfig::aac_lc(44_100, 1),
            channel,
            &pcm,
            AacPcmStepSearchConfig::new(
                0,
                1024,
                AAC_LC_PCM_STEP_CANDIDATES,
                &scale_factor_table,
                spectral_tables,
            ),
        )
        .unwrap();
        let selected =
            encode_pcm_mono_long_block_adts_stream_with_selected_scale_factors_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 1),
                channel,
                &pcm,
                AacPcmLongBlockConfig::new(0, step, 1024),
                &scale_factor_table,
                spectral_tables,
            )
            .unwrap();
        let public_scaffold = encode(&pcm).unwrap();
        let offsets = aac_lc_long_window_scale_factor_band_offsets(44_100).unwrap();
        let production_channel = AacLongBlockConfig::new(180, (offsets.len() - 1) as u8);
        let production_bitrate = aac_lc_default_production_bitrate_bps(1).unwrap();
        let production =
            encode_pcm_mono_long_block_adts_stream_with_offsets_and_selected_scale_factors_and_bitrate_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 1),
                production_channel,
                &pcm,
                offsets,
                AAC_LC_PCM_STEP_CANDIDATES,
                production_bitrate,
                &aac_scale_factor_delta_table(),
                aac_lc_standard_spectral_tables(),
            )
            .unwrap();
        let zero_payload =
            encode_pcm_mono_long_block_adts_stream_with_selected_scale_factors_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 1),
                channel,
                &pcm,
                AacPcmLongBlockConfig::new(0, f32::MAX, 1024),
                &[],
                AacSpectralMagnitudeTables::default(),
            )
            .unwrap();

        assert!(step < f32::MAX);
        assert_eq!(details.step, step);
        assert!(details.frame_len > 0);
        assert!(details.frame_len <= details.frame_capacity_bytes);
        assert_eq!(auto, selected);
        assert_eq!(&public_scaffold[..2], &[0xff, 0xf1]);
        assert_eq!(public_scaffold, production);
        assert_ne!(public_scaffold, zero_payload);
        assert_ne!(auto, zero_payload);
    }

    #[test]
    fn selects_mono_pcm_frame_step_with_max_frame_len() {
        let pcm = AudioBuffer::new(
            44_100,
            1,
            (0..2048)
                .map(|sample| ((sample as f32) * 0.01).sin() * 0.25)
                .collect(),
        )
        .unwrap();
        let channel = AacLongBlockConfig::new(0, 1);
        let scale_factor_table = experimental_aac_scale_factor_delta_table();
        let spectral_tables = experimental_unit_magnitude_spectral_tables();
        let fallback_candidate = [f32::MAX];
        let unconstrained = select_aac_lc_mono_pcm_frame_step_details_by_bit_cost(
            AdtsConfig::aac_lc(44_100, 1),
            channel,
            &pcm,
            AacPcmStepSearchConfig::new(
                0,
                1024,
                AAC_LC_PCM_STEP_CANDIDATES,
                &scale_factor_table,
                spectral_tables,
            ),
        )
        .unwrap();
        let fallback = select_aac_lc_mono_pcm_frame_step_details_by_bit_cost(
            AdtsConfig::aac_lc(44_100, 1),
            channel,
            &pcm,
            AacPcmStepSearchConfig::new(
                0,
                1024,
                &fallback_candidate,
                &scale_factor_table,
                spectral_tables,
            ),
        )
        .unwrap();

        let step = select_aac_lc_mono_pcm_frame_step_with_max_frame_len_by_bit_cost(
            AdtsConfig::aac_lc(44_100, 1),
            channel,
            &pcm,
            AacPcmStepSearchConfig::new(
                0,
                1024,
                AAC_LC_PCM_STEP_CANDIDATES,
                &scale_factor_table,
                spectral_tables,
            ),
            fallback.frame_len,
        )
        .unwrap();
        let details = select_aac_lc_mono_pcm_frame_step_details_with_max_frame_len_by_bit_cost(
            AdtsConfig::aac_lc(44_100, 1),
            channel,
            &pcm,
            AacPcmStepSearchConfig::new(
                0,
                1024,
                AAC_LC_PCM_STEP_CANDIDATES,
                &scale_factor_table,
                spectral_tables,
            ),
            fallback.frame_len,
        )
        .unwrap();

        assert_eq!(step, details.step);
        assert!(details.step > unconstrained.step);
        assert_eq!(details.frame_capacity_bytes, fallback.frame_len);
        assert!(details.frame_len <= fallback.frame_len);

        // When the budget is tighter than even the smallest achievable frame,
        // the search degrades gracefully: rather than failing the whole encode
        // it returns a best-effort, over-budget frame (the coarsest quantizer
        // step, i.e. the smallest frame the candidate list can produce). The
        // declared capacity is clamped to the requested budget.
        let best_effort = select_aac_lc_mono_pcm_frame_step_details_with_max_frame_len_by_bit_cost(
            AdtsConfig::aac_lc(44_100, 1),
            channel,
            &pcm,
            AacPcmStepSearchConfig::new(
                0,
                1024,
                AAC_LC_PCM_STEP_CANDIDATES,
                &scale_factor_table,
                spectral_tables,
            ),
            fallback.frame_len - 1,
        )
        .unwrap();
        assert_eq!(best_effort.frame_capacity_bytes, fallback.frame_len - 1);
        assert!(best_effort.frame_len > fallback.frame_len - 1);
        assert!(best_effort.step >= details.step);
    }

    #[test]
    fn selects_production_offsets_pcm_frame_step_with_max_frame_len() {
        let mono = AudioBuffer::new(
            44_100,
            1,
            (0..2048)
                .map(|sample| ((sample as f32) * 0.01).sin() * 0.25)
                .collect(),
        )
        .unwrap();
        let mut stereo_samples = Vec::new();
        for sample in 0..2048 {
            stereo_samples.push(((sample as f32) * 0.01).sin() * 0.25);
            stereo_samples.push(((sample as f32) * 0.013).cos() * 0.20);
        }
        let stereo = AudioBuffer::new(44_100, 2, stereo_samples).unwrap();
        let offsets = aac_lc_long_window_scale_factor_band_offsets(44_100).unwrap();
        let channel_config = AacLongBlockConfig::new(180, (offsets.len() - 1) as u8);
        let scale_factors = vec![i16::from(channel_config.global_gain); offsets.len() - 1];
        let channel = AacScaleFactorChannel::new(channel_config, &scale_factors);
        let scale_factor_table = aac_scale_factor_delta_zero_table();
        let spectral_tables = aac_unsigned_pairs7_unit_magnitude_spectral_tables();
        let default_mono_bitrate = aac_lc_default_production_bitrate_bps(1).unwrap();
        let default_stereo_bitrate = aac_lc_default_production_bitrate_bps(2).unwrap();

        assert_eq!(default_mono_bitrate, 128_000);
        assert_eq!(default_stereo_bitrate, 256_000);
        assert!(
            aac_lc_adts_max_frame_len_for_bitrate(44_100, default_mono_bitrate).unwrap()
                >= AAC_ADTS_HEADER_LEN
        );
        assert!(aac_lc_default_production_bitrate_bps(0).is_err());
        assert!(aac_lc_default_production_bitrate_bps(3).is_err());

        let mono_unconstrained =
            select_aac_lc_mono_pcm_frame_step_details_with_offsets_and_scale_factors_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 1),
                channel,
                &mono,
                0,
                offsets,
                AAC_LC_PCM_STEP_CANDIDATES,
                scale_factor_table,
                spectral_tables,
            )
            .unwrap();
        let mono_min_frame_len = AAC_LC_PCM_STEP_CANDIDATES
            .iter()
            .copied()
            .filter_map(|candidate| {
                select_aac_lc_mono_pcm_frame_step_details_with_offsets_and_scale_factors_by_bit_cost(
                    AdtsConfig::aac_lc(44_100, 1),
                    channel,
                    &mono,
                    0,
                    offsets,
                    &[candidate],
                    scale_factor_table,
                    spectral_tables,
                )
                .ok()
            })
            .map(|selection| selection.frame_len)
            .min()
            .unwrap();
        let mono_step =
            select_aac_lc_mono_pcm_frame_step_with_offsets_and_scale_factors_and_max_frame_len_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 1),
                channel,
                &mono,
                0,
                offsets,
                AAC_LC_PCM_STEP_CANDIDATES,
                mono_unconstrained.frame_len,
                scale_factor_table,
                spectral_tables,
            )
            .unwrap();
        let mono_details =
            select_aac_lc_mono_pcm_frame_step_details_with_offsets_and_scale_factors_and_max_frame_len_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 1),
                channel,
                &mono,
                0,
                offsets,
                AAC_LC_PCM_STEP_CANDIDATES,
                mono_unconstrained.frame_len,
                scale_factor_table,
                spectral_tables,
            )
            .unwrap();
        let mono_encoded =
            encode_pcm_mono_long_block_adts_stream_with_offsets_and_scale_factors_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 1),
                channel,
                &mono,
                offsets,
                AAC_LC_PCM_STEP_CANDIDATES,
                scale_factor_table,
                spectral_tables,
            )
            .unwrap();
        let mono_budget_encoded =
            encode_pcm_mono_long_block_adts_stream_with_offsets_and_scale_factors_and_max_frame_len_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 1),
                channel,
                &mono,
                offsets,
                AAC_LC_PCM_STEP_CANDIDATES,
                max_adts_frame_len(&mono_encoded),
                scale_factor_table,
                spectral_tables,
            )
            .unwrap();
        let mono_target_bitrate =
            ((max_adts_frame_len(&mono_encoded) as u64 * 8 * 44_100).div_ceil(1024)) as u32;
        let mono_bitrate_budget =
            aac_lc_adts_max_frame_len_for_bitrate(44_100, mono_target_bitrate).unwrap();
        let mono_bitrate_encoded =
            encode_pcm_mono_long_block_adts_stream_with_offsets_and_scale_factors_and_bitrate_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 1),
                channel,
                &mono,
                offsets,
                AAC_LC_PCM_STEP_CANDIDATES,
                mono_target_bitrate,
                scale_factor_table,
                spectral_tables,
            )
            .unwrap();
        let selected_scale_factor_table = aac_scale_factor_delta_table();
        let mono_selected_unconstrained =
            select_aac_lc_mono_pcm_frame_step_details_with_offsets_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 1),
                channel_config,
                &mono,
                0,
                offsets,
                AAC_LC_PCM_STEP_CANDIDATES,
                &selected_scale_factor_table,
                spectral_tables,
            )
            .unwrap();
        let mono_selected_step =
            select_aac_lc_mono_pcm_frame_step_with_offsets_and_max_frame_len_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 1),
                channel_config,
                &mono,
                0,
                offsets,
                AAC_LC_PCM_STEP_CANDIDATES,
                mono_selected_unconstrained.frame_len,
                &selected_scale_factor_table,
                spectral_tables,
            )
            .unwrap();
        let mono_selected_details =
            select_aac_lc_mono_pcm_frame_step_details_with_offsets_and_max_frame_len_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 1),
                channel_config,
                &mono,
                0,
                offsets,
                AAC_LC_PCM_STEP_CANDIDATES,
                mono_selected_unconstrained.frame_len,
                &selected_scale_factor_table,
                spectral_tables,
            )
            .unwrap();
        let mono_selected_encoded =
            encode_pcm_mono_long_block_adts_stream_with_offsets_and_selected_scale_factors_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 1),
                channel_config,
                &mono,
                offsets,
                AAC_LC_PCM_STEP_CANDIDATES,
                &selected_scale_factor_table,
                spectral_tables,
            )
            .unwrap();
        let mono_selected_budget_encoded =
            encode_pcm_mono_long_block_adts_stream_with_offsets_and_selected_scale_factors_and_max_frame_len_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 1),
                channel_config,
                &mono,
                offsets,
                AAC_LC_PCM_STEP_CANDIDATES,
                max_adts_frame_len(&mono_selected_encoded),
                &selected_scale_factor_table,
                spectral_tables,
            )
            .unwrap();
        let mono_selected_target_bitrate =
            ((max_adts_frame_len(&mono_selected_encoded) as u64 * 8 * 44_100).div_ceil(1024))
                as u32;
        let mono_selected_bitrate_budget =
            aac_lc_adts_max_frame_len_for_bitrate(44_100, mono_selected_target_bitrate).unwrap();
        let mono_selected_bitrate_encoded =
            encode_pcm_mono_long_block_adts_stream_with_offsets_and_selected_scale_factors_and_bitrate_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 1),
                channel_config,
                &mono,
                offsets,
                AAC_LC_PCM_STEP_CANDIDATES,
                mono_selected_target_bitrate,
                &selected_scale_factor_table,
                spectral_tables,
            )
            .unwrap();

        assert_eq!(mono_step, mono_unconstrained.step);
        assert_eq!(mono_details.step, mono_unconstrained.step);
        assert_eq!(
            mono_details.frame_capacity_bytes,
            mono_unconstrained.frame_len
        );
        assert_eq!(mono_budget_encoded, mono_encoded);
        assert!(max_adts_frame_len(&mono_bitrate_encoded) <= mono_bitrate_budget);
        assert_eq!(mono_selected_details.step, mono_selected_unconstrained.step);
        assert_eq!(mono_selected_step, mono_selected_unconstrained.step);
        assert_eq!(mono_selected_budget_encoded, mono_selected_encoded);
        assert!(max_adts_frame_len(&mono_selected_bitrate_encoded) <= mono_selected_bitrate_budget);
        // Budget below the smallest achievable frame → best-effort fallback to
        // the coarsest step (smallest, over-budget frame) instead of an error.
        let mono_best_effort =
            select_aac_lc_mono_pcm_frame_step_details_with_offsets_and_scale_factors_and_max_frame_len_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 1),
                channel,
                &mono,
                0,
                offsets,
                AAC_LC_PCM_STEP_CANDIDATES,
                mono_min_frame_len - 1,
                scale_factor_table,
                spectral_tables,
            )
            .unwrap();
        assert_eq!(mono_best_effort.frame_len, mono_min_frame_len);
        assert_eq!(mono_best_effort.frame_capacity_bytes, mono_min_frame_len - 1);

        let stereo_unconstrained =
            select_aac_lc_stereo_pcm_frame_step_details_with_offsets_and_scale_factors_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 2),
                channel,
                channel,
                &stereo,
                0,
                offsets,
                AAC_LC_PCM_STEP_CANDIDATES,
                scale_factor_table,
                spectral_tables,
            )
            .unwrap();
        let stereo_min_frame_len = AAC_LC_PCM_STEP_CANDIDATES
            .iter()
            .copied()
            .filter_map(|candidate| {
                select_aac_lc_stereo_pcm_frame_step_details_with_offsets_and_scale_factors_by_bit_cost(
                    AdtsConfig::aac_lc(44_100, 2),
                    channel,
                    channel,
                    &stereo,
                    0,
                    offsets,
                    &[candidate],
                    scale_factor_table,
                    spectral_tables,
                )
                .ok()
            })
            .map(|selection| selection.frame_len)
            .min()
            .unwrap();
        let stereo_step =
            select_aac_lc_stereo_pcm_frame_step_with_offsets_and_scale_factors_and_max_frame_len_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 2),
                channel,
                channel,
                &stereo,
                0,
                offsets,
                AAC_LC_PCM_STEP_CANDIDATES,
                stereo_unconstrained.frame_len,
                scale_factor_table,
                spectral_tables,
            )
            .unwrap();
        let stereo_details =
            select_aac_lc_stereo_pcm_frame_step_details_with_offsets_and_scale_factors_and_max_frame_len_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 2),
                channel,
                channel,
                &stereo,
                0,
                offsets,
                AAC_LC_PCM_STEP_CANDIDATES,
                stereo_unconstrained.frame_len,
                scale_factor_table,
                spectral_tables,
            )
            .unwrap();
        let stereo_encoded =
            encode_pcm_stereo_long_block_adts_stream_with_offsets_and_scale_factors_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 2),
                channel,
                channel,
                &stereo,
                offsets,
                AAC_LC_PCM_STEP_CANDIDATES,
                scale_factor_table,
                spectral_tables,
            )
            .unwrap();
        let stereo_budget_encoded =
            encode_pcm_stereo_long_block_adts_stream_with_offsets_and_scale_factors_and_max_frame_len_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 2),
                channel,
                channel,
                &stereo,
                offsets,
                AAC_LC_PCM_STEP_CANDIDATES,
                max_adts_frame_len(&stereo_encoded),
                scale_factor_table,
                spectral_tables,
            )
            .unwrap();
        let stereo_target_bitrate =
            ((max_adts_frame_len(&stereo_encoded) as u64 * 8 * 44_100).div_ceil(1024)) as u32;
        let stereo_bitrate_budget =
            aac_lc_adts_max_frame_len_for_bitrate(44_100, stereo_target_bitrate).unwrap();
        let stereo_bitrate_encoded =
            encode_pcm_stereo_long_block_adts_stream_with_offsets_and_scale_factors_and_bitrate_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 2),
                channel,
                channel,
                &stereo,
                offsets,
                AAC_LC_PCM_STEP_CANDIDATES,
                stereo_target_bitrate,
                scale_factor_table,
                spectral_tables,
            )
            .unwrap();
        let stereo_selected_unconstrained =
            select_aac_lc_stereo_pcm_frame_step_details_with_offsets_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 2),
                channel_config,
                channel_config,
                &stereo,
                0,
                offsets,
                AAC_LC_PCM_STEP_CANDIDATES,
                &selected_scale_factor_table,
                spectral_tables,
            )
            .unwrap();
        let stereo_selected_unconstrained_step =
            select_aac_lc_stereo_pcm_frame_step_with_offsets_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 2),
                channel_config,
                channel_config,
                &stereo,
                0,
                offsets,
                AAC_LC_PCM_STEP_CANDIDATES,
                &selected_scale_factor_table,
                spectral_tables,
            )
            .unwrap();
        let stereo_selected_step =
            select_aac_lc_stereo_pcm_frame_step_with_offsets_and_max_frame_len_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 2),
                channel_config,
                channel_config,
                &stereo,
                0,
                offsets,
                AAC_LC_PCM_STEP_CANDIDATES,
                stereo_selected_unconstrained.frame_len,
                &selected_scale_factor_table,
                spectral_tables,
            )
            .unwrap();
        let stereo_selected_details =
            select_aac_lc_stereo_pcm_frame_step_details_with_offsets_and_max_frame_len_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 2),
                channel_config,
                channel_config,
                &stereo,
                0,
                offsets,
                AAC_LC_PCM_STEP_CANDIDATES,
                stereo_selected_unconstrained.frame_len,
                &selected_scale_factor_table,
                spectral_tables,
            )
            .unwrap();
        let stereo_selected_encoded =
            encode_pcm_stereo_long_block_adts_stream_with_offsets_and_selected_scale_factors_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 2),
                channel_config,
                channel_config,
                &stereo,
                offsets,
                AAC_LC_PCM_STEP_CANDIDATES,
                &selected_scale_factor_table,
                spectral_tables,
            )
            .unwrap();
        let stereo_selected_budget_encoded =
            encode_pcm_stereo_long_block_adts_stream_with_offsets_and_selected_scale_factors_and_max_frame_len_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 2),
                channel_config,
                channel_config,
                &stereo,
                offsets,
                AAC_LC_PCM_STEP_CANDIDATES,
                max_adts_frame_len(&stereo_selected_encoded),
                &selected_scale_factor_table,
                spectral_tables,
            )
            .unwrap();
        let stereo_selected_target_bitrate =
            ((max_adts_frame_len(&stereo_selected_encoded) as u64 * 8 * 44_100).div_ceil(1024))
                as u32;
        let stereo_selected_bitrate_budget =
            aac_lc_adts_max_frame_len_for_bitrate(44_100, stereo_selected_target_bitrate).unwrap();
        let stereo_selected_bitrate_encoded =
            encode_pcm_stereo_long_block_adts_stream_with_offsets_and_selected_scale_factors_and_bitrate_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 2),
                channel_config,
                channel_config,
                &stereo,
                offsets,
                AAC_LC_PCM_STEP_CANDIDATES,
                stereo_selected_target_bitrate,
                &selected_scale_factor_table,
                spectral_tables,
            )
            .unwrap();

        assert_eq!(stereo_step, stereo_unconstrained.step);
        assert_eq!(stereo_details.step, stereo_unconstrained.step);
        assert_eq!(
            stereo_details.frame_capacity_bytes,
            stereo_unconstrained.frame_len
        );
        assert_eq!(stereo_budget_encoded, stereo_encoded);
        assert!(max_adts_frame_len(&stereo_bitrate_encoded) <= stereo_bitrate_budget);
        assert_eq!(
            stereo_selected_details.step,
            stereo_selected_unconstrained.step
        );
        assert_eq!(
            stereo_selected_unconstrained_step,
            stereo_selected_unconstrained.step
        );
        assert_eq!(stereo_selected_step, stereo_selected_unconstrained.step);
        assert_eq!(stereo_selected_budget_encoded, stereo_selected_encoded);
        assert!(
            max_adts_frame_len(&stereo_selected_bitrate_encoded) <= stereo_selected_bitrate_budget
        );
        assert!(aac_lc_adts_max_frame_len_for_bitrate(44_100, 1).is_err());
        // Budget below the smallest achievable frame → best-effort fallback.
        let stereo_best_effort =
            select_aac_lc_stereo_pcm_frame_step_details_with_offsets_and_scale_factors_and_max_frame_len_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 2),
                channel,
                channel,
                &stereo,
                0,
                offsets,
                AAC_LC_PCM_STEP_CANDIDATES,
                stereo_min_frame_len - 1,
                scale_factor_table,
                spectral_tables,
            )
            .unwrap();
        assert_eq!(stereo_best_effort.frame_len, stereo_min_frame_len);
        assert_eq!(stereo_best_effort.frame_capacity_bytes, stereo_min_frame_len - 1);
    }

