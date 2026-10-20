//! Negative simulation (design 0001 §2–§5).
//!
//! ```text
//! SceneImage ─► spectral E ─► exposure H (lux·s) ─► irradiation H'
//!            ─► log ─► development curve D_mean ─► adjacency ─► grain ─► Negative
//! ```

pub mod illustrative;
pub mod sensitometry;
pub mod times;

use bw_core::{FilmGeometry, Lut1D, Plane, blur, noise};
use bw_developer::{DevelopmentCalibration, DevelopmentState, ProcessingConditions};
use bw_film::FilmStock;
use rayon::prelude::*;

/// Reflected-light meter calibration (ISO 2720): `H_m = q·K / EI` for a metered mid-tone.
pub const METER_QK: f64 = 0.65 * 12.5;

/// Range of log exposure covered by the development lookup table.
const LOG_H_RANGE: (f64, f64) = (-6.0, 2.0);
const LUT_SIZE: usize = 4096;

/// Scene-linear input.
#[derive(Clone, Debug)]
pub enum SceneImage {
    /// White-balanced, scene-linear Rec.709/sRGB primaries.
    Rgb { r: Plane, g: Plane, b: Plane },
    /// Native monochrome sensor response; no spectral emulation is possible.
    Mono { y: Plane },
}

impl SceneImage {
    pub fn dimensions(&self) -> (usize, usize) {
        let p = match self {
            Self::Rgb { r, .. } => r,
            Self::Mono { y } => y,
        };
        (p.width(), p.height())
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Metering {
    /// The scene value treated as metered mid-grey.
    Anchor { grey: f32 },
    /// Geometric mean of the film response, like an averaging meter.
    LogAverage,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ExposureSettings {
    pub exposure_index: f64,
    pub metering: Metering,
    pub ev_compensation: f64,
    /// Camera veiling flare as a fraction of mean exposure.
    pub flare_fraction: f64,
}

#[derive(Clone, Copy, Debug)]
pub struct RenderOptions {
    pub seed: u64,
    pub geometry: FilmGeometry,
}

/// A developed negative: total diffuse density per pixel plus the record needed to
/// print it and reproduce it.
#[derive(Clone, Debug)]
pub struct Negative {
    pub density: Plane,
    pub base_plus_fog: f64,
    /// Mean (grain-free) density at the metered mid-grey, useful for print exposure.
    pub grey_density: f64,
    pub geometry: FilmGeometry,
    pub state: DevelopmentState,
    pub warnings: Vec<String>,
}

/// Stage 1: film response `E`, linear in scene radiance (design 0001 §5.1).
pub fn film_response(scene: &SceneImage, film: &FilmStock) -> Plane {
    match scene {
        SceneImage::Mono { y } => y.clone(),
        SceneImage::Rgb { r, g, b } => {
            let w = film.spectral_rec709;
            let mut e = r.zeros_like();
            e.data_mut()
                .par_iter_mut()
                .zip(
                    r.data()
                        .par_iter()
                        .zip(g.data().par_iter().zip(b.data().par_iter())),
                )
                .for_each(|(e, (&r, (&g, &b)))| *e = (w.r * r + w.g * g + w.b * b).max(0.0));
            e
        }
    }
}

/// The scene value that the metering mode treats as mid-grey.
pub fn metered_grey(e: &Plane, metering: Metering) -> f64 {
    match metering {
        Metering::Anchor { grey } => f64::from(grey),
        Metering::LogAverage => {
            // Sequential f64 sum: deterministic regardless of thread count.
            let (sum, n) = e
                .data()
                .iter()
                .filter(|&&v| v > 1e-6)
                .fold((0.0f64, 0usize), |(s, n), &v| {
                    (s + f64::from(v).ln(), n + 1)
                });
            if n == 0 { 0.18 } else { (sum / n as f64).exp() }
        }
    }
}

/// Stage 2: film-plane exposure in lux·s (design 0001 §2.4).
pub fn expose(e: &Plane, settings: &ExposureSettings) -> Plane {
    let grey = metered_grey(e, settings.metering).max(1e-9);
    let scale = (METER_QK / settings.exposure_index) * 2f64.powf(settings.ev_compensation) / grey;
    let mut h = e.zeros_like();
    let scale = scale as f32;
    h.data_mut()
        .par_iter_mut()
        .zip(e.data().par_iter())
        .for_each(|(h, &e)| *h = e * scale);
    if settings.flare_fraction > 0.0 {
        let mean = h.data().iter().map(|&v| f64::from(v)).sum::<f64>() / h.data().len() as f64;
        let flare = (settings.flare_fraction * mean) as f32;
        h.data_mut().par_iter_mut().for_each(|v| *v += flare);
    }
    h
}

/// Develop a scene into a negative.
pub fn develop(
    scene: &SceneImage,
    exposure: &ExposureSettings,
    film: &FilmStock,
    process: &ProcessingConditions,
    calibration: &DevelopmentCalibration,
    options: &RenderOptions,
) -> Negative {
    let geometry = options.geometry;
    let state = calibration.resolve(process);
    let mut warnings = calibration.warnings(process);
    if let SceneImage::Mono { .. } = scene {
        warnings.push("monochrome input: spectral emulation skipped".into());
    }

    let e = film_response(scene, film);
    let grey_h = (METER_QK / exposure.exposure_index * 2f64.powf(exposure.ev_compensation)).log10();
    let h = expose(&e, exposure);
    drop(e);

    // Irradiation acts on linear exposure, before the curve.
    let h = blur::gaussian_blur(&h, geometry.um_to_px(film.irradiation_um));

    let curve = Lut1D::from_fn(LOG_H_RANGE.0, LOG_H_RANGE.1, LUT_SIZE, |lh| {
        calibration.net_density(lh, &state)
    })
    .expect("static LUT range is valid");
    let mut net = h.zeros_like();
    net.data_mut()
        .par_iter_mut()
        .zip(h.data().par_iter())
        .for_each(|(d, &h)| *d = curve.eval(h.max(1e-12).log10()));
    drop(h);

    apply_adjacency(&mut net, &state, &geometry);
    apply_grain(
        &mut net,
        film.grain.area_um2 * state.grain_area_factor,
        film.grain.clump_um,
        options.seed,
        &geometry,
    );

    let base = film.base_density as f32;
    net.data_mut()
        .par_iter_mut()
        .for_each(|d| *d = (*d).max(0.0) + base);

    Negative {
        density: net,
        base_plus_fog: film.base_density + state.fog,
        grey_density: film.base_density + calibration.net_density(grey_h, &state),
        geometry,
        state,
        warnings,
    }
}

/// Nelson-type adjacency on net density (design 0001 §3.5). Zero on uniform fields.
fn apply_adjacency(net: &mut Plane, state: &DevelopmentState, geometry: &FilmGeometry) {
    let b = state.adjacency_strength as f32;
    if b <= 0.0 {
        return;
    }
    let neighbourhood = blur::gaussian_blur(net, geometry.um_to_px(state.adjacency_length_um));
    net.data_mut()
        .par_iter_mut()
        .zip(neighbourhood.data().par_iter())
        .for_each(|(d, &s)| *d = (*d - b * *d * (s - *d)).max(0.0));
}

/// Poisson/Selwyn grain (design 0001 §5.2–5.3).
fn apply_grain(net: &mut Plane, area_um2: f64, clump_um: f64, seed: u64, geometry: &FilmGeometry) {
    let (w, h) = (net.width(), net.height());
    let white = Plane::from_fn(w, h, |x, y| noise::gaussian(seed, 0, x as u32, y as u32))
        .expect("dimensions come from an existing plane");
    let field = blur::gaussian_blur(&white, geometry.um_to_px(clump_um));
    let k = (0.434 * area_um2 / geometry.pixel_area_um2()) as f32;
    net.data_mut()
        .par_iter_mut()
        .zip(field.data().par_iter())
        .for_each(|(d, &n)| *d += (k * d.max(0.0)).sqrt() * n);
}

#[cfg(test)]
mod tests {
    use super::*;
    use bw_developer::Agitation;

    fn setup() -> (FilmStock, DevelopmentCalibration, ProcessingConditions) {
        let (film, cal) = illustrative::combination("tri-x", "d76-stock").unwrap();
        let p = ProcessingConditions {
            time_min: 6.75,
            temperature_c: 20.0,
            agitation: Agitation::Intermittent { interval_s: 30.0 },
        };
        (film, cal, p)
    }

    #[test]
    fn metered_grey_lands_at_meter_exposure() {
        let e = Plane::filled(4, 4, 0.18).unwrap();
        let s = ExposureSettings {
            exposure_index: 400.0,
            metering: Metering::Anchor { grey: 0.18 },
            ev_compensation: 1.0,
            flare_fraction: 0.0,
        };
        let h = expose(&e, &s);
        let expected = 2.0 * METER_QK / 400.0;
        assert!((f64::from(h.get(0, 0)) - expected).abs() < 1e-6);
    }

    #[test]
    fn develop_is_deterministic_across_threads() {
        let (film, cal, p) = setup();
        let scene = SceneImage::Mono {
            y: Plane::from_fn(200, 120, |x, y| {
                0.01 + (x as f32 / 200.0) * (1.0 + (y % 7) as f32 / 7.0)
            })
            .unwrap(),
        };
        let exposure = ExposureSettings {
            exposure_index: 400.0,
            metering: Metering::Anchor { grey: 0.18 },
            ev_compensation: 0.0,
            flare_fraction: 0.0,
        };
        let options = RenderOptions {
            seed: 11,
            geometry: FilmGeometry::full_frame_35mm(200, 120).unwrap(),
        };
        let run = |threads| {
            rayon::ThreadPoolBuilder::new()
                .num_threads(threads)
                .build()
                .unwrap()
                .install(|| develop(&scene, &exposure, &film, &p, &cal, &options))
        };
        let (a, b) = (run(1), run(6));
        assert!(
            a.density
                .data()
                .iter()
                .zip(b.density.data())
                .all(|(x, y)| x.to_bits() == y.to_bits())
        );
    }

    #[test]
    fn grain_matches_published_granularity_through_48um_aperture() {
        // Design 0001 §8.1 Selwyn test: uniform field at net D ≈ 1, measured through a
        // 48 µm aperture, must give RMS ≈ G/1000 with G recovered from the grain area.
        let (film, _, _) = setup();
        let n = 1200;
        let geometry = FilmGeometry::from_frame_width(n as f64 * 4.0 / 1000.0, n, n).unwrap(); // 4 µm pixels
        let mut net = Plane::filled(n, n, 1.0).unwrap();
        apply_grain(
            &mut net,
            film.grain.area_um2,
            film.grain.clump_um,
            5,
            &geometry,
        );
        // Box aperture with the same area as a 48 µm circle.
        let side_px = ((bw_film::GrainParams::APERTURE_48_UM2).sqrt() / 4.0).round() as usize;
        let mut samples = Vec::new();
        for by in (0..n - side_px).step_by(side_px) {
            for bx in (0..n - side_px).step_by(side_px) {
                let mut s = 0.0f64;
                for y in by..by + side_px {
                    for x in bx..bx + side_px {
                        s += f64::from(net.get(x, y));
                    }
                }
                samples.push(s / (side_px * side_px) as f64);
            }
        }
        let mean = samples.iter().sum::<f64>() / samples.len() as f64;
        let rms =
            (samples.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / samples.len() as f64).sqrt();
        let expected = (0.434 * film.grain.area_um2 / bw_film::GrainParams::APERTURE_48_UM2).sqrt();
        assert!(
            (rms / expected - 1.0).abs() < 0.12,
            "rms {rms}, expected {expected}"
        );
    }
}
