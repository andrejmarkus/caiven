//! Debugger panel under the game view: disassembly with clickable
//! breakpoints, registers, RAM hex pages and a snapshot timeline scrubber.
//! State logic (breakpoints, .fcdbg persistence, snapshots) is reused from
//! [`crate::debugger::Debugger`]; only the rendering is egui.

use super::app::RunState;
use super::theme;
use crate::debugger::Debugger;
use fc_core::memory::{
    MAP_RAM_BASE, MUSIC_RAM_BASE, PALETTE_RAM_BASE, SFX_RAM_BASE, SPRITE_FLAGS_RAM_BASE,
    SPRITE_SHEET_RAM_BASE,
};
use fc_vm::Vm;
use fc_vm::runtime::ConsoleCore;
use std::path::Path;

const RAM_COLS: usize = 16;
const RAM_ROWS: usize = 6;
const RAM_PAGE: usize = RAM_COLS * RAM_ROWS;
const DISASM_ROWS: f32 = 9.0;

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
    pub recording: bool,
    ram_page: usize,
    resume_ignore: Option<usize>,
    scroll_to_pc: bool,
}

impl Default for DebugState {
    fn default() -> Self {
        Self {
            dbg: Debugger::new(true),
            recording: true,
            ram_page: 0,
            resume_ignore: None,
            scroll_to_pc: false,
        }
    }
}

impl DebugState {
    /// Points breakpoint persistence at `<cart>.fcdbg` and loads it.
    pub fn on_cart_loaded(&mut self, path: &Path) {
        self.dbg.set_fcdbg_path(path.with_extension("fcdbg"));
    }

    /// Suppresses a re-trap on `pc` for the first instruction after resume.
    pub fn set_resume_ignore(&mut self, pc: usize) {
        self.resume_ignore = Some(pc);
    }

    pub fn take_resume_ignore(&mut self) -> Option<usize> {
        self.resume_ignore.take()
    }

    pub fn on_break(&mut self, pc: usize) {
        self.dbg.set_cursor_addr(pc);
        self.scroll_to_pc = true;
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
    disasm(ui, state, core);
    ui.separator();
    registers(ui, state, core);
    ram_view(ui, state, core);
    timeline(ui, state, core, run_state);
}

fn controls(
    ui: &mut egui::Ui,
    state: &mut DebugState,
    core: &mut ConsoleCore,
    run_state: &mut RunState,
) {
    ui.horizontal(|ui| {
        ui.colored_label(theme::ACCENT, "DEBUG");
        ui.checkbox(&mut state.recording, "REC")
            .on_hover_text("record one snapshot per frame for the timeline");

        let paused = *run_state == RunState::Paused;
        if ui
            .add_enabled(paused, egui::Button::new("STEP"))
            .on_hover_text("execute one instruction")
            .clicked()
        {
            core.step();
            state.on_break(core.vm.get_pc());
        }
        if ui
            .add_enabled(paused, egui::Button::new("FRAME"))
            .on_hover_text("run one full frame")
            .clicked()
        {
            let bps = state.dbg.breakpoints().to_vec();
            let ignore = Some(core.vm.get_pc());
            match core.run_frame_bp(&bps, ignore) {
                Some(pc) => state.on_break(pc),
                None => {
                    if state.recording {
                        state.dbg.push_state(core.vm.snapshot());
                    }
                    state.on_break(core.vm.get_pc());
                }
            }
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.colored_label(theme::DIM, format!("{} SNAP", state.dbg.states_len()));
        });
    });
}

fn disasm(ui: &mut egui::Ui, state: &mut DebugState, core: &ConsoleCore) {
    let pc = core.vm.get_pc();
    let addrs = core.vm.get_source_map().sorted_addresses();
    let total = if addrs.is_empty() {
        core.vm.get_program().len()
    } else {
        addrs.len()
    };
    if total == 0 {
        ui.colored_label(theme::DIM, "no program loaded");
        return;
    }

    let row_h = ui.text_style_height(&egui::TextStyle::Monospace);
    let row_h_full = row_h + ui.spacing().item_spacing.y;
    let mut area = egui::ScrollArea::vertical()
        .id_salt("dbg_disasm")
        .max_height(row_h_full * DISASM_ROWS)
        .auto_shrink([false, false]);
    if state.scroll_to_pc {
        state.scroll_to_pc = false;
        let idx = if addrs.is_empty() {
            pc.min(total - 1)
        } else {
            match addrs.binary_search(&pc) {
                Ok(i) => i,
                Err(i) => i.saturating_sub(1),
            }
        };
        area = area.vertical_scroll_offset(idx.saturating_sub(3) as f32 * row_h_full);
    }

    area.show_rows(ui, row_h, total, |ui, range| {
        for i in range {
            let addr = if addrs.is_empty() { i } else { addrs[i] };
            disasm_row(ui, &mut state.dbg, &core.vm, addr, pc);
        }
    });
}

