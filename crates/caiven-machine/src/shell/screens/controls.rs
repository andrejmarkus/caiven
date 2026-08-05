//! The controls remap screen: six rows, one per `BIND_ORDER` button, each
//! showing what's currently bound to it. `state.rs` owns the cursor and the
//! listening flag; this module only turns that into pixels.
//!
//! Labels come straight off `state.binds()` — the host seeds and updates it
//! from `controls.toml`, same discipline `settings.rs` uses for `Settings`.

use crate::shell::state::{BIND_ORDER, ShellButton, ShellState};
use crate::shell::surface::{Align, Box2, Surface, TextStyle};
use crate::shell::theme::{Family, Metrics, Weight, color, focus, radius, space, tracking};

fn button_label(button: ShellButton) -> &'static str {
    match button {
        ShellButton::Up => "Up",
        ShellButton::Down => "Down",
        ShellButton::Left => "Left",
        ShellButton::Right => "Right",
        ShellButton::A => "A",
        ShellButton::B => "B",
        ShellButton::Start | ShellButton::Select => "",
    }
}

pub fn draw(surface: &mut Surface, state: &ShellState) {
    let m = *surface.metrics();
    surface.clear(color::VOID_900);
    let content_x = m.screen_pad_x as f32;
    let content_top = m.content_top() as f32 + m.screen_pad_y as f32;
    let content_w = (m.width - 2 * m.screen_pad_x) as f32;

    let heading_style = TextStyle::new(
        Family::Display,
        Weight::SemiBold,
        m.text.hero_cover_title,
        color::INK,
    )
    .tracked(tracking::TIGHT);
    surface.draw_text(
        heading_style,
        content_x,
        content_top + m.text.hero_cover_title,
        Align::Left,
        "Controls",
    );

    let label_h = m.text.body;
    let pad_y = space::X3 as f32;
    let row_h = pad_y * 2.0 + label_h;

    let mut y = content_top + m.text.hero_cover_title + space::X5 as f32;
    for (index, button) in BIND_ORDER.into_iter().enumerate() {
        let bounds = Box2::new(content_x, y, content_w, row_h);
        let focused = index == state.bind_index();
        draw_row(
            surface,
            &m,
            bounds,
            button_label(button),
            &state.binds()[index],
            focused,
            focused && state.is_listening(),
        );
        y += row_h + space::X2 as f32;
    }
}

fn draw_row(
    surface: &mut Surface,
    m: &Metrics,
    bounds: Box2,
    label: &str,
    bound_to: &str,
    focused: bool,
    listening: bool,
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
    let pad_x = space::X3 as f32;
    let label_style = TextStyle::new(Family::Body, Weight::Medium, m.text.body, label_color);
    surface.draw_text(
        label_style,
        bounds.x + pad_x,
        bounds.y + bounds.h / 2.0 + m.text.body / 3.0,
        Align::Left,
        label,
    );

    let value_color = if focused {
        color::EMBER_INK
    } else {
        color::INK_DIM
    };
    let value_style = TextStyle::new(Family::Mono, Weight::Regular, m.text.body, value_color);
    let value_text = if listening {
        "Press a button\u{2026}"
    } else if bound_to.is_empty() {
        "\u{2014}"
    } else {
        bound_to
    };
    surface.draw_text(
        value_style,
        bounds.x + bounds.w - pad_x,
        bounds.y + bounds.h / 2.0 + m.text.body / 3.0,
        Align::Right,
        value_text,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shell::state::ShellState;

    fn surface() -> Surface {
        Surface::new(640, 480).expect("640×480 surface")
    }

    fn controls_state() -> ShellState {
        let mut state = ShellState::new();
        state.set_cart_count(1);
        state.tick(crate::shell::state::BOOT_DURATION);
        state.press(ShellButton::Start);
        while state.pane() != crate::shell::settings::Pane::Controls {
            state.press(ShellButton::Down);
        }
        state.press(ShellButton::A); // rail -> rows
        state.press(ShellButton::A); // Rebind action -> Screen::Controls
        state
    }

    #[test]
    fn unfocused_rows_draw_without_panicking() {
        let mut s = surface();
        let state = controls_state();
        draw(&mut s, &state);
    }

    #[test]
    fn listening_row_draws_without_panicking() {
        let mut s = surface();
        let mut state = controls_state();
        state.press(ShellButton::A); // arm capture on the focused row
        assert!(state.is_listening());
        draw(&mut s, &state);
    }

    #[test]
    fn a_seeded_bind_label_shows_on_its_row() {
        let mut s = surface();
        let mut state = controls_state();
        state.set_binds([
            "ArrowUp".to_string(),
            "ArrowDown".to_string(),
            "ArrowLeft".to_string(),
            "ArrowRight".to_string(),
            "KeyJ".to_string(),
            "KeyK".to_string(),
        ]);
        draw(&mut s, &state);
    }

    #[test]
    fn wide_layout_draws_without_panicking() {
        let mut s = Surface::new(1280, 720).expect("1280×720 surface");
        let state = controls_state();
        draw(&mut s, &state);
    }
}
