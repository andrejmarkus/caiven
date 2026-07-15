//! Music editor panel: 8 patterns of 16 rows × 2 channels (square, noise),
//! each cell referencing an SFX. Live playback preview with row playhead;
//! edits go straight to music RAM with per-pattern undo snapshots.

use super::sfx_panel;
use super::theme;
use egui::RichText;
use fc_core::memory::MUSIC_RAM_BASE;
use fc_vm::Vm;

const PATTERNS: usize = 8;
const ROWS: usize = 16;
const CHANNELS: usize = 2;
const PATTERN_BYTES: usize = ROWS * CHANNELS;
const UNDO_CAP: usize = 32;

type Snapshot = (usize, [u8; PATTERN_BYTES]);

pub struct MusicState {
    pub pattern: usize,
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
}

impl Default for MusicState {
    fn default() -> Self {
        Self {
            pattern: 0,
            undo: Vec::new(),
            redo: Vec::new(),
        }
    }
}

fn pattern_base(pattern: usize) -> usize {
    MUSIC_RAM_BASE + pattern * PATTERN_BYTES
}

fn cell(vm: &Vm, pattern: usize, row: usize, ch: usize) -> u8 {
    vm.peek_memory(pattern_base(pattern) + row * CHANNELS + ch)
}

fn set_cell(vm: &mut Vm, pattern: usize, row: usize, ch: usize, value: u8) {
    vm.poke_memory(pattern_base(pattern) + row * CHANNELS + ch, value);
}

fn read_pattern(vm: &Vm, pattern: usize) -> [u8; PATTERN_BYTES] {
    let base = pattern_base(pattern);
    std::array::from_fn(|i| vm.peek_memory(base + i))
}

fn write_pattern(vm: &mut Vm, pattern: usize, data: &[u8; PATTERN_BYTES]) {
    let base = pattern_base(pattern);
    for (i, b) in data.iter().enumerate() {
        vm.poke_memory(base + i, *b);
    }
}

fn pattern_is_empty(vm: &Vm, pattern: usize) -> bool {
    let base = pattern_base(pattern);
    (0..PATTERN_BYTES).all(|i| vm.peek_memory(base + i) == 0)
}

impl MusicState {
    /// One-shot undoable edit of the current pattern.
    fn apply_edit(&mut self, vm: &mut Vm, edit: impl FnOnce(&mut Vm, usize)) {
        let before = read_pattern(vm, self.pattern);
        edit(vm, self.pattern);
        if before != read_pattern(vm, self.pattern) {
            self.undo.push((self.pattern, before));
            if self.undo.len() > UNDO_CAP {
                self.undo.remove(0);
            }
            self.redo.clear();
        }
    }

    fn undo(&mut self, vm: &mut Vm) {
        if let Some((pattern, data)) = self.undo.pop() {
            self.redo.push((pattern, read_pattern(vm, pattern)));
            write_pattern(vm, pattern, &data);
            self.pattern = pattern;
        }
    }

    fn redo(&mut self, vm: &mut Vm) {
        if let Some((pattern, data)) = self.redo.pop() {
            self.undo.push((pattern, read_pattern(vm, pattern)));
            write_pattern(vm, pattern, &data);
            self.pattern = pattern;
        }
    }
}

pub fn show(ui: &mut egui::Ui, state: &mut MusicState, vm: &mut Vm) {
    handle_shortcuts(ui, state, vm);

    ui.add_space(4.0);
    show_selector(ui, state, vm);
    show_transport(ui, state, vm);
    ui.add_space(4.0);

    let mp = vm.music_player();
    let playhead = (mp.active && mp.pattern_id as usize == state.pattern).then_some(mp.row as usize);

    egui::ScrollArea::vertical().show(ui, |ui| {
        show_grid(ui, state, vm, playhead);
        ui.add_space(4.0);
        ui.colored_label(
            theme::DIM,
            "Space play · Esc stop · Ctrl+Z/Y undo · channel 0 = square, 1 = noise",
        );
    });
}

