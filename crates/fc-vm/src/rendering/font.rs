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

    pub fn from_image(
        path: &str,
        chars: &str,
        glyph_width: usize,
        glyph_height: usize,
    ) -> Result<Self> {
        let img = image::open(path)
            .with_context(|| format!("failed to open font image at {path}"))?
            .to_rgba8();

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

        Ok(Self {
            glyphs,
            width: glyph_width,
            height: glyph_height,
        })
    }
}
