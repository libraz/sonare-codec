use super::*;

#[cfg(test)]
#[allow(clippy::module_inception)]
mod tests {
    use super::{
        crc16, crc8, decode, decode_subframe, parse_frame_header, parse_streaminfo,
        BlockingStrategy, ChannelAssignment, FlacDecoder, FlacEncoder, StreamInfo,
    };
    use sc_core::{AudioBuffer, Decoder, Encoder};

    #[test]
    fn parses_streaminfo() {
        let flac = flac_with_streaminfo(StreamInfoFixture {
            min_block_size: 4096,
            max_block_size: 4096,
            min_frame_size: 0,
            max_frame_size: 0,
            sample_rate: 48_000,
            channels: 2,
            bits_per_sample: 16,
            total_samples: 1234,
        });
        let info = parse_streaminfo(&flac).unwrap();

        assert_eq!(info.min_block_size, 4096);
        assert_eq!(info.max_block_size, 4096);
        assert_eq!(info.sample_rate, 48_000);
        assert_eq!(info.channels, 2);
        assert_eq!(info.bits_per_sample, 16);
        assert_eq!(info.total_samples, 1234);
    }

    #[test]
    fn rejects_missing_streaminfo() {
        assert!(parse_streaminfo(b"fLaC\x81\0\0\0").is_err());
    }

    #[test]
    fn parses_frame_header() {
        let info = test_streaminfo();
        let header =
            parse_frame_header(&frame_header([0xff, 0xf8, 0x1a, 0x18, 0x00]), &info).unwrap();

        assert_eq!(header.blocking_strategy, BlockingStrategy::FixedBlockSize);
        assert_eq!(header.block_size, 192);
        assert_eq!(header.sample_rate, 48_000);
        assert_eq!(header.channel_assignment, ChannelAssignment::Independent(2));
        assert_eq!(header.bits_per_sample, 16);
        assert_eq!(header.frame_or_sample_number, 0);
        assert_eq!(header.header_len, 6);
    }

    #[test]
    fn decodes_constant_subframe() {
        let (samples, bytes_read) = decode_subframe(&[0x00, 0x12, 0x34], 4, 16).unwrap();

        assert_eq!(samples, vec![0x1234; 4]);
        assert_eq!(bytes_read, 3);
    }

    #[test]
    fn decodes_verbatim_subframe() {
        let (samples, bytes_read) =
            decode_subframe(&[0x02, 0x00, 0x01, 0xff, 0xff], 2, 16).unwrap();

        assert_eq!(samples, vec![1, -1]);
        assert_eq!(bytes_read, 5);
    }

    #[test]
    fn decodes_fixed_subframe_with_rice_residual() {
        let subframe = fixed_order_one_subframe();
        let (samples, _bytes_read) = decode_subframe(&subframe, 4, 16).unwrap();

        assert_eq!(samples, vec![10, 11, 13, 16]);
    }

    #[test]
    fn decodes_lpc_subframe_with_rice_residual() {
        let subframe = lpc_order_one_subframe();
        let (samples, _bytes_read) = decode_subframe(&subframe, 4, 16).unwrap();

        assert_eq!(samples, vec![10, 11, 13, 16]);
    }

    #[test]
    fn decodes_single_constant_frame() {
        let mut flac = flac_with_streaminfo(StreamInfoFixture {
            min_block_size: 192,
            max_block_size: 192,
            min_frame_size: 0,
            max_frame_size: 0,
            sample_rate: 48_000,
            channels: 1,
            bits_per_sample: 16,
            total_samples: 192,
        });
        flac.extend_from_slice(&flac_frame(
            &[0xff, 0xf8, 0x1a, 0x08, 0x00],
            &[0x00, 0x40, 0x00],
        ));

        let decoded = decode(&flac).unwrap();

        assert_eq!(decoded.sample_rate, 48_000);
        assert_eq!(decoded.channels, 1);
        assert_eq!(decoded.frames(), 192);
        assert_eq!(decoded.samples[0], 16_384.0 / 32_767.0);
    }

    #[test]
    fn encodes_verbatim_flac_roundtrip() {
        let pcm = AudioBuffer::new(
            48_000,
            2,
            vec![-1.0, 1.0, -0.5, 0.5, 0.0, 0.25, 0.75, -0.25, 0.125, -0.125],
        )
        .unwrap();

        let flac = super::encode(&pcm).unwrap();
        let decoded = decode(&flac).unwrap();

        assert_eq!(decoded.sample_rate, 48_000);
        assert_eq!(decoded.channels, 2);
        assert_eq!(decoded.frames(), 5);
        assert_pcm_close(&decoded.samples, &pcm.samples, 1.0 / 32_767.0);
    }

