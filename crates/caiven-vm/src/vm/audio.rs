use anyhow::Result;
use caiven_core::memory::MUSIC_CHANNEL_COUNT;
use std::sync::{Arc, Mutex};

#[cfg(any(feature = "sdl2-bundled", feature = "sdl2-dynamic"))]
use anyhow::{Context, anyhow};
#[cfg(any(feature = "sdl2-bundled", feature = "sdl2-dynamic"))]
use sdl2::audio::{AudioCallback, AudioDevice, AudioSpecDesired};

/// Per-channel scale so square+noise summing doesn't hard-clip at full volume.
const CHANNEL_HEADROOM: f32 = 0.5;
/// Overall output attenuation — raw full-scale square/noise waves read as
/// much louder than typical game audio at the same numeric volume.
const MASTER_GAIN: f32 = 0.35;

/// The console's six voices: four typed music channels followed by two
/// voices reserved for sound effects. Reserving the sfx voices is the point
/// of the split — a jump sound can never cut the melody, which is the most
/// confusing audio bug a beginner hits.
pub const MUSIC_VOICE_COUNT: usize = MUSIC_CHANNEL_COUNT;
pub const SFX_VOICE_COUNT: usize = 2;
pub const VOICE_COUNT: usize = MUSIC_VOICE_COUNT + SFX_VOICE_COUNT;
/// Voices `0..MUSIC_VOICE_COUNT` are the music channels, in tracker column
/// order.
pub const MUSIC_VOICE_START: usize = 0;
/// Voices `SFX_VOICE_START..VOICE_COUNT` back `play_sfx`/`stop_sfx` and
/// Studio's SFX-editor preview.
pub const SFX_VOICE_START: usize = MUSIC_VOICE_COUNT;

/// Fixed pan positions selected by the low 4 bits of an SFX step's byte3.
/// Index 0 is deliberately center — a step that never set byte3 (every
/// cart before this change) decodes to center, matching today's output.
pub const PAN_TABLE: [f32; 16] = [
    0.0, -0.125, 0.125, -0.25, 0.25, -0.375, 0.375, -0.5, 0.5, -0.625, 0.625, -0.75, 0.75, -0.875,
    0.875, -1.0,
];

/// Attack/release ramp lengths selected by byte3's 2-bit level fields.
/// Level 0 is instant, matching today's on/off behavior.
pub const ENVELOPE_MS: [f32; 4] = [0.0, 15.0, 50.0, 150.0];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoiceKind {
    /// 50% pulse. Named Square because that is what the waveform is; the
    /// tracker calls the two music voices using it "Pulse 1"/"Pulse 2".
    Square,
    Triangle,
    Noise,
}

/// Timbre of each music channel, fixed by the hardware. A music channel's
/// sound is decided by which column it is in, so "which channel?" is
/// answered by ear rather than by reading a wave byte.
pub const MUSIC_VOICE_KINDS: [VoiceKind; MUSIC_VOICE_COUNT] = [
    VoiceKind::Square,
    VoiceKind::Square,
    VoiceKind::Triangle,
    VoiceKind::Noise,
];

/// One synth voice's target parameters, written by the frame thread and
/// read every audio sample by [`Synth::next_sample`]. `epoch` is bumped on
/// every (re)trigger so the audio thread can tell a stolen/reused voice
/// apart from one still sustaining the same note, even though both look
/// like `gate == true` from the outside.
#[derive(Debug, Clone, Copy)]
pub struct Voice {
    pub kind: VoiceKind,
    pub gate: bool,
    pub frequency: f32,
    pub volume: f32,
    pub pan: f32,
    pub attack_ms: f32,
    pub release_ms: f32,
    pub epoch: u32,
}

