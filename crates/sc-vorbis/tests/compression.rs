//! Compression-overhead regression: audio packets must share Ogg pages (not one
//! page each), and the stream must stay well below raw 16-bit PCM size.

use sc_core::AudioBuffer;

fn sine(rate: u32, channels: u16, frames: usize, freq: f32) -> AudioBuffer {
    let mut samples = Vec::with_capacity(frames * usize::from(channels));
    for i in 0..frames {
        let v = (2.0 * std::f32::consts::PI * freq * i as f32 / rate as f32).sin() * 0.5;
        for _ in 0..channels {
            samples.push(v);
        }
    }
    AudioBuffer::new(rate, channels, samples).expect("pcm")
}

fn page_count(stream: &[u8]) -> usize {
    stream.windows(4).filter(|w| *w == b"OggS").count()
}

#[test]
fn audio_packets_are_batched_into_few_pages() {
    // One second of audio is ~375 short-block packets; batching must fold them
    // into a handful of pages rather than one page per packet.
    let pcm = sine(48_000, 1, 48_000, 440.0);
    let stream = sc_vorbis::encode(&pcm).expect("encode");
    let pages = page_count(&stream);
    assert!(
        pages < 20,
        "expected audio packets batched into few pages, got {pages}"
    );
}

#[test]
fn stream_is_smaller_than_raw_pcm() {
    let pcm = sine(48_000, 1, 48_000, 440.0);
    let stream = sc_vorbis::encode(&pcm).expect("encode");
    let raw16 = 48_000 * 2; // 16-bit mono PCM
                            // The residue value book is Huffman-fitted to the stream, so a tone (whose
                            // whitened residue is sharply peaked at zero) compresses well past 2.5x.
    assert!(
        stream.len() * 5 < raw16 * 2, // i.e. ratio > 2.5x
        "weak compression: {} bytes vs raw {raw16}",
        stream.len()
    );
    // And it still decodes through the standard decoder.
    let decoded = sc_vorbis::decode(&stream).expect("decode");
    assert_eq!(decoded.channels, 1);
}