    #[test]
    fn encodes_24bit_flac_roundtrip_with_finer_precision_than_16bit() {
        use super::{encode_as, FlacBitDepth};

        // A staircase with sub-16-bit-LSB steps: 16-bit quantization collapses
        // adjacent samples together, 24-bit preserves them.
        let samples: Vec<f32> = (0..256).map(|i| i as f32 / 200_000.0).collect();
        let pcm = AudioBuffer::new(48_000, 1, samples.clone()).unwrap();

        let flac24 = encode_as(&pcm, FlacBitDepth::Bits24).unwrap();
        let info = parse_streaminfo(&flac24).unwrap();
        assert_eq!(info.bits_per_sample, 24);

        let decoded24 = decode(&flac24).unwrap();
        assert_eq!(decoded24.frames(), 256);
        // 24-bit step is 1/(2^23 - 1); reconstruction must be within one LSB.
        assert_pcm_close(&decoded24.samples, &samples, 1.0 / 8_388_607.0);

        // The 24-bit reconstruction is strictly closer to the input than 16-bit.
        let decoded16 = decode(&encode_as(&pcm, FlacBitDepth::Bits16).unwrap()).unwrap();
        let err24: f64 = decoded24
            .samples
            .iter()
            .zip(&samples)
            .map(|(d, s)| f64::from((d - s).abs()))
            .sum();
        let err16: f64 = decoded16
            .samples
            .iter()
            .zip(&samples)
            .map(|(d, s)| f64::from((d - s).abs()))
            .sum();
        assert!(
            err24 < err16,
            "24-bit err {err24} not below 16-bit err {err16}"
        );
    }

    #[test]
    fn encoder_uses_fixed_rice_subframe_for_smooth_pcm() {
        let samples = (0..128)
            .map(|sample| sample as f32 / 32_767.0)
            .collect::<Vec<_>>();
        let pcm = AudioBuffer::new(48_000, 1, samples).unwrap();

        let flac = super::encode(&pcm).unwrap();
        let info = parse_streaminfo(&flac).unwrap();
        let header = parse_frame_header(&flac[42..], &info).unwrap();
        let subframe_type = flac[42 + header.header_len] >> 1;
        let decoded = decode(&flac).unwrap();

        assert_eq!(subframe_type, 10);
        assert_eq!(decoded.frames(), 128);
        assert_pcm_close(&decoded.samples, &pcm.samples, 1.0 / 32_767.0);
    }

    #[test]
    fn encoder_uses_constant_subframe_for_constant_pcm() {
        let pcm = AudioBuffer::new(48_000, 1, vec![0.25; 64]).unwrap();

        let flac = super::encode(&pcm).unwrap();
        let info = parse_streaminfo(&flac).unwrap();
        let header = parse_frame_header(&flac[42..], &info).unwrap();
        let subframe_type = flac[42 + header.header_len] >> 1;
        let decoded = decode(&flac).unwrap();

        assert_eq!(subframe_type, 0);
        assert_eq!(decoded.frames(), 64);
        assert_pcm_close(&decoded.samples, &pcm.samples, 1.0 / 32_767.0);
    }

    #[test]
    fn encoder_can_choose_fixed_predictor_orders_two_through_four() {
        assert_encoded_fixed_order((0..128).map(|sample| sample * 64).collect::<Vec<_>>(), 2);
        assert_encoded_fixed_order(
            (0..96)
                .map(|sample| {
                    let centered = sample - 48;
                    centered * centered * 8
                })
                .collect::<Vec<_>>(),
            3,
        );
        assert_encoded_fixed_order(
            (0..48)
                .map(|sample| {
                    let centered = sample - 24;
                    centered * centered * centered
                })
                .collect::<Vec<_>>(),
            4,
        );
    }

    #[test]
    fn encoder_can_choose_stereo_decorrelation() {
        let mut samples = Vec::new();
        for sample in 0..128 {
            let value = sample as f32 / 32_767.0;
            samples.push(value);
            samples.push(value);
        }
        let pcm = AudioBuffer::new(48_000, 2, samples).unwrap();

        let flac = super::encode(&pcm).unwrap();
        let info = parse_streaminfo(&flac).unwrap();
        let header = parse_frame_header(&flac[42..], &info).unwrap();
        let decoded = decode(&flac).unwrap();

        assert!(matches!(
            header.channel_assignment,
            ChannelAssignment::LeftSide | ChannelAssignment::RightSide | ChannelAssignment::MidSide
        ));
        assert_eq!(decoded.frames(), 128);
        assert_pcm_close(&decoded.samples, &pcm.samples, 1.0 / 32_767.0);
    }

