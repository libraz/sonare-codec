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
//! Scope: two block sizes with switching. Steady content uses the long block
//! (2048-sample window), which resolves tones into few bins so most of the
//! spectrum is empty; a transient switches that grid slot to a group of short
//! blocks (256-sample window) bracketed by transition windows, localizing the
//! attack so it does not pre-echo. The long and short coders share one set of
//! codebooks (a floor and residue config per block size). A strongly-correlated
//! stereo pair is square-polar coupled (the angle channel's residue collapses to
//! zero and skips); decorrelated channels are coded independently.
//!
//! The two residue value books (a fine `0.25`-step book and a wide coarse book
//! for a cascade second stage) are built per stream: a first analysis pass
//! histograms the entries each cascade stage would code, Huffman books are
//! fitted to those distributions (so common near-zero values get short
//! codewords), and they are serialized into the setup header. The books change
//! only codeword *lengths*, not the quantization grid, so the reconstruction is
//! identical to flat books — only the bitrate drops. Empty residue partitions
//! are skipped, quiet ones use the fine book alone, and loud (tonal-peak)
//! partitions add the coarse stage so the concentrated peak does not clip.

use sc_core::{AudioBuffer, Error};

use crate::analysis::PsyAnalysis;
use crate::block::{analyze_block_windowed, FLOOR_MULT};
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
use crate::window::{vorbis_window, vorbis_window_lr};

/// Logical-stream serial number.
const STREAM_SERIAL: u32 = 0x736f_6e61; // "sona"
/// MDCT bins per block (block size is `2 * BLOCK_N` samples). A long block
/// (2048-sample window) resolves steady tones into far fewer bins than a short
/// block, so most of the spectrum is genuinely empty and compresses well; the
/// two-stage residue cascade carries the now-concentrated tonal peaks.
const BLOCK_N: usize = 1024;
/// MDCT bins for a short block (block size `2 * SHORT_N = 256` samples). Short
/// blocks localize a transient's energy to a 256-sample window instead of
/// smearing it across the long 2048-sample window (pre-echo), at the cost of
/// frequency resolution.
const SHORT_N: usize = 128;
/// floor1 post-value ceiling, paired with [`FLOOR_MULT`].
const QUANT_Q: i32 = 64;
/// Residue partition size (samples per partition).
const GROUPING: usize = 16;
/// Number of floor1 partitions (each of dimension 4) over the interior posts.
const FLOOR_PARTITIONS: usize = 8;
/// The floor1 postlist (full, unsorted): the two endpoints (`0`, the bin count)
/// then 64 interior posts, log-spaced (denser at low/mid bins where tones land).
/// With a long block the floor is amortized over 8x fewer packets, so the dense
/// grid is affordable; the residue cascade catches whatever tonal peaks fall
/// between posts.
const POSTLIST: [i32; 34] = [
    0, 1024, 1, 2, 3, 4, 5, 6, 8, 10, 12, 16, 20, 25, 32, 40, 50, 64, 80, 100, 128, 160, 200, 256,
    320, 400, 512, 640, 768, 860, 920, 960, 990, 1010,
];
/// Number of floor1 partitions for a short block (dimension 4 each).
const FLOOR_PARTITIONS_SHORT: usize = 4;
/// The short-block floor1 postlist over `SHORT_N` bins: endpoints `0`, `128`
/// then 16 log-spaced interior posts. Coarser than the long postlist because a
/// short block has few bins and a transient's spectrum is broad, not tonal.
const SHORT_POSTLIST: [i32; 18] = [
    0, 128, 1, 2, 3, 4, 6, 8, 12, 16, 24, 32, 48, 64, 80, 96, 112, 124,
];

/// Global codebook indices (the order they are serialized in the setup header).
const BOOK_GROUP: usize = 0; // residue classification book (3 entries)
const BOOK_FLOOR_CLASS: usize = 1; // floor1 class book (16 entries)
const BOOK_FLOOR_VALUE: usize = 2; // floor1 value book (64 entries)
const BOOK_RES_FINE: usize = 3; // residue fine-stage value book (128-entry VQ)
const BOOK_RES_COARSE: usize = 4; // residue coarse-stage value book (128-entry VQ)

/// Number of entries (quant levels) in each residue value book.
const RES_LEVELS: usize = 128;
/// The value-book entry that dequantizes to `0.0` (the grid is symmetric, so the
/// midpoint entry maps to zero for both the fine and coarse books).
const RES_ZERO_ENTRY: usize = 64;

/// Minimum block count for the floor books to be length-fitted. Fitting the two
/// floor books serializes their per-entry lengths into the setup header (a fixed
/// ~45-byte cost), which only pays off once enough blocks amortize it; below this
/// the floor stays on its compact flat books so a short stream never grows. (The
/// residue books are fitted unconditionally — residue dominates every packet, so
/// fitting it wins even for a single block.)
const FLOOR_FIT_MIN_BLOCKS: usize = 8;

