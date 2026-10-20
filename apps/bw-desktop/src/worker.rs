//! Background rendering.
//!
//! One render thread owns the opened image and serves jobs in priority order:
//! load, preview, detail (1:1) view, then style-gallery tiles. Newer requests of the
//! same kind replace older ones, so dragging a slider never builds a queue.
//! Exports run on their own threads so they never block previews.
//!
//! Every job carries the image's adjustments (exposure, white balance, …) and its
//! style; the film format and enlargement come from the style, so grain and edge
//! effects are scaled to the film area each pixel covers.

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender};
use std::time::Instant;

use bw_negative::SceneImage;
use bw_profiles::{ExportRecord, Style};
use bw_raw::RawAdjust;
use eframe::egui;

use crate::export::{Conflict, Format, resolve_conflict};
use crate::source::Source;

/// Long edge of style-gallery tiles.
pub const TILE_EDGE: usize = 360;
/// Side of the 1:1 detail crop.
pub const DETAIL_SIZE: usize = 512;

pub enum Job {
    Load {
        generation: u64,
        path: PathBuf,
    },
    /// Render the preview with its long edge at most `edge` pixels.
    Preview {
        generation: u64,
        style: Style,
        adjust: RawAdjust,
        edge: usize,
    },
    Detail {
        generation: u64,
        style: Style,
        adjust: RawAdjust,
        centre: [f32; 2],
    },
    Tiles {
        generation: u64,
        styles: Vec<Style>,
        adjust: RawAdjust,
    },
}

pub enum Msg {
    Loaded {
        generation: u64,
        source: Arc<Source>,
    },
    LoadFailed {
        generation: u64,
        error: String,
    },
    Preview {
        generation: u64,
        image: egui::ColorImage,
        /// The adjusted colour image, sent when the adjustments or size changed.
        original: Option<egui::ColorImage>,
        millis: u128,
    },
    Detail {
        generation: u64,
        image: egui::ColorImage,
    },
    Tile {
        generation: u64,
        index: usize,
        image: egui::ColorImage,
    },
    TileFailed {
        generation: u64,
        index: usize,
    },
    RenderFailed {
        error: String,
    },
    Thumb {
        path: PathBuf,
        image: egui::ColorImage,
    },
    Exported {
        /// The file written.
        tiff: PathBuf,
        millis: u128,
    },
    ExportFailed {
        error: String,
    },
}

/// The opened image and scenes derived from it, each cached with the settings it was
/// made for.
struct Current {
    full: Arc<Source>,
    /// Reduced copy for previews, and the edge it was made for.
    preview_source: Option<(usize, Source)>,
    /// Adjusted preview scene, keyed by edge and adjustments.
    preview_scene: Option<(usize, RawAdjust, SceneImage)>,
    tile_source: Source,
    tile_scene: Option<(RawAdjust, SceneImage)>,
    /// Adjusted full-resolution scene for the detail view.
    full_scene: Option<(RawAdjust, SceneImage)>,
}

impl Current {
    fn new(full: Arc<Source>) -> Self {
        let tile_source = full.downscaled(TILE_EDGE);
        Self {
            full,
            preview_source: None,
            preview_scene: None,
            tile_source,
            tile_scene: None,
            full_scene: None,
        }
    }

    /// The adjusted preview scene and whether it was just (re)made.
    fn preview(&mut self, edge: usize, adjust: &RawAdjust) -> (&SceneImage, bool) {
        if self.preview_source.as_ref().is_none_or(|(e, _)| *e != edge) {
            self.preview_source = Some((edge, self.full.downscaled(edge)));
        }
        let fresh = self
            .preview_scene
            .as_ref()
            .is_none_or(|(e, a, _)| *e != edge || a != adjust);
        if fresh {
            let source = &self.preview_source.as_ref().expect("just set").1;
            self.preview_scene = Some((edge, *adjust, source.develop(adjust)));
        }
        (&self.preview_scene.as_ref().expect("just set").2, fresh)
    }

