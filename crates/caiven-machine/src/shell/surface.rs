//! The shell's CPU raster surface.
//!
//! There is no GPU on the target device — a Miyoo-class handheld has a 2D
//! blitter and nothing else — so the shell rasterizes into an RGBA buffer
//! with `tiny-skia` and hands that buffer to SDL as a second streaming
//! texture, beside the console framebuffer.
//!
//! The budget is the reason for the dirty flag. Repainting 640×480 of
//! panels and text every frame would eat the whole 16ms on a 1.2GHz
//! Cortex-A7, and a menu has nothing to animate between keypresses, so the
//! surface only redraws when [`Surface::mark_dirty`] says something moved.

use anyhow::{Context, Result, anyhow};
use tiny_skia::{
    FillRule, LineCap, LineJoin, Paint, PathBuilder, Pixmap, PixmapPaint, PixmapRef, Rect, Stroke,
    Transform,
};

use crate::shell::font::Fonts;
use crate::shell::icon::Icon;
use crate::shell::theme::{Color, Family, Metrics, Weight, metrics_for};

/// A rectangle in surface pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Box2 {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Box2 {
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }

    /// Shrinks by `by` on every side, never past zero.
    pub fn inset(self, by: f32) -> Self {
        Self {
            x: self.x + by,
            y: self.y + by,
            w: (self.w - by * 2.0).max(0.0),
            h: (self.h - by * 2.0).max(0.0),
        }
    }

    fn to_rect(self) -> Option<Rect> {
        Rect::from_xywh(self.x, self.y, self.w, self.h)
    }
}

/// Where a run of text sits relative to the x it is given.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    Left,
    Center,
    Right,
}

/// One run of text: which face, how big, how tracked, what color.
#[derive(Debug, Clone, Copy)]
pub struct TextStyle {
    pub family: Family,
    pub weight: Weight,
    pub size: f32,
    pub tracking: f32,
    pub color: Color,
}

impl TextStyle {
    pub const fn new(family: Family, weight: Weight, size: f32, color: Color) -> Self {
        Self {
            family,
            weight,
            size,
            tracking: 0.0,
            color,
        }
    }

    pub const fn tracked(mut self, tracking: f32) -> Self {
        self.tracking = tracking;
        self
    }
}

/// The RGBA surface the shell draws into, plus its glyph cache and dirty
/// flag.
pub struct Surface {
    pixmap: Pixmap,
    metrics: Metrics,
    fonts: Fonts,
    dirty: bool,
    /// Whether every pixel is fully transparent, recomputed in `mark_clean`.
    /// Lets the platform layer skip blending a no-op overlay.
    fully_transparent: bool,
}

impl Surface {
    /// Allocates a surface at the given size and picks the layout that fits
    /// it. Starts dirty — nothing has been drawn yet.
    pub fn new(width: u32, height: u32) -> Result<Self> {
        let pixmap = Pixmap::new(width, height)
            .ok_or_else(|| anyhow!("cannot allocate a {width}×{height} shell surface"))?;
        Ok(Self {
            pixmap,
            metrics: metrics_for(width, height),
            fonts: Fonts::load().context("failed to load the bundled shell faces")?,
            dirty: true,
            // A freshly allocated `Pixmap` is zero-filled, i.e. transparent.
            fully_transparent: true,
        })
    }

    /// The layout tokens for this surface's size.
    pub fn metrics(&self) -> &Metrics {
        &self.metrics
    }

    pub fn width(&self) -> u32 {
        self.pixmap.width()
    }

    pub fn height(&self) -> u32 {
        self.pixmap.height()
    }

    /// Whether the surface needs repainting before the next present.
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Records that shell state changed and the surface no longer matches
    /// it. Cheap enough to call on any state write.
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Marks the surface as matching the state it was drawn from. Call
    /// after a repaint, before uploading to the texture. Also recomputes
    /// [`Self::is_fully_transparent`].
    pub fn mark_clean(&mut self) {
        self.dirty = false;
        self.fully_transparent = self.pixmap.data().chunks_exact(4).all(|px| px[3] == 0);
    }

    /// Whether the last repaint left every pixel fully transparent.
    pub fn is_fully_transparent(&self) -> bool {
        self.fully_transparent
    }

