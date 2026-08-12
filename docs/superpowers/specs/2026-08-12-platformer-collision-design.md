# Platformer collision: one-way platforms, slopes, resolve helper, entity queries

Status: approved, pending implementation plan.

## Context

`caiven-lua-api` audit for "extend the API to make it really great for
building any type of game" started broad but narrowed to a concrete,
already-hit blocker: building a platformer against the current API is
painful because collision is too primitive. Today a cart gets:

- `tile_solid(tx, ty)` / `box_touches_solid(x, y, w, h)` — binary
  solid/not-solid tests only.
- `aabb_overlap` / `circle_overlap` / `point_in_rect` / `point_in_circle` —
  pairwise shape tests, no broad-phase query against the `Entities` list.
- No overlap-resolution helper — every cart re-derives its own "push the
  player out of the wall" logic from scratch.
- No one-way platforms and no slopes — both staples of platformer level
  design.

This design covers four changes, all scoped to the collision system:

1. Two new collision-type shapes: one-way platforms and slopes (both
   directions).
2. A pure-Lua swept move/resolve helper (`move_and_collide`) built on top
   of the existing tile collision primitives plus the two new shapes.
3. A broad-phase entity overlap query (`Entities.overlapping`).
4. Studio collision-type editor support for the two new shapes.

Other API gaps an "any type of game" audit could raise (bigger sprites,
UI/text/menu helpers, more input coverage) are explicitly out of scope —
each is an independent subsystem and gets its own design later if pursued.

## Part 1 — Collision-type shapes (`caiven-core`)

### Current representation

`CollisionType` (`crates/caiven-core/src/collision.rs`) carries a `flags:
CollisionTypeFlags` field, a thin wrapper over a `u8` bitset. Only `SOLID`
(`0b0000_0001`) is defined today. The type's own doc comment already states
the intent: "the representation is a plain `u8` so new bits can be added
later without changing the on-disk format (unknown bits round-trip
untouched)." Adding bits is therefore additive and non-breaking by design.

Three built-in ids exist (`walkable`=0, `solid`=1, `hazard`=2); ids 3-255
are free for cart-defined custom types (already true today).

### Change

Add three new bits to `CollisionTypeFlags`:

```rust
pub const ONE_WAY: u8 = 0b0000_0010;
pub const SLOPE_LEFT: u8 = 0b0000_0100;
pub const SLOPE_RIGHT: u8 = 0b0000_1000;
```

Add accessors `is_one_way()`, `is_slope_left()`, `is_slope_right()`,
mirroring the existing `is_solid()`.

**Convention (documented in the `CollisionTypeFlags` doc comment):** a
collision type is flat-solid, one-way, slope-left, or slope-right —
mutually exclusive. Nothing in `caiven-core` enforces exclusivity (matches
the existing "unknown bits round-trip untouched" philosophy — no
validation layer for flag combinations); `move_and_collide` (Part 2) reads
flags in a fixed priority order (solid, then one-way, then slope) so a
misconfigured type degrades predictably rather than erroring.

No new built-in ids are added — cart authors assign these shapes to
whichever custom id (3-255) they already use, so no existing cart's tile
meanings change. This is purely additive to `caiven-core`.

**Slope geometry** (used by `move_and_collide`, Part 2): within one tile of
size `SPRITE_SIZE` (`ss`), with `lx` the horizontal pixel offset into the
tile (`0..ss-1`):

- `SLOPE_RIGHT` (floor rises left→right — walking right goes uphill):
  `floor_y_in_tile = ss - 1 - lx`
- `SLOPE_LEFT` (mirror — walking left goes uphill):
  `floor_y_in_tile = lx`

## Part 2 — New builtins (`api_registry.rs` + `lua_exec.rs`)

Add three Lua builtins with the same call/error shape as the existing
`collision_is_solid(id)` (Lua error on non-integer/out-of-range `id`
consistent with current behavior, `false` for an id with no defined type):

- `collision_is_one_way(id)`
- `collision_is_slope_left(id)`
- `collision_is_slope_right(id)`

Registered in both `api_registry.rs` (autocomplete/hover/highlighting) and
`lua_exec.rs::register_builtins` (actual binding) per the required-sync
rule for this API surface. Existing `api_registry.rs`-vs-`lua_exec.rs`
drift test extends to cover these three names.

## Part 3 — `move_and_collide(x, y, w, h, dx, dy)` (`prelude/collision.lua`)

New function in the existing opt-in `collision` prelude module (pure Lua,
no Rust runtime cost beyond the three new builtins above, which are only
called when a slope/one-way tile is actually touched). Built entirely on
existing/new tile-query primitives — no new Rust collision-resolution
code.

**Behavior**, axis-separated (horizontal resolved first, then vertical, the
standard tile-platformer order so a corner case does not misresolve both
axes as one diagonal push):

1. **Horizontal (`dx`)**: move the box by `dx`, then test against `SOLID`
   tiles only along the new leading edge column(s); clamp the box to the
   solid boundary if it penetrates. Slopes and one-way tiles never block
   horizontal movement (walking under a ramp or through the side of a
   one-way platform must work).
