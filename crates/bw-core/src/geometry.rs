//! Mapping between image pixels and the film plane.
//!
//! Grain, emulsion light spread and developer adjacency effects have physical sizes
//! on the film (micrometres). Specifying them in µm and converting through
//! [`FilmGeometry`] keeps their appearance correct at any resolution and film format.

use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum GeometryError {
    #[error("frame width must be positive and finite (got {0} mm)")]
    InvalidFrameWidth(f64),
    #[error("image dimensions must be non-zero (got {0}x{1})")]
    ZeroSize(usize, usize),
}

/// Pixel dimensions and film-plane pixel pitch. Pixels are assumed square.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FilmGeometry {
    width_px: usize,
    height_px: usize,
    pixel_pitch_um: f64,
}

impl FilmGeometry {
    /// Geometry for an image whose width spans `frame_width_mm` of film.
    ///
    /// The frame height follows from the image aspect ratio, so cropped images are
    /// described by the film width they actually cover.
    pub fn from_frame_width(
        frame_width_mm: f64,
        width_px: usize,
        height_px: usize,
    ) -> Result<Self, GeometryError> {
        if !(frame_width_mm.is_finite() && frame_width_mm > 0.0) {
            return Err(GeometryError::InvalidFrameWidth(frame_width_mm));
        }
        if width_px == 0 || height_px == 0 {
            return Err(GeometryError::ZeroSize(width_px, height_px));
        }
        Ok(Self {
            width_px,
            height_px,
            pixel_pitch_um: frame_width_mm * 1000.0 / width_px as f64,
        })
    }

    /// 35 mm full frame (36 mm wide).
    pub fn full_frame_35mm(width_px: usize, height_px: usize) -> Result<Self, GeometryError> {
        Self::from_frame_width(36.0, width_px, height_px)
    }

    pub fn width_px(&self) -> usize {
        self.width_px
    }

    pub fn height_px(&self) -> usize {
        self.height_px
    }

    pub fn pixel_pitch_um(&self) -> f64 {
        self.pixel_pitch_um
    }

    /// Film-plane area of one pixel in µm².
    pub fn pixel_area_um2(&self) -> f64 {
        self.pixel_pitch_um * self.pixel_pitch_um
    }

    pub fn frame_height_mm(&self) -> f64 {
        self.height_px as f64 * self.pixel_pitch_um / 1000.0
    }

    /// Convert a film-plane length in µm to pixels.
    pub fn um_to_px(&self, um: f64) -> f64 {
        um / self.pixel_pitch_um
    }

    /// The same film area rendered at a different pixel width (for previews).
    pub fn rescaled(&self, width_px: usize, height_px: usize) -> Result<Self, GeometryError> {
        Self::from_frame_width(
            self.width_px as f64 * self.pixel_pitch_um / 1000.0,
            width_px,
            height_px,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nikon_zf_full_frame_pitch() {
        // 6048 px across 36 mm ≈ 5.95 µm on film.
        let g = FilmGeometry::full_frame_35mm(6048, 4032).unwrap();
        assert!((g.pixel_pitch_um() - 5.952).abs() < 1e-3);
        assert!((g.frame_height_mm() - 24.0).abs() < 1e-9);
        assert!((g.um_to_px(48.0) - 8.064).abs() < 1e-3);
    }

    #[test]
    fn rescaling_preserves_film_area() {
        let g = FilmGeometry::full_frame_35mm(6000, 4000).unwrap();
        let preview = g.rescaled(1500, 1000).unwrap();
        assert!((preview.pixel_pitch_um() - 4.0 * g.pixel_pitch_um()).abs() < 1e-12);
        assert!((preview.frame_height_mm() - g.frame_height_mm()).abs() < 1e-12);
    }

    #[test]
    fn rejects_invalid_input() {
        assert!(FilmGeometry::from_frame_width(0.0, 10, 10).is_err());
        assert!(FilmGeometry::from_frame_width(f64::NAN, 10, 10).is_err());
        assert!(FilmGeometry::from_frame_width(36.0, 0, 10).is_err());
    }
}
