//! Embedded-Lua execution path (Phases A+B of the mlua migration).
//!
//! Lives alongside the bytecode VM rather than replacing it yet — `run_frame`
//! branches to this path when a Lua script is loaded, leaving the existing
//! opcode interpreter untouched for carts that still use it.
//! Names are spelled out rather than abbreviated (`sprite` not `spr`) so the
//! API reads clearly on its own — and `draw_text` rather than `print` so we
//! don't shadow Lua's real `print()`, which stays available for console
//! debugging exactly as anyone coming from vanilla Lua would expect. Math
//! builtins (`sin`/`cos`/`abs`/`flr`/`sqrt`/`max`/`min`/`rnd`) and string
//! helpers (`sub`/`tostring`/`..`) aren't bound here — Lua's own `math` and
//! `string` stdlibs already cover them.
//!
//! Builtins are registered both at load time (so top-level script code and
//! `_init()` can use them, same as any real Lua environment) and once per
//! frame before `_update()` — `register_builtins` is shared between the two
//! call sites so the API surface can't drift between them.

use super::memory::Memory;
use super::palette::Palette;
use super::sfx::{MusicPlayer, SfxPlayer};
use super::{Camera, Vm, VmFault};
use crate::input::{Button, Input};
use crate::rendering::font::Font;
use crate::rendering::screen::ScreenLayer;
use crate::rendering::text::draw_text;
use fc_core::memory::{
    MAP_H, MAP_RAM_BASE, MAP_W, SPRITE_BYTES, SPRITE_FLAGS_RAM_BASE, SPRITE_SHEET_RAM_BASE,
};
use fc_core::{Color, Vec2};
use mlua::{HookTriggers, Lua, Scope, Table, VmState};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Names registered by [`register_builtins`] — excluded from
/// [`Vm::lua_globals`]'s snapshot since they're API surface, not script state.
const BUILTIN_NAMES: &[&str] = &[
    "clear_screen",
    "set_pixel",
    "sprite",
    "button_down",
    "button_pressed",
    "draw_text",
    "draw_number",
    "fill_screen",
    "draw_line",
    "draw_rect",
    "fill_rect",
    "draw_circle",
    "fill_circle",
    "set_camera",
    "set_palette_color",
    "draw_map",
    "get_tile",
    "set_tile",
    "get_sprite_flags",
    "set_sprite_flags",
    "play_sfx",
    "play_music",
    "stop_music",
];

/// Lua's own stdlib globals — also excluded from the snapshot, along with
/// the two script entry points.
const STDLIB_NAMES: &[&str] = &[
    "_G",
    "_VERSION",
    "_init",
    "_update",
    "assert",
    "collectgarbage",
    "coroutine",
    "debug",
    "dofile",
    "error",
    "getmetatable",
    "io",
    "ipairs",
    "load",
    "loadfile",
    "math",
    "next",
    "os",
    "package",
    "pairs",
    "pcall",
    "print",
    "rawequal",
    "rawget",
    "rawlen",
    "rawset",
    "require",
    "select",
    "setmetatable",
    "string",
    "table",
    "tonumber",
    "tostring",
    "type",
    "utf8",
    "xpcall",
];

/// Chunk name given to every loaded script — error messages come back as
/// `cart:<line>: ...`, which [`describe_lua_error`] parses to recover the
/// line for the code editor's clickable error jump. The `=` prefix tells Lua
/// to use the name as-is instead of wrapping it as `[string "cart"]`.
const CHUNK_NAME: &str = "cart";
const CHUNK_SOURCE_NAME: &str = "=cart";

pub(super) struct LuaScript {
    lua: Lua,
}

/// Result of one debug-aware Lua frame ([`Vm::run_frame_lua_bp`]).
#[derive(Debug, Clone)]
pub enum LuaRunOutcome {
    /// `_update()` ran to completion.
    Completed,
    /// Execution stopped at a breakpointed source line; the rest of this
    /// frame's `_update()` did not run.
    Breakpoint(usize),
    /// A genuine Lua runtime error (not a breakpoint stop).
    Error(String),
}

