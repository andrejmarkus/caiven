use crate::vm::Vm;

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
}
