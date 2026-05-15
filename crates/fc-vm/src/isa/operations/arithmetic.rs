use crate::vm::{ExecutionContext, VmFault};
use log::debug;

pub fn move_value(ctx: &mut ExecutionContext) -> Result<(), VmFault> {
    let reg_index = ctx.read_register_index()?;
    let value = ctx.read_word()?;

    debug!("Moved value {} into register {}", value, reg_index);
    ctx.cpu.set_register(reg_index, value as u32);
    Ok(())
}

pub fn move_register(ctx: &mut ExecutionContext) -> Result<(), VmFault> {
    let dest_index = ctx.read_register_index()?;
    let src_index = ctx.read_register_index()?;
    let value = ctx.cpu.get_register_value(src_index);

    debug!(
        "Moved value R{} ({}) into register R{}",
        src_index, value, dest_index
    );
    ctx.cpu.set_register(dest_index, value);
    Ok(())
}

pub fn add_value(ctx: &mut ExecutionContext) -> Result<(), VmFault> {
    let reg_index = ctx.read_register_index()?;
    let value = ctx.read_word()?;

    debug!("Added value {} to register {}", value, reg_index);
    ctx.cpu.increment_register_value(reg_index, value as u32);
    Ok(())
}

pub fn add_register(ctx: &mut ExecutionContext) -> Result<(), VmFault> {
    let dest_index = ctx.read_register_index()?;
    let src_index = ctx.read_register_index()?;
    let value = ctx.cpu.get_register_value(src_index);

    debug!(
        "Added register R{} ({}) to register R{}",
        src_index, value, dest_index
    );
    ctx.cpu.increment_register_value(dest_index, value);
    Ok(())
}

pub fn subtract_value(ctx: &mut ExecutionContext) -> Result<(), VmFault> {
    let reg_index = ctx.read_register_index()?;
    let value = ctx.read_word()?;

    debug!("Subtracted value {} from register {}", value, reg_index);
    ctx.cpu.decrement_register_value(reg_index, value as u32);
    Ok(())
}

pub fn subtract_register(ctx: &mut ExecutionContext) -> Result<(), VmFault> {
    let dest_index = ctx.read_register_index()?;
    let src_index = ctx.read_register_index()?;
    let value = ctx.cpu.get_register_value(src_index);

    debug!(
        "Subtracted register R{} ({}) from register R{}",
        src_index, value, dest_index
    );
    ctx.cpu.decrement_register_value(dest_index, value);
    Ok(())
}

pub fn decrement_value(ctx: &mut ExecutionContext) -> Result<(), VmFault> {
    let reg_index = ctx.read_register_index()?;

    debug!("Decremented register {}", reg_index);
    ctx.cpu.decrement_register_value(reg_index, 1);
    Ok(())
}

pub fn random_value(ctx: &mut ExecutionContext) -> Result<(), VmFault> {
    let reg_index = ctx.read_register_index()?;
    let max = ctx.read_word()? as u32;

    let value = if max == 0 { 0 } else { noise_rnd() % max };

    debug!(
        "Random value {} (max {}) into register {}",
        value, max, reg_index
    );
    ctx.cpu.set_register(reg_index, value);
    Ok(())
}

pub fn set_less_than(ctx: &mut ExecutionContext) -> Result<(), VmFault> {
    let dest_index = ctx.read_register_index()?;
    let src1_index = ctx.read_register_index()?;
    let src2_index = ctx.read_register_index()?;
    let val1 = ctx.cpu.get_register_value(src1_index);
    let val2 = ctx.cpu.get_register_value(src2_index);

    debug!(
        "SLT R{} = (R{} < R{}) ({})",
        dest_index,
        src1_index,
        src2_index,
        val1 < val2
    );
    ctx.cpu
        .set_register(dest_index, if val1 < val2 { 1 } else { 0 });
    Ok(())
}

pub fn mul_register(ctx: &mut ExecutionContext) -> Result<(), VmFault> {
    let dest = ctx.read_register_index()?;
    let src = ctx.read_register_index()?;
    let a = ctx.cpu.get_register_value(dest);
    let b = ctx.cpu.get_register_value(src);
    ctx.cpu.set_register(dest, a.wrapping_mul(b));
    Ok(())
}

pub fn div_register(ctx: &mut ExecutionContext) -> Result<(), VmFault> {
    let dest = ctx.read_register_index()?;
    let src = ctx.read_register_index()?;
    let a = ctx.cpu.get_register_value(dest) as i32;
    let b = ctx.cpu.get_register_value(src) as i32;
    if b == 0 {
        return Err(VmFault::DivisionByZero);
    }
    ctx.cpu.set_register(dest, (a / b) as u32);
    Ok(())
}

pub fn mod_register(ctx: &mut ExecutionContext) -> Result<(), VmFault> {
    let dest = ctx.read_register_index()?;
    let src = ctx.read_register_index()?;
    let a = ctx.cpu.get_register_value(dest) as i32;
    let b = ctx.cpu.get_register_value(src) as i32;
    if b == 0 {
        return Err(VmFault::DivisionByZero);
    }
    ctx.cpu.set_register(dest, (a % b) as u32);
    Ok(())
}

