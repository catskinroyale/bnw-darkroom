//! The Workflow panel: a simple view (film, developer, push/pull, grain) and a
//! detailed view (every processing and printing control). Every control carries a
//! one- or two-line hover explanation of what it does and how it changes the image.

use bw_negative::times::TimeOrigin;
use bw_profiles::{AgitationSpec, ModelSummary, Style};
use eframe::egui::{self, Color32, Pos2, Sense, Stroke, Vec2};

mod help {
    pub const FILM: &str = "The emulsion. It sets how colours turn into greys, the shape of the tone curve, sharpness and grain.";
    pub const DEVELOPER: &str = "The chemistry. It changes speed, contrast, grain and edge sharpness; the dilution changes how strongly.";
    pub const DILUTION: &str = "How much the developer is diluted. More dilute: longer times, more edge sharpness and highlight compensation.";
    pub const PUSH: &str = "Rate the film faster (push) or slower (pull) and develop to match.\nPushing adds contrast and grain, not real shadow detail; pulling softens contrast.";
    pub const GRAIN: &str = "Grain comes from the film and developer. The more the negative is enlarged, the more it shows.";
    pub const FORMAT: &str = "Film size. A larger negative is enlarged less for the same print, so its grain looks finer.";
    pub const CROP: &str = "Use only the centre of the frame. Cropping means enlarging the negative more, so grain grows.";
    pub const GRAIN_SEED: &str = "Picks the random grain pattern. The same number gives identical grain; the amount does not change.";
    pub const NEW_PATTERN: &str = "Same grain amount and character, a different random pattern.";
    pub const GRAIN_READOUT: &str = "Diffuse RMS granularity at this enlargement (Kodak's measure).\nAround 10 looks smooth; above 25 looks gritty.";
    pub const NAME: &str = "Name of this style, used in the style list and in exported file names.";
    pub const EI: &str = "Exposure index the film is rated at. Higher means less exposure: a thinner negative unless developed longer.";
    pub const EV: &str =
        "Extra exposure in stops on top of the EI, like the camera's exposure compensation.";
    pub const TIME: &str = "Development time. 'Published' follows the time table for this film, developer and EI;\nuntick it to set your own. Longer development raises contrast and grain.";
    pub const TEMP: &str = "Developer temperature. Warmer developer works faster: more density and contrast for the same time.";
    pub const AGITATION: &str = "How often the tank is agitated. Less agitation gives stronger edge effects and compresses highlights.";
    pub const GRADE: &str =
        "Contrast of the printing paper: 0 is very soft, 2 is normal, 5 is very hard.";
    pub const PRINT_EXPOSURE: &str = "Enlarger exposure relative to automatic. More exposure makes the print darker; less makes it lighter.";
    pub const CURVE: &str = "The negative's density against log exposure (lux·s). Steeper means more contrast.\nOrange line: the metered mid-grey.";
}

/// Display order and short names for films.
const FILMS: [(&str, &str, &str); 9] = [
    (
        "tri-x",
        "Tri-X",
        "Classic fast cubic-grain film: gritty grain, long toe, punchy mid-tones.",
    ),
    (
        "tmax-100",
        "T-Max 100",
        "Kodak tabular-grain ISO 100 film: extremely fine grain, very sharp, long straight tone curve.",
    ),
    (
        "tmax-400",
        "T-Max 400",
        "Kodak tabular-grain ISO 400 film: fine grain for its speed, crisp and clean, pushes well.",
    ),
    (
        "hp5",
        "HP5",
        "Fast traditional-grain film: softer contrast, wide latitude, pushes well.",
    ),
    (
        "fp4",
        "FP4",
        "Medium-speed traditional film: fine grain, crisp detail, slightly brighter tones.",
    ),
    (
        "delta-100",
        "Delta 100",
        "Tabular-grain ISO 100 film: very fine grain, high sharpness, clean smooth tones.",
    ),
    (
        "delta-400",
        "Delta 400",
        "Tabular-grain ISO 400 film: finer grain than Tri-X or HP5 at the same speed.",
    ),
    (
        "delta-3200",
        "Delta 3200",
        "Very fast film (ISO 1000, rated at 3200): large, prominent grain and low-light character.",
    ),
    (
        "double-x",
        "Double-X",
        "Motion-picture stock: classic cinema look, moderate grain and a soft toe.",
    ),
];

