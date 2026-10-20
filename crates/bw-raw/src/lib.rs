//! Camera raw input (design 0001 §6.1, `bw-raw`).
//!
//! Decoding, demosaicing, the DNG colour model and highlight reconstruction come from
//! LightCraft's `lightcraft-raw` (MIT OR Apache-2.0, pure Rust), pinned in the
//! workspace manifest. This crate adds the scene-referred adjustments made before the
//! film sees the image, and converts the result to the engine's working space
//! (scene-linear Rec.709 primaries, D65).
//!
//! Only scene-referred edits belong here: exposure, white balance, highlight handling
//! and orientation. Tone (contrast, highlights/shadows) is the job of the film,
//! development and paper, so it is deliberately not offered.
//!
//! Known gap: Nikon High Efficiency NEFs (HE / HE★, JPEG XS) cannot be decoded and are
//! reported as unsupported; shoot lossless compressed NEF.

use std::path::Path;

use bw_core::Plane;
use bw_negative::SceneImage;
use lightcraft_color::{Mat3, REC2020, SRGB, Xy, cct};
use lightcraft_geom::Orientation;
use lightcraft_raster::Rgb32f;
use lightcraft_raw::{ColorData, Method, color, highlight};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Raw file extensions the app offers (the decoder reads more; these are the
/// formats of the cameras this project targets: Nikon, Leica DNG, Sony).
pub const RAW_EXTENSIONS: [&str; 3] = ["nef", "dng", "arw"];

pub fn is_raw(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| RAW_EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()))
}

#[derive(Debug, Error)]
pub enum RawFileError {
    #[error("cannot read {0}: {1}")]
    Io(String, std::io::Error),
    #[error(
        "this raw variant isn't supported ({0}). Nikon High Efficiency (HE/HE★) NEFs can't be decoded yet: set the camera to lossless compressed NEF"
    )]
    Unsupported(String),
    #[error("cannot decode raw file: {0}")]
    Decode(String),
}

/// White balance: which light renders as neutral.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "kebab-case")]
pub enum WhiteBalance {
    AsShot,
    /// Grey-world estimate from the image itself.
    Auto,
    Daylight,
    Cloudy,
    Shade,
    Tungsten,
    Fluorescent,
    Flash,
    /// Correlated colour temperature (K) and green–magenta tint.
    Custom {
        temp: f64,
        tint: f64,
    },
}

