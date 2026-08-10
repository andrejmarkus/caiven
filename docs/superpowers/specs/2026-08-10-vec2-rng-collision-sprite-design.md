# Vec2, deterministic RNG, circle/point collision, Sprite wrapper

Status: approved, pending implementation plan.

## Context

`caiven-lua-api` audit for "make the API viable for building any type of
game" (see `2026-08-07-lua-sandbox-and-save-data-design.md`) tracked four
gaps: sprite orientation (done), input completeness, collision/many-sprite
helpers, audio channels. This spec adds a fifth, found while re-auditing the
gameplay stdlib (`crates/caiven-vm/src/vm/prelude.lua`) specifically for
math/physics: it has scalar-only helpers (`lerp`, `tween`, `aabb_overlap`,
tile collision) and no vector type, no exposed/documented RNG story, and no
circle or point collision — all needed for genres beyond a tile-grid
platformer (top-down, shooters, puzzle).

All four additions are pure Lua in `prelude.lua`, mirrored in
`crates/caiven-vm/src/vm/api_registry.rs`'s `PRELUDE` table. No Rust/host
changes, no new Tauri surface. Grep of `carts/` and `games/carts/` found no
existing use of any of the new names, and `math.random`/`math.randomseed`
are already-enabled Lua 5.4 stdlib (`StdLib::MATH`) but unused by any
in-tree cart today — so none of this changes behavior for an existing cart.

## Vec2

Metatable value type — the first in `prelude.lua` (existing helpers like
`Particles` are namespaces/systems, not value types, so this doesn't
conflict with that style; it's the natural idiom once you have a type with
operators).

```lua
Vec2.new(x, y)           -- v.x, v.y
v1 + v2, v1 - v2         -- component-wise
v * scalar, scalar * v   -- scale
-v                       -- unary negate
v1 == v2                 -- component equality
v:length()
v:length_squared()       -- avoids sqrt when only comparing magnitudes
v:normalize()            -- zero-vector input returns Vec2.new(0, 0), not a Lua error
v:dot(other)
v:distance(other)
tostring(v)              -- "(x, y)"
```

Error semantics: arithmetic operators require both operands to be a `Vec2`
(or, for `*`, one `Vec2` and one `number`) — mismatched types raise a Lua
error via the metamethod, not a silent coercion. `normalize()` on a
zero-length vector returns the zero vector rather than erroring or dividing
by zero, since a stray `normalize()` on a just-spawned zero-velocity entity
is a common, non-exceptional case in game code.

## RNG

Deterministic by default, opt-out via `math.randomseed`:

- `prelude.lua` seeds once per fresh VM load: a top-level guard —
  `if not RTK_SEEDED then math.randomseed(1) RTK_SEEDED = true end`. Fresh
  `Lua::new_with` state (full load, `load_lua_source`) has `RTK_SEEDED` nil,
  so it seeds. Hot reload (`hot_reload_lua_source`) reuses the same live
  `Lua` state and re-runs `prelude.lua`, but `RTK_SEEDED` is already `true`
  from the first run, so a hot reload during dev does not reset the live
  RNG stream mid-game.
- Cart calls `math.randomseed(x)` directly to reseed — e.g.
  `math.randomseed(os.time())` for per-run variety, or a fixed value for a
  reproducible replay/test.
- Convenience helpers on top of `math.random`:
  - `random_range(lo, hi)` — integer, inclusive both ends.
  - `random_float(lo, hi)` — float in `[lo, hi)`.
  - `choice(t)` — random element of a non-empty array-like table; empty
    table is a Lua error (an empty-table random pick has no sensible
    result, so fail loudly rather than return `nil` silently).
  - `shuffle(t)` — Fisher-Yates, in place, returns `t`.

## Collision

Extends the existing `aabb_overlap(x1, y1, w1, h1, x2, y2, w2, h2)`:

```lua
circle_overlap(x1, y1, r1, x2, y2, r2)
point_in_rect(px, py, x, y, w, h)
point_in_circle(px, py, cx, cy, r)
```

Plain booleans, same style as `aabb_overlap` — no new error cases, all
arguments are plain numbers.

## Sprite wrapper

Thin, plain-table-plus-methods (matches `Particles` style — no metatable,
since no operators apply to a sprite):

```lua
Sprite.new{ sprite_id = 3, pos = Vec2.new(10, 20), flip_x = false, flip_y = false, rotate = 0 }
s:draw()   -- sprite(s.sprite_id, s.pos.x, s.pos.y, s.flip_x, s.flip_y, s.rotate)
```

Deliberately excludes `:update()`/velocity/lifecycle — that's a game
architecture decision (ECS vs. OOP vs. plain tables) that varies by genre
and belongs in cart-side Lua, not the console API (see "Out of scope").
Move by mutating `s.pos` directly, e.g. `s.pos = s.pos + v`.

## Testing

VM-level tests in `crates/caiven-vm/tests/`:

- Vec2: each operator (`+`, `-`, `*` both operand orders, unary `-`, `==`),
  `length`/`length_squared`/`dot`/`distance` against known values,
  `normalize()` on a non-zero and on a zero vector, mismatched-type
  operator use raises a Lua error.
- RNG: fresh load seeds deterministically (same `random_range` sequence
  across two fresh loads with no explicit seed call); hot reload does not
  reset the sequence (call `random_range` before reload, reload, call
  again, assert continuation not repeat-from-seed); `choice` on empty table
  errors; `shuffle` preserves multiset of elements.
- Collision: `circle_overlap` true/false cases including tangent boundary,
  `point_in_rect`/`point_in_circle` inside/outside/boundary cases.
- Sprite: `:draw()` calls through to `sprite()` with the wrapper's fields
  (can assert via the existing sprite-draw test harness).

## Documentation

- `docs/api-reference.md` — new rows under "Gameplay stdlib" for Vec2, RNG
  helpers, the three collision functions, and `Sprite`.
- `crates/caiven-vm/src/vm/api_registry.rs` — `PRELUDE` entries for every
  new name (autocomplete/hover; also feeds Studio's syntax highlighter).
- `crates/caiven-studio-ui` codemirror Lua definitions — keep in sync so
  the editor doesn't drift from the runtime.
- Example: extend `games/carts/stdlib_demo.cav` (or add a new example cart)
  demonstrating Vec2 movement, a `random_range`-driven spawn, a
  `circle_overlap` collision check, and a `Sprite`-wrapped entity.

## Compatibility

Additive only. `Vec2`, `Sprite`, `circle_overlap`, `point_in_rect`,
`point_in_circle`, `random_range`, `random_float`, `choice`, `shuffle`, and
`RTK_SEEDED` are all new global names with no prior use in any in-tree
cart. The one behavior change is that `math.random()` now produces a
deterministic sequence by default instead of Lua's own startup-entropy
seed — since no in-tree cart calls `math.random`/`math.randomseed` today,
this changes no existing cart's observed behavior, but it is a real
semantic change worth flagging: any cart written *after* this ships that
doesn't explicitly reseed will get the same random sequence on every fresh
run.

## Out of scope (future specs)

- Full entity/game-object system (velocity integration, lifecycle,
  collision-box-attached-to-entity) — a game-architecture choice that
  varies by genre; stays cart-side Lua, not console API.
- Input completeness (`button_released`, diagonal helpers, mouse/pointer,
  second player).
- Audio channels (stoppable/queryable SFX handles, volume/pan).
- Many-sprite/spatial-partition collision helpers (pooling, broad-phase).
