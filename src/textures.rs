//! textures.rs — procedural paper texture generators + custom image handling.
//!
//! All textures are rendered as **premultiplied BGRA** buffers sized to the
//! target monitor (DPI-aware). A "master" is generated at full intensity;
//! global opacity is applied later by linear scaling of the premultiplied
//! channels, which is mathematically exact for source-over compositing.

use crate::noise::{fbm, hash2};

/// Write a premultiplied BGRA pixel (channels clamped so rgb <= a <= 255).
#[inline]
fn put_px(buf: &mut [u8], i: usize, b: f32, g: f32, r: f32, a: f32) {
    let a = if a < 0.0 { 0.0 } else if a > 255.0 { 255.0 } else { a };
    let b = if b < 0.0 { 0.0 } else if b > a { a } else { b };
    let g = if g < 0.0 { 0.0 } else if g > a { a } else { g };
    let r = if r < 0.0 { 0.0 } else if r > a { a } else { r };
    buf[i] = b as u8;
    buf[i + 1] = g as u8;
    buf[i + 2] = r as u8;
    buf[i + 3] = a as u8;
}

/// Generate a procedural preset texture master. `kind` is one of the preset
/// ids; unknown ids fall back to fine grain.
pub fn generate(kind: &str, w: usize, h: usize, dpi: u32, intensity: u32) -> Vec<u8> {
    let s = (dpi as f32 / 96.0).clamp(1.0, 3.2); // DPI scale factor (100%-300%+)
    let inten = (intensity as f32 / 100.0).clamp(0.08, 1.0);
    let mut buf = vec![0u8; w * 4 * h];
    match kind {
        "cotton-paper" => cotton_paper(&mut buf, w, h, s, inten),
        "drawing-paper" => drawing_paper(&mut buf, w, h, s, inten),
        "book-paper" => book_paper(&mut buf, w, h, s, inten),
        "recycled-paper" => recycled_paper(&mut buf, w, h, s, inten),
        "watercolor-paper" => watercolor_paper(&mut buf, w, h, s, inten),
        "xuan-paper" => xuan_paper(&mut buf, w, h, s, inten),
        "offset-paper" => offset_paper(&mut buf, w, h, s, inten),
        _ => fine_grain(&mut buf, w, h, s, inten),
    }
    buf
}

