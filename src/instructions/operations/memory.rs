use crate::input::Input;
use crate::screen::Screen;
use crate::vm::Vm;
use log::info;

pub fn load_from_memory(vm: &mut Vm, _input: &Input, _screen: &mut Screen) {
    let reg_index = vm.get_program()[vm.get_pc()] as usize;
    let address = vm.get_program()[vm.get_pc() + 1] as usize;

    let value = vm.read_memory(address);

    info!(
        "Loaded value {} from memory address {} into register {}",
        value, address, reg_index
    );
    vm.set_register(reg_index, value);
    vm.shift_pc(2);
}

pub fn store_to_memory(vm: &mut Vm, _input: &Input, _screen: &mut Screen) {
    let address = vm.get_program()[vm.get_pc()] as usize;
    let reg_index = vm.get_program()[vm.get_pc() + 1] as usize;

    let value = vm.get_register_value(reg_index);

    info!(
        "Stored value {} from register {} into memory address {}",
        value, reg_index, address
    );
    vm.write_memory(address, value);
    vm.shift_pc(2);
}

pub fn load_from_memory_indirect(vm: &mut Vm, _input: &Input, _screen: &mut Screen) {
    let reg_to_index = vm.get_program()[vm.get_pc()] as usize;
    let reg_from_index = vm.get_program()[vm.get_pc() + 1] as usize;

    let address = vm.get_register_value(reg_from_index) as usize;
    let value = vm.read_memory(address);

    info!(
        "Loaded value {} from memory address in register {} into register {}",
        value, reg_from_index, reg_to_index
    );
    vm.set_register(reg_to_index, value);
    vm.shift_pc(2);
}

pub fn store_to_memory_indirect(vm: &mut Vm, _input: &Input, _screen: &mut Screen) {
    let reg_addr_index = vm.get_program()[vm.get_pc()] as usize;
    let reg_val_index = vm.get_program()[vm.get_pc() + 1] as usize;

    let value = vm.get_register_value(reg_val_index);
    let address = vm.get_register_value(reg_addr_index) as usize;

    info!(
        "Stored value {} from register {} into memory address in register {}",
        value, reg_val_index, reg_addr_index
    );
    vm.write_memory(address, value);
    vm.shift_pc(2);
}
