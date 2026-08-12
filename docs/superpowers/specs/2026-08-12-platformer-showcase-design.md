# Platformer showcase cart

Status: approved, pending implementation plan.

## Context

Caiven has no flagship content cart demonstrating the full stack together —
tile collision (`move_and_collide`, one-way platforms, slopes, hazards),
`entities`, `tween`, `particles`, audio, and persistent input across a
multi-room level. PICO-8 and TIC-80 both use a minimalist single-screen
precision platformer as their benchmark showcase for "can this console do a
real game." This design is a single new content
project — no engine/API changes — that plays the same role for Caiven.

Pure content work: `projects/showcase/platformer/` (`caiven.toml` +
`main.lua` + `sprites.png` + `map.png`), built to
`crates/caiven-studio/resources/examples/platformer.cav` via
`scripts/demo-carts/build.sh`. No changes to `caiven-core`, `caiven-vm`, or
the Lua API surface — everything needed already exists per
`docs/api-reference.md`.

## Scope

8 single-screen rooms (128×128px each = one full console screen, no
scrolling), arranged as a 4×2 grid within the one available 64×64 map bank
(each room = a fixed 16×16-tile slice). Room 1 = start, room 8 = end (flag).
Rooms connect left-to-right and via up/down passages, matching the classic
single-screen precision-platformer layout style. Mechanics demonstrated: run, coyote-time
jump, jump buffering, variable jump height, wall slide, wall jump, 8-dir
dash with ground/wall-refillable single stamina charge, hazards (instant
death + respawn), one-way platforms, a slope, strawberry collectibles, a
death counter, and a win screen.

Out of scope: multiple areas/chapters, crumbling platforms, moving
platforms, enemies, checkpoints mid-room (checkpoint = room entry only),
any new engine API.

## `caiven.toml`

```toml
[cart]
title = "platformer"
author = ""
entry = "main.lua"
entry_point = 0
flags = 0
version = 1

[mods]
require = []

[stdlib]
modules = [
    "vec2",
    "collision",
    "entities",
    "tween",
    "particles",
]
```

No `scenes` or `camera` modules — see State machine and Camera below for
why they're deliberately not used.

## State machine

Hand-rolled, not the `Scenes` stack module — the whole game is one
continuous piece of state (`GAME.mode`) with four values, not independent
push/pop screens with their own enter/exit lifecycles:

- `"title"` — logo + "press A", drawn over room 1's background.
- `"playing"` — normal gameplay update/draw.
- `"dying"` — short freeze + particle burst (~20 frames), then respawn at
  current room's entry point, `mode = "playing"`.
- `"won"` — reached the flag in room 8: shows deaths + strawberries
  collected / total, "press A to restart" resets all state and returns to
  `"title"`.

`Scenes` is a better fit for independently-composed screens (its own design
already covers that use case); here every mode shares the same player/room
state, so a flat `if/elseif` on `GAME.mode` inside one `_update`/`_draw` is
simpler and has no stack-management overhead to reason about.

## Rooms

```lua
ROOMS = {
  { spawn = Vec2.new(...), berry = {x=.., y=..} or nil, flag = false },
  ...
}
```

8 entries, index = room number. `current_room` (1-8) selects which 16×16
tile slice of the map bank is on screen; `set_camera((col)*128, (row)*128)`
where `col, row` derive from room layout (e.g. rooms 1-4 top row, 5-8 bottom
row, with a couple of vertical connectors breaking the straight line for
verticality). Room transition is instant on crossing a screen
edge (player x/y wraps to the opposite edge of the new room) — no smooth
scroll, matching the source material and avoiding continuous-scroll camera
work entirely.

Berries: at most one per room (8 total), each a table added via
`Entities.add` with `.pos`, `.w=8`, `.h=8`, `.collected=false`, drawn only
for the active room, removed from `Entities` (via `.dead=true`) on pickup
with a `particles` burst + collect SFX, tallied in `GAME.berries`.

## Camera

Direct `set_camera(x, y)` per current room, no smoothing, no `Camera`
module — a snap-cut is the intended feel for room transitions here (the
`camera` module's `lerp` follow is for continuous-scroll games; using it
would fight the snap-cut design rather than help it).

## Player physics

Constants tuned by playtest, starting point (60fps frame units, `px` =
pixels):

- Run: max speed ~1.2px/f, ground accel ~0.4, air accel ~0.3, friction
  ~0.3/f when no input.
