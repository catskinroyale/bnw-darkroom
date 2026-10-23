//! Panel widgets: slider rows, accordion sections, expandable rows, segmented buttons,
//! dropdown rows and the tool-strip icons.
//!
//! The slider, section header, expandable ("flyout") row and text button are adapted from
//! LightCraft's `crates/ui-egui/src/widgets.rs` (https://github.com/storytold/lightcraft),
//! Copyright (c) 2026 ArtCraft Team and the LightCraft contributors, licensed under the MIT
//! License (see `assets/LICENSE-LightCraft-MIT`). Modified for BW Darkroom: simpler state,
//! no automation registry or translations, logarithmic sliders, and our own icons.

use eframe::egui::{
    self, Align2, Color32, Id, Painter, Pos2, Rect, Response, Sense, Stroke, StrokeKind, Ui, pos2,
    vec2,
};

use crate::theme::Tokens;

/// Content inset from the panel's left and right edges, as in LightCraft.
pub const PAD_L: f32 = 24.0;
pub const PAD_R: f32 = 22.0;
const SLIDER_ROW_H: f32 = 45.0;
const THUMB_RING: f32 = 7.0;

/// Run `content` inset to the panel's content column.
pub fn inset<R>(ui: &mut Ui, content: impl FnOnce(&mut Ui) -> R) -> R {
    egui::Frame::NONE
        .inner_margin(egui::Margin {
            left: PAD_L as i8,
            right: PAD_R as i8,
            top: 0,
            bottom: 0,
        })
        .show(ui, content)
        .inner
}

/// A slider row: what it controls and how values map onto its track.
pub struct SliderSpec<'a> {
    /// Unique id (also the persistent key for transient state).
    pub id: &'a str,
    pub label: &'a str,
    pub help: &'a str,
    pub min: f64,
    pub max: f64,
    /// What double-clicking resets to.
    pub default: f64,
    pub step: f64,
    pub decimals: usize,
    pub suffix: &'a str,
    /// Show "+" on positive values.
    pub signed: bool,
    /// Logarithmic track (e.g. kelvin, EI).
    pub log: bool,
}

impl SliderSpec<'_> {
    fn position(&self, v: f64) -> f64 {
        let v = v.clamp(self.min, self.max);
        if self.log && self.min > 0.0 {
            (v.ln() - self.min.ln()) / (self.max.ln() - self.min.ln())
        } else {
            (v - self.min) / (self.max - self.min)
        }
    }

    fn value_at(&self, p: f64) -> f64 {
        let p = p.clamp(0.0, 1.0);
        if self.log && self.min > 0.0 {
            (self.min.ln() + p * (self.max.ln() - self.min.ln())).exp()
        } else {
            self.min + p * (self.max - self.min)
        }
    }

    fn snap(&self, v: f64) -> f64 {
        let step = self.step.max(1e-9);
        ((v / step).round() * step).clamp(self.min, self.max)
    }

    /// The value as shown next to the label.
    pub fn format(&self, v: f64) -> String {
        let text = format!("{v:.*}", self.decimals);
        let zero = text
            .trim_start_matches('-')
            .chars()
            .all(|c| c == '0' || c == '.');
        if zero {
            format!("0{}", self.suffix)
        } else if self.signed && v > 0.0 {
            format!("+{text}{}", self.suffix)
        } else {
            format!("{text}{}", self.suffix)
        }
    }

    /// A typed value: "0.5", "+12", "-1,5", "5600 K"; on the steps and in range.
    fn parse(&self, text: &str) -> Option<f64> {
        let t = text.trim().replace(',', ".");
        let t = t.trim_end_matches(|c: char| {
            c.is_alphabetic() || c.is_whitespace() || c == '°' || c == '×' || c == '%'
        });
        let t = t.trim_start_matches('×');
        t.parse::<f64>()
            .ok()
            .filter(|v| v.is_finite())
            .map(|v| self.snap(v))
    }
}

