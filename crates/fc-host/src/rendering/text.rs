use crate::rendering::{font::Font, screen::ScreenLayer};
use fc_core::{Color, Vec2};

pub fn draw_character(font: &Font, layer: &mut ScreenLayer, ch: char, position: Vec2, color: Color) {
    if let Some(glyph) = font.get_glyph(ch) {
        for j in 0..font.get_height() {
            for i in 0..font.get_width() {
                if glyph.pixels[j * font.get_width() + i] {
                    layer.set_pixel(
                        Vec2::new(position.get_x() + i as u32, position.get_y() + j as u32),
                        Color::new_rgb(color.get_r(), color.get_g(), color.get_b()),
                    );
                }
            }
        }
    }
}

pub fn draw_text(font: &Font, layer: &mut ScreenLayer, text: &str, position: Vec2, color: Color) {
    for (i, ch) in text.chars().enumerate() {
        draw_character(
            font,
            layer,
            ch,
            Vec2::new(
                position.get_x() + i as u32 * (font.get_width() as u32 + 1),
                position.get_y(),
            ),
            color,
        );
    }
}
