    use super::{
        aac_escape_table, aac_lc_adts_max_frame_len_for_bitrate,
        aac_lc_default_production_bitrate_bps, aac_lc_long_window_scale_factor_band_offsets,
        aac_lc_standard_signed_pair_tables, aac_lc_standard_signed_quad_tables,
        aac_lc_standard_spectral_tables, aac_lc_standard_unsigned_quad_tables,
        aac_scale_factor_delta_table, aac_scale_factor_delta_zero_table, aac_signed_pairs5_table,
        aac_signed_pairs6_table, aac_signed_quads1_table, aac_signed_quads2_table,
        aac_unit_codebook6_spectral_tables, aac_unit_quad_spectral_tables,
        aac_unsigned_pairs10_table, aac_unsigned_pairs7_table,
        aac_unsigned_pairs7_unit_magnitude_spectral_tables, aac_unsigned_pairs8_table,
        aac_unsigned_pairs9_table, aac_unsigned_quads3_table, aac_unsigned_quads4_table, encode,
        encode_pcm_mono_long_block_adts, encode_pcm_mono_long_block_adts_by_bit_cost,
        encode_pcm_mono_long_block_adts_stream, encode_pcm_mono_long_block_adts_stream_by_bit_cost,
        encode_pcm_mono_long_block_adts_stream_with_auto_step_by_bit_cost,
        encode_pcm_mono_long_block_adts_stream_with_offsets_and_auto_step_by_bit_cost,
        encode_pcm_mono_long_block_adts_stream_with_offsets_and_scale_factors_and_bitrate_by_bit_cost,
        encode_pcm_mono_long_block_adts_stream_with_offsets_and_scale_factors_and_max_frame_len_by_bit_cost,
        encode_pcm_mono_long_block_adts_stream_with_offsets_and_scale_factors_by_bit_cost,
        encode_pcm_mono_long_block_adts_stream_with_offsets_and_selected_scale_factors_and_bitrate_by_bit_cost,
        encode_pcm_mono_long_block_adts_stream_with_offsets_and_selected_scale_factors_and_max_frame_len_by_bit_cost,
        encode_pcm_mono_long_block_adts_stream_with_offsets_and_selected_scale_factors_by_bit_cost,
        encode_pcm_mono_long_block_adts_stream_with_scale_factors,
        encode_pcm_mono_long_block_adts_stream_with_scale_factors_by_bit_cost,
        encode_pcm_mono_long_block_adts_stream_with_selected_scale_factors,
        encode_pcm_mono_long_block_adts_stream_with_selected_scale_factors_by_bit_cost,
        encode_pcm_mono_long_block_adts_stream_with_standard_spectral_offsets_and_scale_factors_and_bitrate_by_bit_cost,
        encode_pcm_mono_long_block_adts_stream_with_standard_spectral_offsets_and_scale_factors_and_max_frame_len_by_bit_cost,
        encode_pcm_mono_long_block_adts_stream_with_standard_spectral_offsets_and_scale_factors_by_bit_cost,
        encode_pcm_mono_long_block_adts_with_scale_factors,
        encode_pcm_mono_long_block_adts_with_scale_factors_by_bit_cost,
        encode_pcm_mono_long_block_adts_with_selected_scale_factors,
        encode_pcm_mono_long_block_adts_with_selected_scale_factors_by_bit_cost,
        encode_pcm_stereo_long_block_adts, encode_pcm_stereo_long_block_adts_by_bit_cost,
        encode_pcm_stereo_long_block_adts_stream,
        encode_pcm_stereo_long_block_adts_stream_by_bit_cost,
        encode_pcm_stereo_long_block_adts_stream_with_auto_step_by_bit_cost,
        encode_pcm_stereo_long_block_adts_stream_with_offsets_and_scale_factors_and_bitrate_by_bit_cost,
        encode_pcm_stereo_long_block_adts_stream_with_offsets_and_scale_factors_and_max_frame_len_by_bit_cost,
        encode_pcm_stereo_long_block_adts_stream_with_offsets_and_scale_factors_by_bit_cost,
        encode_pcm_stereo_long_block_adts_stream_with_offsets_and_selected_scale_factors_and_bitrate_by_bit_cost,
        encode_pcm_stereo_long_block_adts_stream_with_offsets_and_selected_scale_factors_and_max_frame_len_by_bit_cost,
        encode_pcm_stereo_long_block_adts_stream_with_offsets_and_selected_scale_factors_by_bit_cost,
        encode_pcm_stereo_long_block_adts_stream_with_scale_factors,
        encode_pcm_stereo_long_block_adts_stream_with_scale_factors_by_bit_cost,
        encode_pcm_stereo_long_block_adts_stream_with_selected_scale_factors,
        encode_pcm_stereo_long_block_adts_stream_with_selected_scale_factors_by_bit_cost,
        encode_pcm_stereo_long_block_adts_stream_with_standard_spectral_offsets_and_scale_factors_and_bitrate_by_bit_cost,
        encode_pcm_stereo_long_block_adts_stream_with_standard_spectral_offsets_and_scale_factors_and_max_frame_len_by_bit_cost,
        encode_pcm_stereo_long_block_adts_stream_with_standard_spectral_offsets_and_scale_factors_by_bit_cost,
        encode_pcm_stereo_long_block_adts_with_scale_factors,
        encode_pcm_stereo_long_block_adts_with_scale_factors_by_bit_cost,
        encode_pcm_stereo_long_block_adts_with_selected_scale_factors,
        encode_pcm_stereo_long_block_adts_with_selected_scale_factors_by_bit_cost,
        encode_quantized_mono_adts, encode_quantized_mono_adts_by_bit_cost,
        encode_quantized_mono_adts_with_scale_factors,
        encode_quantized_mono_adts_with_scale_factors_by_bit_cost,
        encode_quantized_mono_adts_with_selected_scale_factors,
        encode_quantized_mono_adts_with_selected_scale_factors_by_bit_cost,
        encode_quantized_mono_adts_with_standard_spectral_offsets_and_scale_factors_by_bit_cost,
        encode_quantized_stereo_adts, encode_quantized_stereo_adts_by_bit_cost,
        encode_quantized_stereo_adts_with_scale_factors,
        encode_quantized_stereo_adts_with_scale_factors_by_bit_cost,
        encode_quantized_stereo_adts_with_selected_scale_factors,
        encode_quantized_stereo_adts_with_selected_scale_factors_by_bit_cost,
        encode_quantized_stereo_adts_with_standard_spectral_offsets_and_scale_factors_by_bit_cost,
        experimental_aac_scale_factor_delta_table, experimental_unit_magnitude_spectral_tables,
        frame_adts, frame_adts_stream, mdct_long_block, mux_adts_as_m4a,
        pack_aac_lc_standard_spectral_payload_with_offsets_and_sign_bits_and_scale_factor_bits_by_bit_cost,
        pack_aac_lc_standard_spectral_payload_with_offsets_and_sign_bits_by_bit_cost,
        pack_aac_lc_standard_spectral_payload_with_sign_bits_and_scale_factor_bits_by_bit_cost,
        pack_aac_lc_standard_spectral_payload_with_sign_bits_by_bit_cost,
        pack_channel_pair_raw_data_block, pack_channel_pair_raw_data_block_parts,
        pack_channel_payload_parts, pack_long_block_individual_channel_stream,
        pack_quad_section_data_with_len, pack_scale_factor_deltas_with_table, pack_section_data,
        pack_section_data_with_len, pack_section_data_with_offsets,
        pack_sectioned_spectral_payload,
        pack_sectioned_spectral_payload_by_codebook_id_with_sign_bits,
        pack_sectioned_spectral_payload_by_codebook_id_with_sign_bits_and_scale_factor_bits,
        pack_sectioned_spectral_payload_by_codebook_id_with_sign_bits_and_scale_factor_bits_by_bit_cost,
        pack_sectioned_spectral_payload_by_codebook_id_with_sign_bits_by_bit_cost,
        pack_sectioned_spectral_payload_with_scale_factor_bits,
        pack_sectioned_spectral_payload_with_sign_bits,
        pack_sectioned_spectral_payload_with_sign_bits_and_scale_factor_bits,
        pack_sectioned_spectral_payload_with_sign_bits_and_scale_factor_bits_by_bit_cost,
        pack_sectioned_spectral_payload_with_sign_bits_by_bit_cost,
        pack_sectioned_spectral_quad_payload_with_sign_bits,
        pack_sectioned_spectral_quad_payload_with_sign_bits_by_bit_cost,
        pack_single_channel_raw_data_block, pack_single_channel_raw_data_block_parts,
        pack_spectral_codewords, pack_spectral_codewords_with_len,
        pack_spectral_pairs_with_sign_bits, pack_spectral_pairs_with_table,
        pack_spectral_quad_sections_with_sign_bits, pack_spectral_quads_with_sign_bits,
        pack_spectral_quads_with_table, pack_spectral_section_data_with_len,
        pack_spectral_section_data_with_offsets, pack_spectral_sections,
        pack_spectral_sections_by_codebook_id_with_sign_bits,
        pack_spectral_sections_with_sign_bits, parse_adts_frame,
        plan_aac_lc_standard_spectral_sections_by_bit_cost,
        plan_aac_lc_standard_spectral_sections_by_offsets_by_bit_cost,
        plan_quad_sections_by_bit_cost, plan_scale_factor_deltas,
        plan_scale_factor_deltas_by_offsets, plan_sections, plan_sections_by_bit_cost,
        plan_sections_by_offsets, plan_spectral_scale_factor_deltas_by_offsets,
        plan_spectral_sections_by_bit_cost, plan_spectral_sections_by_offsets_by_bit_cost,
        quantize_long_block, quantize_pcm_long_block,
        select_aac_lc_mono_pcm_frame_step_by_bit_cost,
        select_aac_lc_mono_pcm_frame_step_details_by_bit_cost,
        select_aac_lc_mono_pcm_frame_step_details_with_max_frame_len_by_bit_cost,
        select_aac_lc_mono_pcm_frame_step_details_with_offsets_and_max_frame_len_by_bit_cost,
        select_aac_lc_mono_pcm_frame_step_details_with_offsets_and_scale_factors_and_max_frame_len_by_bit_cost,
        select_aac_lc_mono_pcm_frame_step_details_with_offsets_and_scale_factors_by_bit_cost,
        select_aac_lc_mono_pcm_frame_step_details_with_offsets_by_bit_cost,
        select_aac_lc_mono_pcm_frame_step_with_max_frame_len_by_bit_cost,
        select_aac_lc_mono_pcm_frame_step_with_offsets_and_max_frame_len_by_bit_cost,
        select_aac_lc_mono_pcm_frame_step_with_offsets_and_scale_factors_and_max_frame_len_by_bit_cost,
        select_aac_lc_mono_pcm_stream_frame_details_with_standard_spectral_offsets_and_scale_factors_and_bitrate_by_bit_cost,
        select_aac_lc_stereo_pcm_frame_step_by_bit_cost,
        select_aac_lc_stereo_pcm_frame_step_details_by_bit_cost,
        select_aac_lc_stereo_pcm_frame_step_details_with_max_frame_len_by_bit_cost,
        select_aac_lc_stereo_pcm_frame_step_details_with_offsets_and_max_frame_len_by_bit_cost,
        select_aac_lc_stereo_pcm_frame_step_details_with_offsets_and_scale_factors_and_max_frame_len_by_bit_cost,
        select_aac_lc_stereo_pcm_frame_step_details_with_offsets_and_scale_factors_by_bit_cost,
        select_aac_lc_stereo_pcm_frame_step_details_with_offsets_by_bit_cost,
        select_aac_lc_stereo_pcm_frame_step_with_max_frame_len_by_bit_cost,
        select_aac_lc_stereo_pcm_frame_step_with_offsets_and_max_frame_len_by_bit_cost,
        select_aac_lc_stereo_pcm_frame_step_with_offsets_and_scale_factors_and_max_frame_len_by_bit_cost,
        select_aac_lc_stereo_pcm_frame_step_with_offsets_by_bit_cost,
        select_aac_lc_stereo_pcm_stream_frame_details_with_standard_spectral_offsets_and_scale_factors_and_bitrate_by_bit_cost,
        select_codebook_by_bit_cost, select_quad_codebook_by_bit_cost,
        select_scale_factors_for_quantized_bands,
        select_scale_factors_for_quantized_bands_by_offsets,
        select_scale_factors_for_quantized_bands_by_offsets_with_magnitude_bias,
        select_spectral_codebook_id_by_bit_cost, spectral_pairs_for_section,
        spectral_quads_for_section,
        split_aac_lc_standard_spectral_payload_with_offsets_and_sign_bits_and_scale_factor_bits_by_bit_cost,
        split_aac_lc_standard_spectral_payload_with_offsets_and_sign_bits_by_bit_cost,
        split_aac_lc_standard_spectral_payload_with_sign_bits_and_scale_factor_bits_by_bit_cost,
        split_aac_lc_standard_spectral_payload_with_sign_bits_by_bit_cost,
        split_sectioned_spectral_payload_by_codebook_id_with_sign_bits,
        split_sectioned_spectral_payload_by_codebook_id_with_sign_bits_and_scale_factor_bits,
        split_sectioned_spectral_payload_by_codebook_id_with_sign_bits_and_scale_factor_bits_by_bit_cost,
        split_sectioned_spectral_payload_by_codebook_id_with_sign_bits_by_bit_cost,
        split_sectioned_spectral_payload_with_sign_bits, AacCodebook, AacLongBlockConfig,
        AacPcmFrameStepSelection, AacPcmLongBlockConfig, AacPcmStepSearchConfig, AacQuadSection,
        AacQuantizedChannel, AacQuantizedSpectrum, AacScaleFactorChannel, AacScaleFactorDelta,
        AacScaleFactorSequence, AacSection, AacSpectralMagnitudePair, AacSpectralMagnitudeQuad,
        AacSpectralMagnitudeQuadTables, AacSpectralMagnitudeTables, AacSpectralPair,
        AacSpectralQuad, AacSpectralSection, AacSpectralTables, AdtsConfig, BitWriter,
        AAC_ADTS_HEADER_LEN, AAC_LC_16K_LONG_WINDOW_SCALE_FACTOR_BAND_OFFSETS,
        AAC_LC_24K_LONG_WINDOW_SCALE_FACTOR_BAND_OFFSETS,
        AAC_LC_32K_LONG_WINDOW_SCALE_FACTOR_BAND_OFFSETS,
        AAC_LC_48K_LONG_WINDOW_SCALE_FACTOR_BAND_OFFSETS,
        AAC_LC_64K_LONG_WINDOW_SCALE_FACTOR_BAND_OFFSETS,
        AAC_LC_8K_LONG_WINDOW_SCALE_FACTOR_BAND_OFFSETS,
        AAC_LC_96K_LONG_WINDOW_SCALE_FACTOR_BAND_OFFSETS, AAC_LC_PCM_STEP_CANDIDATES,
    };
    use sc_core::Error;
    use sc_core::{AudioBuffer, HuffmanCode, HuffmanEntry, PackedBits};

    fn max_adts_frame_len(stream: &[u8]) -> usize {
        let mut remaining = stream;
        let mut max_frame_len = 0;
        while !remaining.is_empty() {
            let frame = parse_adts_frame(remaining).unwrap();
            max_frame_len = max_frame_len.max(frame.frame_len);
            remaining = &remaining[frame.frame_len..];
        }
        max_frame_len
    }

    #[test]
    fn frames_raw_access_unit_as_adts() {
        let frame = frame_adts(AdtsConfig::aac_lc(44_100, 2), &[0x11, 0x22]).unwrap();

        assert_eq!(&frame[..7], &[0xff, 0xf1, 0x50, 0x80, 0x01, 0x3f, 0xfc]);
        assert_eq!(&frame[7..], &[0x11, 0x22]);
    }

    #[test]
    fn frames_multiple_access_units_as_adts_stream() {
        let stream = frame_adts_stream(
            AdtsConfig::aac_lc(48_000, 1),
            [&[0xaa][..], &[0xbb, 0xcc][..]],
        )
        .unwrap();

        assert_eq!(stream[0], 0xff);
        assert_eq!(stream[8], 0xff);
        assert_eq!(stream.len(), 17);
    }

    #[test]
    fn muxes_adts_via_mp4_module() {
        let adts = frame_adts(AdtsConfig::aac_lc(44_100, 2), &[0x11, 0x22]).unwrap();
        let m4a = mux_adts_as_m4a(&adts).unwrap();

        assert_eq!(&m4a[4..8], b"ftyp");
        assert!(m4a.windows(4).any(|window| window == b"mdat"));
        assert!(m4a.windows(4).any(|window| window == b"moov"));
    }

    #[test]
    fn computes_long_block_mdct_for_aac_analysis() {
        let mut samples = [0.0_f32; 2048];
        samples[0] = 1.0;

        let coeffs = mdct_long_block(&samples).unwrap();

        assert_eq!(coeffs.len(), 1024);
        assert!(coeffs.iter().any(|coeff| coeff.abs() > 0.0));
        assert_eq!(mdct_long_block(&[0.0; 2048]).unwrap(), vec![0.0; 1024]);
    }

    #[test]
    fn quantizes_long_block_for_aac_analysis() {
        let mut samples = [0.0_f32; 2048];
        samples[0] = 1.0;

        let quantized = quantize_long_block(&samples, 0.001).unwrap();

        assert_eq!(quantized.len(), 1024);
        assert!(quantized.iter().any(|coeff| *coeff != 0));
        assert_eq!(
            quantize_long_block(&[0.0; 2048], 1.0).unwrap(),
            vec![0; 1024]
        );
        assert!(quantize_long_block(&samples, 0.0).is_err());
    }

    #[test]
    fn quantizes_pcm_long_block_for_aac_analysis() {
        let pcm = AudioBuffer::new(44_100, 2, vec![1.0, -1.0, 0.0, 0.0]).unwrap();

        let left = quantize_pcm_long_block(&pcm, 0, 0, 0.001).unwrap();
        let right = quantize_pcm_long_block(&pcm, 1, 0, 0.001).unwrap();
        let padded = quantize_pcm_long_block(&pcm, 0, 10, 1.0).unwrap();

        assert_eq!(left.len(), 1024);
        assert_eq!(right.len(), 1024);
        assert_ne!(left, right);
        assert_eq!(padded, vec![0; 1024]);
        assert!(quantize_pcm_long_block(&pcm, 2, 0, 1.0).is_err());
    }

    #[test]
    fn plans_aac_codebook_sections() {
        let quantized = [0, 0, 0, 0, 1, -1, 0, 1, 3, -4, 0, 2, 9, 0, -5, 1];

        let sections = plan_sections(&quantized, 4).unwrap();

        assert_eq!(
            sections,
            vec![
                AacSection {
                    start: 0,
                    end: 4,
                    codebook: AacCodebook::Zero,
                },
                AacSection {
                    start: 4,
                    end: 12,
                    codebook: AacCodebook::UnsignedPairs7,
                },
                AacSection {
                    start: 12,
                    end: 16,
                    codebook: AacCodebook::UnsignedPairs9,
                },
            ]
        );
        assert_eq!(AacCodebook::Escape.id(), 11);
        assert!(plan_sections(&quantized, 0).is_err());
        assert!(plan_sections(&quantized[..15], 4).is_err());
        assert!(plan_sections(&[8192], 1).is_err());
    }

    #[test]
    fn default_aac_section_planner_uses_available_standard_unsigned_pair_tables() {
        let quantized = [2, -7, 0, 1, 8, -12, 0, 0, 13, 0, 0, 0];
        let sections = plan_sections(&quantized, 4).unwrap();

        assert_eq!(
            sections,
            vec![
                AacSection {
                    start: 0,
                    end: 4,
                    codebook: AacCodebook::UnsignedPairs7,
                },
                AacSection {
                    start: 4,
                    end: 8,
                    codebook: AacCodebook::UnsignedPairs9,
                },
                AacSection {
                    start: 8,
                    end: 12,
                    codebook: AacCodebook::Escape,
                },
            ]
        );
        assert!(
            pack_spectral_sections_with_sign_bits(
                &sections[..2],
                &quantized,
                AacSpectralMagnitudeTables::default(),
            )
            .unwrap()
            .bit_len
                > 0
        );
        assert!(pack_spectral_sections_with_sign_bits(
            &sections[2..],
            &quantized,
            AacSpectralMagnitudeTables::default(),
        )
        .is_err());
    }

    #[test]
    fn plans_aac_codebook_sections_by_bit_cost() {
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

        assert_eq!(
            select_codebook_by_bit_cost(&[1, -1], tables).unwrap(),
            AacCodebook::SignedPairs5
        );
        let pairs6 = [HuffmanEntry {
            symbol: AacSpectralMagnitudePair::new(1, 1),
            code: HuffmanCode::new(0b1, 1).unwrap(),
        }];
        assert_eq!(
            select_codebook_by_bit_cost(
                &[1, -1],
                AacSpectralMagnitudeTables {
                    pairs1: &pairs1,
                    pairs5: &[],
                    pairs6: &pairs6,
                    escape: &[],
                },
            )
            .unwrap(),
            AacCodebook::SignedPairs6
        );
        assert_eq!(
            select_codebook_by_bit_cost(&[0, 0], AacSpectralMagnitudeTables::default()).unwrap(),
            AacCodebook::Zero
        );
        assert_eq!(
            select_codebook_by_bit_cost(&[1, -1], AacSpectralMagnitudeTables::default()).unwrap(),
            AacCodebook::UnsignedPairs8
        );
        assert_eq!(
            select_codebook_by_bit_cost(&[2, 0], AacSpectralMagnitudeTables::default()).unwrap(),
            AacCodebook::UnsignedPairs8
        );
        assert_eq!(
            select_codebook_by_bit_cost(&[12, -12], AacSpectralMagnitudeTables::default()).unwrap(),
            AacCodebook::UnsignedPairs10
        );
        assert!(
            select_codebook_by_bit_cost(&[17, 0], AacSpectralMagnitudeTables::default()).is_err()
        );
        assert_eq!(
            select_codebook_by_bit_cost(
                &[17, 0],
                AacSpectralMagnitudeTables {
                    pairs1: &[],
                    pairs5: &[],
                    pairs6: &[],
                    escape: aac_escape_table(),
                },
            )
            .unwrap(),
            AacCodebook::Escape
        );
        assert_eq!(
            plan_sections_by_bit_cost(&[1, -1, 0, 0], 2, tables).unwrap(),
            vec![
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
            ]
        );
        let default_sections =
            plan_sections_by_bit_cost(&[1, -1, 0, 0], 2, AacSpectralMagnitudeTables::default())
                .unwrap();
        assert_eq!(
            default_sections,
            vec![
                AacSection {
                    start: 0,
                    end: 2,
                    codebook: AacCodebook::UnsignedPairs8,
                },
                AacSection {
                    start: 2,
                    end: 4,
                    codebook: AacCodebook::Zero,
                },
            ]
        );
        assert_eq!(
            pack_section_data_with_len(&default_sections, 2).unwrap(),
            PackedBits {
                bytes: vec![0b1000_0000, 0b1000_0000, 0b0100_0000],
                bit_len: 18,
            }
        );
    }

    #[test]
    fn experimental_unit_tables_pack_nonzero_sections() {
        let tables = experimental_unit_magnitude_spectral_tables();
        let quantized = [1, -1, 0, 1];

        let sections = plan_sections_by_bit_cost(&quantized, 2, tables).unwrap();
        let payload =
            pack_sectioned_spectral_payload_with_sign_bits_by_bit_cost(&quantized, 2, tables)
                .unwrap();
        let adts = encode_quantized_mono_adts_by_bit_cost(
            AdtsConfig::aac_lc(44_100, 1),
            AacLongBlockConfig::new(0, 2),
            &quantized,
            2,
            tables,
        )
        .unwrap();

        assert_eq!(
            sections,
            vec![AacSection {
                start: 0,
                end: 4,
                codebook: AacCodebook::SignedPairs1,
            }]
        );
        assert!(payload.bit_len > 0);
        assert_eq!(&adts[..2], &[0xff, 0xf1]);
        assert!(adts.len() > 7);
    }

    #[test]
    fn plans_and_packs_aac_scale_factor_deltas() {
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
        let deltas = plan_scale_factor_deltas(&sections, 2, &[7, 10, 12, 11], 9).unwrap();
        let table = [
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

        assert_eq!(
            deltas,
            vec![
                AacScaleFactorDelta::new(1),
                AacScaleFactorDelta::new(2),
                AacScaleFactorDelta::new(-1),
            ]
        );
        assert_eq!(
            pack_scale_factor_deltas_with_table(&deltas, &table).unwrap(),
            PackedBits {
                bytes: vec![0b1011_0000],
                bit_len: 6,
            }
        );
        assert!(plan_scale_factor_deltas(&sections, 0, &[7, 10, 12, 11], 9).is_err());
        assert!(plan_scale_factor_deltas(&sections, 2, &[7, 10], 9).is_err());
        assert!(
            pack_scale_factor_deltas_with_table(&[AacScaleFactorDelta::new(3)], &table).is_err()
        );
    }

    #[test]
    fn exposes_standard_aac_scale_factor_delta_table() {
        let table = aac_scale_factor_delta_table();

        assert_eq!(table.len(), 121);
        assert_eq!(table.first().unwrap().symbol, AacScaleFactorDelta::new(-60));
        assert_eq!(table.last().unwrap().symbol, AacScaleFactorDelta::new(60));
        assert_eq!(table[60].symbol, AacScaleFactorDelta::new(0));
        assert_eq!(table[60].code, HuffmanCode::new(0, 1).unwrap());
        assert_eq!(table[59].symbol, AacScaleFactorDelta::new(-1));
        assert_eq!(table[59].code, HuffmanCode::new(0b100, 3).unwrap());
        assert_eq!(table[61].symbol, AacScaleFactorDelta::new(1));
        assert_eq!(table[61].code, HuffmanCode::new(0b1010, 4).unwrap());
        assert_eq!(
            pack_scale_factor_deltas_with_table(
                &[
                    AacScaleFactorDelta::new(-1),
                    AacScaleFactorDelta::new(0),
                    AacScaleFactorDelta::new(1),
                ],
                &table,
            )
            .unwrap(),
            PackedBits {
                bytes: vec![0b1000_1010],
                bit_len: 8,
            }
        );
        assert!(
            pack_scale_factor_deltas_with_table(&[AacScaleFactorDelta::new(61)], &table).is_err()
        );
    }

    #[test]
    fn selects_aac_scale_factors_from_quantized_band_magnitudes() {
        let quantized = [0, 0, 1, -1, 3, -4, 9, 0];
        let sections = plan_sections(&quantized, 2).unwrap();

        let scale_factors = select_scale_factors_for_quantized_bands(&quantized, 2, 100).unwrap();
        let deltas = plan_scale_factor_deltas(&sections, 2, &scale_factors, 100).unwrap();

        assert_eq!(scale_factors, vec![100, 101, 103, 104]);
        assert_eq!(
            deltas,
            vec![
                AacScaleFactorDelta::new(1),
                AacScaleFactorDelta::new(2),
                AacScaleFactorDelta::new(1),
            ]
        );
        assert!(select_scale_factors_for_quantized_bands(&quantized, 0, 100).is_err());
        assert!(select_scale_factors_for_quantized_bands(&quantized[..7], 2, 100).is_err());
        assert!(select_scale_factors_for_quantized_bands(&[i32::MIN, 0], 2, 100).is_err());
    }

    #[test]
    fn plans_aac_sections_with_standard_long_window_offsets() {
        let offsets = aac_lc_long_window_scale_factor_band_offsets(44_100).unwrap();
        let mut quantized = vec![0; 1024];
        quantized[4] = 1;
        quantized[5] = -1;
        quantized[40] = 1;

        let sections = plan_sections_by_offsets(
            &quantized,
            offsets,
            experimental_unit_magnitude_spectral_tables(),
        )
        .unwrap();
        let scale_factors =
            select_scale_factors_for_quantized_bands_by_offsets(&quantized, offsets, 100).unwrap();
        let biased_scale_factors =
            select_scale_factors_for_quantized_bands_by_offsets_with_magnitude_bias(
                &quantized, offsets, 100, 2,
            )
            .unwrap();
        let deltas =
            plan_scale_factor_deltas_by_offsets(&sections, offsets, &scale_factors, 100).unwrap();
        let section_bits = pack_section_data_with_offsets(&sections, offsets).unwrap();

        assert_eq!(scale_factors[1], 101);
        assert_eq!(biased_scale_factors[0], 100);
        assert_eq!(biased_scale_factors[1], 100);

        for sample_rate in [88_200, 96_000] {
            let offsets_96k = aac_lc_long_window_scale_factor_band_offsets(sample_rate).unwrap();
            assert_eq!(
                offsets_96k,
                AAC_LC_96K_LONG_WINDOW_SCALE_FACTOR_BAND_OFFSETS
            );
            assert_eq!(offsets_96k.first().copied(), Some(0));
            assert_eq!(offsets_96k.last().copied(), Some(1024));
            assert_eq!(offsets_96k.len() - 1, 41);
        }
        let offsets_64k = aac_lc_long_window_scale_factor_band_offsets(64_000).unwrap();
        assert_eq!(
            offsets_64k,
            AAC_LC_64K_LONG_WINDOW_SCALE_FACTOR_BAND_OFFSETS
        );
        assert_eq!(offsets_64k.first().copied(), Some(0));
        assert_eq!(offsets_64k.last().copied(), Some(1024));
        assert_eq!(offsets_64k.len() - 1, 47);
        assert_eq!(offsets, AAC_LC_48K_LONG_WINDOW_SCALE_FACTOR_BAND_OFFSETS);
        assert_eq!(offsets.first().copied(), Some(0));
        assert_eq!(offsets.last().copied(), Some(1024));
        assert_eq!(offsets.len() - 1, 49);
        let offsets_32k = aac_lc_long_window_scale_factor_band_offsets(32_000).unwrap();
        assert_eq!(
            offsets_32k,
            AAC_LC_32K_LONG_WINDOW_SCALE_FACTOR_BAND_OFFSETS
        );
        assert_eq!(offsets_32k.first().copied(), Some(0));
        assert_eq!(offsets_32k.last().copied(), Some(1024));
        assert_eq!(offsets_32k.len() - 1, 51);
        for sample_rate in [22_050, 24_000] {
            let offsets_24k = aac_lc_long_window_scale_factor_band_offsets(sample_rate).unwrap();
            assert_eq!(
                offsets_24k,
                AAC_LC_24K_LONG_WINDOW_SCALE_FACTOR_BAND_OFFSETS
            );
            assert_eq!(offsets_24k.first().copied(), Some(0));
            assert_eq!(offsets_24k.last().copied(), Some(1024));
            assert_eq!(offsets_24k.len() - 1, 47);
        }
        for sample_rate in [11_025, 12_000, 16_000] {
            let offsets_16k = aac_lc_long_window_scale_factor_band_offsets(sample_rate).unwrap();
            assert_eq!(
                offsets_16k,
                AAC_LC_16K_LONG_WINDOW_SCALE_FACTOR_BAND_OFFSETS
            );
            assert_eq!(offsets_16k.first().copied(), Some(0));
            assert_eq!(offsets_16k.last().copied(), Some(1024));
            assert_eq!(offsets_16k.len() - 1, 43);
        }
        for sample_rate in [7_350, 8_000] {
            let offsets_8k = aac_lc_long_window_scale_factor_band_offsets(sample_rate).unwrap();
            assert_eq!(offsets_8k, AAC_LC_8K_LONG_WINDOW_SCALE_FACTOR_BAND_OFFSETS);
            assert_eq!(offsets_8k.first().copied(), Some(0));
            assert_eq!(offsets_8k.last().copied(), Some(1024));
            assert_eq!(offsets_8k.len() - 1, 40);
        }
        assert!(sections
            .iter()
            .any(|section| section.codebook == AacCodebook::SignedPairs1));
        assert_eq!(scale_factors.len(), offsets.len() - 1);
        assert_eq!(deltas.len(), 2);
        assert!(section_bits.bit_len > 0);
        assert!(plan_sections_by_offsets(&quantized[..1023], offsets, Default::default()).is_err());
    }

    #[test]
    fn encodes_mono_stream_with_standard_long_window_offsets() {
        let offsets = aac_lc_long_window_scale_factor_band_offsets(44_100).unwrap();
        let pcm = AudioBuffer::new(
            44_100,
            1,
            (0..2048)
                .map(|sample| ((sample as f32) * 0.01).sin() * 0.25)
                .collect(),
        )
        .unwrap();
        let channel = AacLongBlockConfig::new(0, (offsets.len() - 1) as u8);
        let scale_factor_table = experimental_aac_scale_factor_delta_table();
        let flat_scale_factors = vec![120; offsets.len() - 1];
        let flat_channel = AacScaleFactorChannel::new(
            AacLongBlockConfig::new(120, (offsets.len() - 1) as u8),
            &flat_scale_factors,
        );
        let zero_scale_factor_table = aac_scale_factor_delta_zero_table();

        let details = select_aac_lc_mono_pcm_frame_step_details_with_offsets_by_bit_cost(
            AdtsConfig::aac_lc(44_100, 1),
            channel,
            &pcm,
            0,
            offsets,
            AAC_LC_PCM_STEP_CANDIDATES,
            &scale_factor_table,
            experimental_unit_magnitude_spectral_tables(),
        )
        .unwrap();
        let flat_details =
            select_aac_lc_mono_pcm_frame_step_details_with_offsets_and_scale_factors_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 1),
                flat_channel,
                &pcm,
                0,
                offsets,
                AAC_LC_PCM_STEP_CANDIDATES,
                zero_scale_factor_table,
                aac_unsigned_pairs7_unit_magnitude_spectral_tables(),
            )
            .unwrap();
        let adts = encode_pcm_mono_long_block_adts_stream_with_offsets_and_auto_step_by_bit_cost(
            AdtsConfig::aac_lc(44_100, 1),
            channel,
            &pcm,
            offsets,
            AAC_LC_PCM_STEP_CANDIDATES,
            &scale_factor_table,
            experimental_unit_magnitude_spectral_tables(),
        )
        .unwrap();
        let flat_adts =
            encode_pcm_mono_long_block_adts_stream_with_offsets_and_scale_factors_by_bit_cost(
                AdtsConfig::aac_lc(44_100, 1),
                flat_channel,
                &pcm,
                offsets,
                AAC_LC_PCM_STEP_CANDIDATES,
                zero_scale_factor_table,
                aac_unsigned_pairs7_unit_magnitude_spectral_tables(),
            )
            .unwrap();

        assert!(details.step < f32::MAX);
        assert!(flat_details.step < f32::MAX);
        assert_eq!(
            pack_scale_factor_deltas_with_table(
                &[AacScaleFactorDelta::new(0)],
                zero_scale_factor_table
            )
            .unwrap()
            .bit_len,
            1
        );
        assert!(pack_scale_factor_deltas_with_table(
            &[AacScaleFactorDelta::new(1)],
            zero_scale_factor_table
        )
        .is_err());
        assert_eq!(&adts[..2], &[0xff, 0xf1]);
        assert_eq!(&flat_adts[..2], &[0xff, 0xf1]);
        assert!(adts.len() > 7);
        assert!(flat_adts.len() > 7);
    }

    #[test]
    fn packs_aac_section_data() {
        let sections = vec![
            AacSection {
                start: 0,
                end: 4,
                codebook: AacCodebook::Zero,
            },
            AacSection {
                start: 4,
                end: 12,
                codebook: AacCodebook::SignedPairs5,
            },
            AacSection {
                start: 12,
                end: 16,
                codebook: AacCodebook::Escape,
            },
        ];

        let packed = pack_section_data(&sections, 4).unwrap();

        assert_eq!(packed, &[0x00, 0xa8, 0xac, 0x20]);
        assert_eq!(
            pack_section_data_with_len(&sections, 4).unwrap(),
            PackedBits {
                bytes: vec![0x00, 0xa8, 0xac, 0x20],
                bit_len: 27,
            }
        );
        assert_eq!(
            pack_section_data(
                &[AacSection {
                    start: 0,
                    end: 128,
                    codebook: AacCodebook::SignedPairs1,
                }],
                4
            )
            .unwrap(),
            &[0x1f, 0x84]
        );
        assert!(pack_section_data(&sections, 0).is_err());
        assert!(pack_section_data(
            &[AacSection {
                start: 1,
                end: 4,
                codebook: AacCodebook::Zero,
            }],
            4
        )
        .is_err());
    }

    #[test]
    fn extracts_aac_spectral_pairs_for_section() {
        let quantized = [0, 0, 1, -1, 3, 0, -2, 2];

        assert_eq!(
            spectral_pairs_for_section(
                &quantized,
                &AacSection {
                    start: 0,
                    end: 2,
                    codebook: AacCodebook::Zero,
                },
            )
            .unwrap(),
            Vec::<AacSpectralPair>::new()
        );
        assert_eq!(
            spectral_pairs_for_section(
                &quantized,
                &AacSection {
                    start: 2,
                    end: 8,
                    codebook: AacCodebook::SignedPairs5,
                },
            )
            .unwrap(),
            vec![
                AacSpectralPair::new(1, -1),
                AacSpectralPair::new(3, 0),
                AacSpectralPair::new(-2, 2),
            ]
        );
        assert!(spectral_pairs_for_section(
            &quantized,
            &AacSection {
                start: 1,
                end: 4,
                codebook: AacCodebook::SignedPairs1,
            },
        )
        .is_err());
        assert!(spectral_pairs_for_section(
            &quantized,
            &AacSection {
                start: 6,
                end: 10,
                codebook: AacCodebook::SignedPairs5,
            },
        )
        .is_err());
    }

