use crate::input::Input;
use crate::rendering::screen::ScreenLayer;
use crate::vm::Vm;

pub fn set_camera_position(vm: &mut Vm, _input: &Input, _world: &mut ScreenLayer, _ui: &mut ScreenLayer) {
    let rx = vm.get_program()[vm.get_pc()] as usize;
    let ry = vm.get_program()[vm.get_pc() + 1] as usize;

    let x = vm.get_register_value(rx) as u32;
    let y = vm.get_register_value(ry) as u32;

    vm.set_camera_position(x, y);
}

pub fn move_camera(vm: &mut Vm, _input: &Input, _world: &mut ScreenLayer, _ui: &mut ScreenLayer) {
    let rx = vm.get_program()[vm.get_pc()] as usize;
    let ry = vm.get_program()[vm.get_pc() + 1] as usize;

    let dx = vm.get_register_value(rx) as i32;
    let dy = vm.get_register_value(ry) as i32;

    vm.move_camera_by(dx, dy);
}
