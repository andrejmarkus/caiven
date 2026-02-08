use crate::settings::PALETTE_SIZE;
use crate::utils::Color;

pub struct Palette {
    pub colors: [Color; PALETTE_SIZE],
}

impl Palette {
    pub fn new() -> Self {
        let mut colors = [Color::new_rgb(0, 0, 0); PALETTE_SIZE];
        for i in 0..PALETTE_SIZE {
            colors[i] = Color::new_rgb(i as u8, i as u8, i as u8);
        }
        Self { colors }
    }

    pub fn get_colors(&self) -> &[Color] {
        &self.colors
    }

    pub fn set_colors(&mut self, colors: [Color; PALETTE_SIZE]) {
        self.colors = colors;
    }

    pub fn get_color(&self, index: usize) -> Color {
        if index < self.colors.len() {
            self.colors[index]
        } else {
            Color::new_rgb(0, 0, 0)
        }
    }

    pub fn set_color(&mut self, index: usize, color: Color) {
        if index < self.colors.len() {
            self.colors[index] = color;
        }
    }
}