/// Extracts the raw Lua message (no `syntax error:`/`runtime error:` wrapper)
/// and, when present, the 1-based `cart:<line>:` source line.
pub fn describe_lua_error(err: &mlua::Error) -> (Option<usize>, String) {
    let raw = match err {
        mlua::Error::SyntaxError { message, .. } => message.clone(),
        mlua::Error::RuntimeError(message) => message.clone(),
        other => other.to_string(),
    };
    let line = raw
        .strip_prefix(CHUNK_NAME)
        .and_then(|rest| rest.strip_prefix(':'))
        .and_then(|rest| rest.split(':').next())
        .and_then(|n| n.parse().ok());
    (line, raw)
}

fn describe_lua_value(value: &mlua::Value) -> String {
    match value {
        mlua::Value::Nil => "nil".to_string(),
        mlua::Value::Boolean(b) => b.to_string(),
        mlua::Value::Integer(i) => i.to_string(),
        mlua::Value::Number(n) => n.to_string(),
        mlua::Value::String(s) => format!("{:?}", s.to_string_lossy()),
        mlua::Value::Table(_) => "{table}".to_string(),
        mlua::Value::Function(_) => "{function}".to_string(),
        other => format!("{other:?}"),
    }
}

fn plot(layer: &mut ScreenLayer, x: i64, y: i64, color: Color) {
    if x < 0 || y < 0 {
        return;
    }
    layer.set_pixel(Vec2::new(x as u32, y as u32), color);
}

fn cam_offset(camera: &RefCell<&mut Camera>) -> (i64, i64) {
    let c = camera.borrow();
    (c.get_x() as i32 as i64, c.get_y() as i32 as i64)
}

fn draw_line(layer: &mut ScreenLayer, x0: i64, y0: i64, x1: i64, y1: i64, color: Color) {
    let (mut x, mut y) = (x0, y0);
    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    loop {
        plot(layer, x, y, color);
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
}

fn circle_points(cx: i64, cy: i64, r: i64, mut f: impl FnMut(i64, i64)) {
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
            f(px, py);
        }
        y += 1;
        if err < 0 {
            err += 2 * y + 1;
        } else {
            x -= 1;
            err += 2 * (y - x) + 1;
        }
    }
}

