//! Obsidian & Ember design tokens for the console shell.
//!
//! Every color, size, radius, shadow and duration the shell draws with is
//! named here and referenced by name. Inlining a hex code at a call site is
//! how a design system drifts, so don't.
//!
//! Values come from the Caiven Machine design handoff. Where the handoff
//! states a number for both the 640×480 and 1280×720 layouts, both are
//! written out; where it only states the small one, the wide value is the
//! documented ~1.6× scale, rounded.

/// A straight RGBA color, in the same byte order the VM and SDL use
/// (`caiven-core/src/memory.rs`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    /// Opaque color from a `0xRRGGBB` literal, so the constants below read
    /// like the hex codes in the design doc.
    pub const fn rgb(hex: u32) -> Self {
        Self {
            r: ((hex >> 16) & 0xFF) as u8,
            g: ((hex >> 8) & 0xFF) as u8,
            b: (hex & 0xFF) as u8,
            a: 255,
        }
    }

    /// Same color at a fractional alpha. `alpha` is clamped to `0.0..=1.0`.
    pub const fn with_alpha(self, alpha: f32) -> Self {
        let clamped = if alpha < 0.0 {
            0.0
        } else if alpha > 1.0 {
            1.0
        } else {
            alpha
        };
        Self {
            a: (clamped * 255.0 + 0.5) as u8,
            ..self
        }
    }

    /// Premultiplied RGBA bytes, which is what a raster surface wants.
    pub const fn premultiplied(self) -> [u8; 4] {
        let a = self.a as u16;
        [
            ((self.r as u16 * a) / 255) as u8,
            ((self.g as u16 * a) / 255) as u8,
            ((self.b as u16 * a) / 255) as u8,
            self.a,
        ]
    }
}

/// Named colors. `ember` is the *only* interactive color — if something is
/// selectable, focused or in progress, it is ember, and if it is ember it is
/// one of those things.
pub mod color {
    use super::Color;

    /// Focus rings, selected fills, primary buttons, progress bars, stars.
    pub const EMBER: Color = Color::rgb(0xFEB05D);
    /// Text and icons drawn on top of an ember fill.
    pub const EMBER_INK: Color = Color::rgb(0x3A2308);
    /// Hover tint. Desktop only — a handheld has no hover state.
    pub const EMBER_BRIGHT: Color = Color::rgb(0xFFC685);

    /// Background of the "Installed" badge. Never a button, never a large
    /// surface.
    pub const SHEEN_WASH: Color = Color::rgb(0x343A4A);
    /// Text on `SHEEN_WASH`; the Port hostname in the status bar.
    pub const SHEEN_BRIGHT: Color = Color::rgb(0x93A8DE);

    /// Delete-cart fill and crash-screen accents.
    pub const DESTRUCTIVE: Color = Color::rgb(0xE5555F);
    /// Destructive text on a dark surface.
    pub const DESTRUCTIVE_BRIGHT: Color = Color::rgb(0xF27B83);
    /// Text on a destructive fill.
    pub const DESTRUCTIVE_INK: Color = Color::rgb(0x2B0709);

    /// App background.
    pub const VOID_900: Color = Color::rgb(0x2B2A2A);
    /// Status bar, legend bar, cards, panels, selected setting row.
    pub const VOID_800: Color = Color::rgb(0x3F3E3E);
    /// Secondary chips, slider and progress tracks, active nav pill.
    pub const VOID_700: Color = Color::rgb(0x4F4E4E);
    /// Every border and divider, always 1px.
    pub const VOID_600: Color = Color::rgb(0x605E5E);

    /// Primary text.
    pub const INK: Color = Color::rgb(0xF5F2F2);
    /// Secondary text, legend labels.
    pub const INK_DIM: Color = Color::rgb(0x9A9898);
    /// Spec lines, disabled state, captions.
    pub const INK_FAINT: Color = Color::rgb(0x727070);

    /// Behind a running cart, and nothing else.
    pub const CART_BACKDROP: Color = Color::rgb(0x000000);

    /// Deterministic per-cart identity colors, for a cover with no captured
    /// screenshot — the library only knows a cart's header and section
    /// table (T37), never its art. Picked by hashing the cart id; see
    /// `shell/screens/library.rs::swatch_for`.
    pub const SWATCH: [Color; 5] = [
        Color::rgb(0x20337B),
        Color::rgb(0x5E2C5C),
        Color::rgb(0x287252),
        Color::rgb(0x7D523A),
        Color::rgb(0xC83C46),
    ];

