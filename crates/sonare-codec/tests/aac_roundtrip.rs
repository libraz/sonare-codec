//! End-to-end AAC-LC roundtrip smoke tests through Symphonia. These guard the
//! production `encode()` quantizer/scalefactor path: the encoder must fill the
//! bitrate budget (not collapse to ~17 kbps), reconstruct the input shape, and
//! preserve level. Production lossy quality is otherwise gated by the FFmpeg
//! oracle in xtask; these keep the local Symphonia integration honest.

#![cfg(all(feature = "aac", feature = "decode"))]

use sc_core::{AudioBuffer, Format};

/// Linear frequency sweep on every channel. A sweep is non-periodic, so the
/// best-lag correlation below is meaningful (a pure tone self-correlates at many
/// lags and would mask misalignment).
fn sweep_pcm(frames: usize, sample_rate: u32, channels: u16, amplitude: f32) -> AudioBuffer {
    let n = frames as f32;
    let mut samples = Vec::with_capacity(frames * channels as usize);
    for i in 0..frames {
        let t = i as f32 / sample_rate as f32;
        let f = 300.0 + 5_000.0 * (i as f32 / n);
        let v = amplitude * (std::f32::consts::TAU * f * t).sin();
        for _ in 0..channels {
            samples.push(v);
        }
    }
    AudioBuffer::new(sample_rate, channels, samples).unwrap()
}

fn rms(samples: &[f32]) -> f64 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum: f64 = samples.iter().map(|&s| f64::from(s) * f64::from(s)).sum();
    (sum / samples.len() as f64).sqrt()
}

fn correlation(a: &[f32], b: &[f32]) -> f64 {
    let mut dot = 0.0;
    let mut na = 0.0;
    let mut nb = 0.0;
    for i in 0..a.len() {
        dot += f64::from(a[i]) * f64::from(b[i]);
        na += f64::from(a[i]) * f64::from(a[i]);
        nb += f64::from(b[i]) * f64::from(b[i]);
    }
    if na == 0.0 || nb == 0.0 {
        return 0.0;
    }
    dot / (na.sqrt() * nb.sqrt())
}

/// Best-lag correlation of a reference channel against a decoded channel,
/// scanning lags to absorb the AAC codec delay.
fn aligned_channel_corr(reference: &[f32], decoded: &[f32], seg: usize, ref_start: usize) -> f64 {
    let reference = &reference[ref_start..ref_start + seg];
    let mut best = f64::NEG_INFINITY;
    for d in 0..4_096 {
        let start = ref_start + d;
        if start + seg > decoded.len() {
            break;
        }
        let c = correlation(reference, &decoded[start..start + seg]);
        if c > best {
            best = c;
        }
    }
    best
}

#[test]
fn aac_mono_roundtrip_fills_budget_and_reconstructs() {
    let sample_rate = 44_100;
    let frames = 44_100; // 1 second
    let pcm = sweep_pcm(frames, sample_rate, 1, 0.5);
    let secs = frames as f64 / sample_rate as f64;

    let aac = sonare_codec::encode(Format::Aac, &pcm).expect("AAC encode");
    let kbps = (aac.len() as f64 * 8.0) / secs / 1000.0;
    // The over-quantization bug pinned output at ~17 kbps regardless of budget.
    // A correct step search fills the ~128 kbps/channel production budget.
    assert!(kbps > 60.0, "AAC mono bitrate too low: {kbps:.1} kbps");

    let decoded = sonare_codec::decode(&aac).expect("Symphonia decode");
    assert_eq!(decoded.channels, 1);

    let reference: Vec<f32> = pcm.samples.clone();
    let corr = aligned_channel_corr(&reference, &decoded.samples, 8_192, 4_000);
    let level = rms(&decoded.samples) / rms(&reference).max(1e-9);
    eprintln!("aac mono: {kbps:.1} kbps corr={corr:.4} level_ratio={level:.3}");

    assert!(corr > 0.95, "AAC mono correlation too low: {corr:.4}");
    // Level must be preserved within a few dB (scalefactor calibration).
    assert!(
        (0.5..2.0).contains(&level),
        "AAC mono level not preserved: ratio {level:.3}"
    );
}

#[test]
fn aac_explicit_bitrate_api_preserves_level() {
    // `encode_aac_adts_with_bitrate` previously pinned the scale factors to 180
    // regardless of the quantizer step chosen by the bit-cost search, shifting
    // the decoded level by orders of magnitude. It must now track the input
    // level like the production path.
    let sample_rate = 44_100;
    let frames = 44_100;
    let pcm = sweep_pcm(frames, sample_rate, 1, 0.5);

    let aac =
        sonare_codec::encode_aac_adts_with_bitrate(&pcm, 128_000).expect("AAC bitrate encode");
    let decoded = sonare_codec::decode(&aac).expect("Symphonia decode");
    assert_eq!(decoded.channels, 1);

    let corr = aligned_channel_corr(&pcm.samples, &decoded.samples, 8_192, 4_000);
    let level = rms(&decoded.samples) / rms(&pcm.samples).max(1e-9);
    eprintln!("aac bitrate-api: corr={corr:.4} level_ratio={level:.3}");

    assert!(
        corr > 0.95,
        "explicit-bitrate AAC correlation too low: {corr:.4}"
    );
    assert!(
        (0.5..2.0).contains(&level),
        "explicit-bitrate AAC level not preserved: ratio {level:.3}"
    );
}

#[test]
fn aac_stereo_roundtrip_fills_budget_and_reconstructs() {
    let sample_rate = 48_000;
    let frames = 48_000;
    let pcm = sweep_pcm(frames, sample_rate, 2, 0.5);
    let secs = frames as f64 / sample_rate as f64;

    let aac = sonare_codec::encode(Format::Aac, &pcm).expect("AAC encode");
    let kbps = (aac.len() as f64 * 8.0) / secs / 1000.0;
    assert!(kbps > 120.0, "AAC stereo bitrate too low: {kbps:.1} kbps");

    let decoded = sonare_codec::decode(&aac).expect("Symphonia decode");
    assert_eq!(decoded.channels, 2);

    let left: Vec<f32> = pcm.samples.iter().step_by(2).copied().collect();
    let dec_left: Vec<f32> = decoded.samples.iter().step_by(2).copied().collect();
    let corr = aligned_channel_corr(&left, &dec_left, 8_192, 4_000);
    let level = rms(&decoded.samples) / rms(&pcm.samples).max(1e-9);
    eprintln!("aac stereo: {kbps:.1} kbps corr={corr:.4} level_ratio={level:.3}");

    assert!(corr > 0.95, "AAC stereo correlation too low: {corr:.4}");
    assert!(
        (0.5..2.0).contains(&level),
        "AAC stereo level not preserved: ratio {level:.3}"
    );
}
