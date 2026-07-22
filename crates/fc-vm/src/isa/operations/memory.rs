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
    ctx.cpu.set_register(reg_index, value as u32);
    Ok(())
}

pub fn load_word_from_memory(ctx: &mut ExecutionContext) -> Result<(), VmFault> {
    let reg_index = ctx.read_register_index()?;
    let address = ctx.read_word()? as usize;

    let low = ctx.mem.read(address)? as u32;
    let high = ctx.mem.read(address + 1)? as u32;
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
    ctx.cpu.set_register(reg_to_index, value as u32);
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

pub fn push_register(ctx: &mut ExecutionContext) -> Result<(), VmFault> {
    let src = ctx.read_register_index()?;
    let value = ctx.cpu.get_register_value(src);
    let sp = ctx.cpu.get_sp();
    if sp < 4 {
        return Err(VmFault::StackOverflow);
    }
    let sp = sp - 4;
    ctx.mem.write(sp, (value & 0xFF) as u8)?;
    ctx.mem.write(sp + 1, ((value >> 8) & 0xFF) as u8)?;
    ctx.mem.write(sp + 2, ((value >> 16) & 0xFF) as u8)?;
    ctx.mem.write(sp + 3, ((value >> 24) & 0xFF) as u8)?;
    ctx.cpu.set_sp(sp);
    Ok(())
}

pub fn pop_register(ctx: &mut ExecutionContext) -> Result<(), VmFault> {
    let dest = ctx.read_register_index()?;
    let sp = ctx.cpu.get_sp();
    let b0 = ctx.mem.read(sp)? as u32;
    let b1 = ctx.mem.read(sp + 1)? as u32;
    let b2 = ctx.mem.read(sp + 2)? as u32;
    let b3 = ctx.mem.read(sp + 3)? as u32;
    let value = b0 | (b1 << 8) | (b2 << 16) | (b3 << 24);
    ctx.cpu.set_register(dest, value);
    ctx.cpu.set_sp(sp + 4);
    Ok(())
}

pub fn get_sp(ctx: &mut ExecutionContext) -> Result<(), VmFault> {
    let dest = ctx.read_register_index()?;
    ctx.cpu.set_register(dest, ctx.cpu.get_sp() as u32);
    Ok(())
}

pub fn set_sp(ctx: &mut ExecutionContext) -> Result<(), VmFault> {
    let src = ctx.read_register_index()?;
    let addr = ctx.cpu.get_register_value(src) as usize;
    ctx.cpu.set_sp(addr);
    Ok(())
}

pub fn load_dword_from_memory(ctx: &mut ExecutionContext) -> Result<(), VmFault> {
    let dest = ctx.read_register_index()?;
    let addr = ctx.read_word()? as usize;
    let b0 = ctx.mem.read(addr)? as u32;
    let b1 = ctx.mem.read(addr + 1)? as u32;
    let b2 = ctx.mem.read(addr + 2)? as u32;
    let b3 = ctx.mem.read(addr + 3)? as u32;
    ctx.cpu
        .set_register(dest, b0 | (b1 << 8) | (b2 << 16) | (b3 << 24));
    Ok(())
}

pub fn store_dword_to_memory(ctx: &mut ExecutionContext) -> Result<(), VmFault> {
    let addr = ctx.read_word()? as usize;
    let src = ctx.read_register_index()?;
    let value = ctx.cpu.get_register_value(src);
    ctx.mem.write(addr, (value & 0xFF) as u8)?;
    ctx.mem.write(addr + 1, ((value >> 8) & 0xFF) as u8)?;
    ctx.mem.write(addr + 2, ((value >> 16) & 0xFF) as u8)?;
    ctx.mem.write(addr + 3, ((value >> 24) & 0xFF) as u8)?;
    Ok(())
}

pub fn load_dword_from_memory_indirect(ctx: &mut ExecutionContext) -> Result<(), VmFault> {
    let dest = ctx.read_register_index()?;
    let addr_reg = ctx.read_register_index()?;
    let addr = ctx.cpu.get_register_value(addr_reg) as usize;
    let b0 = ctx.mem.read(addr)? as u32;
    let b1 = ctx.mem.read(addr + 1)? as u32;
    let b2 = ctx.mem.read(addr + 2)? as u32;
    let b3 = ctx.mem.read(addr + 3)? as u32;
    ctx.cpu
        .set_register(dest, b0 | (b1 << 8) | (b2 << 16) | (b3 << 24));
    Ok(())
}

pub fn store_dword_to_memory_indirect(ctx: &mut ExecutionContext) -> Result<(), VmFault> {
    let addr_reg = ctx.read_register_index()?;
    let src = ctx.read_register_index()?;
    let addr = ctx.cpu.get_register_value(addr_reg) as usize;
    let value = ctx.cpu.get_register_value(src);
    ctx.mem.write(addr, (value & 0xFF) as u8)?;
    ctx.mem.write(addr + 1, ((value >> 8) & 0xFF) as u8)?;
    ctx.mem.write(addr + 2, ((value >> 16) & 0xFF) as u8)?;
    ctx.mem.write(addr + 3, ((value >> 24) & 0xFF) as u8)?;
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

// Faults if a candidate heap-top would collide with (or pass) the live stack
// pointer — the heap grows up and the stack grows down through the same
// region, and nothing else stops one from silently corrupting the other.
pub fn check_heap_bounds(ctx: &mut ExecutionContext) -> Result<(), VmFault> {
    let candidate_reg = ctx.read_register_index()?;
    let sp_reg = ctx.read_register_index()?;
    let candidate = ctx.cpu.get_register_value(candidate_reg);
    let sp = ctx.cpu.get_register_value(sp_reg);
    if candidate >= sp {
        return Err(VmFault::HeapExhausted);
    }
    Ok(())
}