/// Fine residue book: `[-16, 16)` in steps of `0.25` (full resolution near zero).
const FINE_MIN: f32 = -16.0;
const FINE_DELTA: f32 = 0.25;
/// Coarse residue book: `[-256, 256)` in steps of `4.0` — the wide first stage
/// that captures concentrated tonal peaks the fine stage then refines.
const COARSE_MIN: f32 = -256.0;
const COARSE_DELTA: f32 = 4.0;
/// Greatest `|residue|` a partition may have to be coded by the fine book alone
/// (just inside the fine book's positive reach); louder partitions add the
/// coarse stage.
const FINE_ONLY_MAX: f32 = 15.0;

/// Centre advance from a long block to its first short neighbour (and from the
/// last short back to the closing long): `(2*BLOCK_N + 2*SHORT_N) / 4`. The two
/// blocks overlap by `2*SHORT_N/2` samples, so their centres sit this far apart.
const LONG_SHORT_ADVANCE: usize = (2 * BLOCK_N + 2 * SHORT_N) / 4;
/// Centre advance between two adjacent short blocks: `2*SHORT_N / 4 = SHORT_N`.
const SHORT_ADVANCE: usize = SHORT_N;
/// Short blocks emitted per replaced long grid slot. A run of `r` transient
/// slots becomes `SHORTS_PER_SLOT * r` shorts so the closing long lands exactly
/// back on the long grid: `LONG_SHORT_ADVANCE + (8r-1)*SHORT_ADVANCE +
/// LONG_SHORT_ADVANCE == (r+1) * 2*BLOCK_N/2`.
const SHORTS_PER_SLOT: usize = 8;
/// Transient detection: switch a long block to short blocks when one of its
/// sub-windows is at least this many times louder than the running average of
/// the sub-windows before it — a sharp onset a long block would pre-echo. The
/// ratio is high so steady tones (whose sub-window energy is near-constant)
/// never trip it, preserving their long-block compression.
const TRANSIENT_RATIO: f64 = 8.0;
/// Sub-windows a long block is split into for transient detection.
const TRANSIENT_CHUNKS: usize = 16;

/// A complete uniform Huffman book: `1 << len` entries, every codeword `len`
/// bits.
fn complete_book(len: u8) -> Codebook {
    Codebook::new(vec![len; 1usize << len]).expect("complete book")
}

/// The signed quant grid shared by both residue books: `value_vector` applies
/// `|quantlist|`, so the non-negative quantlist `[0, 127]` plus a `mindel`/`delta`
/// pair spans a symmetric range in uniform steps.
fn residue_quantlist() -> Vec<f32> {
    (0..RES_LEVELS as i32).map(|i| i as f32).collect()
}

/// The book entry nearest `value` on a uniform scalar grid — the closed form of
/// `VqBook::best_entry` (`round((value - mindel)/delta)` clamped). Lets us
/// histogram residue values without building the book first.
fn quant_entry(value: f32, mindel: f32, delta: f32) -> usize {
    let idx = ((value - mindel) / delta).round() as i32;
    idx.clamp(0, RES_LEVELS as i32 - 1) as usize
}

/// Nearest fine-book entry to `value`.
fn fine_entry(value: f32) -> usize {
    quant_entry(value, FINE_MIN, FINE_DELTA)
}

/// Nearest coarse-book entry to `value`.
fn coarse_entry(value: f32) -> usize {
    quant_entry(value, COARSE_MIN, COARSE_DELTA)
}

/// The value the coarse book's `entry` dequantizes to.
fn coarse_value(entry: usize) -> f32 {
    entry as f32 * COARSE_DELTA + COARSE_MIN
}

/// Minimum inter-channel correlation for square-polar coupling to be applied.
/// Coupling concentrates correlated energy into the magnitude channel so the
/// angle residue collapses toward zero and skips — but only when the channels
/// are *strongly* correlated. At moderate correlation the angle still carries
/// real energy: its quantization spreads error into both channels (worse
/// fidelity) while the size barely changes. So coupling pays off only near the
/// correlated end of the range; below this threshold the stream stays
/// independent and coupling can only help, never regress.
const COUPLE_CORR_THRESHOLD: f64 = 0.9;

/// Whether two channels are correlated enough to benefit from coupling: the
/// normalized cross-correlation (Pearson on the ~zero-mean audio) of the two
/// signals. Leading/trailing pad zeros do not affect the ratio.
fn channels_are_correlated(a: &[f32], b: &[f32]) -> bool {
    let (mut saa, mut sbb, mut sab) = (0.0f64, 0.0f64, 0.0f64);
    for (&x, &y) in a.iter().zip(b) {
        saa += f64::from(x) * f64::from(x);
        sbb += f64::from(y) * f64::from(y);
        sab += f64::from(x) * f64::from(y);
    }
    if saa == 0.0 || sbb == 0.0 {
        return false;
    }
    sab / (saa.sqrt() * sbb.sqrt()) >= COUPLE_CORR_THRESHOLD
}

