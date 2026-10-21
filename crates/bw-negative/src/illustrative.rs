//! **Illustrative** films and film/developer calibrations.
//!
//! These parameters exist to exercise the engine end to end before real calibrations
//! are fitted. Curve shapes are invented to be plausible. A few anchor points come
//! from documents: Kodak's Tri-X granularity (17) and Double-X granularity (14), and
//! development times from `data/development/times.csv`. Nothing here is fitted, and
//! no output from these parameters is an emulation of the named product. Every value
//! carries `CalibrationLevel::Illustrative` provenance.
//!
//! T-Max granularity (8 and 10) is Kodak's figure measured in D-76 (F-4016, F-4043);
//! it is divided by √0.9 because grain area is defined at `m_dev = 1` and D-76 stock
//! has `m_dev = 0.9` here, so a T-Max/D-76 render reproduces the published value.

use bw_core::{MonotoneCurve, Provenance};
use bw_developer::{
    AdjacencyParams, Agitation, DevelopmentCalibration, FogModel, GrainModifier, SupportedRange,
};
use bw_film::{FilmStock, GrainParams, SpectralWeights};

use crate::times;

pub const FILMS: [&str; 12] = [
    "tri-x",
    "tmax-100",
    "tmax-400",
    "hp5",
    "fp4",
    "delta-100",
    "delta-400",
    "delta-3200",
    "double-x",
    "kentmere-100",
    "kentmere-200",
    "kentmere-400",
];

/// Developer combinations (developer + dilution).
pub const DEVELOPERS: [&str; 11] = [
    "d76-stock",
    "d76-1+1",
    "d96-stock",
    "hc110-b",
    "hc110-e",
    "hc110-h",
    "hc110-g",
    "rodinal-1+25",
    "rodinal-1+50",
    "rodinal-1+100",
    "fx55-1+9",
];

/// Developer families and their dilutions, in display order.
pub const DEVELOPER_FAMILIES: [(&str, &[&str]); 5] = [
    ("d76", &["stock", "1+1"]),
    ("d96", &["stock"]),
    ("hc110", &["b", "e", "h", "g"]),
    ("rodinal", &["1+25", "1+50", "1+100"]),
    ("fx55", &["1+9"]),
];

/// Map ids saved by earlier versions to current developer combinations.
pub fn canonical_developer(id: &str) -> &str {
    match id {
        "d76" => "d76-stock",
        "hc110" => "hc110-b",
        "rodinal-50" => "rodinal-1+50",
        "rodinal-100" => "rodinal-1+100",
        other => other,
    }
}

/// Split a developer combination id into `(family, dilution)`.
pub fn split_developer(id: &str) -> (&str, &str) {
    canonical_developer(id).split_once('-').unwrap_or((id, ""))
}

struct FilmSpec {
    id: &'static str,
    name: &'static str,
    iso: f64,
    /// EI the film is usually rated at; differs from ISO for Delta 3200.
    box_ei: f64,
    base: f64,
    weights: SpectralWeights,
    irradiation_um: f64,
    rms_granularity: f64,
    /// Contrast (mean gradient) reached at the normal development time.
    normal_gradient: f64,
    /// Toe softness (log H units). Tabular emulsions get a shorter toe.
    toe: f64,
}

struct DeveloperSpec {
    alpha_per_c: f64,
    speed_shift_log_h: f64,
    kappa_stand: f64,
    fog: FogModel,
    adjacency: f64,
    m_dev: f64,
}

