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
        "craft-paper" => craft_paper(&mut buf, w, h, s, inten),
        "notebook" => notebook(&mut buf, w, h, s, inten),
        "parchment" => parchment(&mut buf, w, h, s, inten),
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
// Preset 2: coarse craft paper (warm kraft tint + fibers + flecks)
// ---------------------------------------------------------------------------
fn craft_paper(buf: &mut [u8], w: usize, h: usize, s: f32, inten: f32) {
    for y in 0..h {
        let fy = y as f32;
        for x in 0..w {
            let fx = x as f32;
            let i = (y * w + x) * 4;
            let blotch = fbm(fx * 0.0045 * s, fy * 0.0045 * s, 4, 201) - 0.5;
            let fh = fbm(fx * 0.045 * s, fy * 0.011 * s, 2, 202);
            let fv = fbm(fx * 0.010 * s, fy * 0.050 * s, 2, 203);
            let fleck = hash2(x as i32, y as i32, 204);

            // kraft base color, modulated by large-scale blotches
            let (mut br, mut bg, mut bb, mut a) = (
                194.0 + blotch * 60.0,
                164.0 + blotch * 48.0,
                122.0 + blotch * 30.0,
                inten * 108.0,
            );
            // horizontal fibers
            if fh > 0.60 {
                let k = (fh - 0.60) * inten;
                br += k * 60.0; bg += k * 70.0; bb += k * 60.0;
                a += k * 130.0;
            } else if fh < 0.36 {
                let k = (0.36 - fh) * inten;
                br -= k * 60.0; bg -= k * 55.0; bb -= k * 40.0;
                a += k * 110.0;
            }
            // vertical fibers
            if fv > 0.63 {
                let k = (fv - 0.63) * inten;
                br += k * 50.0; bg += k * 55.0; bb += k * 45.0;
                a += k * 90.0;
            }
            // dark flecks
            if fleck > 0.9955 {
                br = 58.0; bg = 44.0; bb = 26.0;
                a = inten * 190.0;
            }
            put_px(buf, i, bb, bg, br, a);
        }
    }
}

// ---------------------------------------------------------------------------
// Preset 3: notebook paper (ruled lines + red margin + paper wash)
// ---------------------------------------------------------------------------
fn notebook(buf: &mut [u8], w: usize, h: usize, s: f32, inten: f32) {
    let spacing = (32.0 * s).max(24.0);
    let line_th = (1.4 * s).max(1.0);
    let margin_x = 96.0 * s;
    let margin_th = 1.1 * s;
    for y in 0..h {
        let fy = y as f32;
        let row = fy % spacing;
        for x in 0..w {
            let fx = x as f32;
            let i = (y * w + x) * 4;
            if row < line_th {
                // ruled line — pale blue
                put_px(buf, i, 235.0, 160.0, 120.0, inten * 190.0);
            } else if (fx - margin_x).abs() < margin_th {
                // margin line — soft red
                put_px(buf, i, 130.0, 120.0, 235.0, inten * 205.0);
            } else {
                // faint paper wash + micro grain
                let g = hash2(x as i32, y as i32, 303);
                let a = inten * 26.0 + (g - 0.5) * inten * 22.0;
                put_px(buf, i, 244.0, 246.0, 250.0, a);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Preset 4: parchment / aged paper
// ---------------------------------------------------------------------------
fn parchment(buf: &mut [u8], w: usize, h: usize, s: f32, inten: f32) {
    let edge_band = (90.0 * s).max(30.0);
    for y in 0..h {
        let fy = y as f32;
        let dy_edge = (y as f32).min((h as f32 - 1.0 - fy).max(0.0));
        for x in 0..w {
            let fx = x as f32;
            let dx_edge = fx.min((w as f32 - 1.0 - fx).max(0.0));
            let i = (y * w + x) * 4;
            let b1 = fbm(fx * 0.0035 * s, fy * 0.0035 * s, 4, 401);
            let b2 = fbm(fx * 0.013 * s, fy * 0.013 * s, 3, 402);
            let fi = fbm(fx * 0.09 * s, fy * 0.022 * s, 2, 403);
            let edge = {
                let d = dx_edge.min(dy_edge);
                (1.0 - d / edge_band).clamp(0.0, 1.0)
            };
            // aged base: warm cream toward brown where b1 is low
            let warm = (0.5 - b1).clamp(0.0, 1.0);
            let mut br = 233.0 - warm * 63.0;
            let mut bg = 216.0 - warm * 86.0;
            let mut bb = 180.0 - warm * 100.0;
            let mut a = inten * (76.0 + warm * 64.0);
            // deeper stains
            if b2 < 0.42 {
                let k = (0.42 - b2) * inten;
                br -= k * 113.0; bg -= k * 128.0; bb -= k * 132.0;
                a += k * 190.0;
            }
            // edge vignette
            if edge > 0.0 {
                let k = edge * inten;
                br -= k * 137.0; bg -= k * 142.0; bb -= k * 136.0;
                a += k * 97.0;
            }
            // fibers
            if fi > 0.63 {
                let k = (fi - 0.63) * inten;
                br += k * 22.0; bg += k * 38.0; bb += k * 55.0;
                a += k * 90.0;
            } else if fi < 0.34 {
                let k = (0.34 - fi) * inten;
                br -= k * 50.0; bg -= k * 42.0; bb -= k * 30.0;
                a += k * 60.0;
            }
            put_px(buf, i, bb, bg, br, a);
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
