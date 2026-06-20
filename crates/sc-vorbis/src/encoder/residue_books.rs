use super::*;

/// Minimum inter-channel correlation for square-polar coupling to be applied.
/// Coupling concentrates correlated energy into the magnitude channel so the
/// angle residue collapses toward zero and skips — but only when the channels
/// are *strongly* correlated. At moderate correlation the angle still carries
/// real energy: its quantization spreads error into both channels (worse
/// fidelity) while the size barely changes. So coupling pays off only near the
/// correlated end of the range; below this threshold the stream stays
/// independent and coupling can only help, never regress.
pub(crate) const COUPLE_CORR_THRESHOLD: f64 = 0.9;

/// Whether two channels are correlated enough to benefit from coupling: the
/// normalized cross-correlation (Pearson on the ~zero-mean audio) of the two
/// signals. Leading/trailing pad zeros do not affect the ratio.
pub(crate) fn channels_are_correlated(a: &[f32], b: &[f32]) -> bool {
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
pub(crate) fn couple_pair(l: f32, r: f32) -> (f32, f32) {
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
pub(crate) fn couple_channels(mag: &mut [f32], ang: &mut [f32]) {
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
pub(crate) fn couple_stereo_block(raw: &mut [Option<(Vec<i32>, Vec<f32>)>]) {
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
pub(crate) fn snap_residue(residue: &mut [f32]) {
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
pub(crate) fn histogram_cascade(
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
pub(crate) fn residue_value_book(lengths: &[u8], mindel: f32, delta: f32) -> Option<VqBook> {
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
pub(crate) fn fit_book_lengths(counts: &[u64], fallback_len: u8) -> Vec<u8> {
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
