//! The shell's icon set.
//!
//! Lucide (ISC), stroked, no fill — a documented substitution in the Caiven
//! design system. Only the handful of glyphs the shell actually draws are
//! carried: the status bar's Wi-Fi, volume and battery, the Port screen's
//! search, and the empty state's cartridge stand-in.
//!
//! Each icon is its upstream `iconNode` data verbatim, so re-syncing with a
//! newer Lucide is a copy-paste rather than a redraw. Everything is on a
//! 24×24 grid and is scaled to the requested pixel size at build time.

use anyhow::{Result, anyhow};
use svgtypes::{PathParser, PathSegment};
use tiny_skia::{Path, PathBuilder, Transform};

/// The grid every Lucide glyph is drawn on.
const VIEW_BOX: f32 = 24.0;

/// 4/3 · tan(π/8) — how far a cubic's control points sit from its ends when
/// the curve stands in for a quarter circle. Shared with the raster
/// surface, which rounds rectangle corners the same way.
pub const ARC_K: f32 = 0.552_285;

/// One drawing primitive from a Lucide `iconNode` entry.
enum Shape {
    /// An SVG path `d` string.
    Path(&'static str),
    /// `x1, y1, x2, y2`.
    Line(f32, f32, f32, f32),
    /// `cx, cy, r`.
    Circle(f32, f32, f32),
    /// `x, y, width, height, rx`.
    Rect(f32, f32, f32, f32, f32),
}

/// The icons the shell draws. Adding one means adding its `iconNode` from
/// `@lucide/svelte` and nothing else.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Icon {
    /// Status bar, when the device has a link.
    Wifi,
    /// Status bar, at any non-zero volume.
    Volume,
    /// Status bar, when muted.
    VolumeMuted,
    /// Status bar. The charge level is a separate ember-filled rect drawn
    /// inside this outline, not part of the glyph.
    Battery,
    /// Port screen's search field.
    Search,
    /// Empty state. Lucide has no cartridge; `gamepad-2` is the stand-in
    /// the handoff's "cartridge-ish glyph" resolves to.
    Cartridge,
}

impl Icon {
    fn shapes(self) -> &'static [Shape] {
        match self {
            Icon::Wifi => &[
                Shape::Path("M12 20h.01"),
                Shape::Path("M2 8.82a15 15 0 0 1 20 0"),
                Shape::Path("M5 12.859a10 10 0 0 1 14 0"),
                Shape::Path("M8.5 16.429a5 5 0 0 1 7 0"),
            ],
            Icon::Volume => &[
                Shape::Path(SPEAKER_BODY),
                Shape::Path("M16 9a5 5 0 0 1 0 6"),
                Shape::Path("M19.364 18.364a9 9 0 0 0 0-12.728"),
            ],
            Icon::VolumeMuted => &[
                Shape::Path(SPEAKER_BODY),
                Shape::Line(22.0, 9.0, 16.0, 15.0),
                Shape::Line(16.0, 9.0, 22.0, 15.0),
            ],
            Icon::Battery => &[
                Shape::Path("M 22 14 L 22 10"),
                Shape::Rect(2.0, 6.0, 16.0, 12.0, 2.0),
            ],
            Icon::Search => &[
                Shape::Path("m21 21-4.34-4.34"),
                Shape::Circle(11.0, 11.0, 8.0),
            ],
            Icon::Cartridge => &[
                Shape::Line(6.0, 11.0, 10.0, 11.0),
                Shape::Line(8.0, 9.0, 8.0, 13.0),
                Shape::Line(15.0, 12.0, 15.01, 12.0),
                Shape::Line(18.0, 10.0, 18.01, 10.0),
                Shape::Path(GAMEPAD_BODY),
            ],
        }
    }

    /// The interior of the battery outline on the 24-grid, which the status
    /// bar fills with ember in proportion to charge. Inset by the stroke so
    /// the fill does not sit on top of the outline.
    pub const BATTERY_INNER: (f32, f32, f32, f32) = (4.0, 8.0, 12.0, 8.0);

    /// Builds the glyph as a stroked path at `size` px, y-down, with its
    /// top-left at the origin. Caller strokes it in `currentColor` at the
    /// design's 2px (1.6px for the empty-state glyph).
    pub fn path(self, size: f32) -> Result<Path> {
        let scale = size / VIEW_BOX;
        let mut builder = PathBuilder::new();

        for shape in self.shapes() {
            match shape {
                Shape::Path(d) => append_svg_path(&mut builder, d)?,
                Shape::Line(x1, y1, x2, y2) => {
                    builder.move_to(*x1, *y1);
                    // Lucide draws a dot as a 0.01-long line; a zero-length
                    // segment would vanish, so give it a real extent.
                    if (x1 - x2).abs() < 0.05 && (y1 - y2).abs() < 0.05 {
                        builder.line_to(x1 + 0.05, *y2);
                    } else {
                        builder.line_to(*x2, *y2);
                    }
                }
                Shape::Circle(cx, cy, r) => {
                    builder.push_circle(*cx, *cy, *r);
                }
                Shape::Rect(x, y, w, h, rx) => {
                    push_round_rect(&mut builder, *x, *y, *w, *h, *rx);
                }
            }
        }

        let path = builder
            .finish()
            .ok_or_else(|| anyhow!("icon {self:?} produced an empty path"))?;
        path.transform(Transform::from_scale(scale, scale))
            .ok_or_else(|| anyhow!("icon {self:?} could not be scaled to {size}px"))
    }
}