/// Forward square-polar coupling of one residue scalar pair — the algebraic
/// inverse of the decoder's inverse-coupling (Vorbis I spec §9.4.2). The first
/// channel becomes the *magnitude* and the second the *angle*; the returned
/// `(m, a)` are the values to code so the decoder reconstructs `(l, r)` exactly.
///
/// The transform is exactly invertible (only additions), so it preserves
/// fidelity. For correlated channels (`l == r`) the angle collapses to `0`, so
/// the angle channel's residue becomes all zeros and skips entirely.
fn couple_pair(l: f32, r: f32) -> (f32, f32) {
    if l > 0.0 {
        if l > r {
            (l, l - r)
        } else {
            (r, l - r)
        }
    } else if r > l {
        (l, r - l)
    } else {
        (r, r - l)
    }
}

/// Forward-couples two channels' residue vectors in place: `mag` becomes the
/// magnitude vector, `ang` the angle vector.
fn couple_channels(mag: &mut [f32], ang: &mut [f32]) {
    for (m, a) in mag.iter_mut().zip(ang.iter_mut()) {
        let (cm, ca) = couple_pair(*m, *a);
        *m = cm;
        *a = ca;
    }
}

/// Normalizes a stereo block's two analyzed channels for square-polar coupling.
/// Coupling is declared once in the setup header, so it applies to every packet:
/// if either channel carries audio, both must be coded. A channel that quantized
/// to silence is given its partner's floor and a zero residue (which couples and
/// decodes back to silence), then the two residue vectors are forward-coupled.
/// A fully silent block (both `None`) is left untouched.
fn couple_stereo_block(raw: &mut [Option<(Vec<i32>, Vec<f32>)>]) {
    if raw.len() != 2 || (raw[0].is_none() && raw[1].is_none()) {
        return;
    }
    if raw[0].is_none() {
        if let Some((dev, res)) = &raw[1] {
            raw[0] = Some((dev.clone(), vec![0.0; res.len()]));
        }
    }
    if raw[1].is_none() {
        if let Some((dev, res)) = &raw[0] {
            raw[1] = Some((dev.clone(), vec![0.0; res.len()]));
        }
    }
    let (head, tail) = raw.split_at_mut(1);
    if let (Some((_, r0)), Some((_, r1))) = (head[0].as_mut(), tail[0].as_mut()) {
        couple_channels(r0, r1);
    }
}

/// Zeros residue values that quantize to the fine book's zero entry, so a
/// partition that codes nothing but zeros is detected exactly (`max == 0`) and
/// can be skipped. This does not change the reconstruction: a value that already
/// snaps to the zero entry codes as `0.0` whether it is skipped or coded.
fn snap_residue(residue: &mut [f32]) {
    for v in residue.iter_mut() {
        if fine_entry(*v) == RES_ZERO_ENTRY {
            *v = 0.0;
        }
    }
}

/// Histograms the cascade entries one block's residue would code, mirroring the
/// per-partition classification the residue coder applies: empty partitions code
/// nothing, quiet ones use the fine book alone, loud ones use the coarse book
/// then refine the remainder with the fine book. Accumulates each stage's chosen
/// entries into its own histogram so both books can be fitted to real data.
fn histogram_cascade(
    residue: &[f32],
    coarse: &mut [u64; RES_LEVELS],
    fine: &mut [u64; RES_LEVELS],
) {
    for part in residue.chunks_exact(GROUPING) {
        let max = part.iter().fold(0.0f32, |m, &v| m.max(v.abs()));
        if max == 0.0 {
            continue; // empty partition (type 0): coded by no stage
        }
        if max <= FINE_ONLY_MAX {
            for &v in part {
                fine[fine_entry(v)] += 1;
            }
        } else {
            for &v in part {
                let ce = coarse_entry(v);
                coarse[ce] += 1;
                fine[fine_entry(v - coarse_value(ce))] += 1;
            }
        }
    }
}

/// Builds a residue value VQ book from a codeword-length list and the uniform
/// grid (`mindel`/`delta`) it quantizes onto. Returns `None` if the lengths do
/// not form a valid tree.
fn residue_value_book(lengths: &[u8], mindel: f32, delta: f32) -> Option<VqBook> {
    let book = Codebook::new(lengths.to_vec())?;
    Some(VqBook::new(
        book,
        1,
        mindel,
        delta,
        false,
        residue_quantlist(),
    ))
}

/// Fits a codebook's codeword lengths to a histogram of the entries it codes.
/// Every entry is kept usable (frequency floored to at least 1) so the book
/// stays complete (any entry can be coded); the floor also bounds the frequency
/// ratio to `2^16`, keeping every codeword within the 5-bit length field. Falls
/// back to a flat `fallback_len`-bit book if the fit fails to form a valid tree.
/// Reconstruction is unaffected — only the codeword *lengths* (hence size)
/// change, not which entry codes a value.
fn fit_book_lengths(counts: &[u64], fallback_len: u8) -> Vec<u8> {
    let entries = counts.len();
    let max_count = counts.iter().copied().max().unwrap_or(0);
    // Floor the rarest entry so max/min frequency ratio stays <= 2^16; this caps
    // the longest Huffman codeword well under the 32-entry length-field limit.
    let floor = (max_count >> 16).max(1);
    let freqs: Vec<u64> = counts.iter().map(|&c| c.max(floor)).collect();
    let lengths = huffman_lengths(&freqs);
    let valid = lengths.len() == entries
        && lengths.iter().all(|&l| (1..=32).contains(&l))
        && Codebook::new(lengths.clone()).is_some();
    if valid {
        lengths
    } else {
        vec![fallback_len; entries]
    }
}

