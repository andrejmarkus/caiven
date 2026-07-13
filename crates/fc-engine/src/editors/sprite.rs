use fc_core::memory::SPRITE_SHEET_RAM_BASE as SPRITE_SHEET_BASE;
use fc_core::{Color, Vec2};
use fc_vm::rendering::{font::Font, screen::ScreenLayer, text::draw_text};
use fc_vm::vm::Vm;
use winit::keyboard::KeyCode;

use super::util::{Grid, clear_panel, fill_rect, rect_border, theme};
use super::{Editor, button_hit, draw_button};

const SPRITE_SIZE: usize = 8;

/// Zoomed edit canvas: 8x8 sprite pixels at 8x zoom in [0..63, 0..63].
const CANVAS_GRID: Grid = Grid::new(0, 0, 8, 8, 8, 8);
/// Sprite picker: 8x8 sprites in [64..127, 0..63].
const PICKER_GRID: Grid = Grid::new(64, 0, 8, 8, 8, 8);
/// Palette row: 16 colors as 8x8 squares in [0..127, 64..71].
const PALETTE_GRID: Grid = Grid::new(0, 64, 8, 8, 16, 1);

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
        if target == fill {
            return;
        }
        let mut visited = 0u64;
        let mut stack = vec![(px, py)];
        while let Some((x, y)) = stack.pop() {
            let bit = y * SPRITE_SIZE + x;
            if visited & (1u64 << bit) != 0 {
                continue;
            }
            visited |= 1u64 << bit;
            let off = base + bit;
            if vm.peek_memory(off) != target {
                continue;
            }
            vm.poke_memory(off, fill);
            if x > 0 {
                stack.push((x - 1, y));
            }
            if x + 1 < SPRITE_SIZE {
                stack.push((x + 1, y));
            }
            if y > 0 {
                stack.push((x, y - 1));
            }
            if y + 1 < SPRITE_SIZE {
                stack.push((x, y + 1));
            }
        }
    }

    fn handle_click_inner(&mut self, x: u32, y: u32, vm: &mut Vm) {
        if let Some((px, py)) = CANVAS_GRID.cell_at(x, y) {
            let base = SPRITE_SHEET_BASE + self.active_sprite * SPRITE_SIZE * SPRITE_SIZE;
            if self.fill_mode {
                let target = vm.peek_memory(base + py * SPRITE_SIZE + px);
                Self::flood_fill(vm, base, px, py, target, self.active_color);
            } else {
                vm.poke_memory(base + py * SPRITE_SIZE + px, self.active_color);
            }
        } else if let Some((col, row)) = PICKER_GRID.cell_at(x, y) {
            let picker_base = (self.active_sprite / 64) * 64;
            self.active_sprite = picker_base + row * 8 + col;
        } else if let Some((col, _)) = PALETTE_GRID.cell_at(x, y) {
            self.active_color = col as u8;
        }
    }

    fn render_inner(&self, screen: &mut ScreenLayer, vm: &Vm, font: &Font, cursor: (u32, u32)) {
        clear_panel(screen, theme::BG);

        let palette = vm.get_palette();

        // Edit canvas (active sprite zoomed 8x into [0..63, 0..63])
        for py in 0..SPRITE_SIZE {
            for px in 0..SPRITE_SIZE {
                let offset = SPRITE_SHEET_BASE
                    + self.active_sprite * SPRITE_SIZE * SPRITE_SIZE
                    + py * SPRITE_SIZE
                    + px;
                let color_idx = vm.peek_memory(offset) as usize;
                let color = palette
                    .get(color_idx)
                    .copied()
                    .unwrap_or(Color::new_rgb(0, 0, 0));
                fill_rect(screen, px as u32 * 8, py as u32 * 8, 8, 8, color);
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
            let hi = if self.fill_mode {
                Color::new_rgb(255, 160, 0)
            } else {
                Color::new_rgb(255, 255, 255)
            };
            rect_border(screen, cell_x, cell_y, 8, 8, hi);
        }

        // Sprite picker ([64..127, 0..63] — 8 wide × 8 tall), paged by active_sprite
        let picker_base = (self.active_sprite / 64) * 64;
        for row in 0..8usize {
            for col in 0..8usize {
                let idx = picker_base + row * 8 + col;
                let base_x = 64 + col as u32 * 8;
                let base_y = row as u32 * 8;
                for py in 0..SPRITE_SIZE {
                    for px in 0..SPRITE_SIZE {
                        let offset = SPRITE_SHEET_BASE
                            + idx * SPRITE_SIZE * SPRITE_SIZE
                            + py * SPRITE_SIZE
                            + px;
                        let color_idx = vm.peek_memory(offset) as usize;
                        let color = palette
                            .get(color_idx)
                            .copied()
                            .unwrap_or(Color::new_rgb(0, 0, 0));
                        screen.set_pixel(Vec2::new(base_x + px as u32, base_y + py as u32), color);
                    }
                }
                if idx == self.active_sprite {
                    rect_border(screen, base_x, base_y, 8, 8, Color::new_rgb(255, 255, 0));
                }
            }
        }

        // Palette row ([0..127, 64..71])
        for i in 0..16usize {
            let color = palette.get(i).copied().unwrap_or(Color::new_rgb(0, 0, 0));
            fill_rect(screen, i as u32 * 8, 64, 8, 8, color);
            if i == self.active_color as usize {
                rect_border(
                    screen,
                    i as u32 * 8,
                    64,
                    8,
                    8,
                    Color::new_rgb(255, 255, 255),
                );
            }
        }

        // Info label — show page (0-3 of 4 picker pages)
        let page = self.active_sprite / 64;
        let label = format!(
            "SPR:{} COL:{} P{}/4",
            self.active_sprite, self.active_color, page
        );
        draw_text(
            font,
            screen,
            &label,
            Vec2::new(0, 72),
            Color::new_rgb(200, 200, 200),
        );

        // Button row at y=80
        draw_button(screen, font, 0, 80, "FILL", self.fill_mode);
        draw_button(screen, font, 22, 80, "COPY", false);
        if self.clipboard.is_some() {
            draw_button(screen, font, 44, 80, "PST", false);
        }
        // Scroll hint
        let hint = "WHEEL=SPR RCLICK=ERASE".to_string();
        draw_text(
            font,
            screen,
            &hint,
            Vec2::new(0, 89),
            Color::new_rgb(70, 70, 70),
        );
    }
}

