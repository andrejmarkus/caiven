//! The cart detail screen: a large cover, a spec card, and Play/Delete
//! actions the player toggles between with ◄►/▲▼ ([`DetailAction`]).
//!
//! No captured screenshot exists — `CartMeta` carries none, the same fact
//! that shapes the library hero panel (T38) — so the cover reuses that
//! screen's deterministic color-swatch treatment rather than a bitmap.
//!
//! Draws only the content area between the two chrome bars; a screen with
//! nothing selected draws nothing; `state.rs` never enters [`Screen::Detail`]
//! without a selection, and the same discipline is used at T41's
//! `carts.is_empty()` check.

use crate::shell::library::CartMeta;
use crate::shell::screens::library::{format_kb, swatch_for};
use crate::shell::state::{DetailAction, ShellState};
use crate::shell::surface::{Align, Box2, Surface, TextStyle};
use crate::shell::theme::{Family, Metrics, Weight, color, focus, radius, space, tracking};

/// Draws the cart detail screen's content. A no-op if nothing is selected —
/// callers only reach [`Screen::Detail`](crate::shell::state::Screen::Detail)
/// with a selection, but this keeps the module safe to call standalone.
pub fn draw(surface: &mut Surface, state: &ShellState, carts: &[CartMeta]) {
    let Some(cart) = state.selected_cart().and_then(|i| carts.get(i)) else {
        return;
    };
    let m = *surface.metrics();
    surface.clear(color::VOID_900);
    let content_x = m.screen_pad_x as f32;
    let content_w = (m.width - 2 * m.screen_pad_x) as f32;
    let mut y = m.content_top() as f32 + m.screen_pad_y as f32;

    // --- cover + title ------------------------------------------------------
    let cover = Box2::new(content_x, y, m.hero_cover, m.hero_cover);
    surface.fill_rect(cover, radius::DEFAULT, swatch_for(&cart.id));
    surface.stroke_rect(cover, radius::DEFAULT, focus::RING_WIDTH, focus::RING_COLOR);

    let text_x = content_x + m.hero_cover + m.legend_gap as f32;
    let mut ty = y + m.text.detail_title;
    let title_style = TextStyle::new(
        Family::Display,
        Weight::Bold,
        m.text.detail_title,
        color::INK,
    )
    .tracked(tracking::TIGHT);
    surface.draw_text(title_style, text_x, ty, Align::Left, cart.display_title());

    if !cart.author.is_empty() {
        ty += space::X2 as f32 + m.text.body;
        let author_style =
            TextStyle::new(Family::Body, Weight::Regular, m.text.body, color::INK_DIM);
        surface.draw_text(
            author_style,
            text_x,
            ty,
            Align::Left,
            &format!("by {}", cart.author),
        );
    }
    y += m.hero_cover + space::X5 as f32;

    // --- spec card ------------------------------------------------------
    let card_h = draw_spec_card(surface, &m, Box2::new(content_x, y, content_w, 0.0), cart);
    y += card_h + space::X5 as f32;

    // --- actions ------------------------------------------------------
    let action_h = space::X6 as f32 + space::X2 as f32;
    let action_gap = space::X3 as f32;
    let action_w = (content_w - action_gap) / 2.0;
    draw_action(
        surface,
        &m,
        Box2::new(content_x, y, action_w, action_h),
        "Play",
        state.detail_action() == DetailAction::Play,
    );
    draw_action(
        surface,
        &m,
        Box2::new(content_x + action_w + action_gap, y, action_w, action_h),
        "Delete",
        state.detail_action() == DetailAction::Delete,
    );
}

