use crate::input::Input;
use crate::rendering::screen::ScreenLayer;
use crate::vm::Vm;
use log::info;

fn read_address(low: u16, high: u16) -> u16 {
    low | (high << 8)
}

pub fn jump(vm: &mut Vm, _input: &Input, _layer: &mut ScreenLayer) {
    let low = vm.get_program()[vm.get_pc()] as u16;
    let high = vm.get_program()[vm.get_pc() + 1] as u16;
    let address = read_address(low, high);

    info!("Jumping to address {}", address);
    vm.set_pc(address as usize);
}

pub fn jump_if_not_zero(vm: &mut Vm, _input: &Input, _layer: &mut ScreenLayer) {
    let reg_index = vm.get_program()[vm.get_pc()] as usize;
    let low = vm.get_program()[vm.get_pc() + 1] as u16;
    let high = vm.get_program()[vm.get_pc() + 2] as u16;
    let address = read_address(low, high);

    let reg_value = vm.get_register_value(reg_index);
    info!(
        "JNZ check: register {} value is {}, jumping to {} if not zero",
        reg_index, reg_value, address
    );
    if reg_value != 0 {
        vm.set_pc(address as usize);
    } else {
        vm.shift_pc(3);
    }
}

pub fn jump_if_zero(vm: &mut Vm, _input: &Input, _layer: &mut ScreenLayer) {
    let reg_index = vm.get_program()[vm.get_pc()] as usize;
    let low = vm.get_program()[vm.get_pc() + 1] as u16;
    let high = vm.get_program()[vm.get_pc() + 2] as u16;
    let address = read_address(low, high);

    let reg_value = vm.get_register_value(reg_index);
    info!(
        "JZ check: register {} value is {}, jumping to {} if zero",
        reg_index, reg_value, address
    );
    if reg_value == 0 {
        vm.set_pc(address as usize);
    } else {
        vm.shift_pc(3);
    }
}

pub fn wait(vm: &mut Vm, _input: &Input, _layer: &mut ScreenLayer) {
    info!("Waiting for next frame");
    vm.pause();
}
