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
//! Builtin globals are installed once per Lua state as trampolines; each
//! scope (load, frame, hot reload) fills their implementations through
//! `register_builtins`, so the API surface can't drift between call sites.

use super::audio::{SFX_VOICE_COUNT, Sound};
use super::memory::Memory;
use super::palette::Palette;
use super::save_data::SaveData;
use super::sfx::{MusicPlayer, resolve_song_step};
use super::{
    AssetBankKind, AssetBanks, Camera, PooledSfx, Vm, VmFault, allocate_sfx_voice,
    release_sfx_voice, silence_music_voices, unpack_sfx_handle,
};
use crate::input::{Button, Input};
use crate::rendering::font::Font;
use crate::rendering::screen::ScreenLayer;
use crate::rendering::text::draw_text_at;
use caiven_core::memory::{
    COLLISION_RAM_BASE, MAP_H, MAP_RAM_BASE, MAP_W, MUSIC_ORDER_STEPS, PALETTE_RAM_BASE,
    RTC_RAM_BASE, SFX_COUNT, SPRITE_BYTES, SPRITE_COUNT, SPRITE_SHEET_COLS, SPRITE_SHEET_RAM_BASE,
};
use caiven_core::{Color, Vec2};
use mlua::{Debug, HookTriggers, Lua, LuaSerdeExt, MultiValue, Scope, StdLib, Table, VmState};
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
    "button_released",
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
    "collision_is_one_way",
    "collision_is_slope_left",
    "collision_is_slope_right",
    "load_sprite_bank",
    "load_map_bank",
    "load_palette_bank",
    "load_sfx_bank",
    "load_music_bank",
    "play_sfx",
    "stop_sfx",
    "is_sfx_playing",
    "play_music",
    "play_music_song",
    "stop_music",
    "is_music_playing",
    "set_master_volume",
    "set_music_volume",
    "set_sfx_volume",
    "real_time",
    "frame_count",
    "time",
    "SPRITE_SIZE",
    "save_data",
    "load_data",
];

/// Names defined by the always-on prelude core ([`PRELUDE_CORE`]) — also
/// excluded from [`Vm::lua_globals`]'s snapshot, same reasoning as
/// `BUILTIN_NAMES`: API surface, not script state.
const CORE_PRELUDE_NAMES: &[&str] = &[
    "RTK_SEEDED",
    "random_range",
    "random_float",
    "choice",
    "shuffle",
    "lerp",
    "clamp",
    "ease_linear",
    "ease_in_quad",
    "ease_out_quad",
    "ease_in_out_quad",
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

/// Always-on prelude core (RNG, lerp/clamp/easing) — pure Lua, loaded into
/// globals before every module and the cart's own source, so it's available
/// from `_init()` onward like any builtin. Every cart gets this regardless of
/// which [`PRELUDE_MODULES`] it selects.
const PRELUDE_CORE: &str = include_str!("prelude/core.lua");

/// One opt-in gameplay-stdlib module: a pure-Lua source chunk plus the global
/// names it defines (used to keep [`Vm::lua_globals`]'s exclusion set and
/// hot-reload's upvalue-join filter in sync with whichever modules are
/// actually loaded for a cart).
struct PreludeModule {
    /// Manifest-facing id — what a cart's `caiven.toml` `[stdlib] modules`
    /// entry names to opt in.
    name: &'static str,
    source: &'static str,
    globals: &'static [&'static str],
}

/// Opt-in gameplay-facing stdlib (Vec2/Sprite, AABB/tile collision, swept
/// movement, tweens, particles, Scenes, Entities, Camera). Loaded in this
/// order after [`PRELUDE_CORE`] and before the cart's own source.
const PRELUDE_MODULES: &[PreludeModule] = &[
    PreludeModule {
        name: "vec2",
        source: include_str!("prelude/vec2.lua"),
        globals: &["Vec2", "Sprite"],
    },
    PreludeModule {
        name: "collision",
        source: include_str!("prelude/collision.lua"),
        globals: &[
            "aabb_overlap",
            "circle_overlap",
            "point_in_rect",
            "point_in_circle",
            "tile_solid",
            "box_touches_solid",
        ],
    },
    PreludeModule {
        name: "movement",
        source: include_str!("prelude/movement.lua"),
        globals: &["move_and_collide"],
    },
    PreludeModule {
        name: "tween",
        source: include_str!("prelude/tween.lua"),
        globals: &[
            "new_tween",
            "tween_update",
            "new_anim",
            "anim_update",
            "anim_sprite",
        ],
    },
    PreludeModule {
        name: "particles",
        source: include_str!("prelude/particles.lua"),
        globals: &["Particles"],
    },
    PreludeModule {
        name: "scenes",
        source: include_str!("prelude/scenes.lua"),
        globals: &["Scenes"],
    },
    PreludeModule {
        name: "entities",
        source: include_str!("prelude/entities.lua"),
        globals: &["Entities"],
    },
    PreludeModule {
        name: "camera",
        source: include_str!("prelude/camera.lua"),
        globals: &["Camera"],
    },
];

/// Always-on prelude-core global names — exposed for `api_registry`'s
/// `PRELUDE`-vs-registry drift test; nothing outside tests needs this.
#[cfg(test)]
pub(super) fn core_prelude_names() -> &'static [&'static str] {
    CORE_PRELUDE_NAMES
}

