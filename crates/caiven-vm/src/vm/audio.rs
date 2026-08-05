use crate::peripheral::Peripheral;
use crate::vm::memory::Memory;
use anyhow::Result;
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

/// Square+noise synth, one sample at a time. Portable (no cpal) so both the
/// native `Audio` stream callback and the web player's `AudioWorklet` fill
/// export can share the exact same waveform math.
pub struct Synth {
    square_phase: f32,
    noise_phase: f32,
    lfsr: u16,
}

impl Default for Synth {
    fn default() -> Self {
        Self::new()
    }
}

impl Synth {
    pub fn new() -> Self {
        Self {
            square_phase: 0.0,
            noise_phase: 0.0,
            lfsr: 0xACE1,
        }
    }

    /// Advances the synth by one output sample and returns it in `[-1, 1]`.
    pub fn next_sample(&mut self, sound: &Sound, sample_rate: f32) -> f32 {
        let mut mix = 0.0f32;

        if sound.square.enabled && sound.square.volume > 0.0 {
            let v = if self.square_phase < 0.5 { 1.0 } else { -1.0 };
            self.square_phase = (self.square_phase + sound.square.frequency / sample_rate) % 1.0;
            mix += v * sound.square.volume * CHANNEL_HEADROOM;
        }

        if sound.noise.enabled && sound.noise.volume > 0.0 {
            self.noise_phase += sound.noise.rate / sample_rate;
            if self.noise_phase >= 1.0 {
                self.noise_phase -= 1.0;
                let bit = (self.lfsr ^ (self.lfsr >> 2) ^ (self.lfsr >> 3) ^ (self.lfsr >> 5)) & 1;
                self.lfsr = (self.lfsr >> 1) | (bit << 15);
            }
            let v = if (self.lfsr & 1) == 0 { 1.0 } else { -1.0 };
            mix += v * sound.noise.volume * CHANNEL_HEADROOM;
        }

        (mix * MASTER_GAIN).clamp(-1.0, 1.0)
    }
}

#[derive(Debug, Clone)]
pub struct SquareChannel {
    pub enabled: bool,
    pub frequency: f32,
    pub volume: f32,
    pub duration: u16,
}

#[derive(Debug, Clone)]
pub struct NoiseChannel {
    pub enabled: bool,
    pub volume: f32,
    pub rate: f32,
    pub duration: u16,
}

#[derive(Debug, Clone)]
pub struct Sound {
    pub square: SquareChannel,
    pub noise: NoiseChannel,
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
pub trait AudioOut {}

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
    synth: Synth,
    sample_rate: f32,
    channels: usize,
}

#[cfg(any(feature = "sdl2-bundled", feature = "sdl2-dynamic"))]
impl AudioCallback for ConsoleCallback {
    type Channel = i16;

    fn callback(&mut self, out: &mut [i16]) {
        // Never block the audio thread. If the VM holds the lock this
        // frame, emit silence rather than stalling playback.
        let Ok(sound) = self.sound.try_lock() else {
            out.fill(0);
            return;
        };

        for frame in out.chunks_mut(self.channels) {
            let sample = self.synth.next_sample(&sound, self.sample_rate);
            let value = to_i16(sample);
            for slot in frame.iter_mut() {
                *slot = value;
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
impl AudioOut for SdlAudio {}

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

pub struct AudioPeripheral {
    sound: Arc<Mutex<Sound>>,
}

impl AudioPeripheral {
    pub fn new(sound: Arc<Mutex<Sound>>) -> Self {
        Self { sound }
    }
}

impl Peripheral for AudioPeripheral {
    fn name(&self) -> &'static str {
        "audio"
    }

    fn init(&mut self, _mem: &mut Memory) {}

    fn tick(&mut self, _mem: &mut Memory, _frame: u32) {
        let Ok(mut s) = self.sound.try_lock() else {
            return;
        };
        if s.square.enabled && s.square.duration > 0 {
            s.square.duration -= 1;
            if s.square.duration == 0 {
                s.square.enabled = false;
            }
        }
        if s.noise.enabled && s.noise.duration > 0 {
            s.noise.duration -= 1;
            if s.noise.duration == 0 {
                s.noise.enabled = false;
            }
        }
    }
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
