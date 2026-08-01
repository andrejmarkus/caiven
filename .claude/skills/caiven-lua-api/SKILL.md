---
name: caiven-lua-api
description: Design or modify Caiven's public Lua API (builtins registered in caiven-vm/src/vm/api_registry.rs and lua_exec.rs, or the pure-Lua stdlib in prelude.lua). Use whenever a change adds, removes, or alters a function/global exposed to cartridge Lua code.
---

# caiven-lua-api

The single source of truth for the builtin surface is
`crates/caiven-vm/src/vm/api_registry.rs` (feeds Studio's autocomplete/hover
and the syntax highlighter's builtin list) kept in manual sync with
`crates/caiven-vm/src/vm/lua_exec.rs::register_builtins`. There is no
codegen tying these together — a mismatch is a silent bug, so touching one
without the other is the single most common way to break this API.

## Required for any change

- Naming consistency with the existing descriptive style (no cryptic
  abbreviations — see README "Descriptive Builtin API").
- Lua 5.4 semantics (real Lua via `mlua`, not a custom subset — don't design
  around a smaller/different language model).
- Clear argument and return behavior, documented explicitly.
- Error semantics decided deliberately: Lua error vs. silent no-op vs.
  default value — pick one and document it, don't leave it implicit.
- Compatibility analysis: does this change behavior for any cart already
  using the old surface? If yes, it's breaking — say so.
- Runtime cost analysis if the function can be called from
  `_update()`/`_draw()` — see `.claude/rules/vm-runtime.md`.
- Tests in `crates/caiven-vm/tests/`.
- Documentation (README API reference / `docs/`).
- Autocomplete updates: keep `api_registry.rs` in sync with
  `lua_exec.rs::register_builtins` (see the doc comment at the top of
  `api_registry.rs` — it says explicitly "must be kept in sync").
- Example usage: an example cartridge or snippet exercising the new/changed
  API, under `carts/` or in docs.

## Non-negotiable

Existing API behavior must never change silently — no version note, no
compat analysis means don't ship it as-is.