fn disasm_row(ui: &mut egui::Ui, dbg: &mut Debugger, vm: &Vm, addr: usize, pc: usize) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;

        let is_bp = dbg.breakpoints().contains(&addr);
        let dot = if is_bp {
            egui::RichText::new("\u{25CF}").color(theme::ERROR)
        } else {
            egui::RichText::new("\u{00B7}").color(theme::DIM)
        };
        if ui
            .add(egui::Label::new(dot).sense(egui::Sense::click()))
            .on_hover_text("toggle breakpoint")
            .clicked()
        {
            dbg.set_cursor_addr(addr);
            dbg.toggle_bp_at_cursor();
        }

        let color = if addr == pc {
            theme::OK
        } else if addr == dbg.cursor_addr() {
            theme::ACCENT
        } else {
            theme::TEXT
        };
        let text: String = vm
            .get_source_map()
            .get(addr)
            .and_then(|info| info.src_line)
            .and_then(|ln| vm.get_fc_source_line(ln))
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|| vm.disassemble(addr));
        let marker = if addr == pc { ">" } else { " " };
        let line = egui::RichText::new(format!("{marker}0x{addr:04X}  {text}"))
            .monospace()
            .color(color);
        if ui
            .add(
                egui::Label::new(line)
                    .truncate()
                    .sense(egui::Sense::click()),
            )
            .clicked()
        {
            dbg.set_cursor_addr(addr);
        }
    });
}

fn registers(ui: &mut egui::Ui, state: &DebugState, core: &ConsoleCore) {
    let vm = &core.vm;
    ui.horizontal_wrapped(|ui| {
        for (i, val) in vm.get_registers().iter().enumerate() {
            let label = match state.dbg.reg_alias(i) {
                Some(alias) => format!("{}:{}", alias.to_uppercase(), val),
                None => format!("R{i}:{val}"),
            };
            ui.colored_label(theme::TEXT, label);
        }
        ui.colored_label(theme::DIM, format!("PC:0x{:04X}", vm.get_pc()));
        ui.colored_label(
            theme::DIM,
            format!("CAM:{},{}", vm.get_camera_x(), vm.get_camera_y()),
        );
        if vm.is_waiting() {
            ui.colored_label(theme::DIM, "WAIT");
        }
    });
    if let Some(fault) = vm.get_fault() {
        ui.colored_label(theme::ERROR, format!("FAULT: {fault:?}"));
    }
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

fn timeline(
    ui: &mut egui::Ui,
    state: &mut DebugState,
    core: &mut ConsoleCore,
    run_state: &mut RunState,
) {
    let len = state.dbg.states_len();
    ui.horizontal(|ui| {
        ui.colored_label(theme::DIM, "TIME");
        if len < 2 {
            ui.colored_label(theme::DIM, "no snapshots yet");
            return;
        }
        let mut idx = len - 1 - state.dbg.scrub_offset().min(len - 1);
        ui.spacing_mut().slider_width = (ui.available_width() - 70.0).max(60.0);
        let resp = ui.add(egui::Slider::new(&mut idx, 0..=len - 1).show_value(false));
        if resp.changed() {
            state.dbg.set_scrub_offset(len - 1 - idx);
            if let Some(snap) = state.dbg.current_scrub_snapshot() {
                core.vm.restore(&snap);
            }
            *run_state = RunState::Paused;
            state.on_break(core.vm.get_pc());
        }
        let off = state.dbg.scrub_offset();
        if off > 0 {
            ui.colored_label(theme::ACCENT, format!("-{off}"));
        } else {
            ui.colored_label(theme::OK, "LIVE");
        }
    });
}
