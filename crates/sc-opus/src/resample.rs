//! Bandlimited sample-rate conversion for the Opus front end.
//!
//! Opus operates internally at 48 kHz, so PCM at any other rate is converted
//! here before CELT encoding. This is a windowed-sinc (Blackman) resampler with
//! per-output-sample weight normalization so the DC gain stays exactly one and
//! edges are not attenuated. It anti-aliases when downsampling (cutoff drops to
//! the destination Nyquist) and band-limits to the source Nyquist when
//! upsampling. There is no external dependency, so it builds for wasm.

use std::f64::consts::PI;

/// Half the number of sinc zero crossings retained on each side of the kernel.
/// 16 gives a long, clean transition band — overkill is cheap here and keeps the
/// resampler well below Opus's own lossy noise floor.
const SINC_HALF_ZEROS: f64 = 16.0;

/// Opus's fixed internal sample rate.
const TARGET_RATE: u32 = 48_000;

fn sinc(x: f64) -> f64 {
    if x.abs() < 1.0e-9 {
        1.0
    } else {
        let pix = PI * x;
        pix.sin() / pix
    }
}

/// Symmetric Blackman window over `t in [-half_width, half_width]`.
fn blackman(t: f64, half_width: f64) -> f64 {
    let r = t / half_width;
    if r.abs() > 1.0 {
        return 0.0;
    }
    let pr = PI * r;
    0.42 + 0.5 * pr.cos() + 0.08 * (2.0 * pr).cos()
}

/// Resamples one channel from `src_rate` to `dst_rate` with a windowed-sinc
/// kernel. Returns the input unchanged when the rates already match.
fn resample_channel(input: &[f32], src_rate: u32, dst_rate: u32) -> Vec<f32> {
    if src_rate == dst_rate || input.is_empty() {
        return input.to_vec();
    }

    // Output samples per input sample.
    let ratio = f64::from(dst_rate) / f64::from(src_rate);
    // Cutoff in cycles per input sample (input Nyquist = 0.5). Downsampling
    // lowers it to the destination Nyquist to suppress aliasing; upsampling adds
    // no content above the source Nyquist.
    let cutoff = 0.5 * ratio.min(1.0);
    let half_width = SINC_HALF_ZEROS / (2.0 * cutoff); // kernel half-support in input samples
    let taps = half_width.ceil() as isize;
    let n_out = ((input.len() as f64) * ratio).round() as usize;

    let mut out = Vec::with_capacity(n_out);
    for m in 0..n_out {
        let center = m as f64 / ratio; // position in input-sample coordinates
        let i0 = center.floor() as isize;
        let mut acc = 0.0_f64;
        let mut weight = 0.0_f64;
        for k in (i0 - taps)..=(i0 + taps + 1) {
            if k < 0 {
                continue;
            }
            let idx = k as usize;
            if idx >= input.len() {
                continue;
            }
            let dt = center - k as f64;
            let w = 2.0 * cutoff * sinc(2.0 * cutoff * dt) * blackman(dt, half_width);
            acc += f64::from(input[idx]) * w;
            weight += w;
        }
        let value = if weight.abs() > 1.0e-12 {
            acc / weight
        } else {
            0.0
        };
        out.push(value as f32);
    }
    out
}

/// Converts interleaved PCM to 48 kHz, resampling each channel independently.
/// Returns the input unchanged when it is already 48 kHz.
#[must_use]
pub(crate) fn resample_to_48k(samples: &[f32], channels: u16, src_rate: u32) -> Vec<f32> {
    let ch = usize::from(channels);
    if src_rate == TARGET_RATE || ch == 0 || samples.is_empty() {
        return samples.to_vec();
    }

    let frames = samples.len() / ch;
    let mut resampled: Vec<Vec<f32>> = Vec::with_capacity(ch);
    for c in 0..ch {
        let channel: Vec<f32> = (0..frames).map(|f| samples[f * ch + c]).collect();
        resampled.push(resample_channel(&channel, src_rate, TARGET_RATE));
    }

    let n_out = resampled.first().map_or(0, Vec::len);
    let mut out = Vec::with_capacity(n_out * ch);
    for f in 0..n_out {
        for channel in &resampled {
            out.push(channel.get(f).copied().unwrap_or(0.0));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passthrough_when_already_48k() {
        let samples = vec![0.1, -0.2, 0.3, -0.4];
        let out = resample_to_48k(&samples, 2, 48_000);
        assert_eq!(out, samples);
    }

    #[test]
    fn preserves_dc_level() {
        // A constant signal must stay constant (unity DC gain) across the body.
        let input = vec![0.5_f32; 4_410];
        let out = resample_channel(&input, 44_100, 48_000);
        let body = &out[64..out.len() - 64];
        for &s in body {
            assert!((s - 0.5).abs() < 1.0e-3, "DC not preserved: {s}");
        }
    }

    #[test]
    fn output_length_tracks_ratio() {
        let input = vec![0.0_f32; 44_100];
        let out = resample_channel(&input, 44_100, 48_000);
        assert_eq!(out.len(), 48_000);

        let down = resample_channel(&input, 44_100, 24_000);
        assert_eq!(down.len(), 24_000);
    }

    #[test]
    fn preserves_tone_frequency_on_upsample() {
        // A 1 kHz sine at 16 kHz, resampled to 48 kHz, must still be a 1 kHz
        // sine. Compare against a reference generated directly at 48 kHz.
        let src_rate = 16_000;
        let freq = 1_000.0_f64;
        let input: Vec<f32> = (0..16_000)
            .map(|n| (2.0 * PI * freq * n as f64 / f64::from(src_rate)).sin() as f32)
            .collect();
        let out = resample_channel(&input, src_rate, 48_000);

        let reference: Vec<f32> = (0..out.len())
            .map(|n| (2.0 * PI * freq * n as f64 / 48_000.0).sin() as f32)
            .collect();

        // Correlate over a steady interior segment (skip filter ramp-up).
        let seg = &out[480..out.len() - 480];
        let r = &reference[480..reference.len() - 480];
        let mut dot = 0.0_f64;
        let mut na = 0.0_f64;
        let mut nb = 0.0_f64;
        for i in 0..seg.len() {
            dot += f64::from(seg[i]) * f64::from(r[i]);
            na += f64::from(seg[i]).powi(2);
            nb += f64::from(r[i]).powi(2);
        }
        let corr = dot / (na.sqrt() * nb.sqrt());
        assert!(corr > 0.99, "resampled tone correlation too low: {corr}");
    }
}
