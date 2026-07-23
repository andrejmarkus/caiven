use crate::rendering::{font::Font, screen::ScreenLayer};
use caiven_core::{Color, Vec2};

pub fn draw_character(
    font: &Font,
    layer: &mut ScreenLayer,
    ch: char,
    position: Vec2,
    color: Color,
) {
    let glyph = font
        .get_glyph(ch)
        .or_else(|| font.get_glyph(ch.to_ascii_uppercase()));
    if let Some(glyph) = glyph {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rendering::font::Glyph;
    use std::collections::HashMap;

    fn font_with_a_only() -> Font {
        let mut glyphs = HashMap::new();
        glyphs.insert(
            'A',
            Glyph {
                pixels: vec![true; 3 * 5],
            },
        );
        Font::from_glyphs(glyphs, 3, 5)
    }

    fn white() -> Color {
        Color::new_rgb(255, 255, 255)
    }

    fn drawn_pixel_count(layer: &ScreenLayer) -> usize {
        layer.get_pixels().chunks(4).filter(|p| p[3] != 0).count()
    }

    #[test]
    fn lowercase_falls_back_to_uppercase_glyph() {
        let font = font_with_a_only();
        let mut layer = ScreenLayer::new(16, 16);
        draw_character(&font, &mut layer, 'a', Vec2::new(0, 0), white());
        // Falls back to 'A's glyph, so the same 15 pixels light up.
        assert_eq!(drawn_pixel_count(&layer), 15);
    }

    #[test]
    fn unmapped_char_draws_nothing() {
        let font = font_with_a_only();
        let mut layer = ScreenLayer::new(16, 16);
        draw_character(&font, &mut layer, '~', Vec2::new(0, 0), white());
        assert_eq!(drawn_pixel_count(&layer), 0);
    }
}
