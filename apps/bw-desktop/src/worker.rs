//! Background rendering.
//!
//! One render thread owns the decoded image and serves jobs in priority order:
//! load, preview, detail (1:1) view, then style-gallery tiles. Newer requests of the
//! same kind replace older ones, so dragging a slider never builds a queue.
//! Exports run on their own threads so they never block previews.
//!
//! Every render takes the film format and enlargement from its style, so grain and
//! edge effects are scaled to the film area each pixel covers.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender};
use std::time::Instant;

use bw_negative::SceneImage;
use bw_profiles::{ExportRecord, Style};
use eframe::egui;

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
        edge: usize,
    },
    Detail {
        generation: u64,
        style: Style,
        centre: [f32; 2],
    },
    Tiles {
        generation: u64,
        styles: Vec<Style>,
    },
}

pub enum Msg {
    Loaded {
        generation: u64,
        full: Arc<SceneImage>,
        original: egui::ColorImage,
    },
    LoadFailed {
        generation: u64,
        error: String,
    },
    Preview {
        generation: u64,
        image: egui::ColorImage,
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
        tiff: PathBuf,
        millis: u128,
    },
    ExportFailed {
        error: String,
    },
}

struct Current {
    full: Arc<SceneImage>,
    /// Downscaled copy for the preview, and the edge it was made for.
    preview: Option<(usize, SceneImage)>,
    tile: SceneImage,
}

impl Current {
    fn preview_at(&mut self, edge: usize) -> &SceneImage {
        if self.preview.as_ref().is_none_or(|(e, _)| *e != edge) {
            self.preview = Some((edge, bw_io::downscale(&self.full, edge)));
        }
        &self.preview.as_ref().expect("just set").1
    }
}