impl WhiteBalance {
    pub const PRESETS: [WhiteBalance; 8] = [
        WhiteBalance::AsShot,
        WhiteBalance::Auto,
        WhiteBalance::Daylight,
        WhiteBalance::Cloudy,
        WhiteBalance::Shade,
        WhiteBalance::Tungsten,
        WhiteBalance::Fluorescent,
        WhiteBalance::Flash,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            Self::AsShot => "As shot",
            Self::Auto => "Auto",
            Self::Daylight => "Daylight",
            Self::Cloudy => "Cloudy",
            Self::Shade => "Shade",
            Self::Tungsten => "Tungsten",
            Self::Fluorescent => "Fluorescent",
            Self::Flash => "Flash",
            Self::Custom { .. } => "Custom",
        }
    }

    /// Preset temperature and tint, using the conventional values photo editors use.
    fn preset(&self) -> Option<(f64, f64)> {
        Some(match self {
            Self::Daylight => (5500.0, 10.0),
            Self::Cloudy => (6500.0, 10.0),
            Self::Shade => (7500.0, 10.0),
            Self::Tungsten => (2850.0, 0.0),
            Self::Fluorescent => (3800.0, 21.0),
            Self::Flash => (5500.0, 0.0),
            Self::Custom { temp, tint } => (*temp, *tint),
            Self::AsShot | Self::Auto => return None,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Highlights {
    /// Clip every channel at the lowest white-balanced clip level, so clipped areas are neutral.
    Clip,
    /// Rebuild partly clipped channels from the unclipped ones (LightCraft's reconstruction).
    Reconstruct,
}

/// Per-image adjustments made before black-and-white development.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct RawAdjust {
    /// Exposure correction in stops, applied in linear light.
    pub exposure_ev: f64,
    pub white_balance: WhiteBalance,
    pub highlights: Highlights,
    /// Extra clockwise quarter turns on top of the camera's orientation.
    pub rotation: u8,
}

impl Default for RawAdjust {
    fn default() -> Self {
        Self {
            exposure_ev: 0.0,
            white_balance: WhiteBalance::AsShot,
            highlights: Highlights::Reconstruct,
            rotation: 0,
        }
    }
}

/// Camera and capture details shown in the UI.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CaptureInfo {
    pub camera: String,
    pub iso: Option<u32>,
    pub exposure_time: Option<f64>,
    pub f_number: Option<f64>,
    pub focal_length: Option<f64>,
    pub width: usize,
    pub height: usize,
    pub monochrome: bool,
    /// No colour matrix was available for this camera; colour is approximate.
    pub matrix_is_fallback: bool,
    /// Colour comes from matrices fitted to the camera's measured spectral sensitivities.
    pub spectral_matrix: bool,
}

/// "NIKON CORPORATION" + "NIKON Z f" → "Nikon Z f".
fn camera_name(make: Option<&str>, model: Option<&str>) -> String {
    let make = make.map(str::trim).unwrap_or("");
    let model = model.map(str::trim).unwrap_or("");
    let brand = make.split_whitespace().next().unwrap_or("");
    let title = |s: &str| {
        let lower = s.to_lowercase();
        let mut c = lower.chars();
        c.next()
            .map_or(String::new(), |f| f.to_uppercase().chain(c).collect())
    };
    let model_rest = model
        .strip_prefix(brand)
        .map(str::trim)
        .filter(|_| !brand.is_empty())
        .unwrap_or(model);
    match (brand.is_empty(), model_rest.is_empty()) {
        (true, true) => "Unknown camera".into(),
        (true, false) => model_rest.into(),
        (false, true) => title(brand),
        (false, false) => format!("{} {model_rest}", title(brand)),
    }
}

impl CaptureInfo {
    pub fn summary(&self) -> String {
        let mut parts = vec![self.camera.clone()];
        if let Some(t) = self.exposure_time {
            parts.push(if t < 1.0 {
                format!("1/{:.0} s", 1.0 / t)
            } else {
                format!("{t:.1} s")
            });
        }
        if let Some(f) = self.f_number {
            parts.push(format!("f/{f:.1}"));
        }
        if let Some(iso) = self.iso {
            parts.push(format!("ISO {iso}"));
        }
        if let Some(fl) = self.focal_length {
            parts.push(format!("{fl:.0} mm"));
        }
        parts.join(" · ")
    }
}

#[derive(Clone)]
enum Pixels {
    /// Demosaiced camera RGB, not white balanced, white level 1.0, not oriented.
    Rgb(Rgb32f),
    /// Monochrome sensor data (e.g. Leica Monochrom), white level 1.0, not oriented.
    Mono(Plane),
}

/// A decoded raw photo, ready to be developed with [`RawAdjust`] settings.
#[derive(Clone)]
pub struct RawPhoto {
    pixels: Pixels,
    color: ColorData,
    wb_multipliers: Option<[f32; 3]>,
    orientation: Orientation,
    pub info: CaptureInfo,
}

/// Map an EXIF orientation to `(mirror, clockwise quarter turns)` for [`bw_io::transform`].
pub fn exif_transform(o: Orientation) -> (bool, u8) {
    match o {
        Orientation::Normal => (false, 0),
        Orientation::FlipH => (true, 0),
        Orientation::Rotate180 => (false, 2),
        Orientation::FlipV => (true, 2),
        Orientation::Transpose => (true, 3),
        Orientation::Rotate90 => (false, 1),
        Orientation::Transverse => (true, 1),
        Orientation::Rotate270 => (false, 3),
    }
}

/// Decode and demosaic a raw file (Adaptive Homogeneity-Directed demosaic).
pub fn open(path: &Path) -> Result<RawPhoto, RawFileError> {
    let bytes = std::fs::read(path).map_err(|e| RawFileError::Io(path.display().to_string(), e))?;
    let mut raw = lightcraft_raw::decode(&bytes).map_err(|e| match e {
        lightcraft_raw::RawError::Unsupported(s) => RawFileError::Unsupported(s),
        other => RawFileError::Decode(other.to_string()),
    })?;
    drop(bytes);
    // Non-DNG raws (NEF, ARW) carry no colour matrix: use LightCraft's matrices fitted
    // to the camera's measured spectral sensitivities (rawtoaces-data), when listed.
    let spectral_matrix = raw
        .metadata
        .model
        .as_deref()
        .and_then(|model| lightcraft_raw::spectral::find(raw.metadata.make.as_deref(), model))
        .is_some_and(|camera| camera.fill(&mut raw.color));

    let monochrome = raw.cfa.is_none() && raw.cpp == 1;
    let pixels = if monochrome {
        let n = raw
            .normalized()
            .map_err(|e| RawFileError::Decode(e.to_string()))?;
        let c = raw.develop_crop(n.width, n.height);
        let plane = Plane::from_fn(c.width, c.height, |x, y| {
            n.data[(c.y + y) * n.width + c.x + x]
        })
        .map_err(|e| RawFileError::Decode(e.to_string()))?;
        Pixels::Mono(plane)
    } else {
        Pixels::Rgb(
            raw.develop(Method::Ahd)
                .map_err(|e| RawFileError::Decode(e.to_string()))?,
        )
    };
    let (width, height) = match &pixels {
        Pixels::Rgb(img) => (img.width, img.height),
        Pixels::Mono(p) => (p.width(), p.height()),
    };
    let m = &raw.metadata;
    let camera = camera_name(m.make.as_deref(), m.model.as_deref());
    let info = CaptureInfo {
        camera: if camera.is_empty() {
            "Unknown camera".into()
        } else {
            camera
        },
        iso: m.iso,
        exposure_time: m.exposure_time,
        f_number: m.f_number,
        focal_length: m.focal_length,
        width,
        height,
        monochrome,
        matrix_is_fallback: !color::has_matrix(&raw.color),
        spectral_matrix,
    };
    Ok(RawPhoto {
        pixels,
        color: raw.color,
        wb_multipliers: raw.wb_multipliers,
        orientation: raw.orientation,
        info,
    })
}

/// The camera's embedded JPEG preview as RGBA, reduced to fit `max_edge` (for thumbnails).
pub fn thumbnail(path: &Path, max_edge: u32) -> Option<(usize, usize, Vec<u8>)> {
    let bytes = std::fs::read(path).ok()?;
    let jpeg = lightcraft_raw::embedded_preview(&bytes)?;
    let img = image::load_from_memory(&jpeg)
        .ok()?
        .thumbnail(max_edge, max_edge)
        .to_rgba8();
    Some((img.width() as usize, img.height() as usize, img.into_raw()))
}

impl RawPhoto {
    /// A copy reduced so its long edge is at most `max_edge` (box filter, before any
    /// colour processing, which is linear so the result matches developing in full).
    pub fn downscaled(&self, max_edge: usize) -> RawPhoto {
        let mut out = self.clone();
        out.pixels = match &self.pixels {
            Pixels::Mono(p) => {
                let scene = bw_io::downscale(&SceneImage::Mono { y: p.clone() }, max_edge);
                let SceneImage::Mono { y } = scene else {
                    unreachable!("mono stays mono")
                };
                Pixels::Mono(y)
            }
            Pixels::Rgb(img) => {
                let factor = img.width.max(img.height).div_ceil(max_edge.max(1));
                if factor <= 1 {
                    Pixels::Rgb(img.clone())
                } else {
                    Pixels::Rgb(lightcraft_raster::resample::resize(
                        img,
                        (img.width / factor).max(1),
                        (img.height / factor).max(1),
                        lightcraft_raster::resample::Filter::Box,
                    ))
                }
            }
        };
        out
    }

    /// The camera's recorded white (as shot).
    pub fn as_shot_white(&self) -> Xy {
        color::as_shot_white_of(&self.color, self.wb_multipliers)
    }

    /// As-shot white as (temperature K, tint), for showing on the sliders.
    pub fn as_shot_temp_tint(&self) -> (f64, f64) {
        cct::xy_to_temp_tint(self.as_shot_white())
    }

    /// The white point a white-balance setting resolves to.
    pub fn white_for(&self, wb: WhiteBalance) -> Xy {
        match wb {
            WhiteBalance::AsShot => self.as_shot_white(),
            WhiteBalance::Auto => match &self.pixels {
                Pixels::Rgb(img) => {
                    let mult = color::grey_world(img);
                    let neutral = [
                        1.0 / f64::from(mult[0]),
                        1.0 / f64::from(mult[1]),
                        1.0 / f64::from(mult[2]),
                    ];
                    color::neutral_to_xy(&self.color, neutral)
                }
                Pixels::Mono(_) => self.as_shot_white(),
            },
            other => {
                let (t, tint) = other
                    .preset()
                    .expect("presets and custom have a temperature");
                cct::temp_tint_to_xy(t, tint)
            }
        }
    }

    /// (temperature, tint) a white-balance setting resolves to, for the sliders.
    pub fn temp_tint_for(&self, wb: WhiteBalance) -> (f64, f64) {
        cct::xy_to_temp_tint(self.white_for(wb))
    }

    /// Exposure (stops) that would put the log-average scene luminance at 18 %:
    /// what an averaging meter would choose.
    pub fn auto_exposure_ev(&self, adjust: &RawAdjust) -> f64 {
        let probe = RawAdjust {
            exposure_ev: 0.0,
            ..*adjust
        };
        let small = self.downscaled(512).develop(&probe);
        let y = match &small {
            SceneImage::Mono { y } => y.data().to_vec(),
            SceneImage::Rgb { r, g, b } => r
                .data()
                .iter()
                .zip(g.data())
                .zip(b.data())
                .map(|((r, g), b)| 0.2126 * r + 0.7152 * g + 0.0722 * b)
                .collect(),
        };
        let (sum, n) = y
            .iter()
            .filter(|v| **v > 1e-5)
            .fold((0.0f64, 0usize), |(s, n), v| {
                (s + f64::from(*v).ln(), n + 1)
            });
        if n == 0 {
            return 0.0;
        }
        let mean = (sum / n as f64).exp();
        ((0.18 / mean).log2() * 3.0).round() / 3.0
    }

    /// Develop into the engine's scene-linear Rec.709 working space.
    pub fn develop(&self, adjust: &RawAdjust) -> SceneImage {
        let gain = 2f32.powf(adjust.exposure_ev as f32);
        let scene = match &self.pixels {
            Pixels::Mono(p) => {
                let mut y = p.clone();
                y.data_mut()
                    .par_iter_mut()
                    .for_each(|v| *v = (*v * gain).max(0.0));
                SceneImage::Mono { y }
            }
            Pixels::Rgb(img) => {
                let transform =
                    color::camera_transform_of(&self.color, self.white_for(adjust.white_balance));
                let mut cam = img.clone();
                match adjust.highlights {
                    Highlights::Clip => highlight::clip_neutral(&mut cam, transform.wb, 1.0),
                    Highlights::Reconstruct => {
                        highlight::reconstruct(&mut cam, transform.wb, 1.0);
                    }
                }
                // camera → (white balance) → Rec.2020 → Rec.709, then exposure, as one matrix.
                // Values outside Rec.709 stay negative here; the film's spectral projection
                // is linear, so no information is clipped before it.
                let to_709: Mat3 = REC2020.to_space(&SRGB).mul(&transform.matrix);
                let wb = transform.wb;
                let m: [[f32; 3]; 3] = to_709.to_f32();
                let n = cam.width * cam.height;
                let (mut r, mut g, mut b) = (vec![0f32; n], vec![0f32; n], vec![0f32; n]);
                r.par_iter_mut()
                    .zip(g.par_iter_mut())
                    .zip(b.par_iter_mut())
                    .zip(cam.data.par_iter())
                    .for_each(|(((r, g), b), px)| {
                        let c = [px[0] * wb[0], px[1] * wb[1], px[2] * wb[2]];
                        *r = gain * (m[0][0] * c[0] + m[0][1] * c[1] + m[0][2] * c[2]);
                        *g = gain * (m[1][0] * c[0] + m[1][1] * c[1] + m[1][2] * c[2]);
                        *b = gain * (m[2][0] * c[0] + m[2][1] * c[1] + m[2][2] * c[2]);
                    });
                let plane = |v| Plane::from_vec(cam.width, cam.height, v).expect("sizes match");
                SceneImage::Rgb {
                    r: plane(r),
                    g: plane(g),
                    b: plane(b),
                }
            }
        };
        let (flip, turns) = exif_transform(self.orientation);
        bw_io::transform(&scene, flip, (turns + adjust.rotation) % 4)
    }
}

/// Exposure and rotation for non-raw (already rendered) images. White balance and
/// highlight reconstruction need raw data and are not applied.
pub fn adjust_rendered(scene: &SceneImage, adjust: &RawAdjust) -> SceneImage {
    let scaled = if adjust.exposure_ev == 0.0 {
        scene.clone()
    } else {
        bw_io::scale(scene, 2f32.powf(adjust.exposure_ev as f32))
    };
    bw_io::transform(&scaled, false, adjust.rotation % 4)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exif_transforms_compose_to_identity_with_their_inverse_turns() {
        for (o, expected) in [
            (Orientation::Normal, (false, 0)),
            (Orientation::Rotate90, (false, 1)),
            (Orientation::Rotate180, (false, 2)),
            (Orientation::Rotate270, (false, 3)),
        ] {
            assert_eq!(exif_transform(o), expected);
        }
    }

    #[test]
    fn camera_names_drop_repeated_make() {
        assert_eq!(
            camera_name(Some("NIKON CORPORATION"), Some("NIKON Z f")),
            "Nikon Z f"
        );
        assert_eq!(camera_name(Some("SONY"), Some("ILCE-7M4")), "Sony ILCE-7M4");
        assert_eq!(
            camera_name(Some("LEICA"), Some("M MONOCHROM (Typ 246)")),
            "Leica M MONOCHROM (Typ 246)"
        );
        assert_eq!(camera_name(None, None), "Unknown camera");
    }

    #[test]
    fn white_balance_presets_resolve_to_plausible_temperatures() {
        for wb in [
            WhiteBalance::Daylight,
            WhiteBalance::Tungsten,
            WhiteBalance::Shade,
        ] {
            let (t, _) = wb.preset().unwrap();
            let (t2, _) = cct::xy_to_temp_tint(cct::temp_tint_to_xy(t, 0.0));
            assert!((t2 - t).abs() / t < 0.01, "{wb:?}: {t} vs {t2}");
        }
    }

    #[test]
    fn adjustments_round_trip_through_json() {
        let a = RawAdjust {
            exposure_ev: 0.67,
            white_balance: WhiteBalance::Custom {
                temp: 4300.0,
                tint: -5.0,
            },
            highlights: Highlights::Clip,
            rotation: 3,
        };
        let json = serde_json::to_string(&a).unwrap();
        assert!(json.contains("\"mode\":\"custom\""), "{json}");
        let back: RawAdjust = serde_json::from_str(&json).unwrap();
        assert_eq!(back, a);
    }

    #[test]
    fn rendered_images_get_exposure_and_rotation() {
        let y = Plane::from_fn(4, 2, |x, _| x as f32 / 4.0).unwrap();
        let scene = SceneImage::Mono { y };
        let adj = RawAdjust {
            exposure_ev: 1.0,
            rotation: 1,
            ..Default::default()
        };
        let SceneImage::Mono { y } = adjust_rendered(&scene, &adj) else {
            unreachable!()
        };
        assert_eq!((y.width(), y.height()), (2, 4));
        assert!((y.data().iter().copied().fold(0.0f32, f32::max) - 1.5).abs() < 1e-6);
    }

    fn exif_tag_make() -> bw_io::metadata::ExifTag {
        bw_io::metadata::ExifTag::Make
    }

    /// Decodes a real Nikon Z f NEF when `BW_RAW_SAMPLE` points at one (CC0 samples:
    /// raw.pixls.us/data/Nikon/Z f/).
    #[test]
    fn decodes_sample_nef_when_available() {
        let Some(path) = std::env::var_os("BW_RAW_SAMPLE") else {
            eprintln!("skipped: set BW_RAW_SAMPLE to a raw file");
            return;
        };
        let photo = match open(Path::new(&path)) {
            Ok(p) => p,
            Err(RawFileError::Unsupported(s)) => {
                eprintln!("unsupported variant reported cleanly: {s}");
                return;
            }
            Err(e) => panic!("{e}"),
        };
        assert!(photo.info.width > 1000 && photo.info.height > 1000);
        if photo.info.camera.contains("Z f") {
            assert!(
                photo.info.spectral_matrix && !photo.info.matrix_is_fallback,
                "{:?}",
                photo.info
            );
        }
        let (t, _) = photo.as_shot_temp_tint();
        assert!((2000.0..12000.0).contains(&t), "as-shot temperature {t}");
        // The raw file's EXIF (camera, exposure, date) is readable for exports.
        let exif = bw_io::read_exif(Path::new(&path)).expect("raw EXIF");
        assert!(exif.get(exif_tag_make()).is_some());
        let scene = photo.downscaled(800).develop(&RawAdjust::default());
        let (w, h) = scene.dimensions();
        assert!(w.max(h) <= 800);
        if let SceneImage::Rgb { g, .. } = &scene {
            assert!(g.data().iter().all(|v| v.is_finite()));
        }
    }
}
