//! Embedded-Lua execution path — every cart is Lua, `run_frame` always runs
//! `_update()` through here, then `_draw()` if the cart defines it.
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
use super::save_data::{SaveData, SaveDataError};
use super::sfx::{MusicPlayer, SfxPlayer};
use super::{AssetBankKind, AssetBanks, Camera, Vm, VmFault};
use crate::input::{Button, Input};
use crate::rendering::font::Font;
use crate::rendering::screen::ScreenLayer;
use crate::rendering::text::draw_text;
use caiven_core::memory::{
    COLLISION_RAM_BASE, MAP_H, MAP_RAM_BASE, MAP_W, PALETTE_RAM_BASE, RTC_RAM_BASE, SPRITE_BYTES,
    SPRITE_SHEET_RAM_BASE,
};
use caiven_core::{Color, Vec2};
use mlua::{HookTriggers, Lua, LuaSerdeExt, MultiValue, Scope, StdLib, Table, VmState};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

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
    "get_collision",
    "set_collision",
    "collision_type_id",
    "collision_type_name",
    "collision_is_solid",
    "load_sprite_bank",
    "load_map_bank",
    "load_palette_bank",
    "load_sfx_bank",
    "load_music_bank",
    "play_sfx",
    "play_music",
    "stop_music",
    "real_time",
    "frame_count",
    "time",
    "SPRITE_SIZE",
    "dset",
    "dget",
    "save_data",
    "load_data",
];

/// Names defined by [`PRELUDE_SOURCE`] — also excluded from
/// [`Vm::lua_globals`]'s snapshot, same reasoning as `BUILTIN_NAMES`: API
/// surface, not script state. `Particles` is a table, not a function; it's
/// still excluded wholesale rather than snapshotted, since its `list` field
/// churns every frame and isn't useful in a "what does the script think"
/// debugger view.
const PRELUDE_NAMES: &[&str] = &[
    "lerp",
    "clamp",
    "ease_linear",
    "ease_in_quad",
    "ease_out_quad",
    "ease_in_out_quad",
    "aabb_overlap",
    "tile_solid",
    "box_touches_solid",
    "new_tween",
    "tween_update",
    "new_anim",
    "anim_update",
    "anim_sprite",
    "Particles",
    "Vec2",
    "RTK_SEEDED",
    "random_range",
    "random_float",
    "choice",
    "shuffle",
];

/// Lua's own stdlib globals — also excluded from the snapshot, along with
/// the two script entry points.
const STDLIB_NAMES: &[&str] = &[
    "_G",
    "_VERSION",
    "_init",
    "_update",
    "_draw",
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
    "warn",
    "xpcall",
];

/// Chunk name given to every loaded script — error messages come back as
/// `cart:<line>: ...`, which [`describe_lua_error`] parses to recover the
/// line for the code editor's clickable error jump. The `=` prefix tells Lua
/// to use the name as-is instead of wrapping it as `[string "cart"]`.
const CHUNK_NAME: &str = "cart";
const CHUNK_SOURCE_NAME: &str = "=cart";

/// Gameplay-facing stdlib (easing, AABB/tile collision, tweens, particles) —
/// pure Lua, loaded into globals once before the cart's own source so it's
/// available from `_init()` onward like any builtin.
const PRELUDE_SOURCE: &str = include_str!("prelude.lua");

/// Frames per second `time()` assumes when converting `frame_count`.
const TARGET_FPS: f64 = 60.0;
const MAX_CAPTURED_OUTPUT_LINES: usize = 200;

pub(super) struct LuaScript {
    lua: Lua,
    output: Arc<Mutex<Vec<String>>>,
}

