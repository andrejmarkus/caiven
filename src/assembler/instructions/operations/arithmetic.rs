use crate::vm::ExecutionContext;
use log::debug;

pub fn move_value(ctx: &mut ExecutionContext) {
    let reg_index = ctx.vm.read_register_index();
    let value = ctx.vm.read_word();

    debug!("Moved value {} into register {}", value, reg_index);
    ctx.vm.set_register(reg_index, value);
}

pub fn add_value(ctx: &mut ExecutionContext) {
    let reg_index = ctx.vm.read_register_index();
    let value = ctx.vm.read_word();

    debug!("Added value {} to register {}", value, reg_index);
    ctx.vm.increment_register_value(reg_index, value);
}

pub fn decrement_value(ctx: &mut ExecutionContext) {
    let reg_index = ctx.vm.read_register_index();

    debug!("Decremented register {}", reg_index);
    ctx.vm.decrement_register_value(reg_index, 1);
}
