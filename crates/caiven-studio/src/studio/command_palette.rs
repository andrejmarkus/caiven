//! Ctrl+P / Ctrl+Shift+P fuzzy-ish action palette: a single searchable list
//! over every menu/toolbar action, tab switch, "new from template", and
//! "insert builtin" — the single biggest "feels like a pro IDE" signal per
//! the Studio UX roadmap's Phase 3.

use super::app::Tab;
use super::templates;
use caiven_vm::vm::api_registry::{BUILTINS, STDLIB};

#[derive(Clone)]
pub enum PaletteAction {
    New,
    NewTemplate(&'static str),
    Open,
    Save,
    SaveAs,
    Close,
    Exit,
    Run,
    Pause,
    Reset,
    SwitchTab(Tab),
    InsertBuiltin(String),
    ExportScreenshot,
    ExportGif,
}

struct Entry {
    label: String,
    action: PaletteAction,
}

#[derive(Default)]
pub struct PaletteState {
    open: bool,
    query: String,
    selected: usize,
    just_opened: bool,
}

impl PaletteState {
    fn toggle(&mut self) {
        self.open = !self.open;
        if self.open {
            self.query.clear();
            self.selected = 0;
            self.just_opened = true;
        }
    }
}

fn entries(running: bool) -> Vec<Entry> {
    let mut v = vec![
        Entry {
            label: "New Cart".into(),
            action: PaletteAction::New,
        },
        Entry {
            label: "Open...".into(),
            action: PaletteAction::Open,
        },
        Entry {
            label: "Save".into(),
            action: PaletteAction::Save,
        },
        Entry {
            label: "Save As...".into(),
            action: PaletteAction::SaveAs,
        },
        Entry {
            label: "Close Cart".into(),
            action: PaletteAction::Close,
        },
        Entry {
            label: "Exit".into(),
            action: PaletteAction::Exit,
        },
        Entry {
            label: if running {
                "Pause".into()
            } else {
                "Run".into()
            },
            action: if running {
                PaletteAction::Pause
            } else {
                PaletteAction::Run
            },
        },
        Entry {
            label: "Reset".into(),
            action: PaletteAction::Reset,
        },
        Entry {
            label: "Export Screenshot (PNG)".into(),
            action: PaletteAction::ExportScreenshot,
        },
        Entry {
            label: "Export GIF (3s)".into(),
            action: PaletteAction::ExportGif,
        },
    ];
    for tab in Tab::ALL {
        v.push(Entry {
            label: format!("Go to tab: {}", tab.label()),
            action: PaletteAction::SwitchTab(tab),
        });
    }
    for t in templates::TEMPLATES {
        v.push(Entry {
            label: format!("New from template: {}", t.name),
            action: PaletteAction::NewTemplate(t.source),
        });
    }
    for entry in BUILTINS.iter().chain(STDLIB.iter()) {
        let args = entry
            .params
            .iter()
            .map(|p| p.name)
            .collect::<Vec<_>>()
            .join(", ");
        let call = format!("{}({args})", entry.name);
        v.push(Entry {
            label: format!("Insert: {call}"),
            action: PaletteAction::InsertBuiltin(call),
        });
    }
    v
}

/// Handles the Ctrl+P/Ctrl+Shift+P toggle, renders the overlay when open,
/// and returns the chosen action (if any) for the caller to dispatch through
/// the same handlers `MenuAction`/`ToolbarAction` already use.
pub fn show(ctx: &egui::Context, state: &mut PaletteState, running: bool) -> Option<PaletteAction> {
    if ctx.input_mut(|i| {
        i.consume_key(egui::Modifiers::CTRL, egui::Key::P)
            || i.consume_key(egui::Modifiers::CTRL | egui::Modifiers::SHIFT, egui::Key::P)
    }) {
        state.toggle();
    }
    if !state.open {
        return None;
    }
    if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
        state.open = false;
        return None;
    }

    let all = entries(running);
    let query = state.query.to_lowercase();
    let filtered: Vec<&Entry> = all
        .iter()
        .filter(|e| query.is_empty() || e.label.to_lowercase().contains(&query))
        .take(30)
        .collect();
    if state.selected >= filtered.len() {
        state.selected = filtered.len().saturating_sub(1);
    }

    if !filtered.is_empty()
        && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown))
    {
        state.selected = (state.selected + 1) % filtered.len();
    }
    if !filtered.is_empty()
        && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp))
    {
        state.selected = (state.selected + filtered.len() - 1) % filtered.len();
    }
    let enter = ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter));

    let mut result = None;
    egui::Area::new(egui::Id::new("command_palette"))
        .anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, 60.0))
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.set_width(480.0);
                let resp = ui.add(
                    egui::TextEdit::singleline(&mut state.query)
                        .id(egui::Id::new("command_palette_query"))
                        .desired_width(460.0)
                        .hint_text("Type a command..."),
                );
                if state.just_opened {
                    resp.request_focus();
                    state.just_opened = false;
                }
                ui.separator();
                egui::ScrollArea::vertical()
                    .max_height(320.0)
                    .show(ui, |ui| {
                        for (i, e) in filtered.iter().enumerate() {
                            let clicked =
                                ui.selectable_label(i == state.selected, &e.label).clicked();
                            if clicked || (enter && i == state.selected) {
                                result = Some(e.action.clone());
                            }
                        }
                    });
            });
        });

    if result.is_some() {
        state.open = false;
    }
    result
}