/// Registers the full builtin API surface as Lua globals scoped to this
/// call's borrowed VM state. Shared by [`Vm::load_lua_source`] (so top-level
/// script code and `_init()` see the same globals as `_update()`) and
/// [`Vm::run_frame_lua`].
#[allow(clippy::too_many_arguments)]
fn register_builtins<'scope, 'env>(
    scope: &'scope Scope<'scope, 'env>,
    globals: &Table,
    world: &'env RefCell<&'env mut ScreenLayer>,
    ui: &'env RefCell<&'env mut ScreenLayer>,
    memory: &'env RefCell<&'env mut Memory>,
    palette: &'env RefCell<&'env mut Palette>,
    camera: &'env RefCell<&'env mut Camera>,
    sfx_player: &'env RefCell<&'env mut SfxPlayer>,
    music_player: &'env RefCell<&'env mut MusicPlayer>,
    input: &'env Input,
    font: &'env Font,
    sprite_size: u32,
    width: u32,
    height: u32,
) -> mlua::Result<()> {
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
            let color = palette.borrow().get_color(color_index as usize);
            plot(&mut world.borrow_mut(), x, y, color);
            Ok(())
        })?,
    )?;

    globals.set(
        "sprite",
        scope.create_function_mut(move |_, (sprite_id, x, y): (u8, i64, i64)| {
            let base = SPRITE_SHEET_RAM_BASE + sprite_id as usize * SPRITE_BYTES;
            let (cam_x, cam_y) = cam_offset(camera);
            let ss = sprite_size as i64;
            let mem = memory.borrow();
            let mut w = world.borrow_mut();
            for sy in 0..ss {
                for sx in 0..ss {
                    let Ok(pixel) = mem.read(base + (sy * ss + sx) as usize) else {
                        continue;
                    };
                    if pixel == 0 {
                        continue;
                    }
                    let color = palette.borrow().get_color(pixel as usize);
                    plot(&mut w, x + sx - cam_x, y + sy - cam_y, color);
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
        "button_pressed",
        scope.create_function(|_, button_index: u8| {
            Ok(Button::from_u8(button_index)
                .map(|b| input.just_pressed(b))
                .unwrap_or(false))
        })?,
    )?;

    globals.set(
        "draw_text",
        scope.create_function_mut(|_, (text, x, y, color_index): (String, i64, i64, u8)| {
            if x < 0 || y < 0 {
                return Ok(());
            }
            let color = palette.borrow().get_color(color_index as usize);
            draw_text(
                font,
                &mut ui.borrow_mut(),
                &text,
                Vec2::new(x as u32, y as u32),
                color,
            );
            Ok(())
        })?,
    )?;

    globals.set(
        "draw_number",
        scope.create_function_mut(|_, (value, x, y, color_index): (i64, i64, i64, u8)| {
            if x < 0 || y < 0 {
                return Ok(());
            }
            let color = palette.borrow().get_color(color_index as usize);
            draw_text(
                font,
                &mut ui.borrow_mut(),
                &value.to_string(),
                Vec2::new(x as u32, y as u32),
                color,
            );
            Ok(())
        })?,
    )?;

    globals.set(
        "fill_screen",
        scope.create_function_mut(move |_, color_index: u8| {
            let color = palette.borrow().get_color(color_index as usize);
            let mut w = world.borrow_mut();
            for y in 0..height {
                for x in 0..width {
                    w.set_pixel(Vec2::new(x, y), color);
                }
            }
            Ok(())
        })?,
    )?;

    globals.set(
        "draw_line",
        scope.create_function_mut(
            |_, (x0, y0, x1, y1, color_index): (i64, i64, i64, i64, u8)| {
                let color = palette.borrow().get_color(color_index as usize);
                let (cam_x, cam_y) = cam_offset(camera);
                draw_line(
                    &mut world.borrow_mut(),
                    x0 - cam_x,
                    y0 - cam_y,
                    x1 - cam_x,
                    y1 - cam_y,
                    color,
                );
                Ok(())
            },
        )?,
    )?;

    globals.set(
        "draw_rect",
        scope.create_function_mut(|_, (x, y, w, h, color_index): (i64, i64, i64, i64, u8)| {
            if w <= 0 || h <= 0 {
                return Ok(());
            }
            let color = palette.borrow().get_color(color_index as usize);
            let (cam_x, cam_y) = cam_offset(camera);
            let (x, y) = (x - cam_x, y - cam_y);
            let mut layer = world.borrow_mut();
            for ix in x..x + w {
                plot(&mut layer, ix, y, color);
                plot(&mut layer, ix, y + h - 1, color);
            }
            for iy in y..y + h {
                plot(&mut layer, x, iy, color);
                plot(&mut layer, x + w - 1, iy, color);
            }
            Ok(())
        })?,
    )?;

    globals.set(
        "fill_rect",
        scope.create_function_mut(|_, (x, y, w, h, color_index): (i64, i64, i64, i64, u8)| {
            if w <= 0 || h <= 0 {
                return Ok(());
            }
            let color = palette.borrow().get_color(color_index as usize);
            let (cam_x, cam_y) = cam_offset(camera);
            let (x, y) = (x - cam_x, y - cam_y);
            let mut layer = world.borrow_mut();
            for iy in y..y + h {
                for ix in x..x + w {
                    plot(&mut layer, ix, iy, color);
                }
            }
            Ok(())
        })?,
    )?;

    globals.set(
        "draw_circle",
        scope.create_function_mut(|_, (cx, cy, r, color_index): (i64, i64, i64, u8)| {
            if r < 0 {
                return Ok(());
            }
            let color = palette.borrow().get_color(color_index as usize);
            let (cam_x, cam_y) = cam_offset(camera);
            circle_points(cx - cam_x, cy - cam_y, r, |x, y| {
                plot(&mut world.borrow_mut(), x, y, color)
            });
            Ok(())
        })?,
    )?;

    globals.set(
        "fill_circle",
        scope.create_function_mut(|_, (cx, cy, r, color_index): (i64, i64, i64, u8)| {
            if r < 0 {
                return Ok(());
            }
            let color = palette.borrow().get_color(color_index as usize);
            let (cam_x, cam_y) = cam_offset(camera);
            let (cx, cy) = (cx - cam_x, cy - cam_y);
            let mut layer = world.borrow_mut();
            for dy in -r..=r {
                for dx in -r..=r {
                    if dx * dx + dy * dy <= r * r {
                        plot(&mut layer, cx + dx, cy + dy, color);
                    }
                }
            }
            Ok(())
        })?,
    )?;

    globals.set(
        "set_camera",
        scope.create_function_mut(|_, (x, y): (u32, u32)| {
            camera.borrow_mut().set_position(x, y);
            Ok(())
        })?,
    )?;

    globals.set(
        "set_palette_color",
        scope.create_function_mut(|_, (index, r, g, b): (usize, u8, u8, u8)| {
            palette
                .borrow_mut()
                .set_color(index, Color::new_rgb(r, g, b));
            Ok(())
        })?,
    )?;

    globals.set(
        "draw_map",
        scope.create_function_mut(
            move |_, (cx, cy, sx, sy, w, h): (i64, i64, i64, i64, i64, i64)| {
                let (cam_x, cam_y) = cam_offset(camera);
                let ss = sprite_size as i64;
                let mem = memory.borrow();
                let pal = palette.borrow();
                let mut layer = world.borrow_mut();
                for ty in 0..h {
                    let map_y = cy + ty;
                    if !(0..MAP_H as i64).contains(&map_y) {
                        continue;
                    }
                    for tx in 0..w {
                        let map_x = cx + tx;
                        if !(0..MAP_W as i64).contains(&map_x) {
                            continue;
                        }
                        let Ok(tile) =
                            mem.read(MAP_RAM_BASE + map_y as usize * MAP_W + map_x as usize)
                        else {
                            continue;
                        };
                        let base = SPRITE_SHEET_RAM_BASE + tile as usize * SPRITE_BYTES;
                        let ox = sx + tx * ss - cam_x;
                        let oy = sy + ty * ss - cam_y;
                        for py in 0..ss {
                            for px in 0..ss {
                                let Ok(pixel) = mem.read(base + (py * ss + px) as usize) else {
                                    continue;
                                };
                                if pixel == 0 {
                                    continue;
                                }
                                let color = pal.get_color(pixel as usize);
                                plot(&mut layer, ox + px, oy + py, color);
                            }
                        }
                    }
                }
                Ok(())
            },
        )?,
    )?;

    globals.set(
        "get_tile",
        scope.create_function(|_, (x, y): (i64, i64)| {
            if !(0..MAP_W as i64).contains(&x) || !(0..MAP_H as i64).contains(&y) {
                return Ok(0u8);
            }
            Ok(memory
                .borrow()
                .read(MAP_RAM_BASE + y as usize * MAP_W + x as usize)
                .unwrap_or(0))
        })?,
    )?;

    globals.set(
        "set_tile",
        scope.create_function_mut(|_, (x, y, tile): (i64, i64, u8)| {
            if (0..MAP_W as i64).contains(&x) && (0..MAP_H as i64).contains(&y) {
                let _ = memory
                    .borrow_mut()
                    .write(MAP_RAM_BASE + y as usize * MAP_W + x as usize, tile);
            }
            Ok(())
        })?,
    )?;

    globals.set(
        "get_sprite_flags",
        scope.create_function(|_, sprite_id: u8| {
            Ok(memory
                .borrow()
                .read(SPRITE_FLAGS_RAM_BASE + sprite_id as usize)
                .unwrap_or(0))
        })?,
    )?;

    globals.set(
        "set_sprite_flags",
        scope.create_function_mut(|_, (sprite_id, flags): (u8, u8)| {
            let _ = memory
                .borrow_mut()
                .write(SPRITE_FLAGS_RAM_BASE + sprite_id as usize, flags);
            Ok(())
        })?,
    )?;

    globals.set(
        "play_sfx",
        scope.create_function_mut(|_, id: u8| {
            sfx_player.borrow_mut().start(id);
            Ok(())
        })?,
    )?;

    globals.set(
        "play_music",
        scope.create_function_mut(|_, id: u8| {
            music_player.borrow_mut().start(id);
            Ok(())
        })?,
    )?;

    globals.set(
        "stop_music",
        scope.create_function_mut(|_, ()| {
            music_player.borrow_mut().stop();
            Ok(())
        })?,
    )?;

    Ok(())
}

impl Vm {
    /// Loads Lua source, registering the full builtin API first so top-level
    /// script code and `_init()` (called once here, if present) can use it
    /// exactly like `_update()` can. Subsequent frames call `_update()` via
    /// [`Vm::run_frame`].
    pub fn load_lua_source(&mut self, src: &str, input: &Input, font: &Font) -> mlua::Result<()> {
        let lua = Lua::new();

        let world = RefCell::new(&mut self.world);
        let ui = RefCell::new(&mut self.ui);
        let memory = RefCell::new(&mut self.memory);
        let palette = RefCell::new(&mut self.palette);
        let camera = RefCell::new(&mut self.camera);
        let sfx_player = RefCell::new(&mut self.sfx_player);
        let music_player = RefCell::new(&mut self.music_player);
        let sprite_size = self.config.sprite_size;
        let width = self.config.width;
        let height = self.config.height;

        let result: mlua::Result<()> = lua.scope(|scope| {
            let globals = lua.globals();
            register_builtins(
                scope,
                &globals,
                &world,
                &ui,
                &memory,
                &palette,
                &camera,
                &sfx_player,
                &music_player,
                input,
                font,
                sprite_size,
                width,
                height,
            )?;

            lua.load(src).set_name(CHUNK_SOURCE_NAME).exec()?;
            if let Ok(init) = globals.get::<mlua::Function>("_init") {
                init.call::<()>(())?;
            }
            Ok(())
        });
        result?;

        self.script = Some(LuaScript { lua });
        Ok(())
    }

    pub fn has_lua_script(&self) -> bool {
        self.script.is_some()
    }

    /// One Lua-driven frame: re-registers the builtin set against this
    /// frame's borrowed VM state via `Lua::scope`, then calls the script's
    /// `_update()`. Re-registering per frame avoids needing `'static`/`Send`
    /// closures or unsafe aliasing for host state that changes every frame
    /// (screen buffers, input).
    pub(super) fn run_frame_lua(&mut self, input: &Input, font: &Font) {
        let Some(script) = self.script.as_ref() else {
            return;
        };
        let lua = &script.lua;

        let world = RefCell::new(&mut self.world);
        let ui = RefCell::new(&mut self.ui);
        let memory = RefCell::new(&mut self.memory);
        let palette = RefCell::new(&mut self.palette);
        let camera = RefCell::new(&mut self.camera);
        let sfx_player = RefCell::new(&mut self.sfx_player);
        let music_player = RefCell::new(&mut self.music_player);
        let sprite_size = self.config.sprite_size;
        let width = self.config.width;
        let height = self.config.height;

        let result: mlua::Result<()> = lua.scope(|scope| {
            let globals = lua.globals();
            register_builtins(
                scope,
                &globals,
                &world,
                &ui,
                &memory,
                &palette,
                &camera,
                &sfx_player,
                &music_player,
                input,
                font,
                sprite_size,
                width,
                height,
            )?;

            let update: mlua::Function = globals.get("_update")?;
            update.call::<()>(())
        });

        if let Err(e) = result {
            log::error!("Lua runtime error: {e}");
            self.set_fault(VmFault::LuaError);
        }
    }

    /// Like [`Vm::run_frame_lua`], but installs a line hook that aborts
    /// `_update()` as soon as it reaches a breakpointed source line. The
    /// aborted call unwinds Lua's stack (mlua's hooks can't yield outside a
    /// coroutine while borrowing per-frame VM state via `Lua::scope`, so a
    /// suspend-and-resume mid-statement debugger isn't possible here) —
    /// globals and RAM at the moment of the stop are still readable via
    /// [`Vm::lua_globals`] and `peek_memory`, but locals are not: mlua's
    /// safe hook API has no `lua_getlocal` binding. Resuming re-runs
    /// `_update()` from the top, same as any other frame.
    pub fn run_frame_lua_bp(
        &mut self,
        input: &Input,
        font: &Font,
        breakpoints: &[usize],
    ) -> LuaRunOutcome {
        // `run_frame` ticks this; this path grew separately and didn't, so
        // Studio's Running state was silent even though a sound was "active".
        self.tick_audio_players();

        let Some(script) = self.script.as_ref() else {
            return LuaRunOutcome::Completed;
        };
        let lua = &script.lua;

        let world = RefCell::new(&mut self.world);
        let ui = RefCell::new(&mut self.ui);
        let memory = RefCell::new(&mut self.memory);
        let palette = RefCell::new(&mut self.palette);
        let camera = RefCell::new(&mut self.camera);
        let sfx_player = RefCell::new(&mut self.sfx_player);
        let music_player = RefCell::new(&mut self.music_player);
        let sprite_size = self.config.sprite_size;
        let width = self.config.width;
        let height = self.config.height;

        let hit: Rc<Cell<Option<usize>>> = Rc::new(Cell::new(None));
        let hit_hook = hit.clone();
        let bps: Vec<usize> = breakpoints.to_vec();
        lua.set_hook(HookTriggers::EVERY_LINE, move |_lua, debug| {
            let line = debug.curr_line();
            if line > 0 && bps.contains(&(line as usize)) {
                hit_hook.set(Some(line as usize));
                return Err(mlua::Error::runtime("breakpoint"));
            }
            Ok(VmState::Continue)
        });

        let result: mlua::Result<()> = lua.scope(|scope| {
            let globals = lua.globals();
            register_builtins(
                scope,
                &globals,
                &world,
                &ui,
                &memory,
                &palette,
                &camera,
                &sfx_player,
                &music_player,
                input,
                font,
                sprite_size,
                width,
                height,
            )?;

            let update: mlua::Function = globals.get("_update")?;
            update.call::<()>(())
        });
        lua.remove_hook();

        match (hit.get(), result) {
            (Some(line), _) => LuaRunOutcome::Breakpoint(line),
            (None, Ok(())) => LuaRunOutcome::Completed,
            (None, Err(e)) => {
                log::error!("Lua runtime error: {e}");
                self.set_fault(VmFault::LuaError);
                LuaRunOutcome::Error(e.to_string())
            }
        }
    }

    /// Snapshot of the script's global variables, for the Studio debugger's
    /// state inspector. Excludes registered builtins and Lua's own stdlib —
    /// see [`BUILTIN_NAMES`]/[`STDLIB_NAMES`] — so only script-defined state
    /// shows up. Locals aren't enumerable (see [`Vm::run_frame_lua_bp`]).
    pub fn lua_globals(&self) -> Vec<(String, String)> {
        let Some(script) = self.script.as_ref() else {
            return Vec::new();
        };
        let globals = script.lua.globals();
        let mut out: Vec<(String, String)> = globals
            .pairs::<String, mlua::Value>()
            .filter_map(|pair| pair.ok())
            .filter(|(k, _)| {
                !BUILTIN_NAMES.contains(&k.as_str()) && !STDLIB_NAMES.contains(&k.as_str())
            })
            .map(|(k, v)| (k, describe_lua_value(&v)))
            .collect();
        out.sort_by(|a, b| a.0.cmp(&b.0));
        out
    }
}
