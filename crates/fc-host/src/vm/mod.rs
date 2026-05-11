pub mod audio;
pub mod camera;
pub mod context;
pub mod cpu;
pub mod memory;
pub mod palette;

pub use camera::*;
pub use context::*;
pub use palette::*;

use self::cpu::Cpu;
use self::memory::Memory;
use crate::isa::InstructionSet;
use crate::input::Input;
use crate::rendering::screen::ScreenLayer;
use crate::rendering::text::draw_text;
use crate::vm::Camera;
use crate::vm::Palette;
use crate::vm::audio::{NoiseChannel, Sound, SquareChannel};
use fc_core::{Color, Vec2};
use log::error;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VmFault {
    InvalidOpcode(u8),
    InvalidRegister(usize),
    MemoryOutOfBounds(usize),
}

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
}

impl Vm {
    pub fn new(instructions: Arc<InstructionSet>) -> Self {
        let mut cpu = Cpu::new();
        cpu.sp = crate::settings::MEMORY_SIZE;
        Self {
            cpu,
            program: Vec::new(),
            memory: Memory::new(),
            camera: Camera::new(Vec2::new(0, 0)),
            palette: Palette::new(),
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
        }
    }

    pub fn set_fault(&mut self, fault: VmFault) {
        error!("VM FAULT: {:?}", fault);
        self.fault = Some(fault);
        self.waiting = true;
    }

    pub fn get_fault(&self) -> Option<VmFault> {
        self.fault
    }

    pub fn get_sound(&self) -> Sound {
        self.sound.lock().unwrap().clone()
    }

    pub fn set_sound(&mut self, sound: Sound) {
        *self.sound.lock().unwrap() = sound;
    }

    pub fn get_sound_shared(&self) -> Arc<Mutex<Sound>> {
        self.sound.clone()
    }

    pub fn read_byte(&mut self) -> u8 {
        if self.cpu.pc >= self.program.len() {
            self.set_fault(VmFault::MemoryOutOfBounds(self.cpu.pc));
            return 0;
        }
        let byte = self.program[self.cpu.pc];
        self.cpu.pc += 1;
        byte
    }

    pub fn read_word(&mut self) -> u16 {
        let low = self.read_byte() as u16;
        let high = self.read_byte() as u16;
        low | (high << 8)
    }

    pub fn read_register_index(&mut self) -> usize {
        let index = self.read_byte() as usize;
        if index >= self.get_registers_len() {
            self.set_fault(VmFault::InvalidRegister(index));
            return 0;
        }
        index
    }

    pub fn read_register_value(&mut self) -> u16 {
        let index = self.read_register_index();
        self.get_register_value(index)
    }

    pub fn load_program(&mut self, source: &str) -> Result<(), fc_asm::AsmError> {
        let (program, source_map) = fc_asm::assemble_with_source_map(source)?;
        self.program = program;
        self.source_map = source_map;
        self.cpu.pc = 0;
        Ok(())
    }

    pub fn load_rom(&mut self, program: Vec<u8>) {
        self.source_map = fc_asm::generate_source_map(&program);
        self.program = program;
        self.cpu.pc = 0;
    }

