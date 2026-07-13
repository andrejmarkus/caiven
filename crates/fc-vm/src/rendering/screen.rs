use fc_core::memory::RGBA_BYTES;
use fc_core::{Color, Vec2};

pub struct ScreenLayer {
    pixels: Vec<u8>,
    width: u32,
    height: u32,
}

impl ScreenLayer {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            pixels: vec![0; width as usize * height as usize * RGBA_BYTES],
            width,
            height,
        }
    }

    pub fn get_pixels(&self) -> &[u8] {
        &self.pixels
    }

    pub fn set_pixel(&mut self, position: Vec2, color: Color) {
        if position.get_x() >= self.width || position.get_y() >= self.height {
            return;
        }
        let i = (position.get_y() * self.width + position.get_x()) as usize * RGBA_BYTES;
        self.pixels[i..i + RGBA_BYTES].copy_from_slice(&[
            color.get_r(),
            color.get_g(),
            color.get_b(),
            color.get_a(),
        ]);
    }

    pub fn set_pixels(&mut self, data: Vec<u8>) {
        self.pixels = data;
    }

    pub fn clear(&mut self) {
        self.pixels.fill(0);
    }
}

pub struct Screen {
    debug: ScreenLayer,
}

impl Screen {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            debug: ScreenLayer::new(width, height),
        }
    }

    pub fn get_debug_layer(&mut self) -> &mut ScreenLayer {
        &mut self.debug
    }

    pub fn construct(&self, out: &mut [u8], world: &[u8], ui: &[u8]) {
        out.fill(0);
        for layer_pixels in [world, ui, self.debug.get_pixels()] {
            for i in (0..out.len()).step_by(RGBA_BYTES) {
                let a = layer_pixels[i + 3];
                if a == 0 {
                    continue;
                }
                out[i..i + RGBA_BYTES].copy_from_slice(&layer_pixels[i..i + RGBA_BYTES]);
            }
        }
    }
}
