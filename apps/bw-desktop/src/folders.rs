//! The left panel: a collapsible tree of folders to browse. Choosing a folder fills the
//! image strip under the photo; the tree itself never lists image files.
//!
//! The row, disclosure chevron and "open on the way down" behaviour are adapted from
//! LightCraft's `crates/ui-egui/src/panels/left.rs` (https://github.com/storytold/lightcraft),
//! Copyright (c) 2026 ArtCraft Team and the LightCraft contributors, licensed under the MIT
//! License (see `assets/LICENSE-LightCraft-MIT`). Modified for BW Darkroom: no catalog,
//! counts or automation registry, and macOS packages (`.app` and the like) left out.

use std::path::{Path, PathBuf};

use eframe::egui::{self, Align2, Id, Rect, Sense, Ui, pos2, vec2};

use crate::browser::{self, Folder, FolderCache, Place};
use crate::theme::Tokens;
use crate::widgets::{self, Icon, paint_icon};

/// Height of a tree row.
const ROW_HEIGHT: f32 = 29.0;
/// Indent per tree level.
const INDENT: f32 = 16.0;
/// Row layout from the panel's left edge: the chevron's centre lines up with the
/// Folders header's chevron, and each level indents by one icon.
const CHEVRON_X: f32 = 22.0;
const ICON_X: f32 = 32.0;
const LABEL_X: f32 = 54.0;
/// Deepest level the tree opens to.
const MAX_DEPTH: usize = 12;

/// What the user asked for in the tree.
pub enum Action {
    /// Browse this folder: its images go to the image strip.
    Browse(PathBuf),
    /// Choose a folder with the system dialog.
    PickFolder,
}

/// The folder tree: the places (Pictures, Desktop, Home, mounted drives), each opening
/// onto its subfolders, with the folder being browsed highlighted and the folders on the
/// way down to it opened. A folder outside every place is listed as a place of its own.
pub fn show(ui: &mut Ui, cache: &FolderCache, places: &[Place], current: &Path) -> Option<Action> {
    let mut action = None;
    let mut roots: Vec<Place> = places.to_vec();
    if !roots.iter().any(|p| current.starts_with(&p.folder.path)) {
        roots.insert(
            0,
            Place {
                folder: Folder {
                    name: label(current),
                    path: current.to_path_buf(),
                },
                drive: false,
            },
        );
    }
    // The tree that holds the current folder opens down to it: the innermost place, so
    // Pictures (not Home) when browsing inside Pictures.
    let owner = roots
        .iter()
        .enumerate()
        .filter(|(_, p)| current.starts_with(&p.folder.path))
        .max_by_key(|(_, p)| p.folder.path.components().count())
        .map(|(i, _)| i);

    egui::ScrollArea::vertical()
        .auto_shrink(false)
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            if widgets::group_header(ui, "folders.places", "Folders", roots.len()) {
                for (i, place) in roots.iter().enumerate() {
                    let icon = if place.drive {
                        Icon::Drive
                    } else {
                        Icon::Folder
                    };
                    let root = Tree {
                        cache,
                        place: &place.folder.path,
                        current,
                        reveal: owner == Some(i),
                    };
                    root.node(ui, &place.folder, icon, 0, &mut action);
                }
                let (resp, _) = row(ui, "browse", Icon::Plus, "Browse Folder…", false, 0.0);
                if resp
                    .on_hover_text("Choose any folder, for example on a network drive")
                    .clicked()
                {
                    action = Some(Action::PickFolder);
                }
            }
        });
    action
}

struct Tree<'a> {
    cache: &'a FolderCache,
    /// The top-level folder this tree hangs from: a folder listed in two trees (Pictures
    /// on its own and inside Home) opens and closes separately in each.
    place: &'a Path,
    current: &'a Path,
    /// This tree holds the current folder: open the folders on the way down to it.
    reveal: bool,
}

