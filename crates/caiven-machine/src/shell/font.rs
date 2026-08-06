//! The shell's bundled type faces and its glyph raster cache.
//!
//! Every face is compiled into the binary. A handheld has no network and no
//! system font worth depending on, so there is nothing to load at runtime
//! and nothing that can be missing on the device.
//!
//! The faces are static instances, not variable fonts: `fontdue` rasterizes
//! whatever instance the file describes and has no way to set a weight axis,
//! so each weight the design uses is its own subset file. See
//! `assets/fonts/README.md` for how they are produced.

use std::collections::HashMap;

use anyhow::{Context, Result};
use fontdue::{Font, FontSettings};

use crate::shell::theme::{Family, Weight};

macro_rules! face {
    ($name:literal) => {
        include_bytes!(concat!("../../assets/fonts/", $name))
    };
}

/// Every bundled face, keyed by the role that selects it.
const FACES: &[(Family, Weight, &[u8])] = &[
    (Family::Body, Weight::Regular, face!("inter-400.ttf")),
    (Family::Body, Weight::Medium, face!("inter-500.ttf")),
    (Family::Body, Weight::SemiBold, face!("inter-600.ttf")),
    (
        Family::Display,
        Weight::SemiBold,
        face!("space-grotesk-600.ttf"),
    ),
    (
        Family::Display,
        Weight::Bold,
        face!("space-grotesk-700.ttf"),
    ),
    (
        Family::Mono,
        Weight::Regular,
        face!("jetbrains-mono-400.ttf"),
    ),
    (
        Family::Mono,
        Weight::Medium,
        face!("jetbrains-mono-500.ttf"),
    ),
    (Family::Mono, Weight::Bold, face!("jetbrains-mono-700.ttf")),
];

/// A rasterized glyph: an 8-bit coverage mask plus where to put it relative
/// to the pen position.
#[derive(Debug, Clone)]
pub struct Glyph {
    pub width: usize,
    pub height: usize,
    /// Coverage, one byte per pixel, row-major.
    pub coverage: Vec<u8>,
    /// Offset from the pen to the left edge of the mask.
    pub left: i32,
    /// Offset from the baseline to the top edge of the mask, y-down.
    pub top: i32,
    /// How far the pen moves after this glyph, before tracking.
    pub advance: f32,
}

/// Cache key. Size is quantized to 1/4px so that an animated size cannot
/// grow the cache without bound.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct GlyphKey {
    family: Family,
    weight: Weight,
    quarter_px: u32,
    ch: char,
}

/// The bundled faces plus a raster cache in front of them.
///
/// Rasterizing is the expensive half of drawing text on a 1.2GHz Cortex-A7,
/// and the shell redraws the same labels over and over, so every glyph is
/// rasterized once and reused. The cache is never evicted: the shell draws
/// from a fixed set of sizes and a Latin subset, so it converges within the
/// first few screens.
pub struct Fonts {
    faces: HashMap<(Family, Weight), Font>,
    cache: HashMap<GlyphKey, Glyph>,
}

impl Fonts {
    /// Parses every bundled face. Fails only if a vendored file is corrupt,
    /// which is a build-time mistake rather than a runtime condition.
    pub fn load() -> Result<Self> {
        let mut faces = HashMap::with_capacity(FACES.len());
        for (family, weight, bytes) in FACES {
            let font = Font::from_bytes(*bytes, FontSettings::default())
                .map_err(anyhow::Error::msg)
                .with_context(|| format!("failed to parse bundled face {family:?} {weight:?}"))?;
            faces.insert((*family, *weight), font);
        }
        Ok(Self {
            faces,
            cache: HashMap::new(),
        })
    }

    /// The face for a role, falling back within the family so a missing
    /// weight degrades to a readable label rather than to nothing.
    fn face(&self, family: Family, weight: Weight) -> Option<(&Font, Weight)> {
        if let Some(font) = self.faces.get(&(family, weight)) {
            return Some((font, weight));
        }
        // Nearest available weight in the same family.
        self.faces
            .iter()
            .filter(|((f, _), _)| *f == family)
            .min_by_key(|((_, w), _)| (*w as i32 - weight as i32).abs())
            .map(|((_, w), font)| (font, *w))
    }