    fn tile(&mut self, adjust: &RawAdjust) -> &SceneImage {
        if self.tile_scene.as_ref().is_none_or(|(a, _)| a != adjust) {
            self.tile_scene = Some((*adjust, self.tile_source.develop(adjust)));
        }
        &self.tile_scene.as_ref().expect("just set").1
    }

    fn full(&mut self, adjust: &RawAdjust) -> &SceneImage {
        if self.full_scene.as_ref().is_none_or(|(a, _)| a != adjust) {
            self.full_scene = Some((*adjust, self.full.develop(adjust)));
        }
        &self.full_scene.as_ref().expect("just set").1
    }
}

#[derive(Default)]
struct Pending {
    load: Option<(u64, PathBuf)>,
    preview: Option<(u64, Style, RawAdjust, usize)>,
    detail: Option<(u64, Style, RawAdjust, [f32; 2])>,
    tiles: VecDeque<(u64, usize, Style, RawAdjust)>,
}

impl Pending {
    fn is_empty(&self) -> bool {
        self.load.is_none()
            && self.preview.is_none()
            && self.detail.is_none()
            && self.tiles.is_empty()
    }

    fn push(&mut self, job: Job) {
        match job {
            Job::Load { generation, path } => {
                self.load = Some((generation, path));
                self.detail = None;
                self.tiles.clear();
            }
            Job::Preview {
                generation,
                style,
                adjust,
                edge,
            } => self.preview = Some((generation, style, adjust, edge)),
            Job::Detail {
                generation,
                style,
                adjust,
                centre,
            } => self.detail = Some((generation, style, adjust, centre)),
            Job::Tiles {
                generation,
                styles,
                adjust,
            } => {
                self.tiles = styles
                    .into_iter()
                    .enumerate()
                    .map(|(i, s)| (generation, i, s, adjust))
                    .collect();
            }
        }
    }
}

fn gray_image(plane: &bw_core::Plane) -> egui::ColorImage {
    egui::ColorImage::from_gray(
        [plane.width(), plane.height()],
        &bw_io::plane_to_srgb_gray8(plane),
    )
}

fn colour_image(scene: &SceneImage) -> egui::ColorImage {
    let (w, h, px) = bw_io::scene_to_srgb_rgba(scene);
    egui::ColorImage::from_rgba_unmultiplied([w, h], &px)
}

/// Render `scene`, which shows the whole frame `style` describes.
fn render_whole(scene: &SceneImage, style: &Style) -> Result<bw_profiles::Rendered, String> {
    bw_profiles::render(scene, style, style.frame_width_mm()).map_err(|e| e.to_string())
}

