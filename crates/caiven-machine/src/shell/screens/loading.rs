//! The hand-off (loading) screen: a static ember glow behind the cart's
//! label, its title/author, a progress pill, and stage text describing
//! which section is being mounted right now.
//!
//! Full-bleed, no chrome — [`crate::shell::state::Screen::Loading`] is
//! immersive like boot and playing. Progress and stage text are supplied by
//! the caller ([`LoadProgress`]) rather than tracked in [`ShellState`]: the
//! real numbers come from wall-clock elapsed time against actual section
//! loading (SPEC V35), and the host is what walks `Vm::load_cart_sections`
//! (SPEC V36) — this module only draws what it's told.

use caiven_cart::SectionKind;

use crate::shell::library::CartMeta;
use crate::shell::screens::library::swatch_for;
use crate::shell::state::ShellState;
use crate::shell::surface::{Align, Box2, Surface, TextStyle};
use crate::shell::theme::{Family, Weight, color, radius, space, tracking};

/// Real load progress, computed by the host from wall-clock elapsed time
/// against the section table (SPEC V35, V36) — never a synthesized string
/// or a tick count.
pub struct LoadProgress {
    /// Sections mounted so far, `0..=total_sections`.
    pub fraction: f32,
    /// Human stage text, e.g. `"mounting cart · SpriteSheet 2/4"` or
    /// `"running _init()"`. Build with [`stage_text`].
    pub stage: String,
}

/// Short label for a section kind's stage line. Groups the legacy/bank
/// variants of the same asset type under one word, matching how the handoff
/// copy reads ("sprites 4/4", "map 1/1", "sfx 9/16").
fn stage_label(kind: SectionKind) -> &'static str {
    match kind {
        SectionKind::Program | SectionKind::LuaSource => "program",
        SectionKind::SpriteSheet | SectionKind::SpriteBank => "sprites",
        SectionKind::Map | SectionKind::MapBank => "map",
        SectionKind::SfxBank | SectionKind::SfxBanks => "sfx",
        SectionKind::MusicBank | SectionKind::MusicBanks => "music",
        SectionKind::Palette | SectionKind::PaletteBank => "palette",
        SectionKind::Collision | SectionKind::CollisionBank | SectionKind::CollisionTypes => {
            "collision"
        }
        SectionKind::Meta => "meta",
        SectionKind::ModManifest => "mods",
        SectionKind::PreludeModules => "stdlib",
        SectionKind::Custom(_) => "data",
    }
}

/// Builds the stage line for one point in a real load, from the cart's
/// section table (in table order, as `Vm::load_cart_sections` walks it) and
/// how many of those sections are mounted so far. Once every section is
/// mounted, the stage becomes `"running _init()"` — the last real step
/// before a cart plays.
pub fn stage_text(sections: &[SectionKind], mounted: usize) -> String {
    let Some(&current) = sections.get(mounted) else {
        return "running _init()".to_string();
    };
    let label = stage_label(current);
    let total = sections
        .iter()
        .filter(|&&k| stage_label(k) == label)
        .count();
    let index = sections[..=mounted]
        .iter()
        .filter(|&&k| stage_label(k) == label)
        .count();
    format!("mounting cart · {label} {index}/{total}")
}