const FORMATS: [(&str, f64, &str); 4] = [
    (
        "35 mm",
        36.0,
        "36 × 24 mm frame: the most grain for a given print size.",
    ),
    (
        "645",
        56.0,
        "56 × 42 mm medium-format frame: about 1.6× less enlargement than 35 mm.",
    ),
    (
        "6×7",
        70.0,
        "70 × 56 mm medium-format frame: about 2× less enlargement than 35 mm.",
    ),
    (
        "4×5",
        120.0,
        "4 × 5 inch sheet film: almost grainless at normal print sizes.",
    ),
];

const CROPS: [(&str, f64); 4] = [("Full frame", 1.0), ("1.5×", 1.5), ("2×", 2.0), ("3×", 3.0)];

const PUSHES: [(&str, f64); 6] = [
    ("Pull 2", -2.0),
    ("Pull 1", -1.0),
    ("Box", 0.0),
    ("Push 1", 1.0),
    ("Push 2", 2.0),
    ("Push 3", 3.0),
];

fn developer_help(family: &str) -> &'static str {
    match family {
        "d76" => "Fine-grain all-rounder: full film speed, smooth tones, slightly softened grain.",
        "fx55" => {
            "Crawley's Phenidone–vitamin C developer: fine grain with good sharpness and about half a stop of extra speed."
        }
        "d96" => {
            "Kodak's motion-picture negative developer: soft-working, fine grain, gentle contrast."
        }
        "hc110" => {
            "Liquid concentrate: clean, sharp results, moderate grain; the dilution sets time and compensation."
        }
        "rodinal" => {
            "High-acutance developer: crisp edges and pronounced grain; more dilution compensates more."
        }
        _ => "",
    }
}

fn dilution_help(family: &str, dilution: &str) -> &'static str {
    match (family, dilution) {
        ("d76", "stock") => "Undiluted: finest grain and the least edge effect.",
        ("d76", "1+1") => "Diluted 1+1: a little sharper and more compensating, with longer times.",
        ("fx55", "1+9") => {
            "The published working dilution: part A diluted 1+9, part B added just before use."
        }
        ("d96", "stock") => "Used as mixed: the motion-picture working strength.",
        ("hc110", "b") => "Kodak's standard dilution: normal contrast with short times.",
        ("hc110", "e") => "More dilute than B: longer, easier-to-control times.",
        ("hc110", "h") => "Half the strength of B: longer times and slightly more compensation.",
        ("hc110", "g") => "Very dilute: long times and strong compensation for contrasty scenes.",
        ("rodinal", "1+25") => {
            "Strongest dilution: shortest times, sharpest and most prominent grain."
        }
        ("rodinal", "1+50") => "Standard dilution: balanced sharpness and highlight compensation.",
        ("rodinal", "1+100") => {
            "Very dilute, often used for stand development: strong edges and compressed highlights."
        }
        _ => "",
    }
}

/// A toggle chip like the top bar's panel buttons. Returns true when clicked.
fn chip(
    ui: &mut egui::Ui,
    selected: bool,
    enabled: bool,
    label: &str,
    help: &str,
    disabled_help: &str,
) -> bool {
    let response = ui.add_enabled(enabled, egui::Button::selectable(selected, label));
    let response = if enabled {
        response.on_hover_text(help)
    } else {
        response.on_disabled_hover_text(disabled_help)
    };
    response.clicked()
}

fn section(ui: &mut egui::Ui, title: &str, help: &str) {
    ui.add_space(6.0);
    ui.label(egui::RichText::new(title).strong())
        .on_hover_text(help);
}

