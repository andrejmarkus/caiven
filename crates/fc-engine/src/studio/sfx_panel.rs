//! SFX editor panel: 16-step tracker with a drag-to-draw pitch canvas,
//! volume bars, per-step wave/fx toggles and live playback preview.
//! Edits go straight to SFX RAM; undo keeps per-sfx snapshots.

use super::theme;
use egui::{Color32, Pos2, Rect, RichText, Sense, Stroke, StrokeKind, Vec2};
use fc_core::memory::SFX_RAM_BASE;
use fc_vm::Vm;
use fc_vm::vm::sfx::note_name;

pub const STEPS: usize = 16;
pub const SFX_COUNT: usize = 16;
const BYTES_PER_STEP: usize = 4;
const SFX_BYTES: usize = STEPS * BYTES_PER_STEP;
const MAX_NOTE: u8 = 95;
const MAX_VOL: u8 = 15;
const DEFAULT_VOL: u8 = 10;
const UNDO_CAP: usize = 64;

const STEP_W: f32 = 26.0;
const NOTE_H: f32 = 200.0;
const VOL_H: f32 = 64.0;

const PARAM_NOTE: usize = 0;
const PARAM_VOL: usize = 1;
const PARAM_WAVE: usize = 2;
const PARAM_FX: usize = 3;

const FX_LABELS: [&str; 4] = ["--", "SL", "VB", "DR"];

type Snapshot = (usize, [u8; SFX_BYTES]);

pub struct SfxState {
    pub sfx: usize,
    clipboard: Option<[u8; SFX_BYTES]>,
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
    stroke_before: Option<[u8; SFX_BYTES]>,
}

impl Default for SfxState {
    fn default() -> Self {
        Self {
            sfx: 0,
            clipboard: None,
            undo: Vec::new(),
            redo: Vec::new(),
            stroke_before: None,
        }
    }
}

fn sfx_base(sfx: usize) -> usize {
    SFX_RAM_BASE + sfx * SFX_BYTES
}

fn step_param(vm: &Vm, sfx: usize, step: usize, param: usize) -> u8 {
    vm.peek_memory(sfx_base(sfx) + step * BYTES_PER_STEP + param)
}

fn set_step_param(vm: &mut Vm, sfx: usize, step: usize, param: usize, value: u8) {
    vm.poke_memory(sfx_base(sfx) + step * BYTES_PER_STEP + param, value);
}

fn read_sfx(vm: &Vm, sfx: usize) -> [u8; SFX_BYTES] {
    let base = sfx_base(sfx);
    std::array::from_fn(|i| vm.peek_memory(base + i))
}

fn write_sfx(vm: &mut Vm, sfx: usize, data: &[u8; SFX_BYTES]) {
    let base = sfx_base(sfx);
    for (i, b) in data.iter().enumerate() {
        vm.poke_memory(base + i, *b);
    }
}

pub fn sfx_is_empty(vm: &Vm, sfx: usize) -> bool {
    let base = sfx_base(sfx);
    (0..SFX_BYTES).all(|i| vm.peek_memory(base + i) == 0)
}

impl SfxState {
    fn begin_stroke(&mut self, vm: &Vm) {
        if self.stroke_before.is_none() {
            self.stroke_before = Some(read_sfx(vm, self.sfx));
        }
    }

    fn end_stroke(&mut self, vm: &Vm) {
        if let Some(before) = self.stroke_before.take() {
            if before != read_sfx(vm, self.sfx) {
                self.undo.push((self.sfx, before));
                if self.undo.len() > UNDO_CAP {
                    self.undo.remove(0);
                }
                self.redo.clear();
            }
        }
    }

    fn apply_edit(&mut self, vm: &mut Vm, edit: impl FnOnce(&mut Vm, usize)) {
        self.begin_stroke(vm);
        edit(vm, self.sfx);
        self.end_stroke(vm);
    }

