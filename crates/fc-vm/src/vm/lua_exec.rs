//! Minimal embedded-Lua execution path (Phase A of the mlua migration).
//!
//! Lives alongside the bytecode VM rather than replacing it yet — `run_frame`
//! branches to this path when a Lua script is loaded, leaving the existing
//! opcode interpreter untouched for carts that still use it. Only a handful
//! of builtins are wired up here (`clear_screen`, `set_pixel`, `sprite`,
//! `button_down`, `draw_text`); the full API surface is later phases.
//! Names are spelled out rather than abbreviated (`sprite` not `spr`) so the
//! API reads clearly on its own — and `draw_text` rather than `print` so we
//! don't shadow Lua's real `print()`, which stays available for console
//! debugging exactly as anyone coming from vanilla Lua would expect.

use super::{Vm, VmFault};
use crate::input::{Button, Input};
use crate::rendering::font::Font;
use crate::rendering::text::draw_text;
use fc_core::Vec2;
use mlua::Lua;
use std::cell::RefCell;

pub(super) struct LuaScript {
    lua: Lua,
}

impl Vm {
    /// Loads and runs Lua source once (defining `_init`/`_update` as
    /// globals), then calls `_init()` if present. Subsequent frames call
    /// `_update()` via [`Vm::run_frame`].
    pub fn load_lua_source(&mut self, src: &str) -> mlua::Result<()> {
        let lua = Lua::new();
        lua.load(src).exec()?;
        if let Ok(init) = lua.globals().get::<mlua::Function>("_init") {
            init.call::<()>(())?;
        }
        self.script = Some(LuaScript { lua });
        Ok(())
    }

    pub(super) fn has_lua_script(&self) -> bool {
        self.script.is_some()
    }

    /// One Lua-driven frame: re-registers the (small, Phase-A) builtin set
    /// against this frame's borrowed VM state via `Lua::scope`, then calls
    /// the script's `_update()`. Re-registering per frame avoids needing
    /// `'static`/`Send` closures or unsafe aliasing for host state that
    /// changes every frame (screen buffers, input).
    pub(super) fn run_frame_lua(&mut self, input: &Input, font: &Font) {
        let Some(script) = self.script.as_ref() else {
            return;
        };
        let lua = &script.lua;

        let world = RefCell::new(&mut self.world);
        let ui = RefCell::new(&mut self.ui);
        let memory = &self.memory;
        let palette = &self.palette;
        let camera = &self.camera;
        let sprite_size = self.config.sprite_size;

        let result: mlua::Result<()> = lua.scope(|scope| {
            let globals = lua.globals();

            globals.set(
                "clear_screen",
                scope.create_function_mut(|_, ()| {
                    world.borrow_mut().clear();
                    ui.borrow_mut().clear();
                    Ok(())
                })?,
            )?;

            globals.set(
                "set_pixel",
                scope.create_function_mut(|_, (x, y, color_index): (i64, i64, u8)| {
                    if x < 0 || y < 0 {
                        return Ok(());
                    }
                    let color = palette.get_color(color_index as usize);
                    world
                        .borrow_mut()
                        .set_pixel(Vec2::new(x as u32, y as u32), color);
                    Ok(())
                })?,
            )?;

            globals.set(
                "sprite",
                scope.create_function_mut(|_, (sprite_id, x, y): (u8, i64, i64)| {
                    let base = fc_core::memory::SPRITE_SHEET_RAM_BASE
                        + sprite_id as usize * fc_core::memory::SPRITE_BYTES;
                    let cam_x = camera.get_x() as i64;
                    let cam_y = camera.get_y() as i64;
                    let ss = sprite_size as i64;
                    for sy in 0..ss {
                        for sx in 0..ss {
                            let Ok(pixel) = memory.read(base + (sy * ss + sx) as usize) else {
                                continue;
                            };
                            if pixel == 0 {
                                continue;
                            }
                            let (dx, dy) = (x + sx - cam_x, y + sy - cam_y);
                            if dx < 0 || dy < 0 {
                                continue;
                            }
                            let color = palette.get_color(pixel as usize);
                            world
                                .borrow_mut()
                                .set_pixel(Vec2::new(dx as u32, dy as u32), color);
                        }
                    }
                    Ok(())
                })?,
            )?;

            globals.set(
                "button_down",
                scope.create_function(|_, button_index: u8| {
                    Ok(Button::from_u8(button_index)
                        .map(|b| input.is_pressed(b))
                        .unwrap_or(false))
                })?,
            )?;

            globals.set(
                "draw_text",
                scope.create_function_mut(
                    |_, (text, x, y, color_index): (String, i64, i64, u8)| {
                        if x < 0 || y < 0 {
                            return Ok(());
                        }
                        let color = palette.get_color(color_index as usize);
                        draw_text(
                            font,
                            &mut ui.borrow_mut(),
                            &text,
                            Vec2::new(x as u32, y as u32),
                            color,
                        );
                        Ok(())
                    },
                )?,
            )?;

            let update: mlua::Function = globals.get("_update")?;
            update.call::<()>(())
        });

        if let Err(e) = result {
            log::error!("Lua runtime error: {e}");
            self.set_fault(VmFault::LuaError);
        }
    }
}
