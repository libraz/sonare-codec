#![deny(unsafe_code)]
#![warn(clippy::all)]

use thiserror::Error;

/// Interleaved PCM samples normalized to `[-1.0, 1.0]`.
#[derive(Clone, Debug, PartialEq)]
pub struct AudioBuffer {
    pub sample_rate: u32,
    pub channels: u16,
    pub samples: Vec<f32>,
}

impl AudioBuffer {
    /// Creates an audio buffer after checking basic shape invariants.
    pub fn new(sample_rate: u32, channels: u16, samples: Vec<f32>) -> Result<Self, Error> {
        if sample_rate == 0 {
            return Err(Error::InvalidPcm("sample rate must be non-zero"));
        }
        if channels == 0 {
            return Err(Error::InvalidPcm("channel count must be non-zero"));
        }
        let channels = usize::from(channels);
        if samples.len() % channels != 0 {
            return Err(Error::InvalidPcm(
                "interleaved sample count must be divisible by channels",
            ));
        }

        Ok(Self {
            sample_rate,
            channels: u16::try_from(channels)
                .map_err(|_| Error::InvalidPcm("too many channels"))?,
            samples,
        })
    }

    /// Returns the number of interleaved frames in the buffer.
    #[must_use]
    pub fn frames(&self) -> usize {
        self.samples.len() / usize::from(self.channels)
    }

    /// Returns true when the buffer contains no PCM frames.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }

    /// Extracts one channel as a fixed-size block, zero-padding past EOF.
    pub fn channel_block(
        &self,
        channel: usize,
        start_frame: usize,
        frames: usize,
    ) -> Result<Vec<f32>, Error> {
        if channel >= usize::from(self.channels) {
            return Err(Error::InvalidPcm("channel index out of range"));
        }

        let channels = usize::from(self.channels);
        let available_frames = self.frames().saturating_sub(start_frame);
        let copied_frames = frames.min(available_frames);
        let mut out = vec![0.0; frames];
        for (out_frame, sample) in out.iter_mut().take(copied_frames).enumerate() {
            let input_frame = start_frame + out_frame;
            let sample_index = input_frame
                .checked_mul(channels)
                .and_then(|index| index.checked_add(channel))
                .ok_or(Error::InvalidInput("PCM sample index overflow"))?;
            *sample = *self
                .samples
                .get(sample_index)
                .ok_or(Error::InvalidPcm("PCM sample is missing"))?;
        }
        Ok(out)
    }
}

/// Summary of numeric difference between two PCM buffers.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PcmDiff {
    pub max_abs: f32,
    pub rms: f32,
    pub snr_db: Option<f32>,
}

/// Numeric tolerance used to validate lossy or near-lossless PCM round-trips.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PcmTolerance {
    pub max_abs: f32,
    pub rms: f32,
    pub min_snr_db: Option<f32>,
}

impl PcmTolerance {
    #[must_use]
    pub fn new(max_abs: f32, rms: f32, min_snr_db: Option<f32>) -> Self {
        Self {
            max_abs,
            rms,
            min_snr_db,
        }
    }
}

/// Compares two PCM buffers with identical shape.
pub fn compare_pcm(actual: &AudioBuffer, expected: &AudioBuffer) -> Result<PcmDiff, Error> {
    if actual.sample_rate != expected.sample_rate {
        return Err(Error::InvalidPcm("sample rates differ"));
    }
    if actual.channels != expected.channels {
        return Err(Error::InvalidPcm("channel counts differ"));
    }
    if actual.samples.len() != expected.samples.len() {
        return Err(Error::InvalidPcm("sample counts differ"));
    }

    let mut max_abs = 0.0_f32;
    let mut error_power = 0.0_f64;
    let mut signal_power = 0.0_f64;

    for (&actual, &expected) in actual.samples.iter().zip(&expected.samples) {
        let diff = actual - expected;
        max_abs = max_abs.max(diff.abs());
        error_power += f64::from(diff * diff);
        signal_power += f64::from(expected * expected);
    }

    let count = actual.samples.len();
    let rms = if count == 0 {
        0.0
    } else {
        (error_power / count as f64).sqrt() as f32
    };
    let snr_db = if error_power == 0.0 {
        Some(f32::INFINITY)
    } else if signal_power == 0.0 {
        None
    } else {
        Some((10.0 * (signal_power / error_power).log10()) as f32)
    };

    Ok(PcmDiff {
        max_abs,
        rms,
        snr_db,
    })
}