/// A LightCraft-style slider row: label and value above a thin track with a hollow ring
/// thumb. Drag the track (Shift: a tenth as fast); double-click the label or thumb to reset;
/// click the value to type one (Return applies, Esc cancels). Returns true when `value`
/// changed.
pub fn slider(ui: &mut Ui, spec: &SliderSpec, value: &mut f64, enabled: bool) -> bool {
    let t = Tokens::get(ui.ctx());
    let w = ui.available_width();
    let (row, _) = ui.allocate_exact_size(vec2(w, SLIDER_ROW_H), Sense::hover());
    let label_rect = Rect::from_min_size(
        pos2(row.left() + PAD_L, row.top() + 4.0),
        vec2((w - PAD_L - PAD_R).max(40.0), 18.0),
    );
    let track_rect = Rect::from_min_max(
        pos2(row.left() + PAD_L, row.top() + 22.0),
        pos2(row.right() - PAD_R, row.top() + 40.0),
    );
    let id = Id::unique(("slider", spec.id));
    let sense = if enabled {
        Sense::click_and_drag()
    } else {
        Sense::hover()
    };
    let resp = ui.interact(track_rect.expand2(vec2(8.0, 2.0)), id, sense);
    let value_rect = Rect::from_min_max(
        pos2(label_rect.right() - 72.0, label_rect.top()),
        label_rect.max,
    );
    let name_rect =
        Rect::from_min_max(label_rect.min, pos2(value_rect.left(), label_rect.bottom()));
    let label_sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let label_resp = ui
        .interact(name_rect, id.with("label"), label_sense)
        .on_hover_text(format!("{}\nDouble-click to reset.", spec.help));
    let value_resp = ui
        .interact(value_rect, id.with("value"), label_sense)
        .on_hover_text("Click to type a value");

    let before = *value;
    let typing_id = id.with("typing");
    let mut typing: Option<(String, bool)> = ui.data(|d| d.get_temp(typing_id));
    if enabled && typing.is_none() && value_resp.clicked() {
        typing = Some((
            spec.format(*value).trim_start_matches('+').to_string(),
            true,
        ));
    }

    let to_x = |v: f64| track_rect.left() + spec.position(v) as f32 * track_rect.width();
    // Where the thumb was this frame, before any change.
    let thumb_x = to_x(*value);
    let on_thumb = |p: Pos2| (p.x - thumb_x).abs() <= THUMB_RING + 1.0;

    if let Some((mut text, first)) = typing.take() {
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(value_rect)
                .layout(egui::Layout::right_to_left(egui::Align::Center)),
        );
        let edit = child.add(
            egui::TextEdit::singleline(&mut text)
                .font(t.font(12.5))
                .align(egui::Align2::RIGHT_CENTER)
                .desired_width(value_rect.width())
                .margin(egui::Margin::ZERO),
        );
        if first {
            edit.request_focus();
        }
        let (enter, escape) = ui.input(|i| {
            (
                i.key_pressed(egui::Key::Enter),
                i.key_pressed(egui::Key::Escape),
            )
        });
        if escape || !enabled {
            ui.data_mut(|d| d.remove::<(String, bool)>(typing_id));
        } else if enter || (!first && edit.lost_focus()) {
            if let Some(v) = spec.parse(&text) {
                *value = v;
            }
            ui.data_mut(|d| d.remove::<(String, bool)>(typing_id));
        } else {
            typing = Some((text.clone(), false));
            ui.data_mut(|d| d.insert_temp(typing_id, (text, false)));
        }
    } else if enabled && (label_resp.double_clicked() || resp.double_clicked()) {
        *value = spec.default;
    } else if enabled && (resp.dragged() || resp.drag_started()) {
        if let Some(p) = resp.interact_pointer_pos() {
            let fine_id = id.with("fine");
            let pos = if ui.input(|i| i.modifiers.shift) {
                // A tenth of the pointer's travel; the unrounded position is kept so slow
                // drags add up instead of rounding back to the same step.
                let dx = ui.input(|i| i.pointer.delta().x);
                let kept: Option<f64> = ui.data(|d| d.get_temp(fine_id));
                let start = kept.unwrap_or_else(|| spec.position(*value));
                (start + f64::from(dx / track_rect.width()) * 0.1).clamp(0.0, 1.0)
            } else {
                f64::from((p.x - track_rect.left()) / track_rect.width())
            };
            ui.data_mut(|d| d.insert_temp(fine_id, pos));
            *value = spec.snap(spec.value_at(pos));
        }
        if resp.drag_stopped() {
            ui.data_mut(|d| d.remove::<f64>(id.with("fine")));
        }
    } else if enabled
        && resp.clicked()
        && let Some(p) = resp.interact_pointer_pos()
        && !on_thumb(p)
    {
        // A click on the thumb leaves it, so a double-click there is a clean reset.
        let pos = f64::from((p.x - track_rect.left()) / track_rect.width());
        *value = spec.snap(spec.value_at(pos));
    }
    if enabled && resp.hovered() && resp.hover_pos().is_some_and(on_thumb) {
        resp.clone()
            .on_hover_text(format!("{}\nDouble-click to reset.", spec.help));
    }

    // Paint.
    let hovered = resp.hovered() || resp.dragged();
    let text_c = if enabled {
        t.text_label
    } else {
        t.text_disabled
    };
    let p = ui.painter();
    p.text(
        label_rect.left_center(),
        Align2::LEFT_CENTER,
        spec.label,
        t.font(12.5),
        text_c,
    );
    if typing.is_none() {
        p.text(
            label_rect.right_center(),
            Align2::RIGHT_CENTER,
            spec.format(*value),
            t.font(12.5),
            text_c,
        );
    }
    let y = track_rect.center().y;
    let tx = to_x(*value);
    p.rect_filled(
        Rect::from_min_max(
            pos2(track_rect.left(), y - 1.0),
            pos2(track_rect.right(), y + 1.0),
        ),
        1.0,
        t.track,
    );
    p.rect_filled(
        Rect::from_center_size(pos2(tx, y), vec2(THUMB_RING * 2.0 + 4.0, 6.0)),
        0.0,
        t.chrome,
    );
    let ring_c = if !enabled {
        t.text_disabled
    } else if hovered {
        t.thumb_hover
    } else {
        t.thumb
    };
    p.circle_filled(pos2(tx, y), THUMB_RING, t.chrome);
    p.circle_stroke(pos2(tx, y), THUMB_RING, Stroke::new(2.0, ring_c));
    if hovered && enabled {
        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
    }
    (*value - before).abs() > 1e-12
}

