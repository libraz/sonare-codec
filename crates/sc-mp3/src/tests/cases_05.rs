    #[test]
    fn encodes_pcm_frames_with_max_payload_bits() {
        let pcm = AudioBuffer::new(
            44_100,
            1,
            (0..2304)
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
        let first_frame = select_mpeg1_layer3_pcm_frame_step_details_with_table_provider(
            header,
            &pcm,
            0,
            MPEG1_LAYER3_PCM_STEP_CANDIDATES,
            provider,
        )
        .unwrap();
        let budget = first_frame.payload_bit_len;
        let step = select_mpeg1_layer3_pcm_frame_step_with_max_payload_bits_and_table_provider(
            header,
            &pcm,
            0,
            MPEG1_LAYER3_PCM_STEP_CANDIDATES,
            budget,
            provider,
        )
        .unwrap();

        let budgeted = encode_mpeg1_layer3_pcm_frames_with_max_payload_bits_and_table_provider(
            &pcm,
            MPEG1_LAYER3_PCM_STEP_CANDIDATES,
            budget,
            provider,
        )
        .unwrap();
        let budgeted_with_header =
            encode_mpeg1_layer3_pcm_frames_with_header_and_max_payload_bits_and_table_provider(
                header,
                &pcm,
                MPEG1_LAYER3_PCM_STEP_CANDIDATES,
                budget,
                provider,
            )
            .unwrap();
        let selected =
            encode_mpeg1_layer3_pcm_frames_with_selected_scale_factors_and_table_provider(
                &pcm, step, provider,
            )
            .unwrap();

        assert_eq!(budgeted, budgeted_with_header);
        // The budget path and the explicit-step path agree on the first frame,
        // since `step` is the budgeted step selected for frame 0. Later frames
        // carry distinct spectra and may select a different per-frame step.
        let frame_len = header.frame_len();
        assert_eq!(budgeted[..frame_len], selected[..frame_len]);
        assert_eq!(budgeted.len(), selected.len());
        assert_eq!(budgeted.len(), header.frame_len() * 2);
        assert!(
            encode_mpeg1_layer3_pcm_frames_with_max_payload_bits_and_table_provider(
                &pcm,
                MPEG1_LAYER3_PCM_STEP_CANDIDATES,
                0,
                provider,
            )
            .is_err()
        );
    }

    #[test]
    fn encodes_pcm_frames_with_bitrate_helper() {
        let pcm = AudioBuffer::new(
            44_100,
            1,
            (0..1152)
                .map(|sample| ((sample as f32) * 0.01).sin() * 0.25)
                .collect(),
        )
        .unwrap();
        let provider = mpeg1_layer3_standard_table_provider();
        let header = layer3_header_for_capacity(44_100, 1, 96, false, false).unwrap();

        let encoded = encode_mpeg1_layer3_pcm_frames_with_bitrate_and_table_provider(
            &pcm,
            MPEG1_LAYER3_PCM_STEP_CANDIDATES,
            96,
            false,
            false,
            provider,
        )
        .unwrap();
        let explicit = encode_mpeg1_layer3_pcm_frames_with_header_and_auto_step_and_table_provider(
            header,
            &pcm,
            MPEG1_LAYER3_PCM_STEP_CANDIDATES,
            provider,
        )
        .unwrap();
        let parsed = FrameHeader::parse(&encoded[..4]).unwrap();

        assert_eq!(encoded, explicit);
        assert_eq!(parsed, header);
        assert_eq!(parsed.bitrate_kbps, 96);
        assert_eq!(encoded.len(), header.frame_len());
        assert!(
            encode_mpeg1_layer3_pcm_frames_with_bitrate_and_table_provider(
                &pcm,
                MPEG1_LAYER3_PCM_STEP_CANDIDATES,
                123,
                false,
                false,
                provider,
            )
            .is_err()
        );
    }

    #[test]
    fn encodes_perceptual_pcm_frames_with_bitrate_helper() {
        let pcm = AudioBuffer::new(
            44_100,
            1,
            (0..1152)
                .map(|sample| ((sample as f32) * 0.013).sin() * 0.22)
                .collect(),
        )
        .unwrap();
        let provider = mpeg1_layer3_standard_table_provider();
        let header = layer3_header_for_capacity(44_100, 1, 96, false, false).unwrap();
        let candidates = [0.05_f32, 0.1, 0.2, 0.4];

        let encoded = encode_mpeg1_layer3_pcm_frames_with_perceptual_bitrate_and_table_provider(
            &pcm,
            &candidates,
            96,
            false,
            false,
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
        let parsed = FrameHeader::parse(&encoded[..4]).unwrap();

        assert_eq!(encoded, explicit);
        assert_eq!(parsed, header);
        assert_eq!(parsed.bitrate_kbps, 96);
        assert_eq!(encoded.len(), header.frame_len());
        assert!(
            encode_mpeg1_layer3_pcm_frames_with_perceptual_bitrate_and_table_provider(
                &pcm,
                &candidates,
                123,
                false,
                false,
                provider,
            )
            .is_err()
        );
    }

    #[test]
    fn encodes_pcm_frames_with_cbr_bitrate_padding_schedule() {
        let pcm = AudioBuffer::new(
            44_100,
            1,
            (0..(1152 * 3))
                .map(|sample| ((sample as f32) * 0.01).sin() * 0.25)
                .collect(),
        )
        .unwrap();
        let provider = mpeg1_layer3_standard_table_provider();
        let unpadded_header = layer3_header_for_capacity(44_100, 1, 128, false, false).unwrap();
        let padded_header = layer3_header_for_capacity(44_100, 1, 128, true, false).unwrap();

        let cbr = encode_mpeg1_layer3_pcm_frames_with_cbr_bitrate_and_table_provider(
            &pcm,
            MPEG1_LAYER3_PCM_STEP_CANDIDATES,
            128,
            false,
            provider,
        )
        .unwrap();
        let fixed = encode_mpeg1_layer3_pcm_frames_with_bitrate_and_table_provider(
            &pcm,
            MPEG1_LAYER3_PCM_STEP_CANDIDATES,
            128,
            false,
            false,
            provider,
        )
        .unwrap();

        let first = FrameHeader::parse(&cbr[..4]).unwrap();
        let second_offset = first.frame_len();
        let second = FrameHeader::parse(&cbr[second_offset..second_offset + 4]).unwrap();
        let third_offset = second_offset + second.frame_len();
        let third = FrameHeader::parse(&cbr[third_offset..third_offset + 4]).unwrap();

        assert_eq!(first, unpadded_header);
        assert_eq!(second, padded_header);
        assert_eq!(third, padded_header);
        assert_eq!(
            cbr.len(),
            unpadded_header.frame_len() + 2 * padded_header.frame_len()
        );
        assert_eq!(fixed.len(), 3 * unpadded_header.frame_len());
        assert!(cbr.len() > fixed.len());
    }

    #[test]
    fn encodes_perceptual_pcm_frames_with_cbr_bitrate_padding_schedule() {
        let pcm = AudioBuffer::new(
            44_100,
            1,
            (0..(1152 * 3))
                .map(|sample| ((sample as f32) * 0.013).sin() * 0.22)
                .collect(),
        )
        .unwrap();
        let provider = mpeg1_layer3_standard_table_provider();
        let candidates = [0.05_f32, 0.1, 0.2, 0.4];
        let unpadded_header = layer3_header_for_capacity(44_100, 1, 128, false, false).unwrap();
        let padded_header = layer3_header_for_capacity(44_100, 1, 128, true, false).unwrap();

        let cbr = encode_mpeg1_layer3_pcm_frames_with_perceptual_cbr_bitrate_and_table_provider(
            &pcm,
            &candidates,
            128,
            false,
            provider,
        )
        .unwrap();
        let fixed = encode_mpeg1_layer3_pcm_frames_with_perceptual_bitrate_and_table_provider(
            &pcm,
            &candidates,
            128,
            false,
            false,
            provider,
        )
        .unwrap();

        let first = FrameHeader::parse(&cbr[..4]).unwrap();
        let second_offset = first.frame_len();
        let second = FrameHeader::parse(&cbr[second_offset..second_offset + 4]).unwrap();
        let third_offset = second_offset + second.frame_len();
        let third = FrameHeader::parse(&cbr[third_offset..third_offset + 4]).unwrap();

        assert_eq!(first, unpadded_header);
        assert_eq!(second, padded_header);
        assert_eq!(third, padded_header);
        assert_eq!(
            cbr.len(),
            unpadded_header.frame_len() + 2 * padded_header.frame_len()
        );
        assert_eq!(fixed.len(), 3 * unpadded_header.frame_len());
        assert!(cbr.len() > fixed.len());

        let active_cbr =
            encode_mpeg1_layer3_pcm_frames_with_perceptual_active_cbr_bitrate_and_table_provider(
                &pcm,
                &candidates,
                128,
                false,
                provider,
            )
            .unwrap();
        assert_eq!(
            FrameHeader::parse(&active_cbr[..4]).unwrap(),
            unpadded_header
        );
        assert_eq!(
            active_cbr.len(),
            unpadded_header.frame_len() + 2 * padded_header.frame_len()
        );
    }

    #[test]
    fn reservoir_encode_borrows_main_data_across_frames() {
        // Alternate broadband (expensive to quantize) and near-silent (cheap)
        // frames so the shared main-data stream builds a reservoir that later
        // frames reference: main_data_begin must climb above zero somewhere.
        let frames = 8_usize;
        let samples_per_frame = 1152_usize;
        let mut samples = Vec::with_capacity(frames * samples_per_frame);
        for frame in 0..frames {
            let loud = frame % 2 == 0;
            for n in 0..samples_per_frame {
                let t = n as f32;
                let value = if loud {
                    0.3 * ((t * 0.043).sin()
                        + (t * 0.131).sin()
                        + (t * 0.277).sin()
                        + (t * 0.611).sin())
                } else {
                    0.02 * (t * 0.05).sin()
                };
                samples.push(value);
            }
        }
        let pcm = AudioBuffer::new(44_100, 1, samples).unwrap();
        let provider = mpeg1_layer3_standard_table_provider();
        let stream = encode_mpeg1_layer3_pcm_frames_with_reservoir_and_table_provider(
            &pcm,
            MPEG1_LAYER3_PCM_STEP_CANDIDATES,
            128,
            false,
            provider,
        )
        .unwrap();

        // Walk the stream: every frame parses, the buffer is consumed exactly,
        // and main_data_begin (the first 9 side-info bits) exceeds zero on at
        // least one frame, proving cross-frame borrowing.
        let mut offset = 0_usize;
        let mut frame_count = 0_usize;
        let mut max_main_data_begin = 0_u32;
        while offset < stream.len() {
            let header = FrameHeader::parse(&stream[offset..offset + 4]).unwrap();
            let mut reader = BitReader::new(&stream[offset + 4..]);
            let main_data_begin = reader.read_bits(9).unwrap();
            max_main_data_begin = max_main_data_begin.max(main_data_begin);
            offset += header.frame_len();
            frame_count += 1;
        }
        assert_eq!(offset, stream.len(), "frames did not tile the stream");
        assert_eq!(frame_count, frames);
        assert!(
            max_main_data_begin > 0,
            "reservoir never used: main_data_begin stayed zero"
        );
        // The MPEG-1 main_data_begin pointer is 9 bits wide.
        assert!(max_main_data_begin <= 511);
    }

    #[test]
    fn reservoir_frame_details_match_encoded_main_data_begin() {
        let frames = 8_usize;
        let samples_per_frame = 1152_usize;
        let mut samples = Vec::with_capacity(frames * samples_per_frame);
        for frame in 0..frames {
            let loud = frame % 2 == 0;
            for n in 0..samples_per_frame {
                let t = n as f32;
                samples.push(if loud {
                    0.3 * ((t * 0.043).sin()
                        + (t * 0.131).sin()
                        + (t * 0.277).sin()
                        + (t * 0.611).sin())
                } else {
                    0.02 * (t * 0.05).sin()
                });
            }
        }
        let pcm = AudioBuffer::new(44_100, 1, samples).unwrap();
        let provider = mpeg1_layer3_standard_table_provider();
        let details = select_mpeg1_layer3_reservoir_frame_details_with_table_provider(
            &pcm,
            MPEG1_LAYER3_PCM_STEP_CANDIDATES,
            128,
            false,
            provider,
        )
        .unwrap();
        let stream = encode_mpeg1_layer3_pcm_frames_with_reservoir_and_table_provider(
            &pcm,
            MPEG1_LAYER3_PCM_STEP_CANDIDATES,
            128,
            false,
            provider,
        )
        .unwrap();

        assert_eq!(details.len(), frames);
        assert_eq!(details[0].frame_index, 0);
        assert_eq!(details[0].main_data_begin, 0);
        assert!(
            details.iter().any(|detail| detail.main_data_begin > 0),
            "reservoir details never reported borrowing"
        );
        assert!(
            details.iter().any(|detail| detail.reservoir_after > 0),
            "reservoir details never accumulated spare data"
        );
        assert!(details.iter().all(|detail| {
            detail.payload_bit_len <= (detail.frame_capacity_bytes + detail.main_data_begin) * 8
        }));
        assert!(details
            .iter()
            .all(|detail| { detail.perceptual_granules == 0 && detail.calibrated_granules == 2 }));

        let mut offset = 0_usize;
        for detail in &details {
            let header = FrameHeader::parse(&stream[offset..offset + 4]).unwrap();
            assert_eq!(detail.frame_len, header.frame_len());
            assert_eq!(detail.padding, header.padding);
            let mut reader = BitReader::new(&stream[offset + 4..]);
            assert_eq!(detail.main_data_begin as u32, reader.read_bits(9).unwrap());
            offset += header.frame_len();
        }
        assert_eq!(offset, stream.len());
    }

    #[test]
    fn perceptual_reservoir_frame_details_match_encoded_main_data_begin() {
        let frames = 8_usize;
        let samples_per_frame = 1152_usize;
        let mut samples = Vec::with_capacity(frames * samples_per_frame);
        for frame in 0..frames {
            let loud = frame % 2 == 0;
            for n in 0..samples_per_frame {
                let t = n as f32;
                samples.push(if loud {
                    0.24 * ((t * 0.043).sin() + 0.7 * (t * 0.131).sin() + 0.4 * (t * 0.277).sin())
                } else {
                    0.02 * (t * 0.05).sin()
                });
            }
        }
        let pcm = AudioBuffer::new(44_100, 1, samples).unwrap();
        let provider = mpeg1_layer3_standard_table_provider();
        let details = select_mpeg1_layer3_perceptual_reservoir_frame_details_with_table_provider(
            &pcm,
            MPEG1_LAYER3_PCM_STEP_CANDIDATES,
            128,
            false,
            provider,
        )
        .unwrap();
        let stream = encode_mpeg1_layer3_pcm_frames_with_perceptual_reservoir_and_table_provider(
            &pcm,
            MPEG1_LAYER3_PCM_STEP_CANDIDATES,
            128,
            false,
            provider,
        )
        .unwrap();
        let calibrated = encode_mpeg1_layer3_pcm_frames_with_reservoir_and_table_provider(
            &pcm,
            MPEG1_LAYER3_PCM_STEP_CANDIDATES,
            128,
            false,
            provider,
        )
        .unwrap();
        let self_contained =
            encode_mpeg1_layer3_pcm_frames_with_perceptual_active_cbr_bitrate_and_table_provider(
                &pcm,
                MPEG1_LAYER3_PCM_STEP_CANDIDATES,
                128,
                false,
                provider,
            )
            .unwrap();

        assert_eq!(details.len(), frames);
        assert_ne!(stream, calibrated);
        assert_ne!(stream, self_contained);
        assert!(
            details.iter().any(|detail| detail.main_data_begin > 0),
            "perceptual reservoir details never reported borrowing"
        );
        assert!(
            details.iter().any(|detail| detail.reservoir_after > 0),
            "perceptual reservoir details never accumulated spare data"
        );
        assert!(details.iter().all(|detail| {
            detail.payload_bit_len <= (detail.frame_capacity_bytes + detail.main_data_begin) * 8
        }));
        assert!(details
            .iter()
            .all(|detail| { detail.perceptual_granules == 2 && detail.calibrated_granules == 0 }));

        let mut offset = 0_usize;
        for detail in &details {
            let header = FrameHeader::parse(&stream[offset..offset + 4]).unwrap();
            assert_eq!(detail.frame_len, header.frame_len());
            assert_eq!(detail.padding, header.padding);
            let mut reader = BitReader::new(&stream[offset + 4..]);
            assert_eq!(detail.main_data_begin as u32, reader.read_bits(9).unwrap());
            offset += header.frame_len();
        }
        assert_eq!(offset, stream.len());
    }

    #[test]
    fn quality_guarded_perceptual_reservoir_prefers_active_mono_scale_factor_steps() {
        let frames = 2_usize;
        let samples_per_frame = 1152_usize;
        let mut samples = Vec::with_capacity(frames * samples_per_frame);
        for frame in 0..frames * samples_per_frame {
            samples.push(((frame as f32) * 0.01).sin() * 0.25);
        }
        let pcm = AudioBuffer::new(44_100, 1, samples).unwrap();
        let details =
            select_mpeg1_layer3_quality_guarded_perceptual_reservoir_frame_details_with_table_provider(
                &pcm,
                MPEG1_LAYER3_PCM_STEP_CANDIDATES,
                128,
                false,
                mpeg1_layer3_standard_table_provider(),
            )
            .unwrap();

        assert_eq!(details.len(), frames);
        assert!(details.iter().all(|detail| {
            detail.payload_bit_len <= (detail.frame_capacity_bytes + detail.main_data_begin) * 8
        }));
        assert!(details.iter().all(|detail| detail.step >= 1.0));
        assert!(details
            .iter()
            .any(|detail| detail.quality_guard_compared_granules > 0));
        assert!(details.iter().any(|detail| detail.perceptual_granules > 0));
        assert!(details.iter().all(|detail| detail.calibrated_granules == 0));
    }

    #[test]
    fn entropy_target_utilization_profile_summarizes_selected_budget_usage() {
        let frames = 2_usize;
        let samples_per_frame = 1152_usize;
        let samples = (0..frames * samples_per_frame)
            .map(|frame| ((frame as f32) * 0.01).sin() * 0.25)
            .collect::<Vec<_>>();
        let pcm = AudioBuffer::new(44_100, 1, samples).unwrap();
        let details =
            crate::select_mpeg1_layer3_entropy_targeted_perceptual_reservoir_frame_details_with_table_provider(
                &pcm,
                MPEG1_LAYER3_PCM_STEP_CANDIDATES,
                128,
                false,
                0,
                mpeg1_layer3_standard_table_provider(),
            )
            .unwrap();

        let profile = crate::mpeg1_layer3_entropy_target_utilization_profile(&details);
        let selected_profile =
            crate::select_mpeg1_layer3_entropy_target_utilization_profile_with_table_provider(
                &pcm,
                MPEG1_LAYER3_PCM_STEP_CANDIDATES,
                128,
                false,
                0,
                mpeg1_layer3_standard_table_provider(),
            )
            .unwrap();

        assert_eq!(profile, selected_profile);
        assert_eq!(profile.frames, details.len());
        assert!(profile.used_entropy_target_frames > 0);
        assert!(profile.payload_bits > 0);
        assert!(profile.entropy_budget_bits >= profile.payload_bits);
        assert!(profile.utilization > 0.0 && profile.utilization <= 1.0);
    }

    #[test]
    fn first_frame_quality_guarded_candidate_profile_reports_guard_proxy_state() {
        let mut samples = Vec::with_capacity(2304);
        for frame in 0..2304 {
            samples.push(((frame as f32) * 0.01).sin() * 0.25);
        }
        let pcm = AudioBuffer::new(44_100, 1, samples).unwrap();
        let profiles =
            select_mpeg1_layer3_first_frame_quality_guarded_candidate_profile_with_table_provider(
                &pcm,
                MPEG1_LAYER3_PCM_STEP_CANDIDATES,
                128,
                false,
                mpeg1_layer3_standard_table_provider(),
            )
            .unwrap();

        assert!(profiles.iter().any(|profile| profile.step == 1.0));
        let fine_active = profiles
            .iter()
            .find(|profile| profile.step < 1.0 && profile.perceptual_granules > 0)
            .unwrap();
        assert_eq!(fine_active.quality_guard_compared_granules, 2);
        assert!(fine_active.quality_guard_distortion_delta.is_finite());
        let selected =
            select_mpeg1_layer3_quality_guarded_perceptual_reservoir_frame_details_with_table_provider(
                &pcm,
                MPEG1_LAYER3_PCM_STEP_CANDIDATES,
                128,
                false,
                mpeg1_layer3_standard_table_provider(),
            )
            .unwrap();
        assert!(selected.iter().all(|detail| detail.step >= 1.0));
        assert!(profiles
            .iter()
            .all(|profile| profile.payload_bit_len <= profile.frame_capacity_bits));
    }

    #[test]
    fn quality_guard_candidate_tiebreak_prefers_simpler_scale_factors() {
        let mut complex_scale_factors = [0_u8; MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT];
        complex_scale_factors[0] = 4;
        complex_scale_factors[7] = 2;
        let mut simple_scale_factors = [0_u8; MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT];
        simple_scale_factors[0] = 1;

        let previous = Layer3QualityGuardPerceptualCandidate {
            scale_factors: complex_scale_factors,
            quantized: vec![1, 0, -1],
            scalefac_scale: false,
            global_gain: 180,
            distortion: 12.0,
        };
        let same_distortion_simpler = Layer3QualityGuardPerceptualCandidate {
            scale_factors: simple_scale_factors,
            quantized: vec![1, 0, -1],
            scalefac_scale: false,
            global_gain: 180,
            distortion: 12.0,
        };
        let lower_distortion_complex = Layer3QualityGuardPerceptualCandidate {
            distortion: 11.9,
            ..previous.clone()
        };

        assert!(mpeg1_layer3_quality_guard_candidate_is_better(
            &previous,
            &same_distortion_simpler
        ));
        assert!(mpeg1_layer3_quality_guard_candidate_is_better(
            &same_distortion_simpler,
            &lower_distortion_complex
        ));
        assert!(!mpeg1_layer3_quality_guard_candidate_is_better(
            &same_distortion_simpler,
            &previous
        ));
    }

    #[test]
    fn default_nonzero_mono_encode_uses_bit_reservoir() {
        let frames = 8_usize;
        let samples_per_frame = 1152_usize;
        let mut samples = Vec::with_capacity(frames * samples_per_frame);
        for frame in 0..frames {
            let loud = frame % 2 == 0;
            for n in 0..samples_per_frame {
                let t = n as f32;
                samples.push(if loud {
                    0.3 * ((t * 0.043).sin()
                        + (t * 0.131).sin()
                        + (t * 0.277).sin()
                        + (t * 0.611).sin())
                } else {
                    0.02 * (t * 0.05).sin()
                });
            }
        }
        let pcm = AudioBuffer::new(44_100, 1, samples).unwrap();
        let stream = encode(&pcm).unwrap();

        let mut offset = 0_usize;
        let mut max_main_data_begin = 0_u32;
        while offset < stream.len() {
            let header = FrameHeader::parse(&stream[offset..offset + 4]).unwrap();
            let mut reader = BitReader::new(&stream[offset + 4..]);
            max_main_data_begin = max_main_data_begin.max(reader.read_bits(9).unwrap());
            offset += header.frame_len();
        }

        assert_eq!(offset, stream.len());
        assert!(
            max_main_data_begin > 0,
            "default nonzero mono MP3 encode never used the bit reservoir"
        );
    }

    #[test]
    fn default_nonzero_stereo_encode_uses_bit_reservoir() {
        let frames = 8_usize;
        let samples_per_frame = 1152_usize;
        let mut samples = Vec::with_capacity(frames * samples_per_frame * 2);
        for frame in 0..frames {
            let loud = frame % 2 == 0;
            for n in 0..samples_per_frame {
                let t = n as f32;
                let left = if loud {
                    0.28 * ((t * 0.037).sin() + (t * 0.149).sin() + (t * 0.419).sin())
                } else {
                    0.02 * (t * 0.041).sin()
                };
                let right = if loud {
                    0.24 * ((t * 0.053).sin() + (t * 0.173).sin() + (t * 0.337).sin())
                } else {
                    0.018 * (t * 0.047).sin()
                };
                samples.push(left);
                samples.push(right);
            }
        }
        let pcm = AudioBuffer::new(44_100, 2, samples).unwrap();
        let stream = encode(&pcm).unwrap();

        let mut offset = 0_usize;
        let mut max_main_data_begin = 0_u32;
        while offset < stream.len() {
            let header = FrameHeader::parse(&stream[offset..offset + 4]).unwrap();
            let mut reader = BitReader::new(&stream[offset + 4..]);
            max_main_data_begin = max_main_data_begin.max(reader.read_bits(9).unwrap());
            offset += header.frame_len();
        }

        assert_eq!(offset, stream.len());
        assert!(
            max_main_data_begin > 0,
            "default nonzero stereo MP3 encode never used the bit reservoir"
        );
    }

    #[test]
    fn decodes_own_silent_layer3_frames() {
        let pcm = AudioBuffer::new(44_100, 2, vec![0.0; 1153 * 2]).unwrap();
        let mp3 = encode(&pcm).unwrap();

        let decoded = decode(&mp3).unwrap();

        assert_eq!(decoded.sample_rate, 44_100);
        assert_eq!(decoded.channels, 2);
        assert_eq!(decoded.samples.len(), 1152 * 2 * 2);
        assert!(decoded.samples.iter().all(|sample| *sample == 0.0));
    }

    #[test]
    fn rejects_unknown_layer3_payload_for_decode() {
        let pcm = AudioBuffer::new(44_100, 1, vec![0.0; 1152]).unwrap();
        let mut mp3 = encode(&pcm).unwrap();
        *mp3.last_mut().unwrap() = 1;

        let err = decode(&mp3).unwrap_err();

        assert!(matches!(
            err,
            Error::UnsupportedFeature(
                "MP3 decode currently supports sonare silent MPEG-1 Layer III only"
            )
        ));
    }

    #[test]
    fn encodes_non_silent_pcm_as_layer3_scaffold() {
        let pcm = AudioBuffer::new(44_100, 1, vec![0.0, 0.25]).unwrap();
        let zero_payload =
            encode_mpeg1_layer3_pcm_frames_with_selected_scale_factors_and_table_provider(
                &pcm,
                f32::MAX,
                Layer3EntropyTableProvider::default(),
            )
            .unwrap();

        let mp3 = encode(&pcm).unwrap();
        let header = FrameHeader::parse(&mp3[..4]).unwrap();

        assert_eq!(detect(&mp3), Some(Format::Mp3));
        assert_eq!(header.version, MpegVersion::Mpeg1);
        assert_eq!(header.layer, Layer::Layer3);
        assert_eq!(header.channel_mode, ChannelMode::SingleChannel);
        assert_eq!(mp3.len(), header.frame_len());
        assert_ne!(mp3, zero_payload);
    }

    #[test]
    fn decodes_explicit_zero_payload_scaffold_as_zero_pcm() {
        let pcm = AudioBuffer::new(44_100, 1, vec![0.0, 0.25]).unwrap();
        let mp3 = encode_mpeg1_layer3_pcm_frames_with_selected_scale_factors_and_table_provider(
            &pcm,
            f32::MAX,
            Layer3EntropyTableProvider::default(),
        )
        .unwrap();

        let decoded = decode(&mp3).unwrap();

        assert_eq!(decoded.sample_rate, 44_100);
        assert_eq!(decoded.channels, 1);
        assert_eq!(decoded.samples.len(), 1152);
        assert!(decoded.samples.iter().all(|sample| *sample == 0.0));
    }

    #[test]
    fn rejects_unsupported_encode_shape() {
        let pcm = AudioBuffer::new(44_100, 3, vec![0.0; 3]).unwrap();

        let err = encode(&pcm).unwrap_err();

        assert!(matches!(
            err,
            Error::UnsupportedFeature("MP3 encode currently supports mono/stereo only")
        ));

        // 22_000 Hz is not an MPEG-1, MPEG-2 LSF, or MPEG-2.5 Layer III rate.
        // (22_050 Hz is now supported via the MPEG-2 LSF path.)
        let pcm = AudioBuffer::new(22_000, 1, vec![0.0; 576]).unwrap();
        let err = encode(&pcm).unwrap_err();

        assert!(matches!(err, Error::UnsupportedFeature("MP3 sample rate")));
    }

    /// ISO/IEC 11172-3 §2.4.3.4 long-block requantization with zero scale
    /// factors and zero preflag: `xr = sign(is)·|is|^(4/3)·2^((global_gain−210)/4)`.
    fn requantize_long_line(is: i32, global_gain: u8) -> f32 {
        let sign = if is < 0 { -1.0 } else { 1.0 };
        let magnitude = (is.unsigned_abs() as f32).powf(4.0 / 3.0);
        let gain = 2.0_f32.powf(0.25 * (f32::from(global_gain) - 210.0));
        sign * magnitude * gain
    }

    #[test]
    fn global_gain_for_step_inverts_the_quantizer_step() {
        // At step == 1 the gain is the ISO reference value, and each octave of
        // step shifts the gain by 16/3 quarter-dB units.
        assert_eq!(mpeg1_layer3_global_gain_for_step(1.0), 210);
        assert_eq!(mpeg1_layer3_global_gain_for_step(2.0), 215);
        assert_eq!(mpeg1_layer3_global_gain_for_step(0.5), 205);
        // Degenerate steps fall back to the reference gain instead of panicking.
        assert_eq!(mpeg1_layer3_global_gain_for_step(0.0), 210);
        assert_eq!(mpeg1_layer3_global_gain_for_step(-1.0), 210);
        assert_eq!(mpeg1_layer3_global_gain_for_step(f32::NAN), 210);
        // The gain stays inside the 8-bit syntax range for extreme steps.
        assert_eq!(mpeg1_layer3_global_gain_for_step(f32::MIN_POSITIVE), 0);
        assert_eq!(mpeg1_layer3_global_gain_for_step(1.0e30), 255);
    }

    #[test]
    fn calibrated_gain_requantizes_the_long_block_spectrum() {
        // A non-periodic frequency sweep exercises every scale-factor band. The
        // encoder quantizes the (sign-inverted) spectrum, so the ISO
        // requantization with the calibrated gain and zero scale factors must
        // reconstruct that same signal within quantization noise (positive
        // SNR), and finer steps must not regress.
        let samples: Vec<f32> = (0..2304)
            .map(|n| {
                let t = n as f32 / 44_100.0;
                let f = 200.0 + 6000.0 * t;
                0.6 * (std::f32::consts::TAU * f * t).sin()
            })
            .collect();
        let pcm = AudioBuffer::new(44_100, 1, samples).unwrap();
        let spectrum = layer3_long_block_spectrum(&pcm, 0, 576).unwrap();

        let mut previous_snr = f64::NEG_INFINITY;
        for &step in &[1.0_f32, 0.25, 0.05] {
            let global_gain = mpeg1_layer3_global_gain_for_step(step);
            let quantized = quantize_pcm_long_block(&pcm, 0, 576, step).unwrap();

            let mut signal = 0.0_f64;
            let mut noise = 0.0_f64;
            for (&line, &is) in spectrum.iter().zip(quantized.iter()) {
                // The encoder quantizes the negated spectrum.
                let reference = f64::from(-line);
                let reconstructed = f64::from(requantize_long_line(is, global_gain));
                signal += reference * reference;
                let error = reconstructed - reference;
                noise += error * error;
            }

            let snr = 10.0 * (signal / noise.max(1.0e-30)).log10();
            assert!(
                snr > 10.0,
                "step {step} reconstruction SNR too low: {snr} dB"
            );
            assert!(
                snr >= previous_snr - 0.5,
                "finer step {step} regressed SNR: {snr} dB vs {previous_snr} dB"
            );
            previous_snr = snr;
        }
    }