    /// Logo body color. Deliberately not exported as a UI surface — the
    /// handoff forbids it appearing as one, and it exists here only so the
    /// wordmark has a name to reach for if a mark is ever drawn.
    pub const OBSIDIAN_LOGO_ONLY: Color = Color::rgb(0x3B3E48);
}

/// The type faces. Which file backs each is the font module's business
/// (T34); the shell only ever names a role.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Family {
    /// Space Grotesk. Wordmark, screen titles, cart titles.
    Display,
    /// Inter. Rows, labels, blurbs.
    Body,
    /// JetBrains Mono. Clock, versions, sizes, paths, key codes, percentages.
    Mono,
}

/// Font weight, as the numeric weights the handoff names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Weight {
    Regular = 400,
    Medium = 500,
    SemiBold = 600,
    Bold = 700,
}

/// Extra tracking applied to a run of text, in em.
pub mod tracking {
    /// Display text at 24px and up.
    pub const TIGHT: f32 = -0.02;
    /// Uppercase treatment: status-bar wordmark, eyebrows, tab labels, the
    /// "Paused" title.
    pub const CAPS: f32 = 0.08;
    /// Mono spec lines.
    pub const SPEC: f32 = 0.04;
    /// The `MACHINE` lockup under the boot wordmark. Needs a left pad equal
    /// to the tracking to stay optically centered.
    pub const LOCKUP: f32 = 0.42;
}

/// Font sizes in px for one layout. Roles, not sizes, are what call sites
/// reference — a role can be retuned in one place.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TypeScale {
    pub boot_wordmark: f32,
    pub boot_lockup: f32,
    pub hero_title: f32,
    pub detail_title: f32,
    pub crash_title: f32,
    pub loading_title: f32,
    pub empty_title: f32,
    pub hero_cover_title: f32,
    pub port_row_title: f32,
    pub pause_title: f32,
    pub pause_item: f32,
    pub body: f32,
    pub legend_label: f32,
    pub caps_label: f32,
    pub mono_spec: f32,
    pub mono_micro: f32,
    /// The cart name printed across a shelf tile.
    pub shelf_tile_title: f32,
}

/// Bar heights, paddings and gaps in px for one layout.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Metrics {
    pub width: u32,
    pub height: u32,
    pub status_bar_h: u32,
    pub legend_bar_h: u32,
    pub status_bar_pad_x: u32,
    pub legend_bar_pad_x: u32,
    pub legend_gap: u32,
    pub screen_pad_x: u32,
    pub screen_pad_y: u32,
    /// Side length of the library hero cover.
    pub hero_cover: f32,
    /// Side length of a library shelf tile.
    pub shelf_tile: f32,
    /// Gap between adjacent shelf tiles.
    pub shelf_gap: u32,
    pub text: TypeScale,
}

impl Metrics {
    /// Height left for screen content between the two chrome bars. Screens
    /// without chrome (boot, loading, playing, pause, crash) use the full
    /// [`Self::height`] instead.
    pub const fn content_height(&self) -> u32 {
        self.height
            .saturating_sub(self.status_bar_h)
            .saturating_sub(self.legend_bar_h)
    }

    /// Y of the first content pixel below the status bar.
    pub const fn content_top(&self) -> u32 {
        self.status_bar_h
    }
}

/// The handheld layout, and the one the design is authored against.
pub const METRICS_640: Metrics = Metrics {
    width: 640,
    height: 480,
    status_bar_h: 30,
    legend_bar_h: 36,
    status_bar_pad_x: 12,
    legend_bar_pad_x: 14,
    legend_gap: 18,
    screen_pad_x: 18,
    screen_pad_y: 16,
    hero_cover: 186.0,
    shelf_tile: 86.0,
    shelf_gap: 12,
    text: TypeScale {
        boot_wordmark: 72.0,
        boot_lockup: 15.0,
        hero_title: 32.0,
        detail_title: 28.0,
        crash_title: 26.0,
        loading_title: 24.0,
        empty_title: 20.0,
        hero_cover_title: 20.0,
        port_row_title: 15.0,
        pause_title: 15.0,
        pause_item: 14.0,
        body: 13.0,
        legend_label: 12.0,
        caps_label: 12.0,
        mono_spec: 11.0,
        mono_micro: 10.0,
        shelf_tile_title: 11.0,
    },
};

