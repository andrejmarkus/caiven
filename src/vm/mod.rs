pub mod camera;
pub mod cpu;
pub mod memory;
pub mod palette;

pub use camera::*;
pub use palette::*;

use self::cpu::Cpu;
use self::memory::Memory;
use crate::assembler::{Assembler, AssemblyItem, InstructionSet, SourceMap, default_directive_set};
use crate::input::Input;
use crate::rendering::screen::ScreenLayer;
use crate::rendering::text::draw_text;
use crate::utils::Color;
use crate::utils::Vec2;
use crate::vm::Camera;
use crate::vm::Palette;
use log::error;
use std::sync::Arc;

pub struct Vm {
    cpu: Cpu,
    program: Vec<u8>,
    memory: Memory,
    camera: Camera,
    palette: Palette,
    instructions: Arc<InstructionSet>,
    assembler: Assembler,
    source_map: SourceMap,
    waiting: bool,
}

impl Vm {
    pub fn new(instructions: Arc<InstructionSet>) -> Self {
        Self {
            cpu: Cpu::new(),
            program: Vec::new(),
            memory: Memory::new(),
            camera: Camera::new(Vec2::new(0, 0)),
            palette: Palette::new(),
            instructions: instructions.clone(),
            assembler: Assembler::new(instructions, Arc::new(default_directive_set())),
            source_map: SourceMap::new(),
            waiting: false,
        }
    }

    pub fn load_program(&mut self, source: &str) {
        let (program, source_map) = self
            .assembler
            .assemble_with_source_map(source)
            .unwrap_or_else(|e| {
                error!("{}", e.to_string());
                std::process::exit(1);
            });
        self.program = program;
        self.source_map = source_map;
        self.cpu.pc = 0;
    }

    pub fn get_instruction_by_opcode(&self, opcode: u8) -> Option<&crate::assembler::Instruction> {
        self.instructions.get_by_opcode(opcode)
    }

    pub fn get_program(&self) -> &Vec<u8> {
        &self.program
    }

    pub fn get_pc(&self) -> usize {
        self.cpu.pc
    }

    pub fn get_registers(&self) -> &[u8] {
        self.cpu.get_registers()
    }

    pub fn get_registers_len(&self) -> usize {
        self.cpu.get_registers_len()
    }

    pub fn get_register_value(&self, index: usize) -> u8 {
        self.cpu.get_register_value(index)
    }

    pub fn set_register_value(&mut self, index: usize, value: u8) {
        self.cpu.set_register_value(index, value);
    }

    pub fn get_source_map(&self) -> &SourceMap {
        &self.source_map
    }

    pub fn decrement_register_value(&mut self, index: usize, value: u8) {
        self.cpu.decrement_register_value(index, value);
    }

    pub fn increment_register_value(&mut self, index: usize, value: u8) {
        self.cpu.increment_register_value(index, value);
    }

    pub fn set_register(&mut self, index: usize, value: u8) {
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

    pub fn shift_pc(&mut self, offset: isize) {
        self.cpu.shift_pc(offset);
    }

    pub fn get_memory_length(&self) -> usize {
        self.memory.get_length()
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

    pub fn read_memory(&self, address: usize) -> u8 {
        self.memory.read(address)
    }

    pub fn write_memory(&mut self, address: usize, value: u8) {
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
                    AssemblyItem::Instruction {
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
                    AssemblyItem::Directive { name, size } => {
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

    pub fn run_frame(&mut self, input: &Input, world: &mut ScreenLayer) {
        self.waiting = false;

        while !self.waiting {
            self.step(input, world);
        }
    }

    pub fn step(&mut self, input: &Input, world: &mut ScreenLayer) {
        let opcode = self.program[self.cpu.pc];

        let instruction = self
            .instructions
            .get_by_opcode(opcode)
            .unwrap_or_else(|| panic!("Unknown opcode: 0x{:02X}", opcode));

        self.cpu.pc += 1;
        (instruction.execute)(self, input, world);
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
    }
}

#[derive(Clone)]
pub struct VmSnapshot {
    pub pc: usize,
    pub registers: Vec<u8>,
    pub memory: Vec<u8>,
    pub camera_x: u32,
    pub camera_y: u32,
    pub palette: Vec<Color>,
    pub waiting: bool,
}
