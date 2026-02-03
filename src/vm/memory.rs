pub struct Memory {
    ram: [u8; 256],
}

impl Memory {
    pub fn new() -> Self {
        Self { ram: [0; 256] }
    }

    pub fn read(&self, address: usize) -> u8 {
        self.ram[address]
    }

    pub fn write(&mut self, address: usize, value: u8) {
        self.ram[address] = value;
    }
}
