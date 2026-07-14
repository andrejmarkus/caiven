//! Tile map and sprite-flag instructions with implicit RAM bases: MGET,
//! MSET, FGET, FSET, MAPD. Games address the map by cell coordinates and
//! sprites by id — no raw addresses. Out-of-bounds reads yield 0, writes
//! are dropped.

use crate::vm::{ExecutionContext, VmFault};
use fc_core::Vec2;
use fc_core::memory::{
    MAP_H, MAP_RAM_BASE, MAP_W, SPRITE_BYTES, SPRITE_FLAGS_RAM_BASE, SPRITE_SHEET_RAM_BASE,
};

pub fn map_get(ctx: &mut ExecutionContext) -> Result<(), VmFault> {
    let rdest = ctx.read_register_index()?;
    let x = ctx.read_register_value()? as i32;
    let y = ctx.read_register_value()? as i32;

    let tile = if (0..MAP_W as i32).contains(&x) && (0..MAP_H as i32).contains(&y) {
        ctx.mem.read(MAP_RAM_BASE + y as usize * MAP_W + x as usize)?
    } else {
        0
    };
    ctx.cpu.set_register(rdest, tile as u32);
    Ok(())
}

pub fn map_set(ctx: &mut ExecutionContext) -> Result<(), VmFault> {
    let x = ctx.read_register_value()? as i32;
    let y = ctx.read_register_value()? as i32;
    let tile = ctx.read_register_value()? as u8;

    if (0..MAP_W as i32).contains(&x) && (0..MAP_H as i32).contains(&y) {
        ctx.mem
            .write(MAP_RAM_BASE + y as usize * MAP_W + x as usize, tile)?;
    }
    Ok(())
}

pub fn flags_get(ctx: &mut ExecutionContext) -> Result<(), VmFault> {
    let rdest = ctx.read_register_index()?;
    let id = ctx.read_register_value()? as usize & 0xFF;
    let flags = ctx.mem.read(SPRITE_FLAGS_RAM_BASE + id)?;
    ctx.cpu.set_register(rdest, flags as u32);
    Ok(())
}

pub fn flags_set(ctx: &mut ExecutionContext) -> Result<(), VmFault> {
    let id = ctx.read_register_value()? as usize & 0xFF;
    let flags = ctx.read_register_value()? as u8;
    ctx.mem.write(SPRITE_FLAGS_RAM_BASE + id, flags)?;
    Ok(())
}

pub fn map_draw(ctx: &mut ExecutionContext) -> Result<(), VmFault> {
    let cx = ctx.read_register_value()? as i32;
    let cy = ctx.read_register_value()? as i32;
    let sx = ctx.read_register_value()? as i32 as i64;
    let sy = ctx.read_register_value()? as i32 as i64;
    let w = ctx.read_register_value()? as i32;
    let h = ctx.read_register_value()? as i32;

    let cam_x = ctx.camera.get_x() as i32 as i64;
    let cam_y = ctx.camera.get_y() as i32 as i64;
    let ss = ctx.config.sprite_size as i64;

    for ty in 0..h {
        let map_y = cy + ty;
        if !(0..MAP_H as i32).contains(&map_y) {
            continue;
        }
        for tx in 0..w {
            let map_x = cx + tx;
            if !(0..MAP_W as i32).contains(&map_x) {
                continue;
            }
            let tile = ctx
                .mem
                .read(MAP_RAM_BASE + map_y as usize * MAP_W + map_x as usize)?
                as usize;
            let base = SPRITE_SHEET_RAM_BASE + tile * SPRITE_BYTES;
            let ox = sx + tx as i64 * ss - cam_x;
            let oy = sy + ty as i64 * ss - cam_y;
            for py in 0..ss {
                for px in 0..ss {
                    let pixel = ctx.mem.read(base + (py * ss + px) as usize)?;
                    if pixel == 0 {
                        continue;
                    }
                    let (dx, dy) = (ox + px, oy + py);
                    if dx < 0 || dy < 0 {
                        continue;
                    }
                    let color = ctx.palette.get_color(pixel as usize);
                    ctx.world.set_pixel(Vec2::new(dx as u32, dy as u32), color);
                }
            }
        }
    }
    Ok(())
}
