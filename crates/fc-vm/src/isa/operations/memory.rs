use crate::vm::{ExecutionContext, VmFault};
use log::debug;

pub fn load_from_memory(ctx: &mut ExecutionContext) -> Result<(), VmFault> {
    let reg_index = ctx.read_register_index()?;
    let address = ctx.read_word()? as usize;

    let value = ctx.mem.read(address)?;

    debug!(
        "Loaded value {} from memory address {} into register {}",
        value, address, reg_index
    );
    ctx.cpu.set_register(reg_index, value as u16);
    Ok(())
}

pub fn load_word_from_memory(ctx: &mut ExecutionContext) -> Result<(), VmFault> {
    let reg_index = ctx.read_register_index()?;
    let address = ctx.read_word()? as usize;

    let low = ctx.mem.read(address)? as u16;
    let high = ctx.mem.read(address + 1)? as u16;
    let value = low | (high << 8);

    debug!(
        "Loaded word {} from memory address {} into register {}",
        value, address, reg_index
    );
    ctx.cpu.set_register(reg_index, value);
    Ok(())
}

pub fn store_to_memory(ctx: &mut ExecutionContext) -> Result<(), VmFault> {
    let address = ctx.read_word()? as usize;
    let reg_index = ctx.read_register_index()?;

    let value = ctx.cpu.get_register_value(reg_index) as u8;

    debug!(
        "Stored value {} from register {} into memory address {}",
        value, reg_index, address
    );
    ctx.mem.write(address, value)?;
    Ok(())
}

pub fn store_word_to_memory(ctx: &mut ExecutionContext) -> Result<(), VmFault> {
    let address = ctx.read_word()? as usize;
    let reg_index = ctx.read_register_index()?;

    let value = ctx.cpu.get_register_value(reg_index);

    debug!(
        "Stored word {} from register {} into memory address {}",
        value, reg_index, address
    );
    ctx.mem.write(address, (value & 0xFF) as u8)?;
    ctx.mem.write(address + 1, ((value >> 8) & 0xFF) as u8)?;
    Ok(())
}

pub fn load_from_memory_indirect(ctx: &mut ExecutionContext) -> Result<(), VmFault> {
    let reg_to_index = ctx.read_register_index()?;
    let reg_from_index = ctx.read_register_index()?;

    let address = ctx.cpu.get_register_value(reg_from_index) as usize;
    let value = ctx.mem.read(address)?;

    debug!(
        "Loaded value {} from memory address in register {} into register {}",
        value, reg_from_index, reg_to_index
    );
    ctx.cpu.set_register(reg_to_index, value as u16);
    Ok(())
}

pub fn store_to_memory_indirect(ctx: &mut ExecutionContext) -> Result<(), VmFault> {
    let reg_addr_index = ctx.read_register_index()?;
    let reg_val_index = ctx.read_register_index()?;

    let value = ctx.cpu.get_register_value(reg_val_index) as u8;
    let address = ctx.cpu.get_register_value(reg_addr_index) as usize;

    debug!(
        "Stored value {} from register {} into memory address in register {}",
        value, reg_val_index, reg_addr_index
    );
    ctx.mem.write(address, value)?;
    Ok(())
}

// CPY reads from the ROM program bytes (ctx.program), not RAM.
// Use this to copy embedded asset data from ROM into RAM at runtime.
pub fn copy(ctx: &mut ExecutionContext) -> Result<(), VmFault> {
    let dst = ctx.read_word()? as usize;
    let src = ctx.read_word()? as usize;
    let length = ctx.read_word()? as usize;

    debug!(
        "Copying {} bytes from program address {} to memory address {}",
        length, src, dst
    );

    for i in 0..length {
        if dst + i >= ctx.mem.get_length() || src + i >= ctx.program.len() {
            return Err(VmFault::MemoryOutOfBounds(dst + i));
        }
        let value = ctx.program[src + i];
        ctx.mem.write(dst + i, value)?;
    }
    Ok(())
}
