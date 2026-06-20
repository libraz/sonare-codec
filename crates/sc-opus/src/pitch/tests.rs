use super::*;

#[cfg(test)]
#[allow(clippy::module_inception)]
mod tests {
    use super::*;

    /// A buffer of `len` samples preceded by `COMBFILTER_MAXPERIOD` history, both
    /// produced by `f`. Returns `(buf, head)` with `head == COMBFILTER_MAXPERIOD`.
    fn buf_with_history(len: usize, f: impl Fn(usize) -> f32) -> (Vec<f32>, usize) {
        let head = COMBFILTER_MAXPERIOD;
        let mut buf = vec![0.0f32; head + len];
        for (i, v) in buf.iter_mut().enumerate() {
            // Index relative to the first output sample (can be negative history).
            *v = f(i.wrapping_sub(head));
        }
        (buf, head)
    }

    #[test]
    fn zero_gain_is_identity() {
        let (x, head) = buf_with_history(64, |i| (i as f32 * 0.3).sin());
        let win = vec![0.0f32; 0];
        let mut y = vec![999.0f32; 64];
        comb_filter(&mut y, &x, head, 40, 40, 64, 0.0, 0.0, 0, 0, &win, 0);
        assert_eq!(&y[..], &x[head..head + 64]);
    }

    #[test]
    fn dc_steady_state_matches_closed_form() {
        // Equal old/new filter params skip the overlap, so the whole frame runs
        // the constant filter. For a DC input every delayed tap equals 1, giving
        // y = 1 + g*(k0 + 2*k1 + 2*k2) exactly.
        let n = 50;
        let (x, head) = buf_with_history(n, |_| 1.0);
        let win = vec![0.0f32; 0];
        for (tapset, k) in COMB_GAINS.iter().enumerate() {
            let g = 0.5f32;
            let mut y = vec![0.0f32; n];
            comb_filter(&mut y, &x, head, 30, 30, n, g, g, tapset, tapset, &win, 0);
            let expected = 1.0 + g * (k[0] + 2.0 * k[1] + 2.0 * k[2]);
            for &yi in &y {
                assert!(
                    (yi - expected).abs() < 1e-6,
                    "tapset {tapset}: {yi} vs {expected}"
                );
            }
        }
    }

    #[test]
    fn reinforces_a_periodic_component() {
        // A tone whose period equals the comb period should be amplified: each
        // delayed tap lands in phase, so output energy exceeds input energy.
        let period = 32usize;
        let n = 256;
        let (x, head) = buf_with_history(n, |i| {
            (i as f32 / period as f32 * std::f32::consts::TAU).sin()
        });
        let win = vec![0.0f32; 0];
        let mut y = vec![0.0f32; n];
        comb_filter(&mut y, &x, head, period, period, n, 0.8, 0.8, 0, 0, &win, 0);
        let e_in: f32 = x[head..head + n].iter().map(|v| v * v).sum();
        let e_out: f32 = y.iter().map(|v| v * v).sum();
        assert!(
            e_out > 1.3 * e_in,
            "periodic energy not reinforced: {e_out} vs {e_in}"
        );
    }

    #[test]
    fn overlap_cross_fade_is_continuous_and_deterministic() {
        // With differing old/new gains the first `overlap` samples blend the two
        // filters; the result must be reproducible and reduce to the constant
        // filter once past the overlap.
        let n = 120;
        let overlap = 24;
        let (x, head) = buf_with_history(n, |i| {
            (i as f32 * 0.17).cos() + 0.5 * (i as f32 * 0.4).sin()
        });
        // A monotone-ish power-complementary window stand-in: sin ramp.
        let win: Vec<f32> = (0..overlap)
            .map(|i| ((i as f32 + 0.5) / overlap as f32 * std::f32::consts::FRAC_PI_2).sin())
            .collect();

        let mut y1 = vec![0.0f32; n];
        comb_filter(&mut y1, &x, head, 40, 48, n, 0.2, 0.7, 0, 1, &win, overlap);
        let mut y2 = vec![0.0f32; n];
        comb_filter(&mut y2, &x, head, 40, 48, n, 0.2, 0.7, 0, 1, &win, overlap);
        assert_eq!(y1, y2, "comb_filter is not deterministic");

        // Past the overlap the output equals the pure new-filter result.
        let gb = COMB_GAINS[1];
        let (g10, g11, g12) = (0.7 * gb[0], 0.7 * gb[1], 0.7 * gb[2]);
        let mut steady = vec![0.0f32; n - overlap];
        comb_filter_const(&mut steady, &x, head + overlap, 48, g10, g11, g12);
        assert_eq!(
            &y1[overlap..],
            &steady[..],
            "body diverges from constant filter"
        );
    }

