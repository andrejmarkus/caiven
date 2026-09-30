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
use mlua::{Lua, LuaSerdeExt, MultiValue, Scope, StdLib, Table};
use std::cell::{Cell, RefCell};
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
    /// Boxed so the address registered for [`vm_hook`] never moves.
    hooks: Box<HookState>,
    /// Debug frames run here, so a breakpoint can suspend one mid-frame.
    frame_thread: RefCell<Option<FrameThread>>,
    /// The stage a breakpoint suspended; the next debug frame continues it.
    suspended: Cell<Option<FrameStage>>,
    /// Top-level chunk still owed by [`Vm::load_lua_source_deferred`]; the
    /// next frame runs it first.
    pending_chunk: RefCell<Option<mlua::Function>>,
    /// `_init()` still owed by a deferred load; it runs right after the chunk.
    init_pending: Cell<bool>,
}

impl Drop for LuaScript {
    fn drop(&mut self) {
        // Values rooted elsewhere can keep the Lua state alive past `hooks`.
        let _ = set_hook_state(&self.lua, std::ptr::null());
    }
}

/// One step of a debug frame, in call order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FrameStage {
    Chunk,
    Init,
    Update,
    Draw,
}

struct FrameThread {
    thread: mlua::Thread,
    state: *mut mlua_sys::lua_State,
}

/// Result of one debug-aware Lua frame ([`Vm::run_frame_lua_bp`]).
#[derive(Debug, Clone)]
pub enum LuaRunOutcome {
    /// The frame ran to completion.
    Completed,
    /// A breakpoint suspended the frame at this line; the next call
    /// continues it from there.
    Breakpoint(LuaBreakpoint),
    /// A line step ([`LuaStep`]) suspended the frame here, like a breakpoint.
    Step(LuaBreakpoint),
    /// A genuine Lua runtime error (not a breakpoint stop), with the
    /// 1-based source line when [`describe_lua_error`] could recover one.
    Error(Option<LuaBreakpoint>, String),
}

/// A line step from a suspended frame. From a finished function or frame
/// boundary, every kind stops at the next line that runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LuaStep {
    /// The next line, entering any function it calls.
    Into,
    /// The next line in this function or its caller.
    Over,
    /// The next line in the caller.
    Out,
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
/// A suspended call: label, `file:line` and its level on the frame thread.
pub(super) type CallFrame = (String, String, std::os::raw::c_int);

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

/// How a debugger child node is reached from its parent. Plain data, not a
/// Lua value, so the path still resolves after the Lua state is rebuilt.
#[derive(Clone, Debug)]
pub(super) enum DebugStep {
    Field(Vec<u8>),
    Index(i64),
    Upvalue(String),
}

impl DebugStep {
    fn of_key(key: &mlua::Value) -> Option<Self> {
        match key {
            mlua::Value::String(name) => Some(Self::Field(name.as_bytes().to_vec())),
            mlua::Value::Integer(index) => Some(Self::Index(*index)),
            _ => None,
        }
    }

    /// Raw read of this step from `table`; never runs a metamethod.
    fn read(&self, lua: &Lua, table: &Table) -> mlua::Result<mlua::Value> {
        match self {
            Self::Field(name) => table.raw_get(lua.create_string(name)?),
            Self::Index(index) => table.raw_get(*index),
            Self::Upvalue(_) => Ok(mlua::Value::Nil),
        }
    }
}

const WATCH_SYNTAX_ERROR: &str = "Watch must be a name like player.x or items[1]";

/// Whether [`Vm::lua_watch`] accepts `expression`: a name followed by
/// `.field` and `[index]` steps.
pub fn is_watch_expression(expression: &str) -> bool {
    parse_watch_path(expression).is_some()
}

fn parse_watch_path(expression: &str) -> Option<(String, Vec<DebugStep>)> {
    let mut rest = expression.trim();
    let root = take_identifier(&mut rest)?;
    let mut steps = Vec::new();
    while !rest.is_empty() {
        if let Some(after) = rest.strip_prefix('.') {
            rest = after;
            steps.push(DebugStep::Field(take_identifier(&mut rest)?.into_bytes()));
        } else {
            let (index, after) = rest.strip_prefix('[')?.split_once(']')?;
            steps.push(DebugStep::Index(index.parse().ok()?));
            rest = after;
        }
    }
    Some((root, steps))
}

fn take_identifier(rest: &mut &str) -> Option<String> {
    let end = rest
        .find(|c: char| c != '_' && !c.is_ascii_alphanumeric())
        .unwrap_or(rest.len());
    let (name, after) = rest.split_at(end);
    if !is_lua_identifier(name) {
        return None;
    }
    *rest = after;
    Some(name.to_string())
}

/// Maximum table entries [`Vm::expand_debug_node`] returns for one expand —
/// keeps a single click bounded regardless of cart-authored table size.
const MAX_EXPAND_ENTRIES: usize = 200;

fn normalized_debug_source(source: &str) -> String {
    source.trim_start_matches(['@', '=']).replace('\\', "/")
}

/// A frame that still owes the boot gets the load budget on top.
fn frame_budget(boot_pending: bool) -> u32 {
    if boot_pending {
        FRAME_INSTRUCTION_BUDGET + INIT_INSTRUCTION_BUDGET
    } else {
        FRAME_INSTRUCTION_BUDGET
    }
}

/// Registry slot holding the running script's [`HookState`] address.
static HOOK_STATE_KEY: u8 = 0;

fn hook_state_key() -> *const std::ffi::c_void {
    std::ptr::addr_of!(HOOK_STATE_KEY).cast()
}

