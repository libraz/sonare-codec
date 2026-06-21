//! End-to-end MP3 roundtrip smoke tests through Symphonia. Production lossy
//! quality is gated by the FFmpeg oracle in xtask; these tests keep the local
//! Symphonia integration honest by checking that streams decode, carry sane
//! level, and preserve basic channel separation.

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

    assert!(lc > 0.2, "left channel correlation too low: {lc:.4}");
    assert!(rc > 0.6, "right channel correlation too low: {rc:.4}");
    // Channel separation: each decoded channel must match its own input far
    // better than the other channel's input (proves no swap or cross-talk).
    assert!(
        lc > cross + 0.2,
        "channels not separated (L corr {lc:.4} vs cross {cross:.4})"
    );
}

/// Reads the channel-mode field (byte 3, bits 7-6) of the first MP3 frame.
fn first_frame_channel_mode_bits(stream: &[u8]) -> u8 {
    assert_eq!(stream[0], 0xff, "stream must start with a frame sync");
    (stream[3] >> 6) & 0x03
}

#[test]
fn mp3_correlated_stereo_uses_mid_side_and_reconstructs_both_channels() {
    // Strongly correlated channels (right = 0.7 * left) trigger the MS joint-stereo
    // path. Symphonia must apply the inverse mid/side matrix to recover the
    // original left/right pair: a wrong matrix would collapse both channels to the
    // mid signal (equal power) or swap them. The per-channel tone power ratio is
    // the discriminator — it isolates the matrix from the encoder's absolute
    // quantization quality (which the FFmpeg oracle owns), so it stays valid even
    // though the coarse production quantizer reconstructs loud tones imperfectly.
    let sample_rate = 44_100;
    let frames = 22_050;
    let tone = 1_200.0_f32;
    let left: Vec<f32> = (0..frames)
        .map(|i| 0.3 * (std::f32::consts::TAU * tone * (i as f32 / sample_rate as f32)).sin())
        .collect();
    let right: Vec<f32> = left.iter().map(|&l| 0.7 * l).collect();
    let interleaved: Vec<f32> = left
        .iter()
        .zip(&right)
        .flat_map(|(&l, &r)| [l, r])
        .collect();
    let pcm = AudioBuffer::new(sample_rate, 2, interleaved).unwrap();

    let mp3 = sonare_codec::encode(Format::Mp3, &pcm).expect("MP3 encode");
    // 0b01 == joint stereo: the correlated input must take the MS path.
    assert_eq!(
        first_frame_channel_mode_bits(&mp3),
        0b01,
        "correlated stereo must be coded as joint (MS) stereo"
    );

    let decoded = sonare_codec::decode(&mp3).expect("Symphonia decode");
    assert_eq!(decoded.channels, 2, "expected stereo reconstruction");
    let dec_left: Vec<f32> = decoded.samples.iter().step_by(2).copied().collect();
    let dec_right: Vec<f32> = decoded.samples.iter().skip(1).step_by(2).copied().collect();

    // Tone power per channel. The inverse matrix must recover the input amplitude
    // ratio of 0.7, i.e. a power ratio of 0.49; if the decoder put the mid signal
    // on both channels the ratio would be 1.0, and a swap would give ~2.0.
    let pl = goertzel(&dec_left, sample_rate, f64::from(tone) as f32);
    let pr = goertzel(&dec_right, sample_rate, f64::from(tone) as f32);
    let power_ratio = pr / pl.max(1.0e-9);
    eprintln!("MS tone power ratio (right/left)={power_ratio:.3} (target 0.49)");
    assert!(
        pl > 0.0 && pr > 0.0,
        "decoded channels carry no tone energy"
    );
    assert!(
        (0.30..0.70).contains(&power_ratio),
        "MS reconstruction lost the channel level difference: ratio={power_ratio:.3}"
    );
}

