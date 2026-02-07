use crate::{
    rendering::{screen::ScreenLayer, text::draw_text},
    vm::Vm,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DebugMode {
    Running,
    Paused,
    Step,
}

pub struct Debugger {
    mode: DebugMode,
    breakpoints: Vec<usize>,
}

impl Debugger {
    pub fn new() -> Self {
        Debugger {
            mode: DebugMode::Running,
            breakpoints: Vec::new(),
        }
    }

    pub fn get_mode(&self) -> DebugMode {
        self.mode
    }

    pub fn add_breakpoint(&mut self, pc: usize) {
        if !self.breakpoints.contains(&pc) {
            self.breakpoints.push(pc);
        }
    }

    pub fn check_breakpoint(&mut self, pc: usize) {
        if self.breakpoints.contains(&pc) {
            self.mode = DebugMode::Paused;
        }
    }

    pub fn toggle_pause(&mut self) {
        self.mode = match self.mode {
            DebugMode::Running => DebugMode::Paused,
            DebugMode::Paused => DebugMode::Running,
            DebugMode::Step => DebugMode::Running,
        };
    }

    pub fn step(&mut self) {
        self.mode = DebugMode::Step;
    }

    pub fn pause(&mut self) {
        self.mode = DebugMode::Paused;
    }

    pub fn dump_state(&self, vm: &Vm) {
        println!("--- VM state ---");
        println!("PC: {}", vm.get_pc());
        println!("Registers:");
        for (i, val) in vm.get_registers().iter().enumerate() {
            println!("R{}: {}", i, val);
        }
        println!("Camera: x={}, y={}", vm.get_camera_x(), vm.get_camera_y());
        println!("waiting: {}", vm.is_waiting());
        println!("----------------");
    }

    pub fn draw_overlay(&self, screen: &mut ScreenLayer, vm: &Vm) {
        draw_text(
            screen,
            &format!("PC:{}", vm.get_pc()),
            2,
            2,
            [255, 255, 255],
        );
        draw_text(
            screen,
            &format!("R0:{}", vm.get_registers()[0]),
            2,
            10,
            [255, 255, 255],
        );
        draw_text(
            screen,
            &format!("R1:{}", vm.get_registers()[1]),
            2,
            18,
            [255, 255, 255],
        );
        draw_text(
            screen,
            &format!("R2:{}", vm.get_registers()[2]),
            2,
            26,
            [255, 255, 255],
        );
        draw_text(
            screen,
            &format!("R3:{}", vm.get_registers()[3]),
            2,
            34,
            [255, 255, 255],
        );
        draw_text(
            screen,
            &format!("CAMERA X {} Y{}", vm.get_camera_x(), vm.get_camera_y()),
            2,
            50,
            [255, 255, 255],
        );
        draw_text(
            screen,
            &format!("WAITING {}", vm.is_waiting()),
            2,
            58,
            [255, 255, 255],
        );
    }
}
