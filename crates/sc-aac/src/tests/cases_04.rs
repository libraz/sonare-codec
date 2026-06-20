    #[test]
    fn packs_long_block_individual_channel_stream_with_payload_bits() {
        let payload = PackedBits {
            bytes: vec![0b1010_0000],
            bit_len: 3,
        };

        let ics =
            pack_long_block_individual_channel_stream(AacLongBlockConfig::new(120, 3), &payload)
                .unwrap();

        assert_eq!(
            ics,
            PackedBits {
                bytes: vec![0x78, 0x00, 0xd4, 0x00],
                bit_len: 25,
            }
        );
        assert!(pack_long_block_individual_channel_stream(
            AacLongBlockConfig::new(120, 0),
            &payload,
        )
        .is_err());
    }

    #[test]
    fn packs_single_channel_raw_data_block_from_ics_payload() {
        let empty_payload = PackedBits {
            bytes: Vec::new(),
            bit_len: 0,
        };
        let payload = PackedBits {
            bytes: vec![0b1010_0000],
            bit_len: 3,
        };

        assert_eq!(
            pack_single_channel_raw_data_block(AacLongBlockConfig::new(0, 0), &empty_payload)
                .unwrap(),
            [0x00, 0x00, 0x00, 0x07]
        );
        assert_eq!(
            pack_single_channel_raw_data_block(AacLongBlockConfig::new(120, 3), &payload).unwrap(),
            [0x00, 0xf0, 0x01, 0xa8, 0xe0]
        );
    }

    #[test]
    fn packs_channel_pair_raw_data_block_from_ics_payloads() {
        let empty_payload = PackedBits {
            bytes: Vec::new(),
            bit_len: 0,
        };
        let left_payload = PackedBits {
            bytes: vec![0b1010_0000],
            bit_len: 3,
        };
        let right_payload = PackedBits {
            bytes: vec![0b0100_0000],
            bit_len: 2,
        };

        assert_eq!(
            pack_channel_pair_raw_data_block(
                AacLongBlockConfig::new(0, 0),
                &empty_payload,
                AacLongBlockConfig::new(0, 0),
                &empty_payload,
            )
            .unwrap(),
            [0x20, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0e]
        );
        assert_eq!(
            pack_channel_pair_raw_data_block(
                AacLongBlockConfig::new(120, 3),
                &left_payload,
                AacLongBlockConfig::new(64, 2),
                &right_payload,
            )
            .unwrap(),
            [0x20, 0x78, 0x00, 0xd4, 0x20, 0x00, 0x44, 0x70]
        );
    }

    #[test]
    fn encodes_quantized_mono_long_block_as_adts() {
        let quantized = [0, 0, 1, -1, 3, 0, -2, 2];
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

        let adts = encode_quantized_mono_adts(
            AdtsConfig::aac_lc(44_100, 1),
            AacLongBlockConfig::new(120, 3),
            &quantized,
            2,
            AacSpectralMagnitudeTables {
                pairs1: &pairs1,
                pairs5: &pairs5,
                pairs6: &[],
                escape: &[],
            },
        )
        .unwrap();

        assert_eq!(
            adts,
            [
                0xff, 0xf1, 0x50, 0x40, 0x02, 0x3f, 0xfc, 0x00, 0xf0, 0x01, 0x80, 0x2e, 0x31, 0x8f,
                0x37, 0x2b, 0x80,
            ]
        );
        assert!(encode_quantized_mono_adts(
            AdtsConfig::aac_lc(44_100, 2),
            AacLongBlockConfig::new(120, 3),
            &quantized,
            2,
            AacSpectralMagnitudeTables {
                pairs1: &pairs1,
                pairs5: &pairs5,
                pairs6: &[],
                escape: &[],
            },
        )
        .is_err());
    }

    #[test]
    fn encodes_quantized_mono_escape_long_block_as_adts() {
        let quantized = [0, 0, 17, 0];
        let escape = [HuffmanEntry {
            symbol: AacSpectralMagnitudePair::new(16, 0),
            code: HuffmanCode::new(0b10, 2).unwrap(),
        }];
        let sections = plan_sections(&quantized, 2).unwrap();
        let payload = split_sectioned_spectral_payload_with_sign_bits(
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
        let access_unit =
            pack_single_channel_raw_data_block_parts(AacLongBlockConfig::new(120, 2), &payload)
                .unwrap();

        let adts = encode_quantized_mono_adts(
            AdtsConfig::aac_lc(44_100, 1),
            AacLongBlockConfig::new(120, 2),
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

        assert_eq!(&adts[..2], &[0xff, 0xf1]);
        assert_eq!(&adts[7..], access_unit);
        assert!(encode_quantized_mono_adts(
            AdtsConfig::aac_lc(44_100, 1),
            AacLongBlockConfig::new(120, 2),
            &quantized,
            2,
            AacSpectralMagnitudeTables::default(),
        )
        .is_err());
    }

    #[test]
    fn encodes_quantized_mono_long_block_with_scale_factors_as_adts() {
        let quantized = [0, 0, 1, -1, 3, 0, -2, 2];
        let scale_factor_table = [
            HuffmanEntry {
                symbol: AacScaleFactorDelta::new(0),
                code: HuffmanCode::new(0b111, 3).unwrap(),
            },
            HuffmanEntry {
                symbol: AacScaleFactorDelta::new(-1),
                code: HuffmanCode::new(0b0, 1).unwrap(),
            },
            HuffmanEntry {
                symbol: AacScaleFactorDelta::new(1),
                code: HuffmanCode::new(0b10, 2).unwrap(),
            },
            HuffmanEntry {
                symbol: AacScaleFactorDelta::new(2),
                code: HuffmanCode::new(0b110, 3).unwrap(),
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

        let adts = encode_quantized_mono_adts_with_scale_factors(
            AdtsConfig::aac_lc(44_100, 1),
            AacLongBlockConfig::new(120, 3),
            &quantized,
            2,
            &[119, 121, 123, 122],
            &scale_factor_table,
            AacSpectralMagnitudeTables {
                pairs1: &pairs1,
                pairs5: &pairs5,
                pairs6: &[],
                escape: &[],
            },
        )
        .unwrap();

        assert_eq!(
            adts,
            [
                0xff, 0xf1, 0x50, 0x40, 0x02, 0x3f, 0xfc, 0x00, 0xf0, 0x01, 0x80, 0x2e, 0x3b, 0x06,
                0x3c, 0xdc, 0xae,
            ]
        );
        assert!(encode_quantized_mono_adts_with_scale_factors(
            AdtsConfig::aac_lc(44_100, 1),
            AacLongBlockConfig::new(120, 3),
            &quantized,
            2,
            &[119, 121],
            &scale_factor_table,
            AacSpectralMagnitudeTables {
                pairs1: &pairs1,
                pairs5: &pairs5,
                pairs6: &[],
                escape: &[],
            },
        )
        .is_err());
    }

    #[test]
    fn encodes_quantized_mono_long_block_with_selected_scale_factors_as_adts() {
        let quantized = [0, 0, 1, -1, 3, 0, -2, 2];
        let scale_factor_table = [
            HuffmanEntry {
                symbol: AacScaleFactorDelta::new(0),
                code: HuffmanCode::new(0b0, 1).unwrap(),
            },
            HuffmanEntry {
                symbol: AacScaleFactorDelta::new(1),
                code: HuffmanCode::new(0b10, 2).unwrap(),
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
        let tables = AacSpectralMagnitudeTables {
            pairs1: &pairs1,
            pairs5: &pairs5,
            pairs6: &[],
            escape: &[],
        };

        let selected = encode_quantized_mono_adts_with_selected_scale_factors(
            AdtsConfig::aac_lc(44_100, 1),
            AacLongBlockConfig::new(120, 3),
            &quantized,
            2,
            &scale_factor_table,
            tables,
        )
        .unwrap();
        let manual = encode_quantized_mono_adts_with_scale_factors(
            AdtsConfig::aac_lc(44_100, 1),
            AacLongBlockConfig::new(120, 3),
            &quantized,
            2,
            &[120, 121, 122, 122],
            &scale_factor_table,
            tables,
        )
        .unwrap();

        assert_eq!(selected, manual);
        assert!(encode_quantized_mono_adts_with_selected_scale_factors(
            AdtsConfig::aac_lc(44_100, 2),
            AacLongBlockConfig::new(120, 3),
            &quantized,
            2,
            &scale_factor_table,
            tables,
        )
        .is_err());
    }

    #[test]
    fn encodes_quantized_mono_long_block_with_bit_cost_sections_as_adts() {
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
        let expected_payload = split_sectioned_spectral_payload_with_sign_bits(
            &expected_sections,
            &quantized,
            2,
            tables,
        )
        .unwrap();
        let expected_access_unit = pack_single_channel_raw_data_block_parts(
            AacLongBlockConfig::new(120, 2),
            &expected_payload,
        )
        .unwrap();
        let expected = frame_adts(AdtsConfig::aac_lc(44_100, 1), &expected_access_unit).unwrap();

        let encoded = encode_quantized_mono_adts_by_bit_cost(
            AdtsConfig::aac_lc(44_100, 1),
            AacLongBlockConfig::new(120, 2),
            &quantized,
            2,
            tables,
        )
        .unwrap();
        let with_scale_factors = encode_quantized_mono_adts_with_scale_factors_by_bit_cost(
            AdtsConfig::aac_lc(44_100, 1),
            AacLongBlockConfig::new(120, 2),
            &quantized,
            2,
            &[120, 120],
            &[HuffmanEntry {
                symbol: AacScaleFactorDelta::new(0),
                code: HuffmanCode::new(0b0, 1).unwrap(),
            }],
            tables,
        )
        .unwrap();
        let selected = encode_quantized_mono_adts_with_selected_scale_factors_by_bit_cost(
            AdtsConfig::aac_lc(44_100, 1),
            AacLongBlockConfig::new(120, 2),
            &quantized,
            2,
            &[
                HuffmanEntry {
                    symbol: AacScaleFactorDelta::new(0),
                    code: HuffmanCode::new(0b0, 1).unwrap(),
                },
                HuffmanEntry {
                    symbol: AacScaleFactorDelta::new(1),
                    code: HuffmanCode::new(0b10, 2).unwrap(),
                },
            ],
            tables,
        )
        .unwrap();

        assert_eq!(encoded, expected);
        assert_eq!(&with_scale_factors[..2], &[0xff, 0xf1]);
        assert_eq!(&selected[..2], &[0xff, 0xf1]);
        assert!(encode_quantized_mono_adts_by_bit_cost(
            AdtsConfig::aac_lc(44_100, 2),
            AacLongBlockConfig::new(120, 2),
            &quantized,
            2,
            tables,
        )
        .is_err());
    }

    #[test]
    fn encodes_quantized_stereo_long_blocks_as_adts() {
        let left_quantized = [0, 0, 1, -1, 3, 0, -2, 2];
        let right_quantized = [0, 0, -1, 1, 3, 0, 2, -2];
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

        let adts = encode_quantized_stereo_adts(
            AdtsConfig::aac_lc(44_100, 2),
            AacLongBlockConfig::new(120, 3),
            &left_quantized,
            AacLongBlockConfig::new(100, 3),
            &right_quantized,
            2,
            AacSpectralMagnitudeTables {
                pairs1: &pairs1,
                pairs5: &pairs5,
                pairs6: &[],
                escape: &[],
            },
        )
        .unwrap();

        assert_eq!(
            adts,
            [
                0xff, 0xf1, 0x50, 0x80, 0x03, 0x3f, 0xfc, 0x20, 0x78, 0x00, 0xc0, 0x17, 0x18, 0xc7,
                0x9b, 0x94, 0xc8, 0x01, 0x80, 0x2e, 0x31, 0x97, 0x37, 0x27, 0x80,
            ]
        );
        assert!(encode_quantized_stereo_adts(
            AdtsConfig::aac_lc(44_100, 1),
            AacLongBlockConfig::new(120, 3),
            &left_quantized,
            AacLongBlockConfig::new(100, 3),
            &right_quantized,
            2,
            AacSpectralMagnitudeTables {
                pairs1: &pairs1,
                pairs5: &pairs5,
                pairs6: &[],
                escape: &[],
            },
        )
        .is_err());
    }

    #[test]
    fn encodes_quantized_stereo_long_blocks_with_scale_factors_as_adts() {
        let left_quantized = [0, 0, 1, -1, 3, 0, -2, 2];
        let right_quantized = [0, 0, -1, 1, 3, 0, 2, -2];
        let scale_factor_table = [
            HuffmanEntry {
                symbol: AacScaleFactorDelta::new(-1),
                code: HuffmanCode::new(0b0, 1).unwrap(),
            },
            HuffmanEntry {
                symbol: AacScaleFactorDelta::new(1),
                code: HuffmanCode::new(0b10, 2).unwrap(),
            },
            HuffmanEntry {
                symbol: AacScaleFactorDelta::new(2),
                code: HuffmanCode::new(0b110, 3).unwrap(),
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

        let adts = encode_quantized_stereo_adts_with_scale_factors(
            AdtsConfig::aac_lc(44_100, 2),
            AacQuantizedChannel::new(
                AacLongBlockConfig::new(120, 3),
                &left_quantized,
                &[119, 121, 123, 122],
            ),
            AacQuantizedChannel::new(
                AacLongBlockConfig::new(100, 3),
                &right_quantized,
                &[99, 101, 103, 102],
            ),
            2,
            &scale_factor_table,
            AacSpectralMagnitudeTables {
                pairs1: &pairs1,
                pairs5: &pairs5,
                pairs6: &[],
                escape: &[],
            },
        )
        .unwrap();

        assert_eq!(
            adts,
            [
                0xff, 0xf1, 0x50, 0x80, 0x03, 0x5f, 0xfc, 0x20, 0x78, 0x00, 0xc0, 0x17, 0x1d, 0x83,
                0x1e, 0x6e, 0x53, 0x20, 0x06, 0x00, 0xb8, 0xec, 0x19, 0x73, 0x72, 0x78,
            ]
        );
        assert!(encode_quantized_stereo_adts_with_scale_factors(
            AdtsConfig::aac_lc(44_100, 1),
            AacQuantizedChannel::new(
                AacLongBlockConfig::new(120, 3),
                &left_quantized,
                &[119, 121, 123, 122],
            ),
            AacQuantizedChannel::new(
                AacLongBlockConfig::new(100, 3),
                &right_quantized,
                &[99, 101, 103, 102],
            ),
            2,
            &scale_factor_table,
            AacSpectralMagnitudeTables {
                pairs1: &pairs1,
                pairs5: &pairs5,
                pairs6: &[],
                escape: &[],
            },
        )
        .is_err());
    }

    #[test]
    fn encodes_quantized_stereo_long_blocks_with_selected_scale_factors_as_adts() {
        let left_quantized = [0, 0, 1, -1, 3, 0, -2, 2];
        let right_quantized = [0, 0, -1, 1, 3, 0, 2, -2];
        let scale_factor_table = [
            HuffmanEntry {
                symbol: AacScaleFactorDelta::new(0),
                code: HuffmanCode::new(0b0, 1).unwrap(),
            },
            HuffmanEntry {
                symbol: AacScaleFactorDelta::new(1),
                code: HuffmanCode::new(0b10, 2).unwrap(),
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
        let tables = AacSpectralMagnitudeTables {
            pairs1: &pairs1,
            pairs5: &pairs5,
            pairs6: &[],
            escape: &[],
        };

        let selected = encode_quantized_stereo_adts_with_selected_scale_factors(
            AdtsConfig::aac_lc(44_100, 2),
            AacQuantizedSpectrum::new(AacLongBlockConfig::new(120, 3), &left_quantized),
            AacQuantizedSpectrum::new(AacLongBlockConfig::new(100, 3), &right_quantized),
            2,
            &scale_factor_table,
            tables,
        )
        .unwrap();
        let manual = encode_quantized_stereo_adts_with_scale_factors(
            AdtsConfig::aac_lc(44_100, 2),
            AacQuantizedChannel::new(
                AacLongBlockConfig::new(120, 3),
                &left_quantized,
                &[120, 121, 122, 122],
            ),
            AacQuantizedChannel::new(
                AacLongBlockConfig::new(100, 3),
                &right_quantized,
                &[100, 101, 102, 102],
            ),
            2,
            &scale_factor_table,
            tables,
        )
        .unwrap();

        assert_eq!(selected, manual);
        assert!(encode_quantized_stereo_adts_with_selected_scale_factors(
            AdtsConfig::aac_lc(44_100, 1),
            AacQuantizedSpectrum::new(AacLongBlockConfig::new(120, 3), &left_quantized),
            AacQuantizedSpectrum::new(AacLongBlockConfig::new(100, 3), &right_quantized),
            2,
            &scale_factor_table,
            tables,
        )
        .is_err());
    }

    #[test]
    fn encodes_quantized_stereo_long_blocks_with_bit_cost_sections_as_adts() {
        let left_quantized = [1, -1, 0, 0];
        let right_quantized = [-1, 1, 0, 0];
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
        let sections = vec![
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
        let left_payload =
            split_sectioned_spectral_payload_with_sign_bits(&sections, &left_quantized, 2, tables)
                .unwrap();
        let right_payload =
            split_sectioned_spectral_payload_with_sign_bits(&sections, &right_quantized, 2, tables)
                .unwrap();
        let expected_access_unit = pack_channel_pair_raw_data_block_parts(
            AacLongBlockConfig::new(120, 2),
            &left_payload,
            AacLongBlockConfig::new(100, 2),
            &right_payload,
        )
        .unwrap();
        let expected = frame_adts(AdtsConfig::aac_lc(44_100, 2), &expected_access_unit).unwrap();

        let encoded = encode_quantized_stereo_adts_by_bit_cost(
            AdtsConfig::aac_lc(44_100, 2),
            AacLongBlockConfig::new(120, 2),
            &left_quantized,
            AacLongBlockConfig::new(100, 2),
            &right_quantized,
            2,
            tables,
        )
        .unwrap();
        let with_scale_factors = encode_quantized_stereo_adts_with_scale_factors_by_bit_cost(
            AdtsConfig::aac_lc(44_100, 2),
            AacQuantizedChannel::new(
                AacLongBlockConfig::new(120, 2),
                &left_quantized,
                &[120, 120],
            ),
            AacQuantizedChannel::new(
                AacLongBlockConfig::new(100, 2),
                &right_quantized,
                &[100, 100],
            ),
            2,
            &[HuffmanEntry {
                symbol: AacScaleFactorDelta::new(0),
                code: HuffmanCode::new(0b0, 1).unwrap(),
            }],
            tables,
        )
        .unwrap();
        let selected = encode_quantized_stereo_adts_with_selected_scale_factors_by_bit_cost(
            AdtsConfig::aac_lc(44_100, 2),
            AacQuantizedSpectrum::new(AacLongBlockConfig::new(120, 2), &left_quantized),
            AacQuantizedSpectrum::new(AacLongBlockConfig::new(100, 2), &right_quantized),
            2,
            &[
                HuffmanEntry {
                    symbol: AacScaleFactorDelta::new(0),
                    code: HuffmanCode::new(0b0, 1).unwrap(),
                },
                HuffmanEntry {
                    symbol: AacScaleFactorDelta::new(1),
                    code: HuffmanCode::new(0b10, 2).unwrap(),
                },
            ],
            tables,
        )
        .unwrap();

        assert_eq!(encoded, expected);
        assert_eq!(&with_scale_factors[..2], &[0xff, 0xf1]);
        assert_eq!(&selected[..2], &[0xff, 0xf1]);
        assert!(encode_quantized_stereo_adts_by_bit_cost(
            AdtsConfig::aac_lc(44_100, 1),
            AacLongBlockConfig::new(120, 2),
            &left_quantized,
            AacLongBlockConfig::new(100, 2),
            &right_quantized,
            2,
            tables,
        )
        .is_err());
    }

    #[test]
    fn encodes_pcm_mono_long_block_as_adts() {
        let pcm = AudioBuffer::new(44_100, 1, vec![0.0; 2048]).unwrap();

        let adts = encode_pcm_mono_long_block_adts(
            AdtsConfig::aac_lc(44_100, 1),
            AacLongBlockConfig::new(0, 1),
            &pcm,
            AacPcmLongBlockConfig::new(0, 1.0, 1024),
            AacSpectralMagnitudeTables::default(),
        )
        .unwrap();

        assert_eq!(
            adts,
            [0xff, 0xf1, 0x50, 0x40, 0x01, 0xbf, 0xfc, 0x00, 0x00, 0x00, 0x80, 0x23, 0x80,]
        );
        assert!(encode_pcm_mono_long_block_adts(
            AdtsConfig::aac_lc(44_100, 1),
            AacLongBlockConfig::new(0, 1),
            &AudioBuffer::new(44_100, 2, vec![0.0; 4096]).unwrap(),
            AacPcmLongBlockConfig::new(0, 1.0, 1024),
            AacSpectralMagnitudeTables::default(),
        )
        .is_err());
    }

    #[test]
    fn encodes_pcm_mono_long_block_with_scale_factors_as_adts() {
        let pcm = AudioBuffer::new(44_100, 1, vec![0.0; 2048]).unwrap();

        let adts = encode_pcm_mono_long_block_adts_with_scale_factors(
            AdtsConfig::aac_lc(44_100, 1),
            AacScaleFactorChannel::new(AacLongBlockConfig::new(0, 1), &[0]),
            &pcm,
            AacPcmLongBlockConfig::new(0, 1.0, 1024),
            &[],
            AacSpectralMagnitudeTables::default(),
        )
        .unwrap();

        assert_eq!(
            adts,
            [0xff, 0xf1, 0x50, 0x40, 0x01, 0xbf, 0xfc, 0x00, 0x00, 0x00, 0x80, 0x23, 0x80,]
        );
        assert!(encode_pcm_mono_long_block_adts_with_scale_factors(
            AdtsConfig::aac_lc(44_100, 1),
            AacScaleFactorChannel::new(AacLongBlockConfig::new(0, 1), &[0]),
            &AudioBuffer::new(44_100, 2, vec![0.0; 4096]).unwrap(),
            AacPcmLongBlockConfig::new(0, 1.0, 1024),
            &[],
            AacSpectralMagnitudeTables::default(),
        )
        .is_err());
    }

    #[test]
    fn encodes_pcm_mono_long_block_with_selected_scale_factors_as_adts() {
        let pcm = AudioBuffer::new(44_100, 1, vec![0.0; 2048]).unwrap();

        let selected = encode_pcm_mono_long_block_adts_with_selected_scale_factors(
            AdtsConfig::aac_lc(44_100, 1),
            AacLongBlockConfig::new(0, 1),
            &pcm,
            AacPcmLongBlockConfig::new(0, 1.0, 1024),
            &[],
            AacSpectralMagnitudeTables::default(),
        )
        .unwrap();
        let manual = encode_pcm_mono_long_block_adts_with_scale_factors(
            AdtsConfig::aac_lc(44_100, 1),
            AacScaleFactorChannel::new(AacLongBlockConfig::new(0, 1), &[0]),
            &pcm,
            AacPcmLongBlockConfig::new(0, 1.0, 1024),
            &[],
            AacSpectralMagnitudeTables::default(),
        )
        .unwrap();

        assert_eq!(selected, manual);
        assert!(encode_pcm_mono_long_block_adts_with_selected_scale_factors(
            AdtsConfig::aac_lc(44_100, 1),
            AacLongBlockConfig::new(0, 1),
            &AudioBuffer::new(44_100, 2, vec![0.0; 4096]).unwrap(),
            AacPcmLongBlockConfig::new(0, 1.0, 1024),
            &[],
            AacSpectralMagnitudeTables::default(),
        )
        .is_err());
    }

    #[test]
    fn encodes_pcm_mono_long_block_with_bit_cost_sections_as_adts() {
        let pcm = AudioBuffer::new(44_100, 1, vec![0.0; 2048]).unwrap();

        let encoded = encode_pcm_mono_long_block_adts_by_bit_cost(
            AdtsConfig::aac_lc(44_100, 1),
            AacLongBlockConfig::new(0, 1),
            &pcm,
            AacPcmLongBlockConfig::new(0, 1.0, 1024),
            AacSpectralMagnitudeTables::default(),
        )
        .unwrap();
        let with_scale_factors = encode_pcm_mono_long_block_adts_with_scale_factors_by_bit_cost(
            AdtsConfig::aac_lc(44_100, 1),
            AacScaleFactorChannel::new(AacLongBlockConfig::new(0, 1), &[0]),
            &pcm,
            AacPcmLongBlockConfig::new(0, 1.0, 1024),
            &[],
            AacSpectralMagnitudeTables::default(),
        )
        .unwrap();
        let selected = encode_pcm_mono_long_block_adts_with_selected_scale_factors_by_bit_cost(
            AdtsConfig::aac_lc(44_100, 1),
            AacLongBlockConfig::new(0, 1),
            &pcm,
            AacPcmLongBlockConfig::new(0, 1.0, 1024),
            &[],
            AacSpectralMagnitudeTables::default(),
        )
        .unwrap();

        assert_eq!(
            encoded,
            [0xff, 0xf1, 0x50, 0x40, 0x01, 0xbf, 0xfc, 0x00, 0x00, 0x00, 0x80, 0x23, 0x80,]
        );
        assert_eq!(with_scale_factors, encoded);
        assert_eq!(selected, with_scale_factors);
        assert!(encode_pcm_mono_long_block_adts_by_bit_cost(
            AdtsConfig::aac_lc(44_100, 1),
            AacLongBlockConfig::new(0, 1),
            &AudioBuffer::new(44_100, 2, vec![0.0; 4096]).unwrap(),
            AacPcmLongBlockConfig::new(0, 1.0, 1024),
            AacSpectralMagnitudeTables::default(),
        )
        .is_err());
    }

