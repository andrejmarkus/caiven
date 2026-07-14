//! Shape-drawing instructions: LINE, RECT, RECTF, CIRC, CIRCF, PSET.
//! All draw to the world layer in a palette color, camera-relative, with
//! signed coordinates so partially off-screen shapes clip instead of wrapping.

use crate::rendering::screen::ScreenLayer;
use crate::vm::{ExecutionContext, VmFault};
use fc_core::{Color, Vec2};

fn plot(layer: &mut ScreenLayer, x: i64, y: i64, color: Color) {
    if x < 0 || y < 0 {
        return;
    }
    layer.set_pixel(Vec2::new(x as u32, y as u32), color);
}

fn coord(v: u32) -> i64 {
    v as i32 as i64
}

struct DrawArgs {
    color: Color,
    cam_x: i64,
    cam_y: i64,
}

impl DrawArgs {
    fn new(ctx: &ExecutionContext, color_idx: u32) -> Self {
        Self {
            color: ctx.palette.get_color(color_idx as usize),
            cam_x: ctx.camera.get_x() as i32 as i64,
            cam_y: ctx.camera.get_y() as i32 as i64,
        }
    }
}

pub fn pset(ctx: &mut ExecutionContext) -> Result<(), VmFault> {
    let x = coord(ctx.read_register_value()?);
    let y = coord(ctx.read_register_value()?);
    let col = ctx.read_register_value()?;
    let args = DrawArgs::new(ctx, col);
    plot(ctx.world, x - args.cam_x, y - args.cam_y, args.color);
    Ok(())
}

pub fn line(ctx: &mut ExecutionContext) -> Result<(), VmFault> {
    let x0 = coord(ctx.read_register_value()?);
    let y0 = coord(ctx.read_register_value()?);
    let x1 = coord(ctx.read_register_value()?);
    let y1 = coord(ctx.read_register_value()?);
    let col = ctx.read_register_value()?;
    let args = DrawArgs::new(ctx, col);

    let (mut x, mut y) = (x0, y0);
    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    loop {
        plot(ctx.world, x - args.cam_x, y - args.cam_y, args.color);
        if x == x1 && y == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
    }
    Ok(())
}

fn read_rect_args(ctx: &mut ExecutionContext) -> Result<(i64, i64, i64, i64, u32), VmFault> {
    let x = coord(ctx.read_register_value()?);
    let y = coord(ctx.read_register_value()?);
    let w = coord(ctx.read_register_value()?);
    let h = coord(ctx.read_register_value()?);
    let col = ctx.read_register_value()?;
    Ok((x, y, w, h, col))
}

pub fn rect(ctx: &mut ExecutionContext) -> Result<(), VmFault> {
    let (x, y, w, h, col) = read_rect_args(ctx)?;
    if w <= 0 || h <= 0 {
        return Ok(());
    }
    let args = DrawArgs::new(ctx, col);
    let (x, y) = (x - args.cam_x, y - args.cam_y);
    for ix in x..x + w {
        plot(ctx.world, ix, y, args.color);
        plot(ctx.world, ix, y + h - 1, args.color);
    }
    for iy in y..y + h {
        plot(ctx.world, x, iy, args.color);
        plot(ctx.world, x + w - 1, iy, args.color);
    }
    Ok(())
}

pub fn rect_fill(ctx: &mut ExecutionContext) -> Result<(), VmFault> {
    let (x, y, w, h, col) = read_rect_args(ctx)?;
    if w <= 0 || h <= 0 {
        return Ok(());
    }
    let args = DrawArgs::new(ctx, col);
    let (x, y) = (x - args.cam_x, y - args.cam_y);
    for iy in y..y + h {
        for ix in x..x + w {
            plot(ctx.world, ix, iy, args.color);
        }
    }
    Ok(())
}

fn read_circ_args(ctx: &mut ExecutionContext) -> Result<(i64, i64, i64, u32), VmFault> {
    let x = coord(ctx.read_register_value()?);
    let y = coord(ctx.read_register_value()?);
    let r = coord(ctx.read_register_value()?);
    let col = ctx.read_register_value()?;
    Ok((x, y, r, col))
}

pub fn circ(ctx: &mut ExecutionContext) -> Result<(), VmFault> {
    let (cx, cy, r, col) = read_circ_args(ctx)?;
    if r < 0 {
        return Ok(());
    }
    let args = DrawArgs::new(ctx, col);
    let (cx, cy) = (cx - args.cam_x, cy - args.cam_y);

    let mut x = r;
    let mut y = 0;
    let mut err = 1 - r;
    while x >= y {
        for (px, py) in [
            (cx + x, cy + y),
            (cx - x, cy + y),
            (cx + x, cy - y),
            (cx - x, cy - y),
            (cx + y, cy + x),
            (cx - y, cy + x),
            (cx + y, cy - x),
            (cx - y, cy - x),
        ] {
            plot(ctx.world, px, py, args.color);
        }
        y += 1;
        if err < 0 {
            err += 2 * y + 1;
        } else {
            x -= 1;
            err += 2 * (y - x) + 1;
        }
    }
    Ok(())
}

pub fn circ_fill(ctx: &mut ExecutionContext) -> Result<(), VmFault> {
    let (cx, cy, r, col) = read_circ_args(ctx)?;
    if r < 0 {
        return Ok(());
    }
    let args = DrawArgs::new(ctx, col);
    let (cx, cy) = (cx - args.cam_x, cy - args.cam_y);

    for dy in -r..=r {
        for dx in -r..=r {
            if dx * dx + dy * dy <= r * r {
                plot(ctx.world, cx + dx, cy + dy, args.color);
            }
        }
    }
    Ok(())
}