/// Switch film, keeping the push/pull and a developer that has a published time.
fn change_film(s: &mut Style, film: &str) {
    let stops = s.push_stops();
    let manual_time = s.development.time_min;
    s.film = film.to_string();
    if !s.is_available()
        && let Some(dev) = bw_profiles::developer_ids()
            .iter()
            .find(|d| bw_profiles::is_available(film, d))
    {
        s.developer = dev.to_string();
    }
    s.set_push_stops(stops);
    s.development.time_min = manual_time;
}

fn change_developer(s: &mut Style, developer: String) {
    s.developer = developer;
    if s.development.time_min.is_some() {
        s.development.time_min = None;
    }
}

fn origin_label(origin: Option<TimeOrigin>) -> &'static str {
    match origin {
        Some(TimeOrigin::Published) => "published time",
        Some(TimeOrigin::Interpolated) => "interpolated between published times",
        Some(TimeOrigin::Estimated) => "estimated (push/pull rule of thumb)",
        None => "your time",
    }
}

fn film_chips(ui: &mut egui::Ui, s: &mut Style) {
    section(ui, "Film", help::FILM);
    ui.horizontal_wrapped(|ui| {
        for (id, label, help) in FILMS {
            if chip(ui, s.film == id, true, label, help, "") && s.film != id {
                change_film(s, id);
            }
        }
    });
}

fn developer_chips(ui: &mut egui::Ui, s: &mut Style) {
    section(ui, "Developer", help::DEVELOPER);
    let (family, dilution) = {
        let (f, d) = bw_profiles::split_developer(&s.developer);
        (f.to_string(), d.to_string())
    };
    ui.horizontal_wrapped(|ui| {
        for (fam, dilutions) in bw_profiles::developer_families() {
            let first = dilutions
                .iter()
                .map(|d| format!("{fam}-{d}"))
                .find(|id| bw_profiles::is_available(&s.film, id));
            let label = bw_profiles::developer_family_label(fam);
            let no_data = format!("No published {label} times for this film.");
            if chip(
                ui,
                family == *fam,
                first.is_some(),
                label,
                developer_help(fam),
                &no_data,
            ) && family != *fam
                && let Some(id) = first
            {
                change_developer(s, id);
            }
        }
    });
    if let Some((fam, dilutions)) = bw_profiles::developer_families()
        .iter()
        .find(|(f, _)| *f == family)
    {
        ui.horizontal_wrapped(|ui| {
            ui.label(egui::RichText::new("Dilution").weak())
                .on_hover_text(help::DILUTION);
            for d in dilutions.iter() {
                let id = format!("{fam}-{d}");
                let available = bw_profiles::is_available(&s.film, &id);
                let label = bw_profiles::dilution_label(fam, d);
                if chip(
                    ui,
                    dilution == *d,
                    available,
                    &label,
                    dilution_help(fam, d),
                    "No published time for this film at this dilution.",
                ) && dilution != *d
                {
                    change_developer(s, id);
                }
            }
        });
    }
}

fn push_chips(ui: &mut egui::Ui, s: &mut Style) {
    section(ui, "Push / pull", help::PUSH);
    let current = s.push_stops();
    ui.horizontal_wrapped(|ui| {
        for (label, stops) in PUSHES {
            let mut probe = s.clone();
            probe.set_push_stops(stops);
            let time = probe.recommended_time();
            let help = match time {
                Some(t) => format!(
                    "Rate at EI {:.0} and develop {:.1} min ({}).",
                    probe.exposure_index,
                    t.time_min,
                    origin_label(Some(t.origin))
                ),
                None => format!("Rate at EI {:.0}.", probe.exposure_index),
            };
            let selected = (current - stops).abs() < 0.17 && s.development.time_min.is_none();
            if chip(ui, selected, true, label, &help, "") {
                s.set_push_stops(stops);
            }
        }
    });
}

fn development_readout(ui: &mut egui::Ui, s: &Style, summary: Option<&ModelSummary>) {
    let Some(m) = summary else { return };
    let text = format!(
        "Develop {:.1} min at {:.0} °C · EI {:.0} · {}",
        m.time_min,
        s.development.temperature_c,
        s.exposure_index,
        origin_label(m.time_origin)
    );
    ui.label(egui::RichText::new(text).small().weak())
        .on_hover_text(help::TIME);
}

