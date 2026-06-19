//! Robustness sweep: the encoder must stay panic-free and length-exact across
//! the full range of sample rates (every AoTuV `m_val` tier and rate-scaled psy
//! state), channel layouts, signal shapes (silence, DC, full-scale, out-of-range,
//! broadband), and lengths — and every stream must decode through the standard
//! decoder at the exact input length. Most of the suite runs at 48 kHz; this is
//! the breadth check for the rate-dependent analysis chain.

use sc_core::{AudioBuffer, Format};

/// Builds an interleaved buffer from a per-sample closure (the same value on
/// every channel).
fn build(rate: u32, channels: u16, frames: usize, mut f: impl FnMut(usize) -> f32) -> AudioBuffer {
    let mut samples = Vec::with_capacity(frames * usize::from(channels));
    for i in 0..frames {
        let v = f(i);
        for _ in 0..channels {
            samples.push(v);
        }
    }
    AudioBuffer::new(rate, channels, samples).expect("pcm")
}

fn sine(rate: u32, channels: u16, frames: usize, freq: f32, amp: f32) -> AudioBuffer {
    build(rate, channels, frames, |i| {
        amp * (2.0 * std::f32::consts::PI * freq * i as f32 / rate as f32).sin()
    })
}

/// Deterministic broadband noise in `[-amp, amp]` (an LCG, no `rand` dependency).
fn noise(rate: u32, channels: u16, frames: usize, amp: f32) -> AudioBuffer {
    let mut state = 0x2545_f491u32;
    build(rate, channels, frames, |_| {
        state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        ((state >> 9) as f32 / (1u32 << 23) as f32 - 1.0) * amp
    })
}

/// Encodes, asserts the stream is a detectable Ogg Vorbis stream, decodes it
/// back, and asserts the channel layout and sample-accurate length survive.
fn roundtrip_is_exact(pcm: &AudioBuffer, label: &str) {
    let bytes = sc_vorbis::encode(pcm).unwrap_or_else(|e| panic!("{label}: encode failed: {e:?}"));
    assert_eq!(&bytes[..4], b"OggS", "{label}: not an Ogg stream");
    assert_eq!(
        sc_core::detect(&bytes),
        Some(Format::Vorbis),
        "{label}: not detected as Vorbis"
    );
    let decoded =
        sc_vorbis::decode(&bytes).unwrap_or_else(|e| panic!("{label}: decode failed: {e:?}"));
    assert_eq!(
        decoded.channels, pcm.channels,
        "{label}: channel count drifted"
    );
    assert_eq!(
        decoded.sample_rate, pcm.sample_rate,
        "{label}: sample rate drifted"
    );
    assert_eq!(
        decoded.frames(),
        pcm.frames(),
        "{label}: decoded {} frames, expected {}",
        decoded.frames(),
        pcm.frames()
    );
}

#[test]
fn every_sample_rate_tier_roundtrips_exactly() {
    // Spans all AoTuV `m_val` tiers (0 below 26 kHz, .94 in [26k,38k), 1.275
    // above 46 kHz, 1.0 between) and the rate-scaled psy/masking/octave/bark
    // state, on both a tone and broadband content.
    for &rate in &[
        8_000u32, 11_025, 16_000, 22_050, 24_000, 32_000, 44_100, 48_000, 96_000,
    ] {
        // A tone safely below Nyquist at every rate, ~2400 frames.
        let tone = sine(rate, 1, 2400, 600.0, 0.5);
        roundtrip_is_exact(&tone, &format!("tone@{rate}"));
        let bb = noise(rate, 1, 2400, 0.4);
        roundtrip_is_exact(&bb, &format!("noise@{rate}"));
    }
}

#[test]
fn stereo_across_rates_roundtrips_exactly() {
    for &rate in &[8_000u32, 22_050, 44_100, 48_000, 96_000] {
        let pcm = sine(rate, 2, 3000, 440.0, 0.5);
        roundtrip_is_exact(&pcm, &format!("stereo@{rate}"));
    }
}

#[test]
fn extreme_signal_shapes_are_panic_free_and_exact() {
    let rate = 48_000;
    // Pure silence.
    roundtrip_is_exact(&build(rate, 1, 4000, |_| 0.0), "silence");
    // A DC offset (all bins zero but the constant — stresses the floor/MDCT).
    roundtrip_is_exact(&build(rate, 1, 4000, |_| 0.8), "dc");
    // Full-scale tone at the [-1, 1] limit.
    roundtrip_is_exact(&sine(rate, 1, 4000, 1000.0, 1.0), "full_scale");
    // Out-of-range samples (clipping past 1.0): the encoder must not panic or
    // overflow on input it does not normalize.
    roundtrip_is_exact(&sine(rate, 1, 4000, 1000.0, 3.5), "out_of_range");
    // Near-Nyquist and near-DC tones.
    roundtrip_is_exact(&sine(rate, 1, 4000, 23_500.0, 0.5), "near_nyquist");
    roundtrip_is_exact(&sine(rate, 1, 4000, 20.0, 0.5), "near_dc");
    // A loud broadband burst after silence (forces block switching).
    roundtrip_is_exact(
        &build(rate, 1, 8192, |i| {
            if i < 4096 {
                0.0
            } else {
                let mut s = (i as u32).wrapping_mul(2_654_435_761);
                s ^= s >> 15;
                (s as f32 / u32::MAX as f32 - 0.5) * 1.2
            }
        }),
        "switched_burst",
    );
}

#[test]
fn boundary_lengths_roundtrip_exactly() {
    let rate = 48_000;
    // From a single sample up through and just past the long-block boundary, and
    // a multichannel case, all sample-accurate.
    for &(ch, frames) in &[
        (1u16, 1usize),
        (1, 127),
        (1, 256),
        (1, 2047),
        (1, 2048),
        (1, 2049),
        (2, 1),
        (6, 777),
    ] {
        let pcm = sine(rate, ch, frames, 440.0, 0.5);
        roundtrip_is_exact(&pcm, &format!("len_{ch}ch_{frames}"));
    }
}

#[test]
fn empty_input_encodes_to_a_valid_header_only_stream() {
    // Zero frames: the encoder still emits the three Vorbis headers in a valid
    // Ogg stream (no audio packets), and it must not panic.
    let pcm = AudioBuffer::new(48_000, 1, Vec::new()).expect("pcm");
    let bytes = sc_vorbis::encode(&pcm).expect("encode empty");
    assert_eq!(&bytes[..4], b"OggS", "empty: not an Ogg stream");
    assert_eq!(sc_core::detect(&bytes), Some(Format::Vorbis));
}