#[test]
fn mp3_decorrelated_stereo_stays_independent() {
    // Distinct content on each channel keeps the side energy high, so the encoder
    // must NOT switch to MS joint stereo (no regression for true stereo).
    let sample_rate = 44_100;
    let frames = 22_050;
    let interleaved: Vec<f32> = (0..frames)
        .flat_map(|i| {
            let t = i as f32 / sample_rate as f32;
            let l = 0.5 * (std::f32::consts::TAU * 440.0 * t).sin();
            let r = 0.5 * (std::f32::consts::TAU * 3_000.0 * t).sin();
            [l, r]
        })
        .collect();
    let pcm = AudioBuffer::new(sample_rate, 2, interleaved).unwrap();

    let mp3 = sonare_codec::encode(Format::Mp3, &pcm).expect("MP3 encode");
    assert_eq!(
        first_frame_channel_mode_bits(&mp3),
        0b00,
        "decorrelated stereo must stay independent (left/right) stereo"
    );
    let decoded = sonare_codec::decode(&mp3).expect("Symphonia decode");
    assert_eq!(decoded.channels, 2);
}

#[test]
fn mp3_multirate_roundtrip_reconstructs_mono_and_stereo() {
    // A 1 kHz tone must decode through Symphonia for every supported MPEG-1
    // sample rate, in both mono and stereo. FFmpeg oracle tests own production
    // quality; this keeps local decode compatibility and gross tone integrity
    // covered.
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
        assert_eq!(dec.sample_rate, rate);
        assert_eq!(dec.channels, 2);
        assert!(
            dec.samples.len() >= seg * 2,
            "stereo {rate} Hz decoded too few samples: {}",
            dec.samples.len()
        );
        assert!(
            rms(&dec.samples) > 0.05,
            "stereo {rate} Hz decoded near silence"
        );
    }
}

#[test]
fn mp3_mpeg2_lsf_roundtrip_reconstructs_mono_and_stereo() {
    // MPEG-2 LSF (ISO/IEC 13818-3) low-sampling-frequency rates carry one
    // 576-sample granule per frame. A 1 kHz tone must encode through the
    // single-granule calibrated-gain path and decode back through Symphonia for
    // every LSF rate, in both mono and stereo.
    let seg = 8_192;
    let ref_start = 6_000;
    for &rate in &[16_000_u32, 22_050, 24_000] {
        let frames = 22_050;
        let tone: Vec<f32> = (0..frames)
            .map(|i| 0.5 * (std::f32::consts::TAU * 1_000.0 * (i as f32 / rate as f32)).sin())
            .collect();

        let mono = AudioBuffer::new(rate, 1, tone.clone()).unwrap();
        let mp3 = sonare_codec::encode(Format::Mp3, &mono).expect("MPEG-2 LSF mono encode");
        assert_eq!(
            sonare_codec::detect(&mp3),
            Some(Format::Mp3),
            "{rate} Hz: encoded stream must be detected as MP3"
        );
        let dec = sonare_codec::decode(&mp3).expect("Symphonia decode");
        assert_eq!(dec.sample_rate, rate, "{rate} Hz: rate must round-trip");
        assert_eq!(dec.channels, 1, "{rate} Hz: expected mono");
        let mc = aligned_channel_corr(&tone, &dec.samples, seg, ref_start);
        eprintln!("MPEG-2 LSF {rate} Hz mono tone corr={mc:.4}");
        assert!(mc > 0.8, "MPEG-2 LSF {rate} Hz mono corr too low: {mc:.4}");

        let interleaved: Vec<f32> = tone.iter().flat_map(|&s| [s, s]).collect();
        let stereo = AudioBuffer::new(rate, 2, interleaved).unwrap();
        let dec =
            sonare_codec::decode(&sonare_codec::encode(Format::Mp3, &stereo).unwrap()).unwrap();
        assert_eq!(dec.sample_rate, rate);
        assert_eq!(dec.channels, 2);
        assert!(
            rms(&dec.samples) > 0.05,
            "MPEG-2 LSF {rate} Hz stereo decoded near silence"
        );
    }
}

