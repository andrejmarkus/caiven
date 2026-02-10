use crate::vm::ExecutionContext;
use log::info;

pub fn log_register(ctx: &mut ExecutionContext) {
    let reg_index = ctx.vm.read_register_index();
    let value = ctx.vm.get_register_value(reg_index);
    info!("Guest LOG: R{} = {}", reg_index, value);
}

pub fn log_value(ctx: &mut ExecutionContext) {
    let value = ctx.vm.read_word();
    info!("Guest LOG: {}", value);
}

pub fn breakpoint(_ctx: &mut ExecutionContext) {
    // Breakpoints are handled by the debugger checking PC
}
