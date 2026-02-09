use crate::{
    rendering::{screen::ScreenLayer, text::draw_text},
    settings::{MEMORY_BYTES_PER_PAGE, MEMORY_PAGE_SIZE, MEMORY_ROW_BYTES},
    utils::{Color, Vec2},
    vm::{Vm, VmSnapshot},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DebugMode {
    Running,
    Paused,
    Step,
}

pub struct Debugger {
    enabled: bool,
    mode: DebugMode,
    breakpoints: Vec<usize>,
    ram_page: usize,
    states: Vec<VmSnapshot>,
    max_states: usize,
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
        }
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn push_state(&mut self, snapshot: VmSnapshot) {
        if !self.enabled {
            return;
        }
        self.states.push(snapshot);
        if self.states.len() > self.max_states {
            self.states.remove(0);
        }
    }

    pub fn pop_state(&mut self) -> Option<VmSnapshot> {
        if !self.enabled {
            return None;
        }
        self.states.pop()
    }

    pub fn next_ram_page(&mut self) {
        if !self.enabled {
            return;
        }
        self.ram_page = (self.ram_page + 1) % crate::settings::MEMORY_PAGE_COUNT;
    }

    pub fn prev_ram_page(&mut self) {
        if !self.enabled {
            return;
        }
        if self.ram_page == 0 {
            self.ram_page = crate::settings::MEMORY_PAGE_COUNT - 1;
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

    pub fn add_breakpoint(&mut self, pc: usize) {
        if !self.enabled {
            return;
        }
        if !self.breakpoints.contains(&pc) {
            self.breakpoints.push(pc);
        }
    }

    pub fn check_breakpoint(&mut self, pc: usize) {
        if !self.enabled {
            return;
        }
        if self.breakpoints.contains(&pc) {
            self.mode = DebugMode::Paused;
        }
    }

    pub fn toggle_pause(&mut self) {
        if !self.enabled {
            return;
        }
        self.mode = match self.mode {
            DebugMode::Running => DebugMode::Paused,
            DebugMode::Paused => DebugMode::Running,
            DebugMode::Step => DebugMode::Running,
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

    pub fn dump_state(&self, vm: &Vm) {
        if !self.enabled {
            return;
        }
        println!("--- VM state ---");
        println!("PC: {} ({})", vm.get_pc(), vm.disassemble(vm.get_pc()));
        println!("Registers:");
        for (i, val) in vm.get_registers().iter().enumerate() {
            println!("R{}: {}", i, val);
        }
        println!("Camera: ({}, {})", vm.get_camera_x(), vm.get_camera_y());
        println!("Waiting: {}", if vm.is_waiting() { "YES" } else { "NO" });
        println!("----------------");
    }

    pub fn draw_overlay(&self, screen: &mut ScreenLayer, vm: &Vm) {
        if !self.enabled {
            return;
        }
        let color = Color::new_rgb(255, 255, 255);
        draw_text(
            screen,
            &format!("PC:{}", vm.get_pc()),
            Vec2::new(2, 2),
            color,
        );
        draw_text(
            screen,
            &format!("R0:{}", vm.get_registers()[0]),
            Vec2::new(2, 10),
            color,
        );
        draw_text(
            screen,
            &format!("R1:{}", vm.get_registers()[1]),
            Vec2::new(2, 18),
            color,
        );
        draw_text(
            screen,
            &format!("R2:{}", vm.get_registers()[2]),
            Vec2::new(2, 26),
            color,
        );
        draw_text(
            screen,
            &format!("R3:{}", vm.get_registers()[3]),
            Vec2::new(2, 34),
            color,
        );
        draw_text(
            screen,
            &format!("CAMERA:({},{})", vm.get_camera_x(), vm.get_camera_y()),
            Vec2::new(2, 42),
            color,
        );
        draw_text(
            screen,
            &format!("WAITING:{}", if vm.is_waiting() { "YES" } else { "NO" }),
            Vec2::new(2, 50),
            color,
        );
        draw_text(
            screen,
            &format!("STATES:{}", self.states.len()),
            Vec2::new(86, 2),
            color,
        );
        self.render_instruction_info(screen, vm, Vec2::new(2, 58), color);
        self.render_memory_page(screen, vm, Vec2::new(2, 66), color);
    }

    fn render_memory_page(&self, screen: &mut ScreenLayer, vm: &Vm, position: Vec2, color: Color) {
        if !self.enabled {
            return;
        }
        let start_address = self.ram_page * MEMORY_BYTES_PER_PAGE;
        draw_text(
            screen,
            &format!("RAM PAGE: {} (0X{:04X})", self.ram_page, start_address),
            Vec2::new(position.get_x(), position.get_y() + 4),
            color,
        );
        for row in 0..MEMORY_PAGE_SIZE {
            let addr = start_address + row * MEMORY_ROW_BYTES;
            if addr >= vm.get_memory_length() {
                break;
            }
            let mut line = format!("0X{:04X}:", addr);
            for col in 0..MEMORY_ROW_BYTES {
                let i = addr + col;
                if i < vm.get_memory_length() {
                    line.push_str(&format!(" {:02X}", vm.read_memory(i)));
                } else {
                    line.push_str(" --");
                }
            }
            draw_text(
                screen,
                &line,
                Vec2::new(position.get_x(), position.get_y() + 12 + row as u32 * 8),
                color,
            );
        }
    }

    fn render_instruction_info(
        &self,
        screen: &mut ScreenLayer,
        vm: &Vm,
        position: Vec2,
        color: Color,
    ) {
        if !self.enabled {
            return;
        }

        let info = vm.disassemble(vm.get_pc());

        draw_text(screen, &format!("INSTR:{}", info), position, color);
    }
}