/// Result of one debug-aware Lua frame ([`Vm::run_frame_lua_bp`]).
#[derive(Debug, Clone)]
pub enum LuaRunOutcome {
    /// `_update()` ran to completion.
    Completed,
    /// Execution stopped at a breakpointed source line; the rest of this
    /// frame's `_update()` did not run.
    Breakpoint(LuaBreakpoint),
    /// A genuine Lua runtime error (not a breakpoint stop), with the
    /// 1-based source line when [`describe_lua_error`] could recover one.
    Error(Option<LuaBreakpoint>, String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LuaBreakpoint {
    pub source: String,
    pub line: usize,
}

impl LuaBreakpoint {
    pub fn new(source: impl Into<String>, line: usize) -> Self {
        Self {
            source: source.into(),
            line,
        }
    }
}

fn normalized_debug_source(source: &str) -> String {
    source.trim_start_matches(['@', '=']).replace('\\', "/")
}

/// Walks the interpreter stack from the innermost frame outward, for the
/// Studio debugger's Call stack panel. Called from inside the breakpoint
/// hook, where every level is still live — the frame is gone the instant
/// the hook returns, so this can't be deferred to after the run.
fn capture_call_stack(lua: &Lua) -> Vec<(String, String)> {
    let mut frames = Vec::new();
    let mut level = 0usize;
    while let Some(debug) = lua.inspect_stack(level) {
        let names = debug.names();
        let label = names.name.map(|name| name.into_owned()).unwrap_or_else(|| {
            if level == 0 {
                "?".to_string()
            } else {
                "anonymous function".to_string()
            }
        });
        let source = debug.source();
        let file = source
            .short_src
            .map(|src| src.into_owned())
            .unwrap_or_else(|| "?".to_string());
        let line = debug.curr_line();
        if line > 0 {
            frames.push((label, format!("{file}:{line}")));
        }
        level += 1;
        if level > 64 {
            break;
        }
    }
    frames
}

/// Reads the innermost frame's active local variables via raw `lua_getlocal`
/// (V23) — mlua's safe hook API has no locals accessor (R1), so this drops to
/// `mlua_sys` through a reentrant `Lua::exec_raw` call from inside the
/// already-active `EVERY_LINE` hook (proven safe by the T7 spike, R4).
/// Read-only: nothing is pushed back via `lua_setlocal`, and this is only
/// ever called from the Rust-side hook, never reachable from cart Lua (V8).
///
/// `lua_getlocal` enumerates every local active at the current program
/// counter, including ones a later `local` declaration shadows — later
/// declarations come later in the `n` enumeration, so overwriting on a name
/// collision keeps the innermost (currently visible) binding. Names starting
/// with `(` are compiler-internal (e.g. `(for state)`) and are skipped.
fn read_active_locals(state: *mut mlua_sys::lua_State) -> Vec<(String, String)> {
    use std::ffi::CStr;
    use std::os::raw::c_int;

    // `exec_raw` invokes this closure via `lua_pcall` (see mlua's
    // `protect_lua_closure`), which pushes its own trampoline C function
    // onto the call stack — so level 0 here is that trampoline, and the
    // frame we actually want (`_update`, where the `EVERY_LINE` hook fired)
    // is level 1.
    const CALLER_FRAME_LEVEL: std::os::raw::c_int = 1;

    let mut locals: Vec<(String, String)> = Vec::new();
    unsafe {
        let mut ar: mlua_sys::lua_Debug = std::mem::zeroed();
        if mlua_sys::lua_getstack(state, CALLER_FRAME_LEVEL, &mut ar) == 0 {
            return locals;
        }
        let mut n: c_int = 1;
        loop {
            let name_ptr = mlua_sys::lua_getlocal(state, &ar, n);
            if name_ptr.is_null() {
                break;
            }
            n += 1;
            let name = CStr::from_ptr(name_ptr).to_string_lossy().into_owned();
            if name.starts_with('(') {
                mlua_sys::lua_pop(state, 1);
                continue;
            }
            let value = describe_raw_stack_value(state, -1);
            mlua_sys::lua_pop(state, 1);
            match locals.iter_mut().find(|(existing, _)| *existing == name) {
                Some(existing) => existing.1 = value,
                None => locals.push((name, value)),
            }
        }
    }
    locals
}

/// Describes the value at a raw Lua stack index — the `lua_getlocal`
/// counterpart of [`describe_lua_value`], since a raw-stack local isn't an
/// `mlua::Value` without a round-trip this call path avoids. `unsafe`: caller
/// guarantees `idx` is a valid, live stack index.
unsafe fn describe_raw_stack_value(
    state: *mut mlua_sys::lua_State,
    idx: std::os::raw::c_int,
) -> String {
    use std::ffi::CStr;

    unsafe {
        match mlua_sys::lua_type(state, idx) {
            mlua_sys::LUA_TNIL => "nil".to_string(),
            mlua_sys::LUA_TBOOLEAN => (mlua_sys::lua_toboolean(state, idx) != 0).to_string(),
            mlua_sys::LUA_TNUMBER => {
                if mlua_sys::lua_isinteger(state, idx) != 0 {
                    let mut ok = 0;
                    mlua_sys::lua_tointegerx(state, idx, &mut ok).to_string()
                } else {
                    let mut ok = 0;
                    mlua_sys::lua_tonumberx(state, idx, &mut ok).to_string()
                }
            }
            mlua_sys::LUA_TSTRING => {
                let mut len = 0usize;
                let ptr = mlua_sys::lua_tolstring(state, idx, &mut len);
                if ptr.is_null() {
                    "\"\"".to_string()
                } else {
                    let bytes = std::slice::from_raw_parts(ptr as *const u8, len);
                    format!("{:?}", String::from_utf8_lossy(bytes))
                }
            }
            mlua_sys::LUA_TTABLE => "{table}".to_string(),
            mlua_sys::LUA_TFUNCTION => "{function}".to_string(),
            tp => {
                let type_name = mlua_sys::lua_typename(state, tp);
                if type_name.is_null() {
                    "?".to_string()
                } else {
                    format!("{{{}}}", CStr::from_ptr(type_name).to_string_lossy())
                }
            }
        }
    }
}

/// Extracts the raw Lua message (no `syntax error:`/`runtime error:` wrapper)
/// and, when present, the 1-based `cart:<line>:` source line.
pub fn describe_lua_error(err: &mlua::Error) -> (Option<usize>, String) {
    let (location, message) = describe_lua_error_location(err);
    (location.map(|location| location.line), message)
}

/// Extracts source and line from Lua errors. Bundled module syntax failures
/// can mention both wrapper `cart` and actual `ui/panel.lua`; module location
/// wins so Studio opens correct buffer.
pub fn describe_lua_error_location(err: &mlua::Error) -> (Option<LuaBreakpoint>, String) {
    let raw = match err {
        mlua::Error::SyntaxError { message, .. } => message.clone(),
        mlua::Error::RuntimeError(message) => message.clone(),
        other => other.to_string(),
    };
    let mut candidates = Vec::new();
    for (colon, _) in raw.match_indices(':') {
        let line_start = colon + 1;
        let Some(line_end) = raw[line_start..]
            .find(':')
            .map(|offset| line_start + offset)
        else {
            continue;
        };
        let Ok(line) = raw[line_start..line_end].parse::<usize>() else {
            continue;
        };
        let source_start = raw[..colon]
            .rfind(|character: char| {
                character.is_whitespace() || matches!(character, '[' | '(' | '"' | '\'')
            })
            .map_or(0, |index| index + 1);
        let source = normalized_debug_source(
            raw[source_start..colon]
                .trim_matches(|character: char| matches!(character, ']' | ')' | '"' | '\'')),
        );
        if source == CHUNK_NAME || source.ends_with(".lua") {
            candidates.push(LuaBreakpoint { source, line });
        }
    }
    let location = candidates
        .iter()
        .find(|candidate| candidate.source.ends_with(".lua"))
        .cloned()
        .or_else(|| candidates.into_iter().next());
    (location, raw)
}

/// Whether a global name is script-defined state rather than API surface —
/// used by [`Vm::lua_globals`] (debugger inspector), which deliberately
/// excludes `_init`/`_update`/`_draw` since they're entry points, not state.
fn is_script_defined_name(name: &str) -> bool {
    !BUILTIN_NAMES.contains(&name)
        && !PRELUDE_NAMES.contains(&name)
        && !STDLIB_NAMES.contains(&name)
}

/// Whether a global name is eligible for [`Vm::hot_reload_lua_source`]'s
/// upvalue-join snapshot: same script-defined-vs-API-surface line as
/// [`is_script_defined_name`], except `_init`/`_update`/`_draw` are kept in —
/// they aren't "state" for the debugger's purposes, but they're exactly the
/// closures whose captured locals need joining across a reload.
fn is_reload_join_candidate(name: &str) -> bool {
    if BUILTIN_NAMES.contains(&name) || PRELUDE_NAMES.contains(&name) {
        return false;
    }
    matches!(name, "_init" | "_update" | "_draw") || !STDLIB_NAMES.contains(&name)
}

/// Rebinds `new_fn`'s upvalues onto `old_fn`'s upvalue cells wherever the
/// names match, via raw `lua_upvaluejoin` — this is what makes a chunk-scope
/// `local` variable survive [`Vm::hot_reload_lua_source`] instead of
/// resetting to its initializer: after this call, `new_fn` reads and writes
/// the exact same storage `old_fn` did, for every upvalue name they share.
/// mlua doesn't expose upvalue introspection/joining at the safe API level
/// (the `debug` stdlib isn't loaded — it's flagged unsafe — and isn't
/// available to call from Lua either), so this drops to the raw C API mlua
/// re-exports as `mlua::ffi`, scoped via `Lua::exec_raw` so the stack is
/// restored regardless of outcome.
fn join_matching_upvalues(
    lua: &Lua,
    old_fn: &mlua::Function,
    new_fn: &mlua::Function,
) -> mlua::Result<()> {
    use std::ffi::CStr;
    use std::os::raw::c_int;

    unsafe {
        lua.exec_raw::<()>((old_fn.clone(), new_fn.clone()), |state| {
            // Args land at stack indices 1 (old_fn) and 2 (new_fn), per
            // `exec_raw`'s contract.
            let mut old_names = Vec::new();
            let mut i: c_int = 1;
            loop {
                let name = mlua::ffi::lua_getupvalue(state, 1, i);
                if name.is_null() {
                    break;
                }
                mlua::ffi::lua_pop(state, 1);
                old_names.push((i, CStr::from_ptr(name).to_string_lossy().into_owned()));
                i += 1;
            }

            let mut j: c_int = 1;
            loop {
                let name = mlua::ffi::lua_getupvalue(state, 2, j);
                if name.is_null() {
                    break;
                }
                mlua::ffi::lua_pop(state, 1);
                let new_name = CStr::from_ptr(name).to_string_lossy().into_owned();
                if let Some((old_index, _)) =
                    old_names.iter().find(|(_, old_name)| *old_name == new_name)
                {
                    mlua::ffi::lua_upvaluejoin(state, 2, j, 1, *old_index);
                }
                j += 1;
            }
        })
    }
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

/// Replaces Lua's stdout-backed `print` with a VM-owned line buffer. Frontends
/// can drain it without redirecting process stdout or parsing log messages.
fn register_print_sink(lua: &Lua, output: Arc<Mutex<Vec<String>>>) -> mlua::Result<()> {
    let print = lua.create_function(move |lua, values: MultiValue| {
        let tostring: mlua::Function = lua.globals().get("tostring")?;
        let mut parts = Vec::with_capacity(values.len());
        for value in values {
            let rendered: mlua::String = tostring.call(value)?;
            parts.push(rendered.to_string_lossy());
        }
        let mut output = output
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        for line in parts.join("\t").split('\n') {
            if output.len() == MAX_CAPTURED_OUTPUT_LINES {
                output.remove(0);
            }
            output.push(line.to_string());
        }
        Ok(())
    })?;
    lua.globals().set("print", print)
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
    asset_banks: &'env RefCell<&'env mut AssetBanks>,
    save_data: &'env RefCell<&'env mut SaveData>,
    collision_types: &'env [caiven_core::CollisionType],
    input: &'env Input,
    font: &'env Font,
    sprite_size: u32,
    width: u32,
    height: u32,
    frame_count: u32,
) -> mlua::Result<()> {
    globals.set("SPRITE_SIZE", sprite_size)?;

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
        scope.create_function_mut(
            move |_,
                  (sprite_id, x, y, flip_x, flip_y, rotate): (
                u8,
                i64,
                i64,
                Option<bool>,
                Option<bool>,
                Option<i64>,
            )| {
                let rotate_steps = match rotate.unwrap_or(0) {
                    0 => 0,
                    90 => 1,
                    180 => 2,
                    270 => 3,
                    other => {
                        return Err(mlua::Error::RuntimeError(format!(
                            "sprite: rotate must be 0, 90, 180, or 270 (got {other})"
                        )));
                    }
                };
                let flip_x = flip_x.unwrap_or(false);
                let flip_y = flip_y.unwrap_or(false);

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
                        // Rotate (clockwise) about the sprite's own square, then flip.
                        let (mut rx, mut ry) = match rotate_steps {
                            0 => (sx, sy),
                            1 => (ss - 1 - sy, sx),
                            2 => (ss - 1 - sx, ss - 1 - sy),
                            _ => (sy, ss - 1 - sx),
                        };
                        if flip_x {
                            rx = ss - 1 - rx;
                        }
                        if flip_y {
                            ry = ss - 1 - ry;
                        }
                        let color = palette.borrow().get_color(pixel as usize);
                        plot(&mut w, x + rx - cam_x, y + ry - cam_y, color);
                    }
                }
                Ok(())
            },
        )?,
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
        "get_collision",
        scope.create_function(|_, (tx, ty): (i64, i64)| {
            if !(0..MAP_W as i64).contains(&tx) || !(0..MAP_H as i64).contains(&ty) {
                return Ok(0u8);
            }
            Ok(memory
                .borrow()
                .read(COLLISION_RAM_BASE + ty as usize * MAP_W + tx as usize)
                .unwrap_or(0))
        })?,
    )?;

