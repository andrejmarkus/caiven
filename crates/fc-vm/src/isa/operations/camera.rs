use crate::vm::{ExecutionContext, VmFault};

pub fn set_camera_position(ctx: &mut ExecutionContext) -> Result<(), VmFault> {
    let x = ctx.read_register_value()? as u32;
    let y = ctx.read_register_value()? as u32;

    ctx.camera.set_position(x, y);
    Ok(())
}

pub fn move_camera(ctx: &mut ExecutionContext) -> Result<(), VmFault> {
    let dx = ctx.read_register_value()? as i32;
    let dy = ctx.read_register_value()? as i32;

    ctx.camera.move_by(dx, dy);
    Ok(())
}
