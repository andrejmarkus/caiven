use crate::rendering::text::draw_text;
use crate::vm::{ExecutionContext, VmFault};
use fc_core::{Color, Vec2};
use log::debug;

pub fn clear_screen(ctx: &mut ExecutionContext) -> Result<(), VmFault> {
    ctx.world.clear();
    ctx.ui.clear();
    Ok(())
}

pub fn fill_screen(ctx: &mut ExecutionContext) -> Result<(), VmFault> {
    let color_idx = ctx.read_byte()? as usize;
    let color = ctx.palette.get_color(color_idx);

    debug!("Filling screen with palette index {}", color_idx);

    for y in 0..ctx.config.height {
        for x in 0..ctx.config.width {
            ctx.world.set_pixel(Vec2::new(x, y), color);
        }
    }
    Ok(())
}

pub fn draw_pixel(ctx: &mut ExecutionContext) -> Result<(), VmFault> {
    let x = ctx.read_byte()? as u32;
    let y = ctx.read_byte()? as u32;
    let r = ctx.read_byte()?;
    let g = ctx.read_byte()?;
    let b = ctx.read_byte()?;

    debug!(
        "Drawing pixel at ({}, {}) with color ({}, {}, {})",
        x, y, r, g, b
    );
    ctx.world
        .set_pixel(Vec2::new(x, y), Color::new_rgb(r, g, b));
    Ok(())
}

pub fn draw_pixel_from_register(ctx: &mut ExecutionContext) -> Result<(), VmFault> {
    let x = ctx.read_register_value()? as u32;
    let y = ctx.read_register_value()? as u32;
    let r = ctx.read_byte()?;
    let g = ctx.read_byte()?;
    let b = ctx.read_byte()?;

    debug!(
        "Drawing pixel at ({}, {}) with color ({}, {}, {}) from registers",
        x, y, r, g, b
    );
    ctx.world
        .set_pixel(Vec2::new(x, y), Color::new_rgb(r, g, b));
    Ok(())
}

pub fn palette(ctx: &mut ExecutionContext) -> Result<(), VmFault> {
    let index = ctx.read_byte()? as usize;
    let r = ctx.read_byte()?;
    let g = ctx.read_byte()?;
    let b = ctx.read_byte()?;

    debug!(
        "Setting palette index {} to color ({}, {}, {})",
        index, r, g, b
    );

    ctx.palette.set_color(index, Color::new_rgb(r, g, b));
    Ok(())
}

pub fn sprite(ctx: &mut ExecutionContext) -> Result<(), VmFault> {
    let x0 = ctx.read_register_value()? as u32;
    let y0 = ctx.read_register_value()? as u32;
    let base = ctx.read_register_value()? as usize;

    debug!(
        "Drawing sprite at ({}, {}) from memory 0x{:04X}",
        x0, y0, base
    );

    let cam_x = ctx.camera.get_x();
    let cam_y = ctx.camera.get_y();

    let ss = ctx.config.sprite_size;
    for sy in 0..ss {
        for sx in 0..ss {
            let pixel = ctx
                .mem
                .read(base + sy as usize * ss as usize + sx as usize)?;
            if pixel == 0 {
                continue;
            }

            let color = ctx.palette.get_color(pixel as usize);
            ctx.world.set_pixel(
                Vec2::new((x0 + sx).wrapping_sub(cam_x), (y0 + sy).wrapping_sub(cam_y)),
                color,
            );
        }
    }
    Ok(())
}

pub fn print(ctx: &mut ExecutionContext) -> Result<(), VmFault> {
    let x = ctx.read_register_value()? as u32;
    let y = ctx.read_register_value()? as u32;
    let color_idx = ctx.read_register_value()? as usize;
    let base = ctx.read_register_value()? as usize;

    let mut text = String::new();
    let mut i = 0;
    loop {
        let byte = ctx.mem.read(base + i)?;
        if byte == 0 || text.len() >= 64 {
            break;
        }
        text.push(if byte < 128 { byte as char } else { '?' });
        i += 1;
    }

    debug!("Printing text \"{}\" at ({}, {})", text, x, y);
    let color = ctx.palette.get_color(color_idx);
    draw_text(ctx.font, ctx.ui, &text, Vec2::new(x, y), color);
    Ok(())
}

