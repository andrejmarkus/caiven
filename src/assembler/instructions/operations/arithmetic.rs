use crate::input::Input;
use crate::rendering::screen::ScreenLayer;
use crate::vm::Vm;
use log::info;

pub fn move_value(vm: &mut Vm, _input: &Input, _world: &mut ScreenLayer, _ui: &mut ScreenLayer) {
    let reg_index = vm.get_program()[vm.get_pc()] as usize;
    let low = vm.get_program()[vm.get_pc() + 1] as u16;
    let high = vm.get_program()[vm.get_pc() + 2] as u16;
    let value = low | (high << 8);

    if reg_index < vm.get_registers_len() {
        info!("Moved value {} into register {}", value, reg_index);
        vm.set_register(reg_index, value);
    } else {
        panic!("Invalid register index: {}", reg_index);
    }
    vm.shift_pc(3);
}

pub fn add_value(vm: &mut Vm, _input: &Input, _world: &mut ScreenLayer, _ui: &mut ScreenLayer) {
    let reg_index = vm.get_program()[vm.get_pc()] as usize;
    let low = vm.get_program()[vm.get_pc() + 1] as u16;
    let high = vm.get_program()[vm.get_pc() + 2] as u16;
    let value = low | (high << 8);

    if reg_index < vm.get_registers_len() {
        info!("Added value {} to register {}", value, reg_index);
        vm.increment_register_value(reg_index, value);
    } else {
        panic!("Invalid register index: {}", reg_index);
    }
    vm.shift_pc(3);
}

pub fn decrement_value(
    vm: &mut Vm,
    _input: &Input,
    _world: &mut ScreenLayer,
    _ui: &mut ScreenLayer,
) {
    let reg_index = vm.get_program()[vm.get_pc()] as usize;

    if reg_index < vm.get_registers_len() {
        info!("Decremented register {}", reg_index);
        vm.decrement_register_value(reg_index, 1);
    } else {
        panic!("Invalid register index: {}", reg_index);
    }
    vm.shift_pc(1);
}