    fn undo(&mut self, vm: &mut Vm) {
        if let Some((sfx, data)) = self.undo.pop() {
            self.redo.push((sfx, read_sfx(vm, sfx)));
            write_sfx(vm, sfx, &data);
            self.sfx = sfx;
        }
    }

    fn redo(&mut self, vm: &mut Vm) {
        if let Some((sfx, data)) = self.redo.pop() {
            self.undo.push((sfx, read_sfx(vm, sfx)));
            write_sfx(vm, sfx, &data);
            self.sfx = sfx;
        }
    }
}

pub fn show(ui: &mut egui::Ui, state: &mut SfxState, vm: &mut Vm) {
    handle_shortcuts(ui, state, vm);

    ui.add_space(4.0);
    show_selector(ui, state, vm);
    show_transport(ui, state, vm);
    ui.add_space(4.0);

    // Playhead step while this sfx previews (also tracks music channels)
    let playhead = active_step(vm, state.sfx);

    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.label(RichText::new("PITCH").small().color(theme::DIM));
        show_note_canvas(ui, state, vm, playhead);
        ui.label(RichText::new("VOLUME").small().color(theme::DIM));
        show_vol_canvas(ui, state, vm, playhead);
        show_wave_row(ui, state, vm);
        show_fx_row(ui, state, vm);
        ui.add_space(4.0);
        ui.colored_label(
            theme::DIM,
            "LMB draw · RMB erase note · Space play · Ctrl+Z/Y undo · Ctrl+C/V copy sfx",
        );
    });
}

fn active_step(vm: &Vm, sfx: usize) -> Option<usize> {
    let sp = vm.sfx_player();
    if sp.active && sp.sfx_id as usize == sfx {
        return Some(sp.step as usize);
    }
    let mp = vm.music_player();
    for ch in [&mp.ch0, &mp.ch1] {
        if ch.active && ch.sfx_id as usize == sfx {
            return Some(ch.step as usize);
        }
    }
    None
}

fn handle_shortcuts(ui: &mut egui::Ui, state: &mut SfxState, vm: &mut Vm) {
    if ui.ctx().wants_keyboard_input() {
        return;
    }
    let (undo, redo, copy, paste, play) = ui.input_mut(|i| {
        (
            i.consume_key(egui::Modifiers::CTRL, egui::Key::Z),
            i.consume_key(egui::Modifiers::CTRL, egui::Key::Y),
            i.consume_key(egui::Modifiers::CTRL, egui::Key::C),
            i.consume_key(egui::Modifiers::CTRL, egui::Key::V),
            i.key_pressed(egui::Key::Space),
        )
    });
    if undo {
        state.undo(vm);
    }
    if redo {
        state.redo(vm);
    }
    if copy {
        state.clipboard = Some(read_sfx(vm, state.sfx));
    }
    if paste {
        if let Some(data) = state.clipboard {
            state.apply_edit(vm, |vm, sfx| write_sfx(vm, sfx, &data));
        }
    }
    if play {
        vm.start_sfx(state.sfx as u8);
    }
}

fn show_selector(ui: &mut egui::Ui, state: &mut SfxState, vm: &Vm) {
    ui.horizontal(|ui| {
        ui.label("SFX");
        for i in 0..SFX_COUNT {
            let empty = sfx_is_empty(vm, i);
            let color = if empty { theme::DIM } else { theme::ACCENT };
            let text = RichText::new(format!("{i:X}")).color(color);
            if ui.selectable_label(state.sfx == i, text).clicked() {
                state.sfx = i;
            }
        }
    });
}

fn show_transport(ui: &mut egui::Ui, state: &SfxState, vm: &mut Vm) {
    ui.horizontal(|ui| {
        if ui.button("▶ PLAY").clicked() {
            vm.start_sfx(state.sfx as u8);
        }
        if ui.button("■ STOP").clicked() {
            vm.stop_sfx();
        }
        let sp = vm.sfx_player();
        if sp.active {
            ui.colored_label(
                theme::OK,
                format!("playing {:X} · step {:02}", sp.sfx_id, sp.step),
            );
        }
    });
}

