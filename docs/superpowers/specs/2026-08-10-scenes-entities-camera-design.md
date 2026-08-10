# Scenes, Entities, Camera-follow

Status: approved, pending implementation plan.

## Context

Follow-up to `2026-08-10-vec2-rng-collision-sprite-design.md`'s "Out of
scope" list, which explicitly deferred a game-object/architecture layer.
Re-auditing `carts/` shows every example is a single-screen arcade/platformer
(`stdlib_demo.cav`, `tiles.cav`, `movement.cav`, `catch.cav`) — nothing
demonstrates a title screen, pause menu, game-over state, or more than a
handful of hand-tracked entities. That's the biggest structural gap between
"can draw a game" and "can build any type of game": no cart-level convention
for scene/state transitions or entity lifecycle, and camera is a raw
`set_camera(x, y)` position set with no follow/shake convenience.

All three additions are pure Lua in `prelude.lua`, mirrored in
`api_registry.rs`'s `PRELUDE` table — same tier as `Particles`/`Vec2`/`Sprite`.
No new Rust builtins; `Camera` is a wrapper over the existing `set_camera`
builtin. No cart in `carts/` uses any of the new names today, so this is
purely additive.

Naming convention: namespaced modules (`Scenes`, `Entities`, `Camera`) since
all three hold state across frames, consistent with `Particles`/`Sprite`.
Considered also exposing a parallel procedural form (`entities_add(e)`
alongside `Entities.add(e)`); rejected — it would double the
implementation/test/doc/autocomplete surface required per name by
`.claude/rules/lua-api.md` for the same behavior under two names, with no
capability gained. Kept the single existing convention instead.

## Scenes

Stack-based, so a pause menu can sit on top of an active game without
tearing it down.

```lua
Scenes.push(scene)     -- calls scene.enter(scene) if present, pushes on top
Scenes.pop()           -- calls current top's exit(scene) if present, removes it
Scenes.switch(scene)   -- pop() current top, push(scene) — replace in place
Scenes.update()        -- calls top.update(top) if present
Scenes.draw()          -- calls top.draw(top) if present
Scenes.current()       -- returns top scene table, or nil if stack empty
```

`scene` is a plain table; `enter`/`exit`/`update`/`draw` are all optional
(missing ones are a no-op, not an error — mirrors how `Particles` entries
tolerate missing fields).

Error semantics: `pop()`/`switch()` on an empty stack raise a Lua error
(`error("Scenes: pop on empty stack")`) — that's a cart logic bug, not a
state to swallow silently. `update()`/`draw()` on an empty stack are a no-op
(a cart before its first `push()` shouldn't crash).

## Entities

Flat list of plain tables, matching `Particles`' shape (`spawn`/`update`/
`draw`/`clear`/`count`) rather than an ECS — this is a scripting API for
small carts, not a general engine, and the existing `Particles` precedent
already established this idiom.

```lua
Entities.add(e)         -- e = table; e.update(e), e.draw(e) optional; e.dead optional
Entities.update_all()   -- calls e.update(e) per live entity, then sweeps e.dead == true
Entities.draw_all()     -- calls e.draw(e) per live entity, in add order (z-ordering)
Entities.clear()
Entities.count()
Entities.new()          -- returns an independent list with the same add/update_all/
                         -- draw_all/clear/count methods, for carts that want one list
                         -- per scene instead of the shared default list
```

Sweep-on-dead reuses the list in place (swap-remove or compact-in-place),
not a fresh table per frame — per-frame allocation in `update_all()` is a
flagged pattern (`.claude/rules/vm-runtime.md`).

Error semantics: `add()` on a non-table argument raises a Lua error (matches
`table.insert`-style Lua idiom for type mismatches). Missing `update`/`draw`
fields on an individual entity are a silent no-op for that entity, not an
error — mirrors `Particles`.

## Camera

Wraps the existing `set_camera(x, y)` builtin (`lua_exec.rs`, unsigned
`u32` coordinates) with follow/shake convenience:

