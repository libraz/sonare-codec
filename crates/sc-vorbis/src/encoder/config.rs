use super::*;

pub(crate) const STREAM_SERIAL: u32 = 0x736f_6e61; // "sona"
/// MDCT bins per block (block size is `2 * BLOCK_N` samples). A long block
/// (2048-sample window) resolves steady tones into far fewer bins than a short
/// block, so most of the spectrum is genuinely empty and compresses well; the
/// two-stage residue cascade carries the now-concentrated tonal peaks.
pub(crate) const BLOCK_N: usize = 1024;
/// MDCT bins for a short block (block size `2 * SHORT_N = 256` samples). Short
/// blocks localize a transient's energy to a 256-sample window instead of
/// smearing it across the long 2048-sample window (pre-echo), at the cost of
/// frequency resolution.
pub(crate) const SHORT_N: usize = 128;
/// floor1 post-value ceiling, paired with [`FLOOR_MULT`].
pub(crate) const QUANT_Q: i32 = 64;
/// Residue partition size (samples per partition).
pub(crate) const GROUPING: usize = 16;
/// Number of floor1 partitions (each of dimension 4) over the interior posts.
pub(crate) const FLOOR_PARTITIONS: usize = 8;
/// The floor1 postlist (full, unsorted): the two endpoints (`0`, the bin count)
/// then 64 interior posts, log-spaced (denser at low/mid bins where tones land).
/// With a long block the floor is amortized over 8x fewer packets, so the dense
/// grid is affordable; the residue cascade catches whatever tonal peaks fall
/// between posts.
pub(crate) const POSTLIST: [i32; 34] = [
    0, 1024, 1, 2, 3, 4, 5, 6, 8, 10, 12, 16, 20, 25, 32, 40, 50, 64, 80, 100, 128, 160, 200, 256,
    320, 400, 512, 640, 768, 860, 920, 960, 990, 1010,
];
/// Number of floor1 partitions for a short block (dimension 4 each).
pub(crate) const FLOOR_PARTITIONS_SHORT: usize = 4;
/// The short-block floor1 postlist over `SHORT_N` bins: endpoints `0`, `128`
/// then 16 log-spaced interior posts. Coarser than the long postlist because a
/// short block has few bins and a transient's spectrum is broad, not tonal.
pub(crate) const SHORT_POSTLIST: [i32; 18] = [
    0, 128, 1, 2, 3, 4, 6, 8, 12, 16, 24, 32, 48, 64, 80, 96, 112, 124,
];

/// Global codebook indices (the order they are serialized in the setup header).
pub(crate) const BOOK_GROUP: usize = 0; // residue classification book (3 entries)
pub(crate) const BOOK_FLOOR_CLASS: usize = 1; // floor1 class book (16 entries)
pub(crate) const BOOK_FLOOR_VALUE: usize = 2; // floor1 value book (64 entries)
pub(crate) const BOOK_RES_FINE: usize = 3; // residue fine-stage value book (128-entry VQ)
pub(crate) const BOOK_RES_COARSE: usize = 4; // residue coarse-stage value book (128-entry VQ)

/// Number of entries (quant levels) in each residue value book.
pub(crate) const RES_LEVELS: usize = 128;
/// The value-book entry that dequantizes to `0.0` (the grid is symmetric, so the
/// midpoint entry maps to zero for both the fine and coarse books).
pub(crate) const RES_ZERO_ENTRY: usize = 64;

/// Minimum block count for the floor books to be length-fitted. Fitting the two
/// floor books serializes their per-entry lengths into the setup header (a fixed
/// ~45-byte cost), which only pays off once enough blocks amortize it; below this
/// the floor stays on its compact flat books so a short stream never grows. (The
/// residue books are fitted unconditionally — residue dominates every packet, so
/// fitting it wins even for a single block.)
pub(crate) const FLOOR_FIT_MIN_BLOCKS: usize = 8;

