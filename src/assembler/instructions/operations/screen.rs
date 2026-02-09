use crate::input::Input;
use crate::rendering::font::Font;
use crate::rendering::screen::ScreenLayer;
use crate::settings::SPRITE_SIZE;
use crate::utils::{Color, Vec2};
use crate::vm::Vm;
use log::info;

pub fn clear_screen(_vm: &mut Vm, _input: &Input, world: &mut ScreenLayer, ui: &mut ScreenLayer) {
    world.clear();
    ui.clear();
}

pub fn draw_pixel(vm: &mut Vm, _input: &Input, world: &mut ScreenLayer, _ui: &mut ScreenLayer) {
    let x = vm.get_program()[vm.get_pc()] as u32;
    let y = vm.get_program()[vm.get_pc() + 1] as u32;
    let r = vm.get_program()[vm.get_pc() + 2];
    let g = vm.get_program()[vm.get_pc() + 3];
    let b = vm.get_program()[vm.get_pc() + 4];

    info!(
        "Drawing pixel at ({}, {}) with color ({}, {}, {})",
        x, y, r, g, b
    );
    world.set_pixel(Vec2::new(x, y), Color::new_rgb(r, g, b));
    vm.shift_pc(5);
}

pub fn draw_pixel_from_register(
    vm: &mut Vm,
    _input: &Input,
    world: &mut ScreenLayer,
    _ui: &mut ScreenLayer,
) {
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
    world.set_pixel(Vec2::new(x, y), Color::new_rgb(r, g, b));
    vm.shift_pc(5);
}

pub fn palette(vm: &mut Vm, _input: &Input, _world: &mut ScreenLayer, _ui: &mut ScreenLayer) {
    let index = vm.get_program()[vm.get_pc()] as usize;
    let r = vm.get_program()[vm.get_pc() + 1];
    let g = vm.get_program()[vm.get_pc() + 2];
    let b = vm.get_program()[vm.get_pc() + 3];

    info!(
        "Setting palette index {} to color ({}, {}, {})",
        index, r, g, b
    );

    vm.set_palette_color(index, Color::new_rgb(r, g, b));
    vm.shift_pc(4);
}

pub fn sprite(vm: &mut Vm, _input: &Input, world: &mut ScreenLayer, _ui: &mut ScreenLayer) {
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

            let color = vm.get_palette_color(pixel as usize);
            world.set_pixel(
                Vec2::new(
                    (x0 + sx).wrapping_sub(vm.get_camera_x()),
                    (y0 + sy).wrapping_sub(vm.get_camera_y()),
                ),
                color,
            );
        }
    }

    vm.shift_pc(3);
}

pub fn print(vm: &mut Vm, _input: &Input, _world: &mut ScreenLayer, ui: &mut ScreenLayer) {
    let rx = vm.get_program()[vm.get_pc()] as usize;
    let ry = vm.get_program()[vm.get_pc() + 1] as usize;
    let rcolor = vm.get_program()[vm.get_pc() + 2] as usize;
    let raddr = vm.get_program()[vm.get_pc() + 3] as usize;

    let x = vm.get_register_value(rx) as u32;
    let y = vm.get_register_value(ry) as u32;
    let color_idx = vm.get_register_value(rcolor) as usize;
    let addr = vm.get_register_value(raddr) as usize;

    let mut text = String::new();
    let mut current_addr = addr;
    loop {
        let ch = vm.read_memory(current_addr);
        if ch == 0 || text.len() > 64 {
            break;
        }
        text.push(ch as char);
        current_addr += 1;
    }

    info!(
        "Printing text \"{}\" at ({}, {}) with color index {}",
        text, x, y, color_idx
    );

    vm.draw_text(ui, &text, x, y, color_idx);
    vm.shift_pc(4);
}

