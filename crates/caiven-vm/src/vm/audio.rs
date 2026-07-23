use crate::peripheral::Peripheral;
use crate::vm::memory::Memory;
use std::sync::{Arc, Mutex};

#[cfg(feature = "native")]
use anyhow::{Context, Result};
#[cfg(feature = "native")]
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

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

#[cfg(feature = "native")]
pub struct Audio {
    #[allow(dead_code)]
    stream: cpal::Stream,
}

#[cfg(feature = "native")]
impl Audio {
    pub fn new(sound: Arc<Mutex<Sound>>) -> Result<Self> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .context("no default audio output device")?;
        let config = device
            .default_output_config()
            .context("failed to get default audio output config")?;

        let channels = config.channels() as usize;
        let sample_rate = config.sample_rate() as f32;

        let mut synth = Synth::new();

        macro_rules! build_stream {
            ($t:ty, $conv:expr) => {{
                let sound = sound.clone();
                device.build_output_stream(
                    &config.into(),
                    move |out: &mut [$t], _: &cpal::OutputCallbackInfo| {
                        let s = match sound.try_lock() {
                            Ok(s) => s,
                            Err(_) => return,
                        };
                        for frame in out.chunks_mut(channels) {
                            let sample_value = synth.next_sample(&s, sample_rate);
                            let final_sample: $t = $conv(sample_value);
                            for sample in frame.iter_mut() {
                                *sample = final_sample;
                            }
                        }
                    },
                    |_| {},
                    None,
                )
            }};
        }

        let sample_format = config.sample_format();
        let stream = match sample_format {
            cpal::SampleFormat::F32 => build_stream!(f32, |x: f32| x),
            cpal::SampleFormat::I16 => {
                build_stream!(i16, |x: f32| (x * i16::MAX as f32) as i16)
            }
            cpal::SampleFormat::U16 => {
                build_stream!(u16, |x: f32| ((x * 0.5 + 0.5) * u16::MAX as f32) as u16)
            }
            _ => {
                return Err(anyhow::anyhow!(
                    "unsupported audio sample format: {:?}",
                    sample_format
                ));
            }
        }
        .context("failed to build audio output stream")?;

        stream.play().context("failed to start audio playback")?;

        Ok(Self { stream })
    }
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
