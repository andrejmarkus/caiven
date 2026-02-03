use crate::settings::PALETTE_SIZE;

pub struct Palette {
    pub colors: [[u8; 3]; PALETTE_SIZE],
}

impl Palette {
    pub fn new() -> Self {
        let mut colors = [[0; 3]; PALETTE_SIZE];
        for i in 0..PALETTE_SIZE {
            colors[i] = [(i as u8), (i as u8), (i as u8)];
        }
        Self { colors }
    }

    pub fn get_color(&self, index: usize) -> [u8; 3] {
        if index < self.colors.len() {
            self.colors[index]
        } else {
            [0, 0, 0]
        }
    }

    pub fn set_color(&mut self, index: usize, color: [u8; 3]) {
        if index < self.colors.len() {
            self.colors[index] = color;
        }
    }
}
