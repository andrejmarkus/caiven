pub mod audio;
pub mod camera;
pub mod config;
pub mod context;
pub mod cpu;
pub mod fault;
pub mod memory;
pub mod palette;
pub mod sfx;

pub use camera::*;
pub use config::VmConfig;
pub use context::*;
pub use fault::VmFault;
pub use palette::*;

use self::cpu::Cpu;
use self::memory::Memory;
use self::sfx::{MusicPlayer, SfxPlayer, note_to_freq};
use crate::input::Input;
use crate::isa::InstructionSet;
use crate::peripheral::{Peripheral, PeripheralRegistry};
use crate::rendering::font::Font;
use crate::rendering::screen::ScreenLayer;
use crate::vm::Camera;
use crate::vm::Palette;
use crate::vm::audio::{NoiseChannel, Sound, SquareChannel};
use fc_core::{Color, Vec2};
use log::error;
use std::sync::{Arc, Mutex};

fn tick_sfx_channel(
    player: &mut SfxPlayer,
    memory: &Memory,
    sound: &mut Sound,
    forced_wave: Option<u8>,
) {
    if !player.active {
        return;
    }

    if player.tick_count == 0 {
        let base = SfxPlayer::sfx_bytes_base(player.sfx_id, player.step);
        let note = memory.read(base).unwrap_or(0);
        let volume = memory.read(base + 1).unwrap_or(0);
        let wave = forced_wave.unwrap_or_else(|| memory.read(base + 2).unwrap_or(0));

        sound.square.duration = 0;
        sound.noise.duration = 0;

        if note == 0 {
            match forced_wave {
                Some(0) => sound.square.enabled = false,
                Some(1) => sound.noise.enabled = false,
                _ => {
                    sound.square.enabled = false;
                    sound.noise.enabled = false;
                }
            }
        } else {
            let freq = note_to_freq(note);
            let vol = volume as f32 / 15.0;
            if wave == 0 {
                sound.square = SquareChannel {
                    enabled: true,
                    frequency: freq,
                    volume: vol,
                    duration: 0,
                };
                if forced_wave.is_none() {
                    sound.noise.enabled = false;
                }
            } else {
                sound.noise = NoiseChannel {
                    enabled: true,
                    rate: freq,
                    volume: vol,
                    duration: 0,
                };
                if forced_wave.is_none() {
                    sound.square.enabled = false;
                }
            }
        }
    }

    player.tick_count += 1;
    if player.tick_count >= player.ticks_per_step {
        player.tick_count = 0;
        player.step += 1;
        if player.step >= 16 {
            player.active = false;
            match forced_wave {
                Some(0) => sound.square.enabled = false,
                Some(1) => sound.noise.enabled = false,
                _ => {
                    sound.square.enabled = false;
                    sound.noise.enabled = false;
                }
            }
        }
    }
}

pub struct Vm {
    cpu: Cpu,
    program: Vec<u8>,
    memory: Memory,
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
}

