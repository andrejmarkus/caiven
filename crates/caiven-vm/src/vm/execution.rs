//! Frame/step execution and audio player ticking for [`Vm`].

use super::Vm;
use super::memory::Memory;
use super::sfx::{MusicPlayer, SfxPlayer, decode_byte3, note_to_freq};
use crate::input::Input;
use crate::rendering::font::Font;
use crate::vm::audio;
use crate::vm::audio::{Voice, VoiceKind};

fn tick_sfx_channel(
    player: &mut SfxPlayer,
    memory: &Memory,
    voice: &mut Voice,
    forced_kind: Option<VoiceKind>,
    volume_scale: f32,
) {
    if !player.active {
        return;
    }

    if player.tick_count == 0 {
        let base = SfxPlayer::sfx_bytes_base(player.sfx_id, player.step);
        let note = memory.read(base).unwrap_or(0);
        let volume = memory.read(base + 1).unwrap_or(0);
        let wave = memory.read(base + 2).unwrap_or(0);
        let byte3 = memory.read(base + 3).unwrap_or(0);
        let (pan, attack_ms, release_ms) = decode_byte3(byte3);

        if note == 0 {
            voice.gate = false;
        } else {
            voice.kind = forced_kind.unwrap_or(if wave == 0 {
                VoiceKind::Square
            } else {
                VoiceKind::Noise
            });
            voice.frequency = note_to_freq(note);
            voice.volume = (volume as f32 / 15.0) * volume_scale;
            voice.pan = pan;
            voice.attack_ms = attack_ms;
            voice.release_ms = release_ms;
            voice.gate = true;
            voice.epoch = voice.epoch.wrapping_add(1);
        }
    }

    player.tick_count += 1;
    if player.tick_count >= player.ticks_per_step {
        player.tick_count = 0;
        player.step += 1;
        if player.step >= 16 {
            player.active = false;
            voice.gate = false;
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
            tick_sfx_channel(
                &mut self.sfx_player,
                &self.memory,
                &mut s.voices[audio::LEGACY_SFX_VOICE],
                None,
                1.0,
            );
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

        // Voice 0 is hard-assigned to ch0 (forced Square) and voice 1 to
        // ch1 (forced Noise) — the per-step `wave` byte the Music tracker
        // UI lets you set is intentionally ignored here to keep both
        // channels audible at once instead of one overriding the other;
        // it only does something for single-voice SFX playback.
        if let Ok(mut s) = self.sound.try_lock() {
            let (ch0_voice, rest) = s.voices.split_first_mut().expect("voices is non-empty");
            let ch1_voice = &mut rest[0];
            tick_sfx_channel(
                &mut self.music_player.ch0,
                &self.memory,
                ch0_voice,
                Some(VoiceKind::Square),
                1.0,
            );
            tick_sfx_channel(
                &mut self.music_player.ch1,
                &self.memory,
                ch1_voice,
                Some(VoiceKind::Noise),
                1.0,
            );
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

    fn tick_sfx_pool(&mut self) {
        if let Ok(mut s) = self.sound.try_lock() {
            for (i, pooled) in self.sfx_pool.iter_mut().enumerate() {
                tick_sfx_channel(
                    &mut pooled.player,
                    &self.memory,
                    &mut s.voices[audio::SFX_POOL_START + i],
                    None,
                    pooled.volume_scale,
                );
            }
        }
    }

    /// Advances SFX/music playback one frame without running the program —
    /// lets editors preview audio while the game is stopped or paused.
    pub fn tick_audio_players(&mut self) {
        self.tick_music_player();
        self.tick_sfx_player();
        self.tick_sfx_pool();
    }

    pub fn run_frame(&mut self, input: &Input, font: &Font) {
        self.waiting = false;
        self.tick_music_player();
        self.tick_sfx_player();
        self.tick_sfx_pool();
        self.peripherals
            .tick_all(&mut self.memory, self.frame_count);
        self.frame_count = self.frame_count.wrapping_add(1);

        self.run_frame_lua(input, font);
        self.waiting = true;
    }
}
