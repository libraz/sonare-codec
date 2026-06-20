    #[test]
    fn plans_aac_quad_sections_by_bit_cost() {
        let quantized = [1, -1, 0, 1, 0, 1, -1, 0, 0, 0, 0, 0];
        let longer = [
            HuffmanEntry {
                symbol: AacSpectralMagnitudeQuad::new(1, 1, 0, 1),
                code: HuffmanCode::new(0b1110, 4).unwrap(),
            },
            HuffmanEntry {
                symbol: AacSpectralMagnitudeQuad::new(0, 1, 1, 0),
                code: HuffmanCode::new(0b1111, 4).unwrap(),
            },
        ];
        let shorter = [
            HuffmanEntry {
                symbol: AacSpectralMagnitudeQuad::new(1, 1, 0, 1),
                code: HuffmanCode::new(0b10, 2).unwrap(),
            },
            HuffmanEntry {
                symbol: AacSpectralMagnitudeQuad::new(0, 1, 1, 0),
                code: HuffmanCode::new(0b11, 2).unwrap(),
            },
        ];
        let tables = AacSpectralMagnitudeQuadTables {
            quads1: &longer,
            quads3: &shorter,
            ..Default::default()
        };

        assert_eq!(
            select_quad_codebook_by_bit_cost(&quantized[..8], tables).unwrap(),
            3
        );
        assert_eq!(
            select_quad_codebook_by_bit_cost(&quantized[8..], tables).unwrap(),
            0
        );
        assert_eq!(
            plan_quad_sections_by_bit_cost(&quantized, 4, tables).unwrap(),
            vec![
                AacQuadSection {
                    start: 0,
                    end: 8,
                    codebook_id: 3,
                },
                AacQuadSection {
                    start: 8,
                    end: 12,
                    codebook_id: 0,
                },
            ]
        );
        assert_eq!(
            pack_sectioned_spectral_quad_payload_with_sign_bits_by_bit_cost(&quantized, 4, tables)
                .unwrap(),
            PackedBits {
                bytes: vec![0b0011_0001, 0b0000_0000, 0b0110_0101, 0b1010_0000],
                bit_len: 27,
            }
        );
        assert!(plan_quad_sections_by_bit_cost(&quantized, 2, tables).is_err());
        assert!(select_quad_codebook_by_bit_cost(
            &quantized[..8],
            AacSpectralMagnitudeQuadTables::default()
        )
        .is_err());
    }

    #[test]
    fn exposes_core_unit_spectral_table_fixtures_for_aac_workbenches() {
        let quantized = [1, -1, 0, 1, 1, -1, 1, -1, 0, 0, 0, 0];
        let pair_tables = aac_unit_codebook6_spectral_tables();
        let quad_tables = aac_unit_quad_spectral_tables();

        assert_eq!(pair_tables.pairs6.len(), 1);
        assert_eq!(
            pair_tables.pairs6[0].symbol,
            AacSpectralMagnitudePair::new(1, 1)
        );
        assert_eq!(
            pair_tables.pairs6[0].code,
            HuffmanCode::new(0b1, 1).unwrap()
        );
        assert_eq!(quad_tables.quads1.len(), 2);
        assert_eq!(quad_tables.quads3.len(), 2);
        assert_eq!(
            select_quad_codebook_by_bit_cost(&quantized[..4], quad_tables).unwrap(),
            3
        );
        assert_eq!(
            plan_spectral_sections_by_bit_cost(&quantized, 4, pair_tables, quad_tables).unwrap(),
            vec![
                AacSpectralSection {
                    start: 0,
                    end: 4,
                    codebook_id: 3,
                },
                AacSpectralSection {
                    start: 4,
                    end: 8,
                    codebook_id: 6,
                },
                AacSpectralSection {
                    start: 8,
                    end: 12,
                    codebook_id: 0,
                },
            ]
        );
        assert_eq!(
            plan_spectral_sections_by_offsets_by_bit_cost(
                &quantized,
                &[0, 4, 8, 12],
                pair_tables,
                quad_tables
            )
            .unwrap(),
            vec![
                AacSpectralSection {
                    start: 0,
                    end: 4,
                    codebook_id: 3,
                },
                AacSpectralSection {
                    start: 4,
                    end: 8,
                    codebook_id: 6,
                },
                AacSpectralSection {
                    start: 8,
                    end: 12,
                    codebook_id: 0,
                },
            ]
        );
        assert_eq!(
            plan_aac_lc_standard_spectral_sections_by_bit_cost(&[1, -1, 0, 1, 17, 0, 0, 0], 4)
                .unwrap(),
            vec![
                AacSpectralSection {
                    start: 0,
                    end: 4,
                    codebook_id: 4,
                },
                AacSpectralSection {
                    start: 4,
                    end: 8,
                    codebook_id: 11,
                },
            ]
        );
        assert_eq!(
            plan_aac_lc_standard_spectral_sections_by_offsets_by_bit_cost(
                &[1, -1, 0, 1, 17, 0, 0, 0],
                &[0, 4, 8]
            )
            .unwrap(),
            vec![
                AacSpectralSection {
                    start: 0,
                    end: 4,
                    codebook_id: 4,
                },
                AacSpectralSection {
                    start: 4,
                    end: 8,
                    codebook_id: 11,
                },
            ]
        );
        let standard_payload = pack_aac_lc_standard_spectral_payload_with_sign_bits_by_bit_cost(
            &[1, -1, 0, 1, 17, 0, 0, 0],
            4,
        )
        .unwrap();
        let standard_split = split_aac_lc_standard_spectral_payload_with_sign_bits_by_bit_cost(
            &[1, -1, 0, 1, 17, 0, 0, 0],
            4,
        )
        .unwrap();
        let scale_factor_bits = PackedBits {
            bytes: vec![0b1100_0000],
            bit_len: 2,
        };
        let standard_payload_with_scale =
            pack_aac_lc_standard_spectral_payload_with_sign_bits_and_scale_factor_bits_by_bit_cost(
                &[1, -1, 0, 1, 17, 0, 0, 0],
                4,
                scale_factor_bits.clone(),
            )
            .unwrap();
        let standard_split_with_scale =
            split_aac_lc_standard_spectral_payload_with_sign_bits_and_scale_factor_bits_by_bit_cost(
                &[1, -1, 0, 1, 17, 0, 0, 0],
                4,
                scale_factor_bits,
            )
            .unwrap();
        let standard_offsets_payload =
            pack_aac_lc_standard_spectral_payload_with_offsets_and_sign_bits_by_bit_cost(
                &[1, -1, 0, 1, 17, 0, 0, 0],
                &[0, 4, 8],
            )
            .unwrap();
        let standard_offsets_split =
            split_aac_lc_standard_spectral_payload_with_offsets_and_sign_bits_by_bit_cost(
                &[1, -1, 0, 1, 17, 0, 0, 0],
                &[0, 4, 8],
            )
            .unwrap();
        let offsets_scale_factor_bits = PackedBits {
            bytes: vec![0b1100_0000],
            bit_len: 2,
        };
        let standard_offsets_payload_with_scale =
            pack_aac_lc_standard_spectral_payload_with_offsets_and_sign_bits_and_scale_factor_bits_by_bit_cost(
                &[1, -1, 0, 1, 17, 0, 0, 0],
                &[0, 4, 8],
                offsets_scale_factor_bits.clone(),
            )
            .unwrap();
        let standard_offsets_split_with_scale =
            split_aac_lc_standard_spectral_payload_with_offsets_and_sign_bits_and_scale_factor_bits_by_bit_cost(
                &[1, -1, 0, 1, 17, 0, 0, 0],
                &[0, 4, 8],
                offsets_scale_factor_bits,
            )
            .unwrap();
        assert_eq!(standard_split.section_and_scale_factor_bits.bit_len, 18);
        assert_eq!(standard_split.spectral_bits.bit_len, 26);
        assert_eq!(standard_payload.bit_len, 44);
        assert_eq!(
            standard_split_with_scale
                .section_and_scale_factor_bits
                .bit_len,
            20
        );
        assert_eq!(standard_split_with_scale.spectral_bits.bit_len, 26);
        assert_eq!(standard_payload_with_scale.bit_len, 46);
        assert_eq!(
            standard_offsets_split.section_and_scale_factor_bits.bit_len,
            18
        );
        assert_eq!(standard_offsets_split.spectral_bits.bit_len, 26);
        assert_eq!(standard_offsets_payload.bit_len, 44);
        assert_eq!(
            standard_offsets_split_with_scale
                .section_and_scale_factor_bits
                .bit_len,
            20
        );
        assert_eq!(standard_offsets_split_with_scale.spectral_bits.bit_len, 26);
        assert_eq!(standard_offsets_payload_with_scale.bit_len, 46);
        let spectral_scale_deltas = plan_spectral_scale_factor_deltas_by_offsets(
            &plan_aac_lc_standard_spectral_sections_by_offsets_by_bit_cost(
                &[1, -1, 0, 1, 17, 0, 0, 0],
                &[0, 4, 8],
            )
            .unwrap(),
            &[0, 4, 8],
            &[100, 100],
            100,
        )
        .unwrap();
        assert_eq!(
            spectral_scale_deltas,
            vec![AacScaleFactorDelta::new(0), AacScaleFactorDelta::new(0)]
        );
        let standard_adts =
            encode_quantized_mono_adts_with_standard_spectral_offsets_and_scale_factors_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 1),
                AacLongBlockConfig::new(100, 2),
                &[1, -1, 0, 1, 17, 0, 0, 0],
                &[0, 4, 8],
                &[100, 100],
                aac_scale_factor_delta_zero_table(),
            )
            .unwrap();
        let standard_frame = parse_adts_frame(&standard_adts).unwrap();
        assert_eq!(standard_frame.frame_len, standard_adts.len());
        assert_eq!(standard_frame.sample_rate, 44_100);
        assert_eq!(standard_frame.channels, 1);
        let standard_stereo_adts =
            encode_quantized_stereo_adts_with_standard_spectral_offsets_and_scale_factors_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 2),
                AacQuantizedChannel::new(
                    AacLongBlockConfig::new(100, 2),
                    &[1, -1, 0, 1, 17, 0, 0, 0],
                    &[100, 100],
                ),
                AacQuantizedChannel::new(
                    AacLongBlockConfig::new(100, 2),
                    &[0, 1, -1, 0, 0, 0, 17, 0],
                    &[100, 100],
                ),
                &[0, 4, 8],
                aac_scale_factor_delta_zero_table(),
            )
            .unwrap();
        let standard_stereo_frame = parse_adts_frame(&standard_stereo_adts).unwrap();
        assert_eq!(standard_stereo_frame.frame_len, standard_stereo_adts.len());
        assert_eq!(standard_stereo_frame.sample_rate, 44_100);
        assert_eq!(standard_stereo_frame.channels, 2);
        let pcm = AudioBuffer::new(
            44_100,
            1,
            (0..2048)
                .map(|sample| ((sample as f32) * 0.02).sin() * 0.2)
                .collect(),
        )
        .unwrap();
        let long_offsets = aac_lc_long_window_scale_factor_band_offsets(44_100).unwrap();
        let max_sfb = long_offsets.len() - 1;
        let stream_channel = AacLongBlockConfig::new(128, max_sfb as u8);
        let scale_frame0 = vec![128_i16; max_sfb];
        let scale_frame1 = vec![128_i16; max_sfb];
        let scale_frames: [&[i16]; 2] = [&scale_frame0, &scale_frame1];
        let standard_stream =
            encode_pcm_mono_long_block_adts_stream_with_standard_spectral_offsets_and_scale_factors_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 1),
                AacScaleFactorSequence::new(stream_channel, &scale_frames),
                &pcm,
                0,
                0.005,
                long_offsets,
                aac_scale_factor_delta_zero_table(),
            )
            .unwrap();
        let mut frame_count = 0;
        let mut remaining = standard_stream.as_slice();
        while !remaining.is_empty() {
            let frame = parse_adts_frame(remaining).unwrap();
            assert_eq!(frame.sample_rate, 44_100);
            assert_eq!(frame.channels, 1);
            remaining = &remaining[frame.frame_len..];
            frame_count += 1;
        }
        assert_eq!(frame_count, 2);
        let standard_stream_max_frame =
            encode_pcm_mono_long_block_adts_stream_with_standard_spectral_offsets_and_scale_factors_and_max_frame_len_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 1),
                AacScaleFactorSequence::new(stream_channel, &scale_frames),
                &pcm,
                0,
                long_offsets,
                AAC_LC_PCM_STEP_CANDIDATES,
                16,
                aac_scale_factor_delta_zero_table(),
            )
            .unwrap();
        assert!(max_adts_frame_len(&standard_stream_max_frame) <= 16);
        let standard_stream_bitrate =
            encode_pcm_mono_long_block_adts_stream_with_standard_spectral_offsets_and_scale_factors_and_bitrate_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 1),
                AacScaleFactorSequence::new(stream_channel, &scale_frames),
                &pcm,
                0,
                long_offsets,
                AAC_LC_PCM_STEP_CANDIDATES,
                128_000,
                aac_scale_factor_delta_zero_table(),
            )
            .unwrap();
        assert!(
            max_adts_frame_len(&standard_stream_bitrate)
                <= aac_lc_adts_max_frame_len_for_bitrate(44_100, 128_000).unwrap()
        );
        let standard_stream_details =
            select_aac_lc_mono_pcm_stream_frame_details_with_standard_spectral_offsets_and_scale_factors_and_bitrate_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 1),
                AacScaleFactorSequence::new(stream_channel, &scale_frames),
                &pcm,
                0,
                long_offsets,
                AAC_LC_PCM_STEP_CANDIDATES,
                128_000,
                aac_scale_factor_delta_zero_table(),
            )
            .unwrap();
        assert_eq!(standard_stream_details.len(), 2);
        assert!(standard_stream_details.iter().all(|detail| {
            detail.frame_len <= aac_lc_adts_max_frame_len_for_bitrate(44_100, 128_000).unwrap()
        }));
        let stereo_pcm = AudioBuffer::new(
            44_100,
            2,
            (0..2048)
                .flat_map(|sample| {
                    [
                        ((sample as f32) * 0.02).sin() * 0.2,
                        ((sample as f32) * 0.017).cos() * 0.18,
                    ]
                })
                .collect(),
        )
        .unwrap();
        let standard_stereo_stream =
            encode_pcm_stereo_long_block_adts_stream_with_standard_spectral_offsets_and_scale_factors_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 2),
                AacScaleFactorSequence::new(stream_channel, &scale_frames),
                AacScaleFactorSequence::new(stream_channel, &scale_frames),
                &stereo_pcm,
                0,
                0.005,
                long_offsets,
                aac_scale_factor_delta_zero_table(),
            )
            .unwrap();
        let mut stereo_frame_count = 0;
        let mut remaining = standard_stereo_stream.as_slice();
        while !remaining.is_empty() {
            let frame = parse_adts_frame(remaining).unwrap();
            assert_eq!(frame.sample_rate, 44_100);
            assert_eq!(frame.channels, 2);
            remaining = &remaining[frame.frame_len..];
            stereo_frame_count += 1;
        }
        assert_eq!(stereo_frame_count, 2);
        let standard_stereo_stream_max_frame =
            encode_pcm_stereo_long_block_adts_stream_with_standard_spectral_offsets_and_scale_factors_and_max_frame_len_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 2),
                AacScaleFactorSequence::new(stream_channel, &scale_frames),
                AacScaleFactorSequence::new(stream_channel, &scale_frames),
                &stereo_pcm,
                0,
                long_offsets,
                AAC_LC_PCM_STEP_CANDIDATES,
                28,
                aac_scale_factor_delta_zero_table(),
            )
            .unwrap();
        assert!(max_adts_frame_len(&standard_stereo_stream_max_frame) <= 28);
        let standard_stereo_stream_bitrate =
            encode_pcm_stereo_long_block_adts_stream_with_standard_spectral_offsets_and_scale_factors_and_bitrate_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 2),
                AacScaleFactorSequence::new(stream_channel, &scale_frames),
                AacScaleFactorSequence::new(stream_channel, &scale_frames),
                &stereo_pcm,
                0,
                long_offsets,
                AAC_LC_PCM_STEP_CANDIDATES,
                256_000,
                aac_scale_factor_delta_zero_table(),
            )
            .unwrap();
        assert!(
            max_adts_frame_len(&standard_stereo_stream_bitrate)
                <= aac_lc_adts_max_frame_len_for_bitrate(44_100, 256_000).unwrap()
        );
        let standard_stereo_stream_details =
            select_aac_lc_stereo_pcm_stream_frame_details_with_standard_spectral_offsets_and_scale_factors_and_bitrate_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 2),
                AacScaleFactorSequence::new(stream_channel, &scale_frames),
                AacScaleFactorSequence::new(stream_channel, &scale_frames),
                &stereo_pcm,
                0,
                long_offsets,
                AAC_LC_PCM_STEP_CANDIDATES,
                256_000,
                aac_scale_factor_delta_zero_table(),
            )
            .unwrap();
        assert_eq!(standard_stereo_stream_details.len(), 2);
        assert!(standard_stereo_stream_details.iter().all(|detail| {
            detail.frame_len <= aac_lc_adts_max_frame_len_for_bitrate(44_100, 256_000).unwrap()
        }));
        assert!(pack_spectral_section_data_with_offsets(
            &[AacSpectralSection {
                start: 0,
                end: 4,
                codebook_id: 12,
            }],
            &[0, 4]
        )
        .is_err());
    }

    #[test]
    fn plans_standard_id_spectral_sections_by_bit_cost() {
        let quantized = [1, -1, 0, 1, 1, -1, 1, -1, 0, 0, 0, 0];
        let pairs6 = [HuffmanEntry {
            symbol: AacSpectralMagnitudePair::new(1, 1),
            code: HuffmanCode::new(0b1, 1).unwrap(),
        }];
        let quads3 = [HuffmanEntry {
            symbol: AacSpectralMagnitudeQuad::new(1, 1, 0, 1),
            code: HuffmanCode::new(0b10, 2).unwrap(),
        }];
        let pair_tables = AacSpectralMagnitudeTables {
            pairs6: &pairs6,
            ..Default::default()
        };
        let quad_tables = AacSpectralMagnitudeQuadTables {
            quads3: &quads3,
            ..Default::default()
        };
        let sections = vec![
            AacSpectralSection {
                start: 0,
                end: 4,
                codebook_id: 3,
            },
            AacSpectralSection {
                start: 4,
                end: 8,
                codebook_id: 6,
            },
            AacSpectralSection {
                start: 8,
                end: 12,
                codebook_id: 0,
            },
        ];

        assert_eq!(
            select_spectral_codebook_id_by_bit_cost(&quantized[..4], pair_tables, quad_tables)
                .unwrap(),
            3
        );
        assert_eq!(
            select_spectral_codebook_id_by_bit_cost(&quantized[4..8], pair_tables, quad_tables)
                .unwrap(),
            6
        );
        assert_eq!(
            select_spectral_codebook_id_by_bit_cost(&quantized[8..], pair_tables, quad_tables)
                .unwrap(),
            0
        );
        assert_eq!(
            plan_spectral_sections_by_bit_cost(&quantized, 4, pair_tables, quad_tables).unwrap(),
            sections
        );
        assert_eq!(
            pack_spectral_section_data_with_len(&sections, 4).unwrap(),
            PackedBits {
                bytes: vec![0b0011_0000, 0b1011_0000, 0b0100_0000, 0b0010_0000],
                bit_len: 27,
            }
        );
        assert_eq!(
            pack_spectral_sections_by_codebook_id_with_sign_bits(
                &sections,
                &quantized,
                pair_tables,
                quad_tables
            )
            .unwrap(),
            PackedBits {
                bytes: vec![0b1001_0101, 0b1010_0000],
                bit_len: 11,
            }
        );
        let packed = pack_sectioned_spectral_payload_by_codebook_id_with_sign_bits(
            &sections,
            &quantized,
            4,
            pair_tables,
            quad_tables,
        )
        .unwrap();
        assert_eq!(packed.bit_len, 38);
        let split_payload = split_sectioned_spectral_payload_by_codebook_id_with_sign_bits(
            &sections,
            &quantized,
            4,
            pair_tables,
            quad_tables,
        )
        .unwrap();
        assert_eq!(split_payload.section_and_scale_factor_bits.bit_len, 27);
        assert_eq!(split_payload.spectral_bits.bit_len, 11);
        let scale_factor_bits = PackedBits {
            bytes: vec![0b1100_0000],
            bit_len: 2,
        };
        let with_scale_factor_bits =
            pack_sectioned_spectral_payload_by_codebook_id_with_sign_bits_and_scale_factor_bits(
                &sections,
                &quantized,
                4,
                scale_factor_bits.clone(),
                pair_tables,
                quad_tables,
            )
            .unwrap();
        assert_eq!(with_scale_factor_bits.bit_len, 40);
        let split_with_scale_factor_bits =
            split_sectioned_spectral_payload_by_codebook_id_with_sign_bits_and_scale_factor_bits(
                &sections,
                &quantized,
                4,
                scale_factor_bits,
                pair_tables,
                quad_tables,
            )
            .unwrap();
        assert_eq!(
            split_with_scale_factor_bits
                .section_and_scale_factor_bits
                .bit_len,
            29
        );
        assert_eq!(split_with_scale_factor_bits.spectral_bits.bit_len, 11);
        assert_eq!(
            pack_sectioned_spectral_payload_by_codebook_id_with_sign_bits_by_bit_cost(
                &quantized,
                4,
                pair_tables,
                quad_tables
            )
            .unwrap(),
            packed
        );
        assert_eq!(
            pack_sectioned_spectral_payload_by_codebook_id_with_sign_bits_and_scale_factor_bits_by_bit_cost(
                &quantized,
                4,
                PackedBits {
                    bytes: vec![0b1100_0000],
                    bit_len: 2,
                },
                pair_tables,
                quad_tables
            )
            .unwrap()
            .bit_len,
            40
        );
        assert_eq!(
            split_sectioned_spectral_payload_by_codebook_id_with_sign_bits_by_bit_cost(
                &quantized,
                4,
                pair_tables,
                quad_tables
            )
            .unwrap()
            .section_and_scale_factor_bits
            .bit_len,
            27
        );
        let split_by_bit_cost_with_scale =
            split_sectioned_spectral_payload_by_codebook_id_with_sign_bits_and_scale_factor_bits_by_bit_cost(
                &quantized,
                4,
                PackedBits {
                    bytes: vec![0b1100_0000],
                    bit_len: 2,
                },
                pair_tables,
                quad_tables,
            )
            .unwrap();
        assert_eq!(
            split_by_bit_cost_with_scale
                .section_and_scale_factor_bits
                .bit_len,
            29
        );
        assert_eq!(split_by_bit_cost_with_scale.spectral_bits.bit_len, 11);
        assert!(pack_spectral_section_data_with_len(
            &[AacSpectralSection {
                start: 0,
                end: 4,
                codebook_id: 12,
            }],
            4,
        )
        .is_err());
        assert!(
            plan_spectral_sections_by_bit_cost(&quantized, 3, pair_tables, quad_tables).is_err()
        );
    }

    #[test]
    fn packs_aac_sectioned_spectral_payload_with_bit_cost_sections() {
        let quantized = [1, -1, 0, 0];
        let pairs1 = [HuffmanEntry {
            symbol: AacSpectralMagnitudePair::new(1, 1),
            code: HuffmanCode::new(0b1111, 4).unwrap(),
        }];
        let pairs5 = [HuffmanEntry {
            symbol: AacSpectralMagnitudePair::new(1, 1),
            code: HuffmanCode::new(0b0, 1).unwrap(),
        }];
        let tables = AacSpectralMagnitudeTables {
            pairs1: &pairs1,
            pairs5: &pairs5,
            pairs6: &[],
            escape: &[],
        };
        let expected_sections = vec![
            AacSection {
                start: 0,
                end: 2,
                codebook: AacCodebook::SignedPairs5,
            },
            AacSection {
                start: 2,
                end: 4,
                codebook: AacCodebook::Zero,
            },
        ];
        let expected = pack_sectioned_spectral_payload_with_sign_bits(
            &expected_sections,
            &quantized,
            2,
            tables,
        )
        .unwrap();

        let packed =
            pack_sectioned_spectral_payload_with_sign_bits_by_bit_cost(&quantized, 2, tables)
                .unwrap();
        let with_scale_factors =
            pack_sectioned_spectral_payload_with_sign_bits_and_scale_factor_bits_by_bit_cost(
                &quantized,
                2,
                PackedBits {
                    bytes: vec![0b1000_0000],
                    bit_len: 1,
                },
                tables,
            )
            .unwrap();

        assert_eq!(packed, expected);
        assert_eq!(packed.bit_len, 21);
        assert_eq!(with_scale_factors.bit_len, 22);
    }

    #[test]
    fn packs_aac_sectioned_escape_spectral_payload_with_sign_bits() {
        let quantized = [0, 0, 17, 0];
        let sections = plan_sections(&quantized, 2).unwrap();
        let escape = [HuffmanEntry {
            symbol: AacSpectralMagnitudePair::new(16, 0),
            code: HuffmanCode::new(0b10, 2).unwrap(),
        }];

        let packed = pack_sectioned_spectral_payload_with_sign_bits(
            &sections,
            &quantized,
            2,
            AacSpectralMagnitudeTables {
                pairs1: &[],
                pairs5: &[],
                pairs6: &[],
                escape: &escape,
            },
        )
        .unwrap();

        assert_eq!(
            packed,
            PackedBits {
                bytes: vec![0x00, 0xd8, 0x60, 0x40],
                bit_len: 26,
            }
        );
    }

    #[test]
    fn packs_aac_channel_payload_parts_with_scale_factor_bits() {
        let section_bits = PackedBits {
            bytes: vec![0x00, 0x42, 0x12, 0x88],
            bit_len: 30,
        };
        let scale_factor_bits = PackedBits {
            bytes: vec![0b1010_0000],
            bit_len: 3,
        };
        let spectral_bits = PackedBits {
            bytes: vec![0b1001_0011, 0b1000_0000],
            bit_len: 10,
        };

        assert_eq!(
            pack_channel_payload_parts(section_bits, scale_factor_bits, spectral_bits).unwrap(),
            PackedBits {
                bytes: vec![0x00, 0x42, 0x12, 0x8a, 0xc9, 0xc0],
                bit_len: 43,
            }
        );
    }

    #[test]
    fn packs_aac_sectioned_payload_with_scale_factor_bits() {
        let quantized = [0, 0, 1, -1, 3, 0, -2, 2];
        let sections = vec![
            AacSection {
                start: 0,
                end: 2,
                codebook: AacCodebook::Zero,
            },
            AacSection {
                start: 2,
                end: 4,
                codebook: AacCodebook::SignedPairs1,
            },
            AacSection {
                start: 4,
                end: 8,
                codebook: AacCodebook::SignedPairs5,
            },
        ];
        let scale_factor_bits = PackedBits {
            bytes: vec![0b1010_0000],
            bit_len: 3,
        };
        let signed_pairs1 = [HuffmanEntry {
            symbol: AacSpectralPair::new(1, -1),
            code: HuffmanCode::new(0b10, 2).unwrap(),
        }];
        let signed_pairs5 = [
            HuffmanEntry {
                symbol: AacSpectralPair::new(3, 0),
                code: HuffmanCode::new(0b0, 1).unwrap(),
            },
            HuffmanEntry {
                symbol: AacSpectralPair::new(-2, 2),
                code: HuffmanCode::new(0b11, 2).unwrap(),
            },
        ];
        let pairs1 = [HuffmanEntry {
            symbol: AacSpectralMagnitudePair::new(1, 1),
            code: HuffmanCode::new(0b10, 2).unwrap(),
        }];
        let pairs5 = [
            HuffmanEntry {
                symbol: AacSpectralMagnitudePair::new(3, 0),
                code: HuffmanCode::new(0b0, 1).unwrap(),
            },
            HuffmanEntry {
                symbol: AacSpectralMagnitudePair::new(2, 2),
                code: HuffmanCode::new(0b11, 2).unwrap(),
            },
        ];

        assert_eq!(
            pack_sectioned_spectral_payload_with_scale_factor_bits(
                &sections,
                &quantized,
                2,
                scale_factor_bits.clone(),
                AacSpectralTables {
                    signed_pairs1: &signed_pairs1,
                    signed_pairs5: &signed_pairs5,
                    signed_pairs6: &[],
                    escape: &[],
                },
            )
            .unwrap(),
            PackedBits {
                bytes: vec![0x00, 0x88, 0x54, 0x56, 0x60],
                bit_len: 35,
            }
        );
        assert_eq!(
            pack_sectioned_spectral_payload_with_sign_bits_and_scale_factor_bits(
                &sections,
                &quantized,
                2,
                scale_factor_bits,
                AacSpectralMagnitudeTables {
                    pairs1: &pairs1,
                    pairs5: &pairs5,
                    pairs6: &[],
                    escape: &[],
                },
            )
            .unwrap(),
            PackedBits {
                bytes: vec![0x00, 0x88, 0x54, 0x56, 0x4e],
                bit_len: 40,
            }
        );
    }

