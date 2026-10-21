//! Minimal printing (design 0001 §9.1): enough to view a negative as a print.
//!
//! The paper is a symmetric logistic in log exposure whose log-exposure range is set
//! by a contrast grade. It is a placeholder for the Phase 4 paper models and is not
//! calibrated to any paper.

use bw_core::Plane;
use bw_negative::Negative;
use rayon::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Paper {
    pub d_min: f64,
    pub d_max: f64,
}

impl Default for Paper {
    fn default() -> Self {
        Self {
            d_min: 0.06,
            d_max: 2.05,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PrintExposure {
    /// Expose so the negative's metered mid-grey prints at this reflection density.
    MidGreyAt { print_density: f64 },
    /// Expose so clear film (base + fog) prints at this reflection density: the
    /// printer's black point ("minimum time for maximum black") held from negative to
    /// negative. Development changes then print as they do under a real enlarger:
    /// longer development adds density above the base, so blacks hold while shadows
    /// open slightly and highlights brighten.
    BaseAt { print_density: f64 },
    /// Explicit log exposure offset.
    LogOffset(f64),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PrintSettings {
    pub paper: Paper,
    /// Paper grade 0–5; mapped to a log-exposure range.
    pub grade: f64,
    pub exposure: PrintExposure,
    /// Callier factor (1.0 for a diffusion enlarger).
    pub callier_q: f64,
    /// Additional enlarger exposure in stops (positive = longer exposure, darker print).
    pub exposure_adjust_stops: f64,
}

impl Default for PrintSettings {
    fn default() -> Self {
        Self {
            paper: Paper::default(),
            grade: 2.0,
            exposure: PrintExposure::MidGreyAt {
                print_density: 0.75,
            },
            callier_q: 1.0,
            exposure_adjust_stops: 0.0,
        }
    }
}

impl PrintSettings {
    /// Log-exposure range covering 5–95% of the paper's density range. The grade
    /// mapping approximates the spacing of ISO paper ranges and is a placeholder.
    pub fn log_exposure_range(&self) -> f64 {
        (1.40 - 0.2 * self.grade).max(0.2)
    }

    /// Slope of the paper's logistic in log exposure.
    fn slope(&self) -> f64 {
        2.0 * 19f64.ln() / self.log_exposure_range()
    }

    /// Paper log exposure, relative to the curve's midpoint, that prints at `density`.
    pub fn log_exposure_for(&self, density: f64) -> f64 {
        let Paper { d_min, d_max } = self.paper;
        let frac = ((density - d_min) / (d_max - d_min)).clamp(1e-4, 1.0 - 1e-4);
        (frac / (1.0 - frac)).ln() / self.slope()
    }

    /// Reflection density at paper log exposure `log_h`, relative to the curve's midpoint.
    pub fn density_at(&self, log_h: f64) -> f64 {
        let Paper { d_min, d_max } = self.paper;
        d_min + (d_max - d_min) / (1.0 + (-self.slope() * log_h).exp())
    }
}

/// Print a negative. Returns reflectance relative to paper white (1.0 = paper base).
pub fn print(negative: &Negative, settings: &PrintSettings) -> Plane {
    let d_min = settings.paper.d_min;
    let q = settings.callier_q;
    let offset = std::f64::consts::LOG10_2 * settings.exposure_adjust_stops
        + match settings.exposure {
            PrintExposure::LogOffset(o) => o,
            PrintExposure::MidGreyAt { print_density } => {
                settings.log_exposure_for(print_density) + q * negative.grey_density
            }
            PrintExposure::BaseAt { print_density } => {
                settings.log_exposure_for(print_density) + q * negative.base_plus_fog
            }
        };
    let mut out = negative.density.zeros_like();
    out.data_mut()
        .par_iter_mut()
        .zip(negative.density.data().par_iter())
        .for_each(|(r, &d)| {
            let dp = settings.density_at(offset - q * f64::from(d));
            *r = 10f64.powf(-(dp - d_min)) as f32;
        });
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn higher_grade_has_shorter_range() {
        let soft = PrintSettings {
            grade: 0.0,
            ..Default::default()
        };
        let hard = PrintSettings {
            grade: 5.0,
            ..Default::default()
        };
        assert!(hard.log_exposure_range() < soft.log_exposure_range());
    }

    #[test]
    fn log_exposure_for_inverts_density_at() {
        let s = PrintSettings::default();
        for d in [0.1, 0.75, 1.5, 2.0] {
            assert!((s.density_at(s.log_exposure_for(d)) - d).abs() < 1e-9);
        }
    }
}
