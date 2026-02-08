use crate::{
    rendering::{font::Font, screen::ScreenLayer},
    utils::{Color, Vec2},
};

pub fn draw_character(layer: &mut ScreenLayer, ch: char, position: Vec2, color: Color) {
    let font = Font::get_global();
    if let Some(glyph) = font.glyphs.get(&ch) {
        for j in 0..font.height {
            for i in 0..font.width {
                if glyph.pixels[j * font.width + i] {
                    layer.set_pixel(
                        Vec2::new(position.get_x() + i as u32, position.get_y() + j as u32),
                        Color::new_rgb(color.get_r(), color.get_g(), color.get_b()),
                    );
                }
            }
        }
    }
}

pub fn draw_text(layer: &mut ScreenLayer, text: &str, position: Vec2, color: Color) {
    let font = Font::get_global();
    for (i, ch) in text.chars().enumerate() {
        draw_character(
            layer,
            ch,
            Vec2::new(
                position.get_x() + i as u32 * (font.width as u32 + 1),
                position.get_y(),
            ),
            color,
        );
    }
}