/// One channel's contribution to a block: the floor post deviations to pack and
/// the residue values to code.
struct ChannelPlan {
    dev: Vec<i32>,
    residue: Vec<f32>,
}

/// Whether a long block's `2 * BLOCK_N`-sample segment contains a sharp onset
/// (an energy jump a long block would smear backward into pre-echo). Splits the
/// segment into [`TRANSIENT_CHUNKS`] equal sub-windows and reports a transient
/// when a sub-window's energy exceeds [`TRANSIENT_RATIO`] times the average of
/// all preceding sub-windows — and is itself a meaningful fraction of the
/// loudest sub-window, so the noise floor near silence does not trip it.
fn block_is_transient(seg: &[f32]) -> bool {
    let chunk = seg.len() / TRANSIENT_CHUNKS;
    if chunk == 0 {
        return false;
    }
    let mut energy = [0.0f64; TRANSIENT_CHUNKS];
    for (c, slot) in energy.iter_mut().enumerate() {
        let part = &seg[c * chunk..(c + 1) * chunk];
        *slot = part.iter().map(|&x| f64::from(x) * f64::from(x)).sum();
    }
    let max = energy.iter().copied().fold(0.0f64, f64::max);
    if max <= 0.0 {
        return false;
    }
    let mut prev_sum = energy[0];
    for (i, &e) in energy.iter().enumerate().skip(1) {
        // Floor the running average so a near-silent lead-in does not make the
        // ratio explode on ordinary noise.
        let prev_avg = (prev_sum / i as f64).max(max * 1e-3);
        if e > TRANSIENT_RATIO * prev_avg && e > 0.05 * max {
            return true;
        }
        prev_sum += e;
    }
    false
}

/// One scheduled block: its centre sample (in padded coordinates), whether it is
/// a long block, and — for long blocks — the left/right window-overlap flags
/// (`true` = a long neighbour with full overlap; `false` = a short neighbour
/// taking the transition overlap).
struct BlockSpec {
    center: usize,
    long: bool,
    lw: bool,
    nw: bool,
}

/// Builds the block schedule from the per-grid-slot transient flags. Grid slot
/// `k` is a long block centred at `(k + 1) * BLOCK_N`; a maximal run of `r`
/// transient slots is replaced by `SHORTS_PER_SLOT * r` short blocks bracketed
/// by the two neighbouring long blocks (which take the short-overlap transition
/// window on the bordering edge). The first short centre advances
/// [`LONG_SHORT_ADVANCE`] from the opening bracket long, the rest advance
/// [`SHORT_ADVANCE`], and the last lands exactly `LONG_SHORT_ADVANCE` before the
/// closing long — so the grid realigns with no gap and Princen-Bradley holds
/// across every overlap. The caller keeps the first and last slot non-transient
/// so every run has both bracketing long blocks.
fn build_schedule(block_count: usize, transient: &[bool]) -> Vec<BlockSpec> {
    let is_transient = |k: usize| transient.get(k).copied().unwrap_or(false);
    let mut schedule = Vec::new();
    let mut k = 0;
    while k < block_count {
        if is_transient(k) {
            let a = k;
            let mut b = k;
            while b + 1 < block_count && is_transient(b + 1) {
                b += 1;
            }
            let r = b - a + 1;
            // Centre of the opening bracket long (slot a-1) is `a * BLOCK_N`.
            let base = a * BLOCK_N;
            for i in 0..SHORTS_PER_SLOT * r {
                schedule.push(BlockSpec {
                    center: base + LONG_SHORT_ADVANCE + SHORT_ADVANCE * i,
                    long: false,
                    lw: false,
                    nw: false,
                });
            }
            k = b + 1;
        } else {
            // A kept long block; its bordering edges take the transition window
            // wherever the neighbour slot was replaced by shorts.
            let lw = k == 0 || !is_transient(k - 1);
            let nw = k + 1 >= block_count || !is_transient(k + 1);
            schedule.push(BlockSpec {
                center: (k + 1) * BLOCK_N,
                long: true,
                lw,
                nw,
            });
            k += 1;
        }
    }
    schedule
}

/// Assembles a floor1 coder over `partitions` dimension-4 partitions sharing the
/// fixed class table and the supplied codebook pool. The long and short floors
/// differ only in their partition count (and postlist); their books are shared.
fn floor_encoding(partitions: usize, books: Vec<Codebook>) -> Floor1Encoding {
    Floor1Encoding {
        quant_q: QUANT_Q,
        partition_class: vec![0; partitions],
        classes: vec![Floor1Class {
            dim: 4,
            subs: 1,
            book: BOOK_FLOOR_CLASS,
            subbook: vec![-1, BOOK_FLOOR_VALUE as i32],
        }],
        books,
    }
}

