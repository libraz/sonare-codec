use super::*;

pub fn hann_window(len: usize) -> Result<Vec<f64>, Error> {
    if len == 0 {
        return Err(Error::InvalidInput(
            "psychoacoustic window length must be non-zero",
        ));
    }
    let denom = len as f64;
    Ok((0..len)
        .map(|n| 0.5 * (1.0 - (std::f64::consts::TAU * n as f64 / denom).cos()))
        .collect())
}

/// One complex frequency bin of a discrete Fourier transform.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ComplexBin {
    pub re: f64,
    pub im: f64,
}

impl ComplexBin {
    /// Squared magnitude (energy) of the bin.
    #[must_use]
    pub fn energy(self) -> f64 {
        self.re * self.re + self.im * self.im
    }

    /// Magnitude of the bin.
    #[must_use]
    pub fn magnitude(self) -> f64 {
        self.energy().sqrt()
    }

    /// Phase angle of the bin in radians.
    #[must_use]
    pub fn phase(self) -> f64 {
        self.im.atan2(self.re)
    }
}

/// Returns whether `n` is a positive power of two.
pub(crate) fn is_power_of_two(n: usize) -> bool {
    n != 0 && (n & (n - 1)) == 0
}

/// In-place iterative radix-2 Cooley–Tukey FFT over the full complex spectrum.
///
/// `re` and `im` carry the real and imaginary parts of `N` samples, where `N`
/// (their shared length) is a power of two; on return they hold the forward
/// transform `X[k] = Σ x[t]·e^(−i2πkt/N)`. Twiddle factors are advanced by
/// complex multiplication within each stage to avoid a trig call per butterfly.
pub(crate) fn radix2_fft_in_place(re: &mut [f64], im: &mut [f64]) {
    let n = re.len();
    if n < 2 {
        return;
    }

    // Decimation-in-time bit-reversal permutation of the input order.
    let mut j = 0usize;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j |= bit;
        if i < j {
            re.swap(i, j);
            im.swap(i, j);
        }
    }

    // Butterfly stages over spans of length 2, 4, … N.
    let mut span = 2usize;
    while span <= n {
        let angle = -std::f64::consts::TAU / span as f64;
        let (step_cos, step_sin) = (angle.cos(), angle.sin());
        let half = span / 2;
        let mut base = 0usize;
        while base < n {
            let mut w_cos = 1.0_f64;
            let mut w_sin = 0.0_f64;
            for k in 0..half {
                let a = base + k;
                let b = a + half;
                let t_re = w_cos * re[b] - w_sin * im[b];
                let t_im = w_cos * im[b] + w_sin * re[b];
                re[b] = re[a] - t_re;
                im[b] = im[a] - t_im;
                re[a] += t_re;
                im[a] += t_im;
                let next_cos = w_cos * step_cos - w_sin * step_sin;
                let next_sin = w_cos * step_sin + w_sin * step_cos;
                w_cos = next_cos;
                w_sin = next_sin;
            }
            base += span;
        }
        span <<= 1;
    }
}

/// Computes the lower half-spectrum (`0..=N/2`) of a real signal via a direct
/// DFT, returning one [`ComplexBin`] per retained bin. Retained as the reference
/// transform; [`forward_dft_half`] uses it only for non-power-of-two lengths.
pub(crate) fn forward_dft_half_naive(signal: &[f64]) -> Result<Vec<ComplexBin>, Error> {
    let n = signal.len();
    if n == 0 {
        return Err(Error::InvalidInput(
            "psychoacoustic DFT input must be non-empty",
        ));
    }
    let bins = n / 2 + 1;
    let scale = std::f64::consts::TAU / n as f64;
    let mut out = Vec::with_capacity(bins);
    for k in 0..bins {
        let mut re = 0.0_f64;
        let mut im = 0.0_f64;
        for (t, &sample) in signal.iter().enumerate() {
            let angle = scale * k as f64 * t as f64;
            re += sample * angle.cos();
            im -= sample * angle.sin();
        }
        out.push(ComplexBin { re, im });
    }
    Ok(out)
}

/// Computes the lower half-spectrum (`0..=N/2`) of a real signal, returning one
/// [`ComplexBin`] per retained bin.
///
/// Only the non-redundant bins of a real input are returned (`N/2 + 1` of them);
/// the remaining bins are conjugate mirrors. A radix-2 FFT is used when the
/// length is a power of two and a direct DFT otherwise. The signal must be
/// non-empty.
pub fn forward_dft_half(signal: &[f64]) -> Result<Vec<ComplexBin>, Error> {
    let n = signal.len();
    if n == 0 {
        return Err(Error::InvalidInput(
            "psychoacoustic DFT input must be non-empty",
        ));
    }
    if !is_power_of_two(n) {
        return forward_dft_half_naive(signal);
    }
    let mut re = signal.to_vec();
    let mut im = vec![0.0_f64; n];
    radix2_fft_in_place(&mut re, &mut im);
    let bins = n / 2 + 1;
    Ok(re
        .into_iter()
        .zip(im)
        .take(bins)
        .map(|(re, im)| ComplexBin { re, im })
        .collect())
}

/// Returns the per-bin energy (squared magnitude) of the half-spectrum of a real
/// signal.
pub fn power_spectrum(signal: &[f64]) -> Result<Vec<f64>, Error> {
    Ok(forward_dft_half(signal)?
        .into_iter()
        .map(ComplexBin::energy)
        .collect())
}
