use crate::vm::ExecutionContext;
use crate::vm::audio::Sound;
use log::debug;

pub fn play_sound(ctx: &mut ExecutionContext) {
    let freq = ctx.vm.read_register_value() as f32;
    let vol = ctx.vm.read_register_value() as f32 / 100.0;

    debug!("Playing sound: freq={}, vol={}", freq, vol);
    ctx.vm.set_sound(Sound {
        enabled: true,
        frequency: freq,
        volume: vol,
    });
}

pub fn play_sound_value(ctx: &mut ExecutionContext) {
    let freq = ctx.vm.read_word() as f32;
    let vol = ctx.vm.read_byte() as f32 / 100.0;

    debug!("Playing sound: freq={}, vol={}", freq, vol);
    ctx.vm.set_sound(Sound {
        enabled: true,
        frequency: freq,
        volume: vol,
    });
}

pub fn stop_sound(ctx: &mut ExecutionContext) {
    debug!("Stopping sound");
    let mut sound = ctx.vm.get_sound();
    sound.enabled = false;
    ctx.vm.set_sound(sound);
}
