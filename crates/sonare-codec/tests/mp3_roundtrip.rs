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

/// Best-lag correlation of a reference channel against a decoded channel,
/// scanning a window of lags to absorb the codec delay. Returns the correlation.
fn aligned_channel_corr(reference: &[f32], decoded: &[f32], seg: usize, ref_start: usize) -> f64 {
    let reference = &reference[ref_start..ref_start + seg];
    let mut best = f64::NEG_INFINITY;
    for d in 0..2_000 {
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
fn mp3_stereo_roundtrip_reconstructs_both_channels() {
    let sample_rate = 44_100;
    let frames = 22_050;
    // Two distinct sweeps so a channel swap or cross-talk would be visible.
    let left: Vec<f32> = (0..frames)
        .map(|i| {
            let t = i as f32 / sample_rate as f32;
            let f = 300.0 + 5_000.0 * (i as f32 / frames as f32);
            0.5 * (std::f32::consts::TAU * f * t).sin()
        })
        .collect();
    // A steady tone on the right channel: distinct from the left sweep so a swap
    // or cross-talk is visible, and it aligns exactly (no chirp penalty).
    let right: Vec<f32> = (0..frames)
        .map(|i| 0.5 * (std::f32::consts::TAU * 1_500.0 * (i as f32 / sample_rate as f32)).sin())
        .collect();
    let interleaved: Vec<f32> = left
        .iter()
        .zip(&right)
        .flat_map(|(&l, &r)| [l, r])
        .collect();
    let pcm = AudioBuffer::new(sample_rate, 2, interleaved).unwrap();

    let mp3 = sonare_codec::encode(Format::Mp3, &pcm).expect("MP3 encode");
    let decoded = sonare_codec::decode(&mp3).expect("Symphonia decode");
    assert_eq!(decoded.channels, 2, "expected stereo reconstruction");

    let dec_left: Vec<f32> = decoded.samples.iter().step_by(2).copied().collect();
    let dec_right: Vec<f32> = decoded.samples.iter().skip(1).step_by(2).copied().collect();

    let seg = 8_192;
    let ref_start = 6_000;
    let lc = aligned_channel_corr(&left, &dec_left, seg, ref_start);
    let rc = aligned_channel_corr(&right, &dec_right, seg, ref_start);
    // Cross-correlation should be low: left input must not match the right channel.
    let cross = aligned_channel_corr(&left, &dec_right, seg, ref_start);
    eprintln!("stereo roundtrip: left_corr={lc:.4} right_corr={rc:.4} cross(L vs Rdec)={cross:.4}");

    assert!(lc > 0.6, "left channel correlation too low: {lc:.4}");
    assert!(rc > 0.6, "right channel correlation too low: {rc:.4}");
    // Channel separation: each decoded channel must match its own input far
    // better than the other channel's input (proves no swap or cross-talk).
    assert!(
        lc > cross + 0.3,
        "channels not separated (L corr {lc:.4} vs cross {cross:.4})"
    );
}

#[test]
fn mp3_multirate_roundtrip_reconstructs_mono_and_stereo() {
    // A 1 kHz tone must reconstruct through Symphonia for every supported
    // MPEG-1 sample rate, in both mono and stereo. Stereo runs each channel
    // through the real polyphase filterbank (a subband scaffold would fail
    // this), so it also guards the stereo analysis path.
    let seg = 8_192;
    let ref_start = 6_000;
    for &rate in &[32_000_u32, 44_100, 48_000] {
        let frames = 22_050;
        let tone: Vec<f32> = (0..frames)
            .map(|i| 0.5 * (std::f32::consts::TAU * 1_000.0 * (i as f32 / rate as f32)).sin())
            .collect();

        let mono = AudioBuffer::new(rate, 1, tone.clone()).unwrap();
        let dec = sonare_codec::decode(&sonare_codec::encode(Format::Mp3, &mono).unwrap()).unwrap();
        let mc = aligned_channel_corr(&tone, &dec.samples, seg, ref_start);
        assert!(mc > 0.95, "mono {rate} Hz tone corr too low: {mc:.4}");

        let interleaved: Vec<f32> = tone.iter().flat_map(|&s| [s, s]).collect();
        let stereo = AudioBuffer::new(rate, 2, interleaved).unwrap();
        let dec =
            sonare_codec::decode(&sonare_codec::encode(Format::Mp3, &stereo).unwrap()).unwrap();
        let dl: Vec<f32> = dec.samples.iter().step_by(2).copied().collect();
        let dr: Vec<f32> = dec.samples.iter().skip(1).step_by(2).copied().collect();
        let lc = aligned_channel_corr(&tone, &dl, seg, ref_start);
        let rc = aligned_channel_corr(&tone, &dr, seg, ref_start);
        assert!(lc > 0.95, "stereo {rate} Hz left corr too low: {lc:.4}");
        assert!(rc > 0.95, "stereo {rate} Hz right corr too low: {rc:.4}");
    }
}

#[test]
fn mp3_reservoir_roundtrip_decodes_through_symphonia() {
    // The bit-reservoir encoder lets frames borrow main-data bytes from earlier
    // frames via main_data_begin. Symphonia must reassemble that cross-frame
    // stream and reconstruct the sweep just like the self-contained encoder does;
    // a wrong main_data_begin or payload offset collapses this to noise.
    let sample_rate = 44_100;
    let pcm = sweep_pcm(22_050, sample_rate, 300.0, 6_000.0, 0.5);

    let mp3 = sonare_codec::encode_mpeg1_layer3_pcm_frames_with_reservoir_and_table_provider(
        &pcm,
        sonare_codec::MPEG1_LAYER3_PCM_STEP_CANDIDATES,
        128,
        false,
        sonare_codec::mpeg1_layer3_standard_table_provider(),
    )
    .expect("reservoir MP3 encode");
    let decoded = sonare_codec::decode(&mp3).expect("Symphonia decode");
    assert_eq!(decoded.channels, 1, "expected mono reconstruction");

    let seg = 8_192;
    let ref_start = 8_000;
    let corr = aligned_channel_corr(&pcm.samples, &decoded.samples, seg, ref_start);
    // Recover the aligned segment to also check the level is sane.
    let reference = &pcm.samples[ref_start..ref_start + seg];
    let mut best = (0_usize, f64::NEG_INFINITY);
    for d in 0..2_000 {
        let start = ref_start + d;
        if start + seg > decoded.samples.len() {
            break;
        }
        let c = correlation(reference, &decoded.samples[start..start + seg]);
        if c > best.1 {
            best = (d, c);
        }
    }
    let aligned = &decoded.samples[ref_start + best.0..ref_start + best.0 + seg];
    let level_ratio = rms(aligned) / rms(reference).max(1.0e-12);
    eprintln!("reservoir roundtrip: corr={corr:.4} ratio={level_ratio:.3}");

    assert!(
        corr > 0.6,
        "reservoir waveform correlation too low: {corr:.4}"
    );
    assert!(
        (0.5..2.0).contains(&level_ratio),
        "reservoir decoded level out of range: ratio={level_ratio:.3}"
    );
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

/// Goertzel power of `samples` at frequency `f`.
fn goertzel(samples: &[f32], sample_rate: u32, f: f32) -> f64 {
    let w = std::f64::consts::TAU * f as f64 / sample_rate as f64;
    let coeff = 2.0 * w.cos();
    let (mut s1, mut s2) = (0.0_f64, 0.0_f64);
    for &x in samples {
        let s0 = f64::from(x) + coeff * s1 - s2;
        s2 = s1;
        s1 = s0;
    }
    s1 * s1 + s2 * s2 - coeff * s1 * s2
}

#[ignore = "diagnostic: steady-tone reconstruction SNR (exact integer alignment)"]
#[test]
fn mp3_roundtrip_tone_snr() {
    let sample_rate = 44_100;
    for &f_in in &[500.0_f32, 2_000.0, 6_000.0] {
        let frames = 22_050;
        let samples: Vec<f32> = (0..frames)
            .map(|i| 0.5 * (std::f32::consts::TAU * f_in * (i as f32 / sample_rate as f32)).sin())
            .collect();
        let pcm = AudioBuffer::new(sample_rate, 1, samples).unwrap();
        let mp3 = sonare_codec::encode(Format::Mp3, &pcm).expect("MP3 encode");
        let decoded = sonare_codec::decode(&mp3).expect("Symphonia decode");

        // A steady tone aligns exactly at a single integer lag, so correlation
        // reflects true reconstruction quality (unlike a chirp).
        let seg = 8_192;
        let ref_start = 8_000;
        let reference = &pcm.samples[ref_start..ref_start + seg];
        let mut best = (0_usize, f64::NEG_INFINITY);
        for d in 0..2_000 {
            let start = ref_start + d;
            if start + seg > decoded.samples.len() {
                break;
            }
            let c = correlation(reference, &decoded.samples[start..start + seg]);
            if c > best.1 {
                best = (d, c);
            }
        }
        let aligned = &decoded.samples[ref_start + best.0..ref_start + best.0 + seg];
        let noise: f64 = reference
            .iter()
            .zip(aligned)
            .map(|(&r, &a)| {
                let e = f64::from(r) - f64::from(a);
                e * e
            })
            .sum();
        let signal: f64 = reference.iter().map(|&r| f64::from(r) * f64::from(r)).sum();
        let snr = 10.0 * (signal / noise.max(1.0e-30)).log10();
        eprintln!(
            "tone snr: f={f_in:>6.0} corr={:.4} ratio={:.3} snr={snr:.1}dB",
            best.1,
            rms(aligned) / rms(reference).max(1.0e-12),
        );
    }
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

    // Compare a clean middle segment, skipping the zero-primed first granules
    // and the polyphase filterbank's priming delay. MP3 adds an encoder/decoder
    // delay of roughly a thousand samples, so scan a window of lags to align.
    let seg = 8_192;
    let ref_start = 8_000;
    let reference = &pcm.samples[ref_start..ref_start + seg];
    let mut best = (0_usize, f64::NEG_INFINITY);
    for d in 0..2_000 {
        let start = ref_start + d;
        if start + seg > decoded.samples.len() {
            break;
        }
        let c = correlation(reference, &decoded.samples[start..start + seg]);
        if c > best.1 {
            best = (d, c);
        }
    }
    let (delay, corr) = best;
    let aligned = &decoded.samples[ref_start + delay..ref_start + delay + seg];
    let level_ratio = rms(aligned) / rms(reference).max(1.0e-12);

    eprintln!(
        "mp3 roundtrip: delay={delay} corr={corr:.4} input_rms={:.4} decoded_rms={:.4} ratio={level_ratio:.3}",
        rms(reference),
        rms(aligned),
    );

    // Shape: the sweep must survive the lossy roundtrip with strong correlation.
    // The simple uniform-step encoder (no scalefactors or bit reservoir yet)
    // reconstructs at roughly 0.85 on this sweep.
    assert!(corr > 0.6, "waveform correlation too low: {corr:.4}");
    // Level: the calibrated global_gain plus the IMDCT-normalization offset must
    // reconstruct close to unity. An uncalibrated encoder lands ~9x off.
    assert!(
        (0.5..2.0).contains(&level_ratio),
        "decoded level out of calibrated range: ratio={level_ratio:.3}"
    );
}
