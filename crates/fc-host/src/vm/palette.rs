use fc_core::Color;

pub struct Palette {
    colors: Vec<Color>,
}

impl Palette {
    pub fn new(palette_size: usize) -> Self {
        let mut colors = vec![Color::new_rgb(0, 0, 0); palette_size];
        for i in 0..palette_size {
            colors[i] = Color::new_rgb(i as u8, i as u8, i as u8);
        }
        Self { colors }
    }

    pub fn get_colors(&self) -> &[Color] {
        &self.colors
    }

    pub fn set_colors(&mut self, colors: Vec<Color>) {
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