    globals.set(
        "set_collision",
        scope.create_function_mut(|_, (tx, ty, value): (i64, i64, u8)| {
            if (0..MAP_W as i64).contains(&tx) && (0..MAP_H as i64).contains(&ty) {
                let _ = memory.borrow_mut().write(
                    COLLISION_RAM_BASE + ty as usize * MAP_W + tx as usize,
                    value,
                );
            }
            Ok(())
        })?,
    )?;

    globals.set(
        "collision_type_id",
        scope.create_function(move |_, name: String| {
            Ok(caiven_core::collision_type_by_name(collision_types, &name)
                .map(|t| t.id)
                .unwrap_or(0))
        })?,
    )?;

    globals.set(
        "collision_type_name",
        scope.create_function(move |_, id: u8| {
            Ok(caiven_core::collision_type_by_id(collision_types, id)
                .map(|t| t.name.clone())
                .unwrap_or_default())
        })?,
    )?;

    globals.set(
        "collision_is_solid",
        scope
            .create_function(move |_, id: u8| Ok(caiven_core::is_solid_id(collision_types, id)))?,
    )?;

    globals.set(
        "load_sprite_bank",
        scope.create_function_mut(|_, id: u8| {
            Ok(asset_banks.borrow_mut().select_with_companion(
                AssetBankKind::Sprites,
                id,
                &mut memory.borrow_mut(),
            ))
        })?,
    )?;

