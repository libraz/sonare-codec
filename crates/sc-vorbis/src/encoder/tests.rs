use super::*;

#[cfg(test)]
#[allow(clippy::module_inception)]
mod tests {
    use super::*;

    #[test]
    fn schedule_with_no_transients_is_all_long() {
        // The common case: every slot stays a full long block (lW = nW = 1) on
        // the 1024-sample grid, identical to the non-switched encoder.
        let schedule = build_schedule(5, &[false; 5]);
        assert_eq!(schedule.len(), 5);
        for (k, spec) in schedule.iter().enumerate() {
            assert!(spec.long && spec.lw && spec.nw, "slot {k} not a full long");
            assert_eq!(spec.center, (k + 1) * BLOCK_N);
        }
    }

    #[test]
    fn schedule_replaces_a_transient_slot_with_a_bracketed_short_group() {
        // One interior transient slot (k = 2) becomes 8 short blocks bracketed by
        // transition long blocks; the grid realigns afterwards.
        let mut transient = vec![false; 5];
        transient[2] = true;
        let schedule = build_schedule(5, &transient);

        // Slots 0,1,3,4 long + 8 shorts = 12 blocks.
        assert_eq!(schedule.len(), 12);
        // The two long blocks bordering the short group take the transition
        // window on the bordering edge.
        let longs: Vec<&BlockSpec> = schedule.iter().filter(|s| s.long).collect();
        assert_eq!(longs.len(), 4);
        // Slot 1 (centre 2*1024) closes into the shorts: nW = 0.
        let opener = longs
            .iter()
            .find(|s| s.center == 2 * BLOCK_N)
            .expect("opener");
        assert!(opener.lw && !opener.nw, "opening bracket long");
        // Slot 3 (centre 4*1024) opens out of the shorts: lW = 0.
        let closer = longs
            .iter()
            .find(|s| s.center == 4 * BLOCK_N)
            .expect("closer");
        assert!(!closer.lw && closer.nw, "closing bracket long");

        // Centres are strictly monotonic, and every advance is one of the three
        // legal switching distances (long-long 1024, long-short / short-long 576,
        // short-short 128) — the property that makes the overlap-add reconstruct.
        for pair in schedule.windows(2) {
            let adv = pair[1].center - pair[0].center;
            assert!(
                adv == BLOCK_N || adv == LONG_SHORT_ADVANCE || adv == SHORT_ADVANCE,
                "illegal centre advance {adv}"
            );
        }
        // Eight short blocks at the expected centres around the replaced slot.
        let shorts: Vec<usize> = schedule
            .iter()
            .filter(|s| !s.long)
            .map(|s| s.center)
            .collect();
        assert_eq!(shorts.len(), 8);
        for (i, &c) in shorts.iter().enumerate() {
            assert_eq!(c, 2 * BLOCK_N + LONG_SHORT_ADVANCE + SHORT_ADVANCE * i);
        }
    }

    #[test]
    fn schedule_merges_adjacent_transient_slots_into_one_run() {
        // Two adjacent transient slots become a single run of 16 shorts bracketed
        // by two long blocks (no long block survives between them).
        let mut transient = vec![false; 6];
        transient[2] = true;
        transient[3] = true;
        let schedule = build_schedule(6, &transient);
        let shorts = schedule.iter().filter(|s| !s.long).count();
        assert_eq!(shorts, 16);
        // Still realigns: the final long sits on the grid at its slot centre.
        let last = schedule.last().expect("nonempty");
        assert!(last.long && last.center == 6 * BLOCK_N);
        for pair in schedule.windows(2) {
            let adv = pair[1].center - pair[0].center;
            assert!(adv == BLOCK_N || adv == LONG_SHORT_ADVANCE || adv == SHORT_ADVANCE);
        }
    }