    /// Resizes to a new window size, adopting whichever layout fits.
    /// A no-op at the current size, so it is safe to call per resize event.
    pub fn resize(&mut self, width: u32, height: u32) -> Result<()> {
        if width == self.width() && height == self.height() {
            return Ok(());
        }
        self.pixmap = Pixmap::new(width, height)
            .ok_or_else(|| anyhow!("cannot allocate a {width}×{height} shell surface"))?;
        self.metrics = metrics_for(width, height);
        self.dirty = true;
        Ok(())
    }

    /// The finished pixels: premultiplied RGBA, in the byte order SDL's
    /// `ABGR8888` expects and the console framebuffer already uses.
    ///
    /// Premultiplied is what `tiny-skia` produces, and it is what the
    /// texture must be told it is — an overlay uploaded as straight alpha
    /// would fringe dark. Fully opaque content (every menu screen, which
    /// clears to `void-900` first) is identical either way.
    pub fn rgba(&self) -> &[u8] {
        self.pixmap.data()
    }

    /// Borrowed view of the surface, for compositing it into something else.
    pub fn pixmap_ref(&self) -> PixmapRef<'_> {
        self.pixmap.as_ref()
    }

    /// The glyph cache, so a screen can measure text before laying it out.
    pub fn fonts(&mut self) -> &mut Fonts {
        &mut self.fonts
    }

    /// Clears the whole surface to one color. Every repaint starts here.
    pub fn clear(&mut self, color: Color) {
        self.pixmap.fill(to_skia(color));
    }

    /// Fills a rectangle, optionally rounded. `radius` of 0 is a plain
    /// rect; `f32::INFINITY` (the pill token) resolves to half the height.
    pub fn fill_rect(&mut self, bounds: Box2, radius: f32, color: Color) {
        let Some(path) = rounded_rect_path(bounds, radius) else {
            return;
        };
        let mut paint = Paint::default();
        paint.set_color(to_skia(color));
        paint.anti_alias = true;
        self.pixmap.fill_path(
            &path,
            &paint,
            FillRule::Winding,
            Transform::identity(),
            None,
        );
    }

    /// Strokes a rectangle's outline. Every border in the design is 1px, so
    /// that is the default `width`.
    pub fn stroke_rect(&mut self, bounds: Box2, radius: f32, width: f32, color: Color) {
        // A 1px stroke straddles the path, landing half a pixel either
        // side and rendering as two grey rows. Insetting by half the width
        // puts it back on the pixel grid.
        let Some(path) = rounded_rect_path(bounds.inset(width / 2.0), radius) else {
            return;
        };
        let mut paint = Paint::default();
        paint.set_color(to_skia(color));
        paint.anti_alias = true;
        let stroke = Stroke {
            width,
            ..Stroke::default()
        };
        self.pixmap
            .stroke_path(&path, &paint, &stroke, Transform::identity(), None);
    }

    /// Draws a stroked icon with its top-left at `x, y`.
    pub fn draw_icon(
        &mut self,
        icon: Icon,
        x: f32,
        y: f32,
        size: f32,
        stroke_width: f32,
        color: Color,
    ) -> Result<()> {
        let path = icon.path(size)?;
        let mut paint = Paint::default();
        paint.set_color(to_skia(color));
        paint.anti_alias = true;
        let stroke = Stroke {
            width: stroke_width,
            line_cap: LineCap::Round,
            line_join: LineJoin::Round,
            ..Stroke::default()
        };
        self.pixmap.stroke_path(
            &path,
            &paint,
            &stroke,
            Transform::from_translate(x, y),
            None,
        );
        Ok(())
    }

    /// Width a run would occupy, without drawing it.
    pub fn measure_text(&mut self, style: TextStyle, text: &str) -> f32 {
        self.fonts
            .measure(style.family, style.weight, style.size, style.tracking, text)
    }

    /// Draws a run of text with its baseline at `baseline_y`.
    ///
    /// Returns the advance actually consumed, so a caller composing runs
    /// (a label followed by a value) can continue from it.
    pub fn draw_text(
        &mut self,
        style: TextStyle,
        x: f32,
        baseline_y: f32,
        align: Align,
        text: &str,
    ) -> f32 {
        let width = self.measure_text(style, text);
        let mut pen = match align {
            Align::Left => x,
            Align::Center => x - width / 2.0,
            Align::Right => x - width,
        };

        let surface_w = self.pixmap.width() as i32;
        let surface_h = self.pixmap.height() as i32;
        let color = style.color;

        for ch in text.chars() {
            let Some(glyph) = self.fonts.glyph(style.family, style.weight, style.size, ch) else {
                continue;
            };
            let advance = glyph.advance;

            if glyph.width > 0 && glyph.height > 0 {
                let gx = (pen + glyph.left as f32).round() as i32;
                let gy = (baseline_y + glyph.top as f32).round() as i32;
                let (gw, gh) = (glyph.width, glyph.height);
                let coverage = glyph.coverage.as_slice();

                blend_mask(
                    self.pixmap.data_mut(),
                    surface_w,
                    surface_h,
                    coverage,
                    gw,
                    gh,
                    gx,
                    gy,
                    color,
                );
            }

            pen += advance + style.tracking * style.size;
        }

        width
    }

    /// Composites another pixmap — a cart screenshot, a label — at `x, y`.
    pub fn draw_pixmap(&mut self, x: i32, y: i32, source: PixmapRef<'_>, opacity: f32) {
        let paint = PixmapPaint {
            opacity: opacity.clamp(0.0, 1.0),
            // Nearest, always: this is a fantasy console, and its art is
            // 128×128. Smooth-scaling it is the one thing not to do.
            quality: tiny_skia::FilterQuality::Nearest,
            ..PixmapPaint::default()
        };
        self.pixmap
            .draw_pixmap(x, y, source, &paint, Transform::identity(), None);
    }
}