/// Draws the hand-off screen. A no-op if nothing is selected — callers only
/// reach [`Screen::Loading`](crate::shell::state::Screen::Loading) after
/// [`ShellState`] begins a load against a selection, but this keeps the
/// module safe to call standalone.
pub fn draw(
    surface: &mut Surface,
    state: &ShellState,
    carts: &[CartMeta],
    progress: &LoadProgress,
) {
    let Some(cart) = state.selected_cart().and_then(|i| carts.get(i)) else {
        return;
    };
    let m = *surface.metrics();
    surface.clear(color::VOID_900);

    let center_x = m.width as f32 / 2.0;
    let center_y = m.height as f32 * 0.46;
    let glow_side = m.loading_cover * 2.8;
    surface.fill_rect(
        Box2::new(
            center_x - glow_side / 2.0,
            center_y - glow_side / 2.0,
            glow_side,
            glow_side,
        ),
        f32::INFINITY,
        color::EMBER.with_alpha(0.16),
    );

    let cover = Box2::new(
        center_x - m.loading_cover / 2.0,
        center_y - m.loading_cover / 2.0,
        m.loading_cover,
        m.loading_cover,
    );
    surface.fill_rect(cover, radius::DEFAULT, swatch_for(&cart.id));

    let mut ty = cover.y + cover.h + space::X6 as f32;
    let title_style = TextStyle::new(
        Family::Display,
        Weight::Bold,
        m.text.loading_title,
        color::INK,
    )
    .tracked(tracking::TIGHT);
    surface.draw_text(
        title_style,
        center_x,
        ty,
        Align::Center,
        cart.display_title(),
    );

    if !cart.author.is_empty() {
        ty += space::X4 as f32 + m.text.body;
        let author_style =
            TextStyle::new(Family::Body, Weight::Regular, m.text.body, color::INK_DIM);
        surface.draw_text(
            author_style,
            center_x,
            ty,
            Align::Center,
            &format!("by {}", cart.author),
        );
    }

    let bar_w = m.loading_cover * 1.7;
    let bar_h = space::X1 as f32 * 0.75;
    let bar_x = center_x - bar_w / 2.0;
    let bar_y = ty + space::X6 as f32;
    surface.fill_rect(
        Box2::new(bar_x, bar_y, bar_w, bar_h),
        f32::INFINITY,
        color::VOID_700,
    );
    let fraction = progress.fraction.clamp(0.0, 1.0);
    if fraction > 0.0 {
        surface.fill_rect(
            Box2::new(bar_x, bar_y, bar_w * fraction, bar_h),
            f32::INFINITY,
            color::EMBER,
        );
    }

    let stage_style = TextStyle::new(
        Family::Mono,
        Weight::Regular,
        m.text.mono_spec,
        color::INK_FAINT,
    );
    surface.draw_text(
        stage_style,
        center_x,
        bar_y + space::X4 as f32,
        Align::Center,
        &progress.stage,
    );

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
        "Hold MENU at any time to pause",
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shell::state::ShellButton;
    use std::path::PathBuf;

    #[test]
    fn stage_text_counts_sections_of_the_same_kind() {
        let sections = [
            SectionKind::Program,
            SectionKind::SpriteSheet,
            SectionKind::SpriteBank,
            SectionKind::Map,
        ];
        assert_eq!(stage_text(&sections, 0), "mounting cart · program 1/1");
        assert_eq!(stage_text(&sections, 1), "mounting cart · sprites 1/2");
        assert_eq!(stage_text(&sections, 2), "mounting cart · sprites 2/2");
        assert_eq!(stage_text(&sections, 3), "mounting cart · map 1/1");
    }

    #[test]
    fn stage_text_reads_running_init_once_every_section_is_mounted() {
        let sections = [SectionKind::Program];
        assert_eq!(stage_text(&sections, 1), "running _init()");
        assert_eq!(stage_text(&sections, 5), "running _init()");
    }

    #[test]
    fn stage_text_on_an_empty_section_table_is_running_init() {
        assert_eq!(stage_text(&[], 0), "running _init()");
    }

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

    fn loading_state(carts: &[CartMeta]) -> ShellState {
        let mut state = ShellState::new();
        state.tick(crate::shell::state::BOOT_DURATION);
        state.set_cart_count(carts.len());
        state.press(ShellButton::A); // Library "A" -> begin_load -> Loading
        state
    }

    #[test]
    fn drawing_loading_at_every_progress_point_does_not_panic() {
        let mut s = surface();
        let carts = [cart("ember-drift")];
        let state = loading_state(&carts);
        assert_eq!(state.selected_cart(), Some(0));

        for fraction in [0.0, 0.5, 1.0] {
            let progress = LoadProgress {
                fraction,
                stage: stage_text(&carts[0].kinds, (fraction * 2.0) as usize),
            };
            draw(&mut s, &state, &carts, &progress);
        }
    }

    #[test]
    fn drawing_loading_with_no_selection_is_a_safe_no_op() {
        let mut s = surface();
        let state = ShellState::new();
        let progress = LoadProgress {
            fraction: 0.0,
            stage: "mounting cart".to_string(),
        };
        draw(&mut s, &state, &[], &progress);
    }

    #[test]
    fn a_cart_with_no_author_still_draws() {
        let mut s = surface();
        let carts = [CartMeta {
            id: "bare".to_string(),
            path: PathBuf::from("bare.cav"),
            title: String::new(),
            author: String::new(),
            bytes: 512,
            kinds: vec![],
        }];
        let state = loading_state(&carts);
        let progress = LoadProgress {
            fraction: 1.0,
            stage: "running _init()".to_string(),
        };
        draw(&mut s, &state, &carts, &progress);
    }

    #[test]
    fn wide_layout_loading_draws_without_panicking() {
        let mut s = Surface::new(1280, 720).expect("1280×720 surface");
        let carts = [cart("ember-drift")];
        let state = loading_state(&carts);
        let progress = LoadProgress {
            fraction: 0.3,
            stage: stage_text(&carts[0].kinds, 0),
        };
        draw(&mut s, &state, &carts, &progress);
    }
}
