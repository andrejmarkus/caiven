pub mod screen_layer;

pub use screen_layer::*;

pub trait PixelLayer {
    fn set_pixel(&mut self, x: u32, y: u32, r: u8, g: u8, b: u8, a: u8);
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