    #[test]
    fn streaminfo_min_block_size_excludes_partial_final_block() {
        // A multi-frame stream whose final block is a 1-sample remainder. Per RFC 9639
        // §8.2 the STREAMINFO minimum block size excludes the final block, so it must be
        // the nominal 4096 (matching the reference encoder) rather than a clamped `.max(16)`
        // value or the true 1-sample tail.
        let samples = (0..4097)
            .map(|sample| (sample as f32 * 0.01).sin() * 0.5)
            .collect::<Vec<_>>();
        let pcm = AudioBuffer::new(44_100, 1, samples).unwrap();

        let flac = super::encode(&pcm).unwrap();
        let info = parse_streaminfo(&flac).unwrap();
        let header = parse_frame_header(&flac[42..], &info).unwrap();

        assert_eq!(info.min_block_size, 4096);
        assert_eq!(info.max_block_size, 4096);
        assert_eq!(header.blocking_strategy, BlockingStrategy::FixedBlockSize);

        let decoded = decode(&flac).unwrap();
        assert_eq!(decoded.frames(), 4097);
    }

    #[test]
    fn streaminfo_block_size_matches_single_short_frame() {
        // A stream shorter than the nominal block size is a single frame; STREAMINFO
        // must report that frame's true size for both minimum and maximum, never a
        // clamped `.max(16)` value.
        let pcm = AudioBuffer::new(44_100, 1, vec![0.1, -0.2, 0.3, -0.4, 0.5]).unwrap();

        let flac = super::encode(&pcm).unwrap();
        let info = parse_streaminfo(&flac).unwrap();

        assert_eq!(info.min_block_size, 5);
        assert_eq!(info.max_block_size, 5);

        let decoded = decode(&flac).unwrap();
        assert_eq!(decoded.frames(), 5);
    }

    #[test]
    fn encoder_trait_encodes_flac() {
        let pcm = AudioBuffer::new(44_100, 1, vec![0.0, 0.25, -0.25]).unwrap();
        let mut encoder = FlacEncoder::new();
        let flac = encoder.encode(&pcm).unwrap();

        let decoded = decode(&flac).unwrap();

        assert_eq!(decoded.sample_rate, 44_100);
        assert_eq!(decoded.channels, 1);
        assert_pcm_close(&decoded.samples, &pcm.samples, 1.0 / 32_767.0);
    }

    #[test]
    fn rejects_empty_flac_encode() {
        let pcm = AudioBuffer::new(48_000, 1, Vec::new()).unwrap();

        assert!(super::encode(&pcm).is_err());
    }

    #[test]
    fn decodes_32_bit_constant_frame() {
        let mut flac = flac_with_streaminfo(StreamInfoFixture {
            min_block_size: 2,
            max_block_size: 2,
            min_frame_size: 0,
            max_frame_size: 0,
            sample_rate: 48_000,
            channels: 1,
            bits_per_sample: 32,
            total_samples: 2,
        });
        let mut writer = BitWriter::new();
        writer.write_constant_subframe(1_073_741_824, 32);
        flac.extend_from_slice(&flac_frame(
            &[0xff, 0xf8, 0x6a, 0x0e, 0x00, 0x01],
            &writer.finish(),
        ));

        let decoded = decode(&flac).unwrap();

        assert_eq!(decoded.channels, 1);
        assert_eq!(decoded.frames(), 2);
        assert_eq!(
            decoded.samples,
            vec![
                1_073_741_824.0 / 2_147_483_647.0,
                1_073_741_824.0 / 2_147_483_647.0,
            ]
        );
    }

    #[test]
    fn stream_decode_buffers_until_complete_stream() {
        let mut flac = flac_with_streaminfo(StreamInfoFixture {
            min_block_size: 192,
            max_block_size: 192,
            min_frame_size: 0,
            max_frame_size: 0,
            sample_rate: 48_000,
            channels: 1,
            bits_per_sample: 16,
            total_samples: 192,
        });
        flac.extend_from_slice(&flac_frame(
            &[0xff, 0xf8, 0x1a, 0x08, 0x00],
            &[0x00, 0x40, 0x00],
        ));
        let split = flac.len() - 2;
        let mut decoder = FlacDecoder::new();

        assert!(decoder.decode_stream(&flac[..split]).unwrap().is_none());
        let decoded = decoder
            .decode_stream(&flac[split..])
            .unwrap()
            .expect("complete stream should decode");

        assert_eq!(decoded.frames(), 192);
        assert_eq!(decoded.samples[0], 16_384.0 / 32_767.0);
    }