/// Watchdog and breakpoint state read by [`vm_hook`]. mlua keeps one hook
/// callback per `Lua` and strips it from every other thread, so arming a
/// coroutine used to disarm the main thread. One raw hook serves every
/// thread instead, and a coroutine inherits it from the thread creating it.
struct HookState {
    /// Instructions the current call may run; 0 while nothing is watched.
    budget: Cell<u32>,
    instructions: Cell<u32>,
    budget_hit: RefCell<Option<LuaBreakpoint>>,
    breakpoints: RefCell<Vec<LuaBreakpoint>>,
    /// Only the frame thread stops: a stop suspends it rather than unwinding.
    frame_thread: Cell<*mut mlua_sys::lua_State>,
    hit: RefCell<Option<LuaBreakpoint>>,
    /// While a line step runs: stop at the next line at most this deep.
    step_depth: Cell<Option<usize>>,
    /// Stack depth of the last stop; the next step measures from it.
    stop_depth: Cell<usize>,
    /// The last stop came from a step, not a breakpoint.
    stepped: Cell<bool>,
}

enum HookAction {
    Continue,
    Suspend,
    OutOfBudget,
}

impl HookState {
    fn new() -> Self {
        HookState {
            budget: Cell::new(0),
            instructions: Cell::new(0),
            budget_hit: RefCell::new(None),
            breakpoints: RefCell::new(Vec::new()),
            frame_thread: Cell::new(std::ptr::null_mut()),
            hit: RefCell::new(None),
            step_depth: Cell::new(None),
            stop_depth: Cell::new(0),
            stepped: Cell::new(false),
        }
    }

    /// Starts watching one call with a fresh instruction budget.
    fn watch(&self, budget: u32) {
        self.budget.set(budget);
        self.instructions.set(0);
        self.budget_hit.replace(None);
        self.hit.replace(None);
        self.stepped.set(false);
    }

    fn stop_watching(&self) {
        self.budget.set(0);
        self.frame_thread.set(std::ptr::null_mut());
        self.step_depth.set(None);
    }

    /// # Safety
    /// `state` and `ar` must be the arguments Lua passed to the hook.
    unsafe fn on_event(
        &self,
        state: *mut mlua_sys::lua_State,
        ar: *mut mlua_sys::lua_Debug,
    ) -> HookAction {
        let budget = self.budget.get();
        if budget == 0 {
            return HookAction::Continue;
        }
        unsafe {
            if (*ar).event == mlua_sys::LUA_HOOKCOUNT {
                let count = self
                    .instructions
                    .get()
                    .saturating_add(INSTRUCTION_HOOK_STRIDE);
                self.instructions.set(count);
                if count < budget {
                    return HookAction::Continue;
                }
                if self.budget_hit.borrow().is_none()
                    && mlua_sys::lua_getinfo(state, c"Sl".as_ptr(), ar) != 0
                    && (*ar).currentline > 0
                {
                    self.budget_hit.replace(Some(LuaBreakpoint {
                        source: raw_debug_source(ar),
                        line: (*ar).currentline as usize,
                    }));
                }
                return HookAction::OutOfBudget;
            }
            let line = (*ar).currentline;
            if (*ar).event != mlua_sys::LUA_HOOKLINE
                || line <= 0
                || state != self.frame_thread.get()
            {
                return HookAction::Continue;
            }
            let line = line as usize;
            let stepping = self
                .step_depth
                .get()
                .is_some_and(|deepest| stack_depth(state) <= deepest);
            let breakpoints = self.breakpoints.borrow();
            if (!stepping && !breakpoints.iter().any(|breakpoint| breakpoint.line == line))
                || mlua_sys::lua_getinfo(state, c"S".as_ptr(), ar) == 0
            {
                return HookAction::Continue;
            }
            let source = raw_debug_source(ar);
            let at_breakpoint = breakpoints.iter().any(|breakpoint| {
                breakpoint.line == line
                    && (breakpoint.source == "*"
                        || normalized_debug_source(&breakpoint.source) == source)
            });
            // Code called back from C (a `table.sort` comparator) can't suspend.
            if !(stepping || at_breakpoint) || mlua_sys::lua_isyieldable(state) == 0 {
                return HookAction::Continue;
            }
            drop(breakpoints);
            self.stepped.set(!at_breakpoint);
            self.stop_depth.set(stack_depth(state));
            self.hit.replace(Some(LuaBreakpoint { source, line }));
            HookAction::Suspend
        }
    }
}

/// The one hook for every thread of a script; see [`HookState`].
unsafe extern "C-unwind" fn vm_hook(state: *mut mlua_sys::lua_State, ar: *mut mlua_sys::lua_Debug) {
    unsafe {
        mlua_sys::lua_rawgetp(state, mlua_sys::LUA_REGISTRYINDEX, hook_state_key());
        let hooks = mlua_sys::lua_touserdata(state, -1)
            .cast::<HookState>()
            .cast_const();
        mlua_sys::lua_pop(state, 1);
        let Some(hooks) = hooks.as_ref() else {
            return;
        };
        match hooks.on_event(state, ar) {
            HookAction::Continue => {}
            HookAction::Suspend => {
                mlua_sys::lua_yield(state, 0);
            }
            // `lua_error` longjmps out of this frame; nothing here owns memory.
            HookAction::OutOfBudget => {
                mlua_sys::lua_pushlstring(
                    state,
                    EXECUTION_BUDGET_MESSAGE.as_ptr().cast(),
                    EXECUTION_BUDGET_MESSAGE.len(),
                );
                mlua_sys::lua_error(state);
            }
        }
    }
}