/// Shared between `Volume` and `VolumeMuted` — the same speaker body with a
/// different right-hand side.
const SPEAKER_BODY: &str = "M11 4.702a.705.705 0 0 0-1.203-.498L6.413 7.587A1.4 1.4 0 0 1 5.416 8H3a1 1 0 0 0-1 1v6a1 1 0 0 0 1 1h2.416a1.4 1.4 0 0 1 .997.413l3.383 3.384A.705.705 0 0 0 11 19.298z";

const GAMEPAD_BODY: &str = "M17.32 5H6.68a4 4 0 0 0-3.978 3.59c-.006.052-.01.101-.017.152C2.604 9.416 2 14.456 2 16a3 3 0 0 0 3 3c1 0 1.5-.5 2-1l1.414-1.414A2 2 0 0 1 9.828 16h4.344a2 2 0 0 1 1.414.586L17 18c.5.5 1 1 2 1a3 3 0 0 0 3-3c0-1.545-.604-6.584-.685-7.258-.007-.05-.011-.1-.017-.151A4 4 0 0 0 17.32 5z";

/// Parses one SVG `d` string into the builder.
///
/// `PathParser` yields absolute and relative segments and elliptical arcs;
/// arcs are the reason this is a parser rather than a table of points —
/// three of the Wi-Fi strokes are arcs.
fn append_svg_path(builder: &mut PathBuilder, d: &str) -> Result<()> {
    let mut pen = (0.0f32, 0.0f32);
    let mut start = (0.0f32, 0.0f32);
    // Reflection point for smooth curve continuations.
    let mut prev_control: Option<(f32, f32)> = None;

    for segment in PathParser::from(d) {
        let segment = segment.map_err(|e| anyhow!("bad icon path data {d:?}: {e}"))?;
        let is_curve = matches!(
            segment,
            PathSegment::CurveTo { .. }
                | PathSegment::SmoothCurveTo { .. }
                | PathSegment::Quadratic { .. }
                | PathSegment::SmoothQuadratic { .. }
        );

        match segment {
            PathSegment::MoveTo { abs, x, y } => {
                pen = resolve(abs, pen, x as f32, y as f32);
                start = pen;
                builder.move_to(pen.0, pen.1);
            }
            PathSegment::LineTo { abs, x, y } => {
                pen = resolve(abs, pen, x as f32, y as f32);
                builder.line_to(pen.0, pen.1);
            }
            PathSegment::HorizontalLineTo { abs, x } => {
                let x = if abs { x as f32 } else { pen.0 + x as f32 };
                pen = (x, pen.1);
                builder.line_to(pen.0, pen.1);
            }
            PathSegment::VerticalLineTo { abs, y } => {
                let y = if abs { y as f32 } else { pen.1 + y as f32 };
                pen = (pen.0, y);
                builder.line_to(pen.0, pen.1);
            }
            PathSegment::CurveTo {
                abs,
                x1,
                y1,
                x2,
                y2,
                x,
                y,
            } => {
                let c1 = resolve(abs, pen, x1 as f32, y1 as f32);
                let c2 = resolve(abs, pen, x2 as f32, y2 as f32);
                pen = resolve(abs, pen, x as f32, y as f32);
                builder.cubic_to(c1.0, c1.1, c2.0, c2.1, pen.0, pen.1);
                prev_control = Some(c2);
            }
            PathSegment::SmoothCurveTo { abs, x2, y2, x, y } => {
                let c1 = reflect(prev_control, pen);
                let c2 = resolve(abs, pen, x2 as f32, y2 as f32);
                pen = resolve(abs, pen, x as f32, y as f32);
                builder.cubic_to(c1.0, c1.1, c2.0, c2.1, pen.0, pen.1);
                prev_control = Some(c2);
            }
            PathSegment::Quadratic { abs, x1, y1, x, y } => {
                let c = resolve(abs, pen, x1 as f32, y1 as f32);
                pen = resolve(abs, pen, x as f32, y as f32);
                builder.quad_to(c.0, c.1, pen.0, pen.1);
                prev_control = Some(c);
            }
            PathSegment::SmoothQuadratic { abs, x, y } => {
                let c = reflect(prev_control, pen);
                pen = resolve(abs, pen, x as f32, y as f32);
                builder.quad_to(c.0, c.1, pen.0, pen.1);
                prev_control = Some(c);
            }
            PathSegment::EllipticalArc {
                abs,
                rx,
                ry,
                x_axis_rotation,
                large_arc,
                sweep,
                x,
                y,
            } => {
                let end = resolve(abs, pen, x as f32, y as f32);
                append_arc(
                    builder,
                    pen,
                    end,
                    rx as f32,
                    ry as f32,
                    x_axis_rotation as f32,
                    large_arc,
                    sweep,
                );
                pen = end;
            }
            PathSegment::ClosePath { .. } => {
                builder.close();
                pen = start;
            }
        }

        if !is_curve {
            prev_control = None;
        }
    }
    Ok(())
}

