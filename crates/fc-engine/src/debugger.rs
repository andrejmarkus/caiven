use fc_core::{Color, Vec2};
use fc_vm::rendering::{font::Font, screen::ScreenLayer, text::draw_text};
use fc_vm::settings::{MEMORY_BYTES_PER_PAGE, MEMORY_PAGE_COUNT};
use fc_vm::vm::{Vm, VmSnapshot};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DebugMode {
    Running,
    Paused,
    Step,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DebugClickAction {
    None,
    TogglePause,
    Step,
    RestoreScrub,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct FcDbgFile {
    #[serde(default)]
    breakpoints: Vec<usize>,
    #[serde(default)]
    r0: Option<String>,
    #[serde(default)]
    r1: Option<String>,
    #[serde(default)]
    r2: Option<String>,
    #[serde(default)]
    r3: Option<String>,
}

pub struct Debugger {
    enabled: bool,
    mode: DebugMode,
    breakpoints: Vec<usize>,
    ram_page: usize,
    states: Vec<VmSnapshot>,
    max_states: usize,
    cursor_addr: usize,
    scrub_offset: usize,
    reg_aliases: [Option<String>; 4],
    fcdbg_path: Option<PathBuf>,
}

impl Debugger {
    pub fn new(enabled: bool) -> Self {
        Debugger {
            enabled,
            mode: DebugMode::Running,
            breakpoints: Vec::new(),
            ram_page: 0,
            states: Vec::new(),
            max_states: 100,
            cursor_addr: 0,
            scrub_offset: 0,
            reg_aliases: Default::default(),
            fcdbg_path: None,
        }
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn set_fcdbg_path(&mut self, path: PathBuf) {
        self.fcdbg_path = Some(path);
        self.load_fcdbg();
    }

    fn load_fcdbg(&mut self) {
        let Some(path) = &self.fcdbg_path else { return };
        let Ok(text) = std::fs::read_to_string(path) else {
            return;
        };
        let Ok(file) = toml::from_str::<FcDbgFile>(&text) else {
            return;
        };
        self.breakpoints = file.breakpoints;
        self.reg_aliases[0] = file.r0;
        self.reg_aliases[1] = file.r1;
        self.reg_aliases[2] = file.r2;
        self.reg_aliases[3] = file.r3;
    }

    fn save_fcdbg(&self) {
        let Some(path) = &self.fcdbg_path else { return };
        let file = FcDbgFile {
            breakpoints: self.breakpoints.clone(),
            r0: self.reg_aliases[0].clone(),
            r1: self.reg_aliases[1].clone(),
            r2: self.reg_aliases[2].clone(),
            r3: self.reg_aliases[3].clone(),
        };
        if let Ok(text) = toml::to_string(&file) {
            let _ = std::fs::write(path, text);
        }
    }

    pub fn push_state(&mut self, snapshot: VmSnapshot) {
        if !self.enabled {
            return;
        }
        self.states.push(snapshot);
        if self.states.len() > self.max_states {
            self.states.remove(0);
        }
        self.scrub_offset = 0;
    }

    pub fn next_ram_page(&mut self) {
        if !self.enabled {
            return;
        }
        self.ram_page = (self.ram_page + 1) % MEMORY_PAGE_COUNT;
    }

    pub fn prev_ram_page(&mut self) {
        if !self.enabled {
            return;
        }
        if self.ram_page == 0 {
            self.ram_page = MEMORY_PAGE_COUNT - 1;
        } else {
            self.ram_page -= 1;
        }
    }

    pub fn get_mode(&self) -> DebugMode {
        if !self.enabled {
            return DebugMode::Running;
        }
        self.mode
    }

    pub fn check_breakpoint(&mut self, pc: usize) {
        if !self.enabled {
            return;
        }
        if self.breakpoints.contains(&pc) {
            self.mode = DebugMode::Paused;
            self.cursor_addr = pc;
        }
    }

    pub fn toggle_pause(&mut self, vm_pc: usize) {
        if !self.enabled {
            return;
        }
        self.mode = match self.mode {
            DebugMode::Running => {
                self.cursor_addr = vm_pc;
                DebugMode::Paused
            }
            DebugMode::Paused | DebugMode::Step => {
                self.scrub_offset = 0;
                DebugMode::Running
            }
        };
    }

    pub fn step(&mut self) {
        if !self.enabled {
            return;
        }
        self.mode = DebugMode::Step;
    }

    pub fn pause(&mut self, vm_pc: usize) {
        if !self.enabled {
            return;
        }
        self.mode = DebugMode::Paused;
        self.cursor_addr = vm_pc;
    }

    pub fn cursor_up(&mut self, vm: &Vm) {
        if !self.enabled {
            return;
        }
        let addrs = vm.get_source_map().sorted_addresses();
        if addrs.is_empty() {
            self.cursor_addr = self.cursor_addr.saturating_sub(1);
            return;
        }
        let pos = addrs.partition_point(|&a| a < self.cursor_addr);
        if pos > 0 {
            self.cursor_addr = addrs[pos - 1];
        }
    }

    pub fn cursor_down(&mut self, vm: &Vm) {
        if !self.enabled {
            return;
        }
        let addrs = vm.get_source_map().sorted_addresses();
        if addrs.is_empty() {
            self.cursor_addr = self.cursor_addr.saturating_add(1);
            return;
        }
        let pos = addrs.partition_point(|&a| a <= self.cursor_addr);
        if pos < addrs.len() {
            self.cursor_addr = addrs[pos];
        }
    }

    pub fn toggle_bp_at_cursor(&mut self) {
        if !self.enabled {
            return;
        }
        if let Some(pos) = self.breakpoints.iter().position(|&a| a == self.cursor_addr) {
            self.breakpoints.remove(pos);
        } else {
            self.breakpoints.push(self.cursor_addr);
        }
        self.save_fcdbg();
    }

    pub fn scrub_back(&mut self) {
        if !self.enabled || self.states.is_empty() {
            return;
        }
        self.scrub_offset = (self.scrub_offset + 1).min(self.states.len() - 1);
    }

    pub fn scrub_forward(&mut self) {
        if !self.enabled {
            return;
        }
        if self.scrub_offset > 0 {
            self.scrub_offset -= 1;
        }
    }

    pub fn current_scrub_snapshot(&self) -> Option<VmSnapshot> {
        if self.states.is_empty() {
            return None;
        }
        let idx = self.states.len().saturating_sub(1 + self.scrub_offset);
        self.states.get(idx).cloned()
    }

    pub fn dump_state(&self, vm: &Vm) {
        if !self.enabled {
            return;
        }
        println!("--- VM state ---");
        println!(
            "PC: 0x{:04X} ({})",
            vm.get_pc(),
            vm.disassemble(vm.get_pc())
        );
        println!("Registers:");
        for (i, val) in vm.get_registers().iter().enumerate() {
            let label = match &self.reg_aliases[i] {
                Some(alias) => format!("R{}({})", i, alias),
                None => format!("R{}", i),
            };
            println!("  {}: {}", label, val);
        }
        println!("Camera: ({}, {})", vm.get_camera_x(), vm.get_camera_y());
        println!("Waiting: {}", if vm.is_waiting() { "YES" } else { "NO" });
        if !self.breakpoints.is_empty() {
            let bps: Vec<String> = self
                .breakpoints
                .iter()
                .map(|a| format!("0x{:04X}", a))
                .collect();
            println!("Breakpoints: {}", bps.join(", "));
        }
        println!("States: {}/{}", self.states.len(), self.max_states);
        if self.scrub_offset > 0 {
            println!("Scrubbing: {} back from latest", self.scrub_offset);
        }
        println!("----------------");
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Handle a mouse click (or drag) on the debug overlay. Returns what the caller must do next.
    pub fn handle_click(&mut self, x: u32, y: u32, vm: &Vm) -> DebugClickAction {
        if !self.enabled {
            return DebugClickAction::None;
        }

        // Status line buttons (y=0..7)
        if y < 8 {
            if (88..104).contains(&x) {
                return DebugClickAction::TogglePause;
            }
            if (104..120).contains(&x) && self.mode == DebugMode::Paused {
                return DebugClickAction::Step;
            }
            return DebugClickAction::None;
        }

        // Disasm rows (y=8..47, 5 rows × 8px) — only interactive when paused/stepping
        if (8..48).contains(&y) && self.mode != DebugMode::Running {
            let row = ((y - 8) / 8) as usize;
            let addrs = build_disasm_window(vm, self.cursor_addr, 5);
            if let Some(&addr) = addrs.get(row) {
                if x < 8 {
                    // Gutter click: toggle breakpoint at this address
                    self.cursor_addr = addr;
                    self.toggle_bp_at_cursor();
                } else {
                    // Row click: move cursor here
                    self.cursor_addr = addr;
                }
            }
            return DebugClickAction::None;
        }

        // Timeline bar (y=70..72) — drag-friendly, works in all modes
        if (70..73).contains(&y) && !self.states.is_empty() {
            let idx = (x as usize * self.states.len()) / 128;
            let idx = idx.min(self.states.len() - 1);
            self.scrub_offset = self.states.len().saturating_sub(1 + idx);
            return DebugClickAction::RestoreScrub;
        }

        // RAM page nav buttons (y=80..87 — header row; <  at x=96, > at x=112)
        if (80..88).contains(&y) {
            if (96..112).contains(&x) {
                self.prev_ram_page();
            } else if x >= 112 {
                self.next_ram_page();
            }
            return DebugClickAction::None;
        }

        DebugClickAction::None
    }

    /// Minimal status bar for when debugger is enabled but VM is running.
    pub fn draw_status_bar(&self, screen: &mut ScreenLayer, vm: &Vm, font: &Font) {
        if !self.enabled {
            return;
        }
        let cyan = Color::new_rgb(0, 200, 255);
        let yellow = Color::new_rgb(255, 220, 0);
        let pc = vm.get_pc();
        let status = format!("RUN PC:0X{:04X} S:{}", pc, self.states.len());
        draw_text(font, screen, &status, Vec2::new(0, 0), cyan);
        draw_text(font, screen, "PSE", Vec2::new(88, 0), yellow);
    }

    pub fn draw_overlay(&self, screen: &mut ScreenLayer, vm: &Vm, font: &Font) {
        if !self.enabled {
            return;
        }

        let white = Color::new_rgb(255, 255, 255);
        let green = Color::new_rgb(64, 255, 64);
        let yellow = Color::new_rgb(255, 220, 0);
        let red = Color::new_rgb(255, 64, 64);
        let cyan = Color::new_rgb(0, 200, 255);
        let gray = Color::new_rgb(140, 140, 140);

        let pc = vm.get_pc();

        let mode_str = match self.mode {
            DebugMode::Running => "RUN",
            DebugMode::Paused => "PAUSE",
            DebugMode::Step => "STEP",
        };
        let scrub_indicator = if self.scrub_offset > 0 {
            format!("-{}", self.scrub_offset)
        } else {
            String::new()
        };
        let status = format!(
            "{} PC:0X{:04X} S:{}{}",
            mode_str,
            pc,
            self.states.len(),
            scrub_indicator
        );
        draw_text(font, screen, &status, Vec2::new(0, 0), cyan);

        // Clickable buttons on status line: [PSE/RUN] [STP]
        let pause_label = if self.mode == DebugMode::Running {
            "RUN"
        } else {
            "PSE"
        };
        draw_text(font, screen, pause_label, Vec2::new(88, 0), yellow);
        if self.mode == DebugMode::Paused {
            draw_text(font, screen, "STP", Vec2::new(104, 0), green);
        }

        let disasm_addrs = build_disasm_window(vm, self.cursor_addr, 5);
        for (i, &addr) in disasm_addrs.iter().enumerate() {
            let y = (i as u32 + 1) * 8;
            let is_bp = self.breakpoints.contains(&addr);
            let is_cursor = addr == self.cursor_addr;
            let is_pc = addr == pc;

            let bp_char = if is_bp { "*" } else { " " };
            let bp_color = if is_bp { red } else { gray };
            draw_text(font, screen, bp_char, Vec2::new(0, y), bp_color);

            let cursor_char = if is_cursor { ">" } else { " " };
            let cursor_color = if is_cursor { yellow } else { gray };
            draw_text(font, screen, cursor_char, Vec2::new(4, y), cursor_color);

            let text_color = if is_pc { green } else { white };
            let addr_str = format!("0X{:04X}", addr);
            draw_text(font, screen, &addr_str, Vec2::new(8, y), text_color);

            // If this address has an fc-lang source line, show it; else fall back to disasm
            let text: String = vm
                .get_source_map()
                .get(addr)
                .and_then(|info| info.src_line)
                .and_then(|ln| vm.get_fc_source_line(ln))
                .map(|s| s.trim().to_string())
                .unwrap_or_else(|| vm.disassemble(addr));
            let truncated: String = text.chars().take(18).collect();
            draw_text(font, screen, &truncated, Vec2::new(36, y), text_color);
        }

        let regs = vm.get_registers();
        for row in 0..2usize {
            let y = (6 + row as u32) * 8;
            for col in 0..2usize {
                let ri = row * 2 + col;
                let rx = col as u32 * 64;
                let val = regs.get(ri).copied().unwrap_or(0);
                let label = match &self.reg_aliases[ri] {
                    Some(alias) => {
                        let short: String =
                            alias.chars().take(3).collect::<String>().to_uppercase();
                        format!("{}:{}", short, val)
                    }
                    None => format!("R{}:{}", ri, val),
                };
                draw_text(font, screen, &label, Vec2::new(rx, y), white);
            }
        }

        let cam_str = format!(
            "CAM:{},{} WT:{}",
            vm.get_camera_x(),
            vm.get_camera_y(),
            if vm.is_waiting() { "Y" } else { "N" }
        );
        draw_text(font, screen, &cam_str, Vec2::new(0, 64), gray);

        self.render_timeline(screen, 70);
        self.render_memory_page(font, screen, vm, Vec2::new(0, 80), gray);
    }

    fn render_timeline(&self, screen: &mut ScreenLayer, y_start: u32) {
        let filled_color = Color::new_rgb(40, 100, 220);
        let empty_color = Color::new_rgb(20, 30, 60);
        let scrub_color = Color::new_rgb(255, 220, 0);

        let bar_width = 128u32;
        let filled = if self.max_states > 0 && !self.states.is_empty() {
            (self.states.len() as u32 * bar_width) / self.max_states as u32
        } else {
            0
        };

        let scrub_x = if self.scrub_offset > 0 && !self.states.is_empty() {
            let live_idx = self.states.len().saturating_sub(1 + self.scrub_offset);
            Some((live_idx as u32 * bar_width) / self.states.len() as u32)
        } else {
            None
        };

        for y_off in 0..3u32 {
            let y = y_start + y_off;
            for x in 0..bar_width {
                let color = if Some(x) == scrub_x {
                    scrub_color
                } else if x < filled {
                    filled_color
                } else {
                    empty_color
                };
                screen.set_pixel(Vec2::new(x, y), color);
            }
        }
    }

    fn render_memory_page(
        &self,
        font: &Font,
        screen: &mut ScreenLayer,
        vm: &Vm,
        position: Vec2,
        color: Color,
    ) {
        let start = self.ram_page * MEMORY_BYTES_PER_PAGE;
        draw_text(
            font,
            screen,
            &format!("RAM 0X{:04X}", start),
            position,
            color,
        );
        draw_text(
            font,
            screen,
            "<",
            Vec2::new(position.get_x() + 96, position.get_y()),
            color,
        );
        draw_text(
            font,
            screen,
            ">",
            Vec2::new(position.get_x() + 112, position.get_y()),
            color,
        );
        for row in 0..3usize {
            let addr = start + row * 8;
            if addr >= vm.get_memory_length() {
                break;
            }
            let mut line = String::new();
            for col in 0..8usize {
                let i = addr + col;
                if i < vm.get_memory_length() {
                    if col > 0 {
                        line.push(' ');
                    }
                    line.push_str(&format!("{:02X}", vm.peek_memory(i)));
                }
            }
            draw_text(
                font,
                screen,
                &line,
                Vec2::new(position.get_x(), position.get_y() + 8 + row as u32 * 8),
                color,
            );
        }
    }
}

fn build_disasm_window(vm: &Vm, center: usize, count: usize) -> Vec<usize> {
    let addrs = vm.get_source_map().sorted_addresses();
    if addrs.is_empty() {
        let start = center.saturating_sub(count / 2);
        return (start..start + count)
            .filter(|&a| a < vm.get_program().len())
            .collect();
    }
    let idx = match addrs.binary_search(&center) {
        Ok(i) => i,
        Err(i) => i.saturating_sub(1),
    };
    let half = count / 2;
    let start = idx.saturating_sub(half);
    let end = (start + count).min(addrs.len());
    let start = end.saturating_sub(count).min(start);
    addrs[start..end].to_vec()
}