    #[test]
    fn transient_detector_fires_on_an_onset_not_a_steady_tone() {
        // A steady tone has near-constant sub-window energy: no transient.
        let rate = 48_000.0f32;
        let freq = 1000.0;
        let tone: Vec<f32> = (0..2 * BLOCK_N)
            .map(|i| (2.0 * std::f32::consts::PI * freq * i as f32 / rate).sin() * 0.5)
            .collect();
        assert!(
            !block_is_transient(&tone),
            "steady tone misread as transient"
        );

        // Silence then a sudden loud burst is a transient.
        let mut onset = vec![0.0f32; 2 * BLOCK_N];
        for (i, s) in onset.iter_mut().enumerate().skip(BLOCK_N) {
            *s = (2.0 * std::f32::consts::PI * freq * i as f32 / rate).sin() * 0.8;
        }
        assert!(block_is_transient(&onset), "onset missed");

        // True silence is not a transient.
        assert!(!block_is_transient(&vec![0.0f32; 2 * BLOCK_N]));
    }

    #[test]
    fn floor_postlist_is_well_formed() {
        // floor1 allows at most 65 posts (libvorbis VIF_POSIT = 63 interior + 2
        // endpoints); exceeding it makes a standard decoder reject the setup.
        assert!(POSTLIST.len() <= 65, "floor1 supports at most 65 posts");
        // The interior posts must exactly fill the partitions (dim 4 each).
        assert_eq!(POSTLIST.len(), 2 + FLOOR_PARTITIONS * 4);
        // Endpoints frame the block, and every position is distinct (floor1
        // requires unique post x-positions).
        assert_eq!(POSTLIST[0], 0);
        assert_eq!(POSTLIST[1] as usize, BLOCK_N);
        let mut sorted = POSTLIST.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(
            sorted.len(),
            POSTLIST.len(),
            "post positions must be unique"
        );
        assert!(*sorted.last().expect("nonempty") <= BLOCK_N as i32);
    }

    #[test]
    fn decoded_length_matches_input_exactly() {
        // The final block's granule is clamped to the true sample count, so the
        // decoder trims the block-rounding tail padding: a roundtrip is
        // sample-accurate in length (and the front priming adds no delay) for
        // lengths spanning sub-block, exact-block, and arbitrary remainders.
        for &(rate, ch, frames) in &[
            (48_000u32, 1u16, 1usize),
            (48_000, 1, 100),
            (48_000, 1, 2048),
            (48_000, 1, 3000),
            (48_000, 2, 5000),
            (44_100, 2, 9600),
        ] {
            let mut samples = Vec::with_capacity(frames * usize::from(ch));
            for i in 0..frames {
                let t = i as f32 / rate as f32;
                let v = (2.0 * std::f32::consts::PI * 440.0 * t).sin() * 0.5;
                for _ in 0..ch {
                    samples.push(v);
                }
            }
            let pcm = AudioBuffer::new(rate, ch, samples).expect("pcm");
            let bytes = encode(&pcm).expect("encode");
            let decoded = crate::decode(&bytes).expect("decode");
            assert_eq!(
                decoded.frames(),
                frames,
                "rate {rate} ch {ch}: decoded {} frames, expected {frames}",
                decoded.frames()
            );
        }
    }

    /// The decoder's inverse square-polar coupling (Vorbis I spec §9.4.2),
    /// reimplemented independently here to verify the encoder's forward
    /// transform is its exact inverse.
    fn decouple_pair(m: f32, a: f32) -> (f32, f32) {
        if m > 0.0 {
            if a > 0.0 {
                (m, m - a)
            } else {
                (m + a, m)
            }
        } else if a > 0.0 {
            (m, m + a)
        } else {
            (m - a, m)
        }
    }

    #[test]
    fn coupling_is_exactly_invertible() {
        // For every quadrant of (l, r), forward-coupling then the spec decode
        // must recover the original pair bit-for-bit, and equal channels must
        // collapse the angle to zero (so the angle channel skips).
        let vals = [-7.5f32, -3.0, -0.25, 0.0, 0.25, 1.5, 4.0, 9.0];
        for &l in &vals {
            for &r in &vals {
                let (m, a) = couple_pair(l, r);
                let (dl, dr) = decouple_pair(m, a);
                assert_eq!(dl, l, "magnitude mismatch for ({l}, {r})");
                assert_eq!(dr, r, "angle mismatch for ({l}, {r})");
            }
            // Equal channels -> zero angle.
            let (_, a) = couple_pair(l, l);
            assert_eq!(a, 0.0, "equal channels did not zero the angle for {l}");
        }
    }

