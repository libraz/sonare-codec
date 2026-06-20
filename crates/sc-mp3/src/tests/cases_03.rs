    #[test]
    fn mpeg2_lsf_long_compress_decodes_to_selected_partition() {
        // A spread of per-band scale factors exercising distinct group maxima.
        let mut scale_factors = [0_u8; MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT];
        scale_factors[0] = 7; // group 0 (bands 0..6): needs slen 3
        scale_factors[7] = 3; // group 1 (bands 6..11): needs slen 2
        scale_factors[12] = 1; // group 2 (bands 11..16): needs slen 1
        scale_factors[18] = 2; // group 3 (bands 16..21): needs slen 2

        let selection = select_mpeg2_layer3_lsf_long_scale_factor_compress(&scale_factors).unwrap();
        assert!(selection.scalefac_compress < 500);

        // The serialized scalefac_compress must round-trip through the decoder
        // derivation to the identical partition the encoder packed with.
        let (groups, slen) = decode_mpeg2_lsf_long_partition(selection.scalefac_compress);
        assert_eq!(groups, selection.group_sizes);
        assert_eq!(slen, selection.slen);

        // Every band fits its group's bit width, so packing succeeds and the
        // bit length equals the sum of group_size * slen.
        let packed = pack_mpeg2_layer3_lsf_long_scale_factors(&scale_factors, selection).unwrap();
        let expected_bits: usize = (0..4)
            .map(|g| usize::from(selection.group_sizes[g]) * usize::from(selection.slen[g]))
            .sum();
        assert_eq!(packed.bit_len, expected_bits);
    }

    #[test]
    fn mpeg2_lsf_long_compress_prefers_branch_zero_for_silence() {
        let scale_factors = [0_u8; MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT];
        let selection = select_mpeg2_layer3_lsf_long_scale_factor_compress(&scale_factors).unwrap();
        assert_eq!(selection.scalefac_compress, 0);
        assert_eq!(selection.group_sizes, [6, 5, 5, 5]);
        assert_eq!(selection.slen, [0, 0, 0, 0]);
        let packed = pack_mpeg2_layer3_lsf_long_scale_factors(&scale_factors, selection).unwrap();
        assert_eq!(packed.bit_len, 0);
    }

    #[test]
    fn mpeg2_lsf_long_compress_for_granule_records_scalefac_compress() {
        let mut scale_factors = [0_u8; MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT];
        scale_factors[3] = 4;
        let mut granule = Layer3GranuleChannelInfo::default();
        let selection = select_mpeg2_layer3_lsf_long_scale_factor_compress(&scale_factors).unwrap();
        pack_mpeg2_layer3_lsf_long_scale_factors_for_granule(&mut granule, &scale_factors).unwrap();
        assert_eq!(granule.scalefac_compress, selection.scalefac_compress);
    }

    #[test]
    fn mpeg2_lsf_long_compress_rejects_uncodable_high_band() {
        // Band 20 lands in the final group of both preflag=0 branches, whose
        // bit-width caps are 3 (branch 0) and 0 (branch 1). A value of 8 needs
        // four bits, so neither branch can represent it.
        let mut scale_factors = [0_u8; MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT];
        scale_factors[20] = 8;
        assert!(select_mpeg2_layer3_lsf_long_scale_factor_compress(&scale_factors).is_err());
    }

    #[test]
    fn selects_mpeg1_layer3_long_scale_factors_from_quantized_spectrum() {
        let mut quantized = [0_i32; 42];
        quantized[0] = 1;
        quantized[20] = 15;
        quantized[22] = 7;
        quantized[40] = 8191;

        let scale_factors =
            select_mpeg1_layer3_long_scale_factors_for_quantized_spectrum(&quantized).unwrap();

        let mut expected = [0_u8; MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT];
        expected[0] = 1;
        expected[10] = 4;
        expected[11] = 3;
        expected[20] = 7;
        assert_eq!(scale_factors, expected);
        assert!(select_mpeg1_layer3_long_scale_factors_for_quantized_spectrum(&[]).is_err());
        assert!(select_mpeg1_layer3_long_scale_factors_for_quantized_spectrum(&[8192]).is_err());
        assert!(
            select_mpeg1_layer3_long_scale_factors_for_quantized_spectrum(&[i32::MIN]).is_err()
        );
    }

    #[test]
    fn packs_mpeg1_layer3_long_quantized_spectrum_for_granule() {
        let big_value_table = [
            HuffmanEntry {
                symbol: Layer3BigValueMagnitude::new(3, 2),
                code: HuffmanCode::new(0b10, 2).unwrap(),
            },
            HuffmanEntry {
                symbol: Layer3BigValueMagnitude::new(0, 0),
                code: HuffmanCode::new(0b0, 1).unwrap(),
            },
        ];
        let count1_table = [HuffmanEntry {
            symbol: Layer3Count1MagnitudeQuad::new(1, 1, 0, 1),
            code: HuffmanCode::new(0b11, 2).unwrap(),
        }];
        let mut scale_factors = [0_u8; MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT];
        scale_factors[0] = 3;
        scale_factors[10] = 2;
        scale_factors[11] = 1;
        scale_factors[20] = 1;
        let quantized = [3, -2, 0, 0, 1, -1, 0, 1, 0, 0];
        let mut granule = Layer3GranuleChannelInfo::default();

        let packed = pack_mpeg1_layer3_long_quantized_spectrum_for_granule(
            &mut granule,
            &scale_factors,
            &quantized,
            Layer3EntropyTables {
                big_values: &big_value_table,
                count1: &count1_table,
            },
        )
        .unwrap();

        assert_eq!(
            packed,
            PackedBits {
                bytes: vec![
                    0b1100_0000,
                    0b0000_0000,
                    0b0000_1010,
                    0b0000_0001,
                    0b1001_0110,
                    0b1000_0000,
                ],
                bit_len: 42,
            }
        );
        assert_eq!(granule.scalefac_compress, 8);
        assert_eq!(granule.big_values, 2);
        assert_eq!(granule.table_select, [5, 5, 5]);
        assert!(granule.count1table_select);
        assert_eq!(granule.part2_3_length, 42);
    }

    #[test]
    fn packs_mpeg1_layer3_long_quantized_spectrum_with_table_provider() {
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
        let mut scale_factors = [0_u8; MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT];
        scale_factors[11] = 1;
        let quantized = [3, -2, 0, 0, 1, -1, 0, 1, 0, 0];
        let mut granule = Layer3GranuleChannelInfo::default();

        let packed = pack_mpeg1_layer3_long_quantized_spectrum_with_table_provider(
            &mut granule,
            &scale_factors,
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
                bytes: vec![0b1000_0000, 0b0010_0101, 0b1010_0000],
                bit_len: 20,
            }
        );
        assert_eq!(granule.scalefac_compress, 1);
        assert_eq!(granule.table_select, [5, 0, 0]);
        assert!(granule.count1table_select);
        assert_eq!(granule.part2_3_length, 20);
    }

    #[test]
    fn packs_mpeg1_layer3_long_quantized_spectrum_with_selected_scale_factors() {
        let big_value_table = [
            HuffmanEntry {
                symbol: Layer3BigValueMagnitude::new(3, 2),
                code: HuffmanCode::new(0b10, 2).unwrap(),
            },
            HuffmanEntry {
                symbol: Layer3BigValueMagnitude::new(0, 0),
                code: HuffmanCode::new(0b0, 1).unwrap(),
            },
        ];
        let count1_table = [HuffmanEntry {
            symbol: Layer3Count1MagnitudeQuad::new(1, 1, 0, 1),
            code: HuffmanCode::new(0b11, 2).unwrap(),
        }];
        let quantized = [3, -2, 0, 0, 1, -1, 0, 1, 0, 0];
        let scale_factors =
            select_mpeg1_layer3_long_scale_factors_for_quantized_spectrum(&quantized).unwrap();
        let mut manual_granule = Layer3GranuleChannelInfo::default();
        let manual = pack_mpeg1_layer3_long_quantized_spectrum_for_granule(
            &mut manual_granule,
            &scale_factors,
            &quantized,
            Layer3EntropyTables {
                big_values: &big_value_table,
                count1: &count1_table,
            },
        )
        .unwrap();

        let mut selected_granule = Layer3GranuleChannelInfo::default();
        let selected =
            pack_mpeg1_layer3_long_quantized_spectrum_with_selected_scale_factors_for_granule(
                &mut selected_granule,
                &quantized,
                Layer3EntropyTables {
                    big_values: &big_value_table,
                    count1: &count1_table,
                },
            )
            .unwrap();

        assert_eq!(selected, manual);
        assert_eq!(
            selected_granule.scalefac_compress,
            manual_granule.scalefac_compress
        );
        assert_eq!(
            selected_granule.part2_3_length,
            manual_granule.part2_3_length
        );
    }

    #[test]
    fn packs_mpeg1_layer3_long_quantized_spectrum_with_selected_scale_factors_and_provider() {
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
        let quantized = [3, -2, 0, 0, 1, -1, 0, 1, 0, 0];
        let scale_factors =
            select_mpeg1_layer3_long_scale_factors_for_quantized_spectrum(&quantized).unwrap();
        let provider = Layer3EntropyTableProvider {
            big_value_table_5: &big_value_table_5,
            count1_table_1: &count1_table_1,
            ..Default::default()
        };
        let mut manual_granule = Layer3GranuleChannelInfo::default();
        let manual = pack_mpeg1_layer3_long_quantized_spectrum_with_table_provider(
            &mut manual_granule,
            &scale_factors,
            &quantized,
            provider,
        )
        .unwrap();

        let mut selected_granule = Layer3GranuleChannelInfo::default();
        let selected =
            pack_mpeg1_layer3_long_quantized_spectrum_with_selected_scale_factors_and_table_provider(
                &mut selected_granule,
                &quantized,
                provider,
            )
            .unwrap();

        assert_eq!(selected, manual);
        assert_eq!(
            selected_granule.scalefac_compress,
            manual_granule.scalefac_compress
        );
        assert_eq!(
            selected_granule.part2_3_length,
            manual_granule.part2_3_length
        );
    }

    #[test]
    fn packs_mpeg1_layer3_pcm_long_block_with_selected_scale_factors() {
        let pcm = AudioBuffer::new(44_100, 1, vec![0.0; 36]).unwrap();
        let quantized = quantize_pcm_long_block(&pcm, 0, 0, 1.0).unwrap();
        let tables = Layer3EntropyTables {
            big_values: &[],
            count1: &[],
        };
        let mut manual_granule = Layer3GranuleChannelInfo::default();
        let manual =
            pack_mpeg1_layer3_long_quantized_spectrum_with_selected_scale_factors_for_granule(
                &mut manual_granule,
                &quantized,
                tables,
            )
            .unwrap();

        let mut pcm_granule = Layer3GranuleChannelInfo::default();
        let packed = pack_mpeg1_layer3_pcm_long_block_with_calibrated_gain_for_granule(
            &mut pcm_granule,
            &pcm,
            0,
            0,
            1.0,
            tables,
        )
        .unwrap();

        assert_eq!(packed, manual);
        assert_eq!(packed.bit_len, 0);
        assert_eq!(
            pcm_granule.scalefac_compress,
            manual_granule.scalefac_compress
        );
        assert_eq!(pcm_granule.part2_3_length, manual_granule.part2_3_length);
    }

    #[test]
    fn packs_mpeg1_layer3_pcm_long_block_with_perceptual_scale_factors() {
        // A tonal granule: the perceptual path must pack within the 12-bit
        // part2_3 field and leave scalefac_scale at zero.
        let samples: Vec<f32> = (0..1152)
            .map(|n| 0.4 * (std::f64::consts::TAU * 1000.0 * n as f64 / 44_100.0).sin() as f32)
            .collect();
        let pcm = AudioBuffer::new(44_100, 1, samples).unwrap();
        let provider = mpeg1_layer3_standard_table_provider();

        let mut granule = Layer3GranuleChannelInfo::default();
        let packed =
            pack_mpeg1_layer3_pcm_long_block_with_perceptual_scale_factors_and_table_provider(
                &mut granule,
                &pcm,
                0,
                0,
                0.05,
                provider,
            )
            .unwrap();

        assert!(!granule.scalefac_scale);
        assert!(packed.bit_len > 0, "a tonal granule must encode some bits");
        assert!(
            granule.part2_3_length <= 4095,
            "part2_3_length {} exceeds the 12-bit field",
            granule.part2_3_length
        );

        let mut scalefac_scale_granule = Layer3GranuleChannelInfo::default();
        let scalefac_scale_packed =
            pack_mpeg1_layer3_pcm_long_block_with_perceptual_scalefac_scale_and_table_provider(
                &mut scalefac_scale_granule,
                &pcm,
                0,
                0,
                0.05,
                true,
                provider,
            )
            .unwrap();
        assert!(scalefac_scale_granule.scalefac_scale);
        assert!(
            scalefac_scale_packed.bit_len > 0,
            "a tonal granule must encode some bits with scalefac_scale enabled"
        );
        assert!(
            scalefac_scale_granule.part2_3_length <= 4095,
            "part2_3_length {} exceeds the 12-bit field with scalefac_scale enabled",
            scalefac_scale_granule.part2_3_length
        );

        // A silent granule packs cleanly with the reference gain and no bits.
        let silent = AudioBuffer::new(44_100, 1, vec![0.0; 1152]).unwrap();
        let mut silent_granule = Layer3GranuleChannelInfo::default();
        let silent_packed =
            pack_mpeg1_layer3_pcm_long_block_with_perceptual_scale_factors_and_table_provider(
                &mut silent_granule,
                &silent,
                0,
                0,
                0.05,
                provider,
            )
            .unwrap();
        assert_eq!(silent_packed.bit_len, 0);
        assert_eq!(silent_granule.global_gain, 210);
    }

    #[test]
    fn assembles_mpeg1_layer3_pcm_frame_with_perceptual_scale_factors() {
        let samples: Vec<f32> = (0..1152)
            .map(|n| {
                let t = n as f32 / 44_100.0;
                0.25 * (std::f32::consts::TAU * 880.0 * t).sin()
            })
            .collect();
        let pcm = AudioBuffer::new(44_100, 1, samples).unwrap();
        let header = layer3_header_for_capacity(44_100, 1, 128, false, false).unwrap();
        let provider = mpeg1_layer3_standard_table_provider();

        let frame =
            assemble_mpeg1_layer3_pcm_frame_with_perceptual_scale_factors_and_table_provider(
                header, &pcm, 0, 0.1, provider,
            )
            .unwrap();
        let parsed = FrameHeader::parse(&frame[..4]).unwrap();

        assert_eq!(parsed, header);
        assert_eq!(frame.len(), header.frame_len());
        assert!(frame[4..].iter().any(|&byte| byte != 0));
    }

    #[test]
    fn encodes_mpeg1_layer3_pcm_frames_with_perceptual_scale_factors() {
        let samples: Vec<f32> = (0..2304)
            .map(|n| {
                let t = n as f32 / 44_100.0;
                0.20 * (std::f32::consts::TAU * 660.0 * t).sin()
                    + 0.08 * (std::f32::consts::TAU * 2200.0 * t).sin()
            })
            .collect();
        let pcm = AudioBuffer::new(44_100, 1, samples).unwrap();
        let header = layer3_header_for_capacity(44_100, 1, 128, false, false).unwrap();
        let provider = mpeg1_layer3_standard_table_provider();

        let implicit =
            encode_mpeg1_layer3_pcm_frames_with_perceptual_scale_factors_and_table_provider(
                &pcm, 0.1, provider,
            )
            .unwrap();
        let explicit =
            encode_mpeg1_layer3_pcm_frames_with_header_and_perceptual_scale_factors_and_table_provider(
                header,
                &pcm,
                0.1,
                provider,
            )
            .unwrap();

        assert_eq!(implicit, explicit);
        assert_eq!(implicit.len(), header.frame_len() * 2);
        assert_eq!(FrameHeader::parse(&implicit[..4]).unwrap(), header);
        assert_eq!(
            FrameHeader::parse(&implicit[header.frame_len()..header.frame_len() + 4]).unwrap(),
            header
        );
    }

    #[test]
    fn selects_perceptual_pcm_frame_step_with_payload_budget() {
        let samples: Vec<f32> = (0..1152)
            .map(|n| {
                let t = n as f32 / 44_100.0;
                0.18 * (std::f32::consts::TAU * 740.0 * t).sin()
                    + 0.07 * (std::f32::consts::TAU * 1900.0 * t).sin()
            })
            .collect();
        let pcm = AudioBuffer::new(44_100, 1, samples).unwrap();
        let header = layer3_header_for_capacity(44_100, 1, 128, false, false).unwrap();
        let provider = mpeg1_layer3_standard_table_provider();
        let candidates = [0.025_f32, 0.05, 0.1, 0.2];

        let unconstrained =
            select_mpeg1_layer3_pcm_frame_perceptual_step_details_with_table_provider(
                header,
                &pcm,
                0,
                &candidates,
                provider,
            )
            .unwrap();
        let step =
            select_mpeg1_layer3_pcm_frame_perceptual_step_with_max_payload_bits_and_table_provider(
                header,
                &pcm,
                0,
                &candidates,
                unconstrained.payload_bit_len,
                provider,
            )
            .unwrap();
        let budgeted =
            select_mpeg1_layer3_pcm_frame_perceptual_step_details_with_max_payload_bits_and_table_provider(
                header,
                &pcm,
                0,
                &candidates,
                unconstrained.payload_bit_len,
                provider,
            )
            .unwrap();

        assert_eq!(
            select_mpeg1_layer3_pcm_frame_perceptual_step_with_table_provider(
                header,
                &pcm,
                0,
                &candidates,
                provider,
            )
            .unwrap(),
            unconstrained.step
        );
        assert_eq!(step, unconstrained.step);
        assert_eq!(budgeted.step, unconstrained.step);
        assert_eq!(budgeted.frame_capacity_bits, unconstrained.payload_bit_len);
        assert!(budgeted.payload_bit_len <= unconstrained.payload_bit_len);
        assert!(
            select_mpeg1_layer3_pcm_frame_perceptual_step_details_with_max_payload_bits_and_table_provider(
                header,
                &pcm,
                0,
                &candidates,
                0,
                provider,
            )
            .is_err()
        );
    }

    #[test]
    fn selects_perceptual_active_step_when_scale_factors_would_otherwise_stay_zero() {
        let samples: Vec<f32> = (0..1152)
            .map(|n| {
                let t = n as f32 / 44_100.0;
                0.20 * (std::f32::consts::TAU * 660.0 * t).sin()
                    + 0.08 * (std::f32::consts::TAU * 2200.0 * t).sin()
            })
            .collect();
        let pcm = AudioBuffer::new(44_100, 1, samples).unwrap();
        let header = layer3_header_for_capacity(44_100, 1, 128, false, false).unwrap();
        let provider = mpeg1_layer3_standard_table_provider();
        let candidates = [0.0005_f32, 0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0];

        let ordinary = select_mpeg1_layer3_pcm_frame_perceptual_step_details_with_table_provider(
            header,
            &pcm,
            0,
            &candidates,
            provider,
        )
        .unwrap();
        let active =
            select_mpeg1_layer3_pcm_frame_perceptual_active_step_details_with_table_provider(
                header,
                &pcm,
                0,
                &candidates,
                provider,
            )
            .unwrap();
        let active_step = select_mpeg1_layer3_pcm_frame_perceptual_active_step_with_table_provider(
            header,
            &pcm,
            0,
            &candidates,
            provider,
        )
        .unwrap();

        assert_eq!(active_step, active.step);
        assert!(ordinary.step < active.step);
        assert!(active.payload_bit_len <= active.frame_capacity_bits);
        assert!(active.payload_bit_len < ordinary.payload_bit_len);
    }

    #[test]
    fn encodes_perceptual_pcm_frames_with_auto_step_and_payload_budget() {
        let samples: Vec<f32> = (0..2304)
            .map(|n| {
                let t = n as f32 / 44_100.0;
                0.16 * (std::f32::consts::TAU * 520.0 * t).sin()
                    + 0.05 * (std::f32::consts::TAU * 1500.0 * t).sin()
            })
            .collect();
        let pcm = AudioBuffer::new(44_100, 1, samples).unwrap();
        let header = layer3_header_for_capacity(44_100, 1, 128, false, false).unwrap();
        let provider = mpeg1_layer3_standard_table_provider();
        let candidates = [0.025_f32, 0.05, 0.1, 0.2];
        let first_frame =
            select_mpeg1_layer3_pcm_frame_perceptual_step_details_with_table_provider(
                header,
                &pcm,
                0,
                &candidates,
                provider,
            )
            .unwrap();
        let budget = first_frame.payload_bit_len.max(1);

        let auto = encode_mpeg1_layer3_pcm_frames_with_perceptual_auto_step_and_table_provider(
            &pcm,
            &candidates,
            provider,
        )
        .unwrap();
        let explicit =
            encode_mpeg1_layer3_pcm_frames_with_header_and_perceptual_auto_step_and_table_provider(
                header,
                &pcm,
                &candidates,
                provider,
            )
            .unwrap();
        let budgeted =
            encode_mpeg1_layer3_pcm_frames_with_perceptual_max_payload_bits_and_table_provider(
                &pcm,
                &candidates,
                budget,
                provider,
            )
            .unwrap();
        let explicit_budgeted =
            encode_mpeg1_layer3_pcm_frames_with_header_and_perceptual_max_payload_bits_and_table_provider(
                header,
                &pcm,
                &candidates,
                budget,
                provider,
            )
            .unwrap();

        assert_eq!(auto, explicit);
        assert_eq!(budgeted, explicit_budgeted);
        assert_eq!(FrameHeader::parse(&auto[..4]).unwrap(), header);
        assert_eq!(FrameHeader::parse(&budgeted[..4]).unwrap(), header);
        assert_eq!(auto.len(), header.frame_len() * 2);
        assert_eq!(budgeted.len(), header.frame_len() * 2);
    }

    #[test]
    fn packs_mpeg1_layer3_pcm_long_block_with_selected_scale_factors_and_provider() {
        let pcm = AudioBuffer::new(44_100, 1, vec![0.0; 36]).unwrap();
        let quantized = quantize_pcm_long_block(&pcm, 0, 0, 1.0).unwrap();
        let provider = Layer3EntropyTableProvider::default();
        let mut manual_granule = Layer3GranuleChannelInfo::default();
        let manual =
            pack_mpeg1_layer3_long_quantized_spectrum_with_selected_scale_factors_and_table_provider(
                &mut manual_granule,
                &quantized,
                provider,
            )
            .unwrap();

        let mut pcm_granule = Layer3GranuleChannelInfo::default();
        let packed = pack_mpeg1_layer3_pcm_long_block_with_calibrated_gain_and_table_provider(
            &mut pcm_granule,
            &pcm,
            0,
            0,
            1.0,
            provider,
        )
        .unwrap();

        assert_eq!(packed, manual);
        assert_eq!(pcm_granule.big_values, 0);
        assert_eq!(pcm_granule.part2_3_length, manual_granule.part2_3_length);
        assert!(
            pack_mpeg1_layer3_pcm_long_block_with_calibrated_gain_and_table_provider(
                &mut Layer3GranuleChannelInfo::default(),
                &pcm,
                1,
                0,
                1.0,
                provider,
            )
            .is_err()
        );
    }

    #[test]
    fn packs_quantized_spectrum_for_granule() {
        let big_value_table = [
            HuffmanEntry {
                symbol: Layer3BigValueMagnitude::new(3, 2),
                code: HuffmanCode::new(0b10, 2).unwrap(),
            },
            HuffmanEntry {
                symbol: Layer3BigValueMagnitude::new(0, 0),
                code: HuffmanCode::new(0b0, 1).unwrap(),
            },
        ];
        let count1_table = [HuffmanEntry {
            symbol: Layer3Count1MagnitudeQuad::new(1, 1, 0, 1),
            code: HuffmanCode::new(0b11, 2).unwrap(),
        }];
        let quantized = [3, -2, 0, 0, 1, -1, 0, 1, 0, 0];
        let mut granule = Layer3GranuleChannelInfo::default();

        let packed = pack_quantized_spectrum_for_granule(
            &mut granule,
            &quantized,
            Layer3EntropyTables {
                big_values: &big_value_table,
                count1: &count1_table,
            },
        )
        .unwrap();

        assert_eq!(
            packed,
            PackedBits {
                bytes: vec![0b1001_0110, 0b1000_0000],
                bit_len: 10,
            }
        );
        assert_eq!(granule.big_values, 2);
        assert_eq!(granule.table_select, [5, 5, 5]);
        assert!(granule.count1table_select);
        assert_eq!(granule.part2_3_length, 10);
    }

    #[test]
    fn packs_quantized_spectrum_with_scale_factors_for_granule() {
        let big_value_table = [
            HuffmanEntry {
                symbol: Layer3BigValueMagnitude::new(3, 2),
                code: HuffmanCode::new(0b10, 2).unwrap(),
            },
            HuffmanEntry {
                symbol: Layer3BigValueMagnitude::new(0, 0),
                code: HuffmanCode::new(0b0, 1).unwrap(),
            },
        ];
        let count1_table = [HuffmanEntry {
            symbol: Layer3Count1MagnitudeQuad::new(1, 1, 0, 1),
            code: HuffmanCode::new(0b11, 2).unwrap(),
        }];
        let scale_factors = PackedBits {
            bytes: vec![0b1100_0000],
            bit_len: 2,
        };
        let quantized = [3, -2, 0, 0, 1, -1, 0, 1, 0, 0];
        let mut granule = Layer3GranuleChannelInfo::default();

        let packed = pack_quantized_spectrum_with_scale_factors_for_granule(
            &mut granule,
            scale_factors,
            &quantized,
            Layer3EntropyTables {
                big_values: &big_value_table,
                count1: &count1_table,
            },
        )
        .unwrap();

        assert_eq!(
            packed,
            PackedBits {
                bytes: vec![0b1110_0101, 0b1010_0000],
                bit_len: 12,
            }
        );
        assert_eq!(granule.big_values, 2);
        assert_eq!(granule.table_select, [5, 5, 5]);
        assert!(granule.count1table_select);
        assert_eq!(granule.part2_3_length, 12);
    }

    #[test]
    fn packs_quantized_spectrum_with_table_provider() {
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
        let quantized = [3, -2, 0, 0, 1, -1, 0, 1, 0, 0];
        let mut granule = Layer3GranuleChannelInfo::default();

        let packed = pack_quantized_spectrum_with_table_provider(
            &mut granule,
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
                bytes: vec![0b1001_0110, 0b1000_0000],
                bit_len: 10,
            }
        );
        assert_eq!(granule.table_select, [5, 0, 0]);
        assert!(granule.count1table_select);

        let err = pack_quantized_spectrum_with_table_provider(
            &mut Layer3GranuleChannelInfo::default(),
            &quantized,
            Layer3EntropyTableProvider::default(),
        )
        .unwrap_err();
        assert!(matches!(
            err,
            Error::UnsupportedFeature("MP3 big-values Huffman table")
        ));
    }

    #[test]
    fn table_provider_selects_big_value_tables_per_region() {
        let big_value_table_1 = [HuffmanEntry {
            symbol: Layer3BigValueMagnitude::new(1, 0),
            code: HuffmanCode::new(0b0, 1).unwrap(),
        }];
        let big_value_table_5 = [HuffmanEntry {
            symbol: Layer3BigValueMagnitude::new(3, 2),
            code: HuffmanCode::new(0b10, 2).unwrap(),
        }];
        let big_value_table_7 = [HuffmanEntry {
            symbol: Layer3BigValueMagnitude::new(5, 4),
            code: HuffmanCode::new(0b11, 2).unwrap(),
        }];
        // The big-value regions split at the fixed scalefactor-band boundaries
        // into 2 + 2 + remainder pairs, so lay out one homogeneous value per
        // region to exercise distinct per-region table selection.
        let mut quantized = Vec::new();
        for _ in 0..2 {
            quantized.extend_from_slice(&[1, 0]);
        }
        for _ in 0..2 {
            quantized.extend_from_slice(&[3, -2]);
        }
        for _ in 0..2 {
            quantized.extend_from_slice(&[5, 4]);
        }
        let mut granule = Layer3GranuleChannelInfo::default();

        let packed = pack_quantized_spectrum_with_table_provider(
            &mut granule,
            &quantized,
            Layer3EntropyTableProvider {
                big_value_table_1: &big_value_table_1,
                big_value_table_5: &big_value_table_5,
                big_value_table_7: &big_value_table_7,
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(granule.big_values, 6);
        assert_eq!(granule.region0_count, 0);
        assert_eq!(granule.region1_count, 0);
        assert_eq!(granule.table_select, [1, 5, 7]);
        assert!(!granule.count1table_select);
        // region0: 2x[1,0] = 2*(1 code + 1 sign); region1: 2x[3,-2] =
        // 2*(2 code + 2 signs); region2: 2x[5,4] = 2*(2 code + 2 signs).
        assert_eq!(granule.part2_3_length, 20);
        assert_eq!(packed.bit_len, 20);
    }

