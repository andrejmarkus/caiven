# Input/audio query completeness: button_released, is_sfx_playing, is_music_playing, RNG seed docs

Status: approved, pending implementation plan.

## Context

`caiven-lua-api` audit for "make the API viable for building any type of
game," scoped down to fantasy-console-appropriate simplicity (explicitly
excluding rotation/physics/vectors/coroutine-timers/JSON/mouse/UI widgets —
those either fight the console's identity or lack a concrete driving use
case yet). Full audit surfaced four candidate gaps; one (per-player button
args) was investigated and dropped from this pass because it needs new
cross-crate SDL gamepad plumbing (`caiven-machine`'s `Gamepads` explicitly
opens only the first controller today) — out of proportion to the other
three, which are each a small addition on top of state the engine already
tracks. This spec covers the three cheap ones.

## RNG seed (docs only, no code change)

`Lua::new_with` already enables `StdLib::MATH` in full
(`crates/caiven-vm/src/vm/lua_exec.rs:1524-1530`), so `math.randomseed(x)`
is already callable from any cart — it's real Lua 5.4 stdlib, not a
console-specific wrapper. The console seeds to `1` by default at load
(`crates/caiven-vm/src/vm/prelude/core.lua:6`) so runs are deterministic
unless a cart opts out by reseeding itself. This already works; it's just
undocumented, so nobody discovers it via autocomplete/hover.

Add a `math.randomseed` entry to `api_registry.rs`'s stdlib section
(alongside the existing `math.random`, `math.abs`, etc. entries) documenting:
default seed is `1`, call `math.randomseed(x)` to change it, no return
value. Update README API reference with the same note. No `lua_exec.rs`
change, no `BUILTIN_NAMES` change (it's stdlib, not a console builtin).

## `button_released(button_index)`

Mirrors `button_pressed` exactly, using state already tracked.
`crates/caiven-vm/src/input/input.rs`'s `Input` struct already latches
`cur`/`prev` frame arrays every frame (`end_frame`, line 32-34) and already
exposes one edge direction via `just_pressed` (`cur[i] && !prev[i]`, line
23-25).

Add `Input::just_released(button) -> bool` returning
`!cur[i] && prev[i]` — the mirror diff, no new state. Register
`button_released(button_index)` in `lua_exec.rs::register_builtins`
alongside the existing `button_down`/`button_pressed` closures, add to
`BUILTIN_NAMES` (required per `.claude/rules/lua-api.md` — omitting it
causes a `SIGABRT` on hot-reload, not a compile error), add an
`api_registry.rs` entry matching `button_pressed`'s doc shape.

Error semantics: same as `button_down`/`button_pressed` — an out-of-range
`button_index` returns `false`, not a Lua error (consistent with the two
existing functions it mirrors).

## `is_sfx_playing(handle)` / `is_music_playing()`

Both `SfxPlayer` and `MusicPlayer` (`crates/caiven-vm/src/vm/sfx.rs:35-41,
77-86`) already carry a public `active: bool`, set in `start()` and cleared
in `stop()`. The SFX voice pool (`crates/caiven-vm/src/vm/mod.rs`) already
decodes opaque handles via `unpack_sfx_handle` (slot, epoch) and the same
epoch-match pattern is already used by `stop_sfx_voice`/`release_sfx_voice`
to distinguish a still-live handle from one whose voice was since reused by
another `play_sfx` call.

- `is_music_playing()` — reads `self.music_player.active` directly. No new
  state.
- `is_sfx_playing(handle)` — decodes `handle` via the existing
  `unpack_sfx_handle`, checks `pool[slot].epoch == epoch &&
  pool[slot].player.active`. A stale handle (voice finished naturally, or
  reused by a later `play_sfx` call — epoch mismatch) returns `false`, not
  an error — matches `stop_sfx_voice`'s existing silent-no-op-on-stale-handle
  behavior, so a cart tracking a handle across frames doesn't need to guard
  every call with a validity check first.

Register both in `lua_exec.rs::register_builtins`, add both to
`BUILTIN_NAMES`, add both to `api_registry.rs`.

## Testing

VM-level tests in `crates/caiven-vm/tests/`:

- `button_released` fires exactly on the frame after release, not the frame
  the button goes up-to-down, and not on subsequent held-up frames; matches
  `button_pressed`'s existing edge-trigger test shape but for the opposite
  edge.
- `button_released` on an out-of-range index returns `false`.
- `is_sfx_playing(handle)` is `true` immediately after `play_sfx`, `false`
  after the sfx naturally finishes, and `false` for a handle whose voice was
  since stolen by a later `play_sfx` call (epoch mismatch) — reuses the
  existing voice-pool test fixtures from the audio spec
  (`2026-08-12-audio-polyphony-pan-envelope-design.md`).
- `is_music_playing()` is `true` after `play_music`, `false` after
  `stop_music` or after a non-looping track finishes.
- `math.randomseed` doc entry exists in `api_registry.rs` and appears in
  `all_names()` (existing autocomplete-sync test, if one exists for stdlib
  entries — otherwise a small addition to it).

## Documentation

- README API reference: `button_released`, `is_sfx_playing`,
  `is_music_playing`, `math.randomseed` entries.
- `crates/caiven-vm/src/vm/api_registry.rs` — all four entries (kept in
  sync per the file's own doc comment).
- `crates/caiven-studio-ui` codemirror Lua definitions — updated so the
  three new builtins get syntax highlighting/autocomplete.
- Example: extend an existing input-demo or add a short snippet under
  `projects/dev/` showing `button_released` (e.g. "charge attack releases
  on button-up") and `is_sfx_playing`/`is_music_playing` (e.g. "don't
  restart a looping hit sound while it's still playing").

## Compatibility

Purely additive: three new builtin names plus one new doc entry for an
already-existing stdlib function. No existing signature changes, no
behavior change for any cart that doesn't call the new functions.

## Out of scope (future specs, if a concrete need arises)

- Per-player button args / local multiplayer — needs new SDL gamepad
  enumeration and event routing across `caiven-vm` and `caiven-machine`;
  deliberately not bundled with these three cheap additions.
- Vectors, coroutine-based timers, JSON save data, string.split, mouse/
  touch input, UI/menu widgets — flagged in the broader audit, explicitly
  deferred as either fighting the fantasy-console simplicity goal or
  lacking a concrete driving use case.