```lua
Camera.follow(entity, opts)   -- opts = { lerp = 0.1, deadzone_x = 0, deadzone_y = 0 }, all optional
Camera.unfollow()
Camera.shake(amount, duration)  -- duration in frames, linear decay to 0
Camera.update()                 -- reads followed entity position, applies lerp + deadzone,
                                 -- adds any active shake offset, clamps to >= 0, calls set_camera
```

`entity` must expose either `.pos` (a `Vec2`) or `.x`/`.y` numeric fields.

Error semantics: `follow(entity, ...)` raises a Lua error immediately if
`entity` has neither `.pos` nor `.x`/`.y` — fail at the call site, not as a
mysterious "camera never moves" days later. `Camera.update()` before any
`follow()` call is a no-op (camera position unchanged), mirroring `Scenes`'
empty-stack tolerance. Because `set_camera` takes unsigned `u32`, any
computed position (follow target + shake jitter) below zero is clamped to
`0` before the call — documented behavior, not a silent bug, and worth
flagging as a real (if minor) behavior constraint: shake near the world
origin will visibly clip rather than go negative.

## Data flow

None of the three modules auto-wire into each other — a scene's own
`update`/`draw` functions call `Entities`/`Camera` explicitly, same as
today's carts call `Particles.update()`/`.draw()` manually. This keeps each
module independently usable (a cart can use `Entities` alone with no
`Scenes` at all, exactly as `Particles` is used standalone today).

```
_update()
  Scenes.update()   -- top scene's update(scene) typically calls Entities.update_all()
  Camera.update()   -- reads latest entity positions, applies follow/shake, calls set_camera

_draw()
  Scenes.draw()     -- top scene's draw(scene) typically calls Entities.draw_all()
```

## Testing

VM-level tests in `crates/caiven-vm/tests/`:

- Scenes: push/pop/switch call order (`enter`/`exit` fire correctly),
  `current()` reflects top of stack, empty-stack `update()`/`draw()` are
  no-ops, empty-stack `pop()`/`switch()` raise a Lua error.
- Entities: `add` + `update_all` sweep-on-dead removes only dead entities
  and preserves order of survivors, `draw_all` order matches add order,
  `Entities.new()` produces an isolated list independent of the default
  global list, `add()` on a non-table raises a Lua error.
- Camera: `follow` + `update` converges toward target position over
  multiple frames per the `lerp` factor, `shake` decays to zero by the end
  of `duration` frames, `follow(entity, ...)` raises a Lua error when the
  entity has neither `.pos` nor `.x`/`.y`, a computed negative position
  clamps to `0` rather than wrapping/erroring.

## Documentation

- `docs/api-reference.md` — new "Scenes", "Entities", "Camera" subsections
  under "Gameplay stdlib".
- `crates/caiven-vm/src/vm/api_registry.rs` — `PRELUDE` entries for every
  new name.
- `crates/caiven-studio-ui` codemirror Lua definitions kept in sync.
- Example cart: a small title → play → game-over state machine using
  `Scenes`, a couple of moving `Entities`, and `Camera.follow` tracking the
  player — since no existing cart demonstrates multi-scene structure.

## Compatibility

Additive only. `Scenes`, `Entities`, `Camera` are new global names with no
prior use in any in-tree cart; `set_camera`'s own behavior and signature are
unchanged, `Camera` only calls it internally.

## Out of scope (future specs)

- ECS / component-system architecture — rejected in favor of the simpler
  `Particles`-style list, per this design's naming-convention discussion.
- Procedural-style aliases for module functions (`entities_add` etc.) —
  considered and rejected, see Context.
- Timers/delayed-callback scheduling.
- Tilemap authoring helpers (layers, autotile).
- Sound sequencing/volume/pan control.
- Networking.
- Structured/named save slots (current `dset`/`dget`/`save_data`/
  `load_data` unchanged).
