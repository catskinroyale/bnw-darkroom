//! Synthetic scene-linear test chart.
//!
//! Top: 21 half-stop steps around mid-grey (0.18 · 2^-7 … 0.18 · 2^3).
//! Middle: coloured patches of roughly equal luminance, which only a spectral model
//! separates. Bottom: a smooth log gradient, fine bars and a dark disc on a bright
//! field for edge effects.

use bw_core::Plane;
use bw_negative::SceneImage;

pub fn synthetic_scene(width: usize, height: usize) -> SceneImage {
    // Linear Rec.709 patches; luminances are all close to 0.18.
    const PATCHES: [[f32; 3]; 6] = [
        [0.55, 0.08, 0.06], // red
        [0.10, 0.22, 0.05], // green
        [0.10, 0.17, 0.70], // blue
        [0.22, 0.19, 0.04], // yellow-brown
        [0.30, 0.15, 0.10], // skin-like
        [0.12, 0.18, 0.40], // sky-like
    ];
    let band = height / 3;
    let pixel = |x: usize, y: usize| -> [f32; 3] {
        let u = x as f32 / width as f32;
        if y < band {
            let step = ((u * 21.0) as i32).min(20);
            let v = 0.18 * 2f32.powf((step - 14) as f32 / 2.0);
            return [v; 3];
        }
        if y < 2 * band {
            return PATCHES[((u * 6.0) as usize).min(5)];
        }
        let yy = (y - 2 * band) as f32 / band as f32;
        if yy < 0.4 {
            // 10-stop log gradient.
            let v = 0.18 * 2f32.powf(-7.0 + 10.0 * u);
            return [v; 3];
        }
        if u < 0.5 {
            // Bars with increasing frequency on mid-grey.
            let freq = 4.0 + 60.0 * (u / 0.5);
            let v = if (u * freq * 6.0).fract() < 0.5 {
                0.05
            } else {
                0.7
            };
            return [v; 3];
        }
        // Dark disc on bright field.
        let (cx, cy) = (0.75, 0.7);
        let dx = (u - cx) * width as f32 / band as f32;
        let dy = yy - cy;
        let v = if (dx * dx + dy * dy).sqrt() < 0.22 {
            0.01
        } else {
            0.9
        };
        [v; 3]
    };
    let plane = |c: usize| {
        Plane::from_fn(width, height, |x, y| pixel(x, y)[c]).expect("non-zero chart size")
    };
    SceneImage::Rgb {
        r: plane(0),
        g: plane(1),
        b: plane(2),
    }
}
