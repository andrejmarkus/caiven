use crate::input::{Button, Input};
use crate::rendering::screen::ScreenLayer;
use crate::vm::Vm;
use log::debug;

pub fn input(vm: &mut Vm, input: &Input, _world: &mut ScreenLayer, _ui: &mut ScreenLayer) {
    let reg_index = vm.get_program()[vm.get_pc()] as usize;
    let button_code = vm.get_program()[vm.get_pc() + 1];

    let pressed = Button::from_u8(button_code)
        .map(|btn| input.is_pressed(btn))
        .unwrap_or(false);

    debug!(
        "Reading input for button code {} into register {}: {}",
        button_code, reg_index, pressed
    );
    vm.set_register(reg_index, if pressed { 1 } else { 0 } as u16);
    vm.shift_pc(2);
}
