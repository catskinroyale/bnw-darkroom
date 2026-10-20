//! Folder listing and background thumbnail loading.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender};

use eframe::egui;

use crate::worker::Msg;

pub struct Entry {
    pub path: PathBuf,
    pub name: String,
    pub is_dir: bool,
}

/// Sub-folders first, then supported images, each sorted by name. Hidden files skipped.
pub fn list_folder(folder: &Path) -> Vec<Entry> {
    let Ok(read) = std::fs::read_dir(folder) else {
        return Vec::new();
    };
    let mut entries: Vec<Entry> = read
        .filter_map(Result::ok)
        .filter_map(|e| {
            let path = e.path();
            let name = e.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') {
                return None;
            }
            let is_dir = path.is_dir();
            (is_dir || bw_io::is_supported_image(&path)).then_some(Entry { path, name, is_dir })
        })
        .collect();
    entries.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    entries
}

/// Thumbnails to decode; a newer request replaces any unfinished one.
pub struct ThumbRequest {
    pub paths: Vec<PathBuf>,
}

pub const THUMB_EDGE: u32 = 96;

/// Decodes thumbnails one at a time, abandoning a folder as soon as a newer request arrives.
pub fn spawn_thumbnailer(rx: Receiver<ThumbRequest>, tx: Sender<Msg>, ctx: egui::Context) {
    std::thread::Builder::new()
        .name("thumbnails".into())
        .spawn(move || {
            let mut current: Option<ThumbRequest> = None;
            loop {
                if current.as_ref().is_none_or(|r| r.paths.is_empty()) {
                    match rx.recv() {
                        Ok(r) => current = Some(r),
                        Err(_) => return,
                    }
                }
                while let Ok(newer) = rx.try_recv() {
                    current = Some(newer);
                }
                let Some(req) = current.as_mut() else {
                    continue;
                };
                let Some(path) = req.paths.pop() else {
                    continue;
                };
                if let Ok((w, h, px)) = bw_io::load_thumbnail(&path, THUMB_EDGE) {
                    let image = egui::ColorImage::from_rgba_unmultiplied([w, h], &px);
                    if tx.send(Msg::Thumb { path, image }).is_err() {
                        return;
                    }
                    ctx.request_repaint();
                }
            }
        })
        .expect("spawn thumbnail thread");
}
