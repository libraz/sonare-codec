    use super::{
        decode, encode, encode_wav, encode_with_mode, AudioBuffer, EncodeMode, Error, Format,
        StreamDecoder,
    };
    use sc_core::BitReader;

    #[cfg(feature = "opus")]
    use super::encode_opus;

    #[test]
    fn dispatches_wav_roundtrip() {
        let pcm = AudioBuffer::new(44_100, 1, vec![0.0, 0.25, -0.25]).unwrap();
        let wav = encode(Format::Wav, &pcm).unwrap();
        let decoded = decode(&wav).unwrap();

        assert_eq!(
            encode_with_mode(Format::Wav, &pcm, EncodeMode::ProductionOnly).unwrap(),
            wav
        );
        assert_eq!(decoded.sample_rate, pcm.sample_rate);
        assert_eq!(decoded.channels, pcm.channels);
        assert_eq!(decoded.samples.len(), pcm.samples.len());
        assert_eq!(encode_wav(&pcm).unwrap(), wav);
        assert!(matches!(
            super::decode_mp3(&wav),
            Err(Error::UnsupportedFormat)
        ));
        assert!(matches!(
            super::decode_vorbis(&wav),
            Err(Error::UnsupportedFormat)
        ));
        assert!(matches!(
            super::decode_opus(&wav),
            Err(Error::UnsupportedFormat)
        ));
    }

    #[test]
    fn stream_decoder_buffers_until_complete_input() {
        let pcm = AudioBuffer::new(44_100, 1, vec![0.0, 0.25, -0.25]).unwrap();
        // Use a 16-bit fixture so a 2-byte truncation lands mid-frame and is
        // unambiguously incomplete; this test exercises StreamDecoder buffering,
        // not the default WAV sample format.
        let wav = super::encode_wav_as(&pcm, super::WavSampleFormat::Pcm16).unwrap();
        let split = wav.len() - 2;
        let mut decoder = StreamDecoder::new();

        assert!(decoder.decode_stream(&wav[..split]).unwrap().is_none());
        assert!(decoder.buffered_len() > 0);
        let decoded = decoder
            .decode_stream(&wav[split..])
            .unwrap()
            .expect("complete stream should decode");

        assert_eq!(decoded.sample_rate, pcm.sample_rate);
        assert_eq!(decoded.channels, pcm.channels);
        assert_eq!(decoded.samples.len(), pcm.samples.len());
        assert_eq!(decoder.buffered_len(), 0);
    }

    #[test]
    fn unsupported_encode_features_report_actionable_error() {
        // A non-default codec must fail with UnsupportedFeature (which names the
        // feature to enable), not a bare UnsupportedFormat that hides the cause.
        #[cfg(any(
            not(feature = "aac"),
            not(feature = "vorbis"),
            not(feature = "opus")
        ))]
        let pcm = AudioBuffer::new(48_000, 1, vec![0.0, 0.1, -0.1]).unwrap();

        #[cfg(not(feature = "aac"))]
        assert!(matches!(
            encode(Format::Aac, &pcm),
            Err(Error::UnsupportedFeature(_))
        ));
        #[cfg(not(feature = "vorbis"))]
        assert!(matches!(
            encode(Format::Vorbis, &pcm),
            Err(Error::UnsupportedFeature(_))
        ));
        #[cfg(not(feature = "opus"))]
        assert!(matches!(
            encode(Format::Opus, &pcm),
            Err(Error::UnsupportedFeature(_))
        ));
    }

    #[test]
    fn stream_decoder_rejects_oversized_buffer() {
        // A never-completing/garbage stream must not grow the buffer without
        // bound; an oversized accumulation is rejected and the buffer cleared.
        let mut decoder = StreamDecoder::new();
        let huge = vec![0_u8; (64 << 20) + 1];

        let err = decoder.decode_stream(&huge).unwrap_err();

        assert!(matches!(
            err,
            Error::InvalidInput("stream exceeded maximum buffered size")
        ));
        assert_eq!(decoder.buffered_len(), 0);
    }

    #[test]
    fn stream_decoder_clears_buffer_on_hard_error() {
        // A terminal decode error must drop the buffer so the next chunk starts
        // fresh instead of re-decoding (and re-failing on) a growing buffer.
        let pcm = AudioBuffer::new(44_100, 1, vec![0.0, 0.25, -0.25]).unwrap();
        let mut wav = encode(Format::Wav, &pcm).unwrap();
        wav[0..4].copy_from_slice(b"XXXX");
        let mut decoder = StreamDecoder::new();

        assert!(decoder.decode_stream(&wav).is_err());
        assert_eq!(decoder.buffered_len(), 0);
    }

    #[test]
    #[cfg(feature = "opus")]
    fn stream_decoder_buffers_opus_across_packet_boundaries() {
        // Regression: incomplete input is now reported via `Error::Incomplete`,
        // so feeding an Ogg Opus stream in small chunks must buffer (returning
        // `Ok(None)`) until the stream is complete instead of spuriously hard
        // failing when a chunk ends mid page/packet.
        let frames: Vec<f32> = (0..4_096)
            .map(|i| 0.3 * (i as f32 * 0.05).sin())
            .collect();
        let pcm = AudioBuffer::new(48_000, 1, frames).unwrap();
        let opus = encode(Format::Opus, &pcm).expect("opus encode");
        assert!(opus.len() > 40, "opus stream should span multiple pages");

        let mut decoder = StreamDecoder::new();
        let mut decoded = None;
        // A chunk size that does not divide page lengths forces chunk ends to
        // land inside pages and packets.
        for chunk in opus.chunks(13) {
            if let Some(buffer) = decoder
                .decode_stream(chunk)
                .expect("partial Opus chunk must buffer, not hard fail")
            {
                decoded = Some(buffer);
            }
        }

        let decoded = decoded.expect("complete Opus stream should decode");
        assert_eq!(decoded.channels, 1);
        assert_eq!(decoded.samples.len(), pcm.samples.len());
    }

    #[test]
    fn incomplete_input_is_reported_as_incomplete_not_invalid() {
        // The streaming layer relies on `Error::Incomplete` to distinguish
        // "needs more data" from genuinely malformed input. A WAV whose declared
        // data chunk is longer than the bytes provided must surface as
        // incomplete so a stream decoder keeps buffering.
        let pcm = AudioBuffer::new(44_100, 1, vec![0.0, 0.25, -0.25, 0.5]).unwrap();
        let wav = encode(Format::Wav, &pcm).unwrap();
        let truncated = &wav[..wav.len() - 4];

        assert!(matches!(decode(truncated), Err(Error::Incomplete)));
    }

    #[test]
    #[cfg(feature = "mp3")]
    fn production_mode_gates_unsupported_mp3_sample_rate() {
        // EncodeMode::ProductionOnly validates the input against the encoder's
        // supported channel/sample-rate matrix. An MPEG-2.5 rate (8 kHz) is
        // outside it, so non-silent input is rejected up front with an
        // actionable error rather than routed through a non-production path.
        let unsupported: Vec<f32> = (0..4_608).map(|i| 0.2 * (i as f32 * 0.1).sin()).collect();
        let pcm = AudioBuffer::new(8_000, 1, unsupported).unwrap();
        assert!(matches!(
            encode_with_mode(Format::Mp3, &pcm, EncodeMode::ProductionOnly),
            Err(Error::UnsupportedFeature(_))
        ));

        // A supported rate passes the gate and produces a stream.
        let supported: Vec<f32> = (0..4_608).map(|i| 0.2 * (i as f32 * 0.1).sin()).collect();
        let ok = AudioBuffer::new(44_100, 1, supported).unwrap();
        assert!(encode_with_mode(Format::Mp3, &ok, EncodeMode::ProductionOnly).is_ok());
    }

    #[test]
    #[cfg(feature = "flac")]
    fn dispatches_flac_roundtrip() {
        let samples = (0..128)
            .map(|sample| sample as f32 / 32_767.0)
            .collect::<Vec<_>>();
        let pcm = AudioBuffer::new(44_100, 1, samples).unwrap();
        let flac = encode(Format::Flac, &pcm).unwrap();
        let decoded = decode(&flac).unwrap();

        assert_eq!(
            encode_with_mode(Format::Flac, &pcm, EncodeMode::ProductionOnly).unwrap(),
            flac
        );
        assert_eq!(decoded.sample_rate, pcm.sample_rate);
        assert_eq!(decoded.channels, pcm.channels);
        assert_eq!(decoded.samples.len(), pcm.samples.len());
    }

    #[test]
    #[cfg(all(feature = "wav", feature = "decode"))]
    fn default_wav_encode_is_bit_exact_float_lossless() {
        use super::WavSampleFormat;
        // The high-level WAV path defaults to 32-bit float, so even out-of-range
        // mastering headroom (|s| > 1.0) round-trips bit-exactly.
        let pcm = AudioBuffer::new(48_000, 1, vec![2.5, -3.0, 0.5, -1.0, 1.0, 0.0]).unwrap();
        let wav = encode(Format::Wav, &pcm).unwrap();
        let decoded = decode(&wav).unwrap();
        assert_eq!(decoded.samples, pcm.samples);

        // 16-bit is still selectable and clamps/quantizes as expected.
        let wav16 = super::encode_wav_as(&pcm, WavSampleFormat::Pcm16).unwrap();
        assert!(wav16.len() < wav.len());
    }

    #[test]
    #[cfg(all(feature = "flac", feature = "decode"))]
    fn default_flac_encode_is_24bit() {
        use super::FlacBitDepth;
        let samples: Vec<f32> = (0..256).map(|i| i as f32 / 300_000.0).collect();
        let pcm = AudioBuffer::new(48_000, 1, samples.clone()).unwrap();

        // Default FLAC is 24-bit and reconstructs the sub-16-bit detail closely.
        let flac = encode(Format::Flac, &pcm).unwrap();
        let decoded = decode(&flac).unwrap();
        assert_eq!(decoded.frames(), 256);
        let err: f64 = decoded
            .samples
            .iter()
            .zip(&samples)
            .map(|(d, s)| f64::from((d - s).abs()))
            .sum();
        // Far tighter than a 16-bit quantizer (LSB 1/32767) could achieve here.
        assert!(err < 256.0 / 32_767.0, "24-bit FLAC error too high: {err}");

        // 16-bit remains explicitly selectable.
        let flac16 = super::encode_flac_as(&pcm, FlacBitDepth::Bits16).unwrap();
        assert!(decode(&flac16).is_ok());
    }

    #[test]
    fn dispatches_known_unimplemented_formats_as_unsupported() {
        let err = decode(b"ID3\x04\0\0\0\0\0\0").unwrap_err();
        assert!(matches!(err, Error::UnsupportedFormat));
    }

    #[test]
    #[cfg(feature = "opus")]
    fn dispatches_opus_encode_to_ogg_stream() {
        let pcm = AudioBuffer::new(48_000, 1, vec![0.0; 4800]).unwrap();
        let encoded = encode(Format::Opus, &pcm).expect("opus encode");

        assert_eq!(&encoded[..4], b"OggS");
        assert_eq!(super::detect(&encoded), Some(Format::Opus));
        assert_eq!(encode_opus(&pcm).expect("encode_opus"), encoded);
        let production = encode_with_mode(Format::Opus, &pcm, EncodeMode::ProductionOnly)
            .expect("production opus");
        assert_eq!(&production[..4], b"OggS");
        assert_eq!(super::detect(&production), Some(Format::Opus));
    }

    #[test]
    #[cfg(feature = "vorbis")]
    fn dispatches_vorbis_encode_to_ogg_stream() {
        let pcm = AudioBuffer::new(48_000, 1, vec![0.0; 4800]).unwrap();
        let encoded = encode(Format::Vorbis, &pcm).expect("vorbis encode");
        assert_eq!(&encoded[..4], b"OggS");
        assert_eq!(super::detect(&encoded), Some(Format::Vorbis));
        let production = encode_with_mode(Format::Vorbis, &pcm, EncodeMode::ProductionOnly)
            .expect("production vorbis");
        assert_eq!(&production[..4], b"OggS");
        assert_eq!(super::detect(&production), Some(Format::Vorbis));
    }

    #[test]
    #[cfg(feature = "opus")]
    fn dispatches_ffmpeg_generated_ogg_opus_when_available() {
        let Ok(ffmpeg) = std::env::var("SONARE_FFMPEG") else {
            return;
        };
        let path = std::env::temp_dir().join(format!(
            "sonare-codec-umbrella-opus-smoke-{}.opus",
            std::process::id()
        ));

        let status = std::process::Command::new(ffmpeg)
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-f",
                "lavfi",
                "-i",
                "sine=frequency=440:duration=0.05:sample_rate=48000",
                "-ac",
                "1",
                "-c:a",
                "libopus",
                "-y",
            ])
            .arg(&path)
            .status()
            .expect("run ffmpeg");
        assert!(status.success(), "ffmpeg failed with {status}");

        let bytes = std::fs::read(&path).expect("read opus");
        let _ = std::fs::remove_file(&path);
        let decoded = decode(&bytes).expect("decode opus");
        assert_eq!(decoded.sample_rate, 48_000);
        assert_eq!(decoded.channels, 1);
        assert!(!decoded.samples.is_empty());
        assert!(decoded.samples.iter().any(|sample| sample.abs() > 0.0001));
    }

    #[test]
    #[cfg(feature = "vorbis")]
    fn dispatches_ffmpeg_generated_ogg_vorbis_when_available() {
        let Ok(ffmpeg) = std::env::var("SONARE_FFMPEG") else {
            return;
        };
        let path = std::env::temp_dir().join(format!(
            "sonare-codec-umbrella-vorbis-smoke-{}.ogg",
            std::process::id()
        ));

        let status = std::process::Command::new(ffmpeg)
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-f",
                "lavfi",
                "-i",
                "sine=frequency=440:duration=0.05:sample_rate=48000",
                "-ac",
                "1",
                "-c:a",
                "libvorbis",
                "-y",
            ])
            .arg(&path)
            .status()
            .expect("run ffmpeg");
        assert!(status.success(), "ffmpeg failed with {status}");

        let bytes = std::fs::read(&path).expect("read vorbis");
        let _ = std::fs::remove_file(&path);
        let decoded = super::decode_vorbis(&bytes).expect("decode vorbis");
        assert_eq!(decoded.sample_rate, 48_000);
        assert_eq!(decoded.channels, 1);
        assert!(!decoded.samples.is_empty());
        assert!(decoded.samples.iter().any(|sample| sample.abs() > 0.0001));
    }

    #[test]
    #[cfg(feature = "mp3")]
    fn dispatches_silent_mp3_encode() {
        let pcm = AudioBuffer::new(44_100, 2, vec![0.0; 1152 * 2]).unwrap();

        let mp3 = encode(Format::Mp3, &pcm).unwrap();
        let decoded = decode(&mp3).unwrap();

        assert_eq!(
            encode_with_mode(Format::Mp3, &pcm, EncodeMode::ProductionOnly).unwrap(),
            mp3
        );
        assert_eq!(&mp3[..2], &[0xff, 0xfb]);
        assert_eq!(super::detect(&mp3), Some(Format::Mp3));
        assert_eq!(decoded.sample_rate, 44_100);
        assert_eq!(decoded.channels, 2);
        assert_eq!(decoded.samples.len(), pcm.samples.len());
        assert_eq!(
            super::decode_mp3(&mp3).unwrap().samples.len(),
            pcm.samples.len()
        );
    }

    #[test]
    #[cfg(feature = "mp3")]
    fn dispatches_non_silent_mp3_encode_as_layer3_scaffold() {
        for (sample_rate, channels) in [
            (32_000, 1),
            (44_100, 1),
            (48_000, 1),
            (32_000, 2),
            (44_100, 2),
            (48_000, 2),
        ] {
            let mut samples = Vec::new();
            for frame in 0..2048 {
                for channel in 0..channels {
                    let phase = if channel == 0 { 0.01 } else { 0.013 };
                    samples.push(((frame as f32) * phase).sin() * 0.25);
                }
            }
            let pcm = AudioBuffer::new(sample_rate, channels, samples).unwrap();

            let mp3 = encode(Format::Mp3, &pcm).unwrap();
            let production =
                encode_with_mode(Format::Mp3, &pcm, EncodeMode::ProductionOnly).unwrap();
            let zero_payload = super::encode_mpeg1_layer3_pcm_frames_with_selected_scale_factors_and_table_provider(
                    &pcm,
                    f32::MAX,
                    super::Layer3EntropyTableProvider::default(),
                )
                .unwrap();
            let decoded = decode(&mp3).unwrap();

            assert_eq!(&mp3[..2], &[0xff, 0xfb]);
            assert_eq!(
                production, mp3,
                "sample_rate={sample_rate} channels={channels}"
            );
            assert_eq!(super::detect(&mp3), Some(Format::Mp3));
            assert!(mp3.len() > 4);
            assert_ne!(mp3, zero_payload);
            assert_eq!(decoded.sample_rate, sample_rate);
            assert_eq!(decoded.channels, channels);
            assert_eq!(decoded.samples.len(), 2304 * usize::from(channels));
        }
    }

    #[test]
    #[cfg(feature = "mp3")]
    fn production_mono_mp3_uses_low_band_gain_entropy_reservoir_path() {
        let frames = 8_usize;
        let samples_per_frame = 1152_usize;
        let samples = (0..(frames * samples_per_frame))
            .map(|sample| {
                let t = sample as f32;
                0.24 * ((t * 0.043).sin() + 0.5 * (t * 0.131).sin())
            })
            .collect();
        let pcm = AudioBuffer::new(44_100, 1, samples).unwrap();

        let production = encode_with_mode(Format::Mp3, &pcm, EncodeMode::ProductionOnly).unwrap();
        let production_candidates =
            super::mpeg1_layer3_production_pcm_step_candidates(pcm.channels).unwrap();
        let perceptual_cbr =
            super::encode_mpeg1_layer3_pcm_frames_with_perceptual_active_cbr_bitrate_and_table_provider(
                &pcm,
                super::MPEG1_LAYER3_PCM_STEP_CANDIDATES,
                128,
                false,
                super::mpeg1_layer3_standard_table_provider(),
            )
            .unwrap();
        let entropy_targeted_reservoir = super::encode_mpeg1_layer3_pcm_frames_with_entropy_targeted_perceptual_reservoir_and_table_provider(
            &pcm,
            production_candidates,
            128,
            false,
            0,
            super::mpeg1_layer3_standard_table_provider(),
        )
        .unwrap();
        let low_band_gain_reservoir = super::encode_mpeg1_layer3_pcm_frames_with_entropy_targeted_perceptual_quantized_band_gain_and_global_gain_bias_reservoir_and_table_provider(
            &pcm,
            &[2.0],
            128,
            false,
            0,
            super::Layer3QuantizedBandGain {
                band_start: 0,
                band_end: 7,
                gain: 1.5,
            },
            -4,
            super::mpeg1_layer3_standard_table_provider(),
        )
        .unwrap();

        let mut offset = 0_usize;
        let mut frame_index = 0_usize;
        let mut max_main_data_begin = 0_u32;
        while offset < production.len() {
            let header = super::FrameHeader::parse(&production[offset..offset + 4]).unwrap();
            let mut reader = BitReader::new(&production[offset + 4..]);
            let main_data_begin = reader.read_bits(9).unwrap();
            max_main_data_begin = max_main_data_begin.max(main_data_begin);
            offset += header.frame_len();
            frame_index += 1;
        }

        assert_eq!(offset, production.len());
        assert_eq!(frame_index, frames);
        assert!(
            max_main_data_begin > 0,
            "production MP3 stopped using the bit reservoir"
        );
        assert_eq!(
            production, low_band_gain_reservoir,
            "mono production MP3 should use the low-band gain/global-gain-bias entropy reservoir path"
        );
        assert_ne!(
            production, entropy_targeted_reservoir,
            "mono production MP3 should no longer use the older entropy-targeted perceptual reservoir payload"
        );
        assert_ne!(
            production, perceptual_cbr,
            "mono production MP3 should keep the reservoir layout, not the self-contained perceptual CBR layout"
        );
    }

    #[test]
    #[cfg(feature = "mp3")]
    fn production_stereo_mp3_uses_entropy_targeted_perceptual_reservoir_path() {
        let frames = 8_usize;
        let samples_per_frame = 1152_usize;
        let samples = (0..(frames * samples_per_frame * 2))
            .map(|index| {
                let frame = index / (samples_per_frame * 2);
                let t = ((index / 2) % samples_per_frame) as f32;
                let right = index % 2 == 1;
                if frame % 2 == 0 {
                    if right {
                        0.24 * ((t * 0.053).sin() + (t * 0.173).sin() + (t * 0.337).sin())
                    } else {
                        0.28 * ((t * 0.037).sin() + (t * 0.149).sin() + (t * 0.419).sin())
                    }
                } else if right {
                    0.018 * (t * 0.047).sin()
                } else {
                    0.02 * (t * 0.041).sin()
                }
            })
            .collect();
        let pcm = AudioBuffer::new(44_100, 2, samples).unwrap();

        let production = encode_with_mode(Format::Mp3, &pcm, EncodeMode::ProductionOnly).unwrap();
        let production_candidates =
            super::mpeg1_layer3_production_pcm_step_candidates(pcm.channels).unwrap();
        let entropy_targeted_details =
            super::select_mpeg1_layer3_entropy_targeted_perceptual_reservoir_frame_details_with_table_provider(
                &pcm,
                production_candidates,
                128,
                false,
                0,
                super::mpeg1_layer3_standard_table_provider(),
            )
            .unwrap();
        let perceptual_details =
            super::select_mpeg1_layer3_perceptual_reservoir_frame_details_with_table_provider(
                &pcm,
                super::MPEG1_LAYER3_PCM_STEP_CANDIDATES,
                128,
                false,
                super::mpeg1_layer3_standard_table_provider(),
            )
            .unwrap();
        let perceptual_reservoir =
            super::encode_mpeg1_layer3_pcm_frames_with_perceptual_reservoir_and_table_provider(
                &pcm,
                super::MPEG1_LAYER3_PCM_STEP_CANDIDATES,
                128,
                false,
                super::mpeg1_layer3_standard_table_provider(),
            )
            .unwrap();
        let entropy_targeted_reservoir = super::encode_mpeg1_layer3_pcm_frames_with_entropy_targeted_perceptual_reservoir_and_table_provider(
            &pcm,
            production_candidates,
            128,
            false,
            0,
            super::mpeg1_layer3_standard_table_provider(),
        )
        .unwrap();

        assert!(entropy_targeted_details
            .iter()
            .all(|detail| { detail.perceptual_granules == 4 && detail.calibrated_granules == 0 }));
        assert!(entropy_targeted_details.iter().all(|detail| {
            detail.quality_guard_compared_granules == 0
                && detail.quality_guard_distortion_delta == 0.0
        }));
        assert!(entropy_targeted_details
            .iter()
            .any(|detail| detail.used_entropy_target_budget));
        assert_eq!(
            entropy_targeted_details
                .iter()
                .map(|detail| detail.entropy_target_bits)
                .sum::<usize>(),
            entropy_targeted_details
                .iter()
                .map(|detail| detail.frame_capacity_bytes * 8)
                .sum::<usize>()
        );

        let mut offset = 0_usize;
        let mut frame_index = 0_usize;
        let mut max_main_data_begin = 0_u32;
        while offset < production.len() {
            let header = super::FrameHeader::parse(&production[offset..offset + 4]).unwrap();
            let mut reader = BitReader::new(&production[offset + 4..]);
            let main_data_begin = reader.read_bits(9).unwrap();
            assert_eq!(
                entropy_targeted_details[frame_index].main_data_begin as u32,
                main_data_begin
            );
            max_main_data_begin = max_main_data_begin.max(main_data_begin);
            offset += header.frame_len();
            frame_index += 1;
        }

        assert_eq!(offset, production.len());
        assert_eq!(frame_index, entropy_targeted_details.len());
        assert!(
            max_main_data_begin > 0,
            "production stereo MP3 stopped using the bit reservoir"
        );
        assert_eq!(
            production, entropy_targeted_reservoir,
            "stereo production MP3 should use the entropy-targeted perceptual reservoir path"
        );
        assert_ne!(
            production, perceptual_reservoir,
            "stereo production MP3 should no longer use the raw perceptual reservoir path"
        );
        assert_eq!(perceptual_details.len(), entropy_targeted_details.len());
        assert_eq!(super::detect(&production), Some(Format::Mp3));
    }

    #[test]
    #[cfg(feature = "mp3")]
    fn exposes_mp3_pcm_frame_scaffold_helper() {
        assert_eq!(
            super::MPEG1_LAYER3_STANDARD_BIG_VALUE_TABLE_SELECTS,
            &[
                1, 2, 3, 5, 6, 7, 8, 9, 10, 11, 12, 13, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
                26, 27, 28, 29, 30, 31
            ]
        );
        assert_eq!(
            super::MPEG1_LAYER3_MISSING_STANDARD_BIG_VALUE_TABLE_SELECTS,
            &[]
        );
        assert_eq!(
            super::MPEG1_LAYER3_STANDARD_COUNT1_TABLE_SELECTS,
            &[false, true]
        );

        let pcm = AudioBuffer::new(44_100, 2, vec![0.0; 1153 * 2]).unwrap();

        let scaffold =
            super::encode_mpeg1_layer3_pcm_frames_with_selected_scale_factors_and_table_provider(
                &pcm,
                1.0,
                super::Layer3EntropyTableProvider::default(),
            )
            .unwrap();

        assert_eq!(scaffold, encode(Format::Mp3, &pcm).unwrap());
    }

    #[test]
    #[cfg(feature = "mp3")]
    fn exposes_mp3_pcm_payload_budget_helper() {
        let pcm = AudioBuffer::new(
            44_100,
            1,
            (0..1152)
                .map(|sample| ((sample as f32) * 0.01).sin() * 0.25)
                .collect(),
        )
        .unwrap();
        let header = super::FrameHeader {
            version: super::MpegVersion::Mpeg1,
            layer: super::Layer::Layer3,
            protection_absent: true,
            bitrate_kbps: 128,
            sample_rate: 44_100,
            padding: false,
            channel_mode: super::ChannelMode::SingleChannel,
        };
        let provider = super::mpeg1_layer3_standard_table_provider();
        let unconstrained = super::select_mpeg1_layer3_pcm_frame_step_details_with_table_provider(
            header,
            &pcm,
            0,
            super::MPEG1_LAYER3_PCM_STEP_CANDIDATES,
            provider,
        )
        .unwrap();

        let step =
            super::select_mpeg1_layer3_pcm_frame_step_with_max_payload_bits_and_table_provider(
                header,
                &pcm,
                0,
                super::MPEG1_LAYER3_PCM_STEP_CANDIDATES,
                unconstrained.payload_bit_len,
                provider,
            )
            .unwrap();
        let details =
            super::select_mpeg1_layer3_pcm_frame_step_details_with_max_payload_bits_and_table_provider(
                header,
                &pcm,
                0,
                super::MPEG1_LAYER3_PCM_STEP_CANDIDATES,
                unconstrained.payload_bit_len,
                provider,
            )
            .unwrap();
        let budgeted =
            super::encode_mpeg1_layer3_pcm_frames_with_max_payload_bits_and_table_provider(
                &pcm,
                super::MPEG1_LAYER3_PCM_STEP_CANDIDATES,
                unconstrained.payload_bit_len,
                provider,
            )
            .unwrap();
        let selected =
            super::encode_mpeg1_layer3_pcm_frames_with_selected_scale_factors_and_table_provider(
                &pcm, step, provider,
            )
            .unwrap();

        assert_eq!(step, unconstrained.step);
        assert_eq!(details.step, step);
        assert_eq!(details.frame_capacity_bits, unconstrained.payload_bit_len);
        assert!(details.payload_bit_len <= unconstrained.payload_bit_len);
        assert_eq!(super::layer3_main_data_capacity_bits(header).unwrap(), 3168);
        assert_eq!(super::layer3_main_data_capacity_bytes(header).unwrap(), 396);
        assert_eq!(
            super::layer3_main_data_capacity_bytes(
                super::layer3_header_for_capacity(44_100, 2, 128, false, false).unwrap()
            )
            .unwrap(),
            381
        );
        assert_eq!(budgeted, selected);
        assert!(
            super::select_mpeg1_layer3_pcm_frame_step_details_with_max_payload_bits_and_table_provider(
                header,
                &pcm,
                0,
                super::MPEG1_LAYER3_PCM_STEP_CANDIDATES,
                0,
                provider,
            )
            .is_err()
        );
    }

    #[test]
    #[cfg(feature = "mp3")]
    fn exposes_mp3_psychoacoustic_scalefactor_helper() {
        let pcm = AudioBuffer::new(44_100, 1, vec![0.0; 2304]).unwrap();
        let scale_factors = super::select_mpeg1_layer3_psychoacoustic_long_scale_factors(
            &pcm, 0, 576, 0.05, false, 1024,
        )
        .unwrap();

        assert_eq!(
            scale_factors,
            [0_u8; super::MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT]
        );
    }

    #[test]
    #[cfg(feature = "mp3")]
    fn exposes_mp3_perceptual_scalefactor_stream_helper() {
        let pcm = AudioBuffer::new(
            44_100,
            1,
            (0..1152)
                .map(|sample| ((sample as f32) * 0.02).sin() * 0.2)
                .collect(),
        )
        .unwrap();
        let header = super::layer3_header_for_capacity(44_100, 1, 128, false, false).unwrap();
        let candidates = [0.05_f32, 0.1, 0.2];
        let selected =
            super::select_mpeg1_layer3_pcm_frame_perceptual_step_details_with_table_provider(
                header,
                &pcm,
                0,
                &candidates,
                super::mpeg1_layer3_standard_table_provider(),
            )
            .unwrap();
        let active_selected = super::select_mpeg1_layer3_pcm_frame_perceptual_active_step_details_with_table_provider(
                header,
                &pcm,
                0,
                &candidates,
                super::mpeg1_layer3_standard_table_provider(),
            )
            .unwrap();
        let profile_candidates = [0.05_f32, 0.1, 0.2, 1.0];
        let candidate_profile =
            super::select_mpeg1_layer3_first_frame_perceptual_candidate_profile_with_table_provider(
                &pcm,
                &profile_candidates,
                128,
                false,
                super::mpeg1_layer3_standard_table_provider(),
            )
            .unwrap();
        let low_band_profile =
            super::select_mpeg1_layer3_first_frame_low_band_spectral_shape_candidate_profile_with_table_provider(
                &pcm,
                &profile_candidates,
                128,
                false,
                super::mpeg1_layer3_standard_table_provider(),
            )
            .unwrap();
        let band_shape_profile =
            super::select_mpeg1_layer3_first_frame_band_spectral_shape_candidate_profile_with_table_provider(
                &pcm,
                &profile_candidates,
                128,
                false,
                super::mpeg1_layer3_standard_table_provider(),
            )
            .unwrap();
        let bit_allocation =
            super::select_mpeg1_layer3_perceptual_bit_allocation_with_bitrate(&pcm, 128, false, 0)
                .unwrap();
        let encoded =
            super::encode_mpeg1_layer3_pcm_frames_with_perceptual_scale_factors_and_table_provider(
                &pcm,
                0.1,
                super::mpeg1_layer3_standard_table_provider(),
            )
            .unwrap();
        let scalefac_scale_encoded =
            super::encode_mpeg1_layer3_pcm_frames_with_perceptual_scalefac_scale_and_table_provider(
                &pcm,
                0.1,
                true,
                super::mpeg1_layer3_standard_table_provider(),
            )
            .unwrap();
        let allowed_noise_scaled =
            super::encode_mpeg1_layer3_pcm_frames_with_perceptual_allowed_noise_scale_and_table_provider(
                &pcm,
                0.1,
                0.5,
                super::mpeg1_layer3_standard_table_provider(),
            )
            .unwrap();
        let budgeted =
            super::encode_mpeg1_layer3_pcm_frames_with_perceptual_max_payload_bits_and_table_provider(
                &pcm,
                &candidates,
                selected.payload_bit_len,
                super::mpeg1_layer3_standard_table_provider(),
            )
            .unwrap();
        let bitrate_header =
            super::layer3_header_for_capacity(44_100, 1, 96, false, false).unwrap();
        let bitrate_encoded =
            super::encode_mpeg1_layer3_pcm_frames_with_perceptual_bitrate_and_table_provider(
                &pcm,
                &candidates,
                96,
                false,
                false,
                super::mpeg1_layer3_standard_table_provider(),
            )
            .unwrap();
        let cbr_encoded =
            super::encode_mpeg1_layer3_pcm_frames_with_perceptual_cbr_bitrate_and_table_provider(
                &pcm,
                &candidates,
                96,
                false,
                super::mpeg1_layer3_standard_table_provider(),
            )
            .unwrap();
        let active_cbr_encoded =
            super::encode_mpeg1_layer3_pcm_frames_with_perceptual_active_cbr_bitrate_and_table_provider(
                &pcm,
                &candidates,
                96,
                false,
                super::mpeg1_layer3_standard_table_provider(),
            )
            .unwrap();

        assert!(active_selected.payload_bit_len <= active_selected.frame_capacity_bits);
        assert_eq!(candidate_profile.len(), profile_candidates.len());
        assert!(candidate_profile.iter().all(|profile| {
            profile.scale_factor_bands == super::MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT * 2
        }));
        assert!(candidate_profile
            .iter()
            .any(|profile| profile.nonzero_scale_factors > 0));
        assert_eq!(low_band_profile.len(), profile_candidates.len());
        assert!(low_band_profile.iter().all(|profile| {
            profile.low_band_abs_sum <= profile.total_abs_sum
                && profile.low_band_nonzero_lines <= profile.total_nonzero_lines
        }));
        assert!(low_band_profile
            .iter()
            .any(|profile| profile.low_band_nonzero_lines > 0));
        assert_eq!(
            band_shape_profile.len(),
            profile_candidates.len() * super::MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT
        );
        assert!(band_shape_profile.iter().all(|profile| {
            profile.band < super::MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT
                && profile.band_start <= profile.band_end
                && profile.band_abs_sum <= profile.total_abs_sum
                && profile.band_nonzero_lines <= profile.total_nonzero_lines
        }));
        assert!(band_shape_profile
            .iter()
            .any(|profile| profile.band_nonzero_lines > 0));
        assert_eq!(bit_allocation.len(), 2);
        assert_eq!(
            bit_allocation
                .iter()
                .map(|allocation| allocation.target_bits)
                .sum::<usize>(),
            super::layer3_main_data_capacity_bits(header).unwrap()
        );
        assert!(bit_allocation
            .iter()
            .all(|allocation| allocation.perceptual_entropy.is_finite()));
        assert_eq!(encoded.len(), header.frame_len());
        assert_eq!(budgeted.len(), header.frame_len());
        assert_eq!(bitrate_encoded.len(), bitrate_header.frame_len());
        assert_eq!(cbr_encoded.len(), bitrate_header.frame_len());
        assert_eq!(active_cbr_encoded.len(), bitrate_header.frame_len());
        assert_eq!(super::detect(&encoded), Some(Format::Mp3));
        assert_eq!(scalefac_scale_encoded.len(), header.frame_len());
        assert_eq!(super::detect(&scalefac_scale_encoded), Some(Format::Mp3));
        assert_eq!(allowed_noise_scaled.len(), header.frame_len());
        assert_eq!(super::detect(&allowed_noise_scaled), Some(Format::Mp3));
        assert_eq!(super::detect(&budgeted), Some(Format::Mp3));
        assert_eq!(
            super::FrameHeader::parse(&bitrate_encoded[..4]).unwrap(),
            bitrate_header
        );

        let cbr_pcm = AudioBuffer::new(
            44_100,
            1,
            (0..(1152 * 3))
                .map(|sample| ((sample as f32) * 0.013).sin() * 0.25)
                .collect(),
        )
        .unwrap();
        let active_cbr_128 =
            super::encode_mpeg1_layer3_pcm_frames_with_perceptual_active_cbr_bitrate_and_table_provider(
                &cbr_pcm,
                &candidates,
                128,
                false,
                super::mpeg1_layer3_standard_table_provider(),
            )
            .unwrap();
        let first_header = super::FrameHeader::parse(&active_cbr_128[..4]).unwrap();
        let second_offset = first_header.frame_len();
        let second_header =
            super::FrameHeader::parse(&active_cbr_128[second_offset..second_offset + 4]).unwrap();
        let third_offset = second_offset + second_header.frame_len();
        let third_header =
            super::FrameHeader::parse(&active_cbr_128[third_offset..third_offset + 4]).unwrap();
        assert_eq!(first_header.frame_len(), 417);
        assert_eq!(second_header.frame_len(), 418);
        assert_eq!(third_header.frame_len(), 418);
        assert_eq!(active_cbr_128.len(), 1253);

        let reservoir_pcm = AudioBuffer::new(
            44_100,
            1,
            (0..(1152 * 8))
                .map(|sample| {
                    let t = sample as f32;
                    if sample / 1152 % 2 == 0 {
                        0.24 * ((t * 0.043).sin()
                            + 0.7 * (t * 0.131).sin()
                            + 0.4 * (t * 0.277).sin())
                    } else {
                        0.02 * (t * 0.05).sin()
                    }
                })
                .collect(),
        )
        .unwrap();
        let perceptual_reservoir_details =
            super::select_mpeg1_layer3_perceptual_reservoir_frame_details_with_table_provider(
                &reservoir_pcm,
                super::MPEG1_LAYER3_PCM_STEP_CANDIDATES,
                128,
                false,
                super::mpeg1_layer3_standard_table_provider(),
            )
            .unwrap();
        let entropy_targeted_details =
            super::select_mpeg1_layer3_entropy_targeted_perceptual_reservoir_frame_details_with_table_provider(
                &reservoir_pcm,
                super::MPEG1_LAYER3_PCM_STEP_CANDIDATES,
                128,
                false,
                0,
                super::mpeg1_layer3_standard_table_provider(),
            )
            .unwrap();
        let perceptual_reservoir =
            super::encode_mpeg1_layer3_pcm_frames_with_perceptual_reservoir_and_table_provider(
                &reservoir_pcm,
                super::MPEG1_LAYER3_PCM_STEP_CANDIDATES,
                128,
                false,
                super::mpeg1_layer3_standard_table_provider(),
            )
            .unwrap();
        assert_eq!(perceptual_reservoir_details.len(), 8);
        assert_eq!(
            entropy_targeted_details.len(),
            perceptual_reservoir_details.len()
        );
        assert_eq!(
            entropy_targeted_details
                .iter()
                .map(|detail| detail.entropy_target_bits)
                .sum::<usize>(),
            perceptual_reservoir_details
                .iter()
                .map(|detail| detail.frame_capacity_bytes * 8)
                .sum::<usize>()
        );
        assert!(entropy_targeted_details
            .iter()
            .any(|detail| detail.used_entropy_target_budget));
        assert!(perceptual_reservoir_details
            .iter()
            .any(|detail| detail.main_data_begin > 0));
        assert_eq!(super::detect(&perceptual_reservoir), Some(Format::Mp3));
    }

