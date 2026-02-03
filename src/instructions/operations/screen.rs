use crate::input::Input;
use crate::screen::Screen;
use crate::vm::Vm;
use log::info;

pub fn clear_screen(_vm: &mut Vm, _input: &Input, screen: &mut Screen) {
    screen.clear();
}

pub fn draw_pixel(vm: &mut Vm, _input: &Input, screen: &mut Screen) {
    let x = vm.get_program()[vm.get_pc()] as u32;
    let y = vm.get_program()[vm.get_pc() + 1] as u32;
    let r = vm.get_program()[vm.get_pc() + 2];
    let g = vm.get_program()[vm.get_pc() + 3];
    let b = vm.get_program()[vm.get_pc() + 4];

    info!(
        "Drawing pixel at ({}, {}) with color ({}, {}, {})",
        x, y, r, g, b
    );
    screen.set_pixel(x, y, r, g, b);
    vm.shift_pc(5);
}

pub fn draw_pixel_from_register(vm: &mut Vm, _input: &Input, screen: &mut Screen) {
    let rx = vm.get_program()[vm.get_pc()] as usize;
    let ry = vm.get_program()[vm.get_pc() + 1] as usize;
    let r = vm.get_program()[vm.get_pc() + 2];
    let g = vm.get_program()[vm.get_pc() + 3];
    let b = vm.get_program()[vm.get_pc() + 4];

    let x = vm.get_register_value(rx) as u32;
    let y = vm.get_register_value(ry) as u32;

    info!(
        "Drawing pixel at ({}, {}) with color ({}, {}, {}) from registers r{} and r{}",
        x, y, r, g, b, rx, ry
    );
    screen.set_pixel(x, y, r, g, b);
    vm.shift_pc(5);
}

pub fn sprite(vm: &mut Vm, _input: &Input, screen: &mut Screen) {
    let rx = vm.get_program()[vm.get_pc()] as usize;
    let ry = vm.get_program()[vm.get_pc() + 1] as usize;
    let raddr = vm.get_program()[vm.get_pc() + 2] as usize;

    let x0 = vm.get_register_value(rx) as u32;
    let y0 = vm.get_register_value(ry) as u32;
    let base = vm.get_register_value(raddr) as usize;

    info!(
        "Drawing sprite {} at ({}, {}) from registers r{} and r{}",
        raddr, x0, y0, rx, ry
    );

    for sy in 0..8 {
        for sx in 0..8 {
            let pixel = vm.read_memory(base + sy * 8 + sx);
            if pixel == 0 {
                continue;
            }

            screen.set_pixel(x0 + sx as u32, y0 + sy as u32, pixel, pixel, pixel);
        }
    }

    vm.shift_pc(3);
}
