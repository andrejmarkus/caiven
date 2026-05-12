use crate::settings::{HEIGHT, WIDTH};
use fc_core::{Color, Vec2};

pub struct ScreenLayer {
    pixels: Vec<u8>,
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

    pub fn set_pixel(&mut self, position: Vec2, color: Color) {
        if position.get_x() >= WIDTH || position.get_y() >= HEIGHT {
            return;
        }
        let i = (position.get_y() * WIDTH + position.get_x()) * 4;
        self.pixels[i as usize..i as usize + 4].copy_from_slice(&[
            color.get_r(),
            color.get_g(),
            color.get_b(),
            color.get_a(),
        ]);
    }

    pub fn clear(&mut self) {
        self.pixels.fill(0);
    }
}

pub struct Screen {
    debug: ScreenLayer,
}

impl Screen {
    pub fn new() -> Self {
        Self {
            debug: ScreenLayer::new(),
        }
    }

    pub fn get_debug_layer(&mut self) -> &mut ScreenLayer {
        &mut self.debug
    }

    pub fn construct(&self, out: &mut [u8], world: &[u8], ui: &[u8]) {
        out.fill(0);
        for layer_pixels in [world, ui, self.debug.get_pixels()] {
            for i in (0..out.len()).step_by(4) {
                let a = layer_pixels[i + 3];
                if a == 0 {
                    continue;
                }
                out[i..i + 4].copy_from_slice(&layer_pixels[i..i + 4]);
            }
        }
    }
}