fn grain_section(ui: &mut egui::Ui, s: &mut Style, summary: Option<&ModelSummary>) {
    section(ui, "Grain", help::GRAIN);
    ui.horizontal_wrapped(|ui| {
        ui.label(egui::RichText::new("Format").weak())
            .on_hover_text(help::FORMAT);
        for (label, mm, help) in FORMATS {
            if chip(ui, (s.format_mm - mm).abs() < 0.5, true, label, help, "") {
                s.format_mm = mm;
            }
        }
    });
    ui.horizontal_wrapped(|ui| {
        ui.label(egui::RichText::new("Enlarge").weak())
            .on_hover_text(help::CROP);
        for (label, crop) in CROPS {
            let help = if crop == 1.0 {
                "Use the whole frame.".to_string()
            } else {
                format!("Use the central 1/{crop} of the frame's width, enlarged {crop}× more.")
            };
            if chip(ui, (s.crop - crop).abs() < 0.01, true, label, &help, "") {
                s.crop = crop;
            }
        }
    });
    ui.horizontal(|ui| {
        if let Some(m) = summary {
            ui.label(format!(
                "Grain at print: {} ({:.0})",
                m.grain.describe(),
                m.grain.at_print
            ))
            .on_hover_text(help::GRAIN_READOUT);
        }
        if ui
            .small_button("New pattern")
            .on_hover_text(help::NEW_PATTERN)
            .clicked()
        {
            s.seed = s.seed.wrapping_add(1);
        }
    });
    ui.label(
        egui::RichText::new("Press D and click the image to judge grain at 1:1.")
            .small()
            .weak(),
    );
}

/// Simple view: choices as toggle chips, no printing controls.
pub fn simple(ui: &mut egui::Ui, s: &mut Style, summary: Option<&ModelSummary>) {
    film_chips(ui, s);
    developer_chips(ui, s);
    push_chips(ui, s);
    development_readout(ui, s, summary);
    grain_section(ui, s, summary);
}

