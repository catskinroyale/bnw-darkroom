//! Styles: serialisable compositions of film, developer, processing and printing.
//!
//! A style never contains its own rendering logic (design constraint 2). It names
//! existing components and their settings, and [`render`] runs them through the
//! engine.

use bw_core::{CalibrationLevel, FilmGeometry, Plane};
use bw_darkroom::{PrintExposure, PrintSettings, print};
use bw_developer::{Agitation, ProcessingConditions};
use bw_negative::{
    ExposureSettings, Metering, Negative, RenderOptions, SceneImage, develop, illustrative,
    sensitometry,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StyleError {
    #[error("unknown film/developer combination {film}/{developer}")]
    UnknownCombination { film: String, developer: String },
    #[error(transparent)]
    Geometry(#[from] bw_core::GeometryError),
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum AgitationSpec {
    Continuous,
    Intermittent { interval_s: f64 },
    SemiStand { mid_agitations: u8 },
    Stand,
}

impl From<AgitationSpec> for Agitation {
    fn from(a: AgitationSpec) -> Self {
        match a {
            AgitationSpec::Continuous => Agitation::Continuous,
            AgitationSpec::Intermittent { interval_s } => Agitation::Intermittent { interval_s },
            AgitationSpec::SemiStand { mid_agitations } => Agitation::SemiStand { mid_agitations },
            AgitationSpec::Stand => Agitation::Stand,
        }
    }
}

impl AgitationSpec {
    pub fn label(&self) -> String {
        match self {
            Self::Continuous => "Continuous".into(),
            Self::Intermittent { interval_s } => format!("Every {interval_s:.0} s"),
            Self::SemiStand { mid_agitations } => format!("Semi-stand ({mid_agitations})"),
            Self::Stand => "Stand".into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DevelopmentSpec {
    /// Development time in minutes; `None` uses the combination's normal time.
    pub time_min: Option<f64>,
    pub temperature_c: f64,
    pub agitation: AgitationSpec,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PrintSpec {
    /// Paper grade 0–5.
    pub grade: f64,
    /// Extra enlarger exposure in stops relative to the auto mid-grey exposure.
    pub exposure_stops: f64,
}

/// A complete film → developer → print recipe.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Style {
    pub name: String,
    pub film: String,
    pub developer: String,
    pub exposure_index: f64,
    #[serde(default)]
    pub ev_compensation: f64,
    pub development: DevelopmentSpec,
    pub print: PrintSpec,
    #[serde(default = "default_seed")]
    pub seed: u64,
    /// Width of the film frame in mm (36 = 35 mm, 56 = 120 square/645 short side, 120 = 4×5).
    #[serde(default = "default_format")]
    pub format_mm: f64,
    /// How much of the frame's width the image uses: 1 = full frame, 2 = the central
    /// half enlarged. Larger values enlarge the negative more, so grain shows more.
    #[serde(default = "default_crop")]
    pub crop: f64,
}

fn default_seed() -> u64 {
    1
}

fn default_format() -> f64 {
    36.0
}

fn default_crop() -> f64 {
    1.0
}

/// How visible grain will be, from film, development and enlargement.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GrainReadout {
    /// Diffuse RMS granularity ×1000 at net density 1.0, 48 µm aperture, after development.
    pub granularity: f64,
    /// Granularity scaled to the enlargement of a full 35 mm frame (36 mm wide):
    /// what the eye compares when prints are viewed at the same size.
    pub at_print: f64,
}

impl GrainReadout {
    pub fn describe(&self) -> &'static str {
        match self.at_print {
            g if g < 8.0 => "very fine",
            g if g < 12.0 => "fine",
            g if g < 17.0 => "moderate",
            g if g < 24.0 => "pronounced",
            g if g < 34.0 => "coarse",
            _ => "very coarse",
        }
    }
}

impl Style {
    /// Width of film the image spans, in mm.
    pub fn frame_width_mm(&self) -> f64 {
        self.format_mm / self.crop.max(1.0)
    }

    pub fn normal_time(&self) -> Option<f64> {
        illustrative::normal_time(&self.film, &self.developer)
    }

    /// The EI that push and pull are measured from.
    pub fn box_ei(&self) -> f64 {
        illustrative::box_ei(&self.film).unwrap_or(400.0)
    }

    /// The EI the film is exposed at: always box speed. The source file is already
    /// correctly exposed, so push and pull change development only (Zone System N+ and
    /// N−); exposure compensation under- or over-exposes the film.
    pub fn exposure_ei(&self) -> f64 {
        self.box_ei()
    }

    /// Film-plane log exposure (log lux·s) of the metered mid-grey.
    fn grey_log_h(&self) -> f64 {
        (bw_negative::METER_QK / self.exposure_ei()).log10()
            + std::f64::consts::LOG10_2 * self.ev_compensation
    }

    /// Push (+) or pull (−) in stops relative to the film's box EI: development for
    /// that EI, with the exposure kept.
    pub fn push_stops(&self) -> f64 {
        (self.exposure_index / self.box_ei()).log2()
    }

    /// Develop `stops` from box speed, following the published time for that EI.
    pub fn set_push_stops(&mut self, stops: f64) {
        self.exposure_index = (self.box_ei() * 2f64.powf(stops)).round();
        self.development.time_min = None;
    }

    /// The published (or interpolated) development time for the current EI.
    pub fn recommended_time(&self) -> Option<bw_negative::times::TimeLookup> {
        illustrative::development_time(&self.film, &self.developer, self.exposure_index)
    }

    /// Whether a development time is published for this film and developer.
    pub fn is_available(&self) -> bool {
        is_available(&self.film, &self.developer)
    }

    pub fn processing(&self) -> ProcessingConditions {
        ProcessingConditions {
            time_min: self
                .development
                .time_min
                .or_else(|| self.recommended_time().map(|t| t.time_min))
                .unwrap_or(7.0),
            temperature_c: self.development.temperature_c,
            agitation: self.development.agitation.into(),
        }
    }

    /// Print settings. The enlarger exposure holds the black point that normal
    /// development of this film and developer gives with mid-grey printed at the default
    /// density, so a normally developed negative prints exactly as with mid-grey
    /// anchoring, and longer or shorter development opens or closes the tones from there:
    /// blacks hold, shadows open slightly and highlights brighten with a push.
    pub fn print_settings(&self) -> PrintSettings {
        let mut settings = PrintSettings {
            grade: self.print.grade,
            exposure_adjust_stops: self.print.exposure_stops,
            ..Default::default()
        };
        let PrintExposure::MidGreyAt { print_density } = settings.exposure else {
            return settings;
        };
        if let (Ok((_, calibration)), Some(normal_time)) = (self.combination(), self.normal_time())
        {
            let normal = calibration.resolve(&ProcessingConditions {
                time_min: normal_time,
                ..self.processing()
            });
            let grey_above_base = calibration.net_density(self.grey_log_h(), &normal) - normal.fog;
            settings.exposure = PrintExposure::BaseAt {
                print_density: settings.density_at(
                    settings.log_exposure_for(print_density) + settings.callier_q * grey_above_base,
                ),
            };
        }
        settings
    }

    /// Model sensitometry for this style's film/developer/process, without an image.
    pub fn model_summary(&self) -> Result<ModelSummary, StyleError> {
        let (film, calibration) = self.combination()?;
        let process = self.processing();
        let state = calibration.resolve(&process);
        let area = film.grain.area_um2 * state.grain_area_factor;
        let granularity = 1000.0 * (0.434 * area / bw_film::GrainParams::APERTURE_48_UM2).sqrt();
        Ok(ModelSummary {
            grain: GrainReadout {
                granularity,
                at_print: granularity * 36.0 / self.frame_width_mm(),
            },
            time_origin: if self.development.time_min.is_some() {
                None
            } else {
                self.recommended_time().map(|t| t.origin)
            },
            film_name: film.name,
            calibration_level: calibration.provenance.level,
            iso: sensitometry::iso_speed(&calibration, &state),
            curve: sensitometry::characteristic_curve(
                &calibration,
                &state,
                film.base_density,
                (-4.5, 0.5),
                101,
            ),
            grey_log_h: self.grey_log_h(),
            warnings: calibration.warnings(&process),
            time_min: process.time_min,
        })
    }

    fn combination(
        &self,
    ) -> Result<(bw_film::FilmStock, bw_developer::DevelopmentCalibration), StyleError> {
        illustrative::combination(&self.film, &self.developer).ok_or_else(|| {
            StyleError::UnknownCombination {
                film: self.film.clone(),
                developer: self.developer.clone(),
            }
        })
    }
}

pub struct ModelSummary {
    pub grain: GrainReadout,
    /// Where the development time came from; `None` when the user set it.
    pub time_origin: Option<bw_negative::times::TimeOrigin>,
    pub film_name: String,
    pub calibration_level: CalibrationLevel,
    pub iso: Option<sensitometry::IsoSpeed>,
    pub curve: Vec<(f64, f64)>,
    pub grey_log_h: f64,
    pub warnings: Vec<String>,
    pub time_min: f64,
}

pub struct Rendered {
    pub print: Plane,
    pub negative: Negative,
}

/// Render a scene with a style. `frame_width_mm` is the width of film the image spans.
pub fn render(
    scene: &SceneImage,
    style: &Style,
    frame_width_mm: f64,
) -> Result<Rendered, StyleError> {
    let (w, h) = scene.dimensions();
    let geometry = FilmGeometry::from_frame_width(frame_width_mm, w, h)?;
    let (film, calibration) = style.combination()?;
    let exposure = ExposureSettings {
        exposure_index: style.exposure_ei(),
        metering: Metering::Anchor { grey: 0.18 },
        ev_compensation: style.ev_compensation,
        flare_fraction: 0.0,
    };
    let options = RenderOptions {
        seed: style.seed,
        geometry,
    };
    let negative = develop(
        scene,
        &exposure,
        &film,
        &style.processing(),
        &calibration,
        &options,
    );
    let print = print(&negative, &style.print_settings());
    Ok(Rendered { print, negative })
}

/// Everything needed to reproduce an export, written next to it as JSON.
#[derive(Serialize)]
pub struct ExportRecord<'a> {
    pub engine_version: &'static str,
    pub source: String,
    pub style: &'a Style,
    pub resolved_time_min: f64,
    pub frame_width_mm: f64,
    pub calibration_level: &'static str,
    pub warnings: Vec<String>,
    pub note: &'static str,
    /// Per-image adjustments made before development (raw exposure, white balance, …).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_adjustments: Option<serde_json::Value>,
    /// Camera and capture settings, for raw sources.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub camera: Option<String>,
}

impl<'a> ExportRecord<'a> {
    pub fn new(source: String, style: &'a Style, frame_width_mm: f64, rendered: &Rendered) -> Self {
        let level = style
            .model_summary()
            .map(|m| m.calibration_level.label())
            .unwrap_or("unknown");
        Self {
            engine_version: env!("CARGO_PKG_VERSION"),
            source,
            style,
            resolved_time_min: style.processing().time_min,
            frame_width_mm,
            calibration_level: level,
            warnings: rendered.negative.warnings.clone(),
            note: "Film and developer parameters are illustrative placeholders, not fitted to manufacturer data.",
            image_adjustments: None,
            camera: None,
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("record is serialisable")
    }
}

/// Whether a development time is published for this film and developer combination.
pub fn is_available(film: &str, developer: &str) -> bool {
    illustrative::normal_time(film, developer).is_some()
}

pub fn developer_families() -> &'static [(&'static str, &'static [&'static str])] {
    &illustrative::DEVELOPER_FAMILIES
}

/// `(family, dilution)` of a developer combination id.
pub fn split_developer(id: &str) -> (&str, &str) {
    illustrative::split_developer(id)
}

pub fn developer_family_label(family: &str) -> &'static str {
    match family {
        "d76" => "D-76",
        "d96" => "D-96",
        "hc110" => "HC-110",
        "rodinal" => "Rodinal",
        "fx55" => "FX-55",
        _ => "unknown",
    }
}

