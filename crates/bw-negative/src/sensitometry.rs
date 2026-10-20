//! The virtual sensitometer (design 0001 §8.2): model curves without an image.

use bw_developer::{DevelopmentCalibration, DevelopmentState};

/// Total diffuse density vs log10 exposure (lux·s).
pub fn characteristic_curve(
    calibration: &DevelopmentCalibration,
    state: &DevelopmentState,
    base_density: f64,
    log_h: (f64, f64),
    samples: usize,
) -> Vec<(f64, f64)> {
    let step = (log_h.1 - log_h.0) / (samples.max(2) - 1) as f64;
    (0..samples.max(2))
        .map(|i| {
            let h = log_h.0 + step * i as f64;
            (h, base_density + calibration.net_density(h, state))
        })
        .collect()
}

/// ISO 6 speed criteria evaluated on the model curve.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IsoSpeed {
    /// log10 H where density is 0.10 above base+fog.
    pub speed_point_log_h: f64,
    /// Arithmetic speed `0.8 / H_m`.
    pub speed: f64,
    /// Density gain over the next 1.30 log H (ISO conditions require 0.80 ± 0.05).
    pub delta_d_1_30: f64,
}

pub fn iso_speed(
    calibration: &DevelopmentCalibration,
    state: &DevelopmentState,
) -> Option<IsoSpeed> {
    let above_fog = |h: f64| calibration.net_density(h, state) - state.fog;
    let (mut lo, mut hi) = (-7.0, 3.0);
    if above_fog(lo) > 0.1 || above_fog(hi) < 0.1 {
        return None;
    }
    for _ in 0..80 {
        let mid = 0.5 * (lo + hi);
        if above_fog(mid) < 0.1 {
            lo = mid
        } else {
            hi = mid
        }
    }
    let h_m = 0.5 * (lo + hi);
    Some(IsoSpeed {
        speed_point_log_h: h_m,
        speed: 0.8 / 10f64.powf(h_m),
        delta_d_1_30: above_fog(h_m + 1.3) - 0.1,
    })
}
