//! CELT MDCT: overlap window and forward transform.
//!
//! Hand-ported to safe Rust from libopus `celt/modes.c` (the overlap window
//! generation) and `celt/mdct.c` (the forward transform). Derivative work of
//! libopus (BSD-3-Clause); see `LICENSE-THIRDPARTY`.
//!
//! The window is the power-complementary low-overlap window
//! `w[i] = sin(pi/2 * sin^2(pi/2 * (i+0.5)/overlap))`, computed in double
//! precision (as the C does) and stored as `f32`. The defining TDAC property
//! `w[i]^2 + w[overlap-1-i]^2 == 1` is what the tests pin.
//!
//! The forward MDCT (the encoder's transform) is ported here as the fold and
//! pre/post rotation around an N/4-point DFT from `celt/mdct.c`. The inverse
//! MDCT is the decoder's job (Symphonia provides it), so only the forward is
//! needed first-party. The N/4 step is a direct DFT for now — mathematically
//! identical to the mixed-radix `kiss_fft` it replaces, correctness first; a
//! split-radix FFT is a later performance pass.

// Consumed by the CELT MDCT; the live encoder still ships via the Opus FFI path.
#![allow(dead_code)]

use core::f64::consts::PI;

/// `compute_window`: the CELT overlap window of length `overlap`.
#[must_use]
pub fn compute_window(overlap: usize) -> Vec<f32> {
    let half_pi = core::f64::consts::FRAC_PI_2;
    (0..overlap)
        .map(|i| {
            let s = (half_pi * (i as f64 + 0.5) / overlap as f64).sin();
            (half_pi * s * s).sin() as f32
        })
        .collect()
}

/// The CELT MDCT pre/post-rotation twiddle table: `trig[i] = cos(2*pi*(i+1/8)/n)`
/// for `i in 0..n/2`. The `1/8` phase offset is the defining CELT convention
/// (`celt/mdct.c` `clt_mdct_init`); `trig[n/4+i] = -sin(2*pi*(i+1/8)/n)`, so one
/// table supplies both the cosine and the (negated) sine used by the rotations.
fn mdct_trig(n: usize) -> Vec<f32> {
    let n2 = n >> 1;
    (0..n2)
        .map(|i| (2.0 * PI * (i as f64 + 0.125) / n as f64).cos() as f32)
        .collect()
}

