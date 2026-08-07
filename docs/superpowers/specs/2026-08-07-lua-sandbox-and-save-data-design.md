# Lua sandbox hardening + persistent save-data API

Status: approved, pending implementation plan.

## Context

`caiven-lua-api` audit for "make the API viable for building any type of
game" surfaced persistent save data as the highest-priority gap: no cart
can currently save high scores, progress, or settings across sessions,
which blocks most game genres outright.

Investigating where a save API would plug in surfaced a pre-existing
security gap that must be closed first (or in the same change): the Lua
sandbox does not actually block filesystem/process access today, which
would make a "sanctioned" save API meaningless sitting next to an
unsanctioned one.

This design covers two changes:

1. Close the sandbox gap (prerequisite).
2. Add a `dset`/`dget` + `save_data`/`load_data` Lua API for persistent
   save data.

Other gaps identified in the same audit (sprite flip/rotate/scale, input
coverage — button-release/analog/mouse, layered rendering) are explicitly
out of scope here; each is an independent subsystem and gets its own
design/spec later.

## Part 1 — Sandbox hardening

### Problem

`crates/caiven-vm/src/vm/lua_exec.rs:1044` constructs the cart's Lua state
with `Lua::new()`, mlua's default constructor. This loads
`StdLib::ALL_SAFE`, which — per mlua's own definition
(`mlua-0.10.5/src/stdlib.rs:79`, `ALL_SAFE = (1 << 30) - 1`) — excludes
only `DEBUG` and `FFI`. It does **not** exclude `IO` or `OS`. A cart's Lua
code today has real `io.open`, `os.execute`, `os.remove`, etc. in scope.

This directly contradicts `.claude/rules/security.md`: "a cart's Lua code
must not be able to reach the filesystem, network, or process outside the
sanctioned API surface."

There is a `STDLIB_NAMES` list (`lua_exec.rs:100-139`) that already
includes `"io"`, `"os"`, `"dofile"`, `"loadfile"`, `"load"`, `"require"` —
but it is only used to filter the Studio debugger's global-variable
display (`lua_globals()`, `lua_exec.rs:1341-1354`), not to remove those
names from the running state.

A second wrinkle: mlua's `StdLib` bitmask only governs library *tables*
(`io`, `os`, `package`, `string`, `math`, `table`, `coroutine`, `utf8`).
`dofile`, `loadfile`, `load`, and `require` are base-library globals that
reach the C filesystem directly and are **not** gated by the `IO` bit —
removing `IO` from the mask alone would leave `dofile("/etc/passwd")`
working.

### Fix

In `Vm::load_lua_source` (`lua_exec.rs:1043`):

1. Replace `Lua::new()` with
   `Lua::new_with(StdLib::COROUTINE | StdLib::TABLE | StdLib::STRING | StdLib::UTF8 | StdLib::MATH, LuaOptions::default())`.
   This excludes `IO`, `OS`, `PACKAGE` (already excludes `DEBUG`/`FFI` as
   before).
2. After `register_builtins` runs, explicitly remove the base-library
   globals not covered by the mask:
   `globals.set("dofile", Nil)`, `globals.set("loadfile", Nil)`,
   `globals.set("load", Nil)`, `globals.set("require", Nil)`.
3. `STDLIB_NAMES` stays as-is (still correct for debugger filtering — those
   names being absent from globals now is consistent with hiding them).

### Compatibility

Grepped `prelude.lua` and everything under `carts/` for `os.` / `io.`
usage: zero hits. `real_time()`/`time()`/`frame_count()` builtins already
provide the RTC/clock functionality carts need without `os`. No existing
cart or prelude code depends on `io`, `os`, `package`, `dofile`,
`loadfile`, `load`, or `require`. This is a pure hole-closing change with
no behavior change for any cart that stays within the sanctioned API.

### Tests

- `crates/caiven-vm/tests/`: assert `io`, `os`, `package`, `dofile`,
  `loadfile`, `load`, `require` are all `nil` in a loaded cart's globals.
- Assert a cart calling any of the above raises a Lua error (attempt to
  call a nil value) rather than succeeding.
- Existing test suite (prelude, tweens, particles, collision, etc.) must
  still pass unmodified — proves no accidental removal of sanctioned
  surface.

## Part 2 — Persistent save-data API

### API shape

**Slots** — fixed-size numeric storage, for counters/flags/high-scores:

