# Celeste-clone Showcase Cart Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build `projects/showcase/celeste_clone/`, an 8-room Celeste-style
precision platformer that exercises the full Caiven Lua API/stdlib as a
flagship showcase cart, per
`docs/superpowers/specs/2026-08-12-celeste-clone-showcase-design.md`.

**Architecture:** Pure content — one project directory (`caiven.toml` +
`main.lua` + `collision_types.json` + `sprites.hex` + `sfx.hex` +
`music.hex`), no engine changes. `main.lua` holds one hand-rolled state
machine (`GAME.mode`) over a single continuous player/room simulation, built
up task-by-task: world data → physics → hazards/dash → collectibles →
win/HUD → build registration.

**Tech Stack:** Lua 5.4 (Caiven builtins + `vec2`/`collision`/`entities`/
`tween`/`particles` stdlib modules), Python 3 stdlib (`pathlib`, no PIL) for
one-off hex asset generation, `scripts/demo-carts/build.sh` for packing.

## Global Constraints

- Project lives at `projects/showcase/celeste_clone/`; builds to
  `crates/caiven-studio/resources/examples/celeste_clone.cav` via
  `scripts/demo-carts/build.sh` (spec).
- No new Lua builtin or stdlib module — every mechanic maps to an existing
  API in `docs/api-reference.md` (spec).
- `[stdlib] modules = ["vec2", "collision", "entities", "tween",
  "particles"]` in `caiven.toml` — no `scenes`, no `camera` (spec).
- No image editor available — all visual/audio assets are generated as
  `.hex` text (sprite sheet, SFX, music), not `.png`; project-dir format
  accepts `sprites.hex` as a drop-in alternative to `sprites.png`
  (`crates/caiven-cart/src/project.rs` doc comment) and map/palette need no
  asset file at all — both are built at runtime via `set_tile`/
  `set_collision`/`set_palette_color`, following the existing pattern in
  `projects/dev/tiles/main.lua` and `projects/dev/platformer_demo/main.lua`.
  This is a deliberate implementation refinement over the spec's original
  "generate sprites.png and map.png via PIL" plan — same "no editor needed"
  goal, fewer moving parts, follows established repo convention more
  closely.
- No automated gameplay tests exist for carts — every task's verification
  step is a manual `cargo run -p caiven-machine -- <path>` playtest checklist
  item, per the spec's Testing section. This replaces the pytest-style
  "run the failing test" steps from the plan template throughout.
- Collision ids follow `projects/dev/platformer_demo/collision_types.json`
  exactly: `0 walkable, 1 solid, 2 hazard` (built-in) plus `3 platform`
  (one_way), `4 ramp_right` (slope_right), `5 ramp_left` (slope_left).
- Sprite id `0` is reserved as an all-transparent blank tile (map cells
  default to `0` and are drawn via `draw_map`; a non-blank sprite at id 0
  would render garbage over every untouched "air" cell). All content
  sprites start at id `1`.
- Screen/room size is 128×128px = 16×16 tiles. World space is absolute
  pixel/tile coordinates across the whole 64×32-tile region used (4 room
  columns × 2 room rows within the one 64×64 map bank); player position,
  camera, and room lookups all work in these same absolute coordinates —
  camera is `set_camera(floor(player.x/128)*128, floor(player.y/128)*128)`
  recomputed every frame, giving an instant snap-cut on room boundary
  crossing with no manual "wrap" logic (a simplification over the spec's
  literal wraparound description; same snap-cut feel, less code).

---

## World layout reference

Room grid (col, row), each 16×16 tiles, tile origin `(col*16, row*16)`:

```
row 0:  [1: col0] [2: col1] [3: col2] [4: col3]
row 1:  [5: col0] [6: col1] [7: col2] [8: col3]
```