#[test]
fn mp3_mpeg2_lsf_correlated_stereo_uses_mid_side_and_reconstructs_both_channels() {
    // MPEG-2 LSF counterpart of the MPEG-1 MS test: correlated channels at an LSF
    // rate must take the joint-stereo path, and Symphonia must apply the inverse
    // mid/side matrix to recover the original left/right level difference. The
    // per-channel tone power ratio isolates the matrix from the coarse
    // single-granule quantizer's absolute quality.
    let sample_rate = 24_000;
    let frames = 22_050;
    let tone = 1_000.0_f32;
    let left: Vec<f32> = (0..frames)
        .map(|i| 0.3 * (std::f32::consts::TAU * tone * (i as f32 / sample_rate as f32)).sin())
        .collect();
    let right: Vec<f32> = left.iter().map(|&l| 0.7 * l).collect();
    let interleaved: Vec<f32> = left
        .iter()
        .zip(&right)
        .flat_map(|(&l, &r)| [l, r])
        .collect();
    let pcm = AudioBuffer::new(sample_rate, 2, interleaved).unwrap();

    let mp3 = sonare_codec::encode(Format::Mp3, &pcm).expect("MPEG-2 LSF MS encode");
    assert_eq!(
        first_frame_channel_mode_bits(&mp3),
        0b01,
        "correlated LSF stereo must be coded as joint (MS) stereo"
    );

    let decoded = sonare_codec::decode(&mp3).expect("Symphonia decode");
    assert_eq!(decoded.channels, 2, "expected stereo reconstruction");
    let dec_left: Vec<f32> = decoded.samples.iter().step_by(2).copied().collect();
    let dec_right: Vec<f32> = decoded.samples.iter().skip(1).step_by(2).copied().collect();

    let pl = goertzel(&dec_left, sample_rate, tone);
    let pr = goertzel(&dec_right, sample_rate, tone);
    let power_ratio = pr / pl.max(1.0e-9);
    eprintln!("MPEG-2 LSF MS tone power ratio (right/left)={power_ratio:.3} (target 0.49)");
    assert!(
        pl > 0.0 && pr > 0.0,
        "decoded channels carry no tone energy"
    );
    assert!(
        (0.30..0.70).contains(&power_ratio),
        "MPEG-2 LSF MS reconstruction lost the channel level difference: ratio={power_ratio:.3}"
    );
}

#[test]
fn mp3_mpeg2_lsf_decorrelated_stereo_stays_independent() {
    // Distinct content per channel at an LSF rate must keep independent stereo:
    // no MS regression for true stereo on the MPEG-2 path.
    let sample_rate = 24_000;
    let frames = 22_050;
    let interleaved: Vec<f32> = (0..frames)
        .flat_map(|i| {
            let t = i as f32 / sample_rate as f32;
            let l = 0.5 * (std::f32::consts::TAU * 440.0 * t).sin();
            let r = 0.5 * (std::f32::consts::TAU * 3_000.0 * t).sin();
            [l, r]
        })
        .collect();
    let pcm = AudioBuffer::new(sample_rate, 2, interleaved).unwrap();

    let mp3 = sonare_codec::encode(Format::Mp3, &pcm).expect("MPEG-2 LSF encode");
    assert_eq!(
        first_frame_channel_mode_bits(&mp3),
        0b00,
        "decorrelated LSF stereo must stay independent (left/right) stereo"
    );
    let decoded = sonare_codec::decode(&mp3).expect("Symphonia decode");
    assert_eq!(decoded.channels, 2);
}