/// Compares two PCM buffers and validates the result against a tolerance.
pub fn compare_pcm_with_tolerance(
    actual: &AudioBuffer,
    expected: &AudioBuffer,
    tolerance: PcmTolerance,
) -> Result<PcmDiff, Error> {
    if !tolerance.max_abs.is_finite() || tolerance.max_abs < 0.0 {
        return Err(Error::InvalidInput("PCM max_abs tolerance must be finite"));
    }
    if !tolerance.rms.is_finite() || tolerance.rms < 0.0 {
        return Err(Error::InvalidInput("PCM RMS tolerance must be finite"));
    }
    if let Some(min_snr_db) = tolerance.min_snr_db {
        if !min_snr_db.is_finite() {
            return Err(Error::InvalidInput("PCM SNR tolerance must be finite"));
        }
    }

    let diff = compare_pcm(actual, expected)?;
    if diff.max_abs > tolerance.max_abs {
        return Err(Error::InvalidInput("PCM max_abs exceeds tolerance"));
    }
    if diff.rms > tolerance.rms {
        return Err(Error::InvalidInput("PCM RMS exceeds tolerance"));
    }
    if let Some(min_snr_db) = tolerance.min_snr_db {
        let snr_db = diff
            .snr_db
            .ok_or(Error::InvalidInput("PCM SNR is undefined"))?;
        if snr_db < min_snr_db {
            return Err(Error::InvalidInput("PCM SNR is below tolerance"));
        }
    }

    Ok(diff)
}

/// Builds the half-sine window commonly used before MDCT analysis.
pub fn sine_window(len: usize) -> Result<Vec<f32>, Error> {
    if len == 0 {
        return Err(Error::InvalidInput("window length must be non-zero"));
    }

    let len = len as f64;
    Ok((0..len as usize)
        .map(|index| ((std::f64::consts::PI / len) * (index as f64 + 0.5)).sin() as f32)
        .collect())
}

/// Applies an analysis window to a block of samples.
pub fn apply_window(samples: &[f32], window: &[f32]) -> Result<Vec<f32>, Error> {
    if samples.len() != window.len() {
        return Err(Error::InvalidInput("window length must match sample block"));
    }

    Ok(samples
        .iter()
        .zip(window)
        .map(|(&sample, &scale)| sample * scale)
        .collect())
}

/// Computes a direct MDCT for a 2N-sample block, returning N coefficients.
///
/// This intentionally favors clarity over speed. Codec crates can replace this
/// with a faster factorized implementation while preserving the same math.
pub fn mdct(block: &[f32]) -> Result<Vec<f32>, Error> {
    if block.is_empty() || block.len() % 2 != 0 {
        return Err(Error::InvalidInput(
            "MDCT input length must be a non-zero even number",
        ));
    }

    let coeffs = block.len() / 2;
    let coeffs_f64 = coeffs as f64;
    let scale = std::f64::consts::PI / coeffs_f64;
    let half_shift = coeffs_f64 / 2.0;
    let mut out = Vec::with_capacity(coeffs);
    for k in 0..coeffs {
        let k_term = k as f64 + 0.5;
        let mut sum = 0.0_f64;
        for (n, &sample) in block.iter().enumerate() {
            let n_term = n as f64 + 0.5 + half_shift;
            sum += f64::from(sample) * (scale * n_term * k_term).cos();
        }
        out.push(sum as f32);
    }
    Ok(out)
}

/// Quantizes MDCT-like spectral coefficients using a signed power-law curve.
///
/// AAC-LC and MP3 Layer III both use signed magnitude quantization based on a
/// 3/4 power curve before entropy coding. The caller supplies the quantization
/// step and output bound used by the target codec path.
pub fn quantize_spectrum(coeffs: &[f32], step: f32, max_abs_value: i32) -> Result<Vec<i32>, Error> {
    if !step.is_finite() || step <= 0.0 {
        return Err(Error::InvalidInput("quantization step must be positive"));
    }
    if max_abs_value <= 0 {
        return Err(Error::InvalidInput("quantization bound must be positive"));
    }

    let mut out = Vec::with_capacity(coeffs.len());
    for &coeff in coeffs {
        if !coeff.is_finite() {
            return Err(Error::InvalidInput("spectral coefficient must be finite"));
        }

        let magnitude = (coeff.abs().powf(0.75) / step).round();
        if magnitude > max_abs_value as f32 {
            return Err(Error::InvalidInput(
                "quantized spectral coefficient exceeds bound",
            ));
        }
        let quantized = magnitude as i32;
        out.push(if coeff.is_sign_negative() {
            -quantized
        } else {
            quantized
        });
    }
    Ok(out)
}

