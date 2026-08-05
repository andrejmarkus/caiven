//! The Settings screen: a pane rail on the left, the focused pane's rows on
//! the right ([`crate::shell::settings::Pane`], [`crate::shell::settings::Row`]).
//!
//! `state.rs` owns which pane and row are focused and what Left/Right/A do
//! to each [`RowKind`](crate::shell::settings::RowKind) — this module only
//! turns that into pixels. Values come straight off `state.settings()`; the
//! Port pane's Server row reads `port_client::port_url()` directly (it has
//! no `Settings` field of its own yet — there is no UI to edit it, only to
//! see what `CAIVEN_PORT_URL`/the default resolves to).

use crate::shell::settings::{Pane, Row, RowKind, SettingId, Settings};
use crate::shell::state::{Column, ShellState};
use crate::shell::surface::{Align, Box2, Surface, TextStyle};
use crate::shell::theme::{Family, Metrics, Weight, color, focus, radius, space, tracking};

/// Draws the settings screen's content: the pane rail and the focused
/// pane's rows. `version` is threaded in by the caller rather than read off
/// `env!` here, same convention `boot.rs` uses.
pub fn draw(surface: &mut Surface, state: &ShellState, version: &str) {
    let m = *surface.metrics();
    let content_x = m.screen_pad_x as f32;
    let content_top = m.content_top() as f32 + m.screen_pad_y as f32;
    let content_w = (m.width - 2 * m.screen_pad_x) as f32;
    let content_h = m.content_height() as f32 - m.screen_pad_y as f32 * 2.0;

    let rail_w = (m.width as f32 * 0.22).clamp(120.0, 220.0);
    let gap = space::X5 as f32;
    let rail = Box2::new(content_x, content_top, rail_w, content_h);
    draw_rail(surface, &m, rail, state);

    surface.stroke_rect(
        Box2::new(rail.x + rail.w, rail.y, 1.0, rail.h),
        0.0,
        1.0,
        color::VOID_600,
    );

    let rows_x = rail.x + rail.w + gap;
    let rows = Box2::new(rows_x, content_top, content_w - rail.w - gap, content_h);
    draw_rows(surface, &m, rows, state, version);
}

/// The pane list. Ember-filled only while the cursor sits on the rail
/// itself ([`Column::Rail`]) — once it moves into the rows, the current
/// pane keeps a dim ember label so the player still knows which section
/// they're in, but nothing reads as pressable there anymore.
fn draw_rail(surface: &mut Surface, m: &Metrics, bounds: Box2, state: &ShellState) {
    let row_h = m.text.body + space::X4 as f32;
    let on_rail = state.column() == Column::Rail;
    let current = state.pane();

    let mut y = bounds.y;
    for pane in Pane::ALL {
        let row = Box2::new(bounds.x, y, bounds.w, row_h);
        let is_current = pane == current;
        let filled = is_current && on_rail;

        if filled {
            surface.fill_rect(row, radius::DEFAULT, color::EMBER);
        }
        let text_color = if filled {
            color::EMBER_INK
        } else if is_current {
            color::EMBER
        } else {
            color::INK_DIM
        };
        let style = TextStyle::new(Family::Body, Weight::Medium, m.text.body, text_color);
        surface.draw_text(
            style,
            row.x + space::X3 as f32,
            row.y + row.h / 2.0 + m.text.body / 3.0,
            Align::Left,
            pane.label(),
        );
        y += row_h;
    }
}

/// The focused pane's heading and row list.
fn draw_rows(surface: &mut Surface, m: &Metrics, bounds: Box2, state: &ShellState, version: &str) {
    let pane = state.pane();
    let settings = state.settings();
    let focused_id = state.settings_row().map(|row| row.id);

    let heading_style = TextStyle::new(
        Family::Display,
        Weight::SemiBold,
        m.text.hero_cover_title,
        color::INK,
    )
    .tracked(tracking::TIGHT);
    surface.draw_text(
        heading_style,
        bounds.x,
        bounds.y + m.text.hero_cover_title,
        Align::Left,
        pane.label(),
    );

    let label_h = m.text.body;
    let sub_h = m.text.mono_micro;
    let pad_y = space::X3 as f32;
    let row_h = pad_y * 2.0 + label_h + space::X1 as f32 + sub_h;

    let mut y = bounds.y + m.text.hero_cover_title + space::X5 as f32;
    for row in pane.rows() {
        let row_bounds = Box2::new(bounds.x, y, bounds.w, row_h);
        draw_row(
            surface,
            m,
            row_bounds,
            row,
            settings,
            version,
            focused_id == Some(row.id),
        );
        y += row_h + space::X2 as f32;
    }
}

