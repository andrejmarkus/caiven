use super::fault::VmFault;

pub struct Memory {
    ram: Vec<u8>,
}

impl Memory {
    pub fn new(size: usize) -> Self {
        Self { ram: vec![0; size] }
    }

    pub fn get_length(&self) -> usize {
        self.ram.len()
    }

    pub fn get_ram(&self) -> &[u8] {
        &self.ram
    }

    pub fn set_ram(&mut self, ram: Vec<u8>) {
        self.ram = ram;
    }

    pub fn read(&self, address: usize) -> Result<u8, VmFault> {
        if address >= self.ram.len() {
            return Err(VmFault::MemoryOutOfBounds(address));
        }
        Ok(self.ram[address])
    }

    pub fn write(&mut self, address: usize, value: u8) -> Result<(), VmFault> {
        if address >= self.ram.len() {
            return Err(VmFault::MemoryOutOfBounds(address));
        }
        self.ram[address] = value;
        Ok(())
    }
}
