use crate::vm::{ExecutionContext, VmFault};
use log::debug;

pub fn jump(ctx: &mut ExecutionContext) -> Result<(), VmFault> {
    let address = ctx.read_word()?;

    debug!("Jumping to address {}", address);
    ctx.cpu.set_pc(address as usize);
    Ok(())
}

pub fn jump_if_not_zero(ctx: &mut ExecutionContext) -> Result<(), VmFault> {
    let reg_index = ctx.read_register_index()?;
    let address = ctx.read_word()?;

    let reg_value = ctx.cpu.get_register_value(reg_index);
    debug!(
        "JNZ check: register {} value is {}, jumping to {} if not zero",
        reg_index, reg_value, address
    );
    if reg_value != 0 {
        ctx.cpu.set_pc(address as usize);
    }
    Ok(())
}

pub fn jump_if_zero(ctx: &mut ExecutionContext) -> Result<(), VmFault> {
    let reg_index = ctx.read_register_index()?;
    let address = ctx.read_word()?;

    let reg_value = ctx.cpu.get_register_value(reg_index);
    debug!(
        "JZ check: register {} value is {}, jumping to {} if zero",
        reg_index, reg_value, address
    );
    if reg_value == 0 {
        ctx.cpu.set_pc(address as usize);
    }
    Ok(())
}

pub fn wait(ctx: &mut ExecutionContext) -> Result<(), VmFault> {
    debug!("Waiting for next frame");
    *ctx.waiting = true;
    Ok(())
}

pub fn jump_subroutine(ctx: &mut ExecutionContext) -> Result<(), VmFault> {
    let address = ctx.read_word()?;
    let pc = ctx.cpu.get_pc() as u16;

    let pc_low = (pc & 0xFF) as u8;
    let pc_high = ((pc >> 8) & 0xFF) as u8;

    let sp = ctx.cpu.get_sp();
    ctx.mem.write(sp - 1, pc_high)?;
    ctx.mem.write(sp - 2, pc_low)?;
    ctx.cpu.set_sp(sp - 2);

    debug!("JSR to {:04X}, pushing return address {:04X}", address, pc);
    ctx.cpu.set_pc(address as usize);
    Ok(())
}

pub fn return_subroutine(ctx: &mut ExecutionContext) -> Result<(), VmFault> {
    let sp = ctx.cpu.get_sp();
    let pc_low = ctx.mem.read(sp)?;
    let pc_high = ctx.mem.read(sp + 1)?;
    let pc = (pc_low as u16) | ((pc_high as u16) << 8);

    ctx.cpu.set_sp(sp + 2);

    debug!("RET to {:04X}", pc);
    ctx.cpu.set_pc(pc as usize);
    Ok(())
}
