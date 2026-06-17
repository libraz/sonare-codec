use sc_core::{compare_pcm, AudioBuffer};

#[test]
fn fixture_roundtrips_bit_exact() {
    let expected_wav = decode_hex(include_str!("../../../tests/fixtures/wav-pcm16-stereo.hex"));
    let pcm =
        AudioBuffer::new(48_000, 2, vec![0.0, 0.0, 0.5, -0.5, 1.0, -1.0, 0.25, -0.25]).unwrap();

    let encoded = sc_wav::encode(&pcm).unwrap();
    assert_eq!(encoded, expected_wav);

    let decoded = sc_wav::decode(&expected_wav).unwrap();
    let diff = compare_pcm(&decoded, &pcm).unwrap();
    assert!(diff.max_abs <= 1.0 / f32::from(i16::MAX));

    let encoded_again = sc_wav::encode(&decoded).unwrap();
    assert_eq!(encoded_again, expected_wav);
}

#[test]
fn decodes_pcm8_unsigned() {
    let wav = wav_with_format(8, 1, 8, &[0, 128, 255]);
    let decoded = sc_wav::decode(&wav).unwrap();

    assert_eq!(decoded.sample_rate, 8);
    assert_eq!(decoded.channels, 1);
    assert_eq!(decoded.samples, vec![-1.0, 0.0, 127.0 / 128.0]);
}

#[test]
fn encodes_pcm24_and_float32_headers() {
    let pcm = AudioBuffer::new(48_000, 1, vec![-1.0, 0.0, 1.0]).unwrap();

    let pcm24 = sc_wav::encode_as(&pcm, sc_wav::WavSampleFormat::Pcm24).unwrap();
    assert_eq!(&pcm24[20..22], &1_u16.to_le_bytes());
    assert_eq!(&pcm24[34..36], &24_u16.to_le_bytes());
    assert_eq!(sc_wav::decode(&pcm24).unwrap().samples.len(), 3);

    let float32 = sc_wav::encode_as(&pcm, sc_wav::WavSampleFormat::Float32).unwrap();
    assert_eq!(&float32[20..22], &3_u16.to_le_bytes());
    assert_eq!(&float32[34..36], &32_u16.to_le_bytes());
    assert_eq!(sc_wav::decode(&float32).unwrap().samples, pcm.samples);
}

#[test]
fn rejects_truncated_chunk_inside_declared_riff_size() {
    let mut wav = wav_with_format(8, 1, 8, &[128]);
    wav.truncate(wav.len() - 1);

    assert!(sc_wav::decode(&wav).is_err());
}

fn decode_hex(input: &str) -> Vec<u8> {
    let hex = input
        .bytes()
        .filter(|byte| !byte.is_ascii_whitespace())
        .collect::<Vec<_>>();
    assert_eq!(hex.len() % 2, 0);

    hex.chunks_exact(2)
        .map(|chunk| {
            let high = hex_digit(chunk[0]);
            let low = hex_digit(chunk[1]);
            (high << 4) | low
        })
        .collect()
}

fn hex_digit(byte: u8) -> u8 {
    match byte {
        b'0'..=b'9' => byte - b'0',
        b'a'..=b'f' => byte - b'a' + 10,
        b'A'..=b'F' => byte - b'A' + 10,
        _ => panic!("invalid hex digit"),
    }
}

fn wav_with_format(sample_rate: u32, channels: u16, bits_per_sample: u16, data: &[u8]) -> Vec<u8> {
    let bytes_per_sample = bits_per_sample / 8;
    let block_align = channels * bytes_per_sample;
    let byte_rate = sample_rate * u32::from(block_align);
    let data_len = u32::try_from(data.len()).unwrap();
    let riff_size = 4 + (8 + 16) + (8 + data_len);

    let mut out = Vec::new();
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&riff_size.to_le_bytes());
    out.extend_from_slice(b"WAVE");
    out.extend_from_slice(b"fmt ");
    out.extend_from_slice(&16_u32.to_le_bytes());
    out.extend_from_slice(&1_u16.to_le_bytes());
    out.extend_from_slice(&channels.to_le_bytes());
    out.extend_from_slice(&sample_rate.to_le_bytes());
    out.extend_from_slice(&byte_rate.to_le_bytes());
    out.extend_from_slice(&block_align.to_le_bytes());
    out.extend_from_slice(&bits_per_sample.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    out.extend_from_slice(data);
    out
}
