//! The library screen: a hero panel over the focused cart, and a scrollable
//! shelf of every cart plus the trailing Port tile.
//!
//! [`CartMeta`] carries facts only (T37) — no rating, tags or description,
//! and no captured art, since the library reads nothing but a cart's header
//! and section table. The hero panel draws what the format actually knows:
//! title, author, size and section count. A cart's cover is a color swatch
//! picked deterministically from its id, the same treatment the design
//! handoff's own shelf tiles use.
//!
//! Draws only the content area between the two chrome bars — the status bar
//! and legend bar are `shell::screens::chrome`'s job (T39).

use std::ops::Range;

use crate::shell::library::CartMeta;
use crate::shell::state::ShellState;
use crate::shell::surface::{Align, Box2, Surface, TextStyle};
use crate::shell::theme::{Family, Weight, color, focus, radius, space, tracking};

/// Picks a deterministic identity color for a cart with no captured art.
fn swatch_for(id: &str) -> crate::shell::theme::Color {
    let mut hash: u32 = 2_166_136_261;
    for byte in id.bytes() {
        hash ^= u32::from(byte);
        hash = hash.wrapping_mul(16_777_619);
    }
    color::SWATCH[hash as usize % color::SWATCH.len()]
}

/// Formats a byte count the way the spec line wants it: whole kilobytes,
/// rounding up so a small cart never reads "0 KB".
fn format_kb(bytes: u64) -> String {
    let kb = bytes.div_ceil(1024).max(1);
    format!("{kb} KB")
}

/// The shelf's visible tile window: at most `capacity` wide, always
/// containing `selected`. Pure so the scrolling behavior is unit-testable
/// without a surface.
fn shelf_window(selected: usize, tile_count: usize, capacity: usize) -> Range<usize> {
    if capacity == 0 {
        return 0..0;
    }
    if tile_count <= capacity {
        return 0..tile_count;
    }
    let start = selected
        .saturating_sub(capacity - 1)
        .min(tile_count - capacity);
    start..start + capacity
}