/// Each opt-in prelude module's manifest name and the globals it defines —
/// same purpose as [`core_prelude_names`], for the modules rather than core.
pub(super) fn prelude_module_globals() -> Vec<(&'static str, &'static [&'static str])> {
    PRELUDE_MODULES
        .iter()
        .map(|module| (module.name, module.globals))
        .collect()
}

/// Manifest-facing catalog of opt-in prelude modules and the globals each
/// defines — what Studio uses to build the enable/disable UI and the
/// disabled-module editor diagnostic.
pub fn prelude_module_catalog() -> Vec<(&'static str, &'static [&'static str])> {
    prelude_module_globals()
}

/// Frames per second `time()` assumes when converting `frame_count`.
const TARGET_FPS: f64 = 60.0;
const MAX_CAPTURED_OUTPUT_LINES: usize = 200;

/// Lua VM instructions allowed in one `_update()`/`_draw()` pair before the
/// watchdog assumes an infinite loop and aborts the frame instead of hanging
/// the console. A normal frame (map draw, sprites, particles) runs in the
/// thousands to low millions of instructions, so this leaves generous
/// headroom while still bounding a runaway script to well under a second.
const FRAME_INSTRUCTION_BUDGET: u32 = 25_000_000;
/// Separate, larger budget for [`Vm::load_lua_source`] (prelude + the cart's
/// top-level code + `_init()`) and [`Vm::hot_reload_lua_source`] (prelude +
/// top-level re-exec) — one-time setup legitimately costs more than a single
/// frame, but a hostile cart still can't hang loading forever.
const INIT_INSTRUCTION_BUDGET: u32 = 100_000_000;
/// mlua's instruction-count hook fires once per this many executed
/// instructions — the stride trades check granularity for per-frame
/// overhead, since the hook is Rust code called from inside the Lua VM loop.
const INSTRUCTION_HOOK_STRIDE: u32 = 10_000;
/// Plain-language watchdog message — deliberately not an mlua traceback, per
/// the design charter's "must fail with a line number and a plain-language
/// message" watchdog requirement.
pub(crate) const EXECUTION_BUDGET_MESSAGE: &str =
    "your game did not finish drawing this frame — is there a loop that never ends?";
/// Ceiling on a cart's Lua-heap usage (`Lua::set_memory_limit`) — the
/// instruction-count watchdog bounds CPU time, not memory, so a hostile cart
/// growing an unbounded table needs a separate limit.
const LUA_MEMORY_LIMIT_BYTES: usize = 64 * 1024 * 1024;

pub(super) struct LuaScript {
    lua: Lua,
    output: Arc<Mutex<Vec<String>>>,
    /// Every coroutine a cart has spawned via the wrapped `coroutine.create`/
    /// `coroutine.wrap` installed by [`install_coroutine_budget_guard`] —
    /// re-armed with the active execution budget at the top of every call
    /// site that can resume one, so a coroutine created in an earlier call
    /// counts against *that* call's budget rather than running forever on a
    /// stale or absent hook.
    coroutines: Rc<RefCell<Vec<mlua::Thread>>>,
    /// The budget the wrapped `coroutine.create`/`coroutine.wrap` should arm
    /// a brand-new thread with — set fresh before every chunk/`_init`/
    /// `_update` call, cleared afterward so a thread can never be created
    /// outside a window we control.
    active_budget: Rc<RefCell<Option<BudgetCells>>>,
}

/// `(instruction counter, budget-hit sink, budget)` for one execution-budget
/// hook — shared between the main thread and any coroutine it spawns so they
/// count against the same limit. See [`budget_hook`].
type BudgetCells = (Rc<RefCell<u32>>, Rc<RefCell<Option<LuaBreakpoint>>>, u32);

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

/// A displayed value in the Studio debugger's Locals/Globals/Watches/expand
/// panels. `node_id` is `Some` only for a table or function — the
/// expandable value is rooted under that id in [`Vm`]'s `debug_roots` map
/// (see [`Vm::expand_debug_node`]) so a later expand request can find it;
/// scalars have no children and are never rooted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DebugValue {
    pub text: String,
    pub node_id: Option<String>,
}

/// One raw local: display name, display text, and — for a table/function —
/// the owned value rooted for [`Vm::expand_debug_node`]. See
/// [`read_active_locals`].
pub(super) type RawLocal = (String, String, Option<mlua::Value>);

impl DebugValue {
    pub fn as_str(&self) -> &str {
        &self.text
    }
}

// Lets call sites and tests compare a `DebugValue` against its display
// text directly, without a `.text` projection at every assertion.
impl PartialEq<str> for DebugValue {
    fn eq(&self, other: &str) -> bool {
        self.text == other
    }
}

impl PartialEq<&str> for DebugValue {
    fn eq(&self, other: &&str) -> bool {
        self.text == *other
    }
}

impl PartialEq<String> for DebugValue {
    fn eq(&self, other: &String) -> bool {
        &self.text == other
    }
}

/// Maximum table entries [`Vm::expand_debug_node`] returns for one expand —
/// keeps a single click bounded regardless of cart-authored table size.
const MAX_EXPAND_ENTRIES: usize = 200;

fn normalized_debug_source(source: &str) -> String {
    source.trim_start_matches(['@', '=']).replace('\\', "/")
}

/// Normalized source name for a hook callback's current frame, falling back
/// to `"cart"` when mlua can't resolve one — shared by the breakpoint and
/// execution-budget hooks.
fn hook_debug_source(debug: &mlua::Debug) -> String {
    let debug_source = debug.source();
    debug_source
        .short_src
        .as_deref()
        .or(debug_source.source.as_deref())
        .map(normalized_debug_source)
        .unwrap_or_else(|| "cart".to_string())
}

/// Builds one execution-budget hook closure. Shared by the main thread's own
/// hook and every coroutine's (see [`install_coroutine_budget_guard`]) so a
/// hostile cart can't dodge the watchdog by looping forever inside a
/// coroutine instead of the main chunk — both count against the same
/// `instructions`/`budget_hit` cells.
fn budget_hook(
    instructions: Rc<RefCell<u32>>,
    budget_hit: Rc<RefCell<Option<LuaBreakpoint>>>,
    budget: u32,
) -> impl Fn(&Lua, Debug) -> mlua::Result<VmState> {
    move |_lua, debug| {
        let mut count = instructions.borrow_mut();
        *count += INSTRUCTION_HOOK_STRIDE;
        if *count < budget {
            return Ok(VmState::Continue);
        }
        let line = debug.curr_line();
        *budget_hit.borrow_mut() = (line > 0).then(|| LuaBreakpoint {
            source: hook_debug_source(&debug),
            line: line as usize,
        });
        Err(mlua::Error::runtime(EXECUTION_BUDGET_MESSAGE))
    }
}

/// Re-arms every already-tracked coroutine with this call's execution budget
/// — called alongside `lua.set_hook` on the main thread at the top of
/// [`Vm::load_lua_source`], [`Vm::hot_reload_lua_source`],
/// [`Vm::run_frame_lua`], and [`Vm::run_frame_lua_bp`]. mlua only arms the
/// thread `set_hook` is called on, and a hook set once at coroutine-creation
/// time would otherwise keep counting instructions cumulatively across every
/// later call instead of resetting per call like the main thread's does.
fn rearm_coroutines(
    coroutines: &RefCell<Vec<mlua::Thread>>,
    instructions: &Rc<RefCell<u32>>,
    budget_hit: &Rc<RefCell<Option<LuaBreakpoint>>>,
    budget: u32,
) {
    let triggers = HookTriggers::new().every_nth_instruction(INSTRUCTION_HOOK_STRIDE);
    for thread in coroutines.borrow().iter() {
        thread.set_hook(
            triggers,
            budget_hook(instructions.clone(), budget_hit.clone(), budget),
        );
    }
}

/// Overrides `coroutine.create`/`coroutine.wrap` so any thread a cart spawns
/// is armed with the caller's active execution budget (`active_budget`, kept
/// current by [`rearm_coroutines`]'s callers) the moment it's created, and
/// tracked in `coroutines` so it keeps getting re-armed on every later call
/// too. Installed once, in [`Vm::load_lua_source`] — it survives hot reload
/// since that reuses the same `Lua` instance. See [`budget_hook`] for why
/// this exists at all: a plain `coroutine.create` has no budget whatsoever.
fn install_coroutine_budget_guard(
    lua: &Lua,
    globals: &Table,
    coroutines: Rc<RefCell<Vec<mlua::Thread>>>,
    active_budget: Rc<RefCell<Option<BudgetCells>>>,
) -> mlua::Result<()> {
    let coroutine: Table = globals.get("coroutine")?;

    let arm = move |thread: &mlua::Thread, active_budget: &Rc<RefCell<Option<BudgetCells>>>| {
        if let Some((instructions, budget_hit, budget)) = &*active_budget.borrow() {
            thread.set_hook(
                HookTriggers::new().every_nth_instruction(INSTRUCTION_HOOK_STRIDE),
                budget_hook(instructions.clone(), budget_hit.clone(), *budget),
            );
        }
    };

    let create_coroutines = coroutines.clone();
    let create_budget = active_budget.clone();
    let wrapped_create = lua.create_function(move |lua, f: mlua::Function| {
        let thread = lua.create_thread(f)?;
        arm(&thread, &create_budget);
        create_coroutines.borrow_mut().push(thread.clone());
        Ok(thread)
    })?;
    coroutine.set("create", wrapped_create)?;

    let wrap_coroutines = coroutines;
    let wrap_budget = active_budget;
    let wrapped_wrap = lua.create_function(move |lua, f: mlua::Function| {
        let thread = lua.create_thread(f)?;
        arm(&thread, &wrap_budget);
        wrap_coroutines.borrow_mut().push(thread.clone());
        lua.create_function(move |_, args: MultiValue| thread.resume::<MultiValue>(args))
    })?;
    coroutine.set("wrap", wrapped_wrap)?;

    Ok(())
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
///
/// Table/function locals additionally get an owned [`mlua::Value`] fetched
/// via [`fetch_local_value`], so the Studio debugger's expand-on-demand
/// inspector has something to root — a raw stack index doesn't survive past
/// this hook returning, but a value round-tripped through `exec_raw` does
/// (see that function's doc comment).
fn read_active_locals(lua: &Lua, state: *mut mlua_sys::lua_State) -> Vec<RawLocal> {
    use std::ffi::CStr;
    use std::os::raw::c_int;

    // `exec_raw` invokes this closure via `lua_pcall` (see mlua's
    // `protect_lua_closure`), which pushes its own trampoline C function
    // onto the call stack — so level 0 here is that trampoline, and the
    // frame we actually want (`_update`, where the `EVERY_LINE` hook fired)
    // is level 1.
    const CALLER_FRAME_LEVEL: std::os::raw::c_int = 1;

    let mut locals: Vec<RawLocal> = Vec::new();
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
            let local_index = n;
            n += 1;
            let name = CStr::from_ptr(name_ptr).to_string_lossy().into_owned();
            if name.starts_with('(') {
                mlua_sys::lua_pop(state, 1);
                continue;
            }
            let raw_type = mlua_sys::lua_type(state, -1);
            let value = describe_raw_stack_value(state, -1);
            mlua_sys::lua_pop(state, 1);
            let owned = if matches!(raw_type, mlua_sys::LUA_TTABLE | mlua_sys::LUA_TFUNCTION) {
                fetch_local_value(lua, &ar, local_index)
            } else {
                None
            };
            match locals.iter_mut().find(|(existing, _, _)| *existing == name) {
                Some(existing) => {
                    existing.1 = value;
                    existing.2 = owned;
                }
                None => locals.push((name, value, owned)),
            }
        }
    }
    locals
}