/// Persistent open/closed state for a section or expandable row.
fn open_state(ui: &Ui, key: &str, default: bool) -> (Id, bool) {
    let id = Id::unique(("bw-open", key));
    let open = ui.data_mut(|d| *d.get_persisted_mut_or(id, default));
    (id, open)
}

/// An accordion section: a 52 pt header with a chevron and a semibold title, then its
/// content when open. Open state persists between runs.
pub fn section(
    ui: &mut Ui,
    key: &str,
    title: &str,
    default_open: bool,
    content: impl FnOnce(&mut Ui),
) {
    let t = Tokens::get(ui.ctx());
    let (state, open) = open_state(ui, key, default_open);
    let w = ui.available_width();
    let (r, resp) = ui.allocate_exact_size(vec2(w, 52.0), Sense::click());
    let p = ui.painter();
    if resp.hovered() {
        p.rect_filled(r, 0.0, t.chrome.gamma_multiply(1.06));
    }
    let chev = Rect::from_center_size(pos2(r.left() + 30.0, r.center().y), vec2(14.0, 14.0));
    paint_icon(
        p,
        chev,
        if open {
            Icon::ChevronDown
        } else {
            Icon::ChevronRight
        },
        t.text_label,
    );
    p.text(
        pos2(r.left() + 44.0, r.center().y),
        Align2::LEFT_CENTER,
        title,
        t.semibold(14.0),
        t.text,
    );
    let open = if resp.clicked() { !open } else { open };
    ui.data_mut(|d| d.insert_persisted(state, open));
    if open {
        content(ui);
        ui.add_space(10.0);
    }
    divider(ui);
}

/// Open every section and expandable row named in `keys` (e.g. for "Edit…").
pub fn open_all(ctx: &egui::Context, keys: &[&str]) {
    set_open(ctx, keys, true);
}

/// Open or close the sections and expandable rows named in `keys`.
pub fn set_open(ctx: &egui::Context, keys: &[&str], open: bool) {
    ctx.data_mut(|d| {
        for key in keys {
            d.insert_persisted(Id::unique(("bw-open", *key)), open);
        }
    });
}

