use crate::cartridge::Cartridge;
use crate::screen::Screen;
use crate::settings::{HEIGHT, WIDTH};

pub struct BouncingPixel {
    x: i32,
    y: i32,
    vx: i32,
    vy: i32,
}

impl BouncingPixel {
    pub fn new() -> Self {
        Self {
            x: 10,
            y: 10,
            vx: 1,
            vy: 1,
        }
    }
}

impl Cartridge for BouncingPixel {
    fn update(&mut self, screen: &mut Screen) {
        // Clear the screen
        screen.clear();

        // Update position
        self.x += self.vx;
        self.y += self.vy;

        // Bounce off walls
        if self.x <= 0 || self.x >= WIDTH as i32 - 1 {
            self.vx = -self.vx;
        }
        if self.y <= 0 || self.y >= HEIGHT as i32 - 1 {
            self.vy = -self.vy;
        }

        // Draw the pixel
        screen.set_pixel(self.x as u32, self.y as u32, 255, 0, 0);
    }
}
