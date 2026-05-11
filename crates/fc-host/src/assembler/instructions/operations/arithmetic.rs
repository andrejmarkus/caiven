use crate::vm::ExecutionContext;
use log::debug;

pub fn move_value(ctx: &mut ExecutionContext) {
    let reg_index = ctx.vm.read_register_index();
    let value = ctx.vm.read_word();

    debug!("Moved value {} into register {}", value, reg_index);
    ctx.vm.set_register(reg_index, value);
}

pub fn move_register(ctx: &mut ExecutionContext) {
    let dest_index = ctx.vm.read_register_index();
    let src_index = ctx.vm.read_register_index();
    let value = ctx.vm.get_register_value(src_index);

    debug!("Moved value R{} ({}) into register R{}", src_index, value, dest_index);
    ctx.vm.set_register(dest_index, value);
}

pub fn add_value(ctx: &mut ExecutionContext) {
    let reg_index = ctx.vm.read_register_index();
    let value = ctx.vm.read_word();

    debug!("Added value {} to register {}", value, reg_index);
    ctx.vm.increment_register_value(reg_index, value);
}

pub fn add_register(ctx: &mut ExecutionContext) {
    let dest_index = ctx.vm.read_register_index();
    let src_index = ctx.vm.read_register_index();
    let value = ctx.vm.get_register_value(src_index);

    debug!("Added register R{} ({}) to register R{}", src_index, value, dest_index);
    ctx.vm.increment_register_value(dest_index, value);
}

pub fn subtract_value(ctx: &mut ExecutionContext) {
    let reg_index = ctx.vm.read_register_index();
    let value = ctx.vm.read_word();

    debug!("Subtracted value {} from register {}", value, reg_index);
    ctx.vm.decrement_register_value(reg_index, value);
}

pub fn subtract_register(ctx: &mut ExecutionContext) {
    let dest_index = ctx.vm.read_register_index();
    let src_index = ctx.vm.read_register_index();
    let value = ctx.vm.get_register_value(src_index);

    debug!("Subtracted register R{} ({}) from register R{}", src_index, value, dest_index);
    ctx.vm.decrement_register_value(dest_index, value);
}

pub fn decrement_value(ctx: &mut ExecutionContext) {
    let reg_index = ctx.vm.read_register_index();

    debug!("Decremented register {}", reg_index);
    ctx.vm.decrement_register_value(reg_index, 1);
}

pub fn random_value(ctx: &mut ExecutionContext) {
    let reg_index = ctx.vm.read_register_index();
    let max = ctx.vm.read_word();

    let value = if max == 0 { 0 } else { (noise_rnd() % max as u32) as u16 };

    debug!("Random value {} (max {}) into register {}", value, max, reg_index);
    ctx.vm.set_register(reg_index, value);
}

pub fn set_less_than(ctx: &mut ExecutionContext) {
    let dest_index = ctx.vm.read_register_index();
    let src1_index = ctx.vm.read_register_index();
    let src2_index = ctx.vm.read_register_index();
    let val1 = ctx.vm.get_register_value(src1_index);
    let val2 = ctx.vm.get_register_value(src2_index);

    debug!("SLT R{} = (R{} < R{}) ({})", dest_index, src1_index, src2_index, val1 < val2);
    ctx.vm.set_register(dest_index, if val1 < val2 { 1 } else { 0 });
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