/// A collapsible group header in a list (32 pt): chevron, title and item count. Returns
/// whether the group is open; the state persists between runs.
pub fn group_header(ui: &mut Ui, key: &str, title: &str, count: usize) -> bool {
    let t = Tokens::get(ui.ctx());
    let (state, open) = open_state(ui, key, true);
    let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 32.0), Sense::click());
    let p = ui.painter();
    if resp.hovered() {
        p.rect_filled(r, 0.0, t.chrome.gamma_multiply(1.06));
    }
    let chev = Rect::from_center_size(pos2(r.left() + 22.0, r.center().y), vec2(12.0, 12.0));
    paint_icon(
        p,
        chev,
        if open {
            Icon::ChevronDown
        } else {
            Icon::ChevronRight
        },
        t.text_label,
    );
    p.text(
        pos2(r.left() + 36.0, r.center().y),
        Align2::LEFT_CENTER,
        title,
        t.semibold(13.0),
        t.text_label,
    );
    p.text(
        pos2(r.right() - PAD_R, r.center().y),
        Align2::RIGHT_CENTER,
        count.to_string(),
        t.font(11.5),
        t.text_dim,
    );
    let open = if resp.clicked() { !open } else { open };
    ui.data_mut(|d| d.insert_persisted(state, open));
    open
}

/// A thin divider between sections.
pub fn divider(ui: &mut Ui) {
    let t = Tokens::get(ui.ctx());
    let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
    ui.painter().rect_filled(r, 0.0, t.divider);
}

/// An expandable row inside a section, for advanced controls: an inset box with an icon
/// tile, a title and a chevron. Open state persists between runs.
pub fn flyout(
    ui: &mut Ui,
    key: &str,
    title: &str,
    help: &str,
    icon: Icon,
    content: impl FnOnce(&mut Ui),
) {
    let t = Tokens::get(ui.ctx());
    let (state, open) = open_state(ui, key, false);
    let w = ui.available_width();
    let (outer, _) = ui.allocate_exact_size(vec2(w, 40.0), Sense::hover());
    let r = Rect::from_min_max(
        pos2(outer.left() + 16.0, outer.top() + 4.0),
        pos2(outer.right() - 16.0, outer.bottom() - 4.0),
    );
    let resp = ui
        .interact(r, Id::unique(("flyout", key)), Sense::click())
        .on_hover_text(help);
    let p = ui.painter();
    p.rect_filled(
        r,
        4.0,
        if resp.hovered() {
            t.hover.gamma_multiply(0.8)
        } else {
            t.inset
        },
    );
    let tile = Rect::from_min_size(
        pos2(r.left() + 4.0, r.top() + 4.0),
        vec2(r.height() - 8.0, r.height() - 8.0),
    );
    p.rect_filled(tile, 3.0, t.chrome.gamma_multiply(1.15));
    paint_icon(p, tile.shrink(5.0), icon, t.text_label);
    p.text(
        pos2(tile.right() + 10.0, r.center().y),
        Align2::LEFT_CENTER,
        title,
        t.semibold(13.0),
        t.text_label,
    );
    let chev = Rect::from_center_size(pos2(r.right() - 14.0, r.center().y), vec2(14.0, 14.0));
    paint_icon(
        p,
        chev,
        if open {
            Icon::ChevronDown
        } else {
            Icon::ChevronLeft
        },
        t.text_label,
    );
    let open = if resp.clicked() { !open } else { open };
    ui.data_mut(|d| d.insert_persisted(state, open));
    if open {
        ui.add_space(4.0);
        content(ui);
        ui.add_space(4.0);
    }
}

/// A sub-heading inside a section.
pub fn subtitle(ui: &mut Ui, title: &str, help: &str) {
    let t = Tokens::get(ui.ctx());
    let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 28.0), Sense::hover());
    ui.painter().text(
        pos2(r.left() + PAD_L, r.center().y + 2.0),
        Align2::LEFT_CENTER,
        title,
        t.semibold(13.0),
        t.text_label,
    );
    if !help.is_empty() {
        resp.on_hover_text(help);
    }
}