    globals.set(
        "load_map_bank",
        scope.create_function_mut(|_, id: u8| {
            Ok(asset_banks.borrow_mut().select_with_companion(
                AssetBankKind::Map,
                id,
                &mut memory.borrow_mut(),
            ))
        })?,
    )?;

    globals.set(
        "load_palette_bank",
        scope.create_function_mut(|_, id: u8| {
            let selected = asset_banks.borrow_mut().select_with_companion(
                AssetBankKind::Palette,
                id,
                &mut memory.borrow_mut(),
            );
            // Bank switches only move raw bytes through Memory; the
            // render-time Palette (parsed Color list) needs an explicit
            // refresh or on-screen colors would keep showing the bank that
            // was active before this call.
            if selected {
                let mem = memory.borrow();
                let mut colors = palette.borrow_mut();
                for i in 0..16usize {
                    let r = mem.read(PALETTE_RAM_BASE + i * 3).unwrap_or(0);
                    let g = mem.read(PALETTE_RAM_BASE + i * 3 + 1).unwrap_or(0);
                    let b = mem.read(PALETTE_RAM_BASE + i * 3 + 2).unwrap_or(0);
                    colors.set_color(i, Color::new_rgb(r, g, b));
                }
            }
            Ok(selected)
        })?,
    )?;

    globals.set(
        "load_sfx_bank",
        scope.create_function_mut(|_, id: u8| {
            Ok(asset_banks.borrow_mut().select_with_companion(
                AssetBankKind::Sfx,
                id,
                &mut memory.borrow_mut(),
            ))
        })?,
    )?;