/// Fine residue book: `[-16, 16)` in steps of `0.25` (full resolution near zero).
pub(crate) const FINE_MIN: f32 = -16.0;
pub(crate) const FINE_DELTA: f32 = 0.25;
/// Coarse residue book: `[-256, 256)` in steps of `4.0` — the wide first stage
/// that captures concentrated tonal peaks the fine stage then refines.
pub(crate) const COARSE_MIN: f32 = -256.0;
pub(crate) const COARSE_DELTA: f32 = 4.0;
/// Greatest `|residue|` a partition may have to be coded by the fine book alone
/// (just inside the fine book's positive reach); louder partitions add the
/// coarse stage.
pub(crate) const FINE_ONLY_MAX: f32 = 15.0;

/// Centre advance from a long block to its first short neighbour (and from the
/// last short back to the closing long): `(2*BLOCK_N + 2*SHORT_N) / 4`. The two
/// blocks overlap by `2*SHORT_N/2` samples, so their centres sit this far apart.
pub(crate) const LONG_SHORT_ADVANCE: usize = (2 * BLOCK_N + 2 * SHORT_N) / 4;
/// Centre advance between two adjacent short blocks: `2*SHORT_N / 4 = SHORT_N`.
pub(crate) const SHORT_ADVANCE: usize = SHORT_N;
/// Short blocks emitted per replaced long grid slot. A run of `r` transient
/// slots becomes `SHORTS_PER_SLOT * r` shorts so the closing long lands exactly
/// back on the long grid: `LONG_SHORT_ADVANCE + (8r-1)*SHORT_ADVANCE +
/// LONG_SHORT_ADVANCE == (r+1) * 2*BLOCK_N/2`.
pub(crate) const SHORTS_PER_SLOT: usize = 8;
/// Transient detection: switch a long block to short blocks when one of its
/// sub-windows is at least this many times louder than the running average of
/// the sub-windows before it — a sharp onset a long block would pre-echo. The
/// ratio is high so steady tones (whose sub-window energy is near-constant)
/// never trip it, preserving their long-block compression.
pub(crate) const TRANSIENT_RATIO: f64 = 8.0;
/// Sub-windows a long block is split into for transient detection.
pub(crate) const TRANSIENT_CHUNKS: usize = 16;

/// A complete uniform Huffman book: `1 << len` entries, every codeword `len`
/// bits.
pub(crate) fn complete_book(len: u8) -> Codebook {
    Codebook::new(vec![len; 1usize << len]).expect("complete book")
}

/// The signed quant grid shared by both residue books: `value_vector` applies
/// `|quantlist|`, so the non-negative quantlist `[0, 127]` plus a `mindel`/`delta`
/// pair spans a symmetric range in uniform steps.
pub(crate) fn residue_quantlist() -> Vec<f32> {
    (0..RES_LEVELS as i32).map(|i| i as f32).collect()
}

/// The book entry nearest `value` on a uniform scalar grid — the closed form of
/// `VqBook::best_entry` (`round((value - mindel)/delta)` clamped). Lets us
/// histogram residue values without building the book first.
pub(crate) fn quant_entry(value: f32, mindel: f32, delta: f32) -> usize {
    let idx = ((value - mindel) / delta).round() as i32;
    idx.clamp(0, RES_LEVELS as i32 - 1) as usize
}

/// Nearest fine-book entry to `value`.
pub(crate) fn fine_entry(value: f32) -> usize {
    quant_entry(value, FINE_MIN, FINE_DELTA)
}

/// Nearest coarse-book entry to `value`.
pub(crate) fn coarse_entry(value: f32) -> usize {
    quant_entry(value, COARSE_MIN, COARSE_DELTA)
}

/// The value the coarse book's `entry` dequantizes to.
pub(crate) fn coarse_value(entry: usize) -> f32 {
    entry as f32 * COARSE_DELTA + COARSE_MIN
}