/// Blends an 8-bit coverage mask into the surface at `dst_x, dst_y`,
/// clipped to the surface. Straight (non-premultiplied) RGBA, source-over.
///
/// This is the hot loop of every menu repaint — one pass, no allocation,
/// no per-pixel bounds check beyond the row clip computed up front.
#[allow(clippy::too_many_arguments)]
fn blend_mask(
    dst: &mut [u8],
    dst_w: i32,
    dst_h: i32,
    mask: &[u8],
    mask_w: usize,
    mask_h: usize,
    dst_x: i32,
    dst_y: i32,
    color: Color,
) {
    let x0 = dst_x.max(0);
    let y0 = dst_y.max(0);
    let x1 = (dst_x + mask_w as i32).min(dst_w);
    let y1 = (dst_y + mask_h as i32).min(dst_h);
    if x0 >= x1 || y0 >= y1 {
        return;
    }

    for y in y0..y1 {
        let mask_row = (y - dst_y) as usize * mask_w;
        let dst_row = y as usize * dst_w as usize * 4;
        for x in x0..x1 {
            let coverage = mask[mask_row + (x - dst_x) as usize];
            if coverage == 0 {
                continue;
            }
            // Glyph coverage times the style's own alpha.
            let alpha = (coverage as u32 * color.a as u32) / 255;
            let i = dst_row + x as usize * 4;
            let Some(px) = dst.get_mut(i..i + 4) else {
                continue;
            };
            px[0] = blend(px[0], color.r, alpha);
            px[1] = blend(px[1], color.g, alpha);
            px[2] = blend(px[2], color.b, alpha);
            px[3] = px[3].max(alpha as u8);
        }
    }
}

#[inline]
fn blend(dst: u8, src: u8, alpha: u32) -> u8 {
    ((src as u32 * alpha + dst as u32 * (255 - alpha)) / 255) as u8
}

fn to_skia(color: Color) -> tiny_skia::Color {
    tiny_skia::Color::from_rgba8(color.r, color.g, color.b, color.a)
}

