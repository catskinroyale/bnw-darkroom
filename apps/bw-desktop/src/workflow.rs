//! The Develop panel's sections: Film, Development, Grain and Print, with advanced
//! controls in expandable rows. Every control carries a one- or two-line hover
//! explanation of what it does and how it changes the image.

use bw_negative::times::TimeOrigin;
use bw_profiles::{AgitationSpec, GrainSpec, ModelSummary, Style};
use eframe::egui::{self, Color32, Pos2, Sense, Stroke, Vec2};

use crate::widgets::{self, Icon, Segment, SliderSpec, inset};

mod help {
    pub const FILM: &str = "The emulsion. It sets how colours turn into greys, the shape of the tone curve, sharpness and grain.";
    pub const DEVELOPER: &str = "The chemistry. It changes speed, contrast, grain and edge sharpness; the dilution changes how strongly.";
    pub const DILUTION: &str = "How much the developer is diluted. More dilute: longer times, more edge sharpness and highlight compensation.";
    pub const PUSH: &str = "Develop longer (push) or shorter (pull), using the published time for the pushed or pulled EI.\nYour photo's exposure is kept and the print's black point holds: a push keeps blacks, opens shadows slightly,\nbrightens highlights and adds grain; a pull softens contrast. For an underexposed push, lower Exposure comp.";
    pub const FORMAT: &str = "Film size. A larger negative is enlarged less for the same print, so its grain looks finer.";
    pub const CROP: &str = "Use only the centre of the frame. Cropping means enlarging the negative more, so grain grows.";
    pub const NEW_PATTERN: &str = "Same grain amount and character, a different random pattern.";
    pub const GRAIN_SIZE: &str = "Coarseness of the grain clumps, relative to the film's own. Strength stays the same.\n×1 is the film's grain.";
    pub const GRAIN_AMOUNT: &str =
        "Strength of the grain, relative to the film's own. ×1 is the film's grain; 0 removes it.";
    pub const GRAIN_ROUGHNESS: &str = "Character of the grain: below 0 softer and smoother; above 0 grittier, with more\nisolated dark and bright specks. 0 is the film's grain.";
    pub const FILM_GRAIN: &str = "Back to the film's own grain: size ×1, amount ×1, roughness 0.";
    pub const RANDOMISE: &str =
        "A new random pattern with random size, amount and roughness, within a usable range.";
    pub const GRAIN_READOUT: &str = "Diffuse RMS granularity at this enlargement (Kodak's measure).\nAround 10 looks smooth; above 25 looks gritty.";
    pub const EI: &str = "Exposure index the film is developed for: picks the published time. The film is still exposed\nat box speed, because your photo is already exposed; use Exposure comp. to under- or over-expose it.";
    pub const EV: &str = "Under- or over-exposes the film in stops, like the camera's exposure compensation.\nWith a push, a negative value gives the classic underexposed push: empty shadows.";
    pub const TIME: &str = "Development time. 'Published' follows the time table for this film, developer and EI;\nuntick it to set your own. Longer development raises contrast and grain.";
    pub const TEMP: &str = "Developer temperature. Warmer developer works faster: more density and contrast for the same time.";
    pub const AGITATION: &str = "How often the tank is agitated. Less agitation gives stronger edge effects and compresses highlights.";
    pub const GRADE: &str =
        "Contrast of the printing paper: 0 is very soft, 2 is normal, 5 is very hard.";
    pub const PRINT_EXPOSURE: &str = "Enlarger exposure relative to automatic. More exposure makes the print darker; less makes it lighter.";
    pub const CURVE: &str = "The negative's density against log exposure (lux·s). Steeper means more contrast.\nOrange line: the metered mid-grey.";
}

