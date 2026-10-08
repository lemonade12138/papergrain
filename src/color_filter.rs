//! Independent translucent color tint beneath the optional paper layer.
use crate::config::Config;

pub fn compose(dst: &mut [u8], paper: &[u8], cfg: &Config) {
    if cfg.enabled && paper.len() == dst.len() {
        let opacity = cfg.opacity as f32 / 100.0;
        for (out, source) in dst.iter_mut().zip(paper) {
            *out = (*source as f32 * opacity + 0.5).min(255.0) as u8;
        }
    } else {
        dst.fill(0);
    }
    if !cfg.filter_active() { return; }
    // 100% depth is still translucent so the tint cannot become a solid screen.
    let alpha = (cfg.filter_depth.min(100) * 72 + 50) / 100;
    let color = cfg.filter_color();
    let tint = [
        (((color >> 16) & 255) * alpha + 127) / 255,
        (((color >> 8) & 255) * alpha + 127) / 255,
        ((color & 255) * alpha + 127) / 255,
        alpha,
    ];
    for pixel in dst.chunks_exact_mut(4) {
        // Paper is the foreground; deeper tint must not fade its grain.
        let keep = 255 - pixel[3] as u32;
        for channel in 0..4 {
            pixel[channel] = (pixel[channel] as u32 + (tint[channel] * keep + 127) / 255) as u8;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{FILTER_KINDS, FILTER_COLORS};

    #[test]
    fn paper_is_above_tint_and_its_grain_is_not_faded_by_color_depth() {
        let mut cfg = Config::default();
        cfg.opacity = 90;
        cfg.filter_enabled = true;
        cfg.filter_depth = 100;
        let mut output = [0; 4];
        compose(&mut output, &[20, 40, 60, 80], &cfg);
        assert_eq!(output, [47, 81, 105, 124]);

        let paper = [0, 0, 0, 100, 80, 90, 100, 100];
        for kind in FILTER_KINDS {
            cfg.set_filter_kind(kind);
            for depth in 0..=100 {
                cfg.filter_depth = depth;
                let mut output = [0; 8];
                compose(&mut output, &paper, &cfg);
                assert_eq!(output[4] - output[0], 72);
                assert_eq!(output[5] - output[1], 81);
                assert_eq!(output[6] - output[2], 90);
                assert_eq!(output[7], output[3]);
            }
        }
    }

    #[test]
    fn paper_and_color_are_independent_and_zero_depth_is_exactly_unchanged() {
        let paper = [10, 15, 20, 40, 0, 0, 0, 0];
        let mut cfg = Config::default();
        cfg.opacity = 35;
        let mut plain = [0; 8];
        compose(&mut plain, &paper, &cfg);
        assert_eq!(plain, [4, 5, 7, 14, 0, 0, 0, 0]);
        cfg.filter_enabled = true;
        cfg.filter_depth = 0;
        let mut output = [0; 8];
        compose(&mut output, &paper, &cfg);
        assert_eq!(output, plain);
        cfg.filter_depth = 100;
        compose(&mut output, &paper, &cfg);
        assert_ne!(output, plain);
        cfg.enabled = false;
        compose(&mut output, &paper, &cfg);
        assert_eq!(&output[..4], &output[4..]);
        let color_only = output;
        cfg.opacity = 90;
        cfg.texture = "xuan-paper".into();
        cfg.intensity = 100;
        compose(&mut output, &[], &cfg);
        assert_eq!(output, color_only);
        cfg.filter_enabled = false;
        compose(&mut output, &paper, &cfg);
        assert_eq!(output, [0; 8]);
        cfg.enabled = true;
        cfg.opacity = 100;
        compose(&mut output, &paper, &cfg);
        assert_eq!(output, paper);
    }

    #[test]
    fn all_colors_and_depths_remain_premultiplied_and_translucent() {
        for kind in FILTER_KINDS {
            for color in [0, 0xFFFFFF, FILTER_COLORS[0], FILTER_COLORS[1]] {
                let mut cfg = Config::default();
                cfg.opacity = 100;
                cfg.filter_enabled = true;
                cfg.set_filter_kind(kind);
                cfg.filter_custom_color = color;
                for depth in 0..=100 {
                    cfg.filter_depth = depth;
                    for alpha in 0..=255u8 {
                        let paper = [0, alpha / 2, alpha, alpha];
                        let mut output = [0; 4];
                        compose(&mut output, &paper, &cfg);
                        assert!(output[..3].iter().all(|channel| *channel <= output[3]));
                    }
                    cfg.enabled = false;
                    let mut output = [0; 4];
                    compose(&mut output, &[], &cfg);
                    assert!(output[3] <= 72);
                    cfg.enabled = true;
                }
            }
        }
    }
}
