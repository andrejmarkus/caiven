---
paths:
  - "crates/caiven-vm/src/vm/**"
  - "crates/caiven-vm/src/vm/prelude/**"
  - "carts/**"
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
5. An example cartridge or example project under `carts/` exercising it.
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
