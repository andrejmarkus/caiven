use crate::vm::ExecutionContext;
use log::debug;

pub fn jump(ctx: &mut ExecutionContext) {
    let address = ctx.vm.read_word();

    debug!("Jumping to address {}", address);
    ctx.vm.set_pc(address as usize);
}

pub fn jump_if_not_zero(ctx: &mut ExecutionContext) {
    let reg_index = ctx.vm.read_register_index();
    let address = ctx.vm.read_word();

    let reg_value = ctx.vm.get_register_value(reg_index);
    debug!(
        "JNZ check: register {} value is {}, jumping to {} if not zero",
        reg_index, reg_value, address
    );
    if reg_value != 0 {
        ctx.vm.set_pc(address as usize);
    }
}

pub fn jump_if_zero(ctx: &mut ExecutionContext) {
    let reg_index = ctx.vm.read_register_index();
    let address = ctx.vm.read_word();

    let reg_value = ctx.vm.get_register_value(reg_index);
    debug!(
        "JZ check: register {} value is {}, jumping to {} if zero",
        reg_index, reg_value, address
    );
    if reg_value == 0 {
        ctx.vm.set_pc(address as usize);
    }
}

pub fn wait(ctx: &mut ExecutionContext) {
    debug!("Waiting for next frame");
    ctx.vm.pause();
}
