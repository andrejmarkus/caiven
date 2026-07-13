use fc_core::{Color, Vec2};
use fc_vm::rendering::{font::Font, screen::ScreenLayer, text::draw_text};
use fc_vm::vm::Vm;
use winit::keyboard::KeyCode;

use super::Editor;

const PALETTE_RAM_BASE: usize = 0x5800;
const NUM_COLORS: usize = 16;

// Layout (128×120 usable area above tab bar):
//   y=0..7:   16 color swatches (each 8×8)
//   y=8..39:  active color preview (left 32×32) + RGB sliders (right 96px wide)
//   y=40..55: RGB value labels
//   y=56..63: selected slot label

pub struct PaletteEditor {
    pub active_slot: usize,
    pub active_channel: u8, // 0=R, 1=G, 2=B
}

impl PaletteEditor {
    pub fn new() -> Self {
        PaletteEditor {
            active_slot: 1,
            active_channel: 0,
        }
    }

    fn read_color(vm: &Vm, slot: usize) -> (u8, u8, u8) {
        let base = PALETTE_RAM_BASE + slot * 3;
        (
            vm.peek_memory(base),
            vm.peek_memory(base + 1),
            vm.peek_memory(base + 2),
        )
    }

    fn write_color(vm: &mut Vm, slot: usize, r: u8, g: u8, b: u8) {
        let base = PALETTE_RAM_BASE + slot * 3;
        vm.poke_memory(base, r);
        vm.poke_memory(base + 1, g);
        vm.poke_memory(base + 2, b);
        vm.set_palette_color(slot, Color::new_rgb(r, g, b));
    }

    fn draw_slider(layer: &mut ScreenLayer, y: u32, value: u8, active: bool) {
        let track_x: u32 = 32;
        let track_w: u32 = 96;
        let track_color = Color::new_rgb(40, 40, 40);
        let fill_color = if active {
            Color::new_rgb(220, 220, 220)
        } else {
            Color::new_rgb(100, 100, 100)
        };
        let filled = (value as u32 * track_w) / 255;

        for dy in 0..8u32 {
            for dx in 0..track_w {
                let c = if dx < filled { fill_color } else { track_color };
                layer.set_pixel(Vec2::new(track_x + dx, y + dy), c);
            }
        }
        // Thumb
        if filled < track_w {
            for dy in 0..8u32 {
                layer.set_pixel(
                    Vec2::new(track_x + filled, y + dy),
                    Color::new_rgb(255, 255, 255),
                );
            }
        }
    }
}