#[test]
fn mp3_mpeg2_lsf_perceptual_reservoir_decodes_at_each_lsf_rate() {
    // Probe the entropy-targeted perceptual reservoir on the MPEG-2 LSF path
    // (the collector derives a single-granule MPEG-2 header with an 8-bit
    // main_data_begin from the LSF rate). A 1 kHz tone must encode through this
    // path and decode back through Symphonia with sane tone integrity at every
    // LSF rate, in mono and stereo, proving the reservoir machinery is version-
    // generic and the narrowed backward pointer is honoured.
    let seg = 8_192;
    let ref_start = 6_000;
    let provider = sc_mp3::mpeg1_layer3_standard_table_provider();
    for &rate in &[16_000_u32, 22_050, 24_000] {
        let frames = 22_050;
        let tone: Vec<f32> = (0..frames)
            .map(|i| 0.5 * (std::f32::consts::TAU * 1_000.0 * (i as f32 / rate as f32)).sin())
            .collect();

        let mono = AudioBuffer::new(rate, 1, tone.clone()).unwrap();
        let mp3 = sc_mp3::encode_mpeg2_layer3_pcm_frames_with_entropy_targeted_perceptual_reservoir_and_table_provider(
            &mono,
            sc_mp3::MPEG1_LAYER3_PCM_STEP_CANDIDATES,
            sc_mp3::MPEG2_LAYER3_DEFAULT_BITRATE_KBPS,
            false,
            0,
            provider,
        )
        .expect("MPEG-2 LSF perceptual reservoir mono encode");
        assert_eq!(
            sonare_codec::detect(&mp3),
            Some(Format::Mp3),
            "{rate} Hz: perceptual reservoir stream must be detected as MP3"
        );
        let dec = sonare_codec::decode(&mp3).expect("Symphonia decode");
        assert_eq!(dec.sample_rate, rate, "{rate} Hz: rate must round-trip");
        assert_eq!(dec.channels, 1, "{rate} Hz: expected mono");
        let mc = aligned_channel_corr(&tone, &dec.samples, seg, ref_start);
        eprintln!("MPEG-2 LSF perceptual reservoir {rate} Hz mono corr={mc:.4}");
        assert!(
            mc > 0.8,
            "MPEG-2 LSF perceptual reservoir {rate} Hz mono corr too low: {mc:.4}"
        );

        let interleaved: Vec<f32> = tone.iter().flat_map(|&s| [s, s]).collect();
        let stereo = AudioBuffer::new(rate, 2, interleaved).unwrap();
        let mp3 = sc_mp3::encode_mpeg2_layer3_pcm_frames_with_entropy_targeted_perceptual_reservoir_and_table_provider(
            &stereo,
            sc_mp3::MPEG1_LAYER3_PCM_STEP_CANDIDATES,
            sc_mp3::MPEG2_LAYER3_DEFAULT_BITRATE_KBPS,
            false,
            0,
            provider,
        )
        .expect("MPEG-2 LSF perceptual reservoir stereo encode");
        let dec = sonare_codec::decode(&mp3).expect("Symphonia decode");
        assert_eq!(dec.sample_rate, rate);
        assert_eq!(dec.channels, 2);
        assert!(
            rms(&dec.samples) > 0.05,
            "MPEG-2 LSF perceptual reservoir {rate} Hz stereo decoded near silence"
        );
    }
}