#[derive(Default)]
struct Pending {
    load: Option<(u64, PathBuf)>,
    preview: Option<(u64, Style, usize)>,
    detail: Option<(u64, Style, [f32; 2])>,
    tiles: VecDeque<(u64, usize, Style)>,
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
                edge,
            } => self.preview = Some((generation, style, edge)),
            Job::Detail {
                generation,
                style,
                centre,
            } => self.detail = Some((generation, style, centre)),
            Job::Tiles { generation, styles } => {
                self.tiles = styles
                    .into_iter()
                    .enumerate()
                    .map(|(i, s)| (generation, i, s))
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
                    match bw_io::load_scene(&path, bw_io::Encoding::Auto) {
                        Ok(full) => {
                            let small = bw_io::downscale(&full, 1600);
                            let tile = bw_io::downscale(&small, TILE_EDGE);
                            let (w, h, px) = bw_io::scene_to_srgb_rgba(&small);
                            let full = Arc::new(full);
                            current = Some(Current {
                                full: full.clone(),
                                preview: Some((1600, small)),
                                tile,
                            });
                            Msg::Loaded {
                                generation,
                                full,
                                original: egui::ColorImage::from_rgba_unmultiplied([w, h], &px),
                            }
                        }
                        Err(e) => {
                            current = None;
                            Msg::LoadFailed {
                                generation,
                                error: e.to_string(),
                            }
                        }
                    }
                } else if let Some((generation, style, edge)) = pending.preview.take() {
                    let Some(cur) = current.as_mut() else {
                        continue;
                    };
                    let start = Instant::now();
                    match render_whole(cur.preview_at(edge), &style) {
                        Ok(r) => Msg::Preview {
                            generation,
                            image: gray_image(&r.print),
                            millis: start.elapsed().as_millis(),
                        },
                        Err(error) => Msg::RenderFailed { error },
                    }
                } else if let Some((generation, style, centre)) = pending.detail.take() {
                    let Some(cur) = &current else { continue };
                    let (w, h) = cur.full.dimensions();
                    let cx = (centre[0] * w as f32) as usize;
                    let cy = (centre[1] * h as f32) as usize;
                    let x = cx
                        .saturating_sub(DETAIL_SIZE / 2)
                        .min(w.saturating_sub(DETAIL_SIZE));
                    let y = cy
                        .saturating_sub(DETAIL_SIZE / 2)
                        .min(h.saturating_sub(DETAIL_SIZE));
                    let crop = bw_io::crop(&cur.full, x, y, DETAIL_SIZE, DETAIL_SIZE);
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
                } else if let Some((generation, index, style)) = pending.tiles.pop_front() {
                    let Some(cur) = &current else { continue };
                    match render_whole(&cur.tile, &style) {
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

/// File-name-safe version of a style name.
fn slug(name: &str) -> String {
    let mut s: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    while s.contains("--") {
        s = s.replace("--", "-");
    }
    s.trim_matches('-').to_string()
}

/// First `stem__slug[-n].tif` that does not exist yet.
fn unique_path(dir: &Path, stem: &str, style: &str) -> PathBuf {
    let base = format!("{stem}__{}", slug(style));
    let candidate = dir.join(format!("{base}.tif"));
    if !candidate.exists() {
        return candidate;
    }
    (2..)
        .map(|i| dir.join(format!("{base}-{i}.tif")))
        .find(|p| !p.exists())
        .expect("unbounded search")
}

/// Render at full resolution and write a 16-bit TIFF plus a JSON recipe sidecar.
pub fn spawn_export(
    full: Arc<SceneImage>,
    source: PathBuf,
    style: Style,
    dir: PathBuf,
    tx: Sender<Msg>,
    ctx: egui::Context,
) {
    std::thread::Builder::new()
        .name("export".into())
        .spawn(move || {
            let start = Instant::now();
            let result = (|| -> Result<PathBuf, String> {
                std::fs::create_dir_all(&dir)
                    .map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
                let rendered = render_whole(&full, &style)?;
                let stem = source
                    .file_stem()
                    .map_or("image".into(), |s| s.to_string_lossy().into_owned());
                let tiff = unique_path(&dir, &stem, &style.name);
                bw_io::save_print_tiff16(&rendered.print, &tiff).map_err(|e| e.to_string())?;
                let record = ExportRecord::new(
                    source.display().to_string(),
                    &style,
                    style.frame_width_mm(),
                    &rendered,
                );
                std::fs::write(tiff.with_extension("json"), record.to_json())
                    .map_err(|e| e.to_string())?;
                Ok(tiff)
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

    #[test]
    fn export_writes_tiff_and_recipe() {
        let dir = std::env::temp_dir().join(format!("bw-export-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let scene = SceneImage::Mono {
            y: bw_core::Plane::from_fn(96, 64, |x, y| 0.01 + (x + y) as f32 / 160.0).unwrap(),
        };
        let style = bw_profiles::builtin_styles().remove(0);
        let (tx, rx) = std::sync::mpsc::channel();
        let source = PathBuf::from("/photos/frame 01.tif");
        for _ in 0..2 {
            spawn_export(
                Arc::new(scene.clone()),
                source.clone(),
                style.clone(),
                dir.clone(),
                tx.clone(),
                egui::Context::default(),
            );
            match rx.recv_timeout(std::time::Duration::from_secs(30)).unwrap() {
                Msg::Exported { tiff, .. } => {
                    let img = image::open(&tiff).unwrap();
                    assert_eq!((img.width(), img.height()), (96, 64));
                    assert!(matches!(img, image::DynamicImage::ImageLuma16(_)));
                    let json = std::fs::read_to_string(tiff.with_extension("json")).unwrap();
                    assert!(json.contains("\"film\": \"tri-x\""));
                }
                Msg::ExportFailed { error } => panic!("{error}"),
                _ => panic!("unexpected message"),
            }
        }
        // The second export must not overwrite the first.
        let stem = format!("frame 01__{}", slug(&style.name));
        assert!(dir.join(format!("{stem}.tif")).exists());
        assert!(dir.join(format!("{stem}-2.tif")).exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn slugs_are_filename_safe() {
        assert_eq!(slug("Tri-X · D-76, normal"), "tri-x-d-76-normal");
        assert_eq!(slug("Rodinal 1+100 stand"), "rodinal-1-100-stand");
    }
}
