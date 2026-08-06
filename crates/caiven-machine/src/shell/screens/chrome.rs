//! The status bar and legend bar drawn around every screen that has them
//! ([`crate::shell::state::Screen::has_chrome`]). Content screens (library,
//! detail, settings, controls, Port) draw only the area between the two
//! bars; this module owns the bars themselves.
//!
//! [`ShellState`] is deliberately host-free, so it carries no wall clock and
//! no battery reading. Both are host facts — the RTC register
//! (`caiven_core::memory::RTC_RAM_BASE`) and an SDL power query — that the
//! app loop supplies each frame as [`StatusInfo`] (wiring lands with T44).
//! Cart count and volume, by contrast, already live on [`ShellState`] and
//! its [`crate::shell::settings::Settings`], so they are read directly.

use crate::shell::icon::Icon;
use crate::shell::state::{Legend, ShellState};
use crate::shell::surface::{Align, Box2, Surface, TextStyle};
use crate::shell::theme::{Color, Family, Metrics, Weight, color, space};

/// Host facts the status bar needs but [`ShellState`] does not own.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StatusInfo {
    /// 0-23, from the RTC register.
    pub hour: u8,
    /// 0-59, from the RTC register.
    pub minute: u8,
    /// Charge fraction `0.0..=1.0`, or `None` on a device with no battery
    /// (desktop).
    pub battery: Option<f32>,
    pub wifi: bool,
}

/// Draws both bars for the current screen. A no-op on the immersive screens
/// (boot, loading, playing, pause, crash) — safe to call every frame without
/// checking [`crate::shell::state::Screen::has_chrome`] first.
pub fn draw(surface: &mut Surface, state: &ShellState, status: &StatusInfo) {
    if !state.screen().has_chrome() {
        return;
    }
    draw_status_bar(surface, state, status);
    draw_legend_bar(surface, state);
}

fn format_clock(hour: u8, minute: u8) -> String {
    format!("{:02}:{:02}", hour % 24, minute % 60)
}

fn status_icon_style(m: &Metrics) -> f32 {
    m.text.mono_micro + 3.0
}

fn draw_status_bar(surface: &mut Surface, state: &ShellState, status: &StatusInfo) {
    let m = *surface.metrics();
    surface.fill_rect(
        Box2::new(0.0, 0.0, m.width as f32, m.status_bar_h as f32),
        0.0,
        color::VOID_800,
    );
    surface.fill_rect(
        Box2::new(0.0, m.status_bar_h as f32 - 1.0, m.width as f32, 1.0),
        0.0,
        color::VOID_600,
    );

    let baseline = m.status_bar_h as f32 / 2.0 + m.text.mono_spec / 3.0;

    let clock_style = TextStyle::new(Family::Mono, Weight::Medium, m.text.mono_spec, color::INK);
    surface.draw_text(
        clock_style,
        m.status_bar_pad_x as f32,
        baseline,
        Align::Left,
        &format_clock(status.hour, status.minute),
    );

    let count_style = TextStyle::new(
        Family::Mono,
        Weight::Regular,
        m.text.mono_spec,
        color::INK_DIM,
    );
    let count_text = format!("{} carts", state.cart_count());
    surface.draw_text(
        count_style,
        m.width as f32 / 2.0,
        baseline,
        Align::Center,
        &count_text,
    );

    let icon_size = status_icon_style(&m);
    let mut right = (m.width - m.status_bar_pad_x) as f32;
    let icon_y = (m.status_bar_h as f32 - icon_size) / 2.0;

    if let Some(charge) = status.battery {
        right -= icon_size;
        let _ = surface.draw_icon(Icon::Battery, right, icon_y, icon_size, 1.6, color::INK_DIM);
        let scale = icon_size / 24.0;
        let (bx, by, bw, bh) = Icon::BATTERY_INNER;
        let fill_w = bw * scale * charge.clamp(0.0, 1.0);
        if fill_w > 0.0 {
            surface.fill_rect(
                Box2::new(right + bx * scale, icon_y + by * scale, fill_w, bh * scale),
                0.0,
                color::EMBER,
            );
        }
        right -= space::X1 as f32;
    }

    let volume = state.settings().master_volume;
    let volume_icon = if volume == 0 {
        Icon::VolumeMuted
    } else {
        Icon::Volume
    };
    right -= icon_size;
    let _ = surface.draw_icon(volume_icon, right, icon_y, icon_size, 1.6, color::INK_DIM);

    if status.wifi {
        right -= icon_size + space::X1 as f32;
        let _ = surface.draw_icon(Icon::Wifi, right, icon_y, icon_size, 1.6, color::INK_DIM);
    }
}

/// Height of a legend chip, derived from the bar height rather than a
/// separate token — the bar is the only thing that constrains it.
fn chip_height(m: &Metrics) -> f32 {
    (m.legend_bar_h.saturating_sub(space::X1 * 2)) as f32
}

fn chip_style(m: &Metrics) -> TextStyle {
    TextStyle::new(Family::Mono, Weight::Bold, m.text.legend_label, color::INK)
}

fn label_style(m: &Metrics) -> TextStyle {
    TextStyle::new(
        Family::Body,
        Weight::Regular,
        m.text.legend_label,
        color::INK_DIM,
    )
}