pub fn tilemap(ctx: &mut ExecutionContext) -> Result<(), VmFault> {
    let x0 = ctx.read_register_value()? as u32;
    let y0 = ctx.read_register_value()? as u32;
    let tiles_base = ctx.read_register_value()? as usize;
    let map_base = ctx.read_register_value()? as usize;
    let w = ctx.read_byte()? as u32;
    let h = ctx.read_byte()? as u32;

    debug!("Drawing tilemap at ({}, {}) with size {}x{}", x0, y0, w, h);

    let cam_x = ctx.camera.get_x();
    let cam_y = ctx.camera.get_y();

    let ss = ctx.config.sprite_size;
    for ty in 0..h {
        for tx in 0..w {
            let map_index = (ty * w + tx) as usize;
            let tile_index = ctx.mem.read(map_base + map_index)? as usize;

            for sy in 0..ss {
                for sx in 0..ss {
                    let pixel = ctx.mem.read(
                        tiles_base
                            + tile_index * (ss as usize * ss as usize)
                            + sy as usize * ss as usize
                            + sx as usize,
                    )?;
                    if pixel == 0 {
                        continue;
                    }

                    let color = ctx.palette.get_color(pixel as usize);
                    ctx.world.set_pixel(
                        Vec2::new(
                            (x0 + tx * ss + sx).wrapping_sub(cam_x),
                            (y0 + ty * ss + sy).wrapping_sub(cam_y),
                        ),
                        color,
                    );
                }
            }
        }
    }
    Ok(())
}

pub fn tile_at(ctx: &mut ExecutionContext) -> Result<(), VmFault> {
    let rdest = ctx.read_register_index()?;
    let x = ctx.read_register_value()? as u32;
    let y = ctx.read_register_value()? as u32;
    let map_base = ctx.read_register_value()? as usize;
    let w = ctx.read_byte()? as u32;

    let ss = ctx.config.sprite_size;
    let tx = x / ss;
    let ty = y / ss;
    let map_index = (ty * w + tx) as usize;
    let tile_index = ctx.mem.read(map_base + map_index)?;

    debug!(
        "Getting tile index at ({}, {}) -> tile ({}, {}) with index {}",
        x, y, tx, ty, tile_index
    );

    ctx.cpu.set_register(rdest, tile_index as u16);
    Ok(())
}

pub fn tile_solid(ctx: &mut ExecutionContext) -> Result<(), VmFault> {
    let rdest = ctx.read_register_index()?;
    let tile_index = ctx.read_register_value()? as usize;
    let flags_base = ctx.read_register_value()? as usize;

    let flags = ctx.mem.read(flags_base + tile_index)?;

    debug!(
        "Getting solidity of tile {} with flags at {} -> flags {}",
        tile_index, flags_base, flags
    );

    ctx.cpu
        .set_register(rdest, if flags & 1 != 0 { 1 } else { 0 });
    Ok(())
}

pub fn text(ctx: &mut ExecutionContext) -> Result<(), VmFault> {
    let x0 = ctx.read_register_value()? as u32;
    let y = ctx.read_register_value()? as u32;
    let color_idx = ctx.read_register_value()? as usize;
    let base = ctx.read_register_value()? as usize;
    let len = ctx.read_byte()? as usize;

    let color = ctx.palette.get_color(color_idx);
    let font = ctx.font;
    let mut current_x = x0;

    for i in 0..len {
        let ch = ctx.mem.read(base + i)? as char;

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
    Ok(())
}

pub fn draw_number(ctx: &mut ExecutionContext) -> Result<(), VmFault> {
    let x0 = ctx.read_register_value()? as u32;
    let y = ctx.read_register_value()? as u32;
    let color_idx = ctx.read_register_value()? as usize;
    let value = ctx.read_register_value()?;

    let font = ctx.font;
    let color = ctx.palette.get_color(color_idx);
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
    Ok(())
}
