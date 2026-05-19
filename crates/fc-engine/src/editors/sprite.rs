use fc_core::{Color, Vec2};
use fc_vm::rendering::{font::Font, screen::ScreenLayer, text::draw_text};
use fc_vm::vm::Vm;
use winit::keyboard::KeyCode;

use super::Editor;

const SPRITE_SHEET_BASE: usize = 0x4000;
const SPRITE_SIZE: usize = 8;

// Layout (128x128 screen):
//   [0..63, 0..63]    — zoomed edit canvas (active sprite, 8x8 at 8x zoom)
//   [64..127, 0..63]  — sprite picker (8 wide × 8 tall = 64 sprites shown)
//   [0..127, 64..71]  — palette row (16 colors as 8x8 squares)
//   [0..127, 72..79]  — sprite info label

pub struct SpriteEditor {
    pub active_sprite: usize,
    pub active_color: u8,
    clipboard: Option<[u8; 64]>,
    fill_mode: bool,
}

impl SpriteEditor {
    pub fn new() -> Self {
        SpriteEditor {
            active_sprite: 0,
            active_color: 1,
            clipboard: None,
            fill_mode: false,
        }
    }

    fn flood_fill(vm: &mut Vm, base: usize, px: usize, py: usize, target: u8, fill: u8) {
        if target == fill { return; }
        let mut visited = 0u64;
        let mut stack = vec![(px, py)];
        while let Some((x, y)) = stack.pop() {
            let bit = y * SPRITE_SIZE + x;
            if visited & (1u64 << bit) != 0 { continue; }
            visited |= 1u64 << bit;
            let off = base + bit;
            if vm.peek_memory(off) != target { continue; }
            vm.poke_memory(off, fill);
            if x > 0 { stack.push((x - 1, y)); }
            if x + 1 < SPRITE_SIZE { stack.push((x + 1, y)); }
            if y > 0 { stack.push((x, y - 1)); }
            if y + 1 < SPRITE_SIZE { stack.push((x, y + 1)); }
        }
    }

    fn handle_click_inner(&mut self, x: u32, y: u32, vm: &mut Vm) {
        if y < 64 {
            if x < 64 {
                // Edit canvas — 8x8 sprite at 8x zoom in [0..63, 0..63]
                let px = (x / 8) as usize;
                let py = (y / 8) as usize;
                let base = SPRITE_SHEET_BASE + self.active_sprite * SPRITE_SIZE * SPRITE_SIZE;
                if self.fill_mode {
                    let target = vm.peek_memory(base + py * SPRITE_SIZE + px);
                    Self::flood_fill(vm, base, px, py, target, self.active_color);
                } else {
                    vm.poke_memory(base + py * SPRITE_SIZE + px, self.active_color);
                }
            } else {
                // Sprite picker — [64..127, 0..63], 8 sprites wide × 8 sprites tall
                let col = ((x - 64) / 8) as usize;
                let row = (y / 8) as usize;
                self.active_sprite = row * 8 + col;
            }
        } else if y < 72 {
            // Palette row — 16 colors as 8x8 squares across full width
            let col = (x / 8) as usize;
            if col < 16 {
                self.active_color = col as u8;
            }
        }
    }

