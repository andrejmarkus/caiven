pub mod camera;
pub mod cpu;
pub mod memory;
pub mod palette;

pub use camera::*;
pub use palette::*;

use self::cpu::Cpu;
use self::memory::Memory;
use crate::assembler::Assembler;
use crate::assembler::directives::default_directive_set;
use crate::input::Input;
use crate::instructions::InstructionSet;
use crate::rendering::screen::ScreenLayer;
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
    waiting: bool,
}

impl Vm {
    pub fn new(instructions: Arc<InstructionSet>) -> Self {
        Self {
            cpu: Cpu::new(),
            program: Vec::new(),
            memory: Memory::new(),
            camera: Camera::new(),
            palette: Palette::new(),
            instructions: instructions.clone(),
            assembler: Assembler::new(instructions, Arc::new(default_directive_set())),
            waiting: false,
        }
    }

    pub fn load_program(&mut self, source: &str) {
        self.program = self.assembler.assemble(source).unwrap_or_else(|e| {
            error!("{}", e.to_string());
            std::process::exit(1);
        });
        self.cpu.pc = 0;
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

    pub fn decrement_register_value(&mut self, index: usize, value: u8) {
        self.cpu.decrement_register_value(index, value);
    }

    pub fn increment_register_value(&mut self, index: usize, value: u8) {
        self.cpu.increment_register_value(index, value);
    }

    pub fn set_register(&mut self, index: usize, value: u8) {
        self.cpu.set_register(index, value);
    }

    pub fn get_palette_color(&self, index: usize) -> [u8; 3] {
        self.palette.get_color(index)
    }

    pub fn set_palette_color(&mut self, index: usize, r: u8, g: u8, b: u8) {
        self.palette.set_color(index, [r, g, b]);
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

    pub fn read_memory(&self, address: usize) -> u8 {
        self.memory.read(address)
    }

    pub fn write_memory(&mut self, address: usize, value: u8) {
        self.memory.write(address, value)
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
}