fn handle_shortcuts(ui: &mut egui::Ui, state: &mut MusicState, vm: &mut Vm) {
    if ui.ctx().wants_keyboard_input() {
        return;
    }
    let (undo, redo, play, stop) = ui.input_mut(|i| {
        (
            i.consume_key(egui::Modifiers::CTRL, egui::Key::Z),
            i.consume_key(egui::Modifiers::CTRL, egui::Key::Y),
            i.key_pressed(egui::Key::Space),
            i.key_pressed(egui::Key::Escape),
        )
    });
    if undo {
        state.undo(vm);
    }
    if redo {
        state.redo(vm);
    }
    if play {
        vm.start_music(state.pattern as u8);
    }
    if stop {
        vm.stop_music();
    }
}

fn show_selector(ui: &mut egui::Ui, state: &mut MusicState, vm: &Vm) {
    ui.horizontal(|ui| {
        ui.label("PATTERN");
        for i in 0..PATTERNS {
            let empty = pattern_is_empty(vm, i);
            let color = if empty { theme::DIM } else { theme::ACCENT };
            let text = RichText::new(format!("{i}")).color(color);
            if ui.selectable_label(state.pattern == i, text).clicked() {
                state.pattern = i;
            }
        }
    });
}

fn show_transport(ui: &mut egui::Ui, state: &MusicState, vm: &mut Vm) {
    ui.horizontal(|ui| {
        if ui.button("▶ PLAY").clicked() {
            vm.start_music(state.pattern as u8);
        }
        if ui.button("■ STOP").clicked() {
            vm.stop_music();
        }
        let mut loop_on = vm.music_player().loop_on;
        if ui.checkbox(&mut loop_on, "loop").changed() {
            vm.set_music_loop(loop_on);
        }
        let mp = vm.music_player();
        if mp.active {
            ui.colored_label(
                theme::OK,
                format!("playing pattern {} · row {:02}", mp.pattern_id, mp.row),
            );
        }
    });
}

fn sfx_ref_label(value: u8) -> String {
    if value == 0 {
        "--".into()
    } else {
        format!("SFX {:X}", value - 1)
    }
}

fn show_grid(ui: &mut egui::Ui, state: &mut MusicState, vm: &mut Vm, playhead: Option<usize>) {
    egui::Grid::new("music-grid")
        .spacing([12.0, 4.0])
        .show(ui, |ui| {
            ui.label(RichText::new("ROW").small().color(theme::DIM));
            ui.label(RichText::new("SQUARE").small().color(theme::ACCENT));
            ui.label(RichText::new("NOISE").small().color(theme::BUILTIN));
            ui.end_row();

            for row in 0..ROWS {
                let is_playing = playhead == Some(row);
                let row_label = if is_playing {
                    RichText::new(format!("▶ {row:02}")).color(theme::OK)
                } else {
                    RichText::new(format!("  {row:02}")).color(theme::DIM)
                };
                ui.label(row_label.monospace());

                for ch in 0..CHANNELS {
                    show_cell(ui, state, vm, row, ch);
                }
                ui.end_row();
            }
        });
}

fn show_cell(ui: &mut egui::Ui, state: &mut MusicState, vm: &mut Vm, row: usize, ch: usize) {
    let current = cell(vm, state.pattern, row, ch);
    let text = if current == 0 {
        RichText::new(sfx_ref_label(current)).color(theme::DIM)
    } else {
        RichText::new(sfx_ref_label(current)).color(theme::TEXT)
    };
    let mut selected = current;
    egui::ComboBox::from_id_salt(("music-cell", row, ch))
        .selected_text(text)
        .width(90.0)
        .show_ui(ui, |ui| {
            ui.selectable_value(&mut selected, 0, "--");
            for sfx in 0..sfx_panel::SFX_COUNT as u8 {
                let color = if sfx_panel::sfx_is_empty(vm, sfx as usize) {
                    theme::DIM
                } else {
                    theme::TEXT
                };
                ui.selectable_value(
                    &mut selected,
                    sfx + 1,
                    RichText::new(sfx_ref_label(sfx + 1)).color(color),
                );
            }
        });
    if selected != current {
        state.apply_edit(vm, |vm, pattern| set_cell(vm, pattern, row, ch, selected));
    }
}