// ---------------------------------------------------------------------------
// Preset 1: fine paper grain (subtle film-grain over white paper)
// ---------------------------------------------------------------------------
fn fine_grain(buf: &mut [u8], w: usize, h: usize, s: f32, inten: f32) {
    let fq = 0.85 * s;
    for y in 0..h {
        let fy = y as f32;
        for x in 0..w {
            let fx = x as f32;
            let i = (y * w + x) * 4;
            let n = fbm(fx * fq, fy * fq, 3, 101);
            let g = hash2(x as i32, y as i32, 77);
            let d = (n - 0.5) * inten;
            if d > 0.0 {
                let a = d * 2.0 * 140.0;
                put_px(buf, i, 255.0, 255.0, 255.0, a); // bright speck
            } else {
                let a = -d * 2.0 * 120.0;
                put_px(buf, i, 38.0, 36.0, 34.0, a); // dark speck
            }
            if g > 0.9965 {
                put_px(buf, i, 255.0, 255.0, 255.0, inten * 150.0); // sparkle
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Paper surfaces: local relief and sparse fibers rather than opaque backgrounds
// ---------------------------------------------------------------------------
fn paper_relief(buf: &mut [u8], i: usize, relief: f32, inten: f32) {
    let a = (relief.abs() * 260.0 * inten).min(180.0);
    let (b, g, r) = if relief >= 0.0 {
        (255.0, 255.0, 255.0)
    } else {
        (72.0, 74.0, 76.0)
    };
    // Local light and shadow keep the underlying text clear.
    let premul = a / 255.0;
    put_px(buf, i, b * premul, g * premul, r * premul, a);
}

fn cotton_paper(buf: &mut [u8], w: usize, h: usize, s: f32, inten: f32) {
    for y in 0..h {
        let fy = y as f32 / s;
        for x in 0..w {
            let fx = x as f32 / s;
            let formation = fbm(fx * 0.008, fy * 0.008, 2, 501) - 0.5;
            let pulp = fbm(fx * 0.085, fy * 0.085, 2, 502) - 0.5;
            let grain = fbm(fx * 0.64, fy * 0.64, 2, 504) - 0.5;
            let relief = formation * 0.08 + pulp * 0.18 + grain * 0.70;
            paper_relief(buf, (y * w + x) * 4, relief, inten);
        }
    }
    paper_fibers(buf, w, h, s, inten, false);
}

fn paper_fibers(buf: &mut [u8], w: usize, h: usize, s: f32, inten: f32, long: bool) {
    let (spacing, min_len, len_span, width, strength, seed) = if long {
        (18.0, 3.5, 8.5, 0.5, 60.0, 905)
    } else {
        (12.0, 1.2, 2.8, 0.7, 75.0, 505)
    };
    let cell = (spacing * s).round() as usize;
    for cy in 0..h.div_ceil(cell) {
        for cx in 0..w.div_ceil(cell) {
            let hash = |seed| hash2(cx as i32, cy as i32, seed);
            let fx = (cx as f32 + hash(seed)) * cell as f32;
            let fy = (cy as f32 + hash(seed + 1)) * cell as f32;
            let (dy, dx) = (hash(seed + 2) * std::f32::consts::TAU).sin_cos();
            let half_len = (min_len + hash(seed + 3) * len_span) * s;
            let radius = half_len + s;
            let x0 = (fx - radius).max(0.0) as usize;
            let x1 = ((fx + radius).ceil() as usize).min(w);
            let y0 = (fy - radius).max(0.0) as usize;
            let y1 = ((fy + radius).ceil() as usize).min(h);
            let color = if hash(seed + 4) > 0.5 { 255.0 } else { 80.0 };
            for y in y0..y1 {
                for x in x0..x1 {
                    let px = x as f32 - fx;
                    let py = y as f32 - fy;
                    let along = (px * dx + py * dy).abs() / half_len;
                    let across = (px * dy - py * dx).abs() / (width * s);
                    let a = (1.0 - along).max(0.0) * (1.0 - across).max(0.0)
                        * inten * strength;
                    let i = (y * w + x) * 4;
                    let keep = 1.0 - a / 255.0;
                    for c in 0..3 {
                        buf[i + c] = (color * a / 255.0 + buf[i + c] as f32 * keep) as u8;
                    }
                    buf[i + 3] = (a + buf[i + 3] as f32 * keep) as u8;
                }
            }
        }
    }
}

fn drawing_paper(buf: &mut [u8], w: usize, h: usize, s: f32, inten: f32) {
    for y in 0..h {
        let fy = y as f32 / s;
        for x in 0..w {
            let fx = x as f32 / s;
            let tooth = fbm(fx * 0.36, fy * 0.36, 2, 601);
            let lit = fbm((fx - 0.8) * 0.36, (fy - 0.6) * 0.36, 2, 601);
            let grain = fbm(fx * 0.92, fy * 0.92, 2, 602) - 0.5;
            let formation = fbm(fx * 0.026, fy * 0.026, 2, 603) - 0.5;
            let relief = (tooth - 0.5) * 0.52 + (lit - tooth) * 1.8
                + grain * 0.25 + formation * 0.07;
            paper_relief(buf, (y * w + x) * 4, relief * 0.75, inten);
        }
    }
}

fn book_paper(buf: &mut [u8], w: usize, h: usize, s: f32, inten: f32) {
    let tint_alpha = 32.0 * inten;
    let keep = 1.0 - tint_alpha / 255.0;
    for y in 0..h {
        let fy = y as f32 / s;
        for x in 0..w {
            let fx = x as f32 / s;
            let pulp = fbm(fx * 0.12, fy * 0.12, 2, 701) - 0.5;
            let grain = fbm(fx * 0.88, fy * 0.88, 2, 702) - 0.5;
            let i = (y * w + x) * 4;
            paper_relief(buf, i, pulp * 0.09 + grain * 0.38, inten);
            // A faint ivory glaze, composited in premultiplied space.
            for (c, color) in [222.0, 244.0, 254.0].iter().enumerate() {
                buf[i + c] = (color * tint_alpha / 255.0 + buf[i + c] as f32 * keep) as u8;
            }
            buf[i + 3] = (tint_alpha + buf[i + 3] as f32 * keep) as u8;
        }
    }
}

fn recycled_paper(buf: &mut [u8], w: usize, h: usize, s: f32, inten: f32) {
    for y in 0..h {
        let fy = y as f32 / s;
        for x in 0..w {
            let fx = x as f32 / s;
            let pulp = fbm(fx * 0.10, fy * 0.10, 2, 801) - 0.5;
            let grain = fbm(fx * 0.75, fy * 0.75, 2, 802) - 0.5;
            let inclusion = ((fbm(fx * 0.42, fy * 0.42, 2, 803) - 0.73) * 5.0)
                .clamp(0.0, 1.0);
            let relief = pulp * 0.32 + grain * 0.44 - inclusion * 0.45;
            paper_relief(buf, (y * w + x) * 4, relief, inten);
        }
    }
}

fn watercolor_paper(buf: &mut [u8], w: usize, h: usize, s: f32, inten: f32) {
    for y in 0..h {
        let fy = y as f32 / s;
        for x in 0..w {
            let fx = x as f32 / s;
            // Warp the tooth so pressed-paper pits do not form a regular grid.
            let tx = fx + (fbm(fx * 0.027, fy * 0.027, 2, 1001) - 0.5) * 6.0;
            let ty = fy + (fbm(fx * 0.027, fy * 0.027, 2, 1002) - 0.5) * 6.0;
            let tooth = fbm(tx * 0.26, ty * 0.26, 2, 1003);
            let lit = fbm((tx - 0.9) * 0.26, (ty - 0.7) * 0.26, 2, 1003);
            let grain = fbm(fx * 0.80, fy * 0.80, 2, 1004) - 0.5;
            let relief = (tooth - 0.5) * 0.55 + (lit - tooth) * 0.55 + grain * 0.28;
            paper_relief(buf, (y * w + x) * 4, relief * 0.58, inten);
        }
    }
}

fn xuan_paper(buf: &mut [u8], w: usize, h: usize, s: f32, inten: f32) {
    for y in 0..h {
        let fy = y as f32 / s;
        for x in 0..w {
            let fx = x as f32 / s;
            let formation = fbm(fx * 0.016, fy * 0.016, 2, 901) - 0.5;
            let pulp = fbm(fx * 0.13, fy * 0.13, 2, 902) - 0.5;
            let grain = fbm(fx * 0.70, fy * 0.70, 2, 903) - 0.5;
            paper_relief(buf, (y * w + x) * 4,
                         formation * 0.10 + pulp * 0.17 + grain * 0.40, inten);
        }
    }
    paper_fibers(buf, w, h, s, inten, true);
}

// ---------------------------------------------------------------------------
// Matte offset-printing stock: visible warm-gray paper and compressed-pulp pores.
fn offset_paper(buf: &mut [u8], w: usize, h: usize, s: f32, inten: f32) {
    for y in 0..h {
        let fy = y as f32 / s;
        for x in 0..w {
            let fx = x as f32 / s;
            let formation = fbm(fx * 0.018, fy * 0.018, 2, 1101) - 0.5;
            let pulp = fbm(fx * 0.23, fy * 0.30, 2, 1102) - 0.5;
            let grain = fbm(fx * 0.90, fy * 0.90, 2, 1103) - 0.5;
            let a = (50.0 + formation * 6.0 + pulp * 20.0 + grain * 56.0)
                .clamp(18.0, 72.0) * inten;
            let premul = a / 255.0;
            // A darker translucent pigment makes white paper visible without whitening black text.
            put_px(buf, (y * w + x) * 4, 54.0 * premul, 70.0 * premul,
                   84.0 * premul, a);
        }
    }
}

// ---------------------------------------------------------------------------
// Custom user textures
// ---------------------------------------------------------------------------
/// Decoded custom image (premultiplied BGRA straight from GDI+ PARGB lock).
pub struct CustomImage {
    pub w: usize,
    pub h: usize,
    pub pixels: Vec<u8>, // premultiplied BGRA, w*4*h
}

impl CustomImage {
    #[inline]
    fn px(&self, x: usize, y: usize) -> (f32, f32, f32, f32) {
        let i = (y * self.w + x) * 4;
        (
            self.pixels[i] as f32,
            self.pixels[i + 1] as f32,
            self.pixels[i + 2] as f32,
            self.pixels[i + 3] as f32,
        )
    }
}

/// Build a texture master from a custom image:
/// - large images are scaled to *cover* the screen (center crop, bilinear)
/// - small (seamless) images are tiled at native size
/// Intensity scales the alpha channel (1.0 at the default 60%).
pub fn custom_master(src: &CustomImage, w: usize, h: usize, intensity: u32) -> Vec<u8> {
    let mut out = vec![0u8; w * 4 * h];
    let alpha_scale = (intensity as f32 / 60.0).clamp(0.15, 1.6);
    let sw = src.w as f32;
    let sh = src.h as f32;
    let tile = sw * 2.0 <= w as f32 && sh * 2.0 <= h as f32;

    if tile {
        // seamless tiling
        for y in 0..h {
            let sy = y % src.h;
            for x in 0..w {
                let sx = x % src.w;
                let si = (sy * src.w + sx) * 4;
                let di = (y * w + x) * 4;
                let a = (src.pixels[si + 3] as f32 * alpha_scale).min(255.0);
                out[di + 3] = (a + 0.5) as u8;
                if alpha_scale == 1.0 {
                    out[di] = src.pixels[si];
                    out[di + 1] = src.pixels[si + 1];
                    out[di + 2] = src.pixels[si + 2];
                } else {
                    for c in 0..3 {
                        let v = (src.pixels[si + c] as f32 * alpha_scale).min(a);
                        out[di + c] = (v + 0.5) as u8;
                    }
                }
            }
        }
    } else {
        // cover-scale with center crop, bilinear sampling
        let scale = ((w as f32) / sw).max((h as f32) / sh);
        let draw_w = sw * scale;
        let draw_h = sh * scale;
        let off_x = (draw_w - w as f32) / 2.0;
        let off_y = (draw_h - h as f32) / 2.0;
        let max_x = src.w.saturating_sub(1) as f32;
        let max_y = src.h.saturating_sub(1) as f32;
        for y in 0..h {
            let gy = ((off_y + y as f32) / scale - 0.5).clamp(0.0, max_y);
            let y0 = gy.floor();
            let ty = gy - y0;
            let yi0 = y0 as usize;
            let yi1 = (yi0 + 1).min(src.h - 1);
            for x in 0..w {
                let gx = ((off_x + x as f32) / scale - 0.5).clamp(0.0, max_x);
                let x0 = gx.floor();
                let tx = gx - x0;
                let xi0 = x0 as usize;
                let xi1 = (xi0 + 1).min(src.w - 1);
                let p00 = src.px(xi0, yi0);
                let p10 = src.px(xi1, yi0);
                let p01 = src.px(xi0, yi1);
                let p11 = src.px(xi1, yi1);
                let di = (y * w + x) * 4;
                for c in 0..4usize {
                    let pick = |p: (f32, f32, f32, f32)| -> f32 {
                        match c {
                            0 => p.0, 1 => p.1, 2 => p.2, _ => p.3,
                        }
                    };
                    let v = pick(p00) * (1.0 - tx) * (1.0 - ty)
                        + pick(p10) * tx * (1.0 - ty)
                        + pick(p01) * (1.0 - tx) * ty
                        + pick(p11) * tx * ty;
                    let v = v * alpha_scale;
                    out[di + c] = if v > 255.0 { 255 } else { (v + 0.5) as u8 };
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_paper_surfaces_are_stable_distinct_and_premultiplied() {
        for dpi in [96, 192, 288] {
            let fine = generate("fine-grain", 128, 96, dpi, 60);
            let mut previous = vec![fine];
            for kind in ["cotton-paper", "drawing-paper", "book-paper", "recycled-paper",
                         "watercolor-paper", "xuan-paper", "offset-paper"] {
                let pixels = generate(kind, 128, 96, dpi, 60);
                assert!(previous.iter().all(|other| other != &pixels), "duplicate preset: {kind}");
                assert_eq!(pixels.len(), 128 * 96 * 4);
                assert_eq!(pixels, generate(kind, 128, 96, dpi, 60));
                assert!(pixels.chunks_exact(4).all(|p| p[..3].iter().all(|c| *c <= p[3])));
                let alpha_sum = |intensity| -> u64 {
                    generate(kind, 128, 96, dpi, intensity)
                        .chunks_exact(4).map(|p| p[3] as u64).sum()
                };
                assert!(alpha_sum(100) > alpha_sum(60));
                assert!(alpha_sum(60) > alpha_sum(10));
                previous.push(pixels);
                for (w, h) in [(0, 0), (0, 4), (4, 0), (1, 1)] {
                    assert_eq!(generate(kind, w, h, dpi, 100).len(), w * h * 4);
                }
            }
        }
    }

    #[test]
    fn offset_paper_is_visible_after_opacity_scaling_and_keeps_ink_dark() {
        for dpi in [96, 192, 288] {
            for (intensity, opacity, maximum_mean, minimum_grain) in
                [(60, 0.35, 249.0, 4), (100, 0.90, 225.0, 18)] {
                let pixels = generate("offset-paper", 128, 96, dpi, intensity);
                let mut darkest = 255;
                let mut lightest = 0;
                let mut sum = 0u64;
                for p in pixels.chunks_exact(4) {
                    // Match render_overlay's premultiplied-channel rounding, then composite over white/black.
                    let scaled = |c: u8| (c as f32 * opacity + 0.5) as u16;
                    let alpha = scaled(p[3]);
                    for c in &p[..3] {
                        let ink = scaled(*c);
                        let white = ink + 255 - alpha;
                        assert!(ink <= 24, "overlay washes out dark ink");
                        assert!(white >= 198, "paper surface is too dark");
                    }
                    let white = scaled(p[1]) + 255 - alpha;
                    darkest = darkest.min(white);
                    lightest = lightest.max(white);
                    sum += white as u64;
                }
                let mean = sum as f64 / (pixels.len() / 4) as f64;
                assert!(mean <= maximum_mean, "paper is indistinguishable from white: {mean}");
                assert!(lightest - darkest >= minimum_grain, "paper grain is too faint");
                assert!(lightest - darkest <= 42, "grain is too high-contrast");
            }
        }
    }
}
