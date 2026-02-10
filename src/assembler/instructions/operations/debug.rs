use crate::input::Input;
use crate::rendering::screen::ScreenLayer;
use crate::vm::Vm;
use log::info;

pub fn log_register(vm: &mut Vm, _input: &Input, _world: &mut ScreenLayer, _ui: &mut ScreenLayer) {
    let reg_index = vm.get_program()[vm.get_pc()] as usize;

    if reg_index < vm.get_registers_len() {
        let value = vm.get_register_value(reg_index);
        info!("Guest LOG: R{} = {}", reg_index, value);
    } else {
        panic!("Invalid register index: {}", reg_index);
    }
    vm.shift_pc(1);
}

pub fn log_value(vm: &mut Vm, _input: &Input, _world: &mut ScreenLayer, _ui: &mut ScreenLayer) {
    let low = vm.get_program()[vm.get_pc()] as u16;
    let high = vm.get_program()[vm.get_pc() + 1] as u16;
    let value = low | (high << 8);

    info!("Guest LOG: {}", value);
    vm.shift_pc(2);
}
