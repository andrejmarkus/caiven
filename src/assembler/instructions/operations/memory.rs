use crate::input::Input;
use crate::rendering::screen::ScreenLayer;
use crate::vm::Vm;
use log::debug;

pub fn load_from_memory(
    vm: &mut Vm,
    _input: &Input,
    _world: &mut ScreenLayer,
    _ui: &mut ScreenLayer,
) {
    let reg_index = vm.get_program()[vm.get_pc()] as usize;
    let low = vm.get_program()[vm.get_pc() + 1] as u16;
    let high = vm.get_program()[vm.get_pc() + 2] as u16;
    let address = (low | (high << 8)) as usize;

    let value = vm.read_memory(address);

    debug!(
        "Loaded value {} from memory address {} into register {}",
        value, address, reg_index
    );
    vm.set_register(reg_index, value as u16);
    vm.shift_pc(3);
}

pub fn store_to_memory(
    vm: &mut Vm,
    _input: &Input,
    _world: &mut ScreenLayer,
    _ui: &mut ScreenLayer,
) {
    let low = vm.get_program()[vm.get_pc()] as u16;
    let high = vm.get_program()[vm.get_pc() + 1] as u16;
    let address = (low | (high << 8)) as usize;
    let reg_index = vm.get_program()[vm.get_pc() + 2] as usize;

    let value = vm.get_register_value(reg_index) as u8;

    debug!(
        "Stored value {} from register {} into memory address {}",
        value, reg_index, address
    );
    vm.write_memory(address, value);
    vm.shift_pc(3);
}

pub fn load_from_memory_indirect(
    vm: &mut Vm,
    _input: &Input,
    _world: &mut ScreenLayer,
    _ui: &mut ScreenLayer,
) {
    let reg_to_index = vm.get_program()[vm.get_pc()] as usize;
    let reg_from_index = vm.get_program()[vm.get_pc() + 1] as usize;

    let address = vm.get_register_value(reg_from_index) as usize;
    let value = vm.read_memory(address);

    debug!(
        "Loaded value {} from memory address in register {} into register {}",
        value, reg_from_index, reg_to_index
    );
    vm.set_register(reg_to_index, value as u16);
    vm.shift_pc(2);
}

pub fn store_to_memory_indirect(
    vm: &mut Vm,
    _input: &Input,
    _world: &mut ScreenLayer,
    _ui: &mut ScreenLayer,
) {
    let reg_addr_index = vm.get_program()[vm.get_pc()] as usize;
    let reg_val_index = vm.get_program()[vm.get_pc() + 1] as usize;

    let value = vm.get_register_value(reg_val_index) as u8;
    let address = vm.get_register_value(reg_addr_index) as usize;

    debug!(
        "Stored value {} from register {} into memory address in register {}",
        value, reg_val_index, reg_addr_index
    );
    vm.write_memory(address, value);
    vm.shift_pc(2);
}

pub fn copy(vm: &mut Vm, _input: &Input, _world: &mut ScreenLayer, _ui: &mut ScreenLayer) {
    let dst_lo = vm.get_program()[vm.get_pc()] as usize;
    let dst_hi = vm.get_program()[vm.get_pc() + 1] as usize;
    let src_lo = vm.get_program()[vm.get_pc() + 2] as usize;
    let src_hi = vm.get_program()[vm.get_pc() + 3] as usize;
    let len_lo = vm.get_program()[vm.get_pc() + 4] as usize;
    let len_hi = vm.get_program()[vm.get_pc() + 5] as usize;

    let dst = dst_lo | (dst_hi << 8);
    let src = src_lo | (src_hi << 8);
    let length = len_lo | (len_hi << 8);

    debug!(
        "Copying {} bytes from program address {} to memory address {}",
        length, src, dst
    );

    for i in 0..length {
        if dst + i >= vm.get_memory_length() || src + i >= vm.get_program().len() {
            break;
        }
        let value = vm.get_program()[src + i];
        vm.write_memory(dst + i, value);
    }
    vm.shift_pc(6);
}
