//! Vorbis floor1 curve synthesis (the encoder's floor render).
//!
//! Hand-ported to safe Rust from `floor1_inverse2` and `render_line` in
//! libvorbis/aoTuV `lib/floor1.c`, with the `FLOOR1_fromdB_LOOKUP` table
//! transcribed from `lib/mapping0.c`: turns fitted floor posts into the
//! linear per-bin floor curve the encoder divides the spectrum by to form the
//! residue. Derivative work of libvorbis/aoTuV (BSD-3-Clause); see
//! `LICENSE-THIRDPARTY`.

// The lookup table is transcribed verbatim from libvorbis; keep its published
// digits rather than trimming them to the nearest f32 representation.
#![allow(clippy::excessive_precision)]

/// Floor1 dB-to-linear synthesis lookup (`FLOOR1_fromdB_LOOKUP`): index by the
/// integer floor value `0..=255` for the linear floor multiplier.
static FLOOR1_FROMDB_LOOKUP: [f32; 256] = [
    1.0649863e-07f32,
    1.1341951e-07f32,
    1.2079015e-07f32,
    1.2863978e-07f32,
    1.3699951e-07f32,
    1.4590251e-07f32,
    1.5538408e-07f32,
    1.6548181e-07f32,
    1.7623575e-07f32,
    1.8768855e-07f32,
    1.9988561e-07f32,
    2.128753e-07f32,
    2.2670913e-07f32,
    2.4144197e-07f32,
    2.5713223e-07f32,
    2.7384213e-07f32,
    2.9163793e-07f32,
    3.1059021e-07f32,
    3.3077411e-07f32,
    3.5226968e-07f32,
    3.7516214e-07f32,
    3.9954229e-07f32,
    4.2550680e-07f32,
    4.5315863e-07f32,
    4.8260743e-07f32,
    5.1396998e-07f32,
    5.4737065e-07f32,
    5.8294187e-07f32,
    6.2082472e-07f32,
    6.6116941e-07f32,
    7.0413592e-07f32,
    7.4989464e-07f32,
    7.9862701e-07f32,
    8.5052630e-07f32,
    9.0579828e-07f32,
    9.6466216e-07f32,
    1.0273513e-06f32,
    1.0941144e-06f32,
    1.1652161e-06f32,
    1.2409384e-06f32,
    1.3215816e-06f32,
    1.4074654e-06f32,
    1.4989305e-06f32,
    1.5963394e-06f32,
    1.7000785e-06f32,
    1.8105592e-06f32,
    1.9282195e-06f32,
    2.0535261e-06f32,
    2.1869758e-06f32,
    2.3290978e-06f32,
    2.4804557e-06f32,
    2.6416497e-06f32,
    2.8133190e-06f32,
    2.9961443e-06f32,
    3.1908506e-06f32,
    3.3982101e-06f32,
    3.6190449e-06f32,
    3.8542308e-06f32,
    4.1047004e-06f32,
    4.3714470e-06f32,
    4.6555282e-06f32,
    4.9580707e-06f32,
    5.2802740e-06f32,
    5.6234160e-06f32,
    5.9888572e-06f32,
    6.3780469e-06f32,
    6.7925283e-06f32,
    7.2339451e-06f32,
    7.7040476e-06f32,
    8.2047000e-06f32,
    8.7378876e-06f32,
    9.3057248e-06f32,
    9.9104632e-06f32,
    1.0554501e-05f32,
    1.1240392e-05f32,
    1.1970856e-05f32,
    1.2748789e-05f32,
    1.3577278e-05f32,
    1.4459606e-05f32,
    1.5399272e-05f32,
    1.6400004e-05f32,
    1.7465768e-05f32,
    1.8600792e-05f32,
    1.9809576e-05f32,
    2.1096914e-05f32,
    2.2467911e-05f32,
    2.3928002e-05f32,
    2.5482978e-05f32,
    2.7139006e-05f32,
    2.8902651e-05f32,
    3.0780908e-05f32,
    3.2781225e-05f32,
    3.4911534e-05f32,
    3.7180282e-05f32,
    3.9596466e-05f32,
    4.2169667e-05f32,
    4.4910090e-05f32,
    4.7828601e-05f32,
    5.0936773e-05f32,
    5.4246931e-05f32,
    5.7772202e-05f32,
    6.1526565e-05f32,
    6.5524908e-05f32,
    6.9783085e-05f32,
    7.4317983e-05f32,
    7.9147585e-05f32,
    8.4291040e-05f32,
    8.9768747e-05f32,
    9.5602426e-05f32,
    0.00010181521f32,
    0.00010843174f32,
    0.00011547824f32,
    0.00012298267f32,
    0.00013097477f32,
    0.00013948625f32,
    0.00014855085f32,
    0.00015820453f32,
    0.00016848555f32,
    0.00017943469f32,
    0.00019109536f32,
    0.00020351382f32,
    0.00021673929f32,
    0.00023082423f32,
    0.00024582449f32,
    0.00026179955f32,
    0.00027881276f32,
    0.00029693158f32,
    0.00031622787f32,
    0.00033677814f32,
    0.00035866388f32,
    0.00038197188f32,
    0.00040679456f32,
    0.00043323036f32,
    0.00046138411f32,
    0.00049136745f32,
    0.00052329927f32,
    0.00055730621f32,
    0.00059352311f32,
    0.00063209358f32,
    0.00067317058f32,
    0.00071691700f32,
    0.00076350630f32,
    0.00081312324f32,
    0.00086596457f32,
    0.00092223983f32,
    0.00098217216f32,
    0.0010459992f32,
    0.0011139742f32,
    0.0011863665f32,
    0.0012634633f32,
    0.0013455702f32,
    0.0014330129f32,
    0.0015261382f32,
    0.0016253153f32,
    0.0017309374f32,
    0.0018434235f32,
    0.0019632195f32,
    0.0020908006f32,
    0.0022266726f32,
    0.0023713743f32,
    0.0025254795f32,
    0.0026895994f32,
    0.0028643847f32,
    0.0030505286f32,
    0.0032487691f32,
    0.0034598925f32,
    0.0036847358f32,
    0.0039241906f32,
    0.0041792066f32,
    0.0044507950f32,
    0.0047400328f32,
    0.0050480668f32,
    0.0053761186f32,
    0.0057254891f32,
    0.0060975636f32,
    0.0064938176f32,
    0.0069158225f32,
    0.0073652516f32,
    0.0078438871f32,
    0.0083536271f32,
    0.0088964928f32,
    0.009474637f32,
    0.010090352f32,
    0.010746080f32,
    0.011444421f32,
    0.012188144f32,
    0.012980198f32,
    0.013823725f32,
    0.014722068f32,
    0.015678791f32,
    0.016697687f32,
    0.017782797f32,
    0.018938423f32,
    0.020169149f32,
    0.021479854f32,
    0.022875735f32,
    0.024362330f32,
    0.025945531f32,
    0.027631618f32,
    0.029427276f32,
    0.031339626f32,
    0.033376252f32,
    0.035545228f32,
    0.037855157f32,
    0.040315199f32,
    0.042935108f32,
    0.045725273f32,
    0.048696758f32,
    0.051861348f32,
    0.055231591f32,
    0.058820850f32,
    0.062643361f32,
    0.066714279f32,
    0.071049749f32,
    0.075666962f32,
    0.080584227f32,
    0.085821044f32,
    0.091398179f32,
    0.097337747f32,
    0.10366330f32,
    0.11039993f32,
    0.11757434f32,
    0.12521498f32,
    0.13335215f32,
    0.14201813f32,
    0.15124727f32,
    0.16107617f32,
    0.17154380f32,
    0.18269168f32,
    0.19456402f32,
    0.20720788f32,
    0.22067342f32,
    0.23501402f32,
    0.25028656f32,
    0.26655159f32,
    0.28387361f32,
    0.30232132f32,
    0.32196786f32,
    0.34289114f32,
    0.36517414f32,
    0.38890521f32,
    0.41417847f32,
    0.44109412f32,
    0.46975890f32,
    0.50028648f32,
    0.53279791f32,
    0.56742212f32,
    0.60429640f32,
    0.64356699f32,
    0.68538959f32,
    0.72993007f32,
    0.77736504f32,
    0.82788260f32,
    0.88168307f32,
    0.9389798f32,
    1f32,
];