/// A dim one-line note inside a section.
pub fn note(ui: &mut Ui, text: impl Into<String>, help: &str) {
    let t = Tokens::get(ui.ctx());
    inset(ui, |ui| {
        let r = ui.label(
            egui::RichText::new(text.into())
                .font(t.font(11.5))
                .color(t.text_dim),
        );
        if !help.is_empty() {
            r.on_hover_text(help);
        }
    });
}

/// A small bordered text button (24 pt), as LightCraft's Auto / Reset buttons.
pub fn text_button(ui: &mut Ui, label: &str, active: bool, enabled: bool) -> Response {
    let t = Tokens::get(ui.ctx());
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_string(), t.semibold(11.5), t.text);
    let w = (galley.size().x + 16.0).max(37.0);
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (r, resp) = ui.allocate_exact_size(vec2(w, 24.0), sense);
    paint_button(ui.painter(), r, &resp, label, active, enabled, &t);
    resp
}

fn paint_button(
    p: &Painter,
    r: Rect,
    resp: &Response,
    label: &str,
    active: bool,
    enabled: bool,
    t: &Tokens,
) {
    let fill = if active {
        t.pressed
    } else if enabled && resp.hovered() {
        t.hover
    } else {
        t.button
    };
    p.rect_filled(r, 4.0, fill);
    let border = if active { t.text_dim } else { t.button_border };
    p.rect_stroke(r, 4.0, Stroke::new(1.0, border), StrokeKind::Inside);
    let c = if !enabled {
        t.text_disabled
    } else if active {
        t.text
    } else {
        t.text_label
    };
    p.text(
        r.center(),
        Align2::CENTER_CENTER,
        label,
        t.semibold(11.5),
        c,
    );
}

/// One option of a segmented control.
pub struct Segment<'a> {
    pub label: &'a str,
    pub selected: bool,
    pub enabled: bool,
    pub help: &'a str,
}

/// Equal-width buttons, `per_row` to a row, inset to the content column. Returns the
/// index of the option clicked.
pub fn segmented(ui: &mut Ui, segments: &[Segment], per_row: usize) -> Option<usize> {
    let t = Tokens::get(ui.ctx());
    let per_row = per_row.max(1);
    let mut clicked = None;
    inset(ui, |ui| {
        let gap = 4.0;
        let full = ui.available_width();
        for (row, chunk) in segments.chunks(per_row).enumerate() {
            let w = (full - gap * (per_row as f32 - 1.0)) / per_row as f32;
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                for (i, s) in chunk.iter().enumerate() {
                    let sense = if s.enabled {
                        Sense::click()
                    } else {
                        Sense::hover()
                    };
                    let (r, resp) = ui.allocate_exact_size(vec2(w, 24.0), sense);
                    paint_button(ui.painter(), r, &resp, s.label, s.selected, s.enabled, &t);
                    if !s.help.is_empty() {
                        resp.clone().on_hover_text(s.help);
                    }
                    if resp.clicked() {
                        clicked = Some(row * per_row + i);
                    }
                }
            });
            ui.add_space(gap);
        }
    });
    clicked
}

/// A "Label  Value ▾" row that opens a menu, as LightCraft's profile picker.
pub fn dropdown(
    ui: &mut Ui,
    key: &str,
    label: &str,
    value: &str,
    help: &str,
    menu: impl FnOnce(&mut Ui),
) {
    let t = Tokens::get(ui.ctx());
    let w = ui.available_width();
    let (r, _) = ui.allocate_exact_size(vec2(w, 34.0), Sense::hover());
    let p = ui.painter();
    let label_end = p
        .text(
            pos2(r.left() + PAD_L, r.center().y),
            Align2::LEFT_CENTER,
            label,
            t.font(13.0),
            t.text_dim,
        )
        .right();
    let x = (label_end + 10.0).max(r.left() + PAD_L + 70.0);
    let value_galley = p.layout_no_wrap(value.to_string(), t.font(13.0), t.text);
    let max_value_w = (r.right() - PAD_R - 20.0 - x).max(20.0);
    let value_w = value_galley.size().x.min(max_value_w);
    let hit = Rect::from_min_max(
        pos2(x - 4.0, r.top() + 4.0),
        pos2(x + value_w + 22.0, r.bottom() - 4.0),
    );
    let resp = ui
        .interact(hit, Id::unique(("dropdown", key)), Sense::click())
        .on_hover_text(help);
    let p = ui.painter();
    if resp.hovered() {
        p.rect_filled(hit, 4.0, t.hover.gamma_multiply(0.7));
    }
    let clip = Rect::from_min_max(pos2(x, r.top()), pos2(x + value_w, r.bottom()));
    p.with_clip_rect(clip).galley(
        pos2(x, r.center().y - value_galley.size().y / 2.0),
        value_galley,
        t.text,
    );
    paint_icon(
        p,
        Rect::from_center_size(pos2(x + value_w + 11.0, r.center().y), vec2(12.0, 12.0)),
        Icon::ChevronDown,
        t.text_label,
    );
    egui::Popup::menu(&resp).show(|ui| {
        ui.set_min_width(200.0);
        menu(ui);
    });
}