/// Total width one legend entry occupies: its chip, a gap, then its label.
fn entry_width(surface: &mut Surface, m: &Metrics, entry: &Legend) -> f32 {
    let chip_h = chip_height(m);
    let chip_w = (surface.measure_text(chip_style(m), entry.chip) + space::X2 as f32).max(chip_h);
    let label_w = surface.measure_text(label_style(m), entry.label);
    chip_w + space::X2 as f32 + label_w
}

fn draw_entry(surface: &mut Surface, m: &Metrics, entry: &Legend, x: f32, center_y: f32) {
    let chip_h = chip_height(m);
    let chip_w = (surface.measure_text(chip_style(m), entry.chip) + space::X2 as f32).max(chip_h);
    let chip_box = Box2::new(x, center_y - chip_h / 2.0, chip_w, chip_h);

    let (fill, text_color): (Option<Color>, Color) = if entry.primary {
        (Some(color::EMBER), color::EMBER_INK)
    } else {
        (None, color::INK)
    };
    if let Some(fill) = fill {
        surface.fill_rect(chip_box, f32::INFINITY, fill);
    } else {
        surface.stroke_rect(chip_box, f32::INFINITY, 1.0, color::VOID_600);
    }
    let mut style = chip_style(m);
    style.color = text_color;
    surface.draw_text(
        style,
        x + chip_w / 2.0,
        center_y + m.text.legend_label / 3.0,
        Align::Center,
        entry.chip,
    );

    surface.draw_text(
        label_style(m),
        x + chip_w + space::X2 as f32,
        center_y + m.text.legend_label / 3.0,
        Align::Left,
        entry.label,
    );
}

fn draw_legend_bar(surface: &mut Surface, state: &ShellState) {
    let m = *surface.metrics();
    let bar_y = (m.height - m.legend_bar_h) as f32;
    surface.fill_rect(
        Box2::new(0.0, bar_y, m.width as f32, m.legend_bar_h as f32),
        0.0,
        color::VOID_800,
    );
    surface.fill_rect(
        Box2::new(0.0, bar_y, m.width as f32, 1.0),
        0.0,
        color::VOID_600,
    );

    let center_y = bar_y + m.legend_bar_h as f32 / 2.0;
    let mut left_x = m.legend_bar_pad_x as f32;
    let mut right_x = (m.width - m.legend_bar_pad_x) as f32;

    for entry in state.legend() {
        let w = entry_width(surface, &m, &entry);
        if entry.trailing {
            right_x -= w;
            draw_entry(surface, &m, &entry, right_x, center_y);
            right_x -= m.legend_gap as f32;
        } else {
            draw_entry(surface, &m, &entry, left_x, center_y);
            left_x += w + m.legend_gap as f32;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shell::state::{BOOT_DURATION, ShellButton};

    fn quiet_status() -> StatusInfo {
        StatusInfo {
            hour: 7,
            minute: 5,
            battery: Some(0.62),
            wifi: true,
        }
    }

    #[test]
    fn clock_pads_single_digits() {
        assert_eq!(format_clock(7, 5), "07:05");
        assert_eq!(format_clock(23, 59), "23:59");
        // Values a caller should never send, but the format must not panic.
        assert_eq!(format_clock(24, 60), "00:00");
    }

    #[test]
    fn chrome_is_skipped_on_immersive_screens_without_panicking() {
        let mut surface = Surface::new(640, 480).expect("surface");
        let state = ShellState::new();
        assert!(!state.screen().has_chrome());
        draw(&mut surface, &state, &quiet_status());
    }

    #[test]
    fn drawing_chrome_across_every_chrome_screen_does_not_panic() {
        let mut surface = Surface::new(640, 480).expect("surface");
        let mut state = ShellState::new();
        state.set_cart_count(2);
        state.tick(BOOT_DURATION);

        let walk = [
            ShellButton::B,
            ShellButton::B,
            ShellButton::Start,
            ShellButton::A,
            ShellButton::B,
            ShellButton::B,
            ShellButton::Select,
        ];
        draw(&mut surface, &state, &quiet_status());
        for button in walk {
            state.press(button);
            draw(&mut surface, &state, &quiet_status());
        }
    }

    #[test]
    fn a_muted_and_batteryless_status_bar_still_draws() {
        let mut surface = Surface::new(640, 480).expect("surface");
        let mut state = ShellState::new();
        state.tick(BOOT_DURATION);
        // Library -> Settings rail -> Audio pane -> rows -> mute.
        state.press(ShellButton::Start);
        state.press(ShellButton::Down);
        state.press(ShellButton::A);
        for _ in 0..(state.settings().master_volume as usize).div_ceil(10) {
            state.press(ShellButton::Left);
        }
        assert_eq!(state.settings().master_volume, 0);

        let status = StatusInfo {
            hour: 0,
            minute: 0,
            battery: None,
            wifi: false,
        };
        draw(&mut surface, &state, &status);
    }

    #[test]
    fn wide_layout_chrome_draws_without_panicking() {
        let mut surface = Surface::new(1280, 720).expect("surface");
        let mut state = ShellState::new();
        state.set_cart_count(1);
        state.tick(BOOT_DURATION);
        draw(&mut surface, &state, &quiet_status());
    }
}