/// Display order and short names for films.
const FILMS: [(&str, &str, &str); 12] = [
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
        "kentmere-100",
        "Kentmere 100",
        "Harman's budget ISO 100 traditional-grain film: fine grain, classic tones.",
    ),
    (
        "kentmere-200",
        "Kentmere 200",
        "Harman's ISO 200 traditional-grain film: a middle ground of speed and grain.",
    ),
    (
        "kentmere-400",
        "Kentmere 400",
        "Harman's budget ISO 400 film: visible grain and a gentler curve; pushes reasonably.",
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

const FORMATS: [(&str, f64, &str); 5] = [
    (
        "35 mm",
        36.0,
        "36 × 24 mm frame: the most grain for a given print size.",
    ),
    (
        "6×6 / 645",
        56.0,
        "56 mm wide medium-format frame (6×6 or 645): about 1.6× less enlargement than 35 mm.",
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
    (
        "8×10",
        203.0,
        "8 × 10 inch sheet film, often contact printed: no visible grain.",
    ),
];

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

/// Random grain within a usable range, and a new pattern.
fn randomise_grain(s: &mut Style) {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos() as u64);
    let mut state = nanos ^ s.seed.rotate_left(17);
    // SplitMix64: plenty for picking slider values.
    let mut next = || {
        state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    };
    let mut unit = || (next() >> 11) as f64 / (1u64 << 53) as f64;
    let round2 = |v: f64| (v * 100.0).round() / 100.0;
    s.grain = GrainSpec {
        size: round2(0.7 * (2.2f64 / 0.7).powf(unit())),
        amount: round2(0.6 + 1.4 * unit()),
        roughness: round2(-0.6 + 1.4 * unit()),
    };
    s.seed = next() % 100_000;
}

/// Section and expandable-row keys, in panel order (for "Edit…", which opens them all).
pub const SECTION_KEYS: [&str; 7] = [
    "film",
    "development",
    "development.details",
    "development.curve",
    "grain",
    "print",
    "image",
];

fn spec<'a>(
    id: &'a str,
    label: &'a str,
    help: &'a str,
    (min, max, default, step): (f64, f64, f64, f64),
    decimals: usize,
    suffix: &'a str,
) -> SliderSpec<'a> {
    SliderSpec {
        id,
        label,
        help,
        min,
        max,
        default,
        step,
        decimals,
        suffix,
        signed: false,
        log: false,
    }
}

fn film_label(id: &str) -> String {
    FILMS.iter().find(|(f, ..)| *f == id).map_or_else(
        || bw_profiles::film_label(id),
        |(_, label, _)| label.to_string(),
    )
}

/// Film: stock, format and enlargement.
pub fn film_section(ui: &mut egui::Ui, s: &mut Style) {
    let current = film_label(&s.film);
    let mut chosen = None;
    widgets::dropdown(ui, "film", "Film", &current, help::FILM, |ui| {
        for (id, label, help) in FILMS {
            if ui
                .add(egui::Button::selectable(s.film == id, label))
                .on_hover_text(help)
                .clicked()
            {
                chosen = Some(id);
            }
        }
    });
    if let Some(id) = chosen
        && s.film != id
    {
        change_film(s, id);
    }
    if let Some((_, _, about)) = FILMS.iter().find(|(id, ..)| *id == s.film) {
        widgets::note(ui, *about, "");
    }
    widgets::subtitle(ui, "Format", help::FORMAT);
    let formats: Vec<Segment> = FORMATS
        .iter()
        .map(|(label, mm, help)| Segment {
            label,
            selected: (s.format_mm - mm).abs() < 0.5,
            enabled: true,
            help,
        })
        .collect();
    if let Some(i) = widgets::segmented(ui, &formats, 3) {
        s.format_mm = FORMATS[i].1;
    }
    widgets::slider(
        ui,
        &spec(
            "film.enlarge",
            "Enlarge",
            help::CROP,
            (1.0, 4.0, 1.0, 0.1),
            1,
            "×",
        ),
        &mut s.crop,
        true,
    );
}