/// Big-endian bit reader for codec bitstreams.
#[derive(Clone, Debug)]
pub struct BitReader<'a> {
    input: &'a [u8],
    bit_pos: usize,
}

impl<'a> BitReader<'a> {
    #[must_use]
    pub fn new(input: &'a [u8]) -> Self {
        Self { input, bit_pos: 0 }
    }

    #[must_use]
    pub fn bit_pos(&self) -> usize {
        self.bit_pos
    }

    #[must_use]
    pub fn byte_pos(&self) -> usize {
        self.bit_pos.div_ceil(8)
    }

    #[must_use]
    pub fn remaining_bits(&self) -> usize {
        // Saturating: on 32-bit targets (wasm) `len() * 8` would overflow usize
        // for inputs > ~512 MB, panicking in debug and wrapping in release.
        self.input
            .len()
            .saturating_mul(8)
            .saturating_sub(self.bit_pos)
    }

    pub fn read_bool(&mut self) -> Result<bool, Error> {
        Ok(self.read_bits(1)? != 0)
    }

    pub fn read_bits(&mut self, count: u8) -> Result<u32, Error> {
        if count > 32 {
            return Err(Error::InvalidInput("cannot read more than 32 bits at once"));
        }
        if self.remaining_bits() < usize::from(count) {
            return Err(Error::InvalidInput("bitstream is truncated"));
        }

        let mut value = 0_u32;
        for _ in 0..count {
            let byte_index = self.bit_pos / 8;
            let bit_index = 7 - (self.bit_pos % 8);
            let byte = self.input[byte_index];
            value = (value << 1) | u32::from((byte >> bit_index) & 1);
            self.bit_pos += 1;
        }

        Ok(value)
    }

    pub fn read_signed_bits(&mut self, count: u8) -> Result<i32, Error> {
        if count == 0 || count > 32 {
            return Err(Error::InvalidInput("invalid signed bit width"));
        }
        let raw = self.read_bits(count)?;
        if count == 32 {
            return Ok(raw as i32);
        }

        let sign_bit = 1_u32 << (count - 1);
        if raw & sign_bit == 0 {
            Ok(raw as i32)
        } else {
            Ok((raw as i32) - (1_i32 << count))
        }
    }

    pub fn read_unary_zeros(&mut self) -> Result<u32, Error> {
        let mut zeros = 0_u32;
        loop {
            if self.remaining_bits() == 0 {
                return Err(Error::InvalidInput("unary code is truncated"));
            }
            if self.read_bool()? {
                return Ok(zeros);
            }
            zeros = zeros
                .checked_add(1)
                .ok_or(Error::InvalidInput("unary code overflow"))?;
        }
    }

    pub fn byte_align(&mut self) {
        self.bit_pos = self.bit_pos.div_ceil(8) * 8;
    }
}

#[derive(Clone, Debug, Default)]
pub struct BitWriter {
    out: Vec<u8>,
    bit_pos: u8,
}

impl BitWriter {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn bit_len(&self) -> usize {
        self.out.len() * 8
            - usize::from(if self.bit_pos == 0 {
                0
            } else {
                8 - self.bit_pos
            })
    }

    pub fn write_bits(&mut self, value: u32, count: u8) -> Result<(), Error> {
        if count > 32 {
            return Err(Error::InvalidInput(
                "cannot write more than 32 bits at once",
            ));
        }
        if count < 32 && value >= (1_u32 << count) {
            return Err(Error::InvalidInput("bit value exceeds width"));
        }

        for shift in (0..count).rev() {
            if self.bit_pos == 0 {
                self.out.push(0);
            }
            let bit = ((value >> shift) & 1) as u8;
            let byte_index = self.out.len() - 1;
            self.out[byte_index] |= bit << (7 - self.bit_pos);
            self.bit_pos = (self.bit_pos + 1) % 8;
        }
        Ok(())
    }

