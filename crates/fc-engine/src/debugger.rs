use fc_core::{Color, Vec2};
use fc_vm::rendering::{font::Font, screen::ScreenLayer, text::draw_text};
use fc_vm::settings::{MEMORY_BYTES_PER_PAGE, MEMORY_PAGE_COUNT};
use fc_vm::vm::Vm;
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
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct FcDbgFile {
    #[serde(default)]
    breakpoints: Vec<usize>,
}

/// Breakpoint/pause-state model, shared between the raw in-game debug
/// overlay ([`crate::app::App`]) and FC Studio's debugger panel. Line
/// breakpoints, `.fcdbg` persistence and pause/step/run state live here;
/// egui rendering for Studio is in `studio::debug_panel`, ASCII rendering
/// for the raw overlay is below.
pub struct Debugger {
    enabled: bool,
    mode: DebugMode,
    breakpoints: Vec<usize>,
    ram_page: usize,
    cursor_addr: usize,
    fcdbg_path: Option<PathBuf>,
}

impl Debugger {
    pub fn new(enabled: bool) -> Self {
        Debugger {
            enabled,
            mode: DebugMode::Running,
            breakpoints: Vec::new(),
            ram_page: 0,
            cursor_addr: 0,
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
    }

    fn save_fcdbg(&self) {
        let Some(path) = &self.fcdbg_path else { return };
        let file = FcDbgFile {
            breakpoints: self.breakpoints.clone(),
        };
        if let Ok(text) = toml::to_string(&file) {
            let _ = std::fs::write(path, text);
        }
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

    pub fn toggle_pause(&mut self) {
        if !self.enabled {
            return;
        }
        self.mode = match self.mode {
            DebugMode::Running => DebugMode::Paused,
            DebugMode::Paused | DebugMode::Step => DebugMode::Running,
        };
    }

    pub fn step(&mut self) {
        if !self.enabled {
            return;
        }
        self.mode = DebugMode::Step;
    }

    pub fn pause(&mut self) {
        if !self.enabled {
            return;
        }
        self.mode = DebugMode::Paused;
    }

    /// Toggles a breakpoint on a Lua source line, set from the code editor's
    /// gutter.
    pub fn toggle_line_breakpoint(&mut self, line: usize) {
        if let Some(pos) = self.breakpoints.iter().position(|&a| a == line) {
            self.breakpoints.remove(pos);
        } else {
            self.breakpoints.push(line);
        }
        self.save_fcdbg();
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn breakpoints(&self) -> &[usize] {
        &self.breakpoints
    }

    pub fn cursor_addr(&self) -> usize {
        self.cursor_addr
    }

    pub fn set_cursor_addr(&mut self, addr: usize) {
        self.cursor_addr = addr;
    }

    /// Handle a mouse click on the debug overlay's status/RAM-nav rows.
    /// Returns what the caller must do next.
    pub fn handle_click(&mut self, x: u32, y: u32) -> DebugClickAction {
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

        // RAM page nav buttons (y=16..23 — header row; <  at x=96, > at x=112)
        if (16..24).contains(&y) {
            if (96..112).contains(&x) {
                self.prev_ram_page();
            } else if x >= 112 {
                self.next_ram_page();
            }
        }

        DebugClickAction::None
    }

    /// Minimal status bar for when debugger is enabled but the game is
    /// running.
    pub fn draw_status_bar(&self, screen: &mut ScreenLayer, vm: &Vm, font: &Font) {
        if !self.enabled {
            return;
        }
        self.draw_status(screen, vm, font, "RUN");
    }

    /// A cart has no addressable program/registers to disassemble or a
    /// timeline to scrub (mlua's interpreter state isn't cheaply
    /// snapshotable) — this in-game overlay is a quick-glance HUD, so it
    /// just shows run state and any fault; use FC Studio's debugger panel
    /// for line breakpoints and globals inspection.
    fn draw_status(&self, screen: &mut ScreenLayer, vm: &Vm, font: &Font, mode: &str) {
        let cyan = Color::new_rgb(0, 200, 255);
        let red = Color::new_rgb(255, 64, 64);
        draw_text(font, screen, &format!("LUA {mode}"), Vec2::new(0, 0), cyan);
        if let Some(fault) = vm.get_fault() {
            draw_text(font, screen, &format!("{fault:?}"), Vec2::new(0, 8), red);
        }
        self.render_memory_page(font, screen, vm, Vec2::new(0, 16), cyan);
    }

    pub fn draw_overlay(&self, screen: &mut ScreenLayer, vm: &Vm, font: &Font) {
        if !self.enabled {
            return;
        }
        let mode = match self.mode {
            DebugMode::Running => "RUN",
            DebugMode::Paused => "PAUSE",
            DebugMode::Step => "STEP",
        };
        self.draw_status(screen, vm, font, mode);
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
