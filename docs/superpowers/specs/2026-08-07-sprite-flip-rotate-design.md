# Sprite flip + 90°-step rotation

Status: approved, pending implementation plan.

## Context

`caiven-lua-api` audit for "make the API viable for building any type of
game" (see `2026-08-07-lua-sandbox-and-save-data-design.md`) identified four
independent gaps: sprite orientation, input completeness, collision/many-
sprite helpers, audio channels. Each gets its own design/spec. This one
covers sprite orientation — the first, since rendering is a primitive other
systems (animation, particles) already build on.

`sprite(id, x, y)` (`crates/caiven-vm/src/vm/lua_exec.rs:600-621`) always
draws a sprite in its stored orientation. There is no way to face a
character left vs. right, or orient a bullet/ship in a top-down shooter,
without maintaining mirrored duplicate sprites in the sprite bank. Sprites
are fixed at 8×8 (per user constraint — no resize/scale in scope), so
arbitrary-angle rotation is out; flip + 90° steps stays pixel-perfect and
covers 4/8-directional facing.

Draw order was also raised in the audit but turned out to be a non-gap:
drawing is already immediate-mode (call order = draw order) onto `world` or
`ui` layers, matching PICO-8/TIC-80 convention. No change needed there.

## Signature

```lua
sprite(id, x, y, flip_x, flip_y, rotate)
```

- `flip_x`, `flip_y`: boolean, default `false` (nil-safe) — mirror
  horizontally / vertically.
- `rotate`: number, default `0` — degrees clockwise, must be one of
  `0, 90, 180, 270`. Any other value is a Lua error (an argument error, not
  a silent no-op or clamp — a typo'd rotation value should fail loudly
  rather than draw something the cart author didn't ask for).
- Transform order is fixed: rotate first, then flip. Documented explicitly
  so combining both is unambiguous.
- All three new params are optional and trail the existing `(id, x, y)`
  triple, so every existing 3-arg call site keeps its current behavior
  byte-for-byte. Non-breaking.

## Implementation

Extends the existing per-pixel loop in the `sprite` closure
(`lua_exec.rs:600-621`) rather than adding a parallel code path: for each
source pixel `(sx, sy)` in the `ss × ss` sprite, compute a transformed
coordinate via the rotate/flip mapping before the existing `plot()` call.
Pure integer coordinate math on values already in registers — no new
allocation, so no frame-loop cost concern per `.claude/rules/vm-runtime.md`.

## Testing

VM-level tests in `crates/caiven-vm/tests/`:

- No optional args → pixel-identical to current output (regression guard
  for the non-breaking claim).
- Each of `flip_x`, `flip_y`, and both together.
- Each rotation value (`90`, `180`, `270`) alone, and combined with a flip.
- Invalid rotation value (e.g. `45`) → asserts a Lua error, not a silent
  fallback.

## Documentation

- `docs/api-reference.md` — update the `sprite(id, x, y)` row under
  Sprites & Map.
- `crates/caiven-vm/src/vm/api_registry.rs` — keep in sync with the new
  signature (feeds Studio autocomplete/hover).
- `crates/caiven-studio-ui` codemirror Lua definitions — update so the
  editor doesn't drift from the runtime.
- Example: extend `games/carts/stdlib_demo.cav` (or add a new example) to
  show a character flipping to face left/right and/or a bullet rotating
  through the four cardinal directions.

## Compatibility

Additive only — no existing cart calls `sprite` with more than 3 args
today, so no cart's behavior changes. Not a breaking change.

## Out of scope (future specs)

- Input completeness (`button_released`, diagonal helpers).
- Collision & many-sprite helpers (circle/point tests, pooling).
- Audio channels (stoppable/queryable SFX handles).
