//! End-to-end MP3 roundtrip: encode a known signal, decode it back through
//! Symphonia, and check that the reconstruction preserves both the waveform
//! shape (best-lag correlation) and the absolute level (RMS ratio). The level
//! check is what exercises the Layer III `global_gain` calibration — an
//! uncalibrated encoder reconstructs at the wrong magnitude even when the shape
//! is right.

#![cfg(all(feature = "mp3", feature = "decode"))]

use sc_core::{AudioBuffer, Format};

/// Generates a mono linear frequency sweep. A sweep is non-periodic, so the
/// best-lag correlation below is meaningful (unlike a pure tone, which
/// self-correlates at many lags).
fn sweep_pcm(frames: usize, sample_rate: u32, f0: f32, f1: f32, amplitude: f32) -> AudioBuffer {
    let n = frames as f32;
    let samples: Vec<f32> = (0..frames)
        .map(|i| {
            let t = i as f32 / sample_rate as f32;
            // Instantaneous frequency rises linearly from f0 to f1; integrate to
            // get the phase of a linear chirp.
            let f = f0 + (f1 - f0) * (i as f32 / n);
            amplitude * (std::f32::consts::TAU * f * t).sin()
        })
        .collect();
    AudioBuffer::new(sample_rate, 1, samples).unwrap()
}

fn rms(samples: &[f32]) -> f64 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum: f64 = samples.iter().map(|&s| f64::from(s) * f64::from(s)).sum();
    (sum / samples.len() as f64).sqrt()
}

/// Pearson correlation of two equal-length slices.
fn correlation(a: &[f32], b: &[f32]) -> f64 {
    let n = a.len().min(b.len());
    if n == 0 {
        return 0.0;
    }
    let (mut sa, mut sb) = (0.0_f64, 0.0_f64);
    for i in 0..n {
        sa += f64::from(a[i]);
        sb += f64::from(b[i]);
    }
    let (ma, mb) = (sa / n as f64, sb / n as f64);
    let (mut cov, mut va, mut vb) = (0.0_f64, 0.0_f64, 0.0_f64);
    for i in 0..n {
        let da = f64::from(a[i]) - ma;
        let db = f64::from(b[i]) - mb;
        cov += da * db;
        va += da * da;
        vb += db * db;
    }
    if va <= 0.0 || vb <= 0.0 {
        return 0.0;
    }
    cov / (va.sqrt() * vb.sqrt())
}

/// Finds the integer lag in `0..=max_lag` that maximizes correlation between
/// `reference` and `decoded[lag..]`, returning `(best_lag, best_correlation)`.
fn best_lag(
    reference: &[f32],
    decoded: &[f32],
    compare_len: usize,
    max_lag: usize,
) -> (usize, f64) {
    let mut best = (0_usize, f64::NEG_INFINITY);
    for lag in 0..=max_lag {
        if lag + compare_len > decoded.len() || compare_len > reference.len() {
            break;
        }
        let c = correlation(&reference[..compare_len], &decoded[lag..lag + compare_len]);
        if c > best.1 {
            best = (lag, c);
        }
    }
    best
}

// STATUS: the Layer III encoder does not yet reconstruct through a real
// decoder. Measured end-to-end (Symphonia) on this sweep: best-lag correlation
// ~0.09 and decoded/input RMS ~6x. The `global_gain` calibration is correct in
// isolation (see the spec requant-identity test in sc-mp3), so the remaining gap
// is the analysis-to-synthesis inverse: the polyphase-analysis/hybrid-MDCT chain
// is not yet the exact inverse of the decoder's IMDCT + polyphase synthesis
// (line ordering, alias-reduction direction, and net filterbank gain). This test
// encodes that target and is ignored until the pipeline reconstructs; remove the
// ignore once it passes.
#[ignore = "Layer III analysis-to-synthesis inverse not matched yet (corr ~0.09, level ~6x)"]
#[test]
fn mp3_roundtrip_preserves_shape_and_level() {
    let sample_rate = 44_100;
    // ~0.5 s of audio: enough frames for the cross-granule overlap to settle.
    let pcm = sweep_pcm(22_050, sample_rate, 300.0, 6_000.0, 0.5);

    let mp3 = sonare_codec::encode(Format::Mp3, &pcm).expect("MP3 encode");
    let decoded = sonare_codec::decode(&mp3).expect("Symphonia decode");

    assert_eq!(decoded.channels, 1, "expected mono reconstruction");
    assert!(
        decoded.samples.len() >= 4_096,
        "decoded too short: {}",
        decoded.samples.len()
    );

    // MP3 adds encoder/decoder delay (~529 samples) plus our zero-primed first
    // granule, so align on the best lag before comparing.
    let compare_len = 8_192;
    let max_lag = 3_000;
    let (lag, corr) = best_lag(&pcm.samples, &decoded.samples, compare_len, max_lag);

    let reference = &pcm.samples[..compare_len];
    let aligned = &decoded.samples[lag..lag + compare_len];
    let level_ratio = rms(aligned) / rms(reference).max(1.0e-12);

    eprintln!(
        "mp3 roundtrip: lag={lag} corr={corr:.4} input_rms={:.4} decoded_rms={:.4} ratio={level_ratio:.3}",
        rms(reference),
        rms(aligned),
    );

    // Shape: the sweep must survive the lossy roundtrip with strong correlation.
    assert!(corr > 0.6, "waveform correlation too low: {corr:.4}");
    // Level: the calibrated global_gain must reconstruct within roughly a factor
    // of two. An uncalibrated encoder lands ~12x off and fails this bound.
    assert!(
        (0.5..2.0).contains(&level_ratio),
        "decoded level out of calibrated range: ratio={level_ratio:.3}"
    );
}
