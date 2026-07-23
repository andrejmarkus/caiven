//! Top menu bar: File menu (New/Open/Save/Save As/Close/Exit/Recent).

use std::path::{Path, PathBuf};

#[derive(Clone, PartialEq, Eq)]
pub enum MenuAction {
    None,
    New,
    Open,
    OpenRecent(PathBuf),
    ClearRecent,
    Save,
    SaveAs,
    ExportScreenshot,
    ExportGif,
    Close,
    Exit,
}

fn item(ui: &mut egui::Ui, label: &str, shortcut: &str) -> bool {
    ui.add(egui::Button::new(label).shortcut_text(shortcut))
        .clicked()
}

fn recent_label(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

pub fn show(ctx: &egui::Context, recent: &[PathBuf]) -> MenuAction {
    let mut action = MenuAction::None;

    egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button("File", |ui| {
                if item(ui, "New", "Ctrl+N") {
                    action = MenuAction::New;
                    ui.close();
                }
                if item(ui, "Open...", "Ctrl+O") {
                    action = MenuAction::Open;
                    ui.close();
                }
                ui.menu_button("Open Recent", |ui| {
                    if recent.is_empty() {
                        ui.label("(empty)");
                    } else {
                        for path in recent {
                            if ui
                                .button(recent_label(path))
                                .on_hover_text(path.display().to_string())
                                .clicked()
                            {
                                action = MenuAction::OpenRecent(path.clone());
                                ui.close();
                            }
                        }
                        ui.separator();
                        if ui.button("Clear Recently Opened").clicked() {
                            action = MenuAction::ClearRecent;
                            ui.close();
                        }
                    }
                });
                ui.separator();
                if item(ui, "Save", "Ctrl+S") {
                    action = MenuAction::Save;
                    ui.close();
                }
                if item(ui, "Save As...", "Ctrl+Shift+S") {
                    action = MenuAction::SaveAs;
                    ui.close();
                }
                ui.separator();
                ui.menu_button("Export", |ui| {
                    if ui.button("Screenshot (PNG)...").clicked() {
                        action = MenuAction::ExportScreenshot;
                        ui.close();
                    }
                    if ui.button("Record GIF (3s)...").clicked() {
                        action = MenuAction::ExportGif;
                        ui.close();
                    }
                });
                ui.separator();
                if ui.button("Close").clicked() {
                    action = MenuAction::Close;
                    ui.close();
                }
                ui.separator();
                if ui.button("Exit").clicked() {
                    action = MenuAction::Exit;
                    ui.close();
                }
            });
        });
    });

    action
}