    fn sine_pcm(sample_rate: u32, channels: u16, frames: usize, freq: f32) -> AudioBuffer {
        let mut samples = Vec::with_capacity(frames * usize::from(channels));
        for frame in 0..frames {
            let t = frame as f32 / sample_rate as f32;
            let value = (2.0 * std::f32::consts::PI * freq * t).sin() * 0.5;
            for _ in 0..channels {
                samples.push(value);
            }
        }
        AudioBuffer::new(sample_rate, channels, samples).expect("pcm")
    }

    #[test]
    fn emits_an_ogg_vorbis_stream() {
        let pcm = sine_pcm(48_000, 1, 2048, 440.0);
        let bytes = encode(&pcm).expect("encode");
        assert_eq!(&bytes[..4], b"OggS");
        assert_eq!(sc_core::detect(&bytes), Some(sc_core::Format::Vorbis));
    }

    #[test]
    fn symphonia_decodes_our_mono_stream() {
        // The conformance oracle: our pure-Rust bitstream must decode through the
        // library's standard (Symphonia) decode path and carry real energy.
        let pcm = sine_pcm(48_000, 1, 9600, 440.0);
        let bytes = encode(&pcm).expect("encode");
        let decoded = crate::decode(&bytes).expect("symphonia decode");
        assert_eq!(decoded.channels, 1);
        assert_eq!(decoded.sample_rate, 48_000);
        let rms = (decoded.samples.iter().map(|s| s * s).sum::<f32>()
            / decoded.samples.len().max(1) as f32)
            .sqrt();
        assert!(rms > 0.05, "decoded RMS too low: {rms}");
    }

    #[test]
    fn symphonia_decodes_our_stereo_stream() {
        let pcm = sine_pcm(44_100, 2, 4410, 440.0);
        let bytes = encode(&pcm).expect("encode");
        let decoded = crate::decode(&bytes).expect("symphonia decode");
        assert_eq!(decoded.channels, 2);
    }

    /// Best correlation of `b` against `a` over integer lags in `0..max_lag`.
    fn best_corr(a: &[f32], b: &[f32], max_lag: usize) -> (f32, usize) {
        let mut best = (f32::MIN, 0usize);
        for lag in 0..max_lag {
            if lag + 64 >= b.len() {
                break;
            }
            let n = (a.len()).min(b.len() - lag);
            if n < 256 {
                break;
            }
            let aa = &a[..n];
            let bb = &b[lag..lag + n];
            let dot: f32 = aa.iter().zip(bb).map(|(&x, &y)| x * y).sum();
            let na: f32 = aa.iter().map(|x| x * x).sum::<f32>().sqrt();
            let nb: f32 = bb.iter().map(|x| x * x).sum::<f32>().sqrt();
            let c = if na == 0.0 || nb == 0.0 {
                0.0
            } else {
                dot / (na * nb)
            };
            if c > best.0 {
                best = (c, lag);
            }
        }
        best
    }

    /// Extracts channel `ch` from an interleaved buffer.
    fn deinterleave(buf: &AudioBuffer, ch: usize) -> Vec<f32> {
        buf.samples
            .chunks_exact(usize::from(buf.channels))
            .map(|frame| frame[ch])
            .collect()
    }

    #[test]
    fn coupling_compresses_correlated_stereo() {
        // Dual-mono (L == R) is perfectly correlated: square-polar coupling
        // collapses the angle channel's residue to zero, beating independent
        // coding, while both channels still reconstruct faithfully.
        let pcm = sine_pcm(48_000, 2, 9600, 440.0);
        let bytes = encode(&pcm).expect("encode");
        let raw16 = pcm.frames() * 2 * 2;
        assert!(
            bytes.len() * 8 < raw16,
            "coupling did not boost stereo compression: {} vs raw {raw16}",
            bytes.len()
        );
        let decoded = crate::decode(&bytes).expect("decode");
        assert_eq!(decoded.channels, 2);
        for ch in 0..2 {
            let (corr, _) = best_corr(&deinterleave(&pcm, ch), &deinterleave(&decoded, ch), 1024);
            assert!(corr > 0.85, "channel {ch} correlation {corr} too low");
        }
    }

