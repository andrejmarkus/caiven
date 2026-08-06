//! The Port browse screen: a sorted, scrollable list of carts on a Port
//! server, download-on-A. Search/tag/author filtering beyond the sort chip
//! is out of scope this milestone (SPEC §C) — SELECT only cycles
//! [`PortSort`](crate::shell::state::PortSort).
//!
//! `entries` is host-fetched (`port_client::list`) and handed in fresh every
//! time the screen becomes current — `Effect::RefreshPort` resolves
//! synchronously in `app.rs::handle_effect` before the next draw, the same
//! "never draws mid-progress" discipline `loading.rs` documents for
//! `Effect::LoadCart`. There is no captured Port thumbnail in the response
//! (SPEC I.machine-shell-port lists a screenshot endpoint, but wiring pixel
//! decode for it is out of scope here) — rows reuse the library's
//! deterministic id-swatch cover treatment instead.

use crate::port_client::PortEntry;
use crate::shell::icon::Icon;
use crate::shell::state::ShellState;
use crate::shell::surface::{Align, Box2, Surface, TextStyle};
use crate::shell::theme::{Family, Metrics, Weight, color, radius, space, tracking};

use super::library::{format_kb, swatch_for};

/// The visible row window: at most `capacity` rows tall, always containing
/// `selected`. Pure so scrolling is unit-testable without a surface — same
/// shape as `library.rs`'s `shelf_window`, just vertical.
fn row_window(selected: usize, row_count: usize, capacity: usize) -> std::ops::Range<usize> {
    if capacity == 0 {
        return 0..0;
    }
    if row_count <= capacity {
        return 0..row_count;
    }
    let start = selected
        .saturating_sub(capacity - 1)
        .min(row_count - capacity);
    start..start + capacity
}

/// Draws the Port screen's content into `surface`. `entries` is the listing
/// in display order, matching [`ShellState::port_index`]'s indexing.
pub fn draw(surface: &mut Surface, state: &ShellState, entries: &[PortEntry]) {
    let m = *surface.metrics();
    surface.clear(color::VOID_900);
    let content_x = m.screen_pad_x as f32;
    let content_w = (m.width - 2 * m.screen_pad_x) as f32;
    let mut y = (m.content_top() + m.screen_pad_y) as f32;

    // --- eyebrow: "Port" · N carts · Sort: X -----------------------------
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
        "PORT",
    );
    let count_style = TextStyle::new(
        Family::Mono,
        Weight::Regular,
        m.text.mono_micro,
        color::INK_FAINT,
    );
    surface.draw_text(
        count_style,
        content_x + content_w,
        eyebrow_baseline,
        Align::Right,
        &format!(
            "{} · {}",
            state.port_sort().legend_label(),
            match entries.len() {
                1 => "1 cart".to_string(),
                n => format!("{n} carts"),
            }
        ),
    );
    y += m.text.caps_label + space::X4 as f32;

    if entries.is_empty() {
        draw_empty_state(surface, &m, y);
        return;
    }

    let label_h = m.text.body;
    let sub_h = m.text.mono_micro;
    let pad_y = space::X3 as f32;
    let row_h = pad_y * 2.0 + label_h + space::X1 as f32 + sub_h;
    let row_gap = space::X2 as f32;
    let bottom = (m.content_top() + m.content_height()).saturating_sub(m.screen_pad_y) as f32;
    let capacity = (((bottom - y + row_gap) / (row_h + row_gap))
        .floor()
        .max(0.0)) as usize;

    let window = row_window(state.port_index(), entries.len(), capacity);
    for (offset, entry) in entries[window.clone()].iter().enumerate() {
        let index = window.start + offset;
        let bounds = Box2::new(content_x, y, content_w, row_h);
        draw_row(surface, &m, bounds, entry, index == state.port_index());
        y += row_h + row_gap;
    }
}