impl Editor for PaletteEditor {
    fn render(&self, layer: &mut ScreenLayer, vm: &Vm, font: &Font, _cursor: (u32, u32)) {
        // Clear
        let bg = Color::new_rgb(15, 15, 15);
        for y in 0..120u32 {
            for x in 0..128u32 {
                layer.set_pixel(Vec2::new(x, y), bg);
            }
        }

        // Swatch row (y=0..7)
        for i in 0..NUM_COLORS {
            let (r, g, b) = Self::read_color(vm, i);
            let c = Color::new_rgb(r, g, b);
            for dy in 0..8u32 {
                for dx in 0..8u32 {
                    layer.set_pixel(Vec2::new(i as u32 * 8 + dx, dy), c);
                }
            }
            if i == self.active_slot {
                let sel = Color::new_rgb(255, 255, 255);
                for d in 0..8u32 {
                    layer.set_pixel(Vec2::new(i as u32 * 8 + d, 0), sel);
                    layer.set_pixel(Vec2::new(i as u32 * 8 + d, 7), sel);
                    layer.set_pixel(Vec2::new(i as u32 * 8, d), sel);
                    layer.set_pixel(Vec2::new(i as u32 * 8 + 7, d), sel);
                }
            }
        }

        // Active color preview (x=0..31, y=8..39)
        let (r, g, b) = Self::read_color(vm, self.active_slot);
        let preview = Color::new_rgb(r, g, b);
        for dy in 0..32u32 {
            for dx in 0..32u32 {
                layer.set_pixel(Vec2::new(dx, 8 + dy), preview);
            }
        }

        // RGB sliders (x=32..127, y=8/16/24)
        Self::draw_slider(layer, 8, r, self.active_channel == 0);
        Self::draw_slider(layer, 16, g, self.active_channel == 1);
        Self::draw_slider(layer, 24, b, self.active_channel == 2);
        // Channel labels
        let rl = Color::new_rgb(if self.active_channel == 0 { 255 } else { 140 }, 60, 60);
        let gl = Color::new_rgb(60, if self.active_channel == 1 { 255 } else { 140 }, 60);
        let bl = Color::new_rgb(60, 60, if self.active_channel == 2 { 255 } else { 140 });
        draw_text(font, layer, "R", Vec2::new(33, 9), rl);
        draw_text(font, layer, "G", Vec2::new(33, 17), gl);
        draw_text(font, layer, "B", Vec2::new(33, 25), bl);

        // RGB values text (y=40)
        let vals = format!("R:{:03} G:{:03} B:{:03}", r, g, b);
        draw_text(
            font,
            layer,
            &vals,
            Vec2::new(0, 40),
            Color::new_rgb(200, 200, 200),
        );

        // Slot + hex label (y=48)
        let hex = format!("#{:02X}{:02X}{:02X} SL:{}", r, g, b, self.active_slot);
        draw_text(
            font,
            layer,
            &hex,
            Vec2::new(0, 48),
            Color::new_rgb(140, 140, 140),
        );

        // Key hints (y=56)
        draw_text(
            font,
            layer,
            "+-SLOT []CH ^vVAL",
            Vec2::new(0, 56),
            Color::new_rgb(80, 80, 80),
        );
    }

    fn handle_click(&mut self, x: u32, y: u32, vm: &mut Vm) {
        if y < 8 {
            // Swatch row: select slot
            self.active_slot = (x / 8) as usize;
        } else if y < 40 && x >= 32 {
            // Slider click: set channel value
            let ch = ((y - 8) / 8) as usize;
            if ch < 3 {
                let raw_x = x.saturating_sub(32).min(95);
                let value = ((raw_x * 255) / 95) as u8;
                let (mut r, mut g, mut b) = Self::read_color(vm, self.active_slot);
                match ch {
                    0 => r = value,
                    1 => g = value,
                    _ => b = value,
                }
                self.active_channel = ch as u8;
                Self::write_color(vm, self.active_slot, r, g, b);
            }
        }
    }

    fn handle_key(&mut self, key: KeyCode, vm: &mut Vm) {
        match key {
            KeyCode::Equal | KeyCode::NumpadAdd => {
                if self.active_slot + 1 < NUM_COLORS {
                    self.active_slot += 1;
                }
            }
            KeyCode::Minus | KeyCode::NumpadSubtract => {
                if self.active_slot > 0 {
                    self.active_slot -= 1;
                }
            }
            KeyCode::BracketLeft => {
                if self.active_channel > 0 {
                    self.active_channel -= 1;
                }
            }
            KeyCode::BracketRight => {
                if self.active_channel < 2 {
                    self.active_channel += 1;
                }
            }
            KeyCode::ArrowUp => {
                let (mut r, mut g, mut b) = Self::read_color(vm, self.active_slot);
                match self.active_channel {
                    0 => r = r.saturating_add(1),
                    1 => g = g.saturating_add(1),
                    _ => b = b.saturating_add(1),
                }
                Self::write_color(vm, self.active_slot, r, g, b);
            }
            KeyCode::ArrowDown => {
                let (mut r, mut g, mut b) = Self::read_color(vm, self.active_slot);
                match self.active_channel {
                    0 => r = r.saturating_sub(1),
                    1 => g = g.saturating_sub(1),
                    _ => b = b.saturating_sub(1),
                }
                Self::write_color(vm, self.active_slot, r, g, b);
            }
            _ => {}
        }
    }
}
