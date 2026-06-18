//! Pure-Rust Vorbis encoder producing a standard, decoder-compatible stream.
//!
//! Unlike [`crate::stream`] (a self-contained proof format), this emits a real
//! Ogg Vorbis bitstream: the codebooks, floor1, residue, mapping and mode are
//! serialized into a spec setup header ([`crate::setup`]), and each audio packet
//! carries the spec framing (audio bit, mode, per-channel floor, then residue)
//! coded with the *same* codebook objects. A standard decoder (Symphonia, the
//! library's decode path) therefore decodes it. Derivative work of
//! libvorbis/aoTuV (BSD-3-Clause) via its components; see `LICENSE-THIRDPARTY`.
//!
//! Scope: all-short-block (single blocksize), independent channels (no coupling),
//! one floor and one residue config shared by every channel. The perceptual
//! analysis is [`crate::block::analyze_block`].
//!
//! The residue value book is built per stream: a first analysis pass histograms
//! the residue values, a Huffman book is fitted to that distribution (so the
//! common near-zero values get short codewords), and it is serialized into the
//! setup header. Because the book only changes codeword *lengths*, not the
//! quantization grid, the reconstruction is identical to a flat book — only the
//! bitrate drops.

use sc_core::{AudioBuffer, Error};

use crate::analysis::PsyAnalysis;
use crate::block::{analyze_block, FLOOR_MULT};
use crate::codebook::{huffman_lengths, Codebook, VqBook};
use crate::floor1::{
    encode_post_deviations, low_high_neighbors, Floor1Class, Floor1Encoding, Floor1FitInfo,
    Floor1Fitter,
};
use crate::header::{pack_comment_header, pack_identification_header};
use crate::ogg_mux::mux_vorbis;
use crate::oggpack::BitWriter;
use crate::residue::ResidueConfig;
use crate::setup::{
    float32_pack, Floor1Setup, Mapping0Setup, ModeSetup, ResidueSetup, SetupConfig, StaticCodebook,
};

/// Logical-stream serial number.
const STREAM_SERIAL: u32 = 0x736f_6e61; // "sona"
/// MDCT bins per block (block size is `2 * BLOCK_N` samples).
const BLOCK_N: usize = 128;
/// floor1 post-value ceiling, paired with [`FLOOR_MULT`].
const QUANT_Q: i32 = 64;
/// Residue partition size (samples per partition).
const GROUPING: usize = 16;
/// Number of floor1 partitions (each of dimension 4) over the interior posts.
const FLOOR_PARTITIONS: usize = 4;
/// The floor1 postlist (full, unsorted): the two endpoints (`0`, the bin count)
/// then 16 interior posts, log-spaced (denser at low bins). The dense grid lets
/// the floor track narrow tonal peaks so the residue stays near unity instead of
/// blowing past the value book's range.
const POSTLIST: [i32; 18] = [
    0, 128, 1, 2, 3, 4, 6, 8, 11, 15, 20, 27, 36, 48, 63, 82, 104, 120,
];

/// Global codebook indices (the order they are serialized in the setup header).
const BOOK_GROUP: usize = 0; // residue classification book (2 entries)
const BOOK_FLOOR_CLASS: usize = 1; // floor1 class book (16 entries)
const BOOK_FLOOR_VALUE: usize = 2; // floor1 value book (64 entries)
const BOOK_RES_VALUE: usize = 3; // residue value book (128-entry VQ)

/// Number of entries (quant levels) in the residue value book.
const RES_LEVELS: usize = 128;
/// The value book entry that dequantizes to `0.0` (`round((0 + 16)/0.25)`).
const RES_ZERO_ENTRY: usize = 64;

/// A complete uniform Huffman book: `1 << len` entries, every codeword `len`
/// bits.
fn complete_book(len: u8) -> Codebook {
    Codebook::new(vec![len; 1usize << len]).expect("complete book")
}

/// The signed quant grid of the residue value book: `value_vector` applies
/// `|quantlist|`, so the non-negative quantlist `[0, 127]` plus `mindel = -16`
/// and `delta = 0.25` spans `[-16, 16)` in steps of `0.25`.
fn residue_quantlist() -> Vec<f32> {
    (0..RES_LEVELS as i32).map(|i| i as f32).collect()
}

