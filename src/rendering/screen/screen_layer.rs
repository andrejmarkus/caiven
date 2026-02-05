use crate::settings::{HEIGHT, WIDTH};

pub struct ScreenLayer {
    pixels: Vec<u8>,
}

impl super::PixelLayer for ScreenLayer {
    fn set_pixel(&mut self, x: u32, y: u32, r: u8, g: u8, b: u8, a: u8) {
        if x >= WIDTH || y >= HEIGHT {
            return;
        }
        let i = (y * WIDTH + x) * 4;
        self.pixels[i as usize..i as usize + 4].copy_from_slice(&[r, g, b, a]);
    }
}

impl ScreenLayer {
    pub fn new() -> Self {
        Self {
            pixels: vec![0; WIDTH as usize * HEIGHT as usize * 4],
        }
    }

    pub fn get_pixels(&self) -> &[u8] {
        &self.pixels
    }

    pub fn set_pixel(&mut self, x: u32, y: u32, r: u8, g: u8, b: u8, a: u8) {
        <Self as super::PixelLayer>::set_pixel(self, x, y, r, g, b, a);
    }

    pub fn clear(&mut self) {
        self.pixels.fill(0);
    }
}
