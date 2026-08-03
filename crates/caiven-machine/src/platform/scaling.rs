//! Where the console framebuffer lands inside the window.
//!
//! Pure geometry, deliberately free of SDL types so it can be exhaustively
//! unit-tested. The console shell's Settings pane will drive the same two
//! knobs later.

use clap::ValueEnum;

/// How large the framebuffer is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Default)]
pub enum ScaleMode {
    /// Fill the window's height, whatever factor that takes. The handheld
    /// default: a 128×128 console becomes 480×480 on a 640×480 panel.
    #[default]
    Fit,
    /// Exactly 2× the console resolution.
    #[value(name = "2x")]
    Integer2x,
    /// Exactly 3× the console resolution.
    #[value(name = "3x")]
    Integer3x,
}

/// Whether pixels stay square.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Default)]
pub enum AspectMode {
    /// 1:1 pixels, pillarboxed on black.
    #[default]
    Square,
    /// Stretch to the window's full width, distorting pixels.
    Stretch,
}

/// A destination rectangle in window pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DstRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// Places a `console`-sized framebuffer inside a `window`-sized surface.
///
/// The result is always centered and always at least 1×1, so a window
/// smaller than one console pixel still renders something rather than
/// producing a zero-area texture copy.
pub fn dst_rect(
    window: (u32, u32),
    console: (u32, u32),
    scale: ScaleMode,
    aspect: AspectMode,
) -> DstRect {
    let (win_w, win_h) = window;
    let (con_w, con_h) = console;

    if con_w == 0 || con_h == 0 {
        return DstRect {
            x: 0,
            y: 0,
            width: win_w.max(1),
            height: win_h.max(1),
        };
    }

    let height = match scale {
        ScaleMode::Fit => win_h,
        ScaleMode::Integer2x => con_h * 2,
        ScaleMode::Integer3x => con_h * 3,
    }
    .max(1);

    let width = match aspect {
        // Stretch always spans the window regardless of scale mode — the
        // point of the mode is to fill the panel edge to edge.
        AspectMode::Stretch => win_w,
        // Keep pixels square by deriving width from the chosen height.
        AspectMode::Square => (height as u64 * con_w as u64 / con_h as u64) as u32,
    }
    .max(1);

    DstRect {
        x: (win_w as i32 - width as i32) / 2,
        y: (win_h as i32 - height as i32) / 2,
        width,
        height,
    }
}

#[cfg(test)]
mod tests {
    use super::{AspectMode, ScaleMode, dst_rect};

    const CONSOLE: (u32, u32) = (128, 128);
    const HANDHELD: (u32, u32) = (640, 480);
    const DESKTOP: (u32, u32) = (1280, 720);

    #[test]
    fn fit_square_fills_handheld_height_and_pillarboxes() {
        let r = dst_rect(HANDHELD, CONSOLE, ScaleMode::Fit, AspectMode::Square);
        assert_eq!((r.width, r.height), (480, 480));
        // 80px of black on each side of a 640px panel.
        assert_eq!(r.x, 80);
        assert_eq!(r.y, 0);
    }

    #[test]
    fn fit_stretch_spans_the_full_panel_width() {
        let r = dst_rect(HANDHELD, CONSOLE, ScaleMode::Fit, AspectMode::Stretch);
        assert_eq!((r.width, r.height), (640, 480));
        assert_eq!((r.x, r.y), (0, 0));
    }

    #[test]
    fn integer_modes_use_exact_multiples() {
        let two = dst_rect(HANDHELD, CONSOLE, ScaleMode::Integer2x, AspectMode::Square);
        assert_eq!((two.width, two.height), (256, 256));
        let three = dst_rect(HANDHELD, CONSOLE, ScaleMode::Integer3x, AspectMode::Square);
        assert_eq!((three.width, three.height), (384, 384));
    }

    #[test]
    fn integer_modes_stay_centered() {
        let r = dst_rect(HANDHELD, CONSOLE, ScaleMode::Integer2x, AspectMode::Square);
        assert_eq!(r.x, (640 - 256) / 2);
        assert_eq!(r.y, (480 - 256) / 2);
    }

    #[test]
    fn desktop_window_fits_to_height_too() {
        let r = dst_rect(DESKTOP, CONSOLE, ScaleMode::Fit, AspectMode::Square);
        assert_eq!((r.width, r.height), (720, 720));
        assert_eq!(r.x, (1280 - 720) / 2);
    }

    #[test]
    fn non_square_console_keeps_its_aspect_in_square_mode() {
        // Guards the width derivation against being hardcoded to 1:1.
        let r = dst_rect(HANDHELD, (160, 128), ScaleMode::Fit, AspectMode::Square);
        assert_eq!(r.height, 480);
        assert_eq!(r.width, 600);
    }

    #[test]
    fn integer_scale_larger_than_window_is_allowed_and_clips_symmetrically() {
        // 3× a 128px console is 384px, taller than a 200px-high window. The
        // rect goes negative rather than silently shrinking — SDL clips it,
        // and the framebuffer stays at true 3× as asked.
        let r = dst_rect(
            (200, 200),
            CONSOLE,
            ScaleMode::Integer3x,
            AspectMode::Square,
        );
        assert_eq!((r.width, r.height), (384, 384));
        assert_eq!(r.x, (200 - 384) / 2);
        assert!(r.x < 0);
    }

    #[test]
    fn degenerate_sizes_never_produce_a_zero_area_rect() {
        let r = dst_rect((0, 0), CONSOLE, ScaleMode::Fit, AspectMode::Square);
        assert!(r.width >= 1 && r.height >= 1);
        let r = dst_rect(HANDHELD, (0, 0), ScaleMode::Fit, AspectMode::Square);
        assert!(r.width >= 1 && r.height >= 1);
    }
}
