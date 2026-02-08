use crate::settings::{HEIGHT, WIDTH};

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

    pub fn set_pixel(&mut self, x: u32, y: u32, r: u8, g: u8, b: u8, a: u8) {
        if x >= WIDTH || y >= HEIGHT {
            return;
        }
        let i = (y * WIDTH + x) * 4;
        self.pixels[i as usize..i as usize + 4].copy_from_slice(&[r, g, b, a]);
    }

    pub fn clear(&mut self) {
        self.pixels.fill(0);
    }
}

pub struct Screen {
    world: ScreenLayer,
    ui: ScreenLayer,
    debug: ScreenLayer,
}

impl Screen {
    pub fn new() -> Self {
        Self {
            world: ScreenLayer::new(),
            ui: ScreenLayer::new(),
            debug: ScreenLayer::new(),
        }
    }

    pub fn get_world_layer(&mut self) -> &mut ScreenLayer {
        &mut self.world
    }

    pub fn get_ui_layer(&mut self) -> &mut ScreenLayer {
        &mut self.ui
    }

    pub fn get_debug_layer(&mut self) -> &mut ScreenLayer {
        &mut self.debug
    }

    pub fn construct(&self, out: &mut [u8]) {
        out.fill(0);

        for layer in [&self.world, &self.ui, &self.debug] {
            for i in (0..out.len()).step_by(4) {
                let a = layer.get_pixels()[i + 3];
                if a == 0 {
                    continue;
                }
                out[i..i + 4].copy_from_slice(&layer.get_pixels()[i..i + 4]);
            }
        }
    }
}