/// Draws the library screen's content into `surface`. `carts` is the
/// library in display order, matching [`ShellState::selected`]'s indexing.
pub fn draw(surface: &mut Surface, state: &ShellState, carts: &[CartMeta]) {
    let m = *surface.metrics();
    let content_x = m.screen_pad_x as f32;
    let content_w = (m.width - 2 * m.screen_pad_x) as f32;
    let mut y = (m.content_top() + m.screen_pad_y) as f32;

    // --- eyebrow: "Your library" · N of M -------------------------------
    let eyebrow_style = TextStyle::new(
        Family::Display,
        Weight::SemiBold,
        m.text.caps_label,
        color::INK_DIM,
    )
    .tracked(tracking::CAPS);
    let eyebrow_baseline = y + m.text.caps_label * 0.8;
    surface.draw_text(
        eyebrow_style,
        content_x,
        eyebrow_baseline,
        Align::Left,
        "YOUR LIBRARY",
    );
    let counter = format!(
        "{} of {}",
        state.selected().min(carts.len()) + 1,
        carts.len()
    );
    if !carts.is_empty() {
        let counter_style = TextStyle::new(
            Family::Mono,
            Weight::Regular,
            m.text.mono_spec,
            color::INK_FAINT,
        );
        surface.draw_text(
            counter_style,
            content_x + content_w,
            eyebrow_baseline,
            Align::Right,
            &counter,
        );
    }
    y += m.text.caps_label + space::X4 as f32;

    // --- hero -------------------------------------------------------------
    let cover = Box2::new(content_x, y, m.hero_cover, m.hero_cover);
    match state.selected_cart().and_then(|i| carts.get(i)) {
        Some(cart) => draw_hero(
            surface,
            &m,
            cover,
            content_x + m.hero_cover + m.legend_gap as f32,
            content_w - m.hero_cover - m.legend_gap as f32,
            cart,
        ),
        None => draw_hero_port_tile(surface, cover),
    }
    y += m.hero_cover + space::X5 as f32;

    // --- shelf --------------------------------------------------------------
    let shelf_label_style = TextStyle::new(
        Family::Display,
        Weight::SemiBold,
        m.text.caps_label,
        color::INK_FAINT,
    )
    .tracked(tracking::CAPS);
    let shelf_label_baseline = y + m.text.caps_label * 0.8;
    surface.draw_text(
        shelf_label_style,
        content_x,
        shelf_label_baseline,
        Align::Left,
        "SHELF",
    );
    let hint_style = TextStyle::new(
        Family::Mono,
        Weight::Regular,
        m.text.mono_spec,
        color::INK_FAINT,
    );
    surface.draw_text(
        hint_style,
        content_x + content_w,
        shelf_label_baseline,
        Align::Right,
        "◄ ►",
    );
    y += m.text.caps_label + space::X2 as f32;

    let tile_count = carts.len() + 1; // trailing Port tile
    let stride = m.shelf_tile + m.shelf_gap as f32;
    let capacity = (((content_w + m.shelf_gap as f32) / stride).floor() as usize).max(1);
    let window = shelf_window(state.selected(), tile_count, capacity);

    for (slot, index) in window.enumerate() {
        let x = content_x + slot as f32 * stride;
        let bounds = Box2::new(x, y, m.shelf_tile, m.shelf_tile);
        let focused = index == state.selected();
        if index < carts.len() {
            draw_shelf_tile(surface, &m, bounds, &carts[index], focused);
        } else {
            draw_port_tile(surface, &m, bounds, focused);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_hero(
    surface: &mut Surface,
    m: &crate::shell::theme::Metrics,
    cover: Box2,
    text_x: f32,
    text_w: f32,
    cart: &CartMeta,
) {
    surface.fill_rect(cover, radius::DEFAULT, swatch_for(&cart.id));
    surface.stroke_rect(cover, radius::DEFAULT, focus::RING_WIDTH, focus::RING_COLOR);

    let mut y = cover.y;
    let title_style = TextStyle::new(Family::Display, Weight::Bold, m.text.hero_title, color::INK)
        .tracked(tracking::TIGHT);
    y += m.text.hero_title;
    surface.draw_text(title_style, text_x, y, Align::Left, cart.display_title());

    if !cart.author.is_empty() {
        y += space::X1 as f32 + m.text.body;
        let author_style =
            TextStyle::new(Family::Body, Weight::Regular, m.text.body, color::INK_DIM);
        surface.draw_text(
            author_style,
            text_x,
            y,
            Align::Left,
            &format!("by {}", cart.author),
        );
    }

    y += space::X3 as f32 + m.text.mono_spec;
    let spec = format_spec_line(cart);
    let spec_style = TextStyle::new(
        Family::Mono,
        Weight::Regular,
        m.text.mono_spec,
        color::INK_FAINT,
    )
    .tracked(tracking::SPEC);
    surface.draw_text(spec_style, text_x, y, Align::Left, &spec);

    let _ = text_w; // reserved for wrapping once the hero grows a blurb (T42)
}

fn format_spec_line(cart: &CartMeta) -> String {
    let sections = cart
        .kinds
        .iter()
        .map(|kind| kind.name())
        .collect::<Vec<&str>>()
        .join(", ");
    if sections.is_empty() {
        format_kb(cart.bytes)
    } else {
        format!("{} · {sections}", format_kb(cart.bytes))
    }
}

fn draw_hero_port_tile(surface: &mut Surface, cover: Box2) {
    surface.fill_rect(cover, radius::DEFAULT, color::VOID_800);
    surface.stroke_rect(cover, radius::DEFAULT, 1.0, color::VOID_600);
    // The dashed outline the design uses reduces to a solid one here — the
    // raster surface has no dash primitive, and this cover already carries
    // enough affordance to read as "not a cart" beside a swatch.
    let label = TextStyle::new(Family::Mono, Weight::Regular, 11.0, color::INK_FAINT)
        .tracked(tracking::SPEC);
    surface.draw_text(
        label,
        cover.x + cover.w / 2.0,
        cover.y + cover.h / 2.0,
        Align::Center,
        "BROWSE THE PORT",
    );
}

fn draw_shelf_tile(
    surface: &mut Surface,
    m: &crate::shell::theme::Metrics,
    bounds: Box2,
    cart: &CartMeta,
    focused: bool,
) {
    let opacity = if focused {
        1.0
    } else {
        focus::UNFOCUSED_OPACITY
    };
    let fill = swatch_for(&cart.id).with_alpha(opacity);
    surface.fill_rect(bounds, radius::DEFAULT, fill);
    if focused {
        surface.stroke_rect(
            bounds,
            radius::DEFAULT,
            focus::RING_WIDTH,
            focus::RING_COLOR,
        );
    } else {
        surface.stroke_rect(bounds, radius::DEFAULT, 1.0, focus::UNFOCUSED_BORDER);
    }

    let pad = space::X1 as f32 + 3.0;
    let label = TextStyle::new(
        Family::Display,
        Weight::Bold,
        m.text.shelf_tile_title,
        color::INK.with_alpha(opacity),
    );
    surface.draw_text(
        label,
        bounds.x + pad,
        bounds.y + bounds.h - pad,
        Align::Left,
        cart.display_title(),
    );
}

fn draw_port_tile(
    surface: &mut Surface,
    m: &crate::shell::theme::Metrics,
    bounds: Box2,
    focused: bool,
) {
    surface.fill_rect(bounds, radius::DEFAULT, color::VOID_800.with_alpha(0.6));
    let border = if focused {
        focus::RING_COLOR
    } else {
        focus::UNFOCUSED_BORDER
    };
    let width = if focused { focus::RING_WIDTH } else { 1.0 };
    surface.stroke_rect(bounds, radius::DEFAULT, width, border);
    let label = TextStyle::new(
        Family::Mono,
        Weight::Regular,
        m.text.mono_micro,
        color::INK_FAINT,
    );
    surface.draw_text(
        label,
        bounds.x + bounds.w / 2.0,
        bounds.y + bounds.h / 2.0,
        Align::Center,
        "PORT",
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shell::library::CartMeta;
    use crate::shell::state::ShellState;
    use caiven_cart::SectionKind;
    use std::path::PathBuf;

    fn cart(id: &str) -> CartMeta {
        CartMeta {
            id: id.to_string(),
            path: PathBuf::from(format!("{id}.cav")),
            title: String::new(),
            author: String::new(),
            bytes: 1024,
            kinds: vec![SectionKind::Program],
        }
    }

    #[test]
    fn swatch_for_is_deterministic_and_spreads_across_the_palette() {
        assert_eq!(swatch_for("ember-drift"), swatch_for("ember-drift"));
        let colors: Vec<_> = ["a", "b", "c", "d", "e", "f", "g", "h"]
            .iter()
            .map(|id| swatch_for(id))
            .collect();
        assert!(
            colors.iter().any(|c| *c != colors[0]),
            "every id hashed to the same swatch color"
        );
    }

    #[test]
    fn format_kb_rounds_up_and_never_reads_zero() {
        assert_eq!(format_kb(0), "1 KB");
        assert_eq!(format_kb(1), "1 KB");
        assert_eq!(format_kb(1024), "1 KB");
        assert_eq!(format_kb(1025), "2 KB");
    }

    #[test]
    fn shelf_window_stays_narrow_when_everything_fits() {
        assert_eq!(shelf_window(0, 3, 5), 0..3);
        assert_eq!(shelf_window(2, 3, 5), 0..3);
    }

    #[test]
    fn shelf_window_scrolls_to_keep_the_selection_in_view() {
        // 6 tiles, 3 fit: selecting the last one must not leave it clipped.
        assert_eq!(shelf_window(0, 6, 3), 0..3);
        assert_eq!(shelf_window(5, 6, 3), 3..6);
        assert_eq!(shelf_window(2, 6, 3), 0..3);
        assert_eq!(shelf_window(3, 6, 3), 1..4);
    }

    #[test]
    fn shelf_window_never_panics_on_a_zero_capacity_or_empty_shelf() {
        assert_eq!(shelf_window(0, 0, 5), 0..0);
        assert_eq!(shelf_window(0, 4, 0), 0..0);
    }

    fn surface() -> Surface {
        Surface::new(640, 480).expect("640×480 surface")
    }

    #[test]
    fn drawing_an_empty_library_does_not_panic() {
        let mut s = surface();
        let state = ShellState::new();
        draw(&mut s, &state, &[]);
    }

    #[test]
    fn drawing_a_populated_library_marks_the_hero_and_shelf() {
        let mut s = surface();
        s.clear(color::VOID_900);
        let mut state = ShellState::new();
        let carts = vec![cart("ember-drift"), cart("tunnel-rat"), cart("catch")];
        state.set_cart_count(carts.len());
        draw(&mut s, &state, &carts);

        let m = *s.metrics();
        let cover_center_x = (m.screen_pad_x as f32 + m.hero_cover / 2.0) as u32;
        let cover_center_y = m.content_top() + m.screen_pad_y + (m.hero_cover / 2.0) as u32;
        let data = s.rgba();
        let i = (cover_center_y as usize * s.width() as usize + cover_center_x as usize) * 4;
        assert_ne!(
            &data[i..i + 3],
            &[0x2B, 0x2A, 0x2A],
            "hero cover did not draw"
        );
    }

    #[test]
    fn drawing_on_the_port_tile_does_not_panic() {
        let mut s = surface();
        let mut state = ShellState::new();
        let carts = vec![cart("only-cart")];
        state.set_cart_count(carts.len());
        state.tick(crate::shell::state::BOOT_DURATION);
        // Move onto the trailing Port tile.
        for _ in 0..2 {
            state.press(crate::shell::state::ShellButton::Right);
        }
        assert!(state.on_port_tile());
        draw(&mut s, &state, &carts);
    }
}
