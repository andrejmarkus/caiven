//! The Playing screen: the cart owns the whole frame.
//!
//! Per SPEC V37, this shell layer never draws a HUD, frame, or border — the
//! actual game pixels come from the VM's own streaming texture, composited
//! by the platform layer (`platform::window::Display`) underneath this
//! raster surface, scaled per [`crate::shell::settings::Settings`] rather
//! than anything drawn here. The only thing this module can put on screen
//! is an fps counter, and only when Settings › Video › Show fps is on — so
//! most frames this draws nothing at all, clearing to fully transparent so
//! it composites as a no-op.

use crate::shell::state::ShellState;
use crate::shell::surface::{Align, Surface, TextStyle};
use crate::shell::theme::{Family, Weight, color, space};

/// Draws the Playing screen's overlay. `fps` is host-supplied (measured
/// frame timing), the same pattern as `loading.rs`'s `LoadProgress` —
/// `ShellState` stays host-free and carries no timing of its own.
pub fn draw(surface: &mut Surface, state: &ShellState, fps: u32) {
    let m = *surface.metrics();
    surface.clear(color::VOID_900.with_alpha(0.0));

    if !state.settings().show_fps {
        return;
    }

    let style = TextStyle::new(
        Family::Mono,
        Weight::Regular,
        m.text.mono_micro,
        color::EMBER.with_alpha(0.85),
    );
    let baseline = space::X2 as f32 + m.text.mono_micro;
    surface.draw_text(
        style,
        m.width as f32 - space::X3 as f32,
        baseline,
        Align::Right,
        &format!("{fps} fps"),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shell::state::{ShellButton, ShellState};

    fn surface() -> Surface {
        Surface::new(640, 480).expect("640×480 surface")
    }

    fn pixel(s: &Surface, x: u32, y: u32) -> [u8; 4] {
        let i = (y as usize * s.width() as usize + x as usize) * 4;
        let d = s.rgba();
        [d[i], d[i + 1], d[i + 2], d[i + 3]]
    }

    fn any_opaque_pixel(s: &Surface) -> bool {
        s.rgba().chunks_exact(4).any(|px| px[3] != 0)
    }

    /// Turns `Show fps` on the same way a player does: Start from the
    /// library opens Settings on the Video pane (row order Scaling, Aspect,
    /// Show fps), Right enters the rows column, Down twice reaches Show
    /// fps, Right flips the toggle.
    fn state_with_fps_on() -> ShellState {
        let mut state = ShellState::new();
        state.set_cart_count(1);
        state.tick(crate::shell::state::BOOT_DURATION);
        for button in [
            ShellButton::Start,
            ShellButton::Right,
            ShellButton::Down,
            ShellButton::Down,
            ShellButton::Right,
        ] {
            state.press(button);
        }
        assert!(state.settings().show_fps, "Show fps did not toggle on");
        state
    }

    #[test]
    fn fps_hidden_by_default_draws_nothing() {
        let mut s = surface();
        let state = ShellState::new();
        assert!(!state.settings().show_fps);
        draw(&mut s, &state, 60);
        assert!(!any_opaque_pixel(&s));
        assert_eq!(pixel(&s, 0, 0)[3], 0);
    }

    #[test]
    fn fps_shown_draws_top_right_and_nothing_else() {
        let mut s = surface();
        let state = state_with_fps_on();
        draw(&mut s, &state, 60);
        assert!(any_opaque_pixel(&s));
        // Bottom-left corner stays untouched — the readout is a small
        // top-right mark, not a fill.
        assert_eq!(pixel(&s, 0, 479)[3], 0);
    }

    #[test]
    fn drawing_at_wide_layout_does_not_panic() {
        let mut s = Surface::new(1280, 720).expect("1280×720 surface");
        let state = state_with_fps_on();
        draw(&mut s, &state, 144);
    }
}
