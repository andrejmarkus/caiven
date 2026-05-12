use crate::settings::MEMORY_SIZE;
use super::fault::VmFault;

pub struct Memory {
    ram: [u8; MEMORY_SIZE],
}

impl Memory {
    pub fn new() -> Self {
        Self {
            ram: [0; MEMORY_SIZE],
        }
    }

    pub fn get_length(&self) -> usize {
        self.ram.len()
    }

    pub fn get_ram(&self) -> &[u8] {
        &self.ram
    }

    pub fn set_ram(&mut self, ram: [u8; MEMORY_SIZE]) {
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
