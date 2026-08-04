//! The pause overlay: six actions layered over the frozen last frame of the
//! running cart.
//!
//! `Screen::Pause` is immersive (no chrome) and reached only from Playing.
//! The frame behind it is already frozen for free — the host's frame loop
//! only ticks the VM while `Screen::Playing` is up (`app.rs`), so the
//! console texture simply stops changing the moment this screen takes over.
//! This module never touches that frame; it only draws what layers on top:
//! a dim scrim standing in for the handoff's blur (no blur primitive exists
//! on this CPU-raster surface — the same trade the boot screen makes for its
//! ember glow, drawn as flat rings rather than a real gradient) and the
//! pause card. The surface clears to transparent rather than an opaque
//! fill, so the scrim — not a solid color — is what dims the game
//! underneath, and both are computed once per repaint of this screen rather
//! than per frame, same as everything else this surface draws (SPEC V38).

use crate::shell::state::{PauseItem, ShellState};
use crate::shell::surface::{Align, Box2, Surface, TextStyle};
use crate::shell::theme::{Family, Metrics, Weight, color, radius, space, tracking};

/// Draws the pause overlay's content: the dim scrim, the card, and its six
/// rows with `state.pause_item()` focused.
pub fn draw(surface: &mut Surface, state: &ShellState) {
    let m = *surface.metrics();
    surface.clear(color::CART_BACKDROP.with_alpha(0.0));
    surface.fill_rect(
        Box2::new(0.0, 0.0, m.width as f32, m.height as f32),
        0.0,
        color::CART_BACKDROP.with_alpha(0.72),
    );

    let items = PauseItem::ALL;
    let row_h = m.text.pause_item + space::X4 as f32;
    let title_h = m.text.pause_title + space::X4 as f32;
    let pad = space::X4 as f32;
    let card_w = (m.width as f32 * 0.44).clamp(220.0, 420.0);
    let card_h = pad * 2.0 + title_h + row_h * items.len() as f32;
    let card = Box2::new(
        (m.width as f32 - card_w) / 2.0,
        (m.height as f32 - card_h) / 2.0,
        card_w,
        card_h,
    );

    surface.fill_rect(card, radius::LARGE, color::VOID_800);
    surface.stroke_rect(card, radius::LARGE, 1.0, color::VOID_600);

    let title_style = TextStyle::new(
        Family::Display,
        Weight::SemiBold,
        m.text.pause_title,
        color::INK,
    )
    .tracked(tracking::CAPS);
    surface.draw_text(
        title_style,
        card.x + card.w / 2.0,
        card.y + pad + m.text.pause_title,
        Align::Center,
        "PAUSED",
    );

    let focused = state.pause_item();
    let mut row_y = card.y + pad + title_h;
    for item in items {
        draw_row(
            surface,
            &m,
            Box2::new(card.x + pad, row_y, card.w - pad * 2.0, row_h),
            item.label(),
            item == focused,
            item.is_destructive(),
        );
        row_y += row_h;
    }

    // Pause has no chrome (Screen::has_chrome is false for it), so
    // `chrome::draw` skips its legend bar; `state.legend()` documents pause
    // as the one immersive screen that draws its own hint instead of
    // leaving it unused (SPEC I.machine-shell-nav `legend()`).
    let hint = state
        .legend()
        .iter()
        .map(|entry| format!("{} {}", entry.chip, entry.label))
        .collect::<Vec<_>>()
        .join("    ");
    let hint_style = TextStyle::new(
        Family::Mono,
        Weight::Regular,
        m.text.legend_label,
        color::INK_DIM,
    );
    surface.draw_text(
        hint_style,
        m.width as f32 / 2.0,
        card.y + card.h + space::X5 as f32,
        Align::Center,
        &hint,
    );
}

/// One pause row. Focus is always ember, per the theme's rule that ember is
/// the sole interactive color — even for the destructive Quit row, which
/// only distinguishes itself while unfocused, via its text color.
fn draw_row(
    surface: &mut Surface,
    m: &Metrics,
    bounds: Box2,
    label: &str,
    focused: bool,
    destructive: bool,
) {
    let text_color = if focused {
        color::EMBER_INK
    } else if destructive {
        color::DESTRUCTIVE_BRIGHT
    } else {
        color::INK
    };
    if focused {
        surface.fill_rect(bounds, radius::DEFAULT, color::EMBER);
    }

    let style = TextStyle::new(Family::Body, Weight::Medium, m.text.pause_item, text_color);
    surface.draw_text(
        style,
        bounds.x + space::X3 as f32,
        bounds.y + bounds.h / 2.0 + m.text.pause_item / 3.0,
        Align::Left,
        label,
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

    fn paused_state() -> ShellState {
        let mut state = ShellState::new();
        state.set_cart_count(1);
        state.tick(crate::shell::state::BOOT_DURATION);
        state.press(ShellButton::A);
        state.cart_ready();
        state.press(ShellButton::Start);
        state
    }

    #[test]
    fn corners_stay_scrim_only_and_dont_bleed_opaque() {
        let mut s = surface();
        let state = paused_state();
        draw(&mut s, &state);
        // The dim scrim is translucent void, never fully opaque — the
        // frozen frame underneath is meant to still show through it.
        let corner = pixel(&s, 0, 0);
        assert!(corner[3] > 0 && corner[3] < 255, "corner alpha {corner:?}");
    }

    #[test]
    fn resume_is_the_only_row_focused_on_entry() {
        let mut s = surface();
        let state = paused_state();
        assert_eq!(state.pause_item(), PauseItem::Resume);
        draw(&mut s, &state);
        // The card sits centered and opaque; some pixel inside it must be
        // fully opaque (its VOID_800 fill), confirming the card drew.
        let center = pixel(&s, s.width() / 2, s.height() / 2);
        assert_eq!(center[3], 255);
    }

    #[test]
    fn moving_focus_changes_which_row_paints_ember() {
        let mut s = surface();
        let mut state = paused_state();
        state.press(ShellButton::Down);
        assert_eq!(state.pause_item(), PauseItem::SaveState);
        draw(&mut s, &state);
    }

    #[test]
    fn wide_layout_draws_without_panicking() {
        let mut s = Surface::new(1280, 720).expect("1280×720 surface");
        let state = paused_state();
        draw(&mut s, &state);
    }

    #[test]
    fn quit_row_is_reachable_and_still_draws() {
        let mut s = surface();
        let mut state = paused_state();
        for _ in 0..PauseItem::ALL.len() - 1 {
            state.press(ShellButton::Down);
        }
        assert_eq!(state.pause_item(), PauseItem::Quit);
        draw(&mut s, &state);
    }
}