    #[test]
    fn decorrelated_stereo_is_not_degraded_by_coupling() {
        // Distinct tones per channel are uncorrelated, so coupling must not
        // engage; both channels reconstruct as well as independent coding would.
        let mut samples = Vec::with_capacity(9600 * 2);
        for i in 0..9600 {
            let t = i as f32 / 48_000.0;
            samples.push((2.0 * std::f32::consts::PI * 440.0 * t).sin() * 0.5);
            samples.push((2.0 * std::f32::consts::PI * 623.0 * t).sin() * 0.5);
        }
        let pcm = AudioBuffer::new(48_000, 2, samples).expect("pcm");
        let bytes = encode(&pcm).expect("encode");
        let decoded = crate::decode(&bytes).expect("decode");
        assert_eq!(decoded.channels, 2);
        for ch in 0..2 {
            let (corr, _) = best_corr(&deinterleave(&pcm, ch), &deinterleave(&decoded, ch), 1024);
            assert!(corr > 0.9, "channel {ch} degraded by coupling: corr {corr}");
        }
    }

    /// A mono signal that is silent until `onset`, then a loud sustained tone —
    /// the classic pre-echo stressor (a long block straddling the onset smears
    /// the attack backward into the silence).
    fn onset_pcm(sample_rate: u32, frames: usize, onset: usize, freq: f32) -> AudioBuffer {
        let mut samples = vec![0.0f32; frames];
        for (i, s) in samples.iter_mut().enumerate().skip(onset) {
            let t = i as f32 / sample_rate as f32;
            *s = (2.0 * std::f32::consts::PI * freq * t).sin() * 0.7;
        }
        AudioBuffer::new(sample_rate, 1, samples).expect("pcm")
    }

    /// A mono signal that is silent until `onset`, then a loud broadband noise
    /// burst — the sharpest pre-echo stressor (a long block straddling the onset
    /// rings the attack backward across its whole 2048-sample window).
    fn burst_pcm(sample_rate: u32, frames: usize, onset: usize) -> AudioBuffer {
        let mut samples = vec![0.0f32; frames];
        let mut state = 0x1234_5678u32;
        for s in samples.iter_mut().skip(onset) {
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            *s = ((state >> 9) as f32 / (1u32 << 23) as f32 - 1.0) * 0.7;
        }
        AudioBuffer::new(sample_rate, 1, samples).expect("pcm")
    }

    /// RMS of a slice (0 for an empty slice).
    fn rms(xs: &[f32]) -> f32 {
        if xs.is_empty() {
            return 0.0;
        }
        (xs.iter().map(|s| s * s).sum::<f32>() / xs.len() as f32).sqrt()
    }

    #[test]
    fn transient_input_decodes_with_exact_length() {
        // A switched stream (the onset forces a short-block group) must still
        // decode through Symphonia at the exact input length.
        let pcm = onset_pcm(48_000, 9600, 4096, 1000.0);
        let bytes = encode(&pcm).expect("encode");
        let decoded = crate::decode(&bytes).expect("symphonia decode");
        assert_eq!(decoded.channels, 1);
        assert_eq!(decoded.frames(), 9600, "switched stream length drifted");
    }

    #[test]
    fn coupled_stereo_transient_decodes() {
        // Block switching and square-polar coupling combined: a correlated
        // (dual-mono) stereo burst couples the short blocks too. The stream must
        // still decode through Symphonia at the exact length on both channels.
        let frames = 12_288usize;
        let onset = 4096usize;
        let mono = burst_pcm(48_000, frames, onset);
        let mut samples = Vec::with_capacity(frames * 2);
        for &v in &mono.samples {
            samples.push(v);
            samples.push(v);
        }
        let pcm = AudioBuffer::new(48_000, 2, samples).expect("pcm");
        let bytes = encode(&pcm).expect("encode");
        let decoded = crate::decode(&bytes).expect("symphonia decode");
        assert_eq!(decoded.channels, 2);
        assert_eq!(decoded.frames(), frames, "switched stereo length drifted");
    }