/// Development: developer, dilution, push/pull; details and the curve in expandable rows.
pub fn development_section(ui: &mut egui::Ui, s: &mut Style, summary: Option<&ModelSummary>) {
    let (family, dilution) = {
        let (f, d) = bw_profiles::split_developer(&s.developer);
        (f.to_string(), d.to_string())
    };
    let mut chosen = None;
    widgets::dropdown(
        ui,
        "developer",
        "Developer",
        bw_profiles::developer_family_label(&family),
        help::DEVELOPER,
        |ui| {
            for (fam, dilutions) in bw_profiles::developer_families() {
                let first = dilutions
                    .iter()
                    .map(|d| format!("{fam}-{d}"))
                    .find(|id| bw_profiles::is_available(&s.film, id));
                let label = bw_profiles::developer_family_label(fam);
                let r = ui
                    .add_enabled(
                        first.is_some(),
                        egui::Button::selectable(family == *fam, label),
                    )
                    .on_hover_text(developer_help(fam))
                    .on_disabled_hover_text(format!("No published {label} times for this film."));
                if r.clicked() {
                    chosen = first;
                }
            }
        },
    );
    if let Some(id) = chosen
        && id != s.developer
    {
        change_developer(s, id);
    }
    if let Some((fam, dilutions)) = bw_profiles::developer_families()
        .iter()
        .find(|(f, _)| *f == family)
        && dilutions.len() > 1
    {
        widgets::subtitle(ui, "Dilution", help::DILUTION);
        let labels: Vec<String> = dilutions
            .iter()
            .map(|d| bw_profiles::dilution_label(fam, d))
            .collect();
        let segments: Vec<Segment> = dilutions
            .iter()
            .zip(&labels)
            .map(|(d, label)| Segment {
                label,
                selected: dilution == *d,
                enabled: bw_profiles::is_available(&s.film, &format!("{fam}-{d}")),
                help: dilution_help(fam, d),
            })
            .collect();
        if let Some(i) = widgets::segmented(ui, &segments, 4) {
            change_developer(s, format!("{fam}-{}", dilutions[i]));
        }
    }

    widgets::subtitle(ui, "Push / pull", help::PUSH);
    let current = s.push_stops();
    let helps: Vec<String> = PUSHES
        .iter()
        .map(|(_, stops)| {
            let mut probe = s.clone();
            probe.set_push_stops(*stops);
            match probe.recommended_time() {
                Some(t) => format!(
                    "Develop {:.1} min, as for EI {:.0} ({}).",
                    t.time_min,
                    probe.exposure_index,
                    origin_label(Some(t.origin))
                ),
                None => format!("Develop as for EI {:.0}.", probe.exposure_index),
            }
        })
        .collect();
    let segments: Vec<Segment> = PUSHES
        .iter()
        .zip(&helps)
        .map(|((label, stops), help)| Segment {
            label,
            selected: (current - stops).abs() < 0.17 && s.development.time_min.is_none(),
            enabled: true,
            help,
        })
        .collect();
    if let Some(i) = widgets::segmented(ui, &segments, 3) {
        s.set_push_stops(PUSHES[i].1);
    }
    if let Some(m) = summary {
        widgets::note(
            ui,
            format!(
                "Develop {:.1} min at {:.0} °C · {}",
                m.time_min,
                s.development.temperature_c,
                origin_label(m.time_origin)
            ),
            help::TIME,
        );
    } else {
        widgets::note(
            ui,
            "No published development time for this film and developer.",
            "",
        );
    }
    ui.add_space(4.0);

    widgets::flyout(
        ui,
        "development.details",
        "Development details",
        "EI, exposure compensation, time, temperature and agitation",
        Icon::Dial,
        |ui| development_details(ui, s),
    );
    widgets::flyout(
        ui,
        "development.curve",
        "Characteristic curve",
        help::CURVE,
        Icon::Curve,
        |ui| inset(ui, |ui| model_panel(ui, summary)),
    );
}

fn development_details(ui: &mut egui::Ui, s: &mut Style) {
    let box_ei = s.box_ei();
    let ei = SliderSpec {
        log: true,
        ..spec(
            "dev.ei",
            "Develop for EI",
            help::EI,
            (6.0, 12800.0, box_ei, 1.0),
            0,
            " EI",
        )
    };
    widgets::slider(ui, &ei, &mut s.exposure_index, true);
    let ev = SliderSpec {
        signed: true,
        ..spec(
            "dev.ev",
            "Exposure comp.",
            help::EV,
            (-3.0, 3.0, 0.0, 1.0 / 3.0),
            2,
            " EV",
        )
    };
    widgets::slider(ui, &ev, &mut s.ev_compensation, true);

    widgets::subtitle(ui, "Time", help::TIME);
    let published = s.development.time_min.is_none();
    let options = [
        Segment {
            label: "Published",
            selected: published,
            enabled: true,
            help: "Follow the time table for this film, developer and EI.",
        },
        Segment {
            label: "Your own",
            selected: !published,
            enabled: true,
            help: "Set the development time yourself.",
        },
    ];
    match widgets::segmented(ui, &options, 2) {
        Some(0) => s.development.time_min = None,
        Some(_) if published => s.development.time_min = Some(s.processing().time_min),
        _ => {}
    }
    let recommended = s.recommended_time().map(|t| t.time_min).unwrap_or(7.0);
    let time = SliderSpec {
        log: true,
        ..spec(
            "dev.time",
            "Time",
            help::TIME,
            (1.0, 120.0, recommended, 0.25),
            2,
            " min",
        )
    };
    let mut t = s.processing().time_min;
    if widgets::slider(ui, &time, &mut t, !published) {
        s.development.time_min = Some(t);
    }
    widgets::slider(
        ui,
        &spec(
            "dev.temp",
            "Temperature",
            help::TEMP,
            (16.0, 28.0, 20.0, 0.5),
            1,
            " °C",
        ),
        &mut s.development.temperature_c,
        true,
    );

    widgets::subtitle(ui, "Agitation", help::AGITATION);
    let choices = [
        (
            AgitationSpec::Continuous,
            "Continuous",
            "Rotary or constant inversion: even development, least edge effect.",
        ),
        (
            AgitationSpec::Intermittent { interval_s: 30.0 },
            "Every 30 s",
            "The reference scheme for the published times.",
        ),
        (
            AgitationSpec::Intermittent { interval_s: 60.0 },
            "Every 60 s",
            "Gentler: slightly more edge effect and compensation.",
        ),
        (
            AgitationSpec::SemiStand { mid_agitations: 1 },
            "Semi-stand",
            "One agitation halfway: strong compensation and edge effects.",
        ),
        (
            AgitationSpec::Stand,
            "Stand",
            "No agitation after the start: strongest compensation and edges; set a long time.",
        ),
    ];
    let segments: Vec<Segment> = choices
        .iter()
        .map(|(a, label, help)| Segment {
            label,
            selected: s.development.agitation == *a,
            enabled: true,
            help,
        })
        .collect();
    if let Some(i) = widgets::segmented(ui, &segments, 3) {
        s.development.agitation = choices[i].0;
    }
}