fn resolve(abs: bool, pen: (f32, f32), x: f32, y: f32) -> (f32, f32) {
    if abs { (x, y) } else { (pen.0 + x, pen.1 + y) }
}

/// The control point a smooth segment implies: the previous one mirrored
/// through the current pen position.
fn reflect(prev_control: Option<(f32, f32)>, pen: (f32, f32)) -> (f32, f32) {
    match prev_control {
        Some((cx, cy)) => (2.0 * pen.0 - cx, 2.0 * pen.1 - cy),
        None => pen,
    }
}

/// Converts an SVG elliptical arc into cubic segments, per the endpoint →
/// center parameterization in the SVG spec (appendix F.6.5).
#[allow(clippy::too_many_arguments)]
fn append_arc(
    builder: &mut PathBuilder,
    from: (f32, f32),
    to: (f32, f32),
    rx: f32,
    ry: f32,
    rotation_deg: f32,
    large_arc: bool,
    sweep: bool,
) {
    // Degenerate radii: the spec says draw a straight line.
    let (mut rx, mut ry) = (rx.abs(), ry.abs());
    if rx < f32::EPSILON
        || ry < f32::EPSILON
        || (from.0 - to.0).abs() + (from.1 - to.1).abs() == 0.0
    {
        builder.line_to(to.0, to.1);
        return;
    }

    let phi = rotation_deg.to_radians();
    let (sin_phi, cos_phi) = phi.sin_cos();

    let dx2 = (from.0 - to.0) / 2.0;
    let dy2 = (from.1 - to.1) / 2.0;
    let x1p = cos_phi * dx2 + sin_phi * dy2;
    let y1p = -sin_phi * dx2 + cos_phi * dy2;

    // Scale up radii that are too small to span the endpoints.
    let lambda = (x1p * x1p) / (rx * rx) + (y1p * y1p) / (ry * ry);
    if lambda > 1.0 {
        let s = lambda.sqrt();
        rx *= s;
        ry *= s;
    }

    let num = (rx * rx * ry * ry - rx * rx * y1p * y1p - ry * ry * x1p * x1p).max(0.0);
    let den = rx * rx * y1p * y1p + ry * ry * x1p * x1p;
    let coef = if den <= f32::EPSILON {
        0.0
    } else {
        let sign = if large_arc == sweep { -1.0 } else { 1.0 };
        sign * (num / den).sqrt()
    };
    let cxp = coef * rx * y1p / ry;
    let cyp = -coef * ry * x1p / rx;

    let cx = cos_phi * cxp - sin_phi * cyp + (from.0 + to.0) / 2.0;
    let cy = sin_phi * cxp + cos_phi * cyp + (from.1 + to.1) / 2.0;

    let theta = ((y1p - cyp) / ry).atan2((x1p - cxp) / rx);
    let theta_end = ((-y1p - cyp) / ry).atan2((-x1p - cxp) / rx);
    let mut delta = theta_end - theta;
    if !sweep && delta > 0.0 {
        delta -= std::f32::consts::TAU;
    } else if sweep && delta < 0.0 {
        delta += std::f32::consts::TAU;
    }

    // A cubic approximates at most a quarter turn well.
    let segments = (delta.abs() / std::f32::consts::FRAC_PI_2).ceil().max(1.0) as usize;
    let step = delta / segments as f32;
    let alpha = (4.0 / 3.0) * (step / 4.0).tan();

    let mut angle = theta;
    for _ in 0..segments {
        let next = angle + step;
        let (sin_a, cos_a) = angle.sin_cos();
        let (sin_b, cos_b) = next.sin_cos();

        let point = |cos: f32, sin: f32| {
            (
                cx + rx * cos * cos_phi - ry * sin * sin_phi,
                cy + rx * cos * sin_phi + ry * sin * cos_phi,
            )
        };
        let derivative = |cos: f32, sin: f32| {
            (
                -rx * sin * cos_phi - ry * cos * sin_phi,
                -rx * sin * sin_phi + ry * cos * cos_phi,
            )
        };

        let p0 = point(cos_a, sin_a);
        let p1 = point(cos_b, sin_b);
        let d0 = derivative(cos_a, sin_a);
        let d1 = derivative(cos_b, sin_b);

        builder.cubic_to(
            p0.0 + alpha * d0.0,
            p0.1 + alpha * d0.1,
            p1.0 - alpha * d1.0,
            p1.1 - alpha * d1.1,
            p1.0,
            p1.1,
        );
        angle = next;
    }
}

