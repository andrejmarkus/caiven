pub mod audio;
pub mod camera;
pub mod config;
pub mod context;
pub mod cpu;
pub mod fault;
pub mod memory;
pub mod palette;

pub use camera::*;
pub use config::VmConfig;
pub use context::*;
pub use fault::VmFault;
pub use palette::*;

use self::cpu::Cpu;
use self::memory::Memory;
use crate::input::Input;
use crate::isa::InstructionSet;
use crate::rendering::font::Font;
use crate::rendering::screen::ScreenLayer;
use crate::vm::Camera;
use crate::vm::Palette;
use crate::vm::audio::{NoiseChannel, Sound, SquareChannel};
use fc_core::{Color, Vec2};
use log::error;
use std::sync::{Arc, Mutex};

pub struct Vm {
    cpu: Cpu,
    program: Vec<u8>,
    memory: Memory,
    camera: Camera,
    palette: Palette,
    instructions: Arc<InstructionSet>,
    source_map: fc_asm::SourceMap,
    sound: Arc<Mutex<Sound>>,
    waiting: bool,
    fault: Option<VmFault>,
    world: ScreenLayer,
    ui: ScreenLayer,
    config: VmConfig,
}

impl Vm {
    pub fn new(instructions: Arc<InstructionSet>, config: VmConfig) -> Self {
        let mut cpu = Cpu::new(config.register_count);
        cpu.set_sp(config.memory_size);
        Self {
            cpu,
            program: Vec::new(),
            memory: Memory::new(config.memory_size),
            camera: Camera::new(Vec2::new(0, 0)),
            palette: Palette::new(config.palette_size),
            instructions: instructions.clone(),
            sound: Arc::new(Mutex::new(Sound {
                square: SquareChannel {
                    enabled: false,
                    frequency: 440.0,
                    volume: 0.0,
                    duration: 0,
                },
                noise: NoiseChannel {
                    enabled: false,
                    volume: 0.0,
                    rate: 2000.0,
                    duration: 0,
                },
            })),
            source_map: fc_asm::SourceMap::new(),
            waiting: false,
            fault: None,
            world: ScreenLayer::new(config.width, config.height),
            ui: ScreenLayer::new(config.width, config.height),
            config,
        }
    }

    pub fn set_fault(&mut self, fault: VmFault) {
        error!("VM FAULT: {:?}", fault);
        self.fault = Some(fault);
        self.waiting = true;
    }

    pub fn get_sound_shared(&self) -> Arc<Mutex<Sound>> {
        self.sound.clone()
    }

    pub fn load_program(&mut self, source: &str) -> Result<(), fc_asm::AsmError> {
        let (program, source_map) = fc_asm::assemble_with_source_map(source)?;
        self.program = program;
        self.source_map = source_map;
        self.cpu.set_pc(0);
        Ok(())
    }

    pub fn load_rom(&mut self, program: Vec<u8>) {
        self.source_map = fc_asm::generate_source_map(&program);
        self.program = program;
        self.cpu.set_pc(0);
    }

    pub fn load_section_to_ram(&mut self, base: usize, data: &[u8]) {
        for (i, &byte) in data.iter().enumerate() {
            if let Err(e) = self.memory.write(base + i, byte) {
                log::error!("load_section_to_ram: write fault at {}: {:?}", base + i, e);
                break;
            }
        }
    }

    pub fn get_instruction_by_opcode(&self, opcode: u8) -> Option<&crate::isa::Instruction> {
        self.instructions.get_by_opcode(opcode)
    }

    pub fn get_program(&self) -> &Vec<u8> {
        &self.program
    }

    pub fn get_registers(&self) -> &[u16] {
        self.cpu.get_registers()
    }

    pub fn get_source_map(&self) -> &fc_asm::SourceMap {
        &self.source_map
    }

    pub fn get_memory_length(&self) -> usize {
        self.memory.get_length()
    }

    pub fn peek_memory(&self, address: usize) -> u8 {
        self.memory.read(address).unwrap_or(0)
    }

    pub fn get_camera_x(&self) -> u32 {
        self.camera.get_x()
    }

    pub fn get_camera_y(&self) -> u32 {
        self.camera.get_y()
    }

    pub fn is_waiting(&self) -> bool {
        self.waiting
    }

    pub fn get_pc(&self) -> usize {
        self.cpu.get_pc()
    }

    pub fn world_pixels(&self) -> &[u8] {
        self.world.get_pixels()
    }

    pub fn ui_pixels(&self) -> &[u8] {
        self.ui.get_pixels()
    }

