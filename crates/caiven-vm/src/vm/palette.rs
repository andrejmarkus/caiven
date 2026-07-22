use caiven_core::Color;

/// Default 16-color palette: dark shades in the low half, bright in the
/// high half, with the common fantasy-console slot conventions
/// (0 = black, 7 = white, 8 = red).
pub const DEFAULT_COLORS: [(u8, u8, u8); 16] = [
    (10, 10, 16),    // 0  black
    (32, 51, 123),   // 1  dark blue
    (94, 44, 92),    // 2  dark purple
    (40, 114, 82),   // 3  dark green
    (125, 82, 58),   // 4  brown
    (85, 90, 100),   // 5  dark gray
    (170, 175, 185), // 6  light gray
    (240, 240, 235), // 7  white
    (200, 60, 70),   // 8  red
    (235, 130, 50),  // 9  orange
    (250, 210, 80),  // 10 yellow
    (100, 200, 90),  // 11 green
    (70, 140, 235),  // 12 blue
    (130, 120, 160), // 13 lavender
    (235, 120, 150), // 14 pink
    (245, 195, 150), // 15 peach
];

pub struct Palette {
    colors: Vec<Color>,
}

impl Palette {
    pub fn new(palette_size: usize) -> Self {
        let colors = (0..palette_size)
            .map(|i| match DEFAULT_COLORS.get(i) {
                Some(&(r, g, b)) => Color::new_rgb(r, g, b),
                None => Color::new_rgb(i as u8, i as u8, i as u8),
            })
            .collect();
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