/// One Port result row: cover swatch, title + author on the left, size on
/// the right. Same solid-fill-on-focus treatment `settings.rs`/
/// `controls.rs` use for their row lists.
fn draw_row(surface: &mut Surface, m: &Metrics, bounds: Box2, entry: &PortEntry, focused: bool) {
    use crate::shell::theme::focus;

    if focused {
        surface.fill_rect(bounds, radius::DEFAULT, color::EMBER);
    } else {
        surface.stroke_rect(bounds, radius::DEFAULT, 1.0, focus::UNFOCUSED_BORDER);
    }

    let pad_x = space::X3 as f32;
    let cover_side = bounds.h - space::X2 as f32;
    let cover = Box2::new(
        bounds.x + pad_x,
        bounds.y + space::X1 as f32,
        cover_side,
        cover_side,
    );
    surface.fill_rect(cover, radius::SMALL, swatch_for(&entry.id));

    let text_x = cover.x + cover.w + pad_x;
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

    let label_style = TextStyle::new(
        Family::Display,
        Weight::Medium,
        m.text.port_row_title,
        label_color,
    );
    surface.draw_text(
        label_style,
        text_x,
        bounds.y + space::X3 as f32 + m.text.body,
        Align::Left,
        &entry.title,
    );

    let sub_style = TextStyle::new(Family::Body, Weight::Regular, m.text.mono_micro, sub_color);
    surface.draw_text(
        sub_style,
        text_x,
        bounds.y + space::X3 as f32 + m.text.body + space::X1 as f32 + m.text.mono_micro,
        Align::Left,
        &entry.author,
    );

    let value_color = if focused {
        color::EMBER_INK
    } else {
        color::INK_DIM
    };
    let value_style = TextStyle::new(Family::Mono, Weight::Regular, m.text.body, value_color);
    surface.draw_text(
        value_style,
        bounds.x + bounds.w - pad_x,
        bounds.y + bounds.h / 2.0 + m.text.body / 3.0,
        Align::Right,
        &format_kb(entry.bytes),
    );
}

fn draw_empty_state(surface: &mut Surface, m: &Metrics, content_top: f32) {
    let center_x = m.width as f32 / 2.0;
    let content_bottom = (m.content_top() + m.content_height()) as f32;
    let center_y = (content_top + content_bottom) / 2.0;

    let icon_size = m.hero_cover * 0.4;
    let _ = surface.draw_icon(
        Icon::Cartridge,
        center_x - icon_size / 2.0,
        center_y - icon_size * 1.6,
        icon_size,
        1.6,
        color::INK_FAINT,
    );

    let title_style = TextStyle::new(
        Family::Display,
        Weight::SemiBold,
        m.text.empty_title,
        color::INK,
    );
    surface.draw_text(
        title_style,
        center_x,
        center_y,
        Align::Center,
        "No carts found",
    );

    let body_style = TextStyle::new(Family::Body, Weight::Regular, m.text.body, color::INK_DIM);
    surface.draw_text(
        body_style,
        center_x,
        center_y + m.text.body + space::X2 as f32,
        Align::Center,
        "The Port server has nothing to show, or is unreachable.",
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shell::state::ShellButton;

    fn surface() -> Surface {
        Surface::new(640, 480).expect("640×480 surface")
    }

    fn entries(n: usize) -> Vec<PortEntry> {
        (0..n)
            .map(|i| PortEntry {
                id: format!("cart-{i}"),
                title: format!("Cart {i}"),
                author: "Someone".to_string(),
                bytes: 1024 * (i as u64 + 1),
            })
            .collect()
    }

    fn port_state() -> ShellState {
        let mut state = ShellState::new();
        state.set_cart_count(0);
        state.tick(crate::shell::state::BOOT_DURATION);
        state.press(ShellButton::Select);
        state
    }

    #[test]
    fn draws_without_panicking_across_row_counts() {
        let mut s = surface();
        for n in [0, 1, 8, 40] {
            let mut state = port_state();
            state.set_port_count(n);
            draw(&mut s, &state, &entries(n));
        }
    }

    #[test]
    fn wide_layout_draws_without_panicking() {
        let mut s = Surface::new(1280, 720).expect("1280×720 surface");
        let mut state = port_state();
        state.set_port_count(5);
        draw(&mut s, &state, &entries(5));
    }

    #[test]
    fn row_window_stays_narrow_when_everything_fits() {
        assert_eq!(row_window(0, 3, 8), 0..3);
    }

    #[test]
    fn row_window_scrolls_to_keep_the_selection_in_view() {
        assert_eq!(row_window(9, 20, 5), 5..10);
        assert_eq!(row_window(0, 20, 5), 0..5);
        assert_eq!(row_window(19, 20, 5), 15..20);
    }

    #[test]
    fn row_window_never_panics_on_a_zero_capacity_or_empty_list() {
        assert_eq!(row_window(0, 0, 5), 0..0);
        assert_eq!(row_window(0, 5, 0), 0..0);
    }
}