/// The desktop-window and TV layout. `hero_title`, `body`, `legend_label`
/// and `mono_spec` are the exact numbers the handoff gives; the rest follow
/// its documented ~1.6× scale.
pub const METRICS_1280: Metrics = Metrics {
    width: 1280,
    height: 720,
    status_bar_h: 44,
    legend_bar_h: 52,
    status_bar_pad_x: 20,
    legend_bar_pad_x: 24,
    legend_gap: 28,
    screen_pad_x: 36,
    screen_pad_y: 32,
    hero_cover: 297.0,
    shelf_tile: 138.0,
    shelf_gap: 19,
    text: TypeScale {
        boot_wordmark: 115.0,
        boot_lockup: 24.0,
        hero_title: 56.0,
        detail_title: 45.0,
        crash_title: 42.0,
        loading_title: 38.0,
        empty_title: 32.0,
        hero_cover_title: 32.0,
        port_row_title: 24.0,
        pause_title: 24.0,
        pause_item: 22.0,
        body: 17.0,
        legend_label: 15.0,
        caps_label: 15.0,
        mono_spec: 13.0,
        mono_micro: 13.0,
        shelf_tile_title: 17.0,
    },
};

/// Picks the layout for a surface size. The wide layout only applies once
/// the surface is at least as large as it was designed for; anything
/// smaller stays on the handheld scale rather than rendering clipped
/// chrome.
pub const fn metrics_for(width: u32, height: u32) -> Metrics {
    if width >= METRICS_1280.width && height >= METRICS_1280.height {
        METRICS_1280
    } else {
        METRICS_640
    }
}

/// Corner radii in px. 8 is the default; reaching for anything else should
/// be deliberate.
pub mod radius {
    /// Small chips and thumbnails.
    pub const SMALL: f32 = 4.0;
    /// Buttons, rows, cards, cart tiles.
    pub const DEFAULT: f32 = 8.0;
    /// Large panels: the pause card, frame corners.
    pub const LARGE: f32 = 12.0;
    /// Legend chips, badges, the switch. Resolved against the shape's
    /// height at draw time.
    pub const PILL: f32 = f32::INFINITY;
}

/// The 4px base spacing scale.
pub mod space {
    pub const X1: u32 = 4;
    pub const X2: u32 = 8;
    pub const X3: u32 = 12;
    pub const X4: u32 = 16;
    pub const X5: u32 = 20;
    pub const X6: u32 = 24;
}

/// A drop shadow. Shadows are black-only; the one exception is
/// [`shadow::EMBER_GLOW`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Shadow {
    pub offset_x: f32,
    pub offset_y: f32,
    pub blur: f32,
    pub color: Color,
}

pub mod shadow {
    use super::{Color, Shadow, color};

    const fn black(alpha: f32) -> Color {
        color::CART_BACKDROP.with_alpha(alpha)
    }

    pub const SM: Shadow = Shadow {
        offset_x: 0.0,
        offset_y: 1.0,
        blur: 2.0,
        color: black(0.35),
    };
    pub const MD: Shadow = Shadow {
        offset_x: 0.0,
        offset_y: 4.0,
        blur: 12.0,
        color: black(0.4),
    };
    /// The pause card.
    pub const LG: Shadow = Shadow {
        offset_x: 0.0,
        offset_y: 12.0,
        blur: 32.0,
        color: black(0.5),
    };

    /// The single permitted colored glow: hero cover, focused Port row,
    /// focused remap row, the status-bar dot. Drawn as a 1px ember ring
    /// under a soft ember bloom.
    pub const EMBER_GLOW_RING: Color = color::EMBER.with_alpha(0.35);
    pub const EMBER_GLOW: Shadow = Shadow {
        offset_x: 0.0,
        offset_y: 0.0,
        blur: 24.0,
        color: color::EMBER.with_alpha(0.18),
    };
}

/// Focus treatments. Focus is always ember and always visible — it is the
/// only thing telling the user where they are.
pub mod focus {
    use super::{Color, color};

    /// Ring width on the hero cover and focused rows.
    pub const RING_WIDTH: f32 = 2.0;
    pub const RING_COLOR: Color = color::EMBER;
    /// Bloom around the hero cover.
    pub const HERO_BLOOM: Color = color::EMBER.with_alpha(0.22);
    /// Border on an unfocused shelf tile, which is 1px rather than 2px.
    pub const UNFOCUSED_BORDER: Color = color::VOID_600;
    /// Unfocused shelf tiles are dimmed rather than recolored.
    pub const UNFOCUSED_OPACITY: f32 = 0.78;
}

