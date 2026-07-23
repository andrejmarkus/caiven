//! Welcome screen: shown in the Code tab when no cart is open yet. Replaces
//! the old bare "no source open" heading with recents + templates, so a
//! newcomer has somewhere to start instead of a void.

use super::templates::{CartTemplate, TEMPLATES};
use super::theme;
use egui::text::LayoutJob;
use egui::{FontId, TextFormat};
use std::path::{Path, PathBuf};

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

    ui.columns(2, |cols| {
        cols[0].colored_label(theme::DIM, "NEW FROM TEMPLATE");
        cols[0].add_space(4.0);
        for CartTemplate {
            name,
            description,
            source,
        } in TEMPLATES
        {
            if cols[0]
                .selectable_label(false, template_row(name, description))
                .clicked()
            {
                action = WelcomeAction::NewTemplate(source);
            }
        }

        cols[1].colored_label(theme::DIM, "RECENT");
        cols[1].add_space(4.0);
        if recent.is_empty() {
            cols[1].colored_label(theme::DIM, "(nothing opened yet)");
        }
        for path in recent {
            let label = display_name(path);
            if cols[1].selectable_label(false, label).clicked() {
                action = WelcomeAction::OpenRecent(path.clone());
            }
        }
    });

    action
}

fn display_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

/// Name in accent/bold, description dimmed — so the two don't read as one
/// run-on phrase (the plain `"{name:<16}{description}"` string they replace
/// rendered as flat, same-weight text with no visual break between them).
fn template_row(name: &str, description: &str) -> LayoutJob {
    let mut job = LayoutJob::default();
    let font_id = FontId::monospace(14.0);
    job.append(
        &format!("{name:<18}"),
        0.0,
        TextFormat {
            font_id: font_id.clone(),
            color: theme::ACCENT,
            ..Default::default()
        },
    );
    job.append(
        description,
        0.0,
        TextFormat {
            font_id,
            color: theme::DIM,
            ..Default::default()
        },
    );
    job
}