pub fn spawn_renderer(rx: Receiver<Job>, tx: Sender<Msg>, ctx: egui::Context) {
    std::thread::Builder::new()
        .name("renderer".into())
        .spawn(move || {
            let mut current: Option<Current> = None;
            let mut pending = Pending::default();
            loop {
                if pending.is_empty() {
                    match rx.recv() {
                        Ok(job) => pending.push(job),
                        Err(_) => return,
                    }
                }
                while let Ok(job) = rx.try_recv() {
                    pending.push(job);
                }

                let msg = if let Some((generation, path)) = pending.load.take() {
                    match Source::open(&path) {
                        Ok(source) => {
                            let source = Arc::new(source);
                            current = Some(Current::new(source.clone()));
                            Msg::Loaded { generation, source }
                        }
                        Err(error) => {
                            current = None;
                            Msg::LoadFailed { generation, error }
                        }
                    }
                } else if let Some((generation, style, adjust, edge)) = pending.preview.take() {
                    let Some(cur) = current.as_mut() else {
                        continue;
                    };
                    let start = Instant::now();
                    let (scene, fresh) = cur.preview(edge, &adjust);
                    let original = fresh.then(|| colour_image(scene));
                    match render_whole(scene, &style) {
                        Ok(r) => Msg::Preview {
                            generation,
                            image: gray_image(&r.print),
                            original,
                            millis: start.elapsed().as_millis(),
                        },
                        Err(error) => Msg::RenderFailed { error },
                    }
                } else if let Some((generation, style, adjust, centre)) = pending.detail.take() {
                    let Some(cur) = current.as_mut() else {
                        continue;
                    };
                    let full = cur.full(&adjust);
                    let (w, h) = full.dimensions();
                    let cx = (centre[0] * w as f32) as usize;
                    let cy = (centre[1] * h as f32) as usize;
                    let x = cx
                        .saturating_sub(DETAIL_SIZE / 2)
                        .min(w.saturating_sub(DETAIL_SIZE));
                    let y = cy
                        .saturating_sub(DETAIL_SIZE / 2)
                        .min(h.saturating_sub(DETAIL_SIZE));
                    let crop = bw_io::crop(full, x, y, DETAIL_SIZE, DETAIL_SIZE);
                    // The crop spans its share of the frame, so grain and edge effects
                    // are rendered at the full-resolution scale.
                    let crop_mm = style.frame_width_mm() * crop.dimensions().0 as f64 / w as f64;
                    match bw_profiles::render(&crop, &style, crop_mm) {
                        Ok(r) => Msg::Detail {
                            generation,
                            image: gray_image(&r.print),
                        },
                        Err(e) => Msg::RenderFailed {
                            error: e.to_string(),
                        },
                    }
                } else if let Some((generation, index, style, adjust)) = pending.tiles.pop_front() {
                    let Some(cur) = current.as_mut() else {
                        continue;
                    };
                    match render_whole(cur.tile(&adjust), &style) {
                        Ok(r) => Msg::Tile {
                            generation,
                            index,
                            image: gray_image(&r.print),
                        },
                        Err(_) => Msg::TileFailed { generation, index },
                    }
                } else {
                    continue;
                };
                if tx.send(msg).is_err() {
                    return;
                }
                ctx.request_repaint();
            }
        })
        .expect("spawn render thread");
}

/// Everything one export needs; the file name is already resolved from the template.
pub struct ExportJob {
    pub source: Arc<Source>,
    pub source_path: PathBuf,
    pub style: Style,
    pub adjust: RawAdjust,
    /// Output path before the conflict policy is applied.
    pub target: PathBuf,
    pub format: Format,
    pub jpeg_quality: u8,
    pub conflict: Conflict,
    pub long_edge: Option<u32>,
    pub write_recipe: bool,
}

/// The recipe sits next to the image as `<file name>.json` (`print.jpg.json`), so a
/// JPEG and a TIFF of the same name keep separate recipes.
pub fn recipe_path(image: &std::path::Path) -> PathBuf {
    let mut name = image.file_name().unwrap_or_default().to_os_string();
    name.push(".json");
    image.with_file_name(name)
}

