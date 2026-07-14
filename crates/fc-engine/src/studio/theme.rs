//! Retro dark theme for FC Studio: monospace everywhere, near-black panels,
//! amber accent — keeps the fantasy-console feel on a desktop UI.

use egui::{Color32, FontFamily, FontId, TextStyle};

pub const ACCENT: Color32 = Color32::from_rgb(255, 220, 60);
pub const ERROR: Color32 = Color32::from_rgb(235, 90, 90);
pub const OK: Color32 = Color32::from_rgb(90, 200, 110);
pub const DIM: Color32 = Color32::from_rgb(140, 140, 150);

pub fn apply(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();

    style.text_styles = [
        (TextStyle::Heading, FontId::new(18.0, FontFamily::Monospace)),
        (TextStyle::Body, FontId::new(14.0, FontFamily::Monospace)),
        (
            TextStyle::Monospace,
            FontId::new(14.0, FontFamily::Monospace),
        ),
        (TextStyle::Button, FontId::new(14.0, FontFamily::Monospace)),
        (TextStyle::Small, FontId::new(11.0, FontFamily::Monospace)),
    ]
    .into();

    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = Color32::from_rgb(16, 16, 20);
    visuals.window_fill = Color32::from_rgb(20, 20, 26);
    visuals.extreme_bg_color = Color32::from_rgb(10, 10, 13);
    visuals.faint_bg_color = Color32::from_rgb(24, 24, 30);
    visuals.selection.bg_fill = Color32::from_rgb(50, 80, 140);
    visuals.hyperlink_color = ACCENT;
    visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(16, 16, 20);
    visuals.widgets.inactive.bg_fill = Color32::from_rgb(30, 30, 38);
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(45, 45, 58);
    visuals.widgets.active.bg_fill = Color32::from_rgb(60, 60, 78);

    style.visuals = visuals;
    style.spacing.item_spacing = egui::vec2(8.0, 6.0);
    style.spacing.button_padding = egui::vec2(10.0, 4.0);

    ctx.set_style(style);
}