fn push_round_rect(builder: &mut PathBuilder, x: f32, y: f32, w: f32, h: f32, r: f32) {
    let r = r.min(w / 2.0).min(h / 2.0).max(0.0);
    if r <= f32::EPSILON {
        builder.move_to(x, y);
        builder.line_to(x + w, y);
        builder.line_to(x + w, y + h);
        builder.line_to(x, y + h);
        builder.close();
        return;
    }
    let k = r * ARC_K;
    builder.move_to(x + r, y);
    builder.line_to(x + w - r, y);
    builder.cubic_to(x + w - r + k, y, x + w, y + r - k, x + w, y + r);
    builder.line_to(x + w, y + h - r);
    builder.cubic_to(x + w, y + h - r + k, x + w - r + k, y + h, x + w - r, y + h);
    builder.line_to(x + r, y + h);
    builder.cubic_to(x + r - k, y + h, x, y + h - r + k, x, y + h - r);
    builder.line_to(x, y + r);
    builder.cubic_to(x, y + r - k, x + r - k, y, x + r, y);
    builder.close();
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [Icon; 6] = [
        Icon::Wifi,
        Icon::Volume,
        Icon::VolumeMuted,
        Icon::Battery,
        Icon::Search,
        Icon::Cartridge,
    ];

    /// The status bar draws icons at 13px and the empty state at 30px. Both
    /// have to produce a path that lands inside its own box.
    #[test]
    fn every_icon_builds_within_its_box() {
        for icon in ALL {
            for size in [13.0f32, 14.0, 30.0] {
                let path = icon
                    .path(size)
                    .unwrap_or_else(|e| panic!("{icon:?} at {size}px: {e}"));
                let b = path.bounds();
                assert!(
                    b.left() >= -0.5 && b.top() >= -0.5,
                    "{icon:?} at {size}px starts outside its box: {b:?}"
                );
                assert!(
                    b.right() <= size + 0.5 && b.bottom() <= size + 0.5,
                    "{icon:?} at {size}px overflows its box: {b:?}"
                );
            }
        }
    }

    /// A glyph that scales linearly is one that was built from geometry
    /// rather than baked at one size.
    #[test]
    fn icons_scale_linearly() {
        for icon in ALL {
            let small = icon.path(12.0).expect("12px").bounds();
            let large = icon.path(24.0).expect("24px").bounds();
            assert!(
                (large.width() - small.width() * 2.0).abs() < 0.01,
                "{icon:?} width {} vs {}",
                large.width(),
                small.width()
            );
        }
    }

    /// Wi-Fi is nothing but arcs, so it is the arc converter's only real
    /// witness: three concentric bows above a dot.
    #[test]
    fn arc_segments_produce_a_wide_flat_wifi_glyph() {
        let b = Icon::Wifi.path(24.0).expect("wifi").bounds();
        assert!(b.width() > 18.0, "wifi too narrow: {}", b.width());
        // The bows occupy the top, the dot the bottom — the glyph spans
        // most of the grid vertically without filling it.
        assert!(b.height() > 10.0, "wifi too short: {}", b.height());
    }

    #[test]
    fn a_dot_shaped_line_still_has_extent() {
        // Cartridge's two face buttons are Lucide's 0.01-long "dot" lines.
        let b = Icon::Cartridge.path(24.0).expect("cartridge").bounds();
        assert!(b.width() > 0.0 && b.height() > 0.0);
    }

    #[test]
    fn battery_inner_fits_inside_the_battery_outline() {
        let (x, y, w, h) = Icon::BATTERY_INNER;
        let outline = Icon::Battery.path(VIEW_BOX).expect("battery").bounds();
        assert!(x > outline.left() && y > outline.top());
        assert!(x + w < outline.right() && y + h < outline.bottom());
    }

    #[test]
    fn malformed_path_data_is_an_error_not_a_panic() {
        let mut builder = PathBuilder::new();
        assert!(append_svg_path(&mut builder, "M 1 1 Z Q").is_err());
    }
}