/// Detailed view: every control.
pub fn detailed(ui: &mut egui::Ui, s: &mut Style, summary: Option<&ModelSummary>) {
    // Size controls to the panel so it can be resized narrower than their defaults.
    let control_w = (ui.available_width() - 128.0).max(96.0);
    ui.spacing_mut().slider_width = (control_w - 64.0).max(40.0);
    ui.spacing_mut().combo_width = control_w;

    egui::Grid::new("detailed").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
        ui.label("Name").on_hover_text(help::NAME);
        ui.add(egui::TextEdit::singleline(&mut s.name).desired_width(control_w)).on_hover_text(help::NAME);
        ui.end_row();

        ui.label("Film").on_hover_text(help::FILM);
        let film_label = FILMS.iter().find(|(id, ..)| *id == s.film).map_or(s.film.clone(), |(_, l, _)| l.to_string());
        let mut film = s.film.clone();
        egui::ComboBox::from_id_salt("film").selected_text(film_label).show_ui(ui, |ui| {
            for (id, label, help) in FILMS {
                ui.selectable_value(&mut film, id.to_string(), label).on_hover_text(help);
            }
        });
        if film != s.film {
            change_film(s, &film);
        }
        ui.end_row();

        ui.label("Developer").on_hover_text(help::DEVELOPER);
        let mut developer = s.developer.clone();
        egui::ComboBox::from_id_salt("developer")
            .selected_text(bw_profiles::developer_label(&s.developer))
            .show_ui(ui, |ui| {
                for id in bw_profiles::developer_ids() {
                    let (fam, dil) = bw_profiles::split_developer(id);
                    let available = bw_profiles::is_available(&s.film, id);
                    ui.add_enabled_ui(available, |ui| {
                        ui.selectable_value(&mut developer, id.to_string(), bw_profiles::developer_label(id))
                            .on_hover_text(dilution_help(fam, dil))
                            .on_disabled_hover_text("No published time for this film.");
                    });
                }
            });
        if developer != s.developer {
            change_developer(s, developer);
        }
        ui.end_row();

        ui.label("Exposure index").on_hover_text(help::EI);
        ui.horizontal(|ui| {
            ui.add(egui::DragValue::new(&mut s.exposure_index).range(6.0..=12800.0).speed(5.0).suffix(" EI"))
                .on_hover_text(help::EI);
            let stops = s.push_stops();
            let label = match stops {
                x if x > 0.05 => format!("push {x:+.1}"),
                x if x < -0.05 => format!("pull {:.1}", -x),
                _ => "box speed".into(),
            };
            ui.label(egui::RichText::new(label).weak()).on_hover_text(help::PUSH);
        });
        ui.end_row();

        ui.label("Exposure comp.").on_hover_text(help::EV);
        ui.add(egui::Slider::new(&mut s.ev_compensation, -3.0..=3.0).step_by(1.0 / 3.0).suffix(" EV"))
            .on_hover_text(help::EV);
        ui.end_row();

        ui.label("Time").on_hover_text(help::TIME);
        ui.horizontal(|ui| {
            let mut published = s.development.time_min.is_none();
            if ui.checkbox(&mut published, "published").on_hover_text(help::TIME).changed() {
                s.development.time_min = if published {
                    None
                } else {
                    Some(s.processing().time_min)
                };
            }
            if let Some(t) = &mut s.development.time_min {
                ui.add(egui::DragValue::new(t).range(1.0..=120.0).speed(0.1).suffix(" min")).on_hover_text(help::TIME);
            } else if let Some(t) = s.recommended_time() {
                ui.label(egui::RichText::new(format!("{:.2} min", t.time_min)).weak())
                    .on_hover_text(origin_label(Some(t.origin)));
            }
        });
        ui.end_row();

        ui.label("Temperature").on_hover_text(help::TEMP);
        ui.add(egui::Slider::new(&mut s.development.temperature_c, 16.0..=28.0).step_by(0.5).suffix(" °C"))
            .on_hover_text(help::TEMP);
        ui.end_row();

        ui.label("Agitation").on_hover_text(help::AGITATION);
        egui::ComboBox::from_id_salt("agitation")
            .selected_text(s.development.agitation.label())
            .show_ui(ui, |ui| {
                let a = &mut s.development.agitation;
                ui.selectable_value(a, AgitationSpec::Continuous, "Continuous")
                    .on_hover_text("Rotary or constant inversion: even development, least edge effect.");
                ui.selectable_value(a, AgitationSpec::Intermittent { interval_s: 30.0 }, "Every 30 s")
                    .on_hover_text("The reference scheme for the published times.");
                ui.selectable_value(a, AgitationSpec::Intermittent { interval_s: 60.0 }, "Every 60 s")
                    .on_hover_text("Gentler: slightly more edge effect and compensation.");
                ui.selectable_value(a, AgitationSpec::SemiStand { mid_agitations: 1 }, "Semi-stand")
                    .on_hover_text("One agitation halfway: strong compensation and edge effects.");
                ui.selectable_value(a, AgitationSpec::Stand, "Stand")
                    .on_hover_text("No agitation after the start: strongest compensation and edges; set a long time.");
            });
        ui.end_row();

        ui.label("Format").on_hover_text(help::FORMAT);
        let format_label = FORMATS.iter().find(|(_, mm, _)| (s.format_mm - mm).abs() < 0.5).map_or("custom", |f| f.0);
        egui::ComboBox::from_id_salt("format").selected_text(format_label).show_ui(ui, |ui| {
            for (label, mm, help) in FORMATS {
                ui.selectable_value(&mut s.format_mm, mm, label).on_hover_text(help);
            }
        });
        ui.end_row();

        ui.label("Enlarge (crop)").on_hover_text(help::CROP);
        ui.add(egui::Slider::new(&mut s.crop, 1.0..=4.0).step_by(0.1).suffix("×")).on_hover_text(help::CROP);
        ui.end_row();

        ui.label("Grain seed").on_hover_text(help::GRAIN_SEED);
        ui.add(egui::DragValue::new(&mut s.seed)).on_hover_text(help::GRAIN_SEED);
        ui.end_row();

        ui.label("Paper grade").on_hover_text(help::GRADE);
        ui.add(egui::Slider::new(&mut s.print.grade, 0.0..=5.0).step_by(0.5)).on_hover_text(help::GRADE);
        ui.end_row();

        ui.label("Print exposure").on_hover_text(help::PRINT_EXPOSURE);
        ui.add(egui::Slider::new(&mut s.print.exposure_stops, -2.0..=2.0).step_by(1.0 / 6.0).suffix(" stops"))
            .on_hover_text(help::PRINT_EXPOSURE);
        ui.end_row();
    });

    if let Some(m) = summary {
        ui.add_space(4.0);
        ui.label(format!(
            "Grain at print: {} ({:.0})",
            m.grain.describe(),
            m.grain.at_print
        ))
        .on_hover_text(help::GRAIN_READOUT);
    }
    ui.add_space(6.0);
    model_panel(ui, summary);
}

