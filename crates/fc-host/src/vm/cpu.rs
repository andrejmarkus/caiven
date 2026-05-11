use crate::settings::REGISTER_COUNT;

pub struct Cpu {
    pub pc: usize,
    pub sp: usize,
    pub registers: [u16; REGISTER_COUNT],
}

impl Cpu {
    pub fn new() -> Self {
        Self {
            pc: 0,
            sp: 0,
            registers: [0; REGISTER_COUNT],
        }
    }

    pub fn get_registers(&self) -> &[u16] {
        &self.registers
    }

    pub fn get_registers_len(&self) -> usize {
        self.registers.len()
    }

    pub fn get_register_value(&self, index: usize) -> u16 {
        self.registers[index]
    }

    pub fn decrement_register_value(&mut self, index: usize, value: u16) {
        if index < self.registers.len() {
            self.registers[index] = self.registers[index].wrapping_sub(value);
        }
    }

    pub fn increment_register_value(&mut self, index: usize, value: u16) {
        if index < self.registers.len() {
            self.registers[index] = self.registers[index].wrapping_add(value);
        }
    }

    pub fn set_register(&mut self, index: usize, value: u16) {
        if index < self.registers.len() {
            self.registers[index] = value;
        }
    }

    pub fn set_pc(&mut self, address: usize) {
        self.pc = address;
    }

    pub fn get_pc(&self) -> usize {
        self.pc
    }

    pub fn set_sp(&mut self, address: usize) {
        self.sp = address;
    }

    pub fn get_sp(&self) -> usize {
        self.sp
    }

    pub fn shift_pc(&mut self, offset: isize) {
        if offset.is_negative() {
            self.pc = self.pc.saturating_sub(offset.wrapping_abs() as usize);
        } else {
            self.pc = self.pc.saturating_add(offset as usize);
        }
    }
}