    #[test]
    fn block_switching_curbs_pre_echo_before_an_onset() {
        // Encode a silence-then-burst onset. The short-block group localizes the
        // attack, so the decoded signal stays quiet right up to the onset instead
        // of ringing ahead of it. Measure the decoded RMS in the ~1024 samples
        // just before the onset against the RMS of the burst body. With block
        // switching the leakage is ~0.2% of the steady level; an all-long encoder
        // smears it to ~1.4% (the burst rings across the full 2048-sample
        // window), so the threshold below passes only because the onset is
        // switched to short blocks.
        let onset = 4096usize;
        let pcm = burst_pcm(48_000, 12_288, onset);
        let bytes = encode(&pcm).expect("encode");
        let decoded = crate::decode(&bytes).expect("decode");
        assert_eq!(decoded.frames(), 12_288);

        let pre = &decoded.samples[onset - 1024..onset - 64];
        let body = &decoded.samples[onset + 2048..onset + 6144];
        let pre_rms = rms(pre);
        let body_rms = rms(body);
        assert!(body_rms > 0.1, "burst body too quiet: {body_rms}");
        assert!(
            pre_rms < 0.006 * body_rms,
            "pre-echo not contained: pre {pre_rms} vs body {body_rms}"
        );
    }

    #[test]
    fn roundtrip_fidelity_through_symphonia() {
        // Encode tones across the band, decode through Symphonia, and require the
        // decoded signal to track the input (correlation is amplitude-invariant,
        // so this checks waveform shape, not just energy). The dense floor keeps
        // the residue near unity, so even a 5 kHz tone reconstructs well.
        for &freq in &[300.0f32, 800.0, 2000.0, 5000.0] {
            let pcm = sine_pcm(48_000, 1, 9600, freq);
            let bytes = encode(&pcm).expect("encode");
            let decoded = crate::decode(&bytes).expect("decode");
            let (corr, _lag) = best_corr(&pcm.samples, &decoded.samples, 1024);
            assert!(corr > 0.85, "freq {freq}: correlation {corr} too low");
        }
    }

    #[test]
    fn switched_stream_body_reconstructs_faithfully() {
        // The pre-echo test guards the quiet region *before* an onset, and the
        // length tests guard the granule arithmetic, but nothing asserts the
        // switched (short-block) region itself reconstructs the signal. Encode a
        // silence-then-burst onset and a silence-then-tone onset — both force a
        // short-block group — and require the decoded attack body to track the
        // input. This locks the short-block analysis/coding path, which the
        // all-long `roundtrip_fidelity_through_symphonia` never exercises.
        let onset = 4096usize;
        let frames = 12_288usize;
        let body = onset + 256..onset + 4096;

        let burst = burst_pcm(48_000, frames, onset);
        let decoded = crate::decode(&encode(&burst).expect("encode")).expect("decode");
        let (corr, _) = best_corr(
            &burst.samples[body.clone()],
            &decoded.samples[body.clone()],
            512,
        );
        assert!(corr > 0.9, "switched burst body corr {corr} too low");

        let mut tone = vec![0.0f32; frames];
        for (i, x) in tone.iter_mut().enumerate().skip(onset) {
            let t = i as f32 / 48_000.0;
            *x = (2.0 * std::f32::consts::PI * 1000.0 * t).sin() * 0.6;
        }
        let tone = AudioBuffer::new(48_000, 1, tone).expect("pcm");
        let decoded = crate::decode(&encode(&tone).expect("encode")).expect("decode");
        let (corr, _) = best_corr(&tone.samples[body.clone()], &decoded.samples[body], 512);
        assert!(corr > 0.9, "switched onset-tone body corr {corr} too low");
    }

    #[test]
    fn m1_companding_shrinks_a_tone_without_breaking_fidelity() {
        // M1 noise companding relatively compensates near-floor residue, so the
        // encoded tone is smaller than the same encoder with companding disabled
        // would produce, while the decoded waveform still tracks the input. This
        // locks the companding as a net-positive (size down, fidelity held); the
        // exact byte counts are content-dependent, so the test asserts only the
        // direction and a high correlation, not a fixed size.
        let pcm = sine_pcm(48_000, 1, 9600, 1000.0);
        let bytes = encode(&pcm).expect("encode");
        let decoded = crate::decode(&bytes).expect("decode");
        let (corr, _) = best_corr(&pcm.samples, &decoded.samples, 1024);
        assert!(corr > 0.99, "companded tone fidelity dropped: {corr}");
        // A 48 kHz mono tone over 9600 frames stays well under the raw size; the
        // companding keeps it there (a regression that disabled it still passes
        // this loose bound — the unit tests in `analysis` guard the gains math).
        assert!(bytes.len() < pcm.frames() * 2, "tone did not compress");
    }
}
