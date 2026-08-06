//! The crash screen: the real `mlua` error (or load-failure context string)
//! and the frame the cart died on, never a synthesized message (SPEC V39).
//!
//! Full-bleed, no chrome — `Screen::Crash` is immersive like boot and
//! loading (`state.rs`'s `legend()` returns an empty `Vec` for it, same as
//! those two), so the hint line below is drawn here rather than left to
//! `chrome::draw`.

use crate::shell::state::ShellState;
use crate::shell::surface::{Align, Surface, TextStyle};
use crate::shell::theme::{Family, Weight, color, space};

/// Wraps `text` into lines that fit `max_width`, greedily packing words.
/// Caps output at `max_lines`, marking truncation with a trailing `…` on
/// the last line — an `mlua` error is usually one short line, but nothing
/// bounds how long a load-failure context string could get.
fn wrap(
    surface: &mut Surface,
    style: TextStyle,
    text: &str,
    max_width: f32,
    max_lines: usize,
) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        let candidate = if line.is_empty() {
            word.to_string()
        } else {
            format!("{line} {word}")
        };
        if surface.measure_text(style, &candidate) <= max_width || line.is_empty() {
            line = candidate;
        } else {
            lines.push(std::mem::take(&mut line));
            line = word.to_string();
            if lines.len() == max_lines {
                break;
            }
        }
    }
    if lines.len() < max_lines && !line.is_empty() {
        lines.push(line);
    }
    if lines.len() == max_lines
        && let Some(last) = lines.last_mut()
    {
        while surface.measure_text(style, last) > max_width && last.pop().is_some() {}
        last.push('…');
    }
    lines
}

/// Draws the crash screen. A no-op if nothing crashed — mirrors
/// `loading.rs`'s and `detail.rs`'s "safe to call standalone" discipline.
pub fn draw(surface: &mut Surface, state: &ShellState) {
    let Some(crash) = state.crash() else {
        return;
    };
    let m = *surface.metrics();
    surface.clear(color::VOID_900);

    let center_x = m.width as f32 / 2.0;
    let mut ty = m.height as f32 * 0.32;

    let title_style = TextStyle::new(
        Family::Display,
        Weight::Bold,
        m.text.crash_title,
        color::DESTRUCTIVE_BRIGHT,
    );
    surface.draw_text(
        title_style,
        center_x,
        ty,
        Align::Center,
        "Cartridge Crashed",
    );

    let message_style = TextStyle::new(Family::Mono, Weight::Regular, m.text.mono_spec, color::INK);
    let max_width = m.width as f32 * 0.8;
    let lines = wrap(surface, message_style, &crash.message, max_width, 6);
    ty += space::X6 as f32 + m.text.mono_spec;
    for line in &lines {
        surface.draw_text(message_style, center_x, ty, Align::Center, line);
        ty += m.text.mono_spec + space::X2 as f32;
    }

    if let Some(frame) = crash.frame {
        ty += space::X4 as f32;
        let frame_style = TextStyle::new(
            Family::Mono,
            Weight::Regular,
            m.text.caps_label,
            color::INK_FAINT,
        );
        surface.draw_text(
            frame_style,
            center_x,
            ty,
            Align::Center,
            &format!("died at frame {frame}"),
        );
    }

    let hint_style = TextStyle::new(
        Family::Body,
        Weight::Regular,
        m.text.caps_label,
        color::INK_FAINT,
    );
    surface.draw_text(
        hint_style,
        center_x,
        m.height as f32 - space::X6 as f32,
        Align::Center,
        "A retry cart · B back to library",
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shell::state::{BOOT_DURATION, ShellButton};

    fn surface() -> Surface {
        Surface::new(640, 480).expect("640×480 surface")
    }

    fn crashed_state(message: &str, frame: Option<u64>) -> ShellState {
        let mut state = ShellState::new();
        state.tick(BOOT_DURATION);
        state.set_cart_count(1);
        state.press(ShellButton::A); // Library -> Loading
        state.cart_failed(message, frame);
        state
    }

    #[test]
    fn drawing_a_short_message_does_not_panic() {
        let mut s = surface();
        let state = crashed_state("attempt to index a nil value (global 'ply')", Some(412));
        draw(&mut s, &state);
    }

    #[test]
    fn drawing_with_no_frame_does_not_panic() {
        let mut s = surface();
        let state = crashed_state("failed to read cart: unexpected end of file", None);
        draw(&mut s, &state);
    }

    #[test]
    fn drawing_with_no_crash_is_a_safe_no_op() {
        let mut s = surface();
        let state = ShellState::new();
        draw(&mut s, &state);
    }

    #[test]
    fn a_very_long_message_wraps_and_truncates_without_panicking() {
        let mut s = surface();
        let long = "boom ".repeat(200);
        let state = crashed_state(&long, Some(1));
        draw(&mut s, &state);
    }

    #[test]
    fn wide_layout_crash_draws_without_panicking() {
        let mut s = Surface::new(1280, 720).expect("1280×720 surface");
        let state = crashed_state("stack overflow", Some(99999));
        draw(&mut s, &state);
    }

    #[test]
    fn wrap_splits_on_word_boundaries_and_respects_max_lines() {
        let mut s = surface();
        let style = TextStyle::new(Family::Mono, Weight::Regular, 14.0, color::INK);
        let lines = wrap(
            &mut s,
            style,
            "one two three four five six seven eight",
            60.0,
            3,
        );
        assert_eq!(lines.len(), 3);
        assert!(lines.last().unwrap().ends_with('…'));
    }
}