pub fn dilution_label(family: &str, dilution: &str) -> String {
    match (family, dilution) {
        ("hc110", d) => format!(
            "{} ({})",
            d.to_ascii_uppercase(),
            match d {
                "b" => "1+31",
                "e" => "1+47",
                "h" => "1+63",
                "g" => "1+119",
                _ => "?",
            }
        ),
        (_, "stock") => "Stock".into(),
        (_, d) => d.to_string(),
    }
}

pub fn film_ids() -> &'static [&'static str] {
    &illustrative::FILMS
}

pub fn developer_ids() -> &'static [&'static str] {
    &illustrative::DEVELOPERS
}

pub fn film_label(id: &str) -> String {
    illustrative::film(id).map_or_else(|| id.to_string(), |f| f.name)
}

pub fn developer_label(id: &str) -> String {
    let (family, dilution) = split_developer(id);
    format!(
        "{} {}",
        developer_family_label(family),
        dilution_label(family, dilution)
    )
}

/// A photographer-inspired preset: an ordinary [`Style`] plus the reasoning behind it.
/// Presets compose existing components; they have no rendering logic of their own.
#[derive(Clone, Debug)]
pub struct Preset {
    pub photographer: &'static str,
    /// One line describing the look.
    pub look: &'static str,
    /// Which parts are documented (with sources) and which are interpretation.
    pub basis: &'static str,
    /// Image ▸ Contrast to set when the preset is applied (0 = leave as is).
    pub scene_contrast: f64,
    pub style: Style,
}

