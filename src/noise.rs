//! noise.rs — deterministic value noise + fBm used by the procedural paper
//! textures. No RNG state: every pixel is a pure function of (x, y, seed),
//! so textures are stable across renders and machines.

pub fn hash2(ix: i32, iy: i32, seed: u32) -> f32 {
    let mut h = (ix as u32).wrapping_mul(0x27d4eb2d)
        ^ (iy as u32).wrapping_mul(0x165667b1)
        ^ seed.wrapping_mul(0x9e3779b9);
    h ^= h >> 15;
    h = h.wrapping_mul(0x85ebca6b);
    h ^= h >> 13;
    h = h.wrapping_mul(0xc2b2ae35);
    h ^= h >> 16;
    (h & 0x7F_FFFF) as f32 / 8_388_607.0 // [0, 1]
}

fn smooth(t: f32) -> f32 { t * t * (3.0 - 2.0 * t) }

/// 2D value noise with smooth interpolation, output in [0, 1].
pub fn vnoise(x: f32, y: f32, seed: u32) -> f32 {
    let xif = x.floor();
    let yif = y.floor();
    let xi = xif as i32;
    let yi = yif as i32;
    let fx = smooth(x - xif);
    let fy = smooth(y - yif);
    let a = hash2(xi, yi, seed);
    let b = hash2(xi.wrapping_add(1), yi, seed);
    let c = hash2(xi, yi.wrapping_add(1), seed);
    let d = hash2(xi.wrapping_add(1), yi.wrapping_add(1), seed);
    a + (b - a) * fx + (c - a) * fy + (a - b - c + d) * fx * fy
}

/// Fractional Brownian motion built on vnoise, normalized to [0, 1].
pub fn fbm(x: f32, y: f32, octaves: u32, seed: u32) -> f32 {
    let mut sum = 0.0f32;
    let mut amp = 0.5f32;
    let mut norm = 0.0f32;
    let mut fx = x;
    let mut fy = y;
    for o in 0..octaves {
        sum += amp * vnoise(fx, fy, seed.wrapping_add(o.wrapping_mul(1013)));
        norm += amp;
        amp *= 0.5;
        fx *= 2.02;
        fy *= 2.02;
    }
    if norm > 0.0 { sum / norm } else { 0.5 }
}
