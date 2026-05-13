use crate::vm::audio::{NoiseChannel, SquareChannel};
use crate::vm::{ExecutionContext, VmFault};
use log::debug;

pub fn play_sfx(ctx: &mut ExecutionContext) -> Result<(), VmFault> {
    let id = ctx.read_byte()?;
    debug!("SFX {}", id);
    ctx.sfx_player.start(id);
    Ok(())
}

pub fn play_music(ctx: &mut ExecutionContext) -> Result<(), VmFault> {
    let id = ctx.read_byte()?;
    debug!("MUS {}", id);
    ctx.music_player.start(id);
    Ok(())
}

pub fn stop_music_opcode(ctx: &mut ExecutionContext) -> Result<(), VmFault> {
    debug!("NOMUS");
    ctx.music_player.stop();
    ctx.sound.square.enabled = false;
    ctx.sound.noise.enabled = false;
    Ok(())
}

pub fn play_sound(ctx: &mut ExecutionContext) -> Result<(), VmFault> {
    let freq = ctx.read_register_value()? as f32;
    let vol = ctx.read_register_value()? as f32 / 100.0;
    let dur = ctx.read_register_value()?;

    debug!(
        "Playing square sound: freq={}, vol={}, dur={}",
        freq, vol, dur
    );
    ctx.sound.square = SquareChannel {
        enabled: true,
        frequency: freq,
        volume: vol,
        duration: dur,
    };
    Ok(())
}

pub fn play_sound_value(ctx: &mut ExecutionContext) -> Result<(), VmFault> {
    let freq = ctx.read_word()? as f32;
    let vol = ctx.read_byte()? as f32 / 100.0;
    let dur = ctx.read_byte()? as u16;

    debug!(
        "Playing square sound: freq={}, vol={}, dur={}",
        freq, vol, dur
    );
    ctx.sound.square = SquareChannel {
        enabled: true,
        frequency: freq,
        volume: vol,
        duration: dur,
    };
    Ok(())
}

pub fn play_noise(ctx: &mut ExecutionContext) -> Result<(), VmFault> {
    let rate = ctx.read_register_value()? as f32;
    let vol = ctx.read_register_value()? as f32 / 100.0;
    let dur = ctx.read_register_value()?;

    debug!("Playing noise: rate={}, vol={}, dur={}", rate, vol, dur);
    ctx.sound.noise = NoiseChannel {
        enabled: true,
        rate,
        volume: vol,
        duration: dur,
    };
    Ok(())
}

pub fn play_noise_value(ctx: &mut ExecutionContext) -> Result<(), VmFault> {
    let rate = ctx.read_word()? as f32;
    let vol = ctx.read_byte()? as f32 / 100.0;
    let dur = ctx.read_byte()? as u16;

    debug!("Playing noise: rate={}, vol={}, dur={}", rate, vol, dur);
    ctx.sound.noise = NoiseChannel {
        enabled: true,
        rate,
        volume: vol,
        duration: dur,
    };
    Ok(())
}

pub fn stop_sound(ctx: &mut ExecutionContext) -> Result<(), VmFault> {
    debug!("Stopping all sound");
    ctx.sound.square.enabled = false;
    ctx.sound.noise.enabled = false;
    Ok(())
}

pub fn stop_square(ctx: &mut ExecutionContext) -> Result<(), VmFault> {
    debug!("Stopping square sound");
    ctx.sound.square.enabled = false;
    Ok(())
}

pub fn stop_noise(ctx: &mut ExecutionContext) -> Result<(), VmFault> {
    debug!("Stopping noise sound");
    ctx.sound.noise.enabled = false;
    Ok(())
}
