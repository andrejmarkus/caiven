use crate::vm::ExecutionContext;

pub fn set_camera_position(ctx: &mut ExecutionContext) {
    let x = ctx.vm.read_register_value() as u32;
    let y = ctx.vm.read_register_value() as u32;

    ctx.vm.set_camera_position(x, y);
}

pub fn move_camera(ctx: &mut ExecutionContext) {
    let dx = ctx.vm.read_register_value() as i32;
    let dy = ctx.vm.read_register_value() as i32;

    ctx.vm.move_camera_by(dx, dy);
}