    #[test]
    fn rejects_bad_frame_header_crc() {
        let info = test_streaminfo();
        let mut header = frame_header([0xff, 0xf8, 0x1a, 0x18, 0x00]);
        let last = header.len() - 1;
        header[last] ^= 0x01;

        assert!(parse_frame_header(&header, &info).is_err());
    }

    #[test]
    fn parses_seven_byte_coded_sample_number() {
        let mut flac = flac_with_streaminfo(StreamInfoFixture {
            min_block_size: 2,
            max_block_size: 2,
            min_frame_size: 0,
            max_frame_size: 0,
            sample_rate: 48_000,
            channels: 1,
            bits_per_sample: 16,
            total_samples: 0,
        });
        let sample_number = 0x000f_ffff_ffff_u64;
        let mut header_without_crc = vec![0xff, 0xf9, 0x6a, 0x08];
        header_without_crc.extend_from_slice(&utf8_coded_number(sample_number));
        header_without_crc.push(0x01);
        flac.extend_from_slice(&flac_frame(&header_without_crc, &[0x00, 0x00, 0x00]));
        let stream_info = parse_streaminfo(&flac).unwrap();
        let header = parse_frame_header(&flac[42..], &stream_info).unwrap();

        assert_eq!(
            header.blocking_strategy,
            BlockingStrategy::VariableBlockSize
        );
        assert_eq!(header.frame_or_sample_number, sample_number);
    }

    #[test]
    fn decodes_multiple_constant_frames() {
        let mut flac = flac_with_streaminfo(StreamInfoFixture {
            min_block_size: 2,
            max_block_size: 2,
            min_frame_size: 0,
            max_frame_size: 0,
            sample_rate: 48_000,
            channels: 1,
            bits_per_sample: 16,
            total_samples: 4,
        });
        flac.extend_from_slice(&single_channel_constant_frame(0, 10));
        flac.extend_from_slice(&single_channel_constant_frame(1, 20));

        let decoded = decode(&flac).unwrap();

        assert_eq!(decoded.sample_rate, 48_000);
        assert_eq!(decoded.channels, 1);
        assert_eq!(decoded.frames(), 4);
        assert_eq!(
            decoded.samples,
            vec![
                10.0 / 32_767.0,
                10.0 / 32_767.0,
                20.0 / 32_767.0,
                20.0 / 32_767.0,
            ]
        );
    }

    #[test]
    fn validates_streaminfo_md5_when_present() {
        let mut flac = flac_with_streaminfo_and_md5(
            StreamInfoFixture {
                min_block_size: 2,
                max_block_size: 2,
                min_frame_size: 0,
                max_frame_size: 0,
                sample_rate: 48_000,
                channels: 1,
                bits_per_sample: 16,
                total_samples: 2,
            },
            [
                0x8e, 0x20, 0xe9, 0x73, 0x99, 0x77, 0xbd, 0x6e, 0x89, 0x1e, 0xd7, 0x2b, 0x1a, 0x2a,
                0xde, 0xa0,
            ],
        );
        flac.extend_from_slice(&single_channel_constant_frame(0, 10));

        let decoded = decode(&flac).unwrap();

        assert_eq!(decoded.frames(), 2);
    }

    #[test]
    fn rejects_streaminfo_md5_mismatch() {
        let mut flac = flac_with_streaminfo_and_md5(
            StreamInfoFixture {
                min_block_size: 2,
                max_block_size: 2,
                min_frame_size: 0,
                max_frame_size: 0,
                sample_rate: 48_000,
                channels: 1,
                bits_per_sample: 16,
                total_samples: 2,
            },
            [0xff; 16],
        );
        flac.extend_from_slice(&single_channel_constant_frame(0, 10));

        assert!(decode(&flac).is_err());
    }

    #[test]
    fn rejects_non_monotonic_frame_numbers() {
        let mut flac = flac_with_streaminfo(StreamInfoFixture {
            min_block_size: 2,
            max_block_size: 2,
            min_frame_size: 0,
            max_frame_size: 0,
            sample_rate: 48_000,
            channels: 1,
            bits_per_sample: 16,
            total_samples: 4,
        });
        flac.extend_from_slice(&single_channel_constant_frame(0, 10));
        flac.extend_from_slice(&single_channel_constant_frame(2, 20));

        assert!(decode(&flac).is_err());
    }

