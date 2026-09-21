//! Frame/step execution and audio player ticking for [`Vm`].

use super::Vm;
use super::memory::Memory;
use super::sfx::{MusicPlayer, SfxPlayer, decode_byte3, note_to_freq, resolve_song_step};
use crate::input::Input;
use crate::rendering::font::Font;
use crate::vm::audio;
use crate::vm::audio::{MUSIC_VOICE_KINDS, Voice, VoiceKind};
use caiven_core::memory::{MUSIC_PATTERN_ROWS, SFX_COUNT};

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
        for (channel, player) in self.music_player.channels.iter_mut().enumerate() {
            // A cell holds `sfx id + 1`, so 0 means "no note on this channel
            // this row" rather than "SFX 0".
            match self.memory.read(base + channel).unwrap_or(0) {
                0 => player.active = false,
                // A cell past the SFX bank would read music/RTC bytes as a sound.
                sfx_ref if (sfx_ref as usize) > SFX_COUNT => player.active = false,
                sfx_ref => player.start(sfx_ref - 1),
            }
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

        // Each music channel's timbre is fixed by its column, so the per-step
        // `wave` byte the SFX editor writes is ignored here — it only does
        // something when the same SFX is played through a voice of its own.
        if let Ok(mut s) = self.sound.lock() {
            for (channel, player) in self.music_player.channels.iter_mut().enumerate() {
                let (Some(voice), Some(kind)) = (
                    s.voices.get_mut(audio::MUSIC_VOICE_START + channel),
                    MUSIC_VOICE_KINDS.get(channel).copied(),
                ) else {
                    continue;
                };
                tick_sfx_channel(player, &self.memory, voice, Some(kind), 1.0);
            }
        }

        self.music_player.tick_count += 1;
        if self.music_player.tick_count >= self.music_player.ticks_per_row {
            self.music_player.tick_count = 0;
            self.music_player.row += 1;
            if self.music_player.row as usize >= MUSIC_PATTERN_ROWS {
                if self.music_player.song_active {
                    let next = self.music_player.song_step.saturating_add(1);
                    match resolve_song_step(&self.memory, next) {
                        Some((pattern, resolved_step)) => {
                            self.music_player.pattern_id = pattern;
                            self.music_player.song_step = resolved_step;
                            self.music_player.row = 0;
                        }
                        None => {
                            self.music_player.active = false;
                            self.music_player.song_active = false;
                        }
                    }
                } else if self.music_player.loop_on {
                    self.music_player.row = 0;
                } else {
                    self.music_player.active = false;
                }
            }
        }
    }

    fn tick_sfx_pool(&mut self) {
        if let Ok(mut s) = self.sound.lock() {
            for (i, pooled) in self.sfx_pool.iter_mut().enumerate() {
                tick_sfx_channel(
                    &mut pooled.player,
                    &self.memory,
                    &mut s.voices[audio::SFX_VOICE_START + i],
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
        self.tick_sfx_pool();
    }

    pub fn run_frame(&mut self, input: &Input, font: &Font) {
        self.waiting = false;
        self.tick_music_player();
        self.tick_sfx_pool();
        self.peripherals
            .tick_all(&mut self.memory, self.frame_count);
        self.frame_count = self.frame_count.wrapping_add(1);

        self.run_frame_lua(input, font);
        self.waiting = true;
    }
}
