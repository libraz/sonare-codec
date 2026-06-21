    #[test]
    fn selects_stereo_pcm_frame_step_for_experimental_nonzero_payload() {
        let mut samples = Vec::new();
        for sample in 0..2048 {
            samples.push(((sample as f32) * 0.01).sin() * 0.25);
            samples.push(((sample as f32) * 0.013).cos() * 0.20);
        }
        let pcm = AudioBuffer::new(44_100, 2, samples).unwrap();
        let channel = AacLongBlockConfig::new(0, 1);
        let scale_factor_table = experimental_aac_scale_factor_delta_table();
        let spectral_tables = experimental_unit_magnitude_spectral_tables();

        let step = select_aac_lc_stereo_pcm_frame_step_by_bit_cost(
            AdtsConfig::aac_lc(44_100, 2),
            channel,
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
            select_aac_lc_stereo_pcm_frame_step_details_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 2),
                channel,
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
        let auto = encode_pcm_stereo_long_block_adts_stream_with_auto_step_by_bit_cost(
            AdtsConfig::aac_lc(44_100, 2),
            channel,
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
            encode_pcm_stereo_long_block_adts_stream_with_selected_scale_factors_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 2),
                channel,
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
        let production_bitrate = aac_lc_default_production_bitrate_bps(2).unwrap();
        let production_budget =
            aac_lc_adts_max_frame_len_for_bitrate(44_100, production_bitrate).unwrap();
        let production_step =
            select_aac_lc_stereo_pcm_frame_step_with_offsets_and_max_frame_len_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 2),
                production_channel,
                production_channel,
                &pcm,
                0,
                offsets,
                AAC_LC_PCM_STEP_CANDIDATES,
                production_budget,
                &aac_scale_factor_delta_table(),
                aac_lc_standard_spectral_tables(),
            )
            .unwrap();
        let production_details =
            select_aac_lc_stereo_pcm_frame_step_details_with_offsets_and_max_frame_len_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 2),
                production_channel,
                production_channel,
                &pcm,
                0,
                offsets,
                &reversed_candidates,
                production_budget,
                &aac_scale_factor_delta_table(),
                aac_lc_standard_spectral_tables(),
            )
            .unwrap();
        let production =
            encode_pcm_stereo_long_block_adts_stream_with_offsets_and_selected_scale_factors_and_bitrate_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 2),
                production_channel,
                production_channel,
                &pcm,
                offsets,
                AAC_LC_PCM_STEP_CANDIDATES,
                production_bitrate,
                &aac_scale_factor_delta_table(),
                aac_lc_standard_spectral_tables(),
            )
            .unwrap();
        let production_reversed =
            encode_pcm_stereo_long_block_adts_stream_with_offsets_and_selected_scale_factors_and_bitrate_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 2),
                production_channel,
                production_channel,
                &pcm,
                offsets,
                &reversed_candidates,
                production_bitrate,
                &aac_scale_factor_delta_table(),
                aac_lc_standard_spectral_tables(),
            )
            .unwrap();
        let zero_payload =
            encode_pcm_stereo_long_block_adts_stream_with_selected_scale_factors_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 2),
                channel,
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
        assert_eq!(production_details.step, production_step);
        assert!(production_details.frame_len <= production_details.frame_capacity_bytes);
        assert_eq!(auto, selected);
        assert_eq!(public_scaffold, production);
        assert_eq!(production_reversed, production);
        assert_ne!(auto, zero_payload);
        assert_ne!(public_scaffold, zero_payload);
    }

    #[test]
    fn selects_stereo_pcm_frame_step_with_max_frame_len() {
        let mut samples = Vec::new();
        for sample in 0..2048 {
            samples.push(((sample as f32) * 0.01).sin() * 0.25);
            samples.push(((sample as f32) * 0.013).cos() * 0.20);
        }
        let pcm = AudioBuffer::new(44_100, 2, samples).unwrap();
        let channel = AacLongBlockConfig::new(0, 1);
        let scale_factor_table = experimental_aac_scale_factor_delta_table();
        let spectral_tables = experimental_unit_magnitude_spectral_tables();
        let fallback_candidate = [f32::MAX];
        let unconstrained = select_aac_lc_stereo_pcm_frame_step_details_by_bit_cost(
            AdtsConfig::aac_lc(44_100, 2),
            channel,
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
        let fallback = select_aac_lc_stereo_pcm_frame_step_details_by_bit_cost(
            AdtsConfig::aac_lc(44_100, 2),
            channel,
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

        let step = select_aac_lc_stereo_pcm_frame_step_with_max_frame_len_by_bit_cost(
            AdtsConfig::aac_lc(44_100, 2),
            channel,
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
        let details = select_aac_lc_stereo_pcm_frame_step_details_with_max_frame_len_by_bit_cost(
            AdtsConfig::aac_lc(44_100, 2),
            channel,
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
        assert!(
            select_aac_lc_stereo_pcm_frame_step_details_with_max_frame_len_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 2),
                channel,
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
            .is_err()
        );
    }

    #[test]
    fn encodes_pcm_stereo_long_block_adts_stream() {
        let pcm = AudioBuffer::new(44_100, 2, vec![0.0; 4096]).unwrap();
        let frame = [
            0xff, 0xf1, 0x50, 0x80, 0x02, 0x3f, 0xfc, 0x20, 0x00, 0x00, 0x40, 0x10, 0x00, 0x00,
            0x80, 0x23, 0x80,
        ];

        let adts = encode_pcm_stereo_long_block_adts_stream(
            AdtsConfig::aac_lc(44_100, 2),
            AacLongBlockConfig::new(0, 1),
            AacLongBlockConfig::new(0, 1),
            &pcm,
            AacPcmLongBlockConfig::new(0, 1.0, 1024),
            AacSpectralMagnitudeTables::default(),
        )
        .unwrap();

        assert_eq!(adts, [frame, frame].concat());
        assert!(encode_pcm_stereo_long_block_adts_stream(
            AdtsConfig::aac_lc(44_100, 2),
            AacLongBlockConfig::new(0, 1),
            AacLongBlockConfig::new(0, 1),
            &pcm,
            AacPcmLongBlockConfig::new(4096, 1.0, 1024),
            AacSpectralMagnitudeTables::default(),
        )
        .is_err());
    }

    #[test]
    fn encodes_pcm_stereo_long_block_adts_stream_with_scale_factors() {
        let pcm = AudioBuffer::new(44_100, 2, vec![0.0; 4096]).unwrap();
        let frame = [
            0xff, 0xf1, 0x50, 0x80, 0x02, 0x3f, 0xfc, 0x20, 0x00, 0x00, 0x40, 0x10, 0x00, 0x00,
            0x80, 0x23, 0x80,
        ];
        let scale_factors_by_frame: [&[i16]; 2] = [&[0], &[0]];

        let adts = encode_pcm_stereo_long_block_adts_stream_with_scale_factors(
            AdtsConfig::aac_lc(44_100, 2),
            AacScaleFactorSequence::new(AacLongBlockConfig::new(0, 1), &scale_factors_by_frame),
            AacScaleFactorSequence::new(AacLongBlockConfig::new(0, 1), &scale_factors_by_frame),
            &pcm,
            AacPcmLongBlockConfig::new(0, 1.0, 1024),
            &[],
            AacSpectralMagnitudeTables::default(),
        )
        .unwrap();

        assert_eq!(adts, [frame, frame].concat());
        assert!(encode_pcm_stereo_long_block_adts_stream_with_scale_factors(
            AdtsConfig::aac_lc(44_100, 2),
            AacScaleFactorSequence::new(AacLongBlockConfig::new(0, 1), &scale_factors_by_frame),
            AacScaleFactorSequence::new(AacLongBlockConfig::new(0, 1), &[&[0]]),
            &pcm,
            AacPcmLongBlockConfig::new(0, 1.0, 1024),
            &[],
            AacSpectralMagnitudeTables::default(),
        )
        .is_err());
    }

    #[test]
    fn encodes_pcm_stereo_long_block_adts_stream_with_selected_scale_factors() {
        let pcm = AudioBuffer::new(44_100, 2, vec![0.0; 4096]).unwrap();
        let frame = [
            0xff, 0xf1, 0x50, 0x80, 0x02, 0x3f, 0xfc, 0x20, 0x00, 0x00, 0x40, 0x10, 0x00, 0x00,
            0x80, 0x23, 0x80,
        ];

        let adts = encode_pcm_stereo_long_block_adts_stream_with_selected_scale_factors(
            AdtsConfig::aac_lc(44_100, 2),
            AacLongBlockConfig::new(0, 1),
            AacLongBlockConfig::new(0, 1),
            &pcm,
            AacPcmLongBlockConfig::new(0, 1.0, 1024),
            &[],
            AacSpectralMagnitudeTables::default(),
        )
        .unwrap();

        assert_eq!(adts, [frame, frame].concat());
        assert!(
            encode_pcm_stereo_long_block_adts_stream_with_selected_scale_factors(
                AdtsConfig::aac_lc(44_100, 2),
                AacLongBlockConfig::new(0, 1),
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
    fn encodes_pcm_stereo_long_block_adts_stream_with_bit_cost_sections() {
        let pcm = AudioBuffer::new(44_100, 2, vec![0.0; 4096]).unwrap();
        let frame = [
            0xff, 0xf1, 0x50, 0x80, 0x02, 0x3f, 0xfc, 0x20, 0x00, 0x00, 0x40, 0x10, 0x00, 0x00,
            0x80, 0x23, 0x80,
        ];
        let scale_factors_by_frame: [&[i16]; 2] = [&[0], &[0]];

        let encoded = encode_pcm_stereo_long_block_adts_stream_by_bit_cost(
            AdtsConfig::aac_lc(44_100, 2),
            AacLongBlockConfig::new(0, 1),
            AacLongBlockConfig::new(0, 1),
            &pcm,
            AacPcmLongBlockConfig::new(0, 1.0, 1024),
            AacSpectralMagnitudeTables::default(),
        )
        .unwrap();
        let with_scale_factors =
            encode_pcm_stereo_long_block_adts_stream_with_scale_factors_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 2),
                AacScaleFactorSequence::new(AacLongBlockConfig::new(0, 1), &scale_factors_by_frame),
                AacScaleFactorSequence::new(AacLongBlockConfig::new(0, 1), &scale_factors_by_frame),
                &pcm,
                AacPcmLongBlockConfig::new(0, 1.0, 1024),
                &[],
                AacSpectralMagnitudeTables::default(),
            )
            .unwrap();
        let selected =
            encode_pcm_stereo_long_block_adts_stream_with_selected_scale_factors_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 2),
                AacLongBlockConfig::new(0, 1),
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
            encode_pcm_stereo_long_block_adts_stream_with_scale_factors_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 2),
                AacScaleFactorSequence::new(AacLongBlockConfig::new(0, 1), &scale_factors_by_frame),
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
    fn rejects_unrepresentable_adts_config() {
        let err = frame_adts(AdtsConfig::aac_lc(44_123, 2), &[0x00]).unwrap_err();
        assert!(matches!(err, Error::UnsupportedFeature("AAC sample rate")));

        let err = frame_adts(AdtsConfig::aac_lc(44_100, 8), &[0x00]).unwrap_err();
        assert!(matches!(
            err,
            Error::InvalidInput("AAC ADTS channel count exceeds 7")
        ));
    }

    #[test]
    fn encodes_silent_mono_pcm_as_adts() {
        let pcm = AudioBuffer::new(44_100, 1, vec![0.0; 1024]).unwrap();

        let adts = encode(&pcm).unwrap();

        assert_eq!(&adts[..7], &[0xff, 0xf1, 0x50, 0x40, 0x01, 0x7f, 0xfc]);
        assert_eq!(&adts[7..], &[0x00, 0x00, 0x00, 0x07]);
    }

    #[test]
    fn encodes_silent_stereo_pcm_as_multiple_adts_frames() {
        let pcm = AudioBuffer::new(48_000, 2, vec![0.0; 1024 * 2 + 8]).unwrap();

        let adts = encode(&pcm).unwrap();

        assert_eq!(&adts[..7], &[0xff, 0xf1, 0x4c, 0x80, 0x01, 0xdf, 0xfc]);
        assert_eq!(adts[14], 0xff);
    }

    #[test]
    fn encodes_non_silent_mono_pcm_as_long_block_scaffold() {
        let pcm = AudioBuffer::new(44_100, 1, vec![0.0, 0.5]).unwrap();

        let adts = encode(&pcm).unwrap();

        assert_eq!(&adts[..2], &[0xff, 0xf1]);
        assert!(adts.len() > 7);
        assert_ne!(&adts[7..], &[0x00, 0x00, 0x00, 0x0e]);
    }

    #[test]
    fn decodes_explicit_zero_payload_mono_scaffold_as_zero_pcm() {
        let pcm = AudioBuffer::new(44_100, 1, vec![0.0, 0.5]).unwrap();
        let adts = encode_pcm_mono_long_block_adts_stream_with_selected_scale_factors_by_bit_cost(
            AdtsConfig::aac_lc(44_100, 1),
            AacLongBlockConfig::new(0, 1),
            &pcm,
            AacPcmLongBlockConfig::new(0, f32::MAX, 1024),
            &[],
            AacSpectralMagnitudeTables::default(),
        )
        .unwrap();

        let decoded = super::decode(&adts).unwrap();

        assert_eq!(decoded.sample_rate, 44_100);
        assert_eq!(decoded.channels, 1);
        assert_eq!(decoded.samples.len(), 1024);
        assert!(decoded.samples.iter().all(|sample| *sample == 0.0));
    }

    #[test]
    fn encodes_non_silent_stereo_pcm_as_long_block_scaffold() {
        let pcm = AudioBuffer::new(48_000, 2, vec![0.0, 0.25, 0.5, -0.25]).unwrap();

        let adts = encode(&pcm).unwrap();

        assert_eq!(&adts[..2], &[0xff, 0xf1]);
        assert!(adts.len() > 7);
        assert_eq!(adts[2] >> 2, 0x13);
    }

    #[test]
    fn decodes_own_silent_adts() {
        let pcm = AudioBuffer::new(44_100, 2, vec![0.0; 2048]).unwrap();
        let adts = encode(&pcm).unwrap();

        let decoded = super::decode(&adts).unwrap();

        assert_eq!(decoded.sample_rate, 44_100);
        assert_eq!(decoded.channels, 2);
        assert_eq!(decoded.samples.len(), 2048);
        assert!(decoded.samples.iter().all(|sample| *sample == 0.0));
    }

    #[test]
    fn rejects_unknown_aac_payload_for_decode() {
        let adts = frame_adts(AdtsConfig::aac_lc(44_100, 1), &[0xaa]).unwrap();

        let err = super::decode(&adts).unwrap_err();

        assert!(matches!(
            err,
            Error::UnsupportedFeature(
                "AAC decode currently supports sonare silent AAC-LC ADTS only"
            )
        ));
    }

    #[test]
    fn bit_writer_writes_msb_first() {
        let mut writer = BitWriter::new();
        writer.write_bits(0b101, 3).unwrap();
        writer.write_bits(0b11, 2).unwrap();

        assert_eq!(writer.finish_byte_aligned(), &[0b1011_1000]);
    }