#[test]
fn mp3_mpeg2_lsf_perceptual_reservoir_mid_side_reconstructs_both_channels() {
    // The MPEG-2 LSF MS-perceptual path must mark joint stereo and let Symphonia
    // recover the original left/right level difference via the inverse mid/side
    // matrix. As with the other MS tests, the per-channel tone power ratio is the
    // discriminator (it isolates the matrix from the quantizer's absolute quality).
    let sample_rate = 24_000;
    let frames = 22_050;
    let tone = 1_000.0_f32;
    let left: Vec<f32> = (0..frames)
        .map(|i| 0.3 * (std::f32::consts::TAU * tone * (i as f32 / sample_rate as f32)).sin())
        .collect();
    let right: Vec<f32> = left.iter().map(|&l| 0.7 * l).collect();
    let interleaved: Vec<f32> = left
        .iter()
        .zip(&right)
        .flat_map(|(&l, &r)| [l, r])
        .collect();
    let pcm = AudioBuffer::new(sample_rate, 2, interleaved).unwrap();

    let mp3 = sc_mp3::encode_mpeg2_layer3_pcm_frames_with_entropy_targeted_perceptual_reservoir_mid_side_and_table_provider(
        &pcm,
        sc_mp3::MPEG1_LAYER3_PCM_STEP_CANDIDATES,
        sc_mp3::MPEG2_LAYER3_DEFAULT_BITRATE_KBPS,
        false,
        0,
        sc_mp3::mpeg1_layer3_standard_table_provider(),
    )
    .expect("MPEG-2 LSF MS-perceptual encode");
    assert_eq!(
        first_frame_channel_mode_bits(&mp3),
        0b01,
        "MS-perceptual stream must be coded as joint (MS) stereo"
    );

    let decoded = sonare_codec::decode(&mp3).expect("Symphonia decode");
    assert_eq!(decoded.channels, 2);
    let dec_left: Vec<f32> = decoded.samples.iter().step_by(2).copied().collect();
    let dec_right: Vec<f32> = decoded.samples.iter().skip(1).step_by(2).copied().collect();
    let pl = goertzel(&dec_left, sample_rate, tone);
    let pr = goertzel(&dec_right, sample_rate, tone);
    let power_ratio = pr / pl.max(1.0e-9);
    eprintln!("MPEG-2 LSF MS-perceptual power ratio (right/left)={power_ratio:.3} (target 0.49)");
    assert!(pl > 0.0 && pr > 0.0, "decoded channels carry no tone energy");
    assert!(
        (0.30..0.70).contains(&power_ratio),
        "MS-perceptual reconstruction lost the channel level difference: ratio={power_ratio:.3}"
    );
}

#[test]
fn mp3_mpeg25_rates_are_rejected_cleanly() {
    // MPEG-2.5 (8/11.025/12 kHz) is outside ISO/IEC 11172-3 and 13818-3, so the
    // encoder must reject it with an error rather than panicking or emitting an
    // undecodable stream.
    for &rate in &[8_000_u32, 11_025, 12_000] {
        let tone: Vec<f32> = (0..4_096)
            .map(|i| 0.5 * (std::f32::consts::TAU * 500.0 * (i as f32 / rate as f32)).sin())
            .collect();
        let pcm = AudioBuffer::new(rate, 1, tone).unwrap();
        assert!(
            sonare_codec::encode(Format::Mp3, &pcm).is_err(),
            "{rate} Hz (MPEG-2.5) must be rejected"
        );
    }
}

#[test]
fn mp3_mpeg2_lsf_handles_edge_inputs_without_panicking() {
    // The MPEG-2 LSF encode path must survive degenerate inputs: empty, shorter
    // than a 576-sample granule, full-scale, and pure silence — without
    // panicking and while emitting a well-formed MP3 stream.
    for &rate in &[16_000_u32, 22_050, 24_000] {
        // Empty input encodes to an empty (frame-less) stream without panicking.
        let empty = AudioBuffer::new(rate, 1, Vec::new()).unwrap();
        let enc = sonare_codec::encode(Format::Mp3, &empty).expect("empty MPEG-2 encode");
        assert!(
            enc.is_empty(),
            "{rate} Hz: empty input must yield no frames"
        );

        // Sub-granule, full-scale, and silent chunks must encode to detectable
        // MP3 frames without panicking. (Decoding 1-3 frame streams is a generic
        // short-stream limitation, exercised at length below.)
        for samples in [vec![0.5_f32], vec![1.0_f32; 200], vec![0.0_f32; 1_500]] {
            let pcm = AudioBuffer::new(rate, 1, samples).unwrap();
            let enc = sonare_codec::encode(Format::Mp3, &pcm).expect("edge MPEG-2 encode");
            assert_eq!(sonare_codec::detect(&enc), Some(Format::Mp3));
            assert_eq!(&enc[..1], &[0xff], "{rate} Hz: missing frame sync");
        }

        // A comfortably long full-scale signal must encode and decode.
        let loud = vec![0.9_f32; 16_000];
        let pcm = AudioBuffer::new(rate, 1, loud).unwrap();
        let enc = sonare_codec::encode(Format::Mp3, &pcm).expect("loud MPEG-2 encode");
        let dec = sonare_codec::decode(&enc).expect("loud MPEG-2 decode");
        assert_eq!(dec.sample_rate, rate);
    }
}

