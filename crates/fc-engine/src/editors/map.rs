use fc_core::{Color, Vec2};
use fc_vm::rendering::{font::Font, screen::ScreenLayer, text::draw_text};
use fc_vm::vm::Vm;
use winit::keyboard::KeyCode;

use super::{Editor, button_hit, draw_button};

const MAP_RAM_BASE: usize = 0x5000;
const SPRITE_SHEET_BASE: usize = 0x4000;
const MAP_W: usize = 64;
const MAP_H: usize = 32;
const SPRITE_SIZE: usize = 8;

const VIEW_TILES_W: usize = 12;
const VIEW_TILES_H: usize = 15;
const MAP_AREA_W: u32 = (VIEW_TILES_W * SPRITE_SIZE) as u32; // 96

const PICKER_X: u32 = 96;
const PICKER_COLS: usize = 4;
const PICKER_ROWS: usize = 8;

#[derive(Clone, Copy, PartialEq, Eq)]
enum MapEditorMode {
    Paint,
    Select,
    Paste,
}

struct ClipboardRegion {
    w: usize,
    h: usize,
    tiles: Vec<u8>,
}

pub struct MapEditor {
    pub active_sprite: u8,
    view_x: usize,
    view_y: usize,
    edit_mode: MapEditorMode,
    sel_anchor: Option<(usize, usize)>,
    sel_end: Option<(usize, usize)>,
    clipboard: Option<ClipboardRegion>,
}

impl MapEditor {
    pub fn new() -> Self {
        MapEditor {
            active_sprite: 0,
            view_x: 0,
            view_y: 0,
            edit_mode: MapEditorMode::Paint,
            sel_anchor: None,
            sel_end: None,
            clipboard: None,
        }
    }

    fn clamp_view(&mut self) {
        self.view_x = self.view_x.min(MAP_W.saturating_sub(VIEW_TILES_W));
        self.view_y = self.view_y.min(MAP_H.saturating_sub(VIEW_TILES_H));
    }

    fn screen_to_map_tile(&self, x: u32, y: u32) -> Option<(usize, usize)> {
        if x >= MAP_AREA_W || y >= 120 {
            return None;
        }
        let tx = self.view_x + (x / SPRITE_SIZE as u32) as usize;
        let ty = self.view_y + (y / SPRITE_SIZE as u32) as usize;
        if tx < MAP_W && ty < MAP_H {
            Some((tx, ty))
        } else {
            None
        }
    }

    fn map_tile_to_screen(&self, tx: usize, ty: usize) -> Option<(u32, u32)> {
        if tx < self.view_x || ty < self.view_y {
            return None;
        }
        let sx = tx - self.view_x;
        let sy = ty - self.view_y;
        if sx >= VIEW_TILES_W || sy >= VIEW_TILES_H {
            return None;
        }
        Some((
            sx as u32 * SPRITE_SIZE as u32,
            sy as u32 * SPRITE_SIZE as u32,
        ))
    }

    fn selection_rect(&self) -> Option<(usize, usize, usize, usize)> {
        let (ax, ay) = self.sel_anchor?;
        let (ex, ey) = self.sel_end?;
        let x0 = ax.min(ex);
        let y0 = ay.min(ey);
        let x1 = ax.max(ex);
        let y1 = ay.max(ey);
        Some((x0, y0, x1, y1))
    }

    fn fill_selection(&self, vm: &mut Vm, tile: u8) {
        let (x0, y0, x1, y1) = match self.selection_rect() {
            Some(r) => r,
            None => return,
        };
        for row in y0..=y1 {
            for col in x0..=x1 {
                vm.poke_memory(MAP_RAM_BASE + row * MAP_W + col, tile);
            }
        }
    }

    fn copy_selection(&mut self, vm: &Vm) {
        let (x0, y0, x1, y1) = match self.selection_rect() {
            Some(r) => r,
            None => return,
        };
        let w = x1 - x0 + 1;
        let h = y1 - y0 + 1;
        let mut tiles = Vec::with_capacity(w * h);
        for row in y0..=y1 {
            for col in x0..=x1 {
                tiles.push(vm.peek_memory(MAP_RAM_BASE + row * MAP_W + col));
            }
        }
        self.clipboard = Some(ClipboardRegion { w, h, tiles });
    }

