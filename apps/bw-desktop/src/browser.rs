//! Folder listing and background thumbnail loading.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender};

use eframe::egui;

use crate::worker::Msg;

pub struct Entry {
    pub path: PathBuf,
    pub name: String,
    pub is_dir: bool,
    pub kind: Kind,
}

/// What a listed entry is, for the browser's file-type filter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Folder,
    /// Camera raw (NEF, DNG, ARW): what the app is for, always listed.
    Raw,
    /// Lossless bitmaps (TIFF, PNG), e.g. scans or edited masters.
    Bitmap,
    /// Lossy compressed JPEGs.
    Jpeg,
}

/// The kind of an image file, or `None` for files the app can't open.
pub fn kind_of(path: &Path) -> Option<Kind> {
    if bw_raw::is_raw(path) {
        return Some(Kind::Raw);
    }
    if !bw_io::is_supported_image(path) {
        return None;
    }
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    Some(if matches!(ext.as_str(), "jpg" | "jpeg") {
        Kind::Jpeg
    } else {
        Kind::Bitmap
    })
}

/// Which non-raw files the browser lists. Folders and raw files are always listed, so a
/// raw-only view is the default.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Filter {
    pub bitmaps: bool,
    pub jpegs: bool,
}

impl Filter {
    pub fn shows(&self, kind: Kind) -> bool {
        match kind {
            Kind::Folder | Kind::Raw => true,
            Kind::Bitmap => self.bitmaps,
            Kind::Jpeg => self.jpegs,
        }
    }
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
            let kind = if is_dir {
                Kind::Folder
            } else {
                kind_of(&path)?
            };
            Some(Entry {
                path,
                name,
                is_dir,
                kind,
            })
        })
        .collect();
    entries.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    entries
}

/// Shorten `text` to fit `max_width` by replacing its middle with "…", keeping the start
/// and the end (frame number, extension) visible: the long-standing Mac convention for
/// file names. `measure` returns the drawn width of a string.
pub fn middle_ellipsis(text: &str, max_width: f32, measure: impl Fn(&str) -> f32) -> String {
    if measure(text) <= max_width {
        return text.to_string();
    }
    let chars: Vec<char> = text.chars().collect();
    let candidate = |keep: usize| {
        let tail = keep / 2;
        let head = keep - tail;
        let mut s: String = chars[..head].iter().collect();
        s.push('…');
        s.extend(&chars[chars.len() - tail..]);
        s
    };
    // Largest number of kept characters that still fits.
    let (mut lo, mut hi) = (0usize, chars.len());
    while lo < hi {
        let mid = (lo + hi).div_ceil(2);
        if measure(&candidate(mid)) <= max_width {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }
    candidate(lo)
}

/// Short upper-case file type for the list's right-hand tag ("NEF", "JPG", "TIF").
pub fn type_tag(path: &Path) -> String {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_uppercase();
    match ext.as_str() {
        "JPEG" => "JPG".into(),
        "TIFF" => "TIF".into(),
        _ => ext,
    }
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
                // Raw files: the camera's embedded preview, which is fast and already rendered.
                let thumb = if bw_raw::is_raw(&path) {
                    bw_raw::thumbnail(&path, THUMB_EDGE)
                } else {
                    bw_io::load_thumbnail(&path, THUMB_EDGE).ok()
                };
                if let Some((w, h, px)) = thumb {
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A fixed-width "font": every character is 1 unit wide.
    fn width(s: &str) -> f32 {
        s.chars().count() as f32
    }

    #[test]
    fn middle_ellipsis_keeps_start_and_end() {
        assert_eq!(middle_ellipsis("DSC_0040.NEF", 20.0, width), "DSC_0040.NEF");
        let short = middle_ellipsis("Holiday in Iceland 2024 - frame 0137.NEF", 15.0, width);
        assert_eq!(width(&short), 15.0);
        assert!(short.starts_with("Holiday") && short.ends_with(".NEF") && short.contains('…'));
        assert_eq!(middle_ellipsis("abcdef", 1.0, width), "…");
    }

    #[test]
    fn raw_is_always_listed_and_others_are_optional() {
        let kind = |name| kind_of(Path::new(name));
        assert_eq!(kind("a.NEF"), Some(Kind::Raw));
        assert_eq!(kind("a.dng"), Some(Kind::Raw));
        assert_eq!(kind("a.tiff"), Some(Kind::Bitmap));
        assert_eq!(kind("a.png"), Some(Kind::Bitmap));
        assert_eq!(kind("a.JPEG"), Some(Kind::Jpeg));
        assert_eq!(kind("a.txt"), None);
        let raw_only = Filter::default();
        assert!(raw_only.shows(Kind::Raw) && raw_only.shows(Kind::Folder));
        assert!(!raw_only.shows(Kind::Bitmap) && !raw_only.shows(Kind::Jpeg));
        let jpegs = Filter {
            jpegs: true,
            ..Filter::default()
        };
        assert!(jpegs.shows(Kind::Jpeg) && !jpegs.shows(Kind::Bitmap));
    }

    #[test]
    fn type_tags_are_short() {
        assert_eq!(type_tag(Path::new("a.nef")), "NEF");
        assert_eq!(type_tag(Path::new("a.jpeg")), "JPG");
        assert_eq!(type_tag(Path::new("a.TIFF")), "TIF");
    }
}