pub fn tilemap(vm: &mut Vm, _input: &Input, world: &mut ScreenLayer, _ui: &mut ScreenLayer) {
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

    info!(
        "Drawing tilemap at ({}, {}) with size {}x{} from registers r{}, r{}, r{}, r{}",
        x0, y0, w, h, rx, ry, rtiles, rmap
    );

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

                    let color = vm.get_palette_color(pixel as usize);
                    world.set_pixel(
                        Vec2::new(
                            (x0 + tx * SPRITE_SIZE + sx).wrapping_sub(vm.get_camera_x()),
                            (y0 + ty * SPRITE_SIZE + sy).wrapping_sub(vm.get_camera_y()),
                        ),
                        color,
                    );
                }
            }
        }
    }

    vm.shift_pc(6);
}

pub fn tile_at(vm: &mut Vm, _input: &Input, _world: &mut ScreenLayer, _ui: &mut ScreenLayer) {
    let rdest = vm.get_program()[vm.get_pc()] as usize;
    let rx = vm.get_program()[vm.get_pc() + 1] as usize;
    let ry = vm.get_program()[vm.get_pc() + 2] as usize;
    let rmap = vm.get_program()[vm.get_pc() + 3] as usize;
    let w = vm.get_program()[vm.get_pc() + 4] as u32;

    let x = vm.get_register_value(rx) as u32;
    let y = vm.get_register_value(ry) as u32;
    let map_base = vm.get_register_value(rmap) as usize;

    let tx = x / SPRITE_SIZE;
    let ty = y / SPRITE_SIZE;
    let map_index = (ty * w + tx) as usize;
    let tile_index = vm.read_memory(map_base + map_index);

    info!(
        "Getting tile index at ({}, {}) -> tile ({}, {}) with index {}",
        x, y, tx, ty, tile_index
    );

    vm.set_register(rdest, tile_index as u16);
    vm.shift_pc(5);
}

pub fn tile_solid(vm: &mut Vm, _input: &Input, _world: &mut ScreenLayer, _ui: &mut ScreenLayer) {
    let rdest = vm.get_program()[vm.get_pc()] as usize;
    let rtile = vm.get_program()[vm.get_pc() + 1] as usize;
    let rflags = vm.get_program()[vm.get_pc() + 2] as usize;

    let tile_index = vm.get_register_value(rtile) as usize;
    let flags_base = vm.get_register_value(rflags) as usize;

    let flags = vm.read_memory(flags_base + tile_index);

    info!(
        "Getting solidity of tile {} with flags at {} -> flags {}",
        tile_index, flags_base, flags
    );

    vm.set_register(rdest, if flags & 1 != 0 { 1 } else { 0 });
    vm.shift_pc(3);
}

pub fn text(vm: &mut Vm, _input: &Input, _world: &mut ScreenLayer, ui: &mut ScreenLayer) {
    let rx = vm.get_program()[vm.get_pc()] as usize;
    let ry = vm.get_program()[vm.get_pc() + 1] as usize;
    let rcolor = vm.get_program()[vm.get_pc() + 2] as usize;
    let rstr = vm.get_program()[vm.get_pc() + 3] as usize;
    let len = vm.get_program()[vm.get_pc() + 4] as usize;

    let x0 = vm.get_register_value(rx) as u32;
    let y = vm.get_register_value(ry) as u32;
    let color_idx = vm.get_register_value(rcolor) as usize;
    let base = vm.get_register_value(rstr) as usize;

    let font = Font::get_global();
    let color = vm.get_palette_color(color_idx);
    let mut text = String::new();
    let mut current_x = x0;

    for i in 0..len {
        let ch = vm.read_memory(base + i) as char;
        text.push(ch);

        if let Some(glyph) = font.get_glyph(ch) {
            for gy in 0..font.get_height() {
                for gx in 0..font.get_width() {
                    if glyph.pixels[gy * font.get_width() + gx] {
                        ui.set_pixel(Vec2::new(current_x + gx as u32, y + gy as u32), color);
                    }
                }
            }
        }

        current_x += font.get_width() as u32 + 1;
    }

    info!(
        "Printing text \"{}\" at ({}, {}) with color index {}",
        text, x0, y, color_idx
    );

    // Removed buggy vm.draw_text call that used incremented x
    vm.shift_pc(5);
}
