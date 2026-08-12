---
paths:
  - "crates/caiven-vm/src/vm/**"
  - "crates/caiven-vm/src/vm/prelude/**"
  - "carts/**"
  - "projects/**"
---

# Public Lua API

Any public Lua API change (new/changed function in `api_registry.rs`,
`lua_exec.rs`, or `prelude/*.lua`) must ship together with:

1. The implementation.
2. Tests (VM-level, in `crates/caiven-vm/tests/`).
3. Documentation (README API reference and/or `docs/`).
4. Studio autocomplete / language-support updates
   (`crates/caiven-studio-ui` — codemirror Lua definitions) so the editor
   doesn't silently drift from the runtime.
5. An example cartridge or example project under `projects/dev/` exercising
   it.
6. A compatibility analysis: does this change behavior for any existing
   cart? If yes, that's a breaking change — call it out explicitly, don't
   let it happen quietly.
7. Error-behavior documentation: what happens on bad arguments (Lua error?
   silent no-op? default value?) — pick one deliberately and document it.

Naming must match existing convention (descriptive, no cryptic
abbreviations — see README "Descriptive Builtin API"). Argument/return
shapes and error semantics should follow Lua 5.4 idiom, not a bolted-on
foreign convention. Consider runtime cost for anything called from
`_update()`/`_draw()`.

Existing API behavior must never change silently — a behavior change with no
version/compat note is a bug, not a feature.

## Third registry: `BUILTIN_NAMES` in `lua_exec.rs`

A new global registered in `register_builtins` (`lua_exec.rs`) must *also* be
added to the `BUILTIN_NAMES` const near the top of that same file — a list
separate from both `api_registry.rs`'s `BUILTINS` and the `globals.set(...)`
call itself. It marks a name as "API surface, not script state" for
`Vm::lua_globals` (debugger snapshot) and hot-reload's upvalue-join filter
(`is_reload_join_candidate`). Omitting it doesn't fail loudly: hot reload
treats the omitted name as a script-defined closure and calls
`lua_upvaluejoin` on what is actually a native Rust function, which aborts
the process (`SIGABRT`, "Lua function expected" assertion in `lapi.c`) the
next time a cart using that name is hot-reloaded — not a compile error, not
a normal test failure, just a crash the *next* time you happen to run the
right test. `cargo test -p caiven-vm` catches it only if you run the
`hot_reload_tests` (they don't always fail under a parallel run — the
crash can come and go with test scheduling, so a single flaky-looking abort
is not safe to dismiss as flakiness without checking this first).
