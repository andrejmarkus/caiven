//! Right-hand game view: the console framebuffer as an integer-scaled
//! nearest-neighbor texture.

use super::app::RunState;
use super::theme;

pub fn show(
    ui: &mut egui::Ui,
    tex: Option<&egui::TextureHandle>,
    native_size: f32,
    run_state: RunState,
) {
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
        ui.add(image);

        let hint = match run_state {
            RunState::Running => "ARROWS/WASD MOVE · J/K BUTTONS",
            RunState::Paused => "PAUSED",
            RunState::Stopped => "STOPPED",
        };
        ui.colored_label(theme::DIM, hint);
    });
}
