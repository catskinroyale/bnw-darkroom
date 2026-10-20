//! Application state and UI.
//!
//! Layout: folder browser (left), preview (centre), style editor (right), style
//! gallery showing the current image in every style (bottom).

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, Sender};

use bw_negative::SceneImage;
use bw_profiles::{ModelSummary, Style};
use eframe::egui::{self, Color32, Pos2, Sense, Stroke, TextureHandle, TextureOptions, Vec2};

use crate::browser::{self, Entry, ThumbRequest};
use crate::styles::StyleStore;
use crate::worker::{self, Job, Msg};
use crate::workflow;

const KEY_FOLDER: &str = "folder";
const KEY_VIEW: &str = "workflow_view";
const KEY_EXPORT_DIR: &str = "export_dir";
const KEY_PANELS: &str = "panels";
/// Thumbnail size in browser rows.
const THUMB_ROW: f32 = 28.0;

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
    full: Arc<SceneImage>,
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
    loading: bool,
    original: Option<TextureHandle>,

    store: StyleStore,
    current: usize,
    draft: Style,
    summary: Option<ModelSummary>,
    /// Workflow panel shows every control rather than the simple chips.
    detailed: bool,
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
    detail_generation: u64,
    detail: Option<TextureHandle>,

    tiles_generation: u64,
    tiles: Vec<Tile>,

    export_dir: Option<PathBuf>,
    exports_running: usize,
    status: String,

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
        let detailed = get(KEY_VIEW).is_some_and(|v| v == "detailed");
        let export_dir = get(KEY_EXPORT_DIR).map(PathBuf::from);
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
            loading: false,
            original: None,
            tiles: vec![Tile::Pending; store.entries.len()],
            store,
            current: 0,
            summary: draft.model_summary().ok(),
            draft,
            detailed,
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
            detail_generation: 0,
            detail: None,
            tiles_generation: 0,
            export_dir,
            exports_running: 0,
            status: String::new(),
            screenshot: std::env::var_os("BW_DARKROOM_SCREENSHOT")
                .map(|p| (PathBuf::from(p), std::time::Instant::now(), false)),
        };
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
            .filter(|e| !e.is_dir)
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
        self.request_preview();
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
            centre: self.detail_centre,
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
        });
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

    fn delete_current(&mut self) {
        if self.store.entries[self.current].builtin {
            return;
        }
        self.store.entries.remove(self.current);
        self.persist_styles();
        self.select_style(self.current.saturating_sub(1));
        self.request_tiles();
    }

    fn persist_styles(&mut self) {
        if let Err(e) = self.store.save() {
            self.status = format!("Could not save styles: {e}");
        }
    }

    fn export(&mut self, ctx: &egui::Context) {
        let Some(loaded) = &self.loaded else { return };
        let dir = self.export_dir.clone().unwrap_or_else(|| {
            loaded
                .path
                .parent()
                .map_or_else(|| PathBuf::from("."), |p| p.to_path_buf())
                .join("bw-exports")
        });
        self.exports_running += 1;
        self.status = format!("Exporting “{}” at full resolution…", self.draft.name);
        worker::spawn_export(
            loaded.full.clone(),
            loaded.path.clone(),
            self.draft.clone(),
            dir,
            self.msg_tx.clone(),
            ctx.clone(),
        );
    }

    // ----- messages ------------------------------------------------------------

    fn handle_messages(&mut self, ctx: &egui::Context) {
        while let Ok(msg) = self.msgs.try_recv() {
            match msg {
                Msg::Loaded {
                    generation,
                    full,
                    original,
                } if generation == self.image_generation => {
                    let (w, h) = full.dimensions();
                    let path = self.selected.clone().unwrap_or_default();
                    self.status = format!("{} · {w}×{h}", file_name(&path));
                    self.loaded = Some(LoadedImage { path, full });
                    self.loading = false;
                    self.original =
                        Some(ctx.load_texture("original", original, TextureOptions::LINEAR));
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
                    millis,
                } if generation == self.preview_generation => {
                    set_texture(ctx, &mut self.preview, "preview", image);
                    self.preview_ms = Some(millis);
                }
                Msg::Detail { generation, image } if generation == self.detail_generation => {
                    set_texture(ctx, &mut self.detail, "detail", image);
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
        let Some((path, started, requested)) = &mut self.screenshot else {
            return;
        };
        let rendered = (self.preview.is_some() || self.summary.is_none())
            && !self.tiles.iter().any(|t| matches!(t, Tile::Pending));
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
            ui.heading("BW Darkroom");
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
                let export = ui
                    .add_enabled(can_export, egui::Button::new("Export print…"))
                    .on_hover_text(
                        "Render at full resolution and save a 16-bit TIFF plus a JSON recipe",
                    );
                if export.clicked() {
                    self.export(ui.ctx());
                }
                let dir_label = self
                    .export_dir
                    .as_ref()
                    .map_or("Next to image / bw-exports".to_string(), |d| {
                        d.display().to_string()
                    });
                if ui
                    .button("Export folder…")
                    .on_hover_text(format!("Export folder: {dir_label}"))
                    .clicked()
                    && let Some(dir) = rfd::FileDialog::new().pick_folder()
                {
                    self.export_dir = Some(dir);
                }
                if self.exports_running > 0 {
                    ui.spinner();
                }
            });
        });
    }

    fn browser_panel(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui
                .small_button("Up")
                .on_hover_text("Parent folder")
                .clicked()
                && let Some(parent) = self.folder.parent().map(|p| p.to_path_buf())
            {
                self.open_folder(parent);
            }
            ui.add(
                egui::Label::new(egui::RichText::new(file_name(&self.folder)).strong()).truncate(),
            )
            .on_hover_text(self.folder.display().to_string());
        });
        ui.separator();

        let mut open: Option<PathBuf> = None;
        let mut select: Option<PathBuf> = None;
        egui::ScrollArea::vertical()
            .auto_shrink(false)
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 2.0;
                let row = Vec2::new(ui.available_width(), THUMB_ROW + 4.0);
                if self.entries.is_empty() {
                    ui.label(egui::RichText::new("No folders or images here").weak());
                }
                for entry in &self.entries {
                    let button = if entry.is_dir {
                        egui::Button::selectable(false, format!("▸ {}", entry.name))
                    } else {
                        let selected = self.selected.as_ref() == Some(&entry.path);
                        match self.thumbs.get(&entry.path) {
                            Some(tex) => {
                                let thumb = egui::Image::new(tex)
                                    .fit_to_exact_size(Vec2::splat(THUMB_ROW))
                                    .maintain_aspect_ratio(true);
                                egui::Button::selectable(selected, (thumb, entry.name.as_str()))
                            }
                            None => egui::Button::selectable(selected, entry.name.as_str()),
                        }
                    };
                    let response = ui
                        .add(button.truncate().min_size(row))
                        .on_hover_text(&entry.name);
                    if response.clicked() {
                        if entry.is_dir {
                            open = Some(entry.path.clone());
                        } else {
                            select = Some(entry.path.clone());
                        }
                    }
                }
            });
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
                let mut pick = None;
                egui::CollapsingHeader::new(egui::RichText::new("Styles").heading())
                    .default_open(true)
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing.y = 1.0;
                        let row = Vec2::new(ui.available_width(), 0.0);
                        for (i, entry) in self.store.entries.iter().enumerate() {
                            let label = if entry.builtin {
                                entry.style.name.clone()
                            } else {
                                format!("{}  (mine)", entry.style.name)
                            };
                            let button = egui::Button::selectable(i == self.current, label)
                                .truncate()
                                .min_size(row);
                            if ui.add(button).clicked() {
                                pick = Some(i);
                            }
                        }
                    });
                if let Some(i) = pick {
                    self.select_style(i);
                }

                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.heading("Workflow");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add(egui::Button::selectable(self.detailed, "Detailed"))
                            .on_hover_text("Every processing and printing control")
                            .clicked()
                        {
                            self.detailed = true;
                        }
                        if ui
                            .add(egui::Button::selectable(!self.detailed, "Simple"))
                            .on_hover_text("Film, developer, push/pull and grain as quick choices")
                            .clicked()
                        {
                            self.detailed = false;
                        }
                    });
                });
                let before = self.draft.clone();
                if self.detailed {
                    workflow::detailed(ui, &mut self.draft, self.summary.as_ref());
                } else {
                    workflow::simple(ui, &mut self.draft, self.summary.as_ref());
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
                        self.delete_current();
                    }
                });
            });
    }

    fn gallery(&mut self, ui: &mut egui::Ui) {
        let mut pick = None;
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
                            if response.on_hover_text(&entry.style.name).clicked() {
                                pick = Some(i);
                            }
                        });
                    }
                });
            });
        if let Some(i) = pick {
            self.select_style(i);
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
        let size = tex_size * scale;
        let response = ui
            .centered_and_justified(|ui| {
                ui.add(
                    egui::Image::new(&texture)
                        .fit_to_exact_size(size)
                        .sense(Sense::click()),
                )
            })
            .inner;
        if self.detail_enabled
            && response.clicked()
            && let Some(pos) = response.interact_pointer_pos()
        {
            let rect = response.rect;
            self.detail_centre = [
                ((pos.x - rect.min.x) / rect.width()).clamp(0.0, 1.0),
                ((pos.y - rect.min.y) / rect.height()).clamp(0.0, 1.0),
            ];
            self.request_detail();
        }
        if self.detail_enabled {
            let rect = response.rect;
            let c = Pos2::new(
                rect.min.x + self.detail_centre[0] * rect.width(),
                rect.min.y + self.detail_centre[1] * rect.height(),
            );
            ui.painter()
                .circle_stroke(c, 6.0, Stroke::new(1.5, Color32::from_rgb(230, 120, 60)));
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = &ui.ctx().clone();
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
        egui::Panel::left("browser")
            .frame(panel_frame)
            .resizable(true)
            .default_size(210.0)
            .size_range(140.0..=420.0)
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

        self.screenshot_hook(ctx);

        if self.detail_enabled && self.loaded.is_some() {
            let mut open = true;
            egui::Window::new("Detail 1:1")
                .open(&mut open)
                .resizable(false)
                .default_pos([ctx.content_rect().right() - 900.0, 120.0])
                .show(ctx, |ui| match &self.detail {
                    Some(tex) => {
                        ui.add(egui::Image::new(tex).fit_to_exact_size(tex.size_vec2()));
                    }
                    None => {
                        ui.spinner();
                    }
                });
            if !open {
                self.detail_enabled = false;
            }
        }
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        if self.screenshot.is_some() {
            return; // test runs must not change the user's saved state
        }
        storage.set_string(KEY_FOLDER, self.folder.display().to_string());
        storage.set_string(
            KEY_VIEW,
            if self.detailed { "detailed" } else { "simple" }.into(),
        );
        let flag = |b: bool| if b { '1' } else { '0' };
        storage.set_string(
            KEY_PANELS,
            [self.show_browser, self.show_styles, self.show_gallery]
                .map(flag)
                .iter()
                .collect(),
        );
        if let Some(dir) = &self.export_dir {
            storage.set_string(KEY_EXPORT_DIR, dir.display().to_string());
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