/// 16:16 fixed-point multiply: (a * b) >> 16
pub fn fmul_register(ctx: &mut ExecutionContext) -> Result<(), VmFault> {
    let dest = ctx.read_register_index()?;
    let src = ctx.read_register_index()?;
    let a = ctx.cpu.get_register_value(dest) as i64;
    let b = ctx.cpu.get_register_value(src) as i64;
    let result = ((a * b) >> 16) as u32;
    ctx.cpu.set_register(dest, result);
    Ok(())
}

/// 16:16 fixed-point divide: (a << 16) / b
pub fn fdiv_register(ctx: &mut ExecutionContext) -> Result<(), VmFault> {
    let dest = ctx.read_register_index()?;
    let src = ctx.read_register_index()?;
    let a = ctx.cpu.get_register_value(dest) as i64;
    let b = ctx.cpu.get_register_value(src) as i64;
    if b == 0 {
        return Err(VmFault::DivisionByZero);
    }
    let result = ((a << 16) / b) as u32;
    ctx.cpu.set_register(dest, result);
    Ok(())
}

pub fn and_register(ctx: &mut ExecutionContext) -> Result<(), VmFault> {
    let dest = ctx.read_register_index()?;
    let src = ctx.read_register_index()?;
    let result = ctx.cpu.get_register_value(dest) & ctx.cpu.get_register_value(src);
    ctx.cpu.set_register(dest, result);
    Ok(())
}

pub fn or_register(ctx: &mut ExecutionContext) -> Result<(), VmFault> {
    let dest = ctx.read_register_index()?;
    let src = ctx.read_register_index()?;
    let result = ctx.cpu.get_register_value(dest) | ctx.cpu.get_register_value(src);
    ctx.cpu.set_register(dest, result);
    Ok(())
}

pub fn xor_register(ctx: &mut ExecutionContext) -> Result<(), VmFault> {
    let dest = ctx.read_register_index()?;
    let src = ctx.read_register_index()?;
    let result = ctx.cpu.get_register_value(dest) ^ ctx.cpu.get_register_value(src);
    ctx.cpu.set_register(dest, result);
    Ok(())
}

pub fn not_register(ctx: &mut ExecutionContext) -> Result<(), VmFault> {
    let dest = ctx.read_register_index()?;
    let result = !ctx.cpu.get_register_value(dest);
    ctx.cpu.set_register(dest, result);
    Ok(())
}

pub fn shl_register(ctx: &mut ExecutionContext) -> Result<(), VmFault> {
    let dest = ctx.read_register_index()?;
    let shift = ctx.read_byte()?;
    let result = ctx.cpu.get_register_value(dest) << (shift & 31);
    ctx.cpu.set_register(dest, result);
    Ok(())
}

pub fn shr_register(ctx: &mut ExecutionContext) -> Result<(), VmFault> {
    let dest = ctx.read_register_index()?;
    let shift = ctx.read_byte()?;
    let result = ctx.cpu.get_register_value(dest) >> (shift & 31);
    ctx.cpu.set_register(dest, result);
    Ok(())
}

pub fn sar_register(ctx: &mut ExecutionContext) -> Result<(), VmFault> {
    let dest = ctx.read_register_index()?;
    let shift = ctx.read_byte()?;
    let result = (ctx.cpu.get_register_value(dest) as i32) >> (shift & 31);
    ctx.cpu.set_register(dest, result as u32);
    Ok(())
}

pub fn neg_register(ctx: &mut ExecutionContext) -> Result<(), VmFault> {
    let dest = ctx.read_register_index()?;
    let result = (ctx.cpu.get_register_value(dest) as i32).wrapping_neg() as u32;
    ctx.cpu.set_register(dest, result);
    Ok(())
}

pub fn set_less_than_signed(ctx: &mut ExecutionContext) -> Result<(), VmFault> {
    let dest = ctx.read_register_index()?;
    let s1 = ctx.read_register_index()?;
    let s2 = ctx.read_register_index()?;
    let a = ctx.cpu.get_register_value(s1) as i32;
    let b = ctx.cpu.get_register_value(s2) as i32;
    ctx.cpu.set_register(dest, if a < b { 1 } else { 0 });
    Ok(())
}

pub fn set_equal(ctx: &mut ExecutionContext) -> Result<(), VmFault> {
    let dest = ctx.read_register_index()?;
    let s1 = ctx.read_register_index()?;
    let s2 = ctx.read_register_index()?;
    let a = ctx.cpu.get_register_value(s1);
    let b = ctx.cpu.get_register_value(s2);
    ctx.cpu.set_register(dest, if a == b { 1 } else { 0 });
    Ok(())
}

pub fn move_value_32(ctx: &mut ExecutionContext) -> Result<(), VmFault> {
    let dest = ctx.read_register_index()?;
    let value = ctx.read_dword()?;
    ctx.cpu.set_register(dest, value);
    Ok(())
}

fn noise_rnd() -> u32 {
    use std::sync::atomic::{AtomicU32, Ordering};
    static SEED: AtomicU32 = AtomicU32::new(12345);
    let mut x = SEED.load(Ordering::Relaxed);
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    SEED.store(x, Ordering::Relaxed);
    x
}