    globals.set(
        "load_music_bank",
        scope.create_function_mut(|_, id: u8| {
            Ok(asset_banks.borrow_mut().select_with_companion(
                AssetBankKind::Music,
                id,
                &mut memory.borrow_mut(),
            ))
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

    globals.set(
        "real_time",
        scope.create_function(|_, ()| {
            let mem = memory.borrow();
            let hour = mem.read(RTC_RAM_BASE).unwrap_or(0);
            let minute = mem.read(RTC_RAM_BASE + 1).unwrap_or(0);
            let second = mem.read(RTC_RAM_BASE + 2).unwrap_or(0);
            Ok((hour, minute, second))
        })?,
    )?;

    globals.set(
        "frame_count",
        scope.create_function(move |_, ()| Ok(frame_count))?,
    )?;

    globals.set(
        "time",
        scope.create_function(move |_, ()| Ok(frame_count as f64 / TARGET_FPS))?,
    )?;

    globals.set(
        "dset",
        scope.create_function(move |_, (slot, value): (i64, f64)| {
            let slot: u8 = slot.try_into().map_err(|_| {
                mlua::Error::RuntimeError(SaveDataError::SlotOutOfRange(slot as u8).to_string())
            })?;
            save_data
                .borrow_mut()
                .set_slot(slot, value)
                .map_err(|e| mlua::Error::RuntimeError(e.to_string()))
        })?,
    )?;

    globals.set(
        "dget",
        scope.create_function(move |_, slot: i64| {
            let slot: u8 = slot.try_into().unwrap_or(u8::MAX);
            if slot as usize >= crate::vm::SAVE_DATA_SLOT_COUNT {
                return Err(mlua::Error::RuntimeError(
                    SaveDataError::SlotOutOfRange(slot).to_string(),
                ));
            }
            Ok(save_data.borrow().get_slot(slot))
        })?,
    )?;

    globals.set(
        "save_data",
        scope.create_function(move |lua, table: mlua::Table| {
            let value: serde_json::Value = lua.from_value(mlua::Value::Table(table))?;
            save_data
                .borrow_mut()
                .set_blob(value)
                .map_err(|e| mlua::Error::RuntimeError(e.to_string()))
        })?,
    )?;

    globals.set(
        "load_data",
        scope.create_function(move |lua, ()| lua.to_value(save_data.borrow().blob()))?,
    )?;

    Ok(())
}

impl Vm {
    /// Loads Lua source, registering the full builtin API first so top-level
    /// script code and `_init()` (called once here, if present) can use it
    /// exactly like `_update()` can. Subsequent frames call `_update()` via
    /// [`Vm::run_frame`].
    pub fn load_lua_source(&mut self, src: &str, input: &Input, font: &Font) -> mlua::Result<()> {
        // Cart Lua must not reach the filesystem/process: mask out io/os at the
        // StdLib level, then null dofile/loadfile below since they bypass the
        // StdLib mask entirely. PACKAGE stays enabled (mlua auto-disables
        // package.loadlib and the C searchers for it — see `disable_c_modules`)
        // because `require`/`package.preload` back the multi-module bundling
        // format (`caiven_cart::bundle_lua`).
        let lua = Lua::new_with(
            StdLib::COROUTINE
                | StdLib::TABLE
                | StdLib::STRING
                | StdLib::UTF8
                | StdLib::MATH
                | StdLib::PACKAGE,
            mlua::LuaOptions::default(),
        )?;
        {
            let globals = lua.globals();
            let package: Table = globals.get("package")?;
            package.set("path", "")?;
            package.set("cpath", "")?;

            // `disable_c_modules` (called by `Lua::new_with` for StdLib::PACKAGE)
            // only neuters the C-loader searchers (index 3, and removes index 4).
            // Index 2 is the stock Lua-file searcher, which walks `package.path`
            // via C `fopen` regardless of what we set `package.path` to above —
            // a cart could reassign `package.path` at runtime and have `require`
            // read arbitrary files. Remove every searcher but index 1 (preload)
            // so `require` can never resolve anything outside `package.preload`,
            // no matter what a cart later does to `package.path`/`cpath`.
            let searchers: Table = package.get("searchers")?;
            for i in 2..=4 {
                searchers.raw_set(i, mlua::Nil)?;
            }

            // The base library (always loaded regardless of the StdLib mask)
            // exposes `load` with its default "bt" mode, which accepts
            // precompiled Lua bytecode strings — a known memory-safety hazard
            // independent of filesystem access. Replace it with a wrapper that
            // forces text-only ("t") mode, keeping the source-text use that
            // `caiven_cart::bundle_lua` depends on while rejecting bytecode.
            // The 4th arg (`env`) sets the loaded chunk's `_ENV` upvalue only
            // when actually *passed* — real Lua distinguishes "argument
            // omitted" (chunk inherits the caller's globals) from "argument
            // is nil" (chunk gets a nil `_ENV`, so any global access inside
            // it errors). Forward it only when the cart's call actually
            // supplied one, so callers that omit it (like `bundle_lua`'s
            // generated `load(src, name)`) keep the normal global env.
            let base_load: mlua::Function = globals.get("load")?;
            let text_only_load = lua.create_function(move |_, args: mlua::MultiValue| {
                let args: Vec<mlua::Value> = args.into_iter().collect();
                let chunk = args.first().cloned().unwrap_or(mlua::Value::Nil);
                let chunkname = args.get(1).cloned().unwrap_or(mlua::Value::Nil);
                match args.get(3) {
                    Some(env) => {
                        base_load.call::<mlua::MultiValue>((chunk, chunkname, "t", env.clone()))
                    }
                    None => base_load.call::<mlua::MultiValue>((chunk, chunkname, "t")),
                }
            })?;
            globals.set("load", text_only_load)?;
        }
        let output = Arc::new(Mutex::new(Vec::new()));
        if self.capture_lua_output {
            register_print_sink(&lua, Arc::clone(&output))?;
        }

        let world = RefCell::new(&mut self.world);
        let ui = RefCell::new(&mut self.ui);
        let memory = RefCell::new(&mut self.memory);
        let palette = RefCell::new(&mut self.palette);
        let camera = RefCell::new(&mut self.camera);
        let sfx_player = RefCell::new(&mut self.sfx_player);
        let music_player = RefCell::new(&mut self.music_player);
        let asset_banks = RefCell::new(&mut self.asset_banks);
        let save_data = RefCell::new(&mut self.save_data);
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
                &asset_banks,
                &save_data,
                &self.collision_types,
                input,
                font,
                sprite_size,
                width,
                height,
                self.frame_count,
            )?;

            for name in ["dofile", "loadfile"] {
                globals.set(name, mlua::Nil)?;
            }

            lua.load(PRELUDE_SOURCE).set_name("=prelude").exec()?;
            lua.load(src).set_name(CHUNK_SOURCE_NAME).exec()?;
            if let Ok(init) = globals.get::<mlua::Function>("_init") {
                init.call::<()>(())?;
            }
            Ok(())
        });
        result?;

        self.script = Some(LuaScript { lua, output });
        self.fault = None;
        self.waiting = false;
        self.call_stack.clear();
        Ok(())
    }

    /// Drains complete lines emitted by cart `print()` calls since last read.
    pub fn take_lua_output(&mut self) -> Vec<String> {
        let Some(script) = self.script.as_ref() else {
            return Vec::new();
        };
        let mut output = script
            .output
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        std::mem::take(&mut *output)
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
        let asset_banks = RefCell::new(&mut self.asset_banks);
        let save_data = RefCell::new(&mut self.save_data);
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
                &asset_banks,
                &save_data,
                &self.collision_types,
                input,
                font,
                sprite_size,
                width,
                height,
                self.frame_count,
            )?;

