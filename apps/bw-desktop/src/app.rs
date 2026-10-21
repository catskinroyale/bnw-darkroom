//! Application state and UI.
//!
//! Layout: folder browser (left), preview (centre), style editor (right), style
//! gallery showing the current image in every style (bottom).

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, Sender};

use bw_profiles::{ModelSummary, Style};
use eframe::egui::{self, Color32, Pos2, Sense, Stroke, TextureHandle, TextureOptions, Vec2};

use crate::browser::{self, Entry, ThumbRequest};
use crate::export::{Conflict, ExportSettings, Format, NameContext, TOKENS, resolve_conflict};
use crate::source::Source;
use crate::style_menu::{self, StyleAction};
use crate::styles::StyleStore;
use crate::worker::{self, Job, Msg};
use crate::workflow;
use bw_raw::RawAdjust;

const KEY_FOLDER: &str = "folder";
const KEY_VIEW: &str = "workflow_view";
const KEY_THEME: &str = "theme";
const KEY_BROWSER_THUMBS: &str = "browser_thumbs";
const KEY_BROWSER_BITMAPS: &str = "browser_bitmaps";
const KEY_BROWSER_JPEGS: &str = "browser_jpegs";
const KEY_EXPORT_DIR: &str = "export_dir";
const KEY_EXPORT: &str = "export_settings";
const KEY_PANELS: &str = "panels";
/// Thumbnail size in browser rows.
const THUMB_ROW: f32 = 28.0;
/// Side of the detail window's image area, in points.
const DETAIL_VIEW_PT: f32 = 512.0;

/// One style-gallery tile.
#[derive(Clone)]
enum Tile {
    Pending,
    Ready(TextureHandle),
    /// The style cannot be rendered (e.g. no published time for its film and developer).
    Unavailable,
}

struct LoadedImage {
    path: PathBuf,
    source: Arc<Source>,
}

pub struct App {
    jobs: Sender<Job>,
    msgs: Receiver<Msg>,
    msg_tx: Sender<Msg>,
    thumb_tx: Sender<ThumbRequest>,

    folder: PathBuf,
    entries: Vec<Entry>,
    thumbs: HashMap<PathBuf, TextureHandle>,

    selected: Option<PathBuf>,
    image_generation: u64,
    loaded: Option<LoadedImage>,
    /// Per-image adjustments (exposure, white balance, …) for the selected image.
    adjust: RawAdjust,
    /// `adjust` has changes not yet written to the image's sidecar.
    adjust_dirty: bool,
    loading: bool,
    original: Option<TextureHandle>,

    store: StyleStore,
    current: usize,
    draft: Style,
    summary: Option<ModelSummary>,
    /// Display label of the draft's developer, for export file names.
    developer_label: String,
    /// Workflow panel shows every control rather than the simple chips.
    view: workflow::View,
    /// File browser shows thumbnails in its rows (otherwise a compact list).
    browser_thumbs: bool,
    /// Which non-raw files the browser lists (raw files always are).
    browser_filter: browser::Filter,
    /// Long edge, in pixels, the preview is rendered at (follows the screen size).
    preview_edge: usize,

    preview_generation: u64,
    preview: Option<TextureHandle>,
    preview_ms: Option<u128>,
    show_original: bool,

    show_browser: bool,
    show_styles: bool,
    show_gallery: bool,

    detail_enabled: bool,
    detail_centre: [f32; 2],
    /// Detail magnification: 1 = 100 % (one image pixel per screen pixel), 2 = 200 %.
    detail_zoom: u8,
    /// Area of the image the detail window shows, as fractions `[x0, y0, x1, y1]`.
    detail_area: Option<[f32; 4]>,
    /// Screen pixels per point (2 on Retina displays), updated every frame.
    pixels_per_point: f32,
    detail_generation: u64,
    detail: Option<TextureHandle>,

    tiles_generation: u64,
    tiles: Vec<Tile>,

    export: ExportSettings,
    /// The export dialog is open.
    export_open: bool,
    exports_running: usize,
    status: String,

    presets: Vec<bw_profiles::Preset>,
    /// Style being renamed: index, text field, validation error.
    rename_dialog: Option<(usize, String, Option<String>)>,
    /// Style waiting for delete confirmation.
    delete_dialog: Option<usize>,
    /// Open "…" menu: style index and the screen position to show it at.
    style_popup: Option<(usize, Pos2)>,
    /// The style editor window is open (it edits the current style, with live preview).
    edit_open: bool,
    about: crate::about::About,
    theme: egui::ThemePreference,
    #[cfg(target_os = "macos")]
    menu: Option<crate::menu::AppMenu>,