/// A bordered card listing the facts `CartMeta` actually knows: size and
/// section kinds, one per row rather than the hero panel's single inline
/// line, since the detail screen has the room. Returns the card's height so
/// the caller can lay out what comes after it.
fn draw_spec_card(surface: &mut Surface, m: &Metrics, bounds: Box2, cart: &CartMeta) -> f32 {
    let row_h = m.text.body + space::X2 as f32;
    let pad_y = space::X3 as f32;
    let rows = 1 + usize::from(!cart.kinds.is_empty());
    let card_h = pad_y * 2.0 + row_h * rows as f32;
    let card = Box2::new(bounds.x, bounds.y, bounds.w, card_h);

    surface.fill_rect(card, radius::DEFAULT, color::VOID_800);
    surface.stroke_rect(card, radius::DEFAULT, 1.0, color::VOID_600);

    let label_style = TextStyle::new(
        Family::Display,
        Weight::SemiBold,
        m.text.caps_label,
        color::INK_FAINT,
    )
    .tracked(tracking::CAPS);
    let value_style = TextStyle::new(Family::Mono, Weight::Regular, m.text.body, color::INK_DIM);
    let label_x = card.x + space::X4 as f32;
    let value_x = card.x + card.w - space::X4 as f32;
    let mut row_y = card.y + pad_y + row_h * 0.7;

    surface.draw_text(label_style, label_x, row_y, Align::Left, "SIZE");
    surface.draw_text(
        value_style,
        value_x,
        row_y,
        Align::Right,
        &format_kb(cart.bytes),
    );

    if !cart.kinds.is_empty() {
        row_y += row_h;
        let sections = cart
            .kinds
            .iter()
            .map(|kind| kind.name())
            .collect::<Vec<&str>>()
            .join(", ");
        surface.draw_text(label_style, label_x, row_y, Align::Left, "SECTIONS");
        surface.draw_text(value_style, value_x, row_y, Align::Right, &sections);
    }

    card_h
}

/// One of the two toggleable action buttons. The focused one fills ember,
/// matching the legend bar's primary-chip treatment; the other stays an
/// outline, same as an unfocused shelf tile.
fn draw_action(surface: &mut Surface, m: &Metrics, bounds: Box2, label: &str, focused: bool) {
    let (fill, text_color) = if focused {
        (color::EMBER, color::EMBER_INK)
    } else {
        (color::VOID_800, color::INK)
    };
    surface.fill_rect(bounds, radius::DEFAULT, fill);
    let border = if focused {
        focus::RING_COLOR
    } else {
        focus::UNFOCUSED_BORDER
    };
    let width = if focused { focus::RING_WIDTH } else { 1.0 };
    surface.stroke_rect(bounds, radius::DEFAULT, width, border);

    let style = TextStyle::new(Family::Display, Weight::Bold, m.text.body, text_color);
    surface.draw_text(
        style,
        bounds.x + bounds.w / 2.0,
        bounds.y + bounds.h / 2.0 + m.text.body / 3.0,
        Align::Center,
        label,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shell::state::ShellButton;
    use caiven_cart::SectionKind;
    use std::path::PathBuf;

    fn surface() -> Surface {
        Surface::new(640, 480).expect("640×480 surface")
    }

    fn cart(id: &str) -> CartMeta {
        CartMeta {
            id: id.to_string(),
            path: PathBuf::from(format!("{id}.cav")),
            title: "Ember Drift".to_string(),
            author: "Andrej".to_string(),
            bytes: 20_480,
            kinds: vec![SectionKind::Program, SectionKind::SpriteSheet],
        }
    }

    fn detail_state(carts: &[CartMeta]) -> ShellState {
        let mut state = ShellState::new();
        state.tick(crate::shell::state::BOOT_DURATION);
        state.set_cart_count(carts.len());
        state.press(ShellButton::B); // "Details" from the library legend -> Detail
        state
    }

    #[test]
    fn drawing_detail_for_a_selected_cart_does_not_panic() {
        let mut s = surface();
        let carts = [cart("ember-drift")];
        let state = detail_state(&carts);
        assert_eq!(state.selected_cart(), Some(0));
        draw(&mut s, &state, &carts);
    }

    #[test]
    fn drawing_detail_with_no_selection_is_a_safe_no_op() {
        let mut s = surface();
        let state = ShellState::new();
        draw(&mut s, &state, &[]);
    }

    #[test]
    fn toggling_the_action_flips_the_focused_button_without_panicking() {
        let mut s = surface();
        let carts = [cart("ember-drift")];
        let mut state = detail_state(&carts);
        assert_eq!(state.detail_action(), DetailAction::Play);
        draw(&mut s, &state, &carts);

        state.press(ShellButton::Right);
        assert_eq!(state.detail_action(), DetailAction::Delete);
        draw(&mut s, &state, &carts);
    }

    #[test]
    fn wide_layout_detail_draws_without_panicking() {
        let mut s = Surface::new(1280, 720).expect("1280×720 surface");
        let carts = [cart("ember-drift")];
        let state = detail_state(&carts);
        draw(&mut s, &state, &carts);
    }

    #[test]
    fn a_cart_with_no_author_or_sections_still_draws() {
        let mut s = surface();
        let carts = [CartMeta {
            id: "bare".to_string(),
            path: PathBuf::from("bare.cav"),
            title: String::new(),
            author: String::new(),
            bytes: 512,
            kinds: vec![],
        }];
        let state = detail_state(&carts);
        draw(&mut s, &state, &carts);
    }
}
