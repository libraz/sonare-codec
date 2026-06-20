    #[test]
    fn table_provider_prefers_shorter_available_big_value_table() {
        let big_value_table_1 = [HuffmanEntry {
            symbol: Layer3BigValueMagnitude::new(1, 0),
            code: HuffmanCode::new(0b1111, 4).unwrap(),
        }];
        let big_value_table_5 = [
            HuffmanEntry {
                symbol: Layer3BigValueMagnitude::new(1, 0),
                code: HuffmanCode::new(0b0, 1).unwrap(),
            },
            HuffmanEntry {
                symbol: Layer3BigValueMagnitude::new(2, 0),
                code: HuffmanCode::new(0b10, 2).unwrap(),
            },
        ];
        let mut quantized = Vec::new();
        for _ in 0..7 {
            quantized.extend_from_slice(&[1, 0]);
        }
        quantized.extend_from_slice(&[2, 0]);
        let mut granule = Layer3GranuleChannelInfo::default();
        let pairs =
            big_value_pairs(&quantized, plan_spectral_regions(&quantized).unwrap()).unwrap();

        assert_eq!(
            select_big_value_region_tables_by_bit_cost(
                &pairs,
                7,
                1,
                Layer3EntropyTableProvider {
                    big_value_table_1: &big_value_table_1,
                    big_value_table_5: &big_value_table_5,
                    ..Default::default()
                },
            )
            .unwrap()
            .regions
            .map(|selection| selection.table_select),
            [5, 5, 0]
        );

        let packed = pack_quantized_spectrum_with_table_provider(
            &mut granule,
            &quantized,
            Layer3EntropyTableProvider {
                big_value_table_1: &big_value_table_1,
                big_value_table_5: &big_value_table_5,
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(granule.big_values, 8);
        // With the fixed 2 + 2 + remainder split the `[2,0]` pair falls in
        // region2, so every region needs table 5 (table 1 cannot code it).
        assert_eq!(granule.table_select, [5, 5, 5]);
        assert_eq!(granule.part2_3_length, 17);
        assert_eq!(packed.bit_len, 17);
    }

    #[test]
    fn table_provider_prefers_shorter_available_count1_table() {
        let count1_table_0 = [HuffmanEntry {
            symbol: Layer3Count1MagnitudeQuad::new(1, 1, 0, 1),
            code: HuffmanCode::new(0b1111, 4).unwrap(),
        }];
        let count1_table_1 = [HuffmanEntry {
            symbol: Layer3Count1MagnitudeQuad::new(1, 1, 0, 1),
            code: HuffmanCode::new(0b0, 1).unwrap(),
        }];
        let quantized = [1, -1, 0, 1, 0, 0];
        let mut granule = Layer3GranuleChannelInfo::default();

        let packed = pack_quantized_spectrum_with_table_provider(
            &mut granule,
            &quantized,
            Layer3EntropyTableProvider {
                count1_table_0: &count1_table_0,
                count1_table_1: &count1_table_1,
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(granule.big_values, 0);
        assert_eq!(granule.table_select, [0, 0, 0]);
        assert!(granule.count1table_select);
        assert_eq!(granule.part2_3_length, 4);
        assert_eq!(packed.bit_len, 4);
    }

    #[test]
    fn experimental_unit_provider_packs_nonzero_big_values_and_count1() {
        let provider = experimental_unit_magnitude_table_provider();
        let big_value_pairs = [
            Layer3BigValuePair::new(1, -1),
            Layer3BigValuePair::new(0, 0),
        ];

        let big_value_selection =
            select_big_value_region_tables_by_bit_cost(&big_value_pairs, 1, 0, provider).unwrap();
        let big_value_bits = pack_big_value_pairs_with_region_tables_and_provider(
            &big_value_pairs,
            big_value_selection,
            provider,
        )
        .unwrap();

        assert_eq!(big_value_selection.regions[0].table_select, 1);
        assert_eq!(big_value_selection.regions[1].table_select, 0);
        assert_eq!(big_value_selection.regions[2].table_select, 0);
        assert_eq!(big_value_bits.bit_len, 5);

        let count1_quads = [Layer3Count1Quad::new(1, 0, -1, 1)];
        let count1_selection = select_count1_table_by_bit_cost(&count1_quads, provider).unwrap();
        let count1_bits = pack_count1_quads_with_sign_bits(
            &count1_quads,
            provider.count1_table(count1_selection).unwrap(),
        )
        .unwrap();

        assert!(!count1_selection.table_select);
        assert_eq!(count1_bits.bit_len, 7);
    }

    #[test]
    fn standard_provider_packs_table_1_and_count1_codewords() {
        let provider = mpeg1_layer3_standard_table_provider();
        let pairs = [
            Layer3BigValuePair::new(0, 0),
            Layer3BigValuePair::new(0, 1),
            Layer3BigValuePair::new(-1, 0),
            Layer3BigValuePair::new(1, -1),
        ];
        let table_1_selection = Layer3BigValueTableSelection {
            table_select: 1,
            linbits: 0,
            max_magnitude: 1,
        };
        let packed = pack_big_value_pairs_with_linbits(
            &pairs,
            provider.big_value_table(table_1_selection).unwrap(),
            table_1_selection.linbits,
        )
        .unwrap();

        assert_eq!(packed.bit_len, 13);
        assert_eq!(packed.bytes, [0b1001_0011, 0b0000_1000]);

        let x_only = pack_big_value_pairs_with_linbits(
            &[Layer3BigValuePair::new(-1, 0)],
            provider.big_value_table(table_1_selection).unwrap(),
            table_1_selection.linbits,
        )
        .unwrap();
        assert_eq!(x_only.bit_len, 3);
        assert_eq!(x_only.bytes, [0b0110_0000]);

        let sparse_count1 = [Layer3Count1Quad::new(1, 0, 0, 0)];
        let sparse_selection = select_count1_table_by_bit_cost(&sparse_count1, provider).unwrap();
        let sparse_packed = pack_count1_quads_with_sign_bits(
            &sparse_count1,
            provider.count1_table(sparse_selection).unwrap(),
        )
        .unwrap();
        assert!(!sparse_selection.table_select);
        assert_eq!(sparse_packed.bit_len, 5);
        assert_eq!(sparse_packed.bytes, [0b0111_0000]);

        let dense_count1 = [Layer3Count1Quad::new(1, 1, 1, 1)];
        let dense_selection = select_count1_table_by_bit_cost(&dense_count1, provider).unwrap();
        let dense_packed = pack_count1_quads_with_sign_bits(
            &dense_count1,
            provider.count1_table(dense_selection).unwrap(),
        )
        .unwrap();
        assert!(dense_selection.table_select);
        assert_eq!(dense_packed.bit_len, 5);
        assert_eq!(dense_packed.bytes, [0b1000_0000]);
    }

    #[test]
    fn standard_provider_packs_table_2_big_value_codewords() {
        let provider = mpeg1_layer3_standard_table_provider();
        let pairs = [
            Layer3BigValuePair::new(2, 0),
            Layer3BigValuePair::new(0, -2),
            Layer3BigValuePair::new(-2, 2),
        ];
        let table_2_selection = Layer3BigValueTableSelection {
            table_select: 2,
            linbits: 0,
            max_magnitude: 2,
        };

        let packed = pack_big_value_pairs_with_linbits(
            &pairs,
            provider.big_value_table(table_2_selection).unwrap(),
            table_2_selection.linbits,
        )
        .unwrap();

        assert_eq!(packed.bit_len, 5 + 1 + 6 + 1 + 6 + 2);
        assert_eq!(packed.bytes, [0b0001_1000, 0b0001_1000, 0b0001_0000]);
    }

    #[test]
    fn standard_provider_selects_tables_3_and_6_by_bit_cost() {
        let provider = mpeg1_layer3_standard_table_provider();

        let table_3_pairs = [Layer3BigValuePair::new(1, -1)];
        let table_3_selection =
            select_big_value_table_by_bit_cost(&table_3_pairs, provider).unwrap();
        let table_3_packed = pack_big_value_pairs_with_linbits(
            &table_3_pairs,
            provider.big_value_table(table_3_selection).unwrap(),
            table_3_selection.linbits,
        )
        .unwrap();
        assert_eq!(table_3_selection.table_select, 3);
        assert_eq!(table_3_selection.linbits, 0);
        assert_eq!(table_3_packed.bit_len, 4);
        assert_eq!(table_3_packed.bytes, [0b0101_0000]);

        let table_6_pairs = [Layer3BigValuePair::new(3, -1)];
        let table_6_selection =
            select_big_value_table_by_bit_cost(&table_6_pairs, provider).unwrap();
        let table_6_packed = pack_big_value_pairs_with_linbits(
            &table_6_pairs,
            provider.big_value_table(table_6_selection).unwrap(),
            table_6_selection.linbits,
        )
        .unwrap();
        assert_eq!(table_6_selection.table_select, 6);
        assert_eq!(table_6_selection.linbits, 0);
        assert_eq!(table_6_packed.bit_len, 7);
        assert_eq!(table_6_packed.bytes, [0b0001_1010]);
    }

    #[test]
    fn standard_provider_packs_tables_8_and_9_codewords() {
        let provider = mpeg1_layer3_standard_table_provider();

        let table_8_selection = Layer3BigValueTableSelection {
            table_select: 8,
            linbits: 0,
            max_magnitude: 5,
        };
        let table_8_packed = pack_big_value_pairs_with_linbits(
            &[Layer3BigValuePair::new(1, -1)],
            provider.big_value_table(table_8_selection).unwrap(),
            table_8_selection.linbits,
        )
        .unwrap();
        assert_eq!(table_8_packed.bit_len, 4);
        assert_eq!(table_8_packed.bytes, [0b0101_0000]);

        let table_9_pairs = [Layer3BigValuePair::new(0, -3)];
        let table_9_selection =
            select_big_value_table_by_bit_cost(&table_9_pairs, provider).unwrap();
        let table_9_packed = pack_big_value_pairs_with_linbits(
            &table_9_pairs,
            provider.big_value_table(table_9_selection).unwrap(),
            table_9_selection.linbits,
        )
        .unwrap();
        assert_eq!(table_9_selection.table_select, 9);
        assert_eq!(table_9_selection.linbits, 0);
        assert_eq!(table_9_packed.bit_len, 7);
        assert_eq!(table_9_packed.bytes, [0b0011_1010]);
    }

    #[test]
    fn standard_provider_selects_tables_11_and_12_by_bit_cost() {
        let provider = mpeg1_layer3_standard_table_provider();

        let table_11_pairs = [Layer3BigValuePair::new(0, -6)];
        let table_11_selection =
            select_big_value_table_by_bit_cost(&table_11_pairs, provider).unwrap();
        let table_11_packed = pack_big_value_pairs_with_linbits(
            &table_11_pairs,
            provider.big_value_table(table_11_selection).unwrap(),
            table_11_selection.linbits,
        )
        .unwrap();
        assert_eq!(table_11_selection.table_select, 11);
        assert_eq!(table_11_selection.linbits, 0);
        assert_eq!(table_11_packed.bit_len, 9);
        assert_eq!(table_11_packed.bytes, [0x15, 0x80]);

        let table_12_pairs = [Layer3BigValuePair::new(1, -5)];
        let table_12_selection =
            select_big_value_table_by_bit_cost(&table_12_pairs, provider).unwrap();
        let table_12_packed = pack_big_value_pairs_with_linbits(
            &table_12_pairs,
            provider.big_value_table(table_12_selection).unwrap(),
            table_12_selection.linbits,
        )
        .unwrap();
        assert_eq!(table_12_selection.table_select, 12);
        assert_eq!(table_12_selection.linbits, 0);
        assert_eq!(table_12_packed.bit_len, 9);
        assert_eq!(table_12_packed.bytes, [0x20, 0x80]);
    }

    #[test]
    fn standard_provider_selects_tables_15_and_24_by_bit_cost() {
        let provider = mpeg1_layer3_standard_table_provider();

        let table_15_pairs = [Layer3BigValuePair::new(0, -4)];
        let table_15_selection =
            select_big_value_table_by_bit_cost(&table_15_pairs, provider).unwrap();
        let table_15_packed = pack_big_value_pairs_with_linbits(
            &table_15_pairs,
            provider.big_value_table(table_15_selection).unwrap(),
            table_15_selection.linbits,
        )
        .unwrap();
        assert_eq!(table_15_selection.table_select, 15);
        assert_eq!(table_15_selection.linbits, 0);
        assert_eq!(table_15_packed.bit_len, 8);
        assert_eq!(table_15_packed.bytes, [0x5f]);

        let table_24_pairs = [Layer3BigValuePair::new(1, -14)];
        let table_24_selection =
            select_big_value_table_by_bit_cost(&table_24_pairs, provider).unwrap();
        let table_24_packed = pack_big_value_pairs_with_linbits(
            &table_24_pairs,
            provider.big_value_table(table_24_selection).unwrap(),
            table_24_selection.linbits,
        )
        .unwrap();
        assert_eq!(table_24_selection.table_select, 24);
        assert_eq!(table_24_selection.linbits, 4);
        assert_eq!(table_24_packed.bit_len, 12);
        assert_eq!(table_24_packed.bytes, [0x45, 0xd0]);
    }

    #[test]
    fn standard_provider_packs_count1_only_quantized_spectrum() {
        let provider = mpeg1_layer3_standard_table_provider();
        let mut granule = Layer3GranuleChannelInfo::default();

        let packed =
            pack_quantized_spectrum_with_table_provider(&mut granule, &[1, 1, 1, 1], provider)
                .unwrap();

        assert_eq!(granule.big_values, 0);
        assert!(granule.count1table_select);
        assert_eq!(granule.part2_3_length, 5);
        assert_eq!(packed.bit_len, 5);
        assert_eq!(packed.bytes, [0b1000_0000]);
    }

    #[test]
    fn standard_big_value_provider_alias_includes_count1_tables() {
        let provider = mpeg1_layer3_standard_big_value_table_provider();
        let selection =
            select_count1_table_by_bit_cost(&[Layer3Count1Quad::new(1, 1, 1, 1)], provider)
                .unwrap();

        assert!(selection.table_select);
        assert!(provider.count1_table(selection).is_ok());
    }

    #[test]
    fn standard_provider_advertises_resolvable_table_selects() {
        let provider = mpeg1_layer3_standard_table_provider();

        assert_eq!(
            MPEG1_LAYER3_STANDARD_BIG_VALUE_TABLE_SELECTS,
            &[
                1, 2, 3, 5, 6, 7, 8, 9, 10, 11, 12, 13, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
                26, 27, 28, 29, 30, 31
            ]
        );
        for &table_select in MPEG1_LAYER3_STANDARD_BIG_VALUE_TABLE_SELECTS {
            let linbits = match table_select {
                16 => 1,
                17 => 2,
                18 => 3,
                19 => 4,
                20 => 6,
                21 => 8,
                22 => 10,
                23 => 13,
                24 => 4,
                25 => 5,
                26 => 6,
                27 => 7,
                28 => 8,
                29 => 9,
                30 => 11,
                31 => 13,
                _ => 0,
            };
            let table = provider
                .big_value_table(Layer3BigValueTableSelection {
                    table_select,
                    linbits,
                    max_magnitude: if table_select >= 16 { 16 } else { 1 },
                })
                .unwrap();
            assert!(!table.is_empty());
        }
        assert_eq!(MPEG1_LAYER3_MISSING_STANDARD_BIG_VALUE_TABLE_SELECTS, &[]);
        for &table_select in MPEG1_LAYER3_MISSING_STANDARD_BIG_VALUE_TABLE_SELECTS {
            let err = provider
                .big_value_table(Layer3BigValueTableSelection {
                    table_select,
                    linbits: 0,
                    max_magnitude: 1,
                })
                .unwrap_err();
            assert!(matches!(
                err,
                Error::UnsupportedFeature("MP3 big-values Huffman table")
            ));
        }

        assert_eq!(MPEG1_LAYER3_STANDARD_COUNT1_TABLE_SELECTS, &[false, true]);
        for &table_select in MPEG1_LAYER3_STANDARD_COUNT1_TABLE_SELECTS {
            let table = provider
                .count1_table(Layer3Count1TableSelection {
                    table_select,
                    max_nonzero_values: 1,
                })
                .unwrap();
            assert!(!table.is_empty());
        }
    }

    #[test]
    fn packs_quantized_spectrum_with_scale_factors_and_table_provider() {
        let big_value_table_5 = [
            HuffmanEntry {
                symbol: Layer3BigValueMagnitude::new(3, 2),
                code: HuffmanCode::new(0b10, 2).unwrap(),
            },
            HuffmanEntry {
                symbol: Layer3BigValueMagnitude::new(0, 0),
                code: HuffmanCode::new(0b0, 1).unwrap(),
            },
        ];
        let count1_table_1 = [HuffmanEntry {
            symbol: Layer3Count1MagnitudeQuad::new(1, 1, 0, 1),
            code: HuffmanCode::new(0b11, 2).unwrap(),
        }];
        let scale_factors = PackedBits {
            bytes: vec![0b1000_0000],
            bit_len: 1,
        };
        let quantized = [3, -2, 0, 0, 1, -1, 0, 1, 0, 0];
        let mut granule = Layer3GranuleChannelInfo::default();

        let packed = pack_quantized_spectrum_with_scale_factors_and_table_provider(
            &mut granule,
            scale_factors,
            &quantized,
            Layer3EntropyTableProvider {
                big_value_table_5: &big_value_table_5,
                count1_table_1: &count1_table_1,
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(
            packed,
            PackedBits {
                bytes: vec![0b1100_1011, 0b0100_0000],
                bit_len: 11,
            }
        );
        assert_eq!(granule.part2_3_length, 11);
        assert_eq!(granule.table_select, [5, 0, 0]);
        assert!(granule.count1table_select);
    }

    #[test]
    fn packs_mp3_big_value_pairs_from_table() {
        let table = [
            HuffmanEntry {
                symbol: Layer3BigValuePair::new(0, 0),
                code: HuffmanCode::new(0b0, 1).unwrap(),
            },
            HuffmanEntry {
                symbol: Layer3BigValuePair::new(2, -1),
                code: HuffmanCode::new(0b10, 2).unwrap(),
            },
            HuffmanEntry {
                symbol: Layer3BigValuePair::new(-3, 1),
                code: HuffmanCode::new(0b111, 3).unwrap(),
            },
        ];
        let pairs = [
            Layer3BigValuePair::new(2, -1),
            Layer3BigValuePair::new(0, 0),
            Layer3BigValuePair::new(-3, 1),
        ];

        assert_eq!(
            pack_big_value_pairs_with_table(&pairs, &table).unwrap(),
            PackedBits {
                bytes: vec![0b1001_1100],
                bit_len: 6,
            }
        );
        assert!(pack_big_value_pairs_with_table(&[Layer3BigValuePair::new(4, 4)], &table).is_err());
    }

    #[test]
    fn packs_mp3_big_value_pairs_with_sign_bits() {
        let table = [
            HuffmanEntry {
                symbol: Layer3BigValueMagnitude::new(0, 0),
                code: HuffmanCode::new(0b0, 1).unwrap(),
            },
            HuffmanEntry {
                symbol: Layer3BigValueMagnitude::new(2, 1),
                code: HuffmanCode::new(0b10, 2).unwrap(),
            },
            HuffmanEntry {
                symbol: Layer3BigValueMagnitude::new(3, 1),
                code: HuffmanCode::new(0b111, 3).unwrap(),
            },
        ];
        let pairs = [
            Layer3BigValuePair::new(2, -1),
            Layer3BigValuePair::new(0, 0),
            Layer3BigValuePair::new(-3, 1),
        ];

        assert_eq!(
            pack_big_value_pairs_with_sign_bits(&pairs, &table).unwrap(),
            PackedBits {
                bytes: vec![0b1001_0111, 0b1000_0000],
                bit_len: 10,
            }
        );
        assert!(
            pack_big_value_pairs_with_sign_bits(&[Layer3BigValuePair::new(4, 4)], &table).is_err()
        );
    }

    #[test]
    fn packs_mp3_big_value_pairs_with_linbits() {
        let table = [
            HuffmanEntry {
                symbol: Layer3BigValueMagnitude::new(15, 15),
                code: HuffmanCode::new(0b10, 2).unwrap(),
            },
            HuffmanEntry {
                symbol: Layer3BigValueMagnitude::new(1, 15),
                code: HuffmanCode::new(0b111, 3).unwrap(),
            },
        ];
        let pairs = [
            Layer3BigValuePair::new(18, -15),
            Layer3BigValuePair::new(-1, 16),
        ];

        // Escape linbits and signs interleave per value: code, linbits_x,
        // sign_x, linbits_y, sign_y. Pair (18,-15): `10` `0011` `0` `0000` `1`;
        // pair (-1,16): `111` `1` `0001` `0`.
        assert_eq!(
            pack_big_value_pairs_with_linbits(&pairs, &table, 4).unwrap(),
            PackedBits {
                bytes: vec![0b1000_1100, 0b0001_1111, 0b0001_0000],
                bit_len: 21,
            }
        );
        assert!(
            pack_big_value_pairs_with_linbits(&[Layer3BigValuePair::new(32, 0)], &table, 4)
                .is_err()
        );
        assert!(pack_big_value_pairs_with_linbits(&pairs, &table, 17).is_err());
    }

    #[test]
    fn packs_mp3_count1_quads_from_table() {
        let table = [
            HuffmanEntry {
                symbol: Layer3Count1Quad::new(0, 0, 0, 0),
                code: HuffmanCode::new(0b0, 1).unwrap(),
            },
            HuffmanEntry {
                symbol: Layer3Count1Quad::new(1, -1, 0, 1),
                code: HuffmanCode::new(0b10, 2).unwrap(),
            },
            HuffmanEntry {
                symbol: Layer3Count1Quad::new(-1, 0, 1, 0),
                code: HuffmanCode::new(0b111, 3).unwrap(),
            },
        ];
        let quads = [
            Layer3Count1Quad::new(1, -1, 0, 1),
            Layer3Count1Quad::new(0, 0, 0, 0),
            Layer3Count1Quad::new(-1, 0, 1, 0),
        ];

        assert_eq!(
            pack_count1_quads_with_table(&quads, &table).unwrap(),
            PackedBits {
                bytes: vec![0b1001_1100],
                bit_len: 6,
            }
        );
        assert!(
            pack_count1_quads_with_table(&[Layer3Count1Quad::new(1, 1, 1, 1)], &table).is_err()
        );
    }

    #[test]
    fn packs_mp3_count1_quads_with_sign_bits() {
        let table = [
            HuffmanEntry {
                symbol: Layer3Count1MagnitudeQuad::new(0, 0, 0, 0),
                code: HuffmanCode::new(0b0, 1).unwrap(),
            },
            HuffmanEntry {
                symbol: Layer3Count1MagnitudeQuad::new(1, 1, 0, 1),
                code: HuffmanCode::new(0b10, 2).unwrap(),
            },
            HuffmanEntry {
                symbol: Layer3Count1MagnitudeQuad::new(1, 0, 1, 0),
                code: HuffmanCode::new(0b111, 3).unwrap(),
            },
        ];
        let quads = [
            Layer3Count1Quad::new(1, -1, 0, 1),
            Layer3Count1Quad::new(0, 0, 0, 0),
            Layer3Count1Quad::new(-1, 0, 1, 0),
        ];

        assert_eq!(
            pack_count1_quads_with_sign_bits(&quads, &table).unwrap(),
            PackedBits {
                bytes: vec![0b1001_0011, 0b1100_0000],
                bit_len: 11,
            }
        );
        assert!(
            pack_count1_quads_with_sign_bits(&[Layer3Count1Quad::new(2, 0, 0, 0)], &table).is_err()
        );
    }

    #[test]
    fn encodes_silent_mono_pcm_as_layer3_frames() {
        let pcm = AudioBuffer::new(44_100, 1, vec![0.0; 1152]).unwrap();

        let mp3 = encode(&pcm).unwrap();
        let header = FrameHeader::parse(&mp3[..4]).unwrap();

        assert_eq!(detect(&mp3), Some(Format::Mp3));
        assert_eq!(header.version, MpegVersion::Mpeg1);
        assert_eq!(header.layer, Layer::Layer3);
        assert_eq!(header.bitrate_kbps, 128);
        assert_eq!(header.sample_rate, 44_100);
        assert_eq!(header.channel_mode, ChannelMode::SingleChannel);
        assert_eq!(mp3.len(), header.frame_len());
    }

    #[test]
    fn encodes_silent_stereo_pcm_as_multiple_layer3_frames() {
        let pcm = AudioBuffer::new(48_000, 2, vec![0.0; 1153 * 2]).unwrap();

        let mp3 = encode(&pcm).unwrap();
        let header = FrameHeader::parse(&mp3[..4]).unwrap();

        assert_eq!(header.sample_rate, 48_000);
        assert_eq!(header.channel_mode, ChannelMode::Stereo);
        assert_eq!(mp3.len(), header.frame_len() * 2);
        assert_eq!(
            FrameHeader::parse(&mp3[header.frame_len()..header.frame_len() + 4]).unwrap(),
            header
        );
    }

    #[test]
    fn encodes_silent_pcm_with_experimental_frame_scaffold() {
        let pcm = AudioBuffer::new(44_100, 2, vec![0.0; 1153 * 2]).unwrap();
        let expected = encode(&pcm).unwrap();

        let table_encoded = encode_mpeg1_layer3_pcm_frames_with_selected_scale_factors(
            &pcm,
            1.0,
            Layer3EntropyTables {
                big_values: &[],
                count1: &[],
            },
        )
        .unwrap();
        let provider_encoded =
            encode_mpeg1_layer3_pcm_frames_with_selected_scale_factors_and_table_provider(
                &pcm,
                1.0,
                Layer3EntropyTableProvider::default(),
            )
            .unwrap();

        assert_eq!(table_encoded, expected);
        assert_eq!(provider_encoded, expected);
    }

    #[test]
    fn encodes_silent_pcm_with_explicit_experimental_header() {
        let header = FrameHeader {
            version: MpegVersion::Mpeg1,
            layer: Layer::Layer3,
            protection_absent: true,
            bitrate_kbps: 128,
            sample_rate: 44_100,
            padding: false,
            channel_mode: ChannelMode::SingleChannel,
        };
        let pcm = AudioBuffer::new(44_100, 1, vec![0.0; 1153]).unwrap();
        let expected = encode(&pcm).unwrap();

        let encoded = encode_mpeg1_layer3_pcm_frames_with_header_and_selected_scale_factors(
            header,
            &pcm,
            1.0,
            Layer3EntropyTables {
                big_values: &[],
                count1: &[],
            },
        )
        .unwrap();
        let provider_encoded =
            encode_mpeg1_layer3_pcm_frames_with_header_and_selected_scale_factors_and_table_provider(
                header,
                &pcm,
                1.0,
                Layer3EntropyTableProvider::default(),
            )
            .unwrap();

        assert_eq!(encoded, expected);
        assert_eq!(provider_encoded, expected);

        let stereo_header = FrameHeader {
            channel_mode: ChannelMode::Stereo,
            ..header
        };
        assert!(
            encode_mpeg1_layer3_pcm_frames_with_header_and_selected_scale_factors(
                stereo_header,
                &pcm,
                1.0,
                Layer3EntropyTables {
                    big_values: &[],
                    count1: &[],
                },
            )
            .is_err()
        );
    }

    #[test]
    fn selects_pcm_frame_step_for_standard_nonzero_payload() {
        let pcm = AudioBuffer::new(
            44_100,
            1,
            (0..1152)
                .map(|sample| ((sample as f32) * 0.01).sin() * 0.25)
                .collect(),
        )
        .unwrap();
        let header = FrameHeader {
            version: MpegVersion::Mpeg1,
            layer: Layer::Layer3,
            protection_absent: true,
            bitrate_kbps: 128,
            sample_rate: 44_100,
            padding: false,
            channel_mode: ChannelMode::SingleChannel,
        };
        let provider = mpeg1_layer3_standard_table_provider();

        let step = select_mpeg1_layer3_pcm_frame_step_with_table_provider(
            header,
            &pcm,
            0,
            MPEG1_LAYER3_PCM_STEP_CANDIDATES,
            provider,
        )
        .unwrap();
        let reversed_candidates = MPEG1_LAYER3_PCM_STEP_CANDIDATES
            .iter()
            .copied()
            .rev()
            .collect::<Vec<_>>();
        let details: Layer3PcmFrameStepSelection =
            select_mpeg1_layer3_pcm_frame_step_details_with_table_provider(
                header,
                &pcm,
                0,
                &reversed_candidates,
                provider,
            )
            .unwrap();
        let auto = encode_mpeg1_layer3_pcm_frames_with_auto_step_and_table_provider(
            &pcm,
            MPEG1_LAYER3_PCM_STEP_CANDIDATES,
            provider,
        )
        .unwrap();
        let selected =
            encode_mpeg1_layer3_pcm_frames_with_selected_scale_factors_and_table_provider(
                &pcm, step, provider,
            )
            .unwrap();
        let zero_payload =
            encode_mpeg1_layer3_pcm_frames_with_selected_scale_factors_and_table_provider(
                &pcm,
                f32::MAX,
                Layer3EntropyTableProvider::default(),
            )
            .unwrap();

        assert!(step < f32::MAX);
        assert_eq!(details.step, step);
        assert!(details.payload_bit_len > 0);
        assert!(details.payload_bit_len <= details.frame_capacity_bits);
        assert_eq!(auto, selected);
        assert_ne!(auto, zero_payload);
    }

    #[test]
    fn selects_pcm_frame_step_with_max_payload_bits() {
        let pcm = AudioBuffer::new(
            44_100,
            1,
            (0..1152)
                .map(|sample| ((sample as f32) * 0.01).sin() * 0.25)
                .collect(),
        )
        .unwrap();
        let header = FrameHeader {
            version: MpegVersion::Mpeg1,
            layer: Layer::Layer3,
            protection_absent: true,
            bitrate_kbps: 128,
            sample_rate: 44_100,
            padding: false,
            channel_mode: ChannelMode::SingleChannel,
        };
        let provider = mpeg1_layer3_standard_table_provider();
        let unconstrained = select_mpeg1_layer3_pcm_frame_step_details_with_table_provider(
            header,
            &pcm,
            0,
            MPEG1_LAYER3_PCM_STEP_CANDIDATES,
            provider,
        )
        .unwrap();
        let positive_payload_selections = MPEG1_LAYER3_PCM_STEP_CANDIDATES
            .iter()
            .copied()
            .filter_map(|candidate| {
                select_mpeg1_layer3_pcm_frame_step_details_with_table_provider(
                    header,
                    &pcm,
                    0,
                    &[candidate],
                    provider,
                )
                .ok()
            })
            .filter(|selection| selection.payload_bit_len > 0)
            .collect::<Vec<_>>();
        let budget = positive_payload_selections
            .iter()
            .filter(|selection| selection.step > unconstrained.step)
            .map(|selection| selection.payload_bit_len)
            .min()
            .expect("at least one coarser positive-payload MP3 step candidate");
        let min_positive_budget = positive_payload_selections
            .iter()
            .map(|selection| selection.payload_bit_len)
            .min()
            .unwrap();
        let positive_payload_candidates = positive_payload_selections
            .iter()
            .map(|selection| selection.step)
            .collect::<Vec<_>>();

        let step = select_mpeg1_layer3_pcm_frame_step_with_max_payload_bits_and_table_provider(
            header,
            &pcm,
            0,
            MPEG1_LAYER3_PCM_STEP_CANDIDATES,
            budget,
            provider,
        )
        .unwrap();
        let details =
            select_mpeg1_layer3_pcm_frame_step_details_with_max_payload_bits_and_table_provider(
                header,
                &pcm,
                0,
                MPEG1_LAYER3_PCM_STEP_CANDIDATES,
                budget,
                provider,
            )
            .unwrap();

        assert_eq!(step, details.step);
        assert!(details.step > unconstrained.step);
        assert_eq!(details.frame_capacity_bits, budget);
        assert!(details.payload_bit_len <= budget);
        assert!(
            select_mpeg1_layer3_pcm_frame_step_details_with_max_payload_bits_and_table_provider(
                header,
                &pcm,
                0,
                &positive_payload_candidates,
                min_positive_budget - 1,
                provider,
            )
            .is_err()
        );
    }

