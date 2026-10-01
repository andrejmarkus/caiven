---
paths:
  - "crates/caiven-vm/**"
  - "crates/caiven-machine/**"
---

# VM and frame loop

- Treat per-frame allocations as suspicious. `_update()`/`_draw()` run every
  frame; anything allocating there (new `Vec`, `String`, boxed closures)
  needs a reason, not a shrug.
- Preserve deterministic behavior where the API implies it (RNG seeding,
  fixed timestep, RTC). Don't silently change timing semantics
  (`src/timing.rs`, `src/vm/rtc.rs`) — a change there affects every cartridge
  that assumes current behavior.
- Keep host/runtime responsibility boundaries clear: `caiven-vm` owns
  execution, rendering, input, audio primitives; `caiven-machine` owns
  process/window lifecycle and hot-reload orchestration around a VM
  instance. Don't blur which crate owns which.
- Hot paths of note: `src/vm/lua_exec.rs` (Lua call dispatch),
  `src/vm/api_registry.rs` (builtin registration), `src/rendering/*`,
  `src/input/*`. Changes here warrant a benchmark comparison
  (`caiven-benchmark` skill) if the change could affect per-frame cost.
- Audio (`src/vm/audio.rs`, `sfx.rs`) runs adjacent to a real-time thread via
  SDL2 (`sdl2::audio::AudioCallback`) — never block or allocate unpredictably
  on that path; see `.claude/rules/security.md` for the sandbox-boundary
  angle.
- Audio lifecycle: a host pausing the game calls `Vm::suspend_audio` and
  `resume_audio`, never `stop_audio` (that discards music `_init()` started,
  and Run from pause does not rerun `_init()`). `stop_audio` is for a fresh
  run only. `ConsoleCore` keeps one output for its life: `adopt_vm` moves the
  new VM onto the open sound, never reopens the device per cart.
- Debugger views: carts keep state in file-scope `local`s (upvalues of
  `_update` & co.), not `_G`. Any inspector reading only globals or frame
  locals shows nothing for them; `file_scope_locals` in `lua_exec.rs` covers it.
- Studio loads carts with `load_lua_source_deferred`: top-level code and
  `_init()` run at the top of the first frame, so breakpoints reach them.
  Anything checking boot effects after a Studio load runs a frame (or
  `finish_lua_boot`).
- Hooks: never use mlua's `set_hook`/`Thread::set_hook`. mlua keeps one hooked
  thread and strips the hook from the rest, so arming a coroutine silently
  disarmed the main thread's watchdog and breakpoints. `vm_hook` is one raw
  hook for every thread; coroutines inherit it.
- A breakpoint suspends the frame thread from inside the hook. Resume it with
  `resume_raw`, never mlua's `Thread::resume`: that resets the thread's stack
  top afterwards and cuts into the suspended frame's live registers.
- An `mlua::Value` outlives its `Lua` only as a weak ref: using it after a
  reload panics ("Lua instance is destroyed"). `load_lua` clears every
  debugger field holding values (`locals`, `debug_roots`); expanded child
  nodes keep plain-data paths (`DebugStep`), not values.
- Line steps (`LuaStep`) reuse the breakpoint stop: the hook suspends at the
  next line no deeper than `HookState::step_depth`, measured by `stack_depth`
  from the last stop. A new callback resets the limit to its first line, so a
  step never runs past the end of `_update` into the rest of the frame.
- Layers clear to transparent (`clear_screen()` is alpha 0); only
  `Screen::construct` turns them into a frame, backed with opaque black.
  Anything producing an image (screenshots, covers, canvases) composes
  through it, never its own layer merge, or covers export as blank PNGs.
