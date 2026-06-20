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
