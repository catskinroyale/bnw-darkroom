//! The Image panel: per-image adjustments made before black-and-white development.
//! Scene-referred only (exposure, white balance, highlight handling, rotation): tone
//! is left to the film, developer and paper.

use bw_raw::{Highlights, RawAdjust, WhiteBalance};
use eframe::egui;

use crate::source::Source;

mod help {
    pub const EXPOSURE: &str = "Brightens or darkens the scene before it reaches the film, in stops.\nUse it to correct the camera's exposure; push and pull are set in the workflow.";
    pub const CONTRAST: &str = "Scene contrast before the film, around mid-grey, like changing the lighting ratio.\nThe film and paper still shape the print; use paper grade for print contrast.";
    pub const AUTO_EXPOSURE: &str =
        "Set exposure like an averaging meter: the scene's average brightness becomes mid-grey.";
    pub const WB: &str = "Which light counts as neutral. Film turns colours into greys,\nso white balance changes which tones come out lighter or darker.";
    pub const TEMP: &str = "Colour temperature of the light in kelvin: lower for warm tungsten light, higher for shade.";
    pub const TINT: &str =
        "Green–magenta balance of the light; corrects fluorescent and mixed lighting.";
    pub const HIGHLIGHTS: &str = "What to do where the sensor clipped.";
    pub const RECONSTRUCT: &str =
        "Rebuild partly clipped highlights from the colour channels that still hold detail.";
    pub const CLIP: &str = "Render clipped areas as neutral white, with no reconstruction.";
    pub const ROTATE_LEFT: &str = "Rotate the image 90° anticlockwise.";
    pub const ROTATE_RIGHT: &str = "Rotate the image 90° clockwise.";
    pub const RESET: &str = "Back to the camera's settings: no exposure change, as-shot white balance, no extra rotation.";
    pub const RENDERED: &str = "This is an already-processed image, so white balance and highlight recovery need a raw file.";
}

fn wb_help(wb: &WhiteBalance) -> &'static str {
    match wb {
        WhiteBalance::AsShot => "The white balance the camera recorded.",
        WhiteBalance::Auto => "Estimated from the image, assuming the scene averages to grey.",
        WhiteBalance::Daylight => "Direct sunlight, about 5500 K.",
        WhiteBalance::Cloudy => "Overcast daylight, about 6500 K.",
        WhiteBalance::Shade => "Open shade lit by blue sky, about 7500 K.",
        WhiteBalance::Tungsten => "Household incandescent light, about 2850 K.",
        WhiteBalance::Fluorescent => {
            "Cool-white fluorescent tubes, about 3800 K with a green cast."
        }
        WhiteBalance::Flash => "Electronic flash, about 5500 K.",
        WhiteBalance::Custom { .. } => "Your own temperature and tint.",
    }
}

fn chip(ui: &mut egui::Ui, selected: bool, label: &str, help: &str) -> bool {
    ui.add(egui::Button::selectable(selected, label))
        .on_hover_text(help)
        .clicked()
}

