//! Debugger panel under the game view: run/step controls, script globals
//! and RAM hex pages. Breakpoint/mode state is reused from
//! [`crate::debugger::Debugger`]; only the rendering is egui.

use super::app::RunState;
use super::theme;
use crate::debugger::Debugger;
use fc_core::memory::{
    MAP_RAM_BASE, MUSIC_RAM_BASE, PALETTE_RAM_BASE, SFX_RAM_BASE, SPRITE_FLAGS_RAM_BASE,
    SPRITE_SHEET_RAM_BASE,
};
use fc_vm::runtime::ConsoleCore;
use std::path::Path;

const RAM_COLS: usize = 16;
const RAM_ROWS: usize = 6;
const RAM_PAGE: usize = RAM_COLS * RAM_ROWS;

const RAM_REGIONS: [(&str, usize); 7] = [
    ("0x0000 WORK", 0),
    ("0x4000 SPRITES", SPRITE_SHEET_RAM_BASE),
    ("0x8000 MAP", MAP_RAM_BASE),
    ("0x9000 FLAGS", SPRITE_FLAGS_RAM_BASE),
    ("0x9100 PALETTE", PALETTE_RAM_BASE),
    ("0x9200 SFX", SFX_RAM_BASE),
    ("0x9600 MUSIC", MUSIC_RAM_BASE),
];

pub struct DebugState {
    pub dbg: Debugger,
    /// Most recent Lua runtime error text — `VmFault::LuaError` stays a unit
    /// variant (kept `Copy` on purpose, see `fc-vm`), so the message is
    /// tracked here instead, Studio-side only.
    pub last_error: Option<String>,
    ram_page: usize,
}

impl Default for DebugState {
    fn default() -> Self {
        Self {
            dbg: Debugger::new(),
            last_error: None,
            ram_page: 0,
        }
    }
}

impl DebugState {
    /// Points breakpoint persistence at `<cart>.fcdbg` and loads it.
    pub fn on_cart_loaded(&mut self, path: &Path) {
        self.dbg.set_fcdbg_path(path.with_extension("fcdbg"));
        self.last_error = None;
    }

    /// Marks a breakpoint line as hit.
    pub fn on_break(&mut self, line: usize) {
        self.dbg.set_cursor_addr(line);
    }
}

pub fn show(
    ui: &mut egui::Ui,
    state: &mut DebugState,
    core: &mut ConsoleCore,
    run_state: &mut RunState,
) {
    controls(ui, state, core, run_state);
    ui.separator();
    status(ui, state, core);
    ui.separator();
    globals(ui, core);
    ram_view(ui, state, core);
}

/// Line breakpoints are set from the code editor's gutter (see
/// `code_panel::gutter`) — this panel just runs/steps and reports state.
/// There's no instruction-level STEP for Lua (mlua's safe hook API can't
/// yield outside a coroutine while `run_frame_lua`'s per-frame `Lua::scope`
/// borrows are live — see `Vm::run_frame_lua_bp`), so STEP here means
/// "run one frame", same granularity as RUN honoring breakpoints.
fn controls(
    ui: &mut egui::Ui,
    state: &mut DebugState,
    core: &mut ConsoleCore,
    run_state: &mut RunState,
) {
    ui.horizontal(|ui| {
        ui.colored_label(theme::ACCENT, "DEBUG");
        let paused = *run_state == RunState::Paused;
        if ui
            .add_enabled(paused, egui::Button::new("STEP"))
            .on_hover_text("run one frame")
            .clicked()
        {
            let bps = state.dbg.breakpoints().to_vec();
            apply_lua_outcome(state, core.run_frame_lua_bp(&bps));
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let bps = state.dbg.breakpoints().len();
            ui.colored_label(
                theme::DIM,
                format!("{bps} BREAKPOINT{}", if bps == 1 { "" } else { "S" }),
            );
        });
    });
}

fn apply_lua_outcome(state: &mut DebugState, outcome: fc_vm::LuaRunOutcome) {
    match outcome {
        fc_vm::LuaRunOutcome::Completed => {}
        fc_vm::LuaRunOutcome::Breakpoint(line) => {
            state.on_break(line);
            state.last_error = None;
        }
        fc_vm::LuaRunOutcome::Error(msg) => {
            state.last_error = Some(msg);
        }
    }
}

fn status(ui: &mut egui::Ui, state: &DebugState, core: &ConsoleCore) {
    ui.horizontal(|ui| {
        if state.dbg.breakpoints().contains(&state.dbg.cursor_addr()) {
            ui.colored_label(
                theme::ACCENT,
                format!("stopped at line {}", state.dbg.cursor_addr()),
            );
        } else {
            ui.colored_label(theme::DIM, "click a line number to set a breakpoint");
        }
    });
    if let Some(err) = &state.last_error {
        ui.colored_label(theme::ERROR, err);
    } else if let Some(fault) = core.vm.get_fault() {
        ui.colored_label(theme::ERROR, format!("FAULT: {fault:?}"));
    }
}

/// Script-defined globals — mlua's safe hook API has no `lua_getlocal`
/// binding, so locals aren't enumerable at a breakpoint; globals are the
/// best available state inspector (see `Vm::lua_globals`).
fn globals(ui: &mut egui::Ui, core: &ConsoleCore) {
    ui.colored_label(theme::DIM, "GLOBALS");
    let globals = core.vm.lua_globals();
    if globals.is_empty() {
        ui.colored_label(theme::DIM, "(none)");
        return;
    }
    egui::ScrollArea::vertical()
        .id_salt("dbg_globals")
        .max_height(80.0)
        .show(ui, |ui| {
            for (name, value) in &globals {
                ui.colored_label(theme::TEXT, format!("{name} = {value}"));
            }
        });
}

fn ram_view(ui: &mut egui::Ui, state: &mut DebugState, core: &ConsoleCore) {
    let mem_len = core.vm.get_memory_length();
    let pages = mem_len.div_ceil(RAM_PAGE).max(1);
    state.ram_page = state.ram_page.min(pages - 1);

    ui.horizontal(|ui| {
        ui.colored_label(theme::DIM, "RAM");
        if ui.small_button("<").clicked() {
            state.ram_page = (state.ram_page + pages - 1) % pages;
        }
        ui.colored_label(theme::TEXT, format!("0x{:04X}", state.ram_page * RAM_PAGE));
        if ui.small_button(">").clicked() {
            state.ram_page = (state.ram_page + 1) % pages;
        }
        egui::ComboBox::from_id_salt("dbg_ram_region")
            .selected_text("GOTO")
            .width(70.0)
            .show_ui(ui, |ui| {
                for (name, base) in RAM_REGIONS {
                    if ui.selectable_label(false, name).clicked() {
                        state.ram_page = base / RAM_PAGE;
                    }
                }
            });
    });

    for row in 0..RAM_ROWS {
        let base = state.ram_page * RAM_PAGE + row * RAM_COLS;
        if base >= mem_len {
            break;
        }
        let mut line = format!("0x{base:04X} ");
        for col in 0..RAM_COLS {
            let i = base + col;
            if i >= mem_len {
                break;
            }
            line.push(' ');
            line.push_str(&format!("{:02X}", core.vm.peek_memory(i)));
        }
        ui.label(
            egui::RichText::new(line)
                .monospace()
                .size(12.0)
                .color(theme::TEXT),
        );
    }
}
