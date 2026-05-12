use crate::input::Input;
use crate::rendering::screen::ScreenLayer;
use super::audio::Sound;
use super::camera::Camera;
use super::cpu::Cpu;
use super::fault::VmFault;
use super::memory::Memory;
use super::palette::Palette;

pub struct ExecutionContext<'a> {
    pub cpu: &'a mut Cpu,
    pub mem: &'a mut Memory,
    pub palette: &'a mut Palette,
    pub camera: &'a mut Camera,
    pub sound: &'a mut Sound,
    pub program: &'a [u8],
    pub input: &'a Input,
    pub world: &'a mut ScreenLayer,
    pub ui: &'a mut ScreenLayer,
    pub waiting: &'a mut bool,
}

impl<'a> ExecutionContext<'a> {
    pub fn read_byte(&mut self) -> Result<u8, VmFault> {
        let pc = self.cpu.get_pc();
        if pc >= self.program.len() {
            return Err(VmFault::MemoryOutOfBounds(pc));
        }
        let byte = self.program[pc];
        self.cpu.set_pc(pc + 1);
        Ok(byte)
    }

    pub fn read_word(&mut self) -> Result<u16, VmFault> {
        let low = self.read_byte()? as u16;
        let high = self.read_byte()? as u16;
        Ok(low | (high << 8))
    }

    pub fn read_register_index(&mut self) -> Result<usize, VmFault> {
        let index = self.read_byte()? as usize;
        if index >= self.cpu.get_registers_len() {
            return Err(VmFault::InvalidRegister(index));
        }
        Ok(index)
    }

    pub fn read_register_value(&mut self) -> Result<u16, VmFault> {
        let index = self.read_register_index()?;
        Ok(self.cpu.get_register_value(index))
    }
}
