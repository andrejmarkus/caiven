pub mod audio;
pub mod camera;
pub mod config;
pub mod context;
pub mod cpu;
mod debug;
mod execution;
pub mod fault;
mod lua_exec;
pub mod memory;
pub mod palette;
pub mod sfx;

pub use camera::*;
pub use config::VmConfig;
pub use context::*;
pub use debug::VmSnapshot;
pub use fault::VmFault;
pub use lua_exec::{LuaRunOutcome, describe_lua_error};
pub use palette::*;

use self::cpu::Cpu;
use self::memory::Memory;
use self::sfx::{MusicPlayer, SfxPlayer};
use crate::isa::InstructionSet;
use crate::peripheral::{Peripheral, PeripheralRegistry};
use crate::rendering::screen::ScreenLayer;
use crate::vm::Camera;
use crate::vm::audio::{NoiseChannel, Sound, SquareChannel};
use fc_core::{Color, Vec2};
use log::error;
use std::sync::{Arc, Mutex};

/// Native table storage: table id-1 indexes the outer Vec; each table is an
/// insertion-ordered association list of (key, value). Lives outside guest
/// RAM so tables can grow without a fixed cap.
pub type TableStore = Vec<Vec<(u32, u32)>>;

pub struct Vm {
    cpu: Cpu,
    program: Vec<u8>,
    memory: Memory,
    tables: TableStore,
    camera: Camera,
    palette: Palette,
    instructions: Arc<InstructionSet>,
    source_map: fc_asm::SourceMap,
    fc_source_lines: Vec<String>,
    sound: Arc<Mutex<Sound>>,
    sfx_player: SfxPlayer,
    music_player: MusicPlayer,
    peripherals: PeripheralRegistry,
    frame_count: u32,
    waiting: bool,
    fault: Option<VmFault>,
    world: ScreenLayer,
    ui: ScreenLayer,
    config: VmConfig,
    script: Option<lua_exec::LuaScript>,
}

impl Vm {
    pub fn new(instructions: Arc<InstructionSet>, config: VmConfig) -> Self {
        let mut cpu = Cpu::new(config.register_count);
        cpu.set_sp(config.memory_size);
        Self {
            cpu,
            program: Vec::new(),
            memory: Memory::new(config.memory_size),
            tables: Vec::new(),
            camera: Camera::new(Vec2::new(0, 0)),
            palette: Palette::new(config.palette_size),
            instructions: instructions.clone(),
            sound: Arc::new(Mutex::new(Sound {
                square: SquareChannel {
                    enabled: false,
                    frequency: 440.0,
                    volume: 0.0,
                    duration: 0,
                },
                noise: NoiseChannel {
                    enabled: false,
                    volume: 0.0,
                    rate: 2000.0,
                    duration: 0,
                },
            })),
            source_map: fc_asm::SourceMap::new(),
            fc_source_lines: Vec::new(),
            sfx_player: SfxPlayer::new(),
            music_player: MusicPlayer::new(),
            peripherals: PeripheralRegistry::new(),
            frame_count: 0,
            waiting: false,
            fault: None,
            world: ScreenLayer::new(config.width, config.height),
            ui: ScreenLayer::new(config.width, config.height),
            config,
            script: None,
        }
    }

    pub fn register_peripheral(&mut self, p: impl Peripheral + 'static) {
        self.peripherals.register(p);
    }

