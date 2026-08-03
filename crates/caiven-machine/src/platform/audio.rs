//! SDL2 audio output for the console synth.
//!
//! Requests signed 16-bit samples rather than float: handheld SDL ports are
//! inconsistent about float output, and S16 is the format every one of them
//! supports. Whatever the device actually grants is honoured as-is.

use anyhow::{Context, Result, anyhow};
use caiven_vm::vm::audio::{AudioFactory, AudioOut, Sound, Synth};
use sdl2::audio::{AudioCallback, AudioDevice, AudioSpecDesired};
use std::sync::{Arc, Mutex};

/// Sample rate requested from the device. SDL may grant something else; the
/// synth is told whatever was actually obtained.
const DESIRED_SAMPLE_RATE: i32 = 44_100;
/// Buffer size in sample frames. 512 @ 44.1kHz is ~11ms — small enough that
/// sound effects feel attached to the frame that fired them, large enough
/// not to starve a 1.2GHz Cortex-A7.
const DESIRED_BUFFER_FRAMES: u16 = 512;

/// Renders synth samples on SDL's audio thread.
struct ConsoleCallback {
    sound: Arc<Mutex<Sound>>,
    synth: Synth,
    sample_rate: f32,
    channels: usize,
}

impl AudioCallback for ConsoleCallback {
    type Channel = i16;

    fn callback(&mut self, out: &mut [i16]) {
        // Never block the audio thread. If the VM holds the lock this frame,
        // emit silence rather than stalling playback — same policy the cpal
        // backend uses.
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
fn to_i16(sample: f32) -> i16 {
    (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16
}

/// An open SDL audio device. Dropping it stops playback.
pub struct SdlAudio {
    #[allow(dead_code)]
    device: AudioDevice<ConsoleCallback>,
}

impl AudioOut for SdlAudio {}

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

/// Builds an [`AudioFactory`] backed by SDL, for `ConsoleCore` to open the
/// output with.
///
/// The subsystem is cloned into the closure because `reset_vm` reopens the
/// device on every cart reload.
pub fn sdl_audio_factory(audio: sdl2::AudioSubsystem) -> AudioFactory {
    Box::new(move |sound| {
        SdlAudio::new(&audio, sound)
            .map(|a| Box::new(a) as Box<dyn AudioOut>)
            .context("SDL audio unavailable")
    })
}

#[cfg(test)]
mod tests {
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
