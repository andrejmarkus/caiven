//! Frame/step execution and audio player ticking for [`Vm`].

use super::memory::Memory;
use super::sfx::{MusicPlayer, SfxPlayer, note_to_freq};
use super::{ExecutionContext, Vm, VmFault};
use crate::input::Input;
use crate::rendering::font::Font;
use crate::vm::audio::{NoiseChannel, Sound, SquareChannel};
use std::sync::Arc;

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

impl Vm {
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

    /// Advances SFX/music playback one frame without running the program —
    /// lets editors preview audio while the game is stopped or paused.
    pub fn tick_audio_players(&mut self) {
        self.tick_music_player();
        self.tick_sfx_player();
    }

    pub fn run_frame(&mut self, input: &Input, font: &Font) {
        self.waiting = false;
        self.tick_music_player();
        self.tick_sfx_player();
        self.peripherals
            .tick_all(&mut self.memory, self.frame_count);
        self.frame_count = self.frame_count.wrapping_add(1);

        if self.has_lua_script() {
            self.run_frame_lua(input, font);
            self.waiting = true;
            return;
        }

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

    /// Like [`Vm::run_frame`], but pauses when the PC reaches an address in
    /// `breakpoints`, returning that address (the frame is left incomplete).
    /// `ignore` exempts one address for the first instruction so resuming
    /// from a breakpoint does not immediately re-trap; resuming restarts the
    /// frame ticks (audio, peripherals, frame counter).
    pub fn run_frame_bp(
        &mut self,
        input: &Input,
        font: &Font,
        breakpoints: &[usize],
        ignore: Option<usize>,
    ) -> Option<usize> {
        self.waiting = false;
        self.tick_music_player();
        self.tick_sfx_player();
        self.peripherals
            .tick_all(&mut self.memory, self.frame_count);
        self.frame_count = self.frame_count.wrapping_add(1);

        let mut steps = 0u32;
        while !self.waiting {
            let pc = self.cpu.get_pc();
            if breakpoints.contains(&pc) && (steps > 0 || ignore != Some(pc)) {
                return Some(pc);
            }
            self.step(input, font);
            steps += 1;
            if steps >= 1_000_000 {
                self.set_fault(VmFault::StepLimitExceeded);
                break;
            }
        }
        None
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
            tables: &mut self.tables,
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
}
