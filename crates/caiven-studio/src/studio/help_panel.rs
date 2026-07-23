//! API reference panel (Help tab): searchable list of every console builtin
//! and leaned-on Lua stdlib member, read straight from
//! `caiven_vm::vm::api_registry` (the single source of truth intellisense
//! already uses). Clicking a row inserts a ready-to-fill call at the code
//! editor's cursor.

use super::theme;
use caiven_vm::vm::api_registry::{ApiEntry, BUILTINS, STDLIB};

#[derive(Default)]
pub struct HelpState {
    query: String,
}

pub enum HelpAction {
    None,
    Insert(String),
}

pub fn show(ui: &mut egui::Ui, state: &mut HelpState) -> HelpAction {
    let mut action = HelpAction::None;

    ui.horizontal(|ui| {
        ui.colored_label(theme::ACCENT, "API Reference");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.colored_label(theme::DIM, "click a row to insert at the code cursor");
        });
    });
    ui.horizontal(|ui| {
        ui.colored_label(theme::DIM, "search:");
        ui.add(
            egui::TextEdit::singleline(&mut state.query)
                .desired_width(240.0)
                .hint_text("filter by name or description"),
        );
        if ui.button("✕").clicked() {
            state.query.clear();
        }
    });
    ui.separator();

    let query = state.query.to_lowercase();
    let matches = |e: &ApiEntry| {
        query.is_empty()
            || e.name.to_lowercase().contains(&query)
            || e.doc.to_lowercase().contains(&query)
    };

    egui::ScrollArea::vertical().show(ui, |ui| {
        section(ui, "Console API", BUILTINS, &matches, &mut action);
        ui.add_space(10.0);
        section(ui, "Lua Standard Library", STDLIB, &matches, &mut action);
    });

    action
}

fn section(
    ui: &mut egui::Ui,
    title: &str,
    entries: &'static [ApiEntry],
    matches: &impl Fn(&ApiEntry) -> bool,
    action: &mut HelpAction,
) {
    let visible: Vec<&ApiEntry> = entries.iter().filter(|e| matches(e)).collect();
    if visible.is_empty() {
        return;
    }
    ui.colored_label(theme::ACCENT, title);
    for entry in visible {
        if row(ui, entry) {
            *action = HelpAction::Insert(call_signature(entry));
        }
    }
}

/// Draws one entry's signature (clickable) and its doc wrapped below,
/// returns `true` if clicked. Stacked rather than side-by-side in a
/// `ui.horizontal` — an egui `Label` there defaults to no-wrap and clips at
/// the panel edge instead of wrapping (the same bug hit in the welcome
/// screen's template list).
fn row(ui: &mut egui::Ui, entry: &ApiEntry) -> bool {
    let params = entry
        .params
        .iter()
        .map(|p| format!("{}: {}", p.name, p.ty))
        .collect::<Vec<_>>()
        .join(", ");
    let signature = format!("{}({}) -> {}", entry.name, params, entry.returns);

    let resp = ui.add(
        egui::Label::new(
            egui::RichText::new(signature)
                .monospace()
                .color(theme::BUILTIN),
        )
        .sense(egui::Sense::click()),
    );
    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    ui.add(egui::Label::new(egui::RichText::new(entry.doc).color(theme::DIM)).wrap());
    ui.add_space(4.0);
    resp.clicked()
}

/// The text spliced into the code editor on click: the call with param
/// names as fill-in placeholders, e.g. `sprite(sprite_id, x, y)`.
fn call_signature(entry: &ApiEntry) -> String {
    let args = entry
        .params
        .iter()
        .map(|p| p.name)
        .collect::<Vec<_>>()
        .join(", ");
    format!("{}({args})", entry.name)
}