    #[test]
    fn pitch_xcorr_matches_direct_dot_products() {
        let len = 40;
        let max_pitch = 24;
        let x: Vec<f32> = (0..len).map(|i| (i as f32 * 0.31).sin()).collect();
        let y: Vec<f32> = (0..len + max_pitch)
            .map(|i| (i as f32 * 0.17).cos() - 0.3 * i as f32 * 0.01)
            .collect();
        let mut xcorr = vec![0.0f32; max_pitch];
        celt_pitch_xcorr(&x, &y, &mut xcorr, len, max_pitch);
        for i in 0..max_pitch {
            let want: f32 = (0..len).map(|j| x[j] * y[i + j]).sum();
            assert!(
                (xcorr[i] - want).abs() < 1e-3,
                "lag {i}: {} vs {want}",
                xcorr[i]
            );
        }
    }

    #[test]
    fn find_best_pitch_picks_the_normalised_peak() {
        let len = 20;
        let max_pitch = 16;
        // Flat energy in y so the decision is driven purely by the correlation.
        let y = vec![1.0f32; len + max_pitch];
        let mut xcorr = vec![0.0f32; max_pitch];
        xcorr[5] = 100.0;
        xcorr[11] = 40.0;
        let mut best = [0usize; 2];
        find_best_pitch(&xcorr, &y, len, max_pitch, &mut best);
        assert_eq!(best[0], 5, "strongest lag");
        assert_eq!(best[1], 11, "second strongest lag");
    }

    #[test]
    fn pitch_search_recovers_a_known_lag() {
        // Full-rate counts; the signals are at half rate. A half-rate sinusoid
        // whose period exceeds the search range has a single in-range match, at
        // the offset where the current frame `x_lp` was lifted from `y`. The
        // returned lag is full-rate, i.e. twice the half-rate offset.
        let len = 256usize;
        let max_pitch = 200usize;
        let half_period = 90.0f32;
        let y_len = (len + max_pitch) >> 1;
        let y: Vec<f32> = (0..y_len + 4)
            .map(|i| (i as f32 / half_period * std::f32::consts::TAU).sin())
            .collect();
        let half_lag = 15usize; // half-rate offset the frame is taken from
        let frame = len >> 1;
        let x_lp: Vec<f32> = (0..frame).map(|k| y[half_lag + k]).collect();

        let pitch = pitch_search(&x_lp, &y, len, max_pitch);
        let expected_full = 2 * half_lag;
        assert!(
            (pitch as i32 - expected_full as i32).abs() <= 2,
            "recovered full-rate lag {pitch} not within 2 of {expected_full}"
        );
    }

    #[test]
    fn autocorr_matches_definition() {
        let n = 50;
        let lag = 4;
        let x: Vec<f32> = (0..n).map(|i| (i as f32 * 0.23).sin() + 0.1).collect();
        let mut ac = [0.0f32; 5];
        celt_autocorr(&x, &mut ac, n, lag);
        for k in 0..=lag {
            let want: f32 = (k..n).map(|i| x[i] * x[i - k]).sum();
            assert!((ac[k] - want).abs() < 1e-3, "lag {k}: {} vs {want}", ac[k]);
        }
        // Lag 0 is the energy and dominates.
        assert!(ac[0] >= ac[1].abs());
    }

    #[test]
    fn lpc_recovers_a_first_order_predictor() {
        // The autocorrelation of an AR(1) process is r[k] = rho^k. Levinson-Durbin
        // must then recover the whitening filter A(z) = 1 - rho z^-1, i.e.
        // lpc[0] = -rho and the higher orders ~0.
        let rho = 0.8f32;
        let ac: Vec<f32> = (0..=4).map(|k| rho.powi(k)).collect();
        let mut lpc = [0.0f32; 4];
        celt_lpc(&mut lpc, &ac, 4);
        assert!(
            (lpc[0] + rho).abs() < 1e-4,
            "lpc[0] = {} (want {})",
            lpc[0],
            -rho
        );
        for (i, &c) in lpc.iter().enumerate().skip(1) {
            assert!(c.abs() < 1e-4, "lpc[{i}] = {c} not ~0");
        }
    }

