use crate::rendering::{font::Font, screen::ScreenLayer};
use caiven_core::{Color, Vec2};

pub fn draw_character(
    font: &Font,
    layer: &mut ScreenLayer,
    ch: char,
    position: Vec2,
    color: Color,
) {
    draw_character_at(
        font,
        layer,
        ch,
        position.get_x() as i64,
        position.get_y() as i64,
        color,
    );
}

/// Signed-position glyph draw: pixels off any edge are clipped one by one,
/// so text straddling the screen edge still shows its visible part.
pub fn draw_character_at(
    font: &Font,
    layer: &mut ScreenLayer,
    ch: char,
    x: i64,
    y: i64,
    color: Color,
) {
    let glyph = font
        .get_glyph(ch)
        .or_else(|| font.get_glyph(ch.to_ascii_uppercase()));
    let Some(glyph) = glyph else {
        return;
    };
    for j in 0..font.get_height() {
        for i in 0..font.get_width() {
            if !glyph.pixels[j * font.get_width() + i] {
                continue;
            }
            let (px, py) = (x + i as i64, y + j as i64);
            if let (Ok(px), Ok(py)) = (u32::try_from(px), u32::try_from(py)) {
                layer.set_pixel(Vec2::new(px, py), color);
            }
        }
    }
}

pub fn draw_text(font: &Font, layer: &mut ScreenLayer, text: &str, position: Vec2, color: Color) {
    draw_text_at(
        font,
        layer,
        text,
        position.get_x() as i64,
        position.get_y() as i64,
        color,
    );
}

pub fn draw_text_at(
    font: &Font,
    layer: &mut ScreenLayer,
    text: &str,
    x: i64,
    y: i64,
    color: Color,
) {
    let advance = font.get_width() as i64 + 1;
    for (i, ch) in text.chars().enumerate() {
        draw_character_at(
            font,
            layer,
            ch,
            x.saturating_add(i as i64 * advance),
            y,
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
    fn text_starting_off_the_left_edge_is_clipped_not_dropped() {
        let font = font_with_a_only();
        let mut layer = ScreenLayer::new(16, 16);
        // First glyph keeps only its rightmost column (5 px); the second, at x = 2, is whole.
        draw_text_at(&font, &mut layer, "AA", -2, 0, white());
        assert_eq!(drawn_pixel_count(&layer), 5 + 15);
    }

    #[test]
    fn unmapped_char_draws_nothing() {
        let font = font_with_a_only();
        let mut layer = ScreenLayer::new(16, 16);
        draw_character(&font, &mut layer, '~', Vec2::new(0, 0), white());
        assert_eq!(drawn_pixel_count(&layer), 0);
    }
}
