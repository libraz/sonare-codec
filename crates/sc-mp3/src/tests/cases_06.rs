    /// ISO/IEC 11172-3 §2.4.3.4 long-block requantization including per-band
    /// scale-factor attenuation `2^(-0.5·(1+scalefac_scale)·sf[sfb])`.
    fn requantize_long_line_with_scalefactors(
        is: i32,
        line: usize,
        global_gain: u8,
        scale_factors: &[u8; MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT],
        scalefac_scale: bool,
        sample_rate: u32,
    ) -> f32 {
        let index = mpeg1_layer3_long_scalefactor_band_index(sample_rate).unwrap();
        // scalefac_multiplier = 0.5·(1 + scalefac_scale) per ISO §2.4.3.4.
        let multiplier = if scalefac_scale { 1.0 } else { 0.5 };
        let attenuation = match index[1..=MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT]
            .iter()
            .position(|&boundary| line < usize::from(boundary))
        {
            Some(band) => 2.0_f32.powf(-multiplier * f32::from(scale_factors[band])),
            None => 1.0,
        };
        requantize_long_line(is, global_gain) * attenuation
    }

    #[test]
    fn scalefactor_quantizer_inverts_through_requantization() {
        // A decaying multitone spectrum: uniform scale-factor amplification must
        // reconstruct it (the encoder gain and decoder attenuation cancel) and
        // never lower the SNR versus zero scale factors, because amplifying the
        // pre-rounded magnitude buys finer effective quantization.
        let spectrum: Vec<f32> = (0..576)
            .map(|line| {
                let decay = (-(line as f32) / 200.0).exp();
                0.4 * decay * ((line as f32) * 0.21).sin()
            })
            .collect();
        let step = 0.05_f32;
        let global_gain = mpeg1_layer3_global_gain_for_step(step);

        let snr_for = |scale_factors: &[u8; MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT]| {
            let quantized = quantize_mpeg1_layer3_long_spectrum_with_scalefactors(
                &spectrum,
                step,
                scale_factors,
                false,
                44_100,
            )
            .unwrap();
            let mut signal = 0.0_f64;
            let mut noise = 0.0_f64;
            for (line, (&xr, &is)) in spectrum.iter().zip(quantized.iter()).enumerate() {
                let reconstructed = f64::from(requantize_long_line_with_scalefactors(
                    is,
                    line,
                    global_gain,
                    scale_factors,
                    false,
                    44_100,
                ));
                let reference = f64::from(xr);
                signal += reference * reference;
                let error = reconstructed - reference;
                noise += error * error;
            }
            10.0 * (signal / noise.max(1.0e-30)).log10()
        };

        let zero = [0_u8; MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT];
        let amplified = [3_u8; MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT];
        let snr_zero = snr_for(&zero);
        let snr_amplified = snr_for(&amplified);

        assert!(
            snr_zero > 20.0,
            "baseline reconstruction SNR too low: {snr_zero} dB"
        );
        assert!(
            snr_amplified >= snr_zero - 0.5,
            "scale-factor amplification regressed SNR: {snr_amplified} dB vs {snr_zero} dB"
        );
    }

    #[test]
    fn scalefactor_quantizer_rejects_nonpositive_step() {
        let spectrum = vec![0.1_f32; 576];
        let zero = [0_u8; MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT];
        assert!(quantize_mpeg1_layer3_long_spectrum_with_scalefactors(
            &spectrum, 0.0, &zero, false, 44_100
        )
        .is_err());
    }

    #[test]
    fn long_scalefactor_band_index_is_well_formed() {
        for rate in [32_000_u32, 44_100, 48_000] {
            let index = mpeg1_layer3_long_scalefactor_band_index(rate).unwrap();
            assert_eq!(index[0], 0, "{rate}: first boundary must be line 0");
            assert_eq!(
                index[MPEG1_LAYER3_LONG_SCALEFACTOR_BAND_BOUNDARIES - 1],
                576,
                "{rate}: long block spans 576 lines"
            );
            for pair in index.windows(2) {
                assert!(
                    pair[1] > pair[0],
                    "{rate}: boundaries must increase strictly ({} !> {})",
                    pair[1],
                    pair[0]
                );
            }
        }
    }

    #[test]
    fn long_scalefactor_band_index_rejects_unknown_rate() {
        assert!(mpeg1_layer3_long_scalefactor_band_index(22_050).is_err());
    }

    #[test]
    fn long_scalefactor_band_range_tiles_transmitted_bands() {
        // The 21 transmitted bands cover a contiguous prefix; the residual
        // highest band (no transmitted scale factor) carries the remainder.
        let (first_start, _) = mpeg1_layer3_long_scalefactor_band_range(0, 44_100).unwrap();
        assert_eq!(first_start, 0);
        let mut cursor = 0_usize;
        for band in 0..MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT {
            let (start, end) = mpeg1_layer3_long_scalefactor_band_range(band, 44_100).unwrap();
            assert_eq!(start, cursor, "band {band} is not contiguous");
            assert!(end > start, "band {band} is empty");
            cursor = end;
        }
        // 44.1 kHz: transmitted bands end at line 418; lines 418..576 are the
        // residual band that carries no transmitted scale factor.
        assert_eq!(cursor, 418);
        assert!(mpeg1_layer3_long_scalefactor_band_range(21, 44_100).is_err());
    }

    #[test]
    fn general_long_scalefactor_band_index_covers_iso_rates() {
        // MPEG-1 (ISO/IEC 11172-3) and MPEG-2 LSF (ISO/IEC 13818-3) rates.
        for rate in [32_000_u32, 44_100, 48_000, 16_000, 22_050, 24_000] {
            let index = layer3_long_scalefactor_band_index(rate).unwrap();
            assert_eq!(index[0], 0, "{rate}: first boundary must be line 0");
            assert_eq!(
                index[MPEG1_LAYER3_LONG_SCALEFACTOR_BAND_BOUNDARIES - 1],
                576,
                "{rate}: long block spans 576 lines"
            );
            for pair in index.windows(2) {
                assert!(
                    pair[1] > pair[0],
                    "{rate}: boundaries must increase strictly ({} !> {})",
                    pair[1],
                    pair[0]
                );
            }
        }
    }

    #[test]
    fn general_long_scalefactor_band_index_matches_mpeg1_for_mpeg1_rates() {
        for rate in [32_000_u32, 44_100, 48_000] {
            assert_eq!(
                layer3_long_scalefactor_band_index(rate).unwrap(),
                mpeg1_layer3_long_scalefactor_band_index(rate).unwrap(),
            );
        }
    }

    #[test]
    fn mpeg2_lsf_long_band_tables_match_iso_13818_3() {
        // 16 kHz shares the 22.05 kHz table; 24 kHz diverges from band 12 up.
        assert_eq!(
            layer3_long_scalefactor_band_index(16_000).unwrap(),
            layer3_long_scalefactor_band_index(22_050).unwrap(),
        );
        assert_ne!(
            layer3_long_scalefactor_band_index(24_000).unwrap(),
            layer3_long_scalefactor_band_index(22_050).unwrap(),
        );
        // Spot-check the normative boundaries (ISO/IEC 13818-3 Table B.8).
        assert_eq!(layer3_long_scalefactor_band_index(22_050).unwrap()[12], 116);
        assert_eq!(layer3_long_scalefactor_band_index(24_000).unwrap()[12], 114);
    }

    #[test]
    fn mpeg2_lsf_long_band_range_excludes_mpeg25_rates() {
        // 24 kHz transmitted bands tile contiguously up to the residual band.
        let mut cursor = 0_usize;
        for band in 0..MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT {
            let (start, end) = layer3_long_scalefactor_band_range(band, 24_000).unwrap();
            assert_eq!(start, cursor, "band {band} is not contiguous");
            assert!(end > start, "band {band} is empty");
            cursor = end;
        }
        assert_eq!(cursor, 540);
        // MPEG-2.5 (8/11.025/12 kHz) is outside ISO 11172-3 / 13818-3.
        for rate in [8_000_u32, 11_025, 12_000] {
            assert!(layer3_long_scalefactor_band_index(rate).is_err());
        }
    }

    #[test]
    fn short_scalefactor_band_index_is_well_formed() {
        for rate in [44_100_u32, 48_000, 32_000, 22_050, 24_000, 16_000] {
            let index = layer3_short_scalefactor_band_index(rate).unwrap();
            assert_eq!(index.len(), 14, "rate {rate} short index has 14 boundaries");
            assert_eq!(index[0], 0, "rate {rate} starts at line 0");
            assert_eq!(index[13], 192, "rate {rate} ends at the window length");
            for pair in index.windows(2) {
                assert!(pair[1] > pair[0], "rate {rate} boundaries strictly increase");
            }
        }
    }

    #[test]
    fn short_scalefactor_band_index_rejects_unknown_rate() {
        for rate in [8_000_u32, 11_025, 12_000, 96_000] {
            assert!(layer3_short_scalefactor_band_index(rate).is_err());
        }
    }

    #[test]
    fn short_reorder_map_is_a_permutation_for_all_rates() {
        for rate in [44_100_u32, 48_000, 32_000, 22_050, 24_000, 16_000] {
            let map = layer3_short_reorder_map(rate).unwrap();
            let mut sorted = map.to_vec();
            sorted.sort_unstable();
            let expected: Vec<usize> = (0..576).collect();
            assert_eq!(sorted, expected, "rate {rate} reorder map is a permutation");
        }
    }

    #[test]
    fn short_reorder_map_groups_each_band_window_major() {
        // Short band 0 at 44.1 kHz spans window-local lines [0, 4): width 4.
        // The bitstream emits all four lines of window 0 (raw 0..4), then window
        // 1 (raw 6..10), then window 2 (raw 12..16). Band 1 spans [4, 8): its
        // window-0 lines are global frequencies 4 and 5 (subband 0, lines 4, 5 ->
        // raw 4, 5) followed by frequencies 6, 7 (subband 1, lines 0, 1 -> raw
        // 18, 19).
        let map = layer3_short_reorder_map(44_100).unwrap();
        let expected = [0, 1, 2, 3, 6, 7, 8, 9, 12, 13, 14, 15, 4, 5, 18, 19];
        assert_eq!(&map[..expected.len()], &expected);
    }

    #[test]
    fn short_scalefactor_band_range_tiles_reordered_lines() {
        for rate in [44_100_u32, 48_000, 32_000, 22_050, 24_000, 16_000] {
            let mut cursor = 0_usize;
            for band in 0..13 {
                let (start, end) = layer3_short_scalefactor_band_range(band, rate).unwrap();
                assert_eq!(start, cursor, "rate {rate} band {band} is not contiguous");
                assert!(end > start, "rate {rate} band {band} is empty");
                cursor = end;
            }
            assert_eq!(cursor, 576, "rate {rate} short bands tile the granule");
            assert!(layer3_short_scalefactor_band_range(13, rate).is_err());
        }
    }

    /// ISO/IEC 11172-3 §2.4.3.4 short-block requantization for one reordered
    /// line: `xr = sign(is)·|is|^(4/3)·2^(0.25·(global_gain−210−8·subblock_gain))
    /// ·2^(−0.5·(1+scalefac_scale)·sf)`.
    #[test]
    fn short_quantizer_inverts_through_requantization() {
        let rate = 44_100_u32;
        let index = layer3_short_scalefactor_band_index(rate).unwrap();
        // A decaying multitone spectrum already in bitstream reorder order.
        let spectrum: Vec<f32> = (0..576)
            .map(|i| {
                let decay = (-(i as f32) / 250.0).exp();
                0.3 * decay * ((i as f32) * 0.17).sin()
            })
            .collect();
        let step = 0.05_f32;
        let scalefac_scale = false;
        let global_gain = mpeg1_layer3_global_gain_for_step(step);
        let subblock_gain = [0_u8, 1, 2];
        let mut scale_factors = [[0_u8; 3]; 12];
        for (band, windows) in scale_factors.iter_mut().enumerate() {
            for (window, sf) in windows.iter_mut().enumerate() {
                *sf = ((band + window) % 4) as u8;
            }
        }

        let quantized = quantize_mpeg1_layer3_short_spectrum_with_scalefactors(
            &spectrum,
            step,
            &scale_factors,
            &subblock_gain,
            scalefac_scale,
            rate,
        )
        .unwrap();
        assert_eq!(quantized.len(), 576);

        // Requantize each line with the decoder formula, walking the same
        // band/window order the encoder used, and measure reconstruction SNR.
        let multiplier = if scalefac_scale { 1.0_f64 } else { 0.5 };
        let mut signal = 0.0_f64;
        let mut noise = 0.0_f64;
        let mut pos = 0_usize;
        for band in 0..index.len() - 1 {
            let width = usize::from(index[band + 1]) - usize::from(index[band]);
            for (window, &sbg) in subblock_gain.iter().enumerate() {
                let sf = if band < 12 { scale_factors[band][window] } else { 0 };
                for _ in 0..width {
                    let is = quantized[pos];
                    let sign = if is < 0 { -1.0_f64 } else { 1.0 };
                    let magnitude = (is.unsigned_abs() as f64).powf(4.0 / 3.0);
                    let gain = 2.0_f64.powf(
                        0.25 * (f64::from(global_gain) - 210.0 - 8.0 * f64::from(sbg)),
                    );
                    let attenuation = 2.0_f64.powf(-multiplier * f64::from(sf));
                    let reconstructed = sign * magnitude * gain * attenuation;
                    let reference = f64::from(spectrum[pos]);
                    signal += reference * reference;
                    let error = reconstructed - reference;
                    noise += error * error;
                    pos += 1;
                }
            }
        }
        let snr = 10.0 * (signal / noise.max(1.0e-30)).log10();
        assert!(snr > 20.0, "short requant reconstruction SNR too low: {snr} dB");
    }

    #[test]
    fn short_quantizer_rejects_invalid_inputs() {
        let scale_factors = [[0_u8; 3]; 12];
        let subblock_gain = [0_u8; 3];
        let full = vec![0.1_f32; 576];
        // Non-positive step.
        assert!(quantize_mpeg1_layer3_short_spectrum_with_scalefactors(
            &full, 0.0, &scale_factors, &subblock_gain, false, 44_100
        )
        .is_err());
        // Wrong granule length.
        let short = vec![0.1_f32; 575];
        assert!(quantize_mpeg1_layer3_short_spectrum_with_scalefactors(
            &short, 0.05, &scale_factors, &subblock_gain, false, 44_100
        )
        .is_err());
        // Unsupported sample rate.
        assert!(quantize_mpeg1_layer3_short_spectrum_with_scalefactors(
            &full, 0.05, &scale_factors, &subblock_gain, false, 11_025
        )
        .is_err());
    }

    #[test]
    fn short_block_packer_fills_window_switching_and_roundtrips_scalefactors() {
        let provider = mpeg1_layer3_standard_table_provider();
        let mut scale_factors = [[0_u8; 3]; 12];
        for (band, windows) in scale_factors.iter_mut().enumerate() {
            for (window, sf) in windows.iter_mut().enumerate() {
                *sf = ((band + 2 * window) % 4) as u8;
            }
        }
        let subblock_gain = [1_u8, 0, 2];

        // A reordered quantized spectrum with energy in region0 (lines < 36),
        // region1 (>= 36), and a unit-magnitude count1 tail.
        let mut quantized = vec![0_i32; 576];
        for (line, value) in quantized.iter_mut().enumerate().take(30) {
            *value = (line % 7) as i32 - 3;
        }
        for (offset, value) in quantized[36..80].iter_mut().enumerate() {
            *value = ((offset + 36) % 5) as i32 - 2;
        }
        for (offset, value) in quantized[80..96].iter_mut().enumerate() {
            *value = ((offset + 80) % 3) as i32 - 1;
        }

        let mut granule = Layer3GranuleChannelInfo::default();
        let packed = pack_mpeg1_layer3_short_quantized_spectrum_with_table_provider(
            &mut granule,
            &scale_factors,
            &subblock_gain,
            &quantized,
            provider,
        )
        .unwrap();

        // The granule carries a block_type 2 window-switching descriptor with the
        // supplied subblock_gain and two region table selects.
        let ws = granule
            .window_switching
            .expect("short block sets window switching");
        assert_eq!(ws.block_type, 2);
        assert!(!ws.mixed_block_flag);
        assert_eq!(ws.subblock_gain, subblock_gain);

        // part2_3_length is self-consistent with the packed payload.
        assert_eq!(usize::from(granule.part2_3_length), packed.bit_len);
        assert!(granule.big_values > 0);

        // The packed scale-factor bits round-trip under the selected widths.
        let selection = select_mpeg1_layer3_short_scale_factor_compress(&scale_factors).unwrap();
        assert_eq!(granule.scalefac_compress, selection.scalefac_compress);
        let sf_bits = pack_mpeg1_layer3_short_scale_factors(&scale_factors, selection).unwrap();
        let bytes = sf_bits.bytes;
        let mut bit_pos = 0_usize;
        let mut read = |width: u8| -> u32 {
            let mut value = 0_u32;
            for _ in 0..width {
                let byte = bytes[bit_pos / 8];
                let bit = (byte >> (7 - (bit_pos % 8))) & 1;
                value = (value << 1) | u32::from(bit);
                bit_pos += 1;
            }
            value
        };
        for (band, windows) in scale_factors.iter().enumerate() {
            let width = if band < 6 { selection.slen1 } else { selection.slen2 };
            for &sf in windows {
                assert_eq!(read(width), u32::from(sf), "scalefactor band {band} mismatch");
            }
        }
        assert_eq!(bit_pos, sf_bits.bit_len);
    }