/// `clt_mdct_forward`: the CELT forward MDCT of one block.
///
/// Hand-ported to safe Rust from the float build of libopus `celt/mdct.c`
/// (`clt_mdct_forward_c`): a windowed length-`n` -> `n/2` MDCT computed as a
/// fold and a pre/post rotation around an `n/4`-point DFT. Derivative work of
/// libopus (BSD-3-Clause); see `LICENSE-THIRDPARTY`.
///
/// `input` holds the `n + overlap` time samples this block reads (the block plus
/// the look-ahead the window mixes in); `window` is [`compute_window`] of length
/// `overlap`; `output` receives the `n/2` MDCT coefficients carrying the CELT
/// `1/(n/4)` analysis scaling. `stride` interleaves several short blocks into one
/// spectrum (pass `1` for a single block).
pub fn clt_mdct_forward(
    input: &[f32],
    output: &mut [f32],
    window: &[f32],
    n: usize,
    overlap: usize,
    stride: usize,
) {
    let n2 = n >> 1;
    let n4 = n >> 2;
    let scale = 1.0f32 / n4 as f32;
    let trig = mdct_trig(n);

    // --- Window, shuffle, fold: input -> f (n/4 complex, interleaved re,im). ---
    // The pointer walk mirrors the C exactly: xp1/xp2 scan the input inward from
    // both halves, wp1/wp2 scan the window, the three loops handle the leading
    // windowed taper, the unwindowed core, and the trailing windowed taper.
    let mut f = vec![0.0f32; n2];
    {
        let n2i = n2 as isize;
        let mut xp1 = (overlap >> 1) as isize;
        let mut xp2 = (n2 - 1 + (overlap >> 1)) as isize;
        let mut wp1 = (overlap >> 1) as isize;
        let mut wp2 = ((overlap >> 1) as isize) - 1;
        let inp = |k: isize| input[k as usize];
        let win = |k: isize| window[k as usize];
        let l1 = (overlap + 3) >> 2;
        let mut yp = 0usize;
        let mut i = 0usize;
        while i < l1 {
            f[yp] = win(wp2) * inp(xp1 + n2i) + win(wp1) * inp(xp2);
            f[yp + 1] = win(wp1) * inp(xp1) - win(wp2) * inp(xp2 - n2i);
            yp += 2;
            xp1 += 2;
            xp2 -= 2;
            wp1 += 2;
            wp2 -= 2;
            i += 1;
        }
        wp1 = 0;
        wp2 = (overlap as isize) - 1;
        while i < n4 - l1 {
            f[yp] = inp(xp2);
            f[yp + 1] = inp(xp1);
            yp += 2;
            xp1 += 2;
            xp2 -= 2;
            i += 1;
        }
        while i < n4 {
            f[yp] = -win(wp1) * inp(xp1 - n2i) + win(wp2) * inp(xp2);
            f[yp + 1] = win(wp2) * inp(xp1) + win(wp1) * inp(xp2 + n2i);
            yp += 2;
            xp1 += 2;
            xp2 -= 2;
            wp1 += 2;
            wp2 -= 2;
            i += 1;
        }
    }

    // --- Pre-rotation + analysis scaling -> g (n/4 complex), natural order. ---
    // g_m = scale * (f[2m] + j f[2m+1]) * (cos phi_m - j sin phi_m).
    let mut gr = vec![0.0f32; n4];
    let mut gi = vec![0.0f32; n4];
    for i in 0..n4 {
        let re = f[2 * i];
        let im = f[2 * i + 1];
        let t0 = trig[i];
        let t1 = trig[n4 + i];
        gr[i] = (re * t0 - im * t1) * scale;
        gi[i] = (im * t0 + re * t1) * scale;
    }

    // --- N/4-point forward DFT: F[k] = sum_m g[m] e^{-2*pi*j*k*m/(n/4)}. ---
    // Natural-order in/out, equivalent to the bit-reversed kiss_fft it replaces.
    let mut fr = vec![0.0f32; n4];
    let mut fi = vec![0.0f32; n4];
    for k in 0..n4 {
        let mut accr = 0.0f64;
        let mut acci = 0.0f64;
        for m in 0..n4 {
            let ang = -2.0 * PI * ((k * m) % n4) as f64 / n4 as f64;
            let (s, c) = ang.sin_cos();
            accr += gr[m] as f64 * c - gi[m] as f64 * s;
            acci += gr[m] as f64 * s + gi[m] as f64 * c;
        }
        fr[k] = accr as f32;
        fi[k] = acci as f32;
    }

    // --- Post-rotation -> output (n/2 reals), folded from both ends. ---
    for i in 0..n4 {
        let t0 = trig[i];
        let t1 = trig[n4 + i];
        output[stride * (2 * i)] = fi[i] * t1 - fr[i] * t0;
        output[stride * (n2 - 1 - 2 * i)] = fr[i] * t1 + fi[i] * t0;
    }
}

/// The fixed mode parameters [`compute_mdcts`] needs from the CELT mode.
pub struct MdctConfig<'a> {
    /// Overlap (and window) length, e.g. 120 for the 48 kHz mode.
    pub overlap: usize,
    /// The shortest MDCT's frame size (e.g. 120 for the 48 kHz mode).
    pub short_mdct_size: usize,
    /// The overlap window, [`compute_window`] of length `overlap`.
    pub window: &'a [f32],
}

