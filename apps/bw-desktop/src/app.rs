//! Application state and UI.
//!
//! Layout: folder tree (left), preview (centre), the Develop or Styles panel (right)
//! chosen from an icon strip at the far right, and two strips under the photo: the style
//! gallery showing the current image in every style, and the images of the chosen folder. The panel design follows LightCraft's (see `theme` and
//! `widgets`).

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
use crate::theme::Tokens;
use crate::updates::{self, CheckResult, Release};
use crate::widgets::{self, Icon, inset};
use crate::worker::{self, Job, Msg};
use crate::workflow;
use bw_raw::RawAdjust;

const KEY_FOLDER: &str = "folder";
const KEY_MODULE: &str = "right_module";
const KEY_THEME: &str = "theme";
const KEY_BROWSER_BITMAPS: &str = "browser_bitmaps";
const KEY_BROWSER_JPEGS: &str = "browser_jpegs";
const KEY_UPDATE_AUTO: &str = "update_auto";
const KEY_UPDATE_LAST: &str = "update_last_check";
const KEY_UPDATE_SKIPPED: &str = "update_skipped";

/// Release notes from the changelog (Markdown): headings in bold, bullets as "•", and
/// emphasis and code markers dropped.
fn release_notes(ui: &mut egui::Ui, notes: &str) {
    for line in notes.lines() {
        let plain = line.replace("**", "").replace('`', "");
        let trimmed = plain.trim_start();
        if trimmed.is_empty() {
            ui.add_space(4.0);
        } else if let Some(heading) = trimmed.strip_prefix('#') {
            ui.label(egui::RichText::new(heading.trim_start_matches('#').trim()).strong());
        } else if let Some(item) = trimmed.strip_prefix("- ") {
            let indent = (plain.len() - trimmed.len()) as f32 * 4.0;
            ui.horizontal_wrapped(|ui| {
                ui.add_space(indent);
                ui.label(format!("•  {item}"));
            });
        } else {
            ui.label(trimmed);
        }
    }
}

/// A scripted click or drag for screenshot tests: move to the start, press, (for a drag)
/// move to the end in steps, release, one event per frame.
struct ScriptedPointer {
    at_s: f64,
    start: Pos2,
    end: Option<Pos2>,
    stage: u8,
}

impl ScriptedPointer {
    const DRAG_STEPS: u8 = 12;

    /// "t:x,y" (click) or "t:x,y>x2,y2" (drag).
    fn parse(spec: &str) -> Option<Self> {
        let point = |s: &str| {
            let (x, y) = s.split_once(',')?;
            Some(egui::pos2(x.trim().parse().ok()?, y.trim().parse().ok()?))
        };
        let (t, rest) = spec.split_once(':')?;
        let (start, end) = match rest.split_once('>') {
            Some((a, b)) => (point(a)?, Some(point(b)?)),
            None => (point(rest)?, None),
        };
        Some(Self {
            at_s: t.trim().parse().ok()?,
            start,
            end,
            stage: 0,
        })
    }

    fn last_stage(&self) -> u8 {
        if self.end.is_some() {
            2 + Self::DRAG_STEPS
        } else {
            2
        }
    }

    fn done(&self) -> bool {
        self.stage > self.last_stage()
    }

    fn next_event(&mut self) -> egui::Event {
        let stage = self.stage;
        self.stage += 1;
        let button = |pos, pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: Default::default(),
        };
        let end = self.end.unwrap_or(self.start);
        match stage {
            0 => egui::Event::PointerMoved(self.start),
            1 => button(self.start, true),
            s if s == self.last_stage() => button(end, false),
            s => {
                let t = f32::from(s - 1) / f32::from(Self::DRAG_STEPS);
                egui::Event::PointerMoved(self.start + (end - self.start) * t)
            }
        }
    }
}

/// The panel the tool strip shows on the right.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Module {
    Develop,
    Styles,
}

impl Module {
    fn key(self) -> &'static str {
        match self {
            Module::Develop => "develop",
            Module::Styles => "styles",
        }
    }

    fn from_key(key: &str) -> Module {
        if key == "styles" {
            Module::Styles
        } else {
            Module::Develop
        }
    }
}

/// Which group a style is listed under.
#[derive(Clone, Copy, PartialEq, Eq)]
enum StyleGroup {
    Films,
    Photographers,
    Mine,
}

impl StyleGroup {
    const ALL: [(StyleGroup, &'static str, &'static str); 3] = [
        (StyleGroup::Films, "styles.films", "Films & processes"),
        (
            StyleGroup::Photographers,
            "styles.photographers",
            "Photographers",
        ),
        (StyleGroup::Mine, "styles.mine", "My styles"),
    ];

    fn of(entry: &crate::styles::StyleEntry) -> StyleGroup {
        if !entry.builtin {
            StyleGroup::Mine
        } else if entry.style.name.starts_with("Inspired by") {
            StyleGroup::Photographers
        } else {
            StyleGroup::Films
        }
    }
}