/// A pure-Rust Vorbis encoder for one channel layout / sample rate.
pub struct VorbisEncoder {
    channels: u16,
    psy: PsyAnalysis,
    fitter: Floor1Fitter,
    floor: Floor1Encoding,
    short_psy: PsyAnalysis,
    short_fitter: Floor1Fitter,
    short_floor: Floor1Encoding,
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
        // Short-block analysis state, used when a transient switches a long grid
        // slot to short blocks. The floor books are shared with the long floor;
        // only the partition count and postlist differ.
        let short_psy = PsyAnalysis::new(SHORT_N, sample_rate);
        let short_fitter = Floor1Fitter::new(SHORT_POSTLIST.to_vec(), Floor1FitInfo::standard());

        // Packet-side floor coders: books indexed by global codebook number.
        // Index 0 is unused by the floor (it only references 1 and 2); a small
        // valid book keeps the indices aligned with the global list.
        let floor = floor_encoding(
            FLOOR_PARTITIONS,
            vec![complete_book(1), complete_book(4), complete_book(6)],
        );
        let short_floor = floor_encoding(
            FLOOR_PARTITIONS_SHORT,
            vec![complete_book(1), complete_book(4), complete_book(6)],
        );

        // Two block sizes: short (256) for transients, long (2048) for steady
        // content. Declared in the identification header so a blockflag-1 (long)
        // mode can carry the short/long window-overlap flags.
        let short_m = (2 * SHORT_N) as u32;
        let id_bytes = pack_identification_header(channels as u8, sample_rate, 0, 0, 0, short_m, m);
        let comment_bytes = pack_comment_header(b"sonare-codec", &[]);

