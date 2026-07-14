//! Top toolbar: run controls, cart name and FPS readout.

use super::app::RunState;
use super::theme;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ToolbarAction {
    None,
    Run,
    Pause,
    Reset,
    Save,
}

pub fn show(
    ctx: &egui::Context,
    cart_name: &str,
    run_state: RunState,
    fps: f32,
) -> ToolbarAction {
    let mut action = ToolbarAction::None;

    egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
        ui.horizontal(|ui| {
            let running = run_state == RunState::Running;

            let run_label = if running { "⏸ PAUSE" } else { "▶ RUN" };
            if ui.button(run_label).clicked() {
                action = if running {
                    ToolbarAction::Pause
                } else {
                    ToolbarAction::Run
                };
            }
            if ui.button("⟳ RESET").clicked() {
                action = ToolbarAction::Reset;
            }
            if ui.button("💾 SAVE").clicked() {
                action = ToolbarAction::Save;
            }

            ui.separator();
            ui.colored_label(theme::ACCENT, cart_name);
            match run_state {
                RunState::Running => ui.colored_label(theme::OK, "RUNNING"),
                RunState::Paused => ui.colored_label(theme::DIM, "PAUSED"),
                RunState::Stopped => ui.colored_label(theme::DIM, "STOPPED"),
            };

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.colored_label(theme::DIM, format!("{fps:>3.0} FPS"));
            });
        });
    });

    action
}