    pub fn disassemble(&self, pc: usize) -> String {
        let program = self.get_program();
        let source_map = self.get_source_map();

        if pc >= program.len() {
            return "OUT OF BOUNDS".to_string();
        }

        let mut info_parts = Vec::new();
        if let Some(address_info) = source_map.get(pc) {
            for label in &address_info.labels {
                info_parts.push(format!("[{}]", label.to_uppercase()));
            }

            if let Some(item) = &address_info.item {
                match item {
                    fc_asm::ItemInfo::Instruction {
                        name: _,
                        opcode: _,
                        size,
                    } => {
                        let opcode = program[pc];
                        let instruction = self.get_instruction_by_opcode(opcode);
                        if let Some(instr) = instruction {
                            let end = (pc + size).min(program.len());
                            let bytes = &program[pc..end];
                            if bytes.len() < *size {
                                info_parts.push(format!("{} (INCOMPLETE)", instr.name));
                            } else {
                                info_parts.push((instr.debug_info)(bytes));
                            }
                        } else {
                            info_parts.push(format!("UNKNOWN OPCODE: 0X{:02X}", opcode));
                        }
                    }
                    fc_asm::ItemInfo::Directive { name, size } => {
                        let end = (pc + size).min(program.len());
                        let bytes = &program[pc..end];
                        let hex_string: Vec<String> =
                            bytes.iter().map(|b| format!("{:02X}", b)).collect();
                        info_parts.push(format!("{} {}", name, hex_string.join(" ")));
                    }
                }
            }
        }

        if info_parts.is_empty() {
            format!(".DB 0X{:02X}", program[pc])
        } else {
            info_parts.join(" ")
        }
    }

    pub fn run_frame(&mut self, input: &Input, font: &Font) {
        self.waiting = false;

        {
            let mut s = self.sound.lock().unwrap();
            if s.square.enabled && s.square.duration > 0 {
                s.square.duration -= 1;
                if s.square.duration == 0 {
                    s.square.enabled = false;
                }
            }
            if s.noise.enabled && s.noise.duration > 0 {
                s.noise.duration -= 1;
                if s.noise.duration == 0 {
                    s.noise.enabled = false;
                }
            }
        }

        while !self.waiting {
            self.step(input, font);
        }
    }

    pub fn step(&mut self, input: &Input, font: &Font) {
        if self.fault.is_some() {
            return;
        }

        if self.program.is_empty() {
            self.waiting = true;
            return;
        }

        let pc = self.cpu.get_pc();
        if pc >= self.program.len() {
            self.waiting = true;
            return;
        }

        let opcode = self.program[pc];

        let handler = {
            let instruction = self.instructions.get_by_opcode(opcode);
            if let Some(instr) = instruction {
                instr.execute
            } else {
                self.set_fault(VmFault::InvalidOpcode(opcode));
                return;
            }
        };

        self.cpu.set_pc(pc + 1);

        let sound_arc = Arc::clone(&self.sound);
        let mut sound_guard = sound_arc.lock().unwrap();
        let mut ctx = ExecutionContext {
            cpu: &mut self.cpu,
            mem: &mut self.memory,
            palette: &mut self.palette,
            camera: &mut self.camera,
            sound: &mut sound_guard,
            program: &self.program,
            input,
            font,
            config: &self.config,
            world: &mut self.world,
            ui: &mut self.ui,
            waiting: &mut self.waiting,
        };
        let result = (handler)(&mut ctx);
        drop(sound_guard);
        if let Err(fault) = result {
            self.set_fault(fault);
        }
    }

    pub fn snapshot(&self) -> VmSnapshot {
        VmSnapshot {
            pc: self.cpu.get_pc(),
            registers: self.cpu.get_registers().to_vec(),
            memory: self.memory.get_ram().to_vec(),
            camera_x: self.camera.get_x(),
            camera_y: self.camera.get_y(),
            palette: self.palette.get_colors().to_vec(),
            waiting: self.waiting,
            fault: self.fault,
        }
    }

    pub fn restore(&mut self, snapshot: &VmSnapshot) {
        self.cpu.set_pc(snapshot.pc);
        for (i, val) in snapshot.registers.iter().enumerate() {
            self.cpu.set_register(i, *val);
        }
        self.memory.set_ram(snapshot.memory.clone());
        self.camera
            .set_position(snapshot.camera_x, snapshot.camera_y);
        self.palette.set_colors(snapshot.palette.clone());
        self.waiting = snapshot.waiting;
        self.fault = snapshot.fault;
    }
}

#[derive(Clone)]
pub struct VmSnapshot {
    pub pc: usize,
    pub registers: Vec<u16>,
    pub memory: Vec<u8>,
    pub camera_x: u32,
    pub camera_y: u32,
    pub palette: Vec<Color>,
    pub waiting: bool,
    pub fault: Option<VmFault>,
}
