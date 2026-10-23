//! Folder listing, the folder tree's places and subfolder cache, and background
//! thumbnail loading.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use eframe::egui;

use crate::worker::Msg;

/// An image in the folder being browsed.
pub struct Entry {
    pub path: PathBuf,
    pub name: String,
    pub kind: Kind,
}

/// What a listed image is, for the image strip's file-type filter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
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

/// Which non-raw files the image strip lists. Raw files always are, so a raw-only view is
/// the default.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Filter {
    pub bitmaps: bool,
    pub jpegs: bool,
}

impl Filter {
    pub fn shows(&self, kind: Kind) -> bool {
        match kind {
            Kind::Raw => true,
            Kind::Bitmap => self.bitmaps,
            Kind::Jpeg => self.jpegs,
        }
    }
}

/// The folder's supported images, sorted by name. Hidden files and folders skipped.
pub fn list_folder(folder: &Path) -> Vec<Entry> {
    let Ok(read) = std::fs::read_dir(folder) else {
        return Vec::new();
    };
    let mut entries: Vec<Entry> = read
        .filter_map(Result::ok)
        .filter_map(|e| {
            let path = e.path();
            let name = e.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') || path.is_dir() {
                return None;
            }
            let kind = kind_of(&path)?;
            Some(Entry { path, name, kind })
        })
        .collect();
    entries.sort_by_key(|e| e.name.to_lowercase());
    entries
}

/// Extensions of macOS packages: folders on disk that Finder shows as a single item
/// (apps, libraries, documents). They never hold photos to browse, so the folder tree
/// leaves them out.
const PACKAGE_EXTENSIONS: &[&str] = &[
    "app",
    "appex",
    "bundle",
    "framework",
    "kext",
    "plugin",
    "prefpane",
    "qlgenerator",
    "saver",
    "xpc",
    "photoslibrary",
    "photolibrary",
    "migratedphotolibrary",
    "aplibrary",
    "musiclibrary",
    "tvlibrary",
    "imovielibrary",
    "fcpbundle",
    "lrdata",
    "lrlibrary",
    "logicx",
    "band",
    "pages",
    "numbers",
    "key",
    "rtfd",
    "xcodeproj",
    "xcworkspace",
    "playground",
    "pkg",
    "mpkg",
];

/// Whether `path` names a macOS package (by its extension), such as `Photos Library.photoslibrary`
/// or `Safari.app`.
pub fn is_package(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| PACKAGE_EXTENSIONS.iter().any(|p| p.eq_ignore_ascii_case(e)))
}

/// A folder in the tree: its display name and path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Folder {
    pub name: String,
    pub path: PathBuf,
}

/// The folder's subfolders, sorted by name; hidden folders and packages left out.
pub fn list_subfolders(folder: &Path) -> Vec<Folder> {
    let Ok(read) = std::fs::read_dir(folder) else {
        return Vec::new();
    };
    let mut folders: Vec<Folder> = read
        .filter_map(Result::ok)
        .filter_map(|e| {
            let path = e.path();
            let name = e.file_name().to_string_lossy().into_owned();
            // `path.is_dir` follows symbolic links, as Finder does.
            (!name.starts_with('.') && path.is_dir() && !is_package(&path))
                .then_some(Folder { name, path })
        })
        .collect();
    folders.sort_by_key(|f| f.name.to_lowercase());
    folders
}

/// A top-level folder of the tree; `drive` marks a mounted volume (drawn with a drive icon).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Place {
    pub folder: Folder,
    pub drive: bool,
}

/// The tree's top-level folders: Pictures, Desktop and Home, then mounted volumes such as
/// memory cards and external drives (the start-up disk is left out: it is reached through
/// Home).
pub fn places() -> Vec<Place> {
    let mut out: Vec<Place> = [
        ("Pictures", dirs::picture_dir()),
        ("Desktop", dirs::desktop_dir()),
        ("Home", dirs::home_dir()),
    ]
    .into_iter()
    .filter_map(|(name, path)| {
        let path = path.filter(|p| p.is_dir())?;
        Some(Place {
            folder: Folder {
                name: name.into(),
                path,
            },
            drive: false,
        })
    })
    .collect();
    out.extend(
        list_subfolders(Path::new("/Volumes"))
            .into_iter()
            .filter_map(|folder| {
                // The start-up disk appears in /Volumes as a link to "/".
                let target = std::fs::canonicalize(&folder.path).ok()?;
                (target != Path::new("/")).then_some(Place {
                    folder,
                    drive: true,
                })
            }),
    );
    out
}

