use super::*;

/// One channel's contribution to a block: the floor post deviations to pack and
/// the residue values to code.
pub(crate) struct ChannelPlan {
    pub(crate) dev: Vec<i32>,
    pub(crate) residue: Vec<f32>,
}

/// Whether a long block's `2 * BLOCK_N`-sample segment contains a sharp onset
/// (an energy jump a long block would smear backward into pre-echo). Splits the
/// segment into [`TRANSIENT_CHUNKS`] equal sub-windows and reports a transient
/// when a sub-window's energy exceeds [`TRANSIENT_RATIO`] times the average of
/// all preceding sub-windows — and is itself a meaningful fraction of the
/// loudest sub-window, so the noise floor near silence does not trip it.
pub(crate) fn block_is_transient(seg: &[f32]) -> bool {
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
pub(crate) struct BlockSpec {
    pub(crate) center: usize,
    pub(crate) long: bool,
    pub(crate) lw: bool,
    pub(crate) nw: bool,
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
pub(crate) fn build_schedule(block_count: usize, transient: &[bool]) -> Vec<BlockSpec> {
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
pub(crate) fn floor_encoding(partitions: usize, books: Vec<Codebook>) -> Floor1Encoding {
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
