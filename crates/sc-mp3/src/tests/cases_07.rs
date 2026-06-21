    use crate::reservoir::{
        collect_mpeg1_layer3_entropy_targeted_reservoir_frames_with_table_provider,
        encode_mpeg2_layer3_pcm_frames_with_auto_step_mid_side_and_table_provider,
        mid_side_encode_buffer, should_encode_stereo_as_mid_side, Layer3ReservoirPayloadMode,
    };

    /// Builds an interleaved stereo `AudioBuffer` from separate channel slices.
    fn interleave_stereo(sample_rate: u32, left: &[f32], right: &[f32]) -> AudioBuffer {
        let samples: Vec<f32> = left
            .iter()
            .zip(right.iter())
            .flat_map(|(&l, &r)| [l, r])
            .collect();
        AudioBuffer::new(sample_rate, 2, samples).unwrap()
    }

    /// A modest correlated stereo signal (right is a scaled copy of left) whose
    /// side channel is small, so the mid/side decision selects MS.
    fn correlated_stereo(sample_rate: u32, frames: usize) -> AudioBuffer {
        let left: Vec<f32> = (0..frames)
            .map(|i| {
                let t = i as f32 / sample_rate as f32;
                0.25 * (std::f32::consts::TAU * 700.0 * t).sin()
                    + 0.15 * (std::f32::consts::TAU * 1_900.0 * t).sin()
            })
            .collect();
        let right: Vec<f32> = left.iter().map(|&l| 0.6 * l).collect();
        interleave_stereo(sample_rate, &left, &right)
    }

    #[test]
    fn joint_stereo_header_sets_ms_mode_extension() {
        // Joint stereo must serialize channel-mode bits 0b01 and mode_extension
        // bits 0b10 (ms_stereo on, intensity off): byte 3 == (0b01<<6)|(0b10<<4).
        let header = FrameHeader {
            version: MpegVersion::Mpeg1,
            layer: Layer::Layer3,
            protection_absent: true,
            bitrate_kbps: 128,
            sample_rate: 44_100,
            padding: false,
            channel_mode: ChannelMode::JointStereo,
        };
        assert_eq!(header.to_bytes().unwrap()[3], 0b0110_0000);

        // Independent stereo leaves mode_extension zero.
        let stereo = FrameHeader {
            channel_mode: ChannelMode::Stereo,
            ..header
        };
        assert_eq!(stereo.to_bytes().unwrap()[3], 0b0000_0000);
    }

    #[test]
    fn mid_side_encode_buffer_inverts_to_left_right() {
        // The orthonormal mid/side transform must be exactly invertible: applying
        // the decoder matrix L=(M+S)/√2, R=(M-S)/√2 recovers the input pair.
        let sample_rate = 44_100;
        let left: Vec<f32> = (0..512)
            .map(|i| 0.4 * (std::f32::consts::TAU * 500.0 * (i as f32 / sample_rate as f32)).sin())
            .collect();
        let right: Vec<f32> = (0..512)
            .map(|i| 0.3 * (std::f32::consts::TAU * 900.0 * (i as f32 / sample_rate as f32)).sin())
            .collect();
        let pcm = interleave_stereo(sample_rate, &left, &right);

        let mid_side = mid_side_encode_buffer(&pcm).unwrap();
        assert_eq!(mid_side.channels, 2);
        assert_eq!(mid_side.samples.len(), pcm.samples.len());

        let scale = std::f32::consts::FRAC_1_SQRT_2;
        for (i, frame) in mid_side.samples.chunks_exact(2).enumerate() {
            let (mid, side) = (frame[0], frame[1]);
            let recovered_left = (mid + side) * scale;
            let recovered_right = (mid - side) * scale;
            assert!((recovered_left - left[i]).abs() < 1.0e-5, "left[{i}]");
            assert!((recovered_right - right[i]).abs() < 1.0e-5, "right[{i}]");
        }
    }

    #[test]
    fn mid_side_decision_tracks_channel_correlation() {
        let sample_rate = 44_100;
        let frames = 4_096;

        // Correlated channels: side energy is small, so MS is chosen.
        assert!(should_encode_stereo_as_mid_side(&correlated_stereo(sample_rate, frames)).unwrap());

        // Decorrelated channels (distinct tones): side energy is large, stay L/R.
        let left: Vec<f32> = (0..frames)
            .map(|i| 0.4 * (std::f32::consts::TAU * 440.0 * (i as f32 / sample_rate as f32)).sin())
            .collect();
        let right: Vec<f32> = (0..frames)
            .map(|i| 0.4 * (std::f32::consts::TAU * 3_100.0 * (i as f32 / sample_rate as f32)).sin())
            .collect();
        let decorrelated = interleave_stereo(sample_rate, &left, &right);
        assert!(!should_encode_stereo_as_mid_side(&decorrelated).unwrap());

        // Mono is never mid/side.
        let mono = AudioBuffer::new(sample_rate, 1, vec![0.1_f32; frames]).unwrap();
        assert!(!should_encode_stereo_as_mid_side(&mono).unwrap());
    }

    #[test]
    fn mpeg2_lsf_mid_side_encode_marks_joint_stereo_and_is_size_compatible() {
        // The MPEG-2 LSF MS path builds a JointStereo header directly and codes the
        // M/S buffer through the single-granule auto-step assembler. Every frame's
        // channel-mode bits must read joint stereo (0b01), and the byte length must
        // match the independent-stereo encode of the same M/S buffer, proving the
        // joint-stereo header is layout-compatible at LSF rates.
        let sample_rate = 24_000;
        let frames = 4 * 576;
        let pcm = correlated_stereo(sample_rate, frames);
        let mid_side = mid_side_encode_buffer(&pcm).unwrap();
        let provider = mpeg1_layer3_standard_table_provider();
        let candidates = crate::MPEG1_LAYER3_PCM_STEP_CANDIDATES;
        let bitrate = crate::MPEG2_LAYER3_DEFAULT_BITRATE_KBPS;

        let ms = encode_mpeg2_layer3_pcm_frames_with_auto_step_mid_side_and_table_provider(
            &pcm, bitrate, candidates, provider,
        )
        .unwrap();
        let independent =
            crate::encode_mpeg2_layer3_pcm_frames_with_auto_step_and_table_provider(
                &mid_side, bitrate, candidates, provider,
            )
            .unwrap();

        assert_eq!(
            ms.len(),
            independent.len(),
            "joint-stereo header must be byte-size compatible with independent stereo"
        );
        assert!(!ms.is_empty(), "expected encoded frames");
        assert_eq!(ms[0], 0xff, "missing frame sync");
        assert_eq!((ms[3] >> 6) & 0x03, 0b01, "first frame must be joint stereo");
    }

    #[test]
    fn mid_side_uses_fewer_main_data_bits_than_independent() {
        // The point of MS joint stereo at constant bitrate: the near-silent side
        // channel costs far fewer bits than coding both channels independently,
        // freeing reservoir headroom for the mid channel. Compare the total
        // main-data payload bits of the two encodings of the same correlated
        // signal at the same bitrate and quantizer-step set.
        let sample_rate = 44_100;
        let frames = 8 * 1152;
        let pcm = correlated_stereo(sample_rate, frames);
        let candidates = crate::mpeg1_layer3_production_pcm_step_candidates(2).unwrap();
        let provider = mpeg1_layer3_standard_table_provider();

        let sum_payload_bits = |buffer: &AudioBuffer| -> usize {
            collect_mpeg1_layer3_entropy_targeted_reservoir_frames_with_table_provider(
                buffer,
                candidates,
                128,
                false,
                0,
                provider,
                Layer3ReservoirPayloadMode::PerceptualActive,
            )
            .unwrap()
            .iter()
            .map(|(frame, _, _)| frame.payload_bit_len)
            .sum()
        };

        let independent_bits = sum_payload_bits(&pcm);
        let mid_side_bits = sum_payload_bits(&mid_side_encode_buffer(&pcm).unwrap());

        assert!(
            mid_side_bits < independent_bits,
            "MS should use fewer main-data bits: ms={mid_side_bits} indep={independent_bits}"
        );
    }
