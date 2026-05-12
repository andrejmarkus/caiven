use fc_core::{Color, Vec2};
use fc_vm::rendering::{font::Font, screen::ScreenLayer, text::draw_text};
use fc_vm::vm::Vm;
use winit::keyboard::KeyCode;

use super::Editor;

const MAP_RAM_BASE: usize = 0x5000;
const SPRITE_SHEET_BASE: usize = 0x4000;
const MAP_W: usize = 64;
const MAP_H: usize = 32;
const SPRITE_SIZE: usize = 8;

// Map viewport: x=0..95, y=0..119  → 12 tiles wide × 15 tiles tall at 8px
const VIEW_TILES_W: usize = 12;
const VIEW_TILES_H: usize = 15;
const MAP_AREA_W: u32 = (VIEW_TILES_W * SPRITE_SIZE) as u32; // 96

// Sprite picker: x=96..127, y=0..63 → 4 wide × 8 tall
const PICKER_X: u32 = 96;
const PICKER_COLS: usize = 4;
const PICKER_ROWS: usize = 8;

pub struct MapEditor {
    pub active_sprite: u8,
    view_x: usize,
    view_y: usize,
}

impl MapEditor {
    pub fn new() -> Self {
        MapEditor { active_sprite: 0, view_x: 0, view_y: 0 }
    }

    fn clamp_view(&mut self) {
        self.view_x = self.view_x.min(MAP_W.saturating_sub(VIEW_TILES_W));
        self.view_y = self.view_y.min(MAP_H.saturating_sub(VIEW_TILES_H));
    }

    fn draw_sprite(screen: &mut ScreenLayer, vm: &Vm, sprite_idx: usize, base_x: u32, base_y: u32) {
        let palette = vm.get_palette();
        for py in 0..SPRITE_SIZE {
            for px in 0..SPRITE_SIZE {
                let offset = SPRITE_SHEET_BASE
                    + sprite_idx * SPRITE_SIZE * SPRITE_SIZE
                    + py * SPRITE_SIZE
                    + px;
                let ci = vm.peek_memory(offset) as usize;
                let color = palette.get(ci).copied().unwrap_or(Color::new_rgb(0, 0, 0));
                screen.set_pixel(Vec2::new(base_x + px as u32, base_y + py as u32), color);
            }
        }
    }
}

impl Editor for MapEditor {
    fn render(&self, layer: &mut ScreenLayer, vm: &Vm, font: &Font, _cursor: (u32, u32)) {
        // Map viewport background
        let bg = Color::new_rgb(10, 10, 10);
        for y in 0..120u32 {
            for x in 0..MAP_AREA_W {
                layer.set_pixel(Vec2::new(x, y), bg);
            }
        }

        // Draw visible tiles
        for ty in 0..VIEW_TILES_H {
            let map_ty = self.view_y + ty;
            if map_ty >= MAP_H {
                break;
            }
            for tx in 0..VIEW_TILES_W {
                let map_tx = self.view_x + tx;
                if map_tx >= MAP_W {
                    break;
                }
                let tile_idx = vm.peek_memory(MAP_RAM_BASE + map_ty * MAP_W + map_tx) as usize;
                let bx = tx as u32 * SPRITE_SIZE as u32;
                let by = ty as u32 * SPRITE_SIZE as u32;
                Self::draw_sprite(layer, vm, tile_idx, bx, by);
            }
        }

        // Grid overlay (single-pixel lines)
        let grid = Color::new_rgb(45, 45, 45);
        for i in 0..=VIEW_TILES_W as u32 {
            let x = i * SPRITE_SIZE as u32;
            if x < MAP_AREA_W {
                for y in 0..120u32 {
                    layer.set_pixel(Vec2::new(x, y), grid);
                }
            }
        }
        for i in 0..=VIEW_TILES_H as u32 {
            let y = i * SPRITE_SIZE as u32;
            if y < 120 {
                for x in 0..MAP_AREA_W {
                    layer.set_pixel(Vec2::new(x, y), grid);
                }
            }
        }

        // Sprite picker background
        let picker_bg = Color::new_rgb(20, 20, 20);
        for y in 0..120u32 {
            for x in PICKER_X..128u32 {
                layer.set_pixel(Vec2::new(x, y), picker_bg);
            }
        }

        // Draw sprite picker (4 cols × 8 rows = 32 sprites)
        for row in 0..PICKER_ROWS {
            for col in 0..PICKER_COLS {
                let idx = row * PICKER_COLS + col;
                let bx = PICKER_X + col as u32 * SPRITE_SIZE as u32;
                let by = row as u32 * SPRITE_SIZE as u32;
                Self::draw_sprite(layer, vm, idx, bx, by);
                if idx == self.active_sprite as usize {
                    let sel = Color::new_rgb(255, 255, 0);
                    for d in 0..SPRITE_SIZE as u32 {
                        layer.set_pixel(Vec2::new(bx + d, by), sel);
                        layer.set_pixel(Vec2::new(bx + d, by + SPRITE_SIZE as u32 - 1), sel);
                        layer.set_pixel(Vec2::new(bx, by + d), sel);
                        layer.set_pixel(Vec2::new(bx + SPRITE_SIZE as u32 - 1, by + d), sel);
                    }
                }
            }
        }

        // Info text: sprite index + scroll position
        let label = format!("S:{}", self.active_sprite);
        draw_text(font, layer, &label, Vec2::new(PICKER_X, 64), Color::new_rgb(200, 200, 200));
        let scroll = format!("{},{}", self.view_x, self.view_y);
        draw_text(font, layer, &scroll, Vec2::new(PICKER_X, 72), Color::new_rgb(120, 120, 120));
    }

    fn handle_click(&mut self, x: u32, y: u32, vm: &mut Vm) {
        if y >= 120 {
            return;
        }
        if x < MAP_AREA_W {
            // Map viewport: paint tile
            let tx = self.view_x + (x / SPRITE_SIZE as u32) as usize;
            let ty = self.view_y + (y / SPRITE_SIZE as u32) as usize;
            if tx < MAP_W && ty < MAP_H {
                vm.poke_memory(MAP_RAM_BASE + ty * MAP_W + tx, self.active_sprite);
            }
        } else if x >= PICKER_X && y < (PICKER_ROWS * SPRITE_SIZE) as u32 {
            // Sprite picker: select active sprite
            let col = ((x - PICKER_X) / SPRITE_SIZE as u32) as usize;
            let row = (y / SPRITE_SIZE as u32) as usize;
            let idx = row * PICKER_COLS + col;
            if idx < 64 {
                self.active_sprite = idx as u8;
            }
        }
    }

    fn handle_key(&mut self, key: KeyCode, _vm: &mut Vm) {
        match key {
            KeyCode::ArrowLeft => {
                self.view_x = self.view_x.saturating_sub(1);
                self.clamp_view();
            }
            KeyCode::ArrowRight => {
                self.view_x += 1;
                self.clamp_view();
            }
            KeyCode::ArrowUp => {
                self.view_y = self.view_y.saturating_sub(1);
                self.clamp_view();
            }
            KeyCode::ArrowDown => {
                self.view_y += 1;
                self.clamp_view();
            }
            KeyCode::BracketLeft => {
                if self.active_sprite > 0 {
                    self.active_sprite -= 1;
                }
            }
            KeyCode::BracketRight => {
                if self.active_sprite < 63 {
                    self.active_sprite += 1;
                }
            }
            _ => {}
        }
    }
}