impl Voice {
    pub fn silent() -> Self {
        Self {
            kind: VoiceKind::Square,
            gate: false,
            frequency: 440.0,
            volume: 0.0,
            pan: 0.0,
            attack_ms: 0.0,
            release_ms: 0.0,
            epoch: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Sound {
    pub voices: [Voice; VOICE_COUNT],
    /// Runtime-only multiplier layered on top of authored per-step
    /// volume, clamped to `[0, 1]` by whoever writes it. Not persisted to
    /// cart data.
    pub master_volume: f32,
    pub music_volume: f32,
    pub sfx_volume: f32,
}

impl Default for Sound {
    fn default() -> Self {
        Self {
            voices: [Voice::silent(); VOICE_COUNT],
            master_volume: 1.0,
            music_volume: 1.0,
            sfx_volume: 1.0,
        }
    }
}

/// Per-voice waveform + envelope synth, one output sample at a time.
/// Portable (no cpal) so both the native `Audio` stream callback and the
/// web player's `AudioWorklet` fill export can share the exact same
/// waveform math.
pub struct Synth {
    phase: [f32; VOICE_COUNT],
    lfsr: [u16; VOICE_COUNT],
    env_level: [f32; VOICE_COUNT],
    env_epoch: [u32; VOICE_COUNT],
}

impl Default for Synth {
    fn default() -> Self {
        Self::new()
    }
}

impl Synth {
    pub fn new() -> Self {
        Self {
            phase: [0.0; VOICE_COUNT],
            lfsr: [0xACE1; VOICE_COUNT],
            env_level: [0.0; VOICE_COUNT],
            env_epoch: [0; VOICE_COUNT],
        }
    }

    /// Advances every voice by one output sample and returns the mixed
    /// stereo pair, each in `[-1, 1]`.
    pub fn next_sample(&mut self, sound: &Sound, sample_rate: f32) -> (f32, f32) {
        let mut left = 0.0f32;
        let mut right = 0.0f32;

        for i in 0..VOICE_COUNT {
            let voice = &sound.voices[i];

            // A changed epoch means this voice was (re)triggered since the
            // last sample — reset phase/envelope even if `gate` still
            // reads the same as before (e.g. a stolen voice retriggered
            // mid-note).
            if voice.epoch != self.env_epoch[i] {
                self.env_epoch[i] = voice.epoch;
                self.env_level[i] = 0.0;
                self.phase[i] = 0.0;
            }

            if voice.gate {
                let step = if voice.attack_ms <= 0.0 {
                    1.0
                } else {
                    1000.0 / (voice.attack_ms * sample_rate)
                };
                self.env_level[i] = (self.env_level[i] + step).min(1.0);
            } else {
                let step = if voice.release_ms <= 0.0 {
                    1.0
                } else {
                    1000.0 / (voice.release_ms * sample_rate)
                };
                self.env_level[i] = (self.env_level[i] - step).max(0.0);
            }

            if voice.volume <= 0.0 || (self.env_level[i] <= 0.0 && !voice.gate) {
                continue;
            }

            let raw = match voice.kind {
                VoiceKind::Square => {
                    let v = if self.phase[i] < 0.5 { 1.0 } else { -1.0 };
                    self.phase[i] = (self.phase[i] + voice.frequency / sample_rate) % 1.0;
                    v
                }
                VoiceKind::Triangle => {
                    self.phase[i] = (self.phase[i] + voice.frequency / sample_rate) % 1.0;
                    // Rises -1 → 1 over the first half of the period and back
                    // down over the second, so the odd-harmonic-only timbre
                    // sits under a pulse lead without masking it.
                    4.0 * (self.phase[i] - 0.5).abs() - 1.0
                }
                VoiceKind::Noise => {
                    self.phase[i] += voice.frequency / sample_rate;
                    if self.phase[i] >= 1.0 {
                        self.phase[i] -= 1.0;
                        let bit = (self.lfsr[i]
                            ^ (self.lfsr[i] >> 2)
                            ^ (self.lfsr[i] >> 3)
                            ^ (self.lfsr[i] >> 5))
                            & 1;
                        self.lfsr[i] = (self.lfsr[i] >> 1) | (bit << 15);
                    }
                    if (self.lfsr[i] & 1) == 0 { 1.0 } else { -1.0 }
                }
            };

            let group_volume = if i < SFX_VOICE_START {
                sound.music_volume
            } else {
                sound.sfx_volume
            };
            let amp = raw * voice.volume * self.env_level[i] * CHANNEL_HEADROOM * group_volume;

            let pan = voice.pan;
            left += amp * (1.0 - pan.max(0.0));
            right += amp * (1.0 + pan.min(0.0));
        }

        (
            (left * MASTER_GAIN * sound.master_volume).clamp(-1.0, 1.0),
            (right * MASTER_GAIN * sound.master_volume).clamp(-1.0, 1.0),
        )
    }
}

/// An open audio output owned by the front-end.
///
/// Purely an RAII handle: the implementation streams samples from a
/// [`Synth`] on its own real-time thread for as long as the value is alive,
/// and dropping it silences the console. There is nothing to call.
///
/// The trait exists so the backend is supplied by whichever binary
/// constructs the [`crate::runtime::ConsoleCore`] — `caiven-machine` passes
/// in the `AudioSubsystem` it already owns for video via
/// [`sdl_audio_factory`], while Studio and tests get one via
/// [`sdl_default_audio_factory`]/[`ConsoleCore::new`](crate::runtime::ConsoleCore::new).
/// Front-end-supplied injection (rather than a plain constructor) keeps a
/// front-end that already owns an SDL context from opening a second one.
///
/// Deliberately not `Send`: SDL's audio device handle is thread-bound, and
/// `ConsoleCore` is already constructed on the thread that runs it.
pub trait AudioOut {
    /// Stops the device pulling samples. The synth thread keeps running
    /// idle, but nothing reaches the speaker until [`AudioOut::resume`] is
    /// called — front ends use this when the VM itself stops ticking (e.g.
    /// the pause menu), since the audio thread otherwise keeps rendering
    /// whatever the `Sound` state was left at, unaware the game paused.
    fn pause(&mut self) {}
    /// Resumes a device previously stopped with [`AudioOut::pause`].
    fn resume(&mut self) {}
}

/// Opens an audio output bound to `sound`. Returns `Err` when no device is
/// available; callers treat that as non-fatal and run the console silently.
///
/// Not `Send`/`Sync` for the same reason as [`AudioOut`]: SDL's subsystem
/// handles are thread-bound, and a `ConsoleCore` is used on the thread that
/// created it regardless.
pub type AudioFactory = Box<dyn Fn(Arc<Mutex<Sound>>) -> Result<Box<dyn AudioOut>>>;

/// Sample rate requested from the device. SDL may grant something else; the
/// synth is told whatever was actually obtained.
#[cfg(any(feature = "sdl2-bundled", feature = "sdl2-dynamic"))]
const DESIRED_SAMPLE_RATE: i32 = 44_100;
/// Buffer size in sample frames. 512 @ 44.1kHz is ~11ms — small enough that
/// sound effects feel attached to the frame that fired them, large enough
/// not to starve a 1.2GHz Cortex-A7.
#[cfg(any(feature = "sdl2-bundled", feature = "sdl2-dynamic"))]
const DESIRED_BUFFER_FRAMES: u16 = 512;

/// Renders synth samples on SDL's audio thread.
///
/// Requests signed 16-bit samples rather than float: handheld SDL ports are
/// inconsistent about float output, and S16 is the format every one of them
/// supports. Whatever the device actually grants is honoured as-is.
#[cfg(any(feature = "sdl2-bundled", feature = "sdl2-dynamic"))]
struct ConsoleCallback {
    sound: Arc<Mutex<Sound>>,
    local: Sound,
    synth: Synth,
    sample_rate: f32,
    channels: usize,
}

#[cfg(any(feature = "sdl2-bundled", feature = "sdl2-dynamic"))]
impl AudioCallback for ConsoleCallback {
    type Channel = i16;

    fn callback(&mut self, out: &mut [i16]) {
        // Copy under a short lock and render outside it; on contention keep
        // the previous copy rather than stalling or going silent.
        if let Ok(sound) = self.sound.try_lock() {
            self.local.clone_from(&sound);
        }

        for frame in out.chunks_mut(self.channels) {
            let (l, r) = self.synth.next_sample(&self.local, self.sample_rate);
            if self.channels <= 1 {
                let mono = to_i16((l + r) * 0.5);
                for slot in frame.iter_mut() {
                    *slot = mono;
                }
            } else {
                frame[0] = to_i16(l);
                frame[1] = to_i16(r);
                for slot in frame.iter_mut().skip(2) {
                    *slot = to_i16(l);
                }
            }
        }
    }
}

/// Converts a synth sample in `[-1, 1]` to signed 16-bit.
///
/// Clamps first: `Synth::next_sample` already limits its output, but an
/// out-of-range value would otherwise wrap on cast and turn a loud sound
/// into a full-scale click.
#[cfg(any(feature = "sdl2-bundled", feature = "sdl2-dynamic"))]
fn to_i16(sample: f32) -> i16 {
    (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16
}

/// An open SDL audio device. Dropping it stops playback.
#[cfg(any(feature = "sdl2-bundled", feature = "sdl2-dynamic"))]
pub struct SdlAudio {
    #[allow(dead_code)]
    device: AudioDevice<ConsoleCallback>,
}

#[cfg(any(feature = "sdl2-bundled", feature = "sdl2-dynamic"))]
impl AudioOut for SdlAudio {
    fn pause(&mut self) {
        self.device.pause();
    }

    fn resume(&mut self) {
        self.device.resume();
    }
}

#[cfg(any(feature = "sdl2-bundled", feature = "sdl2-dynamic"))]
impl SdlAudio {
    fn new(audio: &sdl2::AudioSubsystem, sound: Arc<Mutex<Sound>>) -> Result<Self> {
        let desired = AudioSpecDesired {
            freq: Some(DESIRED_SAMPLE_RATE),
            channels: None,
            samples: Some(DESIRED_BUFFER_FRAMES),
        };

        let device = audio
            .open_playback(None, &desired, |spec| ConsoleCallback {
                sound,
                local: Sound::default(),
                synth: Synth::new(),
                // Honour what the device granted, not what was asked for —
                // getting this wrong detunes every sound.
                sample_rate: spec.freq as f32,
                channels: spec.channels as usize,
            })
            .map_err(|e| anyhow!("failed to open SDL audio device: {e}"))?;

        let spec = device.spec();
        log::info!(
            "audio output: SDL ({}ch @ {}Hz, {} sample buffer)",
            spec.channels,
            spec.freq,
            spec.samples
        );

        device.resume();
        Ok(Self { device })
    }
}

/// Builds an [`AudioFactory`] backed by SDL, for a front-end that already
/// owns an `AudioSubsystem` (`caiven-machine` opens one for video already)
/// to hand `ConsoleCore` rather than have it open a second SDL context.
///
/// The subsystem is cloned into the closure because `reset_vm` reopens the
/// device on every cart reload.
#[cfg(any(feature = "sdl2-bundled", feature = "sdl2-dynamic"))]
pub fn sdl_audio_factory(audio: sdl2::AudioSubsystem) -> AudioFactory {
    Box::new(move |sound| {
        SdlAudio::new(&audio, sound)
            .map(|a| Box::new(a) as Box<dyn AudioOut>)
            .context("SDL audio unavailable")
    })
}

/// The default used by every front-end that doesn't already own an SDL
/// context (Studio, tests): opens its own audio-only SDL subsystem on
/// first use. `AudioDevice` keeps that subsystem (and its parent `Sdl`
/// context) alive internally for as long as the device is open, so nothing
/// needs to be held onto here beyond the closure itself.
#[cfg(any(feature = "sdl2-bundled", feature = "sdl2-dynamic"))]
pub fn sdl_default_audio_factory() -> AudioFactory {
    Box::new(|sound| {
        let sdl = sdl2::init().map_err(|e| anyhow!("failed to init SDL: {e}"))?;
        let audio = sdl
            .audio()
            .map_err(|e| anyhow!("failed to init SDL audio subsystem: {e}"))?;
        SdlAudio::new(&audio, sound).map(|a| Box::new(a) as Box<dyn AudioOut>)
    })
}

#[cfg(all(test, any(feature = "sdl2-bundled", feature = "sdl2-dynamic")))]
mod sdl_audio_tests {
    use super::to_i16;

    #[test]
    fn full_scale_samples_map_to_the_i16_extremes() {
        assert_eq!(to_i16(1.0), i16::MAX);
        assert_eq!(to_i16(-1.0), -i16::MAX);
        assert_eq!(to_i16(0.0), 0);
    }

    #[test]
    fn out_of_range_samples_clamp_instead_of_wrapping() {
        // Without the clamp these would wrap and produce a full-scale click
        // of the opposite sign.
        assert_eq!(to_i16(4.0), i16::MAX);
        assert_eq!(to_i16(-4.0), -i16::MAX);
    }

    #[test]
    fn midscale_sample_is_proportional() {
        let half = to_i16(0.5);
        assert!((half - i16::MAX / 2).abs() <= 1, "got {half}");
    }
}

#[cfg(test)]
mod voice_tests {
    use super::*;

    #[test]
    fn console_has_six_voices_four_of_them_typed_music_channels() {
        assert_eq!(VOICE_COUNT, 6);
        assert_eq!(MUSIC_VOICE_KINDS.len(), MUSIC_VOICE_COUNT);
        assert_eq!(SFX_VOICE_START + SFX_VOICE_COUNT, VOICE_COUNT);
        assert_eq!(
            MUSIC_VOICE_KINDS,
            [
                VoiceKind::Square,
                VoiceKind::Square,
                VoiceKind::Triangle,
                VoiceKind::Noise
            ]
        );
    }

    /// A triangle must ramp rather than jump between its extremes — that
    /// gradual slope is the whole reason it sits under a pulse lead instead
    /// of fighting it.
    #[test]
    fn triangle_voice_ramps_instead_of_switching_between_extremes() {
        let mut sound = Sound::default();
        sound.voices[SFX_VOICE_START] = Voice {
            kind: VoiceKind::Triangle,
            gate: true,
            frequency: 440.0,
            volume: 1.0,
            pan: 0.0,
            attack_ms: 0.0,
            release_ms: 0.0,
            epoch: 1,
        };
        let mut synth = Synth::new();
        let sample_rate = 44_100.0;
        let mut samples = Vec::new();
        // One full period at 440 Hz.
        for _ in 0..(sample_rate / 440.0) as usize {
            let (l, _r) = synth.next_sample(&sound, sample_rate);
            samples.push(l);
        }

        let biggest_jump = samples
            .windows(2)
            .map(|pair| (pair[1] - pair[0]).abs())
            .fold(0.0f32, f32::max);
        let span = samples.iter().fold(0.0f32, |acc, s| acc.max(s.abs()));
        assert!(
            biggest_jump < span,
            "triangle stepped {biggest_jump} between samples across a {span} span — that is a square, not a ramp"
        );
    }

    #[test]
    fn pan_table_index_zero_is_center() {
        assert_eq!(PAN_TABLE[0], 0.0);
    }

    #[test]
    fn pan_table_alternates_left_right_growing_outward() {
        assert_eq!(PAN_TABLE[1], -0.125);
        assert_eq!(PAN_TABLE[2], 0.125);
        assert_eq!(PAN_TABLE[15], -1.0);
    }

    #[test]
    fn envelope_levels_map_to_documented_ramp_lengths() {
        assert_eq!(ENVELOPE_MS, [0.0, 15.0, 50.0, 150.0]);
    }

    #[test]
    fn instant_envelope_reaches_full_volume_within_one_sample() {
        let mut sound = Sound::default();
        sound.voices[SFX_VOICE_START] = Voice {
            kind: VoiceKind::Square,
            gate: true,
            frequency: 440.0,
            volume: 1.0,
            pan: 0.0,
            attack_ms: 0.0,
            release_ms: 0.0,
            epoch: 1,
        };
        let mut synth = Synth::new();
        let (l, _r) = synth.next_sample(&sound, 44_100.0);
        assert!(
            l.abs() > 0.0,
            "expected audible output on first sample, got {l}"
        );
    }

    #[test]
    fn center_pan_matches_todays_equal_channel_output() {
        // byte3 == 0 decodes to pan 0.0; equal-gain center must reproduce
        // the old mono-duplicated-to-both-channels behavior exactly.
        let mut sound = Sound::default();
        sound.voices[SFX_VOICE_START] = Voice {
            kind: VoiceKind::Square,
            gate: true,
            frequency: 440.0,
            volume: 1.0,
            pan: 0.0,
            attack_ms: 0.0,
            release_ms: 0.0,
            epoch: 1,
        };
        let mut synth = Synth::new();
        let (l, r) = synth.next_sample(&sound, 44_100.0);
        assert_eq!(l, r);
    }

    #[test]
    fn hard_left_pan_silences_right_channel() {
        let mut sound = Sound::default();
        sound.voices[SFX_VOICE_START] = Voice {
            kind: VoiceKind::Square,
            gate: true,
            frequency: 440.0,
            volume: 1.0,
            pan: -1.0,
            attack_ms: 0.0,
            release_ms: 0.0,
            epoch: 1,
        };
        let mut synth = Synth::new();
        let (_l, r) = synth.next_sample(&sound, 44_100.0);
        assert_eq!(r, 0.0);
    }

    #[test]
    fn released_voice_fades_to_silence_over_release_samples() {
        let mut sound = Sound::default();
        sound.voices[SFX_VOICE_START] = Voice {
            kind: VoiceKind::Square,
            gate: false,
            frequency: 440.0,
            volume: 1.0,
            pan: 0.0,
            attack_ms: 0.0,
            release_ms: 150.0,
            epoch: 1,
        };
        let mut synth = Synth::new();
        // Force env_level to 1.0 as if the note had just been playing.
        synth.env_level[SFX_VOICE_START] = 1.0;
        synth.env_epoch[SFX_VOICE_START] = 1;
        let sample_rate = 44_100.0;
        // +1 sample of headroom: (0.150 * sample_rate) can truncate below the
        // exact sample count needed due to floating-point rounding.
        let release_samples = (0.150 * sample_rate) as usize + 1;
        for _ in 0..release_samples {
            synth.next_sample(&sound, sample_rate);
        }
        assert!(synth.env_level[SFX_VOICE_START] <= 0.0);
    }

    #[test]
    fn retrigger_via_epoch_resets_envelope_even_while_gated() {
        let mut sound = Sound::default();
        sound.voices[SFX_VOICE_START] = Voice {
            kind: VoiceKind::Square,
            gate: true,
            frequency: 440.0,
            volume: 1.0,
            pan: 0.0,
            attack_ms: 150.0,
            release_ms: 0.0,
            epoch: 1,
        };
        let mut synth = Synth::new();
        synth.env_level[SFX_VOICE_START] = 1.0;
        synth.env_epoch[SFX_VOICE_START] = 1;
        // Same epoch: envelope must NOT reset.
        synth.next_sample(&sound, 44_100.0);
        assert!(synth.env_level[SFX_VOICE_START] > 0.9);
        // New epoch (retrigger/steal): envelope must reset to a fresh attack ramp.
        sound.voices[SFX_VOICE_START].epoch = 2;
        synth.next_sample(&sound, 44_100.0);
        assert!(synth.env_level[SFX_VOICE_START] < 0.9);
    }
}