/// Animation timings. The motion budget on a handheld is deliberately
/// small: only the focus ring and the boot glow run longer than a single
/// transition, and nothing bounces, springs or scale-pops.
pub mod motion {
    use std::time::Duration;

    /// Selection feedback.
    pub const SELECT: Duration = Duration::from_millis(120);
    /// Panel and pane changes.
    pub const PANEL: Duration = Duration::from_millis(180);
    /// Boot glow, and the boot progress bar it paces.
    pub const BOOT: Duration = Duration::from_millis(1200);

    /// `cubic-bezier(.16, 1, .3, 1)` — the one easing curve.
    pub const EASE: [f32; 4] = [0.16, 1.0, 0.3, 1.0];
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rgb_parses_hex_channels_in_order() {
        let c = Color::rgb(0xFEB05D);
        assert_eq!((c.r, c.g, c.b, c.a), (0xFE, 0xB0, 0x5D, 255));
    }

    #[test]
    fn with_alpha_clamps_and_scales() {
        assert_eq!(color::EMBER.with_alpha(0.0).a, 0);
        assert_eq!(color::EMBER.with_alpha(1.0).a, 255);
        assert_eq!(color::EMBER.with_alpha(-1.0).a, 0);
        assert_eq!(color::EMBER.with_alpha(2.0).a, 255);
        // 0.35 is the ember ring alpha the handoff specifies.
        assert_eq!(color::EMBER.with_alpha(0.35).a, 89);
    }

    #[test]
    fn premultiply_scales_channels_by_alpha() {
        let opaque = color::EMBER.premultiplied();
        assert_eq!(opaque, [0xFE, 0xB0, 0x5D, 255]);

        let clear = color::EMBER.with_alpha(0.0).premultiplied();
        assert_eq!(clear, [0, 0, 0, 0]);

        // Premultiplied channels can never exceed alpha, which is the
        // invariant a raster backend will assert on.
        let half = color::EMBER.with_alpha(0.5).premultiplied();
        assert!(half[0] <= half[3] && half[1] <= half[3] && half[2] <= half[3]);
    }

    #[test]
    fn content_height_excludes_both_chrome_bars() {
        assert_eq!(METRICS_640.content_height(), 480 - 30 - 36);
        assert_eq!(METRICS_640.content_top(), 30);
        assert_eq!(METRICS_1280.content_height(), 720 - 44 - 52);
    }

    /// The handoff's hard floor. A shell that renders 9px text is unreadable
    /// on a 640×480 handheld panel, so guard every role at once.
    #[test]
    fn no_type_role_falls_below_ten_px() {
        for m in [METRICS_640, METRICS_1280] {
            let t = m.text;
            for size in [
                t.boot_wordmark,
                t.boot_lockup,
                t.hero_title,
                t.detail_title,
                t.crash_title,
                t.loading_title,
                t.empty_title,
                t.hero_cover_title,
                t.port_row_title,
                t.pause_title,
                t.pause_item,
                t.body,
                t.legend_label,
                t.caps_label,
                t.mono_spec,
                t.mono_micro,
                t.shelf_tile_title,
            ] {
                assert!(size >= 10.0, "type role {size}px below the 10px floor");
            }
        }
    }

    /// Retuning one layout without the other is the easy mistake here, so
    /// this fails the build rather than a test run.
    const _WIDE_LAYOUT_IS_NEVER_SMALLER: () = {
        assert!(METRICS_1280.status_bar_h > METRICS_640.status_bar_h);
        assert!(METRICS_1280.legend_bar_h > METRICS_640.legend_bar_h);
        assert!(METRICS_1280.screen_pad_x > METRICS_640.screen_pad_x);
        assert!(METRICS_1280.text.body > METRICS_640.text.body);
        assert!(METRICS_1280.text.hero_title > METRICS_640.text.hero_title);
        assert!(METRICS_1280.hero_cover > METRICS_640.hero_cover);
        assert!(METRICS_1280.shelf_tile > METRICS_640.shelf_tile);
    };

    #[test]
    fn metrics_for_only_upgrades_at_the_full_wide_size() {
        assert_eq!(metrics_for(640, 480), METRICS_640);
        assert_eq!(metrics_for(1280, 720), METRICS_1280);
        assert_eq!(metrics_for(1920, 1080), METRICS_1280);
        // A window one pixel short of the wide layout keeps the small one
        // rather than clipping its chrome.
        assert_eq!(metrics_for(1280, 719), METRICS_640);
        assert_eq!(metrics_for(1279, 720), METRICS_640);
    }
}