/// Draw the panel. Returns true when `adjust` changed.
pub fn show(
    ui: &mut egui::Ui,
    adjust: &mut RawAdjust,
    source: Option<&Source>,
    detailed: bool,
) -> bool {
    let before = *adjust;
    let Some(source) = source else {
        ui.label(egui::RichText::new("No image open").weak());
        return false;
    };

    match source.capture() {
        Some(info) => {
            let mut line = info.summary();
            if info.monochrome {
                line.push_str(" · monochrome sensor");
            }
            ui.label(egui::RichText::new(line).small())
                .on_hover_text(format!("Raw file, {} × {} px", info.width, info.height));
            if info.spectral_matrix {
                ui.label(egui::RichText::new("Colour from measured camera spectral sensitivities").small().weak())
                    .on_hover_text("Camera matrices fitted to this camera's measured spectral sensitivities (rawtoaces-data, Academy Software Foundation).");
            }
            if info.matrix_is_fallback {
                ui.colored_label(
                    ui.visuals().warn_fg_color,
                    "No colour data for this camera: colours, and so their grey values, are approximate.",
                );
            }
        }
        None => {
            ui.label(egui::RichText::new("Rendered image (TIFF/JPEG/PNG)").small())
                .on_hover_text(help::RENDERED);
        }
    }

    // Exposure
    ui.horizontal(|ui| {
        ui.label("Exposure").on_hover_text(help::EXPOSURE);
        ui.add(
            egui::Slider::new(&mut adjust.exposure_ev, -4.0..=4.0)
                .step_by(1.0 / 6.0)
                .suffix(" EV"),
        )
        .on_hover_text(help::EXPOSURE);
        if ui
            .small_button("Auto")
            .on_hover_text(help::AUTO_EXPOSURE)
            .clicked()
        {
            adjust.exposure_ev = auto_exposure(source, adjust);
        }
    });

    ui.horizontal(|ui| {
        ui.label("Contrast").on_hover_text(help::CONTRAST);
        ui.add(egui::Slider::new(&mut adjust.contrast, -100.0..=100.0).step_by(5.0))
            .on_hover_text(help::CONTRAST);
    });

    let raw = source.raw();
    let colour = raw.is_some_and(|p| !p.info.monochrome);

    // White balance (colour raw files only)
    ui.add_space(4.0);
    ui.label(egui::RichText::new("White balance").strong())
        .on_hover_text(help::WB);
    if let (true, Some(photo)) = (colour, raw) {
        ui.horizontal_wrapped(|ui| {
            for wb in WhiteBalance::PRESETS {
                if chip(ui, adjust.white_balance == wb, wb.label(), wb_help(&wb)) {
                    adjust.white_balance = wb;
                }
            }
        });
        if detailed || matches!(adjust.white_balance, WhiteBalance::Custom { .. }) {
            let (shown_temp, shown_tint) = photo.temp_tint_for(adjust.white_balance);
            let (mut temp, mut tint) = (shown_temp, shown_tint);
            let t = ui
                .add(
                    egui::Slider::new(&mut temp, 2000.0..=12000.0)
                        .logarithmic(true)
                        .fixed_decimals(0)
                        .suffix(" K")
                        .text("Temp"),
                )
                .on_hover_text(help::TEMP);
            let n = ui
                .add(
                    egui::Slider::new(&mut tint, -100.0..=100.0)
                        .fixed_decimals(0)
                        .text("Tint"),
                )
                .on_hover_text(help::TINT);
            // Only a real move counts: the logarithmic slider can nudge the value by a
            // fraction just by being drawn, which must not turn "As shot" into Custom.
            let moved = (temp - shown_temp).abs() > 0.5 || (tint - shown_tint).abs() > 0.5;
            if (t.changed() || n.changed()) && moved {
                adjust.white_balance = WhiteBalance::Custom {
                    temp: temp.round(),
                    tint: tint.round(),
                };
            }
        }
    } else {
        let why = if raw.is_some() {
            "Monochrome sensor: there is no colour to balance."
        } else {
            help::RENDERED
        };
        ui.label(egui::RichText::new(why).small().weak());
    }

    // Highlights (colour raw, detailed view)
    if colour && detailed {
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            ui.label(egui::RichText::new("Highlights").strong())
                .on_hover_text(help::HIGHLIGHTS);
            if chip(
                ui,
                adjust.highlights == Highlights::Reconstruct,
                "Reconstruct",
                help::RECONSTRUCT,
            ) {
                adjust.highlights = Highlights::Reconstruct;
            }
            if chip(
                ui,
                adjust.highlights == Highlights::Clip,
                "Clip",
                help::CLIP,
            ) {
                adjust.highlights = Highlights::Clip;
            }
        });
    }

    // Rotation and reset
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        if ui
            .button("Rotate left")
            .on_hover_text(help::ROTATE_LEFT)
            .clicked()
        {
            adjust.rotation = (adjust.rotation + 3) % 4;
        }
        if ui
            .button("Rotate right")
            .on_hover_text(help::ROTATE_RIGHT)
            .clicked()
        {
            adjust.rotation = (adjust.rotation + 1) % 4;
        }
        if ui
            .add_enabled(*adjust != RawAdjust::default(), egui::Button::new("Reset"))
            .on_hover_text(help::RESET)
            .clicked()
        {
            *adjust = RawAdjust::default();
        }
    });

    *adjust != before
}

/// Exposure that puts the scene's log-average luminance at 18 %.
fn auto_exposure(source: &Source, adjust: &RawAdjust) -> f64 {
    match source {
        Source::Raw(photo) => photo.auto_exposure_ev(adjust),
        Source::Raster(_) => {
            let probe = RawAdjust {
                exposure_ev: 0.0,
                ..*adjust
            };
            let scene = source.downscaled(512).develop(&probe);
            let lum: Vec<f32> = match &scene {
                bw_negative::SceneImage::Mono { y } => y.data().to_vec(),
                bw_negative::SceneImage::Rgb { r, g, b } => r
                    .data()
                    .iter()
                    .zip(g.data())
                    .zip(b.data())
                    .map(|((r, g), b)| 0.2126 * r + 0.7152 * g + 0.0722 * b)
                    .collect(),
            };
            let logs: Vec<f64> = lum
                .iter()
                .filter(|v| **v > 1e-5)
                .map(|v| f64::from(*v).ln())
                .collect();
            if logs.is_empty() {
                return 0.0;
            }
            let mean = (logs.iter().sum::<f64>() / logs.len() as f64).exp();
            ((0.18 / mean).log2() * 3.0).round() / 3.0
        }
    }
}