```
dset(slot, value)   -- slot: integer 0-63, value: Lua number
dget(slot)          -- returns number, 0 if never set
```

- `slot` outside `0..63` → Lua error (`"dset: slot out of range (0-63)"`),
  matching the existing strict-bounds convention of `set_tile`/`set_pixel`.
- `value` not coercible to a Lua number → Lua error.
- 64 slots mirrors PICO-8's `dset`/`dget` precedent and fits comfortably
  next to the console's existing small fixed-size regions (16-color
  palette, 64×64 collision grid) in `docs/api-reference.md`'s memory map
  philosophy.

**Blob** — structured save data, for settings/progress objects:

```
save_data(table)    -- table: string/number keys, string/number/bool/
                     --   nested-table values only
load_data()          -- returns previously saved table, or {} if none
```

- Serialized to JSON internally (`serde_json`, already a workspace
  dependency via other crates — confirm at implementation time).
- Packed size capped at **4 KiB**. Over cap → Lua error naming the actual
  size vs. the cap, not silent truncation.
- A value that can't serialize (function, userdata, thread) → Lua error
  naming the offending key.
- `load_data()` before any `save_data()` call → returns `{}`, not an
  error — matches `load_sprite_bank`'s "returns false when missing"
  precedent of failing soft on absent optional data. (Blob absence is a
  normal first-run state, not a fault.)

### Storage

Mirrors the existing (Lua-inaccessible) save-state precedent in
`crates/caiven-machine/src/shell/save_state.rs`:

- Machine: `<exe_dir>/saves/<cart_id>.cavdata`, where `cart_id()` is the
  same sanitized-filename-stem function already used for
  `<cart_id>.cavstate` (`library.rs:156-167` — rejects empty/`.`/`..`/
  path-separator/NUL stems).
- Studio: a new Tauri command in `cart_io.rs`-style, scoped to a path
  derived from the open project — not a general-purpose `fs` capability
  exposed to the frontend (Studio currently grants no `fs:*` capability at
  all; this stays true).
- Format: `.cavdata` file = slots array (64 × f64, fixed) + length-prefixed
  JSON blob, magic-tagged and versioned the same way `.cavstate` is
  (`save_state.rs:33-42`) — decode defensively, never panic on truncated/
  corrupt input (same requirement as cart parsing in
  `.claude/rules/cart-format.md`).
- Both `dset`/`save_data` write through host Rust functions registered in
  `register_builtins`, the same pattern as `load_sprite_bank` — never
  through Lua's `io`, consistent with Part 1.
- Write timing: write-through on every `dset`/`save_data` call (simplest,
  correct-by-construction; revisit only if profiling shows it's hot enough
  to matter — `dset`/`save_data` are not expected to be called from
  `_update()`/`_draw()` every frame the way drawing calls are).

### Compatibility

Net-new globals (`dset`, `dget`, `save_data`, `load_data`). No existing
cart references these names (checked against `carts/` and
`prelude.lua`) — zero behavior change for any existing cart.

### Tests

`crates/caiven-vm/tests/`:

- `dset`/`dget` round-trip, default-0 on unset slot, out-of-range slot
  errors both directions.
- `save_data`/`load_data` round-trip with nested table, empty-table
  default on first load, oversized-table error, unserializable-value
  error naming the key.
- Persistence across a simulated reload (`load_lua_source` called twice
  against the same `cart_id`).

### Documentation & tooling

- `docs/api-reference.md`: new "Persistent Data" section next to
  "Audio"/"System", following the existing table format.
- `crates/caiven-vm/src/vm/api_registry.rs`: entries for `dset`, `dget`,
  `save_data`, `load_data` kept in sync with `lua_exec.rs` registration
  (per `.claude/rules/lua-api.md`).
- Studio autocomplete/codemirror Lua definitions
  (`crates/caiven-studio-ui`) updated for the four new globals.
- Example: extend `games/carts/stdlib_demo.cav` (or a new small cart under
  `carts/`) with a persisted high-score counter using `dset`/`dget`.

## Out of scope (future, separate specs)

- Sprite flip/rotate/scale, multi-cell blit.
- Input coverage: button-release events, analog stick, mouse/touch.
- Layered rendering / z-order, multiple map banks in RAM simultaneously.
- Cloud/cross-device sync of save data (local file only, for now).