    pub fn assemble(&self, source: &str) -> Result<Vec<u8>, fc_asm::AsmError> {
        fc_asm::assemble(source)
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

    pub fn get_registers_len(&self) -> usize {
        self.cpu.get_registers_len()
    }

    pub fn get_register_value(&self, index: usize) -> u16 {
        self.cpu.get_register_value(index)
    }

    pub fn get_source_map(&self) -> &fc_asm::SourceMap {
        &self.source_map
    }

    pub fn decrement_register_value(&mut self, index: usize, value: u16) {
        self.cpu.decrement_register_value(index, value);
    }

    pub fn increment_register_value(&mut self, index: usize, value: u16) {
        self.cpu.increment_register_value(index, value);
    }

    pub fn set_register(&mut self, index: usize, value: u16) {
        self.cpu.set_register(index, value);
    }

    pub fn get_palette_color(&self, index: usize) -> Color {
        self.palette.get_color(index)
    }

    pub fn set_palette_color(&mut self, index: usize, color: Color) {
        self.palette.set_color(index, color);
    }

    pub fn set_pc(&mut self, address: usize) {
        self.cpu.set_pc(address);
    }

    pub fn get_pc(&self) -> usize {
        self.cpu.get_pc()
    }

    pub fn set_sp(&mut self, address: usize) {
        self.cpu.set_sp(address);
    }

    pub fn get_sp(&self) -> usize {
        self.cpu.get_sp()
    }

    pub fn shift_pc(&mut self, offset: isize) {
        self.cpu.shift_pc(offset);
    }

    pub fn get_memory_length(&self) -> usize {
        self.memory.get_length()
    }

    pub fn read_memory(&mut self, address: usize) -> u8 {
        if address >= self.get_memory_length() {
            self.set_fault(VmFault::MemoryOutOfBounds(address));
            return 0;
        }
        self.memory.read(address)
    }

    pub fn get_camera_x(&self) -> u32 {
        self.camera.get_x()
    }

    pub fn get_camera_y(&self) -> u32 {
        self.camera.get_y()
    }

    pub fn set_camera_position(&mut self, x: u32, y: u32) {
        self.camera.set_position(x, y);
    }

    pub fn move_camera_by(&mut self, dx: i32, dy: i32) {
        self.camera.move_by(dx, dy);
    }

    pub fn is_waiting(&self) -> bool {
        self.waiting
    }

    pub fn pause(&mut self) {
        self.waiting = true;
    }

    pub fn draw_text(
        &self,
        layer: &mut ScreenLayer,
        text: &str,
        x: u32,
        y: u32,
        color_index: usize,
    ) {
        let color = self.palette.get_color(color_index);
        draw_text(layer, text, Vec2::new(x, y), color);
    }

    pub fn peek_memory(&self, address: usize) -> u8 {
        if address >= self.get_memory_length() {
            return 0;
        }
        self.memory.read(address)
    }

    pub fn write_memory(&mut self, address: usize, value: u8) {
        if address >= self.get_memory_length() {
            self.set_fault(VmFault::MemoryOutOfBounds(address));
            return;
        }
        self.memory.write(address, value)
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

    pub fn run_frame(&mut self, input: &Input, world: &mut ScreenLayer, ui: &mut ScreenLayer) {
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
            self.step(input, world, ui);
        }
    }

    pub fn step(&mut self, input: &Input, world: &mut ScreenLayer, ui: &mut ScreenLayer) {
        if self.fault.is_some() {
            return;
        }

        if self.program.is_empty() {
            self.waiting = true;
            return;
        }

        if self.cpu.pc >= self.program.len() {
            self.waiting = true;
            return;
        }

        let opcode = self.program[self.cpu.pc];

        let handler = {
            let instruction = self.instructions.get_by_opcode(opcode);
            if let Some(instr) = instruction {
                instr.execute
            } else {
                self.set_fault(VmFault::InvalidOpcode(opcode));
                return;
            }
        };

        self.cpu.pc += 1;
        let mut ctx = ExecutionContext::new(self, input, world, ui);
        (handler)(&mut ctx);
    }

    pub fn snapshot(&self) -> VmSnapshot {
        VmSnapshot {
            pc: self.cpu.pc,
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
        self.cpu.pc = snapshot.pc;
        for (i, val) in snapshot.registers.iter().enumerate() {
            self.cpu.set_register(i, *val);
        }
        self.memory
            .set_ram(snapshot.memory.clone().try_into().unwrap());
        self.camera
            .set_position(snapshot.camera_x, snapshot.camera_y);
        self.palette
            .set_colors(snapshot.palette.clone().try_into().unwrap());
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