/// A 40 pt icon button for the tool strip: a rounded square when active or hovered.
pub fn strip_button(ui: &mut Ui, icon: Icon, active: bool, enabled: bool, help: &str) -> Response {
    let t = Tokens::get(ui.ctx());
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (r, resp) = ui.allocate_exact_size(vec2(40.0, 40.0), sense);
    let p = ui.painter();
    let square = Rect::from_center_size(r.center(), vec2(32.0, 32.0));
    if active {
        p.rect_filled(square, 4.0, t.tool_active);
    } else if enabled && resp.hovered() {
        p.rect_filled(square, 4.0, t.hover.gamma_multiply(0.7));
    }
    let c = if !enabled {
        t.text_disabled
    } else if active {
        t.pick
    } else {
        t.icon
    };
    paint_icon(
        p,
        Rect::from_center_size(r.center(), vec2(22.0, 22.0)),
        icon,
        c,
    );
    resp.on_hover_text(help)
}

/// Line icons drawn in code on a 20 × 20 grid with 1.5 px strokes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Icon {
    ChevronDown,
    ChevronRight,
    ChevronLeft,
    /// Develop: three slider tracks with knobs.
    Sliders,
    /// Styles: a 2 × 2 grid of swatches.
    Grid,
    /// Export: an arrow leaving a tray.
    Export,
    Info,
    /// Advanced settings: a gear-like dial.
    Dial,
    /// Characteristic curve: an S-curve in a frame.
    Curve,
    /// Colour temperature: a thermometer.
    Thermometer,
    /// A folder in the folder tree.
    Folder,
    /// A mounted volume (memory card, external drive).
    Drive,
    /// Add (Browse Folder…).
    Plus,
}

