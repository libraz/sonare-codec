//! Vorbis noise-masking regression.
//!
//! Hand-ported to safe Rust from `bark_noise_hybridmp` and the bark-window
//! setup in `_vp_psy_init` (libvorbis/aoTuV `lib/psy.c`, the portable non-SSE
//! paths): a sliding weighted linear regression over each bin's Bark-scale
//! window estimates the local noise floor of a log-magnitude spectrum.
//! Derivative work of libvorbis/aoTuV (BSD-3-Clause); see `LICENSE-THIRDPARTY`.
//!
//! Each bin carries a packed window `((lo-1) << 16) | (hi-1)` (a low edge that
//! may be negative — reflected at DC — and a high edge). The regression keeps
//! running prefix sums of the weighted moments and solves a 2×2 weighted
//! least-squares per bin. The prefix sums are accumulated in `f64`: libvorbis
//! uses `f32` and tolerates the resulting noise-floor jitter, but the windowed
//! differences of large prefix sums lose too much to `f32` cancellation, and
//! this is an internal perceptual quantity (never part of the bitstream).

// Feeds the masking mix; the live encoder still ships via FFI.
#![allow(dead_code)]

use crate::psy::to_bark;

/// Builds the per-bin Bark-scale noise windows (`_vp_psy_init`'s `bark` loop),
/// each packed as `((lo - 1) << 16) | (hi - 1)`.
///
/// `noisewindowlo`/`noisewindowhi` are the window widths in Bark and
/// `noisewindowlomin`/`himin` the minimum widths in bins. The per-bin frequency
/// uses integer Hz-per-bin, exactly as libvorbis does.
#[must_use]
pub fn build_bark_windows(
    n: usize,
    rate: u32,
    noisewindowlo: f32,
    noisewindowhi: f32,
    noisewindowlomin: i32,
    noisewindowhimin: i32,
) -> Vec<i32> {
    if n == 0 || rate == 0 {
        return Vec::new();
    }
    // Integer Hz per bin, matching the C `rate/(2*n)`.
    let hz_per_bin = (rate / (2 * n as u32)) as f32;
    let bark_at = |k: i32| to_bark(hz_per_bin * k as f32);

    let mut out = vec![0i32; n];
    let mut lo: i32 = -99;
    let mut hi: i32 = 1;
    let nn = n as i32;

    for (i, slot) in out.iter_mut().enumerate() {
        let ii = i as i32;
        let bark = bark_at(ii);
        while lo + noisewindowlomin < ii && bark_at(lo) < bark - noisewindowlo {
            lo += 1;
        }
        while hi <= nn && (hi < ii + noisewindowhimin || bark_at(hi) < bark + noisewindowhi) {
            hi += 1;
        }
        *slot = ((lo - 1) << 16) + (hi - 1);
    }
    out
}

/// Weighted-moment prefix sums of `f + offset` (clamped to `>= 1`), in `f64`.
struct Moments {
    n: Vec<f64>,
    x: Vec<f64>,
    xx: Vec<f64>,
    y: Vec<f64>,
    xy: Vec<f64>,
}

impl Moments {
    fn build(f: &[f32], offset: f64) -> Self {
        let len = f.len();
        let mut m = Moments {
            n: vec![0.0; len],
            x: vec![0.0; len],
            xx: vec![0.0; len],
            y: vec![0.0; len],
            xy: vec![0.0; len],
        };
        if len == 0 {
            return m;
        }

        // The first sample carries half weight and contributes only to N/X/Y.
        let mut y = f64::from(f[0]) + offset;
        if y < 1.0 {
            y = 1.0;
        }
        let mut t_n = y * y * 0.5;
        let mut t_x = t_n;
        let mut t_xx = 0.0;
        let mut t_y = t_n * y;
        let mut t_xy = 0.0;
        m.n[0] = t_n;
        m.x[0] = t_x;
        m.xx[0] = t_xx;
        m.y[0] = t_y;
        m.xy[0] = t_xy;

        // Indexed to mirror the C prefix-sum recurrence (i is also x).
        #[allow(clippy::needless_range_loop)]
        for i in 1..len {
            let x = i as f64;
            let mut y = f64::from(f[i]) + offset;
            if y < 1.0 {
                y = 1.0;
            }
            let w = y * y;
            t_n += w;
            t_x += w * x;
            t_xx += w * x * x;
            t_y += w * y;
            t_xy += w * x * y;
            m.n[i] = t_n;
            m.x[i] = t_x;
            m.xx[i] = t_xx;
            m.y[i] = t_y;
            m.xy[i] = t_xy;
        }
        m
    }
}