/// Builds a rect path, rounding the corners by `radius`. `f32::INFINITY`
/// (the pill token) rounds to half the shorter side.
fn rounded_rect_path(bounds: Box2, radius: f32) -> Option<tiny_skia::Path> {
    if bounds.w <= 0.0 || bounds.h <= 0.0 {
        return None;
    }
    let limit = bounds.w.min(bounds.h) / 2.0;
    let r = if radius.is_finite() {
        radius.clamp(0.0, limit)
    } else {
        limit
    };

    if r <= f32::EPSILON {
        return bounds.to_rect().map(PathBuilder::from_rect);
    }

    let (x, y, w, h) = (bounds.x, bounds.y, bounds.w, bounds.h);
    let k = r * crate::shell::icon::ARC_K;
    let mut b = PathBuilder::new();
    b.move_to(x + r, y);
    b.line_to(x + w - r, y);
    b.cubic_to(x + w - r + k, y, x + w, y + r - k, x + w, y + r);
    b.line_to(x + w, y + h - r);
    b.cubic_to(x + w, y + h - r + k, x + w - r + k, y + h, x + w - r, y + h);
    b.line_to(x + r, y + h);
    b.cubic_to(x + r - k, y + h, x, y + h - r + k, x, y + h - r);
    b.line_to(x, y + r);
    b.cubic_to(x, y + r - k, x + r - k, y, x + r, y);
    b.close();
    b.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shell::theme::{METRICS_640, METRICS_1280, color, radius};

    fn surface() -> Surface {
        Surface::new(640, 480).expect("640×480 surface")
    }

    fn pixel(s: &Surface, x: u32, y: u32) -> [u8; 4] {
        let i = (y as usize * s.width() as usize + x as usize) * 4;
        let d = s.rgba();
        [d[i], d[i + 1], d[i + 2], d[i + 3]]
    }

    #[test]
    fn a_new_surface_adopts_the_layout_for_its_size() {
        assert_eq!(*surface().metrics(), METRICS_640);
        let wide = Surface::new(1280, 720).expect("wide surface");
        assert_eq!(*wide.metrics(), METRICS_1280);
    }

    #[test]
    fn clear_writes_the_color_in_rgba_byte_order() {
        let mut s = surface();
        s.clear(color::VOID_900);
        // 0x2B2A2A, opaque — R then G then B then A, matching the console
        // framebuffer and SDL's ABGR8888.
        assert_eq!(pixel(&s, 0, 0), [0x2B, 0x2A, 0x2A, 0xFF]);
        assert_eq!(pixel(&s, 639, 479), [0x2B, 0x2A, 0x2A, 0xFF]);
    }

    #[test]
    fn fill_rect_covers_its_interior_and_leaves_the_outside_alone() {
        let mut s = surface();
        s.clear(color::VOID_900);
        s.fill_rect(Box2::new(100.0, 100.0, 60.0, 40.0), 0.0, color::EMBER);
        assert_eq!(pixel(&s, 130, 120), [0xFE, 0xB0, 0x5D, 0xFF]);
        assert_eq!(pixel(&s, 99, 99), [0x2B, 0x2A, 0x2A, 0xFF]);
        assert_eq!(pixel(&s, 161, 141), [0x2B, 0x2A, 0x2A, 0xFF]);
    }

    #[test]
    fn a_rounded_rect_leaves_its_corners_empty() {
        let mut s = surface();
        s.clear(color::VOID_900);
        s.fill_rect(
            Box2::new(10.0, 10.0, 80.0, 80.0),
            radius::LARGE,
            color::EMBER,
        );
        // The very corner pixel is outside a 12px radius.
        assert_eq!(pixel(&s, 10, 10), [0x2B, 0x2A, 0x2A, 0xFF]);
        assert_eq!(pixel(&s, 50, 50), [0xFE, 0xB0, 0x5D, 0xFF]);
    }

    #[test]
    fn a_pill_radius_rounds_to_half_the_short_side() {
        let mut s = surface();
        s.clear(color::VOID_900);
        // A 19px-tall legend chip: the pill ends are half-circles.
        s.fill_rect(
            Box2::new(20.0, 20.0, 60.0, 20.0),
            radius::PILL,
            color::EMBER,
        );
        assert_eq!(pixel(&s, 20, 20), [0x2B, 0x2A, 0x2A, 0xFF]);
        assert_eq!(pixel(&s, 50, 30), [0xFE, 0xB0, 0x5D, 0xFF]);
    }

    #[test]
    fn degenerate_rects_draw_nothing_instead_of_panicking() {
        let mut s = surface();
        s.clear(color::VOID_900);
        s.fill_rect(Box2::new(10.0, 10.0, 0.0, 40.0), 0.0, color::EMBER);
        s.fill_rect(Box2::new(10.0, 10.0, 40.0, -5.0), 0.0, color::EMBER);
        s.stroke_rect(Box2::new(10.0, 10.0, 1.0, 1.0), 0.0, 4.0, color::EMBER);
        assert_eq!(pixel(&s, 20, 20), [0x2B, 0x2A, 0x2A, 0xFF]);
    }

    #[test]
    fn drawn_text_marks_the_surface_and_reports_its_width() {
        let mut s = surface();
        s.clear(color::VOID_900);
        let style = TextStyle::new(Family::Body, Weight::Regular, 13.0, color::INK);
        let width = s.draw_text(style, 20.0, 40.0, Align::Left, "Your library");
        assert!(width > 0.0);

        let inked = (0..640)
            .flat_map(|x| (0..60).map(move |y| (x, y)))
            .filter(|(x, y)| pixel(&s, *x, *y) != [0x2B, 0x2A, 0x2A, 0xFF])
            .count();
        assert!(inked > 20, "text drew {inked} pixels");
    }

    #[test]
    fn text_alignment_places_the_run_around_its_anchor() {
        let mut s = surface();
        let style = TextStyle::new(Family::Mono, Weight::Regular, 11.0, color::INK);
        let width = s.measure_text(style, "12 carts");

        let ink_columns = |s: &Surface| {
            let mut min = 640u32;
            let mut max = 0u32;
            for x in 0..640 {
                for y in 0..40 {
                    if pixel(s, x, y)[3] != 0 {
                        min = min.min(x);
                        max = max.max(x);
                    }
                }
            }
            (min, max)
        };

        s.clear(Color::rgb(0x000000).with_alpha(0.0));
        s.draw_text(style, 300.0, 20.0, Align::Right, "12 carts");
        let (min, max) = ink_columns(&s);
        assert!(max <= 300, "right-aligned run crosses its anchor at {max}");
        assert!(
            (min as f32) > 300.0 - width - 2.0,
            "right-aligned run starts too early at {min}"
        );

        s.clear(Color::rgb(0x000000).with_alpha(0.0));
        s.draw_text(style, 300.0, 20.0, Align::Center, "12 carts");
        let (min, max) = ink_columns(&s);
        let center = (min + max) as f32 / 2.0;
        assert!(
            (center - 300.0).abs() < 3.0,
            "centered run sits at {center}"
        );
    }

    /// A glyph landing off the edge must clip, not panic and not wrap onto
    /// the next row.
    #[test]
    fn text_drawn_off_the_edges_clips() {
        let mut s = surface();
        s.clear(color::VOID_900);
        let style = TextStyle::new(Family::Display, Weight::Bold, 72.0, color::INK);
        s.draw_text(style, -200.0, 30.0, Align::Left, "Caiven");
        s.draw_text(style, 600.0, 470.0, Align::Left, "Caiven");
        s.draw_text(style, 20.0, -50.0, Align::Left, "Caiven");
        s.draw_text(style, 20.0, 900.0, Align::Left, "Caiven");
    }

    #[test]
    fn icons_draw_without_error_at_the_sizes_the_chrome_uses() {
        let mut s = surface();
        s.clear(color::VOID_900);
        for icon in [Icon::Wifi, Icon::Volume, Icon::Battery, Icon::Search] {
            s.draw_icon(icon, 10.0, 10.0, 13.0, 2.0, color::INK_DIM)
                .unwrap_or_else(|e| panic!("{icon:?}: {e}"));
        }
        s.draw_icon(Icon::Cartridge, 10.0, 100.0, 30.0, 1.6, color::INK_FAINT)
            .expect("cartridge glyph");
    }

    #[test]
    fn the_dirty_flag_starts_set_and_clears_on_demand() {
        let mut s = surface();
        assert!(s.is_dirty(), "a surface with nothing drawn is dirty");
        s.mark_clean();
        assert!(!s.is_dirty());
        s.mark_dirty();
        assert!(s.is_dirty());
    }

    #[test]
    fn resizing_reallocates_switches_layout_and_redirties() {
        let mut s = surface();
        s.mark_clean();
        s.resize(640, 480).expect("same size");
        assert!(!s.is_dirty(), "resizing to the current size is a no-op");

        s.resize(1280, 720).expect("grow");
        assert!(s.is_dirty());
        assert_eq!((s.width(), s.height()), (1280, 720));
        assert_eq!(*s.metrics(), METRICS_1280);
        assert_eq!(s.rgba().len(), 1280 * 720 * 4);
    }

    #[test]
    fn a_console_framebuffer_composites_at_nearest_neighbour() {
        let mut s = surface();
        s.clear(color::VOID_900);

        // Stand in for a 2×2 cart screenshot.
        let mut cart = Pixmap::new(2, 2).expect("2×2 pixmap");
        cart.fill(tiny_skia::Color::from_rgba8(0xFE, 0xB0, 0x5D, 0xFF));
        s.draw_pixmap(4, 4, cart.as_ref(), 1.0);

        assert_eq!(pixel(&s, 5, 5), [0xFE, 0xB0, 0x5D, 0xFF]);
        assert_eq!(pixel(&s, 3, 3), [0x2B, 0x2A, 0x2A, 0xFF]);
    }
}