/// Subfolder listings for the folder tree, made on a worker thread so a slow drive or a
/// sleeping network share never holds up a frame, and refreshed every few seconds while
/// shown.
pub struct FolderCache {
    entries: Arc<Mutex<HashMap<PathBuf, Listing>>>,
    ctx: egui::Context,
}

#[derive(Default)]
struct Listing {
    folders: Option<Vec<Folder>>,
    listed: Option<Instant>,
    running: bool,
}

const LISTING_REFRESH: Duration = Duration::from_secs(3);

impl FolderCache {
    pub fn new(ctx: egui::Context) -> Self {
        Self {
            entries: Arc::default(),
            ctx,
        }
    }

    /// The subfolders of `path`, or `None` until they have been listed once.
    pub fn subfolders(&self, path: &Path) -> Option<Vec<Folder>> {
        let mut map = self.entries.lock().unwrap_or_else(PoisonError::into_inner);
        let entry = map.entry(path.to_path_buf()).or_default();
        let due = !entry.running && entry.listed.is_none_or(|t| t.elapsed() >= LISTING_REFRESH);
        if due {
            entry.running = true;
            entry.listed = Some(Instant::now());
            let (entries, ctx, path) = (self.entries.clone(), self.ctx.clone(), path.to_path_buf());
            let spawned = std::thread::Builder::new()
                .name("folder-list".into())
                .spawn(move || {
                    let folders = list_subfolders(&path);
                    let mut map = entries.lock().unwrap_or_else(PoisonError::into_inner);
                    let entry = map.entry(path).or_default();
                    entry.folders = Some(folders);
                    entry.running = false;
                    drop(map);
                    ctx.request_repaint();
                });
            if spawned.is_err() {
                entry.running = false;
            }
        }
        entry.folders.clone()
    }
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

/// Long edge of decoded thumbnails, in pixels: sharp in the image strip on Retina screens.
pub const THUMB_EDGE: u32 = 256;

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
        assert!(raw_only.shows(Kind::Raw));
        assert!(!raw_only.shows(Kind::Bitmap) && !raw_only.shows(Kind::Jpeg));
        let jpegs = Filter {
            jpegs: true,
            ..Filter::default()
        };
        assert!(jpegs.shows(Kind::Jpeg) && !jpegs.shows(Kind::Bitmap));
    }

    #[test]
    fn packages_are_not_folders_to_browse() {
        assert!(is_package(Path::new("/Applications/Safari.app")));
        assert!(is_package(Path::new("Photos Library.photoslibrary")));
        assert!(is_package(Path::new("Catalog Previews.LRDATA")));
        assert!(!is_package(Path::new("/Users/me/Pictures/2024")));
        assert!(!is_package(Path::new("Iceland.trip")));
    }

    #[test]
    fn subfolders_skip_packages_hidden_folders_and_files() {
        let root = std::env::temp_dir().join(format!("bw-subfolders-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        for dir in ["b roll", "A trip", "Viewer.app/Contents", ".cache"] {
            std::fs::create_dir_all(root.join(dir)).unwrap();
        }
        std::fs::write(root.join("frame.NEF"), b"").unwrap();
        let names: Vec<String> = list_subfolders(&root).into_iter().map(|f| f.name).collect();
        assert_eq!(names, ["A trip", "b roll"]);
        // Images only: no folders, packages or hidden files.
        let images: Vec<String> = list_folder(&root).into_iter().map(|e| e.name).collect();
        assert_eq!(images, ["frame.NEF"]);
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn type_tags_are_short() {
        assert_eq!(type_tag(Path::new("a.nef")), "NEF");
        assert_eq!(type_tag(Path::new("a.jpeg")), "JPG");
        assert_eq!(type_tag(Path::new("a.TIFF")), "TIF");
    }
}