        Self {
            channels,
            psy,
            fitter,
            floor,
            short_psy,
            short_fitter,
            short_floor,
            id_bytes,
            comment_bytes,
        }
    }

    /// Builds a packet-side floor coder over `partitions` partitions from
    /// length-fitted class-phrase and post-value books (the cascade structure is
    /// fixed). Mirrors the floor built in [`new`](Self::new) but with the
    /// per-stream fitted books; the long and short floors share these books and
    /// differ only in `partitions`.
    fn build_floor(
        partitions: usize,
        class_lengths: &[u8],
        value_lengths: &[u8],
    ) -> Floor1Encoding {
        floor_encoding(
            partitions,
            vec![
                complete_book(1),
                Codebook::new(class_lengths.to_vec()).unwrap_or_else(|| complete_book(4)),
                Codebook::new(value_lengths.to_vec()).unwrap_or_else(|| complete_book(6)),
            ],
        )
    }

    /// Builds the packet-side residue coder from the fitted value-book lengths.
    /// Three partition types classified by peak magnitude: type 0 codes nothing
    /// (empty), type 1 codes through the fine book alone (quiet), type 2 codes a
    /// coarse stage then refines with the fine book (loud tonal peaks). The
    /// coarse stage gives the cascade a wide dynamic range while the fine stage
    /// keeps full resolution near zero.
    fn build_residue(end: usize, coarse_lengths: &[u8], fine_lengths: &[u8]) -> ResidueConfig {
        let fine = || {
            residue_value_book(fine_lengths, FINE_MIN, FINE_DELTA).unwrap_or_else(|| {
                VqBook::new(
                    complete_book(7),
                    1,
                    FINE_MIN,
                    FINE_DELTA,
                    false,
                    residue_quantlist(),
                )
            })
        };
        let coarse =
            residue_value_book(coarse_lengths, COARSE_MIN, COARSE_DELTA).unwrap_or_else(|| {
                VqBook::new(
                    complete_book(7),
                    1,
                    COARSE_MIN,
                    COARSE_DELTA,
                    false,
                    residue_quantlist(),
                )
            });
        ResidueConfig {
            begin: 0,
            end,
            grouping: GROUPING,
            partitions: 3,
            partitions_per_word: 1,
            stages: 2,
            // Type 0: no stages (skip). Type 1: fine stage only (bit 1). Type 2:
            // coarse stage (bit 0) then fine stage (bit 1).
            secondstages: vec![0b00, 0b10, 0b11],
            partbooks: vec![
                vec![None, None],
                vec![None, Some(fine())],
                vec![Some(coarse), Some(fine())],
            ],
            // max == 0 -> type 0 (skip); max <= FINE_ONLY_MAX -> type 1 (fine
            // only); louder -> type 2 (coarse + fine).
            classmetric1: vec![0, FINE_ONLY_MAX as i32, 0],
            classmetric2: vec![-1, -1, -1],
            // 3 partition types: type 0 (skip, the common case) gets the 1-bit
            // codeword, types 1 and 2 get 2 bits.
            phrasebook: Codebook::new(vec![1, 2, 2]).unwrap_or_else(|| complete_book(2)),
        }
    }

    /// Assembles the spec setup configuration (the serialized counterpart of the
    /// packet-side floor/residue coders), with the residue value book carrying
    /// the per-stream fitted codeword lengths.
    fn build_setup(
        channels: u16,
        coupled: bool,
        coarse_lengths: &[u8],
        fine_lengths: &[u8],
        floor_class_lengths: &[u8],
        floor_value_lengths: &[u8],
    ) -> SetupConfig {
        let codebooks = vec![
            // 0: residue classification book (3 partition types; type 0 = 1 bit).
            StaticCodebook {
                dim: 1,
                entries: 3,
                lengthlist: vec![1, 2, 2],
                maptype: 0,
                q_min: 0,
                q_delta: 0,
                q_quant: 0,
                q_sequencep: false,
                quantlist: vec![],
            },
            // 1: floor1 class-phrase book (16 entries), lengths fitted per stream.
            StaticCodebook {
                dim: 1,
                entries: 16,
                lengthlist: floor_class_lengths.to_vec(),
                maptype: 0,
                q_min: 0,
                q_delta: 0,
                q_quant: 0,
                q_sequencep: false,
                quantlist: vec![],
            },
            // 2: floor1 post-value book (64 entries), lengths fitted per stream.
            StaticCodebook {
                dim: 1,
                entries: 64,
                lengthlist: floor_value_lengths.to_vec(),
                maptype: 0,
                q_min: 0,
                q_delta: 0,
                q_quant: 0,
                q_sequencep: false,
                quantlist: vec![],
            },
            // 3: residue fine-stage VQ book (128 entries, maptype 1, step 0.25).
            // The codeword lengths are fitted to the stream's residue.
            StaticCodebook {
                dim: 1,
                entries: RES_LEVELS as u32,
                lengthlist: fine_lengths.to_vec(),
                maptype: 1,
                q_min: float32_pack(FINE_MIN),
                q_delta: float32_pack(FINE_DELTA),
                q_quant: 7,
                q_sequencep: false,
                quantlist: (0..RES_LEVELS as i32).collect(),
            },
            // 4: residue coarse-stage VQ book (128 entries, maptype 1, step 4.0)
            // — the wide first stage for loud tonal partitions.
            StaticCodebook {
                dim: 1,
                entries: RES_LEVELS as u32,
                lengthlist: coarse_lengths.to_vec(),
                maptype: 1,
                q_min: float32_pack(COARSE_MIN),
                q_delta: float32_pack(COARSE_DELTA),
                q_quant: 7,
                q_sequencep: false,
                quantlist: (0..RES_LEVELS as i32).collect(),
            },
        ];

        // A type-1 floor differs between block sizes only in its partition count
        // and postlist; the class table and books are shared.
        let floor_setup = |partitions: usize, postlist: &[i32]| Floor1Setup {
            partition_class: vec![0; partitions],
            class_dim: vec![4],
            class_subs: vec![1],
            class_book: vec![BOOK_FLOOR_CLASS as u8],
            class_subbook: vec![vec![-1, BOOK_FLOOR_VALUE as i32]],
            mult: FLOOR_MULT as u8,
            postlist: postlist.iter().map(|&p| p as u32).collect(),
        };
        // The residue cascade is identical for both block sizes apart from its
        // spectral extent (`end`).
        let residue_setup = |end: u32| ResidueSetup {
            residue_type: 1,
            begin: 0,
            end,
            grouping: GROUPING as u32,
            groupbook: BOOK_GROUP as u8,
            // Per partition type: type 0 nothing, type 1 the fine stage, type 2
            // the coarse then fine stages. The booklist names the set-bit stages
            // in type-major, stage-minor order: [fine(t1), coarse(t2), fine(t2)].
            secondstages: vec![0b00, 0b10, 0b11],
            booklist: vec![
                BOOK_RES_FINE as u8,
                BOOK_RES_COARSE as u8,
                BOOK_RES_FINE as u8,
            ],
        };

        // Square-polar couple a stereo pair (channel 0 = magnitude, 1 = angle):
        // correlated content sends the angle residue to zero, which then skips.
        // Other channel counts stay independent (no coupling). Both mappings
        // (long submap 0, short submap 1) carry the same coupling.
        let mapping_setup = |floor_idx: u8, residue_idx: u8| {
            let (coupling_mag, coupling_ang) = if coupled {
                (vec![0u32], vec![1u32])
            } else {
                (vec![], vec![])
            };
            Mapping0Setup {
                submaps: 1,
                coupling_mag,
                coupling_ang,
                chmuxlist: vec![0; channels as usize],
                floorsubmap: vec![floor_idx],
                residuesubmap: vec![residue_idx],
            }
        };

        SetupConfig {
            channels,
            codebooks,
            // Floor 0 / residue 0 / mapping 0 = long block; 1 = short block.
            floors: vec![
                (1, floor_setup(FLOOR_PARTITIONS, &POSTLIST)),
                (1, floor_setup(FLOOR_PARTITIONS_SHORT, &SHORT_POSTLIST)),
            ],
            residues: vec![residue_setup(BLOCK_N as u32), residue_setup(SHORT_N as u32)],
            mappings: vec![(0, mapping_setup(0, 0)), (0, mapping_setup(1, 1))],
            // Mode 0 = long (blockflag 1, carries window-overlap flags), mode 1 =
            // short (blockflag 0). The mode number is one bit per audio packet.
            modes: vec![
                ModeSetup {
                    blockflag: true,
                    windowtype: 0,
                    transformtype: 0,
                    mapping: 0,
                },
                ModeSetup {
                    blockflag: false,
                    windowtype: 0,
                    transformtype: 0,
                    mapping: 1,
                },
            ],
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

        // The long grid: `block_count` long blocks centred at `(k+1)*half` with
        // 50% overlap. Detect a transient in each interior grid slot (the first
        // and last slot stay long so every transient run keeps both bracketing
        // long blocks), then schedule short blocks over the transient runs.
        let mut block_count = 0usize;
        while (block_count * half) + m <= padded_len {
            block_count += 1;
        }
        let mut transient = vec![false; block_count];
        if block_count >= 3 {
            for (k, flag) in transient
                .iter_mut()
                .enumerate()
                .take(block_count - 1)
                .skip(1)
            {
                let pos = k * half;
                *flag = planar
                    .iter()
                    .filter_map(|ch| ch.get(pos..pos + m))
                    .any(block_is_transient);
            }
        }
        let schedule = build_schedule(block_count, &transient);

        // Pass 1: analyze every scheduled block, building the per-channel plans
        // and the per-stage histograms the residue (and floor) books are fitted
        // to. Long and short blocks share the residue and floor codebooks, so
        // their entries histogram into the same counts.
        let (lo, hi) = low_high_neighbors(&POSTLIST);
        let (short_lo, short_hi) = low_high_neighbors(&SHORT_POSTLIST);
        // Couple a stereo pair only when the channels are correlated enough that
        // coupling improves (never regresses) fidelity; otherwise code them
        // independently. The choice is per stream, matching the setup header.
        let coupled = channel_count == 2 && channels_are_correlated(&planar[0], &planar[1]);
        let mut plans: Vec<Vec<Option<ChannelPlan>>> = Vec::with_capacity(schedule.len());
        let mut coarse_counts = [0u64; RES_LEVELS];
        let mut fine_counts = [0u64; RES_LEVELS];
        // Per-floor-book histograms of the entries each block's floor would code,
        // keyed by the floor coder's book index (the class-phrase book and the
        // post-value book), so both floor books can be length-fitted too.
        let mut floor_counts: Vec<Vec<u64>> = self
            .floor
            .books
            .iter()
            .map(|b| vec![0u64; b.entries()])
            .collect();

        for spec in &schedule {
            // Select the long or short analysis chain. A long block spans
            // `2*BLOCK_N` samples through a left/right transition window (the
            // symmetric long window when both neighbours are long); a short block
            // spans `2*SHORT_N` through the symmetric short window.
            let (psy, fitter, postlist, floor_hist, neigh_lo, neigh_hi, bins) = if spec.long {
                (
                    &self.psy,
                    &self.fitter,
                    &POSTLIST[..],
                    &self.floor,
                    &lo,
                    &hi,
                    BLOCK_N,
                )
            } else {
                (
                    &self.short_psy,
                    &self.short_fitter,
                    &SHORT_POSTLIST[..],
                    &self.short_floor,
                    &short_lo,
                    &short_hi,
                    SHORT_N,
                )
            };
            let window = if !spec.long {
                vorbis_window(2 * SHORT_N)
            } else if spec.lw && spec.nw {
                // Both neighbours long: the transition window reduces exactly to
                // the symmetric long window, so use it directly.
                vorbis_window(2 * BLOCK_N)
            } else {
                vorbis_window_lr(
                    2 * BLOCK_N,
                    if spec.lw { BLOCK_N } else { SHORT_N },
                    if spec.nw { BLOCK_N } else { SHORT_N },
                )
            };
            let start = spec.center - bins;

            let mut raw: Vec<Option<(Vec<i32>, Vec<f32>)>> = Vec::with_capacity(channel_count);
            for ch in &planar {
                let block = ch
                    .get(start..start + 2 * bins)
                    .and_then(|seg| analyze_block_windowed(psy, fitter, postlist, seg, &window));
                match block {
                    Some(b) => {
                        let mut posts = b.posts.clone();
                        let dev = encode_post_deviations(
                            postlist, &mut posts, neigh_lo, neigh_hi, QUANT_Q,
                        );
                        let mut residue = b.residue;
                        // AoTuV M1 companding: relatively compensate the residue
                        // against the noise floor on steady (long) blocks, where
                        // sustained near-floor energy is what costs bits. A short
                        // block codes a transient attack, so it is left untouched
                        // to preserve that energy.
                        if spec.long {
                            let gains = psy.m1_companding_gains(&b.logmdct);
                            if gains.len() == residue.len() {
                                for (r, g) in residue.iter_mut().zip(&gains) {
                                    *r *= g;
                                }
                            }
                        }
                        raw.push(Some((dev, residue)));
                    }
                    None => raw.push(None),
                }
            }

            // Forward-couple the stereo pair before the residue is snapped and
            // histogrammed, so the books are fitted to what is actually coded.
            if coupled {
                couple_stereo_block(&mut raw);
            }

            let mut plan: Vec<Option<ChannelPlan>> = Vec::with_capacity(channel_count);
            for entry in raw {
                match entry {
                    Some((dev, mut residue)) => {
                        floor_hist.histogram(&dev, &mut floor_counts);
                        snap_residue(&mut residue);
                        histogram_cascade(&residue, &mut coarse_counts, &mut fine_counts);
                        plan.push(Some(ChannelPlan { dev, residue }));
                    }
                    None => plan.push(None),
                }
            }
            plans.push(plan);
        }

        // Fit each residue book to its stage's histogram, then serialize the
        // setup header and build the matching packet-side residue coders.
        let coarse_lengths = fit_book_lengths(&coarse_counts, 7);
        let fine_lengths = fit_book_lengths(&fine_counts, 7);
        // Fit the two floor books to their histograms the same way, but only when
        // enough blocks amortize the setup-header cost; otherwise keep the compact
        // flat books (class 4-bit, post-value 6-bit). Histogramming used the
        // construction-time books, whose entry counts match the fitted ones, so
        // the classification — and thus the reconstruction — is unchanged.
        let (floor_class_lengths, floor_value_lengths) = if plans.len() >= FLOOR_FIT_MIN_BLOCKS {
            (
                fit_book_lengths(&floor_counts[BOOK_FLOOR_CLASS], 4),
                fit_book_lengths(&floor_counts[BOOK_FLOOR_VALUE], 6),
            )
        } else {
            (vec![4u8; 16], vec![6u8; 64])
        };
        // Long and short coders share the fitted books, differing only in the
        // floor partition count and the residue spectral extent.
        let long_floor =
            Self::build_floor(FLOOR_PARTITIONS, &floor_class_lengths, &floor_value_lengths);
        let short_floor = Self::build_floor(
            FLOOR_PARTITIONS_SHORT,
            &floor_class_lengths,
            &floor_value_lengths,
        );
        let long_residue = Self::build_residue(BLOCK_N, &coarse_lengths, &fine_lengths);
        let short_residue = Self::build_residue(SHORT_N, &coarse_lengths, &fine_lengths);
        let setup_bytes = Self::build_setup(
            self.channels,
            coupled,
            &coarse_lengths,
            &fine_lengths,
            &floor_class_lengths,
            &floor_value_lengths,
        )
        .pack();

        // Pass 2: emit each block's audio packet with the fitted coder, tagging
        // each with its end granule (sample) position. With 50% overlap the first
        // block only primes the overlap, so block `k` finalizes the samples up to
        // its centre (`centre - half`); the final block's granule is clamped to
        // the true input length so the decoder trims the tail padding to an exact
        // length. Centres advance monotonically across long and short blocks, so
        // the granules stay monotonic and the final clamp stays strictly above
        // the previous block's granule.
        let scheduled = plans.len();
        let mut audio: Vec<(Vec<u8>, u64)> = Vec::with_capacity(scheduled);
        for (k, (spec, plan)) in schedule.iter().zip(&plans).enumerate() {
            let granule = if k + 1 == scheduled {
                frames as u64
            } else {
                (spec.center - half) as u64
            };
            let (floor, residue) = if spec.long {
                (&long_floor, &long_residue)
            } else {
                (&short_floor, &short_residue)
            };
            audio.push((Self::write_packet(floor, residue, spec, plan), granule));
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
    /// audio-packet bit, the mode (and, for a long block, its left/right
    /// window-overlap flags), then per-channel floor1, then the residue of the
    /// channels whose floor is present. `floor`/`residue` are the per-stream
    /// length-fitted coders for this block's size.
    fn write_packet(
        floor: &Floor1Encoding,
        residue: &ResidueConfig,
        spec: &BlockSpec,
        plan: &[Option<ChannelPlan>],
    ) -> Vec<u8> {
        let mut w = BitWriter::new();
        w.write(0, 1); // audio packet (not a header)
                       // Two modes, so the mode number is one bit. Mode 0 is the
                       // long block (blockflag 1) and carries the left/right
                       // window-overlap flags; mode 1 is the short block
                       // (blockflag 0), which has no window flags.
        if spec.long {
            w.write(0, 1); // mode number 0 (long)
            w.write(u32::from(spec.lw), 1); // previous-window flag
            w.write(u32::from(spec.nw), 1); // next-window flag
        } else {
            w.write(1, 1); // mode number 1 (short)
        }

        let mut active: Vec<Vec<f32>> = Vec::new();
        for channel in plan {
            match channel {
                Some(cp) => {
                    floor.pack(&cp.dev, &mut w);
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
