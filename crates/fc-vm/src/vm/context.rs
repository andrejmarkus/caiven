use super::audio::Sound;
use super::camera::Camera;
use super::config::VmConfig;
use super::cpu::Cpu;
use super::fault::VmFault;
use super::memory::Memory;
use super::palette::Palette;
use super::sfx::{MusicPlayer, SfxPlayer};
use crate::input::Input;
use crate::rendering::font::Font;
use crate::rendering::screen::ScreenLayer;

pub struct ExecutionContext<'a> {
    pub cpu: &'a mut Cpu,
    pub mem: &'a mut Memory,
    pub palette: &'a mut Palette,
    pub camera: &'a mut Camera,
    pub sound: &'a mut Sound,
    pub sfx_player: &'a mut SfxPlayer,
    pub music_player: &'a mut MusicPlayer,
    pub program: &'a [u8],
    pub input: &'a Input,
    pub font: &'a Font,
    pub config: &'a VmConfig,
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

    pub fn read_dword(&mut self) -> Result<u32, VmFault> {
        let b0 = self.read_byte()? as u32;
        let b1 = self.read_byte()? as u32;
        let b2 = self.read_byte()? as u32;
        let b3 = self.read_byte()? as u32;
        Ok(b0 | (b1 << 8) | (b2 << 16) | (b3 << 24))
    }

    pub fn read_register_index(&mut self) -> Result<usize, VmFault> {
        let index = self.read_byte()? as usize;
        if index >= self.cpu.get_registers_len() {
            return Err(VmFault::InvalidRegister(index));
        }
        Ok(index)
    }

    pub fn read_register_value(&mut self) -> Result<u32, VmFault> {
        let index = self.read_register_index()?;
        Ok(self.cpu.get_register_value(index))
    }
}
