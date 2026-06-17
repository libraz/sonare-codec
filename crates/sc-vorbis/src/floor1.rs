//! Vorbis floor1 curve primitives.
//!
//! Hand-ported to safe Rust from libvorbis/aoTuV `lib/floor1.c`: the integer
//! line rasterizer (`render_point` / `render_line0`) that turns floor posts into
//! a per-bin floor curve, and the dB quantizer (`vorbis_dBquant`) used when
//! fitting that curve. Derivative work of libvorbis/aoTuV (BSD-3-Clause); see
//! `LICENSE-THIRDPARTY`.

// Consumed by the floor1 fit/encode stage; the live encoder still ships via FFI.
#![allow(dead_code)]

/// Interpolates the integer floor value of the line `(x0,y0)-(x1,y1)` at `x`.
///
/// The high bit of `y0`/`y1` is a post "used" flag in libvorbis and is masked
/// off here exactly as in the C.
#[must_use]
pub fn render_point(x0: i32, x1: i32, y0: i32, y1: i32, x: i32) -> i32 {
    let y0 = y0 & 0x7fff;
    let y1 = y1 & 0x7fff;
    let dy = y1 - y0;
    let adx = x1 - x0;
    let ady = dy.abs();
    let err = ady * (x - x0);
    let off = err / adx;
    if dy < 0 {
        y0 - off
    } else {
        y0 + off
    }
}

/// Rasterizes the line `(x0,y0)-(x1,y1)` into `d[x0..min(n, x1)]` as integers.
///
/// This is the integer DDA libvorbis uses to build the log-domain floor mask.
pub fn render_line0(n: i32, x0: i32, x1: i32, y0: i32, y1: i32, d: &mut [i32]) {
    let dy = y1 - y0;
    let adx = x1 - x0;
    let mut ady = dy.abs();
    let base = dy / adx;
    let sy = if dy < 0 { base - 1 } else { base + 1 };
    let mut x = x0;
    let mut y = y0;
    let mut err = 0;

    ady -= (base * adx).abs();

    let n = n.min(x1);

    if x < n {
        d[x as usize] = y;
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
        d[x as usize] = y;
        x += 1;
    }
}

/// Quantizes a linear floor magnitude to libvorbis's dB index in `0..=1023`.
#[must_use]
pub fn vorbis_db_quant(x: f32) -> i32 {
    let i = (x * 7.3142857 + 1023.5) as i32;
    i.clamp(0, 1023)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_point_hits_endpoints() {
        assert_eq!(render_point(0, 128, 10, 200, 0), 10);
        assert_eq!(render_point(0, 128, 10, 200, 128), 200);
        // Decreasing line.
        assert_eq!(render_point(5, 50, 300, 100, 5), 300);
        assert_eq!(render_point(5, 50, 300, 100, 50), 100);
    }

    #[test]
    fn render_point_midpoint() {
        // Halfway along a 0..100 / 0..200 line is ~100.
        assert_eq!(render_point(0, 100, 0, 200, 50), 100);
    }

    #[test]
    fn render_line0_matches_render_point_within_one() {
        let cases = [
            (0, 128, 10, 200),
            (0, 100, 200, 5),
            (3, 64, 0, 63),
            (0, 200, 512, 1),
        ];
        for &(x0, x1, y0, y1) in &cases {
            let mut d = vec![-9999; (x1 + 1) as usize];
            render_line0(x1 + 1, x0, x1, y0, y1, &mut d);
            assert_eq!(d[x0 as usize], y0 & 0x7fff, "start value");
            for x in x0..x1 {
                let direct = render_point(x0, x1, y0, y1, x);
                assert!(
                    (d[x as usize] - direct).abs() <= 1,
                    "x={x}: dda={} direct={direct}",
                    d[x as usize]
                );
            }
        }
    }

    #[test]
    fn render_line0_respects_n_cap() {
        let mut d = vec![-1; 200];
        // n caps the fill below x1.
        render_line0(50, 0, 128, 10, 200, &mut d);
        assert_eq!(d[0], 10, "start value");
        assert_ne!(d[49], -1, "within n must be filled");
        assert_eq!(d[50], -1, "beyond n must be untouched");
        assert_eq!(d[127], -1, "beyond n must be untouched");
    }

    #[test]
    fn db_quant_clamps() {
        assert_eq!(vorbis_db_quant(0.0), 1023);
        assert_eq!(vorbis_db_quant(-1000.0), 0);
        assert_eq!(vorbis_db_quant(1000.0), 1023);
        // Mid-range monotonic.
        assert!(vorbis_db_quant(-50.0) < vorbis_db_quant(-40.0));
    }
}