    pub fn registered_peripheral_names(&self) -> Vec<&'static str> {
        self.peripherals.names()
    }

    pub fn set_fault(&mut self, fault: VmFault) {
        error!("VM FAULT: {:?}", fault);
        self.fault = Some(fault);
        self.waiting = true;
    }

    pub fn get_sound_shared(&self) -> Arc<Mutex<Sound>> {
        self.sound.clone()
    }

    pub fn load_program(&mut self, source: &str) -> Result<(), fc_asm::AsmError> {
        let (program, source_map) = fc_asm::assemble_with_source_map(source)?;
        self.program = program;
        self.source_map = source_map;
        self.cpu.set_pc(0);
        self.frame_count = 0;
        self.tables.clear();
        self.peripherals.init_all(&mut self.memory);
        Ok(())
    }

    pub fn load_rom(&mut self, program: Vec<u8>) {
        self.source_map = fc_asm::generate_source_map(&program);
        self.program = program;
        self.fc_source_lines.clear();
        self.cpu.set_pc(0);
        self.frame_count = 0;
        self.tables.clear();
        self.peripherals.init_all(&mut self.memory);
    }

    pub fn load_rom_with_source_map(&mut self, program: Vec<u8>, source_map: fc_asm::SourceMap) {
        self.program = program;
        self.source_map = source_map;
        self.fc_source_lines.clear();
        self.cpu.set_pc(0);
        self.frame_count = 0;
        self.tables.clear();
        self.peripherals.init_all(&mut self.memory);
    }

    pub fn set_fc_source(&mut self, src: &str) {
        self.fc_source_lines = src.lines().map(|l| l.to_string()).collect();
    }

    pub fn get_fc_source_line(&self, line: usize) -> Option<&str> {
        // source lines are 1-based in AST
        self.fc_source_lines
            .get(line.saturating_sub(1))
            .map(|s| s.as_str())
    }

    pub fn load_section_to_ram(&mut self, base: usize, data: &[u8]) {
        for (i, &byte) in data.iter().enumerate() {
            if let Err(e) = self.memory.write(base + i, byte) {
                log::error!("load_section_to_ram: write fault at {}: {:?}", base + i, e);
                break;
            }
        }
    }

    pub fn get_instruction_by_opcode(&self, opcode: u8) -> Option<&crate::isa::Instruction> {
        self.instructions.get_by_opcode(opcode)
    }

    pub fn get_program(&self) -> &[u8] {
        &self.program
    }

    pub fn get_registers(&self) -> &[u32] {
        self.cpu.get_registers()
    }

    pub fn get_source_map(&self) -> &fc_asm::SourceMap {
        &self.source_map
    }

    pub fn get_memory_length(&self) -> usize {
        self.memory.get_length()
    }

    pub fn peek_memory(&self, address: usize) -> u8 {
        self.memory.read(address).unwrap_or(0)
    }

    pub fn get_camera_x(&self) -> u32 {
        self.camera.get_x()
    }

    pub fn get_camera_y(&self) -> u32 {
        self.camera.get_y()
    }

    pub fn is_waiting(&self) -> bool {
        self.waiting
    }

    pub fn get_fault(&self) -> Option<VmFault> {
        self.fault
    }

    pub fn get_pc(&self) -> usize {
        self.cpu.get_pc()
    }

    pub fn world_pixels(&self) -> &[u8] {
        self.world.get_pixels()
    }

    pub fn ui_pixels(&self) -> &[u8] {
        self.ui.get_pixels()
    }

    pub fn get_palette(&self) -> &[Color] {
        self.palette.get_colors()
    }

    pub fn set_palette_color(&mut self, index: usize, color: Color) {
        self.palette.set_color(index, color);
    }

    pub fn set_palette_from_bytes(&mut self, bytes: &[u8]) {
        for i in 0..16.min(bytes.len() / 3) {
            let r = bytes[i * 3];
            let g = bytes[i * 3 + 1];
            let b = bytes[i * 3 + 2];
            self.palette.set_color(i, Color::new_rgb(r, g, b));
        }
    }

    pub fn poke_memory(&mut self, address: usize, value: u8) {
        let _ = self.memory.write(address, value);
    }

    pub fn start_sfx(&mut self, id: u8) {
        self.sfx_player.start(id);
    }

    pub fn stop_sfx(&mut self) {
        self.sfx_player.stop();
        if let Ok(mut s) = self.sound.try_lock() {
            s.square.enabled = false;
            s.noise.enabled = false;
        }
    }

    pub fn start_music(&mut self, pattern_id: u8) {
        self.music_player.start(pattern_id);
    }

    pub fn stop_music(&mut self) {
        self.music_player.stop();
        if let Ok(mut s) = self.sound.try_lock() {
            s.square.enabled = false;
            s.noise.enabled = false;
        }
    }

    pub fn sfx_player(&self) -> &SfxPlayer {
        &self.sfx_player
    }

    pub fn music_player(&self) -> &MusicPlayer {
        &self.music_player
    }

    pub fn set_music_loop(&mut self, on: bool) {
        self.music_player.loop_on = on;
    }
}