    /// Test hook: `BW_DARKROOM_SCREENSHOT=<png>` saves one screenshot of the window once
    /// the image, preview and gallery have rendered, then quits.
    screenshot: Option<(PathBuf, std::time::Instant, bool)>,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, start: Option<PathBuf>) -> Self {
        let ctx = cc.egui_ctx.clone();
        let (jobs, job_rx) = mpsc::channel();
        let (msg_tx, msgs) = mpsc::channel();
        let (thumb_tx, thumb_rx) = mpsc::channel();
        worker::spawn_renderer(job_rx, msg_tx.clone(), ctx.clone());
        browser::spawn_thumbnailer(thumb_rx, msg_tx.clone(), ctx);

        let storage = cc.storage;
        let get = |key: &str| storage.and_then(|s| s.get_string(key));
        // A file argument opens its folder and selects it.
        let start_image = start.as_ref().filter(|p| p.is_file()).cloned();
        let start_folder = match &start_image {
            Some(file) => file.parent().map(PathBuf::from),
            None => start,
        };
        let folder = start_folder
            .or_else(|| get(KEY_FOLDER).map(PathBuf::from))
            .filter(|p| p.is_dir())
            .or_else(dirs::picture_dir)
            .or_else(dirs::home_dir)
            .unwrap_or_else(|| PathBuf::from("/"));
        let view = workflow::View::from_key(get(KEY_VIEW).as_deref().unwrap_or(""));
        let export = get(KEY_EXPORT)
            .and_then(|s| serde_json::from_str::<ExportSettings>(&s).ok())
            .unwrap_or_else(|| ExportSettings {
                // carry over the folder chosen before the export dialog existed
                folder: get(KEY_EXPORT_DIR).map(PathBuf::from),
                ..Default::default()
            });
        // Panel visibility as three 0/1 flags: browser, styles, gallery.
        let panels: Vec<bool> = get(KEY_PANELS)
            .map(|s| s.chars().map(|c| c == '1').collect())
            .filter(|v: &Vec<bool>| v.len() == 3)
            .unwrap_or_else(|| vec![true, true, true]);

        let store = StyleStore::load();
        let draft = store.entries[0].style.clone();
        let mut app = Self {
            jobs,
            msgs,
            msg_tx,
            thumb_tx,
            folder: PathBuf::new(),
            entries: Vec::new(),
            thumbs: HashMap::new(),
            selected: None,
            image_generation: 0,
            loaded: None,
            adjust: RawAdjust::default(),
            adjust_dirty: false,
            loading: false,
            original: None,
            tiles: vec![Tile::Pending; store.entries.len()],
            store,
            current: 0,
            summary: draft.model_summary().ok(),
            developer_label: bw_profiles::developer_label(&draft.developer),
            draft,
            view,
            browser_thumbs: get(KEY_BROWSER_THUMBS).is_some_and(|v| v == "1"),
            browser_filter: browser::Filter {
                bitmaps: get(KEY_BROWSER_BITMAPS).is_some_and(|v| v == "1"),
                jpegs: get(KEY_BROWSER_JPEGS).is_some_and(|v| v == "1"),
            },
            presets: bw_profiles::photographer_presets(),
            rename_dialog: None,
            delete_dialog: None,
            edit_open: false,
            style_popup: None,
            preview_edge: 1600,
            preview_generation: 0,
            preview: None,
            preview_ms: None,
            show_original: false,
            show_browser: panels[0],
            show_styles: panels[1],
            show_gallery: panels[2],
            detail_enabled: false,
            detail_centre: [0.5, 0.5],
            detail_zoom: 1,
            detail_area: None,
            pixels_per_point: cc.egui_ctx.pixels_per_point(),
            detail_generation: 0,
            detail: None,
            tiles_generation: 0,
            export,
            export_open: false,
            exports_running: 0,
            status: String::new(),
            about: Default::default(),
            theme: match get(KEY_THEME).as_deref() {
                Some("light") => egui::ThemePreference::Light,
                Some("dark") => egui::ThemePreference::Dark,
                _ => egui::ThemePreference::System,
            },
            #[cfg(target_os = "macos")]
            menu: crate::menu::AppMenu::install(&cc.egui_ctx),
            screenshot: std::env::var_os("BW_DARKROOM_SCREENSHOT")
                .map(|p| (PathBuf::from(p), std::time::Instant::now(), false)),
        };
        if std::env::var_os("BW_DARKROOM_SCREENSHOT_FRESH").is_some() {
            // Test runs can start from default panel sizes, ignoring saved layout.
            cc.egui_ctx.memory_mut(|m| *m = egui::Memory::default());
        }
        cc.egui_ctx.set_theme(app.theme);
        app.open_folder(folder);
        if let Some(image) = start_image {
            app.select_image(image);
        }
        app
    }

    // ----- actions -------------------------------------------------------------

    fn open_folder(&mut self, folder: PathBuf) {
        self.entries = browser::list_folder(&folder);
        self.folder = folder;
        let paths: Vec<PathBuf> = self
            .entries
            .iter()
            .filter(|e| !e.is_dir && !self.thumbs.contains_key(&e.path))
            .map(|e| e.path.clone())
            .rev() // the thumbnailer pops from the end, so list order is load order
            .collect();
        let _ = self.thumb_tx.send(ThumbRequest { paths });
    }

    fn select_image(&mut self, path: PathBuf) {
        if self.selected.as_ref() == Some(&path) {
            return;
        }
        self.save_adjust();
        self.adjust = crate::source::load_adjust(&path);
        self.image_generation += 1;
        self.selected = Some(path.clone());
        self.loaded = None;
        self.loading = true;
        self.preview = None;
        self.original = None;
        self.detail = None;
        self.tiles.iter_mut().for_each(|t| *t = Tile::Pending);
        self.status = format!("Loading {}…", file_name(&path));
        let _ = self.jobs.send(Job::Load {
            generation: self.image_generation,
            path,
        });
    }

    fn step_image(&mut self, delta: isize) {
        let images: Vec<&PathBuf> = self
            .entries
            .iter()
            .filter(|e| !e.is_dir && self.browser_filter.shows(e.kind))
            .map(|e| &e.path)
            .collect();
        if images.is_empty() {
            return;
        }
        let index = self
            .selected
            .as_ref()
            .and_then(|s| images.iter().position(|p| *p == s))
            .map_or(0, |i| {
                (i as isize + delta).rem_euclid(images.len() as isize) as usize
            });
        let next = images[index].clone();
        self.select_image(next);
    }

    /// The draft style changed: refresh model summary, preview and detail.
    fn draft_changed(&mut self) {
        self.summary = self.draft.model_summary().ok();
        self.developer_label = bw_profiles::developer_label(&self.draft.developer);
        self.request_preview();
    }

    /// Write the selected image's adjustments to its sidecar if they changed.
    fn save_adjust(&mut self) {
        if !self.adjust_dirty {
            return;
        }
        if let Some(path) = &self.selected
            && let Err(e) = crate::source::save_adjust(path, &self.adjust)
        {
            self.status = format!("Could not save image adjustments: {e}");
        }
        self.adjust_dirty = false;
    }

    /// The image adjustments changed: re-render everything that shows the image.
    fn adjust_changed(&mut self) {
        self.adjust_dirty = true;
        self.request_preview();
        self.request_tiles();
    }

    fn request_preview(&mut self) {
        // No summary means the style can't be rendered; the panel explains why.
        if self.loaded.is_none() || self.summary.is_none() {
            return;
        }
        self.preview_generation += 1;
        let _ = self.jobs.send(Job::Preview {
            generation: self.preview_generation,
            style: self.draft.clone(),
            adjust: self.adjust,
            edge: self.preview_edge,
        });
        if self.detail_enabled {
            self.request_detail();
        }
    }

    fn request_detail(&mut self) {
        if self.loaded.is_none() {
            return;
        }
        self.detail_generation += 1;
        let _ = self.jobs.send(Job::Detail {
            generation: self.detail_generation,
            style: self.draft.clone(),
            adjust: self.adjust,
            centre: self.detail_centre,
            // Enough image pixels to fill the view at this zoom on this screen.
            crop_px: (DETAIL_VIEW_PT * self.pixels_per_point / f32::from(self.detail_zoom)).round()
                as usize,
        });
    }

    fn request_tiles(&mut self) {
        if self.loaded.is_none() {
            return;
        }
        self.tiles_generation += 1;
        self.tiles = vec![Tile::Pending; self.store.entries.len()];
        let _ = self.jobs.send(Job::Tiles {
            generation: self.tiles_generation,
            styles: self.store.styles(),
            adjust: self.adjust,
        });
    }

    /// Apply a photographer preset: select its built-in style (or use it as the draft)
    /// and set the image contrast it calls for.
    fn apply_preset(&mut self, index: usize) {
        let preset = self.presets[index].clone();
        match self
            .store
            .entries
            .iter()
            .position(|e| e.builtin && e.style.name == preset.style.name)
        {
            Some(i) => self.select_style(i),
            None => {
                self.draft = preset.style;
                self.draft_changed();
            }
        }
        if preset.scene_contrast != 0.0 && self.adjust.contrast != preset.scene_contrast {
            self.adjust.contrast = preset.scene_contrast;
            self.adjust_changed();
        }
    }

    fn select_style(&mut self, index: usize) {
        self.current = index;
        self.draft = self.store.entries[index].style.clone();
        self.draft_changed();
    }

    fn is_modified(&self) -> bool {
        self.store
            .entries
            .get(self.current)
            .is_some_and(|e| e.style != self.draft)
    }

    fn save_as_new(&mut self) {
        let mut style = self.draft.clone();
        style.name = self.store.unique_name(&style.name);
        self.store.entries.push(crate::styles::StyleEntry {
            style: style.clone(),
            builtin: false,
        });
        self.current = self.store.entries.len() - 1;
        self.draft = style;
        self.persist_styles();
        self.request_tiles();
    }

    fn save_changes(&mut self) {
        let entry = &mut self.store.entries[self.current];
        if entry.builtin {
            return;
        }
        entry.style = self.draft.clone();
        self.persist_styles();
        self.request_tiles();
    }

    fn delete_style(&mut self, index: usize) {
        match self.store.delete(index) {
            Ok(removed) => {
                self.persist_styles();
                self.status = format!("Deleted style “{}”", removed.style.name);
                if self.current >= index {
                    self.select_style(
                        self.current
                            .saturating_sub(1)
                            .min(self.store.entries.len() - 1),
                    );
                }
                self.request_tiles();
            }
            Err(e) => self.status = e,
        }
    }

    fn handle_style_action(&mut self, action: StyleAction) {
        match action {
            StyleAction::Edit(i) => {
                self.select_style(i);
                self.edit_open = true;
            }
            StyleAction::Duplicate(i) => {
                let j = self.store.duplicate(i);
                self.persist_styles();
                self.select_style(j);
                self.request_tiles();
                self.status = format!("Duplicated as “{}”", self.store.entries[j].style.name);
            }
            StyleAction::Rename(i) => {
                self.rename_dialog = Some((i, self.store.entries[i].style.name.clone(), None));
            }
            StyleAction::Export(i) => {
                let style = self.store.entries[i].style.clone();
                let file = format!("{}.json", crate::export::slug(&style.name));
                if let Some(path) = rfd::FileDialog::new()
                    .set_file_name(file)
                    .add_filter("Style", &["json"])
                    .save_file()
                {
                    let result = serde_json::to_string_pretty(&style)
                        .map_err(|e| e.to_string())
                        .and_then(|json| std::fs::write(&path, json).map_err(|e| e.to_string()));
                    self.status = match result {
                        Ok(()) => format!("Exported “{}” to {}", style.name, path.display()),
                        Err(e) => format!("Could not export style: {e}"),
                    };
                }
            }
            StyleAction::MoveUp(i) | StyleAction::MoveDown(i) => {
                let delta = if matches!(action, StyleAction::MoveUp(_)) {
                    -1
                } else {
                    1
                };
                if let Some(j) = self.store.move_by(i, delta) {
                    // Keep the selection on the same style as it moves.
                    if self.current == i {
                        self.current = j;
                    } else if self.current == j {
                        self.current = i;
                    }
                    self.persist_styles();
                    self.request_tiles();
                }
            }
            StyleAction::Delete(i) => self.delete_dialog = Some(i),
        }
    }

    fn import_styles(&mut self) {
        let Some(paths) = rfd::FileDialog::new()
            .add_filter("Style", &["json"])
            .pick_files()
        else {
            return;
        };
        let mut imported = 0;
        let mut first = None;
        for path in paths {
            match std::fs::read_to_string(&path)
                .map_err(|e| e.to_string())
                .and_then(|t| crate::styles::parse_style_file(&t))
            {
                Ok(styles) => {
                    imported += styles.len();
                    first = first.or(self.store.import(styles));
                }
                Err(e) => self.status = format!("{}: {e}", path.display()),
            }
        }
        if let Some(i) = first {
            self.persist_styles();
            self.select_style(i);
            self.request_tiles();
            self.status = format!(
                "Imported {imported} style{}",
                if imported == 1 { "" } else { "s" }
            );
        }
    }

    /// The "…" menu for one style, drawn above everything at the button that opened it.
    /// Closes when an item is chosen, Escape is pressed or the pointer clicks elsewhere.
    fn style_popup(&mut self, ctx: &egui::Context) {
        let Some((index, pos)) = self.style_popup else {
            return;
        };
        if index >= self.store.entries.len() {
            self.style_popup = None;
            return;
        }
        let builtin = self.store.entries[index].builtin;
        let (up, down) = (
            self.store.can_move(index, -1),
            self.store.can_move(index, 1),
        );
        let mut action = None;
        let area = egui::Area::new(egui::Id::unique("style-popup"))
            .order(egui::Order::Foreground)
            .fixed_pos(pos)
            .constrain(true)
            .show(ctx, |ui| {
                egui::Frame::menu(ui.style()).show(ui, |ui| {
                    // Look like a menu: frameless, full-width items.
                    ui.style_mut().visuals.button_frame = false;
                    ui.set_min_width(160.0);
                    ui.with_layout(egui::Layout::top_down_justified(egui::Align::Min), |ui| {
                        action = style_menu::items(ui, index, builtin, up, down);
                    });
                });
            });
        // A press away from the menu (and not on the "…" that opened it) closes it.
        let clicked_elsewhere = ctx.input(|i| i.pointer.any_pressed())
            && !area.response.contains_pointer()
            && ctx
                .input(|i| i.pointer.interact_pos())
                .is_some_and(|p| p.distance(pos) > 30.0);
        if action.is_some() || clicked_elsewhere || ctx.input(|i| i.key_pressed(egui::Key::Escape))
        {
            self.style_popup = None;
        }
        if let Some(a) = action {
            self.handle_style_action(a);
        }
    }

    /// Rename and delete confirmation dialogs, and the style editor window.
    fn style_dialogs(&mut self, ctx: &egui::Context) {
        if let Some((index, mut name, error)) = self.rename_dialog.take() {
            let mut keep = true;
            let mut commit = false;
            egui::Window::new("Rename style")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    let field = ui.add(egui::TextEdit::singleline(&mut name).desired_width(320.0));
                    field.request_focus();
                    if field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        commit = true;
                    }
                    if let Some(e) = &error {
                        ui.colored_label(ui.visuals().warn_fg_color, e);
                    }
                    ui.horizontal(|ui| {
                        commit |= ui.button("Rename").clicked();
                        if ui.button("Cancel").clicked()
                            || ui.input(|i| i.key_pressed(egui::Key::Escape))
                        {
                            keep = false;
                        }
                    });
                });
            if commit {
                match self.store.rename(index, &name) {
                    Ok(()) => {
                        if self.current == index {
                            self.draft.name = self.store.entries[index].style.name.clone();
                        }
                        self.persist_styles();
                        keep = false;
                    }
                    Err(e) => {
                        self.rename_dialog = Some((index, name, Some(e)));
                        return;
                    }
                }
            }
            if keep {
                self.rename_dialog = Some((index, name, error));
            }
        }

        if let Some(index) = self.delete_dialog {
            let name = self.store.entries[index].style.name.clone();
            let mut close = false;
            egui::Window::new("Delete style?")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label(format!("Delete “{name}”? This can't be undone."));
                    ui.horizontal(|ui| {
                        if ui.button("Delete").clicked() {
                            self.delete_style(index);
                            close = true;
                        }
                        if ui.button("Cancel").clicked()
                            || ui.input(|i| i.key_pressed(egui::Key::Escape))
                        {
                            close = true;
                        }
                    });
                });
            if close {
                self.delete_dialog = None;
            }
        }

        if self.edit_open {
            let mut open = true;
            let builtin = self.store.entries[self.current].builtin;
            let modified = self.is_modified();
            let before = self.draft.clone();
            let mut save = false;
            let mut save_new = false;
            let mut revert = false;
            egui::Window::new(format!(
                "Edit style · {}",
                self.store.entries[self.current].style.name
            ))
            .id(egui::Id::unique("style-editor"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_pos([ctx.content_rect().center().x - 200.0, 80.0])
            .show(ctx, |ui| {
                ui.set_width(400.0);
                if builtin {
                    ui.label(
                        egui::RichText::new("Built-in style: save your changes as a new style.")
                            .small()
                            .weak(),
                    );
                }
                egui::ScrollArea::vertical()
                    .max_height(560.0)
                    .show(ui, |ui| {
                        workflow::detailed(ui, &mut self.draft, self.summary.as_ref());
                    });
                ui.separator();
                ui.horizontal(|ui| {
                    save = ui
                        .add_enabled(!builtin && modified, egui::Button::new("Save changes"))
                        .on_disabled_hover_text(if builtin {
                            "Built-in styles are read-only"
                        } else {
                            "No changes"
                        })
                        .clicked();
                    save_new = ui.button("Save as new style").clicked();
                    revert = ui
                        .add_enabled(modified, egui::Button::new("Revert"))
                        .clicked();
                });
            });
            if self.draft != before {
                self.draft_changed();
            }
            if save {
                self.save_changes();
            }
            if save_new {
                self.save_as_new();
            }
            if revert {
                self.select_style(self.current);
            }
            self.edit_open = open;
        }
    }

    fn persist_styles(&mut self) {
        if let Err(e) = self.store.save() {
            self.status = format!("Could not save styles: {e}");
        }
    }

    /// Values for the file-name template, for the current image and style.
    fn name_context<'a>(&'a self, source: &'a std::path::Path) -> NameContext<'a> {
        NameContext {
            source,
            style: &self.draft.name,
            film: self
                .summary
                .as_ref()
                .map_or(&self.draft.film, |m| &m.film_name),
            developer: &self.developer_label,
            ei: self.draft.exposure_index,
            unix_time: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_secs()),
        }
    }

    fn export(&mut self, ctx: &egui::Context) {
        self.save_adjust();
        let Some(loaded) = &self.loaded else { return };
        let name = self.export.file_name(&self.name_context(&loaded.path));
        let target = self.export.folder_for(&loaded.path).join(name);
        let job = worker::ExportJob {
            source: loaded.source.clone(),
            source_path: loaded.path.clone(),
            style: self.draft.clone(),
            adjust: self.adjust,
            target,
            format: self.export.format.resolve(&loaded.path),
            jpeg_quality: self.export.jpeg_quality,
            conflict: self.export.conflict,
            long_edge: self.export.long_edge,
            write_recipe: self.export.write_recipe,
        };
        if self.export.uses_number() {
            self.export.next_number += 1;
        }
        self.exports_running += 1;
        self.status = format!("Exporting “{}”…", self.draft.name);
        worker::spawn_export(job, self.msg_tx.clone(), ctx.clone());
    }

    fn export_dialog(&mut self, ctx: &egui::Context) {
        let Some(path) = self.loaded.as_ref().map(|l| l.path.clone()) else {
            self.export_open = false;
            return;
        };
        let mut open = true;
        let mut go = false;
        let mut cancel = false;
        // Owned copies of the template values, so the dialog can borrow the settings mutably.
        let (style, film, developer, ei, unix_time) = {
            let c = self.name_context(&path);
            (
                c.style.to_string(),
                c.film.to_string(),
                c.developer.to_string(),
                c.ei,
                c.unix_time,
            )
        };
        egui::Window::new("Export")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.set_width(460.0);
                let e = &mut self.export;
                egui::Grid::new("export").num_columns(2).spacing([12.0, 8.0]).show(ui, |ui| {
                    ui.label("Format").on_hover_text("File type to save.");
                    ui.horizontal_wrapped(|ui| {
                        for f in Format::ALL {
                            if ui.add(egui::Button::selectable(e.format == f, f.label())).on_hover_text(f.help()).clicked() {
                                e.format = f;
                            }
                        }
                    });
                    ui.end_row();

                    let resolved = e.format.resolve(&path);
                    if resolved == Format::Jpeg {
                        ui.label("JPEG quality").on_hover_text("Higher keeps more detail and grain structure but makes larger files. 85–95 suits most prints; below 70 grain turns blocky.");
                        ui.add(egui::Slider::new(&mut e.jpeg_quality, 1..=100))
                            .on_hover_text("Compression quality, 1–100.");
                        ui.end_row();
                    }

                    ui.label("Size").on_hover_text("Full resolution, or resized after rendering so the long edge is this many pixels.");
                    ui.horizontal_wrapped(|ui| {
                        if ui.add(egui::Button::selectable(e.long_edge.is_none(), "Full")).on_hover_text("The image's full resolution.").clicked() {
                            e.long_edge = None;
                        }
                        for px in [4096u32, 2048, 1080] {
                            if ui.add(egui::Button::selectable(e.long_edge == Some(px), px.to_string()))
                                .on_hover_text(format!("Long edge {px} px."))
                                .clicked()
                            {
                                e.long_edge = Some(px);
                            }
                        }
                        if let Some(px) = &mut e.long_edge {
                            ui.add(egui::DragValue::new(px).range(64..=20000).suffix(" px"))
                                .on_hover_text("Custom long edge in pixels.");
                        }
                    });
                    ui.end_row();

                    ui.label("File name").on_hover_text("Template for the file name. Click a token to add it.");
                    ui.add(egui::TextEdit::singleline(&mut e.template).desired_width(f32::INFINITY))
                        .on_hover_text("Template for the file name; the extension is added for you.");
                    ui.end_row();

                    ui.label("");
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing.x = 4.0;
                        for (token, help) in TOKENS {
                            if ui.small_button(token).on_hover_text(help).clicked() {
                                e.template.push_str(token);
                            }
                        }
                    });
                    ui.end_row();

                    ui.label("Numbering").on_hover_text("The {n} token: a counter that goes up by one after each export.");
                    ui.horizontal(|ui| {
                        if e.uses_number() {
                            ui.label("next");
                            ui.add(egui::DragValue::new(&mut e.next_number).range(0..=999_999))
                                .on_hover_text("The number the next export gets.");
                            ui.label("digits");
                            ui.add(egui::DragValue::new(&mut e.digits).range(1..=6))
                                .on_hover_text("Zero padding: 3 digits gives 001, 002, …");
                        } else if ui.small_button("Add {n}").on_hover_text("Number exports in sequence.").clicked() {
                            e.template.push_str("_{n}");
                        }
                    });
                    ui.end_row();

                    ui.label("If file exists").on_hover_text("What to do when a file with that name is already there.");
                    ui.horizontal(|ui| {
                        if ui.add(egui::Button::selectable(e.conflict == Conflict::AddSuffix, "Add -2, -3…"))
                            .on_hover_text("Keep the existing file and add a number to the new one.")
                            .clicked()
                        {
                            e.conflict = Conflict::AddSuffix;
                        }
                        if ui.add(egui::Button::selectable(e.conflict == Conflict::Overwrite, "Overwrite"))
                            .on_hover_text("Replace the existing file.")
                            .clicked()
                        {
                            e.conflict = Conflict::Overwrite;
                        }
                    });
                    ui.end_row();

                    ui.label("Folder").on_hover_text("Where exports are saved.");
                    ui.horizontal(|ui| {
                        let label = e.folder.as_ref().map_or("bw-exports next to each image".into(), |d| d.display().to_string());
                        ui.add(egui::Label::new(egui::RichText::new(label).small()).truncate());
                        if ui.small_button("Choose…").on_hover_text("Pick a folder for all exports.").clicked()
                            && let Some(dir) = rfd::FileDialog::new().pick_folder()
                        {
                            e.folder = Some(dir);
                        }
                        if e.folder.is_some() && ui.small_button("Next to image").on_hover_text("Save into a bw-exports folder beside each image.").clicked() {
                            e.folder = None;
                        }
                    });
                    ui.end_row();

                    ui.label("Recipe").on_hover_text("A JSON file with the full workflow, so the print can be reproduced.");
                    ui.checkbox(&mut e.write_recipe, "Save the recipe (.json) next to the image");
                    ui.end_row();
                });

                let name = e.file_name(&NameContext {
                    source: &path,
                    style: &style,
                    film: &film,
                    developer: &developer,
                    ei,
                    unix_time,
                });
                let target = resolve_conflict(e.folder_for(&path).join(&name), e.conflict);
                ui.separator();
                ui.label(egui::RichText::new("Will save").weak());
                ui.add(egui::Label::new(egui::RichText::new(target.display().to_string()).monospace().small()).wrap());
                if target.exists() {
                    ui.colored_label(ui.visuals().warn_fg_color, "This will replace an existing file.");
                }
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui.button("Export").on_hover_text("Render at full quality and save.").clicked() {
                        go = true;
                    }
                    if ui.button("Cancel").clicked() {
                        cancel = true;
                    }
                });
            });
        if go {
            self.export(ctx);
            self.export_open = false;
        }
        if !open || cancel {
            self.export_open = false;
        }
    }

    // ----- messages ------------------------------------------------------------

    fn handle_messages(&mut self, ctx: &egui::Context) {
        while let Ok(msg) = self.msgs.try_recv() {
            match msg {
                Msg::Loaded { generation, source } if generation == self.image_generation => {
                    let path = self.selected.clone().unwrap_or_default();
                    self.status = match source.capture() {
                        Some(info) => format!("{} · {}", file_name(&path), info.summary()),
                        None => file_name(&path),
                    };
                    self.loaded = Some(LoadedImage { path, source });
                    self.loading = false;
                    self.request_preview();
                    self.request_tiles();
                }
                Msg::LoadFailed { generation, error } if generation == self.image_generation => {
                    self.loading = false;
                    self.status = format!("Could not open image: {error}");
                }
                Msg::Preview {
                    generation,
                    image,
                    original,
                    millis,
                } if generation == self.preview_generation => {
                    if let Some(original) = original {
                        set_texture(ctx, &mut self.original, "original", original);
                    }
                    set_texture(ctx, &mut self.preview, "preview", image);
                    self.preview_ms = Some(millis);
                }
                Msg::Detail {
                    generation,
                    image,
                    area,
                } if generation == self.detail_generation => {
                    // Nearest-neighbour: at 200 % each image pixel shows as a crisp block.
                    match &mut self.detail {
                        Some(tex) => tex.set(image, TextureOptions::NEAREST),
                        None => {
                            self.detail =
                                Some(ctx.load_texture("detail", image, TextureOptions::NEAREST));
                        }
                    }
                    self.detail_area = Some(area);
                }
                Msg::Tile {
                    generation,
                    index,
                    image,
                } if generation == self.tiles_generation => {
                    if let Some(slot) = self.tiles.get_mut(index) {
                        *slot = Tile::Ready(ctx.load_texture(
                            format!("tile-{index}"),
                            image,
                            TextureOptions::LINEAR,
                        ));
                    }
                }
                Msg::TileFailed { generation, index } if generation == self.tiles_generation => {
                    if let Some(slot) = self.tiles.get_mut(index) {
                        *slot = Tile::Unavailable;
                    }
                }
                Msg::Thumb { path, image } => {
                    let tex =
                        ctx.load_texture(path.display().to_string(), image, TextureOptions::LINEAR);
                    self.thumbs.insert(path, tex);
                }
                Msg::RenderFailed { error } => self.status = format!("Render failed: {error}"),
                Msg::Exported { tiff, millis } => {
                    self.exports_running = self.exports_running.saturating_sub(1);
                    self.status = format!(
                        "Exported {} ({:.1} s)",
                        tiff.display(),
                        millis as f64 / 1000.0
                    );
                }
                Msg::ExportFailed { error } => {
                    self.exports_running = self.exports_running.saturating_sub(1);
                    self.status = format!("Export failed: {error}");
                }
                _ => {} // stale result for an older request
            }
        }
    }

    fn screenshot_hook(&mut self, ctx: &egui::Context) {
        if self.screenshot.is_none() {
            return;
        }
        // `BW_DARKROOM_SCREENSHOT_DETAIL=x,y,zoom` opens the detail view at that point.
        let detail_spec = std::env::var("BW_DARKROOM_SCREENSHOT_DETAIL").ok();
        if let Some(spec) = &detail_spec
            && self.loaded.is_some()
            && !self.detail_enabled
        {
            let v: Vec<f32> = spec
                .split(',')
                .filter_map(|p| p.trim().parse().ok())
                .collect();
            if let [x, y, zoom] = v[..] {
                self.detail_enabled = true;
                self.detail_centre = [x, y];
                self.detail_zoom = zoom as u8;
                self.request_detail();
            }
        }
        let Some((path, started, requested)) = &mut self.screenshot else {
            return;
        };
        // `BW_DARKROOM_SCREENSHOT_THEME=dark|light` and `…_VIEW=simple|detailed|photographers`
        // set the theme and workflow view for the shot (not saved: test runs don't persist).
        if let Some(theme) = std::env::var_os("BW_DARKROOM_SCREENSHOT_THEME") {
            let theme = if theme == "dark" {
                egui::ThemePreference::Dark
            } else {
                egui::ThemePreference::Light
            };
            if self.theme != theme {
                self.theme = theme;
                ctx.set_theme(theme);
            }
        }
        if let Some(view) = std::env::var_os("BW_DARKROOM_SCREENSHOT_VIEW") {
            self.view = workflow::View::from_key(&view.to_string_lossy());
        }
        // `BW_DARKROOM_SCREENSHOT_THUMBS=1|0` sets the file browser's thumbnail rows.
        if let Some(v) = std::env::var_os("BW_DARKROOM_SCREENSHOT_THUMBS") {
            self.browser_thumbs = v == "1";
        }
        // `BW_DARKROOM_SCREENSHOT_SHOW=bitmaps,jpegs` (or empty) sets the browser's filter.
        if let Some(v) = std::env::var_os("BW_DARKROOM_SCREENSHOT_SHOW") {
            let v = v.to_string_lossy();
            self.browser_filter = browser::Filter {
                bitmaps: v.contains("bitmaps"),
                jpegs: v.contains("jpegs"),
            };
        }
        // `BW_DARKROOM_SCREENSHOT_EDIT=1` opens the style editor for the shot.
        if std::env::var_os("BW_DARKROOM_SCREENSHOT_EDIT").is_some() {
            self.edit_open = true;
        }
        // `BW_DARKROOM_SCREENSHOT_ABOUT=1` opens the About window for the shot.
        if std::env::var_os("BW_DARKROOM_SCREENSHOT_ABOUT").is_some() {
            self.about.open = true;
        }
        // `BW_DARKROOM_SCREENSHOT_EXPORT=1` also opens the export dialog for the shot.
        if self.loaded.is_some() && std::env::var_os("BW_DARKROOM_SCREENSHOT_EXPORT").is_some() {
            self.export_open = true;
        }
        let rendered = (self.preview.is_some() || self.summary.is_none())
            && !self.tiles.iter().any(|t| matches!(t, Tile::Pending))
            && (detail_spec.is_none() || self.detail_area.is_some());
        if !*requested && (rendered || started.elapsed().as_secs() > 30) {
            *requested = true;
            let path = path.clone();
            ctx.request_screenshot(move |image| {
                let [w, h] = image.size;
                let rgba: Vec<u8> = image.pixels.iter().flat_map(|c| c.to_array()).collect();
                if let Some(buf) = image::RgbaImage::from_raw(w as u32, h as u32, rgba) {
                    let _ = buf.save(&path);
                }
                std::process::exit(0);
            });
        }
        ctx.request_repaint_after(std::time::Duration::from_millis(200));
    }

    // ----- panels --------------------------------------------------------------

    fn top_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("BW Darkroom")
                .on_hover_text(format!("Version {}", crate::version::full()));
            #[cfg(not(target_os = "macos"))]
            if ui.small_button("About").clicked() {
                self.about.open = true;
            }
            ui.separator();
            ui.toggle_value(&mut self.show_browser, "Files")
                .on_hover_text("Show or hide the file browser (B)");
            ui.toggle_value(&mut self.show_styles, "Workflow")
                .on_hover_text("Show or hide the styles and workflow panel (W)");
            ui.toggle_value(&mut self.show_gallery, "Gallery")
                .on_hover_text("Show or hide the style gallery (G)");
            ui.separator();
            if ui.button("Open folder…").clicked()
                && let Some(folder) = rfd::FileDialog::new()
                    .set_directory(&self.folder)
                    .pick_folder()
            {
                self.open_folder(folder);
            }
            ui.separator();
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let can_export = self.loaded.is_some();
                let mut theme = self.theme;
                for (choice, label, help) in [
                    (
                        egui::ThemePreference::Dark,
                        "Dark",
                        "Dark interface: easier on the eyes when judging prints in a dim room",
                    ),
                    (egui::ThemePreference::Light, "Light", "Light interface"),
                    (
                        egui::ThemePreference::System,
                        "Auto",
                        "Follow the system appearance (light or dark)",
                    ),
                ] {
                    ui.selectable_value(&mut theme, choice, label)
                        .on_hover_text(help);
                }
                if theme != self.theme {
                    self.theme = theme;
                    ui.ctx().set_theme(theme);
                }
                ui.separator();
                if ui
                    .add_enabled(can_export, egui::Button::new("Export…"))
                    .on_hover_text(
                        "Choose format, size and file name, then render at full quality (⌘E)",
                    )
                    .clicked()
                {
                    self.export_open = true;
                }
                if self.exports_running > 0 {
                    ui.spinner();
                }
            });
        });
    }

    /// The file browser: a compact list by default (one text row per file, names
    /// shortened in the middle so the frame number and extension stay visible, the file
    /// type as a right-hand tag, the thumbnail on hover), or rows with thumbnails.
    fn browser_panel(&mut self, ui: &mut egui::Ui) {
        let mut open: Option<PathBuf> = None;
        let mut select: Option<PathBuf> = None;

        ui.horizontal(|ui| {
            if ui
                .small_button("Up")
                .on_hover_text("Parent folder")
                .clicked()
            {
                open = self.folder.parent().map(|p| p.to_path_buf());
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.toggle_value(&mut self.browser_thumbs, egui::RichText::new("Thumbs").small())
                    .on_hover_text("Show thumbnails in the list (taller rows). Off: compact list, thumbnail on hover.");
                ui.add(egui::Label::new(egui::RichText::new(file_name(&self.folder)).strong()).truncate())
                    .on_hover_text(self.folder.display().to_string());
            });
        });
        // File types: raw is fixed on (the app is for raw files); the others are opt-in.
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            ui.add_enabled(
                false,
                egui::Button::selectable(true, egui::RichText::new("RAW").small()),
            )
            .on_disabled_hover_text("Camera raw files (NEF, DNG, ARW) are always listed");
            ui.toggle_value(
                &mut self.browser_filter.bitmaps,
                egui::RichText::new("TIFF/PNG").small(),
            )
            .on_hover_text(
                "Also list lossless bitmaps (TIFF, PNG), such as scans or edited masters",
            );
            ui.toggle_value(
                &mut self.browser_filter.jpegs,
                egui::RichText::new("JPEG").small(),
            )
            .on_hover_text("Also list compressed JPEG files");
        });
        let filter = self.browser_filter;
        let images = self
            .entries
            .iter()
            .filter(|e| !e.is_dir && filter.shows(e.kind))
            .count();
        let folders = self.entries.iter().filter(|e| e.is_dir).count();
        let hidden = self
            .entries
            .iter()
            .filter(|e| !filter.shows(e.kind))
            .count();
        let plural =
            |n: usize, one: &str, many: &str| format!("{n} {}", if n == 1 { one } else { many });
        let mut count = plural(images, "image", "images");
        if folders > 0 {
            count.push_str(&format!(" · {}", plural(folders, "folder", "folders")));
        }
        if hidden > 0 {
            count.push_str(&format!(" · {hidden} hidden"));
        }
        ui.label(egui::RichText::new(count).small().weak());
        let mut show_all = false;
        ui.separator();

        let thumbs_on = self.browser_thumbs;
        egui::ScrollArea::vertical()
            .auto_shrink(false)
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = if thumbs_on { 2.0 } else { 0.0 };
                let row_h = if thumbs_on {
                    THUMB_ROW + 4.0
                } else {
                    ui.spacing().interact_size.y
                };
                let width = ui.available_width();
                let padding = 2.0 * ui.spacing().button_padding.x + ui.spacing().item_spacing.x;
                let button_font = egui::TextStyle::Button.resolve(ui.style());
                let small_font = egui::TextStyle::Small.resolve(ui.style());
                let ctx = ui.ctx().clone();
                let measure = |text: &str, font: &egui::FontId| {
                    ctx.fonts_mut(|f| {
                        f.layout_no_wrap(text.to_owned(), font.clone(), Color32::WHITE)
                            .size()
                            .x
                    })
                };
                if images + folders == 0 {
                    if hidden > 0 {
                        ui.label(
                            egui::RichText::new(format!(
                                "No raw files here; {} hidden",
                                plural(hidden, "JPEG/TIFF/PNG file", "JPEG/TIFF/PNG files")
                            ))
                            .weak(),
                        );
                        show_all = ui.small_button("Show them").clicked();
                    } else {
                        ui.label(egui::RichText::new("No folders or images here").weak());
                    }
                }
                for entry in self.entries.iter().filter(|e| filter.shows(e.kind)) {
                    if entry.is_dir {
                        // Folders carry a "Folder" tag, like the file-type tags on images.
                        let room = width - padding - measure("Folder", &small_font);
                        let name = browser::middle_ellipsis(&entry.name, room, |t| {
                            measure(t, &button_font)
                        });
                        let response = ui
                            .add(
                                egui::Button::selectable(false, name)
                                    .right_text(egui::RichText::new("Folder").small().weak())
                                    .truncate()
                                    .min_size(Vec2::new(width, row_h)),
                            )
                            .on_hover_text(format!("Open folder “{}”", entry.name));
                        if response.clicked() {
                            open = Some(entry.path.clone());
                        }
                        continue;
                    }
                    let selected = self.selected.as_ref() == Some(&entry.path);
                    let tag = browser::type_tag(&entry.path);
                    let thumb = self.thumbs.get(&entry.path);
                    // Thumbnail plus the button's gap before the text, when shown.
                    let thumb_w = if thumbs_on && thumb.is_some() {
                        THUMB_ROW + ui.spacing().icon_spacing
                    } else {
                        0.0
                    };
                    let room = width - padding - thumb_w - measure(&tag, &small_font);
                    let name =
                        browser::middle_ellipsis(&entry.name, room, |t| measure(t, &button_font));
                    let button = match (thumbs_on, thumb) {
                        (true, Some(tex)) => egui::Button::selectable(
                            selected,
                            (
                                egui::Image::new(tex)
                                    .fit_to_exact_size(Vec2::splat(THUMB_ROW))
                                    .maintain_aspect_ratio(true),
                                name,
                            ),
                        ),
                        _ => egui::Button::selectable(selected, name),
                    }
                    .right_text(egui::RichText::new(&tag).small().weak())
                    .truncate()
                    .min_size(Vec2::new(width, row_h));
                    let response = ui.add(button).on_hover_ui(|ui| {
                        if let Some(tex) = thumb {
                            ui.add(egui::Image::new(tex).max_size(Vec2::splat(200.0)));
                        }
                        ui.label(&entry.name);
                    });
                    if response.clicked() {
                        select = Some(entry.path.clone());
                    }
                }
            });
        if show_all {
            self.browser_filter = browser::Filter {
                bitmaps: true,
                jpegs: true,
            };
        }
        if let Some(folder) = open {
            self.open_folder(folder);
        }
        if let Some(path) = select {
            self.select_image(path);
        }
    }

    fn style_panel(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical()
            .auto_shrink(false)
            .show(ui, |ui| {
                egui::CollapsingHeader::new(egui::RichText::new("Image").heading())
                    .default_open(true)
                    .show(ui, |ui| {
                        let source = self.loaded.as_ref().map(|l| l.source.clone());
                        if crate::image_panel::show(
                            ui,
                            &mut self.adjust,
                            source.as_deref(),
                            self.view == workflow::View::Detailed,
                        ) {
                            self.adjust_changed();
                        }
                    });
                ui.add_space(4.0);
                let mut pick = None;
                let menu_action: Option<StyleAction> = None;
                let mut popup_request = None;
                let mut import = false;
                egui::CollapsingHeader::new(egui::RichText::new("Styles").heading())
                    .default_open(true)
                    .show(ui, |ui| {
                        if ui
                            .small_button("Import…")
                            .on_hover_text("Add styles from .json files exported from BW Darkroom")
                            .clicked()
                        {
                            import = true;
                        }
                        ui.spacing_mut().item_spacing.y = 1.0;
                        for (i, entry) in self.store.entries.iter().enumerate() {
                            let label = if entry.builtin {
                                entry.style.name.clone()
                            } else {
                                format!("{}  (mine)", entry.style.name)
                            };
                            // Full-width row as before, with the "…" menu button laid on
                            // top of its right end: the row's width never changes, so the
                            // resizable panel can't be pushed wider.
                            const ELLIPSIS_W: f32 = 24.0;
                            let row = Vec2::new(ui.available_width(), 0.0);
                            let button = egui::Button::selectable(i == self.current, label)
                                .truncate()
                                .right_text(" ".repeat(4))
                                .min_size(row);
                            let row_response = ui.add(button);
                            if row_response.clicked() {
                                pick = Some(i);
                            }
                            let rect = egui::Rect::from_min_max(
                                egui::pos2(
                                    row_response.rect.right() - ELLIPSIS_W,
                                    row_response.rect.top(),
                                ),
                                row_response.rect.right_bottom(),
                            );
                            // The "…" is a click area over the row's right end that takes no
                            // layout space (`interact` + painting, no allocation): anything that
                            // asked for even a pixel more than the row would widen the
                            // resizable panel, which widens the rows, and so on.
                            let ellipsis = ui.interact(
                                rect,
                                egui::Id::unique(("style-ellipsis", i)),
                                Sense::click(),
                            );
                            let visuals = ui.visuals();
                            if ellipsis.hovered() {
                                ui.painter().rect_filled(
                                    rect.shrink(1.0),
                                    3.0,
                                    visuals.widgets.hovered.weak_bg_fill,
                                );
                            }
                            let colour = if ellipsis.hovered() {
                                visuals.strong_text_color()
                            } else {
                                visuals.text_color()
                            };
                            ui.painter().text(
                                rect.center(),
                                egui::Align2::CENTER_CENTER,
                                "…",
                                egui::TextStyle::Button.resolve(ui.style()),
                                colour,
                            );
                            let ellipsis = ellipsis.on_hover_text(
                                "Edit, duplicate, rename, export or delete this style",
                            );
                            // `BW_DARKROOM_SCREENSHOT_STYLEMENU=<index>` opens a menu for screenshots.
                            let screenshot_menu = self.screenshot.is_some()
                                && self.style_popup.is_none()
                                && std::env::var("BW_DARKROOM_SCREENSHOT_STYLEMENU")
                                    .ok()
                                    .and_then(|v| v.parse::<usize>().ok())
                                    == Some(i);
                            if ellipsis.clicked() || screenshot_menu {
                                popup_request = Some((i, rect.left_bottom()));
                            }
                        }
                    });
                if let Some(i) = pick {
                    self.select_style(i);
                }
                if let Some(a) = menu_action {
                    self.handle_style_action(a);
                }
                if let Some(request) = popup_request {
                    // Clicking the same "…" again closes its menu.
                    self.style_popup = match self.style_popup {
                        Some((open, _)) if open == request.0 => None,
                        _ => Some(request),
                    };
                }
                if import {
                    self.import_styles();
                }

                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.heading("Workflow");
                });
                ui.horizontal(|ui| {
                    for (view, label, help) in workflow::View::ALL {
                        if ui
                            .add(egui::Button::selectable(self.view == view, label))
                            .on_hover_text(help)
                            .clicked()
                        {
                            self.view = view;
                        }
                    }
                });
                let before = self.draft.clone();
                match self.view {
                    workflow::View::Detailed => {
                        workflow::detailed(ui, &mut self.draft, self.summary.as_ref());
                    }
                    workflow::View::Simple => {
                        workflow::simple(ui, &mut self.draft, self.summary.as_ref());
                    }
                    workflow::View::Photographers => {
                        if let Some(i) = workflow::photographers(ui, &self.draft, &self.presets) {
                            self.apply_preset(i);
                        }
                    }
                }
                if self.draft != before {
                    self.draft_changed();
                }

                ui.add_space(6.0);
                ui.separator();
                let builtin = self.store.entries[self.current].builtin;
                let modified = self.is_modified();
                ui.horizontal_wrapped(|ui| {
                    if ui.button("Save as new style").clicked() {
                        self.save_as_new();
                    }
                    if ui
                        .add_enabled(!builtin && modified, egui::Button::new("Save changes"))
                        .on_disabled_hover_text(if builtin {
                            "Built-in styles are read-only"
                        } else {
                            "No changes"
                        })
                        .clicked()
                    {
                        self.save_changes();
                    }
                    if ui
                        .add_enabled(modified, egui::Button::new("Revert"))
                        .clicked()
                    {
                        self.select_style(self.current);
                    }
                    if ui
                        .add_enabled(!builtin, egui::Button::new("Delete"))
                        .clicked()
                    {
                        self.delete_dialog = Some(self.current);
                    }
                });
            });
    }

    fn gallery(&mut self, ui: &mut egui::Ui) {
        let mut pick = None;
        let mut menu_action = None;
        egui::ScrollArea::horizontal()
            .auto_shrink([false, true])
            .show(ui, |ui| {
                let tile_h = (ui.available_height() - 22.0).clamp(50.0, 260.0);
                let size = Vec2::new(tile_h * 1.5, tile_h);
                ui.horizontal(|ui| {
                    for (i, entry) in self.store.entries.iter().enumerate() {
                        ui.vertical(|ui| {
                            ui.set_width(size.x);
                            let response = match self.tiles.get(i) {
                                Some(Tile::Ready(tex)) => ui.add(
                                    egui::Image::new(tex)
                                        .fit_to_exact_size(size)
                                        .maintain_aspect_ratio(true)
                                        .sense(Sense::click()),
                                ),
                                Some(Tile::Unavailable) => {
                                    let (rect, response) =
                                        ui.allocate_exact_size(size, Sense::click());
                                    ui.put(
                                        rect,
                                        egui::Label::new(
                                            egui::RichText::new("No published time").small().weak(),
                                        ),
                                    );
                                    response.on_hover_text(
                                        "This film and developer have no published development time, so the style can't be rendered.",
                                    )
                                }
                                _ => {
                                    let (rect, response) =
                                        ui.allocate_exact_size(size, Sense::click());
                                    if self.loaded.is_some() {
                                        ui.put(rect, egui::Spinner::new());
                                    }
                                    response
                                }
                            };
                            let text = egui::RichText::new(&entry.style.name).small();
                            let text = if i == self.current {
                                text.strong()
                            } else {
                                text
                            };
                            ui.add(egui::Label::new(text).truncate());
                            if i == self.current {
                                ui.painter().rect_stroke(
                                    response.rect.expand(2.0),
                                    2.0,
                                    Stroke::new(2.0, ui.visuals().selection.bg_fill),
                                    egui::StrokeKind::Outside,
                                );
                            }
                            let response = response.on_hover_text(format!(
                                "{} (right-click for options)",
                                entry.style.name
                            ));
                            if response.clicked() {
                                pick = Some(i);
                            }
                            let (up, down) = (self.store.can_move(i, -1), self.store.can_move(i, 1));
                            response.context_menu(|ui| {
                                if let Some(a) = style_menu::items(ui, i, entry.builtin, up, down) {
                                    menu_action = Some(a);
                                }
                            });
                        });
                    }
                });
            });
        if let Some(i) = pick {
            self.select_style(i);
        }
        if let Some(a) = menu_action {
            self.handle_style_action(a);
        }
    }

    fn central(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.show_original, "Show original (O)");
            if ui
                .checkbox(&mut self.detail_enabled, "Detail 1:1 (D)")
                .on_hover_text("Click the image to inspect grain and edges at full resolution")
                .changed()
                && self.detail_enabled
            {
                self.request_detail();
            }
            if let Some(ms) = self.preview_ms {
                ui.label(
                    egui::RichText::new(format!("preview {ms} ms"))
                        .weak()
                        .small(),
                );
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(egui::RichText::new(&self.status).small());
            });
        });

        if self.summary.is_none() && self.loaded.is_some() && !self.show_original {
            ui.colored_label(
                ui.visuals().warn_fg_color,
                "This film and developer have no published development time. Choose another developer.",
            );
        }
        let texture = if self.show_original {
            self.original.as_ref()
        } else {
            self.preview.as_ref().or(self.original.as_ref())
        };
        let Some(texture) = texture.cloned() else {
            ui.centered_and_justified(|ui| {
                if self.loading {
                    ui.spinner();
                } else {
                    ui.label(
                        egui::RichText::new("Choose an image from the folder on the left").weak(),
                    );
                }
            });
            return;
        };

        let available = ui.available_size();
        // Render the preview at the panel's real pixel size, so grain is shown as it
        // will look at this viewing size (rounded to avoid re-rendering on tiny resizes).
        let wanted = ((available.x.max(available.y) * ui.ctx().pixels_per_point()) as usize)
            .div_ceil(256)
            * 256;
        let wanted = wanted.clamp(768, 3072);
        if wanted != self.preview_edge {
            self.preview_edge = wanted;
            self.request_preview();
        }
        let tex_size = texture.size_vec2();
        let scale = (available.x / tex_size.x).min(available.y / tex_size.y);
        // Lay the image out explicitly: clicks and the detail outline are measured against
        // the rectangle the image is actually drawn in, not the whole panel.
        let (area_rect, response) = ui.allocate_exact_size(available, Sense::click());
        let image_rect = egui::Rect::from_center_size(area_rect.center(), tex_size * scale);
        egui::Image::new(&texture).paint_at(ui, image_rect);
        if self.detail_enabled
            && response.clicked()
            && let Some(pos) = response.interact_pointer_pos()
            && image_rect.contains(pos)
        {
            self.detail_centre = [
                ((pos.x - image_rect.min.x) / image_rect.width()).clamp(0.0, 1.0),
                ((pos.y - image_rect.min.y) / image_rect.height()).clamp(0.0, 1.0),
            ];
            self.request_detail();
        }
        if self.detail_enabled
            && let Some([x0, y0, x1, y1]) = self.detail_area
        {
            // Outline of exactly what the detail window shows (moved inwards at edges).
            let at = |fx: f32, fy: f32| {
                Pos2::new(
                    image_rect.min.x + fx * image_rect.width(),
                    image_rect.min.y + fy * image_rect.height(),
                )
            };
            let outline = egui::Rect::from_min_max(at(x0, y0), at(x1, y1));
            let painter = ui.painter_at(image_rect);
            painter.rect_stroke(
                outline,
                0.0,
                Stroke::new(2.5, Color32::from_black_alpha(140)),
                egui::StrokeKind::Outside,
            );
            painter.rect_stroke(
                outline,
                0.0,
                Stroke::new(1.5, Color32::from_rgb(230, 120, 60)),
                egui::StrokeKind::Outside,
            );
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = &ui.ctx().clone();
        if (ctx.pixels_per_point() - self.pixels_per_point).abs() > 0.01 {
            // Moved to a display with a different pixel density: keep 100 % exact.
            self.pixels_per_point = ctx.pixels_per_point();
            if self.detail_enabled {
                self.request_detail();
            }
        }
        self.handle_messages(ctx);

        if !ctx.egui_wants_keyboard_input() {
            let (right, left, o, d) = ctx.input(|i| {
                if i.key_pressed(egui::Key::B) {
                    self.show_browser = !self.show_browser;
                }
                if i.key_pressed(egui::Key::W) {
                    self.show_styles = !self.show_styles;
                }
                if i.key_pressed(egui::Key::G) {
                    self.show_gallery = !self.show_gallery;
                }
                if i.key_pressed(egui::Key::F) {
                    // Focus mode: hide everything, or bring everything back.
                    let any = self.show_browser || self.show_styles || self.show_gallery;
                    self.show_browser = !any;
                    self.show_styles = !any;
                    self.show_gallery = !any;
                }
                (
                    i.key_pressed(egui::Key::ArrowRight) || i.key_pressed(egui::Key::ArrowDown),
                    i.key_pressed(egui::Key::ArrowLeft) || i.key_pressed(egui::Key::ArrowUp),
                    i.key_pressed(egui::Key::O),
                    i.key_pressed(egui::Key::D),
                )
            });
            if right {
                self.step_image(1);
            }
            if left {
                self.step_image(-1);
            }
            if o {
                self.show_original = !self.show_original;
            }
            if d {
                self.detail_enabled = !self.detail_enabled;
                if self.detail_enabled {
                    self.request_detail();
                }
            }
        }

        egui::Panel::top("top").show(ui, |ui| self.top_bar(ui));
        let mut show_browser = self.show_browser;
        let panel_frame =
            egui::Frame::side_top_panel(ui.style()).inner_margin(egui::Margin::symmetric(12, 8));
        // New id: the compact list starts at its narrower default instead of a width
        // saved for the old thumbnail list.
        egui::Panel::left("files")
            .frame(panel_frame)
            .resizable(true)
            .default_size(170.0)
            .size_range(120.0..=360.0)
            .show_collapsible(ui, &mut show_browser, |ui| self.browser_panel(ui));
        self.show_browser = show_browser;
        let mut show_styles = self.show_styles;
        egui::Panel::right("styles")
            .frame(panel_frame)
            .resizable(true)
            .default_size(300.0)
            .size_range(250.0..=480.0)
            .show_collapsible(ui, &mut show_styles, |ui| self.style_panel(ui));
        self.show_styles = show_styles;
        let mut show_gallery = self.show_gallery;
        egui::Panel::bottom("gallery")
            .resizable(true)
            .default_size(120.0)
            .size_range(80.0..=300.0)
            .show_collapsible(ui, &mut show_gallery, |ui| self.gallery(ui));
        self.show_gallery = show_gallery;
        egui::CentralPanel::default().show(ui, |ui| self.central(ui));

        if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::E))
            && self.loaded.is_some()
        {
            self.export_open = true;
        }
        if self.export_open {
            self.export_dialog(ctx);
        }
        #[cfg(target_os = "macos")]
        if self.menu.as_ref().is_some_and(|m| m.take_about_request()) {
            self.about.open = true;
        }
        self.about.show(ctx);
        self.style_popup(ctx);
        self.style_dialogs(ctx);
        self.screenshot_hook(ctx);

        if self.detail_enabled && self.loaded.is_some() {
            let mut open = true;
            let mut zoom = self.detail_zoom;
            let ppp = self.pixels_per_point;
            egui::Window::new("Detail")
                .open(&mut open)
                .resizable(false)
                // Open over the right-hand panel so the outlined area on the image stays visible.
                .default_pos([ctx.content_rect().right() - DETAIL_VIEW_PT - 40.0, 60.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        for (z, label, help) in [
                            (1u8, "100%", "One image pixel per screen pixel: the export at full size"),
                            (2u8, "200%", "Magnified two times: each image pixel is a 2 × 2 block, for judging grain"),
                        ] {
                            ui.selectable_value(&mut zoom, z, label).on_hover_text(help);
                        }
                        ui.label(
                            egui::RichText::new("Click the image to move the outlined area").small().weak(),
                        );
                    });
                    match &self.detail {
                        Some(tex) => {
                            // Texture pixels → points so the zoom is exact on any display.
                            let size = tex.size_vec2() * f32::from(self.detail_zoom) / ppp;
                            ui.add(egui::Image::new(tex).fit_to_exact_size(size));
                        }
                        None => {
                            ui.allocate_ui(egui::vec2(DETAIL_VIEW_PT, DETAIL_VIEW_PT), |ui| ui.spinner());
                        }
                    }
                });
            if zoom != self.detail_zoom {
                self.detail_zoom = zoom;
                self.request_detail();
            }
            if !open {
                self.detail_enabled = false;
            }
        }
    }

    /// Screenshot test runs must not overwrite the user's saved panel sizes and window
    /// positions (egui's own memory is persisted separately from `save`).
    fn persist_egui_memory(&self) -> bool {
        self.screenshot.is_none()
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        if self.screenshot.is_some() {
            return; // test runs must not change the user's saved state
        }
        self.save_adjust();
        storage.set_string(KEY_FOLDER, self.folder.display().to_string());
        storage.set_string(KEY_VIEW, self.view.key().into());
        let theme = match self.theme {
            egui::ThemePreference::Light => "light",
            egui::ThemePreference::Dark => "dark",
            egui::ThemePreference::System => "system",
        };
        storage.set_string(KEY_THEME, theme.into());
        storage.set_string(
            KEY_BROWSER_THUMBS,
            if self.browser_thumbs { "1" } else { "0" }.into(),
        );
        let flag_str = |b: bool| if b { "1" } else { "0" }.to_string();
        storage.set_string(KEY_BROWSER_BITMAPS, flag_str(self.browser_filter.bitmaps));
        storage.set_string(KEY_BROWSER_JPEGS, flag_str(self.browser_filter.jpegs));
        let flag = |b: bool| if b { '1' } else { '0' };
        storage.set_string(
            KEY_PANELS,
            [self.show_browser, self.show_styles, self.show_gallery]
                .map(flag)
                .iter()
                .collect(),
        );
        if let Ok(json) = serde_json::to_string(&self.export) {
            storage.set_string(KEY_EXPORT, json);
        }
    }
}

// ----- widgets -----------------------------------------------------------------

fn set_texture(
    ctx: &egui::Context,
    slot: &mut Option<TextureHandle>,
    name: &str,
    image: egui::ColorImage,
) {
    match slot {
        Some(tex) => tex.set(image, TextureOptions::LINEAR),
        None => *slot = Some(ctx.load_texture(name, image, TextureOptions::LINEAR)),
    }
}

fn file_name(path: &std::path::Path) -> String {
    path.file_name().map_or_else(
        || path.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    )
}