/// Maps a pointer position inside `rect` to (step, value in 0..=max).
fn cell_value(rect: Rect, pos: Pos2, max: u8) -> Option<(usize, u8)> {
    if !rect.contains(pos) {
        return None;
    }
    let step = ((pos.x - rect.min.x) / STEP_W) as usize;
    if step >= STEPS {
        return None;
    }
    let frac = ((rect.max.y - pos.y) / rect.height()).clamp(0.0, 1.0);
    Some((step, (frac * max as f32).round() as u8))
}

fn step_x(rect: Rect, step: usize) -> f32 {
    rect.min.x + step as f32 * STEP_W
}

fn draw_step_grid(painter: &egui::Painter, rect: Rect, playhead: Option<usize>) {
    painter.rect_filled(rect, 0.0, Color32::from_rgb(10, 10, 13));
    if let Some(step) = playhead {
        let r = Rect::from_min_max(
            Pos2::new(step_x(rect, step), rect.min.y),
            Pos2::new(step_x(rect, step) + STEP_W, rect.max.y),
        );
        painter.rect_filled(r, 0.0, Color32::from_rgb(40, 40, 26));
    }
    let grid = Stroke::new(1.0, Color32::from_rgb(35, 35, 45));
    for i in 0..=STEPS {
        let x = step_x(rect, i);
        painter.line_segment([Pos2::new(x, rect.min.y), Pos2::new(x, rect.max.y)], grid);
    }
    // Beat guides every 4 steps
    let beat = Stroke::new(1.0, Color32::from_rgb(60, 60, 75));
    for i in (0..=STEPS).step_by(4) {
        let x = step_x(rect, i);
        painter.line_segment([Pos2::new(x, rect.min.y), Pos2::new(x, rect.max.y)], beat);
    }
}

fn wave_color(wave: u8) -> Color32 {
    if wave == 0 { theme::ACCENT } else { theme::BUILTIN }
}

fn show_note_canvas(ui: &mut egui::Ui, state: &mut SfxState, vm: &mut Vm, playhead: Option<usize>) {
    let size = Vec2::new(STEPS as f32 * STEP_W, NOTE_H);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click_and_drag());
    let painter = ui.painter_at(rect);
    draw_step_grid(&painter, rect, playhead);

    // Note bars: height ∝ pitch, colored by wave
    for step in 0..STEPS {
        let note = step_param(vm, state.sfx, step, PARAM_NOTE);
        if note == 0 {
            continue;
        }
        let h = (note as f32 / MAX_NOTE as f32) * (NOTE_H - 4.0);
        let x = step_x(rect, step);
        let bar = Rect::from_min_max(
            Pos2::new(x + 2.0, rect.max.y - h - 2.0),
            Pos2::new(x + STEP_W - 2.0, rect.max.y - 2.0),
        );
        let wave = step_param(vm, state.sfx, step, PARAM_WAVE);
        painter.rect_filled(bar, 1.0, wave_color(wave));
        // Pitch tick on top of the bar for readability
        painter.rect_filled(
            Rect::from_min_max(bar.min, Pos2::new(bar.max.x, bar.min.y + 3.0)),
            0.0,
            Color32::WHITE,
        );
    }

    let pointer = resp.interact_pointer_pos();
    let primary = ui.input(|i| i.pointer.primary_down());
    let secondary = ui.input(|i| i.pointer.secondary_down());
    if resp.is_pointer_button_down_on() && (primary || secondary) {
        if let Some((step, value)) = pointer.and_then(|p| cell_value(rect, p, MAX_NOTE)) {
            state.begin_stroke(vm);
            if secondary {
                set_step_param(vm, state.sfx, step, PARAM_NOTE, 0);
            } else {
                set_step_param(vm, state.sfx, step, PARAM_NOTE, value.max(1));
                if step_param(vm, state.sfx, step, PARAM_VOL) == 0 {
                    set_step_param(vm, state.sfx, step, PARAM_VOL, DEFAULT_VOL);
                }
            }
        }
    } else {
        state.end_stroke(vm);
    }

    // Hover readout
    if let Some(pos) = resp.hover_pos() {
        if let Some((step, _)) = cell_value(rect, pos, MAX_NOTE) {
            let note = step_param(vm, state.sfx, step, PARAM_NOTE);
            let vol = step_param(vm, state.sfx, step, PARAM_VOL);
            painter.rect_stroke(
                Rect::from_min_max(
                    Pos2::new(step_x(rect, step), rect.min.y),
                    Pos2::new(step_x(rect, step) + STEP_W, rect.max.y),
                ),
                0.0,
                Stroke::new(1.0, theme::ACCENT),
                StrokeKind::Inside,
            );
            painter.text(
                rect.min + Vec2::new(4.0, 4.0),
                egui::Align2::LEFT_TOP,
                format!("step {step:02} · {} · vol {vol}", note_name(note)),
                egui::FontId::monospace(12.0),
                theme::TEXT,
            );
        }
    }
}