fn film_spec(id: &str) -> Option<FilmSpec> {
    let w = |r, g, b| SpectralWeights { r, g, b };
    #[rustfmt::skip]
    let spec = match id {
        "tri-x" => FilmSpec { id: "tri-x", name: "Tri-X-like 400", iso: 400.0, box_ei: 400.0, base: 0.25,
            weights: w(0.24, 0.56, 0.20), irradiation_um: 3.0, rms_granularity: 17.0, normal_gradient: 0.58, toe: 0.25 },
        "tmax-100" => FilmSpec { id: "tmax-100", name: "T-Max-100-like", iso: 100.0, box_ei: 100.0, base: 0.20,
            weights: w(0.22, 0.58, 0.20), irradiation_um: 1.6, rms_granularity: 8.0 / 0.948_7, normal_gradient: 0.60, toe: 0.16 },
        "tmax-400" => FilmSpec { id: "tmax-400", name: "T-Max-400-like", iso: 400.0, box_ei: 400.0, base: 0.22,
            weights: w(0.23, 0.57, 0.20), irradiation_um: 2.2, rms_granularity: 10.0 / 0.948_7, normal_gradient: 0.60, toe: 0.18 },
        "kentmere-100" => FilmSpec { id: "kentmere-100", name: "Kentmere-100-like", iso: 100.0, box_ei: 100.0, base: 0.21,
            weights: w(0.25, 0.55, 0.20), irradiation_um: 2.4, rms_granularity: 11.0, normal_gradient: 0.62, toe: 0.24 },
        "kentmere-200" => FilmSpec { id: "kentmere-200", name: "Kentmere-200-like", iso: 200.0, box_ei: 200.0, base: 0.22,
            weights: w(0.25, 0.55, 0.20), irradiation_um: 2.8, rms_granularity: 14.0, normal_gradient: 0.60, toe: 0.27 },
        "kentmere-400" => FilmSpec { id: "kentmere-400", name: "Kentmere-400-like", iso: 400.0, box_ei: 400.0, base: 0.23,
            weights: w(0.26, 0.54, 0.20), irradiation_um: 3.3, rms_granularity: 19.0, normal_gradient: 0.56, toe: 0.30 },
        "hp5" => FilmSpec { id: "hp5", name: "HP5-like 400", iso: 400.0, box_ei: 400.0, base: 0.22,
            weights: w(0.26, 0.54, 0.20), irradiation_um: 3.2, rms_granularity: 18.0, normal_gradient: 0.56, toe: 0.32 },
        "fp4" => FilmSpec { id: "fp4", name: "FP4-like 125", iso: 125.0, box_ei: 125.0, base: 0.20,
            weights: w(0.22, 0.58, 0.20), irradiation_um: 2.2, rms_granularity: 11.0, normal_gradient: 0.62, toe: 0.22 },
        "delta-100" => FilmSpec { id: "delta-100", name: "Delta-100-like", iso: 100.0, box_ei: 100.0, base: 0.18,
            weights: w(0.23, 0.57, 0.20), irradiation_um: 1.8, rms_granularity: 8.0, normal_gradient: 0.62, toe: 0.18 },
        "delta-400" => FilmSpec { id: "delta-400", name: "Delta-400-like", iso: 400.0, box_ei: 400.0, base: 0.20,
            weights: w(0.24, 0.56, 0.20), irradiation_um: 2.5, rms_granularity: 12.0, normal_gradient: 0.60, toe: 0.20 },
        "delta-3200" => FilmSpec { id: "delta-3200", name: "Delta-3200-like", iso: 1000.0, box_ei: 3200.0, base: 0.24,
            weights: w(0.26, 0.54, 0.20), irradiation_um: 3.5, rms_granularity: 22.0, normal_gradient: 0.58, toe: 0.24 },
        "double-x" => FilmSpec { id: "double-x", name: "Double-X-like 250", iso: 250.0, box_ei: 250.0, base: 0.28,
            weights: w(0.27, 0.53, 0.20), irradiation_um: 3.0, rms_granularity: 14.0, normal_gradient: 0.60, toe: 0.26 },
        _ => return None,
    };
    Some(spec)
}

