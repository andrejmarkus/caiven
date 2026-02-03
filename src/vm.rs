use crate::assembler::assemble;
use crate::input::Input;
use crate::instruction_set::InstructionSet;
use crate::screen::Screen;

pub struct Vm {
    pc: usize,
    program: Vec<u8>,
    registers: [u8; 4],
    ram: [u8; 256],
    waiting: bool,
    instructions: InstructionSet,
}

impl Vm {
    pub fn new(instructions: InstructionSet) -> Self {
        Self {
            pc: 0,
            program: Vec::new(),
            registers: [0; 4],
            ram: [0; 256],
            waiting: false,
            instructions,
        }
    }

    pub fn load_program(&mut self, source: &str) {
        self.program = assemble(source, &self.instructions);
    }

    pub fn get_program(&self) -> &Vec<u8> {
        &self.program
    }

    pub fn get_pc(&self) -> usize {
        self.pc
    }

    pub fn get_registers_len(&self) -> usize {
        self.registers.len()
    }

    pub fn get_register_value(&self, index: usize) -> u8 {
        self.registers[index]
    }

    pub fn decrement_register_value(&mut self, index: usize, value: u8) {
        if index < self.registers.len() {
            self.registers[index] = self.registers[index].wrapping_sub(value);
        }
    }

    pub fn increment_register_value(&mut self, index: usize, value: u8) {
        if index < self.registers.len() {
            self.registers[index] = self.registers[index].wrapping_add(value);
        }
    }

    pub fn set_register(&mut self, index: usize, value: u8) {
        if index < self.registers.len() {
            self.registers[index] = value;
        }
    }

    pub fn set_pc(&mut self, address: usize) {
        self.pc = address;
    }

    pub fn shift_pc(&mut self, offset: isize) {
        if offset.is_negative() {
            self.pc = self.pc.saturating_sub(offset.wrapping_abs() as usize);
        } else {
            self.pc = self.pc.saturating_add(offset as usize);
        }
    }

    pub fn pause(&mut self) {
        self.waiting = true;
    }

    pub fn read_memory(&self, address: usize) -> u8 {
        self.ram[address]
    }

    pub fn write_memory(&mut self, address: usize, value: u8) {
        self.ram[address] = value;
    }

    pub fn run_frame(&mut self, input: &Input, screen: &mut Screen) {
        self.waiting = false;

        while !self.waiting {
            self.step(input, screen);
        }
    }

    pub fn step(&mut self, input: &Input, screen: &mut Screen) {
        let opcode = self.program[self.pc];

        let instruction = self
            .instructions
            .get_by_opcode(opcode)
            .unwrap_or_else(|| panic!("Unknown opcode: 0x{:02X}", opcode));

        self.pc += 1;
        (instruction.execute)(self, input, screen);
    }
}
