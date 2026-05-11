use crate::rendering::font::Font;
use crate::settings::SPRITE_SIZE;
use fc_core::{Color, Vec2};
use crate::vm::ExecutionContext;
use log::debug;

pub fn clear_screen(ctx: &mut ExecutionContext) {
    ctx.world.clear();
    ctx.ui.clear();
}

pub fn fill_screen(ctx: &mut ExecutionContext) {
    let color_idx = ctx.vm.read_byte() as usize;
    let color = ctx.vm.get_palette_color(color_idx);

    debug!("Filling screen with palette index {}", color_idx);

    for y in 0..crate::settings::HEIGHT {
        for x in 0..crate::settings::WIDTH {
            ctx.world.set_pixel(Vec2::new(x, y), color);
        }
    }
}

pub fn draw_pixel(ctx: &mut ExecutionContext) {
    let x = ctx.vm.read_byte() as u32;
    let y = ctx.vm.read_byte() as u32;
    let r = ctx.vm.read_byte();
    let g = ctx.vm.read_byte();
    let b = ctx.vm.read_byte();

    debug!(
        "Drawing pixel at ({}, {}) with color ({}, {}, {})",
        x, y, r, g, b
    );
    ctx.world
        .set_pixel(Vec2::new(x, y), Color::new_rgb(r, g, b));
}

pub fn draw_pixel_from_register(ctx: &mut ExecutionContext) {
    let x = ctx.vm.read_register_value() as u32;
    let y = ctx.vm.read_register_value() as u32;
    let r = ctx.vm.read_byte();
    let g = ctx.vm.read_byte();
    let b = ctx.vm.read_byte();

    debug!(
        "Drawing pixel at ({}, {}) with color ({}, {}, {}) from registers",
        x, y, r, g, b
    );
    ctx.world
        .set_pixel(Vec2::new(x, y), Color::new_rgb(r, g, b));
}

pub fn palette(ctx: &mut ExecutionContext) {
    let index = ctx.vm.read_byte() as usize;
    let r = ctx.vm.read_byte();
    let g = ctx.vm.read_byte();
    let b = ctx.vm.read_byte();

    debug!(
        "Setting palette index {} to color ({}, {}, {})",
        index, r, g, b
    );

    ctx.vm.set_palette_color(index, Color::new_rgb(r, g, b));
}

pub fn sprite(ctx: &mut ExecutionContext) {
    let x0 = ctx.vm.read_register_value() as u32;
    let y0 = ctx.vm.read_register_value() as u32;
    let base = ctx.vm.read_register_value() as usize;

    debug!(
        "Drawing sprite at ({}, {}) from memory 0x{:04X}",
        x0, y0, base
    );

    for sy in 0..SPRITE_SIZE {
        for sx in 0..SPRITE_SIZE {
            let pixel = ctx
                .vm
                .read_memory(base + sy as usize * SPRITE_SIZE as usize + sx as usize);
            if pixel == 0 {
                continue;
            }

            let color = ctx.vm.get_palette_color(pixel as usize);
            ctx.world.set_pixel(
                Vec2::new(
                    (x0 + sx).wrapping_sub(ctx.vm.get_camera_x()),
                    (y0 + sy).wrapping_sub(ctx.vm.get_camera_y()),
                ),
                color,
            );
        }
    }
}

pub fn print(ctx: &mut ExecutionContext) {
    let x = ctx.vm.read_register_value() as u32;
    let y = ctx.vm.read_register_value() as u32;
    let color_idx = ctx.vm.read_register_value() as usize;
    let base = ctx.vm.read_register_value() as usize;

    let mut text = String::new();
    let mut i = 0;
    loop {
        let byte = ctx.vm.read_memory(base + i);
        if byte == 0 || text.len() > 64 {
            break;
        }
        text.push(byte as char);
        i += 1;
    }

    debug!("Printing text \"{}\" at ({}, {})", text, x, y);
    ctx.vm.draw_text(ctx.ui, &text, x, y, color_idx);
}

pub fn draw_tile(ctx: &mut ExecutionContext) {
    let x0 = ctx.vm.read_register_value() as u32;
    let y0 = ctx.vm.read_register_value() as u32;
    let base = ctx.vm.read_register_value() as usize;

    for sy in 0..SPRITE_SIZE {
        for sx in 0..SPRITE_SIZE {
            let pixel = ctx
                .vm
                .read_memory(base + sy as usize * SPRITE_SIZE as usize + sx as usize);
            if pixel == 0 {
                continue;
            }

            let color = ctx.vm.get_palette_color(pixel as usize);
            ctx.world.set_pixel(
                Vec2::new(
                    (x0 * SPRITE_SIZE + sx).wrapping_sub(ctx.vm.get_camera_x()),
                    (y0 * SPRITE_SIZE + sy).wrapping_sub(ctx.vm.get_camera_y()),
                ),
                color,
            );
        }
    }
}