    #[test]
    fn fir5_impulse_response_is_the_taps() {
        // Feeding a unit impulse through the 5-tap FIR yields [1, num0..num4].
        let num = [0.5f32, -0.25, 0.125, 0.1, -0.05];
        let mut x = vec![0.0f32; 8];
        x[0] = 1.0;
        celt_fir5(&mut x, &num, 8);
        let want = [1.0, num[0], num[1], num[2], num[3], num[4], 0.0, 0.0];
        for (i, (&g, &w)) in x.iter().zip(&want).enumerate() {
            assert!((g - w).abs() < 1e-6, "tap {i}: {g} vs {w}");
        }
    }

    #[test]
    fn pitch_downsample_halves_length_and_whitens() {
        // A smooth, strongly low-pass signal has high lag-1 correlation; after
        // decimation + LPC whitening the normalised lag-1 correlation must drop.
        let len = 512;
        let raw: Vec<f32> = (0..len)
            .map(|i| (i as f32 * 0.03).sin() + 0.5 * (i as f32 * 0.012).sin())
            .collect();
        let half = len >> 1;

        // Plain decimation (the pre-whitening reference): lag-1 correlation.
        let dec: Vec<f32> = (0..half).map(|i| raw[2 * i]).collect();
        let norm_lag1 = |s: &[f32]| {
            let e: f32 = s.iter().map(|v| v * v).sum();
            let c: f32 = (1..s.len()).map(|i| s[i] * s[i - 1]).sum();
            c / e.max(1e-9)
        };
        let before = norm_lag1(&dec);

        let mut x_lp = vec![0.0f32; half];
        let chans: [&[f32]; 1] = [&raw];
        pitch_downsample(&chans, &mut x_lp, len, 1);
        let after = norm_lag1(&x_lp);

        assert!(
            after.abs() < before.abs(),
            "whitening did not reduce lag-1 correlation: {after} vs {before}"
        );
    }

    #[test]
    fn remove_doubling_corrects_an_octave_error() {
        // A pure tone at full-rate period 30 (half-rate 15) correlates equally at
        // every multiple of its period. Seeded with the doubled lag (60), the
        // search must pull the estimate back to the fundamental (~30).
        let maxperiod = 256usize;
        let minperiod = COMBFILTER_MINPERIOD;
        let n = 256usize;
        let half_period = 15.0f32; // half-rate samples per cycle
        let head = maxperiod / 2; // 128
        let buf_len = head + n / 2; // 256
                                    // x[head + j] is analysis sample j; negative-index history is the tone too.
        let x: Vec<f32> = (0..buf_len)
            .map(|i| {
                let t = i as f32 - head as f32;
                (t / half_period * std::f32::consts::TAU).sin()
            })
            .collect();

        let mut t0 = 60i32; // doubled (octave-too-low) estimate
        let pg = remove_doubling(&x, maxperiod, minperiod, n, &mut t0, 0, 0.0);
        assert!(
            (t0 - 30).abs() <= 2,
            "octave not corrected: full-rate lag {t0} (want ~30)"
        );
        assert!(pg > 0.5, "pitch gain too low for a pure tone: {pg}");
    }

    /// Build a `cc`-plane `in` buffer (`cc * (n + overlap)`) whose frame region
    /// is a harmonically rich tone of full-rate period `period` at PCM scale (a
    /// pure sine would be annihilated by the LPC whitening; CELT runs on
    /// broadband, pulse-like signals). The overlap prefix is left at zero.
    fn periodic_in(n: usize, overlap: usize, cc: usize, period: f32) -> Vec<f32> {
        let stride = n + overlap;
        let mut buf = vec![0.0f32; cc * stride];
        for c in 0..cc {
            for i in 0..n {
                let phase = i as f32 / period * std::f32::consts::TAU;
                // A sawtooth-like sum of harmonics keeps a strong pitch pulse in
                // the LPC residual.
                let s: f32 = (1..=8).map(|h| (phase * h as f32).sin() / h as f32).sum();
                buf[c * stride + overlap + i] = 3000.0 * s;
            }
        }
        buf
    }