2. **Vertical (`dy`)**: move the (already horizontally resolved) box by
   `dy`, then, in priority order:
   - `SOLID` tiles: clamp against the boundary in the direction of travel
     (blocks both upward and downward movement — walls/ceilings/floors).
   - `ONE_WAY` tiles: only considered when `dy > 0` (descending) **and**
     the box's pre-move bottom edge was at or above the tile's top edge
     (so you can jump up through it and only land when arriving from
     above). Clamps the box's bottom to the tile top.
   - `SLOPE_LEFT`/`SLOPE_RIGHT` tiles: sample floor height (Part 1
     geometry) at each pixel column the box's horizontal span covers,
     take the highest (smallest-y) floor point, and clamp the box's
     bottom there — but only when moving down onto it or already
     resting on it (never pushes the box down when it's above the slope
     and still falling past the point, matching normal floor behavior).
3. Returns `nx, ny, touch` where `touch = { ground = bool, ceiling = bool,
   left = bool, right = bool }` reporting which sides were blocked this
   call.

**Error semantics:** consistent with the rest of the `collision` module —
no argument validation beyond what Lua itself provides (numbers required
by arithmetic use); a `nil`/non-number argument is a regular Lua runtime
error, not a silent no-op.

## Part 4 — `Entities.overlapping(x, y, w, h)` (`prelude/entities.lua`)

New method on the entity-list object (both the shared `Entities` global and
any list from `Entities.new()`). Iterates the list and returns entries
whose `.pos` (a `Vec2`, per the existing `vec2` module convention already
used in every example) and `.w`/`.h` fields overlap the given box via the
existing `aabb_overlap`. Entities missing `.pos`/`.w`/`.h` are silently
skipped — the entity table shape is intentionally caller-defined
everywhere else in this module (`update`/`draw`/`dead` are the only
conventions today), so a missing field here is "this entity doesn't
participate in box queries," not an error.

No new Rust code. `entities` module already has no declared dependency on
`vec2` in the manifest system — using `.pos` as a `Vec2` is a convention,
not an enforced dependency, same as today.

## Part 5 — Studio collision-type editor

Currently `solid: bool` flows end-to-end as a single checkbox:

- `crates/caiven-core` → `CollisionTypeFlags`
- `crates/caiven-studio/src/tauri_app.rs::CollisionTypePayload` (`solid:
  bool`, `From`/`From` conversions against `caiven_core::CollisionType`)
- `crates/caiven-studio-ui/src/types.ts::CollisionType` (`solid: boolean`)
- `crates/caiven-studio-ui/src/lib/ipc.ts` (built-in type seed data)
- `crates/caiven-studio-ui/src/components/Workspace.svelte` (checkbox UI,
  `updateCollisionType`)

Change `solid: bool` to a `shape` field across this whole chain:

```ts
type CollisionShape = 'none' | 'solid' | 'one_way' | 'slope_left' | 'slope_right';
```

with the payload/type/UI updated in lockstep. The checkbox becomes a
5-option radio group (reflecting the Part 1 mutual-exclusivity
convention — the editor UI enforces what `caiven-core` leaves
unenforced). Built-in seed rows (`walkable`/`solid`/`hazard`) map to
`none`/`solid`/`none` respectively, unchanged from today's behavior.

This changes an internal IPC payload shape, not cart data or the public
Lua API — no compatibility note needed for existing carts (their persisted
collision-type tables still decode the same way; only the editor's
in-memory/IPC representation changes).

## Testing

- `caiven-core/src/collision.rs`: unit tests for the three new flag
  accessors (present/absent, unknown-bit round-trip already covered by the
  existing test).
- `crates/caiven-vm/tests/lua_script.rs`: new tests following the existing
  `prelude_tile_solid_and_box_touches_solid` /
  `custom_solid_collision_type_is_respected_by_tile_solid` pattern for:
  - `collision_is_one_way` / `collision_is_slope_left` /
    `collision_is_slope_right` against a custom type.
  - `move_and_collide`: flat solid ground (ground=true, position clamped),
    one-way platform landed on from above, one-way platform passed through
    from below/side, wall block on both `left`/`right`, ceiling block,
    slope-left and slope-right height resolution at a few sample x
    offsets.
- `crates/caiven-vm/tests/lua_script.rs`: `Entities.overlapping` — returns
  matching entries, skips entities missing `.pos`/`.w`/`.h`.
- Existing `api_registry.rs`-vs-`lua_exec.rs` drift test extended to the
  three new builtin names (should already fail loudly if one side is
  missed — verify it catches these before considering Part 2 done).

## Documentation

- `docs/api-reference.md`: extend the `collision` module table with
  `move_and_collide`, and the System-level table with the three new
  `collision_is_*` builtins; document the shape mutual-exclusivity
  convention.
- Example: extend `carts/dev/stdlib_demo.cav` (or add a new
  `carts/dev/platformer_demo.cav`) to demonstrate a one-way platform, a
  slope, and `move_and_collide` driving player movement — per the
  `lua-api` rule requiring an example for any public Lua API change.

## Out of scope

- Slope shapes steeper/shallower than one tile's 45° diagonal (multi-tile
  ramps, quarter-pipes) — not requested, adds real complexity to the
  height-sampling formula.
- Circle/entity-vs-tile collision (only AABB-based `move_and_collide` is
  covered; `circle_overlap` remains a standalone pairwise test as today).
- Any non-collision API gap (sprite size, UI/text, input, audio) —
  separate designs.