/// The residue book entry nearest `value` on the quant grid — the closed form
/// of `VqBook::best_entry` for this uniform scalar lattice (`round((v + 16)/0.25)`
/// clamped to a valid entry). Used to histogram residue values without building
/// the book first.
fn residue_entry(value: f32) -> usize {
    let idx = (value * 4.0 + 64.0).round() as i32;
    idx.clamp(0, RES_LEVELS as i32 - 1) as usize
}

/// Zeros residue values that quantize to the book's zero entry, so a partition
/// that codes nothing but zeros is detected exactly (`max == 0`) and can be
/// skipped. This does not change the reconstruction: a value that already snaps
/// to the zero entry codes as `0.0` whether it is skipped or coded.
fn snap_residue(residue: &mut [f32]) {
    for v in residue.iter_mut() {
        if residue_entry(*v) == RES_ZERO_ENTRY {
            *v = 0.0;
        }
    }
}

/// Builds the residue value VQ book from a codeword-length list (one length per
/// quant level). Returns `None` if the lengths do not form a valid tree.
fn residue_value_book(lengths: &[u8]) -> Option<VqBook> {
    let book = Codebook::new(lengths.to_vec())?;
    Some(VqBook::new(
        book,
        1,
        -16.0,
        0.25,
        false,
        residue_quantlist(),
    ))
}

/// Fits the residue value book's codeword lengths to a histogram of chosen
/// entries. Every level is kept usable (frequency floored to at least 1) so the
/// book stays complete and `best_entry` can pick any grid point; the floor also
/// bounds the frequency ratio, keeping codewords within the 5-bit length field.
/// Falls back to a flat 7-bit book if the fit ever fails to form a valid tree.
fn fit_residue_lengths(counts: &[u64; RES_LEVELS]) -> Vec<u8> {
    let max_count = counts.iter().copied().max().unwrap_or(0);
    // Floor the rarest level so max/min frequency ratio stays <= 2^16; this caps
    // the longest Huffman codeword well under the 32-bit length-field limit.
    let floor = (max_count >> 16).max(1);
    let freqs: Vec<u64> = counts.iter().map(|&c| c.max(floor)).collect();
    let lengths = huffman_lengths(&freqs);
    let valid = lengths.len() == RES_LEVELS
        && lengths.iter().all(|&l| (1..=32).contains(&l))
        && residue_value_book(&lengths).is_some();
    if valid {
        lengths
    } else {
        vec![7u8; RES_LEVELS]
    }
}

/// One channel's contribution to a block: the floor post deviations to pack and
/// the residue values to code.
struct ChannelPlan {
    dev: Vec<i32>,
    residue: Vec<f32>,
}

/// A pure-Rust Vorbis encoder for one channel layout / sample rate.
pub struct VorbisEncoder {
    channels: u16,
    psy: PsyAnalysis,
    fitter: Floor1Fitter,
    floor: Floor1Encoding,
    id_bytes: Vec<u8>,
    comment_bytes: Vec<u8>,
}

impl VorbisEncoder {
    /// Builds the encoder for `channels` at `sample_rate` Hz.
    #[must_use]
    pub fn new(channels: u16, sample_rate: u32) -> Self {
        let n = BLOCK_N;
        let m = (2 * n) as u32;
        let psy = PsyAnalysis::new(n, sample_rate);
        let fitter = Floor1Fitter::new(POSTLIST.to_vec(), Floor1FitInfo::standard());

        // Packet-side floor coder: books indexed by global codebook number.
        let floor = Floor1Encoding {
            quant_q: QUANT_Q,
            // `FLOOR_PARTITIONS` partitions of dimension 4 cover the interior posts.
            partition_class: vec![0; FLOOR_PARTITIONS],
            classes: vec![Floor1Class {
                dim: 4,
                subs: 1,
                book: BOOK_FLOOR_CLASS,
                subbook: vec![-1, BOOK_FLOOR_VALUE as i32],
            }],
            // Index 0 is unused by the floor (it only references 1 and 2); a
            // small valid book keeps the indices aligned with the global list.
            books: vec![complete_book(1), complete_book(4), complete_book(6)],
        };

        let id_bytes = pack_identification_header(channels as u8, sample_rate, 0, 0, 0, m, m);
        let comment_bytes = pack_comment_header(b"sonare-codec", &[]);

        Self {
            channels,
            psy,
            fitter,
            floor,
            id_bytes,
            comment_bytes,
        }
    }