fn model_panel(ui: &mut egui::Ui, summary: Option<&ModelSummary>) {
    let Some(summary) = summary else {
        ui.colored_label(
            ui.visuals().warn_fg_color,
            "No published development time for this film and developer.",
        );
        return;
    };
    ui.label(egui::RichText::new("Negative · characteristic curve").strong())
        .on_hover_text(help::CURVE);
    curve_plot(ui, summary);
    egui::Grid::new("model").num_columns(2).spacing([12.0, 2.0]).show(ui, |ui| {
        ui.label("Development").on_hover_text(help::TIME);
        ui.label(format!("{:.2} min · {}", summary.time_min, origin_label(summary.time_origin)));
        ui.end_row();
        ui.label("Model ISO speed")
            .on_hover_text("The film speed the model gives under these conditions (ISO 6 method).");
        ui.label(summary.iso.map_or("—".into(), |i| format!("{:.0}", i.speed)));
        ui.end_row();
        ui.label("Calibration")
            .on_hover_text("How the film and developer parameters were obtained. Illustrative means not yet fitted to data.");
        ui.label(summary.calibration_level.label());
        ui.end_row();
    });
    for w in &summary.warnings {
        ui.colored_label(ui.visuals().warn_fg_color, format!("Warning: {w}"));
    }
}

/// Density vs log exposure, with the metered mid-grey marked.
fn curve_plot(ui: &mut egui::Ui, summary: &ModelSummary) {
    let width = ui.available_width().min(320.0);
    let (response, painter) = ui.allocate_painter(Vec2::new(width, 120.0), Sense::hover());
    let rect = response.rect.shrink(4.0);
    let (x0, x1, y1) = (-4.5, 0.5, 3.0);
    let to_screen = |lh: f64, d: f64| {
        Pos2::new(
            rect.min.x + ((lh - x0) / (x1 - x0)) as f32 * rect.width(),
            rect.max.y - (d.min(y1) / y1) as f32 * rect.height(),
        )
    };
    let grid = Stroke::new(0.5, ui.visuals().weak_text_color().gamma_multiply(0.5));
    for d in [0.0, 1.0, 2.0, 3.0] {
        painter.line_segment([to_screen(x0, d), to_screen(x1, d)], grid);
    }
    for lh in [-4.0, -3.0, -2.0, -1.0, 0.0] {
        painter.line_segment([to_screen(lh, 0.0), to_screen(lh, y1)], grid);
    }
    let grey = Stroke::new(1.0, Color32::from_rgb(220, 120, 70));
    painter.line_segment(
        [
            to_screen(summary.grey_log_h, 0.0),
            to_screen(summary.grey_log_h, y1),
        ],
        grey,
    );
    let points: Vec<Pos2> = summary
        .curve
        .iter()
        .map(|&(lh, d)| to_screen(lh, d))
        .collect();
    painter.add(egui::Shape::line(
        points,
        Stroke::new(1.8, ui.visuals().strong_text_color()),
    ));
    response.on_hover_text(help::CURVE);
}