    #[test]
    fn rejects_non_final_frame_block_size_below_streaminfo_minimum() {
        let mut flac = flac_with_streaminfo(StreamInfoFixture {
            min_block_size: 4,
            max_block_size: 4,
            min_frame_size: 0,
            max_frame_size: 0,
            sample_rate: 48_000,
            channels: 1,
            bits_per_sample: 16,
            total_samples: 4,
        });
        flac.extend_from_slice(&single_channel_constant_frame(0, 10));
        flac.extend_from_slice(&single_channel_constant_frame(1, 20));

        assert!(decode(&flac).is_err());
    }

    #[test]
    fn allows_final_frame_block_size_below_streaminfo_minimum() {
        let mut flac = flac_with_streaminfo(StreamInfoFixture {
            min_block_size: 4,
            max_block_size: 4,
            min_frame_size: 0,
            max_frame_size: 0,
            sample_rate: 48_000,
            channels: 1,
            bits_per_sample: 16,
            total_samples: 2,
        });
        flac.extend_from_slice(&single_channel_constant_frame(0, 10));

        let decoded = decode(&flac).unwrap();

        assert_eq!(decoded.frames(), 2);
    }

    #[test]
    fn rejects_frame_size_below_streaminfo_minimum() {
        let mut flac = flac_with_streaminfo(StreamInfoFixture {
            min_block_size: 2,
            max_block_size: 2,
            min_frame_size: 13,
            max_frame_size: 0,
            sample_rate: 48_000,
            channels: 1,
            bits_per_sample: 16,
            total_samples: 2,
        });
        flac.extend_from_slice(&single_channel_constant_frame(0, 10));

        assert!(decode(&flac).is_err());
    }

    #[test]
    fn rejects_frame_size_above_streaminfo_maximum() {
        let mut flac = flac_with_streaminfo(StreamInfoFixture {
            min_block_size: 2,
            max_block_size: 2,
            min_frame_size: 0,
            max_frame_size: 7,
            sample_rate: 48_000,
            channels: 1,
            bits_per_sample: 16,
            total_samples: 2,
        });
        flac.extend_from_slice(&single_channel_constant_frame(0, 10));

        assert!(decode(&flac).is_err());
    }

    #[test]
    fn rejects_total_sample_count_mismatch() {
        let mut flac = flac_with_streaminfo(StreamInfoFixture {
            min_block_size: 2,
            max_block_size: 2,
            min_frame_size: 0,
            max_frame_size: 0,
            sample_rate: 48_000,
            channels: 1,
            bits_per_sample: 16,
            total_samples: 3,
        });
        flac.extend_from_slice(&single_channel_constant_frame(0, 10));

        assert!(decode(&flac).is_err());
    }

    #[test]
    fn decodes_single_fixed_frame() {
        let mut flac = flac_with_streaminfo(StreamInfoFixture {
            min_block_size: 4,
            max_block_size: 4,
            min_frame_size: 0,
            max_frame_size: 0,
            sample_rate: 48_000,
            channels: 1,
            bits_per_sample: 16,
            total_samples: 4,
        });
        flac.extend_from_slice(&flac_frame(
            &[0xff, 0xf8, 0x6a, 0x08, 0x00, 0x03],
            &fixed_order_one_subframe(),
        ));

        let decoded = decode(&flac).unwrap();

        assert_eq!(decoded.sample_rate, 48_000);
        assert_eq!(decoded.channels, 1);
        assert_eq!(decoded.frames(), 4);
        assert_eq!(
            decoded.samples,
            vec![
                10.0 / 32_767.0,
                11.0 / 32_767.0,
                13.0 / 32_767.0,
                16.0 / 32_767.0,
            ]
        );
    }

    #[test]
    fn decodes_single_lpc_frame() {
        let mut flac = flac_with_streaminfo(StreamInfoFixture {
            min_block_size: 4,
            max_block_size: 4,
            min_frame_size: 0,
            max_frame_size: 0,
            sample_rate: 48_000,
            channels: 1,
            bits_per_sample: 16,
            total_samples: 4,
        });
        flac.extend_from_slice(&flac_frame(
            &[0xff, 0xf8, 0x6a, 0x08, 0x00, 0x03],
            &lpc_order_one_subframe(),
        ));

        let decoded = decode(&flac).unwrap();

        assert_eq!(decoded.sample_rate, 48_000);
        assert_eq!(decoded.channels, 1);
        assert_eq!(decoded.frames(), 4);
        assert_eq!(
            decoded.samples,
            vec![
                10.0 / 32_767.0,
                11.0 / 32_767.0,
                13.0 / 32_767.0,
                16.0 / 32_767.0,
            ]
        );
    }

