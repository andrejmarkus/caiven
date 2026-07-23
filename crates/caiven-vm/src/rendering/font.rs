use anyhow::{Context, Result};
use std::collections::HashMap;

pub struct Glyph {
    pub pixels: Vec<bool>,
}

pub struct Font {
    glyphs: HashMap<char, Glyph>,
    width: usize,
    height: usize,
}

impl Font {
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
    /// bypassing `from_image`'s disk read (asset paths are relative to the
    /// workspace root, not each crate's test working directory).
    #[cfg(test)]
    pub(crate) fn from_glyphs(glyphs: HashMap<char, Glyph>, width: usize, height: usize) -> Self {
        Self {
            glyphs,
            width,
            height,
        }
    }

    pub fn from_image(
        path: &str,
        chars: &str,
        glyph_width: usize,
        glyph_height: usize,
    ) -> Result<Self> {
        let img = image::open(path)
            .with_context(|| format!("failed to open font image at {path}"))?
            .to_rgba8();
        Ok(Self::from_rgba8(&img, chars, glyph_width, glyph_height))
    }

    /// Decodes a font sheet from an in-memory image (e.g. `include_bytes!`),
    /// for hosts without filesystem access such as the web player.
    pub fn from_bytes(
        bytes: &[u8],
        chars: &str,
        glyph_width: usize,
        glyph_height: usize,
    ) -> Result<Self> {
        let img = image::load_from_memory(bytes)
            .context("failed to decode font image from memory")?
            .to_rgba8();
        Ok(Self::from_rgba8(&img, chars, glyph_width, glyph_height))
    }

    fn from_rgba8(
        img: &image::RgbaImage,
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
                    let pixel = img.get_pixel((x0 + x) as u32, y as u32);
                    pixels.push(pixel[3] > 0);
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