/// What the update dialog is showing.
enum UpdateDialog {
    Checking,
    Result(CheckResult),
}
const KEY_EXPORT_DIR: &str = "export_dir";
const KEY_EXPORT: &str = "export_settings";
const KEY_PANELS: &str = "panels";
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

    /// The folder being browsed, and its images (the image strip).
    folder: PathBuf,
    entries: Vec<Entry>,
    thumbs: HashMap<PathBuf, TextureHandle>,
    /// Subfolder listings for the folder tree.
    folder_cache: browser::FolderCache,
    /// The folder tree's top-level folders, and when they were last listed.
    places: Vec<browser::Place>,
    places_listed: std::time::Instant,
    /// Scroll the image strip to the selected image on the next frame.
    reveal_in_strip: bool,

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
    /// Which panel the tool strip shows on the right.
    module: Module,
    /// Open every Develop section on the next frame ("Edit…" from a style's menu).
    open_all_sections: bool,
    /// Which non-raw files the image strip lists (raw files always are).
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
    show_filmstrip: bool,

    detail_enabled: bool,
    detail_centre: [f32; 2],
    /// Detail magnification: 1 = 100 % (one image pixel per screen pixel), 2 = 200 %.
    detail_zoom: u8,
    /// Area of the image the detail window shows, as fractions `[x0, y0, x1, y1]`.
    detail_area: Option<[f32; 4]>,
    /// Screen pixels per point (2 on Retina displays), updated every frame.
    pixels_per_point: f32,
    detail_generation: u64,
    /// Zoom of the latest detail request, and of the detail texture once it arrives:
    /// the texture is drawn at the zoom it was rendered for, never the newly chosen one.
    detail_requested_zoom: u8,
    detail_texture_zoom: u8,
    /// Centre of the latest detail request, and of the texture on screen: while a drag
    /// waits for a new render, the texture slides by the difference.
    detail_requested_centre: [f32; 2],
    detail_texture_centre: [f32; 2],
    /// Generation of the detail texture on screen; behind `detail_generation` while a
    /// newer render is on its way.
    detail_shown_generation: u64,
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
    about: crate::about::About,
    theme: egui::ThemePreference,
    #[cfg(target_os = "macos")]
    menu: Option<crate::menu::AppMenu>,

    /// Test hook: `BW_DARKROOM_SCREENSHOT=<png>` saves one screenshot of the window once
    /// the image, preview and gallery have rendered, then quits.
    screenshot: Option<(PathBuf, std::time::Instant, bool)>,
    /// Screenshot runs: whether `BW_DARKROOM_SCREENSHOT_THEN_PUSH` has been applied.
    screenshot_then_applied: bool,
    /// Screenshot runs: clicks still to inject, as (seconds after start, x, y, stage).
    screenshot_clicks: Vec<ScriptedPointer>,

    /// Check GitHub for a newer release once a day.
    update_auto: bool,
    /// When the last check succeeded, in seconds since the Unix epoch.
    update_last_check: u64,
    /// A check in flight.
    update_rx: Option<Receiver<CheckResult>>,
    /// A newer release, once a check has found one.
    update_available: Option<Release>,
    /// A version the user chose to skip: the top-bar notice stays hidden for it.
    update_skipped: Option<String>,
    update_dialog: Option<UpdateDialog>,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, start: Option<PathBuf>) -> Self {
        let ctx = cc.egui_ctx.clone();
        let (jobs, job_rx) = mpsc::channel();
        let (msg_tx, msgs) = mpsc::channel();
        let (thumb_tx, thumb_rx) = mpsc::channel();
        worker::spawn_renderer(job_rx, msg_tx.clone(), ctx.clone());
        browser::spawn_thumbnailer(thumb_rx, msg_tx.clone(), ctx.clone());

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
        let module = Module::from_key(get(KEY_MODULE).as_deref().unwrap_or(""));
        let export = get(KEY_EXPORT)
            .and_then(|s| serde_json::from_str::<ExportSettings>(&s).ok())
            .unwrap_or_else(|| ExportSettings {
                // carry over the folder chosen before the export dialog existed
                folder: get(KEY_EXPORT_DIR).map(PathBuf::from),
                ..Default::default()
            });
        // Panel visibility as 0/1 flags: folders, styles, gallery, image strip (older
        // versions saved the first three).
        let mut panels: Vec<bool> = get(KEY_PANELS)
            .map(|s| s.chars().map(|c| c == '1').collect())
            .filter(|v: &Vec<bool>| v.len() == 3 || v.len() == 4)
            .unwrap_or_default();
        panels.resize(4, true);

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
            folder_cache: browser::FolderCache::new(ctx),
            places: browser::places(),
            places_listed: std::time::Instant::now(),
            reveal_in_strip: false,
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
            module,
            open_all_sections: false,
            update_auto: get(KEY_UPDATE_AUTO).is_none_or(|v| v != "0"),
            update_last_check: get(KEY_UPDATE_LAST)
                .and_then(|v| v.parse().ok())
                .unwrap_or(0),
            update_rx: None,
            update_available: None,
            update_skipped: get(KEY_UPDATE_SKIPPED).filter(|v| !v.is_empty()),
            update_dialog: None,
            browser_filter: browser::Filter {
                bitmaps: get(KEY_BROWSER_BITMAPS).is_some_and(|v| v == "1"),
                jpegs: get(KEY_BROWSER_JPEGS).is_some_and(|v| v == "1"),
            },
            presets: bw_profiles::photographer_presets(),
            rename_dialog: None,
            delete_dialog: None,
            style_popup: None,
            preview_edge: 1600,
            preview_generation: 0,
            preview: None,
            preview_ms: None,
            show_original: false,
            show_browser: panels[0],
            show_styles: panels[1],
            show_gallery: panels[2],
            show_filmstrip: panels[3],
            detail_enabled: false,
            detail_centre: [0.5, 0.5],
            detail_zoom: 1,
            detail_area: None,
            pixels_per_point: cc.egui_ctx.pixels_per_point(),
            detail_generation: 0,
            detail_requested_zoom: 1,
            detail_texture_zoom: 1,
            detail_requested_centre: [0.5, 0.5],
            detail_texture_centre: [0.5, 0.5],
            detail_shown_generation: 0,
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
            screenshot_then_applied: false,
            // `BW_DARKROOM_SCREENSHOT_CLICKS=t:x,y;t:x,y>x2,y2` clicks at window points
            // (x, y), or drags from (x, y) to (x2, y2), t seconds after start, through
            // egui's own input: real interaction in tests.
            screenshot_clicks: std::env::var("BW_DARKROOM_SCREENSHOT_CLICKS")
                .ok()
                .map(|v| v.split(';').filter_map(ScriptedPointer::parse).collect())
                .unwrap_or_default(),
            screenshot: std::env::var_os("BW_DARKROOM_SCREENSHOT")
                .map(|p| (PathBuf::from(p), std::time::Instant::now(), false)),
        };
        if std::env::var_os("BW_DARKROOM_SCREENSHOT_FRESH").is_some() {
            // Test runs can start from default panel sizes, ignoring saved layout.
            cc.egui_ctx.memory_mut(|m| *m = egui::Memory::default());
        }
        crate::theme::install(&cc.egui_ctx);
        cc.egui_ctx.set_theme(app.theme);
        app.open_folder(folder);
        if let Some(image) = start_image {
            app.select_image(image);
        }
        // Test runs never touch the network.
        if app.update_auto
            && app.screenshot.is_none()
            && updates::now_s().saturating_sub(app.update_last_check) >= updates::CHECK_INTERVAL_S
        {
            app.update_rx = Some(updates::spawn_check(&cc.egui_ctx));
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
            .filter(|e| !self.thumbs.contains_key(&e.path))
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
        self.reveal_in_strip = true;
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
            .filter(|e| self.browser_filter.shows(e.kind))
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
        self.detail_requested_zoom = self.detail_zoom;
        self.detail_requested_centre = self.detail_centre;
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
                self.module = Module::Develop;
                self.show_styles = true;
                self.open_all_sections = true;
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
        // A press away from the menu (and not on the "…" that opened it) closes it. The
        // container test counts the menu's own items: plain `contains_pointer` is false
        // over a child widget, which closed the menu on the press before an item's click.
        let clicked_elsewhere = ctx.input(|i| i.pointer.any_pressed())
            && !area.response.container_contains_pointer()
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
    }

    /// A check the user asked for: show its progress and result in the update dialog.
    fn check_for_updates(&mut self, ctx: &egui::Context) {
        self.update_dialog = Some(UpdateDialog::Checking);
        if self.update_rx.is_none() {
            self.update_rx = Some(updates::spawn_check(ctx));
        }
    }

    fn poll_update_check(&mut self) {
        let Some(result) = self.update_rx.as_ref().and_then(|rx| rx.try_recv().ok()) else {
            return;
        };
        self.update_rx = None;
        // A failed check is retried at the next start rather than a day later.
        if !matches!(result, CheckResult::Failed(_)) {
            self.update_last_check = updates::now_s();
        }
        if let CheckResult::Available(release) = &result {
            self.update_available = Some(release.clone());
        }
        if matches!(self.update_dialog, Some(UpdateDialog::Checking)) {
            self.update_dialog = Some(UpdateDialog::Result(result));
        }
    }

    /// The Software Update window: progress, result, release notes and the setting.
    fn update_window(&mut self, ctx: &egui::Context) {
        let Some(dialog) = &self.update_dialog else {
            return;
        };
        let current = crate::version::VERSION;
        let mut open = true;
        let mut close = false;
        let mut skip = None;
        let mut download = None;
        egui::Window::new("Software Update")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.set_width(420.0);
                match dialog {
                    UpdateDialog::Checking => {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label("Checking for updates…");
                        });
                    }
                    UpdateDialog::Result(CheckResult::UpToDate) => {
                        ui.label(egui::RichText::new("You're up to date").strong());
                        ui.label(format!("BW Darkroom {current} is the latest version."));
                    }
                    UpdateDialog::Result(CheckResult::Failed(error)) => {
                        ui.label(egui::RichText::new("Couldn't check for updates").strong());
                        ui.label(format!("{error}. Try again later."));
                    }
                    UpdateDialog::Result(CheckResult::Available(release)) => {
                        ui.label(
                            egui::RichText::new(format!(
                                "BW Darkroom {} is available",
                                release.version
                            ))
                            .heading(),
                        );
                        ui.label(format!(
                            "You have {current}. Download the new version, quit BW Darkroom and \
                             replace it in Applications; your styles and settings are kept."
                        ));
                        if !release.notes.is_empty() {
                            ui.add_space(4.0);
                            ui.label(egui::RichText::new("What's new").strong());
                            egui::ScrollArea::vertical()
                                .max_height(220.0)
                                .show(ui, |ui| {
                                    release_notes(ui, &release.notes);
                                });
                        }
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            if ui
                                .button("Download…")
                                .on_hover_text(
                                    "Open the release page, with the installer, in your browser",
                                )
                                .clicked()
                            {
                                download = Some(release.url.clone());
                            }
                            if ui.button("Later").clicked() {
                                close = true;
                            }
                            if ui
                                .button("Skip This Version")
                                .on_hover_text("Stop showing the update notice for this version")
                                .clicked()
                            {
                                skip = Some(release.version.clone());
                            }
                        });
                    }
                }
                ui.separator();
                ui.checkbox(
                    &mut self.update_auto,
                    "Check for updates automatically (once a day)",
                )
                .on_hover_text(
                    "Asks GitHub for the latest release when the app starts. Nothing is sent \
                         except the request itself.",
                );
            });
        if let Some(url) = download {
            if let Err(e) = updates::open_in_browser(&url) {
                self.status = format!("Could not open {url}: {e}");
            }
            close = true;
        }
        if let Some(version) = skip {
            self.update_skipped = Some(version);
            close = true;
        }
        if close || !open {
            self.update_dialog = None;
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
                    self.detail_texture_zoom = self.detail_requested_zoom;
                    self.detail_texture_centre = self.detail_requested_centre;
                    self.detail_shown_generation = generation;
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
        // `BW_DARKROOM_SCREENSHOT_GRAIN=amount,size,roughness` sets the style's grain.
        if let Ok(spec) = std::env::var("BW_DARKROOM_SCREENSHOT_GRAIN")
            && let [amount, size, roughness] = spec
                .split(',')
                .filter_map(|p| p.trim().parse().ok())
                .collect::<Vec<f64>>()[..]
        {
            let grain = bw_profiles::GrainSpec {
                amount,
                size,
                roughness,
            };
            if self.draft.grain != grain {
                self.draft.grain = grain;
                self.draft_changed();
            }
        }
        // `BW_DARKROOM_SCREENSHOT_UPDATE=<version>` fakes a found update (no network);
        // `…_UPDATE_DIALOG=1` also opens its dialog.
        if let Ok(version) = std::env::var("BW_DARKROOM_SCREENSHOT_UPDATE")
            && self.update_available.is_none()
        {
            let release = Release {
                version,
                url: updates::RELEASES_URL.into(),
                notes:
                    "### Added\n- **Grain controls**: size, amount and roughness.\n\n### Fixed\n- \
                        The style menu's items can be clicked again."
                        .into(),
            };
            if std::env::var_os("BW_DARKROOM_SCREENSHOT_UPDATE_DIALOG").is_some() {
                self.update_dialog = Some(UpdateDialog::Result(CheckResult::Available(
                    release.clone(),
                )));
            }
            self.update_available = Some(release);
        }
        // `BW_DARKROOM_SCREENSHOT_THEN_PUSH=<stops>`: once the detail has arrived, change
        // the push as the workflow panel would, and shoot after the detail re-renders.
        if let Ok(stops) = std::env::var("BW_DARKROOM_SCREENSHOT_THEN_PUSH")
            && let Ok(stops) = stops.parse::<f64>()
            && !self.screenshot_then_applied
            && self.detail_area.is_some()
        {
            self.screenshot_then_applied = true;
            self.draft.set_push_stops(stops);
            self.draft_changed();
            self.detail_area = None;
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
        // `…_VIEW=develop|styles` picks the right-hand panel.
        if let Some(view) = std::env::var_os("BW_DARKROOM_SCREENSHOT_VIEW") {
            self.module = Module::from_key(&view.to_string_lossy());
        }
        // `BW_DARKROOM_SCREENSHOT_SHOW=bitmaps,jpegs` (or empty) sets the image strip's filter.
        if let Some(v) = std::env::var_os("BW_DARKROOM_SCREENSHOT_SHOW") {
            let v = v.to_string_lossy();
            self.browser_filter = browser::Filter {
                bitmaps: v.contains("bitmaps"),
                jpegs: v.contains("jpegs"),
            };
        }
        // `BW_DARKROOM_SCREENSHOT_CLOSE=image,film` closes sections (after `…_EDIT`).
        let close = std::env::var("BW_DARKROOM_SCREENSHOT_CLOSE").ok();
        // `BW_DARKROOM_SCREENSHOT_EDIT=1` opens every Develop section, as "Edit…" does.
        if std::env::var_os("BW_DARKROOM_SCREENSHOT_EDIT").is_some()
            && !self.screenshot_then_applied
        {
            self.module = Module::Develop;
            widgets::open_all(ctx, &workflow::SECTION_KEYS);
        }
        if let Some(close) = &close {
            let keys: Vec<&str> = close.split(',').collect();
            widgets::set_open(ctx, &keys, false);
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
        // `BW_DARKROOM_SCREENSHOT_WAIT=<seconds>` holds the shot, e.g. for scripted clicks.
        let wait = std::env::var("BW_DARKROOM_SCREENSHOT_WAIT")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(0);
        let rendered = rendered && started.elapsed().as_secs() >= wait;
        if !*requested && (rendered || started.elapsed().as_secs() > 30.max(wait + 5)) {
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
            ui.toggle_value(&mut self.show_browser, "Folders")
                .on_hover_text("Show or hide the folder tree (B)");
            ui.toggle_value(&mut self.show_styles, "Workflow")
                .on_hover_text("Show or hide the styles and workflow panel (W)");
            ui.toggle_value(&mut self.show_gallery, "Gallery")
                .on_hover_text("Show or hide the style gallery: this photo in every style (G)");
            ui.toggle_value(&mut self.show_filmstrip, "Images")
                .on_hover_text("Show or hide the strip of photos in the chosen folder (I)");
            ui.separator();
            if ui
                .button("Open folder…")
                .on_hover_text("Choose a folder; its photos appear in the strip below the image")
                .clicked()
            {
                self.pick_folder();
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
                if let Some(release) = &self.update_available
                    && self.update_skipped.as_deref() != Some(release.version.as_str())
                {
                    ui.separator();
                    let notice = egui::RichText::new(format!("Update: {}", release.version))
                        .color(ui.visuals().hyperlink_color);
                    if ui
                        .button(notice)
                        .on_hover_text(format!(
                            "BW Darkroom {} is available (you have {}). Click for details.",
                            release.version,
                            crate::version::VERSION
                        ))
                        .clicked()
                    {
                        self.update_dialog = Some(UpdateDialog::Result(CheckResult::Available(
                            release.clone(),
                        )));
                    }
                }
            });
        });
    }

    /// The folder tree. Choosing a folder fills the image strip; the tree never lists
    /// files, so finding a folder and choosing a photo stay separate.
    fn browser_panel(&mut self, ui: &mut egui::Ui) {
        // Drives come and go (memory cards): list the places again now and then.
        if self.places_listed.elapsed() >= std::time::Duration::from_secs(5) {
            self.places = browser::places();
            self.places_listed = std::time::Instant::now();
        }
        match crate::folders::show(ui, &self.folder_cache, &self.places, &self.folder) {
            Some(crate::folders::Action::Browse(folder)) => {
                if folder != self.folder {
                    self.open_folder(folder);
                }
            }
            Some(crate::folders::Action::PickFolder) => self.pick_folder(),
            None => {}
        }
    }

    fn pick_folder(&mut self) {
        if let Some(folder) = rfd::FileDialog::new()
            .set_directory(&self.folder)
            .pick_folder()
        {
            self.open_folder(folder);
        }
    }

    /// The image strip: the chosen folder's photos as thumbnails, scrolling sideways.
    /// Clicking one opens it in the main view. The header names the folder, counts the
    /// photos and sets which file types are listed.
    fn filmstrip(&mut self, ui: &mut egui::Ui) {
        let filter = self.browser_filter;
        let images = self.entries.iter().filter(|e| filter.shows(e.kind)).count();
        let hidden = self.entries.len() - images;
        let plural =
            |n: usize, one: &str, many: &str| format!("{n} {}", if n == 1 { one } else { many });
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            ui.add(
                egui::Label::new(egui::RichText::new(file_name(&self.folder)).strong()).truncate(),
            )
            .on_hover_text(self.folder.display().to_string());
            let mut count = plural(images, "photo", "photos");
            if hidden > 0 {
                count.push_str(&format!(" · {hidden} hidden"));
            }
            ui.label(egui::RichText::new(count).small().weak());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // File types: raw is fixed on (the app is for raw files); the others are opt-in.
                ui.toggle_value(
                    &mut self.browser_filter.jpegs,
                    egui::RichText::new("JPEG").small(),
                )
                .on_hover_text("Also list compressed JPEG files");
                ui.toggle_value(
                    &mut self.browser_filter.bitmaps,
                    egui::RichText::new("TIFF/PNG").small(),
                )
                .on_hover_text(
                    "Also list lossless bitmaps (TIFF, PNG), such as scans or edited masters",
                );
                ui.add_enabled(
                    false,
                    egui::Button::selectable(true, egui::RichText::new("RAW").small()),
                )
                .on_disabled_hover_text("Camera raw files (NEF, DNG, ARW) are always listed");
            });
        });

        let mut select = None;
        let mut show_all = false;
        let reveal = std::mem::take(&mut self.reveal_in_strip);
        egui::ScrollArea::horizontal()
            .auto_shrink([false, true])
            .show(ui, |ui| {
                if images == 0 {
                    ui.horizontal(|ui| {
                        if hidden > 0 {
                            ui.label(
                                egui::RichText::new(format!(
                                    "No raw files here; {} hidden",
                                    plural(hidden, "JPEG/TIFF/PNG file", "JPEG/TIFF/PNG files")
                                ))
                                .weak(),
                            );
                            show_all = ui
                                .small_button("Show them")
                                .on_hover_text("List JPEG, TIFF and PNG files too")
                                .clicked();
                        } else {
                            ui.label(
                                egui::RichText::new(
                                    "No photos in this folder. Choose another on the left.",
                                )
                                .weak(),
                            );
                        }
                    });
                    return;
                }
                let tile_h = (ui.available_height() - 20.0).clamp(40.0, 220.0);
                let size = Vec2::new(tile_h * 1.5, tile_h);
                let ctx = ui.ctx().clone();
                let small = egui::TextStyle::Small.resolve(ui.style());
                let measure = |text: &str| {
                    ctx.fonts_mut(|f| {
                        f.layout_no_wrap(text.to_owned(), small.clone(), Color32::WHITE)
                            .size()
                            .x
                    })
                };
                let accent = Tokens::get(ui.ctx()).accent;
                ui.horizontal(|ui| {
                    for entry in self.entries.iter().filter(|e| filter.shows(e.kind)) {
                        let selected = self.selected.as_ref() == Some(&entry.path);
                        ui.vertical(|ui| {
                            ui.set_width(size.x);
                            let (rect, response) = ui.allocate_exact_size(size, Sense::click());
                            match self.thumbs.get(&entry.path) {
                                Some(tex) => {
                                    // Fit inside the tile, keeping the photo's shape.
                                    let t = tex.size_vec2();
                                    let scale = (size.x / t.x).min(size.y / t.y);
                                    let shown =
                                        egui::Rect::from_center_size(rect.center(), t * scale);
                                    egui::Image::new(tex).paint_at(ui, shown);
                                }
                                None => {
                                    ui.put(rect, egui::Spinner::new());
                                }
                            }
                            if selected {
                                ui.painter().rect_stroke(
                                    rect.expand(2.0),
                                    2.0,
                                    Stroke::new(2.0, accent),
                                    egui::StrokeKind::Outside,
                                );
                                if reveal {
                                    response.scroll_to_me(None);
                                }
                            } else if response.hovered() {
                                ui.painter().rect_stroke(
                                    rect.expand(1.0),
                                    2.0,
                                    Stroke::new(1.0, ui.visuals().weak_text_color()),
                                    egui::StrokeKind::Outside,
                                );
                            }
                            let name = browser::middle_ellipsis(&entry.name, size.x, measure);
                            let text = egui::RichText::new(name).small();
                            ui.add(
                                egui::Label::new(if selected { text.strong() } else { text })
                                    .truncate(),
                            );
                            let response = response.on_hover_text(format!(
                                "{} ({})\nClick to open it in the main view.",
                                entry.name,
                                browser::type_tag(&entry.path)
                            ));
                            if response.clicked() {
                                select = Some(entry.path.clone());
                            }
                        });
                    }
                });
            });
        if show_all {
            self.browser_filter = browser::Filter {
                bitmaps: true,
                jpegs: true,
            };
        }
        if let Some(path) = select {
            self.select_image(path);
        }
    }

    /// The far-right icon strip: Develop and Styles at the top, Export and About at the
    /// bottom. Clicking the active panel's icon hides the panel.
    fn tool_strip(&mut self, ui: &mut egui::Ui) {
        let t = Tokens::get(ui.ctx());
        ui.add_space(6.0);
        ui.vertical_centered(|ui| {
            ui.spacing_mut().item_spacing.y = 6.0;
            for (module, icon, help) in [
                (
                    Module::Develop,
                    Icon::Sliders,
                    "Develop: image, film, development, grain and print (W)",
                ),
                (
                    Module::Styles,
                    Icon::Grid,
                    "Styles: films and processes, photographers and your own",
                ),
            ] {
                let active = self.show_styles && self.module == module;
                if widgets::strip_button(ui, icon, active, true, help).clicked() {
                    if active {
                        self.show_styles = false;
                    } else {
                        self.module = module;
                        self.show_styles = true;
                    }
                }
            }
            let (r, _) = ui.allocate_exact_size(egui::vec2(32.0, 1.0), Sense::hover());
            ui.painter()
                .rect_filled(r, 0.0, t.button_border.gamma_multiply(0.7));
        });
        ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
            ui.add_space(8.0);
            ui.spacing_mut().item_spacing.y = 6.0;
            if widgets::strip_button(ui, Icon::Info, self.about.open, true, "About BW Darkroom")
                .clicked()
            {
                self.about.open = !self.about.open;
            }
            let can_export = self.loaded.is_some();
            if widgets::strip_button(
                ui,
                Icon::Export,
                self.export_open,
                can_export,
                "Export (⌘E)",
            )
            .clicked()
            {
                self.export_open = true;
            }
        });
    }

    /// The Develop panel: capture line, header with save actions, the style picker, then
    /// the Image, Film, Development, Grain and Print sections.
    fn develop_panel(&mut self, ui: &mut egui::Ui) {
        let t = Tokens::get(ui.ctx());
        if std::mem::take(&mut self.open_all_sections) {
            widgets::open_all(ui.ctx(), &workflow::SECTION_KEYS);
        }
        let builtin = self.store.entries[self.current].builtin;
        let modified = self.is_modified();
        let (mut save, mut save_new, mut revert, mut browse) = (false, false, false, false);
        let mut pick: Option<usize> = None;
        let mut adjust_changed = false;
        let source = self.loaded.as_ref().map(|l| l.source.clone());
        egui::ScrollArea::vertical()
            .auto_shrink(false)
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                ui.add_space(8.0);
                if let Some(src) = source.as_deref() {
                    let (line, hover, warning) = crate::image_panel::capture_summary(src);
                    ui.vertical_centered(|ui| {
                        ui.label(
                            egui::RichText::new(line)
                                .font(t.font(11.5))
                                .color(t.text_dim),
                        )
                        .on_hover_text(hover);
                    });
                    if let Some(w) = warning {
                        inset(ui, |ui| {
                            ui.label(egui::RichText::new(w).font(t.font(11.5)).color(t.caution));
                        });
                    }
                }
                ui.add_space(12.0);
                inset(ui, |ui| {
                    ui.label(
                        egui::RichText::new("Develop")
                            .font(t.semibold(15.0))
                            .color(t.text),
                    );
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        let read_only =
                            "Built-in styles are read-only: use Save as… to keep your changes";
                        save = widgets::text_button(ui, "Save", false, !builtin && modified)
                            .on_hover_text(if builtin {
                                read_only
                            } else {
                                "Save the changes to this style"
                            })
                            .clicked();
                        save_new = widgets::text_button(ui, "Save as…", false, true)
                            .on_hover_text("Keep these settings as a new style of your own")
                            .clicked();
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            revert = widgets::text_button(ui, "Revert", false, modified)
                                .on_hover_text("Discard the changes made to this style")
                                .clicked();
                        });
                    });
                });
                ui.add_space(12.0);
                widgets::divider(ui);

                // The style picker, as LightCraft's profile row, with a browse button.
                let name = if modified {
                    format!("{} (edited)", self.draft.name)
                } else {
                    self.draft.name.clone()
                };
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    let w = ui.available_width() - 44.0;
                    ui.allocate_ui(egui::vec2(w, 34.0), |ui| {
                        widgets::dropdown(ui, "style", "Style", &name, "Choose a style", |ui| {
                            for (group, _, title) in StyleGroup::ALL {
                                ui.label(
                                    egui::RichText::new(title)
                                        .font(t.semibold(11.5))
                                        .color(t.text_dim),
                                );
                                for (i, entry) in self.store.entries.iter().enumerate() {
                                    if StyleGroup::of(entry) == group
                                        && ui
                                            .add(egui::Button::selectable(
                                                i == self.current,
                                                &entry.style.name,
                                            ))
                                            .clicked()
                                    {
                                        pick = Some(i);
                                    }
                                }
                                ui.add_space(4.0);
                            }
                        });
                    });
                    browse = widgets::strip_button(ui, Icon::Grid, false, true, "Browse styles")
                        .clicked();
                });
                ui.add_space(4.0);
                widgets::divider(ui);

                widgets::section(ui, "image", "Image", true, |ui| {
                    adjust_changed =
                        crate::image_panel::show(ui, &mut self.adjust, source.as_deref());
                });
                let before = self.draft.clone();
                let draft = &mut self.draft;
                let summary = self.summary.as_ref();
                widgets::section(ui, "film", "Film", true, |ui| {
                    workflow::film_section(ui, draft)
                });
                widgets::section(ui, "development", "Development", true, |ui| {
                    workflow::development_section(ui, draft, summary)
                });
                widgets::section(ui, "grain", "Grain", false, |ui| {
                    workflow::grain_section(ui, draft, summary)
                });
                widgets::section(ui, "print", "Print", false, |ui| {
                    workflow::print_section(ui, draft)
                });
                if self.draft != before {
                    self.draft_changed();
                }
            });
        if adjust_changed {
            self.adjust_changed();
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
        if browse {
            self.module = Module::Styles;
        }
        if let Some(i) = pick {
            self.pick_style(i);
        }
    }

    /// Apply style `i`: photographer styles bring their scene contrast, as presets do.
    fn pick_style(&mut self, i: usize) {
        let name = &self.store.entries[i].style.name;
        match self.presets.iter().position(|p| &p.style.name == name) {
            Some(p) => self.apply_preset(p),
            None => self.select_style(i),
        }
    }

    /// The Styles panel: styles in collapsible groups, as LightCraft's presets. Click to
    /// apply; right-click or "…" for Edit, Duplicate, Rename, Export, Move and Delete.
    fn styles_panel(&mut self, ui: &mut egui::Ui) {
        let t = Tokens::get(ui.ctx());
        let mut pick = None;
        let mut popup_request = None;
        let mut menu_action = None;
        let (mut import, mut save_new) = (false, false);
        ui.add_space(12.0);
        inset(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("Styles")
                        .font(t.semibold(15.0))
                        .color(t.text),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    import = widgets::text_button(ui, "Import…", false, true)
                        .on_hover_text("Add styles from .json files exported from BW Darkroom")
                        .clicked();
                    save_new = widgets::text_button(ui, "Save as…", false, true)
                        .on_hover_text("Keep the current settings as a new style of your own")
                        .clicked();
                });
            });
        });
        ui.add_space(12.0);
        widgets::divider(ui);
        egui::ScrollArea::vertical()
            .auto_shrink(false)
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                for (group, key, title) in StyleGroup::ALL {
                    let members: Vec<usize> = (0..self.store.entries.len())
                        .filter(|&i| StyleGroup::of(&self.store.entries[i]) == group)
                        .collect();
                    if members.is_empty() || !widgets::group_header(ui, key, title, members.len()) {
                        continue;
                    }
                    for i in members {
                        let entry = &self.store.entries[i];
                        let w = ui.available_width();
                        let (r, resp) = ui.allocate_exact_size(egui::vec2(w, 26.0), Sense::click());
                        let selected = i == self.current;
                        let menu_open = self.style_popup.is_some_and(|(o, _)| o == i);
                        let p = ui.painter();
                        if selected {
                            p.rect_filled(r, 0.0, t.tool_active);
                        } else if resp.hovered() || menu_open {
                            p.rect_filled(r, 0.0, t.hover.gamma_multiply(0.7));
                        }
                        let text_rect = egui::Rect::from_min_max(
                            egui::pos2(r.left() + 40.0, r.top()),
                            egui::pos2(r.right() - 34.0, r.bottom()),
                        );
                        let colour = if selected { t.text } else { t.text_label };
                        p.with_clip_rect(text_rect).text(
                            text_rect.left_center(),
                            egui::Align2::LEFT_CENTER,
                            &entry.style.name,
                            t.font(13.0),
                            colour,
                        );
                        let hover = match self
                            .presets
                            .iter()
                            .find(|p| p.style.name == entry.style.name)
                        {
                            Some(p) => format!("{}\n\n{}", p.look, p.basis),
                            None => entry.style.name.clone(),
                        };
                        let resp = resp.on_hover_text(hover);
                        // "…" over the row's right end, shown on hover, selection or while open.
                        let dots = egui::Rect::from_min_max(
                            egui::pos2(r.right() - 32.0, r.top()),
                            egui::pos2(r.right() - 8.0, r.bottom()),
                        );
                        let ellipsis = ui.interact(
                            dots,
                            egui::Id::unique(("style-ellipsis", i)),
                            Sense::click(),
                        );
                        if resp.hovered() || ellipsis.hovered() || selected || menu_open {
                            if ellipsis.hovered() {
                                ui.painter().rect_filled(dots.shrink(2.0), 3.0, t.hover);
                            }
                            ui.painter().text(
                                dots.center(),
                                egui::Align2::CENTER_CENTER,
                                "…",
                                t.font(13.0),
                                t.text_label,
                            );
                        }
                        let ellipsis = ellipsis
                            .on_hover_text("Edit, duplicate, rename, export or delete this style");
                        // `BW_DARKROOM_SCREENSHOT_STYLEMENU=<index>` opens a menu for screenshots.
                        let screenshot_menu = self.screenshot.is_some()
                            && self.style_popup.is_none()
                            && std::env::var("BW_DARKROOM_SCREENSHOT_STYLEMENU")
                                .ok()
                                .and_then(|v| v.parse::<usize>().ok())
                                == Some(i);
                        if ellipsis.clicked() || screenshot_menu {
                            popup_request = Some((i, dots.left_bottom()));
                        } else if resp.clicked() {
                            pick = Some(i);
                        }
                        let (up, down) = (self.store.can_move(i, -1), self.store.can_move(i, 1));
                        let builtin = entry.builtin;
                        resp.context_menu(|ui| {
                            if let Some(a) = style_menu::items(ui, i, builtin, up, down) {
                                menu_action = Some(a);
                            }
                        });
                    }
                    ui.add_space(6.0);
                }
            });
        if let Some(i) = pick {
            self.pick_style(i);
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
        if save_new {
            self.save_as_new();
        }
    }

    fn gallery(&mut self, ui: &mut egui::Ui) {
        let mut pick = None;
        let mut menu_action = None;
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Styles").strong())
                .on_hover_text("The photo above rendered in every style. Click one to use it.");
            ui.label(
                egui::RichText::new("this photo in every style")
                    .small()
                    .weak(),
            );
        });
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
                        egui::RichText::new(
                            "Choose a folder on the left, then a photo from the strip below",
                        )
                        .weak(),
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
                if i.key_pressed(egui::Key::I) {
                    self.show_filmstrip = !self.show_filmstrip;
                }
                if i.key_pressed(egui::Key::F) {
                    // Focus mode: hide everything, or bring everything back.
                    let any = self.show_browser
                        || self.show_styles
                        || self.show_gallery
                        || self.show_filmstrip;
                    self.show_browser = !any;
                    self.show_styles = !any;
                    self.show_gallery = !any;
                    self.show_filmstrip = !any;
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
            egui::Frame::side_top_panel(ui.style()).inner_margin(egui::Margin::symmetric(0, 4));
        // New id: the folder tree starts at its own default width, not the file list's.
        egui::Panel::left("folders")
            .frame(panel_frame)
            .resizable(true)
            .default_size(220.0)
            .size_range(160.0..=420.0)
            .show_collapsible(ui, &mut show_browser, |ui| self.browser_panel(ui));
        self.show_browser = show_browser;
        let tokens = Tokens::get(ctx);
        egui::Panel::right("tool_strip")
            .exact_size(48.0)
            .resizable(false)
            .frame(
                egui::Frame::NONE
                    .fill(tokens.chrome)
                    .stroke(egui::Stroke::new(1.0, tokens.divider)),
            )
            .show(ui, |ui| self.tool_strip(ui));
        let mut show_styles = self.show_styles;
        let tools_frame = egui::Frame::side_top_panel(ui.style()).inner_margin(0);
        egui::Panel::right("tools")
            .frame(tools_frame)
            .resizable(true)
            .default_size(290.0)
            .size_range(260.0..=480.0)
            .show_collapsible(ui, &mut show_styles, |ui| match self.module {
                Module::Develop => self.develop_panel(ui),
                Module::Styles => self.styles_panel(ui),
            });
        self.show_styles = show_styles;
        // The image strip sits at the very bottom, the style gallery above it.
        let mut show_filmstrip = self.show_filmstrip;
        egui::Panel::bottom("filmstrip")
            .resizable(true)
            .default_size(160.0)
            .size_range(90.0..=280.0)
            .show_collapsible(ui, &mut show_filmstrip, |ui| self.filmstrip(ui));
        self.show_filmstrip = show_filmstrip;
        let mut show_gallery = self.show_gallery;
        egui::Panel::bottom("gallery")
            .resizable(true)
            .default_size(120.0)
            .size_range(80.0..=300.0)
            .show_collapsible(ui, &mut show_gallery, |ui| self.gallery(ui));
        self.show_gallery = show_gallery;
        // The photo sits on the darker canvas colour, as in LightCraft.
        egui::CentralPanel::default()
            .frame(egui::Frame::central_panel(ui.style()).fill(tokens.canvas))
            .show(ui, |ui| self.central(ui));

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
        #[cfg(target_os = "macos")]
        if self.menu.as_ref().is_some_and(|m| m.take_check_request()) {
            self.check_for_updates(ctx);
        }
        if std::mem::take(&mut self.about.check_requested) {
            self.check_for_updates(ctx);
        }
        self.poll_update_check();
        self.update_window(ctx);
        self.about.show(ctx);
        self.style_popup(ctx);
        self.style_dialogs(ctx);
        self.screenshot_hook(ctx);

        if self.detail_enabled && self.loaded.is_some() {
            let mut open = true;
            let mut zoom = self.detail_zoom;
            let ppp = self.pixels_per_point;
            let mut drag: Option<(Vec2, Vec2, [f32; 2])> = None;
            egui::Window::new("Detail")
                // A fresh id drops the oversized window size saved by earlier versions.
                .id(egui::Id::unique("detail-window"))
                .open(&mut open)
                .resizable(false)
                // Shrink as well as grow with the content.
                .auto_sized()
                // Open over the left of the image, leaving the Workflow panel free to adjust
                // while watching the detail.
                .default_pos([ctx.content_rect().left() + 240.0, 90.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        for (z, label, help) in [
                            (1u8, "100%", "One image pixel per screen pixel: the export at full size"),
                            (2u8, "200%", "Magnified two times: each image pixel is a 2 × 2 block, for judging grain"),
                        ] {
                            ui.selectable_value(&mut zoom, z, label).on_hover_text(help);
                        }
                        if self.detail.is_some() && self.detail_shown_generation != self.detail_generation {
                            ui.spinner().on_hover_text("Updating to the latest settings");
                        }
                        ui.label(
                            egui::RichText::new("Drag here, or click the photo, to move the area").small().weak(),
                        );
                    });
                    match &self.detail {
                        Some(tex) => {
                            // Texture pixels → points so the zoom is exact on any display.
                            let size = tex.size_vec2() * f32::from(self.detail_texture_zoom) / ppp;
                            let (rect, response) = ui.allocate_exact_size(size, Sense::drag());
                            let [x0, y0, x1, y1] = self.detail_area.unwrap_or([0.0, 0.0, 1.0, 1.0]);
                            let frac = Vec2::new((x1 - x0) / size.x, (y1 - y0) / size.y);
                            // Until the render for the current centre arrives, slide the one
                            // on screen by the difference, clipped to the view.
                            let (tc, c) = (self.detail_texture_centre, self.detail_centre);
                            let shift = Vec2::new((tc[0] - c[0]) / frac.x, (tc[1] - c[1]) / frac.y);
                            let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
                            ui.painter()
                                .with_clip_rect(rect)
                                .image(tex.id(), rect.translate(shift), uv, Color32::WHITE);
                            if response.dragged() {
                                ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                                drag = Some((response.drag_delta(), frac, [x1 - x0, y1 - y0]));
                            } else if response.hovered() {
                                ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
                            }
                            response.on_hover_text("Drag to move the area you're inspecting");
                        }
                        None => {
                            ui.allocate_ui(egui::vec2(DETAIL_VIEW_PT, DETAIL_VIEW_PT), |ui| ui.spinner());
                        }
                    }
                });
            if let Some((delta, frac, [w, h])) = drag
                && delta != Vec2::ZERO
            {
                // Grabbing the photo: dragging right shows what lies to the left. The area
                // stays inside the frame.
                let clamp = |v: f32, half: f32| v.clamp(half.min(0.5), (1.0 - half).max(0.5));
                self.detail_centre = [
                    clamp(self.detail_centre[0] - delta.x * frac.x, w / 2.0),
                    clamp(self.detail_centre[1] - delta.y * frac.y, h / 2.0),
                ];
                self.request_detail();
            }
            if zoom != self.detail_zoom {
                self.detail_zoom = zoom;
                self.request_detail();
            }
            if !open {
                self.detail_enabled = false;
            }
        }
    }

    fn raw_input_hook(&mut self, ctx: &egui::Context, raw_input: &mut egui::RawInput) {
        let Some((_, started, _)) = &self.screenshot else {
            return;
        };
        let now = started.elapsed().as_secs_f64();
        // One event per frame, one gesture at a time.
        if let Some(gesture) = self.screenshot_clicks.iter_mut().find(|g| !g.done()) {
            if now >= gesture.at_s {
                raw_input.events.push(gesture.next_event());
            }
            ctx.request_repaint();
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
        storage.set_string(KEY_MODULE, self.module.key().into());
        let theme = match self.theme {
            egui::ThemePreference::Light => "light",
            egui::ThemePreference::Dark => "dark",
            egui::ThemePreference::System => "system",
        };
        storage.set_string(KEY_THEME, theme.into());
        let flag_str = |b: bool| if b { "1" } else { "0" }.to_string();
        storage.set_string(KEY_BROWSER_BITMAPS, flag_str(self.browser_filter.bitmaps));
        storage.set_string(KEY_UPDATE_AUTO, flag_str(self.update_auto));
        storage.set_string(KEY_UPDATE_LAST, self.update_last_check.to_string());
        storage.set_string(
            KEY_UPDATE_SKIPPED,
            self.update_skipped.clone().unwrap_or_default(),
        );
        storage.set_string(KEY_BROWSER_JPEGS, flag_str(self.browser_filter.jpegs));
        let flag = |b: bool| if b { '1' } else { '0' };
        storage.set_string(
            KEY_PANELS,
            [
                self.show_browser,
                self.show_styles,
                self.show_gallery,
                self.show_filmstrip,
            ]
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
