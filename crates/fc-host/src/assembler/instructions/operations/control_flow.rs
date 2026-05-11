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

pub fn jump_subroutine(ctx: &mut ExecutionContext) {
    let address = ctx.vm.read_word();
    let pc = ctx.vm.get_pc() as u16;

    let pc_low = (pc & 0xFF) as u8;
    let pc_high = ((pc >> 8) & 0xFF) as u8;

    let sp = ctx.vm.get_sp();
    ctx.vm.write_memory(sp - 1, pc_high);
    ctx.vm.write_memory(sp - 2, pc_low);
    ctx.vm.set_sp(sp - 2);

    debug!("JSR to {:04X}, pushing return address {:04X}", address, pc);
    ctx.vm.set_pc(address as usize);
}

pub fn return_subroutine(ctx: &mut ExecutionContext) {
    let sp = ctx.vm.get_sp();
    let pc_low = ctx.vm.read_memory(sp);
    let pc_high = ctx.vm.read_memory(sp + 1);
    let pc = (pc_low as u16) | ((pc_high as u16) << 8);

    ctx.vm.set_sp(sp + 2);

    debug!("RET to {:04X}", pc);
    ctx.vm.set_pc(pc as usize);
}