/// Photographer presets. Every value is an interpretation built from the app's own
/// components; sources and the documented/undocumented split are in
/// `docs/research/photographer-presets.md`.
pub fn photographer_presets() -> Vec<Preset> {
    #[allow(clippy::too_many_arguments)]
    fn style(
        name: &str,
        film: &str,
        developer: &str,
        push: f64,
        agitation: AgitationSpec,
        format_mm: f64,
        crop: f64,
        grade: f64,
        print_stops: f64,
    ) -> Style {
        let mut s = Style {
            name: name.into(),
            film: film.into(),
            developer: developer.into(),
            exposure_index: 400.0,
            ev_compensation: 0.0,
            development: DevelopmentSpec {
                time_min: None,
                temperature_c: 20.0,
                agitation,
            },
            print: PrintSpec {
                grade,
                exposure_stops: print_stops,
            },
            seed: 1,
            format_mm,
            crop,
        };
        s.set_push_stops(push);
        s
    }
    let every = |interval_s| AgitationSpec::Intermittent { interval_s };
    vec![
        Preset {
            photographer: "Don McCullin",
            look: "Dark, heavy, brooding prints with dense shadows and gritty 35 mm grain.",
            basis: "Documented: Tri-X in a Nikon F (he took thirty rolls to Vietnam); a yellow filter; \
                    deliberately dark prints (\"Darkness, to me, is energy\") with heavy dodging and \
                    burning, all printed himself. Not documented: developer and paper. HC-110 B, grade 3, \
                    +1 stop print exposure and scene contrast stand in for his printing.",
            scene_contrast: 20.0,
            style: style(
                "Inspired by Don McCullin",
                "tri-x",
                "hc110-b",
                0.0,
                every(30.0),
                36.0,
                1.2,
                3.0,
                1.0,
            ),
        },
        Preset {
            photographer: "Ansel Adams",
            look: "Full-scale large-format landscape: deep skies, separated tones, no visible grain.",
            basis: "Documented: the Zone System; HC-110 recommended in The Negative; Moonrise shot on 8×10 \
                    with water-bath development; later prints cooler, more contrasty and glossy, with \
                    Dektol and selenium toning. Interpretation: FP4 stands in for Isopan, and HC-110 \
                    dilution H with sparse agitation for dilution G and the water bath. No selenium toning \
                    or filters in the app.",
            scene_contrast: 15.0,
            style: style(
                "Inspired by Ansel Adams",
                "fp4",
                "hc110-h",
                0.0,
                every(60.0),
                203.0,
                1.0,
                3.0,
                0.5,
            ),
        },
        Preset {
            photographer: "Bill Brandt",
            look: "Stark post-war contrast: chalky highlights, crushed blacks, mid-tones sacrificed.",
            basis: "Documented: \"Before 1951, I liked my prints dark and muddy. Now I prefer the very \
                    contrasting black-and-white effect\"; compositions finished and cropped under the \
                    enlarger; very high-contrast Shadow of Light prints; 6×6 cameras (Rolleiflex, \
                    Hasselblad). Not documented: film and developer. Grade 5 and strong scene contrast \
                    are interpretation.",
            scene_contrast: 40.0,
            style: style(
                "Inspired by Bill Brandt",
                "hp5",
                "d76-stock",
                1.0,
                every(30.0),
                56.0,
                1.3,
                5.0,
                0.5,
            ),
        },
        Preset {
            photographer: "Josef Koudelka",
            look: "Hard, grainy wide-angle reportage: deep blacks with detail held at both ends.",
            basis: "Documented: cinema film in 1968 (Aperture); 35 mm with a 25 mm wide-angle; his \
                    printer describes high-contrast negatives printed to relatively high contrast with \
                    detail in highlights and shadows. Reported in one secondary source only: East German \
                    400 movie film pushed in hot developer. Double-X, itself a motion-picture film, pushed \
                    two stops stands in.",
            scene_contrast: 25.0,
            style: style(
                "Inspired by Josef Koudelka",
                "double-x",
                "hc110-b",
                2.0,
                every(30.0),
                36.0,
                1.0,
                3.0,
                0.5,
            ),
        },
        Preset {
            photographer: "Edward Weston",
            look: "8×10 contact print: luminous, finely detailed tones with long, smooth gradation.",
            basis: "Documented: an 8×10 view camera; pyro-soda tray development by inspection; contact \
                    prints only, under a bare bulb; amidol print developer; a move from matte palladium \
                    to glossy prints. Not documented: film stock. The app has no pyro: Rodinal 1+50 is the \
                    nearest high-acutance developer that holds highlights, without pyro's stain.",
            scene_contrast: 0.0,
            style: style(
                "Inspired by Edward Weston",
                "fp4",
                "rodinal-1+50",
                0.0,
                every(60.0),
                203.0,
                1.0,
                2.0,
                0.0,
            ),
        },
    ]
}