/// Grain: size, amount and roughness relative to the film's own grain.
pub fn grain_section(ui: &mut egui::Ui, s: &mut Style, summary: Option<&ModelSummary>) {
    let size = SliderSpec {
        log: true,
        ..spec(
            "grain.size",
            "Size",
            help::GRAIN_SIZE,
            (0.5, 3.0, 1.0, 0.01),
            2,
            "×",
        )
    };
    widgets::slider(ui, &size, &mut s.grain.size, true);
    widgets::slider(
        ui,
        &spec(
            "grain.amount",
            "Amount",
            help::GRAIN_AMOUNT,
            (0.0, 3.0, 1.0, 0.01),
            2,
            "×",
        ),
        &mut s.grain.amount,
        true,
    );
    let mut rough = s.grain.roughness * 100.0;
    let roughness = SliderSpec {
        signed: true,
        ..spec(
            "grain.roughness",
            "Roughness",
            help::GRAIN_ROUGHNESS,
            (-100.0, 100.0, 0.0, 1.0),
            0,
            "",
        )
    };
    if widgets::slider(ui, &roughness, &mut rough, true) {
        s.grain.roughness = rough / 100.0;
    }
    inset(ui, |ui| {
        ui.horizontal(|ui| {
            if widgets::text_button(ui, "Film's grain", false, !s.grain.is_film_grain())
                .on_hover_text(help::FILM_GRAIN)
                .clicked()
            {
                s.grain = GrainSpec::default();
            }
            if widgets::text_button(ui, "Randomise", false, true)
                .on_hover_text(help::RANDOMISE)
                .clicked()
            {
                randomise_grain(s);
            }
            if widgets::text_button(ui, "New pattern", false, true)
                .on_hover_text(help::NEW_PATTERN)
                .clicked()
            {
                s.seed = s.seed.wrapping_add(1);
            }
        });
    });
    ui.add_space(6.0);
    if let Some(m) = summary {
        let adjusted = if s.grain.is_film_grain() {
            ""
        } else {
            " · adjusted from the film"
        };
        widgets::note(
            ui,
            format!(
                "Grain at print: {} ({:.0}){adjusted}",
                m.grain.describe(),
                m.grain.at_print
            ),
            help::GRAIN_READOUT,
        );
    }
    widgets::note(ui, "Press D and click the image to judge grain at 1:1.", "");
}

/// Print: paper grade and enlarger exposure.
pub fn print_section(ui: &mut egui::Ui, s: &mut Style) {
    widgets::slider(
        ui,
        &spec(
            "print.grade",
            "Paper grade",
            help::GRADE,
            (0.0, 5.0, 2.0, 0.5),
            1,
            "",
        ),
        &mut s.print.grade,
        true,
    );
    let exposure = SliderSpec {
        signed: true,
        ..spec(
            "print.exposure",
            "Print exposure",
            help::PRINT_EXPOSURE,
            (-2.0, 2.0, 0.0, 1.0 / 6.0),
            2,
            " stops",
        )
    };
    widgets::slider(ui, &exposure, &mut s.print.exposure_stops, true);
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