    #[test]
    fn rejects_bad_frame_footer_crc() {
        let mut flac = flac_with_streaminfo(StreamInfoFixture {
            min_block_size: 2,
            max_block_size: 2,
            min_frame_size: 0,
            max_frame_size: 0,
            sample_rate: 48_000,
            channels: 1,
            bits_per_sample: 16,
            total_samples: 2,
        });
        flac.extend_from_slice(&single_channel_constant_frame(0, 10));
        let last = flac.len() - 1;
        flac[last] ^= 0x01;

        assert!(decode(&flac).is_err());
    }

    #[test]
    fn decodes_left_side_stereo_frame() {
        let decoded = decode(&stereo_constant_flac(0x88, 20, 5, 16, 17)).unwrap();

        assert_eq!(decoded.channels, 2);
        assert_eq!(decoded.frames(), 2);
        assert_eq!(
            decoded.samples,
            vec![
                20.0 / 32_767.0,
                15.0 / 32_767.0,
                20.0 / 32_767.0,
                15.0 / 32_767.0,
            ]
        );
    }

    #[test]
    fn decodes_right_side_stereo_frame() {
        let decoded = decode(&stereo_constant_flac(0x98, 5, 15, 17, 16)).unwrap();

        assert_eq!(decoded.channels, 2);
        assert_eq!(decoded.frames(), 2);
        assert_eq!(
            decoded.samples,
            vec![
                20.0 / 32_767.0,
                15.0 / 32_767.0,
                20.0 / 32_767.0,
                15.0 / 32_767.0,
            ]
        );
    }

    #[test]
    fn decodes_mid_side_stereo_frame() {
        let decoded = decode(&stereo_constant_flac(0xa8, 17, 6, 16, 17)).unwrap();

        assert_eq!(decoded.channels, 2);
        assert_eq!(decoded.frames(), 2);
        assert_eq!(
            decoded.samples,
            vec![
                20.0 / 32_767.0,
                14.0 / 32_767.0,
                20.0 / 32_767.0,
                14.0 / 32_767.0,
            ]
        );
    }

    struct StreamInfoFixture {
        min_block_size: u16,
        max_block_size: u16,
        min_frame_size: u32,
        max_frame_size: u32,
        sample_rate: u32,
        channels: u8,
        bits_per_sample: u8,
        total_samples: u64,
    }

    fn flac_with_streaminfo(fixture: StreamInfoFixture) -> Vec<u8> {
        flac_with_streaminfo_and_md5(fixture, [0; 16])
    }

    fn flac_with_streaminfo_and_md5(fixture: StreamInfoFixture, md5: [u8; 16]) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(b"fLaC");
        out.push(0x80);
        out.extend_from_slice(&34_u32.to_be_bytes()[1..4]);
        out.extend_from_slice(&fixture.min_block_size.to_be_bytes());
        out.extend_from_slice(&fixture.max_block_size.to_be_bytes());
        out.extend_from_slice(&fixture.min_frame_size.to_be_bytes()[1..4]);
        out.extend_from_slice(&fixture.max_frame_size.to_be_bytes()[1..4]);