    #[test]
    fn run_prefilter_detects_a_periodic_frame() {
        // A strongly periodic frame should enable the post-filter with a pitch
        // near the true period and a valid quantised gain.
        let n = 480usize;
        let overlap = 120usize;
        let short = 120usize;
        let period = 64.0f32;
        let window: Vec<f32> = (0..overlap)
            .map(|i| ((i as f32 + 0.5) / overlap as f32 * std::f32::consts::FRAC_PI_2).sin())
            .collect();
        let mut in_buf = periodic_in(n, overlap, 1, period);
        let mut st = PrefilterState::new(1, overlap);
        let pf = run_prefilter(
            &mut in_buf,
            n,
            1,
            overlap,
            short,
            &window,
            0,
            true,
            100,
            &mut st,
        );

        assert!(pf.pf_on, "post-filter should engage on a periodic frame");
        // The detected period should be a (sub-)multiple of the true period.
        let r = (pf.pitch_index as f32 / period).round();
        assert!(
            r >= 1.0 && (pf.pitch_index as f32 - r * period).abs() < 8.0,
            "pitch {} not near a multiple of {period}",
            pf.pitch_index
        );
        assert!((0..=7).contains(&pf.qg), "qg out of range: {}", pf.qg);
        assert!(
            (pf.gain - 0.09375 * (pf.qg + 1) as f32).abs() < 1e-6,
            "gain not the dequantised qg"
        );
        // State carried for the next frame.
        assert_eq!(st.prefilter_period, pf.pitch_index);
        assert_eq!(st.prefilter_gain, pf.gain);
    }

    #[test]
    fn run_prefilter_disables_on_silence_and_is_a_passthrough() {
        // A silent frame has no pitch: the filter stays off and (gain 0) the
        // frame passes through unchanged.
        let n = 480usize;
        let overlap = 120usize;
        let short = 120usize;
        let window = vec![0.0f32; overlap];
        let mut in_buf = vec![0.0f32; n + overlap];
        // A deterministic non-zero frame so "unchanged" is meaningful.
        for i in 0..n {
            in_buf[overlap + i] = ((i * 7 % 13) as f32 - 6.0) * 0.01;
        }
        let original = in_buf.clone();
        let mut st = PrefilterState::new(1, overlap);
        let pf = run_prefilter(
            &mut in_buf,
            n,
            1,
            overlap,
            short,
            &window,
            0,
            true,
            100,
            &mut st,
        );
        assert!(
            !pf.pf_on,
            "post-filter should be off for an aperiodic frame"
        );
        assert_eq!(pf.gain, 0.0);
        // gain0 (prev) and gain1 are both 0 on the first frame -> copy.
        assert_eq!(
            &in_buf[overlap..overlap + n],
            &original[overlap..overlap + n],
            "zero-gain prefilter must be a passthrough"
        );
    }

    #[test]
    fn postfilter_params_round_trip_bit_exact() {
        // Encode a populated post-filter header and decode it back unchanged.
        let total_bits = 1000i32;
        for (pitch_index, qg, tapset) in [(80, 5, 0usize), (15, 0, 2), (1000, 7, 1)] {
            let pf = PostfilterParams {
                pf_on: true,
                pitch_index,
                gain: 0.09375 * (qg + 1) as f32,
                qg,
                tapset,
            };
            let mut enc = RangeEncoder::new(64);
            enc.enc_bit_logp(false, 15); // a leading silence flag, as in a real frame
            encode_postfilter(&mut enc, &pf, 0, total_bits);
            let bytes = enc.done();

            let mut dec = RangeDecoder::new(&bytes);
            assert!(!dec.dec_bit_logp(15));
            let got = decode_postfilter(&mut dec, 0, total_bits).expect("post-filter on");
            assert_eq!(got.pitch_index, pitch_index, "pitch mismatch");
            assert_eq!(got.qg, qg, "qg mismatch");
            assert_eq!(got.tapset, tapset, "tapset mismatch");
            assert!((got.gain - pf.gain).abs() < 1e-9, "gain mismatch");
        }
    }

    #[test]
    fn postfilter_off_round_trips() {
        let total_bits = 1000i32;
        let pf = PostfilterParams {
            pf_on: false,
            pitch_index: COMBFILTER_MINPERIOD as i32,
            gain: 0.0,
            qg: 0,
            tapset: 0,
        };
        let mut enc = RangeEncoder::new(64);
        enc.enc_bit_logp(false, 15);
        encode_postfilter(&mut enc, &pf, 0, total_bits);
        let bytes = enc.done();

        let mut dec = RangeDecoder::new(&bytes);
        assert!(!dec.dec_bit_logp(15));
        assert!(
            decode_postfilter(&mut dec, 0, total_bits).is_none(),
            "off header must decode to None"
        );
    }
}
