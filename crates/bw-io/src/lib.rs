//! Image input, resampling and output.
//!
//! The engine works on scene-linear planes. This crate converts files to and from
//! that representation; it contains no photographic modelling.

use std::path::Path;

use bw_core::Plane;
use bw_negative::{Negative, SceneImage};
use image::{DynamicImage, ImageBuffer, Luma};
use rayon::prelude::*;

/// File extensions the loader accepts.
pub const SUPPORTED_EXTENSIONS: [&str; 5] = ["tif", "tiff", "png", "jpg", "jpeg"];

pub fn is_supported_image(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| SUPPORTED_EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()))
}

/// How input pixel values relate to scene light.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Encoding {
    /// 8-bit files are sRGB-encoded; 16-bit and float files are linear.
    Auto,
    Srgb,
    Linear,
}

pub fn srgb_to_linear(c: f32) -> f32 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

pub fn linear_to_srgb(c: f32) -> f32 {
    let c = c.clamp(0.0, 1.0);
    if c <= 0.003_130_8 {
        12.92 * c
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}

fn to_u8(v: f32) -> u8 {
    (v * 255.0 + 0.5) as u8
}

pub fn load_scene(
    path: &Path,
    encoding: Encoding,
) -> Result<SceneImage, Box<dyn std::error::Error + Send + Sync>> {
    let img = image::open(path)?;
    let eight_bit = matches!(
        img,
        DynamicImage::ImageLuma8(_)
            | DynamicImage::ImageLumaA8(_)
            | DynamicImage::ImageRgb8(_)
            | DynamicImage::ImageRgba8(_)
    );
    let srgb = match encoding {
        Encoding::Srgb => true,
        Encoding::Linear => false,
        Encoding::Auto => eight_bit,
    };
    let decode = |v: f32| if srgb { srgb_to_linear(v) } else { v };
    let (w, h) = (img.width() as usize, img.height() as usize);

    if img.color().has_color() {
        let rgb = img.to_rgb32f();
        drop(img);
        let channel =
            |c: usize| Plane::from_vec(w, h, rgb.pixels().map(|p| decode(p.0[c])).collect());
        Ok(SceneImage::Rgb {
            r: channel(0)?,
            g: channel(1)?,
            b: channel(2)?,
        })
    } else {
        let luma = img.to_luma32f();
        Ok(SceneImage::Mono {
            y: Plane::from_vec(w, h, luma.pixels().map(|p| decode(p.0[0])).collect())?,
        })
    }
}

/// Small sRGB RGBA thumbnail for browsing, `(width, height, pixels)`.
pub fn load_thumbnail(path: &Path, max_edge: u32) -> image::ImageResult<(usize, usize, Vec<u8>)> {
    let thumb = image::open(path)?.thumbnail(max_edge, max_edge).to_rgba8();
    Ok((
        thumb.width() as usize,
        thumb.height() as usize,
        thumb.into_raw(),
    ))
}

fn map_planes(scene: &SceneImage, f: impl Fn(&Plane) -> Plane) -> SceneImage {
    match scene {
        SceneImage::Rgb { r, g, b } => SceneImage::Rgb {
            r: f(r),
            g: f(g),
            b: f(b),
        },
        SceneImage::Mono { y } => SceneImage::Mono { y: f(y) },
    }
}

/// Box-filter downscale (in linear light) so the long edge is at most `max_edge`.
/// Returns a copy when the image is already small enough.
pub fn downscale(scene: &SceneImage, max_edge: usize) -> SceneImage {
    let (w, h) = scene.dimensions();
    let factor = w.max(h).div_ceil(max_edge.max(1));
    if factor <= 1 {
        return scene.clone();
    }
    let (nw, nh) = ((w / factor).max(1), (h / factor).max(1));
    map_planes(scene, |p| {
        let mut out = Plane::filled(nw, nh, 0.0).expect("non-zero size");
        let norm = 1.0 / (factor * factor) as f32;
        out.data_mut()
            .par_chunks_mut(nw)
            .enumerate()
            .for_each(|(oy, row)| {
                for (ox, o) in row.iter_mut().enumerate() {
                    let mut s = 0.0f32;
                    for y in oy * factor..(oy + 1) * factor {
                        s += p.row(y)[ox * factor..(ox + 1) * factor].iter().sum::<f32>();
                    }
                    *o = s * norm;
                }
            });
        out
    })
}

/// Rectangular crop, clamped to the image.
pub fn crop(scene: &SceneImage, x: usize, y: usize, width: usize, height: usize) -> SceneImage {
    let (w, h) = scene.dimensions();
    let x = x.min(w - 1);
    let y = y.min(h - 1);
    let cw = width.min(w - x).max(1);
    let ch = height.min(h - y).max(1);
    map_planes(scene, |p| {
        Plane::from_fn(cw, ch, |cx, cy| p.get(x + cx, y + cy)).expect("non-zero size")
    })
}

/// Mirror left–right, then rotate clockwise by `quarter_turns` × 90°.
///
/// Every EXIF orientation is one of these eight combinations; see
/// `bw_raw::exif_transform` for the mapping.
pub fn transform(scene: &SceneImage, flip_h: bool, quarter_turns: u8) -> SceneImage {
    let turns = quarter_turns % 4;
    if !flip_h && turns == 0 {
        return scene.clone();
    }
    map_planes(scene, |p| {
        let (w, h) = (p.width(), p.height());
        let (ow, oh) = if turns % 2 == 1 { (h, w) } else { (w, h) };
        Plane::from_fn(ow, oh, |ox, oy| {
            // Output pixel → source pixel of the flipped image, undoing the rotation.
            let (fx, fy) = match turns {
                0 => (ox, oy),
                1 => (oy, h - 1 - ox),
                2 => (w - 1 - ox, h - 1 - oy),
                _ => (w - 1 - oy, ox),
            };
            let sx = if flip_h { w - 1 - fx } else { fx };
            p.get(sx, fy)
        })
        .expect("non-zero size")
    })
}

/// Multiply every channel by `factor` (exposure in linear light).
pub fn scale(scene: &SceneImage, factor: f32) -> SceneImage {
    map_planes(scene, |p| {
        let mut out = p.clone();
        out.data_mut().par_iter_mut().for_each(|v| *v *= factor);
        out
    })
}

/// Scene-linear image as 8-bit sRGB RGBA for display.
pub fn scene_to_srgb_rgba(scene: &SceneImage) -> (usize, usize, Vec<u8>) {
    let (w, h) = scene.dimensions();
    let px: Vec<u8> = match scene {
        SceneImage::Rgb { r, g, b } => (0..w * h)
            .into_par_iter()
            .flat_map_iter(|i| {
                [
                    to_u8(linear_to_srgb(r.data()[i])),
                    to_u8(linear_to_srgb(g.data()[i])),
                    to_u8(linear_to_srgb(b.data()[i])),
                    255,
                ]
            })
            .collect(),
        SceneImage::Mono { y } => y
            .data()
            .par_iter()
            .flat_map_iter(|&v| {
                let c = to_u8(linear_to_srgb(v));
                [c, c, c, 255]
            })
            .collect(),
    };
    (w, h, px)
}

/// Linear reflectance as 8-bit sRGB grey.
pub fn plane_to_srgb_gray8(plane: &Plane) -> Vec<u8> {
    plane
        .data()
        .par_iter()
        .map(|&v| to_u8(linear_to_srgb(v)))
        .collect()
}

/// Reflectance (relative to paper white) as an 8-bit sRGB PNG.
pub fn save_srgb_png(plane: &Plane, path: &Path) -> image::ImageResult<()> {
    let buf: ImageBuffer<Luma<u8>, Vec<u8>> = ImageBuffer::from_raw(
        plane.width() as u32,
        plane.height() as u32,
        plane_to_srgb_gray8(plane),
    )
    .expect("buffer matches dimensions");
    buf.save(path)
}

/// Reflectance as a 16-bit greyscale TIFF with the sRGB transfer curve, suitable for
/// viewing and editing applications. No ICC profile is embedded.
pub fn save_print_tiff16(plane: &Plane, path: &Path) -> image::ImageResult<()> {
    let data: Vec<u16> = plane
        .data()
        .par_iter()
        .map(|&v| (linear_to_srgb(v) * 65535.0 + 0.5) as u16)
        .collect();
    let buf: ImageBuffer<Luma<u16>, Vec<u16>> =
        ImageBuffer::from_raw(plane.width() as u32, plane.height() as u32, data)
            .expect("buffer matches dimensions");
    buf.save(path)
}

/// Reflectance as an 8-bit greyscale JPEG (sRGB transfer curve) at `quality` 1–100.
pub fn save_print_jpeg(plane: &Plane, path: &Path, quality: u8) -> image::ImageResult<()> {
    let data = plane_to_srgb_gray8(plane);
    let file = std::io::BufWriter::new(std::fs::File::create(path)?);
    let mut encoder =
        image::codecs::jpeg::JpegEncoder::new_with_quality(file, quality.clamp(1, 100));
    encoder.encode(
        &data,
        plane.width() as u32,
        plane.height() as u32,
        image::ExtendedColorType::L8,
    )
}

/// Reflectance as a 16-bit greyscale PNG (sRGB transfer curve).
pub fn save_print_png16(plane: &Plane, path: &Path) -> image::ImageResult<()> {
    let data: Vec<u16> = plane
        .data()
        .par_iter()
        .map(|&v| (linear_to_srgb(v) * 65535.0 + 0.5) as u16)
        .collect();
    let buf: ImageBuffer<Luma<u16>, Vec<u16>> =
        ImageBuffer::from_raw(plane.width() as u32, plane.height() as u32, data)
            .expect("buffer matches dimensions");
    buf.save_with_format(path, image::ImageFormat::Png)
}

/// Resample a plane (linear light) so its long edge is `long_edge` pixels, with a
/// Lanczos filter. Planes already that size or smaller are returned unchanged.
pub fn resize_long_edge(plane: &Plane, long_edge: usize) -> Plane {
    let (w, h) = (plane.width(), plane.height());
    if w.max(h) <= long_edge || long_edge == 0 {
        return plane.clone();
    }
    let scale = long_edge as f64 / w.max(h) as f64;
    let (nw, nh) = (
        ((w as f64 * scale).round() as u32).max(1),
        ((h as f64 * scale).round() as u32).max(1),
    );
    let buf: ImageBuffer<Luma<f32>, Vec<f32>> =
        ImageBuffer::from_raw(w as u32, h as u32, plane.data().to_vec()).expect("buffer matches");
    let out = image::imageops::resize(&buf, nw, nh, image::imageops::FilterType::Lanczos3);
    let data = out.into_raw().into_iter().map(|v| v.max(0.0)).collect();
    Plane::from_vec(nw as usize, nh as usize, data).expect("sizes match")
}

/// Linear reflectance as a 16-bit TIFF.
pub fn save_linear_tiff16(plane: &Plane, path: &Path) -> image::ImageResult<()> {
    let data: Vec<u16> = plane
        .data()
        .par_iter()
        .map(|&v| (v.clamp(0.0, 1.0) * 65535.0 + 0.5) as u16)
        .collect();
    let buf: ImageBuffer<Luma<u16>, Vec<u16>> =
        ImageBuffer::from_raw(plane.width() as u32, plane.height() as u32, data)
            .expect("buffer matches dimensions");
    buf.save(path)
}

/// The negative as seen on a light box: transmittance relative to clear base.
pub fn save_negative_png(negative: &Negative, path: &Path) -> image::ImageResult<()> {
    let base = negative.base_plus_fog as f32;
    let d = &negative.density;
    let data: Vec<u8> = d
        .data()
        .par_iter()
        .map(|&v| to_u8(linear_to_srgb(10f32.powf(-(v - base).max(0.0)))))
        .collect();
    let buf: ImageBuffer<Luma<u8>, Vec<u8>> =
        ImageBuffer::from_raw(d.width() as u32, d.height() as u32, data)
            .expect("buffer matches dimensions");
    buf.save(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn srgb_round_trip() {
        for i in 0..=100 {
            let v = i as f32 / 100.0;
            assert!((linear_to_srgb(srgb_to_linear(v)) - v).abs() < 1e-5);
        }
    }

    #[test]
    fn downscale_preserves_mean_light() {
        let y = Plane::from_fn(100, 60, |x, y| ((x * 7 + y * 3) % 11) as f32 / 10.0).unwrap();
        let mean = y.data().iter().sum::<f32>() / y.data().len() as f32;
        let small = downscale(&SceneImage::Mono { y }, 25);
        let SceneImage::Mono { y: s } = small else {
            unreachable!()
        };
        assert_eq!((s.width(), s.height()), (25, 15));
        let smean = s.data().iter().sum::<f32>() / s.data().len() as f32;
        assert!((smean - mean).abs() < 1e-3);
    }

    #[test]
    fn transform_rotates_and_flips() {
        // 3×2 image with values 10·y + x.
        let y = Plane::from_fn(3, 2, |x, y| (10 * y + x) as f32).unwrap();
        let scene = SceneImage::Mono { y };
        let get = |s: &SceneImage| match s {
            SceneImage::Mono { y } => (y.width(), y.height(), y.data().to_vec()),
            _ => unreachable!(),
        };
        // 90° clockwise: the left column (0, 10) becomes the top row reversed (10, 0).
        assert_eq!(
            get(&transform(&scene, false, 1)),
            (2, 3, vec![10.0, 0.0, 11.0, 1.0, 12.0, 2.0])
        );
        assert_eq!(
            get(&transform(&scene, false, 2)),
            (3, 2, vec![12.0, 11.0, 10.0, 2.0, 1.0, 0.0])
        );
        assert_eq!(
            get(&transform(&scene, true, 0)),
            (3, 2, vec![2.0, 1.0, 0.0, 12.0, 11.0, 10.0])
        );
        // Four quarter turns are the identity.
        let four = transform(&transform(&scene, false, 3), false, 1);
        assert_eq!(get(&four), get(&scene));
    }

    #[test]
    fn writes_jpeg_and_png_and_resizes() {
        let dir = std::env::temp_dir().join(format!("bw-io-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let plane = Plane::from_fn(300, 200, |x, _| x as f32 / 300.0).unwrap();
        let small = resize_long_edge(&plane, 150);
        assert_eq!((small.width(), small.height()), (150, 100));
        assert_eq!(resize_long_edge(&plane, 1000).width(), 300);

        let (hi, lo) = (dir.join("q95.jpg"), dir.join("q20.jpg"));
        save_print_jpeg(&plane, &hi, 95).unwrap();
        save_print_jpeg(&plane, &lo, 20).unwrap();
        assert!(std::fs::metadata(&hi).unwrap().len() > std::fs::metadata(&lo).unwrap().len());
        assert!(matches!(
            image::open(&hi).unwrap(),
            DynamicImage::ImageLuma8(_)
        ));

        let png = dir.join("p.png");
        save_print_png16(&plane, &png).unwrap();
        assert!(matches!(
            image::open(&png).unwrap(),
            DynamicImage::ImageLuma16(_)
        ));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn crop_is_clamped() {
        let y = Plane::from_fn(10, 10, |x, y| (y * 10 + x) as f32).unwrap();
        let c = crop(&SceneImage::Mono { y }, 8, 8, 5, 5);
        let SceneImage::Mono { y: c } = c else {
            unreachable!()
        };
        assert_eq!((c.width(), c.height()), (2, 2));
        assert_eq!(c.get(0, 0), 88.0);
    }
}
