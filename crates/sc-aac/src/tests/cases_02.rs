    #[test]
    fn packs_aac_spectral_codewords() {
        let codes = [
            HuffmanCode::new(0b10, 2).unwrap(),
            HuffmanCode::new(0b011, 3).unwrap(),
            HuffmanCode::new(0b1, 1).unwrap(),
        ];

        assert_eq!(pack_spectral_codewords(&codes).unwrap(), &[0b1001_1100]);
        assert_eq!(
            pack_spectral_codewords_with_len(&codes).unwrap(),
            PackedBits {
                bytes: vec![0b1001_1100],
                bit_len: 6,
            }
        );
    }

    #[test]
    fn packs_aac_spectral_pairs_from_table() {
        let table = [
            HuffmanEntry {
                symbol: AacSpectralPair::new(0, 0),
                code: HuffmanCode::new(0b0, 1).unwrap(),
            },
            HuffmanEntry {
                symbol: AacSpectralPair::new(1, -1),
                code: HuffmanCode::new(0b10, 2).unwrap(),
            },
            HuffmanEntry {
                symbol: AacSpectralPair::new(-2, 1),
                code: HuffmanCode::new(0b110, 3).unwrap(),
            },
        ];
        let pairs = [
            AacSpectralPair::new(1, -1),
            AacSpectralPair::new(0, 0),
            AacSpectralPair::new(-2, 1),
        ];

        assert_eq!(
            pack_spectral_pairs_with_table(&pairs, &table).unwrap(),
            PackedBits {
                bytes: vec![0b1001_1000],
                bit_len: 6,
            }
        );
        assert!(pack_spectral_pairs_with_table(&[AacSpectralPair::new(2, 2)], &table).is_err());
    }

    #[test]
    fn packs_aac_spectral_pairs_with_sign_bits() {
        let table = [
            HuffmanEntry {
                symbol: AacSpectralMagnitudePair::new(0, 0),
                code: HuffmanCode::new(0b0, 1).unwrap(),
            },
            HuffmanEntry {
                symbol: AacSpectralMagnitudePair::new(1, 1),
                code: HuffmanCode::new(0b10, 2).unwrap(),
            },
            HuffmanEntry {
                symbol: AacSpectralMagnitudePair::new(2, 0),
                code: HuffmanCode::new(0b110, 3).unwrap(),
            },
        ];
        let pairs = [
            AacSpectralPair::new(1, -1),
            AacSpectralPair::new(-2, 0),
            AacSpectralPair::new(0, 0),
        ];

        assert_eq!(
            super::pack_spectral_pairs_with_sign_bits(&pairs, &table).unwrap(),
            PackedBits {
                bytes: vec![0b1001_1101, 0b0000_0000],
                bit_len: 9,
            }
        );
        assert!(super::pack_spectral_pairs_with_sign_bits(
            &[AacSpectralPair::new(i16::MIN, 0)],
            &table,
        )
        .is_err());
    }

    #[test]
    fn exposes_aac_unsigned_pairs7_unit_magnitude_table() {
        let table = super::aac_unsigned_pairs7_unit_magnitude_table();
        assert_eq!(table.len(), 4);
        assert_eq!(table[0].symbol, AacSpectralMagnitudePair::new(0, 0));
        assert_eq!(table[0].code, HuffmanCode::new(0b0, 1).unwrap());
        assert_eq!(table[1].symbol, AacSpectralMagnitudePair::new(0, 1));
        assert_eq!(table[1].code, HuffmanCode::new(0b101, 3).unwrap());
        assert_eq!(table[2].symbol, AacSpectralMagnitudePair::new(1, 0));
        assert_eq!(table[2].code, HuffmanCode::new(0b100, 3).unwrap());
        assert_eq!(table[3].symbol, AacSpectralMagnitudePair::new(1, 1));
        assert_eq!(table[3].code, HuffmanCode::new(0b1100, 4).unwrap());

        let pairs = [
            AacSpectralPair::new(0, 0),
            AacSpectralPair::new(1, -1),
            AacSpectralPair::new(-1, 0),
            AacSpectralPair::new(0, 1),
        ];
        assert_eq!(
            super::pack_spectral_pairs_with_sign_bits(&pairs, table).unwrap(),
            PackedBits {
                bytes: vec![0b0110_0011, 0b0011_0100],
                bit_len: 15,
            }
        );
    }

    #[test]
    fn exposes_full_aac_unsigned_pairs7_table() {
        let table = aac_unsigned_pairs7_table();

        assert_eq!(table.len(), 64);
        assert_eq!(table[0].symbol, AacSpectralMagnitudePair::new(0, 0));
        assert_eq!(table[0].code, HuffmanCode::new(0b0, 1).unwrap());
        assert_eq!(table[9].symbol, AacSpectralMagnitudePair::new(1, 1));
        assert_eq!(table[9].code, HuffmanCode::new(0x00c, 4).unwrap());
        assert_eq!(table[18].symbol, AacSpectralMagnitudePair::new(2, 2));
        assert_eq!(table[18].code, HuffmanCode::new(0x072, 7).unwrap());
        assert_eq!(table[63].symbol, AacSpectralMagnitudePair::new(7, 7));
        assert_eq!(table[63].code, HuffmanCode::new(0xfff, 12).unwrap());

        let packed = pack_sectioned_spectral_payload_with_sign_bits_by_bit_cost(
            &[2, -2],
            2,
            AacSpectralMagnitudeTables::default(),
        )
        .unwrap();
        assert_eq!(
            packed,
            PackedBits {
                bytes: vec![0b1000_0000, 0b1011_0010],
                bit_len: 15,
            }
        );
    }

    #[test]
    fn exposes_standard_aac_signed_pairs5_and_6_tables() {
        let pairs5 = aac_signed_pairs5_table();
        let pairs6 = aac_signed_pairs6_table();

        assert_eq!(pairs5.len(), 81);
        assert_eq!(pairs5[0].symbol, AacSpectralPair::new(-4, -4));
        assert_eq!(pairs5[0].code, HuffmanCode::new(0x1fff, 13).unwrap());
        assert_eq!(pairs5[40].symbol, AacSpectralPair::new(0, 0));
        assert_eq!(pairs5[40].code, HuffmanCode::new(0, 1).unwrap());
        assert_eq!(pairs5[80].symbol, AacSpectralPair::new(4, 4));
        assert_eq!(pairs5[80].code, HuffmanCode::new(0x1ffe, 13).unwrap());

        assert_eq!(pairs6.len(), 81);
        assert_eq!(pairs6[0].symbol, AacSpectralPair::new(-4, -4));
        assert_eq!(pairs6[0].code, HuffmanCode::new(0x7fe, 11).unwrap());
        assert_eq!(pairs6[40].symbol, AacSpectralPair::new(0, 0));
        assert_eq!(pairs6[40].code, HuffmanCode::new(0, 4).unwrap());
        assert_eq!(pairs6[80].symbol, AacSpectralPair::new(4, 4));
        assert_eq!(pairs6[80].code, HuffmanCode::new(0x7fc, 11).unwrap());

        let tables = aac_lc_standard_signed_pair_tables();
        assert_eq!(
            pack_spectral_pairs_with_table(&[AacSpectralPair::new(1, -1)], tables.signed_pairs6)
                .unwrap(),
            PackedBits {
                bytes: vec![0b0111_0000],
                bit_len: 4,
            }
        );
        assert_eq!(
            pack_spectral_pairs_with_table(&[AacSpectralPair::new(3, -2)], tables.signed_pairs5)
                .unwrap(),
            PackedBits {
                bytes: vec![0b1111_1010, 0b0100_0000],
                bit_len: 10,
            }
        );
    }

    #[test]
    fn standard_aac_pair_workbench_uses_direct_signed_pairs5_and_6() {
        assert_eq!(
            plan_aac_lc_standard_spectral_sections_by_bit_cost(&[0, 1], 2).unwrap(),
            vec![AacSpectralSection {
                start: 0,
                end: 2,
                codebook_id: 5,
            }]
        );
        assert_eq!(
            split_aac_lc_standard_spectral_payload_with_sign_bits_by_bit_cost(&[0, 1], 2)
                .unwrap()
                .spectral_bits
                .bit_len,
            4
        );

        assert_eq!(
            plan_aac_lc_standard_spectral_sections_by_bit_cost(&[1, -1], 2).unwrap(),
            vec![AacSpectralSection {
                start: 0,
                end: 2,
                codebook_id: 6,
            }]
        );
        assert_eq!(
            split_aac_lc_standard_spectral_payload_with_sign_bits_by_bit_cost(&[1, -1], 2)
                .unwrap()
                .spectral_bits
                .bit_len,
            4
        );
    }

    #[test]
    fn exposes_standard_aac_signed_quads1_and_2_tables() {
        let quads1 = aac_signed_quads1_table();
        let quads2 = aac_signed_quads2_table();

        assert_eq!(quads1.len(), 81);
        assert_eq!(quads1[0].symbol, AacSpectralQuad::new(-1, -1, -1, -1));
        assert_eq!(quads1[0].code, HuffmanCode::new(0x7f8, 11).unwrap());
        assert_eq!(quads1[40].symbol, AacSpectralQuad::new(0, 0, 0, 0));
        assert_eq!(quads1[40].code, HuffmanCode::new(0, 1).unwrap());
        assert_eq!(quads1[80].symbol, AacSpectralQuad::new(1, 1, 1, 1));
        assert_eq!(quads1[80].code, HuffmanCode::new(0x7f4, 11).unwrap());

        assert_eq!(quads2.len(), 81);
        assert_eq!(quads2[0].symbol, AacSpectralQuad::new(-1, -1, -1, -1));
        assert_eq!(quads2[0].code, HuffmanCode::new(0x1f3, 9).unwrap());
        assert_eq!(quads2[40].symbol, AacSpectralQuad::new(0, 0, 0, 0));
        assert_eq!(quads2[40].code, HuffmanCode::new(0, 3).unwrap());
        assert_eq!(quads2[80].symbol, AacSpectralQuad::new(1, 1, 1, 1));
        assert_eq!(quads2[80].code, HuffmanCode::new(0x1f6, 9).unwrap());

        let tables = aac_lc_standard_signed_quad_tables();
        assert_eq!(tables.quads1.len(), 81);
        assert_eq!(tables.quads2.len(), 81);
        assert_eq!(
            pack_spectral_quads_with_table(&[AacSpectralQuad::new(1, -1, 1, -1)], tables.quads2)
                .unwrap()
                .bit_len,
            8
        );
    }

    #[test]
    fn standard_aac_quad_workbench_uses_direct_signed_quads1_and_2() {
        assert_eq!(
            plan_aac_lc_standard_spectral_sections_by_bit_cost(&[0, 0, 0, 1], 4).unwrap(),
            vec![AacSpectralSection {
                start: 0,
                end: 4,
                codebook_id: 1,
            }]
        );
        assert_eq!(
            split_aac_lc_standard_spectral_payload_with_sign_bits_by_bit_cost(&[0, 0, 0, 1], 4)
                .unwrap()
                .spectral_bits
                .bit_len,
            5
        );

        assert_eq!(
            plan_aac_lc_standard_spectral_sections_by_bit_cost(&[1, -1, 1, -1], 4).unwrap(),
            vec![AacSpectralSection {
                start: 0,
                end: 4,
                codebook_id: 2,
            }]
        );
        assert_eq!(
            split_aac_lc_standard_spectral_payload_with_sign_bits_by_bit_cost(&[1, -1, 1, -1], 4)
                .unwrap()
                .spectral_bits
                .bit_len,
            8
        );
    }

    #[test]
    fn exposes_standard_aac_unsigned_quads3_and_4_tables() {
        let quads3 = aac_unsigned_quads3_table();
        let quads4 = aac_unsigned_quads4_table();

        assert_eq!(quads3.len(), 81);
        assert_eq!(quads3[0].symbol, AacSpectralMagnitudeQuad::new(0, 0, 0, 0));
        assert_eq!(quads3[0].code, HuffmanCode::new(0, 1).unwrap());
        assert_eq!(quads3[40].symbol, AacSpectralMagnitudeQuad::new(1, 1, 1, 1));
        assert_eq!(quads3[40].code, HuffmanCode::new(0x74, 7).unwrap());
        assert_eq!(quads3[80].symbol, AacSpectralMagnitudeQuad::new(2, 2, 2, 2));
        assert_eq!(quads3[80].code, HuffmanCode::new(0x7ffa, 15).unwrap());

        assert_eq!(quads4.len(), 81);
        assert_eq!(quads4[0].symbol, AacSpectralMagnitudeQuad::new(0, 0, 0, 0));
        assert_eq!(quads4[0].code, HuffmanCode::new(0x7, 4).unwrap());
        assert_eq!(quads4[40].symbol, AacSpectralMagnitudeQuad::new(1, 1, 1, 1));
        assert_eq!(quads4[40].code, HuffmanCode::new(0, 4).unwrap());
        assert_eq!(quads4[80].symbol, AacSpectralMagnitudeQuad::new(2, 2, 2, 2));
        assert_eq!(quads4[80].code, HuffmanCode::new(0x7fc, 11).unwrap());

        let tables = aac_lc_standard_unsigned_quad_tables();
        assert_eq!(tables.quads3.len(), 81);
        assert_eq!(tables.quads4.len(), 81);
        assert_eq!(
            select_quad_codebook_by_bit_cost(&[1, -1, 1, -1], tables).unwrap(),
            4
        );
        assert_eq!(
            pack_spectral_quad_sections_with_sign_bits(
                &[AacQuadSection {
                    start: 0,
                    end: 4,
                    codebook_id: 4,
                }],
                &[1, -1, 1, -1],
                tables,
            )
            .unwrap()
            .bit_len,
            8
        );
    }

    #[test]
    fn exposes_full_aac_unsigned_pairs8_table() {
        let table = aac_unsigned_pairs8_table();

        assert_eq!(table.len(), 64);
        assert_eq!(table[0].symbol, AacSpectralMagnitudePair::new(0, 0));
        assert_eq!(table[0].code, HuffmanCode::new(0x00e, 5).unwrap());
        assert_eq!(table[9].symbol, AacSpectralMagnitudePair::new(1, 1));
        assert_eq!(table[9].code, HuffmanCode::new(0x000, 3).unwrap());
        assert_eq!(table[63].symbol, AacSpectralMagnitudePair::new(7, 7));
        assert_eq!(table[63].code, HuffmanCode::new(0x3ff, 10).unwrap());

        let packed = pack_sectioned_spectral_payload_with_sign_bits_by_bit_cost(
            &[1, -1],
            2,
            AacSpectralMagnitudeTables::default(),
        )
        .unwrap();
        assert_eq!(
            packed,
            PackedBits {
                bytes: vec![0b1000_0000, 0b1000_0100],
                bit_len: 14,
            }
        );
    }

    #[test]
    fn exposes_full_aac_unsigned_pairs9_table() {
        let table = aac_unsigned_pairs9_table();

        assert_eq!(table.len(), 169);
        assert_eq!(table[0].symbol, AacSpectralMagnitudePair::new(0, 0));
        assert_eq!(table[0].code, HuffmanCode::new(0x0000, 1).unwrap());
        assert_eq!(table[14].symbol, AacSpectralMagnitudePair::new(1, 1));
        assert_eq!(table[14].code, HuffmanCode::new(0x000c, 4).unwrap());
        assert_eq!(table[168].symbol, AacSpectralMagnitudePair::new(12, 12));
        assert_eq!(table[168].code, HuffmanCode::new(0x7fff, 15).unwrap());
    }

    #[test]
    fn exposes_full_aac_unsigned_pairs10_table() {
        let table = aac_unsigned_pairs10_table();

        assert_eq!(table.len(), 169);
        assert_eq!(table[0].symbol, AacSpectralMagnitudePair::new(0, 0));
        assert_eq!(table[0].code, HuffmanCode::new(0x022, 6).unwrap());
        assert_eq!(table[14].symbol, AacSpectralMagnitudePair::new(1, 1));
        assert_eq!(table[14].code, HuffmanCode::new(0x000, 4).unwrap());
        assert_eq!(table[168].symbol, AacSpectralMagnitudePair::new(12, 12));
        assert_eq!(table[168].code, HuffmanCode::new(0xfff, 12).unwrap());
    }

    #[test]
    fn exposes_standard_aac_escape_table() {
        let table = aac_escape_table();

        assert_eq!(table.len(), 289);
        assert_eq!(table[0].symbol, AacSpectralMagnitudePair::new(0, 0));
        assert_eq!(table[0].code, HuffmanCode::new(0x000, 4).unwrap());
        assert_eq!(table[18].symbol, AacSpectralMagnitudePair::new(1, 1));
        assert_eq!(table[18].code, HuffmanCode::new(0x001, 4).unwrap());
        assert_eq!(table[288].symbol, AacSpectralMagnitudePair::new(16, 16));
        assert_eq!(table[288].code, HuffmanCode::new(0x004, 5).unwrap());
        assert_eq!(
            pack_spectral_pairs_with_sign_bits(&[AacSpectralPair::new(-17, 0)], table).unwrap(),
            PackedBits {
                bytes: vec![0b1110_0001, 0b0100_0010],
                bit_len: 15,
            }
        );
        assert_eq!(
            pack_sectioned_spectral_payload_with_sign_bits_by_bit_cost(
                &[12, -12],
                2,
                AacSpectralMagnitudeTables {
                    pairs1: &[],
                    pairs5: &[],
                    pairs6: &[],
                    escape: table,
                },
            )
            .unwrap(),
            PackedBits {
                bytes: vec![0b1011_0000, 0b1111_1110, 0b0111_0100],
                bit_len: 22,
            }
        );
    }

    #[test]
    fn standard_aac_lc_spectral_tables_include_escape_codebook() {
        let tables = aac_lc_standard_spectral_tables();

        assert_eq!(
            select_codebook_by_bit_cost(&[17, 0], tables).unwrap(),
            AacCodebook::Escape
        );
        assert_eq!(
            pack_sectioned_spectral_payload_with_sign_bits_by_bit_cost(&[17, 0], 2, tables)
                .unwrap()
                .bit_len,
            24
        );
    }

    #[test]
    fn packs_aac_escape_spectral_pairs_with_suffix_bits() {
        let table = [
            HuffmanEntry {
                symbol: AacSpectralMagnitudePair::new(16, 0),
                code: HuffmanCode::new(0b10, 2).unwrap(),
            },
            HuffmanEntry {
                symbol: AacSpectralMagnitudePair::new(16, 16),
                code: HuffmanCode::new(0b11, 2).unwrap(),
            },
        ];

        assert_eq!(
            super::pack_spectral_pairs_with_sign_bits(&[AacSpectralPair::new(-17, 0)], &table)
                .unwrap(),
            PackedBits {
                bytes: vec![0b1010_0001],
                bit_len: 8,
            }
        );
        assert_eq!(
            super::pack_spectral_pairs_with_sign_bits(&[AacSpectralPair::new(32, -18)], &table)
                .unwrap(),
            PackedBits {
                bytes: vec![0b1101_1000, 0b0000_0010],
                bit_len: 16,
            }
        );
    }

    #[test]
    fn packs_aac_spectral_sections_with_sign_bits() {
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

        let packed = pack_spectral_sections_with_sign_bits(
            &sections,
            &quantized,
            AacSpectralMagnitudeTables {
                pairs1: &pairs1,
                pairs5: &pairs5,
                pairs6: &[],
                escape: &[],
            },
        )
        .unwrap();

        assert_eq!(
            packed,
            PackedBits {
                bytes: vec![0b1001_0011, 0b1000_0000],
                bit_len: 10,
            }
        );
    }

    #[test]
    fn packs_aac_spectral_sections_from_tables() {
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

        let packed = pack_spectral_sections(
            &sections,
            &quantized,
            AacSpectralTables {
                signed_pairs1: &signed_pairs1,
                signed_pairs5: &signed_pairs5,
                signed_pairs6: &[],
                escape: &[],
            },
        )
        .unwrap();

        assert_eq!(
            packed,
            PackedBits {
                bytes: vec![0b1001_1000],
                bit_len: 5,
            }
        );
    }

    #[test]
    fn packs_aac_sectioned_spectral_payload() {
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

        let packed = pack_sectioned_spectral_payload(
            &sections,
            &quantized,
            2,
            AacSpectralTables {
                signed_pairs1: &signed_pairs1,
                signed_pairs5: &signed_pairs5,
                signed_pairs6: &[],
                escape: &[],
            },
        )
        .unwrap();

        assert_eq!(
            packed,
            PackedBits {
                bytes: vec![0x00, 0x88, 0x54, 0x53],
                bit_len: 32,
            }
        );
        assert!(
            pack_spectral_sections(&sections[1..2], &quantized, AacSpectralTables::default(),)
                .is_err()
        );
    }

    #[test]
    fn packs_aac_sectioned_spectral_payload_with_sign_bits() {
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

        let packed = pack_sectioned_spectral_payload_with_sign_bits(
            &sections,
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
            packed,
            PackedBits {
                bytes: vec![0x00, 0x88, 0x54, 0x52, 0x70],
                bit_len: 37,
            }
        );
    }

    #[test]
    fn packs_aac_codebook6_sections_from_caller_table() {
        let quantized = [1, -1];
        let sections = vec![AacSection {
            start: 0,
            end: 2,
            codebook: AacCodebook::SignedPairs6,
        }];
        let signed_pairs6 = [HuffmanEntry {
            symbol: AacSpectralPair::new(1, -1),
            code: HuffmanCode::new(0b10, 2).unwrap(),
        }];
        let magnitude_pairs6 = [HuffmanEntry {
            symbol: AacSpectralMagnitudePair::new(1, 1),
            code: HuffmanCode::new(0b0, 1).unwrap(),
        }];

        assert_eq!(
            pack_spectral_sections(
                &sections,
                &quantized,
                AacSpectralTables {
                    signed_pairs1: &[],
                    signed_pairs5: &[],
                    signed_pairs6: &signed_pairs6,
                    escape: &[],
                },
            )
            .unwrap(),
            PackedBits {
                bytes: vec![0b1000_0000],
                bit_len: 2,
            }
        );
        assert_eq!(
            pack_sectioned_spectral_payload_with_sign_bits(
                &sections,
                &quantized,
                2,
                AacSpectralMagnitudeTables {
                    pairs1: &[],
                    pairs5: &[],
                    pairs6: &magnitude_pairs6,
                    escape: &[],
                },
            )
            .unwrap(),
            PackedBits {
                bytes: vec![0b0110_0000, 0b1001_0000],
                bit_len: 12,
            }
        );
    }

    #[test]
    fn converts_and_packs_aac_spectral_quads_with_sign_bits() {
        let quantized = [1, -1, 0, 2, 0, 0, 0, 0];
        let section = AacSection {
            start: 0,
            end: 4,
            codebook: AacCodebook::SignedPairs1,
        };
        let quads = spectral_quads_for_section(&quantized, &section).unwrap();
        let signed_table = [HuffmanEntry {
            symbol: AacSpectralQuad::new(1, -1, 0, 2),
            code: HuffmanCode::new(0b11, 2).unwrap(),
        }];
        let magnitude_table = [HuffmanEntry {
            symbol: AacSpectralMagnitudeQuad::new(1, 1, 0, 2),
            code: HuffmanCode::new(0b101, 3).unwrap(),
        }];

        assert_eq!(quads, vec![AacSpectralQuad::new(1, -1, 0, 2)]);
        assert_eq!(
            spectral_quads_for_section(
                &quantized,
                &AacSection {
                    start: 4,
                    end: 8,
                    codebook: AacCodebook::Zero,
                },
            )
            .unwrap(),
            Vec::<AacSpectralQuad>::new()
        );
        assert_eq!(
            pack_spectral_quads_with_table(&quads, &signed_table).unwrap(),
            PackedBits {
                bytes: vec![0b1100_0000],
                bit_len: 2,
            }
        );
        assert_eq!(
            pack_spectral_quads_with_sign_bits(&quads, &magnitude_table).unwrap(),
            PackedBits {
                bytes: vec![0b1010_1000],
                bit_len: 6,
            }
        );
        assert!(spectral_quads_for_section(
            &quantized,
            &AacSection {
                start: 0,
                end: 2,
                codebook: AacCodebook::SignedPairs1,
            },
        )
        .is_err());
    }

    #[test]
    fn packs_aac_quad_sections_with_sign_bits() {
        let quantized = [1, -1, 0, 1, 0, 1, -1, 0, 0, 0, 0, 0];
        let sections = vec![
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
        ];
        let table = [
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
            quads3: &table,
            ..Default::default()
        };

        assert_eq!(
            pack_quad_section_data_with_len(&sections, 4).unwrap(),
            PackedBits {
                bytes: vec![0b0011_0001, 0b0000_0000, 0b0100_0000],
                bit_len: 18,
            }
        );
        assert_eq!(
            pack_spectral_quad_sections_with_sign_bits(&sections, &quantized, tables).unwrap(),
            PackedBits {
                bytes: vec![0b1001_0110, 0b1000_0000],
                bit_len: 9,
            }
        );
        assert_eq!(
            pack_sectioned_spectral_quad_payload_with_sign_bits(&sections, &quantized, 4, tables)
                .unwrap(),
            PackedBits {
                bytes: vec![0b0011_0001, 0b0000_0000, 0b0110_0101, 0b1010_0000],
                bit_len: 27,
            }
        );
        assert!(pack_quad_section_data_with_len(
            &[AacQuadSection {
                start: 0,
                end: 4,
                codebook_id: 5,
            }],
            4,
        )
        .is_err());
    }

