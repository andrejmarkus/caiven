use fc_core::Vec2;
use fc_core::memory::SFX_RAM_BASE as SFX_BANK_BASE;
use fc_vm::rendering::{font::Font, screen::ScreenLayer, text::draw_text};
use fc_vm::vm::Vm;
use fc_vm::vm::sfx::note_name;
use winit::keyboard::KeyCode;

use super::util::{Grid, clear_panel, fill_rect, rect_border, theme};
use super::{Editor, draw_button};

const STEPS: u8 = 16;
const BYTES_PER_STEP: usize = 4;
const BYTES_PER_SFX: usize = STEPS as usize * BYTES_PER_STEP;

const PARAM_NOTE: u8 = 0;
const PARAM_VOL: u8 = 1;
const PARAM_WAVE: u8 = 2;
const PARAM_FX: u8 = 3;

const COL_STEP: u32 = 1;
const COL_NOTE: u32 = 16;
const COL_VOL: u32 = 35;
const COL_WAVE: u32 = 72;
const COL_FX: u32 = 82;

const SELECTOR_H: u32 = 8;
const ROW_H: u32 = 7;
const GRID_TOP: u32 = 9;

/// SFX selector strip: 16 boxes of 8px across the top.
const SELECTOR_GRID: Grid = Grid::new(0, 0, 8, SELECTOR_H, 16, 1);
/// Step rows below the column headers.
const STEP_GRID: Grid = Grid::new(0, GRID_TOP, 128, ROW_H, 1, STEPS as u32);

fn step_base(sfx_id: u8, step: u8) -> usize {
    SFX_BANK_BASE + (sfx_id as usize) * BYTES_PER_SFX + (step as usize) * BYTES_PER_STEP
}

fn read_step(vm: &Vm, sfx_id: u8, step: u8) -> [u8; 4] {
    let base = step_base(sfx_id, step);
    [
        vm.peek_memory(base),
        vm.peek_memory(base + 1),
        vm.peek_memory(base + 2),
        vm.peek_memory(base + 3),
    ]
}

fn write_param(vm: &mut Vm, sfx_id: u8, step: u8, param: u8, value: u8) {
    vm.poke_memory(step_base(sfx_id, step) + param as usize, value);
}

pub struct SfxEditor {
    sfx_id: u8,
    step: u8,
    param: u8,
}

impl SfxEditor {
    pub fn new() -> Self {
        SfxEditor {
            sfx_id: 0,
            step: 0,
            param: PARAM_NOTE,
        }
    }
}

impl Editor for SfxEditor {
    fn render(&self, layer: &mut ScreenLayer, vm: &Vm, font: &Font, _cursor: (u32, u32)) {
        clear_panel(layer, theme::BG);

        // SFX selector strip (16 × 8px boxes)
        for id in 0u8..16 {
            let x = id as u32 * 8;
            let col = if id == self.sfx_id {
                theme::ACTIVE
            } else {
                theme::DIM
            };
            rect_border(layer, x, 0, 7, 7, col);
            let label = format!("{:X}", id);
            draw_text(font, layer, &label, Vec2::new(x + 2, 1), col);
        }

        // Column headers at y=8
        let hy = SELECTOR_H;
        draw_text(font, layer, "ST", Vec2::new(COL_STEP, hy), theme::HEADER);
        draw_text(font, layer, "NOTE", Vec2::new(COL_NOTE, hy), theme::HEADER);
        draw_text(font, layer, "VOL", Vec2::new(COL_VOL, hy), theme::HEADER);
        draw_text(font, layer, "W", Vec2::new(COL_WAVE, hy), theme::HEADER);
        draw_text(font, layer, "FX", Vec2::new(COL_FX, hy), theme::HEADER);

        // Step rows
        for s in 0..STEPS {
            let y = GRID_TOP + s as u32 * ROW_H;
            let [note, vol, wave, fx] = read_step(vm, self.sfx_id, s);
            let is_cur = s == self.step;

            if is_cur {
                fill_rect(layer, 0, y, 128, 5, theme::SEL_BG);
            }

            let base_col = if is_cur { theme::ACTIVE } else { theme::DIM };

            // Step number
            draw_text(
                font,
                layer,
                &format!("{:02X}", s),
                Vec2::new(COL_STEP, y),
                base_col,
            );

            // Note
            let note_col = if is_cur && self.param == PARAM_NOTE {
                theme::SELECTED
            } else {
                base_col
            };
            draw_text(
                font,
                layer,
                &note_name(note),
                Vec2::new(COL_NOTE, y),
                note_col,
            );

            // Volume bar (max 30px wide = vol * 2)
            let vol_col = if is_cur && self.param == PARAM_VOL {
                theme::SELECTED
            } else {
                theme::BAR
            };
            let bar_w = (vol as u32 * 2).min(30);
            fill_rect(layer, COL_VOL, y + 1, bar_w, 3, vol_col);
            let vol_label = format!("{:X}", vol);
            draw_text(font, layer, &vol_label, Vec2::new(COL_VOL + 32, y), vol_col);

            // Wave
            let wave_col = if is_cur && self.param == PARAM_WAVE {
                theme::SELECTED
            } else {
                base_col
            };
            draw_text(
                font,
                layer,
                if wave == 0 { "S" } else { "N" },
                Vec2::new(COL_WAVE, y),
                wave_col,
            );

            // FX
            let fx_col = if is_cur && self.param == PARAM_FX {
                theme::SELECTED
            } else {
                base_col
            };
            let fx_str = match fx {
                1 => "SL",
                2 => "VB",
                3 => "DR",
                _ => "--",
            };
            draw_text(font, layer, fx_str, Vec2::new(COL_FX, y), fx_col);
        }

        // PLAY / STOP buttons drawn last (top-right, above grid rows)
        draw_button(layer, font, 95, 10, "PLAY", false);
        draw_button(layer, font, 95, 18, "STOP", false);
    }

