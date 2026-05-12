use crate::input::Button;
use crate::vm::{ExecutionContext, VmFault};
use log::debug;

pub fn input(ctx: &mut ExecutionContext) -> Result<(), VmFault> {
    let reg_index = ctx.read_register_index()?;
    let button_code = ctx.read_byte()?;

    let pressed = Button::from_u8(button_code)
        .map(|btn| ctx.input.is_pressed(btn))
        .unwrap_or(false);

    debug!(
        "Reading input for button code {} into register {}: {}",
        button_code, reg_index, pressed
    );
    ctx.cpu.set_register(reg_index, if pressed { 1 } else { 0 });
    Ok(())
}