- Gravity ~0.35px/f², terminal fall ~4.5px/f, cut fall speed on wall-slide
  to ~1px/f.
- Jump: initial vy ~-4.8, held-jump extends (cut vy toward 0 if A released
  while vy < 0, i.e. variable height), coyote time 6 frames after leaving
  ground, jump buffer 4 frames before landing.
- Wall jump: off a wall-slide, pushes away from wall + up, ~10-frame
  input-lock on horizontal control so it reads as a deliberate jump, not an
  instant re-stick.
- Dash: on B press with a charge available — direction from held d-pad
  (8-way; no direction held + facing left/right = horizontal dash in facing
  direction), fixed speed ~3.5px/f, duration ~10 frames, gravity disabled
  during the dash, `particles` trail spawned each dash frame. Charge
  (`player.dashes`, 0 or 1) refills to 1 on any ground or wall touch.
- All movement resolved via `move_and_collide(x, y, w, h, dx, dy)` from the
  existing `collision` module (already axis-separated, already handles
  solid/one-way/slope) — no custom collision code in this cart.

## Hazards and death

Spike tiles use the built-in `hazard` collision type (id 2,
`collision_type_id("hazard")`). Hazard detection is a direct
`get_collision` check under the player box each frame, not routed through
`move_and_collide`, since a hazard must trigger death on touch rather than
block movement. On touch: `mode = "dying"`, SFX, particle
burst at player position, freeze input; after the freeze timer, respawn at
`ROOMS[current_room].spawn`, `GAME.deaths += 1`, `mode = "playing"`.

## Audio

4 SFX slots (jump, dash, death, collect) authored directly as SFX-bank
steps (`play_sfx(id)`), one looping music track (`play_music(id)`, started
on entering `"playing"` from `"title"`, stopped on `"won"`).

## Art pipeline

No image editor available for this work — `sprites.png`/`map.png` are
generated by a small one-off Python (PIL) script that writes the exact
formats `crates/caiven-cart/src/asset_png.rs` decodes: a 128×128 8-bit
indexed PNG (16×16 grid of 8×8 sprites, PLTE = cart palette) and a 64×64
8-bit grayscale PNG (pixel value = tile/sprite id). The script is a
throwaway build tool, not part of the shipped project — it lives under
`scripts/demo-carts/` only if reused, otherwise in the scratchpad and
discarded once the PNGs are committed.

Palette (16 entries): sky blue, 2-3 dirt/ground browns, grass green accent,
player red/pink, strawberry red + green leaf, spike red + white highlight,
flag yellow/white, black outline, plus a couple of spares for particles/UI
text. Sprites needed: player idle/run frames (walk-cycle via `tween`'s
`new_anim`), ground/platform tile, one-way platform tile, slope tile,
spike, strawberry, flag, background dot/star deco.

## Testing / verification

No engine or Lua API changes, so no `caiven-vm` unit tests apply. Manual
verification only:

1. `scripts/demo-carts/build.sh` packs the project into the example `.cav`.
2. `cargo run -p caiven-machine -- crates/caiven-studio/resources/examples/platformer.cav`
   (or the built `carts/showcase/...` path, whichever `build.sh` targets)
   — play through all 8 rooms confirming: jump/coyote/buffer feel
   reasonable, wall slide+jump works both walls, dash refills on
   ground/wall touch and not mid-air, every hazard kills and respawns at
   the room's spawn point (not room 1), every berry is reachable and
   increments the counter exactly once, the flag in room 8 triggers the
   win screen with correct deaths/berries tally, and "press A" from both
   title and win screens behaves correctly.
3. `scripts/claude/check-lua-api.sh` and `scripts/claude/check-cart-compat.sh`
   as the narrowest relevant existing checks (content-only cart, but these
   confirm nothing about the stdlib/cart-format contract broke).

## Documentation

`docs/api-reference.md` already documents every function this cart uses —
no doc changes needed there. If `crates/caiven-studio/src/studio/examples.rs`
maintains a human-readable list/description of the Examples gallery
entries, add this cart's entry there (title + one-line description) —
confirm during implementation whether such a list exists.

## Out of scope

- Any new Lua builtin or stdlib module — pure content using the existing
  API surface end to end.
- Multiple chapters/areas, mid-room checkpoints, enemies, moving/crumbling
  platforms — not requested, adds scope beyond an 8-room showcase.
- Automated gameplay tests (no scripted-input test harness exists for
  carts today) — verification is manual playtest per above.
