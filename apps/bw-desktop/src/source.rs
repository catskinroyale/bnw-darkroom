//! An opened image: a rendered raster file or a camera raw, plus its per-image
//! adjustments stored in a sidecar next to it (`<file>.bw.json`), so edits are
//! non-destructive and travel with the photo.

use std::path::{Path, PathBuf};

use bw_negative::SceneImage;
use bw_raw::{CaptureInfo, RawAdjust, RawPhoto};
use serde::{Deserialize, Serialize};

pub enum Source {
    Raster(SceneImage),
    Raw(Box<RawPhoto>),
}

impl Source {
    pub fn open(path: &Path) -> Result<Source, String> {
        if bw_raw::is_raw(path) {
            bw_raw::open(path)
                .map(|p| Source::Raw(Box::new(p)))
                .map_err(|e| e.to_string())
        } else {
            bw_io::load_scene(path, bw_io::Encoding::Auto)
                .map(Source::Raster)
                .map_err(|e| e.to_string())
        }
    }

    /// The scene after per-image adjustments, at full resolution.
    pub fn develop(&self, adjust: &RawAdjust) -> SceneImage {
        match self {
            Source::Raster(scene) => bw_raw::adjust_rendered(scene, adjust),
            Source::Raw(photo) => photo.develop(adjust),
        }
    }

    /// A reduced copy (long edge ≤ `edge`) to develop previews from quickly.
    pub fn downscaled(&self, edge: usize) -> Source {
        match self {
            Source::Raster(scene) => Source::Raster(bw_io::downscale(scene, edge)),
            Source::Raw(photo) => Source::Raw(Box::new(photo.downscaled(edge))),
        }
    }

    pub fn capture(&self) -> Option<&CaptureInfo> {
        match self {
            Source::Raw(photo) => Some(&photo.info),
            Source::Raster(_) => None,
        }
    }

    pub fn raw(&self) -> Option<&RawPhoto> {
        match self {
            Source::Raw(photo) => Some(photo),
            Source::Raster(_) => None,
        }
    }
}

#[derive(Serialize, Deserialize)]
struct Sidecar {
    version: u32,
    adjust: RawAdjust,
}

pub fn sidecar_path(image: &Path) -> PathBuf {
    let mut name = image.file_name().unwrap_or_default().to_os_string();
    name.push(".bw.json");
    image.with_file_name(name)
}

/// The image's saved adjustments, or the defaults.
pub fn load_adjust(image: &Path) -> RawAdjust {
    std::fs::read_to_string(sidecar_path(image))
        .ok()
        .and_then(|s| serde_json::from_str::<Sidecar>(&s).ok())
        .map(|s| s.adjust)
        .unwrap_or_default()
}

/// Save adjustments; default adjustments remove the sidecar instead of writing one.
pub fn save_adjust(image: &Path, adjust: &RawAdjust) -> std::io::Result<()> {
    let path = sidecar_path(image);
    if *adjust == RawAdjust::default() {
        return match std::fs::remove_file(&path) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e),
            _ => Ok(()),
        };
    }
    let json = serde_json::to_string_pretty(&Sidecar {
        version: 1,
        adjust: *adjust,
    })?;
    std::fs::write(path, json)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sidecar_round_trip_and_cleanup() {
        let dir = std::env::temp_dir().join(format!("bw-sidecar-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let image = dir.join("DSC_0001.NEF");
        assert_eq!(load_adjust(&image), RawAdjust::default());
        let adjust = RawAdjust {
            exposure_ev: 0.5,
            rotation: 1,
            ..Default::default()
        };
        save_adjust(&image, &adjust).unwrap();
        assert!(dir.join("DSC_0001.NEF.bw.json").exists());
        assert_eq!(load_adjust(&image), adjust);
        save_adjust(&image, &RawAdjust::default()).unwrap();
        assert!(!dir.join("DSC_0001.NEF.bw.json").exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
