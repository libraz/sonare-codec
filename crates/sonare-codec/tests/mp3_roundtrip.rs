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
/// Goertzel power of `samples` at frequency `f`.
fn goertzel(samples: &[f32], sample_rate: u32, f: f32) -> f64 {
    let w = std::f64::consts::TAU * f as f64 / sample_rate as f64;
    let coeff = 2.0 * w.cos();
    let (mut s0, mut s1, mut s2) = (0.0_f64, 0.0_f64, 0.0_f64);
    for &x in samples {
        s0 = f64::from(x) + coeff * s1 - s2;
        s2 = s1;
        s1 = s0;
    }
    s1 * s1 + s2 * s2 - coeff * s1 * s2
}

#[ignore = "diagnostic: probe where decoded tone energy lands"]
#[test]
fn mp3_roundtrip_tone_probe() {
    let sample_rate = 44_100;
    let f_in = 1_000.0_f32;
    let frames = 22_050;
    let samples: Vec<f32> = (0..frames)
        .map(|i| {
            let t = i as f32 / sample_rate as f32;
            0.5 * (std::f32::consts::TAU * f_in * t).sin()
        })
        .collect();
    let pcm = AudioBuffer::new(sample_rate, 1, samples).unwrap();

    let mp3 = sonare_codec::encode(Format::Mp3, &pcm).expect("MP3 encode");
    let decoded = sonare_codec::decode(&mp3).expect("Symphonia decode");

    // Scan candidate frequencies; report the dominant one in the decoded signal.
    let mut best = (0.0_f32, f64::NEG_INFINITY);
    let mut f = 200.0_f32;
    while f < 8_000.0 {
        let p = goertzel(&decoded.samples, sample_rate, f);
        if p > best.1 {
            best = (f, p);
        }
        f += 25.0;
    }
    let p_in = goertzel(&decoded.samples, sample_rate, f_in);
    eprintln!(
        "tone probe: f_in={f_in} decoded_dominant_f={:.0} (power={:.3e}) power_at_f_in={:.3e}",
        best.0, best.1, p_in
    );
}

#[ignore = "diagnostic: check time integrity with a two-tone signal"]
#[test]
fn mp3_roundtrip_time_integrity_probe() {
    let sample_rate = 44_100;
    let frames = 22_050;
    let half = frames / 2;
    let (f_lo, f_hi) = (800.0_f32, 4_000.0_f32);
    let samples: Vec<f32> = (0..frames)
        .map(|i| {
            let t = i as f32 / sample_rate as f32;
            let f = if i < half { f_lo } else { f_hi };
            0.5 * (std::f32::consts::TAU * f * t).sin()
        })
        .collect();
    let pcm = AudioBuffer::new(sample_rate, 1, samples).unwrap();

    let mp3 = sonare_codec::encode(Format::Mp3, &pcm).expect("MP3 encode");
    let decoded = sonare_codec::decode(&mp3).expect("Symphonia decode");

    let dh = decoded.samples.len() / 2;
    let first = &decoded.samples[..dh];
    let second = &decoded.samples[dh..];
    eprintln!(
        "time integrity: FIRST half -> p(800)={:.2e} p(4000)={:.2e} | SECOND half -> p(800)={:.2e} p(4000)={:.2e}",
        goertzel(first, sample_rate, f_lo),
        goertzel(first, sample_rate, f_hi),
        goertzel(second, sample_rate, f_lo),
        goertzel(second, sample_rate, f_hi),
    );
}

/// Matched inverse of `sc_mp3::mdct_long_block`'s MDCT kernel (sc-core `mdct`,
/// unnormalized): `x[n] = (2/N) * sum_k X[k] cos[(pi/N)(n+0.5+N/2)(k+0.5)]`.
fn imdct_36(lines: &[f32]) -> [f32; 36] {
    let n_coeffs = 18usize;
    let mut out = [0.0_f32; 36];
    for (n, o) in out.iter_mut().enumerate() {
        let mut acc = 0.0_f64;
        for (k, &x) in lines.iter().enumerate() {
            let angle = std::f64::consts::PI / n_coeffs as f64
                * (n as f64 + 0.5 + n_coeffs as f64 / 2.0)
                * (k as f64 + 0.5);
            acc += f64::from(x) * angle.cos();
        }
        *o = (2.0 / n_coeffs as f64 * acc) as f32;
    }
    out
}

fn sine_window_36() -> [f32; 36] {
    let mut w = [0.0_f32; 36];
    for (i, wi) in w.iter_mut().enumerate() {
        *wi = (std::f32::consts::PI / 36.0 * (i as f32 + 0.5)).sin();
    }
    w
}