/// Multiplies the floor curve `d` along the line from `(x0, y0)` to `(x1, y1)`
/// by the dB lookup at each integer `y` (`render_line`). `y0`/`y1` index the
/// lookup; the integer DDA keeps `y` between them.
fn render_line(d: &mut [f32], x0: i32, x1: i32, y0: i32, y1: i32) {
    let mut n = d.len() as i32;
    let adx = x1 - x0;
    if adx <= 0 {
        return;
    }
    let dy = y1 - y0;
    let ady0 = dy.abs();
    let base = dy / adx;
    let sy = if dy < 0 { base - 1 } else { base + 1 };
    let ady = ady0 - (base * adx).abs();

    if n > x1 {
        n = x1;
    }
    let lookup = |y: i32| FLOOR1_FROMDB_LOOKUP[y.clamp(0, 255) as usize];

    let mut x = x0;
    let mut y = y0;
    let mut err = 0;
    if x >= 0 && x < n {
        d[x as usize] *= lookup(y);
    }
    x += 1;
    while x < n {
        err += ady;
        if err >= adx {
            err -= adx;
            y += sy;
        } else {
            y += base;
        }
        if x >= 0 {
            d[x as usize] *= lookup(y);
        }
        x += 1;
    }
}

/// Synthesizes the linear floor curve over `n` bins from the fitted floor
/// `posts` (`floor1_inverse2`). `postlist` holds each post's bin position
/// (`postlist[0] == 0`, `postlist[1] == n`); `posts[i]` is the post height,
/// with `0x8000` marking a declined post that is skipped. Heights are scaled by
/// `mult` and clamped into the lookup before rendering.
///
/// Returns a per-bin curve of linear floor multipliers (all `1.0` if `posts` is
/// empty or the sizes are inconsistent).
#[must_use]
pub fn render_floor1(postlist: &[i32], posts: &[i32], mult: i32, n: usize) -> Vec<f32> {
    let mut out = vec![1.0f32; n];
    if postlist.len() != posts.len() || postlist.len() < 2 || n == 0 {
        return out;
    }

    // Posts in ascending bin-position order (the fitter's forward index).
    let mut order: Vec<usize> = (0..postlist.len()).collect();
    order.sort_by_key(|&i| postlist[i]);

    let clamp_y = |raw: i32| (raw * mult).clamp(0, 255);

    let mut lx = 0i32;
    let mut ly = clamp_y(posts[order[0]] & 0x7fff);
    let mut hx = 0i32;

    for &current in order.iter().skip(1) {
        let raw = posts[current];
        // A post carrying the 0x8000 declined flag is not a vertex.
        if raw & 0x7fff != raw {
            continue;
        }
        hx = postlist[current];
        let hy = clamp_y(raw & 0x7fff);
        render_line(&mut out, lx, hx, ly, hy);
        lx = hx;
        ly = hy;
    }

    // Hold the last value out to the end of the block.
    let tail = FLOOR1_FROMDB_LOOKUP[ly.clamp(0, 255) as usize];
    for slot in out.iter_mut().skip(hx.max(0) as usize) {
        *slot *= tail;
    }
    out
}