    fn stamp_clipboard(&self, tx: usize, ty: usize, vm: &mut Vm) {
        let cb = match &self.clipboard {
            Some(c) => c,
            None => return,
        };
        for row in 0..cb.h {
            for col in 0..cb.w {
                let mx = tx + col;
                let my = ty + row;
                if mx < MAP_W && my < MAP_H {
                    vm.poke_memory(MAP_RAM_BASE + my * MAP_W + mx, cb.tiles[row * cb.w + col]);
                }
            }
        }
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

    fn draw_rect_border(layer: &mut ScreenLayer, x: u32, y: u32, w: u32, h: u32, color: Color) {
        for dx in 0..w {
            if x + dx < 128 {
                layer.set_pixel(Vec2::new(x + dx, y), color);
                if y + h > 0 {
                    layer.set_pixel(Vec2::new(x + dx, y + h - 1), color);
                }
            }
        }
        for dy in 0..h {
            if y + dy < 128 {
                layer.set_pixel(Vec2::new(x, y + dy), color);
                if x + w > 0 {
                    layer.set_pixel(Vec2::new(x + w - 1, y + dy), color);
                }
            }
        }
    }
}

impl Editor for MapEditor {
    fn render(&self, layer: &mut ScreenLayer, vm: &Vm, font: &Font, cursor: (u32, u32)) {
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

        // Grid overlay
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

        // Selection rectangle highlight
        if (self.edit_mode == MapEditorMode::Select || self.edit_mode == MapEditorMode::Paint)
            && let Some((x0, y0, x1, y1)) = self.selection_rect()
        {
            let sel_color = Color::new_rgb(255, 200, 0);
            // Clamp to viewport
            let vx0 = x0.max(self.view_x);
            let vy0 = y0.max(self.view_y);
            let vx1 = x1.min(self.view_x + VIEW_TILES_W - 1);
            let vy1 = y1.min(self.view_y + VIEW_TILES_H - 1);
            if vx0 <= vx1 && vy0 <= vy1 {
                let sx = (vx0 - self.view_x) as u32 * SPRITE_SIZE as u32;
                let sy = (vy0 - self.view_y) as u32 * SPRITE_SIZE as u32;
                let sw = (vx1 - vx0 + 1) as u32 * SPRITE_SIZE as u32;
                let sh = (vy1 - vy0 + 1) as u32 * SPRITE_SIZE as u32;
                Self::draw_rect_border(layer, sx, sy, sw, sh, sel_color);
            }
        }

        // Paste mode: ghost preview at cursor tile
        if self.edit_mode == MapEditorMode::Paste
            && let Some(cb) = &self.clipboard
            && cursor.0 < MAP_AREA_W
            && cursor.1 < 120
            && let Some((tx, ty)) = self.screen_to_map_tile(cursor.0, cursor.1)
            && let Some((sx, sy)) = self.map_tile_to_screen(tx, ty)
        {
            let pw = cb.w as u32 * SPRITE_SIZE as u32;
            let ph = cb.h as u32 * SPRITE_SIZE as u32;
            Self::draw_rect_border(layer, sx, sy, pw, ph, Color::new_rgb(0, 255, 128));
        }

        // Sprite picker background
        let picker_bg = Color::new_rgb(20, 20, 20);
        for y in 0..120u32 {
            for x in PICKER_X..128u32 {
                layer.set_pixel(Vec2::new(x, y), picker_bg);
            }
        }

        // Draw sprite picker (4 cols × 8 rows = 32 sprites), paged by active_sprite
        let picker_base = (self.active_sprite as usize / 32) * 32;
        for row in 0..PICKER_ROWS {
            for col in 0..PICKER_COLS {
                let idx = picker_base + row * PICKER_COLS + col;
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

        // Info text
        let label = format!("S:{}", self.active_sprite);
        draw_text(
            font,
            layer,
            &label,
            Vec2::new(PICKER_X, 64),
            Color::new_rgb(200, 200, 200),
        );
        let page_label = format!("P{}", picker_base / 32);
        draw_text(
            font,
            layer,
            &page_label,
            Vec2::new(PICKER_X + 20, 64),
            Color::new_rgb(120, 120, 120),
        );
        let scroll = format!("{},{}", self.view_x, self.view_y);
        draw_text(
            font,
            layer,
            &scroll,
            Vec2::new(PICKER_X, 72),
            Color::new_rgb(120, 120, 120),
        );
        let mode_label = match self.edit_mode {
            MapEditorMode::Paint => "",
            MapEditorMode::Select => "SEL",
            MapEditorMode::Paste => "PST",
        };
        if !mode_label.is_empty() {
            draw_text(
                font,
                layer,
                mode_label,
                Vec2::new(PICKER_X, 80),
                Color::new_rgb(255, 200, 0),
            );
        }
        if self.clipboard.is_some() {
            draw_text(
                font,
                layer,
                "CB",
                Vec2::new(PICKER_X + 20, 80),
                Color::new_rgb(0, 200, 100),
            );
        }

        // Sidebar action buttons at y=88..119
        draw_button(
            layer,
            font,
            PICKER_X + 1,
            88,
            "SEL",
            self.edit_mode == MapEditorMode::Select,
        );
        if self.edit_mode == MapEditorMode::Select {
            draw_button(layer, font, PICKER_X + 1, 96, "CPY", false);
            draw_button(layer, font, PICKER_X + 1, 104, "FIL", false);
        }
        if self.clipboard.is_some() {
            draw_button(
                layer,
                font,
                PICKER_X + 1,
                112,
                "PST",
                self.edit_mode == MapEditorMode::Paste,
            );
        }
    }

    fn handle_right_click(&mut self, x: u32, y: u32, vm: &mut Vm) {
        if y >= 120 || x >= MAP_AREA_W {
            return;
        }
        if let Some((tx, ty)) = self.screen_to_map_tile(x, y) {
            vm.poke_memory(MAP_RAM_BASE + ty * MAP_W + tx, 0);
        }
    }

    fn handle_scroll(&mut self, dx: f32, dy: f32, _vm: &mut Vm) {
        if dy > 0.0 {
            self.view_y = self.view_y.saturating_sub(1);
        } else if dy < 0.0 {
            self.view_y += 1;
        }
        if dx > 0.0 {
            self.view_x += 1;
        } else if dx < 0.0 {
            self.view_x = self.view_x.saturating_sub(1);
        }
        self.clamp_view();
    }

    fn handle_click(&mut self, x: u32, y: u32, vm: &mut Vm) {
        if y >= 120 {
            return;
        }

        // Sidebar action buttons (below picker at y>=88)
        if x >= PICKER_X && y >= 88 {
            if button_hit(PICKER_X + 1, 88, "SEL", x, y) {
                self.edit_mode = match self.edit_mode {
                    MapEditorMode::Select => {
                        self.sel_anchor = None;
                        self.sel_end = None;
                        MapEditorMode::Paint
                    }
                    _ => {
                        self.sel_anchor = None;
                        self.sel_end = None;
                        MapEditorMode::Select
                    }
                };
            } else if button_hit(PICKER_X + 1, 96, "CPY", x, y) {
                self.copy_selection(vm);
            } else if button_hit(PICKER_X + 1, 104, "FIL", x, y) {
                let tile = self.active_sprite;
                self.fill_selection(vm, tile);
            } else if button_hit(PICKER_X + 1, 112, "PST", x, y) && self.clipboard.is_some() {
                self.edit_mode = MapEditorMode::Paste;
            }
            return;
        }

        match self.edit_mode {
            MapEditorMode::Paint => {
                if x < MAP_AREA_W {
                    if let Some((tx, ty)) = self.screen_to_map_tile(x, y) {
                        vm.poke_memory(MAP_RAM_BASE + ty * MAP_W + tx, self.active_sprite);
                    }
                } else if x >= PICKER_X && y < (PICKER_ROWS * SPRITE_SIZE) as u32 {
                    let col = ((x - PICKER_X) / SPRITE_SIZE as u32) as usize;
                    let row = (y / SPRITE_SIZE as u32) as usize;
                    let picker_base = (self.active_sprite as usize / 32) * 32;
                    let idx = picker_base + row * PICKER_COLS + col;
                    if idx < 256 {
                        self.active_sprite = idx as u8;
                    }
                }
            }
            MapEditorMode::Select => {
                if x < MAP_AREA_W
                    && let Some(tile) = self.screen_to_map_tile(x, y)
                {
                    self.sel_anchor = Some(tile);
                    self.sel_end = Some(tile);
                }
            }
            MapEditorMode::Paste => {
                if x < MAP_AREA_W
                    && let Some((tx, ty)) = self.screen_to_map_tile(x, y)
                {
                    self.stamp_clipboard(tx, ty, vm);
                }
            }
        }
    }

    fn handle_drag(&mut self, x: u32, y: u32, vm: &mut Vm) {
        if y >= 120 {
            return;
        }
        match self.edit_mode {
            MapEditorMode::Paint => {
                if x < MAP_AREA_W
                    && let Some((tx, ty)) = self.screen_to_map_tile(x, y)
                {
                    vm.poke_memory(MAP_RAM_BASE + ty * MAP_W + tx, self.active_sprite);
                }
            }
            MapEditorMode::Select => {
                if x < MAP_AREA_W
                    && let Some(tile) = self.screen_to_map_tile(x, y)
                {
                    self.sel_end = Some(tile);
                }
            }
            MapEditorMode::Paste => {}
        }
    }

    fn handle_key(&mut self, key: KeyCode, vm: &mut Vm) {
        match key {
            KeyCode::ArrowLeft => {
                if self.edit_mode == MapEditorMode::Paint || self.edit_mode == MapEditorMode::Select
                {
                    self.view_x = self.view_x.saturating_sub(1);
                    self.clamp_view();
                }
            }
            KeyCode::ArrowRight => {
                if self.edit_mode == MapEditorMode::Paint || self.edit_mode == MapEditorMode::Select
                {
                    self.view_x += 1;
                    self.clamp_view();
                }
            }
            KeyCode::ArrowUp => {
                if self.edit_mode == MapEditorMode::Paint || self.edit_mode == MapEditorMode::Select
                {
                    self.view_y = self.view_y.saturating_sub(1);
                    self.clamp_view();
                }
            }
            KeyCode::ArrowDown => {
                if self.edit_mode == MapEditorMode::Paint || self.edit_mode == MapEditorMode::Select
                {
                    self.view_y += 1;
                    self.clamp_view();
                }
            }
            KeyCode::BracketLeft => {
                self.active_sprite = self.active_sprite.saturating_sub(1);
            }
            KeyCode::BracketRight => {
                self.active_sprite = self.active_sprite.saturating_add(1);
            }
            // Page through picker in blocks of 32
            KeyCode::PageUp => {
                let page = self.active_sprite as usize / 32;
                if page > 0 {
                    self.active_sprite = ((page - 1) * 32) as u8;
                }
            }
            KeyCode::PageDown => {
                let page = self.active_sprite as usize / 32;
                if page < 7 {
                    self.active_sprite = ((page + 1) * 32) as u8;
                }
            }
            KeyCode::KeyS => match self.edit_mode {
                MapEditorMode::Select => {
                    self.edit_mode = MapEditorMode::Paint;
                    self.sel_anchor = None;
                    self.sel_end = None;
                }
                _ => {
                    self.edit_mode = MapEditorMode::Select;
                    self.sel_anchor = None;
                    self.sel_end = None;
                }
            },
            KeyCode::KeyC => {
                if self.edit_mode == MapEditorMode::Select {
                    self.copy_selection(vm);
                }
            }
            KeyCode::KeyF => {
                if self.edit_mode == MapEditorMode::Select {
                    let tile = self.active_sprite;
                    self.fill_selection(vm, tile);
                }
            }
            KeyCode::Delete => {
                if self.edit_mode == MapEditorMode::Select {
                    self.fill_selection(vm, 0);
                }
            }
            KeyCode::KeyV => {
                if self.clipboard.is_some() {
                    self.edit_mode = MapEditorMode::Paste;
                }
            }
            KeyCode::Escape => {
                self.edit_mode = MapEditorMode::Paint;
            }
            _ => {}
        }
    }
}
