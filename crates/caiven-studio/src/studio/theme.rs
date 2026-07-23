//! Retro dark theme for Caiven Studio: monospace everywhere, near-black panels,
//! ember accent — keeps the fantasy-console feel on a desktop UI.
//! Brand colors (ACCENT, ERROR) are shared with Caiven Port — see docs/brand-colors.md.

use egui::{Color32, FontFamily, FontId, TextStyle};

/// brand ember (#FEB05D) — already light enough for small dense text, no tint needed.
pub const ACCENT: Color32 = Color32::from_rgb(254, 176, 93);
pub const ERROR: Color32 = Color32::from_rgb(229, 85, 95);
pub const OK: Color32 = Color32::from_rgb(90, 200, 110);
pub const DIM: Color32 = Color32::from_rgb(140, 140, 150);

// Syntax highlighting
pub const TEXT: Color32 = Color32::from_rgb(220, 220, 225);
pub const KEYWORD: Color32 = ACCENT;
pub const BUILTIN: Color32 = Color32::from_rgb(120, 180, 255);
pub const STRING: Color32 = Color32::from_rgb(150, 210, 130);
pub const NUMBER: Color32 = Color32::from_rgb(200, 140, 255);
pub const COMMENT: Color32 = Color32::from_rgb(100, 105, 115);
pub const ERROR_BG: Color32 = Color32::from_rgb(70, 25, 25);

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