/// `compute_mdcts`: transform one CELT frame's time-domain input into its MDCT
/// spectrum, looping [`clt_mdct_forward`] over the short blocks.
///
/// Hand-ported to safe Rust from libopus `celt/celt_encoder.c` (`compute_mdcts`).
/// Derivative work of libopus (BSD-3-Clause); see `LICENSE-THIRDPARTY`.
///
/// `short_blocks` is `0` for a single long block, otherwise the number of short
/// blocks `B`. `input` holds `cc_channels` regions of `B*N + overlap` samples
/// (`N` the per-block frame size); `out` holds `cc_channels` regions of `B*N`
/// coefficients with the `B` blocks interleaved (block `b` at stride `B`). When
/// `cc_channels == 2` but `c_channels == 1`, both spectra are still transformed
/// and then averaged into the first region. `upsample` (`> 1`) scales the live
/// band and clears the synthetic top.
#[allow(clippy::too_many_arguments)]
pub fn compute_mdcts(
    cfg: &MdctConfig,
    short_blocks: usize,
    lm: usize,
    input: &[f32],
    out: &mut [f32],
    c_channels: usize,
    cc_channels: usize,
    upsample: usize,
) {
    let overlap = cfg.overlap;
    let (b, n) = if short_blocks != 0 {
        (short_blocks, cfg.short_mdct_size)
    } else {
        (1, cfg.short_mdct_size << lm)
    };
    let tlen = 2 * n; // MDCT transform length: N -> N/2 outputs needs length 2N.
    let chan_in = b * n + overlap;
    let chan_out = b * n;

    for c in 0..cc_channels {
        for blk in 0..b {
            // Interleave the sub-blocks: block `blk` writes at offset `blk`,
            // stride `b`, into this channel's output region.
            let in_off = c * chan_in + blk * n;
            let out_off = c * chan_out + blk;
            let in_slice = &input[in_off..(c + 1) * chan_in];
            let out_slice = &mut out[out_off..(c + 1) * chan_out];
            clt_mdct_forward(in_slice, out_slice, cfg.window, tlen, overlap, b);
        }
    }

    // Downmix a stereo input to a mono spectrum by averaging the two channels.
    if cc_channels == 2 && c_channels == 1 {
        for i in 0..chan_out {
            out[i] = 0.5 * out[i] + 0.5 * out[chan_out + i];
        }
    }

    if upsample != 1 {
        let bound = chan_out / upsample;
        for c in 0..c_channels {
            let base = c * chan_out;
            for v in out[base..base + bound].iter_mut() {
                *v *= upsample as f32;
            }
            for v in out[base + bound..base + chan_out].iter_mut() {
                *v = 0.0;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn window_is_power_complementary() {
        // w[i]^2 + w[overlap-1-i]^2 == 1 (the Princen-Bradley TDAC condition).
        for &overlap in &[120usize, 60, 240] {
            let w = compute_window(overlap);
            assert_eq!(w.len(), overlap);
            for i in 0..overlap {
                let sum = w[i] * w[i] + w[overlap - 1 - i] * w[overlap - 1 - i];
                assert!((sum - 1.0).abs() < 1e-5, "i={i}: w^2 sum {sum}");
            }
        }
    }

    #[test]
    fn window_is_monotonic_and_bounded() {
        let overlap = 120;
        let w = compute_window(overlap);
        assert!(w[0] > 0.0 && w[0] < 0.05, "start {}", w[0]);
        assert!(w[overlap - 1] > 0.99, "end {}", w[overlap - 1]);
        for i in 1..overlap {
            assert!(w[i] >= w[i - 1], "not monotonic at {i}");
            assert!((0.0..=1.0).contains(&w[i]), "out of range at {i}");
        }
    }

    #[test]
    fn window_midpoint_is_half_amplitude() {
        // By symmetry the window crosses 1/sqrt(2) at its centre.
        let overlap = 120;
        let w = compute_window(overlap);
        let mid = w[overlap / 2 - 1] * w[overlap / 2 - 1] + w[overlap / 2] * w[overlap / 2];
        assert!((mid - 1.0).abs() < 1e-5);
    }

    /// Runs the forward MDCT for one block, returning the `n/2` coefficients.
    fn fwd(input: &[f32], n: usize, overlap: usize) -> Vec<f32> {
        let w = compute_window(overlap);
        let mut out = vec![0.0f32; n / 2];
        clt_mdct_forward(input, &mut out, &w, n, overlap, 1);
        out
    }

    /// Builds the analysis matrix `M` (rows = MDCT coefficients, cols = the input
    /// samples the transform actually reads) by feeding unit impulses.
    fn analysis_matrix(n: usize, overlap: usize) -> (Vec<Vec<f32>>, usize) {
        let l = n + overlap;
        let n2 = n / 2;
        // Column j is the response to the impulse at input index j.
        let mut cols = vec![vec![0.0f32; n2]; l];
        for (j, col) in cols.iter_mut().enumerate() {
            let mut e = vec![0.0f32; l];
            e[j] = 1.0;
            *col = fwd(&e, n, overlap);
        }
        // Transpose into rows (one per coefficient).
        let mut rows = vec![vec![0.0f32; l]; n2];
        for (j, col) in cols.iter().enumerate() {
            for k in 0..n2 {
                rows[k][j] = col[k];
            }
        }
        (rows, l)
    }

    #[test]
    fn forward_basis_rows_are_orthogonal() {
        // A correct windowed MDCT is an orthogonal lapped transform: its analysis
        // functions (rows of M) are mutually orthogonal and share one norm. This
        // pins the fold/rotation/DFT without needing an external decoder oracle.
        for &(n, overlap) in &[(240usize, 120usize), (480, 120), (120, 120)] {
            let (rows, _l) = analysis_matrix(n, overlap);
            let n2 = n / 2;
            let mut diag = 0.0f64;
            for r in &rows {
                diag += r.iter().map(|&v| (v * v) as f64).sum::<f64>();
            }
            diag /= n2 as f64; // mean row energy
            assert!(diag > 1e-6, "degenerate basis for n={n}");
            // Off-diagonal Gram entries must vanish; diagonal entries must match.
            for a in 0..n2 {
                let self_dot: f64 = rows[a].iter().map(|&v| (v * v) as f64).sum();
                assert!(
                    (self_dot - diag).abs() < 1e-3 * diag,
                    "n={n} row {a} norm {self_dot} != mean {diag}"
                );
                for b in (a + 1)..n2 {
                    let dot: f64 = rows[a]
                        .iter()
                        .zip(&rows[b])
                        .map(|(&x, &y)| (x * y) as f64)
                        .sum();
                    assert!(
                        dot.abs() < 1e-3 * diag,
                        "n={n} rows {a},{b} not orthogonal: {dot}"
                    );
                }
            }
        }
    }

    #[test]
    fn forward_is_deterministic() {
        let n = 240;
        let overlap = 120;
        let input: Vec<f32> = (0..n + overlap)
            .map(|i| (0.017 * i as f32).sin() * 0.3)
            .collect();
        assert_eq!(fwd(&input, n, overlap), fwd(&input, n, overlap));
    }

    #[test]
    fn forward_localizes_a_pure_tone() {
        // A sinusoid at the centre frequency of coefficient k0 should put almost
        // all of the MDCT energy at (or immediately around) k0 — this pins the
        // frequency mapping and rules out a reflected or permuted spectrum.
        let n = 256;
        let overlap = 120;
        let n2 = n / 2;
        let k0 = 40usize;
        // MDCT coefficient k responds to angular frequency (k+0.5)*pi/n2 over the
        // block; build that tone across the full read span.
        let freq = (k0 as f32 + 0.5) * core::f32::consts::PI / n2 as f32;
        let input: Vec<f32> = (0..n + overlap).map(|i| (freq * i as f32).cos()).collect();
        let out = fwd(&input, n, overlap);
        let peak = (0..n2)
            .max_by(|&a, &b| out[a].abs().total_cmp(&out[b].abs()))
            .unwrap();
        assert!(
            peak.abs_diff(k0) <= 1,
            "tone for coeff {k0} peaked at {peak}"
        );
        let total: f32 = out.iter().map(|&v| v * v).sum();
        let near: f32 = (k0.saturating_sub(2)..=(k0 + 2).min(n2 - 1))
            .map(|k| out[k] * out[k])
            .sum();
        assert!(
            near > 0.7 * total,
            "energy not concentrated: {near}/{total}"
        );
    }

    fn cfg(window: &[f32]) -> MdctConfig<'_> {
        MdctConfig {
            overlap: 120,
            short_mdct_size: 120,
            window,
        }
    }

    #[test]
    fn compute_mdcts_long_block_matches_direct_forward() {
        // short_blocks=0, lm=0: one long block of length 2*short_mdct_size.
        let overlap = 120;
        let n = 120; // short_mdct_size << lm (lm=0)
        let w = compute_window(overlap);
        let input: Vec<f32> = (0..n + overlap)
            .map(|i| (0.031 * i as f32).sin() * 0.4)
            .collect();
        let mut got = vec![0.0f32; n];
        compute_mdcts(&cfg(&w), 0, 0, &input, &mut got, 1, 1, 1);

        let mut want = vec![0.0f32; n];
        clt_mdct_forward(&input, &mut want, &w, 2 * n, overlap, 1);
        for (a, b) in got.iter().zip(&want) {
            assert!((a - b).abs() < 1e-5, "long block mismatch {a} vs {b}");
        }
    }

    #[test]
    fn compute_mdcts_short_blocks_interleave_by_stride() {
        // B short blocks land interleaved (block b at offset b, stride B); pulling
        // each stride back out must equal that block transformed on its own.
        let overlap = 120;
        let n = 120;
        let b = 4usize;
        let w = compute_window(overlap);
        let chan_in = b * n + overlap;
        let input: Vec<f32> = (0..chan_in).map(|i| (0.013 * i as f32).cos()).collect();
        let mut got = vec![0.0f32; b * n];
        compute_mdcts(&cfg(&w), b, 0, &input, &mut got, 1, 1, 1);

        for blk in 0..b {
            let block_in = &input[blk * n..blk * n + n + overlap];
            let mut want = vec![0.0f32; n];
            clt_mdct_forward(block_in, &mut want, &w, 2 * n, overlap, 1);
            for k in 0..n {
                let got_k = got[blk + k * b];
                assert!(
                    (got_k - want[k]).abs() < 1e-5,
                    "block {blk} coeff {k}: {got_k} vs {}",
                    want[k]
                );
            }
        }
    }

    #[test]
    fn compute_mdcts_downmixes_stereo_to_mono() {
        let overlap = 120;
        let n = 120;
        let w = compute_window(overlap);
        let chan_in = n + overlap;
        let mut input = vec![0.0f32; 2 * chan_in];
        for i in 0..chan_in {
            input[i] = (0.02 * i as f32).sin();
            input[chan_in + i] = (0.05 * i as f32).cos();
        }
        // Mono output (C=1) from a stereo input (CC=2): both spectra are
        // transformed (out spans CC regions) and averaged into the first.
        let mut mono = vec![0.0f32; 2 * n];
        compute_mdcts(&cfg(&w), 0, 0, &input, &mut mono, 1, 2, 1);

        let mut l = vec![0.0f32; n];
        let mut r = vec![0.0f32; n];
        clt_mdct_forward(&input[..chan_in], &mut l, &w, 2 * n, overlap, 1);
        clt_mdct_forward(&input[chan_in..], &mut r, &w, 2 * n, overlap, 1);
        for k in 0..n {
            let want = 0.5 * l[k] + 0.5 * r[k];
            assert!((mono[k] - want).abs() < 1e-5, "downmix {k}");
        }
    }

    #[test]
    fn compute_mdcts_upsample_scales_and_clears_top() {
        let overlap = 120;
        let n = 120;
        let w = compute_window(overlap);
        let input: Vec<f32> = (0..n + overlap).map(|i| (0.02 * i as f32).sin()).collect();
        let mut base = vec![0.0f32; n];
        compute_mdcts(&cfg(&w), 0, 0, &input, &mut base, 1, 1, 1);
        let mut up = vec![0.0f32; n];
        compute_mdcts(&cfg(&w), 0, 0, &input, &mut up, 1, 1, 2);

        let bound = n / 2;
        for k in 0..bound {
            assert!((up[k] - 2.0 * base[k]).abs() < 1e-5, "scale {k}");
        }
        for &v in &up[bound..] {
            assert_eq!(v, 0.0, "top must be cleared");
        }
    }
}
