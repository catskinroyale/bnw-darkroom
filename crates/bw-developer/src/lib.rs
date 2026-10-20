//! Developer behaviour and processing conditions (design 0001 §3.3–3.5, §6.4).
//!
//! The development-growth model:
//!
//! ```text
//! D_net(h, τ) = D_f(τ) + D∞(h) · (1 − exp(−τ / τc(h)))
//! τc(h)       = τc0 · (1 + κ · D∞(h))
//! τ           = t · exp(α (T − 20 °C)) · r_A(agitation)
//! ```

use bw_core::{MonotoneCurve, Provenance};

/// Agitation scheme. Mapped to an agitation index in [0, 1] (0 = stand, 1 = continuous).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Agitation {
    /// Rotary or continuous inversion.
    Continuous,
    /// Agitation at a fixed interval (e.g. every 30 s or 60 s).
    Intermittent { interval_s: f64 },
    /// Initial agitation plus a few agitations during development.
    SemiStand { mid_agitations: u8 },
    /// Initial agitation only.
    Stand,
}

impl Agitation {
    /// Agitation index. The mapping is a Level-1 convention, not a measurement.
    pub fn index(&self) -> f64 {
        match *self {
            Self::Continuous => 1.0,
            Self::Intermittent { interval_s } => 1.0 / (1.0 + interval_s.max(1.0) / 120.0),
            Self::SemiStand { mid_agitations } => (0.1 + 0.05 * f64::from(mid_agitations)).min(0.4),
            Self::Stand => 0.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProcessingConditions {
    pub time_min: f64,
    pub temperature_c: f64,
    pub agitation: Agitation,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FogModel {
    pub f0: f64,
    pub f1_per_min: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AdjacencyParams {
    /// Nelson strength `B` at the reference agitation.
    pub strength_ref: f64,
    /// Diffusion length (Gaussian σ) in µm.
    pub length_um: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GrainModifier {
    /// Developer grain-area multiplier relative to the film's reference developer.
    pub m_dev: f64,
    /// Exponent for growth of grain area with development.
    pub p: f64,
    /// Effective development at which `m_dev` applies.
    pub tau_ref_min: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SupportedRange {
    pub time_min: (f64, f64),
    pub temperature_c: (f64, f64),
}

/// Empirical behaviour of one (film, developer, dilution). The only place where
/// developer behaviour is defined.
#[derive(Clone, Debug, PartialEq)]
pub struct DevelopmentCalibration {
    pub film_id: String,
    pub developer_id: String,
    pub dilution: String,
    /// Asymptotic net density vs log10 exposure (lux·s).
    pub d_inf: MonotoneCurve,
    pub tau_c0_min: f64,
    pub reference_agitation: Agitation,
    pub alpha_per_c: f64,
    /// d r_A / d(agitation index).
    pub agitation_rate_slope: f64,
    /// Compensation κ reached at stand development.
    pub kappa_stand: f64,
    pub fog: FogModel,
    pub adjacency: AdjacencyParams,
    pub grain: GrainModifier,
    pub supported: SupportedRange,
    pub provenance: Provenance,
}

/// A calibration resolved for specific processing conditions.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DevelopmentState {
    /// Effective development in reference-condition minutes.
    pub tau_min: f64,
    pub kappa: f64,
    pub adjacency_strength: f64,
    pub adjacency_length_um: f64,
    /// Multiplier on the film's grain area.
    pub grain_area_factor: f64,
    pub fog: f64,
}

impl DevelopmentCalibration {
    pub fn resolve(&self, p: &ProcessingConditions) -> DevelopmentState {
        let a = p.agitation.index();
        let a_ref = self.reference_agitation.index();
        let r_t = (self.alpha_per_c * (p.temperature_c - 20.0)).exp();
        let r_a = (1.0 + self.agitation_rate_slope * (a - a_ref)).max(0.1);
        let tau = p.time_min.max(0.0) * r_t * r_a;
        // Compensation only appears when agitation is below the reference.
        let deficit = ((a_ref - a) / a_ref.max(1e-6)).clamp(0.0, 1.0);
        DevelopmentState {
            tau_min: tau,
            kappa: self.kappa_stand * deficit,
            adjacency_strength: (self.adjacency.strength_ref * (1.0 + 1.5 * (a_ref - a))).max(0.0),
            adjacency_length_um: self.adjacency.length_um,
            grain_area_factor: self.grain.m_dev
                * (tau / self.grain.tau_ref_min).max(1e-3).powf(self.grain.p),
            fog: self.fog.f0 + self.fog.f1_per_min * tau,
        }
    }

    /// Net density above base (including fog) at log exposure `h`.
    pub fn net_density(&self, h: f64, s: &DevelopmentState) -> f64 {
        let d_inf = self.d_inf.eval(h).max(0.0);
        let tau_c = self.tau_c0_min * (1.0 + s.kappa * d_inf);
        s.fog + d_inf * (1.0 - (-s.tau_min / tau_c).exp())
    }

    /// Conditions outside the range backed by the calibration's data.
    pub fn warnings(&self, p: &ProcessingConditions) -> Vec<String> {
        let mut w = Vec::new();
        let (t0, t1) = self.supported.time_min;
        if !(t0..=t1).contains(&p.time_min) {
            w.push(format!(
                "time {:.1} min outside supported {t0}–{t1} min",
                p.time_min
            ));
        }
        let (c0, c1) = self.supported.temperature_c;
        if !(c0..=c1).contains(&p.temperature_c) {
            w.push(format!(
                "temperature {:.1} °C outside supported {c0}–{c1} °C",
                p.temperature_c
            ));
        }
        w
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn calibration() -> DevelopmentCalibration {
        DevelopmentCalibration {
            film_id: "test".into(),
            developer_id: "test".into(),
            dilution: "stock".into(),
            d_inf: MonotoneCurve::new(&[(-3.5, 0.0), (-2.5, 0.2), (-1.0, 1.4), (0.5, 2.6)])
                .unwrap(),
            tau_c0_min: 6.0,
            reference_agitation: Agitation::Intermittent { interval_s: 30.0 },
            alpha_per_c: 0.09,
            agitation_rate_slope: 0.25,
            kappa_stand: 0.5,
            fog: FogModel {
                f0: 0.05,
                f1_per_min: 0.01,
            },
            adjacency: AdjacencyParams {
                strength_ref: 0.1,
                length_um: 40.0,
            },
            grain: GrainModifier {
                m_dev: 1.0,
                p: 0.5,
                tau_ref_min: 6.0,
            },
            supported: SupportedRange {
                time_min: (4.0, 14.0),
                temperature_c: (18.0, 24.0),
            },
            provenance: Provenance::illustrative("test"),
        }
    }

    fn conditions(time_min: f64) -> ProcessingConditions {
        ProcessingConditions {
            time_min,
            temperature_c: 20.0,
            agitation: Agitation::Intermittent { interval_s: 30.0 },
        }
    }

    #[test]
    fn contrast_rises_and_saturates_with_time() {
        let c = calibration();
        let gamma = |t: f64| {
            let s = c.resolve(&conditions(t));
            (c.net_density(-1.0, &s) - c.net_density(-2.0, &s)) / 1.0
        };
        let (g4, g8, g16, g32) = (gamma(4.0), gamma(8.0), gamma(16.0), gamma(32.0));
        assert!(g4 < g8 && g8 < g16 && g16 < g32);
        assert!(g8 - g4 > g32 - g16, "growth must slow down");
    }

    #[test]
    fn extended_development_adds_little_shadow_density() {
        // Push behaviour (design 0001 §1.1): highlights gain far more than shadows.
        let c = calibration();
        let (n, p) = (c.resolve(&conditions(6.0)), c.resolve(&conditions(12.0)));
        let shadow_gain = c.net_density(-2.8, &p) - c.net_density(-2.8, &n);
        let highlight_gain = c.net_density(0.0, &p) - c.net_density(0.0, &n);
        assert!(highlight_gain > 5.0 * shadow_gain);
    }

    #[test]
    fn warmer_is_faster() {
        let c = calibration();
        let mut warm = conditions(6.0);
        warm.temperature_c = 24.0;
        assert!(c.resolve(&warm).tau_min > c.resolve(&conditions(6.0)).tau_min);
        assert!(!c.warnings(&warm).iter().any(|w| w.contains("temperature")));
        warm.temperature_c = 30.0;
        assert!(c.warnings(&warm).iter().any(|w| w.contains("temperature")));
    }

    #[test]
    fn stand_compensates_and_strengthens_edges() {
        let c = calibration();
        let normal = c.resolve(&conditions(6.0));
        let stand = c.resolve(&ProcessingConditions {
            agitation: Agitation::Stand,
            ..conditions(6.0)
        });
        assert_eq!(normal.kappa, 0.0);
        assert!(stand.kappa > 0.0);
        assert!(stand.adjacency_strength > normal.adjacency_strength);
    }
}
