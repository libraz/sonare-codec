//! CELT pre-emphasis input filter.
//!
//! Hand-ported to safe Rust from the float build of libopus `celt/celt_encoder.c`
//! (`celt_preemphasis`). Derivative work of libopus (BSD-3-Clause); see
//! `LICENSE-THIRDPARTY`.
//!
//! Pre-emphasis is the first encoder stage: it de-interleaves one channel from
//! the input PCM, optionally upsamples and clips it, and applies the first-order
//! high-pass `inp[i] = x[i] - 0.85*x[i-1]` whose history is carried across frames
//! in `mem`. The matching de-emphasis runs in the decoder (Symphonia), so this is
//! encoder-only.

// Consumed by the CELT encode entry point; the live encoder still ships via the
// Opus FFI path.
#![allow(dead_code)]

/// `celt_preemphasis`: write `n` pre-emphasised samples of one channel into `inp`.
///
/// `pcm` is the interleaved input read at channel stride `cc` (the caller offsets
/// `pcm` to the channel); `coef0` is the pre-emphasis coefficient (0.85 for the
/// standard modes); `mem` carries the filter state across frames. `upsample > 1`
/// zero-stuffs the output before filtering; `clip` bounds the input to ±65536 to
/// keep encodes portable. Only the standard `coef[1] == 0` path is implemented
/// (custom modes are out of scope).
#[allow(clippy::too_many_arguments)]
pub fn celt_preemphasis(
    pcm: &[f32],
    inp: &mut [f32],
    n: usize,
    cc: usize,
    upsample: usize,
    coef0: f32,
    mem: &mut f32,
    clip: bool,
) {
    let mut m = *mem;
    let nu = n / upsample;

    if upsample != 1 {
        for v in inp[..n].iter_mut() {
            *v = 0.0;
        }
    }
    // De-interleave (and zero-stuff) the channel into the output buffer.
    for i in 0..nu {
        inp[i * upsample] = pcm[cc * i];
    }
    if clip {
        // Clip the input so non-portable out-of-range files don't get encoded.
        for i in 0..nu {
            inp[i * upsample] = inp[i * upsample].clamp(-65536.0, 65536.0);
        }
    }

    // First-order pre-emphasis high-pass over the whole (possibly stuffed) frame.
    for v in inp[..n].iter_mut() {
        let x = *v;
        *v = x - m;
        m = coef0 * x;
    }
    *mem = m;
}

#[cfg(test)]
mod tests {
    use super::*;

    const COEF: f32 = 0.85;

    #[test]
    fn preemphasis_matches_first_order_highpass() {
        let pcm: Vec<f32> = (0..16).map(|i| (i as f32 * 0.5).sin()).collect();
        let mut inp = vec![0.0f32; pcm.len()];
        let mut mem = 0.0f32;
        celt_preemphasis(&pcm, &mut inp, pcm.len(), 1, 1, COEF, &mut mem, false);
        // inp[i] = x[i] - 0.85*x[i-1], with x[-1] = 0.
        let mut prev = 0.0f32;
        for (i, &x) in pcm.iter().enumerate() {
            let want = x - COEF * prev;
            assert!(
                (inp[i] - want).abs() < 1e-6,
                "sample {i}: {} vs {want}",
                inp[i]
            );
            prev = x;
        }
        // mem holds 0.85 * last input for the next frame.
        assert!((mem - COEF * pcm[pcm.len() - 1]).abs() < 1e-6);
    }

    #[test]
    fn preemphasis_memory_chains_across_frames() {
        let pcm: Vec<f32> = (0..32).map(|i| (i as f32 * 0.21).cos()).collect();
        // One shot over the whole signal.
        let mut whole = vec![0.0f32; pcm.len()];
        let mut mem_w = 0.0f32;
        celt_preemphasis(&pcm, &mut whole, pcm.len(), 1, 1, COEF, &mut mem_w, false);
        // Two halves carrying mem between them must match exactly.
        let mut split = vec![0.0f32; pcm.len()];
        let mut mem_s = 0.0f32;
        celt_preemphasis(
            &pcm[..16],
            &mut split[..16],
            16,
            1,
            1,
            COEF,
            &mut mem_s,
            false,
        );
        celt_preemphasis(
            &pcm[16..],
            &mut split[16..],
            16,
            1,
            1,
            COEF,
            &mut mem_s,
            false,
        );
        assert_eq!(whole, split);
        assert_eq!(mem_w.to_bits(), mem_s.to_bits());
    }

    #[test]
    fn preemphasis_deinterleaves_by_channel_stride() {
        // Stereo interleaved L,R,L,R...; reading channel 1 (offset 1, stride 2)
        // must pick the R samples only.
        let n = 8usize;
        let mut interleaved = vec![0.0f32; 2 * n];
        for i in 0..n {
            interleaved[2 * i] = i as f32; // L
            interleaved[2 * i + 1] = -(i as f32); // R
        }
        let mut inp = vec![0.0f32; n];
        let mut mem = 0.0f32;
        celt_preemphasis(&interleaved[1..], &mut inp, n, 2, 1, COEF, &mut mem, false);
        let mut prev = 0.0f32;
        for (i, &got) in inp.iter().enumerate() {
            let x = -(i as f32);
            assert!((got - (x - COEF * prev)).abs() < 1e-6, "R sample {i}");
            prev = x;
        }
    }

    #[test]
    fn preemphasis_clip_bounds_input() {
        let pcm = vec![100_000.0f32, -200_000.0, 0.0, 50_000.0];
        let mut inp = vec![0.0f32; pcm.len()];
        let mut mem = 0.0f32;
        celt_preemphasis(&pcm, &mut inp, pcm.len(), 1, 1, COEF, &mut mem, true);
        // First clipped input is +65536; inp[0] = 65536 - 0.
        assert!((inp[0] - 65536.0).abs() < 1e-3);
        // Second: clipped to -65536, minus 0.85*65536 memory.
        assert!((inp[1] - (-65536.0 - COEF * 65536.0)).abs() < 1e-1);
    }

    #[test]
    fn preemphasis_upsample_zero_stuffs() {
        let pcm = vec![1.0f32, 2.0, 3.0, 4.0];
        let n = pcm.len() * 2;
        let mut inp = vec![9.0f32; n]; // pre-filled to confirm clearing
        let mut mem = 0.0f32;
        celt_preemphasis(&pcm, &mut inp, n, 1, 2, COEF, &mut mem, false);
        // Even positions carry the (filtered) samples, odd positions the stuffed
        // zeros run through the same recursive filter.
        let mut prev = 0.0f32;
        let stuffed = [pcm[0], 0.0, pcm[1], 0.0, pcm[2], 0.0, pcm[3], 0.0];
        for i in 0..n {
            let want = stuffed[i] - COEF * prev;
            assert!((inp[i] - want).abs() < 1e-6, "upsampled sample {i}");
            prev = stuffed[i];
        }
    }
}
