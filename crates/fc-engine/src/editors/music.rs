use fc_core::Vec2;
use fc_vm::rendering::{font::Font, screen::ScreenLayer, text::draw_text};
use fc_vm::vm::Vm;
use fc_vm::vm::sfx::MUSIC_BANK_BASE;
use winit::keyboard::KeyCode;

use super::util::{Grid, clear_panel, fill_rect, rect_border, theme};
use super::{Editor, draw_button};

const PATTERNS: u8 = 8;
const ROWS: u8 = 16;

const SELECTOR_H: u32 = 8;
const GRID_TOP: u32 = 9;
const ROW_H: u32 = 7;

const COL_ROW: u32 = 1;
const COL_CH0: u32 = 18;
const COL_CH1: u32 = 72;

/// Pattern selector strip: 8 boxes of 16px across the top.
const SELECTOR_GRID: Grid = Grid::new(0, 0, 16, SELECTOR_H, PATTERNS as u32, 1);
/// Pattern rows below the column headers.
const ROW_GRID: Grid = Grid::new(0, GRID_TOP, 128, ROW_H, 1, ROWS as u32);

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
    if val == 0 {
        "--".to_string()
    } else {
        format!("SFX {:X}", val - 1)
    }
}

pub struct MusicEditor {
    pattern_id: u8,
    row: u8,
    channel: u8,
}

impl MusicEditor {
    pub fn new() -> Self {
        MusicEditor {
            pattern_id: 0,
            row: 0,
            channel: 0,
        }
    }
}

impl Editor for MusicEditor {
    fn render(&self, layer: &mut ScreenLayer, vm: &Vm, font: &Font, _cursor: (u32, u32)) {
        clear_panel(layer, theme::BG);

        // Pattern selector strip (8 × 16px boxes)
        for pid in 0u8..PATTERNS {
            let x = pid as u32 * 16;
            let col = if pid == self.pattern_id {
                theme::ACTIVE
            } else {
                theme::DIM
            };
            rect_border(layer, x, 0, 15, 7, col);
            draw_text(font, layer, &format!("{}", pid), Vec2::new(x + 5, 1), col);
        }

        // Column headers
        let hy = SELECTOR_H;
        draw_text(font, layer, "RW", Vec2::new(COL_ROW, hy), theme::HEADER);
        draw_text(
            font,
            layer,
            "SQ CHANNEL",
            Vec2::new(COL_CH0, hy),
            theme::HEADER,
        );
        draw_text(
            font,
            layer,
            "NS CHANNEL",
            Vec2::new(COL_CH1, hy),
            theme::HEADER,
        );

        // Row grid
        for r in 0..ROWS {
            let y = GRID_TOP + r as u32 * ROW_H;
            let (ch0, ch1) = read_row(vm, self.pattern_id, r);
            let is_cur = r == self.row;

            if is_cur {
                fill_rect(layer, 0, y, 128, 5, theme::SEL_BG);
            }

            let base_col = if is_cur { theme::ACTIVE } else { theme::DIM };

            draw_text(
                font,
                layer,
                &format!("{:02X}", r),
                Vec2::new(COL_ROW, y),
                base_col,
            );

            let ch0_col = if is_cur && self.channel == 0 {
                theme::SELECTED
            } else if ch0 == 0 {
                theme::EMPTY
            } else {
                base_col
            };
            draw_text(font, layer, &sfx_label(ch0), Vec2::new(COL_CH0, y), ch0_col);

            let ch1_col = if is_cur && self.channel == 1 {
                theme::SELECTED
            } else if ch1 == 0 {
                theme::EMPTY
            } else {
                base_col
            };
            draw_text(font, layer, &sfx_label(ch1), Vec2::new(COL_CH1, y), ch1_col);
        }

        // PLAY / STOP buttons drawn last
        draw_button(layer, font, 100, 10, "PLAY", false);
        draw_button(layer, font, 100, 18, "STOP", false);
    }

    fn handle_scroll(&mut self, _dx: f32, dy: f32, _vm: &mut Vm) {
        if dy < 0.0 && self.pattern_id < PATTERNS - 1 {
            self.pattern_id += 1;
        } else if dy > 0.0 && self.pattern_id > 0 {
            self.pattern_id -= 1;
        }
    }

    fn handle_click(&mut self, x: u32, y: u32, vm: &mut Vm) {
        // PLAY/STOP buttons (check before row grid)
        if x >= 100 {
            if (10..17).contains(&y) {
                vm.start_music(self.pattern_id);
                return;
            }
            if (18..25).contains(&y) {
                vm.stop_music();
                return;
            }
        }
        if let Some((col, _)) = SELECTOR_GRID.cell_at(x, y) {
            self.pattern_id = col as u8;
        } else if let Some((_, row)) = ROW_GRID.cell_at(x, y) {
            self.row = row as u8;
            self.channel = if x >= COL_CH1 { 1 } else { 0 };
        }
    }

    fn handle_key(&mut self, key: KeyCode, vm: &mut Vm) {
        match key {
            KeyCode::BracketLeft => {
                if self.pattern_id > 0 {
                    self.pattern_id -= 1;
                }
            }
            KeyCode::BracketRight => {
                if self.pattern_id < PATTERNS - 1 {
                    self.pattern_id += 1;
                }
            }
            KeyCode::ArrowUp => {
                if self.row > 0 {
                    self.row -= 1;
                }
            }
            KeyCode::ArrowDown => {
                if self.row < ROWS - 1 {
                    self.row += 1;
                }
            }
            KeyCode::ArrowLeft => {
                if self.channel > 0 {
                    self.channel -= 1;
                }
            }
            KeyCode::ArrowRight => {
                if self.channel < 1 {
                    self.channel += 1;
                }
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
