use crate::input::Input;
use crate::screen::Screen;
use crate::settings::SPRITE_SIZE;
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

pub fn palette(vm: &mut Vm, _input: &Input, _screen: &mut Screen) {
    let index = vm.get_program()[vm.get_pc()] as usize;
    let r = vm.get_program()[vm.get_pc() + 1];
    let g = vm.get_program()[vm.get_pc() + 2];
    let b = vm.get_program()[vm.get_pc() + 3];

    info!(
        "Setting palette index {} to color ({}, {}, {})",
        index, r, g, b
    );

    vm.set_palette_color(index, r, g, b);
    vm.shift_pc(4);
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

    for sy in 0..SPRITE_SIZE {
        for sx in 0..SPRITE_SIZE {
            let pixel = vm.read_memory(base + sy as usize * SPRITE_SIZE as usize + sx as usize);
            if pixel == 0 {
                continue;
            }

            let [r, g, b] = vm.get_palette_color(pixel as usize);
            screen.set_pixel(
                (x0 + sx as u32).wrapping_sub(vm.get_camera_x()),
                (y0 + sy as u32).wrapping_sub(vm.get_camera_y()),
                r,
                g,
                b,
            );
        }
    }

    vm.shift_pc(3);
}

pub fn tilemap(vm: &mut Vm, _input: &Input, screen: &mut Screen) {
    let rx = vm.get_program()[vm.get_pc()] as usize;
    let ry = vm.get_program()[vm.get_pc() + 1] as usize;
    let rtiles = vm.get_program()[vm.get_pc() + 2] as usize;
    let rmap = vm.get_program()[vm.get_pc() + 3] as usize;
    let w = vm.get_program()[vm.get_pc() + 4] as u32;
    let h = vm.get_program()[vm.get_pc() + 5] as u32;

    let x0 = vm.get_register_value(rx) as u32;
    let y0 = vm.get_register_value(ry) as u32;
    let tiles_base = vm.get_register_value(rtiles) as usize;
    let map_base = vm.get_register_value(rmap) as usize;

    for ty in 0..h {
        for tx in 0..w {
            let map_index = (ty * w + tx) as usize;
            let tile_index = vm.read_memory(map_base + map_index) as usize;

            for sy in 0..SPRITE_SIZE {
                for sx in 0..SPRITE_SIZE {
                    let pixel = vm.read_memory(
                        tiles_base
                            + tile_index * (SPRITE_SIZE as usize * SPRITE_SIZE as usize)
                            + sy as usize * SPRITE_SIZE as usize
                            + sx as usize,
                    );
                    if pixel == 0 {
                        continue;
                    }

                    let [r, g, b] = vm.get_palette_color(pixel as usize);
                    screen.set_pixel(
                        (x0 + tx * SPRITE_SIZE + sx).wrapping_sub(vm.get_camera_x()),
                        (y0 + ty * SPRITE_SIZE + sy).wrapping_sub(vm.get_camera_y()),
                        r,
                        g,
                        b,
                    );
                }
            }
        }
    }

    vm.shift_pc(6);
}