impl Editor for SpriteEditor {
    fn render(&self, layer: &mut ScreenLayer, vm: &Vm, font: &Font, cursor: (u32, u32)) {
        self.render_inner(layer, vm, font, cursor);
    }

    fn handle_click(&mut self, x: u32, y: u32, vm: &mut Vm) {
        // Button row at y=80..86 — don't pass through to canvas
        if (80..87).contains(&y) {
            if button_hit(0, 80, "FILL", x, y) {
                self.fill_mode = !self.fill_mode;
            } else if button_hit(22, 80, "COPY", x, y) {
                let base = SPRITE_SHEET_BASE + self.active_sprite * SPRITE_SIZE * SPRITE_SIZE;
                let mut buf = [0u8; 64];
                for (i, b) in buf.iter_mut().enumerate() {
                    *b = vm.peek_memory(base + i);
                }
                self.clipboard = Some(buf);
            } else if button_hit(44, 80, "PST", x, y)
                && let Some(buf) = self.clipboard
            {
                let base = SPRITE_SHEET_BASE + self.active_sprite * SPRITE_SIZE * SPRITE_SIZE;
                for (i, b) in buf.iter().enumerate() {
                    vm.poke_memory(base + i, *b);
                }
            }
            return;
        }
        self.handle_click_inner(x, y, vm);
    }

    fn handle_drag(&mut self, x: u32, y: u32, vm: &mut Vm) {
        // Skip button row during drag
        if y < 80 {
            self.handle_click_inner(x, y, vm);
        }
    }

    fn handle_right_click(&mut self, x: u32, y: u32, vm: &mut Vm) {
        // Erase: paint color 0 on canvas
        if let Some((px, py)) = CANVAS_GRID.cell_at(x, y) {
            let base = SPRITE_SHEET_BASE + self.active_sprite * SPRITE_SIZE * SPRITE_SIZE;
            vm.poke_memory(base + py * SPRITE_SIZE + px, 0);
        }
    }

    fn handle_scroll(&mut self, _dx: f32, dy: f32, _vm: &mut Vm) {
        if dy < 0.0 && self.active_sprite < 255 {
            self.active_sprite += 1;
        } else if dy > 0.0 && self.active_sprite > 0 {
            self.active_sprite -= 1;
        }
    }

    fn handle_key(&mut self, key: KeyCode, vm: &mut Vm) {
        const SPRITE_BYTES: usize = SPRITE_SIZE * SPRITE_SIZE;
        match key {
            KeyCode::PageUp => {
                let page = self.active_sprite / 64;
                if page > 0 {
                    self.active_sprite = (page - 1) * 64;
                }
            }
            KeyCode::PageDown => {
                let page = self.active_sprite / 64;
                if page < 3 {
                    self.active_sprite = (page + 1) * 64;
                }
            }
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