/// Points [`vm_hook`] at `hooks`; null detaches it.
fn set_hook_state(lua: &Lua, hooks: *const HookState) -> mlua::Result<()> {
    unsafe {
        lua.exec_raw::<()>((), |state| {
            if hooks.is_null() {
                mlua_sys::lua_pushnil(state);
            } else {
                mlua_sys::lua_pushlightuserdata(state, hooks.cast_mut().cast());
            }
            mlua_sys::lua_rawsetp(state, mlua_sys::LUA_REGISTRYINDEX, hook_state_key());
        })
    }
}

fn hook_mask(lines: bool) -> std::os::raw::c_int {
    mlua_sys::LUA_MASKCOUNT | if lines { mlua_sys::LUA_MASKLINE } else { 0 }
}

/// Arms [`vm_hook`] on `state`, or disarms it for a zero `mask`.
unsafe fn set_raw_hook(state: *mut mlua_sys::lua_State, mask: std::os::raw::c_int) {
    let hook: Option<mlua_sys::lua_Hook> = (mask != 0).then_some(vm_hook);
    unsafe {
        mlua_sys::lua_sethook(
            state,
            hook,
            mask,
            INSTRUCTION_HOOK_STRIDE as std::os::raw::c_int,
        )
    };
}

/// Arms the main thread. Coroutines copy the hook of the thread creating them.
fn set_main_hook(lua: &Lua, mask: std::os::raw::c_int) {
    unsafe {
        let _ = lua.exec_raw::<()>((), |state| set_raw_hook(state, mask));
    }
}

fn raw_thread_state(lua: &Lua, thread: &mlua::Thread) -> mlua::Result<*mut mlua_sys::lua_State> {
    let mut raw = std::ptr::null_mut();
    unsafe {
        lua.exec_raw::<()>(thread.clone(), |state| {
            raw = mlua_sys::lua_tothread(state, -1);
            mlua_sys::lua_pop(state, 1);
        })?;
    }
    Ok(raw)
}

/// Resumes `thread` with raw `lua_resume`; `Ok(true)` once it finished,
/// `Ok(false)` when it yielded. mlua's `Thread::resume` resets the thread's
/// stack top afterwards, which cuts into the registers of a frame a
/// breakpoint suspended.
fn resume_raw(lua: &Lua, thread: *mut mlua_sys::lua_State) -> mlua::Result<bool> {
    let mut status = mlua_sys::LUA_OK;
    let error = unsafe {
        lua.exec_raw::<mlua::Value>((), |state| {
            let mut results = 0;
            status = mlua_sys::lua_resume(thread, state, 0, &mut results);
            if status == mlua_sys::LUA_OK || status == mlua_sys::LUA_YIELD {
                mlua_sys::lua_pop(thread, results);
            } else if mlua_sys::lua_type(thread, -1) == mlua_sys::LUA_TSTRING {
                mlua_sys::luaL_traceback(state, thread, mlua_sys::lua_tostring(thread, -1), 0);
                mlua_sys::lua_pop(thread, 1);
            } else {
                mlua_sys::lua_xmove(thread, state, 1);
            }
        })?
    };
    match status {
        mlua_sys::LUA_OK => Ok(true),
        mlua_sys::LUA_YIELD => Ok(false),
        _ => Err(match error {
            mlua::Value::Error(error) => *error,
            mlua::Value::String(message) => {
                mlua::Error::RuntimeError(message.to_string_lossy().to_string())
            }
            other => mlua::Error::RuntimeError(format!("{other:?}")),
        }),
    }
}

/// [`normalized_debug_source`] for a raw `lua_Debug` filled with "S".
unsafe fn raw_debug_source(ar: *const mlua_sys::lua_Debug) -> String {
    let short_src = unsafe { std::ffi::CStr::from_ptr((*ar).short_src.as_ptr()) };
    let short_src = short_src.to_string_lossy();
    if short_src.is_empty() {
        "cart".to_string()
    } else {
        normalized_debug_source(&short_src)
    }
}

/// Calls active on `state`, C ones included.
unsafe fn stack_depth(state: *mut mlua_sys::lua_State) -> usize {
    let mut depth = 0;
    unsafe {
        let mut ar: mlua_sys::lua_Debug = std::mem::zeroed();
        while mlua_sys::lua_getstack(state, depth, &mut ar) != 0 {
            depth += 1;
        }
    }
    depth as usize
}

impl LuaScript {
    fn boot_pending(&self) -> bool {
        self.pending_chunk.borrow().is_some() || self.init_pending.get()
    }

    /// Runs what a deferred load still owes, on the calling thread.
    fn run_pending_boot(&self) -> mlua::Result<()> {
        let chunk = self.pending_chunk.borrow().clone();
        if let Some(chunk) = chunk {
            chunk.call::<()>(())?;
            self.pending_chunk.take();
        }
        if self.init_pending.get() {
            if let Ok(init) = self.lua.globals().get::<mlua::Function>("_init") {
                init.call::<()>(())?;
            }
            self.init_pending.set(false);
        }
        Ok(())
    }

    fn first_stage(&self) -> FrameStage {
        if self.pending_chunk.borrow().is_some() {
            FrameStage::Chunk
        } else if self.init_pending.get() {
            FrameStage::Init
        } else {
            FrameStage::Update
        }
    }

