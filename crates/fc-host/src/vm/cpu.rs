pub struct Cpu {
    pc: usize,
    sp: usize,
    registers: Vec<u16>,
}

impl Cpu {
    pub fn new(register_count: usize) -> Self {
        Self {
            pc: 0,
            sp: 0,
            registers: vec![0; register_count],
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
}