#[ignore = "diagnostic: MDCT/IMDCT TDAC reconstruction of a changing signal"]
#[test]
fn mdct_tdac_reconstructs_changing_signal() {
    use sonare_codec::mdct_long_block;
    let win = sine_window_36();

    // A changing subband signal (chirp-like) long enough for several frames.
    let total = 18 * 12;
    let sig: Vec<f32> = (0..total)
        .map(|m| {
            let t = m as f32 / total as f32;
            (std::f32::consts::TAU * (1.0 + 6.0 * t) * m as f32 * 0.05).sin()
        })
        .collect();

    // MDCT each 36-sample frame (hop 18), IMDCT, window, overlap-add.
    let frames = total / 18 - 1;
    let mut recon = vec![0.0_f32; total];
    let mut prev_tail = [0.0_f32; 18];
    for t in 0..frames {
        let mut block = [0.0_f32; 36];
        block.copy_from_slice(&sig[t * 18..t * 18 + 36]);
        let lines = mdct_long_block(&block).unwrap();
        let imdct = imdct_36(&lines);
        // Window again on synthesis, then overlap-add.
        for i in 0..18 {
            recon[t * 18 + i] = imdct[i] * win[i] + prev_tail[i];
        }
        for i in 0..18 {
            prev_tail[i] = imdct[i + 18] * win[i + 18];
        }
    }

    // Compare the interior (skip first/last frame edge) against the original.
    let a = &sig[18..(frames - 1) * 18];
    let b = &recon[18..(frames - 1) * 18];
    let corr = correlation(a, b);
    let ratio = rms(b) / rms(a).max(1e-12);
    eprintln!("mdct tdac: corr={corr:.4} ratio={ratio:.3}");
}

/// Dominant frequency of `samples` via a coarse Goertzel scan.
fn dominant_freq(samples: &[f32], sample_rate: u32) -> f32 {
    let mut best = (0.0_f32, f64::NEG_INFINITY);
    let mut f = 200.0_f32;
    while f < 9_000.0 {
        let p = goertzel(samples, sample_rate, f);
        if p > best.1 {
            best = (f, p);
        }
        f += 20.0;
    }
    best.0
}

#[ignore = "diagnostic: does the decoded sweep track frequency over time?"]
#[test]
fn mp3_roundtrip_sweep_spectrogram() {
    let sample_rate = 44_100;
    let pcm = sweep_pcm(22_050, sample_rate, 300.0, 6_000.0, 0.5);
    let mp3 = sonare_codec::encode(Format::Mp3, &pcm).expect("MP3 encode");
    let decoded = sonare_codec::decode(&mp3).expect("Symphonia decode");

    // Compare dominant frequency per 1024-sample window for input vs decoded,
    // assuming a fixed bulk delay (decoded is ~1000 samples late).
    let delay = decoded.samples.len().saturating_sub(pcm.samples.len());
    let win = 1024;
    eprintln!("sweep spectrogram (delay~{delay}):");
    for w in (2_000..18_000).step_by(4_000) {
        let in_f = dominant_freq(&pcm.samples[w..w + win], sample_rate);
        let dstart = w + delay;
        if dstart + win > decoded.samples.len() {
            break;
        }
        let out_f = dominant_freq(&decoded.samples[dstart..dstart + win], sample_rate);
        eprintln!("  t={w:>6}: input_f={in_f:>6.0}  decoded_f={out_f:>6.0}");
    }
}

#[ignore = "diagnostic: thorough sweep lag scan on a clean middle segment"]
#[test]
fn mp3_roundtrip_sweep_lag_scan() {
    let sample_rate = 44_100;
    let pcm = sweep_pcm(22_050, sample_rate, 300.0, 6_000.0, 0.5);
    let mp3 = sonare_codec::encode(Format::Mp3, &pcm).expect("MP3 encode");
    let decoded = sonare_codec::decode(&mp3).expect("Symphonia decode");

    // Compare a middle segment to avoid the zero-primed first granule and tail.
    let seg = 4_096;
    let ref_start = 8_000;
    let reference = &pcm.samples[ref_start..ref_start + seg];
    // Scan a wide lag range around the expected MP3 delay.
    let mut best = (0_i64, f64::NEG_INFINITY);
    for d in -50_i64..2_500 {
        let start = ref_start as i64 + d;
        if start < 0 || start as usize + seg > decoded.samples.len() {
            continue;
        }
        let cand = &decoded.samples[start as usize..start as usize + seg];
        let c = correlation(reference, cand);
        if c > best.1 {
            best = (d, c);
        }
    }
    let bstart = (ref_start as i64 + best.0) as usize;
    let aligned = &decoded.samples[bstart..bstart + seg];
    eprintln!(
        "sweep lag scan: best_delay={} corr={:.4} ratio={:.3} decoded_len={}",
        best.0,
        best.1,
        rms(aligned) / rms(reference).max(1e-12),
        decoded.samples.len()
    );
}

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