/// Solves the 2×2 weighted regression for the estimate at position `x`,
/// clamped non-negative.
fn regress(t_n: f64, t_x: f64, t_xx: f64, t_y: f64, t_xy: f64, x: f64) -> (f64, f64, f64, f64) {
    let a = t_y * t_xx - t_x * t_xy;
    let b = t_n * t_xy - t_x * t_y;
    let d = t_n * t_xx - t_x * t_x;
    let mut r = (a + x * b) / d;
    if r < 0.0 {
        r = 0.0;
    }
    (r, a, b, d)
}

/// Estimates the noise floor of `f` over each bin's Bark window `bark`, writing
/// `noise[i] = regression(i) - offset` (`bark_noise_hybridmp`, non-SSE path).
///
/// When `fixed > 0` a second, fixed-width regression pass takes the per-bin
/// minimum. `bark`, `f` and `noise` are all length `n`.
pub fn bark_noise_hybridmp(bark: &[i32], f: &[f32], noise: &mut [f32], offset: f32, fixed: i32) {
    let n = f.len();
    if bark.len() != n || noise.len() != n || n == 0 {
        return;
    }
    let offset = f64::from(offset);
    let m = Moments::build(f, offset);

    let read = |buf: &[f64], idx: i32| -> f64 { buf[idx as usize] };

    let mut a = 0.0;
    let mut b = 0.0;
    let mut d = 1.0;
    let mut i = 0usize;

    // Low-frequency region: the window reflects across DC (lo < 0).
    while i < n {
        let lo = bark[i] >> 16;
        let hi = bark[i] & 0xffff;
        if lo >= 0 || -lo >= n as i32 || hi >= n as i32 {
            break;
        }
        let nlo = -lo;
        let t_n = read(&m.n, hi) + read(&m.n, nlo);
        let t_x = read(&m.x, hi) - read(&m.x, nlo);
        let t_xx = read(&m.xx, hi) + read(&m.xx, nlo);
        let t_y = read(&m.y, hi) + read(&m.y, nlo);
        let t_xy = read(&m.xy, hi) - read(&m.xy, nlo);
        let (r, na, nb, nd) = regress(t_n, t_x, t_xx, t_y, t_xy, i as f64);
        (a, b, d) = (na, nb, nd);
        noise[i] = (r - offset) as f32;
        i += 1;
    }

    // Interior: the window is a plain prefix-sum difference.
    while i < n {
        let lo = bark[i] >> 16;
        let hi = bark[i] & 0xffff;
        if lo < 0 || lo >= n as i32 || hi >= n as i32 {
            break;
        }
        let t_n = read(&m.n, hi) - read(&m.n, lo);
        let t_x = read(&m.x, hi) - read(&m.x, lo);
        let t_xx = read(&m.xx, hi) - read(&m.xx, lo);
        let t_y = read(&m.y, hi) - read(&m.y, lo);
        let t_xy = read(&m.xy, hi) - read(&m.xy, lo);
        let (r, na, nb, nd) = regress(t_n, t_x, t_xx, t_y, t_xy, i as f64);
        (a, b, d) = (na, nb, nd);
        noise[i] = (r - offset) as f32;
        i += 1;
    }

    // Remaining high bins reuse the last regression coefficients.
    while i < n {
        let mut r = (a + i as f64 * b) / d;
        if r < 0.0 {
            r = 0.0;
        }
        noise[i] = (r - offset) as f32;
        i += 1;
    }

    if fixed <= 0 {
        return;
    }

    // Fixed-width pass: take the per-bin minimum of the two estimates.
    let mut i = 0usize;
    while i < n {
        let hi = i as i32 + fixed / 2;
        let lo = hi - fixed;
        if hi >= n as i32 || lo >= 0 {
            break;
        }
        let nlo = -lo;
        let t_n = read(&m.n, hi) + read(&m.n, nlo);
        let t_x = read(&m.x, hi) - read(&m.x, nlo);
        let t_xx = read(&m.xx, hi) + read(&m.xx, nlo);
        let t_y = read(&m.y, hi) + read(&m.y, nlo);
        let t_xy = read(&m.xy, hi) - read(&m.xy, nlo);
        let (r, na, nb, nd) = regress_unclamped(t_n, t_x, t_xx, t_y, t_xy, i as f64);
        (a, b, d) = (na, nb, nd);
        let v = (r - offset) as f32;
        if v < noise[i] {
            noise[i] = v;
        }
        i += 1;
    }
    while i < n {
        let hi = i as i32 + fixed / 2;
        let lo = hi - fixed;
        if hi >= n as i32 || lo < 0 {
            break;
        }
        let t_n = read(&m.n, hi) - read(&m.n, lo);
        let t_x = read(&m.x, hi) - read(&m.x, lo);
        let t_xx = read(&m.xx, hi) - read(&m.xx, lo);
        let t_y = read(&m.y, hi) - read(&m.y, lo);
        let t_xy = read(&m.xy, hi) - read(&m.xy, lo);
        let (r, na, nb, nd) = regress_unclamped(t_n, t_x, t_xx, t_y, t_xy, i as f64);
        (a, b, d) = (na, nb, nd);
        let v = (r - offset) as f32;
        if v < noise[i] {
            noise[i] = v;
        }
        i += 1;
    }
    while i < n {
        let r = (a + i as f64 * b) / d;
        let v = (r - offset) as f32;
        if v < noise[i] {
            noise[i] = v;
        }
        i += 1;
    }
}