        let packed = (u64::from(fixture.sample_rate) << 44)
            | (u64::from(fixture.channels - 1) << 41)
            | (u64::from(fixture.bits_per_sample - 1) << 36)
            | fixture.total_samples;
        out.extend_from_slice(&packed.to_be_bytes());
        out.extend_from_slice(&md5);
        out
    }

    fn test_streaminfo() -> StreamInfo {
        parse_streaminfo(&flac_with_streaminfo(StreamInfoFixture {
            min_block_size: 192,
            max_block_size: 4096,
            min_frame_size: 0,
            max_frame_size: 0,
            sample_rate: 48_000,
            channels: 2,
            bits_per_sample: 16,
            total_samples: 0,
        }))
        .unwrap()
    }

    fn fixed_order_one_subframe() -> Vec<u8> {
        let mut writer = BitWriter::new();
        writer.write_bits(0, 1);
        writer.write_bits(9, 6);
        writer.write_bits(0, 1);
        writer.write_signed_bits(10, 16);
        writer.write_bits(0, 2);
        writer.write_bits(0, 4);
        writer.write_bits(2, 4);
        writer.write_rice_signed(1, 2);
        writer.write_rice_signed(2, 2);
        writer.write_rice_signed(3, 2);
        writer.finish()
    }

    fn lpc_order_one_subframe() -> Vec<u8> {
        let mut writer = BitWriter::new();
        writer.write_bits(0, 1);
        writer.write_bits(32, 6);
        writer.write_bits(0, 1);
        writer.write_signed_bits(10, 16);
        writer.write_bits(3, 4);
        writer.write_signed_bits(0, 5);
        writer.write_signed_bits(1, 4);
        writer.write_bits(0, 2);
        writer.write_bits(0, 4);
        writer.write_bits(2, 4);
        writer.write_rice_signed(1, 2);
        writer.write_rice_signed(2, 2);
        writer.write_rice_signed(3, 2);
        writer.finish()
    }

    fn stereo_constant_flac(
        channel_assignment_and_sample_size: u8,
        first_sample: i32,
        second_sample: i32,
        first_bits_per_sample: u8,
        second_bits_per_sample: u8,
    ) -> Vec<u8> {
        let mut flac = flac_with_streaminfo(StreamInfoFixture {
            min_block_size: 2,
            max_block_size: 2,
            min_frame_size: 0,
            max_frame_size: 0,
            sample_rate: 48_000,
            channels: 2,
            bits_per_sample: 16,
            total_samples: 2,
        });
        let header_without_crc = [
            0xff,
            0xf8,
            0x6a,
            channel_assignment_and_sample_size,
            0x00,
            0x01,
        ];
        let mut writer = BitWriter::new();
        writer.write_constant_subframe(first_sample, first_bits_per_sample);
        writer.write_constant_subframe(second_sample, second_bits_per_sample);
        flac.extend_from_slice(&flac_frame(&header_without_crc, &writer.finish()));
        flac
    }

    fn single_channel_constant_frame(frame_number: u8, sample: i32) -> Vec<u8> {
        let mut writer = BitWriter::new();
        writer.write_constant_subframe(sample, 16);
        flac_frame(
            &[0xff, 0xf8, 0x6a, 0x08, frame_number, 0x01],
            &writer.finish(),
        )
    }

    fn frame_header<const N: usize>(header_without_crc: [u8; N]) -> Vec<u8> {
        let mut header = header_without_crc.to_vec();
        header.push(crc8(&header));
        header
    }

    fn flac_frame(header_without_crc: &[u8], subframes: &[u8]) -> Vec<u8> {
        let mut frame = header_without_crc.to_vec();
        frame.push(crc8(&frame));
        frame.extend_from_slice(subframes);
        frame.extend_from_slice(&crc16(&frame).to_be_bytes());
        frame
    }

    fn utf8_coded_number(value: u64) -> Vec<u8> {
        assert!(value <= 0x000f_ffff_ffff);
        if value <= 0x7f {
            return vec![value as u8];
        }
        if value <= 0x7ff {
            return vec![0xc0 | ((value >> 6) as u8), 0x80 | ((value & 0x3f) as u8)];
        }
        if value <= 0xffff {
            return vec![
                0xe0 | ((value >> 12) as u8),
                0x80 | (((value >> 6) & 0x3f) as u8),
                0x80 | ((value & 0x3f) as u8),
            ];
        }
        if value <= 0x1f_ffff {
            return vec![
                0xf0 | ((value >> 18) as u8),
                0x80 | (((value >> 12) & 0x3f) as u8),
                0x80 | (((value >> 6) & 0x3f) as u8),
                0x80 | ((value & 0x3f) as u8),
            ];
        }
        if value <= 0x03ff_ffff {
            return vec![
                0xf8 | ((value >> 24) as u8),
                0x80 | (((value >> 18) & 0x3f) as u8),
                0x80 | (((value >> 12) & 0x3f) as u8),
                0x80 | (((value >> 6) & 0x3f) as u8),
                0x80 | ((value & 0x3f) as u8),
            ];
        }
        if value <= 0x7fff_ffff {
            return vec![
                0xfc | ((value >> 30) as u8),
                0x80 | (((value >> 24) & 0x3f) as u8),
                0x80 | (((value >> 18) & 0x3f) as u8),
                0x80 | (((value >> 12) & 0x3f) as u8),
                0x80 | (((value >> 6) & 0x3f) as u8),
                0x80 | ((value & 0x3f) as u8),
            ];
        }
        vec![
            0xfe,
            0x80 | (((value >> 30) & 0x3f) as u8),
            0x80 | (((value >> 24) & 0x3f) as u8),
            0x80 | (((value >> 18) & 0x3f) as u8),
            0x80 | (((value >> 12) & 0x3f) as u8),
            0x80 | (((value >> 6) & 0x3f) as u8),
            0x80 | ((value & 0x3f) as u8),
        ]
    }

    fn assert_pcm_close(actual: &[f32], expected: &[f32], epsilon: f32) {
        assert_eq!(actual.len(), expected.len());
        for (&actual, &expected) in actual.iter().zip(expected) {
            assert!(
                (actual - expected).abs() <= epsilon,
                "sample mismatch: actual={actual}, expected={expected}, epsilon={epsilon}"
            );
        }
    }

    fn assert_encoded_fixed_order(samples: Vec<i32>, expected_order: u8) {
        let pcm_samples = samples
            .iter()
            .map(|&sample| sample as f32 / 32_767.0)
            .collect::<Vec<_>>();
        let pcm = AudioBuffer::new(48_000, 1, pcm_samples).unwrap();

        let flac = super::encode(&pcm).unwrap();
        let info = parse_streaminfo(&flac).unwrap();
        let header = parse_frame_header(&flac[42..], &info).unwrap();
        let subframe_type = flac[42 + header.header_len] >> 1;
        let decoded = decode(&flac).unwrap();

        assert_eq!(subframe_type, 8 + expected_order);
        assert_eq!(decoded.frames(), samples.len());
        assert_pcm_close(&decoded.samples, &pcm.samples, 1.0 / 32_767.0);
    }

    struct BitWriter {
        bytes: Vec<u8>,
        bit_pos: usize,
    }

    impl BitWriter {
        fn new() -> Self {
            Self {
                bytes: Vec::new(),
                bit_pos: 0,
            }
        }

        fn write_bits(&mut self, value: u32, count: u8) {
            for bit_index in (0..count).rev() {
                let bit = ((value >> bit_index) & 1) as u8;
                self.write_bit(bit);
            }
        }

        fn write_signed_bits(&mut self, value: i32, count: u8) {
            let mask = if count == 32 {
                u32::MAX
            } else {
                (1_u32 << count) - 1
            };
            self.write_bits((value as u32) & mask, count);
        }

        fn write_rice_signed(&mut self, value: i32, rice_parameter: u8) {
            let folded = if value >= 0 {
                (value as u32) << 1
            } else {
                ((-value as u32) << 1) - 1
            };
            let quotient = folded >> rice_parameter;
            for _ in 0..quotient {
                self.write_bit(0);
            }
            self.write_bit(1);
            if rice_parameter > 0 {
                self.write_bits(folded & ((1_u32 << rice_parameter) - 1), rice_parameter);
            }
        }

        fn write_constant_subframe(&mut self, sample: i32, bits_per_sample: u8) {
            self.write_bits(0, 1);
            self.write_bits(0, 6);
            self.write_bits(0, 1);
            self.write_signed_bits(sample, bits_per_sample);
        }

        fn finish(self) -> Vec<u8> {
            self.bytes
        }

        fn write_bit(&mut self, bit: u8) {
            if self.bit_pos % 8 == 0 {
                self.bytes.push(0);
            }
            let byte_index = self.bit_pos / 8;
            let bit_index = 7 - (self.bit_pos % 8);
            self.bytes[byte_index] |= bit << bit_index;
            self.bit_pos += 1;
        }
    }

    #[test]
    fn decode_does_not_allocate_on_crafted_total_samples() {
        // A crafted STREAMINFO with the maximum 36-bit total_samples must not
        // drive an unbounded allocation (decompression bomb): decode must fail
        // gracefully against the tiny remaining input, not OOM/abort.
        let pcm = AudioBuffer::new(48_000, 2, vec![0.1, -0.1, 0.2, -0.2]).unwrap();
        let mut encoder = FlacEncoder::new();
        let mut flac = encoder.encode(&pcm).unwrap();

        // STREAMINFO packed 8-byte field starts at offset 18 (marker 4 + block
        // header 4 + min/max block 4 + min/max frame 6); total_samples is its
        // low 36 bits. Set them all to 1 (~68.7 billion frames).
        let mut packed = u64::from_be_bytes(flac[18..26].try_into().unwrap());
        packed |= 0x0000_000F_FFFF_FFFF;
        flac[18..26].copy_from_slice(&packed.to_be_bytes());

        assert!(decode(&flac).is_err());
    }
}