Room 4 (top-right) has an open floor on its right half; falling through
drops into room 5 (bottom-left) below it — the vertical connector. All other
transitions are horizontal (walking off one room's right edge onto the
next room's left edge, floor heights matched at the shared border).

All room content (floor/platform/hazard/slope rects, spawn point, berry
position, flag) is defined once in Task 3 and consumed unchanged by every
later task — no room data is redefined or restructured after Task 3.

---

### Task 1: Project scaffold

**Files:**
- Create: `projects/showcase/celeste_clone/caiven.toml`
- Create: `projects/showcase/celeste_clone/collision_types.json`
- Create: `projects/showcase/celeste_clone/main.lua`

**Interfaces:**
- Produces: a bootable cart with empty `_init`/`_update`/`_draw`, the
  `[stdlib]` module set, and the 6-entry collision type table every later
  task relies on.

- [ ] **Step 1: Write `caiven.toml`**

```toml
[cart]
title = "celeste_clone"
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

- [ ] **Step 2: Write `collision_types.json`**

```json
[
  {"id": 0, "name": "walkable", "color": [0, 0, 0], "shape": "none"},
  {"id": 1, "name": "solid", "color": [255, 176, 0], "shape": "solid"},
  {"id": 2, "name": "hazard", "color": [224, 32, 32], "shape": "none"},
  {"id": 3, "name": "platform", "color": [0, 200, 0], "shape": "one_way"},
  {"id": 4, "name": "ramp_right", "color": [0, 200, 200], "shape": "slope_right"},
  {"id": 5, "name": "ramp_left", "color": [200, 200, 0], "shape": "slope_left"}
]
```

- [ ] **Step 3: Write a stub `main.lua`**

```lua
function _init()
  set_palette_color(0, 100, 160, 230) -- sky blue, placeholder until Task 3
end

function _update()
end

function _draw()
  clear_screen()
  draw_text("CELESTE CLONE - BOOT OK", 4, 60, 7)
end
```

- [ ] **Step 4: Build and boot it**

Run:
```bash
cargo build -p caiven-studio
target/debug/caiven-studio build projects/showcase/celeste_clone --out /tmp/celeste_clone.cav
cargo run -p caiven-machine -- /tmp/celeste_clone.cav
```
Expected: window opens, sky-blue background, "CELESTE CLONE - BOOT OK" text
visible, no panics/Lua errors in terminal output. Close the window.

- [ ] **Step 5: Commit**

```bash
git add projects/showcase/celeste_clone/caiven.toml \
        projects/showcase/celeste_clone/collision_types.json \
        projects/showcase/celeste_clone/main.lua
git commit -m "feat(showcase): scaffold celeste_clone project"
```

---

### Task 2: Art and audio asset generation script

**Files:**
- Create: `scripts/demo-carts/gen_celeste_assets.py`
- Create (generated by the script, then committed): `projects/showcase/celeste_clone/sprites.hex`
- Create (generated by the script, then committed): `projects/showcase/celeste_clone/sfx.hex`
- Create (generated by the script, then committed): `projects/showcase/celeste_clone/music.hex`
- Modify: `projects/showcase/celeste_clone/main.lua`

**Interfaces:**
- Produces: sprite ids `0`(blank) `1`/`2`/`3`(player idle/run1/run2)
  `4`(ground) `5`(one-way platform) `6`(spike) `7`(berry) `8`(flag)
  `9`(slope_right) `10`(slope_left); SFX slots `0`(jump) `1`(dash)
  `2`(death) `3`(collect); music slot `0`(loop track). Later tasks reference
  these ids by name via Lua constants defined in Task 3, not raw numbers.

- [ ] **Step 1: Write the generator script**

```python
#!/usr/bin/env python3
"""One-off asset generator for the celeste_clone showcase cart.

Writes sprites.hex (sprite-major RAM order, id*64 + sy*8 + sx, one hex
digit pair per byte = one palette index 0-15 per pixel, 0 = transparent),
sfx.hex (one line per SFX slot, 16 steps x 4 bytes = note/volume/wave/byte3),
and music.hex (one line per music slot, 8 steps x 4 bytes) directly as the
project-dir .hex text format (crates/caiven-cart/src/text.rs), so no image
library is needed. Re-run after editing SPRITES/SFX_STEPS/MUSIC_STEPS below;
outputs are committed, this script is not part of the shipped cart.
"""
import pathlib

OUT = pathlib.Path(__file__).resolve().parents[2] / "projects" / "showcase" / "celeste_clone"

SPRITE_BYTES = 64  # 8x8 pixels, 1 byte/pixel


def sprite_from_rows(rows, legend):
    px = []
    for row in rows:
        for ch in row:
            px.append(legend.get(ch, 0))
    assert len(px) == SPRITE_BYTES, f"sprite must be 8x8, got {len(px)} pixels"
    return px


BLANK = [0] * SPRITE_BYTES

PLAYER_IDLE = sprite_from_rows([
    "..1111..",
    ".111111.",
    ".122221.",
    ".111111.",
    "..1111..",
    "..1111..",
    ".11..11.",
    "11....11",
], {"1": 4, "2": 5})

PLAYER_RUN1 = sprite_from_rows([
    "..1111..",
    ".111111.",
    ".122221.",
    ".111111.",
    "..1111..",
    ".11111..",
    "11...11.",
    "1.....1.",
], {"1": 4, "2": 5})

PLAYER_RUN2 = sprite_from_rows([
    "..1111..",
    ".111111.",
    ".122221.",
    ".111111.",
    "..1111..",
    "..11111.",
    ".11...11",
    ".1.....1",
], {"1": 4, "2": 5})

GROUND = sprite_from_rows([
    "33333333",
    "11211112",
    "11111111",
    "12111121",
    "11111111",
    "11112111",
    "11111112",
    "12111111",
], {"1": 1, "2": 2, "3": 3})

PLATFORM = sprite_from_rows([
    "33333333",
    "22222222",
    "........",
    "........",
    "........",
    "........",
    "........",
    "........",
], {"2": 2, "3": 3})

SPIKE = sprite_from_rows([
    "...11...",
    "...11...",
    "..1111..",
    "..1111..",
    ".111111.",
    ".111111.",
    "11111111",
    "22222222",
], {"1": 8, "2": 9})

BERRY = sprite_from_rows([
    "..77....",
    ".766....",
    "76666667",
    "66666666",
    "66666666",
    ".666666.",
    "..6666..",
    "...66...",
], {"6": 6, "7": 7})

FLAG = sprite_from_rows([
    "9.......",
    "9AAAA...",
    "9ABAAA..",
    "9AAAA...",
    "9.......",
    "9.......",
    "9.......",
    "9.......",
], {"9": 12, "A": 10, "B": 11})

SLOPE_RIGHT = sprite_from_rows([
    "0000000G",
    "000000GG",
    "00000GG1",
    "0000GG11",
    "000GG111",
    "00GG1111",
    "0GG11111",
    "GG111111",
], {"G": 3, "1": 1})

SLOPE_LEFT = sprite_from_rows([
    "G0000000",
    "GG000000",
    "1GG00000",
    "11GG0000",
    "111GG000",
    "1111GG00",
    "11111GG0",
    "111111GG",
], {"G": 3, "1": 1})

# Order matches the sprite ids documented in the plan/main.lua constants.
SPRITES = [
    BLANK,        # 0
    PLAYER_IDLE,  # 1
    PLAYER_RUN1,  # 2
    PLAYER_RUN2,  # 3
    GROUND,       # 4
    PLATFORM,     # 5
    SPIKE,        # 6
    BERRY,        # 7
    FLAG,         # 8
    SLOPE_RIGHT,  # 9
    SLOPE_LEFT,   # 10
]


def write_sprites_hex():
    lines = []
    for sprite in SPRITES:
        lines.append("".join(f"{b:02x}" for b in sprite))
    (OUT / "sprites.hex").write_text("\n".join(lines) + "\n")


# SFX: note (MIDI-ish 0-127), volume (0-15), wave (0-3), byte3 (pan/envelope,
# 0 = center pan, instant envelope). One line per slot, 16 steps max;
# trailing all-zero steps may be omitted, decoder treats missing tail as 0.
def sfx_line(steps):
    step_bytes = []
    for note, vol, wave, byte3 in steps:
        step_bytes += [note, vol, wave, byte3]
    return "".join(f"{b:02x}" for b in step_bytes)


SFX_STEPS = [
    # 0: jump - short rising blip
    [(48, 10, 0, 0), (55, 10, 0, 0), (60, 9, 0, 0)],
    # 1: dash - quick noise burst
    [(40, 12, 2, 0), (40, 8, 2, 0)],
    # 2: death - short descending tone
    [(52, 12, 1, 0), (46, 10, 1, 0), (40, 8, 1, 0), (34, 6, 1, 0)],
    # 3: collect - bright two-note chime
    [(64, 11, 0, 0), (71, 11, 0, 0)],
]


def write_sfx_hex():
    lines = [sfx_line(steps) for steps in SFX_STEPS]
    (OUT / "sfx.hex").write_text("\n".join(lines) + "\n")


# Music: 8 steps x 4 bytes per slot. One short looping melody.
MUSIC_STEPS = [
    [
        (48, 8, 0, 0), (52, 8, 0, 0), (55, 8, 0, 0), (52, 8, 0, 0),
        (48, 8, 0, 0), (55, 8, 0, 0), (52, 8, 0, 0), (48, 8, 0, 0),
    ],
]


def write_music_hex():
    lines = [sfx_line(steps) for steps in MUSIC_STEPS]
    (OUT / "music.hex").write_text("\n".join(lines) + "\n")


if __name__ == "__main__":
    write_sprites_hex()
    write_sfx_hex()
    write_music_hex()
    print(f"wrote sprites.hex, sfx.hex, music.hex to {OUT}")
```

- [ ] **Step 2: Run it**

Run: `python3 scripts/demo-carts/gen_celeste_assets.py`
Expected: prints the output path; `sprites.hex`, `sfx.hex`, `music.hex` now
exist under `projects/showcase/celeste_clone/`.

- [ ] **Step 3: Temporarily extend `main.lua` to verify the assets visually/audibly**

Replace the stub `_draw`/`_update` from Task 1 with a debug viewer (this
code is replaced again in Task 3, it only exists to verify Task 2's output):

```lua
function _init()
  set_palette_color(0, 100, 160, 230)  -- sky
  set_palette_color(1, 92, 58, 33)     -- dirt dark
  set_palette_color(2, 132, 86, 48)    -- dirt light
  set_palette_color(3, 60, 168, 60)    -- grass
  set_palette_color(4, 220, 70, 90)    -- player body
  set_palette_color(5, 255, 220, 210)  -- player face
  set_palette_color(6, 220, 40, 60)    -- berry red
  set_palette_color(7, 60, 160, 70)    -- berry leaf
  set_palette_color(8, 230, 40, 40)    -- spike red
  set_palette_color(9, 255, 255, 255)  -- spike highlight
  set_palette_color(10, 250, 210, 40)  -- flag yellow
  set_palette_color(11, 255, 255, 255) -- flag white
  set_palette_color(12, 20, 20, 20)    -- outline/pole
  set_palette_color(13, 255, 255, 255) -- particle white
  set_palette_color(14, 255, 255, 0)   -- UI text
  set_palette_color(15, 40, 40, 40)    -- spare
end

function _update()
  if button_pressed(4) then play_sfx(0) end
  if button_pressed(5) then play_sfx(1) end
  if button_pressed(2) then play_sfx(2) end
  if button_pressed(3) then play_sfx(3) end
  if button_pressed(6) then
    if is_music_playing() then stop_music() else play_music(0) end
  end
end

function _draw()
  clear_screen()
  for id = 0, 10 do
    sprite(id, 4 + id * 11, 40)
  end
  draw_text("A/B/LEFT/RIGHT: SFX  SELECT: MUSIC", 2, 2, 14)
end
```

- [ ] **Step 4: Build and check visually/audibly**

Run:
```bash
target/debug/caiven-studio build projects/showcase/celeste_clone --out /tmp/celeste_clone.cav
cargo run -p caiven-machine -- /tmp/celeste_clone.cav
```
Expected: 11 sprites drawn in a row (id 0 invisible/blank, then player,
ground, platform, spike, berry, flag, and both slope tiles as recognizable
diagonal shapes — slope_right filled toward bottom-left, slope_left mirrored
toward bottom-right). Pressing A/B/Left/Right plays four distinct short
sounds; Select toggles a looping melody. Close the window.

- [ ] **Step 5: Commit**

```bash
git add scripts/demo-carts/gen_celeste_assets.py \
        projects/showcase/celeste_clone/sprites.hex \
        projects/showcase/celeste_clone/sfx.hex \
        projects/showcase/celeste_clone/music.hex \
        projects/showcase/celeste_clone/main.lua
git commit -m "feat(showcase): generate celeste_clone sprite/sfx/music assets"
```

---

### Task 3: World data, room painter, camera, state machine skeleton

**Files:**
- Modify: `projects/showcase/celeste_clone/main.lua` (full rewrite of body,
  replacing Task 2's debug viewer)

**Interfaces:**
- Produces: `TILE = 8`, `ROOM_TILES = 16`, `ROOM_PX = 128`; sprite id
  constants `SPR_BLANK..SPR_SLOPE_LEFT`; collision id constants
  `COL_WALKABLE, COL_SOLID, COL_HAZARD, COL_PLATFORM, COL_RAMP_R, COL_RAMP_L`;
  `ROOMS[1..8]` table, each `{col, row, tiles = {...rects...}, spawn =
  {x=,y=}, berry = {x=,y=} or nil, flag = {x=,y=} or nil}` in **world pixel**
  coordinates for `spawn`/`berry`/`flag`; `paint_world()` (called once from
  `_init`, iterates `ROOMS` and paints every rect via `set_tile`/
  `set_collision`); `room_at(px, py)` returning the room table under a world
  pixel position; `update_camera(px, py)` wrapping `set_camera`; `GAME =
  {mode = "title", deaths = 0, berries = 0, current_room = 1}`.
- Consumes: sprite/SFX ids from Task 2 (referenced here only to name the
  constants; not yet drawn as a real player).

- [ ] **Step 1: Replace `main.lua` with world data + room painter + camera + skeleton state machine**

```lua
TILE = 8
ROOM_TILES = 16
ROOM_PX = TILE * ROOM_TILES -- 128

SPR_BLANK = 0
SPR_PLAYER_IDLE = 1
SPR_PLAYER_RUN1 = 2
SPR_PLAYER_RUN2 = 3
SPR_GROUND = 4
SPR_PLATFORM = 5
SPR_SPIKE = 6
SPR_BERRY = 7
SPR_FLAG = 8
SPR_SLOPE_RIGHT = 9
SPR_SLOPE_LEFT = 10

SFX_JUMP = 0
SFX_DASH = 1
SFX_DEATH = 2
SFX_COLLECT = 3
MUSIC_MAIN = 0

-- Resolved once in _init from collision_types.json (ids there are stable,
-- but resolving by name keeps this file correct if the table ever changes).
COL_WALKABLE, COL_SOLID, COL_HAZARD, COL_PLATFORM, COL_RAMP_R, COL_RAMP_L = nil, nil, nil, nil, nil, nil

local function rect(x0, y0, x1, y1, col, spr)
  return { x0 = x0, y0 = y0, x1 = x1, y1 = y1, col = col, spr = spr }
end

-- Room tile rects are in ROOM-LOCAL tile coordinates (0-15). paint_world()
-- offsets them by each room's (col, row) * ROOM_TILES before painting.
ROOMS = {
  [1] = {
    col = 0, row = 0,
    tiles = {
      -- filled in Step 2 below (kept out of this listing to avoid duplication)
    },
    spawn = { x = 16, y = 13 * TILE },
    berry = { x = 7 * TILE, y = 9 * TILE },
    flag = nil,
  },
}
```

Do not actually leave a placeholder comment in the file — Step 2 below gives
the complete `ROOMS` table body; this Step 1 snippet only establishes the
constants and helper that Step 2's full table depends on. Combine Step 1 and
Step 2 into one edit when writing the file.

- [ ] **Step 2: Write the complete `ROOMS` table (all 8 rooms)**

```lua
ROOMS = {
  [1] = {
    col = 0, row = 0,
    tiles = {
      rect(0, 14, 15, 15, "solid", SPR_GROUND),
      rect(6, 10, 8, 10, "solid", SPR_GROUND), -- tutorial hop platform
    },
    spawn = { x = 2 * TILE, y = 13 * TILE },
    berry = { x = 7 * TILE, y = 9 * TILE },
    flag = nil,
  },
  [2] = {
    col = 1, row = 0,
    tiles = {
      rect(0, 14, 5, 15, "solid", SPR_GROUND),
      rect(8, 14, 15, 15, "solid", SPR_GROUND),
      rect(6, 15, 7, 15, "hazard", SPR_SPIKE),
      rect(11, 11, 12, 11, "solid", SPR_GROUND),
    },
    spawn = { x = 1 * TILE, y = 13 * TILE },
    berry = { x = 11 * TILE, y = 10 * TILE },
    flag = nil,
  },
  [3] = {
    col = 2, row = 0,
    tiles = {
      rect(0, 14, 15, 15, "solid", SPR_GROUND),
      rect(6, 4, 6, 13, "solid", SPR_GROUND),
      rect(9, 4, 9, 13, "solid", SPR_GROUND),
      rect(7, 3, 8, 3, "solid", SPR_GROUND), -- shaft-top ledge, berry sits here
    },
    spawn = { x = 1 * TILE, y = 13 * TILE },
    berry = { x = 7 * TILE, y = 2 * TILE },
    flag = nil,
  },
  [4] = {
    col = 3, row = 0,
    tiles = {
      rect(0, 14, 9, 15, "solid", SPR_GROUND),
      rect(2, 10, 6, 10, "platform", SPR_PLATFORM),
    },
    spawn = { x = 1 * TILE, y = 13 * TILE },
    berry = { x = 4 * TILE, y = 9 * TILE },
    flag = nil,
  },
  [5] = {
    col = 0, row = 1,
    tiles = {
      rect(0, 14, 4, 15, "solid", SPR_GROUND),
      rect(11, 14, 15, 15, "solid", SPR_GROUND),
      rect(5, 15, 10, 15, "hazard", SPR_SPIKE),
    },
    spawn = { x = 1 * TILE, y = 13 * TILE },
    berry = { x = 7 * TILE, y = 11 * TILE },
    flag = nil,
  },
  [6] = {
    col = 1, row = 1,
    tiles = {
      rect(0, 14, 6, 14, "solid", SPR_GROUND),
      rect(0, 15, 6, 15, "solid", SPR_GROUND),
      rect(7, 13, 7, 13, "ramp_right", SPR_SLOPE_RIGHT),
      rect(8, 13, 15, 13, "solid", SPR_GROUND),
      rect(8, 14, 15, 15, "solid", SPR_GROUND),
      rect(12, 13, 12, 13, "hazard", SPR_SPIKE),
      rect(12, 10, 12, 10, "solid", SPR_GROUND),
    },
    spawn = { x = 1 * TILE, y = 13 * TILE },
    berry = { x = 12 * TILE, y = 9 * TILE },
    flag = nil,
  },
  [7] = {
    col = 2, row = 1,
    tiles = {
      rect(0, 13, 3, 15, "solid", SPR_GROUND),
      rect(4, 14, 4, 15, "solid", SPR_GROUND),
      rect(9, 14, 9, 15, "solid", SPR_GROUND),
      rect(4, 8, 4, 13, "solid", SPR_GROUND),
      rect(9, 8, 9, 13, "solid", SPR_GROUND),
      rect(6, 10, 7, 10, "platform", SPR_PLATFORM),
      rect(5, 15, 8, 15, "hazard", SPR_SPIKE),
      rect(10, 13, 15, 15, "solid", SPR_GROUND),
    },
    spawn = { x = 1 * TILE, y = 12 * TILE },
    berry = { x = 6 * TILE, y = 8 * TILE },
    flag = nil,
  },
  [8] = {
    col = 3, row = 1,
    tiles = {
      rect(0, 13, 15, 15, "solid", SPR_GROUND),
      rect(8, 15, 9, 15, "hazard", SPR_SPIKE),
      rect(10, 11, 10, 11, "solid", SPR_GROUND),
    },
    spawn = { x = 1 * TILE, y = 12 * TILE },
    berry = { x = 10 * TILE, y = 10 * TILE },
    flag = { x = 13 * TILE, y = 11 * TILE },
  },
}
```

- [ ] **Step 3: Write `paint_world`, `room_at`, `update_camera`, palette, and the state-machine skeleton**

```lua
local function paint_world()
  for _, room in ipairs(ROOMS) do
    local ox, oy = room.col * ROOM_TILES, room.row * ROOM_TILES
    for _, r in ipairs(room.tiles) do
      local col_id = collision_type_id(r.col)
      for ty = r.y0, r.y1 do
        for tx = r.x0, r.x1 do
          set_tile(ox + tx, oy + ty, r.spr)
          set_collision(ox + tx, oy + ty, col_id)
        end
      end
    end
  end
end

function room_at(px, py)
  local col = math.floor(px / ROOM_PX)
  local row = math.floor(py / ROOM_PX)
  for _, room in ipairs(ROOMS) do
    if room.col == col and room.row == row then return room end
  end
  return nil
end

function update_camera(px, py)
  local col = math.floor(px / ROOM_PX)
  local row = math.floor(py / ROOM_PX)
  set_camera(col * ROOM_PX, row * ROOM_PX)
end

local function set_palette()
  set_palette_color(0, 100, 160, 230)
  set_palette_color(1, 92, 58, 33)
  set_palette_color(2, 132, 86, 48)
  set_palette_color(3, 60, 168, 60)
  set_palette_color(4, 220, 70, 90)
  set_palette_color(5, 255, 220, 210)
  set_palette_color(6, 220, 40, 60)
  set_palette_color(7, 60, 160, 70)
  set_palette_color(8, 230, 40, 40)
  set_palette_color(9, 255, 255, 255)
  set_palette_color(10, 250, 210, 40)
  set_palette_color(11, 255, 255, 255)
  set_palette_color(12, 20, 20, 20)
  set_palette_color(13, 255, 255, 255)
  set_palette_color(14, 255, 255, 0)
  set_palette_color(15, 40, 40, 40)
end

function _init()
  COL_WALKABLE = collision_type_id("walkable")
  COL_SOLID = collision_type_id("solid")
  COL_HAZARD = collision_type_id("hazard")
  COL_PLATFORM = collision_type_id("platform")
  COL_RAMP_R = collision_type_id("ramp_right")
  COL_RAMP_L = collision_type_id("ramp_left")

  set_palette()
  paint_world()

  GAME = { mode = "title", deaths = 0, berries = 0 }
  debug_pos = Vec2.new(ROOMS[1].spawn.x, ROOMS[1].spawn.y)
end

function _update()
  if GAME.mode == "title" then
    if button_pressed(4) then GAME.mode = "playing" end
    return
  end

  -- Placeholder movement for this task only (a fixed-speed walker with no
  -- gravity/collision) so room painting and camera snapping can be verified
  -- end to end before Task 4 adds real physics. Replaced in Task 4.
  local dx, dy = 0, 0
  if button_down(2) then dx = dx - 2 end
  if button_down(3) then dx = dx + 2 end
  if button_down(0) then dy = dy - 2 end
  if button_down(1) then dy = dy + 2 end
  debug_pos.x = clamp(debug_pos.x + dx, 0, 4 * ROOM_PX - TILE)
  debug_pos.y = clamp(debug_pos.y + dy, 0, 2 * ROOM_PX - TILE)
  update_camera(debug_pos.x, debug_pos.y)
end

function _draw()
  clear_screen()
  if GAME.mode == "title" then
    draw_text("CELESTE CLONE", 36, 50, 14)
    draw_text("PRESS A", 46, 66, 7)
    return
  end
  local room = room_at(debug_pos.x, debug_pos.y)
  local ox, oy = room.col * ROOM_TILES, room.row * ROOM_TILES
  draw_map(ox, oy, ox * TILE, oy * TILE, ROOM_TILES, ROOM_TILES)
  sprite(SPR_PLAYER_IDLE, math.floor(debug_pos.x), math.floor(debug_pos.y))
  if room.berry then
    sprite(SPR_BERRY, room.berry.x, room.berry.y)
  end
  if room.flag then
    sprite(SPR_FLAG, room.flag.x, room.flag.y)
  end
end
```

`draw_map(cell_x, cell_y, sx, sy, w, h)` draws map cells `[cell_x, cell_x+w)
x [cell_y, cell_y+h)` at screen position `(sx, sy)`; since `set_camera` has
already been applied for this frame, passing world tile-space coordinates
scaled by `TILE` for `sx, sy` keeps the block aligned under the camera the
same way `sprite()`/`fill_rect()` are camera-aware.

- [ ] **Step 4: Build and playtest room painting + camera**

Run:
```bash
target/debug/caiven-studio build projects/showcase/celeste_clone --out /tmp/celeste_clone.cav
cargo run -p caiven-machine -- /tmp/celeste_clone.cav
```
Expected: title screen shows, A starts play. A dot-like player icon can be
walked (arrows) through all 4×2 rooms; each room's floor/platform/hazard/
slope tiles render as painted in Step 2 (recognizable shapes, no garbage
sprites in empty air); camera snaps instantly to the next 128×128 cell when
crossing a room boundary; every room's berry (and room 8's flag) is visible
in the right place. Close the window.

- [ ] **Step 5: Commit**

```bash
git add projects/showcase/celeste_clone/main.lua
git commit -m "feat(showcase): celeste_clone world data, room painter, camera"
```

---

### Task 4: Player core physics (run, gravity, jump, coyote, buffer, variable height)

**Files:**
- Modify: `projects/showcase/celeste_clone/main.lua`

**Interfaces:**
- Consumes: `ROOMS`, `paint_world`, `room_at`, `update_camera`, `TILE`,
  sprite/collision constants, `GAME` from Task 3.
- Produces: `player = { pos = Vec2, vx, vy, w = 6, h = 8, facing, on_ground,
  coyote_timer, jump_buffer, anim }` and `physics_update(input)` (called from
  `_update` when `GAME.mode == "playing"`), replacing Task 3's placeholder
  walker. Later tasks (5-9) read/extend this same `player` table — do not
  rename its fields.

- [ ] **Step 1: Replace the placeholder walker with real player state and physics**

Remove `debug_pos` and the placeholder movement block from `_update`/
`_draw` (Task 3, Step 3). Add:

```lua
RUN_MAX = 1.2
RUN_ACCEL_GROUND = 0.4
RUN_ACCEL_AIR = 0.3
FRICTION = 0.3
GRAVITY = 0.35
FALL_MAX = 4.5
JUMP_VY = -4.8
JUMP_CUT_MULT = 0.5
COYOTE_MAX = 6
BUFFER_MAX = 4
PLAYER_W, PLAYER_H = 6, 8

local function spawn_player(spawn)
  player = {
    pos = Vec2.new(spawn.x, spawn.y),
    vx = 0, vy = 0,
    w = PLAYER_W, h = PLAYER_H,
    facing = 1,
    on_ground = false,
    coyote_timer = 0,
    jump_buffer = 0,
    anim = new_anim({ SPR_PLAYER_RUN1, SPR_PLAYER_IDLE, SPR_PLAYER_RUN2, SPR_PLAYER_IDLE }, 8),
  }
end

local function player_horizontal(input)
  local accel = player.on_ground and RUN_ACCEL_GROUND or RUN_ACCEL_AIR
  if input.left then
    player.vx = math.max(player.vx - accel, -RUN_MAX)
    player.facing = -1
  elseif input.right then
    player.vx = math.min(player.vx + accel, RUN_MAX)
    player.facing = 1
  else
    if player.vx > 0 then player.vx = math.max(0, player.vx - FRICTION)
    elseif player.vx < 0 then player.vx = math.min(0, player.vx + FRICTION) end
  end
end

local function player_vertical(input)
  if player.jump_buffer > 0 then player.jump_buffer = player.jump_buffer - 1 end
  if input.jump_pressed then player.jump_buffer = BUFFER_MAX end

  if not player.on_ground then
    player.vy = clamp(player.vy + GRAVITY, -99, FALL_MAX)
  end

  if player.jump_buffer > 0 and (player.on_ground or player.coyote_timer > 0) then
    player.vy = JUMP_VY
    player.jump_buffer = 0
    player.coyote_timer = 0
    player.on_ground = false
    play_sfx(SFX_JUMP)
  elseif input.jump_released and player.vy < 0 then
    player.vy = player.vy * JUMP_CUT_MULT
  end
end

local function player_move_and_collide()
  local nx = move_and_collide(player.pos.x, player.pos.y, player.w, player.h, player.vx, 0)
  player.pos.x = nx

  local _, ny, touch = move_and_collide(player.pos.x, player.pos.y, player.w, player.h, 0, player.vy)
  player.pos.y = ny

  if touch.ground then
    if not player.on_ground then player.coyote_timer = COYOTE_MAX end
    player.on_ground = true
    player.vy = 0
  else
    if player.on_ground then player.coyote_timer = COYOTE_MAX end
    player.on_ground = false
  end
  if touch.ceiling and player.vy < 0 then player.vy = 0 end
  if player.coyote_timer > 0 and not player.on_ground then
    player.coyote_timer = player.coyote_timer - 1
  end
end

function physics_update(input)
  player_horizontal(input)
  player_vertical(input)
  player_move_and_collide()
  anim_update(player.anim)
end

local function read_input()
  return {
    left = button_down(2), right = button_down(3),
    jump_pressed = button_pressed(4), jump_released = button_released(4),
  }
end
```

- [ ] **Step 2: Wire it into `_init`/`_update`/`_draw`**

In `_init`, after `paint_world()`, replace the old `debug_pos = ...` line
with `spawn_player(ROOMS[1].spawn)`.

In `_update`, replace the placeholder movement block with:
```lua
  physics_update(read_input())
  update_camera(player.pos.x, player.pos.y)
```

In `_draw`, replace the `room_at(debug_pos.x, ...)` line and the
`sprite(SPR_PLAYER_IDLE, ...)` line with:
```lua
  local room = room_at(player.pos.x, player.pos.y)
  local ox, oy = room.col * ROOM_TILES, room.row * ROOM_TILES
  draw_map(ox, oy, ox * TILE, oy * TILE, ROOM_TILES, ROOM_TILES)
  local frame = player.on_ground and math.abs(player.vx) > 0.1 and anim_sprite(player.anim) or SPR_PLAYER_IDLE
  sprite(frame, math.floor(player.pos.x), math.floor(player.pos.y), player.facing < 0)
```

- [ ] **Step 3: Build and playtest in room 1**

Run:
```bash
target/debug/caiven-studio build projects/showcase/celeste_clone --out /tmp/celeste_clone.cav
cargo run -p caiven-machine -- /tmp/celeste_clone.cav
```
Expected: press A from title, player spawns standing on room 1's floor.
Left/Right run with acceleration/friction (not instant snap to top speed),
sprite flips facing left, walk-cycle animates while moving on ground. A
jumps; holding A vs tapping it noticeably changes jump height (variable
height). Walking off the tutorial platform's edge and jumping a beat late
still catches (coyote time); pressing A just before landing still jumps on
landing (buffer). Standing still lands cleanly with `vy` reset. Close the
window.

- [ ] **Step 4: Commit**

```bash
git add projects/showcase/celeste_clone/main.lua
git commit -m "feat(showcase): celeste_clone player run/jump/coyote/buffer physics"
```

---

### Task 5: Wall slide and wall jump

**Files:**
- Modify: `projects/showcase/celeste_clone/main.lua`

**Interfaces:**
- Consumes: `player` table and `player_move_and_collide`/`player_vertical`
  from Task 4 (extends both; keeps existing field names).
- Produces: `player.wall_dir` (`-1`/`0`/`1`), `player.walljump_lock` (frames
  of horizontal-input lock after a wall jump).

- [ ] **Step 1: Add wall-slide detection and wall jump**

Add constants near Task 4's:
```lua
WALL_SLIDE_MAX = 1.0
WALLJUMP_VX = 2.2
WALLJUMP_VY = -4.6
WALLJUMP_LOCK = 10
```

In `spawn_player`, add `wall_dir = 0, walljump_lock = 0` to the returned
table.

Modify `player_move_and_collide` to detect wall contact after the
horizontal move (touch is currently only read from the vertical call):
```lua
local function player_move_and_collide()
  local nx, _, htouch = move_and_collide(player.pos.x, player.pos.y, player.w, player.h, player.vx, 0)
  player.pos.x = nx
  if htouch.left then player.wall_dir = -1
  elseif htouch.right then player.wall_dir = 1
  else player.wall_dir = 0 end

  local _, ny, touch = move_and_collide(player.pos.x, player.pos.y, player.w, player.h, 0, player.vy)
  player.pos.y = ny

  if touch.ground then
    if not player.on_ground then player.coyote_timer = COYOTE_MAX end
    player.on_ground = true
    player.vy = 0
  else
    if player.on_ground then player.coyote_timer = COYOTE_MAX end
    player.on_ground = false
  end
  if touch.ceiling and player.vy < 0 then player.vy = 0 end
  if player.coyote_timer > 0 and not player.on_ground then
    player.coyote_timer = player.coyote_timer - 1
  end
end
```

`move_and_collide` returns `nx, ny, touch` from a single call covering both
axes it was given (`dx` or `dy` nonzero, not both) — the horizontal call
above already returns `touch.left`/`touch.right` for that axis, no separate
query needed.

Modify `player_vertical` to slide down walls and wall-jump:
```lua
local function player_vertical(input)
  if player.jump_buffer > 0 then player.jump_buffer = player.jump_buffer - 1 end
  if input.jump_pressed then player.jump_buffer = BUFFER_MAX end
  if player.walljump_lock > 0 then player.walljump_lock = player.walljump_lock - 1 end

  local sliding = not player.on_ground and player.wall_dir ~= 0 and player.vy > 0
  if not player.on_ground then
    local cap = sliding and WALL_SLIDE_MAX or FALL_MAX
    player.vy = clamp(player.vy + GRAVITY, -99, cap)
  end

  if player.jump_buffer > 0 and (player.on_ground or player.coyote_timer > 0) then
    player.vy = JUMP_VY
    player.jump_buffer = 0
    player.coyote_timer = 0
    player.on_ground = false
    play_sfx(SFX_JUMP)
  elseif player.jump_buffer > 0 and sliding then
    player.vy = WALLJUMP_VY
    player.vx = -player.wall_dir * WALLJUMP_VX
    player.facing = -player.wall_dir
    player.walljump_lock = WALLJUMP_LOCK
    player.jump_buffer = 0
    play_sfx(SFX_JUMP)
  elseif input.jump_released and player.vy < 0 then
    player.vy = player.vy * JUMP_CUT_MULT
  end
end
```

Modify `player_horizontal` to skip input control during the wall-jump lock:
```lua
local function player_horizontal(input)
  if player.walljump_lock > 0 then return end
  -- ... existing body unchanged
end
```

- [ ] **Step 2: Build and playtest in room 3's wall-jump shaft**

Run: same build+run commands as Task 4.
Expected: walking into room 3's narrow vertical shaft and holding toward
either wall while airborne slows the fall to a slide (visibly slower than a
normal fall); pressing A while sliding kicks the player away from the wall
and upward, and horizontal input is locked for a beat afterward (player
doesn't instantly re-stick to the wall). Chained wall jumps (left wall,
right wall, left wall...) can climb the full shaft to the berry ledge at the
top. Close the window.

- [ ] **Step 3: Commit**

```bash
git add projects/showcase/celeste_clone/main.lua
git commit -m "feat(showcase): celeste_clone wall slide and wall jump"
```

---

### Task 6: Dash

**Files:**
- Modify: `projects/showcase/celeste_clone/main.lua`

**Interfaces:**
- Consumes: `player`, `physics_update`, `read_input` from Tasks 4-5.
- Produces: `player.dashes` (0 or 1), `player.dash_timer`, `player.dashing`;
  `read_input` gains `dash_pressed` and 8-way direction fields.

- [ ] **Step 1: Add dash state, input, and update logic**

Constants:
```lua
DASH_SPEED = 3.5
DASH_FRAMES = 10
```

In `spawn_player`, add `dashes = 1, dash_timer = 0, dashing = false,
dash_vx = 0, dash_vy = 0`.

Extend `read_input`:
```lua
local function read_input()
  return {
    left = button_down(2), right = button_down(3),
    up = button_down(0), down = button_down(1),
    jump_pressed = button_pressed(4), jump_released = button_released(4),
    dash_pressed = button_pressed(5),
  }
end
```

Add a dash-start helper and fold it into `physics_update`:
```lua
local function try_start_dash(input)
  if not input.dash_pressed or player.dashes <= 0 or player.dashing then return end
  local dx, dy = 0, 0
  if input.left then dx = -1 elseif input.right then dx = 1 end
  if input.up then dy = -1 elseif input.down then dy = 1 end
  if dx == 0 and dy == 0 then dx = player.facing end
  local len = math.sqrt(dx * dx + dy * dy)
  player.dashing = true
  player.dash_timer = DASH_FRAMES
  player.dash_vx = (dx / len) * DASH_SPEED
  player.dash_vy = (dy / len) * DASH_SPEED
  player.dashes = player.dashes - 1
  play_sfx(SFX_DASH)
end

function physics_update(input)
  try_start_dash(input)

  if player.dashing then
    player.vx, player.vy = player.dash_vx, player.dash_vy
    Particles.spawn(player.pos.x + player.w / 2, player.pos.y + player.h / 2,
      -player.dash_vx * 0.3, -player.dash_vy * 0.3, 13, 12)
    player.dash_timer = player.dash_timer - 1
    if player.dash_timer <= 0 then
      player.dashing = false
      player.vx = player.dash_vx * 0.5
      player.vy = math.min(player.dash_vy, 0)
    end
  else
    player_horizontal(input)
    player_vertical(input)
  end

  player_move_and_collide()
  if player.on_ground or player.wall_dir ~= 0 then player.dashes = 1 end
  anim_update(player.anim)
  Particles.update()
end
```

Add `Particles.draw()` to `_draw`, right after the `draw_map` call, so the
dash trail renders under the player sprite:
```lua
  Particles.draw()
```

- [ ] **Step 2: Build and playtest room 5's dash-required chasm**

Run: same build+run commands as Task 4.
Expected: pressing B with a direction held dashes 8-way at a fixed speed
with a short particle trail, ignoring gravity for its duration; pressing B
with no direction held dashes horizontally in the facing direction; the
dash charge does not refill again until touching ground or a wall (dashing
twice in a row mid-air is impossible); room 5's chasm (too wide to clear by
jumping alone) is crossable with one dash. Falling into the chasm's spikes
still kills (hazards land in Task 7 — for now falling just falls forever if
Task 7 isn't done yet; note that and move on). Close the window.

- [ ] **Step 3: Commit**

```bash
git add projects/showcase/celeste_clone/main.lua
git commit -m "feat(showcase): celeste_clone 8-directional dash"
```

---

### Task 7: Hazards, death, and respawn

**Files:**
- Modify: `projects/showcase/celeste_clone/main.lua`

**Interfaces:**
- Consumes: `player`, `GAME`, `ROOMS`, `room_at`, `COL_HAZARD` from earlier
  tasks.
- Produces: `GAME.mode` gains `"dying"`; `check_hazard()` and
  `handle_dying()`, called from `_update`.

- [ ] **Step 1: Add hazard detection and the dying/respawn sequence**

```lua
DYING_FRAMES = 20

local function player_touches_hazard()
  local tx0 = math.floor(player.pos.x / TILE)
  local ty0 = math.floor(player.pos.y / TILE)
  local tx1 = math.floor((player.pos.x + player.w - 1) / TILE)
  local ty1 = math.floor((player.pos.y + player.h - 1) / TILE)
  for ty = ty0, ty1 do
    for tx = tx0, tx1 do
      if get_collision(tx, ty) == COL_HAZARD then return true end
    end
  end
  return false
end

local function start_dying()
  GAME.mode = "dying"
  GAME.dying_timer = DYING_FRAMES
  play_sfx(SFX_DEATH)
  for i = 1, 12 do
    local a = (i / 12) * 6.28318
    Particles.spawn(player.pos.x + player.w / 2, player.pos.y + player.h / 2,
      math.cos(a) * 1.5, math.sin(a) * 1.5, 8, 18)
  end
  GAME.deaths = GAME.deaths + 1
end

local function handle_dying()
  Particles.update()
  GAME.dying_timer = GAME.dying_timer - 1
  if GAME.dying_timer <= 0 then
    local room = room_at(player.pos.x, player.pos.y) or ROOMS[GAME.last_room or 1]
    spawn_player(room.spawn)
    GAME.mode = "playing"
  end
end
```

- [ ] **Step 2: Wire hazard detection and dying into `_update`/`_draw`**

Replace `_update`'s body (from Task 6) with a mode dispatch. `GAME.mode ==
"title"` keeps its existing branch; add:
```lua
  if GAME.mode == "playing" then
    local room = room_at(player.pos.x, player.pos.y)
    if room then GAME.last_room = room end
    physics_update(read_input())
    update_camera(player.pos.x, player.pos.y)
    if player_touches_hazard() then start_dying() end
    return
  end

  if GAME.mode == "dying" then
    handle_dying()
    return
  end
```

`GAME.last_room` records the room the player was in just before dying (a
hazard can be touched while overlapping a room boundary), so respawn always
uses a room the player was actually standing in — set `GAME.last_room = ROOMS[1]`
alongside the other `GAME` fields in `_init`.

`_draw` needs no change for "dying" beyond what it already draws (particles
draw via the existing `Particles.draw()` call regardless of mode; the player
sprite briefly not being drawn during the freeze is fine and reads as
"exploded").

- [ ] **Step 3: Build and playtest every hazard**

Run: same build+run commands as Task 4.
Expected: touching any spike tile (rooms 2, 5, 6, 7, 8) freezes input for a
beat, plays a distinct death sound, bursts particles at the player's
position, then respawns the player at the spawn point of the room they were
last standing in (not always room 1) — verify this specifically in room 5
and room 6, where the room's spawn is not the world origin. The on-screen
death counter isn't drawn yet (Task 9); confirm via a temporary
`draw_text(tostring(GAME.deaths), ...)` line if needed, then remove it
before committing. Close the window.

- [ ] **Step 4: Commit**

```bash
git add projects/showcase/celeste_clone/main.lua
git commit -m "feat(showcase): celeste_clone hazards, death, and respawn"
```

---

### Task 8: Berries

**Files:**
- Modify: `projects/showcase/celeste_clone/main.lua`

**Interfaces:**
- Consumes: `ROOMS[n].berry`, `player`, `GAME`, `Entities`, `Particles` from
  earlier tasks.
- Produces: `spawn_berries()` (called once from `_init`), each berry
  entity `{pos = Vec2, w = 8, h = 8, room = n}`; `GAME.berries` incremented
  on pickup.

- [ ] **Step 1: Turn each room's static `berry` field into an `Entities` pickup**

```lua
local function spawn_berries()
  Entities.clear()
  for _, room in ipairs(ROOMS) do
    if room.berry then
      Entities.add({
        pos = Vec2.new(room.berry.x, room.berry.y),
        w = 8, h = 8,
        room = room,
        is_berry = true,
      })
    end
  end
end

local function update_berries()
  for _, e in ipairs(Entities.overlapping(player.pos.x, player.pos.y, player.w, player.h)) do
    if e.is_berry and not e.dead then
      e.dead = true
      GAME.berries = GAME.berries + 1
      play_sfx(SFX_COLLECT)
      for i = 1, 8 do
        local a = (i / 8) * 6.28318
        Particles.spawn(e.pos.x + 4, e.pos.y + 4, math.cos(a) * 1.2, math.sin(a) * 1.2, 6, 16)
      end
    end
  end
  Entities.update_all()
end
```

Call `spawn_berries()` from `_init`, right after `spawn_player(ROOMS[1].spawn)`.

In `_update`'s `"playing"` branch, add `update_berries()` right after
`physics_update(...)`.

- [ ] **Step 2: Draw only the current room's un-collected berries**

In `_draw`, replace the old `if room.berry then sprite(SPR_BERRY, ...) end`
block (from Task 3/4) with:
```lua
  for _, e in ipairs(Entities.list) do
    if e.is_berry and e.room == room then
      sprite(SPR_BERRY, math.floor(e.pos.x), math.floor(e.pos.y))
    end
  end
```

- [ ] **Step 3: Build and playtest all 8 berries**

Run: same build+run commands as Task 4.
Expected: every room's berry is reachable (room 1: tutorial hop; room 2:
small jump onto the ledge; room 3: climb the wall-jump shaft; room 4: land
on the one-way platform; room 5: mid-dash across the chasm — must be timed,
missing it means falling into the spikes below; room 6: jump above the
spike; room 7: same wall-jump shaft pattern as room 3; room 8: small
detour). Each berry disappears with a particle burst and a chime exactly
once (walking back over an already-collected berry's tile does nothing).
Close the window.

- [ ] **Step 4: Commit**

```bash
git add projects/showcase/celeste_clone/main.lua
git commit -m "feat(showcase): celeste_clone berry collectibles"
```

---

### Task 9: Flag, win screen, HUD, and title/restart loop

**Files:**
- Modify: `projects/showcase/celeste_clone/main.lua`

**Interfaces:**
- Consumes: `GAME`, `ROOMS[8].flag`, `player` from earlier tasks.
- Produces: `GAME.mode` gains `"won"`; `reset_game()`; a HUD line drawn
  during `"playing"`/`"dying"`.

- [ ] **Step 1: Add the flag touch check, win screen, and restart**

```lua
local function player_touches_flag()
  local flag = ROOMS[8].flag
  if not flag then return false end
  return aabb_overlap(player.pos.x, player.pos.y, player.w, player.h,
    flag.x, flag.y, 8, 8)
end

function reset_game()
  GAME = { mode = "title", deaths = 0, berries = 0, last_room = ROOMS[1] }
  spawn_player(ROOMS[1].spawn)
  spawn_berries()
  stop_music()
end
```

- [ ] **Step 2: Wire flag/win/restart and music into `_init`/`_update`/`_draw`**

Replace `_init`'s `GAME = {...}` / `spawn_player` / `spawn_berries` lines
with a single `reset_game()` call (after `paint_world()`).

In `_update`'s `"title"` branch, start music on entering play:
```lua
  if GAME.mode == "title" then
    if button_pressed(4) then
      GAME.mode = "playing"
      play_music(MUSIC_MAIN)
    end
    return
  end
```

In the `"playing"` branch, check the flag right after the hazard check:
```lua
    if player_touches_hazard() then start_dying() end
    if player_touches_flag() then
      GAME.mode = "won"
      stop_music()
    end
    return
```

Add a `"won"` branch:
```lua
  if GAME.mode == "won" then
    if button_pressed(4) then reset_game() end
    return
  end
```

In `_draw`, add a HUD line during `"playing"`/`"dying"` (after the existing
per-room drawing, before any early return):
```lua
  if GAME.mode == "playing" or GAME.mode == "dying" then
    draw_text("DEATHS " .. GAME.deaths .. "  BERRIES " .. GAME.berries .. "/8", 2, 2, 14)
  end
```

Add a `"won"` draw branch (mirroring the existing `"title"` early-return
shape at the top of `_draw`):
```lua
  if GAME.mode == "won" then
    clear_screen()
    draw_text("YOU WIN", 44, 40, 14)
    draw_text("DEATHS " .. GAME.deaths, 40, 56, 7)
    draw_text("BERRIES " .. GAME.berries .. "/8", 38, 68, 7)
    draw_text("PRESS A", 46, 84, 7)
    return
  end
```

- [ ] **Step 3: Build and playtest the full loop**

Run: same build+run commands as Task 4.
Expected: title screen -> A starts play and music. HUD shows live death and
berry counts. Reaching room 8's flag stops music, shows "YOU WIN" with the
correct final deaths/berries tally. Pressing A on the win screen fully
resets (deaths back to 0, all berries reappear, player back at room 1,
music stopped until A is pressed again on the title screen) and returns to
the title screen. Close the window.

- [ ] **Step 4: Commit**

```bash
git add projects/showcase/celeste_clone/main.lua
git commit -m "feat(showcase): celeste_clone flag, win screen, HUD, restart loop"
```

---

### Task 10: Build registration and final verification

**Files:**
- Modify: `crates/caiven-studio/src/studio/examples.rs`

**Interfaces:**
- Produces: `celeste_clone` entry in `EXAMPLES`, bumping its length from 5
  to 6.

- [ ] **Step 1: Register the example**

In `crates/caiven-studio/src/studio/examples.rs`, change `EXAMPLES: [Example; 5]`
to `EXAMPLES: [Example; 6]` and append a new entry after `scenes-demo`:

```rust
    Example {
        id: "celeste-clone",
        name: "Celeste Clone",
        description: "8-room precision platformer showcasing the full Lua API and stdlib",
        bytes: include_bytes!("../../resources/examples/celeste_clone.cav"),
    },
```

- [ ] **Step 2: Run the full demo-cart build**

Run: `scripts/demo-carts/build.sh`
Expected: rebuilds all showcase/dev carts including
`crates/caiven-studio/resources/examples/celeste_clone.cav`, no errors.

- [ ] **Step 3: Rust build check**

Run: `cargo build -p caiven-studio`
Expected: compiles clean — confirms `include_bytes!` resolves and the array
length matches the literal entry count.

- [ ] **Step 4: Narrow project checks**

Run:
```bash
scripts/claude/check-lua-api.sh
scripts/claude/check-cart-compat.sh
```
Expected: both pass — this cart is content-only (no API/format changes),
these confirm nothing in the stdlib/cart-format contract broke.

- [ ] **Step 5: Full manual playtest pass**

Run: `cargo run -p caiven-machine -- crates/caiven-studio/resources/examples/celeste_clone.cav`

Walk through the full spec checklist in one sitting:
- Title screen -> A starts play, music starts.
- Room 1: run/jump feel reasonable; tutorial platform reachable; berry 1
  collectible.
- Room 2: coyote time and jump buffer both usable at the chasm; hazard
  kills and respawns at room 2's spawn; berry 2 collectible.
- Room 3: wall slide visibly slower than free-fall on both walls; chained
  wall jumps reach the shaft top; berry 3 collectible.
- Room 4: one-way platform can be jumped up through and landed on, not
  fallen through while standing; falling off the right side drops into
  room 5 (vertical connector); berry 4 collectible.
- Room 5: chasm requires a dash to cross; missing the dash and hitting the
  spikes respawns at room 5's spawn, not room 1 or room 4; berry 5
  (mid-dash pickup) collectible.
- Room 6: slope traversable; hazard requires a hop; berry 6 collectible.
- Room 7: combination of wall-jump shaft, one-way platform, and hazard gap
  all work together; berry 7 collectible.
- Room 8: hazard near the flag; touching the flag ends the run with the
  correct deaths/berries tally (8/8 if every berry above was collected in
  this same run); "press A" restarts cleanly back to the title screen.
- HUD death/berry counts update live and correctly throughout.

Note and fix any drift from this checklist directly in `main.lua` before
considering the task done (tune the constants from Tasks 4-6 or adjust a
room rect from Task 3 if a jump/dash is unreasonably tight or a gap is
untraversable — these are exactly the "constants tuned by playtest" the
spec calls out as a starting point, not a locked target).

- [ ] **Step 6: Commit**

```bash
git add crates/caiven-studio/src/studio/examples.rs
git commit -m "feat(studio): add Celeste Clone to the examples gallery"
```

If Step 5 required follow-up fixes to `projects/showcase/celeste_clone/main.lua`,
rerun `scripts/demo-carts/build.sh` first so the committed `.cav` matches,
then commit those fixes separately:
```bash
git add projects/showcase/celeste_clone/main.lua \
        crates/caiven-studio/resources/examples/celeste_clone.cav
git commit -m "fix(showcase): tune celeste_clone playtest feedback"
```

---

## Self-review notes

- **Spec coverage:** run/coyote/buffer/variable-height (T4), wall slide/wall
  jump (T5), 8-dir dash with ground/wall refill (T6), hazards+respawn (T7),
  one-way platform + slope (T3 tiles, exercised T4/T5/T7 playtests),
  berries (T8), death counter + win screen (T9), examples gallery entry +
  manual verification + narrow checks (T10). No spec section lacks a task.
- **Deviations from the spec, called out explicitly (not silent):** assets
  are generated as `.hex` (sprites/SFX/music) instead of `.png`, and the map/
  palette are built at runtime instead of as a `map.png`/`palette.png` file
  — both covered in Global Constraints above with the reasoning (matches
  existing repo convention in `projects/dev/tiles` and
  `projects/dev/platformer_demo`, avoids needing an image-encoding library).
  Room transitions use an absolute-world-coordinate camera derivation
  instead of literal x/y wraparound — same instant snap-cut result, simpler
  implementation. Berry count matches the spec's "8 total" (one per room)
  exactly.
- **Placeholder scan:** no TBD/TODO left in any task's code; Task 3 Step 1's
  short `ROOMS` snippet is explicitly folded into Step 2's complete table
  in the same edit, not left partial in the file.
- **Type/name consistency:** `player` fields (`pos`, `vx`, `vy`, `w`, `h`,
  `facing`, `on_ground`, `coyote_timer`, `jump_buffer`, `wall_dir`,
  `walljump_lock`, `dashes`, `dashing`, `dash_timer`, `anim`) are introduced
  once in Task 4/5/6 and reused unchanged through Task 9. `GAME` fields
  (`mode`, `deaths`, `berries`, `last_room`, `dying_timer`) likewise
  introduced once (Task 3/7/9) and reused, not renamed. `ROOMS[n]` shape
  (`col`, `row`, `tiles`, `spawn`, `berry`, `flag`) is fixed in Task 3 and
  never restructured later.
