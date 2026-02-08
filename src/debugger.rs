use crate::{
    rendering::{screen::ScreenLayer, text::draw_text},
    utils::{Color, Vec2},
    vm::Vm,
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
}

impl Debugger {
    pub fn new(enabled: bool) -> Self {
        Debugger {
            enabled,
            mode: DebugMode::Running,
            breakpoints: Vec::new(),
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
        println!("PC: {}", vm.get_pc());
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
        draw_text(screen, "DEBUG:", Vec2::new(2, 2), color);
        draw_text(
            screen,
            &format!("PC:{}", vm.get_pc()),
            Vec2::new(2, 10),
            color,
        );
        draw_text(
            screen,
            &format!("R0:{}", vm.get_registers()[0]),
            Vec2::new(2, 18),
            color,
        );
        draw_text(
            screen,
            &format!("R1:{}", vm.get_registers()[1]),
            Vec2::new(2, 26),
            color,
        );
        draw_text(
            screen,
            &format!("R2:{}", vm.get_registers()[2]),
            Vec2::new(2, 34),
            color,
        );
        draw_text(
            screen,
            &format!("R3:{}", vm.get_registers()[3]),
            Vec2::new(2, 42),
            color,
        );
        draw_text(
            screen,
            &format!("CAMERA:({},{})", vm.get_camera_x(), vm.get_camera_y()),
            Vec2::new(2, 50),
            color,
        );
        draw_text(
            screen,
            &format!("WAITING:{}", if vm.is_waiting() { "YES" } else { "NO" }),
            Vec2::new(2, 58),
            color,
        );
    }
}
