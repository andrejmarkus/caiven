use std::collections::HashMap;
use std::sync::OnceLock;

pub static GLOBAL_FONT: OnceLock<Font> = OnceLock::new();

pub struct Glyph {
    pub pixels: Vec<bool>,
}

pub struct Font {
    glyphs: HashMap<char, Glyph>,
    width: usize,
    height: usize,
}

impl Font {
    pub fn init_global(path: &str, chars: &str, glyph_width: usize, glyph_height: usize) {
        let font = Self::from_image(path, chars, glyph_width, glyph_height);
        let _ = GLOBAL_FONT.set(font);
    }

    pub fn get_global() -> &'static Font {
        GLOBAL_FONT
            .get()
            .expect("Font must be initialized before use. Call Font::init_global first.")
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

    pub fn from_image(path: &str, chars: &str, glyph_width: usize, glyph_height: usize) -> Self {
        let img = image::open(path)
            .expect("Failed to load font image")
            .to_rgba8();

        let mut glyphs = HashMap::new();
        for (i, ch) in chars.chars().enumerate() {
            let x0 = i * glyph_width;
            let y0 = 0;

            let mut pixels = Vec::with_capacity(glyph_width * glyph_height);
            for y in 0..glyph_height {
                for x in 0..glyph_width {
                    let pixel = img.get_pixel((x0 + x) as u32, (y0 + y) as u32);
                    let alpha = pixel[3];
                    pixels.push(alpha > 0);
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
