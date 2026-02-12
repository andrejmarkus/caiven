use crate::vm::ExecutionContext;
use crate::vm::audio::{NoiseChannel, SquareChannel};
use log::debug;

pub fn play_sound(ctx: &mut ExecutionContext) {
    let freq = ctx.vm.read_register_value() as f32;
    let vol = ctx.vm.read_register_value() as f32 / 100.0;
    let dur = ctx.vm.read_register_value();

    debug!(
        "Playing square sound: freq={}, vol={}, dur={}",
        freq, vol, dur
    );
    let mut sound = ctx.vm.get_sound();
    sound.square = SquareChannel {
        enabled: true,
        frequency: freq,
        volume: vol,
        duration: dur,
    };
    ctx.vm.set_sound(sound);
}

pub fn play_sound_value(ctx: &mut ExecutionContext) {
    let freq = ctx.vm.read_word() as f32;
    let vol = ctx.vm.read_byte() as f32 / 100.0;
    let dur = ctx.vm.read_byte() as u16;

    debug!(
        "Playing square sound: freq={}, vol={}, dur={}",
        freq, vol, dur
    );
    let mut sound = ctx.vm.get_sound();
    sound.square = SquareChannel {
        enabled: true,
        frequency: freq,
        volume: vol,
        duration: dur,
    };
    ctx.vm.set_sound(sound);
}

pub fn play_noise(ctx: &mut ExecutionContext) {
    let rate = ctx.vm.read_register_value() as f32;
    let vol = ctx.vm.read_register_value() as f32 / 100.0;
    let dur = ctx.vm.read_register_value();

    debug!("Playing noise: rate={}, vol={}, dur={}", rate, vol, dur);
    let mut sound = ctx.vm.get_sound();
    sound.noise = NoiseChannel {
        enabled: true,
        rate,
        volume: vol,
        duration: dur,
    };
    ctx.vm.set_sound(sound);
}

pub fn play_noise_value(ctx: &mut ExecutionContext) {
    let rate = ctx.vm.read_word() as f32;
    let vol = ctx.vm.read_byte() as f32 / 100.0;
    let dur = ctx.vm.read_byte() as u16;

    debug!("Playing noise: rate={}, vol={}, dur={}", rate, vol, dur);
    let mut sound = ctx.vm.get_sound();
    sound.noise = NoiseChannel {
        enabled: true,
        rate,
        volume: vol,
        duration: dur,
    };
    ctx.vm.set_sound(sound);
}

pub fn stop_sound(ctx: &mut ExecutionContext) {
    debug!("Stopping all sound");
    let mut sound = ctx.vm.get_sound();
    sound.square.enabled = false;
    sound.noise.enabled = false;
    ctx.vm.set_sound(sound);
}

pub fn stop_square(ctx: &mut ExecutionContext) {
    debug!("Stopping square sound");
    let mut sound = ctx.vm.get_sound();
    sound.square.enabled = false;
    ctx.vm.set_sound(sound);
}

pub fn stop_noise(ctx: &mut ExecutionContext) {
    debug!("Stopping noise sound");
    let mut sound = ctx.vm.get_sound();
    sound.noise.enabled = false;
    ctx.vm.set_sound(sound);
}