/// Developer behaviour by family and dilution. More dilute solutions exhaust faster
/// at dense areas (more compensation and edge effect) and have less solvent action.
fn developer_spec(id: &str) -> Option<DeveloperSpec> {
    let fog = |f0, f1_per_min| FogModel { f0, f1_per_min };
    #[rustfmt::skip]
    let spec = match canonical_developer(id) {
        "d76-stock" => DeveloperSpec { alpha_per_c: 0.087, speed_shift_log_h: 0.0, kappa_stand: 0.15, fog: fog(0.04, 0.004), adjacency: 0.04, m_dev: 0.90 },
        "d76-1+1" => DeveloperSpec { alpha_per_c: 0.055, speed_shift_log_h: 0.0, kappa_stand: 0.30, fog: fog(0.04, 0.003), adjacency: 0.08, m_dev: 0.98 },
        "d96-stock" => DeveloperSpec { alpha_per_c: 0.087, speed_shift_log_h: 0.0, kappa_stand: 0.20, fog: fog(0.04, 0.004), adjacency: 0.05, m_dev: 0.95 },
        "hc110-b" => DeveloperSpec { alpha_per_c: 0.098, speed_shift_log_h: 0.0, kappa_stand: 0.30, fog: fog(0.05, 0.006), adjacency: 0.10, m_dev: 1.00 },
        "hc110-e" => DeveloperSpec { alpha_per_c: 0.095, speed_shift_log_h: 0.0, kappa_stand: 0.40, fog: fog(0.05, 0.005), adjacency: 0.12, m_dev: 1.00 },
        "hc110-h" => DeveloperSpec { alpha_per_c: 0.093, speed_shift_log_h: 0.0, kappa_stand: 0.50, fog: fog(0.05, 0.004), adjacency: 0.14, m_dev: 1.02 },
        "hc110-g" => DeveloperSpec { alpha_per_c: 0.090, speed_shift_log_h: 0.0, kappa_stand: 0.70, fog: fog(0.04, 0.003), adjacency: 0.18, m_dev: 1.05 },
        "rodinal-1+25" => DeveloperSpec { alpha_per_c: 0.09, speed_shift_log_h: 0.05, kappa_stand: 0.35, fog: fog(0.03, 0.003), adjacency: 0.20, m_dev: 1.35 },
        "rodinal-1+50" => DeveloperSpec { alpha_per_c: 0.09, speed_shift_log_h: 0.10, kappa_stand: 0.50, fog: fog(0.03, 0.002), adjacency: 0.25, m_dev: 1.30 },
        "fx55-1+9" => DeveloperSpec { alpha_per_c: 0.09, speed_shift_log_h: -0.15, kappa_stand: 0.30, fog: fog(0.04, 0.004), adjacency: 0.15, m_dev: 0.95 },
        "rodinal-1+100" => DeveloperSpec { alpha_per_c: 0.09, speed_shift_log_h: 0.15, kappa_stand: 0.90, fog: fog(0.03, 0.001), adjacency: 0.30, m_dev: 1.25 },
        _ => return None,
    };
    Some(spec)
}

/// The EI a film is usually rated at (the zero point for push and pull).
pub fn box_ei(film: &str) -> Option<f64> {
    film_spec(film).map(|f| f.box_ei)
}

/// Published (or interpolated) time for `ei`; `None` when the combination has no data.
pub fn development_time(film: &str, developer: &str, ei: f64) -> Option<times::TimeLookup> {
    times::lookup(film, canonical_developer(developer), ei)
}

/// Normal development time at box speed (min).
pub fn normal_time(film: &str, developer: &str) -> Option<f64> {
    development_time(film, developer, box_ei(film)?).map(|t| t.time_min)
}

pub fn film(id: &str) -> Option<FilmStock> {
    let s = film_spec(id)?;
    Some(FilmStock {
        id: s.id.into(),
        name: s.name.into(),
        nominal_iso: s.iso,
        box_ei: s.box_ei,
        base_density: s.base,
        spectral_rec709: s.weights,
        irradiation_um: s.irradiation_um,
        grain: GrainParams {
            area_um2: GrainParams::area_from_rms_granularity(s.rms_granularity),
            clump_um: 2.5,
        },
        provenance: Provenance::illustrative("Shape-plausible placeholder film"),
    })
}

/// Film stock plus its illustrative calibration for a developer combination.
/// `None` when either is unknown or no development time is published for the pair.
pub fn combination(
    film_id: &str,
    developer_id: &str,
) -> Option<(FilmStock, DevelopmentCalibration)> {
    let f = film_spec(film_id)?;
    let developer_id = canonical_developer(developer_id);
    let d = developer_spec(developer_id)?;
    let t_normal = normal_time(film_id, developer_id)?;

    // The time constant scales with the published normal time, so every pair reaches
    // the same fraction of full development at its normal time.
    let tau_c0_min = 0.9 * t_normal;
    let g = 1.0 - (-t_normal / tau_c0_min).exp();
    let gamma_inf = f.normal_gradient / g;
    // Place the ISO speed point (0.10 above fog) at the film's ISO under normal development.
    let speed_point = (0.8 / f.iso).log10() + d.speed_shift_log_h;
    let d_inf = asymptotic_curve(gamma_inf, f.toe, speed_point, 0.1 / g);
    let (family, dilution) = split_developer(developer_id);

    let stock = film(film_id)?;
    let calibration = DevelopmentCalibration {
        film_id: f.id.into(),
        developer_id: family.into(),
        dilution: dilution.into(),
        d_inf,
        tau_c0_min,
        reference_agitation: Agitation::Intermittent { interval_s: 30.0 },
        alpha_per_c: d.alpha_per_c,
        agitation_rate_slope: 0.25,
        kappa_stand: d.kappa_stand,
        fog: d.fog,
        adjacency: AdjacencyParams {
            strength_ref: d.adjacency,
            length_um: 35.0,
        },
        grain: GrainModifier {
            m_dev: d.m_dev,
            p: 0.5,
            tau_ref_min: t_normal,
        },
        supported: SupportedRange {
            time_min: (0.5 * t_normal, 2.5 * t_normal),
            temperature_c: (18.0, 24.0),
        },
        provenance: Provenance::illustrative(
            "Curve shape invented; anchored to ISO speed and a normal gradient",
        ),
    };
    Some((stock, calibration))
}