/// One settings row: label + sub on the left, value on the right. Flanking
/// `◄ ►` glyphs appear only on rows Left/Right actually adjust
/// ([`RowKind::is_adjustable`]) — an Action or Readout row isn't a
/// stepper, so it doesn't borrow that affordance.
fn draw_row(
    surface: &mut Surface,
    m: &Metrics,
    bounds: Box2,
    row: &Row,
    settings: &Settings,
    version: &str,
    focused: bool,
) {
    if focused {
        surface.fill_rect(bounds, radius::DEFAULT, color::EMBER);
    } else {
        surface.stroke_rect(bounds, radius::DEFAULT, 1.0, focus::UNFOCUSED_BORDER);
    }

    let label_color = if focused {
        color::EMBER_INK
    } else {
        color::INK
    };
    let sub_color = if focused {
        color::EMBER_INK
    } else {
        color::INK_FAINT
    };
    let pad_x = space::X3 as f32;
    let pad_y = space::X3 as f32;

    let label_style = TextStyle::new(Family::Body, Weight::Medium, m.text.body, label_color);
    surface.draw_text(
        label_style,
        bounds.x + pad_x,
        bounds.y + pad_y + m.text.body,
        Align::Left,
        row.label,
    );

    if !row.sub.is_empty() {
        let sub_style = TextStyle::new(Family::Body, Weight::Regular, m.text.mono_micro, sub_color);
        surface.draw_text(
            sub_style,
            bounds.x + pad_x,
            bounds.y + pad_y + m.text.body + space::X1 as f32 + m.text.mono_micro,
            Align::Left,
            row.sub,
        );
    }

    let value = row_value(row, settings, version);
    let value_color = if focused {
        color::EMBER_INK
    } else {
        color::INK_DIM
    };
    let value_style = TextStyle::new(Family::Mono, Weight::Regular, m.text.body, value_color);
    let value_text = if row.kind.is_adjustable() {
        format!("\u{25C4} {value} \u{25BA}")
    } else {
        value
    };
    surface.draw_text(
        value_style,
        bounds.x + bounds.w - pad_x,
        bounds.y + bounds.h / 2.0 + m.text.body / 3.0,
        Align::Right,
        &value_text,
    );
}

/// The right-hand text for one row. Everything here comes off `Settings`
/// (or the caller-supplied `version`), except the Port `Server` row, which
/// reads `port_client::port_url()` directly — never a hardcoded stand-in.
fn row_value(row: &Row, settings: &Settings, version: &str) -> String {
    match row.kind {
        RowKind::Choice { options } => {
            let index = settings.choice_index(row.id).unwrap_or(0);
            options.get(index).copied().unwrap_or_default().to_string()
        }
        RowKind::Toggle => {
            debug_assert_eq!(
                row.id,
                SettingId::ShowFps,
                "only ShowFps is a Toggle row today"
            );
            if settings.show_fps { "On" } else { "Off" }.to_string()
        }
        RowKind::Range => {
            let percent = match row.id {
                SettingId::MasterVolume => settings.master_volume,
                SettingId::SfxVolume => settings.sfx_volume,
                SettingId::MusicVolume => settings.music_volume,
                _ => 0,
            };
            format!("{percent}%")
        }
        RowKind::Action => String::new(),
        RowKind::Readout => match row.id {
            SettingId::Version => version.to_string(),
            SettingId::Server => crate::port_client::port_url(),
            _ => String::new(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shell::state::{ShellButton, ShellState};

    fn surface() -> Surface {
        Surface::new(640, 480).expect("640×480 surface")
    }

    fn settings_state() -> ShellState {
        let mut state = ShellState::new();
        state.set_cart_count(1);
        state.tick(crate::shell::state::BOOT_DURATION);
        state.press(ShellButton::A);
        state.cart_ready();
        state.press(ShellButton::Start);
        // Pause -> Settings.
        while state.pause_item() != crate::shell::state::PauseItem::Settings {
            state.press(ShellButton::Down);
        }
        state.press(ShellButton::A);
        state
    }

    #[test]
    fn rail_focus_draws_without_panicking() {
        let mut s = surface();
        let state = settings_state();
        assert_eq!(state.column(), Column::Rail);
        draw(&mut s, &state, "0.1.0");
    }

    #[test]
    fn every_pane_draws_every_row_kind_without_panicking() {
        let mut s = surface();
        let mut state = settings_state();
        for _ in 0..Pane::ALL.len() {
            state.press(ShellButton::Right); // rail -> rows
            for _ in 0..state.pane().rows().len().max(1) {
                draw(&mut s, &state, "0.1.0");
                state.press(ShellButton::Down);
            }
            state.press(ShellButton::B); // rows -> rail
            state.press(ShellButton::Down); // next pane
        }
    }

    #[test]
    fn wide_layout_settings_draws_without_panicking() {
        let mut s = Surface::new(1280, 720).expect("1280×720 surface");
        let state = settings_state();
        draw(&mut s, &state, "0.1.0");
    }
}
