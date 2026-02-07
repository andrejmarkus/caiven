use crate::rendering::font::Font;
use crate::rendering::screen::PixelLayer;

pub fn draw_character(layer: &mut dyn PixelLayer, ch: char, x: u32, y: u32, color: [u8; 3]) {
    let font = Font::get_global();
    if let Some(glyph) = font.glyphs.get(&ch) {
        for j in 0..font.height {
            for i in 0..font.width {
                if glyph.pixels[j * font.width + i] {
                    layer.set_pixel(
                        x + i as u32,
                        y + j as u32,
                        color[0],
                        color[1],
                        color[2],
                        255,
                    );
                }
            }
        }
    }
}

pub fn draw_text(layer: &mut dyn PixelLayer, text: &str, x: u32, y: u32, color: [u8; 3]) {
    let font = Font::get_global();
    for (i, ch) in text.chars().enumerate() {
        draw_character(layer, ch, x + i as u32 * (font.width as u32 + 1), y, color);
    }
}