impl Tree<'_> {
    fn node(
        &self,
        ui: &mut Ui,
        folder: &Folder,
        icon: Icon,
        depth: usize,
        action: &mut Option<Action>,
    ) {
        let path = &folder.path;
        let open_id = Id::unique(("bw-folder-open", self.place, path));
        let mut open = ui.data_mut(|d| *d.get_persisted_mut_or(open_id, false));
        let selected = self.current == path;
        if self.reveal && !selected && self.current.starts_with(path) {
            // Opened once per browsed folder: closing it again afterwards sticks.
            let revealed_id = Id::unique(("bw-folder-revealed", self.place, path));
            let target = self.current.to_path_buf();
            if ui.data(|d| d.get_temp::<PathBuf>(revealed_id)).as_ref() != Some(&target) {
                open = true;
                ui.data_mut(|d| d.insert_temp(revealed_id, target));
            }
        }
        // Listing a row's subfolders while it is drawn tells whether it gets a chevron.
        let children = self.cache.subfolders(path);
        let has_children = children.as_ref().is_none_or(|c| !c.is_empty());
        let indent = depth as f32 * INDENT;
        // Keyed by place and path: a folder listed in two trees is two rows, and with one
        // id the copy drawn last would take both rows' clicks.
        let key = format!("{}\n{}", self.place.display(), path.display());
        let (resp, rect) = row(ui, &key, icon, &folder.name, selected, indent);
        let mut chevron_clicked = false;
        if has_children {
            let t = Tokens::get(ui.ctx());
            let c = pos2(rect.left() + CHEVRON_X + indent, rect.center().y);
            let hit = Rect::from_center_size(c, vec2(16.0, 16.0));
            let tri = ui.interact(hit, open_id.with("chevron"), Sense::click());
            let colour = if tri.hovered() { t.text } else { t.text_dim };
            let chevron = if open {
                Icon::ChevronDown
            } else {
                Icon::ChevronRight
            };
            paint_icon(
                ui.painter(),
                Rect::from_center_size(c, vec2(14.0, 14.0)),
                chevron,
                colour,
            );
            let tri = tri.on_hover_text(if open {
                "Hide subfolders"
            } else {
                "Show subfolders"
            });
            if tri.clicked() {
                open = !open;
                chevron_clicked = true;
            }
        }
        ui.data_mut(|d| d.insert_persisted(open_id, open));
        let resp = resp.on_hover_text(format!(
            "{}\nClick to show its photos below the image.",
            path.display()
        ));
        if !chevron_clicked && resp.clicked() {
            *action = Some(Action::Browse(path.clone()));
        }
        if resp.double_clicked() && has_children {
            ui.data_mut(|d| d.insert_persisted(open_id, !open));
        }
        resp.context_menu(|ui| {
            if ui
                .button("Show in Finder")
                .on_hover_text("Open this folder in a Finder window")
                .clicked()
            {
                let _ = std::process::Command::new("open").arg(path).spawn();
                ui.close();
            }
        });
        if open && depth < MAX_DEPTH {
            for child in children.unwrap_or_default() {
                self.node(ui, &child, Icon::Folder, depth + 1, action);
            }
        }
    }
}

/// One tree row: the full panel width, a rounded highlight when selected or hovered, the
/// icon after the chevron's gutter, and the name shortened to fit (in full on hover).
fn row(
    ui: &mut Ui,
    key: &str,
    icon: Icon,
    name: &str,
    selected: bool,
    indent: f32,
) -> (egui::Response, Rect) {
    let t = Tokens::get(ui.ctx());
    let id = Id::unique(("bw-folder-row", key));
    let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), ROW_HEIGHT), Sense::hover());
    let resp = ui.interact(r, id, Sense::click());
    let inner = Rect::from_min_max(
        r.min + vec2(8.0, 0.0),
        pos2((r.right() - 8.0).max(r.left() + 8.0), r.bottom()),
    );
    if selected {
        ui.painter().rect_filled(inner, 4.0, t.canvas);
    } else if resp.hovered() {
        ui.painter()
            .rect_filled(inner, 4.0, t.hover.gamma_multiply(0.6));
    }
    paint_icon(
        ui.painter(),
        Rect::from_min_size(
            pos2(r.left() + ICON_X + indent, r.center().y - 8.0),
            vec2(16.0, 16.0),
        ),
        icon,
        if selected { t.text } else { t.icon },
    );
    let font = t.font(13.5);
    let colour = if selected { t.text } else { t.text_label };
    let left = r.left() + LABEL_X + indent;
    let room = (r.right() - 16.0 - left).max(0.0);
    let measure = |s: &str| {
        ui.painter()
            .layout_no_wrap(s.to_string(), font.clone(), colour)
            .size()
            .x
    };
    let shown = browser::middle_ellipsis(name, room, measure);
    ui.painter().text(
        pos2(left, r.center().y),
        Align2::LEFT_CENTER,
        &shown,
        font,
        colour,
    );
    (resp, r)
}

/// A folder's name for display ("/" for the disk's root).
fn label(path: &Path) -> String {
    path.file_name().map_or_else(
        || path.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    )
}