/// Paint `icon` in `rect`.
pub fn paint_icon(p: &Painter, rect: Rect, icon: Icon, color: Color32) {
    let s = rect.width().min(rect.height()) / 20.0;
    let o = rect.center() - vec2(10.0, 10.0) * s;
    let at = |x: f32, y: f32| o + vec2(x, y) * s;
    let stroke = Stroke::new(1.5, color);
    let line = |pts: &[(f32, f32)]| {
        for w in pts.windows(2) {
            p.line_segment([at(w[0].0, w[0].1), at(w[1].0, w[1].1)], stroke);
        }
    };
    match icon {
        Icon::ChevronDown => line(&[(5.0, 7.5), (10.0, 12.5), (15.0, 7.5)]),
        Icon::ChevronRight => line(&[(7.5, 5.0), (12.5, 10.0), (7.5, 15.0)]),
        Icon::ChevronLeft => line(&[(12.5, 5.0), (7.5, 10.0), (12.5, 15.0)]),
        Icon::Sliders => {
            for (y, knob) in [(5.0, 13.0), (10.0, 6.0), (15.0, 11.0)] {
                line(&[(2.5, y), (17.5, y)]);
                p.circle_filled(at(knob, y), 2.2 * s, color);
            }
        }
        Icon::Grid => {
            for (x, y) in [(3.0, 3.0), (11.0, 3.0), (3.0, 11.0), (11.0, 11.0)] {
                p.rect_stroke(
                    Rect::from_min_size(at(x, y), vec2(6.0, 6.0) * s),
                    1.0,
                    stroke,
                    StrokeKind::Middle,
                );
            }
        }
        Icon::Export => {
            line(&[(3.5, 11.0), (3.5, 16.5), (16.5, 16.5), (16.5, 11.0)]);
            line(&[(10.0, 3.0), (10.0, 12.5)]);
            line(&[(6.5, 6.5), (10.0, 3.0), (13.5, 6.5)]);
        }
        Icon::Info => {
            p.circle_stroke(at(10.0, 10.0), 7.5 * s, stroke);
            line(&[(10.0, 9.0), (10.0, 14.0)]);
            p.circle_filled(at(10.0, 6.3), 1.0 * s, color);
        }
        Icon::Dial => {
            p.circle_stroke(at(10.0, 10.0), 3.0 * s, stroke);
            for k in 0..8 {
                let a = k as f32 * std::f32::consts::FRAC_PI_4;
                let (c, sn) = (a.cos(), a.sin());
                p.line_segment(
                    [
                        at(10.0 + 5.0 * c, 10.0 + 5.0 * sn),
                        at(10.0 + 7.5 * c, 10.0 + 7.5 * sn),
                    ],
                    stroke,
                );
            }
        }
        Icon::Curve => {
            p.rect_stroke(
                Rect::from_min_size(at(2.5, 2.5), vec2(15.0, 15.0) * s),
                1.5,
                stroke,
                StrokeKind::Middle,
            );
            let pts: Vec<Pos2> = (0..=12)
                .map(|i| {
                    let x = i as f32 / 12.0;
                    let y = x * x * (3.0 - 2.0 * x);
                    at(4.0 + 12.0 * x, 16.0 - 12.0 * y)
                })
                .collect();
            p.add(egui::Shape::line(pts, stroke));
        }
        Icon::Thermometer => {
            line(&[(8.5, 3.5), (8.5, 12.0)]);
            line(&[(11.5, 3.5), (11.5, 12.0)]);
            line(&[(8.5, 3.5), (11.5, 3.5)]);
            p.circle_stroke(at(10.0, 14.5), 3.0 * s, stroke);
        }
        Icon::Folder => line(&[
            (2.5, 5.5),
            (2.5, 15.5),
            (17.5, 15.5),
            (17.5, 7.0),
            (9.5, 7.0),
            (8.0, 4.5),
            (3.5, 4.5),
            (2.5, 5.5),
        ]),
        Icon::Drive => {
            p.rect_stroke(
                Rect::from_min_size(at(2.5, 6.0), vec2(15.0, 8.0) * s),
                1.5,
                stroke,
                StrokeKind::Middle,
            );
            p.circle_filled(at(14.0, 10.0), 1.0 * s, color);
        }
        Icon::Plus => {
            line(&[(10.0, 4.0), (10.0, 16.0)]);
            line(&[(4.0, 10.0), (16.0, 10.0)]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(log: bool) -> SliderSpec<'static> {
        SliderSpec {
            id: "t",
            label: "T",
            help: "",
            min: if log { 2000.0 } else { -4.0 },
            max: if log { 12000.0 } else { 4.0 },
            default: 0.0,
            step: if log { 10.0 } else { 0.05 },
            decimals: if log { 0 } else { 2 },
            suffix: if log { " K" } else { "" },
            signed: !log,
            log,
        }
    }

    #[test]
    fn positions_round_trip() {
        for log in [false, true] {
            let s = spec(log);
            for p in [0.0, 0.25, 0.5, 1.0] {
                assert!((s.position(s.value_at(p)) - p).abs() < 1e-9);
            }
        }
        // Logarithmic: the middle of the track is the geometric mean.
        let s = spec(true);
        assert!((s.value_at(0.5) - (2000.0f64 * 12000.0).sqrt()).abs() < 1e-6);
    }

    #[test]
    fn formats_and_parses_values() {
        let s = spec(false);
        assert_eq!(s.format(0.5), "+0.50");
        assert_eq!(s.format(-0.0), "0");
        assert_eq!(s.format(-1.25), "-1.25");
        assert_eq!(s.parse("+1,5"), Some(1.5));
        assert_eq!(s.parse("9"), Some(4.0));
        assert_eq!(s.parse("abc"), None);
        assert_eq!(spec(true).parse("5600 K"), Some(5600.0));
    }
}