    fn handle_scroll(&mut self, _dx: f32, dy: f32, _vm: &mut Vm) {
        if dy < 0.0 && self.sfx_id < 15 {
            self.sfx_id += 1;
        } else if dy > 0.0 && self.sfx_id > 0 {
            self.sfx_id -= 1;
        }
    }

    fn handle_click(&mut self, x: u32, y: u32, vm: &mut Vm) {
        // PLAY/STOP buttons (drawn on top of grid rows, check first)
        if x >= 95 {
            if (10..17).contains(&y) {
                vm.start_sfx(self.sfx_id);
                return;
            }
            if (18..25).contains(&y) {
                vm.stop_sfx();
                return;
            }
        }
        if let Some((col, _)) = SELECTOR_GRID.cell_at(x, y) {
            self.sfx_id = col as u8;
        } else if let Some((_, row)) = STEP_GRID.cell_at(x, y) {
            self.step = row as u8;
            self.param = if x >= COL_FX {
                PARAM_FX
            } else if x >= COL_WAVE {
                PARAM_WAVE
            } else if x >= COL_VOL {
                PARAM_VOL
            } else {
                PARAM_NOTE
            };
        }
    }

    fn handle_key(&mut self, key: KeyCode, vm: &mut Vm) {
        match key {
            KeyCode::ArrowUp => {
                self.step = self.step.saturating_sub(1);
            }
            KeyCode::ArrowDown => {
                if self.step < STEPS - 1 {
                    self.step += 1;
                }
            }
            KeyCode::ArrowLeft => {
                if self.param > 0 {
                    self.param -= 1;
                }
            }
            KeyCode::ArrowRight => {
                if self.param < 3 {
                    self.param += 1;
                }
            }
            KeyCode::BracketLeft => {
                if self.sfx_id > 0 {
                    self.sfx_id -= 1;
                }
            }
            KeyCode::BracketRight => {
                if self.sfx_id < 15 {
                    self.sfx_id += 1;
                }
            }
            KeyCode::Equal | KeyCode::NumpadAdd => {
                let [note, vol, wave, fx] = read_step(vm, self.sfx_id, self.step);
                match self.param {
                    PARAM_NOTE => write_param(
                        vm,
                        self.sfx_id,
                        self.step,
                        PARAM_NOTE,
                        note.saturating_add(1).min(95),
                    ),
                    PARAM_VOL => write_param(
                        vm,
                        self.sfx_id,
                        self.step,
                        PARAM_VOL,
                        vol.saturating_add(1).min(15),
                    ),
                    PARAM_WAVE => {
                        write_param(vm, self.sfx_id, self.step, PARAM_WAVE, (wave + 1) % 2)
                    }
                    PARAM_FX => write_param(vm, self.sfx_id, self.step, PARAM_FX, (fx + 1) % 4),
                    _ => {}
                }
            }
            KeyCode::Minus | KeyCode::NumpadSubtract => {
                let [note, vol, wave, fx] = read_step(vm, self.sfx_id, self.step);
                match self.param {
                    PARAM_NOTE => write_param(
                        vm,
                        self.sfx_id,
                        self.step,
                        PARAM_NOTE,
                        note.saturating_sub(1),
                    ),
                    PARAM_VOL => {
                        write_param(vm, self.sfx_id, self.step, PARAM_VOL, vol.saturating_sub(1))
                    }
                    PARAM_WAVE => {
                        write_param(vm, self.sfx_id, self.step, PARAM_WAVE, (wave + 1) % 2)
                    }
                    PARAM_FX => {
                        write_param(vm, self.sfx_id, self.step, PARAM_FX, fx.saturating_sub(1))
                    }
                    _ => {}
                }
            }
            KeyCode::Space => {
                vm.start_sfx(self.sfx_id);
            }
            KeyCode::Delete | KeyCode::Backspace => {
                for offset in 0..BYTES_PER_STEP {
                    vm.poke_memory(step_base(self.sfx_id, self.step) + offset, 0);
                }
            }
            _ => {}
        }
    }
}
