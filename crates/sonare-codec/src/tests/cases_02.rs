    #[test]
    #[cfg(feature = "mp3")]
    fn exposes_mp3_pcm_bitrate_helper() {
        let pcm = AudioBuffer::new(
            44_100,
            1,
            (0..1152)
                .map(|sample| ((sample as f32) * 0.01).sin() * 0.25)
                .collect(),
        )
        .unwrap();
        let provider = super::mpeg1_layer3_standard_table_provider();
        let header = super::layer3_header_for_capacity(44_100, 1, 96, false, false).unwrap();

        let encoded = super::encode_mpeg1_layer3_pcm_frames_with_bitrate_and_table_provider(
            &pcm,
            super::MPEG1_LAYER3_PCM_STEP_CANDIDATES,
            96,
            false,
            false,
            provider,
        )
        .unwrap();
        let parsed = super::FrameHeader::parse(&encoded[..4]).unwrap();

        assert_eq!(parsed, header);
        assert_eq!(parsed.bitrate_kbps, 96);
        assert_eq!(encoded.len(), header.frame_len());
        assert!(
            super::encode_mpeg1_layer3_pcm_frames_with_bitrate_and_table_provider(
                &pcm,
                super::MPEG1_LAYER3_PCM_STEP_CANDIDATES,
                123,
                false,
                false,
                provider,
            )
            .is_err()
        );
    }

    #[test]
    #[cfg(feature = "aac")]
    fn exposes_aac_adts_to_m4a_mux() {
        let adts =
            super::frame_aac_adts(super::AdtsConfig::aac_lc(44_100, 2), &[0x11, 0x22]).unwrap();
        let m4a = super::mux_aac_adts_as_m4a(&adts).unwrap();
        let demuxed = super::demux_m4a_as_aac_adts(&m4a).unwrap();

        assert_eq!(&m4a[4..8], b"ftyp");
        assert!(m4a.windows(4).any(|window| window == b"mdat"));
        assert_eq!(demuxed, adts);
    }

    #[test]
    #[cfg(feature = "aac")]
    fn exposes_aac_pcm_scale_factor_stream_helper() {
        let pcm = AudioBuffer::new(44_100, 1, vec![0.0; 2048]).unwrap();
        let scale_factors_by_frame: [&[i16]; 2] = [&[0], &[0]];
        let selected =
            super::select_scale_factors_for_quantized_bands(&[0, 0, 1, -1], 2, 100).unwrap();
        let offsets = [0, 2, 4];
        let selected_by_offsets = super::select_scale_factors_for_quantized_bands_by_offsets(
            &[0, 0, 1, -1],
            &offsets,
            100,
        )
        .unwrap();
        let biased_by_offsets =
            super::select_scale_factors_for_quantized_bands_by_offsets_with_magnitude_bias(
                &[0, 0, 1, -1],
                &offsets,
                100,
                2,
            )
            .unwrap();
        let quantized_adts = super::encode_quantized_mono_adts_with_selected_scale_factors(
            super::AdtsConfig::aac_lc(44_100, 1),
            super::AacLongBlockConfig::new(0, 1),
            &[0, 0],
            2,
            &[],
            super::AacSpectralMagnitudeTables::default(),
        )
        .unwrap();
        let selected_pcm_adts = super::encode_pcm_mono_long_block_adts_with_selected_scale_factors(
            super::AdtsConfig::aac_lc(44_100, 1),
            super::AacLongBlockConfig::new(0, 1),
            &pcm,
            super::AacPcmLongBlockConfig::new(0, 1.0, 1024),
            &[],
            super::AacSpectralMagnitudeTables::default(),
        )
        .unwrap();
        let selected_stream_adts =
            super::encode_pcm_mono_long_block_adts_stream_with_selected_scale_factors(
                super::AdtsConfig::aac_lc(44_100, 1),
                super::AacLongBlockConfig::new(0, 1),
                &pcm,
                super::AacPcmLongBlockConfig::new(0, 1.0, 1024),
                &[],
                super::AacSpectralMagnitudeTables::default(),
            )
            .unwrap();

        let adts = super::encode_pcm_mono_long_block_adts_stream_with_scale_factors(
            super::AdtsConfig::aac_lc(44_100, 1),
            super::AacScaleFactorSequence::new(
                super::AacLongBlockConfig::new(0, 1),
                &scale_factors_by_frame,
            ),
            &pcm,
            super::AacPcmLongBlockConfig::new(0, 1.0, 1024),
            &[],
            super::AacSpectralMagnitudeTables::default(),
        )
        .unwrap();

        assert_eq!(&adts[..2], &[0xff, 0xf1]);
        assert_eq!(&quantized_adts[..2], &[0xff, 0xf1]);
        assert_eq!(&selected_pcm_adts[..2], &[0xff, 0xf1]);
        assert_eq!(&selected_stream_adts[..2], &[0xff, 0xf1]);
        assert_eq!(adts.len(), 26);
        assert_eq!(selected_stream_adts.len(), 26);
        assert_eq!(selected, vec![100, 101]);
        assert_eq!(selected_by_offsets, vec![100, 101]);
        assert_eq!(biased_by_offsets, vec![100, 100]);
    }

    #[test]
    #[cfg(feature = "aac")]
    fn exposes_aac_pcm_bitrate_budget_stream_helper() {
        fn max_adts_frame_len(stream: &[u8]) -> usize {
            let mut max_len = 0;
            let mut offset = 0;
            while offset + 7 <= stream.len() {
                let frame_len = (((stream[offset + 3] & 0x03) as usize) << 11)
                    | ((stream[offset + 4] as usize) << 3)
                    | ((stream[offset + 5] as usize) >> 5);
                assert!(frame_len >= 7);
                assert!(offset + frame_len <= stream.len());
                max_len = max_len.max(frame_len);
                offset += frame_len;
            }
            assert_eq!(offset, stream.len());
            max_len
        }

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
        let offsets = super::aac_lc_long_window_scale_factor_band_offsets(44_100).unwrap();
        let channel_config = super::AacLongBlockConfig::new(180, (offsets.len() - 1) as u8);
        let scale_factors = vec![i16::from(channel_config.global_gain); offsets.len() - 1];
        let channel = super::AacScaleFactorChannel::new(channel_config, &scale_factors);
        let scale_factor_table = super::aac_scale_factor_delta_zero_table();
        let spectral_tables = super::aac_unsigned_pairs7_unit_magnitude_spectral_tables();

        let mono_target_bitrate = 10_000;
        let mono_budget =
            super::aac_lc_adts_max_frame_len_for_bitrate(44_100, mono_target_bitrate).unwrap();
        let mono_adts =
            super::encode_pcm_mono_long_block_adts_stream_with_offsets_and_scale_factors_and_bitrate_by_bit_cost(
                super::AdtsConfig::aac_lc(44_100, 1),
                channel,
                &mono,
                offsets,
                super::AAC_LC_PCM_STEP_CANDIDATES,
                mono_target_bitrate,
                scale_factor_table,
                spectral_tables,
            )
            .unwrap();

        let stereo_target_bitrate = 14_000;
        let stereo_budget =
            super::aac_lc_adts_max_frame_len_for_bitrate(44_100, stereo_target_bitrate).unwrap();
        let stereo_adts =
            super::encode_pcm_stereo_long_block_adts_stream_with_offsets_and_scale_factors_and_bitrate_by_bit_cost(
                super::AdtsConfig::aac_lc(44_100, 2),
                channel,
                channel,
                &stereo,
                offsets,
                super::AAC_LC_PCM_STEP_CANDIDATES,
                stereo_target_bitrate,
                scale_factor_table,
                spectral_tables,
            )
            .unwrap();
        let mono_adts_high_level =
            super::encode_aac_adts_with_bitrate(&mono, mono_target_bitrate).unwrap();
        let stereo_adts_high_level =
            super::encode_aac_adts_with_bitrate(&stereo, stereo_target_bitrate).unwrap();
        let selected_scale_factor_table = super::aac_scale_factor_delta_table();
        let selected_mono_target_bitrate = 128_000;
        let selected_stereo_target_bitrate = 256_000;
        let selected_mono_adts = super::encode_aac_adts_with_selected_scale_factors_and_bitrate(
            &mono,
            selected_mono_target_bitrate,
        )
        .unwrap();
        let selected_stereo_adts = super::encode_aac_adts_with_selected_scale_factors_and_bitrate(
            &stereo,
            selected_stereo_target_bitrate,
        )
        .unwrap();
        let selected_mono_details = super::aac_selected_scale_factor_frame_details_with_bitrate(
            &mono,
            selected_mono_target_bitrate,
        )
        .unwrap();
        let selected_stereo_details = super::aac_selected_scale_factor_frame_details_with_bitrate(
            &stereo,
            selected_stereo_target_bitrate,
        )
        .unwrap();
        let selected_mono_core_details =
            super::select_aac_lc_mono_pcm_stream_frame_details_with_offsets_and_selected_scale_factors_and_bitrate_by_bit_cost(
                super::AdtsConfig::aac_lc(44_100, 1),
                channel_config,
                &mono,
                offsets,
                super::AAC_LC_PCM_STEP_CANDIDATES,
                selected_mono_target_bitrate,
                &selected_scale_factor_table,
                spectral_tables,
            )
            .unwrap();
        let selected_stereo_core_details =
            super::select_aac_lc_stereo_pcm_stream_frame_details_with_offsets_and_selected_scale_factors_and_bitrate_by_bit_cost(
                super::AdtsConfig::aac_lc(44_100, 2),
                channel_config,
                channel_config,
                &stereo,
                offsets,
                super::AAC_LC_PCM_STEP_CANDIDATES,
                selected_stereo_target_bitrate,
                &selected_scale_factor_table,
                spectral_tables,
            )
            .unwrap();

        assert!(super::AAC_LC_PCM_STEP_CANDIDATES.contains(&0.2));
        assert!(!super::AAC_LC_PCM_STEP_CANDIDATES.contains(&0.15));
        assert!(super::AAC_STANDARD_ID_PCM_STEP_CANDIDATES.contains(&0.15));
        assert!(super::AAC_STANDARD_ID_PCM_STEP_CANDIDATES.contains(&0.075));
        assert_eq!(
            super::aac_standard_id_selected_scale_factor_global_gain(1).unwrap(),
            128
        );
        assert_eq!(
            super::aac_standard_id_selected_scale_factor_global_gain(2).unwrap(),
            126
        );
        assert_eq!(
            super::aac_standard_id_selected_scale_factor_magnitude_bias(),
            16
        );
        assert_eq!(
            super::aac_standard_id_selected_scale_factor_parameters(1).unwrap(),
            (128, 16)
        );
        assert_eq!(
            super::aac_standard_id_selected_scale_factor_parameters(2).unwrap(),
            (126, 16)
        );
        assert_eq!(
            super::aac_standard_id_selected_scale_factor_balanced_parameters(1).unwrap(),
            (136, 8, 2047)
        );
        assert_eq!(
            super::aac_standard_id_selected_scale_factor_balanced_parameters(2).unwrap(),
            (138, 4, 1535)
        );
        let mono_balance_profile =
            super::aac_standard_id_selected_scale_factor_balance_profile(1).unwrap();
        let stereo_balance_profile =
            super::aac_standard_id_selected_scale_factor_balance_profile(2).unwrap();
        assert_eq!(mono_balance_profile.recommended_global_gain, 128);
        assert_eq!(mono_balance_profile.global_gain_deltas, &[0, 2, 4, 6, 8]);
        assert_eq!(mono_balance_profile.magnitude_biases, &[8, 12, 16, 20]);
        assert_eq!(mono_balance_profile.selected_global_gain, 136);
        assert_eq!(mono_balance_profile.selected_magnitude_bias, 8);
        assert_eq!(mono_balance_profile.max_quantized_abs, 2047);
        assert_eq!(stereo_balance_profile.recommended_global_gain, 126);
        assert_eq!(stereo_balance_profile.global_gain_deltas, &[8, 12, 16]);
        assert_eq!(stereo_balance_profile.magnitude_biases, &[4, 8, 12]);
        assert_eq!(stereo_balance_profile.selected_global_gain, 138);
        assert_eq!(stereo_balance_profile.selected_magnitude_bias, 4);
        assert_eq!(stereo_balance_profile.max_quantized_abs, 1535);
        let mono_quality_control_candidates =
            super::aac_standard_id_quality_control_candidates_for_balance_profile_with_bitrate(
                &mono,
                selected_mono_target_bitrate,
            )
            .unwrap();
        let stereo_quality_control_candidates =
            super::aac_standard_id_quality_control_candidates_for_balance_profile_with_bitrate(
                &stereo,
                selected_stereo_target_bitrate,
            )
            .unwrap();
        assert_eq!(
            mono_quality_control_candidates.len(),
            mono_balance_profile.global_gain_deltas.len()
                * mono_balance_profile.magnitude_biases.len()
        );
        assert_eq!(
            stereo_quality_control_candidates.len(),
            stereo_balance_profile.global_gain_deltas.len()
                * stereo_balance_profile.magnitude_biases.len()
        );
        assert!(mono_quality_control_candidates.iter().all(|candidate| {
            candidate.profile.min_frame_budget_slack >= 0
                && candidate.profile.max_abs <= i32::try_from(candidate.max_quantized_abs).unwrap()
        }));
        assert!(stereo_quality_control_candidates.iter().all(|candidate| {
            candidate.profile.min_frame_budget_slack >= 0
                && candidate.profile.max_abs <= i32::try_from(candidate.max_quantized_abs).unwrap()
        }));
        assert!(mono_quality_control_candidates.iter().any(|candidate| {
            candidate.global_gain == mono_balance_profile.selected_global_gain
                && candidate.scale_factor_magnitude_bias
                    == mono_balance_profile.selected_magnitude_bias
                && candidate.max_quantized_abs == mono_balance_profile.max_quantized_abs
        }));
        assert!(stereo_quality_control_candidates.iter().any(|candidate| {
            candidate.global_gain == stereo_balance_profile.selected_global_gain
                && candidate.scale_factor_magnitude_bias
                    == stereo_balance_profile.selected_magnitude_bias
                && candidate.max_quantized_abs == stereo_balance_profile.max_quantized_abs
        }));
        assert!(super::aac_standard_id_selected_scale_factor_global_gain(3).is_err());
        assert!(super::aac_standard_id_selected_scale_factor_balance_profile(3).is_err());
        assert_eq!(&mono_adts[..2], &[0xff, 0xf1]);
        assert_eq!(&stereo_adts[..2], &[0xff, 0xf1]);
        assert_eq!(mono_adts_high_level, mono_adts);
        assert_eq!(stereo_adts_high_level, stereo_adts);
        assert_eq!(selected_mono_details, selected_mono_core_details);
        assert_eq!(selected_stereo_details, selected_stereo_core_details);
        assert_eq!(selected_mono_details.len(), 2);
        assert_eq!(selected_stereo_details.len(), 2);
        assert_eq!(
            selected_mono_details
                .iter()
                .map(|detail| detail.frame_len)
                .max()
                .unwrap(),
            max_adts_frame_len(&selected_mono_adts)
        );
        assert_eq!(
            selected_stereo_details
                .iter()
                .map(|detail| detail.frame_len)
                .max()
                .unwrap(),
            max_adts_frame_len(&selected_stereo_adts)
        );
        assert!(max_adts_frame_len(&mono_adts) <= mono_budget);
        assert!(max_adts_frame_len(&stereo_adts) <= stereo_budget);
        assert!(super::aac_lc_adts_max_frame_len_for_bitrate(44_100, 1).is_err());
        assert!(super::encode_aac_adts_with_bitrate(&mono, 1).is_err());
    }

    #[test]
    #[cfg(feature = "aac")]
    fn exposes_aac_unsigned_pairs7_unit_magnitude_table() {
        let table = super::aac_unsigned_pairs7_unit_magnitude_table();
        assert_eq!(table.len(), 4);
        assert_eq!(table[0].symbol, super::AacSpectralMagnitudePair::new(0, 0));
        assert_eq!(table[0].code, super::HuffmanCode::new(0b0, 1).unwrap());
        assert_eq!(table[1].symbol, super::AacSpectralMagnitudePair::new(0, 1));
        assert_eq!(table[1].code, super::HuffmanCode::new(0b101, 3).unwrap());
        assert_eq!(table[2].symbol, super::AacSpectralMagnitudePair::new(1, 0));
        assert_eq!(table[2].code, super::HuffmanCode::new(0b100, 3).unwrap());
        assert_eq!(table[3].symbol, super::AacSpectralMagnitudePair::new(1, 1));
        assert_eq!(table[3].code, super::HuffmanCode::new(0b1100, 4).unwrap());
    }

    #[test]
    #[cfg(feature = "aac")]
    fn exposes_aac_unsigned_pairs7_table() {
        let table = super::aac_unsigned_pairs7_table();

        assert_eq!(table.len(), 64);
        assert_eq!(table[0].symbol, super::AacSpectralMagnitudePair::new(0, 0));
        assert_eq!(table[0].code, super::HuffmanCode::new(0, 1).unwrap());
        assert_eq!(table[63].symbol, super::AacSpectralMagnitudePair::new(7, 7));
        assert_eq!(table[63].code, super::HuffmanCode::new(0xfff, 12).unwrap());
    }

    #[test]
    #[cfg(feature = "aac")]
    fn exposes_aac_signed_pairs5_and_6_tables() {
        let pairs5 = super::aac_signed_pairs5_table();
        let pairs6 = super::aac_signed_pairs6_table();

        assert_eq!(pairs5.len(), 81);
        assert_eq!(pairs5[40].symbol, super::AacSpectralPair::new(0, 0));
        assert_eq!(pairs5[40].code, super::HuffmanCode::new(0, 1).unwrap());
        assert_eq!(pairs6.len(), 81);
        assert_eq!(pairs6[40].symbol, super::AacSpectralPair::new(0, 0));
        assert_eq!(pairs6[40].code, super::HuffmanCode::new(0, 4).unwrap());

        let tables = super::aac_lc_standard_signed_pair_tables();
        assert_eq!(tables.signed_pairs5.len(), 81);
        assert_eq!(tables.signed_pairs6.len(), 81);
        assert_eq!(
            super::pack_spectral_pairs_with_table(
                &[super::AacSpectralPair::new(1, -1)],
                tables.signed_pairs6,
            )
            .unwrap()
            .bit_len,
            4
        );
        assert_eq!(
            super::plan_aac_lc_standard_spectral_sections_by_bit_cost(&[0, 1], 2).unwrap(),
            vec![super::AacSpectralSection {
                start: 0,
                end: 2,
                codebook_id: 5,
            }]
        );
        assert_eq!(
            super::split_aac_lc_standard_spectral_payload_with_sign_bits_by_bit_cost(&[1, -1], 2)
                .unwrap()
                .spectral_bits
                .bit_len,
            4
        );
    }

    #[test]
    #[cfg(feature = "aac")]
    fn exposes_aac_unsigned_quads3_and_4_tables() {
        let quads3 = super::aac_unsigned_quads3_table();
        let quads4 = super::aac_unsigned_quads4_table();

        assert_eq!(quads3.len(), 81);
        assert_eq!(
            quads3[40].symbol,
            super::AacSpectralMagnitudeQuad::new(1, 1, 1, 1)
        );
        assert_eq!(quads3[40].code, super::HuffmanCode::new(0x74, 7).unwrap());
        assert_eq!(quads4.len(), 81);
        assert_eq!(
            quads4[40].symbol,
            super::AacSpectralMagnitudeQuad::new(1, 1, 1, 1)
        );
        assert_eq!(quads4[40].code, super::HuffmanCode::new(0, 4).unwrap());

        let tables = super::aac_lc_standard_unsigned_quad_tables();
        assert_eq!(tables.quads3.len(), 81);
        assert_eq!(tables.quads4.len(), 81);
        assert_eq!(
            super::select_quad_codebook_by_bit_cost(&[1, -1, 1, -1], tables).unwrap(),
            4
        );
    }

    #[test]
    #[cfg(feature = "aac")]
    fn exposes_aac_signed_quads1_and_2_tables() {
        let quads1 = super::aac_signed_quads1_table();
        let quads2 = super::aac_signed_quads2_table();

        assert_eq!(quads1.len(), 81);
        assert_eq!(quads1[40].symbol, super::AacSpectralQuad::new(0, 0, 0, 0));
        assert_eq!(quads1[40].code, super::HuffmanCode::new(0, 1).unwrap());
        assert_eq!(quads2.len(), 81);
        assert_eq!(quads2[40].symbol, super::AacSpectralQuad::new(0, 0, 0, 0));
        assert_eq!(quads2[40].code, super::HuffmanCode::new(0, 3).unwrap());

        let tables = super::aac_lc_standard_signed_quad_tables();
        assert_eq!(tables.quads1.len(), 81);
        assert_eq!(tables.quads2.len(), 81);
        assert_eq!(
            super::plan_aac_lc_standard_spectral_sections_by_bit_cost(&[1, -1, 1, -1], 4).unwrap()
                [0]
            .codebook_id,
            2
        );
    }

    #[test]
    #[cfg(feature = "aac")]
    fn exposes_aac_unsigned_pairs8_table() {
        let table = super::aac_unsigned_pairs8_table();

        assert_eq!(table.len(), 64);
        assert_eq!(table[0].symbol, super::AacSpectralMagnitudePair::new(0, 0));
        assert_eq!(table[0].code, super::HuffmanCode::new(0x00e, 5).unwrap());
        assert_eq!(table[9].symbol, super::AacSpectralMagnitudePair::new(1, 1));
        assert_eq!(table[9].code, super::HuffmanCode::new(0, 3).unwrap());
        assert_eq!(table[63].symbol, super::AacSpectralMagnitudePair::new(7, 7));
        assert_eq!(table[63].code, super::HuffmanCode::new(0x3ff, 10).unwrap());
    }

    #[test]
    #[cfg(feature = "aac")]
    fn exposes_aac_scale_factor_delta_table() {
        let table = super::aac_scale_factor_delta_table();

        assert_eq!(table.len(), 121);
        assert_eq!(table[0].symbol, super::AacScaleFactorDelta::new(-60));
        assert_eq!(table[60].symbol, super::AacScaleFactorDelta::new(0));
        assert_eq!(table[60].code, super::HuffmanCode::new(0, 1).unwrap());
        assert_eq!(table[120].symbol, super::AacScaleFactorDelta::new(60));
    }

    #[test]
    #[cfg(feature = "aac")]
    #[rustfmt::skip]
    fn exposes_aac_spectral_quad_helpers() { let quads = [super::AacSpectralQuad::new(1, -1, 0, 1)]; let sections = [super::AacQuadSection { start: 0, end: 4, codebook_id: 2, }]; let quantized = [1, -1, 0, 1]; let signed_table = [super::HuffmanEntry { symbol: quads[0], code: super::HuffmanCode::new(0b11, 2).unwrap(), }]; let magnitude_table = [super::HuffmanEntry { symbol: super::AacSpectralMagnitudeQuad::new(1, 1, 0, 1), code: super::HuffmanCode::new(0b10, 2).unwrap(), }]; assert_eq!( super::pack_spectral_quads_with_table(&quads, &signed_table) .unwrap() .bit_len, 2 ); assert_eq!( super::pack_spectral_quads_with_sign_bits(&quads, &magnitude_table) .unwrap() .bit_len, 5 ); let tables = super::AacSpectralMagnitudeQuadTables { quads2: &magnitude_table, ..Default::default() }; let unit_pair_tables = super::aac_unit_codebook6_spectral_tables(); let unit_quad_tables = super::aac_unit_quad_spectral_tables(); assert_eq!( super::select_quad_codebook_by_bit_cost(&quantized, tables).unwrap(), 2 ); assert_eq!( super::plan_quad_sections_by_bit_cost(&quantized, 4, tables).unwrap(), sections ); assert_eq!( super::pack_quad_section_data_with_len(&sections, 4) .unwrap() .bit_len, 9 ); assert_eq!( super::pack_spectral_quad_sections_with_sign_bits(&sections, &quantized, tables) .unwrap() .bit_len, 5 ); assert_eq!( super::pack_sectioned_spectral_quad_payload_with_sign_bits( &sections, &quantized, 4, tables, ) .unwrap() .bit_len, 14 ); assert_eq!( super::pack_sectioned_spectral_quad_payload_with_sign_bits_by_bit_cost( &quantized, 4, tables, ) .unwrap() .bit_len, 14 ); assert_eq!(unit_pair_tables.pairs6.len(), 1); assert_eq!(unit_quad_tables.quads1.len(), 2); assert_eq!(unit_quad_tables.quads3.len(), 2); assert_eq!( super::plan_spectral_sections_by_bit_cost( &[1, -1, 0, 1, 1, -1, 1, -1, 0, 0, 0, 0], 4, unit_pair_tables, unit_quad_tables, ) .unwrap(), vec![ super::AacSpectralSection { start: 0, end: 4, codebook_id: 3, }, super::AacSpectralSection { start: 4, end: 8, codebook_id: 6, }, super::AacSpectralSection { start: 8, end: 12, codebook_id: 0, }, ] ); assert_eq!( super::plan_aac_lc_standard_spectral_sections_by_bit_cost( &[1, -1, 0, 1, 17, 0, 0, 0], 4 ) .unwrap(), vec![ super::AacSpectralSection { start: 0, end: 4, codebook_id: 4, }, super::AacSpectralSection { start: 4, end: 8, codebook_id: 11, }, ] ); assert_eq!( super::pack_aac_lc_standard_spectral_payload_with_sign_bits_by_bit_cost( &[1, -1, 0, 1, 17, 0, 0, 0], 4 ) .unwrap() .bit_len, 44 ); let standard_split = super::split_aac_lc_standard_spectral_payload_with_sign_bits_by_bit_cost( &[1, -1, 0, 1, 17, 0, 0, 0], 4, ) .unwrap(); assert_eq!(standard_split.section_and_scale_factor_bits.bit_len, 18); assert_eq!(standard_split.spectral_bits.bit_len, 26); assert_eq!( super::plan_aac_lc_standard_spectral_sections_by_offsets_by_bit_cost( &[1, -1, 0, 1, 17, 0, 0, 0], &[0, 4, 8] ) .unwrap(), vec![ super::AacSpectralSection { start: 0, end: 4, codebook_id: 4, }, super::AacSpectralSection { start: 4, end: 8, codebook_id: 11, }, ] ); assert_eq!( super::pack_aac_lc_standard_spectral_payload_with_offsets_and_sign_bits_by_bit_cost( &[1, -1, 0, 1, 17, 0, 0, 0], &[0, 4, 8] ) .unwrap() .bit_len, 44 ); let standard_offsets_split = super::split_aac_lc_standard_spectral_payload_with_offsets_and_sign_bits_by_bit_cost( &[1, -1, 0, 1, 17, 0, 0, 0], &[0, 4, 8], ) .unwrap(); assert_eq!( standard_offsets_split.section_and_scale_factor_bits.bit_len, 18 ); assert_eq!(standard_offsets_split.spectral_bits.bit_len, 26); let standard_offsets_sections = super::plan_aac_lc_standard_spectral_sections_by_offsets_by_bit_cost( &[1, -1, 0, 1, 17, 0, 0, 0], &[0, 4, 8], ) .unwrap(); assert_eq!( super::plan_spectral_scale_factor_deltas_by_offsets( &standard_offsets_sections, &[0, 4, 8], &[100, 100], 100 ) .unwrap(), vec![ super::AacScaleFactorDelta::new(0), super::AacScaleFactorDelta::new(0) ] ); let standard_adts = super::encode_quantized_mono_adts_with_standard_spectral_offsets_and_scale_factors_by_bit_cost( super::AdtsConfig::aac_lc(44_100, 1), super::AacLongBlockConfig::new(100, 2), &[1, -1, 0, 1, 17, 0, 0, 0], &[0, 4, 8], &[100, 100], super::aac_scale_factor_delta_zero_table(), ) .unwrap(); assert_eq!(&standard_adts[..2], &[0xff, 0xf1]); let pcm = AudioBuffer::new( 44_100, 1, (0..2048) .map(|sample| ((sample as f32) * 0.02).sin() * 0.2) .collect(), ) .unwrap(); let long_offsets = super::aac_lc_long_window_scale_factor_band_offsets(44_100).unwrap(); let max_sfb = long_offsets.len() - 1; let scale_frame0 = vec![128_i16; max_sfb]; let scale_frame1 = vec![128_i16; max_sfb]; let scale_frames: [&[i16]; 2] = [&scale_frame0, &scale_frame1]; let max_adts_frame_len = |stream: &[u8]| -> usize { let mut offset = 0usize; let mut max_frame_len = 0usize; while offset < stream.len() { let frame_len = (usize::from(stream[offset + 3] & 0x03) << 11) | (usize::from(stream[offset + 4]) << 3) | usize::from(stream[offset + 5] >> 5); max_frame_len = max_frame_len.max(frame_len); offset += frame_len; } assert_eq!(offset, stream.len()); max_frame_len }; let standard_stream = super::encode_pcm_mono_long_block_adts_stream_with_standard_spectral_offsets_and_scale_factors_by_bit_cost( super::AdtsConfig::aac_lc(44_100, 1), super::AacScaleFactorSequence::new( super::AacLongBlockConfig::new(128, max_sfb as u8), &scale_frames, ), &pcm, 0, 0.005, long_offsets, super::aac_scale_factor_delta_zero_table(), ) .unwrap(); let mut offset = 0usize; let mut frame_count = 0usize; while offset < standard_stream.len() { assert_eq!(standard_stream[offset], 0xff); assert_eq!(standard_stream[offset + 1] & 0xf0, 0xf0); let frame_len = (usize::from(standard_stream[offset + 3] & 0x03) << 11) | (usize::from(standard_stream[offset + 4]) << 3) | usize::from(standard_stream[offset + 5] >> 5); offset += frame_len; frame_count += 1; } assert_eq!(frame_count, 2); assert_eq!(offset, standard_stream.len()); let standard_bitrate_stream = super::encode_pcm_mono_long_block_adts_stream_with_standard_spectral_offsets_and_scale_factors_and_bitrate_by_bit_cost( super::AdtsConfig::aac_lc(44_100, 1), super::AacScaleFactorSequence::new( super::AacLongBlockConfig::new(128, max_sfb as u8), &scale_frames, ), &pcm, 0, long_offsets, super::AAC_LC_PCM_STEP_CANDIDATES, 128_000, super::aac_scale_factor_delta_zero_table(), ) .unwrap(); assert!( max_adts_frame_len(&standard_bitrate_stream) <= super::aac_lc_adts_max_frame_len_for_bitrate(44_100, 128_000).unwrap() ); let high_level_standard_bitrate_stream = super::encode_aac_adts_with_standard_spectral_offsets_and_bitrate(&pcm, 128_000, 128) .unwrap(); assert_eq!(&high_level_standard_bitrate_stream[..2], &[0xff, 0xf1]); assert!( max_adts_frame_len(&high_level_standard_bitrate_stream) <= super::aac_lc_adts_max_frame_len_for_bitrate(44_100, 128_000).unwrap() ); let high_level_standard_m4a = super::encode_m4a_with_standard_spectral_offsets_and_bitrate(&pcm, 128_000, 128) .unwrap(); assert_eq!(&high_level_standard_m4a[4..8], b"ftyp"); let high_level_selected_standard_bitrate_stream = super::encode_aac_adts_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_bitrate( &pcm, 128_000, 128, 16, ) .unwrap(); let core_selected_standard_bitrate_stream = super::encode_pcm_mono_long_block_adts_stream_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_bitrate_by_bit_cost( super::AdtsConfig::aac_lc(44_100, 1), super::AacLongBlockConfig::new(128, max_sfb as u8), &pcm, 0, long_offsets, 16, super::AAC_LC_PCM_STEP_CANDIDATES, 128_000, &super::aac_scale_factor_delta_table(), ) .unwrap(); assert_eq!( high_level_selected_standard_bitrate_stream, core_selected_standard_bitrate_stream ); let high_level_selected_standard_details = super::aac_standard_selected_scale_factor_frame_details_with_magnitude_bias_and_bitrate( &pcm, 128_000, 128, 16, ) .unwrap(); let core_selected_standard_details = super::select_aac_lc_mono_pcm_stream_frame_details_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_bitrate_by_bit_cost( super::AdtsConfig::aac_lc(44_100, 1), super::AacLongBlockConfig::new(128, max_sfb as u8), &pcm, 0, long_offsets, 16, super::AAC_LC_PCM_STEP_CANDIDATES, 128_000, &super::aac_scale_factor_delta_table(), ) .unwrap(); assert_eq!( high_level_selected_standard_details, core_selected_standard_details ); assert_eq!( high_level_selected_standard_details .iter() .map(|selection| selection.frame_len) .max(), Some(max_adts_frame_len( &high_level_selected_standard_bitrate_stream )) ); assert!( max_adts_frame_len(&high_level_selected_standard_bitrate_stream) <= super::aac_lc_adts_max_frame_len_for_bitrate(44_100, 128_000).unwrap() ); let high_level_selected_standard_m4a = super::encode_m4a_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_bitrate( &pcm, 128_000, 128, 16, ) .unwrap(); assert_eq!(&high_level_selected_standard_m4a[4..8], b"ftyp"); let recommended_selected_standard_bitrate_stream = super::encode_aac_adts_with_recommended_standard_spectral_offsets_and_selected_scale_factors_and_bitrate( &pcm, 128_000, ) .unwrap(); assert_eq!( recommended_selected_standard_bitrate_stream, high_level_selected_standard_bitrate_stream ); let recommended_selected_standard_details = super::aac_recommended_standard_selected_scale_factor_frame_details_with_bitrate( &pcm, 128_000, ) .unwrap(); assert_eq!( super::aac_standard_id_selected_scale_factor_balanced_max_quantized_abs(1).unwrap(), super::AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_MONO_BALANCED_MAX_QUANTIZED_ABS ); assert_eq!( super::aac_standard_id_selected_scale_factor_balanced_max_quantized_abs(2).unwrap(), super::AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_STEREO_BALANCED_MAX_QUANTIZED_ABS ); assert_eq!( recommended_selected_standard_details, high_level_selected_standard_details ); let recommended_selected_profile = super::aac_recommended_standard_selected_scale_factor_profile_for_frame_details( &pcm, &recommended_selected_standard_details, ) .unwrap(); let expected_recommended_selected_profile = super::aac_standard_selected_scale_factor_profile_for_frame_details_with_magnitude_bias( &pcm, &recommended_selected_standard_details, super::AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_MONO_GLOBAL_GAIN, super::AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_MAGNITUDE_BIAS, ) .unwrap(); assert_eq!( recommended_selected_profile, expected_recommended_selected_profile ); assert_eq!(recommended_selected_profile.frames, 2); assert_eq!(recommended_selected_profile.channels, 1); assert_eq!(recommended_selected_profile.bands, 2 * max_sfb); assert!(recommended_selected_profile.mean_delta.is_finite()); let recommended_payload_breakdown = super::aac_recommended_standard_id_payload_breakdown_for_frame_details( &pcm, &recommended_selected_standard_details, ) .unwrap(); let expected_recommended_payload_breakdown = super::aac_standard_id_payload_breakdown_for_frame_details_with_magnitude_bias( &pcm, &recommended_selected_standard_details, super::AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_MONO_GLOBAL_GAIN, super::AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_MAGNITUDE_BIAS, ) .unwrap(); assert_eq!( recommended_payload_breakdown, expected_recommended_payload_breakdown ); assert_eq!(recommended_payload_breakdown.frames, 2); assert_eq!(recommended_payload_breakdown.channels, 1); assert!(recommended_payload_breakdown.sections > 0); assert!(recommended_payload_breakdown.spectral_bits > 0); assert!( recommended_payload_breakdown.total_bits() >= recommended_payload_breakdown.spectral_bits ); let balanced_selected_standard_stream = super::encode_aac_adts_with_balanced_standard_spectral_offsets_and_selected_scale_factors_and_bitrate( &pcm, 128_000, ) .unwrap(); let expected_balanced_selected_standard_stream = super::encode_aac_adts_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_max_quantized_abs_and_bitrate( &pcm, 128_000, super::AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_MONO_BALANCED_GLOBAL_GAIN, super::AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_BALANCED_MAGNITUDE_BIAS, super::AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_MONO_BALANCED_MAX_QUANTIZED_ABS, ) .unwrap(); assert_eq!( balanced_selected_standard_stream, expected_balanced_selected_standard_stream ); let balanced_selected_standard_details = super::aac_balanced_standard_selected_scale_factor_frame_details_with_bitrate( &pcm, 128_000, ) .unwrap(); let expected_balanced_selected_standard_details = super::aac_standard_selected_scale_factor_frame_details_with_magnitude_bias_max_quantized_abs_and_bitrate( &pcm, 128_000, super::AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_MONO_BALANCED_GLOBAL_GAIN, super::AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_BALANCED_MAGNITUDE_BIAS, super::AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_MONO_BALANCED_MAX_QUANTIZED_ABS, ) .unwrap(); assert_eq!( balanced_selected_standard_details, expected_balanced_selected_standard_details ); let balanced_selected_profile = super::aac_balanced_standard_selected_scale_factor_profile_for_frame_details( &pcm, &balanced_selected_standard_details, ) .unwrap(); let expected_balanced_selected_profile = super::aac_standard_selected_scale_factor_profile_for_frame_details_with_magnitude_bias( &pcm, &balanced_selected_standard_details, super::AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_MONO_BALANCED_GLOBAL_GAIN, super::AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_MONO_BALANCED_MAGNITUDE_BIAS, ) .unwrap(); assert_eq!( balanced_selected_profile, expected_balanced_selected_profile ); assert_eq!(balanced_selected_profile.frames, 2); assert_eq!(balanced_selected_profile.channels, 1); assert_eq!(balanced_selected_profile.bands, 2 * max_sfb); assert!(balanced_selected_profile.mean_delta.is_finite()); let balanced_payload_breakdown = super::aac_balanced_standard_id_payload_breakdown_for_frame_details( &pcm, &balanced_selected_standard_details, ) .unwrap(); let expected_balanced_payload_breakdown = super::aac_standard_id_payload_breakdown_for_frame_details_with_magnitude_bias( &pcm, &balanced_selected_standard_details, super::AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_MONO_BALANCED_GLOBAL_GAIN, super::AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_MONO_BALANCED_MAGNITUDE_BIAS, ) .unwrap(); assert_eq!( balanced_payload_breakdown, expected_balanced_payload_breakdown ); assert_eq!(balanced_payload_breakdown.frames, 2); assert_eq!(balanced_payload_breakdown.channels, 1); assert!(balanced_payload_breakdown.sections > 0); assert!(balanced_payload_breakdown.spectral_bits > 0); let balanced_selected_standard_m4a = super::encode_m4a_with_balanced_standard_spectral_offsets_and_selected_scale_factors_and_bitrate( &pcm, 128_000, ) .unwrap(); assert_eq!( super::demux_m4a_as_aac_adts(&balanced_selected_standard_m4a).unwrap(), balanced_selected_standard_stream ); let high_level_selected_standard_limited_stream = super::encode_aac_adts_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_max_quantized_abs_and_bitrate( &pcm, 128_000, 128, 16, 12, ) .unwrap(); let core_selected_standard_limited_stream = super::encode_pcm_mono_long_block_adts_stream_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_max_quantized_abs_and_bitrate_by_bit_cost( super::AdtsConfig::aac_lc(44_100, 1), super::AacLongBlockConfig::new(128, max_sfb as u8), &pcm, 0, long_offsets, 16, super::AAC_STANDARD_ID_PCM_STEP_CANDIDATES, 12, 128_000, &super::aac_scale_factor_delta_table(), ) .unwrap(); assert_eq!( high_level_selected_standard_limited_stream, core_selected_standard_limited_stream ); let recommended_selected_standard_limited_stream = super::encode_aac_adts_with_recommended_standard_spectral_offsets_and_selected_scale_factors_max_quantized_abs_and_bitrate( &pcm, 128_000, 12, ) .unwrap(); assert_eq!( recommended_selected_standard_limited_stream, high_level_selected_standard_limited_stream ); let recommended_selected_standard_limited_details = super::aac_recommended_standard_selected_scale_factor_frame_details_with_max_quantized_abs_and_bitrate( &pcm, 128_000, 12, ) .unwrap(); assert_eq!( recommended_selected_standard_limited_details .iter() .map(|selection| selection.frame_len) .max(), Some(max_adts_frame_len( &recommended_selected_standard_limited_stream )) ); let recommended_selected_standard_m4a = super::encode_m4a_with_recommended_standard_spectral_offsets_and_selected_scale_factors_and_bitrate( &pcm, 128_000, ) .unwrap(); assert_eq!( recommended_selected_standard_m4a, high_level_selected_standard_m4a ); let recommended_selected_standard_limited_m4a = super::encode_m4a_with_recommended_standard_spectral_offsets_and_selected_scale_factors_max_quantized_abs_and_bitrate( &pcm, 128_000, 12, ) .unwrap(); assert_eq!(&recommended_selected_standard_limited_m4a[4..8], b"ftyp"); let standard_bitrate_details = super::select_aac_lc_mono_pcm_stream_frame_details_with_standard_spectral_offsets_and_scale_factors_and_bitrate_by_bit_cost( super::AdtsConfig::aac_lc(44_100, 1), super::AacScaleFactorSequence::new( super::AacLongBlockConfig::new(128, max_sfb as u8), &scale_frames, ), &pcm, 0, long_offsets, super::AAC_LC_PCM_STEP_CANDIDATES, 128_000, super::aac_scale_factor_delta_zero_table(), ) .unwrap(); assert_eq!(standard_bitrate_details.len(), 2); assert!(standard_bitrate_details.iter().all(|detail| { detail.frame_len <= super::aac_lc_adts_max_frame_len_for_bitrate(44_100, 128_000).unwrap() })); let stereo_pcm = AudioBuffer::new( 44_100, 2, (0..2048) .flat_map(|sample| { [ ((sample as f32) * 0.02).sin() * 0.2, ((sample as f32) * 0.017).cos() * 0.18, ] }) .collect(), ) .unwrap(); let standard_stereo_stream = super::encode_pcm_stereo_long_block_adts_stream_with_standard_spectral_offsets_and_scale_factors_by_bit_cost( super::AdtsConfig::aac_lc(44_100, 2), super::AacScaleFactorSequence::new( super::AacLongBlockConfig::new(128, max_sfb as u8), &scale_frames, ), super::AacScaleFactorSequence::new( super::AacLongBlockConfig::new(128, max_sfb as u8), &scale_frames, ), &stereo_pcm, 0, 0.005, long_offsets, super::aac_scale_factor_delta_zero_table(), ) .unwrap(); let mut offset = 0usize; let mut stereo_frame_count = 0usize; while offset < standard_stereo_stream.len() { assert_eq!(standard_stereo_stream[offset], 0xff); assert_eq!(standard_stereo_stream[offset + 1] & 0xf0, 0xf0); let channels = ((standard_stereo_stream[offset + 2] & 0x01) << 2) | ((standard_stereo_stream[offset + 3] >> 6) & 0x03); assert_eq!(channels, 2); let frame_len = (usize::from(standard_stereo_stream[offset + 3] & 0x03) << 11) | (usize::from(standard_stereo_stream[offset + 4]) << 3) | usize::from(standard_stereo_stream[offset + 5] >> 5); offset += frame_len; stereo_frame_count += 1; } assert_eq!(stereo_frame_count, 2); assert_eq!(offset, standard_stereo_stream.len()); let standard_stereo_bitrate_stream = super::encode_pcm_stereo_long_block_adts_stream_with_standard_spectral_offsets_and_scale_factors_and_bitrate_by_bit_cost( super::AdtsConfig::aac_lc(44_100, 2), super::AacScaleFactorSequence::new( super::AacLongBlockConfig::new(128, max_sfb as u8), &scale_frames, ), super::AacScaleFactorSequence::new( super::AacLongBlockConfig::new(128, max_sfb as u8), &scale_frames, ), &stereo_pcm, 0, long_offsets, super::AAC_LC_PCM_STEP_CANDIDATES, 256_000, super::aac_scale_factor_delta_zero_table(), ) .unwrap(); assert!( max_adts_frame_len(&standard_stereo_bitrate_stream) <= super::aac_lc_adts_max_frame_len_for_bitrate(44_100, 256_000).unwrap() ); let standard_stereo_bitrate_details = super::select_aac_lc_stereo_pcm_stream_frame_details_with_standard_spectral_offsets_and_scale_factors_and_bitrate_by_bit_cost( super::AdtsConfig::aac_lc(44_100, 2), super::AacScaleFactorSequence::new( super::AacLongBlockConfig::new(128, max_sfb as u8), &scale_frames, ), super::AacScaleFactorSequence::new( super::AacLongBlockConfig::new(128, max_sfb as u8), &scale_frames, ), &stereo_pcm, 0, long_offsets, super::AAC_LC_PCM_STEP_CANDIDATES, 256_000, super::aac_scale_factor_delta_zero_table(), ) .unwrap(); assert_eq!(standard_stereo_bitrate_details.len(), 2); assert!(standard_stereo_bitrate_details.iter().all(|detail| { detail.frame_len <= super::aac_lc_adts_max_frame_len_for_bitrate(44_100, 256_000).unwrap() })); let high_level_selected_standard_stereo_bitrate_stream = super::encode_aac_adts_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_bitrate( &stereo_pcm, 256_000, 128, 16, ) .unwrap(); let core_selected_standard_stereo_bitrate_stream = super::encode_pcm_stereo_long_block_adts_stream_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_bitrate_by_bit_cost( super::AdtsConfig::aac_lc(44_100, 2), super::AacLongBlockConfig::new(128, max_sfb as u8), super::AacLongBlockConfig::new(128, max_sfb as u8), &stereo_pcm, 0, long_offsets, 16, super::AAC_STANDARD_ID_PCM_STEP_CANDIDATES, 256_000, &super::aac_scale_factor_delta_table(), ) .unwrap(); assert_eq!( high_level_selected_standard_stereo_bitrate_stream, core_selected_standard_stereo_bitrate_stream ); let high_level_selected_standard_stereo_details = super::aac_standard_selected_scale_factor_frame_details_with_magnitude_bias_and_bitrate( &stereo_pcm, 256_000, 128, 16, ) .unwrap(); let core_selected_standard_stereo_details = super::select_aac_lc_stereo_pcm_stream_frame_details_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_bitrate_by_bit_cost( super::AdtsConfig::aac_lc(44_100, 2), super::AacLongBlockConfig::new(128, max_sfb as u8), super::AacLongBlockConfig::new(128, max_sfb as u8), &stereo_pcm, 0, long_offsets, 16, super::AAC_STANDARD_ID_PCM_STEP_CANDIDATES, 256_000, &super::aac_scale_factor_delta_table(), ) .unwrap(); assert_eq!( high_level_selected_standard_stereo_details, core_selected_standard_stereo_details ); let recommended_selected_standard_stereo_bitrate_stream = super::encode_aac_adts_with_recommended_standard_spectral_offsets_and_selected_scale_factors_and_bitrate( &stereo_pcm, 256_000, ) .unwrap(); let core_recommended_selected_standard_stereo_bitrate_stream = super::encode_pcm_stereo_long_block_adts_stream_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_bitrate_by_bit_cost( super::AdtsConfig::aac_lc(44_100, 2), super::AacLongBlockConfig::new(126, max_sfb as u8), super::AacLongBlockConfig::new(126, max_sfb as u8), &stereo_pcm, 0, long_offsets, 16, super::AAC_STANDARD_ID_PCM_STEP_CANDIDATES, 256_000, &super::aac_scale_factor_delta_table(), ) .unwrap(); assert_eq!( recommended_selected_standard_stereo_bitrate_stream, core_recommended_selected_standard_stereo_bitrate_stream ); let recommended_selected_standard_stereo_details = super::aac_recommended_standard_selected_scale_factor_frame_details_with_bitrate( &stereo_pcm, 256_000, ) .unwrap(); let core_recommended_selected_standard_stereo_details = super::select_aac_lc_stereo_pcm_stream_frame_details_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_bitrate_by_bit_cost( super::AdtsConfig::aac_lc(44_100, 2), super::AacLongBlockConfig::new(126, max_sfb as u8), super::AacLongBlockConfig::new(126, max_sfb as u8), &stereo_pcm, 0, long_offsets, 16, super::AAC_STANDARD_ID_PCM_STEP_CANDIDATES, 256_000, &super::aac_scale_factor_delta_table(), ) .unwrap(); assert_eq!( recommended_selected_standard_stereo_details, core_recommended_selected_standard_stereo_details ); let recommended_selected_standard_stereo_profile = super::aac_recommended_standard_selected_scale_factor_profile_for_frame_details( &stereo_pcm, &recommended_selected_standard_stereo_details, ) .unwrap(); let expected_recommended_selected_standard_stereo_profile = super::aac_standard_selected_scale_factor_profile_for_frame_details_with_magnitude_bias( &stereo_pcm, &recommended_selected_standard_stereo_details, super::AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_STEREO_GLOBAL_GAIN, super::AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_MAGNITUDE_BIAS, ) .unwrap(); assert_eq!( recommended_selected_standard_stereo_profile, expected_recommended_selected_standard_stereo_profile ); assert_eq!(recommended_selected_standard_stereo_profile.frames, 2); assert_eq!(recommended_selected_standard_stereo_profile.channels, 2); assert_eq!( recommended_selected_standard_stereo_profile.bands, 4 * max_sfb ); assert!(recommended_selected_standard_stereo_profile .mean_delta .is_finite()); let balanced_selected_standard_stereo_stream = super::encode_aac_adts_with_balanced_standard_spectral_offsets_and_selected_scale_factors_and_bitrate( &stereo_pcm, 256_000, ) .unwrap(); let expected_balanced_selected_standard_stereo_stream = super::encode_aac_adts_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_max_quantized_abs_and_bitrate( &stereo_pcm, 256_000, super::AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_STEREO_BALANCED_GLOBAL_GAIN, super::AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_STEREO_BALANCED_MAGNITUDE_BIAS, super::AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_STEREO_BALANCED_MAX_QUANTIZED_ABS, ) .unwrap(); assert_eq!( balanced_selected_standard_stereo_stream, expected_balanced_selected_standard_stereo_stream ); let balanced_selected_standard_stereo_details = super::aac_balanced_standard_selected_scale_factor_frame_details_with_bitrate( &stereo_pcm, 256_000, ) .unwrap(); let expected_balanced_selected_standard_stereo_details = super::aac_standard_selected_scale_factor_frame_details_with_magnitude_bias_max_quantized_abs_and_bitrate( &stereo_pcm, 256_000, super::AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_STEREO_BALANCED_GLOBAL_GAIN, super::AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_STEREO_BALANCED_MAGNITUDE_BIAS, super::AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_STEREO_BALANCED_MAX_QUANTIZED_ABS, ) .unwrap(); assert_eq!( balanced_selected_standard_stereo_details, expected_balanced_selected_standard_stereo_details ); let balanced_selected_standard_stereo_profile = super::aac_balanced_standard_selected_scale_factor_profile_for_frame_details( &stereo_pcm, &balanced_selected_standard_stereo_details, ) .unwrap(); let expected_balanced_selected_standard_stereo_profile = super::aac_standard_selected_scale_factor_profile_for_frame_details_with_magnitude_bias( &stereo_pcm, &balanced_selected_standard_stereo_details, super::AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_STEREO_BALANCED_GLOBAL_GAIN, super::AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_STEREO_BALANCED_MAGNITUDE_BIAS, ) .unwrap(); assert_eq!( balanced_selected_standard_stereo_profile, expected_balanced_selected_standard_stereo_profile ); assert_eq!(balanced_selected_standard_stereo_profile.frames, 2); assert_eq!(balanced_selected_standard_stereo_profile.channels, 2); assert_eq!(balanced_selected_standard_stereo_profile.bands, 4 * max_sfb); assert!(balanced_selected_standard_stereo_profile .mean_delta .is_finite()); let recommended_selected_standard_stereo_limited_stream = super::encode_aac_adts_with_recommended_standard_spectral_offsets_and_selected_scale_factors_max_quantized_abs_and_bitrate( &stereo_pcm, 256_000, 12, ) .unwrap(); let core_recommended_selected_standard_stereo_limited_stream = super::encode_pcm_stereo_long_block_adts_stream_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_max_quantized_abs_and_bitrate_by_bit_cost( super::AdtsConfig::aac_lc(44_100, 2), super::AacLongBlockConfig::new(126, max_sfb as u8), super::AacLongBlockConfig::new(126, max_sfb as u8), &stereo_pcm, 0, long_offsets, 16, super::AAC_STANDARD_ID_PCM_STEP_CANDIDATES, 12, 256_000, &super::aac_scale_factor_delta_table(), ) .unwrap(); assert_eq!( recommended_selected_standard_stereo_limited_stream, core_recommended_selected_standard_stereo_limited_stream ); let recommended_selected_standard_stereo_limited_details = super::aac_recommended_standard_selected_scale_factor_frame_details_with_max_quantized_abs_and_bitrate( &stereo_pcm, 256_000, 12, ) .unwrap(); assert_eq!( recommended_selected_standard_stereo_limited_details .iter() .map(|selection| selection.frame_len) .max(), Some(max_adts_frame_len( &recommended_selected_standard_stereo_limited_stream )) ); let recommended_selected_standard_stereo_limited_m4a = super::encode_m4a_with_recommended_standard_spectral_offsets_and_selected_scale_factors_max_quantized_abs_and_bitrate( &stereo_pcm, 256_000, 12, ) .unwrap(); assert_eq!( &recommended_selected_standard_stereo_limited_m4a[4..8], b"ftyp" ); assert_eq!( high_level_selected_standard_stereo_details .iter() .map(|selection| selection.frame_len) .max(), Some(max_adts_frame_len( &high_level_selected_standard_stereo_bitrate_stream )) ); assert!( max_adts_frame_len(&high_level_selected_standard_stereo_bitrate_stream) <= super::aac_lc_adts_max_frame_len_for_bitrate(44_100, 256_000).unwrap() ); let mixed_quantized = [1, -1, 0, 1, 1, -1, 1, -1, 0, 0, 0, 0]; let pairs6 = [super::HuffmanEntry { symbol: super::AacSpectralMagnitudePair::new(1, 1), code: super::HuffmanCode::new(0b1, 1).unwrap(), }]; let pair_tables = super::AacSpectralMagnitudeTables { pairs6: &pairs6, ..Default::default() }; let quad_tables = super::AacSpectralMagnitudeQuadTables { quads3: &magnitude_table, ..Default::default() }; let mixed_sections = vec![ super::AacSpectralSection { start: 0, end: 4, codebook_id: 3, }, super::AacSpectralSection { start: 4, end: 8, codebook_id: 6, }, super::AacSpectralSection { start: 8, end: 12, codebook_id: 0, }, ]; assert_eq!( super::select_spectral_codebook_id_by_bit_cost( &mixed_quantized[..4], pair_tables, quad_tables, ) .unwrap(), 3 ); assert_eq!( super::plan_spectral_sections_by_bit_cost( &mixed_quantized, 4, pair_tables, quad_tables, ) .unwrap(), mixed_sections ); assert_eq!( super::pack_spectral_section_data_with_len(&mixed_sections, 4) .unwrap() .bit_len, 27 ); assert_eq!( super::pack_spectral_sections_by_codebook_id_with_sign_bits( &mixed_sections, &mixed_quantized, pair_tables, quad_tables, ) .unwrap() .bit_len, 11 ); assert_eq!( super::pack_sectioned_spectral_payload_by_codebook_id_with_sign_bits_by_bit_cost( &mixed_quantized, 4, pair_tables, quad_tables, ) .unwrap() .bit_len, 38 ); assert_eq!( super::split_sectioned_spectral_payload_by_codebook_id_with_sign_bits( &mixed_sections, &mixed_quantized, 4, pair_tables, quad_tables, ) .unwrap() .spectral_bits .bit_len, 11 ); assert_eq!( super::pack_sectioned_spectral_payload_by_codebook_id_with_sign_bits_and_scale_factor_bits( &mixed_sections, &mixed_quantized, 4, super::PackedBits { bytes: vec![0b1100_0000], bit_len: 2, }, pair_tables, quad_tables, ) .unwrap() .bit_len, 40 ); assert_eq!( super::pack_sectioned_spectral_payload_by_codebook_id_with_sign_bits_and_scale_factor_bits_by_bit_cost( &mixed_quantized, 4, super::PackedBits { bytes: vec![0b1100_0000], bit_len: 2, }, pair_tables, quad_tables, ) .unwrap() .bit_len, 40 ); assert_eq!( super::split_sectioned_spectral_payload_by_codebook_id_with_sign_bits_and_scale_factor_bits( &mixed_sections, &mixed_quantized, 4, super::PackedBits { bytes: vec![0b1100_0000], bit_len: 2, }, pair_tables, quad_tables, ) .unwrap() .section_and_scale_factor_bits .bit_len, 29 ); assert_eq!( super::split_sectioned_spectral_payload_by_codebook_id_with_sign_bits_by_bit_cost( &mixed_quantized, 4, pair_tables, quad_tables, ) .unwrap() .spectral_bits .bit_len, 11 ); assert_eq!( super::split_sectioned_spectral_payload_by_codebook_id_with_sign_bits_and_scale_factor_bits_by_bit_cost( &mixed_quantized, 4, super::PackedBits { bytes: vec![0b1100_0000], bit_len: 2, }, pair_tables, quad_tables, ) .unwrap() .section_and_scale_factor_bits .bit_len, 29 ); }