/// Soft-toe straight line with a gentle shoulder:
/// `D∞ = Dsat · tanh(γ s ln(1 + e^((h − h0)/s)) / Dsat)`, with `h0` chosen so that
/// `D∞(speed_point) = target`.
fn asymptotic_curve(gamma: f64, toe: f64, speed_point: f64, target: f64) -> MonotoneCurve {
    const D_SAT: f64 = 3.2;
    let shape = |h: f64, h0: f64| {
        let x = (h - h0) / toe;
        let softplus = if x > 30.0 { x } else { x.exp().ln_1p() };
        D_SAT * (gamma * toe * softplus / D_SAT).tanh()
    };
    let (mut lo, mut hi) = (speed_point - 3.0, speed_point + 3.0);
    for _ in 0..80 {
        let mid = 0.5 * (lo + hi);
        if shape(speed_point, mid) > target {
            lo = mid
        } else {
            hi = mid
        }
    }
    let h0 = 0.5 * (lo + hi);
    let points: Vec<(f64, f64)> = (0..=80)
        .map(|i| {
            let h = -6.0 + 0.1 * f64::from(i);
            (h, shape(h, h0))
        })
        .collect();
    MonotoneCurve::new(&points).expect("analytic curve is monotone")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sensitometry::iso_speed;
    use bw_developer::ProcessingConditions;

    fn conditions(time_min: f64) -> ProcessingConditions {
        ProcessingConditions {
            time_min,
            temperature_c: 20.0,
            agitation: Agitation::Intermittent { interval_s: 30.0 },
        }
    }

    #[test]
    fn every_available_combination_hits_iso_speed_at_normal_time() {
        let mut checked = 0;
        for film_id in FILMS {
            for dev in DEVELOPERS {
                let Some((film, cal)) = combination(film_id, dev) else {
                    continue;
                };
                let t = normal_time(film_id, dev).unwrap();
                let iso = iso_speed(&cal, &cal.resolve(&conditions(t))).unwrap();
                let stops = (iso.speed / film.nominal_iso).log2();
                let shift =
                    developer_spec(dev).unwrap().speed_shift_log_h / std::f64::consts::LOG10_2;
                assert!(
                    (stops + shift).abs() < 0.1,
                    "{film_id}/{dev}: {stops:+.2} stops"
                );
                checked += 1;
            }
        }
        assert!(checked > 0);
    }

    #[test]
    fn two_stop_push_gains_under_a_stop_of_real_speed() {
        let (_, cal) = combination("tri-x", "d76-stock").unwrap();
        let at = |t| iso_speed(&cal, &cal.resolve(&conditions(t))).unwrap().speed;
        let normal = normal_time("tri-x", "d76-stock").unwrap();
        let gain = (at(normal * 1.6) / at(normal)).log2();
        assert!(gain > 0.0 && gain < 1.0, "speed gain {gain:.2} stops");
    }

    #[test]
    fn legacy_ids_still_resolve() {
        assert_eq!(canonical_developer("hc110"), "hc110-b");
        assert_eq!(split_developer("rodinal-1+50"), ("rodinal", "1+50"));
        assert!(combination("tri-x", "d76").is_some());
    }

    #[test]
    fn every_developer_family_member_is_known() {
        for (family, dilutions) in DEVELOPER_FAMILIES {
            for d in dilutions {
                let id = format!("{family}-{d}");
                assert!(DEVELOPERS.contains(&id.as_str()), "{id}");
                assert!(developer_spec(&id).is_some(), "{id}");
            }
        }
    }
}