    /// Rasterizes one glyph, or returns the cached mask.
    pub fn glyph(&mut self, family: Family, weight: Weight, size: f32, ch: char) -> Option<&Glyph> {
        let (_, resolved) = self.face(family, weight)?;
        let key = GlyphKey {
            family,
            weight: resolved,
            quarter_px: (size.max(0.0) * 4.0).round() as u32,
            ch,
        };

        if !self.cache.contains_key(&key) {
            let font = self.faces.get(&(family, resolved))?;
            let quantized = key.quarter_px as f32 / 4.0;
            let (metrics, coverage) = font.rasterize(ch, quantized);
            self.cache.insert(
                key,
                Glyph {
                    width: metrics.width,
                    height: metrics.height,
                    coverage,
                    left: metrics.xmin,
                    // fontdue reports ymin from the baseline upward; the
                    // raster surface is y-down, so the mask's top edge sits
                    // this far above the baseline.
                    top: -(metrics.ymin + metrics.height as i32),
                    advance: metrics.advance_width,
                },
            );
        }
        self.cache.get(&key)
    }

    /// Advance width of a string in px, including letter tracking in em.
    ///
    /// Measures through the same cache the draw path uses, so a measured
    /// width and a drawn width can never disagree.
    pub fn measure(
        &mut self,
        family: Family,
        weight: Weight,
        size: f32,
        tracking: f32,
        text: &str,
    ) -> f32 {
        let mut width = 0.0;
        for ch in text.chars() {
            if let Some(glyph) = self.glyph(family, weight, size, ch) {
                width += glyph.advance;
            }
            width += tracking * size;
        }
        // Tracking applies between glyphs, not after the last one.
        (width - tracking * size).max(0.0)
    }

    /// Line height for a size: the face's own ascent-to-descent plus gap.
    pub fn line_height(&self, family: Family, weight: Weight, size: f32) -> f32 {
        let Some((font, _)) = self.face(family, weight) else {
            return size;
        };
        font.horizontal_line_metrics(size)
            .map(|m| m.new_line_size)
            .unwrap_or(size)
    }

    /// Distance from the top of a line box down to the baseline.
    pub fn ascent(&self, family: Family, weight: Weight, size: f32) -> f32 {
        let Some((font, _)) = self.face(family, weight) else {
            return size;
        };
        font.horizontal_line_metrics(size)
            .map(|m| m.ascent)
            .unwrap_or(size)
    }

