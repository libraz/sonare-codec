    #[test]
    fn unified_encode_rejects_unknown_format() {
        let err = encode_audio("unknown", 44_100, 1, &[0.0]).unwrap_err();

        assert_eq!(err, "unsupported format");
    }

    #[test]
    fn unified_encode_produces_ogg_streams_for_lossy_encoders() {
        let samples = vec![0.0; 128];

        // Opus and Vorbis are both pure-Rust encoders that compile to wasm, so
        // the wasm surface enables their features and the unified entry point
        // produces real Ogg streams rather than reporting them as unsupported.
        let opus = encode_audio("opus", 48_000, 1, &samples).expect("opus encode");
        assert_eq!(&opus[..4], b"OggS");

        let vorbis = encode_audio("vorbis", 48_000, 1, &samples).expect("vorbis encode");
        assert_eq!(&vorbis[..4], b"OggS");
    }

    #[test]
    fn dedicated_lossy_encoders_produce_ogg_streams() {
        let samples = vec![0.0; 128];

        let vorbis = encode_vorbis(48_000, 1, &samples).expect("encode_vorbis");
        assert_eq!(&vorbis[..4], b"OggS");
        assert_eq!(
            sonare_codec::detect(&vorbis),
            Some(sonare_codec::Format::Vorbis)
        );

        let opus = encode_opus(48_000, 1, &samples).expect("encode_opus");
        assert_eq!(&opus[..4], b"OggS");
        assert_eq!(
            sonare_codec::detect(&opus),
            Some(sonare_codec::Format::Opus)
        );
    }