/// Develop and render at full resolution, then write the image (and its JSON recipe).
pub fn spawn_export(job: ExportJob, tx: Sender<Msg>, ctx: egui::Context) {
    std::thread::Builder::new()
        .name("export".into())
        .spawn(move || {
            let start = Instant::now();
            let result = (|| -> Result<PathBuf, String> {
                if let Some(dir) = job.target.parent() {
                    std::fs::create_dir_all(dir)
                        .map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
                }
                let scene = job.source.develop(&job.adjust);
                let rendered = render_whole(&scene, &job.style)?;
                drop(scene);
                let print = match job.long_edge {
                    Some(edge) => bw_io::resize_long_edge(&rendered.print, edge as usize),
                    None => rendered.print.clone(),
                };
                let path = resolve_conflict(job.target.clone(), job.conflict);
                match job.format {
                    Format::Jpeg => bw_io::save_print_jpeg(&print, &path, job.jpeg_quality),
                    Format::Png16 => bw_io::save_print_png16(&print, &path),
                    Format::Tiff16 | Format::SameAsOriginal => {
                        bw_io::save_print_tiff16(&print, &path)
                    }
                }
                .map_err(|e| e.to_string())?;
                if job.write_recipe {
                    let mut record = ExportRecord::new(
                        job.source_path.display().to_string(),
                        &job.style,
                        job.style.frame_width_mm(),
                        &rendered,
                    );
                    record.image_adjustments = serde_json::to_value(job.adjust).ok();
                    record.camera = job.source.capture().map(|c| c.summary());
                    std::fs::write(recipe_path(&path), record.to_json())
                        .map_err(|e| e.to_string())?;
                }
                Ok(path)
            })();
            let msg = match result {
                Ok(tiff) => Msg::Exported {
                    tiff,
                    millis: start.elapsed().as_millis(),
                },
                Err(error) => Msg::ExportFailed { error },
            };
            let _ = tx.send(msg);
            ctx.request_repaint();
        })
        .expect("spawn export thread");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn job(dir: &Path, format: Format, conflict: Conflict) -> ExportJob {
        let scene = SceneImage::Mono {
            y: bw_core::Plane::from_fn(96, 64, |x, y| 0.01 + (x + y) as f32 / 160.0).unwrap(),
        };
        ExportJob {
            source: Arc::new(Source::Raster(scene)),
            source_path: PathBuf::from("/photos/frame 01.tif"),
            style: bw_profiles::builtin_styles().remove(0),
            adjust: RawAdjust {
                rotation: 1,
                ..Default::default()
            },
            target: dir.join(format!("out.{}", format.extension())),
            format,
            jpeg_quality: 80,
            conflict,
            long_edge: None,
            write_recipe: true,
        }
    }

    fn run(job: ExportJob) -> PathBuf {
        let (tx, rx) = std::sync::mpsc::channel();
        spawn_export(job, tx, egui::Context::default());
        match rx.recv_timeout(std::time::Duration::from_secs(30)).unwrap() {
            Msg::Exported { tiff, .. } => tiff,
            Msg::ExportFailed { error } => panic!("{error}"),
            _ => panic!("unexpected message"),
        }
    }

    #[test]
    fn exports_each_format_with_recipe_and_conflict_policy() {
        let dir = std::env::temp_dir().join(format!("bw-export-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);

        let tiff = run(job(&dir, Format::Tiff16, Conflict::AddSuffix));
        let img = image::open(&tiff).unwrap();
        assert_eq!(
            (img.width(), img.height()),
            (64, 96),
            "rotated a quarter turn"
        );
        assert!(matches!(img, image::DynamicImage::ImageLuma16(_)));
        let json = std::fs::read_to_string(recipe_path(&tiff)).unwrap();
        assert!(json.contains("\"film\": \"tri-x\"") && json.contains("\"rotation\": 1"));

        let jpg = run(job(&dir, Format::Jpeg, Conflict::AddSuffix));
        assert!(matches!(
            image::open(&jpg).unwrap(),
            image::DynamicImage::ImageLuma8(_)
        ));

        // A second export to the same name: suffix, or overwrite.
        assert_eq!(
            run(job(&dir, Format::Jpeg, Conflict::AddSuffix)),
            dir.join("out-2.jpg")
        );
        assert_eq!(
            run(job(&dir, Format::Jpeg, Conflict::Overwrite)),
            dir.join("out.jpg")
        );

        let mut small = job(&dir, Format::Png16, Conflict::Overwrite);
        small.long_edge = Some(48);
        small.write_recipe = false;
        let png = run(small);
        let img = image::open(&png).unwrap();
        assert_eq!(img.width().max(img.height()), 48);
        assert!(!recipe_path(&png).exists());
        assert!(recipe_path(&jpg).exists() && recipe_path(&tiff).exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