pub fn tilemap(ctx: &mut ExecutionContext) {
    let x0 = ctx.vm.read_register_value() as u32;
    let y0 = ctx.vm.read_register_value() as u32;
    let tiles_base = ctx.vm.read_register_value() as usize;
    let map_base = ctx.vm.read_register_value() as usize;
    let w = ctx.vm.read_byte() as u32;
    let h = ctx.vm.read_byte() as u32;

    debug!("Drawing tilemap at ({}, {}) with size {}x{}", x0, y0, w, h);

    for ty in 0..h {
        for tx in 0..w {
            let map_index = (ty * w + tx) as usize;
            let tile_index = ctx.vm.read_memory(map_base + map_index) as usize;

            for sy in 0..SPRITE_SIZE {
                for sx in 0..SPRITE_SIZE {
                    let pixel = ctx.vm.read_memory(
                        tiles_base
                            + tile_index * (SPRITE_SIZE as usize * SPRITE_SIZE as usize)
                            + sy as usize * SPRITE_SIZE as usize
                            + sx as usize,
                    );
                    if pixel == 0 {
                        continue;
                    }

                    let color = ctx.vm.get_palette_color(pixel as usize);
                    ctx.world.set_pixel(
                        Vec2::new(
                            (x0 + tx * SPRITE_SIZE + sx).wrapping_sub(ctx.vm.get_camera_x()),
                            (y0 + ty * SPRITE_SIZE + sy).wrapping_sub(ctx.vm.get_camera_y()),
                        ),
                        color,
                    );
                }
            }
        }
    }
}

pub fn tile_at(ctx: &mut ExecutionContext) {
    let rdest = ctx.vm.read_register_index();
    let x = ctx.vm.read_register_value() as u32;
    let y = ctx.vm.read_register_value() as u32;
    let map_base = ctx.vm.read_register_value() as usize;
    let w = ctx.vm.read_byte() as u32;

    let tx = x / SPRITE_SIZE;
    let ty = y / SPRITE_SIZE;
    let map_index = (ty * w + tx) as usize;
    let tile_index = ctx.vm.read_memory(map_base + map_index);

    debug!(
        "Getting tile index at ({}, {}) -> tile ({}, {}) with index {}",
        x, y, tx, ty, tile_index
    );

    ctx.vm.set_register(rdest, tile_index as u16);
}

pub fn tile_solid(ctx: &mut ExecutionContext) {
    let rdest = ctx.vm.read_register_index();
    let tile_index = ctx.vm.read_register_value() as usize;
    let flags_base = ctx.vm.read_register_value() as usize;

    let flags = ctx.vm.read_memory(flags_base + tile_index);

    debug!(
        "Getting solidity of tile {} with flags at {} -> flags {}",
        tile_index, flags_base, flags
    );

    ctx.vm
        .set_register(rdest, if flags & 1 != 0 { 1 } else { 0 });
}

pub fn text(ctx: &mut ExecutionContext) {
    let x0 = ctx.vm.read_register_value() as u32;
    let y = ctx.vm.read_register_value() as u32;
    let color_idx = ctx.vm.read_register_value() as usize;
    let base = ctx.vm.read_register_value() as usize;
    let len = ctx.vm.read_byte() as usize;

    let font = Font::get_global();
    let color = ctx.vm.get_palette_color(color_idx);
    let mut current_x = x0;

    for i in 0..len {
        let ch = ctx.vm.read_memory(base + i) as char;

        if let Some(glyph) = font.get_glyph(ch) {
            for gy in 0..font.get_height() {
                for gx in 0..font.get_width() {
                    if glyph.pixels[gy * font.get_width() + gx] {
                        ctx.ui
                            .set_pixel(Vec2::new(current_x + gx as u32, y + gy as u32), color);
                    }
                }
            }
        }
        current_x += font.get_width() as u32 + 1;
    }
}

pub fn draw_number(ctx: &mut ExecutionContext) {
    let x0 = ctx.vm.read_register_value() as u32;
    let y = ctx.vm.read_register_value() as u32;
    let color_idx = ctx.vm.read_register_value() as usize;
    let value = ctx.vm.read_register_value();

    let font = Font::get_global();
    let color = ctx.vm.get_palette_color(color_idx);
    let mut current_x = x0;

    let s = value.to_string();
    for ch in s.chars() {
        if let Some(glyph) = font.get_glyph(ch) {
            for gy in 0..font.get_height() {
                for gx in 0..font.get_width() {
                    if glyph.pixels[gy * font.get_width() + gx] {
                        ctx.ui
                            .set_pixel(Vec2::new(current_x + gx as u32, y + gy as u32), color);
                    }
                }
            }
        }
        current_x += font.get_width() as u32 + 1;
    }
}