/// Divides the MDCT spectrum by the floor curve to form the residue the encoder
/// quantizes (`residue[i] = mdct[i] / floor[i]`). Bins with a non-positive
/// floor pass through unchanged (the floor is always positive in practice).
#[must_use]
pub fn spectral_residue(mdct: &[f32], floor: &[f32]) -> Vec<f32> {
    mdct.iter()
        .zip(floor)
        .map(|(&m, &f)| if f > 0.0 { m / f } else { m })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup_runs_from_silence_to_unity() {
        assert!(FLOOR1_FROMDB_LOOKUP[0] > 0.0 && FLOOR1_FROMDB_LOOKUP[0] < 1e-6);
        assert_eq!(FLOOR1_FROMDB_LOOKUP[255], 1.0);
        // The lookup is monotonically increasing in dB.
        for w in FLOOR1_FROMDB_LOOKUP.windows(2) {
            assert!(w[1] > w[0], "lookup not monotonic");
        }
    }

    #[test]
    fn flat_posts_render_a_flat_floor() {
        // Two posts at the same height -> a constant floor across the block.
        let n = 64;
        let postlist = [0, n as i32, 32];
        let posts = [40, 40, 40];
        let floor = render_floor1(&postlist, &posts, 1, n);
        let expected = FLOOR1_FROMDB_LOOKUP[40];
        for (i, &v) in floor.iter().enumerate() {
            assert!((v - expected).abs() < 1e-6, "bin {i}: {v}");
        }
    }

    #[test]
    fn rising_posts_render_a_rising_floor() {
        let n = 128;
        let postlist = [0, n as i32, 64];
        let posts = [20, 200, 110]; // low at DC, high at Nyquist
        let floor = render_floor1(&postlist, &posts, 1, n);
        assert!(
            floor[1] < floor[120],
            "floor did not rise: {} {}",
            floor[1],
            floor[120]
        );
        assert!(floor.iter().all(|v| v.is_finite()));
    }

    #[test]
    fn declined_posts_are_skipped() {
        // The midpoint post is declined; the floor interpolates 0 -> Nyquist.
        let n = 64;
        let postlist = [0, n as i32, 32];
        let posts = [40, 40, 100 | 0x8000];
        let floor = render_floor1(&postlist, &posts, 1, n);
        let expected = FLOOR1_FROMDB_LOOKUP[40];
        for &v in &floor {
            assert!(
                (v - expected).abs() < 1e-6,
                "declined post affected the floor"
            );
        }
    }

    #[test]
    fn residue_then_floor_reconstructs_the_spectrum() {
        let mdct = vec![0.5f32, -1.2, 0.03, 2.0, -0.7];
        let floor = vec![0.1f32, 0.4, 0.02, 1.5, 0.25];
        let residue = spectral_residue(&mdct, &floor);
        for i in 0..mdct.len() {
            assert!((residue[i] * floor[i] - mdct[i]).abs() < 1e-6, "bin {i}");
        }
    }

    #[test]
    fn render_is_safe_for_empty_or_tiny_inputs() {
        assert_eq!(render_floor1(&[], &[], 1, 8), vec![1.0; 8]);
        assert_eq!(render_floor1(&[0, 8], &[10, 10], 1, 0), Vec::<f32>::new());
    }
}