    /// Number of glyphs currently cached. Diagnostics only.
    pub fn cached_glyphs(&self) -> usize {
        self.cache.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shell::theme::{METRICS_640, tracking};

    fn fonts() -> Fonts {
        Fonts::load().expect("bundled faces must parse")
    }

    #[test]
    fn every_bundled_face_parses() {
        let f = fonts();
        for (family, weight, _) in FACES {
            assert!(
                f.faces.contains_key(&(*family, *weight)),
                "{family:?} {weight:?} missing"
            );
        }
        assert_eq!(f.faces.len(), FACES.len());
    }

    /// The subset is built for the shell's copy. Anything the design writes
    /// has to be renderable, or it silently drops out of a label.
    #[test]
    fn subset_covers_the_shell_character_set() {
        let mut f = fonts();
        let ascii: String = (0x20u8..0x7F).map(|c| c as char).collect();
        for ch in ascii.chars().chain("·×…’".chars()) {
            for family in [Family::Display, Family::Body, Family::Mono] {
                assert!(
                    f.glyph(family, Weight::Regular, 13.0, ch).is_some(),
                    "{family:?} cannot render {ch:?}"
                );
            }
        }
    }

    /// The legend's ◄ ► chips are drawn in the mono face — Space Grotesk's
    /// subset has no geometric-shape block, which is why the chips are mono
    /// in the design rather than display.
    #[test]
    fn mono_face_covers_the_legend_arrows() {
        let mut f = fonts();
        for ch in ['◄', '►'] {
            let g = f
                .glyph(Family::Mono, Weight::Bold, 10.0, ch)
                .unwrap_or_else(|| panic!("mono cannot render {ch:?}"));
            assert!(g.width > 0 && g.height > 0, "{ch:?} rasterized empty");
        }
    }

    #[test]
    fn rasterizing_the_same_glyph_twice_reuses_the_cache() {
        let mut f = fonts();
        f.glyph(Family::Body, Weight::Regular, 13.0, 'A');
        let after_first = f.cached_glyphs();
        f.glyph(Family::Body, Weight::Regular, 13.0, 'A');
        assert_eq!(f.cached_glyphs(), after_first);

        // A size that quantizes to the same quarter-pixel is the same entry.
        f.glyph(Family::Body, Weight::Regular, 13.01, 'A');
        assert_eq!(f.cached_glyphs(), after_first);

        // A genuinely different size is not.
        f.glyph(Family::Body, Weight::Regular, 20.0, 'A');
        assert_eq!(f.cached_glyphs(), after_first + 1);
    }

    #[test]
    fn missing_weight_falls_back_within_the_family() {
        let mut f = fonts();
        // Display ships 600 and 700 only; asking for 400 must still draw.
        let g = f.glyph(Family::Display, Weight::Regular, 32.0, 'C');
        assert!(g.is_some_and(|g| g.width > 0));
    }

    #[test]
    fn mono_face_is_monospaced() {
        let mut f = fonts();
        let narrow = f.measure(Family::Mono, Weight::Regular, 12.0, 0.0, "i");
        let wide = f.measure(Family::Mono, Weight::Regular, 12.0, 0.0, "W");
        assert!(
            (narrow - wide).abs() < 0.01,
            "mono advances differ: {narrow} vs {wide}"
        );
    }

    #[test]
    fn measure_grows_with_length_and_tracking() {
        let mut f = fonts();
        let one = f.measure(Family::Body, Weight::Regular, 13.0, 0.0, "M");
        let three = f.measure(Family::Body, Weight::Regular, 13.0, 0.0, "MMM");
        assert!(three > one * 2.5, "{three} not ~3× {one}");

        let tracked = f.measure(Family::Body, Weight::Regular, 13.0, tracking::CAPS, "MMM");
        // Two gaps between three glyphs.
        assert!((tracked - three - 2.0 * tracking::CAPS * 13.0).abs() < 0.01);

        // A single glyph gets no trailing tracking.
        let one_tracked = f.measure(Family::Body, Weight::Regular, 13.0, tracking::CAPS, "M");
        assert!((one_tracked - one).abs() < 0.01);

        assert_eq!(f.measure(Family::Body, Weight::Regular, 13.0, 0.0, ""), 0.0);
    }

    /// Whatever the design's px size means, it has to fit the box the
    /// design gives it. Body text at 13px inside a 36px legend bar is the
    /// tightest of these.
    #[test]
    fn line_height_fits_the_chrome_bars() {
        let f = fonts();
        let m = METRICS_640;
        let legend = f.line_height(Family::Body, Weight::Regular, m.text.legend_label);
        assert!(
            legend < m.legend_bar_h as f32,
            "legend text {legend}px exceeds the {}px bar",
            m.legend_bar_h
        );
        let status = f.line_height(Family::Mono, Weight::Regular, m.text.mono_spec);
        assert!(
            status < m.status_bar_h as f32,
            "status text {status}px exceeds the {}px bar",
            m.status_bar_h
        );
    }

    #[test]
    fn ascent_is_within_the_line_box() {
        let f = fonts();
        for family in [Family::Display, Family::Body, Family::Mono] {
            let ascent = f.ascent(family, Weight::Regular, 13.0);
            let line = f.line_height(family, Weight::Regular, 13.0);
            assert!(
                ascent > 0.0 && ascent <= line,
                "{family:?}: {ascent}/{line}"
            );
        }
    }
}
