    #[test]
    #[cfg(feature = "aac")]
    fn exposes_aac_codebook7_section_planning() {
        let sections = super::plan_sections_by_bit_cost(
            &[1, -1, 0, 0],
            2,
            super::AacSpectralMagnitudeTables::default(),
        )
        .unwrap();

        assert_eq!(
            sections,
            vec![
                super::AacSection {
                    start: 0,
                    end: 2,
                    codebook: super::AacCodebook::UnsignedPairs8,
                },
                super::AacSection {
                    start: 2,
                    end: 4,
                    codebook: super::AacCodebook::Zero,
                },
            ]
        );
    }

    #[test]
    #[cfg(feature = "aac")]
    fn dispatches_silent_aac_encode_as_adts() {
        let pcm = AudioBuffer::new(44_100, 1, vec![0.0; 1024]).unwrap();

        let adts = encode(Format::Aac, &pcm).unwrap();
        let decoded = decode(&adts).unwrap();
        let m4a = super::mux_aac_adts_as_m4a(&adts).unwrap();
        let decoded_m4a = decode(&m4a).unwrap();
        let decoded_aac_helper = super::decode_aac(&adts).unwrap();
        let decoded_m4a_helper = super::decode_aac(&m4a).unwrap();

        assert_eq!(
            encode_with_mode(Format::Aac, &pcm, EncodeMode::ProductionOnly).unwrap(),
            adts
        );
        assert_eq!(&adts[..2], &[0xff, 0xf1]);
        assert_eq!(super::detect(&adts), Some(Format::Aac));
        assert_eq!(decoded.sample_rate, 44_100);
        assert_eq!(decoded.channels, 1);
        assert_eq!(decoded_m4a.sample_rate, 44_100);
        assert_eq!(decoded_m4a.channels, 1);
        assert_eq!(decoded_m4a.samples.len(), pcm.samples.len());
        assert_eq!(decoded_aac_helper.samples.len(), pcm.samples.len());
        assert_eq!(decoded_m4a_helper.samples.len(), pcm.samples.len());
    }

    #[test]
    #[cfg(feature = "aac")]
    fn dispatches_non_silent_aac_encode_as_adts_scaffold() {
        for (sample_rate, channels) in [
            (7_350, 1),
            (8_000, 1),
            (11_025, 1),
            (12_000, 1),
            (16_000, 1),
            (22_050, 1),
            (24_000, 1),
            (32_000, 1),
            (44_100, 1),
            (48_000, 1),
            (64_000, 1),
            (88_200, 1),
            (96_000, 1),
            (7_350, 2),
            (8_000, 2),
            (11_025, 2),
            (12_000, 2),
            (16_000, 2),
            (22_050, 2),
            (24_000, 2),
            (32_000, 2),
            (44_100, 2),
            (48_000, 2),
            (64_000, 2),
            (88_200, 2),
            (96_000, 2),
        ] {
            let mut samples = Vec::new();
            for frame in 0..2048 {
                for channel in 0..channels {
                    let phase = if channel == 0 { 0.01 } else { 0.013 };
                    samples.push(((frame as f32) * phase).sin() * 0.25);
                }
            }
            let pcm = AudioBuffer::new(sample_rate, channels, samples).unwrap();

            let adts = encode(Format::Aac, &pcm).unwrap();
            let production =
                encode_with_mode(Format::Aac, &pcm, EncodeMode::ProductionOnly).unwrap();
            let m4a = super::mux_aac_adts_as_m4a(&adts).unwrap();

            assert_eq!(&adts[..2], &[0xff, 0xf1]);
            assert_eq!(&production[..2], &[0xff, 0xf1]);
            assert_eq!(production, adts);
            assert_eq!(super::detect(&adts), Some(Format::Aac));
            assert!(adts.len() > 7);
            assert!(m4a.len() > adts.len());
        }
    }