            let update: mlua::Function = globals.get("_update")?;
            update.call::<()>(())?;
            if let Ok(draw) = globals.get::<mlua::Function>("_draw") {
                draw.call::<()>(())?;
            }
            Ok(())
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
    /// globals and RAM at the moment of the stop are readable via
    /// [`Vm::lua_globals`] and `peek_memory`; locals are readable too, via
    /// [`Vm::lua_debug_locals`] — mlua's safe hook API has no `lua_getlocal`
    /// binding, so that path drops to raw `mlua_sys` FFI (see
    /// [`read_active_locals`]). Resuming re-runs `_update()` from the top,
    /// same as any other frame.
    pub fn run_frame_lua_bp(
        &mut self,
        input: &Input,
        font: &Font,
        breakpoints: &[LuaBreakpoint],
    ) -> LuaRunOutcome {
        // `run_frame` ticks these; this path grew separately and didn't, so
        // Studio's Running state was silent even though a sound was "active".
        self.tick_audio_players();
        self.peripherals
            .tick_all(&mut self.memory, self.frame_count);
        self.frame_count = self.frame_count.wrapping_add(1);

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
        let asset_banks = RefCell::new(&mut self.asset_banks);
        let save_data = RefCell::new(&mut self.save_data);
        let sprite_size = self.config.sprite_size;
        let width = self.config.width;
        let height = self.config.height;

        let hit: Rc<RefCell<Option<LuaBreakpoint>>> = Rc::new(RefCell::new(None));
        let stack: Rc<RefCell<Vec<(String, String)>>> = Rc::new(RefCell::new(Vec::new()));
        let locals: Rc<RefCell<Vec<(String, String)>>> = Rc::new(RefCell::new(Vec::new()));
        // EVERY_LINE fires a Rust callback per Lua instruction executed —
        // real overhead on any script with loops. Only pay for it when
        // there's actually something to break on.
        if !breakpoints.is_empty() {
            let hit_hook = hit.clone();
            let stack_hook = stack.clone();
            let locals_hook = locals.clone();
            let bps: Vec<LuaBreakpoint> = breakpoints.to_vec();
            lua.set_hook(HookTriggers::EVERY_LINE, move |lua, debug| {
                let line = debug.curr_line();
                let debug_source = debug.source();
                let source = debug_source
                    .short_src
                    .as_deref()
                    .or(debug_source.source.as_deref())
                    .map(normalized_debug_source)
                    .unwrap_or_else(|| "cart".to_string());
                let matched = (line > 0)
                    .then(|| {
                        bps.iter().find(|breakpoint| {
                            breakpoint.line == line as usize
                                && (breakpoint.source == "*"
                                    || normalized_debug_source(&breakpoint.source) == source)
                        })
                    })
                    .flatten();
                if let Some(breakpoint) = matched {
                    *hit_hook.borrow_mut() = Some(LuaBreakpoint {
                        source,
                        line: breakpoint.line,
                    });
                    *stack_hook.borrow_mut() = capture_call_stack(lua);
                    // Reentrant `exec_raw` call from inside this
                    // already-active hook — proven safe by the T7 spike
                    // (R4). Reads locals via raw `mlua_sys` FFI since mlua's
                    // safe hook API has no `lua_getlocal` binding (R1, V23).
                    // `exec_raw`'s `R` is read off the Lua stack, not the
                    // closure's return value, so the result is threaded out
                    // via the captured cell instead.
                    let mut read_locals = Vec::new();
                    let _: mlua::Result<()> = unsafe {
                        lua.exec_raw((), |state| read_locals = read_active_locals(state))
                    };
                    *locals_hook.borrow_mut() = read_locals;
                    return Err(mlua::Error::runtime("breakpoint"));
                }
                Ok(VmState::Continue)
            });
        }

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
                &asset_banks,
                &save_data,
                &self.collision_types,
                input,
                font,
                sprite_size,
                width,
                height,
                self.frame_count,
            )?;

