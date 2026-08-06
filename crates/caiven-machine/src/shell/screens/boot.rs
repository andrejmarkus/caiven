//! The boot screen: ember radial behind the wordmark, a progress bar tied to
//! [`ShellState::boot_elapsed`], and a spec line built from the running
//! binary's version and the VM's configured limits.
//!
//! No chrome — [`crate::shell::state::Screen::Boot`] is immersive, so this
//! module owns the whole frame rather than a content area between two bars.

use caiven_vm::VmConfig;

use crate::shell::state::{BOOT_DURATION, ShellState};
use crate::shell::surface::{Align, Box2, Surface, TextStyle};
use crate::shell::theme::{Family, Weight, color, space, tracking};

/// Concentric rings behind the wordmark, outermost first, each fainter than
/// the last — a radial glow built from flat fills since the surface has no
/// gradient primitive.
const GLOW_RINGS: &[(f32, f32)] = &[(1.0, 0.05), (0.7, 0.09), (0.45, 0.14), (0.22, 0.22)];

fn format_kb(bytes: usize) -> String {
    let kb = bytes.div_ceil(1024).max(1);
    format!("{kb}KB")
}

fn spec_line(version: &str, config: &VmConfig) -> String {
    format!(
        "v{version} · {}×{} · {} · {} colors",
        config.width,
        config.height,
        format_kb(config.memory_size),
        config.palette_size
    )
}

/// Draws the boot screen. `version` is normally `env!("CARGO_PKG_VERSION")`;
/// `config` is the VM's configured limits, both threaded through by the
/// caller rather than read off a global so the module stays testable.
pub fn draw(surface: &mut Surface, state: &ShellState, version: &str, config: &VmConfig) {
    let m = *surface.metrics();
    surface.clear(color::VOID_900);

    let center_x = m.width as f32 / 2.0;
    let center_y = m.height as f32 * 0.42;
    let glow_radius = m.text.boot_wordmark * 1.6;

    for &(scale, alpha) in GLOW_RINGS {
        let side = glow_radius * scale;
        surface.fill_rect(
            Box2::new(center_x - side / 2.0, center_y - side / 2.0, side, side),
            f32::INFINITY,
            color::EMBER.with_alpha(alpha),
        );
    }

    let wordmark_style = TextStyle::new(
        Family::Display,
        Weight::Bold,
        m.text.boot_wordmark,
        color::INK,
    )
    .tracked(tracking::TIGHT);
    surface.draw_text(
        wordmark_style,
        center_x,
        center_y + m.text.boot_wordmark / 3.0,
        Align::Center,
        "CAIVEN",
    );

    // The lockup is letter-spaced, which pushes its visual center right of
    // its box center by half the tracking; nudge left to compensate (see
    // `tracking::LOCKUP`'s doc comment).
    let lockup_style = TextStyle::new(
        Family::Mono,
        Weight::Medium,
        m.text.boot_lockup,
        color::INK_DIM,
    )
    .tracked(tracking::LOCKUP);
    let lockup_pad = m.text.boot_lockup * tracking::LOCKUP / 2.0;
    surface.draw_text(
        lockup_style,
        center_x + lockup_pad,
        center_y + m.text.boot_wordmark * 0.62,
        Align::Center,
        "MACHINE",
    );

    let bar_w = m.width as f32 * 0.42;
    let bar_h = space::X1 as f32;
    let bar_x = center_x - bar_w / 2.0;
    let bar_y = m.height as f32 * 0.78;
    surface.fill_rect(
        Box2::new(bar_x, bar_y, bar_w, bar_h),
        f32::INFINITY,
        color::VOID_700,
    );
    let progress = if BOOT_DURATION.is_zero() {
        1.0
    } else {
        (state.boot_elapsed().as_secs_f32() / BOOT_DURATION.as_secs_f32()).clamp(0.0, 1.0)
    };
    if progress > 0.0 {
        surface.fill_rect(
            Box2::new(bar_x, bar_y, bar_w * progress, bar_h),
            f32::INFINITY,
            color::EMBER,
        );
    }

    let spec_style = TextStyle::new(
        Family::Mono,
        Weight::Regular,
        m.text.mono_micro,
        color::INK_FAINT,
    )
    .tracked(tracking::SPEC);
    surface.draw_text(
        spec_style,
        center_x,
        bar_y + space::X4 as f32,
        Align::Center,
        &spec_line(version, config),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> VmConfig {
        VmConfig::default()
    }

    #[test]
    fn spec_line_reads_version_and_limits() {
        let line = spec_line("0.1.0", &test_config());
        assert!(line.starts_with("v0.1.0 · "));
        assert!(line.contains("colors"));
    }

    #[test]
    fn kb_never_reads_zero_for_a_small_size() {
        assert_eq!(format_kb(1), "1KB");
        assert_eq!(format_kb(1024), "1KB");
        assert_eq!(format_kb(1025), "2KB");
    }

    #[test]
    fn drawing_boot_does_not_panic_at_every_progress_point() {
        let mut surface = Surface::new(640, 480).expect("surface");
        let mut state = ShellState::new();
        let config = test_config();
        draw(&mut surface, &state, "0.1.0", &config);
        state.tick(BOOT_DURATION / 2);
        draw(&mut surface, &state, "0.1.0", &config);
        state.tick(BOOT_DURATION);
        draw(&mut surface, &state, "0.1.0", &config);
    }

    #[test]
    fn wide_layout_boot_draws_without_panicking() {
        let mut surface = Surface::new(1280, 720).expect("surface");
        let state = ShellState::new();
        draw(&mut surface, &state, "0.1.0", &test_config());
    }
}
