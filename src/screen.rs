use crate::settings::{HEIGHT, WIDTH};

pub struct Screen {
    pub pixels: [u8; (WIDTH * HEIGHT * 4) as usize],
}

impl Screen {
    pub fn new() -> Self {
        Self {
            pixels: [0; (WIDTH * HEIGHT * 4) as usize],
        }
    }

    pub fn clear(&mut self) {
        for pixel in self.pixels.chunks_exact_mut(4) {
            pixel[0] = 0; // R
            pixel[1] = 0; // G
            pixel[2] = 0; // B
            pixel[3] = 255; // A
        }
    }

    pub fn set_pixel(&mut self, x: u32, y: u32, r: u8, g: u8, b: u8) {
        if x >= WIDTH || y >= HEIGHT {
            return;
        }
        let index = ((y * WIDTH + x) * 4) as usize;
        self.pixels[index] = r;
        self.pixels[index + 1] = g;
        self.pixels[index + 2] = b;
        self.pixels[index + 3] = 255; // A
    }
}