    /// Builds the packet-side residue coder from the fitted value-book lengths.
    /// Two partition types: type 0 codes nothing (empty partitions whose snapped
    /// residue is all zero), type 1 codes through the value book. An all-zero
    /// partition therefore costs only its phrase word, not 16 coded values.
    fn build_residue(res_lengths: &[u8]) -> ResidueConfig {
        let value_book = residue_value_book(res_lengths).unwrap_or_else(|| {
            VqBook::new(complete_book(7), 1, -16.0, 0.25, false, residue_quantlist())
        });
        ResidueConfig {
            begin: 0,
            end: BLOCK_N,
            grouping: GROUPING,
            partitions: 2,
            partitions_per_word: 1,
            stages: 1,
            // Type 0: no coded stages (skip). Type 1: stage 0 codes values.
            secondstages: vec![0, 1],
            partbooks: vec![vec![None], vec![Some(value_book)]],
            // A snapped all-zero partition has max == 0 -> type 0 (skip);
            // anything else falls through to type 1 (coded).
            classmetric1: vec![0, 0],
            classmetric2: vec![-1, -1],
            phrasebook: complete_book(1), // 2 entries, codes the partition type
        }
    }

    /// Assembles the spec setup configuration (the serialized counterpart of the
    /// packet-side floor/residue coders), with the residue value book carrying
    /// the per-stream fitted codeword lengths.
    fn build_setup(channels: u16, res_lengths: &[u8]) -> SetupConfig {
        let codebooks = vec![
            // 0: residue classification book (2 entries, length-1 each).
            StaticCodebook {
                dim: 1,
                entries: 2,
                lengthlist: vec![1, 1],
                maptype: 0,
                q_min: 0,
                q_delta: 0,
                q_quant: 0,
                q_sequencep: false,
                quantlist: vec![],
            },
            // 1: floor1 class book (16 entries).
            StaticCodebook {
                dim: 1,
                entries: 16,
                lengthlist: vec![4; 16],
                maptype: 0,
                q_min: 0,
                q_delta: 0,
                q_quant: 0,
                q_sequencep: false,
                quantlist: vec![],
            },
            // 2: floor1 value book (64 entries).
            StaticCodebook {
                dim: 1,
                entries: 64,
                lengthlist: vec![6; 64],
                maptype: 0,
                q_min: 0,
                q_delta: 0,
                q_quant: 0,
                q_sequencep: false,
                quantlist: vec![],
            },
            // 3: residue value VQ book (128 entries, maptype 1). The codeword
            // lengths are fitted to the stream's residue distribution.
            StaticCodebook {
                dim: 1,
                entries: RES_LEVELS as u32,
                lengthlist: res_lengths.to_vec(),
                maptype: 1,
                q_min: float32_pack(-16.0),
                q_delta: float32_pack(0.25),
                q_quant: 7,
                q_sequencep: false,
                quantlist: (0..RES_LEVELS as i32).collect(),
            },
        ];

        let floor = Floor1Setup {
            partition_class: vec![0; FLOOR_PARTITIONS],
            class_dim: vec![4],
            class_subs: vec![1],
            class_book: vec![BOOK_FLOOR_CLASS as u8],
            class_subbook: vec![vec![-1, BOOK_FLOOR_VALUE as i32]],
            mult: FLOOR_MULT as u8,
            postlist: POSTLIST.iter().map(|&p| p as u32).collect(),
        };

        let residue = ResidueSetup {
            residue_type: 1,
            begin: 0,
            end: BLOCK_N as u32,
            grouping: GROUPING as u32,
            groupbook: BOOK_GROUP as u8,
            // Type 0 codes nothing; type 1 codes one stage through the value book
            // (the booklist names only the set-bit stages).
            secondstages: vec![0, 1],
            booklist: vec![BOOK_RES_VALUE as u8],
        };

        let mapping = Mapping0Setup {
            submaps: 1,
            coupling_mag: vec![],
            coupling_ang: vec![],
            chmuxlist: vec![0; channels as usize],
            floorsubmap: vec![0],
            residuesubmap: vec![0],
        };

        SetupConfig {
            channels,
            codebooks,
            floors: vec![(1, floor)],
            residues: vec![residue],
            mappings: vec![(0, mapping)],
            modes: vec![ModeSetup {
                blockflag: false,
                windowtype: 0,
                transformtype: 0,
                mapping: 0,
            }],
        }
    }

