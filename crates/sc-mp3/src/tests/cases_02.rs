    #[test]
    fn computes_long_block_mdct_for_layer3_analysis() {
        let mut samples = [0.0_f32; 36];
        samples[0] = 1.0;

        let coeffs = mdct_long_block(&samples).unwrap();

        assert_eq!(coeffs.len(), 18);
        assert!(coeffs.iter().any(|coeff| coeff.abs() > 0.0));
        assert_eq!(mdct_long_block(&[0.0; 36]).unwrap(), vec![0.0; 18]);
    }

    #[test]
    fn builds_layer3_analysis_subband_blocks() {
        let pcm = AudioBuffer::new(
            44_100,
            1,
            (0..2304)
                .map(|sample| ((sample as f32) * 0.01).sin() * 0.25)
                .collect(),
        )
        .unwrap();

        let low = layer3_analysis_subband_block(&pcm, 0, 0, 0).unwrap();
        let high = layer3_analysis_subband_block(&pcm, 0, 0, 31).unwrap();
        let padded = layer3_analysis_subband_block(&pcm, 0, 4096, 0).unwrap();

        assert_eq!(low.len(), 36);
        assert_eq!(high.len(), 36);
        assert_ne!(low, high);
        assert!(low.iter().any(|sample| sample.abs() > 0.0));
        assert_eq!(padded, [0.0; 36]);
        assert!(layer3_analysis_subband_block(&pcm, 0, 0, 32).is_err());
    }

    #[test]
    fn analysis_filterbank_localizes_tones_by_subband() {
        let sample_rate = 44_100.0_f32;
        // Subband `b` covers the band centered near (b + 0.5) * sample_rate / 64.
        for band in [0_usize, 5, 16, 28] {
            let freq = (band as f32 + 0.5) * sample_rate / 64.0;
            let pcm = AudioBuffer::new(
                44_100,
                1,
                (0..4096)
                    .map(|n| {
                        (2.0 * core::f32::consts::PI * freq * (n as f32) / sample_rate).sin() * 0.5
                    })
                    .collect(),
            )
            .unwrap();

            // Analyse a granule whose 512-sample window is fully populated.
            let energy = |subband: usize| -> f32 {
                layer3_analysis_subband_block(&pcm, 0, 1152, subband)
                    .unwrap()
                    .iter()
                    .map(|s| s * s)
                    .sum()
            };
            let peak = (0..32)
                .max_by(|a, b| energy(*a).partial_cmp(&energy(*b)).unwrap())
                .unwrap();
            assert_eq!(
                peak, band,
                "tone at {freq} Hz should peak in subband {band}"
            );
        }
    }

    #[test]
    fn frequency_inversion_is_scoped_and_self_inverse() {
        let original: [f32; LONG_BLOCK_GRANULE_SAMPLES] = core::array::from_fn(|i| i as f32 + 1.0);

        // Even subbands are untouched.
        let mut even = original;
        apply_frequency_inversion(0, &mut even);
        assert_eq!(even, original);

        // Odd subbands negate odd-indexed samples only.
        let mut odd = original;
        apply_frequency_inversion(1, &mut odd);
        for (i, (got, base)) in odd.iter().zip(original.iter()).enumerate() {
            if i % 2 == 1 {
                assert_eq!(*got, -*base);
            } else {
                assert_eq!(*got, *base);
            }
        }

        // Applying the inversion twice restores the input.
        apply_frequency_inversion(1, &mut odd);
        assert_eq!(odd, original);
    }

    #[test]
    fn alias_reduction_inverts_the_decoder_rotation() {
        let mut spectrum: Vec<f32> = (0..576).map(|i| ((i * 7) % 13) as f32 - 6.0).collect();
        let original = spectrum.clone();

        apply_alias_reduction(&mut spectrum);
        assert_ne!(
            spectrum, original,
            "alias reduction should change the spectrum"
        );

        // The decoder applies the forward rotation; it must undo the encoder's.
        for boundary in 0..(filterbank::SUBBANDS - 1) {
            let upper_base =
                boundary * LONG_BLOCK_GRANULE_SAMPLES + (LONG_BLOCK_GRANULE_SAMPLES - 1);
            let lower_base = (boundary + 1) * LONG_BLOCK_GRANULE_SAMPLES;
            for (i, &c) in ALIAS_REDUCTION_C.iter().enumerate() {
                let cs = 1.0 / (1.0 + c * c).sqrt();
                let ca = c / (1.0 + c * c).sqrt();
                let upper = upper_base - i;
                let lower = lower_base + i;
                let a = spectrum[upper];
                let b = spectrum[lower];
                spectrum[upper] = a * cs - b * ca;
                spectrum[lower] = b * cs + a * ca;
            }
        }

        for (got, base) in spectrum.iter().zip(original.iter()) {
            assert!(
                (got - base).abs() < 1e-5,
                "rotation pair should be transparent"
            );
        }
    }

    #[test]
    fn long_block_spectrum_shape_and_silence() {
        let silent = AudioBuffer::new(44_100, 1, vec![0.0; 2304]).unwrap();
        let spectrum = layer3_long_block_spectrum(&silent, 0, 0).unwrap();
        assert_eq!(spectrum.len(), 576);
        assert!(spectrum.iter().all(|line| *line == 0.0));

        let tone = AudioBuffer::new(
            44_100,
            1,
            (0..4096).map(|n| (n as f32 * 0.05).sin() * 0.4).collect(),
        )
        .unwrap();
        let spectrum = layer3_long_block_spectrum(&tone, 0, 1152).unwrap();
        assert_eq!(spectrum.len(), 576);
        assert!(spectrum.iter().any(|line| line.abs() > 0.0));
    }

    #[test]
    fn quantizes_long_block_for_layer3_analysis() {
        let mut samples = [0.0_f32; 36];
        samples[0] = 1.0;

        let quantized = quantize_long_block(&samples, 0.001).unwrap();

        assert_eq!(quantized.len(), 18);
        assert!(quantized.iter().any(|coeff| *coeff != 0));
        assert_eq!(quantize_long_block(&[0.0; 36], 1.0).unwrap(), vec![0; 18]);
        assert!(quantize_long_block(&samples, 0.0).is_err());
    }

    #[test]
    fn quantizes_pcm_long_block_for_layer3_analysis() {
        let pcm = AudioBuffer::new(44_100, 2, vec![1.0, -1.0, 0.0, 0.0]).unwrap();

        let left = quantize_pcm_long_block(&pcm, 0, 0, 0.001).unwrap();
        let right = quantize_pcm_long_block(&pcm, 1, 0, 0.001).unwrap();
        let padded = quantize_pcm_long_block(&pcm, 0, 10, 1.0).unwrap();

        assert_eq!(left.len(), 576);
        assert_eq!(right.len(), 576);
        assert_ne!(left, right);
        assert_eq!(padded, vec![0; 576]);
        assert!(quantize_pcm_long_block(&pcm, 2, 0, 1.0).is_err());
    }

    #[test]
    fn quantizes_mono_with_polyphase_and_stereo_with_compatibility_filterbank() {
        let mono = AudioBuffer::new(
            44_100,
            1,
            (0..2304)
                .map(|sample| ((sample as f32) * 0.017).sin() * 0.35)
                .collect(),
        )
        .unwrap();
        let mono_spectrum = layer3_long_block_spectrum(&mono, 0, 576).unwrap();
        let inverted: Vec<f32> = mono_spectrum.into_iter().map(|line| -line).collect();
        let expected_mono = quantize_spectrum(&inverted, 0.01, 8191).unwrap();

        assert_eq!(
            quantize_pcm_long_block(&mono, 0, 576, 0.01).unwrap(),
            expected_mono
        );

        // Stereo stays on the compatibility subband path until the real
        // polyphase stereo path passes the FFmpeg readiness oracle.
        let stereo = AudioBuffer::new(
            44_100,
            2,
            (0..2304)
                .flat_map(|sample| {
                    [
                        ((sample as f32) * 0.013).sin() * 0.30,
                        ((sample as f32) * 0.021).cos() * 0.20,
                    ]
                })
                .collect(),
        )
        .unwrap();
        for channel in 0..2 {
            let mut expected = Vec::with_capacity(576);
            for subband in 0..32 {
                let block = layer3_analysis_subband_block(&stereo, channel, 576, subband).unwrap();
                expected.extend(quantize_long_block(&block, 0.01).unwrap());
            }
            assert_eq!(
                quantize_pcm_long_block(&stereo, channel, 576, 0.01).unwrap(),
                expected
            );
        }
        // The two channels carry distinct signals, so their spectra differ.
        assert_ne!(
            quantize_pcm_long_block(&stereo, 0, 576, 0.01).unwrap(),
            quantize_pcm_long_block(&stereo, 1, 576, 0.01).unwrap(),
        );
    }

    #[test]
    fn plans_layer3_spectral_regions() {
        let all_zero = plan_spectral_regions(&[0; 18]).unwrap();
        assert_eq!(
            all_zero,
            Layer3SpectralRegions {
                big_values: 0,
                count1: 0,
                rzero: 18,
            }
        );

        let mixed = plan_spectral_regions(&[3, -2, 0, 0, 1, -1, 0, 1, 0, 0]).unwrap();
        assert_eq!(
            mixed,
            Layer3SpectralRegions {
                big_values: 2,
                count1: 1,
                rzero: 2,
            }
        );

        let count1_only = plan_spectral_regions(&[1, -1, 0, 1, 0, 0, 0, 0]).unwrap();
        assert_eq!(
            count1_only,
            Layer3SpectralRegions {
                big_values: 0,
                count1: 1,
                rzero: 4,
            }
        );
        assert!(plan_spectral_regions(&[]).is_err());
        assert!(plan_spectral_regions(&[8192]).is_err());
    }

    #[test]
    fn extracts_layer3_big_value_pairs() {
        let quantized = [3, -2, 0, 0, 1, -1, 0, 1, 0, 0];
        let regions = plan_spectral_regions(&quantized).unwrap();

        assert_eq!(
            big_value_pairs(&quantized, regions).unwrap(),
            vec![
                Layer3BigValuePair::new(3, -2),
                Layer3BigValuePair::new(0, 0),
            ]
        );
        assert_eq!(
            big_value_pairs(
                &[0, 0, 0, 0],
                Layer3SpectralRegions {
                    big_values: 0,
                    count1: 0,
                    rzero: 4,
                },
            )
            .unwrap(),
            Vec::<Layer3BigValuePair>::new()
        );
        assert!(big_value_pairs(
            &[1, 2],
            Layer3SpectralRegions {
                big_values: 2,
                count1: 0,
                rzero: 0,
            },
        )
        .is_err());
    }

    #[test]
    fn selects_layer3_big_value_table_class() {
        assert_eq!(
            select_big_value_table(&[]).unwrap(),
            Layer3BigValueTableSelection {
                table_select: 0,
                linbits: 0,
                max_magnitude: 0,
            }
        );
        assert_eq!(
            select_big_value_table(&[Layer3BigValuePair::new(1, -1)]).unwrap(),
            Layer3BigValueTableSelection {
                table_select: 1,
                linbits: 0,
                max_magnitude: 1,
            }
        );
        assert_eq!(
            select_big_value_table(&[Layer3BigValuePair::new(3, -2)]).unwrap(),
            Layer3BigValueTableSelection {
                table_select: 5,
                linbits: 0,
                max_magnitude: 3,
            }
        );
        // Tables 16..=23 share table 16's codewords but carry fixed linbits
        // widths; the decoder reads linbits from table_select, so magnitude 18
        // (needs linbits 2) must emit table 17, not table 16 with a free width.
        assert_eq!(
            select_big_value_table(&[Layer3BigValuePair::new(18, -15)]).unwrap(),
            Layer3BigValueTableSelection {
                table_select: 17,
                linbits: 2,
                max_magnitude: 18,
            }
        );
        assert_eq!(
            select_big_value_table(&[Layer3BigValuePair::new(8191, 0)]).unwrap(),
            Layer3BigValueTableSelection {
                table_select: 23,
                linbits: 13,
                max_magnitude: 8191,
            }
        );

        let mut granule = Layer3GranuleChannelInfo {
            big_values: 4,
            ..Default::default()
        };
        apply_big_value_table_to_granule(
            &mut granule,
            Layer3BigValueTableSelection {
                table_select: 16,
                linbits: 4,
                max_magnitude: 20,
            },
        );
        assert_eq!(granule.table_select, [16, 16, 16]);

        granule.big_values = 0;
        apply_big_value_table_to_granule(
            &mut granule,
            Layer3BigValueTableSelection {
                table_select: 1,
                linbits: 0,
                max_magnitude: 1,
            },
        );
        assert_eq!(granule.table_select, [0, 0, 0]);
    }

    #[test]
    fn selects_layer3_big_value_tables_per_region() {
        let pairs = [
            Layer3BigValuePair::new(1, 0),
            Layer3BigValuePair::new(0, -1),
            Layer3BigValuePair::new(3, -2),
            Layer3BigValuePair::new(5, 4),
        ];

        assert_eq!(
            select_big_value_region_tables(&pairs, 2, 1).unwrap(),
            Layer3BigValueRegionTableSelection {
                regions: [
                    Layer3BigValueTableSelection {
                        table_select: 1,
                        linbits: 0,
                        max_magnitude: 1,
                    },
                    Layer3BigValueTableSelection {
                        table_select: 5,
                        linbits: 0,
                        max_magnitude: 3,
                    },
                    Layer3BigValueTableSelection {
                        table_select: 7,
                        linbits: 0,
                        max_magnitude: 5,
                    },
                ],
                region0_pairs: 2,
                region1_pairs: 1,
            }
        );

        let err = select_big_value_region_tables(&pairs, 3, 2).unwrap_err();
        assert!(matches!(
            err,
            Error::InvalidInput("MP3 big-values region exceeds spectrum length")
        ));
    }

    #[test]
    fn selects_layer3_big_value_table_by_bit_cost() {
        let pairs = [Layer3BigValuePair::new(1, 0)];
        let table_1 = [HuffmanEntry {
            symbol: Layer3BigValueMagnitude::new(1, 0),
            code: HuffmanCode::new(0b1111, 4).unwrap(),
        }];
        let table_5 = [HuffmanEntry {
            symbol: Layer3BigValueMagnitude::new(1, 0),
            code: HuffmanCode::new(0b0, 1).unwrap(),
        }];

        assert_eq!(
            select_big_value_table_by_bit_cost(
                &pairs,
                Layer3EntropyTableProvider {
                    big_value_table_1: &table_1,
                    big_value_table_5: &table_5,
                    ..Default::default()
                },
            )
            .unwrap(),
            Layer3BigValueTableSelection {
                table_select: 5,
                linbits: 0,
                max_magnitude: 1,
            }
        );
        assert_eq!(
            select_big_value_table_by_bit_cost(
                &[Layer3BigValuePair::new(0, 0)],
                Default::default()
            )
            .unwrap(),
            Layer3BigValueTableSelection {
                table_select: 0,
                linbits: 0,
                max_magnitude: 0,
            }
        );
        let err = select_big_value_table_by_bit_cost(&pairs, Default::default()).unwrap_err();
        assert!(matches!(
            err,
            Error::UnsupportedFeature("MP3 big-values Huffman table")
        ));
    }

    #[test]
    fn extracts_layer3_count1_quads() {
        let quantized = [3, -2, 0, 0, 1, -1, 0, 1, 0, 0];
        let regions = plan_spectral_regions(&quantized).unwrap();

        assert_eq!(
            count1_quads(&quantized, regions).unwrap(),
            vec![Layer3Count1Quad::new(1, -1, 0, 1)]
        );
        assert_eq!(
            count1_quads(
                &[0, 0, 0, 0],
                Layer3SpectralRegions {
                    big_values: 0,
                    count1: 0,
                    rzero: 4,
                },
            )
            .unwrap(),
            Vec::<Layer3Count1Quad>::new()
        );
        assert!(count1_quads(
            &[1, 2, 0, 0],
            Layer3SpectralRegions {
                big_values: 0,
                count1: 1,
                rzero: 0,
            },
        )
        .is_err());
        assert!(count1_quads(
            &[1, 0],
            Layer3SpectralRegions {
                big_values: 0,
                count1: 1,
                rzero: 0,
            },
        )
        .is_err());
    }

    #[test]
    fn selects_layer3_count1_table_class() {
        assert_eq!(
            select_count1_table(&[]).unwrap(),
            Layer3Count1TableSelection {
                table_select: false,
                max_nonzero_values: 0,
            }
        );
        assert_eq!(
            select_count1_table(&[Layer3Count1Quad::new(1, 0, -1, 0)]).unwrap(),
            Layer3Count1TableSelection {
                table_select: false,
                max_nonzero_values: 2,
            }
        );
        assert_eq!(
            select_count1_table(&[
                Layer3Count1Quad::new(1, -1, 0, 1),
                Layer3Count1Quad::new(0, 0, 0, 0),
            ])
            .unwrap(),
            Layer3Count1TableSelection {
                table_select: true,
                max_nonzero_values: 3,
            }
        );
        assert!(select_count1_table(&[Layer3Count1Quad::new(2, 0, 0, 0)]).is_err());

        let mut granule = Layer3GranuleChannelInfo::default();
        apply_count1_table_to_granule(
            &mut granule,
            Layer3Count1TableSelection {
                table_select: true,
                max_nonzero_values: 4,
            },
        );
        assert!(granule.count1table_select);
    }

    #[test]
    fn selects_layer3_count1_table_by_bit_cost() {
        let quads = [Layer3Count1Quad::new(1, -1, 0, 1)];
        let table_0 = [HuffmanEntry {
            symbol: Layer3Count1MagnitudeQuad::new(1, 1, 0, 1),
            code: HuffmanCode::new(0b1111, 4).unwrap(),
        }];
        let table_1 = [HuffmanEntry {
            symbol: Layer3Count1MagnitudeQuad::new(1, 1, 0, 1),
            code: HuffmanCode::new(0b0, 1).unwrap(),
        }];

        assert_eq!(
            select_count1_table_by_bit_cost(
                &quads,
                Layer3EntropyTableProvider {
                    count1_table_0: &table_0,
                    count1_table_1: &table_1,
                    ..Default::default()
                },
            )
            .unwrap(),
            Layer3Count1TableSelection {
                table_select: true,
                max_nonzero_values: 3,
            }
        );
        assert_eq!(
            select_count1_table_by_bit_cost(
                &[Layer3Count1Quad::new(0, 0, 0, 0)],
                Default::default()
            )
            .unwrap(),
            Layer3Count1TableSelection {
                table_select: false,
                max_nonzero_values: 0,
            }
        );
        let err = select_count1_table_by_bit_cost(&quads, Default::default()).unwrap_err();
        assert!(matches!(
            err,
            Error::UnsupportedFeature("MP3 count1 Huffman table")
        ));
    }

    #[test]
    fn applies_spectral_regions_to_side_info_granule() {
        let header = FrameHeader::parse(&[0xff, 0xfb, 0x90, 0xc0]).unwrap();
        let mut side_info = Layer3SideInfo::silent(&header);
        let silent = side_info.pack(&header).unwrap();

        apply_spectral_regions_to_granule(
            &mut side_info.granules[0][0],
            Layer3SpectralRegions {
                big_values: 9,
                count1: 2,
                rzero: 12,
            },
        )
        .unwrap();

        let granule = side_info.granules[0][0];
        assert_eq!(granule.big_values, 9);
        assert_eq!(granule.table_select, [1, 1, 0]);
        // Region addresses are fixed at the rate-independent low scalefactor
        // bands so the packer's pair split matches the decoder's interpretation.
        assert_eq!(granule.region0_count, 0);
        assert_eq!(granule.region1_count, 0);
        assert!(granule.count1table_select);
        assert_ne!(side_info.pack(&header).unwrap(), silent);

        let mut empty = Layer3GranuleChannelInfo::default();
        apply_spectral_regions_to_granule(
            &mut empty,
            Layer3SpectralRegions {
                big_values: 0,
                count1: 0,
                rzero: 18,
            },
        )
        .unwrap();
        assert_eq!(empty.table_select, [0; 3]);
        assert!(!empty.count1table_select);

        let err = apply_spectral_regions_to_granule(
            &mut empty,
            Layer3SpectralRegions {
                big_values: 289,
                count1: 0,
                rzero: 0,
            },
        )
        .unwrap_err();
        assert!(matches!(
            err,
            Error::InvalidInput("MP3 big_values exceeds side-info range")
        ));
    }

    #[test]
    fn packs_mp3_main_data_codewords() {
        let codes = [
            HuffmanCode::new(0b11, 2).unwrap(),
            HuffmanCode::new(0b001, 3).unwrap(),
            HuffmanCode::new(0b0, 1).unwrap(),
        ];

        assert_eq!(pack_main_data_codewords(&codes).unwrap(), &[0b1100_1000]);
        assert_eq!(
            pack_main_data_codewords_with_len(&codes).unwrap(),
            PackedBits {
                bytes: vec![0b1100_1000],
                bit_len: 6,
            }
        );

        let mut granule = Layer3GranuleChannelInfo::default();
        let packed = pack_main_data_codewords_for_granule(&mut granule, &codes).unwrap();
        assert_eq!(packed.bit_len, 6);
        assert_eq!(granule.part2_3_length, 6);

        let err =
            apply_part2_3_length_to_granule(&mut granule, usize::from(u16::MAX) + 1).unwrap_err();
        assert!(matches!(
            err,
            Error::InvalidInput("MP3 part2_3_length exceeds side-info range")
        ));
    }

    #[test]
    fn packs_mp3_main_data_regions_for_granule() {
        let big_values = PackedBits {
            bytes: vec![0b1010_0000],
            bit_len: 3,
        };
        let count1 = PackedBits {
            bytes: vec![0b1100_0000],
            bit_len: 2,
        };

        assert_eq!(
            pack_main_data_regions(big_values.clone(), count1.clone()).unwrap(),
            PackedBits {
                bytes: vec![0b1011_1000],
                bit_len: 5,
            }
        );

        let mut granule = Layer3GranuleChannelInfo::default();
        let packed = pack_main_data_regions_for_granule(&mut granule, big_values, count1).unwrap();
        assert_eq!(packed.bit_len, 5);
        assert_eq!(granule.part2_3_length, 5);
        assert!(pack_main_data_regions(
            PackedBits {
                bytes: vec![0],
                bit_len: 9,
            },
            PackedBits {
                bytes: vec![],
                bit_len: 0,
            },
        )
        .is_err());
    }

    #[test]
    fn packs_mp3_main_data_parts_for_granule() {
        let scale_factors = PackedBits {
            bytes: vec![0b1100_0000],
            bit_len: 2,
        };
        let big_values = PackedBits {
            bytes: vec![0b1010_0000],
            bit_len: 3,
        };
        let count1 = PackedBits {
            bytes: vec![0b0100_0000],
            bit_len: 2,
        };

        assert_eq!(
            pack_main_data_parts(scale_factors.clone(), big_values.clone(), count1.clone())
                .unwrap(),
            PackedBits {
                bytes: vec![0b1110_1010],
                bit_len: 7,
            }
        );

        let mut granule = Layer3GranuleChannelInfo::default();
        let packed =
            pack_main_data_parts_for_granule(&mut granule, scale_factors, big_values, count1)
                .unwrap();
        assert_eq!(packed.bit_len, 7);
        assert_eq!(granule.part2_3_length, 7);
        assert!(pack_main_data_parts(
            PackedBits {
                bytes: vec![0],
                bit_len: 9,
            },
            PackedBits {
                bytes: vec![],
                bit_len: 0,
            },
            PackedBits {
                bytes: vec![],
                bit_len: 0,
            },
        )
        .is_err());
    }

    #[test]
    fn packs_mpeg1_layer3_long_scale_factors_for_granule() {
        let mut scale_factors = [0_u8; MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT];
        scale_factors[0] = 3;
        scale_factors[10] = 2;
        scale_factors[11] = 1;
        scale_factors[20] = 1;

        let selection = select_mpeg1_layer3_long_scale_factor_compress(&scale_factors).unwrap();
        assert_eq!(
            selection,
            Layer3ScaleFactorCompress {
                scalefac_compress: 8,
                slen1: 2,
                slen2: 1,
            }
        );
        assert_eq!(
            pack_mpeg1_layer3_long_scale_factors(&scale_factors, selection).unwrap(),
            PackedBits {
                bytes: vec![0b1100_0000, 0b0000_0000, 0b0000_1010, 0b0000_0001],
                bit_len: 32,
            }
        );

        let mut granule = Layer3GranuleChannelInfo::default();
        let packed =
            pack_mpeg1_layer3_long_scale_factors_for_granule(&mut granule, &scale_factors).unwrap();
        assert_eq!(packed.bit_len, 32);
        assert_eq!(granule.scalefac_compress, 8);

        apply_scale_factor_compress_to_granule(
            &mut granule,
            Layer3ScaleFactorCompress {
                scalefac_compress: 15,
                slen1: 4,
                slen2: 3,
            },
        );
        assert_eq!(granule.scalefac_compress, 15);
    }

    #[test]
    fn packs_zero_width_mpeg1_layer3_long_scale_factors() {
        let scale_factors = [0_u8; MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT];
        let mut granule = Layer3GranuleChannelInfo::default();

        let packed =
            pack_mpeg1_layer3_long_scale_factors_for_granule(&mut granule, &scale_factors).unwrap();

        assert_eq!(
            packed,
            PackedBits {
                bytes: vec![],
                bit_len: 0,
            }
        );
        assert_eq!(granule.scalefac_compress, 0);
    }

    #[test]
    fn rejects_unrepresentable_mpeg1_layer3_long_scale_factors() {
        let mut scale_factors = [0_u8; MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT];
        scale_factors[11] = 8;

        assert!(select_mpeg1_layer3_long_scale_factor_compress(&scale_factors).is_err());
        assert!(pack_mpeg1_layer3_long_scale_factors(
            &scale_factors,
            Layer3ScaleFactorCompress {
                scalefac_compress: 8,
                slen1: 2,
                slen2: 1,
            },
        )
        .is_err());
        assert!(pack_mpeg1_layer3_long_scale_factors(
            &[0_u8; MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT],
            Layer3ScaleFactorCompress {
                scalefac_compress: 16,
                slen1: 4,
                slen2: 4,
            },
        )
        .is_err());
    }

    /// Mirrors the ISO/IEC 13818-3 §2.4.3.2 decoder partition derivation so
    /// tests can confirm the encoder's `scalefac_compress` reconstructs the
    /// same group sizes and bit widths a conformant decoder would compute.
    fn decode_mpeg2_lsf_long_partition(scalefac_compress: u16) -> ([u8; 4], [u8; 4]) {
        if scalefac_compress < 400 {
            let high = scalefac_compress >> 4;
            let slen = [
                (high / 5) as u8,
                (high % 5) as u8,
                ((scalefac_compress & 0xf) >> 2) as u8,
                (scalefac_compress & 0x3) as u8,
            ];
            ([6, 5, 5, 5], slen)
        } else {
            let t = scalefac_compress - 400;
            let high = t >> 2;
            let slen = [(high / 5) as u8, (high % 5) as u8, (t & 0x3) as u8, 0];
            ([6, 5, 7, 3], slen)
        }
    }

