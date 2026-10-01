use anyhow::{Context, Result};
use std::collections::HashMap;

/// Glyphs available in the built-in font sheet, in sheet order.
pub const FONT_GLYPHS: &str = " 0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ!?\"'()+-=.:,[]<>/%_";

pub struct Glyph {
    pub pixels: Vec<bool>,
}

pub struct Font {
    glyphs: HashMap<char, Glyph>,
    width: usize,
    height: usize,
}

impl Font {
    /// The same embedded game font for desktop, browser and headless rendering.
    pub fn builtin() -> Result<Self> {
        Self::from_bytes(
            include_bytes!("../../../../assets/font.png"),
            FONT_GLYPHS,
            3,
            5,
        )
        .context("failed to initialize embedded font")
    }

    pub fn empty() -> Self {
        Self {
            glyphs: HashMap::new(),
            width: 0,
            height: 0,
        }
    }

    pub fn get_glyph(&self, ch: char) -> Option<&Glyph> {
        self.glyphs.get(&ch)
    }

    pub fn get_width(&self) -> usize {
        self.width
    }

    pub fn get_height(&self) -> usize {
        self.height
    }

    /// Test-only constructor for building a `Font` from in-memory glyphs,
    /// without decoding a font sheet.
    #[cfg(test)]
    pub(crate) fn from_glyphs(glyphs: HashMap<char, Glyph>, width: usize, height: usize) -> Self {
        Self {
            glyphs,
            width,
            height,
        }
    }

    /// Decodes a font sheet from an in-memory image (e.g. `include_bytes!`),
    /// for hosts without filesystem access such as the web player.
    pub fn from_bytes(
        bytes: &[u8],
        chars: &str,
        glyph_width: usize,
        glyph_height: usize,
    ) -> Result<Self> {
        let (width, _, rgba) = caiven_cart::asset_png::png_to_rgba(bytes)
            .map_err(anyhow::Error::msg)
            .context("failed to decode font image from memory")?;
        Ok(Self::from_rgba8(
            &rgba,
            width as usize,
            chars,
            glyph_width,
            glyph_height,
        ))
    }

    fn from_rgba8(
        rgba: &[u8],
        sheet_width: usize,
        chars: &str,
        glyph_width: usize,
        glyph_height: usize,
    ) -> Self {
        let mut glyphs = HashMap::new();
        for (i, ch) in chars.chars().enumerate() {
            let x0 = i * glyph_width;
            let mut pixels = Vec::with_capacity(glyph_width * glyph_height);
            for y in 0..glyph_height {
                for x in 0..glyph_width {
                    let alpha = rgba.get((y * sheet_width + x0 + x) * 4 + 3);
                    pixels.push(alpha.is_some_and(|a| *a > 0));
                }
            }
            glyphs.insert(ch, Glyph { pixels });
        }

        Self {
            glyphs,
            width: glyph_width,
            height: glyph_height,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_listed_glyph_has_ink_in_the_builtin_font() {
        let font = Font::builtin().expect("builtin font");
        for ch in FONT_GLYPHS.chars().filter(|c| *c != ' ') {
            let glyph = font.get_glyph(ch).expect("glyph present");
            assert!(glyph.pixels.iter().any(|p| *p), "{ch:?} is blank");
        }
    }
}
