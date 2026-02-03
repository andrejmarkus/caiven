pub mod cpu;
pub mod memory;

use self::cpu::Cpu;
use self::memory::Memory;
use crate::assembler::Assembler;
use crate::input::Input;
use crate::instructions::InstructionSet;
use crate::screen::Screen;
use log::error;
use std::sync::Arc;

pub struct Vm {
    cpu: Cpu,
    program: Vec<u8>,
    memory: Memory,
    waiting: bool,
    instructions: Arc<InstructionSet>,
    assembler: Assembler,
}

impl Vm {
    pub fn new(instructions: Arc<InstructionSet>) -> Self {
        Self {
            cpu: Cpu::new(),
            program: Vec::new(),
            memory: Memory::new(),
            waiting: false,
            instructions: instructions.clone(),
            assembler: Assembler::new(instructions),
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

    pub fn set_pc(&mut self, address: usize) {
        self.cpu.set_pc(address);
    }

    pub fn shift_pc(&mut self, offset: isize) {
        self.cpu.shift_pc(offset);
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

    pub fn run_frame(&mut self, input: &Input, screen: &mut Screen) {
        self.waiting = false;

        while !self.waiting {
            self.step(input, screen);
        }
    }

    pub fn step(&mut self, input: &Input, screen: &mut Screen) {
        let opcode = self.program[self.cpu.pc];

        let instruction = self
            .instructions
            .get_by_opcode(opcode)
            .unwrap_or_else(|| panic!("Unknown opcode: 0x{:02X}", opcode));

        self.cpu.pc += 1;
        (instruction.execute)(self, input, screen);
    }
}
