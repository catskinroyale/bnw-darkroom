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
}

/// Print a negative. Returns reflectance relative to paper white (1.0 = paper base).
pub fn print(negative: &Negative, settings: &PrintSettings) -> Plane {
    let Paper { d_min, d_max } = settings.paper;
    let k = 2.0 * 19f64.ln() / settings.log_exposure_range();
    let q = settings.callier_q;
    let offset = std::f64::consts::LOG10_2 * settings.exposure_adjust_stops
        + match settings.exposure {
            PrintExposure::LogOffset(o) => o,
            PrintExposure::MidGreyAt { print_density } => {
                let frac = ((print_density - d_min) / (d_max - d_min)).clamp(1e-4, 1.0 - 1e-4);
                (frac / (1.0 - frac)).ln() / k + q * negative.grey_density
            }
        };
    let mut out = negative.density.zeros_like();
    out.data_mut()
        .par_iter_mut()
        .zip(negative.density.data().par_iter())
        .for_each(|(r, &d)| {
            let log_h = offset - q * f64::from(d);
            let dp = d_min + (d_max - d_min) / (1.0 + (-k * log_h).exp());
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
}