            let update: mlua::Function = globals.get("_update")?;
            update.call::<()>(())?;
            if let Ok(draw) = globals.get::<mlua::Function>("_draw") {
                draw.call::<()>(())?;
            }
            Ok(())
        });
        lua.remove_hook();

        if hit.borrow().is_some() {
            self.call_stack = stack.borrow().clone();
            self.locals = locals.borrow().clone();
        } else {
            self.call_stack.clear();
            self.locals.clear();
        }

        let breakpoint = hit.borrow().clone();
        match (breakpoint, result) {
            (Some(breakpoint), _) => LuaRunOutcome::Breakpoint(breakpoint),
            (None, Ok(())) => LuaRunOutcome::Completed,
            (None, Err(e)) => {
                log::error!("Lua runtime error: {e}");
                self.set_fault(VmFault::LuaError);
                let (location, message) = describe_lua_error_location(&e);
                LuaRunOutcome::Error(location, message)
            }
        }
    }

    /// The Lua call stack captured at the moment the last breakpoint was
    /// hit, deepest frame first — cleared once execution resumes past a
    /// breakpoint. Each entry is `(frame label, "file:line")`.
    pub fn lua_call_stack(&self) -> Vec<(String, String)> {
        self.call_stack.clone()
    }

    /// Local variables at the innermost frame, captured at the moment the
    /// last breakpoint was hit — cleared once execution resumes past a
    /// breakpoint. Read via raw FFI from inside the `EVERY_LINE` hook (see
    /// [`read_active_locals`]); empty if no breakpoint has fired yet.
    pub fn lua_debug_locals(&self) -> Vec<(String, String)> {
        self.locals.clone()
    }

    /// Snapshot of the script's global variables, for the Studio debugger's
    /// state inspector. Excludes registered builtins, the gameplay prelude,
    /// and Lua's own stdlib — see [`BUILTIN_NAMES`]/[`PRELUDE_NAMES`]/
    /// [`STDLIB_NAMES`] — so only script-defined state shows up. For locals
    /// at a breakpoint, see [`Vm::lua_debug_locals`].
    pub fn lua_globals(&self) -> Vec<(String, String)> {
        let Some(script) = self.script.as_ref() else {
            return Vec::new();
        };
        let globals = script.lua.globals();
        let mut out: Vec<(String, String)> = globals
            .pairs::<String, mlua::Value>()
            .filter_map(|pair| pair.ok())
            .filter(|(k, _)| is_script_defined_name(k))
            .map(|(k, v)| (k, describe_lua_value(&v)))
            .collect();
        out.sort_by(|a, b| a.0.cmp(&b.0));
        out
    }

    /// Hot-reloads Lua source onto the *already running* script instance,
    /// preserving state instead of rebuilding a fresh `Lua` VM and re-running
    /// `_init()` the way [`Vm::load_lua_source`] does. Falls back to a full
    /// [`Vm::load_lua_source`] if nothing is running yet — there is no state
    /// to preserve on a first load.
    ///
    /// The project's tutorial idiom keeps state as top-level `local`
    /// variables (chunk upvalues captured by `_init`/`_update`/`_draw`), not
    /// Lua globals — Lua has no generic way to enumerate or snapshot those
    /// upvalues, so a naive "just re-run the chunk" reload would reinitialize
    /// exactly the state this is meant to preserve. Instead: the new chunk
    /// executes on the *same* live `Lua` instance (upvalues cannot be joined
    /// across two separate `Lua` states), and for every script-defined
    /// function whose name exists both before and after the reload, the new
    /// closure's upvalues are rebound (matched by name) onto the old
    /// closure's upvalue cells via `lua_upvaluejoin`, so `_update`/`_draw` —
    /// looked up fresh from globals every frame — pick up the preserved state
    /// on the very next frame. Unmatched names (renamed/removed/new
    /// variables) simply keep the fresh initializer from this reload — no
    /// error, best-effort by design, matching how existing Lua hot-reload
    /// tooling behaves.
    ///
    /// The new source is syntax-checked (compiled without executing) before
    /// anything else runs, so a typo mid-edit leaves the live state fully
    /// untouched. A runtime error during the new chunk's *top-level*
    /// execution (not inside a function body) is a narrower, documented risk:
    /// Lua has no transactional exec, so some globals may already be
    /// reassigned by the time such an error surfaces. This project's
    /// convention keeps top-level code to pure declarations, so this is not
    /// expected to be reachable in practice; Reset remains the recovery path
    /// if it is.
    pub fn hot_reload_lua_source(
        &mut self,
        src: &str,
        input: &Input,
        font: &Font,
    ) -> mlua::Result<()> {
        let Some(script) = self.script.as_ref() else {
            return self.load_lua_source(src, input, font);
        };

        // Syntax-check first: compiling without executing means a bad chunk
        // never touches the live instance at all.
        script
            .lua
            .load(src)
            .set_name(CHUNK_SOURCE_NAME)
            .into_function()?;

        // Snapshot old script-defined top-level functions by name, to join
        // upvalues against once the new chunk has executed.
        let old_functions: Vec<(String, mlua::Function)> = {
            let globals = script.lua.globals();
            globals
                .pairs::<String, mlua::Value>()
                .filter_map(|pair| pair.ok())
                .filter(|(name, _)| is_reload_join_candidate(name))
                .filter_map(|(name, value)| match value {
                    mlua::Value::Function(f) => Some((name, f)),
                    _ => None,
                })
                .collect()
        };
        let lua = &script.lua;

        let world = RefCell::new(&mut self.world);
        let ui = RefCell::new(&mut self.ui);
        let memory = RefCell::new(&mut self.memory);
        let palette = RefCell::new(&mut self.palette);
        let camera = RefCell::new(&mut self.camera);
        let sfx_player = RefCell::new(&mut self.sfx_player);
        let music_player = RefCell::new(&mut self.music_player);
        let asset_banks = RefCell::new(&mut self.asset_banks);
        let save_data = RefCell::new(&mut self.save_data);
        let sprite_size = self.config.sprite_size;
        let width = self.config.width;
        let height = self.config.height;
        let frame_count = self.frame_count;
        let collision_types = &self.collision_types;

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
                &asset_banks,
                &save_data,
                collision_types,
                input,
                font,
                sprite_size,
                width,
                height,
                frame_count,
            )?;

            lua.load(PRELUDE_SOURCE).set_name("=prelude").exec()?;
            lua.load(src).set_name(CHUNK_SOURCE_NAME).exec()?;
            // Deliberately not calling `_init()` — that's what makes this a
            // reload rather than a reset.
            Ok(())
        });
        result?;

        let globals = lua.globals();
        for (name, old_fn) in &old_functions {
            if let Ok(new_fn) = globals.get::<mlua::Function>(name.as_str()) {
                join_matching_upvalues(lua, old_fn, &new_fn)?;
            }
        }

        self.fault = None;
        self.waiting = false;
        self.call_stack.clear();
        Ok(())
    }

    /// Reads a dotted global/table path without executing Lua. Studio uses
    /// this for debugger watches, so expressions cannot mutate cart state.
    pub fn lua_watch(&self, expression: &str) -> Result<String, String> {
        let parts: Vec<_> = expression.split('.').collect();
        if parts.is_empty()
            || parts.iter().any(|part| {
                part.is_empty()
                    || !part
                        .chars()
                        .all(|char| char == '_' || char.is_ascii_alphanumeric())
                    || part
                        .chars()
                        .next()
                        .is_some_and(|char| char.is_ascii_digit())
            })
        {
            return Err("Watch must be a dotted identifier".to_string());
        }
        let script = self
            .script
            .as_ref()
            .ok_or_else(|| "No Lua cart loaded".to_string())?;
        let mut value: mlua::Value = script
            .lua
            .globals()
            .get(parts[0])
            .map_err(|error| error.to_string())?;
        for part in &parts[1..] {
            value = match value {
                mlua::Value::Table(table) => table.get(*part).map_err(|error| error.to_string())?,
                _ => return Err(format!("{} is not a table", expression)),
            };
        }
        if matches!(value, mlua::Value::Nil) {
            Err("nil".to_string())
        } else {
            Ok(describe_lua_value(&value))
        }
    }
}

#[cfg(test)]
mod watch_tests {
    use crate::input::Input;
    use crate::rendering::font::Font;
    use crate::{Vm, VmConfig};