    /// Puts `function` on the frame thread, reusing the thread when it
    /// finished cleanly last time.
    fn start_frame_thread(&self, function: mlua::Function) -> mlua::Result<()> {
        let mut slot = self.frame_thread.borrow_mut();
        if let Some(frame) = slot.as_ref()
            && frame.thread.reset(function.clone()).is_ok()
        {
            return Ok(());
        }
        let thread = self.lua.create_thread(function)?;
        let state = raw_thread_state(&self.lua, &thread)?;
        *slot = Some(FrameThread { thread, state });
        Ok(())
    }

    /// Runs the owed boot, `_update()` and `_draw()` on the frame thread,
    /// continuing `resume` when a breakpoint suspended the last call.
    /// `Ok(true)` means a breakpoint suspended it again.
    fn run_frame_stages(&self, resume: Option<FrameStage>, lines: bool) -> mlua::Result<bool> {
        let globals = self.lua.globals();
        let mut stage = resume.unwrap_or_else(|| self.first_stage());
        let mut fresh = resume.is_none();
        loop {
            // A step that outlived its function stops at the next callback's first line.
            if fresh && self.hooks.step_depth.get().is_some() {
                self.hooks.step_depth.set(Some(usize::MAX));
            }
            let runnable = if fresh {
                let function = match stage {
                    FrameStage::Chunk => self.pending_chunk.borrow().clone(),
                    FrameStage::Init => globals.get::<mlua::Function>("_init").ok(),
                    FrameStage::Update => Some(globals.get::<mlua::Function>("_update")?),
                    FrameStage::Draw => globals.get::<mlua::Function>("_draw").ok(),
                };
                match function {
                    Some(function) => {
                        self.start_frame_thread(function)?;
                        true
                    }
                    None => false,
                }
            } else {
                true
            };
            if runnable && self.resume_frame_thread(lines)? {
                self.suspended.set(Some(stage));
                return Ok(true);
            }
            stage = match stage {
                FrameStage::Chunk => {
                    self.pending_chunk.take();
                    if self.init_pending.get() {
                        FrameStage::Init
                    } else {
                        FrameStage::Update
                    }
                }
                FrameStage::Init => {
                    self.init_pending.set(false);
                    FrameStage::Update
                }
                FrameStage::Update => FrameStage::Draw,
                FrameStage::Draw => return Ok(false),
            };
            fresh = true;
        }
    }

    /// Resumes the frame thread; `Ok(true)` when a breakpoint suspended it.
    fn resume_frame_thread(&self, lines: bool) -> mlua::Result<bool> {
        let Some((thread, state)) = self
            .frame_thread
            .borrow()
            .as_ref()
            .map(|frame| (frame.thread.clone(), frame.state))
        else {
            return Ok(false);
        };
        unsafe { set_raw_hook(state, hook_mask(lines)) };
        self.hooks.frame_thread.set(state);
        let resumed = resume_raw(&self.lua, state);
        self.hooks.frame_thread.set(std::ptr::null_mut());
        drop(thread);
        match resumed {
            Err(error) => {
                self.frame_thread.take();
                return Err(error);
            }
            Ok(true) => return Ok(false),
            Ok(false) => {}
        }
        if self.hooks.hit.borrow().is_none() {
            // The cart itself yielded; on the main thread that's an error too.
            self.frame_thread.take();
            return Err(mlua::Error::runtime(
                "attempt to yield from outside a coroutine",
            ));
        }
        Ok(true)
    }
}

/// Walks a thread suspended at a breakpoint, innermost frame first, for the
/// Studio debugger's Call stack panel: label, `file:line` and stack level.
fn capture_call_stack(lua: &Lua, thread: *mut mlua_sys::lua_State) -> Vec<CallFrame> {
    use std::ffi::CStr;

    // Entry callbacks start the frame thread, so Lua can't name their frames.
    let globals = lua.globals();
    let entries: Vec<(&str, Option<String>, Option<usize>)> = ["_init", "_update", "_draw"]
        .into_iter()
        .filter_map(|name| {
            let info = globals.raw_get::<mlua::Function>(name).ok()?.info();
            Some((name, info.source, info.line_defined))
        })
        .collect();
    let mut frames = Vec::new();
    for level in 0..=64 {
        unsafe {
            let mut ar: mlua_sys::lua_Debug = std::mem::zeroed();
            if mlua_sys::lua_getstack(thread, level, &mut ar) == 0
                || mlua_sys::lua_getinfo(thread, c"nSl".as_ptr(), &mut ar) == 0
            {
                break;
            }
            let line = ar.currentline;
            if line <= 0 {
                continue;
            }
            let source = (!ar.source.is_null())
                .then(|| CStr::from_ptr(ar.source).to_string_lossy().into_owned());
            let line_defined = usize::try_from(ar.linedefined).ok();
            let label = (!ar.name.is_null())
                .then(|| CStr::from_ptr(ar.name).to_string_lossy().into_owned())
                .or_else(|| {
                    entries
                        .iter()
                        .find(|(_, entry_source, entry_line)| {
                            *entry_source == source && *entry_line == line_defined
                        })
                        .map(|(name, _, _)| name.to_string())
                })
                .unwrap_or_else(|| {
                    if !ar.what.is_null() && CStr::from_ptr(ar.what) == c"main" {
                        "main chunk".to_string()
                    } else if level == 0 {
                        "?".to_string()
                    } else {
                        "anonymous function".to_string()
                    }
                });
            let file = CStr::from_ptr(ar.short_src.as_ptr()).to_string_lossy();
            frames.push((label, format!("{file}:{line}"), level));
        }
    }
    frames
}