impl Vm {
    pub fn new(instructions: Arc<InstructionSet>, config: VmConfig) -> Self {
        let mut cpu = Cpu::new(config.register_count);
        cpu.set_sp(config.memory_size);
        Self {
            cpu,
            program: Vec::new(),
            memory: Memory::new(config.memory_size),
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
        self.peripherals.init_all(&mut self.memory);
        Ok(())
    }

    pub fn load_rom(&mut self, program: Vec<u8>) {
        self.source_map = fc_asm::generate_source_map(&program);
        self.program = program;
        self.fc_source_lines.clear();
        self.cpu.set_pc(0);
        self.frame_count = 0;
        self.peripherals.init_all(&mut self.memory);
    }

    pub fn load_rom_with_source_map(&mut self, program: Vec<u8>, source_map: fc_asm::SourceMap) {
        self.program = program;
        self.source_map = source_map;
        self.fc_source_lines.clear();
        self.cpu.set_pc(0);
        self.frame_count = 0;
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

    fn trigger_music_row(&mut self) {
        let base =
            MusicPlayer::pattern_row_base(self.music_player.pattern_id, self.music_player.row);
        let ch0_ref = self.memory.read(base).unwrap_or(0);
        let ch1_ref = self.memory.read(base + 1).unwrap_or(0);
        if ch0_ref > 0 {
            self.music_player.ch0.start(ch0_ref - 1);
        } else {
            self.music_player.ch0.active = false;
        }
        if ch1_ref > 0 {
            self.music_player.ch1.start(ch1_ref - 1);
        } else {
            self.music_player.ch1.active = false;
        }
    }

    fn tick_sfx_player(&mut self) {
        if !self.sfx_player.active {
            return;
        }
        if let Ok(mut s) = self.sound.try_lock() {
            tick_sfx_channel(&mut self.sfx_player, &self.memory, &mut s, None);
        }
    }

    fn tick_music_player(&mut self) {
        if !self.music_player.active {
            return;
        }

        // First tick of a new row: load SFX references into channel players
        if self.music_player.tick_count == 0 {
            self.trigger_music_row();
        }

        if let Ok(mut s) = self.sound.try_lock() {
            tick_sfx_channel(&mut self.music_player.ch0, &self.memory, &mut s, Some(0));
            tick_sfx_channel(&mut self.music_player.ch1, &self.memory, &mut s, Some(1));
        }

        self.music_player.tick_count += 1;
        if self.music_player.tick_count >= self.music_player.ticks_per_row {
            self.music_player.tick_count = 0;
            self.music_player.row += 1;
            if self.music_player.row >= 16 {
                if self.music_player.loop_on {
                    self.music_player.row = 0;
                } else {
                    self.music_player.active = false;
                }
            }
        }
    }

    pub fn disassemble(&self, pc: usize) -> String {
        let program = self.get_program();
        let source_map = self.get_source_map();

        if pc >= program.len() {
            return "OUT OF BOUNDS".to_string();
        }

        let mut info_parts = Vec::new();
        if let Some(address_info) = source_map.get(pc) {
            for label in &address_info.labels {
                info_parts.push(format!("[{}]", label.to_uppercase()));
            }

            if let Some(item) = &address_info.item {
                match item {
                    fc_asm::ItemInfo::Instruction {
                        name: _,
                        opcode: _,
                        size,
                    } => {
                        let opcode = program[pc];
                        let instruction = self.get_instruction_by_opcode(opcode);
                        if let Some(instr) = instruction {
                            let end = (pc + size).min(program.len());
                            let bytes = &program[pc..end];
                            if bytes.len() < *size {
                                info_parts.push(format!("{} (INCOMPLETE)", instr.name));
                            } else {
                                info_parts.push((instr.debug_info)(bytes));
                            }
                        } else {
                            info_parts.push(format!("UNKNOWN OPCODE: 0X{:02X}", opcode));
                        }
                    }
                    fc_asm::ItemInfo::Directive { name, size } => {
                        let end = (pc + size).min(program.len());
                        let bytes = &program[pc..end];
                        let hex_string: Vec<String> =
                            bytes.iter().map(|b| format!("{:02X}", b)).collect();
                        info_parts.push(format!("{} {}", name, hex_string.join(" ")));
                    }
                }
            }
        }

        if info_parts.is_empty() {
            format!(".DB 0X{:02X}", program[pc])
        } else {
            info_parts.join(" ")
        }
    }

    pub fn run_frame(&mut self, input: &Input, font: &Font) {
        self.waiting = false;
        self.tick_music_player();
        self.tick_sfx_player();
        self.peripherals
            .tick_all(&mut self.memory, self.frame_count);
        self.frame_count = self.frame_count.wrapping_add(1);

        let mut steps = 0u32;
        while !self.waiting {
            self.step(input, font);
            steps += 1;
            if steps >= 1_000_000 {
                self.set_fault(VmFault::StepLimitExceeded);
                break;
            }
        }
    }

    pub fn step(&mut self, input: &Input, font: &Font) {
        if self.fault.is_some() {
            self.waiting = true;
            return;
        }

        if self.program.is_empty() {
            self.waiting = true;
            return;
        }

        let pc = self.cpu.get_pc();
        if pc >= self.program.len() {
            self.waiting = true;
            return;
        }

        let opcode = self.program[pc];

        let handler = {
            let instruction = self.instructions.get_by_opcode(opcode);
            if let Some(instr) = instruction {
                instr.execute
            } else {
                self.set_fault(VmFault::InvalidOpcode(opcode));
                return;
            }
        };

        self.cpu.set_pc(pc + 1);

        let sound_arc = Arc::clone(&self.sound);
        // A poisoned lock only means another thread panicked mid-write;
        // sound state is non-critical, so recover rather than propagate the panic.
        let mut sound_guard = sound_arc.lock().unwrap_or_else(|e| e.into_inner());
        let mut ctx = ExecutionContext {
            cpu: &mut self.cpu,
            mem: &mut self.memory,
            palette: &mut self.palette,
            camera: &mut self.camera,
            sound: &mut sound_guard,
            sfx_player: &mut self.sfx_player,
            music_player: &mut self.music_player,
            program: &self.program,
            input,
            font,
            config: &self.config,
            world: &mut self.world,
            ui: &mut self.ui,
            waiting: &mut self.waiting,
        };
        let result = (handler)(&mut ctx);
        drop(sound_guard);
        if let Err(fault) = result {
            self.set_fault(fault);
        }
    }

    pub fn snapshot(&self) -> VmSnapshot {
        VmSnapshot {
            pc: self.cpu.get_pc(),
            registers: self.cpu.get_registers().to_vec(),
            memory: self.memory.get_ram().to_vec(),
            camera_x: self.camera.get_x(),
            camera_y: self.camera.get_y(),
            palette: self.palette.get_colors().to_vec(),
            waiting: self.waiting,
            fault: self.fault,
            frame_count: self.frame_count,
            world: self.world.get_pixels().to_vec(),
            ui: self.ui.get_pixels().to_vec(),
            sfx_player: self.sfx_player.clone(),
            music_player: self.music_player.clone(),
        }
    }

    pub fn restore(&mut self, snapshot: &VmSnapshot) {
        self.cpu.set_pc(snapshot.pc);
        for (i, val) in snapshot.registers.iter().enumerate() {
            self.cpu.set_register(i, *val);
        }
        self.memory.set_ram(snapshot.memory.clone());
        self.camera
            .set_position(snapshot.camera_x, snapshot.camera_y);
        self.palette.set_colors(snapshot.palette.clone());
        self.waiting = snapshot.waiting;
        self.fault = snapshot.fault;
        self.frame_count = snapshot.frame_count;
        self.world.set_pixels(snapshot.world.clone());
        self.ui.set_pixels(snapshot.ui.clone());
        self.sfx_player = snapshot.sfx_player.clone();
        self.music_player = snapshot.music_player.clone();
    }
}

#[derive(Clone)]
pub struct VmSnapshot {
    pub pc: usize,
    pub registers: Vec<u32>,
    pub memory: Vec<u8>,
    pub camera_x: u32,
    pub camera_y: u32,
    pub palette: Vec<Color>,
    pub waiting: bool,
    pub fault: Option<VmFault>,
    pub frame_count: u32,
    pub world: Vec<u8>,
    pub ui: Vec<u8>,
    pub sfx_player: SfxPlayer,
    pub music_player: MusicPlayer,
}