    pub fn write_bytes(&mut self, bytes: &[u8]) -> Result<(), Error> {
        if self.bit_pos == 0 {
            self.out.extend_from_slice(bytes);
            return Ok(());
        }

        for &byte in bytes {
            self.write_bits(u32::from(byte), 8)?;
        }
        Ok(())
    }

    #[must_use]
    pub fn finish_byte_aligned(self) -> Vec<u8> {
        self.out
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HuffmanCode {
    pub bits: u32,
    pub len: u8,
}

impl HuffmanCode {
    pub fn new(bits: u32, len: u8) -> Result<Self, Error> {
        if len == 0 || len > 32 {
            return Err(Error::InvalidInput("invalid Huffman code length"));
        }
        if len < 32 && bits >= (1_u32 << len) {
            return Err(Error::InvalidInput("Huffman code bits exceed length"));
        }
        Ok(Self { bits, len })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackedBits {
    pub bytes: Vec<u8>,
    pub bit_len: usize,
}

/// Appends only the meaningful bits from a packed bit buffer.
pub fn write_packed_bits(writer: &mut BitWriter, bits: &PackedBits) -> Result<(), Error> {
    if bits.bit_len > bits.bytes.len() * 8 {
        return Err(Error::InvalidInput("packed bit length exceeds byte buffer"));
    }

    let mut bit_index = 0;
    if writer.bit_pos == 0 {
        let full_bytes = bits.bit_len / 8;
        if full_bytes > 0 {
            writer.write_bytes(&bits.bytes[..full_bytes])?;
            bit_index = full_bytes * 8;
        }
    }

    while bit_index < bits.bit_len {
        let byte = bits.bytes[bit_index / 8];
        let bit = (byte >> (7 - (bit_index % 8))) & 1;
        writer.write_bits(u32::from(bit), 1)?;
        bit_index += 1;
    }
    Ok(())
}

/// Concatenates byte-padded bit buffers while preserving exact bit lengths.
pub fn concat_packed_bits(parts: &[PackedBits]) -> Result<PackedBits, Error> {
    let mut writer = BitWriter::new();
    for part in parts {
        write_packed_bits(&mut writer, part)?;
    }
    let bit_len = writer.bit_len();
    Ok(PackedBits {
        bytes: writer.finish_byte_aligned(),
        bit_len,
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HuffmanEntry<T> {
    pub symbol: T,
    pub code: HuffmanCode,
}

/// Packs a sequence of already-selected Huffman codewords MSB-first.
pub fn pack_huffman_codes_with_len(codes: &[HuffmanCode]) -> Result<PackedBits, Error> {
    let mut writer = BitWriter::new();
    for code in codes {
        writer.write_bits(code.bits, code.len)?;
    }
    let bit_len = writer.bit_len();
    Ok(PackedBits {
        bytes: writer.finish_byte_aligned(),
        bit_len,
    })
}

/// Packs a sequence of already-selected Huffman codewords MSB-first.
pub fn pack_huffman_codes(codes: &[HuffmanCode]) -> Result<Vec<u8>, Error> {
    Ok(pack_huffman_codes_with_len(codes)?.bytes)
}

/// Looks up a symbol in a caller-supplied Huffman table.
pub fn lookup_huffman_code<T: PartialEq>(
    table: &[HuffmanEntry<T>],
    symbol: &T,
) -> Result<HuffmanCode, Error> {
    table
        .iter()
        .find(|entry| &entry.symbol == symbol)
        .map(|entry| entry.code)
        .ok_or(Error::InvalidInput("Huffman symbol is not in table"))
}

/// Maps symbols through a caller-supplied Huffman table and packs the codewords.
pub fn pack_huffman_symbols_with_len<T: PartialEq>(
    symbols: &[T],
    table: &[HuffmanEntry<T>],
) -> Result<PackedBits, Error> {
    let mut writer = BitWriter::new();
    for symbol in symbols {
        let code = lookup_huffman_code(table, symbol)?;
        writer.write_bits(code.bits, code.len)?;
    }
    let bit_len = writer.bit_len();
    Ok(PackedBits {
        bytes: writer.finish_byte_aligned(),
        bit_len,
    })
}

/// Supported container or codec formats.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Format {
    Wav,
    Flac,
    Mp3,
    Vorbis,
    Opus,
    Aac,
}

/// Common decode interface.
pub trait Decoder {
    fn decode(&mut self, input: &[u8]) -> Result<AudioBuffer, Error>;

    fn decode_stream(&mut self, chunk: &[u8]) -> Result<Option<AudioBuffer>, Error>;
}

/// Common encode interface.
pub trait Encoder {
    fn encode(&mut self, pcm: &AudioBuffer) -> Result<Vec<u8>, Error>;
}

/// Error type shared across codecs and bindings.
#[derive(Debug, Error)]
pub enum Error {
    #[error("unsupported format")]
    UnsupportedFormat,
    #[error("invalid PCM buffer: {0}")]
    InvalidPcm(&'static str),
    #[error("invalid input: {0}")]
    InvalidInput(&'static str),
    #[error("unsupported codec feature: {0}")]
    UnsupportedFeature(&'static str),
    /// The input ended before a complete decodable unit was available. A
    /// streaming caller should buffer more data and retry; a one-shot caller
    /// should treat it as a truncated stream. This is distinct from
    /// [`Error::InvalidInput`], which marks input that is malformed rather than
    /// merely incomplete.
    #[error("incomplete input: more data required")]
    Incomplete,
}

/// Detects a supported audio format from leading bytes.
#[must_use]
pub fn detect(input: &[u8]) -> Option<Format> {
    if input.len() >= 12 && input.get(0..4) == Some(b"RIFF") && input.get(8..12) == Some(b"WAVE") {
        return Some(Format::Wav);
    }
    if input.get(0..4) == Some(b"fLaC") {
        return Some(Format::Flac);
    }
    if input.get(0..4) == Some(b"OggS") {
        if contains(input, b"OpusHead") {
            return Some(Format::Opus);
        }
        if contains(input, b"vorbis") {
            return Some(Format::Vorbis);
        }
        return None;
    }
    if input.len() >= 12 && is_mp4_brand(input.get(4..12)) {
        return Some(Format::Aac);
    }
    if has_adts_sync(input) {
        return Some(Format::Aac);
    }
    if input.get(0..3) == Some(b"ID3") || has_mp3_frame_sync(input) {
        return Some(Format::Mp3);
    }
    None
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

fn is_mp4_brand(box_header: Option<&[u8]>) -> bool {
    let Some(box_header) = box_header else {
        return false;
    };
    box_header.get(0..4) == Some(b"ftyp")
        && matches!(
            box_header.get(4..8),
            Some(b"M4A ") | Some(b"mp42") | Some(b"isom") | Some(b"iso2")
        )
}

fn has_mp3_frame_sync(input: &[u8]) -> bool {
    let Some((&first, rest)) = input.split_first() else {
        return false;
    };
    let Some(&second) = rest.first() else {
        return false;
    };
    first == 0xff && (second & 0xe0) == 0xe0
}

fn has_adts_sync(input: &[u8]) -> bool {
    let Some((&first, rest)) = input.split_first() else {
        return false;
    };
    let Some(&second) = rest.first() else {
        return false;
    };
    first == 0xff && (second & 0xf6) == 0xf0
}

#[cfg(test)]
mod tests {
    use super::{
        apply_window, compare_pcm, compare_pcm_with_tolerance, concat_packed_bits, detect, mdct,
        pack_huffman_codes, pack_huffman_codes_with_len, pack_huffman_symbols_with_len,
        quantize_spectrum, sine_window, AudioBuffer, BitReader, BitWriter, Format, HuffmanCode,
        HuffmanEntry, PackedBits, PcmTolerance,
    };

    #[test]
    fn detects_wav() {
        assert_eq!(detect(b"RIFF\x24\0\0\0WAVEfmt "), Some(Format::Wav));
    }

    #[test]
    fn detects_codec_signatures() {
        assert_eq!(detect(b"fLaC\0\0\0\0"), Some(Format::Flac));
        assert_eq!(detect(b"ID3\x04\0\0\0\0\0\0"), Some(Format::Mp3));
        assert_eq!(detect(b"OggS\0\0\0OpusHead"), Some(Format::Opus));
        assert_eq!(detect(b"OggS\0\0\0\x01vorbis"), Some(Format::Vorbis));
        assert_eq!(detect(b"\0\0\0\x18ftypM4A "), Some(Format::Aac));
        assert_eq!(detect(&[0xff, 0xf1, 0x50, 0x80]), Some(Format::Aac));
    }

    #[test]
    fn validates_interleaved_shape() {
        assert!(AudioBuffer::new(48_000, 2, vec![0.0, 0.0, 1.0]).is_err());
    }

    #[test]
    fn extracts_zero_padded_channel_blocks() {
        let pcm = AudioBuffer::new(48_000, 2, vec![1.0, -1.0, 2.0, -2.0, 3.0, -3.0]).unwrap();

        assert_eq!(pcm.channel_block(0, 0, 4).unwrap(), [1.0, 2.0, 3.0, 0.0]);
        assert_eq!(pcm.channel_block(1, 1, 3).unwrap(), [-2.0, -3.0, 0.0]);
        assert_eq!(pcm.channel_block(0, 4, 2).unwrap(), [0.0, 0.0]);
        assert!(pcm.channel_block(2, 0, 1).is_err());
    }

    #[test]
    fn compares_pcm() {
        let actual = AudioBuffer::new(48_000, 1, vec![0.0, 0.5]).unwrap();
        let expected = AudioBuffer::new(48_000, 1, vec![0.0, 0.25]).unwrap();
        let diff = compare_pcm(&actual, &expected).unwrap();

        assert_eq!(diff.max_abs, 0.25);
        assert!(diff.rms > 0.0);
        assert!(diff.snr_db.is_some());
    }

    #[test]
    fn validates_pcm_tolerance() {
        let actual = AudioBuffer::new(48_000, 1, vec![0.0, 0.251, -0.249]).unwrap();
        let expected = AudioBuffer::new(48_000, 1, vec![0.0, 0.25, -0.25]).unwrap();
        let passing = PcmTolerance::new(0.002, 0.001, Some(45.0));

        let diff = compare_pcm_with_tolerance(&actual, &expected, passing).unwrap();

        assert!(diff.max_abs <= passing.max_abs);
        assert!(diff.rms <= passing.rms);
        assert!(diff.snr_db.unwrap() >= 45.0);

        assert!(compare_pcm_with_tolerance(
            &actual,
            &expected,
            PcmTolerance::new(0.0001, 1.0, None)
        )
        .is_err());
        assert!(compare_pcm_with_tolerance(
            &actual,
            &expected,
            PcmTolerance::new(1.0, 0.0001, None)
        )
        .is_err());
        assert!(compare_pcm_with_tolerance(
            &actual,
            &expected,
            PcmTolerance::new(1.0, 1.0, Some(80.0))
        )
        .is_err());
        assert!(compare_pcm_with_tolerance(
            &AudioBuffer::new(48_000, 1, vec![0.001]).unwrap(),
            &AudioBuffer::new(48_000, 1, vec![0.0]).unwrap(),
            PcmTolerance::new(1.0, 1.0, Some(0.0))
        )
        .is_err());
    }

    #[test]
    fn builds_sine_window() {
        let window = sine_window(4).unwrap();

        assert_eq!(window.len(), 4);
        assert!((window[0] - 0.382_683_43).abs() < 1.0e-6);
        assert!((window[1] - 0.923_879_5).abs() < 1.0e-6);
        assert_eq!(window[0], window[3]);
        assert_eq!(window[1], window[2]);
    }

    #[test]
    fn applies_window() {
        let samples = [1.0, -2.0, 3.0];
        let window = [0.5, 0.25, 0.0];

        assert_eq!(apply_window(&samples, &window).unwrap(), [0.5, -0.5, 0.0]);
        assert!(apply_window(&samples, &window[..2]).is_err());
    }

    #[test]
    fn computes_mdct_directly() {
        let coeffs = mdct(&[1.0, 0.0, 0.0, 0.0]).unwrap();

        assert_eq!(coeffs.len(), 2);
        assert!((coeffs[0] - 0.382_683_43).abs() < 1.0e-6);
        assert!((coeffs[1] + 0.923_879_5).abs() < 1.0e-6);
        assert_eq!(mdct(&[0.0; 8]).unwrap(), vec![0.0; 4]);
        assert!(mdct(&[]).is_err());
        assert!(mdct(&[1.0, 2.0, 3.0]).is_err());
    }

    #[test]
    fn quantizes_spectrum_with_power_law() {
        let coeffs = [-16.0, -1.0, 0.0, 1.0, 16.0];

        assert_eq!(
            quantize_spectrum(&coeffs, 1.0, 100).unwrap(),
            [-8, -1, 0, 1, 8]
        );
        assert_eq!(
            quantize_spectrum(&coeffs, 2.0, 100).unwrap(),
            [-4, -1, 0, 1, 4]
        );
        assert!(quantize_spectrum(&coeffs, 0.0, 100).is_err());
        assert!(quantize_spectrum(&coeffs, 1.0, 7).is_err());
        assert!(quantize_spectrum(&[f32::NAN], 1.0, 100).is_err());
    }

    #[test]
    fn reads_bits_big_endian() {
        let mut reader = BitReader::new(&[0b1010_0101, 0b1100_0000]);

        assert_eq!(reader.read_bits(4).unwrap(), 0b1010);
        assert!(!reader.read_bool().unwrap());
        assert_eq!(reader.read_bits(3).unwrap(), 0b101);
        assert_eq!(reader.read_signed_bits(4).unwrap(), -4);
        assert_eq!(reader.byte_pos(), 2);
    }

    #[test]
    fn remaining_bits_tracks_reads_without_overflow() {
        let mut reader = BitReader::new(&[0xff, 0xff]);
        assert_eq!(reader.remaining_bits(), 16);
        reader.read_bits(5).unwrap();
        assert_eq!(reader.remaining_bits(), 11);
        reader.read_bits(11).unwrap();
        assert_eq!(reader.remaining_bits(), 0);
        // Saturating math keeps remaining_bits well-defined (no panic/wrap) even
        // when the consumed position meets the end of the buffer.
        assert!(reader.read_bool().is_err());
        assert_eq!(reader.remaining_bits(), 0);
    }

    #[test]
    fn writes_bits_big_endian() {
        let mut writer = BitWriter::new();

        writer.write_bits(0b101, 3).unwrap();
        writer.write_bytes(&[0b0101_0101]).unwrap();

        assert_eq!(writer.bit_len(), 11);
        assert_eq!(writer.finish_byte_aligned(), &[0b1010_1010, 0b1010_0000]);
    }

    #[test]
    fn packs_huffman_codes() {
        let codes = [
            HuffmanCode::new(0b1, 1).unwrap(),
            HuffmanCode::new(0b010, 3).unwrap(),
            HuffmanCode::new(0b11, 2).unwrap(),
        ];

        assert_eq!(pack_huffman_codes(&codes).unwrap(), &[0b1010_1100]);
        assert_eq!(
            pack_huffman_codes_with_len(&codes).unwrap(),
            PackedBits {
                bytes: vec![0b1010_1100],
                bit_len: 6,
            }
        );
        assert!(HuffmanCode::new(0b100, 2).is_err());
        assert!(HuffmanCode::new(0, 0).is_err());
    }

    #[test]
    fn packs_huffman_symbols_from_table() {
        let table = [
            HuffmanEntry {
                symbol: 'a',
                code: HuffmanCode::new(0b0, 1).unwrap(),
            },
            HuffmanEntry {
                symbol: 'b',
                code: HuffmanCode::new(0b10, 2).unwrap(),
            },
            HuffmanEntry {
                symbol: 'c',
                code: HuffmanCode::new(0b11, 2).unwrap(),
            },
        ];

        assert_eq!(
            pack_huffman_symbols_with_len(&['b', 'a', 'c'], &table).unwrap(),
            PackedBits {
                bytes: vec![0b1001_1000],
                bit_len: 5,
            }
        );
        assert!(pack_huffman_symbols_with_len(&['d'], &table).is_err());
    }

    #[test]
    fn concatenates_packed_bits_without_padding_bits() {
        let parts = [
            PackedBits {
                bytes: vec![0b1010_0000],
                bit_len: 3,
            },
            PackedBits {
                bytes: vec![0b1100_0000],
                bit_len: 2,
            },
        ];

        assert_eq!(
            concat_packed_bits(&parts).unwrap(),
            PackedBits {
                bytes: vec![0b1011_1000],
                bit_len: 5,
            }
        );
        assert!(concat_packed_bits(&[PackedBits {
            bytes: vec![0],
            bit_len: 9,
        }])
        .is_err());
    }

    #[test]
    fn reads_unary_zeros() {
        let mut reader = BitReader::new(&[0b0010_0000]);

        assert_eq!(reader.read_unary_zeros().unwrap(), 2);
        assert_eq!(reader.bit_pos(), 3);
    }
}