/// Reads the active locals of the function a breakpoint suspended `thread`
/// in, via raw `lua_getlocal` (mlua has no locals accessor). Read-only.
///
/// `lua_getlocal` enumerates every local active at the current program
/// counter, including ones a later `local` declaration shadows — later
/// declarations come later in the `n` enumeration, so overwriting on a name
/// collision keeps the innermost (currently visible) binding. Names starting
/// with `(` are compiler-internal (e.g. `(for state)`) and are skipped.
///
/// Table/function locals also get an owned [`mlua::Value`] (see
/// [`fetch_local_value`]) for the expand-on-demand inspector to root.
fn read_active_locals(
    lua: &Lua,
    thread: *mut mlua_sys::lua_State,
    level: std::os::raw::c_int,
) -> Vec<RawLocal> {
    use std::ffi::CStr;
    use std::os::raw::c_int;

    let mut locals: Vec<RawLocal> = Vec::new();
    unsafe {
        let mut ar: mlua_sys::lua_Debug = std::mem::zeroed();
        // Level 0 is the function the breakpoint suspended.
        if mlua_sys::lua_getstack(thread, level, &mut ar) == 0
            || mlua_sys::lua_checkstack(thread, 1) == 0
        {
            return locals;
        }
        let mut n: c_int = 1;
        loop {
            let name_ptr = mlua_sys::lua_getlocal(thread, &ar, n);
            if name_ptr.is_null() {
                break;
            }
            let local_index = n;
            n += 1;
            let name = CStr::from_ptr(name_ptr).to_string_lossy().into_owned();
            if name.starts_with('(') {
                mlua_sys::lua_pop(thread, 1);
                continue;
            }
            let raw_type = mlua_sys::lua_type(thread, -1);
            let value = describe_raw_stack_value(thread, -1);
            mlua_sys::lua_pop(thread, 1);
            let owned = if matches!(raw_type, mlua_sys::LUA_TTABLE | mlua_sys::LUA_TFUNCTION) {
                fetch_local_value(lua, thread, &ar, local_index)
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

/// Fetches local slot `n` of the suspended `thread` as an owned
/// [`mlua::Value`]: the slot is moved onto the main stack inside `exec_raw`,
/// whose result conversion gives a table/function a registry reference that
/// outlives the frame.
unsafe fn fetch_local_value(
    lua: &Lua,
    thread: *mut mlua_sys::lua_State,
    ar: &mlua_sys::lua_Debug,
    n: std::os::raw::c_int,
) -> Option<mlua::Value> {
    let ar_ptr = ar as *const mlua_sys::lua_Debug;
    unsafe {
        lua.exec_raw::<mlua::Value>((), move |state| {
            if !mlua_sys::lua_getlocal(thread, ar_ptr, n).is_null() {
                mlua_sys::lua_xmove(thread, state, 1);
            }
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

/// Chunk-scope `local` state reachable from the entry callbacks — the tutorial
/// idiom keeps all game state there, invisible to a plain `_G` walk.
/// Helper functions are followed, not listed; first binding of a name wins.
fn file_scope_locals(lua: &Lua) -> Vec<(String, mlua::Value)> {
    let globals = lua.globals();
    let mut pending: Vec<mlua::Function> = ["_init", "_update", "_draw"]
        .iter()
        .filter_map(|name| globals.raw_get::<mlua::Function>(*name).ok())
        .collect();
    let mut seen = std::collections::HashSet::new();
    let mut out: Vec<(String, mlua::Value)> = Vec::new();
    let mut next = 0;
    while let Some(function) = pending.get(next).cloned() {
        next += 1;
        if !seen.insert(function.to_pointer()) {
            continue;
        }
        for (name, value) in list_function_upvalues(lua, &function) {
            // C upvalues are unnamed; `_ENV` is the globals table itself.
            if name.is_empty() || name == "_ENV" {
                continue;
            }
            match value {
                mlua::Value::Function(child) => pending.push(child),
                value if !out.iter().any(|(existing, _)| *existing == name) => {
                    out.push((name, value));
                }
                _ => {}
            }
        }
    }
    out
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
            if let Ok(mut s) = sound_for_master_volume.lock() {
                s.master_volume = (v as f32).clamp(0.0, 1.0);
            }
            Ok(())
        })?,
    )?;

    let sound_for_music_volume = sound.clone();
    impls.set(
        "set_music_volume",
        scope.create_function_mut(move |_, v: f64| {
            if let Ok(mut s) = sound_for_music_volume.lock() {
                s.music_volume = (v as f32).clamp(0.0, 1.0);
            }
            Ok(())
        })?,
    )?;

    impls.set(
        "set_sfx_volume",
        scope.create_function_mut(move |_, v: f64| {
            if let Ok(mut s) = sound.lock() {
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
        self.load_lua(src, input, font, true)
    }

    /// Like [`Vm::load_lua_source`], but the cart's top-level code and
    /// `_init()` run at the start of the next frame, under that frame's hooks,
    /// so Studio breakpoints stop there too. A syntax error still fails here.
    pub fn load_lua_source_deferred(
        &mut self,
        src: &str,
        input: &Input,
        font: &Font,
    ) -> mlua::Result<()> {
        self.load_lua(src, input, font, false)
    }

    fn load_lua(
        &mut self,
        src: &str,
        input: &Input,
        font: &Font,
        boot_now: bool,
    ) -> mlua::Result<()> {
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

        let hooks = Box::new(HookState::new());
        set_hook_state(&lua, &*hooks)?;

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

        // Top-level code and `_init()` get the watchdog too, so an endless
        // loop there fails the load instead of hanging it.
        hooks.watch(INIT_INSTRUCTION_BUDGET);
        set_main_hook(&lua, hook_mask(false));

        let result: mlua::Result<Option<mlua::Function>> = lua.scope(|scope| {
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
            let chunk = lua.load(src).set_name(CHUNK_SOURCE_NAME).into_function()?;
            if !boot_now {
                return Ok(Some(chunk));
            }
            chunk.call::<()>(())?;
            if let Ok(init) = globals.get::<mlua::Function>("_init") {
                init.call::<()>(())?;
            }
            Ok(None)
        });
        set_main_hook(&lua, 0);
        hooks.stop_watching();
        let pending_chunk = result?;

        self.script = Some(LuaScript {
            lua,
            output,
            hooks,
            frame_thread: RefCell::new(None),
            suspended: Cell::new(None),
            init_pending: Cell::new(!boot_now),
            pending_chunk: RefCell::new(pending_chunk),
        });
        self.fault = None;
        self.fault_message = None;
        self.waiting = false;
        self.call_stack.clear();
        self.selected_frame = 0;
        // Values of the old Lua state panic on use once it is gone.
        self.locals.clear();
        self.debug_roots.clear();
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

    /// Runs the top-level code and `_init()` a deferred load still owes, now
    /// and without breakpoints, for a host that shows the booted state before
    /// the first frame.
    pub fn finish_lua_boot(&mut self, input: &Input, font: &Font) {
        if self.script.as_ref().is_some_and(LuaScript::boot_pending) {
            self.run_lua_entry(input, font, false);
        }
    }

    pub(super) fn run_frame_lua(&mut self, input: &Input, font: &Font) {
        self.run_lua_entry(input, font, true);
    }

    /// One Lua-driven frame: refills the builtin implementations against this
    /// frame's borrowed VM state via `Lua::scope` (the permanent globals are
    /// trampolines into them), runs any owed boot, then `_update()`/`_draw()`
    /// unless `frame` is false.
    fn run_lua_entry(&mut self, input: &Input, font: &Font, frame: bool) {
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

        script.hooks.watch(frame_budget(script.boot_pending()));
        set_main_hook(lua, hook_mask(false));

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

            script.run_pending_boot()?;
            if !frame {
                return Ok(());
            }
            let update: mlua::Function = globals.get("_update")?;
            update.call::<()>(())?;
            if let Ok(draw) = globals.get::<mlua::Function>("_draw") {
                draw.call::<()>(())?;
            }
            Ok(())
        });
        set_main_hook(lua, 0);
        script.hooks.stop_watching();
        let budget_hit = script.hooks.budget_hit.borrow().clone();

        // Checked ahead of `result`, not just on `Err`: a cart can wrap its
        // own runaway loop in `pcall`, which swallows the hook's error
        // before it ever reaches `update.call`, so `result` comes back `Ok`
        // even though the watchdog had to step in. That must still surface
        // as a fault instead of silently reporting a normal frame.
        if let Some(location) = budget_hit {
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

    /// Like `Vm::run_frame_lua`, but the frame runs on a Lua thread that a
    /// breakpoint suspends rather than unwinds: this returns
    /// [`LuaRunOutcome::Breakpoint`] with the frame still alive, and the next
    /// call continues it from the breakpointed line, so no code runs twice.
    /// While suspended, [`Vm::lua_globals`], [`Vm::lua_debug_locals`] and
    /// [`Vm::lua_call_stack`] show the frame's live state.
    pub fn run_frame_lua_bp(
        &mut self,
        input: &Input,
        font: &Font,
        breakpoints: &[LuaBreakpoint],
    ) -> LuaRunOutcome {
        self.run_frame_lua_step(input, font, breakpoints, None)
    }

    /// [`Vm::run_frame_lua_bp`] that also stops after one line `step`,
    /// reporting [`LuaRunOutcome::Step`].
    pub fn run_frame_lua_step(
        &mut self,
        input: &Input,
        font: &Font,
        breakpoints: &[LuaBreakpoint],
        step: Option<LuaStep>,
    ) -> LuaRunOutcome {
        let resume = self
            .script
            .as_ref()
            .and_then(|script| script.suspended.take());
        // A continued frame already did this when it started.
        if resume.is_none() {
            // `run_frame` ticks these; this path grew separately and didn't, so
            // Studio's Running state was silent even though a sound was "active".
            self.tick_audio_players();
            self.peripherals
                .tick_all(&mut self.memory, self.frame_count);
            self.frame_count = self.frame_count.wrapping_add(1);
        }

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

        script.hooks.watch(frame_budget(script.boot_pending()));
        *script.hooks.breakpoints.borrow_mut() = breakpoints.to_vec();
        let from = script.hooks.stop_depth.get();
        script
            .hooks
            .step_depth
            .set(step.map(|step| match (resume, step) {
                (None, _) | (_, LuaStep::Into) => usize::MAX,
                (Some(_), LuaStep::Over) => from,
                (Some(_), LuaStep::Out) => from.saturating_sub(1),
            }));
        // Lua that a builtin runs on the main thread keeps the watchdog.
        set_main_hook(lua, hook_mask(false));

        let result: mlua::Result<bool> = lua.scope(|scope| {
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
            script.run_frame_stages(resume, !breakpoints.is_empty() || step.is_some())
        });
        set_main_hook(lua, 0);
        script.hooks.stop_watching();
        let hit = script.hooks.hit.borrow().clone();
        let budget_hit = script.hooks.budget_hit.borrow().clone();
        let stepped = script.hooks.stepped.get();

        let suspended_thread = match result {
            Ok(true) => script
                .frame_thread
                .borrow()
                .as_ref()
                .map(|frame| frame.state),
            _ => None,
        };
        match suspended_thread {
            Some(thread) => {
                self.call_stack = capture_call_stack(lua, thread);
                self.locals = read_active_locals(lua, thread, 0);
            }
            None => {
                self.call_stack.clear();
                self.locals.clear();
            }
        }
        self.selected_frame = 0;

        // `budget_hit` is checked ahead of `result` for the same reason as
        // `run_frame_lua`: a cart's own `pcall` around a runaway loop can
        // swallow the watchdog's error, leaving `result` looking normal.
        match (result, hit, budget_hit) {
            (_, _, Some(location)) => {
                if let Some(script) = self.script.as_ref() {
                    script.suspended.set(None);
                    script.frame_thread.take();
                }
                self.call_stack.clear();
                self.locals.clear();
                log::error!(
                    "Lua execution budget exceeded at {}:{}",
                    location.source,
                    location.line
                );
                self.set_fault(VmFault::ExecutionBudgetExceeded);
                LuaRunOutcome::Error(Some(location), EXECUTION_BUDGET_MESSAGE.to_string())
            }
            (Ok(true), Some(location), None) if stepped => LuaRunOutcome::Step(location),
            (Ok(true), Some(breakpoint), None) => LuaRunOutcome::Breakpoint(breakpoint),
            (Ok(_), _, None) => LuaRunOutcome::Completed,
            (Err(e), _, None) => {
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
        self.call_stack
            .iter()
            .map(|(label, location, _)| (label.clone(), location.clone()))
            .collect()
    }

    /// Index into [`Vm::lua_call_stack`] whose locals the debugger shows.
    pub fn lua_selected_frame(&self) -> usize {
        self.selected_frame
    }

    /// Shows frame `index` of the suspended call stack in
    /// [`Vm::lua_debug_locals`] and resolves watches against it.
    pub fn select_lua_frame(&mut self, index: usize) -> Result<(), String> {
        let (_, _, level) = self
            .call_stack
            .get(index)
            .ok_or_else(|| "No such call stack frame".to_string())?;
        let script = self
            .script
            .as_ref()
            .ok_or_else(|| "No Lua cart loaded".to_string())?;
        let thread = script
            .frame_thread
            .borrow()
            .as_ref()
            .filter(|_| script.suspended.get().is_some())
            .map(|frame| frame.state)
            .ok_or_else(|| "The frame is no longer paused".to_string())?;
        self.locals = read_active_locals(&script.lua, thread, *level);
        self.selected_frame = index;
        Ok(())
    }

    /// Local variables at the innermost frame, captured at the moment the
    /// last breakpoint was hit — cleared once execution resumes past a
    /// breakpoint. Read via raw FFI from the suspended frame thread (see
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

    /// Snapshot of the script's global variables plus its file-scope
    /// `local`s, for the Studio debugger's state inspector. Excludes
    /// registered builtins, the gameplay prelude, and Lua's own stdlib, so
    /// only script-defined state shows up. For locals at a breakpoint, see
    /// [`Vm::lua_debug_locals`]. Table and function values are rooted for
    /// [`Vm::expand_debug_node`].
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
        for (name, value) in file_scope_locals(&script.lua) {
            if !out.iter().any(|(existing, _)| *existing == name) {
                out.push((name, value));
            }
        }
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

    /// Describes the child `key_text` of expanded node `parent`. A child an
    /// expand handed out resolves by path from its parent's current value,
    /// so it stays expandable, and live, after the next tick re-roots it.
    fn child_debug_value(
        &mut self,
        parent: &str,
        key_text: &str,
        step: Option<DebugStep>,
        value: mlua::Value,
    ) -> DebugValue {
        let id = format!("{parent}/{key_text}");
        match step {
            Some(step) if matches!(value, mlua::Value::Table(_) | mlua::Value::Function(_)) => {
                self.debug_children
                    .insert(id.clone(), (parent.to_string(), step));
                DebugValue {
                    text: describe_lua_value(&value),
                    node_id: Some(id),
                }
            }
            // A table or float key has no path; it lasts until the next tick.
            _ => self.root_debug_value(id, value),
        }
    }

    /// The current value behind `id`: a root, or a child walked from one.
    fn resolve_debug_node(&self, id: &str) -> Option<mlua::Value> {
        if let Some(value) = self.debug_roots.get(id) {
            return Some(value.clone());
        }
        let (parent, step) = self.debug_children.get(id)?;
        let lua = &self.script.as_ref()?.lua;
        let value = match (self.resolve_debug_node(parent)?, step) {
            (mlua::Value::Function(function), DebugStep::Upvalue(name)) => {
                list_function_upvalues(lua, &function)
                    .into_iter()
                    .find(|(upvalue, _)| upvalue == name)?
                    .1
            }
            (mlua::Value::Table(table), step) => step.read(lua, &table).ok()?,
            _ => return None,
        };
        (!value.is_nil()).then_some(value)
    }

    /// Drops every table/function value rooted for the debugger's
    /// expand-on-demand inspector — call once per tick, before re-gathering
    /// locals/globals/watches, so a root id never outlives the pause/step it
    /// was captured in. Expanded children resolve through their root again.
    pub fn clear_debug_roots(&mut self) {
        self.debug_roots.clear();
    }

    /// Returns the immediate children of a table/function previously
    /// rooted by [`Vm::lua_globals`], [`Vm::lua_debug_locals`],
    /// [`Vm::lua_watch`], or a prior call to this method. Read-only: never
    /// evaluates Lua, only walks the value — same posture as
    /// [`Vm::lua_watch`]. Returns `Err` (never panics) for an unknown or
    /// stale id, e.g. a root after [`Vm::clear_debug_roots`] ran.
    pub fn expand_debug_node(
        &mut self,
        node_id: &str,
    ) -> Result<Vec<(String, DebugValue)>, String> {
        let value = self
            .resolve_debug_node(node_id)
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
                    let debug_value =
                        self.child_debug_value(node_id, &key_text, DebugStep::of_key(&key), entry);
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
                        let step = DebugStep::Upvalue(name.clone());
                        let debug_value = self.child_debug_value(
                            node_id,
                            &format!("upvalue:{name}"),
                            Some(step),
                            value,
                        );
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
        // The boot never ran, so there is no state to keep.
        if script.pending_chunk.borrow().is_some() {
            return self.load_lua(src, input, font, false);
        }

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

        // The re-exec gets the same watchdog as `load_lua_source`.
        script.hooks.watch(INIT_INSTRUCTION_BUDGET);
        set_main_hook(lua, hook_mask(false));

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
        set_main_hook(lua, 0);
        script.hooks.stop_watching();
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
        self.selected_frame = 0;
        Ok(())
    }

    /// Reads a watch path like `player.x` or `items[1].hp` without executing
    /// Lua, so a watch can't mutate cart state. The name resolves like Lua
    /// would at the breakpoint: a local of the stopped function first, then
    /// globals and file-scope locals. A table/function result is rooted
    /// under `"watch:<expression>"` for [`Vm::expand_debug_node`].
    pub fn lua_watch(&mut self, expression: &str) -> Result<DebugValue, String> {
        let (root, steps) =
            parse_watch_path(expression).ok_or_else(|| WATCH_SYNTAX_ERROR.to_string())?;
        let script = self
            .script
            .as_ref()
            .ok_or_else(|| "No Lua cart loaded".to_string())?;
        let mut value = match self.locals.iter().find(|(name, _, _)| *name == root) {
            // Scalar locals are only kept as display text.
            Some((_, text, None)) if steps.is_empty() => {
                return Ok(DebugValue {
                    text: text.clone(),
                    node_id: None,
                });
            }
            Some((_, _, owned)) => owned.clone().unwrap_or(mlua::Value::Nil),
            None => {
                let global: mlua::Value = script
                    .lua
                    .globals()
                    .raw_get(root.as_str())
                    .map_err(|error| error.to_string())?;
                if global.is_nil() {
                    file_scope_locals(&script.lua)
                        .into_iter()
                        .find(|(name, _)| *name == root)
                        .map_or(mlua::Value::Nil, |(_, local)| local)
                } else {
                    global
                }
            }
        };
        for step in &steps {
            value = match value {
                mlua::Value::Table(table) => step
                    .read(&script.lua, &table)
                    .map_err(|error| error.to_string())?,
                _ => return Err(format!("{expression} is not a table")),
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
    fn watch_does_not_fire_index_metamethods() {
        let mut vm = Vm::new(VmConfig::default());
        vm.load_lua_source(
            "hits = 0
player = setmetatable({}, { __index = function(_, k) hits = hits + 1 return 1 end })
function _update() end",
            &Input::new(),
            &Font::empty(),
        )
        .expect("watch fixture should load");
        assert!(vm.lua_watch("player.missing").is_err());
        assert_eq!(vm.lua_watch("hits").map(|v| v.text), Ok("0".to_string()));
    }

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
    fn watch_expressions_are_names_fields_and_indexes() {
        use super::is_watch_expression;
        assert!(is_watch_expression("player.x"));
        assert!(is_watch_expression(" _state.enemy_2.hp "));
        assert!(is_watch_expression("enemies[1].hp"));
        assert!(is_watch_expression("grid[2][-1]"));
        assert!(!is_watch_expression("player.x + 1"));
        assert!(!is_watch_expression("player..x"));
        assert!(!is_watch_expression("2player.x"));
        assert!(!is_watch_expression("enemies[i]"));
        assert!(!is_watch_expression("enemies[1"));
        assert!(!is_watch_expression("f()"));
    }

    #[test]
    fn indexed_watch_reads_array_items() {
        let mut vm = Vm::new(VmConfig::default());
        vm.load_lua_source(
            "enemies = { { hp = 3 }, { hp = 5 } }\nfunction _update() end",
            &Input::new(),
            &Font::empty(),
        )
        .expect("watch fixture should load");
        assert_eq!(
            vm.lua_watch("enemies[2].hp").map(|v| v.text),
            Ok("5".to_string())
        );
        assert!(vm.lua_watch("enemies[3].hp").is_err());
    }

    #[test]
    fn file_scope_locals_show_as_globals_and_watches() {
        let mut vm = Vm::new(VmConfig::default());
        vm.load_lua_source(
            "local paddle_x = 12
local balls = { { x = 3 } }
local function step() paddle_x = paddle_x + #balls end
function _update() step() end",
            &Input::new(),
            &Font::empty(),
        )
        .expect("file-scope fixture should load");
        let globals: Vec<(String, String)> = vm
            .lua_globals()
            .into_iter()
            .map(|(name, value)| (name, value.text))
            .collect();
        assert_eq!(
            globals,
            vec![
                ("balls".to_string(), "{table}".to_string()),
                ("paddle_x".to_string(), "12".to_string()),
            ]
        );
        assert_eq!(
            vm.lua_watch("paddle_x").map(|v| v.text),
            Ok("12".to_string())
        );
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
