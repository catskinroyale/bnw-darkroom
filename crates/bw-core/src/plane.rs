//! Planar single-channel `f32` image storage.

use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum PlaneError {
    #[error("plane dimensions must be non-zero (got {width}x{height})")]
    ZeroSize { width: usize, height: usize },
    #[error("data length {len} does not match {width}x{height}")]
    LengthMismatch {
        width: usize,
        height: usize,
        len: usize,
    },
}

/// One channel of image data in row-major order.
///
/// All engine stages operate on planes rather than interleaved pixels: separable
/// filters, per-channel stages and SIMD-friendly loops all benefit from the layout.
#[derive(Clone, Debug, PartialEq)]
pub struct Plane {
    width: usize,
    height: usize,
    data: Vec<f32>,
}

impl Plane {
    /// A plane filled with `value`.
    pub fn filled(width: usize, height: usize, value: f32) -> Result<Self, PlaneError> {
        if width == 0 || height == 0 {
            return Err(PlaneError::ZeroSize { width, height });
        }
        Ok(Self {
            width,
            height,
            data: vec![value; width * height],
        })
    }

    pub fn from_vec(width: usize, height: usize, data: Vec<f32>) -> Result<Self, PlaneError> {
        if width == 0 || height == 0 {
            return Err(PlaneError::ZeroSize { width, height });
        }
        if data.len() != width * height {
            return Err(PlaneError::LengthMismatch {
                width,
                height,
                len: data.len(),
            });
        }
        Ok(Self {
            width,
            height,
            data,
        })
    }

    /// A plane where each pixel is `f(x, y)`.
    pub fn from_fn(
        width: usize,
        height: usize,
        mut f: impl FnMut(usize, usize) -> f32,
    ) -> Result<Self, PlaneError> {
        let mut plane = Self::filled(width, height, 0.0)?;
        for y in 0..height {
            for x in 0..width {
                plane.data[y * width + x] = f(x, y);
            }
        }
        Ok(plane)
    }

    #[inline]
    pub fn width(&self) -> usize {
        self.width
    }

    #[inline]
    pub fn height(&self) -> usize {
        self.height
    }

    #[inline]
    pub fn get(&self, x: usize, y: usize) -> f32 {
        self.data[y * self.width + x]
    }

    #[inline]
    pub fn set(&mut self, x: usize, y: usize, value: f32) {
        self.data[y * self.width + x] = value;
    }

    #[inline]
    pub fn row(&self, y: usize) -> &[f32] {
        &self.data[y * self.width..(y + 1) * self.width]
    }

    pub fn data(&self) -> &[f32] {
        &self.data
    }

    pub fn data_mut(&mut self) -> &mut [f32] {
        &mut self.data
    }

    pub fn into_vec(self) -> Vec<f32> {
        self.data
    }

    /// A zeroed plane with the same dimensions.
    pub fn zeros_like(&self) -> Self {
        Self {
            width: self.width,
            height: self.height,
            data: vec![0.0; self.data.len()],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_bad_dimensions() {
        assert_eq!(
            Plane::filled(0, 4, 0.0),
            Err(PlaneError::ZeroSize {
                width: 0,
                height: 4
            })
        );
        assert_eq!(
            Plane::from_vec(2, 2, vec![0.0; 3]),
            Err(PlaneError::LengthMismatch {
                width: 2,
                height: 2,
                len: 3
            })
        );
    }

    #[test]
    fn from_fn_is_row_major() {
        let p = Plane::from_fn(3, 2, |x, y| (y * 10 + x) as f32).unwrap();
        assert_eq!(p.row(1), &[10.0, 11.0, 12.0]);
        assert_eq!(p.get(2, 0), 2.0);
    }
}