/// Combines the noise floor and tone mask into the final per-bin masking curve
/// (`logmask`) — the base mix of `_vp_offset_and_mix`, before the AoTuV M1/M3/M4
/// refinements.
///
/// For each bin: `val = min(noise[i] + noiseoffset[i], noisemaxsupp)` and
/// `tval = tone[i] + toneatt`; the louder of the two masks the bin
/// (`logmask[i] = max(val, tval)`). `noiseoffset` is the per-bin noise offset
/// for the selected curve, `toneatt` the tone master attenuation, and
/// `noisemaxsupp` the noise-suppression ceiling. A no-op on mismatched lengths.
pub fn offset_and_mix(
    noise: &[f32],
    tone: &[f32],
    noiseoffset: &[f32],
    toneatt: f32,
    noisemaxsupp: f32,
    logmask: &mut [f32],
) {
    let n = noise.len();
    if tone.len() != n || noiseoffset.len() != n || logmask.len() != n {
        return;
    }
    for i in 0..n {
        let mut val = noise[i] + noiseoffset[i];
        if val > noisemaxsupp {
            val = noisemaxsupp;
        }
        let tval = tone[i] + toneatt;
        logmask[i] = val.max(tval);
    }
}

/// Like [`regress`] but without the non-negative clamp (the fixed-width pass
/// does not clamp `R` before the per-bin minimum).
fn regress_unclamped(
    t_n: f64,
    t_x: f64,
    t_xx: f64,
    t_y: f64,
    t_xy: f64,
    x: f64,
) -> (f64, f64, f64, f64) {
    let a = t_y * t_xx - t_x * t_xy;
    let b = t_n * t_xy - t_x * t_y;
    let d = t_n * t_xx - t_x * t_x;
    let r = (a + x * b) / d;
    (r, a, b, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bark_windows_slide_monotonically() {
        let bark = build_bark_windows(1024, 48_000, 0.5, 0.5, 1, 1);
        assert_eq!(bark.len(), 1024);
        let mut last_lo = i32::MIN;
        let mut last_hi = i32::MIN;
        for &packed in &bark {
            let lo = packed >> 16;
            let hi = packed & 0xffff;
            assert!(lo >= last_lo, "lo edge went backwards");
            assert!(hi >= last_hi, "hi edge went backwards");
            last_lo = lo;
            last_hi = hi;
        }
    }

    #[test]
    fn degenerate_inputs_are_empty_or_no_ops() {
        assert!(build_bark_windows(0, 48_000, 0.5, 0.5, 1, 1).is_empty());
        let mut noise = vec![0.0f32; 4];
        // Length mismatch: no panic, no write.
        bark_noise_hybridmp(&[0; 3], &[0.0; 4], &mut noise, 140.0, -1);
        assert!(noise.iter().all(|&v| v == 0.0));
    }

    #[test]
    fn flat_spectrum_recovers_its_own_level() {
        // A constant log-spectrum: the regression of a constant is that
        // constant, so noise[i] == f[i] (the offset cancels).
        let n = 1024;
        let bark = build_bark_windows(n, 48_000, 0.5, 0.5, 1, 1);
        let f = vec![-50.0f32; n];
        let mut noise = vec![0.0f32; n];
        bark_noise_hybridmp(&bark, &f, &mut noise, 140.0, -1);
        for (i, &v) in noise.iter().enumerate() {
            assert!((v + 50.0).abs() < 0.5, "bin {i}: {v}");
        }
    }

    #[test]
    fn output_is_finite_for_a_varied_spectrum() {
        let n = 1024;
        let bark = build_bark_windows(n, 48_000, 0.5, 0.5, 1, 1);
        // A descending spectrum with a couple of peaks.
        let f: Vec<f32> = (0..n)
            .map(|i| -40.0 - i as f32 * 0.05 + if i % 128 == 0 { 30.0 } else { 0.0 })
            .collect();
        let mut noise = vec![0.0f32; n];
        bark_noise_hybridmp(&bark, &f, &mut noise, 140.0, -1);
        assert!(noise.iter().all(|v| v.is_finite()));
        // The estimated floor should sit at or below the strong tonal peaks.
        for i in (0..n).step_by(128) {
            assert!(noise[i] <= f[i] + 1.0, "floor above peak at {i}");
        }
    }

    #[test]
    fn fixed_pass_only_lowers_the_estimate() {
        let n = 512;
        let bark = build_bark_windows(n, 48_000, 0.5, 0.5, 1, 1);
        let f: Vec<f32> = (0..n)
            .map(|i| -30.0 - (i as f32 * 0.1).sin() * 10.0)
            .collect();

        let mut base = vec![0.0f32; n];
        bark_noise_hybridmp(&bark, &f, &mut base, 140.0, -1);

        let mut fixed = vec![0.0f32; n];
        bark_noise_hybridmp(&bark, &f, &mut fixed, 140.0, 64);

        for (i, (&b, &x)) in base.iter().zip(&fixed).enumerate() {
            assert!(x <= b + 1e-3, "fixed pass raised bin {i}: {x} > {b}");
        }
    }

    #[test]
    fn mix_takes_the_louder_of_noise_and_tone() {
        let noise = vec![-60.0f32, -40.0, -50.0];
        let tone = vec![-50.0f32, -55.0, -50.0];
        let off = vec![0.0f32; 3];
        let mut logmask = vec![0.0f32; 3];
        // toneatt 0, a high suppression ceiling so nothing is capped.
        offset_and_mix(&noise, &tone, &off, 0.0, 100.0, &mut logmask);
        // bin 0: tone -50 > noise -60 -> -50; bin 1: noise -40 > tone -55 -> -40;
        // bin 2: equal -> -50.
        assert_eq!(logmask, vec![-50.0, -40.0, -50.0]);
    }

    #[test]
    fn mix_applies_the_noise_offset_and_tone_attenuation() {
        let noise = vec![-60.0f32];
        let tone = vec![-70.0f32];
        let off = vec![5.0f32]; // noise becomes -55
        let mut logmask = vec![0.0f32; 1];
        // toneatt -10 makes tone -80; noise -55 wins.
        offset_and_mix(&noise, &tone, &off, -10.0, 100.0, &mut logmask);
        assert_eq!(logmask[0], -55.0);
    }

    #[test]
    fn mix_caps_the_noise_term_at_the_suppression_ceiling() {
        let noise = vec![10.0f32];
        let tone = vec![-90.0f32];
        let off = vec![0.0f32];
        let mut logmask = vec![0.0f32; 1];
        // noise 10 is capped at -20; tone -90 is quieter, so logmask = -20.
        offset_and_mix(&noise, &tone, &off, 0.0, -20.0, &mut logmask);
        assert_eq!(logmask[0], -20.0);
    }

    #[test]
    fn mix_is_a_no_op_on_length_mismatch() {
        let noise = vec![-60.0f32; 4];
        let tone = vec![-50.0f32; 4];
        let off = vec![0.0f32; 3]; // wrong length
        let mut logmask = vec![1.0f32; 4];
        offset_and_mix(&noise, &tone, &off, 0.0, 100.0, &mut logmask);
        assert!(logmask.iter().all(|&v| v == 1.0));
    }
}
