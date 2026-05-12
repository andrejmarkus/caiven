use crate::vm::{ExecutionContext, VmFault};
use log::info;

pub fn log_register(ctx: &mut ExecutionContext) -> Result<(), VmFault> {
    let reg_index = ctx.read_register_index()?;
    let value = ctx.cpu.get_register_value(reg_index);
    info!("Guest LOG: R{} = {}", reg_index, value);
    Ok(())
}

pub fn log_value(ctx: &mut ExecutionContext) -> Result<(), VmFault> {
    let value = ctx.read_word()?;
    info!("Guest LOG: {}", value);
    Ok(())
}

pub fn breakpoint(_ctx: &mut ExecutionContext) -> Result<(), VmFault> {
    Ok(())
}