    fn render_inner(&self, screen: &mut ScreenLayer, vm: &Vm, font: &Font, cursor: (u32, u32)) {
        let bg = Color::new_rgb(15, 15, 15);
        for y in 0..120u32 {
            for x in 0..128u32 {
                screen.set_pixel(Vec2::new(x, y), bg);
            }
        }

        let palette = vm.get_palette();

        // Edit canvas (active sprite zoomed 8x into [0..63, 0..63])
        for py in 0..SPRITE_SIZE {
            for px in 0..SPRITE_SIZE {
                let offset = SPRITE_SHEET_BASE
                    + self.active_sprite * SPRITE_SIZE * SPRITE_SIZE
                    + py * SPRITE_SIZE
                    + px;
                let color_idx = vm.peek_memory(offset) as usize;
                let color = palette.get(color_idx).copied().unwrap_or(Color::new_rgb(0, 0, 0));
                for dy in 0..8u32 {
                    for dx in 0..8u32 {
                        screen.set_pixel(Vec2::new(px as u32 * 8 + dx, py as u32 * 8 + dy), color);
                    }
                }
            }
        }

        // Grid overlay on edit canvas
        let grid = Color::new_rgb(40, 40, 40);
        for i in 0..=8u32 {
            for t in 0..64u32 {
                screen.set_pixel(Vec2::new(i * 8, t), grid);
                screen.set_pixel(Vec2::new(t, i * 8), grid);
            }
        }

        // Cursor highlight on edit canvas
        let (cx, cy) = cursor;
        if cx < 64 && cy < 64 {
            let cell_x = (cx / 8) * 8;
            let cell_y = (cy / 8) * 8;
            let hi = if self.fill_mode { Color::new_rgb(255, 160, 0) } else { Color::new_rgb(255, 255, 255) };
            for d in 0..8u32 {
                screen.set_pixel(Vec2::new(cell_x + d, cell_y), hi);
                screen.set_pixel(Vec2::new(cell_x + d, cell_y + 7), hi);
                screen.set_pixel(Vec2::new(cell_x, cell_y + d), hi);
                screen.set_pixel(Vec2::new(cell_x + 7, cell_y + d), hi);
            }
        }

        // Sprite picker ([64..127, 0..63] — 8 wide × 8 tall)
        for row in 0..8usize {
            for col in 0..8usize {
                let idx = row * 8 + col;
                let base_x = 64 + col as u32 * 8;
                let base_y = row as u32 * 8;
                for py in 0..SPRITE_SIZE {
                    for px in 0..SPRITE_SIZE {
                        let offset = SPRITE_SHEET_BASE
                            + idx * SPRITE_SIZE * SPRITE_SIZE
                            + py * SPRITE_SIZE
                            + px;
                        let color_idx = vm.peek_memory(offset) as usize;
                        let color = palette.get(color_idx).copied().unwrap_or(Color::new_rgb(0, 0, 0));
                        screen.set_pixel(Vec2::new(base_x + px as u32, base_y + py as u32), color);
                    }
                }
                if idx == self.active_sprite {
                    let sel = Color::new_rgb(255, 255, 0);
                    for d in 0..8u32 {
                        screen.set_pixel(Vec2::new(base_x + d, base_y), sel);
                        screen.set_pixel(Vec2::new(base_x + d, base_y + 7), sel);
                        screen.set_pixel(Vec2::new(base_x, base_y + d), sel);
                        screen.set_pixel(Vec2::new(base_x + 7, base_y + d), sel);
                    }
                }
            }
        }

        // Palette row ([0..127, 64..71])
        for i in 0..16usize {
            let color = palette.get(i).copied().unwrap_or(Color::new_rgb(0, 0, 0));
            for dy in 0..8u32 {
                for dx in 0..8u32 {
                    screen.set_pixel(Vec2::new(i as u32 * 8 + dx, 64 + dy), color);
                }
            }
            if i == self.active_color as usize {
                let sel = Color::new_rgb(255, 255, 255);
                for d in 0..8u32 {
                    screen.set_pixel(Vec2::new(i as u32 * 8 + d, 64), sel);
                    screen.set_pixel(Vec2::new(i as u32 * 8 + d, 71), sel);
                    screen.set_pixel(Vec2::new(i as u32 * 8, 64 + d), sel);
                    screen.set_pixel(Vec2::new(i as u32 * 8 + 7, 64 + d), sel);
                }
            }
        }

        // Info label
        let fill_str = if self.fill_mode { " [F]=FILL*" } else { " [F]=FILL" };
        let clip_str = if self.clipboard.is_some() { " [V]=PASTE" } else { "" };
        let label = format!("SPR:{} COL:{}{}{}", self.active_sprite, self.active_color, fill_str, clip_str);
        let label_col = if self.fill_mode { Color::new_rgb(255, 200, 80) } else { Color::new_rgb(200, 200, 200) };
        draw_text(font, screen, &label, Vec2::new(0, 72), label_col);
    }
}

impl Editor for SpriteEditor {
    fn render(&self, layer: &mut ScreenLayer, vm: &Vm, font: &Font, cursor: (u32, u32)) {
        self.render_inner(layer, vm, font, cursor);
    }

    fn handle_click(&mut self, x: u32, y: u32, vm: &mut Vm) {
        self.handle_click_inner(x, y, vm);
    }

    fn handle_key(&mut self, key: KeyCode, vm: &mut Vm) {
        const SPRITE_BYTES: usize = SPRITE_SIZE * SPRITE_SIZE;
        match key {
            KeyCode::KeyC => {
                let base = SPRITE_SHEET_BASE + self.active_sprite * SPRITE_BYTES;
                let mut buf = [0u8; 64];
                for (i, b) in buf.iter_mut().enumerate() {
                    *b = vm.peek_memory(base + i);
                }
                self.clipboard = Some(buf);
            }
            KeyCode::KeyV => {
                if let Some(buf) = self.clipboard {
                    let base = SPRITE_SHEET_BASE + self.active_sprite * SPRITE_BYTES;
                    for (i, b) in buf.iter().enumerate() {
                        vm.poke_memory(base + i, *b);
                    }
                }
            }
            KeyCode::KeyF => {
                self.fill_mode = !self.fill_mode;
            }
            _ => {}
        }
    }
}