/// Re-fetches local slot `n` at `ar` (already known live — called
/// immediately after the same slot was read by [`read_active_locals`]) as
/// an owned [`mlua::Value`]. `lua_getlocal` can be called more than once
/// for the same slot while the frame is still active, so this is just a
/// second, cheap fetch. Reentrant `exec_raw` call from inside the
/// already-active hook (same pattern already proven safe for the outer
/// `read_active_locals` call — mlua's per-`Lua` lock is a reentrant mutex).
/// The value `exec_raw`'s closure leaves on the stack is converted via
/// `FromLuaMulti`, which for a table/function creates a real
/// registry-backed reference — unlike a raw stack index, that reference
/// stays valid after this hook (and the `_update` frame) unwinds.
unsafe fn fetch_local_value(
    lua: &Lua,
    ar: &mlua_sys::lua_Debug,
    n: std::os::raw::c_int,
) -> Option<mlua::Value> {
    let ar_ptr = ar as *const mlua_sys::lua_Debug;
    unsafe {
        lua.exec_raw::<mlua::Value>((), move |state| {
            mlua_sys::lua_getlocal(state, ar_ptr, n);
        })
        .ok()
    }
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
/// `active_prelude_names` is the cart's *currently selected* prelude module
/// globals (see [`Vm::active_prelude_names`]), not the full static set — a
/// cart that excludes `camera` must not have a cart-defined global also
/// named `Camera` hidden from the inspector.
fn is_script_defined_name(name: &str, active_prelude_names: &[&str]) -> bool {
    !BUILTIN_NAMES.contains(&name)
        && !active_prelude_names.contains(&name)
        && !STDLIB_NAMES.contains(&name)
}

/// Whether a global name is eligible for [`Vm::hot_reload_lua_source`]'s
/// upvalue-join snapshot: same script-defined-vs-API-surface line as
/// [`is_script_defined_name`], except `_init`/`_update`/`_draw` are kept in —
/// they aren't "state" for the debugger's purposes, but they're exactly the
/// closures whose captured locals need joining across a reload.
fn is_reload_join_candidate(name: &str, active_prelude_names: &[&str]) -> bool {
    if BUILTIN_NAMES.contains(&name) || active_prelude_names.contains(&name) {
        return false;
    }
    matches!(name, "_init" | "_update" | "_draw") || !STDLIB_NAMES.contains(&name)
}

/// Rebinds `new_fn`'s non-function upvalues onto `old_fn`'s cells wherever the
/// names match, via raw `lua_upvaluejoin`, so chunk-scope `local` state survives
/// [`Vm::hot_reload_lua_source`]. Function-valued upvalues are not joined (the
/// edited body must win); instead their own upvalues are joined recursively, so
/// state stays one cell between `_update` and the helpers it calls.
/// `mlua` has no safe upvalue-join API, hence the raw `mlua::ffi` calls.
fn join_matching_upvalues(
    lua: &Lua,
    old_fn: &mlua::Function,
    new_fn: &mlua::Function,
) -> mlua::Result<()> {
    let mut seen = std::collections::HashSet::new();
    join_upvalues_rec(lua, old_fn, new_fn, &mut seen)
}

fn join_upvalues_rec(
    lua: &Lua,
    old_fn: &mlua::Function,
    new_fn: &mlua::Function,
    seen: &mut std::collections::HashSet<*const std::ffi::c_void>,
) -> mlua::Result<()> {
    if !seen.insert(new_fn.to_pointer()) {
        return Ok(());
    }
    let old_ups = list_function_upvalues_indexed(lua, old_fn);
    let new_ups = list_function_upvalues_indexed(lua, new_fn);
    for (new_index, name, new_value) in &new_ups {
        let Some((old_index, _, old_value)) = old_ups.iter().find(|(_, n, _)| n == name) else {
            continue;
        };
        match (new_value, old_value) {
            (mlua::Value::Function(new_child), mlua::Value::Function(old_child)) => {
                join_upvalues_rec(lua, old_child, new_child, seen)?;
            }
            (mlua::Value::Function(_), _) | (_, mlua::Value::Function(_)) => {}
            _ => {
                let (new_index, old_index) = (*new_index, *old_index);
                unsafe {
                    lua.exec_raw::<()>((old_fn.clone(), new_fn.clone()), move |state| {
                        mlua::ffi::lua_upvaluejoin(state, 2, new_index, 1, old_index);
                    })?;
                }
            }
        }
    }
    Ok(())
}

/// Lists a function's upvalues by name with their current values, for the
/// Studio debugger's expand-on-demand inspector — same raw `lua_getupvalue`
/// walk [`join_matching_upvalues`] already uses to enumerate upvalues by
/// name, but here to read rather than rebind them. Two passes: first
/// collects `(index, name)` pairs cheaply (popping each value immediately),
/// then re-fetches each by index via its own `exec_raw` call so the value
/// left on the stack converts into an owned, registry-backed
/// [`mlua::Value`] — the same technique [`fetch_local_value`] uses for
/// locals. Unknown-index fetches are skipped rather than erroring; this is
/// a best-effort debugger view, not a correctness-critical path.
fn list_function_upvalues(lua: &Lua, function: &mlua::Function) -> Vec<(String, mlua::Value)> {
    list_function_upvalues_indexed(lua, function)
        .into_iter()
        .map(|(_, name, value)| (name, value))
        .collect()
}

fn list_function_upvalues_indexed(
    lua: &Lua,
    function: &mlua::Function,
) -> Vec<(std::os::raw::c_int, String, mlua::Value)> {
    use std::ffi::CStr;
    use std::os::raw::c_int;

    let mut names: Vec<(c_int, String)> = Vec::new();
    let _: mlua::Result<()> = unsafe {
        lua.exec_raw::<()>((function.clone(),), |state| {
            let mut i: c_int = 1;
            loop {
                let name = mlua::ffi::lua_getupvalue(state, 1, i);
                if name.is_null() {
                    break;
                }
                mlua::ffi::lua_pop(state, 1);
                names.push((i, CStr::from_ptr(name).to_string_lossy().into_owned()));
                i += 1;
            }
        })
    };

    names
        .into_iter()
        .filter_map(|(i, name)| {
            let value = unsafe {
                lua.exec_raw::<mlua::Value>((function.clone(),), move |state| {
                    mlua::ffi::lua_getupvalue(state, 1, i);
                    // Leave only the fetched value on the stack — otherwise
                    // `exec_raw`'s single-value `FromLuaMulti` picks up the
                    // bottommost of the two (the function argument, at
                    // index 1) instead of the value `lua_getupvalue` just
                    // pushed on top.
                    mlua::ffi::lua_remove(state, 1);
                })
            };
            value.ok().map(|value| (i, name, value))
        })
        .collect()
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

/// Formats a table key for [`Vm::expand_debug_node`]'s child rows — a
/// bare field name for string keys that read like a Lua identifier (so
/// `t.x` shows as `x`, matching dotted-watch syntax), `[i]` for integer
/// keys (array-like tables), and [`describe_lua_value`]'s generic
/// representation for everything else.
fn describe_table_key(key: &mlua::Value) -> String {
    match key {
        mlua::Value::String(s) => {
            let text = s.to_string_lossy();
            if is_lua_identifier(&text) {
                text
            } else {
                describe_lua_value(key)
            }
        }
        mlua::Value::Integer(i) => format!("[{i}]"),
        other => describe_lua_value(other),
    }
}

fn is_lua_identifier(text: &str) -> bool {
    let mut chars = text.chars();
    match chars.next() {
        Some(c) if c == '_' || c.is_ascii_alphabetic() => {}
        _ => return false,
    }
    chars.all(|c| c == '_' || c.is_ascii_alphanumeric())
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

/// Intersects the rectangle `[x, x+w) x [y, y+h)` with the screen
/// `[0, width) x [0, height)`, returning the clipped range or `None` when
/// they don't overlap at all. Computed once before rasterizing — a cart
/// passing a huge `w`/`h` (or coordinates far outside the screen) must not
/// turn one Lua call into a native loop proportional to that size instead of
/// the visible area: the instruction-count watchdog only sees Lua bytecode
/// between calls, not work done inside a single native one, so an unclipped
/// per-pixel loop here would hang the process with no way for the watchdog
/// to ever step in.
fn clip_rect(
    x: i64,
    y: i64,
    w: i64,
    h: i64,
    width: u32,
    height: u32,
) -> Option<(i64, i64, i64, i64)> {
    let x0 = x.max(0);
    let y0 = y.max(0);
    let x1 = (x + w).min(width as i64);
    let y1 = (y + h).min(height as i64);
    (x1 > x0 && y1 > y0).then_some((x0, y0, x1, y1))
}

/// Radius past which a filled/outlined circle can no longer add any visible
/// pixel regardless of where it's centered — the diagonal of the screen is
/// the largest radius a circle could need to fully cover it. Clamping to
/// this before rasterizing bounds `fill_circle`'s `O(r^2)` loop (and
/// `draw_circle`'s `O(r)` one) the same way [`clip_rect`] bounds the
/// rectangle builtins — see its doc comment for why that has to happen
/// before the loop rather than per-pixel inside it.
fn max_render_radius(width: u32, height: u32) -> i64 {
    let (w, h) = (width as i64, height as i64);
    // Integer-safe ceiling of `sqrt(w*w + h*h)`: `isqrt` floors, so bump by
    // one to stay past the true diagonal.
    (w * w + h * h).isqrt() + 1
}

/// Steps a Bresenham line from `(x0,y0)` to `(x1,y1)` always takes exactly —
/// clamped before the loop starts for the same reason as [`clip_rect`]: a
/// cart-supplied endpoint far outside the screen must not turn one Lua call
/// into a native loop proportional to that distance. Every point beyond this
/// many steps from either endpoint of a legitimate on-screen line is already
/// off-screen and invisible (`plot` drops it), so this only changes behavior
/// for lines no cart has a real reason to draw.
const MAX_LINE_STEPS: i64 = 8192;

fn draw_line(layer: &mut ScreenLayer, x0: i64, y0: i64, x1: i64, y1: i64, color: Color) {
    let (mut x, mut y) = (x0, y0);
    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    let steps = dx.max(-dy).saturating_add(1).min(MAX_LINE_STEPS);
    for _ in 0..steps {
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

const BUILTIN_IMPL_KEY: &str = "caiven_builtin_impl";

/// The hidden table holding this frame's scoped builtin implementations.
fn builtin_impls(lua: &Lua) -> mlua::Result<Table> {
    lua.named_registry_value(BUILTIN_IMPL_KEY)
}

/// Installs the permanent builtin globals once per Lua state. Each is a
/// stable trampoline into the per-frame scoped implementation, so a cart's
/// aliases (`local d = draw_text`) and wrappers survive across frames.
fn install_builtin_trampolines(lua: &Lua, sprite_size: u32) -> mlua::Result<()> {
    let impls = lua.create_table()?;
    lua.set_named_registry_value(BUILTIN_IMPL_KEY, impls.clone())?;
    let globals = lua.globals();
    globals.set("SPRITE_SIZE", sprite_size)?;
    for &name in BUILTIN_NAMES.iter().filter(|&&n| n != "SPRITE_SIZE") {
        let impls = impls.clone();
        let trampoline = lua.create_function(move |_, args: MultiValue| {
            impls.get::<mlua::Function>(name)?.call::<MultiValue>(args)
        })?;
        globals.set(name, trampoline)?;
    }
    Ok(())
}

/// Fills `impls` with the full builtin API surface, scoped to this call's
/// borrowed VM state. Shared by [`Vm::load_lua_source`], hot reload and
/// [`Vm::run_frame_lua`]; the permanent globals dispatch into it.
#[allow(clippy::too_many_arguments)]
fn register_builtins<'scope, 'env>(
    scope: &'scope Scope<'scope, 'env>,
    impls: &Table,
    world: &'env RefCell<&'env mut ScreenLayer>,
    ui: &'env RefCell<&'env mut ScreenLayer>,
    memory: &'env RefCell<&'env mut Memory>,
    palette: &'env RefCell<&'env mut Palette>,
    camera: &'env RefCell<&'env mut Camera>,
    music_player: &'env RefCell<&'env mut MusicPlayer>,
    sfx_pool: &'env RefCell<&'env mut [PooledSfx; SFX_VOICE_COUNT]>,
    next_sfx_age: &'env RefCell<&'env mut u64>,
    sound: Arc<Mutex<Sound>>,
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
    impls.set(
        "clear_screen",
        scope.create_function_mut(|_, ()| {
            world.borrow_mut().clear();
            ui.borrow_mut().clear();
            Ok(())
        })?,
    )?;

    impls.set(
        "set_pixel",
        scope.create_function_mut(|_, (x, y, color_index): (i64, i64, u8)| {
            let color = palette.borrow().get_color(color_index as usize);
            plot(&mut world.borrow_mut(), x, y, color);
            Ok(())
        })?,
    )?;

    #[allow(clippy::type_complexity)]
    let sprite_fn = scope.create_function_mut(
        move |_,
              (sprite_id, x, y, flip_x, flip_y, rotate, sw, sh): (
            u8,
            i64,
            i64,
            Option<bool>,
            Option<bool>,
            Option<i64>,
            Option<i64>,
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
            let sw = sw.unwrap_or(1);
            let sh = sh.unwrap_or(1);
            if sw < 1 || sh < 1 {
                return Err(mlua::Error::RuntimeError(format!(
                    "sprite: w and h must be at least 1 (got {sw}, {sh})"
                )));
            }
            let cols = SPRITE_SHEET_COLS as i64;
            let rows = (SPRITE_COUNT / SPRITE_SHEET_COLS) as i64;
            let sheet_col = sprite_id as i64 % cols;
            let sheet_row = sprite_id as i64 / cols;
            if sheet_col + sw > cols || sheet_row + sh > rows {
                return Err(mlua::Error::RuntimeError(format!(
                    "sprite: {sw}x{sh} sprites starting at {sprite_id} runs past the sheet edge"
                )));
            }

            let (cam_x, cam_y) = cam_offset(camera);
            let ss = sprite_size as i64;
            // Source-space block spanning every covered sprite before rotation.
            let bw = ss * sw;
            let bh = ss * sh;
            let mem = memory.borrow();
            let mut w = world.borrow_mut();
            for by in 0..bh {
                for bx in 0..bw {
                    let tile_col = bx / ss;
                    let tile_row = by / ss;
                    // sprite_id already encodes sheet_row/sheet_col, so offsetting by the
                    // tile's row/col within the block lands on the right sheet slot.
                    let tile_id = sprite_id as usize
                        + tile_row as usize * SPRITE_SHEET_COLS
                        + tile_col as usize;
                    let base = SPRITE_SHEET_RAM_BASE + tile_id * SPRITE_BYTES;
                    let sx = bx % ss;
                    let sy = by % ss;
                    let Ok(pixel) = mem.read(base + (sy * ss + sx) as usize) else {
                        continue;
                    };
                    if pixel == 0 {
                        continue;
                    }
                    // Rotate (clockwise) about the whole block, then flip.
                    let (mut rx, mut ry) = match rotate_steps {
                        0 => (bx, by),
                        1 => (bh - 1 - by, bx),
                        2 => (bw - 1 - bx, bh - 1 - by),
                        _ => (by, bw - 1 - bx),
                    };
                    let (out_w, out_h) = if rotate_steps % 2 == 0 {
                        (bw, bh)
                    } else {
                        (bh, bw)
                    };
                    if flip_x {
                        rx = out_w - 1 - rx;
                    }
                    if flip_y {
                        ry = out_h - 1 - ry;
                    }
                    let color = palette.borrow().get_color(pixel as usize);
                    plot(&mut w, x + rx - cam_x, y + ry - cam_y, color);
                }
            }
            Ok(())
        },
    )?;
    impls.set("sprite", sprite_fn)?;

    impls.set(
        "button_down",
        scope.create_function(|_, button_index: i64| {
            Ok(u8::try_from(button_index)
                .ok()
                .and_then(Button::from_u8)
                .map(|b| input.is_pressed(b))
                .unwrap_or(false))
        })?,
    )?;

    impls.set(
        "button_pressed",
        scope.create_function(|_, button_index: i64| {
            Ok(u8::try_from(button_index)
                .ok()
                .and_then(Button::from_u8)
                .map(|b| input.just_pressed(b))
                .unwrap_or(false))
        })?,
    )?;

    impls.set(
        "button_released",
        scope.create_function(|_, button_index: i64| {
            Ok(u8::try_from(button_index)
                .ok()
                .and_then(Button::from_u8)
                .map(|b| input.just_released(b))
                .unwrap_or(false))
        })?,
    )?;

    impls.set(
        "draw_text",
        scope.create_function_mut(|_, (text, x, y, color_index): (String, i64, i64, u8)| {
            let color = palette.borrow().get_color(color_index as usize);
            draw_text_at(font, &mut ui.borrow_mut(), &text, x, y, color);
            Ok(())
        })?,
    )?;

    impls.set(
        "draw_number",
        scope.create_function_mut(|_, (value, x, y, color_index): (i64, i64, i64, u8)| {
            let color = palette.borrow().get_color(color_index as usize);
            draw_text_at(font, &mut ui.borrow_mut(), &value.to_string(), x, y, color);
            Ok(())
        })?,
    )?;

    impls.set(
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

    impls.set(
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

    impls.set(
        "draw_rect",
        scope.create_function_mut(
            move |_, (x, y, w, h, color_index): (i64, i64, i64, i64, u8)| {
                if w <= 0 || h <= 0 {
                    return Ok(());
                }
                let color = palette.borrow().get_color(color_index as usize);
                let (cam_x, cam_y) = cam_offset(camera);
                let (x, y) = (x - cam_x, y - cam_y);
                let Some((cx0, cy0, cx1, cy1)) = clip_rect(x, y, w, h, width, height) else {
                    return Ok(());
                };
                let mut layer = world.borrow_mut();
                for ix in cx0..cx1 {
                    plot(&mut layer, ix, y, color);
                    plot(&mut layer, ix, y + h - 1, color);
                }
                for iy in cy0..cy1 {
                    plot(&mut layer, x, iy, color);
                    plot(&mut layer, x + w - 1, iy, color);
                }
                Ok(())
            },
        )?,
    )?;

    impls.set(
        "fill_rect",
        scope.create_function_mut(
            move |_, (x, y, w, h, color_index): (i64, i64, i64, i64, u8)| {
                if w <= 0 || h <= 0 {
                    return Ok(());
                }
                let color = palette.borrow().get_color(color_index as usize);
                let (cam_x, cam_y) = cam_offset(camera);
                let (x, y) = (x - cam_x, y - cam_y);
                let Some((x0, y0, x1, y1)) = clip_rect(x, y, w, h, width, height) else {
                    return Ok(());
                };
                let mut layer = world.borrow_mut();
                for iy in y0..y1 {
                    for ix in x0..x1 {
                        plot(&mut layer, ix, iy, color);
                    }
                }
                Ok(())
            },
        )?,
    )?;

    impls.set(
        "draw_circle",
        scope.create_function_mut(move |_, (cx, cy, r, color_index): (i64, i64, i64, u8)| {
            if r < 0 {
                return Ok(());
            }
            let r = r.min(max_render_radius(width, height));
            let color = palette.borrow().get_color(color_index as usize);
            let (cam_x, cam_y) = cam_offset(camera);
            circle_points(cx - cam_x, cy - cam_y, r, |x, y| {
                plot(&mut world.borrow_mut(), x, y, color)
            });
            Ok(())
        })?,
    )?;

    impls.set(
        "fill_circle",
        scope.create_function_mut(move |_, (cx, cy, r, color_index): (i64, i64, i64, u8)| {
            if r < 0 {
                return Ok(());
            }
            let r = r.min(max_render_radius(width, height));
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

    impls.set(
        "set_camera",
        scope.create_function_mut(|_, (x, y): (i64, i64)| {
            // Stored as i32 bit patterns so negative scroll survives the u32 slot.
            let to_slot = |v: i64| v.clamp(i32::MIN as i64, i32::MAX as i64) as i32 as u32;
            camera.borrow_mut().set_position(to_slot(x), to_slot(y));
            Ok(())
        })?,
    )?;

    impls.set(
        "set_palette_color",
        scope.create_function_mut(|_, (index, r, g, b): (usize, u8, u8, u8)| {
            palette
                .borrow_mut()
                .set_color(index, Color::new_rgb(r, g, b));
            if index < 16 {
                let mut mem = memory.borrow_mut();
                for (offset, byte) in [r, g, b].into_iter().enumerate() {
                    let _ = mem.write(PALETTE_RAM_BASE + index * 3 + offset, byte);
                }
            }
            Ok(())
        })?,
    )?;

    impls.set(
        "draw_map",
        scope.create_function_mut(
            move |_, (cx, cy, sx, sy, w, h): (i64, i64, i64, i64, i64, i64)| {
                // A cart-supplied `w`/`h` far larger than the map must not
                // turn this into a native loop proportional to that size
                // instead of the map's actual 128x128 tiles — same hang risk
                // `clip_rect` guards against for the pixel-rect builtins.
                let Some((mx0, my0, mx1, my1)) =
                    clip_rect(cx, cy, w, h, MAP_W as u32, MAP_H as u32)
                else {
                    return Ok(());
                };
                let (cam_x, cam_y) = cam_offset(camera);
                let ss = sprite_size as i64;
                let mem = memory.borrow();
                let pal = palette.borrow();
                let mut layer = world.borrow_mut();
                for map_y in my0..my1 {
                    let ty = map_y - cy;
                    for map_x in mx0..mx1 {
                        let tx = map_x - cx;
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

    impls.set(
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

    impls.set(
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

    impls.set(
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

    impls.set(
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

    impls.set(
        "collision_type_id",
        scope.create_function(move |_, name: String| {
            Ok(caiven_core::collision_type_by_name(collision_types, &name)
                .map(|t| t.id)
                .unwrap_or(0))
        })?,
    )?;

    impls.set(
        "collision_type_name",
        scope.create_function(move |_, id: u8| {
            Ok(caiven_core::collision_type_by_id(collision_types, id)
                .map(|t| t.name.clone())
                .unwrap_or_default())
        })?,
    )?;

    impls.set(
        "collision_is_solid",
        scope
            .create_function(move |_, id: u8| Ok(caiven_core::is_solid_id(collision_types, id)))?,
    )?;

    impls.set(
        "collision_is_one_way",
        scope.create_function(move |_, id: u8| {
            Ok(caiven_core::collision_type_by_id(collision_types, id)
                .is_some_and(|t| t.flags.is_one_way()))
        })?,
    )?;

    impls.set(
        "collision_is_slope_left",
        scope.create_function(move |_, id: u8| {
            Ok(caiven_core::collision_type_by_id(collision_types, id)
                .is_some_and(|t| t.flags.is_slope_left()))
        })?,
    )?;

    impls.set(
        "collision_is_slope_right",
        scope.create_function(move |_, id: u8| {
            Ok(caiven_core::collision_type_by_id(collision_types, id)
                .is_some_and(|t| t.flags.is_slope_right()))
        })?,
    )?;

    impls.set(
        "load_sprite_bank",
        scope.create_function_mut(|_, name: String| {
            Ok(asset_banks.borrow_mut().select_with_companion(
                AssetBankKind::Sprites,
                &name,
                &mut memory.borrow_mut(),
            ))
        })?,
    )?;

    impls.set(
        "load_map_bank",
        scope.create_function_mut(|_, name: String| {
            Ok(asset_banks.borrow_mut().select_with_companion(
                AssetBankKind::Map,
                &name,
                &mut memory.borrow_mut(),
            ))
        })?,
    )?;

    impls.set(
        "load_palette_bank",
        scope.create_function_mut(|_, name: String| {
            let selected = asset_banks.borrow_mut().select_with_companion(
                AssetBankKind::Palette,
                &name,
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

    impls.set(
        "load_sfx_bank",
        scope.create_function_mut(|_, name: String| {
            Ok(asset_banks.borrow_mut().select_with_companion(
                AssetBankKind::Sfx,
                &name,
                &mut memory.borrow_mut(),
            ))
        })?,
    )?;

    impls.set(
        "load_music_bank",
        scope.create_function_mut(|_, name: String| {
            Ok(asset_banks.borrow_mut().select_with_companion(
                AssetBankKind::Music,
                &name,
                &mut memory.borrow_mut(),
            ))
        })?,
    )?;

    impls.set(
        "play_sfx",
        scope.create_function_mut(move |_, (id, opts): (u8, Option<mlua::Table>)| {
            let volume = match &opts {
                Some(t) => t.get::<Option<f64>>("volume")?.unwrap_or(1.0) as f32,
                None => 1.0,
            };
            if id as usize >= SFX_COUNT {
                return Err(mlua::Error::RuntimeError(format!(
                    "play_sfx: id must be 0-{} (got {id})",
                    SFX_COUNT - 1
                )));
            }
            let handle = allocate_sfx_voice(
                &mut sfx_pool.borrow_mut(),
                &mut next_sfx_age.borrow_mut(),
                id,
                volume,
            );
            Ok(handle)
        })?,
    )?;

    let sound_for_stop_sfx = sound.clone();
    impls.set(
        "stop_sfx",
        scope.create_function_mut(move |_, handle: u32| {
            release_sfx_voice(&mut sfx_pool.borrow_mut(), &sound_for_stop_sfx, handle);
            Ok(())
        })?,
    )?;

    impls.set(
        "is_sfx_playing",
        scope.create_function(move |_, handle: u32| {
            let (slot, epoch) = unpack_sfx_handle(handle);
            let slot = slot as usize;
            let pool = sfx_pool.borrow();
            Ok(slot < pool.len() && pool[slot].epoch == epoch && pool[slot].player.active)
        })?,
    )?;

    impls.set(
        "play_music",
        scope.create_function_mut(|_, id: u8| {
            music_player.borrow_mut().start(id);
            Ok(())
        })?,
    )?;

    impls.set(
        "play_music_song",
        scope.create_function_mut(|_, start_step: Option<i64>| {
            let step = start_step
                .unwrap_or(0)
                .clamp(0, MUSIC_ORDER_STEPS as i64 - 1) as u8;
            let resolved = resolve_song_step(&memory.borrow(), step);
            let mut player = music_player.borrow_mut();
            match resolved {
                Some((pattern, resolved_step)) => {
                    player.pattern_id = pattern;
                    player.song_step = resolved_step;
                    player.row = 0;
                    player.tick_count = 0;
                    player.active = true;
                    player.song_active = true;
                }
                None => {
                    player.active = false;
                    player.song_active = false;
                }
            }
            Ok(())
        })?,
    )?;

    let sound_for_stop_music = sound.clone();
    impls.set(
        "stop_music",
        scope.create_function_mut(move |_, ()| {
            music_player.borrow_mut().stop();
            silence_music_voices(&sound_for_stop_music);
            Ok(())
        })?,
    )?;

    impls.set(
        "is_music_playing",
        scope.create_function(move |_, ()| Ok(music_player.borrow().active))?,
    )?;

    let sound_for_master_volume = sound.clone();
    impls.set(
        "set_master_volume",
        scope.create_function_mut(move |_, v: f64| {
            if let Ok(mut s) = sound_for_master_volume.try_lock() {
                s.master_volume = (v as f32).clamp(0.0, 1.0);
            }
            Ok(())
        })?,
    )?;

    let sound_for_music_volume = sound.clone();
    impls.set(
        "set_music_volume",
        scope.create_function_mut(move |_, v: f64| {
            if let Ok(mut s) = sound_for_music_volume.try_lock() {
                s.music_volume = (v as f32).clamp(0.0, 1.0);
            }
            Ok(())
        })?,
    )?;

    impls.set(
        "set_sfx_volume",
        scope.create_function_mut(move |_, v: f64| {
            if let Ok(mut s) = sound.try_lock() {
                s.sfx_volume = (v as f32).clamp(0.0, 1.0);
            }
            Ok(())
        })?,
    )?;

    impls.set(
        "real_time",
        scope.create_function(|_, ()| {
            let mem = memory.borrow();
            let hour = mem.read(RTC_RAM_BASE).unwrap_or(0);
            let minute = mem.read(RTC_RAM_BASE + 1).unwrap_or(0);
            let second = mem.read(RTC_RAM_BASE + 2).unwrap_or(0);
            Ok((hour, minute, second))
        })?,
    )?;

    impls.set(
        "frame_count",
        scope.create_function(move |_, ()| Ok(frame_count))?,
    )?;

    impls.set(
        "time",
        scope.create_function(move |_, ()| Ok(frame_count as f64 / TARGET_FPS))?,
    )?;

    impls.set(
        "save_data",
        scope.create_function(move |lua, table: mlua::Table| {
            let value: serde_json::Value = lua.from_value(mlua::Value::Table(table))?;
            save_data
                .borrow_mut()
                .set_blob(value)
                .map_err(|e| mlua::Error::RuntimeError(e.to_string()))
        })?,
    )?;

    impls.set(
        "load_data",
        scope.create_function(move |lua, ()| lua.to_value(save_data.borrow().blob()))?,
    )?;

    Ok(())
}

impl Vm {
    /// Sets the cart's opt-in gameplay-stdlib module selection (`[stdlib]
    /// modules` in `caiven.toml`), validated against [`prelude_module_catalog`].
    /// Errors by name on any unknown module rather than silently dropping it
    /// — a typo'd module name should fail cart load, not quietly leave
    /// globals missing. Takes effect on the next [`Vm::load_lua_source`] or
    /// [`Vm::hot_reload_lua_source`]; the resolved set is stored on the `Vm`
    /// so hot-reload doesn't need it re-supplied.
    pub fn set_prelude_modules(&mut self, modules: &[&str]) -> Result<(), String> {
        let mut resolved = Vec::with_capacity(modules.len());
        for &name in modules {
            let module = PRELUDE_MODULES
                .iter()
                .find(|candidate| candidate.name == name)
                .ok_or_else(|| format!("unknown stdlib module: \"{name}\""))?;
            resolved.push(module.name);
        }
        self.active_prelude_modules = resolved;
        Ok(())
    }

    /// The cart's currently enabled `[stdlib]` module names, as last set by
    /// [`Vm::set_prelude_modules`].
    pub fn active_prelude_modules(&self) -> &[&'static str] {
        &self.active_prelude_modules
    }

    /// The cart's currently selected [`PRELUDE_MODULES`] entries, in table
    /// order (not manifest order, so load order is deterministic regardless
    /// of how a cart lists them).
    fn selected_prelude_modules(&self) -> impl Iterator<Item = &'static PreludeModule> + '_ {
        PRELUDE_MODULES
            .iter()
            .filter(move |module| self.active_prelude_modules.contains(&module.name))
    }

    /// Union of [`CORE_PRELUDE_NAMES`] and the currently selected modules'
    /// globals — the debugger/hot-reload exclusion set for *this* cart, as
    /// opposed to the full static list. See [`is_script_defined_name`].
    /// `Particles` is a table, not a function; it's still excluded wholesale
    /// rather than snapshotted, since its `list` field churns every frame
    /// and isn't useful in a "what does the script think" debugger view.
    fn active_prelude_names(&self) -> Vec<&'static str> {
        let mut names: Vec<&'static str> = CORE_PRELUDE_NAMES.to_vec();
        for module in self.selected_prelude_modules() {
            names.extend_from_slice(module.globals);
        }
        names
    }

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
        // Bounds a hostile cart's own Lua-heap growth (runaway table/string
        // allocation) independently of the instruction-count watchdog, which
        // only catches CPU time, not memory. Generous relative to the 128
        // KiB cart cap so legitimate carts — including ones bundling several
        // prelude modules — never come close.
        lua.set_memory_limit(LUA_MEMORY_LIMIT_BYTES)?;
        install_builtin_trampolines(&lua, self.config.sprite_size)?;
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

        // A cart's top-level code and `_init()` currently ran with no
        // instruction budget at all — only per-frame `_update()`/`_draw()`
        // did. An infinite loop here hung load instead of the frame loop.
        let coroutines: Rc<RefCell<Vec<mlua::Thread>>> = Rc::new(RefCell::new(Vec::new()));
        let active_budget: Rc<RefCell<Option<BudgetCells>>> = Rc::new(RefCell::new(None));
        install_coroutine_budget_guard(
            &lua,
            &lua.globals(),
            coroutines.clone(),
            active_budget.clone(),
        )?;

        let selected_modules: Vec<&'static PreludeModule> =
            self.selected_prelude_modules().collect();
        let world = RefCell::new(&mut self.world);
        let ui = RefCell::new(&mut self.ui);
        let memory = RefCell::new(&mut self.memory);
        let palette = RefCell::new(&mut self.palette);
        let camera = RefCell::new(&mut self.camera);
        let music_player = RefCell::new(&mut self.music_player);
        let sfx_pool = RefCell::new(&mut self.sfx_pool);
        let next_sfx_age = RefCell::new(&mut self.next_sfx_age);
        let sound = self.sound.clone();
        let asset_banks = RefCell::new(&mut self.asset_banks);
        let save_data = RefCell::new(&mut self.save_data);
        let sprite_size = self.config.sprite_size;
        let width = self.config.width;
        let height = self.config.height;

        let instructions: Rc<RefCell<u32>> = Rc::new(RefCell::new(0));
        let budget_hit: Rc<RefCell<Option<LuaBreakpoint>>> = Rc::new(RefCell::new(None));
        *active_budget.borrow_mut() = Some((
            instructions.clone(),
            budget_hit.clone(),
            INIT_INSTRUCTION_BUDGET,
        ));
        lua.set_hook(
            HookTriggers::new().every_nth_instruction(INSTRUCTION_HOOK_STRIDE),
            budget_hook(
                instructions.clone(),
                budget_hit.clone(),
                INIT_INSTRUCTION_BUDGET,
            ),
        );
        rearm_coroutines(
            &coroutines,
            &instructions,
            &budget_hit,
            INIT_INSTRUCTION_BUDGET,
        );

        let result: mlua::Result<()> = lua.scope(|scope| {
            let globals = lua.globals();
            register_builtins(
                scope,
                &builtin_impls(&lua)?,
                &world,
                &ui,
                &memory,
                &palette,
                &camera,
                &music_player,
                &sfx_pool,
                &next_sfx_age,
                sound.clone(),
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

            lua.load(PRELUDE_CORE).set_name("=prelude:core").exec()?;
            for module in &selected_modules {
                lua.load(module.source)
                    .set_name(format!("=prelude:{}", module.name))
                    .exec()?;
            }
            lua.load(src).set_name(CHUNK_SOURCE_NAME).exec()?;
            if let Ok(init) = globals.get::<mlua::Function>("_init") {
                init.call::<()>(())?;
            }
            Ok(())
        });
        lua.remove_hook();
        *active_budget.borrow_mut() = None;
        result?;

        self.script = Some(LuaScript {
            lua,
            output,
            coroutines,
            active_budget,
        });
        self.fault = None;
        self.fault_message = None;
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

    /// One Lua-driven frame: refills the builtin implementations against this
    /// frame's borrowed VM state via `Lua::scope` (the permanent globals are
    /// trampolines into them), then calls the script's `_update()`/`_draw()`.
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
        let music_player = RefCell::new(&mut self.music_player);
        let sfx_pool = RefCell::new(&mut self.sfx_pool);
        let next_sfx_age = RefCell::new(&mut self.next_sfx_age);
        let sound = self.sound.clone();
        let asset_banks = RefCell::new(&mut self.asset_banks);
        let save_data = RefCell::new(&mut self.save_data);
        let sprite_size = self.config.sprite_size;
        let width = self.config.width;
        let height = self.config.height;

        let budget_hit: Rc<RefCell<Option<LuaBreakpoint>>> = Rc::new(RefCell::new(None));
        let instructions: Rc<RefCell<u32>> = Rc::new(RefCell::new(0));
        *script.active_budget.borrow_mut() = Some((
            instructions.clone(),
            budget_hit.clone(),
            FRAME_INSTRUCTION_BUDGET,
        ));
        lua.set_hook(
            HookTriggers::new().every_nth_instruction(INSTRUCTION_HOOK_STRIDE),
            budget_hook(
                instructions.clone(),
                budget_hit.clone(),
                FRAME_INSTRUCTION_BUDGET,
            ),
        );
        rearm_coroutines(
            &script.coroutines,
            &instructions,
            &budget_hit,
            FRAME_INSTRUCTION_BUDGET,
        );

        let result: mlua::Result<()> = lua.scope(|scope| {
            let globals = lua.globals();
            register_builtins(
                scope,
                &builtin_impls(lua)?,
                &world,
                &ui,
                &memory,
                &palette,
                &camera,
                &music_player,
                &sfx_pool,
                &next_sfx_age,
                sound.clone(),
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
        *script.active_budget.borrow_mut() = None;

        // Checked ahead of `result`, not just on `Err`: a cart can wrap its
        // own runaway loop in `pcall`, which swallows the hook's error
        // before it ever reaches `update.call`, so `result` comes back `Ok`
        // even though the watchdog had to step in. That must still surface
        // as a fault instead of silently reporting a normal frame.
        if let Some(location) = budget_hit.borrow().clone() {
            log::error!(
                "Lua execution budget exceeded at {}:{}",
                location.source,
                location.line
            );
            self.set_fault(VmFault::ExecutionBudgetExceeded);
        } else if let Err(e) = result {
            log::error!("Lua runtime error: {e}");
            let (_, message) = describe_lua_error_location(&e);
            self.set_fault_with_message(VmFault::LuaError, Some(message));
        }
    }

    /// Like `Vm::run_frame_lua`, but also installs a line hook that aborts
    /// `_update()` as soon as it reaches a breakpointed source line (the
    /// execution-budget watchdog runs here too, same as in
    /// `Vm::run_frame_lua`). The aborted call unwinds Lua's stack (mlua's
    /// hooks can't yield outside a
    /// coroutine while borrowing per-frame VM state via `Lua::scope`, so a
    /// suspend-and-resume mid-statement debugger isn't possible here) —
    /// globals and RAM at the moment of the stop are readable via
    /// [`Vm::lua_globals`] and `peek_memory`; locals are readable too, via
    /// [`Vm::lua_debug_locals`] — mlua's safe hook API has no `lua_getlocal`
    /// binding, so that path drops to raw `mlua_sys` FFI (see
    /// `read_active_locals`). Resuming re-runs `_update()` from the top,
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
        let music_player = RefCell::new(&mut self.music_player);
        let sfx_pool = RefCell::new(&mut self.sfx_pool);
        let next_sfx_age = RefCell::new(&mut self.next_sfx_age);
        let sound = self.sound.clone();
        let asset_banks = RefCell::new(&mut self.asset_banks);
        let save_data = RefCell::new(&mut self.save_data);
        let sprite_size = self.config.sprite_size;
        let width = self.config.width;
        let height = self.config.height;

        let hit: Rc<RefCell<Option<LuaBreakpoint>>> = Rc::new(RefCell::new(None));
        let stack: Rc<RefCell<Vec<(String, String)>>> = Rc::new(RefCell::new(Vec::new()));
        let locals: Rc<RefCell<Vec<RawLocal>>> = Rc::new(RefCell::new(Vec::new()));
        let budget_hit: Rc<RefCell<Option<LuaBreakpoint>>> = Rc::new(RefCell::new(None));
        let instructions: Rc<RefCell<u32>> = Rc::new(RefCell::new(0));
        {
            let hit_hook = hit.clone();
            let stack_hook = stack.clone();
            let locals_hook = locals.clone();
            let budget_sink = budget_hit.clone();
            let instructions_hook = instructions.clone();
            let bps: Vec<LuaBreakpoint> = breakpoints.to_vec();
            // The execution-budget count trigger always runs; EVERY_LINE
            // (real per-instruction overhead) is only added when there's
            // actually something to break on.
            let mut triggers = HookTriggers::new().every_nth_instruction(INSTRUCTION_HOOK_STRIDE);
            if !bps.is_empty() {
                triggers = triggers.every_line();
            }
            lua.set_hook(triggers, move |lua, debug| {
                if debug.event() == mlua::DebugEvent::Count {
                    let mut count = instructions_hook.borrow_mut();
                    *count += INSTRUCTION_HOOK_STRIDE;
                    if *count < FRAME_INSTRUCTION_BUDGET {
                        return Ok(VmState::Continue);
                    }
                    let line = debug.curr_line();
                    *budget_sink.borrow_mut() = (line > 0).then(|| LuaBreakpoint {
                        source: hook_debug_source(&debug),
                        line: line as usize,
                    });
                    return Err(mlua::Error::runtime(EXECUTION_BUDGET_MESSAGE));
                }

                let line = debug.curr_line();
                let source = hook_debug_source(&debug);
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
                        lua.exec_raw((), |state| read_locals = read_active_locals(lua, state))
                    };
                    *locals_hook.borrow_mut() = read_locals;
                    return Err(mlua::Error::runtime("breakpoint"));
                }
                Ok(VmState::Continue)
            });
        }
        // Breakpoint checking only applies to the main thread — a coroutine
        // still only gets the plain budget hook, same as `run_frame_lua`.
        *script.active_budget.borrow_mut() = Some((
            instructions.clone(),
            budget_hit.clone(),
            FRAME_INSTRUCTION_BUDGET,
        ));
        rearm_coroutines(
            &script.coroutines,
            &instructions,
            &budget_hit,
            FRAME_INSTRUCTION_BUDGET,
        );

        let result: mlua::Result<()> = lua.scope(|scope| {
            let globals = lua.globals();
            register_builtins(
                scope,
                &builtin_impls(lua)?,
                &world,
                &ui,
                &memory,
                &palette,
                &camera,
                &music_player,
                &sfx_pool,
                &next_sfx_age,
                sound.clone(),
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
        *script.active_budget.borrow_mut() = None;

        if hit.borrow().is_some() {
            self.call_stack = stack.borrow().clone();
            self.locals = locals.borrow().clone();
        } else {
            self.call_stack.clear();
            self.locals.clear();
        }

        let breakpoint = hit.borrow().clone();
        // `budget_hit` is checked ahead of `result` for the same reason as
        // `run_frame_lua`: a cart's own `pcall` around a runaway loop can
        // swallow the watchdog's error before it reaches `update.call`,
        // leaving `result` looking like a normal `Ok(())`.
        match (breakpoint, budget_hit.borrow().clone(), result) {
            (Some(breakpoint), _, _) => LuaRunOutcome::Breakpoint(breakpoint),
            (None, Some(location), _) => {
                log::error!(
                    "Lua execution budget exceeded at {}:{}",
                    location.source,
                    location.line
                );
                self.set_fault(VmFault::ExecutionBudgetExceeded);
                LuaRunOutcome::Error(Some(location), EXECUTION_BUDGET_MESSAGE.to_string())
            }
            (None, None, Ok(())) => LuaRunOutcome::Completed,
            (None, None, Err(e)) => {
                log::error!("Lua runtime error: {e}");
                let (location, message) = describe_lua_error_location(&e);
                self.set_fault_with_message(VmFault::LuaError, Some(message.clone()));
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
    /// `read_active_locals`); empty if no breakpoint has fired yet. Table
    /// and function values are rooted for [`Vm::expand_debug_node`] — see
    /// `Vm::root_debug_value`.
    pub fn lua_debug_locals(&mut self) -> Vec<(String, DebugValue)> {
        let locals = self.locals.clone();
        locals
            .into_iter()
            .map(|(name, text, owned)| {
                let debug_value = match owned {
                    Some(value) => self.root_debug_value(format!("local:{name}"), value),
                    None => DebugValue {
                        text,
                        node_id: None,
                    },
                };
                (name, debug_value)
            })
            .collect()
    }

    /// Snapshot of the script's global variables, for the Studio debugger's
    /// state inspector. Excludes registered builtins, the gameplay prelude,
    /// and Lua's own stdlib, so only script-defined state shows up. For locals
    /// at a breakpoint, see [`Vm::lua_debug_locals`]. Table and function
    /// values are rooted for [`Vm::expand_debug_node`].
    pub fn lua_globals(&mut self) -> Vec<(String, DebugValue)> {
        let Some(script) = self.script.as_ref() else {
            return Vec::new();
        };
        let active_prelude_names = self.active_prelude_names();
        let globals = script.lua.globals();
        let mut out: Vec<(String, mlua::Value)> = globals
            .pairs::<String, mlua::Value>()
            .filter_map(|pair| pair.ok())
            .filter(|(k, _)| is_script_defined_name(k, &active_prelude_names))
            .collect();
        out.sort_by(|a, b| a.0.cmp(&b.0));
        out.into_iter()
            .map(|(name, value)| {
                let id = format!("global:{name}");
                let debug_value = self.root_debug_value(id, value);
                (name, debug_value)
            })
            .collect()
    }

    /// Roots `value` under `id` in [`Vm::debug_roots`] when it's a table or
    /// function (so [`Vm::expand_debug_node`] can find it later), and
    /// returns the display `DebugValue` for it. Scalars are never rooted —
    /// they have no children.
    fn root_debug_value(&mut self, id: String, value: mlua::Value) -> DebugValue {
        let text = describe_lua_value(&value);
        let node_id = match &value {
            mlua::Value::Table(_) | mlua::Value::Function(_) => {
                self.debug_roots.insert(id.clone(), value);
                Some(id)
            }
            _ => None,
        };
        DebugValue { text, node_id }
    }

    /// Drops every table/function value rooted for the debugger's
    /// expand-on-demand inspector — call once per tick, before re-gathering
    /// locals/globals/watches, so a node id handed to the frontend never
    /// stays valid past the pause/step it was captured in.
    pub fn clear_debug_roots(&mut self) {
        self.debug_roots.clear();
    }

    /// Returns the immediate children of a table/function previously
    /// rooted by [`Vm::lua_globals`], [`Vm::lua_debug_locals`],
    /// [`Vm::lua_watch`], or a prior call to this method. Read-only: never
    /// evaluates Lua, only walks an already-captured value — same posture
    /// as [`Vm::lua_watch`]. Returns `Err` (never panics) for an unknown or
    /// stale id, e.g. after [`Vm::clear_debug_roots`] ran.
    pub fn expand_debug_node(
        &mut self,
        node_id: &str,
    ) -> Result<Vec<(String, DebugValue)>, String> {
        let value = self
            .debug_roots
            .get(node_id)
            .cloned()
            .ok_or_else(|| "Value is no longer available".to_string())?;
        match value {
            mlua::Value::Table(table) => {
                let mut out = Vec::new();
                for (index, pair) in table.pairs::<mlua::Value, mlua::Value>().enumerate() {
                    let Ok((key, entry)) = pair else { continue };
                    if index >= MAX_EXPAND_ENTRIES {
                        out.push((
                            "…".to_string(),
                            DebugValue {
                                text: "entries truncated".to_string(),
                                node_id: None,
                            },
                        ));
                        break;
                    }
                    let key_text = describe_table_key(&key);
                    let child_id = format!("{node_id}/{key_text}");
                    let debug_value = self.root_debug_value(child_id, entry);
                    out.push((key_text, debug_value));
                }
                Ok(out)
            }
            mlua::Value::Function(function) => {
                let Some(script) = self.script.as_ref() else {
                    return Ok(Vec::new());
                };
                let upvalues = list_function_upvalues(&script.lua, &function);
                Ok(upvalues
                    .into_iter()
                    .map(|(name, value)| {
                        let child_id = format!("{node_id}/upvalue:{name}");
                        let debug_value = self.root_debug_value(child_id, value);
                        (name, debug_value)
                    })
                    .collect())
            }
            _ => Ok(Vec::new()),
        }
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
    /// closure's non-function upvalues are rebound (matched by name, recursing
    /// through function-valued ones so edited helpers stay live) onto the old
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
        let active_prelude_names = self.active_prelude_names();
        let old_functions: Vec<(String, mlua::Function)> = {
            let globals = script.lua.globals();
            globals
                .pairs::<String, mlua::Value>()
                .filter_map(|pair| pair.ok())
                .filter(|(name, _)| is_reload_join_candidate(name, &active_prelude_names))
                .filter_map(|(name, value)| match value {
                    mlua::Value::Function(f) => Some((name, f)),
                    _ => None,
                })
                .collect()
        };
        let lua = &script.lua;
        let selected_modules: Vec<&'static PreludeModule> =
            self.selected_prelude_modules().collect();

        let world = RefCell::new(&mut self.world);
        let ui = RefCell::new(&mut self.ui);
        let memory = RefCell::new(&mut self.memory);
        let palette = RefCell::new(&mut self.palette);
        let camera = RefCell::new(&mut self.camera);
        let music_player = RefCell::new(&mut self.music_player);
        let sfx_pool = RefCell::new(&mut self.sfx_pool);
        let next_sfx_age = RefCell::new(&mut self.next_sfx_age);
        let sound = self.sound.clone();
        let asset_banks = RefCell::new(&mut self.asset_banks);
        let save_data = RefCell::new(&mut self.save_data);
        let sprite_size = self.config.sprite_size;
        let width = self.config.width;
        let height = self.config.height;
        let frame_count = self.frame_count;
        let collision_types = &self.collision_types;

        // Same watchdog gap as `load_lua_source`: the re-exec below ran with
        // no instruction budget at all before this fix.
        let instructions: Rc<RefCell<u32>> = Rc::new(RefCell::new(0));
        let budget_hit: Rc<RefCell<Option<LuaBreakpoint>>> = Rc::new(RefCell::new(None));
        *script.active_budget.borrow_mut() = Some((
            instructions.clone(),
            budget_hit.clone(),
            INIT_INSTRUCTION_BUDGET,
        ));
        lua.set_hook(
            HookTriggers::new().every_nth_instruction(INSTRUCTION_HOOK_STRIDE),
            budget_hook(
                instructions.clone(),
                budget_hit.clone(),
                INIT_INSTRUCTION_BUDGET,
            ),
        );
        rearm_coroutines(
            &script.coroutines,
            &instructions,
            &budget_hit,
            INIT_INSTRUCTION_BUDGET,
        );

        let result: mlua::Result<()> = lua.scope(|scope| {
            register_builtins(
                scope,
                &builtin_impls(lua)?,
                &world,
                &ui,
                &memory,
                &palette,
                &camera,
                &music_player,
                &sfx_pool,
                &next_sfx_age,
                sound.clone(),
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

            lua.load(PRELUDE_CORE).set_name("=prelude:core").exec()?;
            for module in &selected_modules {
                lua.load(module.source)
                    .set_name(format!("=prelude:{}", module.name))
                    .exec()?;
            }
            lua.load(src).set_name(CHUNK_SOURCE_NAME).exec()?;
            // Deliberately not calling `_init()` — that's what makes this a
            // reload rather than a reset.
            Ok(())
        });
        lua.remove_hook();
        *script.active_budget.borrow_mut() = None;
        result?;

        let globals = lua.globals();
        for (name, old_fn) in &old_functions {
            if let Ok(new_fn) = globals.get::<mlua::Function>(name.as_str()) {
                join_matching_upvalues(lua, old_fn, &new_fn)?;
            }
        }

        self.fault = None;
        self.fault_message = None;
        self.waiting = false;
        self.call_stack.clear();
        Ok(())
    }

    /// Reads a dotted global/table path without executing Lua. Studio uses
    /// this for debugger watches, so expressions cannot mutate cart state.
    /// A table/function result is rooted under `"watch:<expression>"` for
    /// [`Vm::expand_debug_node`].
    pub fn lua_watch(&mut self, expression: &str) -> Result<DebugValue, String> {
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
            Ok(self.root_debug_value(format!("watch:{expression}"), value))
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
        assert_eq!(
            vm.lua_watch("player.x").map(|v| v.text),
            Ok("72".to_string())
        );
        assert_eq!(
            vm.lua_watch("player.nested.alive").map(|v| v.text),
            Ok("true".to_string())
        );
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
mod builtin_identity_tests {
    use crate::input::Input;
    use crate::rendering::font::Font;
    use crate::{Vm, VmConfig};

    #[test]
    fn builtin_aliases_and_user_wrappers_survive_frames() {
        let input = Input::new();
        let font = Font::empty();
        let mut vm = Vm::new(VmConfig::default());
        vm.load_lua_source(
            r#"
local alias = fill_rect
local orig = set_pixel
calls = 0
function set_pixel(...) calls = calls + 1 return orig(...) end
function _update() alias(0, 0, 2, 2, 1) set_pixel(5, 5, 1) end
"#,
            &input,
            &font,
        )
        .expect("chunk should load");
        for _ in 0..3 {
            vm.run_frame_lua(&input, &font);
        }
        assert!(
            vm.get_fault().is_none(),
            "aliased builtin must keep working"
        );
        let calls: i64 = vm
            .script
            .as_ref()
            .expect("script should be loaded")
            .lua
            .globals()
            .get("calls")
            .expect("calls global");
        assert_eq!(calls, 3, "user wrapper must not be re-clobbered per frame");
    }
}

#[cfg(test)]
mod audio_builtin_tests {
    use crate::input::Input;
    use crate::rendering::font::Font;
    use crate::vm::audio::{MUSIC_VOICE_COUNT, MUSIC_VOICE_START};
    use crate::{Vm, VmConfig};

    fn vm_with(src: &str) -> (Vm, Input, Font) {
        let input = Input::new();
        let font = Font::empty();
        let mut vm = Vm::new(VmConfig::default());
        vm.load_lua_source(src, &input, &font)
            .expect("chunk should load");
        (vm, input, font)
    }

    #[test]
    fn lua_stop_music_closes_voice_gates() {
        let (mut vm, input, font) = vm_with("function _update() stop_music() end");
        {
            let sound = vm.get_sound_shared();
            let mut s = sound.lock().expect("sound lock");
            for v in s
                .voices
                .iter_mut()
                .skip(MUSIC_VOICE_START)
                .take(MUSIC_VOICE_COUNT)
            {
                v.gate = true;
            }
        }
        vm.run_frame_lua(&input, &font);
        let sound = vm.get_sound_shared();
        let s = sound.lock().expect("sound lock");
        assert!(
            s.voices
                .iter()
                .skip(MUSIC_VOICE_START)
                .take(MUSIC_VOICE_COUNT)
                .all(|v| !v.gate),
            "stop_music must silence held notes"
        );
    }

    #[test]
    fn play_sfx_rejects_ids_past_the_bank() {
        let (mut vm, input, font) = vm_with("function _update() play_sfx(16) end");
        vm.run_frame_lua(&input, &font);
        assert!(
            vm.get_fault().is_some(),
            "id 16 is outside the 16-slot bank"
        );
    }
}

#[cfg(test)]
mod arg_range_tests {
    use crate::input::Input;
    use crate::rendering::font::Font;
    use crate::{Vm, VmConfig};

    #[test]
    fn out_of_range_numbers_follow_the_documented_contract() {
        let input = Input::new();
        let font = Font::empty();
        let mut vm = Vm::new(VmConfig::default());
        vm.load_lua_source(
            r#"
function _update()
  assert(button_down(300) == false)
  assert(button_pressed(-1) == false)
  play_music_song(999)
  set_camera(-5, -7)
end
"#,
            &input,
            &font,
        )
        .expect("chunk should load");
        vm.run_frame_lua(&input, &font);
        assert!(vm.get_fault().is_none(), "{:?}", vm.fault_message());
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

    fn eval_i64(vm: &Vm, expr: &str) -> i64 {
        vm.script
            .as_ref()
            .expect("script should be loaded")
            .lua
            .load(format!("return {expr}"))
            .eval::<i64>()
            .expect("expression should evaluate")
    }

    #[test]
    fn edited_local_function_body_takes_effect_and_state_stays_shared() {
        let input = Input::new();
        let font = Font::empty();
        let mut vm = Vm::new(VmConfig::default());
        let src = |step: i32| {
            format!(
                "local score = 0
local function bump() score = score + {step} end
                 function _update() bump() end
function get_score() return score end
"
            )
        };
        vm.load_lua_source(&src(1), &input, &font)
            .expect("chunk A should load");
        vm.run_frame_lua(&input, &font);
        vm.run_frame_lua(&input, &font);
        assert_eq!(get_score(&vm), 2);

        vm.hot_reload_lua_source(&src(10), &input, &font)
            .expect("reload should succeed");
        assert_eq!(get_score(&vm), 2, "state survives");
        vm.run_frame_lua(&input, &font);
        assert_eq!(get_score(&vm), 12, "edited helper body is live");
    }

    #[test]
    fn prelude_state_and_metatables_survive_reload() {
        let input = Input::new();
        let font = Font::empty();
        let mut vm = Vm::new(VmConfig::default());
        let src = "function _update() end";
        vm.set_prelude_modules(&["vec2", "particles", "scenes", "entities", "camera"])
            .expect("modules should exist");
        vm.load_lua_source(src, &input, &font)
            .expect("chunk should load");
        let lua = &vm.script.as_ref().expect("script").lua;
        lua.load(
            "Scenes.push({}) Particles.spawn(1, 1, 0, 0, 1, 99) Camera.x = 7              Entities.add({}) held = Vec2.new(1, 2)",
        )
        .exec()
        .expect("setup should run");

        vm.hot_reload_lua_source(src, &input, &font)
            .expect("reload should succeed");
        assert_eq!(eval_i64(&vm, "#Scenes.stack"), 1);
        assert_eq!(eval_i64(&vm, "Particles.count()"), 1);
        assert_eq!(eval_i64(&vm, "Camera.x"), 7);
        assert_eq!(eval_i64(&vm, "Entities.count()"), 1);
        assert_eq!(eval_i64(&vm, "(held + Vec2.new(1, 1)).x"), 2);
    }
}