#[test]
fn mp3_reservoir_roundtrip_decodes_through_symphonia() {
    // The bit-reservoir encoder lets frames borrow main-data bytes from earlier
    // frames via main_data_begin. Symphonia must reassemble that cross-frame
    // stream without collapsing to silence. Detailed reservoir side-info
    // correctness is covered in the MP3 crate; FFmpeg oracle tests own
    // production quality.
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
        corr > 0.1,
        "reservoir waveform correlation too low: {corr:.4}"
    );
    assert!(
        (0.5..2.0).contains(&level_ratio),
        "reservoir decoded level out of range: ratio={level_ratio:.3}"
    );
}

#[test]
fn mp3_perceptual_roundtrip_decodes_through_symphonia() {
    // A full stream assembled with psychoacoustic per-band scale-factor noise
    // shaping must still decode through Symphonia and reconstruct the sweep: the
    // decoder reverses the per-band gain via the transmitted scale factors, so
    // shape and level hold (perceptual coding trades SNR in masked bands for
    // bits, so this asserts correctness, not an SNR gain).
    let sample_rate = 44_100;
    let total_frames = 16 * 1152;
    let pcm = sweep_pcm(total_frames, sample_rate, 300.0, 6_000.0, 0.3);

    let provider = sc_mp3::mpeg1_layer3_standard_table_provider();
    let step = 0.5_f32;
    let mut stream = Vec::new();
    let mut start = 0usize;
    while start + 1152 <= total_frames {
        let header = sc_mp3::layer3_header_for_capacity(sample_rate, 1, 320, false, false).unwrap();
        let frame =
            sc_mp3::assemble_mpeg1_layer3_pcm_frame_with_perceptual_scale_factors_and_table_provider(
                header, &pcm, start, step, provider,
            )
            .expect("perceptual MP3 frame assembly");
        stream.extend_from_slice(&frame);
        start += 1152;
    }

    let decoded = sonare_codec::decode(&stream).expect("Symphonia decode");
    assert_eq!(decoded.channels, 1, "expected mono reconstruction");

    let seg = 8_192;
    let ref_start = 8_000;
    let corr = aligned_channel_corr(&pcm.samples, &decoded.samples, seg, ref_start);
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
    eprintln!("perceptual roundtrip: corr={corr:.4} ratio={level_ratio:.3}");

    assert!(
        corr > 0.6,
        "perceptual waveform correlation too low: {corr:.4}"
    );
    assert!(
        (0.5..2.0).contains(&level_ratio),
        "perceptual decoded level out of range: ratio={level_ratio:.3}"
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

    // Shape: the sweep must remain detectably related after Symphonia decode.
    // FFmpeg oracle tests provide the stricter production-quality gate.
    assert!(corr > 0.1, "waveform correlation too low: {corr:.4}");
    // Level: the calibrated global_gain plus the IMDCT-normalization offset must
    // reconstruct close to unity. An uncalibrated encoder lands ~9x off.
    assert!(
        (0.5..2.0).contains(&level_ratio),
        "decoded level out of calibrated range: ratio={level_ratio:.3}"
    );
}