fn show_vol_canvas(ui: &mut egui::Ui, state: &mut SfxState, vm: &mut Vm, playhead: Option<usize>) {
    let size = Vec2::new(STEPS as f32 * STEP_W, VOL_H);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click_and_drag());
    let painter = ui.painter_at(rect);
    draw_step_grid(&painter, rect, playhead);

    for step in 0..STEPS {
        let vol = step_param(vm, state.sfx, step, PARAM_VOL);
        if vol == 0 {
            continue;
        }
        let h = (vol as f32 / MAX_VOL as f32) * (VOL_H - 4.0);
        let x = step_x(rect, step);
        painter.rect_filled(
            Rect::from_min_max(
                Pos2::new(x + 2.0, rect.max.y - h - 2.0),
                Pos2::new(x + STEP_W - 2.0, rect.max.y - 2.0),
            ),
            1.0,
            theme::OK,
        );
    }

    if resp.is_pointer_button_down_on() && ui.input(|i| i.pointer.any_down()) {
        if let Some((step, value)) = resp
            .interact_pointer_pos()
            .and_then(|p| cell_value(rect, p, MAX_VOL))
        {
            state.begin_stroke(vm);
            set_step_param(vm, state.sfx, step, PARAM_VOL, value);
        }
    } else {
        state.end_stroke(vm);
    }
}

fn show_wave_row(ui: &mut egui::Ui, state: &mut SfxState, vm: &mut Vm) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        for step in 0..STEPS {
            let wave = step_param(vm, state.sfx, step, PARAM_WAVE);
            let label = RichText::new(if wave == 0 { "S" } else { "N" })
                .color(wave_color(wave))
                .monospace();
            let btn = egui::Button::new(label).min_size(Vec2::new(STEP_W - 2.0, 18.0));
            if ui.add(btn).on_hover_text("wave: Square / Noise").clicked() {
                state.apply_edit(vm, |vm, sfx| {
                    set_step_param(vm, sfx, step, PARAM_WAVE, (wave + 1) % 2);
                });
            }
        }
    });
}

fn show_fx_row(ui: &mut egui::Ui, state: &mut SfxState, vm: &mut Vm) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        for step in 0..STEPS {
            let fx = step_param(vm, state.sfx, step, PARAM_FX).min(3);
            let color = if fx == 0 { theme::DIM } else { theme::TEXT };
            let label = RichText::new(FX_LABELS[fx as usize]).color(color).monospace();
            let btn = egui::Button::new(label).min_size(Vec2::new(STEP_W - 2.0, 18.0));
            if ui
                .add(btn)
                .on_hover_text("fx: -- / SLide / ViBrato / DRop")
                .clicked()
            {
                state.apply_edit(vm, |vm, sfx| {
                    set_step_param(vm, sfx, step, PARAM_FX, (fx + 1) % 4);
                });
            }
        }
    });
}