    /// Encodes interleaved PCM into a complete Ogg Vorbis byte stream.
    #[must_use]
    pub fn encode(&self, pcm: &AudioBuffer) -> Vec<u8> {
        let channel_count = usize::from(self.channels);
        let n = BLOCK_N;
        let m = 2 * n;
        let half = n;
        let frames = pcm.frames();

        // De-interleave, with a half-block of priming pad in front and a tail pad
        // out to a whole number of hops (so every sample gets an overlap partner).
        let needed = frames + 2 * half;
        let padded_len = needed.div_ceil(half) * half;
        let mut planar = vec![vec![0.0f32; padded_len]; channel_count];
        for (f, frame) in pcm.samples.chunks_exact(channel_count).enumerate() {
            for (ch, &sample) in frame.iter().enumerate() {
                planar[ch][half + f] = sample;
            }
        }

        // Pass 1: analyze every block, building the per-channel plans and a
        // histogram of the residue values the value book must code.
        let (lo, hi) = low_high_neighbors(&POSTLIST);
        let mut plans: Vec<Vec<Option<ChannelPlan>>> = Vec::new();
        let mut counts = [0u64; RES_LEVELS];
        let mut pos = 0usize;
        while pos + m <= padded_len {
            let mut plan: Vec<Option<ChannelPlan>> = Vec::with_capacity(channel_count);
            for ch in &planar {
                let frame = &ch[pos..pos + m];
                match analyze_block(&self.psy, &self.fitter, &POSTLIST, frame) {
                    Some(block) => {
                        let mut posts = block.posts.clone();
                        let dev = encode_post_deviations(&POSTLIST, &mut posts, &lo, &hi, QUANT_Q);
                        let mut residue = block.residue;
                        snap_residue(&mut residue);
                        // Only non-empty partitions are coded, so only they
                        // contribute to the value book's histogram.
                        for part in residue.chunks_exact(GROUPING) {
                            if part.iter().any(|&v| v != 0.0) {
                                for &v in part {
                                    counts[residue_entry(v)] += 1;
                                }
                            }
                        }
                        plan.push(Some(ChannelPlan { dev, residue }));
                    }
                    None => plan.push(None),
                }
            }
            plans.push(plan);
            pos += half;
        }

        // Fit the residue value book to the histogram, then serialize the setup
        // header and build the matching packet-side residue coder.
        let res_lengths = fit_residue_lengths(&counts);
        let setup_bytes = Self::build_setup(self.channels, &res_lengths).pack();
        let residue = Self::build_residue(&res_lengths);

        // Pass 2: emit each block's audio packet with the fitted coder.
        let mut audio: Vec<(Vec<u8>, u64)> = Vec::with_capacity(plans.len());
        let mut granule = 0u64;
        for plan in &plans {
            granule += half as u64;
            audio.push((self.write_packet(plan, &residue), granule));
        }

        mux_vorbis(
            STREAM_SERIAL,
            &self.id_bytes,
            &self.comment_bytes,
            &setup_bytes,
            &audio,
        )
    }

    /// Writes one block's audio packet from its per-channel plans: the
    /// audio-packet bit, then per-channel floor1, then the residue of the
    /// channels whose floor is present.
    fn write_packet(&self, plan: &[Option<ChannelPlan>], residue: &ResidueConfig) -> Vec<u8> {
        let mut w = BitWriter::new();
        w.write(0, 1); // audio packet (not a header)
                       // One mode (blockflag 0): mode number is 0 bits, no long-block window
                       // flags follow.

        let mut active: Vec<Vec<f32>> = Vec::new();
        for channel in plan {
            match channel {
                Some(cp) => {
                    self.floor.pack(&cp.dev, &mut w);
                    active.push(cp.residue.clone());
                }
                None => {
                    // Floor unused for this channel: clear the floor's present
                    // flag; this channel's residue is not coded.
                    w.write(0, 1);
                }
            }
        }

        // The submap codes the residue of exactly the present-floor channels.
        if !active.is_empty() {
            residue.encode(&active, &mut w);
        }

        w.into_bytes()
    }
}

/// Encodes interleaved PCM into an Ogg Vorbis stream, validating the layout.
pub fn encode(pcm: &AudioBuffer) -> Result<Vec<u8>, Error> {
    if pcm.sample_rate == 0 {
        return Err(Error::InvalidPcm("sample rate must be non-zero"));
    }
    if pcm.channels == 0 || pcm.channels > 255 {
        return Err(Error::InvalidPcm("unsupported Vorbis channel count"));
    }
    let encoder = VorbisEncoder::new(pcm.channels, pcm.sample_rate);
    Ok(encoder.encode(pcm))
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