/// The styles shipped with the app. Named by process, not by photographer: profiles
/// attributed to photographers need documented sources (design 0001, brief §5).
pub fn builtin_styles() -> Vec<Style> {
    let normal = |time_min: Option<f64>| DevelopmentSpec {
        time_min,
        temperature_c: 20.0,
        agitation: AgitationSpec::Intermittent { interval_s: 30.0 },
    };
    let s = |name: &str, film: &str, developer: &str, push: f64, grade: f64| {
        let mut style = Style {
            name: name.into(),
            film: film.into(),
            developer: developer.into(),
            exposure_index: 400.0,
            ev_compensation: 0.0,
            development: normal(None),
            print: PrintSpec {
                grade,
                exposure_stops: 0.0,
            },
            seed: 1,
            format_mm: 36.0,
            crop: 1.0,
        };
        style.set_push_stops(push);
        style
    };
    let mut styles = vec![
        s("Tri-X · D-76", "tri-x", "d76-stock", 0.0, 2.0),
        s("Tri-X · HC-110 B", "tri-x", "hc110-b", 0.0, 2.0),
        s("Tri-X · Rodinal 1+50", "tri-x", "rodinal-1+50", 0.0, 2.0),
        s("Tri-X pushed 2 stops", "tri-x", "d76-stock", 2.0, 2.0),
        s("Tri-X pulled 1 stop", "tri-x", "d76-stock", -1.0, 2.0),
        s("HP5 · D-76", "hp5", "d76-stock", 0.0, 2.0),
        s("HP5 pushed 2 stops · HC-110 B", "hp5", "hc110-b", 2.0, 3.0),
        s("FP4 · D-76 1+1", "fp4", "d76-1+1", 0.0, 2.0),
        s("FP4 · Rodinal 1+50", "fp4", "rodinal-1+50", 0.0, 2.5),
        s("Delta 100 · D-76", "delta-100", "d76-stock", 0.0, 2.0),
        s("Delta 400 · HC-110 B", "delta-400", "hc110-b", 0.0, 2.0),
        s("Delta 3200 · D-76", "delta-3200", "d76-stock", 0.0, 2.0),
        s("Double-X · D-76", "double-x", "d76-stock", 0.0, 2.0),
        s("Double-X · D-96", "double-x", "d96-stock", 0.0, 2.0),
        s("T-Max 100 · D-76", "tmax-100", "d76-stock", 0.0, 2.0),
        s("T-Max 400 · HC-110 B", "tmax-400", "hc110-b", 0.0, 2.0),
        s(
            "T-Max 400 pushed 2 stops · D-76",
            "tmax-400",
            "d76-stock",
            2.0,
            2.0,
        ),
        s("HP5 · FX-55", "hp5", "fx55-1+9", 0.0, 2.0),
        s("Hard print · grade 4", "tri-x", "d76-stock", 0.0, 4.0),
        s("Soft print · grade 0", "hp5", "d76-stock", 0.0, 0.0),
    ];
    let mut stand = s("Rodinal 1+100 stand", "tri-x", "rodinal-1+100", 0.0, 2.0);
    stand.development = DevelopmentSpec {
        time_min: Some(60.0),
        temperature_c: 20.0,
        agitation: AgitationSpec::Stand,
    };
    styles.push(stand);
    styles.extend(photographer_presets().into_iter().map(|p| p.style));
    // Only ship styles whose film/developer pair has a published time.
    styles.retain(Style::is_available);
    styles
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_styles_resolve_and_round_trip() {
        for style in builtin_styles() {
            style.model_summary().expect("known combination");
            let json = serde_json::to_string(&style).unwrap();
            let back: Style = serde_json::from_str(&json).unwrap();
            assert_eq!(back, style);
        }
    }

    #[test]
    fn photographer_presets_are_renderable_builtins() {
        let presets = photographer_presets();
        assert_eq!(presets.len(), 5);
        let builtins = builtin_styles();
        for p in &presets {
            let summary = p
                .style
                .model_summary()
                .unwrap_or_else(|e| panic!("{}: {e}", p.photographer));
            assert!(summary.iso.is_some(), "{}", p.photographer);
            assert!(
                builtins.iter().any(|s| s.name == p.style.name),
                "{} not in builtins",
                p.photographer
            );
            assert!(
                !p.basis.is_empty() && p.basis.contains("ocumented"),
                "{}",
                p.photographer
            );
            assert!((-100.0..=100.0).contains(&p.scene_contrast));
        }
    }

    #[test]
    fn push_stops_round_trip_and_follow_published_times() {
        let mut s = builtin_styles().remove(0);
        s.set_push_stops(2.0);
        assert_eq!(s.exposure_index, 1600.0);
        assert!((s.push_stops() - 2.0).abs() < 1e-9);
        let normal = s.normal_time().unwrap();
        assert!(s.processing().time_min > normal, "a push develops longer");
        s.set_push_stops(-1.0);
        assert!(s.processing().time_min < normal, "a pull develops shorter");
    }

    #[test]
    fn grain_grows_with_enlargement() {
        let mut s = builtin_styles().remove(0);
        let full = s.model_summary().unwrap().grain;
        s.crop = 2.0;
        let cropped = s.model_summary().unwrap().grain;
        assert!((cropped.at_print / full.at_print - 2.0).abs() < 1e-9);
        assert_eq!(cropped.granularity, full.granularity);
    }

    #[test]
    fn old_styles_without_new_fields_still_load() {
        let json = r#"{"name":"x","film":"tri-x","developer":"d76","exposure_index":400,
            "development":{"time_min":null,"temperature_c":20,"agitation":{"kind":"stand"}},
            "print":{"grade":2,"exposure_stops":0}}"#;
        let s: Style = serde_json::from_str(json).unwrap();
        assert_eq!((s.format_mm, s.crop), (36.0, 1.0));
        assert!(s.is_available());
    }

    #[test]
    fn render_produces_print_of_scene_size() {
        let scene = SceneImage::Mono {
            y: Plane::from_fn(64, 40, |x, _| 0.01 + x as f32 / 64.0).unwrap(),
        };
        let r = render(&scene, &builtin_styles()[0], 36.0).unwrap();
        assert!(builtin_styles().len() >= 3);
        assert_eq!((r.print.width(), r.print.height()), (64, 40));
        assert!(r.print.data().iter().all(|v| v.is_finite()));
    }
}
