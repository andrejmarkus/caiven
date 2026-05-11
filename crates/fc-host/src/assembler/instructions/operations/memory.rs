use crate::vm::ExecutionContext;
use log::debug;

pub fn load_from_memory(ctx: &mut ExecutionContext) {
    let reg_index = ctx.vm.read_register_index();
    let address = ctx.vm.read_word() as usize;

    let value = ctx.vm.read_memory(address);

    debug!(
        "Loaded value {} from memory address {} into register {}",
        value, address, reg_index
    );
    ctx.vm.set_register(reg_index, value as u16);
}

pub fn load_word_from_memory(ctx: &mut ExecutionContext) {
    let reg_index = ctx.vm.read_register_index();
    let address = ctx.vm.read_word() as usize;

    let low = ctx.vm.read_memory(address) as u16;
    let high = ctx.vm.read_memory(address + 1) as u16;
    let value = low | (high << 8);

    debug!(
        "Loaded word {} from memory address {} into register {}",
        value, address, reg_index
    );
    ctx.vm.set_register(reg_index, value);
}

pub fn store_to_memory(ctx: &mut ExecutionContext) {
    let address = ctx.vm.read_word() as usize;
    let reg_index = ctx.vm.read_register_index();

    let value = ctx.vm.get_register_value(reg_index) as u8;

    debug!(
        "Stored value {} from register {} into memory address {}",
        value, reg_index, address
    );
    ctx.vm.write_memory(address, value);
}

pub fn store_word_to_memory(ctx: &mut ExecutionContext) {
    let address = ctx.vm.read_word() as usize;
    let reg_index = ctx.vm.read_register_index();

    let value = ctx.vm.get_register_value(reg_index);

    debug!(
        "Stored word {} from register {} into memory address {}",
        value, reg_index, address
    );
    ctx.vm.write_memory(address, (value & 0xFF) as u8);
    ctx.vm.write_memory(address + 1, ((value >> 8) & 0xFF) as u8);
}

pub fn load_from_memory_indirect(ctx: &mut ExecutionContext) {
    let reg_to_index = ctx.vm.read_register_index();
    let reg_from_index = ctx.vm.read_register_index();

    let address = ctx.vm.get_register_value(reg_from_index) as usize;
    let value = ctx.vm.read_memory(address);

    debug!(
        "Loaded value {} from memory address in register {} into register {}",
        value, reg_from_index, reg_to_index
    );
    ctx.vm.set_register(reg_to_index, value as u16);
}

pub fn store_to_memory_indirect(ctx: &mut ExecutionContext) {
    let reg_addr_index = ctx.vm.read_register_index();
    let reg_val_index = ctx.vm.read_register_index();

    let value = ctx.vm.get_register_value(reg_val_index) as u8;
    let address = ctx.vm.get_register_value(reg_addr_index) as usize;

    debug!(
        "Stored value {} from register {} into memory address in register {}",
        value, reg_val_index, reg_addr_index
    );
    ctx.vm.write_memory(address, value);
}

pub fn copy(ctx: &mut ExecutionContext) {
    let dst = ctx.vm.read_word() as usize;
    let src = ctx.vm.read_word() as usize;
    let length = ctx.vm.read_word() as usize;

    debug!(
        "Copying {} bytes from program address {} to memory address {}",
        length, src, dst
    );

    let program = ctx.vm.get_program().clone();
    for i in 0..length {
        if dst + i >= ctx.vm.get_memory_length() || src + i >= program.len() {
            break;
        }
        let value = program[src + i];
        ctx.vm.write_memory(dst + i, value);
    }
}