    #[test]
    fn dotted_watch_reads_without_executing_code() {
        let mut vm = Vm::new(VmConfig::default());
        vm.load_lua_source(
            "player = { x = 72, nested = { alive = true } }\nfunction _update() end",
            &Input::new(),
            &Font::empty(),
        )
        .expect("watch fixture should load");
        assert_eq!(vm.lua_watch("player.x"), Ok("72".to_string()));
        assert_eq!(vm.lua_watch("player.nested.alive"), Ok("true".to_string()));
        assert!(vm.lua_watch("player.x + 1").is_err());
        assert!(!vm.lua_globals().iter().any(|(name, _)| name == "warn"));
    }

    #[test]
    fn print_stream_is_buffered_and_drained() {
        let input = Input::new();
        let font = Font::empty();
        let mut vm = Vm::new(VmConfig::default());
        vm.set_lua_output_capture(true);
        vm.load_lua_source(
            r#"
print("load", 7, true)
function _init() print("init") end
function _update() print("frame") end
"#,
            &input,
            &font,
        )
        .expect("captured print fixture should load");

        assert_eq!(vm.take_lua_output(), vec!["load\t7\ttrue", "init"]);
        assert!(vm.take_lua_output().is_empty());

        vm.run_frame_lua(&input, &font);
        assert_eq!(vm.take_lua_output(), vec!["frame"]);
    }

    #[test]
    fn print_stream_is_bounded_before_frontend_drain() {
        let mut vm = Vm::new(VmConfig::default());
        vm.set_lua_output_capture(true);
        vm.load_lua_source(
            "for i = 1, 205 do print(i) end\nfunction _update() end",
            &Input::new(),
            &Font::empty(),
        )
        .expect("bounded print fixture should load");

        let output = vm.take_lua_output();
        assert_eq!(output.len(), 200);
        assert_eq!(output.first().map(String::as_str), Some("6"));
        assert_eq!(output.last().map(String::as_str), Some("205"));
    }

    #[test]
    fn print_capture_is_opt_in() {
        let mut vm = Vm::new(VmConfig::default());
        vm.load_lua_source(
            "print('native stdout')\nfunction _update() end",
            &Input::new(),
            &Font::empty(),
        )
        .expect("native print fixture should load");

        assert!(vm.take_lua_output().is_empty());
    }
}

#[cfg(test)]
mod hot_reload_tests {
    use crate::input::Input;
    use crate::rendering::font::Font;
    use crate::{Vm, VmConfig};

    fn get_score(vm: &Vm) -> i64 {
        vm.script
            .as_ref()
            .expect("script should be loaded")
            .lua
            .globals()
            .get::<mlua::Function>("get_score")
            .expect("get_score should be defined")
            .call::<i64>(())
            .expect("get_score should not error")
    }

    #[test]
    fn hot_reload_preserves_top_level_locals_matched_by_name() {
        let input = Input::new();
        let font = Font::empty();
        let mut vm = Vm::new(VmConfig::default());
        vm.load_lua_source(
            r#"
local score = 0
function _update() score = score + 1 end
function get_score() return score end
"#,
            &input,
            &font,
        )
        .expect("chunk A should load");

        vm.run_frame_lua(&input, &font);
        vm.run_frame_lua(&input, &font);
        vm.run_frame_lua(&input, &font);
        assert_eq!(get_score(&vm), 3);

        // Same variable name, different `_update` body — should preserve the
        // existing `score` upvalue instead of resetting it to 0.
        vm.hot_reload_lua_source(
            r#"
local score = 0
function _update() score = score + 2 end
function get_score() return score end
"#,
            &input,
            &font,
        )
        .expect("hot reload with matching names should succeed");

        assert_eq!(
            get_score(&vm),
            3,
            "state should survive a matching-name reload"
        );

        vm.run_frame_lua(&input, &font);
        assert_eq!(
            get_score(&vm),
            5,
            "reloaded _update body should apply to preserved state"
        );
    }

    #[test]
    fn hot_reload_resets_renamed_locals_to_their_fresh_initializer() {
        let input = Input::new();
        let font = Font::empty();
        let mut vm = Vm::new(VmConfig::default());
        vm.load_lua_source(
            r#"
local score = 0
function _update() score = score + 1 end
function get_score() return score end
"#,
            &input,
            &font,
        )
        .expect("chunk A should load");

        vm.run_frame_lua(&input, &font);
        vm.run_frame_lua(&input, &font);
        vm.run_frame_lua(&input, &font);
        assert_eq!(get_score(&vm), 3);

        // Renamed local — no upvalue name match, so it can't be preserved;
        // this must not error, it just falls back to the fresh initializer.
        vm.hot_reload_lua_source(
            r#"
local points = 0
function _update() points = points + 1 end
function get_score() return points end
"#,
            &input,
            &font,
        )
        .expect("hot reload with a renamed local should still succeed");

        assert_eq!(
            get_score(&vm),
            0,
            "renamed local has no match, resets to its initializer"
        );
    }

    #[test]
    fn hot_reload_syntax_error_leaves_running_script_untouched() {
        let input = Input::new();
        let font = Font::empty();
        let mut vm = Vm::new(VmConfig::default());
        vm.load_lua_source(
            r#"
local score = 0
function _update() score = score + 1 end
function get_score() return score end
"#,
            &input,
            &font,
        )
        .expect("chunk A should load");

        vm.run_frame_lua(&input, &font);
        vm.run_frame_lua(&input, &font);
        vm.run_frame_lua(&input, &font);
        assert_eq!(get_score(&vm), 3);

        let result = vm.hot_reload_lua_source(
            "function _update( score = score + 1 end", // missing closing paren
            &input,
            &font,
        );
        assert!(result.is_err());

        assert_eq!(get_score(&vm), 3, "old state must survive a failed reload");
        vm.run_frame_lua(&input, &font);
        assert_eq!(
            get_score(&vm),
            4,
            "old _update must still be callable after a failed reload"
        );
    }
}
