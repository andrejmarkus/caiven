//! Right-hand game view: the console framebuffer as an integer-scaled
//! nearest-neighbor texture. When paused on a Lua fault, draws an overlay
//! explaining the error over the frozen frame instead of leaving it
//! silently stuck.

use super::app::RunState;
use super::debug_panel::LuaError;
use super::theme;

/// What the caller should do after `show` returns.
#[derive(PartialEq, Eq)]
pub enum GamePanelAction {
    None,
    /// User clicked "JUMP TO LINE" on the error overlay.
    JumpToError,
}

pub fn show(
    ui: &mut egui::Ui,
    tex: Option<&egui::TextureHandle>,
    native_size: f32,
    run_state: RunState,
    error: Option<&LuaError>,
) -> GamePanelAction {
    let mut action = GamePanelAction::None;
    ui.vertical_centered(|ui| {
        ui.add_space(4.0);
        let Some(tex) = tex else {
            ui.colored_label(theme::DIM, "no cart loaded");
            return;
        };

        let avail = ui.available_size();
        let scale = ((avail.x.min(avail.y - 24.0)) / native_size)
            .floor()
            .max(1.0);
        let size = egui::vec2(native_size * scale, native_size * scale);

        let image = egui::Image::from_texture(tex).fit_to_exact_size(size);
        let resp = ui.add(image);

        if run_state == RunState::Paused
            && let Some(err) = error
        {
            action = error_overlay(ui, resp.rect, err);
        }

        let hint = match run_state {
            RunState::Running => "ARROWS/WASD MOVE · J/K BUTTONS",
            RunState::Paused => "PAUSED",
            RunState::Stopped => "STOPPED",
        };
        ui.colored_label(theme::DIM, hint);
    });
    action
}

/// Draws a dark scrim + error text over the frozen game frame, with a
/// jump-to-line button when the fault's source line is known.
fn error_overlay(ui: &mut egui::Ui, rect: egui::Rect, err: &LuaError) -> GamePanelAction {
    ui.painter()
        .rect_filled(rect, 0.0, egui::Color32::from_black_alpha(215));

    let mut action = GamePanelAction::None;
    ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(rect.height() * 0.25);
            ui.colored_label(theme::ERROR, "SCRIPT ERROR");
            ui.add_space(4.0);
            ui.add(
                egui::Label::new(
                    egui::RichText::new(&err.message)
                        .monospace()
                        .color(theme::TEXT),
                )
                .wrap(),
            );
            ui.add_space(8.0);
            if err.line.is_some() && ui.button("JUMP TO LINE").clicked() {
                action = GamePanelAction::JumpToError;
            }
        });
    });
    action
}
