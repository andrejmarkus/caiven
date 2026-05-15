pub struct Cpu {
    pc: usize,
    sp: usize,
    registers: Vec<u32>,
}

impl Cpu {
    pub fn new(register_count: usize) -> Self {
        Self {
            pc: 0,
            sp: 0,
            registers: vec![0u32; register_count],
        }
    }

    pub fn get_registers(&self) -> &[u32] {
        &self.registers
    }

    pub fn get_registers_len(&self) -> usize {
        self.registers.len()
    }

    pub fn get_register_value(&self, index: usize) -> u32 {
        self.registers.get(index).copied().unwrap_or(0)
    }

    pub fn decrement_register_value(&mut self, index: usize, value: u32) {
        if index < self.registers.len() {
            self.registers[index] = self.registers[index].wrapping_sub(value);
        }
    }

    pub fn increment_register_value(&mut self, index: usize, value: u32) {
        if index < self.registers.len() {
            self.registers[index] = self.registers[index].wrapping_add(value);
        }
    }

    pub fn set_register(&mut self, index: usize, value: u32) {
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
