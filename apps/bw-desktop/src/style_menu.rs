//! The "…" menu for a style (Styles list) and the same menu on a gallery tile's
//! right-click. The menu only reports what was chosen; the app carries it out.

use eframe::egui;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StyleAction {
    Edit(usize),
    Duplicate(usize),
    Rename(usize),
    Export(usize),
    MoveUp(usize),
    MoveDown(usize),
    Delete(usize),
}

/// Menu entries for style `index`. `can_up`/`can_down` say whether it can be reordered.
pub fn items(
    ui: &mut egui::Ui,
    index: usize,
    builtin: bool,
    can_up: bool,
    can_down: bool,
) -> Option<StyleAction> {
    let mut action = None;
    let read_only = "Built-in styles are read-only: duplicate one to change it";
    if ui
        .button("Edit…")
        .on_hover_text("Open every setting of this style in an editor, with a live preview")
        .clicked()
    {
        action = Some(StyleAction::Edit(index));
    }
    if ui
        .button("Duplicate")
        .on_hover_text("Make an editable copy of this style")
        .clicked()
    {
        action = Some(StyleAction::Duplicate(index));
    }
    if ui
        .add_enabled(!builtin, egui::Button::new("Rename…"))
        .on_hover_text("Give this style a new name")
        .on_disabled_hover_text(read_only)
        .clicked()
    {
        action = Some(StyleAction::Rename(index));
    }
    if ui
        .button("Export…")
        .on_hover_text("Save this style as a .json file to share or back up")
        .clicked()
    {
        action = Some(StyleAction::Export(index));
    }
    ui.separator();
    if ui
        .add_enabled(can_up, egui::Button::new("Move up"))
        .on_hover_text("Move this style up the list")
        .on_disabled_hover_text("Only your own styles can be reordered, below the built-ins")
        .clicked()
    {
        action = Some(StyleAction::MoveUp(index));
    }
    if ui
        .add_enabled(can_down, egui::Button::new("Move down"))
        .on_hover_text("Move this style down the list")
        .on_disabled_hover_text("Only your own styles can be reordered, below the built-ins")
        .clicked()
    {
        action = Some(StyleAction::MoveDown(index));
    }
    ui.separator();
    if ui
        .add_enabled(!builtin, egui::Button::new("Delete…"))
        .on_hover_text("Remove this style (asks first)")
        .on_disabled_hover_text(read_only)
        .clicked()
    {
        action = Some(StyleAction::Delete(index));
    }
    action
}
