use std::sync::Arc;
use std::sync::Mutex;

use cpal::traits::DeviceTrait;
use cpal::traits::HostTrait;
use cpal::traits::StreamTrait;

#[derive(Debug, Clone)]
pub struct Sound {
    pub enabled: bool,
    pub frequency: f32,
    pub volume: f32,
}

pub struct Audio {
    stream: cpal::Stream,
}

impl Audio {
    pub fn new(sound: Arc<Mutex<Sound>>) -> Self {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .expect("Failed to get default output device");
        let config = device
            .default_output_config()
            .expect("Failed to get default output config");

        let sample_rate = config.sample_rate() as f32;
        let channels = config.channels() as usize;
        let mut phase = 0.0f32;

        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => device.build_output_stream(
                &config.into(),
                move |out: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    let s = sound.lock().unwrap();
                    for frame in out.chunks_mut(channels) {
                        let value = if s.enabled && s.volume > 0.0 {
                            let v = if phase < 0.5 { 1.0 } else { -1.0 };
                            phase = (phase + s.frequency / sample_rate) % 1.0;
                            v * s.volume
                        } else {
                            0.0
                        };
                        for sample in frame.iter_mut() {
                            *sample = value;
                        }
                    }
                },
                |_| {},
                None,
            ),
            cpal::SampleFormat::I16 => device.build_output_stream(
                &config.into(),
                move |out: &mut [i16], _: &cpal::OutputCallbackInfo| {
                    let s = sound.lock().unwrap();
                    for frame in out.chunks_mut(channels) {
                        let value = if s.enabled && s.volume > 0.0 {
                            let v = if phase < 0.5 { 1.0 } else { -1.0 };
                            phase = (phase + s.frequency / sample_rate) % 1.0;
                            v * s.volume
                        } else {
                            0.0
                        };
                        let sample_value = (value * i16::MAX as f32) as i16;
                        for sample in frame.iter_mut() {
                            *sample = sample_value;
                        }
                    }
                },
                |_| {},
                None,
            ),
            cpal::SampleFormat::U16 => device.build_output_stream(
                &config.into(),
                move |out: &mut [u16], _: &cpal::OutputCallbackInfo| {
                    let s = sound.lock().unwrap();
                    for frame in out.chunks_mut(channels) {
                        let value = if s.enabled && s.volume > 0.0 {
                            let v = if phase < 0.5 { 1.0 } else { -1.0 };
                            phase = (phase + s.frequency / sample_rate) % 1.0;
                            v * s.volume
                        } else {
                            0.0
                        };
                        let sample_value = ((value * 0.5 + 0.5) * u16::MAX as f32) as u16;
                        for sample in frame.iter_mut() {
                            *sample = sample_value;
                        }
                    }
                },
                |_| {},
                None,
            ),
            _ => panic!("Unsupported sample format"),
        }
        .unwrap();

        stream.play().unwrap();

        Self { stream }
    }
}
