//! Welcome screen: shown in the Code tab when no cart is open yet. Replaces
//! the old bare "no source open" heading with recents + templates, so a
//! newcomer has somewhere to start instead of a void.

use super::templates::{CartTemplate, TEMPLATES};
use super::theme;
use std::path::{Path, PathBuf};

/// Width reserved for the name column so every description starts at the
/// same x, monospace-font characters wide (fixed pixel width would need
/// exact glyph metrics; padding a monospace string is exact by construction).
const NAME_COLUMN_CHARS: usize = 18;

pub enum WelcomeAction {
    None,
    NewBlank,
    NewTemplate(&'static str),
    Open,
    OpenRecent(PathBuf),
}

pub fn show(ui: &mut egui::Ui, recent: &[PathBuf]) -> WelcomeAction {
    let mut action = WelcomeAction::None;

    ui.add_space(16.0);
    ui.vertical_centered(|ui| {
        ui.heading("CAIVEN STUDIO");
        ui.colored_label(theme::DIM, "a fantasy console for real Lua games");
    });
    ui.add_space(20.0);

    ui.horizontal(|ui| {
        ui.add_space(8.0);
        if ui.button("NEW CART").clicked() {
            action = WelcomeAction::NewBlank;
        }
        if ui.button("OPEN…").clicked() {
            action = WelcomeAction::Open;
        }
    });
    ui.add_space(16.0);

    // Stacked full-width sections, not side-by-side `ui.columns` — columns
    // only offset each sub-`Ui`'s starting cursor, they don't clip width, so
    // an unwrapped or wrapped-wider-than-expected description would overflow
    // straight into the other column and render on top of it.
    ui.colored_label(theme::DIM, "NEW FROM TEMPLATE");
    ui.add_space(4.0);
    for (
        i,
        CartTemplate {
            name,
            description,
            source,
        },
    ) in TEMPLATES.into_iter().enumerate()
    {
        let row = ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(format!("{name:<NAME_COLUMN_CHARS$}"))
                    .color(theme::ACCENT)
                    .monospace(),
            );
            // `.wrap()` is required: a `Label` inside `ui.horizontal()`
            // defaults to no-wrap (`Extend`) so rows of widgets don't
            // awkwardly break — without it, long descriptions were
            // overflowing past the panel edge instead of wrapping.
            ui.add(
                egui::Label::new(
                    egui::RichText::new(description)
                        .color(theme::DIM)
                        .monospace(),
                )
                .wrap(),
            );
        });
        let resp = ui.interact(
            row.response.rect,
            egui::Id::new(("welcome_template", i)),
            egui::Sense::click(),
        );
        if resp.hovered() {
            ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::PointingHand);
        }
        if resp.clicked() {
            action = WelcomeAction::NewTemplate(source);
        }
    }

    ui.add_space(16.0);
    ui.colored_label(theme::DIM, "RECENT");
    ui.add_space(4.0);
    if recent.is_empty() {
        ui.colored_label(theme::DIM, "(nothing opened yet)");
    }
    for (i, path) in recent.iter().enumerate() {
        let resp = ui.label(display_name(path));
        let resp = ui.interact(
            resp.rect,
            egui::Id::new(("welcome_recent", i)),
            egui::Sense::click(),
        );
        if resp.hovered() {
            ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::PointingHand);
        }
        if resp.clicked() {
            action = WelcomeAction::OpenRecent(path.clone());
        }
    }

    action
}

fn display_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}
