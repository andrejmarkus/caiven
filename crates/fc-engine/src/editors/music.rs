use fc_core::{Color, Vec2};
use fc_vm::rendering::{font::Font, screen::ScreenLayer, text::draw_text};
use fc_vm::vm::sfx::MUSIC_BANK_BASE;
use fc_vm::vm::Vm;
use winit::keyboard::KeyCode;

use super::Editor;

const PATTERNS: u8 = 8;
const ROWS: u8 = 16;

const SELECTOR_H: u32 = 8;
const GRID_TOP: u32 = 9;
const ROW_H: u32 = 7;

const COL_ROW: u32 = 1;
const COL_CH0: u32 = 18;
const COL_CH1: u32 = 72;

fn c_active() -> Color { Color::new_rgb(255, 220, 60) }
fn c_dim() -> Color { Color::new_rgb(160, 160, 160) }
fn c_header() -> Color { Color::new_rgb(200, 200, 200) }
fn c_sel_bg() -> Color { Color::new_rgb(30, 50, 90) }
fn c_sel_cell() -> Color { Color::new_rgb(80, 140, 220) }
fn c_empty() -> Color { Color::new_rgb(80, 80, 80) }

fn pattern_row_base(pattern_id: u8, row: u8) -> usize {
    MUSIC_BANK_BASE + (pattern_id as usize) * (ROWS as usize * 2) + (row as usize) * 2
}

fn read_row(vm: &Vm, pattern_id: u8, row: u8) -> (u8, u8) {
    let base = pattern_row_base(pattern_id, row);
    (vm.peek_memory(base), vm.peek_memory(base + 1))
}

fn write_channel(vm: &mut Vm, pattern_id: u8, row: u8, channel: u8, value: u8) {
    let base = pattern_row_base(pattern_id, row);
    vm.poke_memory(base + channel as usize, value);
}

fn sfx_label(val: u8) -> String {
    if val == 0 { "--".to_string() } else { format!("SFX {:X}", val - 1) }
}

pub struct MusicEditor {
    pattern_id: u8,
    row: u8,
    channel: u8,
}

impl MusicEditor {
    pub fn new() -> Self {
        MusicEditor { pattern_id: 0, row: 0, channel: 0 }
    }
}

impl Editor for MusicEditor {
    fn render(&self, layer: &mut ScreenLayer, vm: &Vm, font: &Font, _cursor: (u32, u32)) {
        let bg = Color::new_rgb(15, 15, 15);
        for y in 0..120u32 {
            for x in 0..128u32 {
                layer.set_pixel(Vec2::new(x, y), bg);
            }
        }

        // Pattern selector strip (8 × 16px boxes)
        for pid in 0u8..PATTERNS {
            let x = pid as u32 * 16;
            let col = if pid == self.pattern_id { c_active() } else { c_dim() };
            for dx in 0..15u32 {
                layer.set_pixel(Vec2::new(x + dx, 0), col);
                layer.set_pixel(Vec2::new(x + dx, 6), col);
            }
            for dy in 1..6u32 {
                layer.set_pixel(Vec2::new(x, dy), col);
                layer.set_pixel(Vec2::new(x + 14, dy), col);
            }
            draw_text(font, layer, &format!("{}", pid), Vec2::new(x + 5, 1), col);
        }

        // Column headers
        let hy = SELECTOR_H;
        draw_text(font, layer, "RW", Vec2::new(COL_ROW, hy), c_header());
        draw_text(font, layer, "SQ CHANNEL", Vec2::new(COL_CH0, hy), c_header());
        draw_text(font, layer, "NS CHANNEL", Vec2::new(COL_CH1, hy), c_header());

        // Row grid
        for r in 0..ROWS {
            let y = GRID_TOP + r as u32 * ROW_H;
            let (ch0, ch1) = read_row(vm, self.pattern_id, r);
            let is_cur = r == self.row;

            if is_cur {
                for px in 0..128u32 {
                    for dy in 0..5u32 {
                        layer.set_pixel(Vec2::new(px, y + dy), c_sel_bg());
                    }
                }
            }

            let base_col = if is_cur { c_active() } else { c_dim() };

            draw_text(font, layer, &format!("{:02X}", r), Vec2::new(COL_ROW, y), base_col);

            let ch0_col = if is_cur && self.channel == 0 { c_sel_cell() } else if ch0 == 0 { c_empty() } else { base_col };
            draw_text(font, layer, &sfx_label(ch0), Vec2::new(COL_CH0, y), ch0_col);

            let ch1_col = if is_cur && self.channel == 1 { c_sel_cell() } else if ch1 == 0 { c_empty() } else { base_col };
            draw_text(font, layer, &sfx_label(ch1), Vec2::new(COL_CH1, y), ch1_col);
        }
    }

    fn handle_click(&mut self, x: u32, y: u32, _vm: &mut Vm) {
        if y < SELECTOR_H {
            self.pattern_id = (x / 16).min(PATTERNS as u32 - 1) as u8;
        } else if y >= GRID_TOP {
            let r = ((y - GRID_TOP) / ROW_H).min(ROWS as u32 - 1) as u8;
            self.row = r;
            self.channel = if x >= COL_CH1 { 1 } else { 0 };
        }
    }

    fn handle_key(&mut self, key: KeyCode, vm: &mut Vm) {
        match key {
            KeyCode::BracketLeft => {
                if self.pattern_id > 0 { self.pattern_id -= 1; }
            }
            KeyCode::BracketRight => {
                if self.pattern_id < PATTERNS - 1 { self.pattern_id += 1; }
            }
            KeyCode::ArrowUp => {
                if self.row > 0 { self.row -= 1; }
            }
            KeyCode::ArrowDown => {
                if self.row < ROWS - 1 { self.row += 1; }
            }
            KeyCode::ArrowLeft => {
                if self.channel > 0 { self.channel -= 1; }
            }
            KeyCode::ArrowRight => {
                if self.channel < 1 { self.channel += 1; }
            }
            KeyCode::Equal | KeyCode::NumpadAdd => {
                let (ch0, ch1) = read_row(vm, self.pattern_id, self.row);
                let cur = if self.channel == 0 { ch0 } else { ch1 };
                let next = (cur + 1).min(16);
                write_channel(vm, self.pattern_id, self.row, self.channel, next);
            }
            KeyCode::Minus | KeyCode::NumpadSubtract => {
                let (ch0, ch1) = read_row(vm, self.pattern_id, self.row);
                let cur = if self.channel == 0 { ch0 } else { ch1 };
                let next = cur.saturating_sub(1);
                write_channel(vm, self.pattern_id, self.row, self.channel, next);
            }
            KeyCode::Space => {
                vm.start_music(self.pattern_id);
            }
            KeyCode::Escape => {
                vm.stop_music();
            }
            KeyCode::Delete | KeyCode::Backspace => {
                write_channel(vm, self.pattern_id, self.row, self.channel, 0);
            }
            _ => {}
        }
    }
}
