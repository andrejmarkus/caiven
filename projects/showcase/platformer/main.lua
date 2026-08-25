TILE = 8
-- One room is exactly one screen: 24 x 16 tiles at 192 x 128 pixels.
ROOM_TILES_W = 24
ROOM_TILES_H = 16
ROOM_PX_W = TILE * ROOM_TILES_W -- 192
ROOM_PX_H = TILE * ROOM_TILES_H -- 128

SPR_BLANK = 0
SPR_PLAYER_IDLE = 1
SPR_PLAYER_RUN1 = 2
SPR_PLAYER_RUN2 = 3
SPR_PLAYER_RUN3 = 4
SPR_PLAYER_JUMP = 5
SPR_PLAYER_FALL = 6
SPR_PLAYER_WALL_SLIDE = 7
SPR_PLAYER_DASH = 8
SPR_PLAYER_DEATH = 9
SPR_GROUND_TOP = 10
SPR_GROUND_FILL = 11
SPR_PLATFORM = 12
SPR_SLOPE_RIGHT = 13
SPR_SLOPE_LEFT = 14
SPR_SPIKE = 15
SPR_BERRY = 16
SPR_FLAG_A = 17
SPR_FLAG_B = 18
SPR_CAVE_TOP = 19
SPR_CAVE_FILL = 20
SPR_RUIN_TOP = 21
SPR_RUIN_FILL = 22
SPR_CLOUD = 23

-- Sky backdrop color (palette slot 0). Sprite pixels can never draw this
-- index (the VM's sprite() builtin treats raw pixel byte 0 as transparent),
-- so it is reserved for the world backdrop, painted fresh each frame with
-- fill_screen before draw_map.
COLOR_SKY = 0

SFX_JUMP = 0
SFX_DASH = 1
SFX_DEATH = 2
SFX_COLLECT = 3

-- Resolved once in _init from collision_types.json (ids there are stable,
-- but resolving by name keeps this file correct if the table ever changes).
COL_WALKABLE, COL_SOLID, COL_HAZARD, COL_PLATFORM, COL_RAMP_R, COL_RAMP_L = nil, nil, nil, nil, nil, nil

local function rect(x0, y0, x1, y1, col, spr)
  return { x0 = x0, y0 = y0, x1 = x1, y1 = y1, col = col, spr = spr }
end

-- Room tile rects are in ROOM-LOCAL tile coordinates (x 0-23, y 0-15).
-- paint_world() offsets them by each room's (col, row) * room size before
-- painting.
ROOMS = {
  [1] = {
    col = 0, row = 0,
    tiles = {
      rect(0, 14, 23, 14, "solid", SPR_GROUND_TOP),
      rect(0, 15, 23, 15, "solid", SPR_GROUND_FILL),
      rect(6, 10, 8, 10, "solid", SPR_GROUND_TOP), -- tutorial hop platform
      rect(15, 11, 17, 11, "solid", SPR_GROUND_TOP), -- second hop, a little lower
    },
    spawn = { x = 2 * TILE, y = 13 * TILE },
    berry = { x = 7 * TILE, y = 9 * TILE },
    flag = nil,
  },
  [2] = {
    col = 1, row = 0,
    tiles = {
      rect(0, 14, 5, 14, "solid", SPR_GROUND_TOP),
      rect(0, 15, 5, 15, "solid", SPR_GROUND_FILL),
      rect(8, 14, 10, 14, "solid", SPR_GROUND_TOP),
      rect(8, 15, 10, 15, "solid", SPR_GROUND_FILL),
      -- x11-12 deliberately open beneath the floating block below: a real
      -- descent shaft into room 6 (row 1) directly underneath, not just a
      -- hazard gap — the whole lower row is only reachable through here.
      rect(13, 14, 15, 14, "solid", SPR_GROUND_TOP),
      rect(13, 15, 15, 15, "solid", SPR_GROUND_FILL),
      rect(18, 14, 23, 14, "solid", SPR_GROUND_TOP),
      rect(18, 15, 23, 15, "solid", SPR_GROUND_FILL),
      rect(6, 15, 7, 15, "hazard", SPR_SPIKE),
      rect(16, 15, 17, 15, "hazard", SPR_SPIKE),
      rect(11, 11, 12, 11, "solid", SPR_GROUND_TOP),
    },
    spawn = { x = 1 * TILE, y = 13 * TILE },
    berry = { x = 11 * TILE, y = 10 * TILE },
    flag = nil,
  },
  [3] = {
    col = 2, row = 0,
    tiles = {
      rect(0, 14, 23, 14, "solid", SPR_CAVE_TOP),
      rect(0, 15, 23, 15, "solid", SPR_CAVE_FILL),
      rect(6, 4, 6, 13, "solid", SPR_CAVE_FILL),
      rect(9, 4, 9, 13, "solid", SPR_CAVE_FILL),
      -- "platform" (one-way) not "solid": a solid tile spanning both corridor
      -- columns would seal the wall-jump shaft shut with no gap to pass
      -- through on the way up; one-way only blocks descending, so the climb
      -- passes through it and the player lands on top on the way back down.
      rect(7, 3, 8, 3, "platform", SPR_PLATFORM),
      -- Descent route back to ground level on the right half of the room.
      rect(13, 11, 15, 11, "platform", SPR_PLATFORM),
      rect(18, 8, 20, 8, "solid", SPR_CAVE_TOP),
    },
    spawn = { x = 1 * TILE, y = 13 * TILE },
    berry = { x = 7 * TILE, y = 2 * TILE },
    flag = nil,
  },
  [4] = {
    col = 3, row = 0,
    tiles = {
      rect(0, 14, 13, 14, "solid", SPR_CAVE_TOP),
      rect(0, 15, 13, 15, "solid", SPR_CAVE_FILL),
      -- Extends to the room's right edge (the original 8-room map ended
      -- here, so this half stayed open) so the floor connects seamlessly
      -- into room 9's ruins entrance rather than dropping the walker into
      -- a void at the map's old boundary.
      rect(16, 14, 23, 14, "solid", SPR_CAVE_TOP),
      rect(16, 15, 23, 15, "solid", SPR_CAVE_FILL),
      rect(2, 10, 6, 10, "platform", SPR_PLATFORM),
      rect(9, 11, 11, 11, "platform", SPR_PLATFORM),
      rect(18, 11, 20, 11, "platform", SPR_PLATFORM),
    },
    spawn = { x = 1 * TILE, y = 13 * TILE },
    berry = { x = 4 * TILE, y = 9 * TILE },
    flag = nil,
  },
  [5] = {
    col = 0, row = 1,
    tiles = {
      rect(0, 14, 4, 14, "solid", SPR_GROUND_TOP),
      rect(0, 15, 4, 15, "solid", SPR_GROUND_FILL),
      rect(11, 14, 15, 14, "solid", SPR_GROUND_TOP),
      rect(11, 15, 15, 15, "solid", SPR_GROUND_FILL),
      rect(20, 14, 23, 14, "solid", SPR_GROUND_TOP),
      rect(20, 15, 23, 15, "solid", SPR_GROUND_FILL),
      rect(5, 15, 10, 15, "hazard", SPR_SPIKE),
      rect(16, 15, 19, 15, "hazard", SPR_SPIKE),
      -- Stepping stone: the second spike field is too wide for a flat jump.
      rect(17, 12, 18, 12, "platform", SPR_PLATFORM),
    },
    spawn = { x = 1 * TILE, y = 13 * TILE },
    berry = { x = 7 * TILE, y = 11 * TILE },
    flag = nil,
  },
  [6] = {
    col = 1, row = 1,
    tiles = {
      rect(0, 14, 6, 14, "solid", SPR_GROUND_TOP),
      rect(0, 15, 6, 15, "solid", SPR_GROUND_FILL),
      rect(7, 13, 7, 13, "ramp_right", SPR_SLOPE_RIGHT),
      rect(8, 13, 23, 13, "solid", SPR_GROUND_TOP),
      rect(8, 14, 23, 15, "solid", SPR_GROUND_FILL),
      rect(12, 13, 12, 13, "hazard", SPR_SPIKE),
      rect(19, 13, 19, 13, "hazard", SPR_SPIKE),
      rect(12, 10, 12, 10, "solid", SPR_GROUND_TOP),
      rect(16, 10, 17, 10, "platform", SPR_PLATFORM),
    },
    spawn = { x = 1 * TILE, y = 13 * TILE },
    berry = { x = 12 * TILE, y = 9 * TILE },
    flag = nil,
  },
  [7] = {
    col = 2, row = 1,
    tiles = {
      rect(0, 13, 3, 13, "solid", SPR_CAVE_TOP),
      rect(0, 14, 3, 15, "solid", SPR_CAVE_FILL),
      rect(4, 14, 4, 15, "solid", SPR_CAVE_TOP),
      rect(9, 14, 9, 15, "solid", SPR_CAVE_TOP),
      rect(4, 8, 4, 13, "solid", SPR_CAVE_FILL),
      rect(9, 8, 9, 13, "solid", SPR_CAVE_FILL),
      rect(6, 10, 7, 10, "platform", SPR_PLATFORM),
      rect(5, 15, 8, 15, "hazard", SPR_SPIKE),
      rect(10, 13, 23, 13, "solid", SPR_CAVE_TOP),
      rect(10, 14, 23, 15, "solid", SPR_CAVE_FILL),
      rect(17, 10, 19, 10, "platform", SPR_PLATFORM),
      rect(21, 12, 22, 12, "hazard", SPR_SPIKE),
    },
    spawn = { x = 1 * TILE, y = 12 * TILE },
    berry = { x = 6 * TILE, y = 8 * TILE },
    flag = nil,
  },
  [8] = {
    col = 3, row = 1,
    tiles = {
      rect(0, 13, 23, 13, "solid", SPR_CAVE_TOP),
      rect(0, 14, 23, 15, "solid", SPR_CAVE_FILL),
      rect(8, 15, 9, 15, "hazard", SPR_SPIKE),
      rect(10, 11, 10, 11, "solid", SPR_CAVE_TOP),
      rect(16, 11, 17, 11, "platform", SPR_PLATFORM),
    },
    spawn = { x = 1 * TILE, y = 12 * TILE },
    berry = { x = 10 * TILE, y = 10 * TILE },
    flag = nil,
  },
  [9] = {
    col = 4, row = 0,
    tiles = {
      rect(0, 14, 3, 14, "solid", SPR_RUIN_TOP),
      rect(0, 15, 3, 15, "solid", SPR_RUIN_FILL),
      -- x4-5 left open: the climbing shaft from room 11 (row 1) directly
      -- below arrives here.
      rect(6, 14, 13, 14, "solid", SPR_RUIN_TOP),
      rect(6, 15, 13, 15, "solid", SPR_RUIN_FILL),
      rect(16, 14, 23, 14, "solid", SPR_RUIN_TOP),
      rect(16, 15, 23, 15, "solid", SPR_RUIN_FILL),
      rect(14, 15, 15, 15, "hazard", SPR_SPIKE),
      rect(9, 10, 11, 10, "platform", SPR_PLATFORM),
      rect(18, 10, 19, 10, "platform", SPR_PLATFORM),
    },
    spawn = { x = 1 * TILE, y = 13 * TILE },
    berry = { x = 10 * TILE, y = 9 * TILE },
    flag = nil,
  },
  [10] = {
    col = 5, row = 0,
    tiles = {
      rect(0, 14, 23, 14, "solid", SPR_RUIN_TOP),
      rect(0, 15, 23, 15, "solid", SPR_RUIN_FILL),
      rect(4, 10, 4, 13, "solid", SPR_RUIN_FILL),
      rect(9, 10, 9, 13, "solid", SPR_RUIN_FILL),
      rect(5, 9, 8, 9, "platform", SPR_PLATFORM),
      rect(14, 11, 16, 11, "platform", SPR_PLATFORM),
    },
    spawn = { x = 1 * TILE, y = 13 * TILE },
    berry = { x = 6 * TILE, y = 8 * TILE },
    flag = nil,
  },
  [11] = {
    col = 4, row = 1,
    tiles = {
      rect(0, 14, 11, 14, "solid", SPR_RUIN_TOP),
      rect(13, 14, 17, 14, "solid", SPR_RUIN_TOP),
      rect(19, 14, 23, 14, "solid", SPR_RUIN_TOP),
      rect(0, 15, 11, 15, "solid", SPR_RUIN_FILL),
      rect(13, 15, 17, 15, "solid", SPR_RUIN_FILL),
      rect(19, 15, 23, 15, "solid", SPR_RUIN_FILL),
      rect(12, 15, 12, 15, "hazard", SPR_SPIKE),
      rect(18, 15, 18, 15, "hazard", SPR_SPIKE),
      -- Wall-jump climbing shaft (same technique as room 3) rising to the
      -- gap left open in room 9 directly above — the low road's way back
      -- up to the high road, not a dead end.
      rect(4, 2, 4, 13, "solid", SPR_RUIN_FILL),
      rect(7, 2, 7, 13, "solid", SPR_RUIN_FILL),
      rect(5, 2, 6, 2, "platform", SPR_PLATFORM),
    },
    spawn = { x = 1 * TILE, y = 13 * TILE },
    berry = { x = 15 * TILE, y = 13 * TILE },
    flag = nil,
  },
  [12] = {
    col = 5, row = 1,
    tiles = {
      rect(0, 14, 7, 14, "solid", SPR_RUIN_TOP),
      rect(0, 15, 7, 15, "solid", SPR_RUIN_FILL),
      rect(10, 14, 23, 14, "solid", SPR_RUIN_TOP),
      rect(10, 15, 23, 15, "solid", SPR_RUIN_FILL),
      rect(8, 15, 9, 15, "hazard", SPR_SPIKE),
      rect(13, 10, 15, 10, "platform", SPR_PLATFORM),
      rect(4, 9, 4, 13, "solid", SPR_RUIN_FILL),
    },
    spawn = { x = 1 * TILE, y = 13 * TILE },
    berry = { x = 5 * TILE, y = 12 * TILE },
    flag = nil,
  },
  [13] = {
    col = 6, row = 0,
    tiles = {
      -- Ruins give way to open sky: past this ledge there is no floor at
      -- all — falling off the world edge already means death/respawn
      -- (see _update's "playing" branch), so a bottomless sky reach is a
      -- real, intentional biome, not a gap that needs patching.
      rect(0, 14, 5, 14, "solid", SPR_RUIN_TOP),
      rect(0, 15, 5, 15, "solid", SPR_RUIN_FILL),
      rect(8, 11, 10, 11, "platform", SPR_CLOUD),
      rect(13, 9, 15, 9, "platform", SPR_CLOUD),
      rect(17, 12, 19, 12, "platform", SPR_CLOUD),
      rect(20, 9, 23, 9, "platform", SPR_CLOUD),
    },
    spawn = { x = 1 * TILE, y = 13 * TILE },
    berry = { x = 14 * TILE, y = 8 * TILE },
    flag = nil,
  },
  [14] = {
    col = 7, row = 0,
    tiles = {
      rect(1, 10, 3, 10, "platform", SPR_CLOUD),
      rect(6, 7, 8, 7, "platform", SPR_CLOUD),
      rect(11, 10, 13, 10, "platform", SPR_CLOUD),
      rect(16, 7, 18, 7, "platform", SPR_CLOUD),
      rect(20, 10, 23, 10, "platform", SPR_CLOUD),
    },
    spawn = { x = 1 * TILE, y = 13 * TILE },
    berry = { x = 7 * TILE, y = 6 * TILE },
    flag = nil,
  },
  [15] = {
    col = 6, row = 1,
    tiles = {
      rect(0, 14, 4, 14, "solid", SPR_RUIN_TOP),
      rect(0, 15, 4, 15, "solid", SPR_RUIN_FILL),
      rect(7, 11, 9, 11, "platform", SPR_CLOUD),
      rect(12, 8, 14, 8, "platform", SPR_CLOUD),
      rect(17, 11, 19, 11, "platform", SPR_CLOUD),
      rect(21, 13, 23, 13, "platform", SPR_CLOUD),
    },
    spawn = { x = 1 * TILE, y = 13 * TILE },
    berry = { x = 13 * TILE, y = 7 * TILE },
    flag = nil,
  },
  [16] = {
    col = 7, row = 1,
    tiles = {
      rect(0, 13, 3, 13, "platform", SPR_CLOUD),
      rect(6, 10, 8, 10, "platform", SPR_CLOUD),
      rect(11, 7, 13, 7, "platform", SPR_CLOUD),
      -- The finale: a small sky temple to plant the flag on.
      rect(16, 10, 20, 10, "solid", SPR_RUIN_TOP),
      rect(16, 11, 20, 15, "solid", SPR_RUIN_FILL),
    },
    spawn = { x = 1 * TILE, y = 13 * TILE },
    berry = { x = 7 * TILE, y = 9 * TILE },
    flag = { x = 18 * TILE, y = 9 * TILE },
  },
}

TOTAL_BERRIES = 0
for _, room in ipairs(ROOMS) do
  if room.berry then TOTAL_BERRIES = TOTAL_BERRIES + 1 end
end

local function paint_world()
  for _, room in ipairs(ROOMS) do
    local ox, oy = room.col * ROOM_TILES_W, room.row * ROOM_TILES_H
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
  local col = math.floor(px / ROOM_PX_W)
  local row = math.floor(py / ROOM_PX_H)
  for _, room in ipairs(ROOMS) do
    if room.col == col and room.row == row then return room end
  end
  return nil
end

-- Screen shake: a brief camera jitter on hard impacts (death, landing hard).
-- Pure hand-tuned juice on top of the existing set_camera call, not a new
-- module or builtin.
SHAKE_TIMER, SHAKE_AMOUNT = 0, 0

function start_shake(amount, duration)
  SHAKE_AMOUNT = amount
  SHAKE_TIMER = duration
end

function update_camera(px, py)
  -- physics_update can push the player past the grid's outer edge for a
  -- single frame before the nil-room death check (which runs before this,
  -- on the prior frame's position) catches it next frame — clamp so
  -- set_camera (u32 args) never sees a negative or out-of-grid coordinate.
  local col = math.max(0, math.min(7, math.floor(px / ROOM_PX_W)))
  local row = math.max(0, math.min(1, math.floor(py / ROOM_PX_H)))
  local cx, cy = col * ROOM_PX_W, row * ROOM_PX_H
  if SHAKE_TIMER > 0 then
    cx = math.max(0, cx + math.floor(random_float(-SHAKE_AMOUNT, SHAKE_AMOUNT)))
    cy = math.max(0, cy + math.floor(random_float(-SHAKE_AMOUNT, SHAKE_AMOUNT)))
    SHAKE_TIMER = SHAKE_TIMER - 1
  end
  set_camera(cx, cy)
end

-- 16-color palette: ink outline + white + four hue ramps (stone, foliage,
-- wood/player, ember/hazard) at three shades each, plus two accents (sky
-- backdrop, gold) — see scripts/demo-carts/gen_platformer_assets.py's
-- PALETTE table, which this must stay in sync with (sprite pixel indices
-- there assume these exact slot colors).
local function set_palette()
  set_palette_color(0, 168, 186, 214)  -- sky (accent)
  set_palette_color(1, 24, 20, 28)     -- ink
  set_palette_color(2, 240, 236, 224)  -- white
  set_palette_color(3, 58, 56, 72)     -- stone dark
  set_palette_color(4, 104, 100, 122)  -- stone mid
  set_palette_color(5, 158, 154, 170)  -- stone light
  set_palette_color(6, 32, 68, 46)     -- foliage dark
  set_palette_color(7, 60, 120, 74)    -- foliage mid
  set_palette_color(8, 132, 190, 108)  -- foliage light
  set_palette_color(9, 92, 54, 34)     -- wood dark
  set_palette_color(10, 168, 96, 48)   -- wood mid
  set_palette_color(11, 230, 162, 90)  -- wood light
  set_palette_color(12, 96, 24, 26)    -- ember dark
  set_palette_color(13, 188, 46, 40)   -- ember mid
  set_palette_color(14, 236, 108, 52)  -- ember light
  set_palette_color(15, 255, 214, 88)  -- gold (accent)
end

-- ROOMS[n].spawn/berry/flag are authored in the same ROOM-LOCAL tile-pixel
-- space as ROOMS[n].tiles (see paint_world's ox/oy offset) — this converts
-- one local {x,y} point to absolute world pixel coordinates, the space
-- player.pos/room_at/update_camera all operate in.
local function room_point(room, local_pt)
  return { x = local_pt.x + room.col * ROOM_PX_W, y = local_pt.y + room.row * ROOM_PX_H }
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

  reset_game()
end

local function player_touches_flag()
  local flag = ROOMS[16].flag
  if not flag then return false end
  local wp = room_point(ROOMS[16], flag)
  return aabb_overlap(player.pos.x, player.pos.y, player.w, player.h,
    wp.x, wp.y, 8, 8)
end

FLAG_ANIM = nil

function reset_game()
  GAME = { mode = "title", deaths = 0, berries = 0, last_room = ROOMS[1] }
  spawn_player(room_point(ROOMS[1], ROOMS[1].spawn))
  spawn_berries()
  stop_music()
  FLAG_ANIM = new_anim({ SPR_FLAG_A, SPR_FLAG_B }, 20)
end

function spawn_berries()
  Entities.clear()
  for _, room in ipairs(ROOMS) do
    if room.berry then
      local wp = room_point(room, room.berry)
      Entities.add({
        pos = Vec2.new(wp.x, wp.y),
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
        Particles.spawn(e.pos.x + 4, e.pos.y + 4, math.cos(a) * 1.2, math.sin(a) * 1.2, 15, 16)
      end
    end
  end
  Entities.update_all()
end

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
  start_shake(3, 14)
  for i = 1, 12 do
    local a = (i / 12) * 6.28318
    Particles.spawn(player.pos.x + player.w / 2, player.pos.y + player.h / 2,
      math.cos(a) * 1.5, math.sin(a) * 1.5, 13, 18)
  end
  GAME.deaths = GAME.deaths + 1
end

local function handle_dying()
  Particles.update()
  GAME.dying_timer = GAME.dying_timer - 1
  if GAME.dying_timer <= 0 then
    local room = room_at(player.pos.x, player.pos.y) or GAME.last_room
    spawn_player(room_point(room, room.spawn))
    GAME.mode = "playing"
  end
end

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
WALL_SLIDE_MAX = 1.0
WALLJUMP_VX = 2.2
WALLJUMP_VY = -4.6
WALLJUMP_LOCK = 10
DASH_SPEED = 3.5
DASH_FRAMES = 10

function spawn_player(spawn)
  player = {
    pos = Vec2.new(spawn.x, spawn.y),
    vx = 0, vy = 0,
    w = PLAYER_W, h = PLAYER_H,
    facing = 1,
    on_ground = false,
    coyote_timer = 0,
    jump_buffer = 0,
    wall_dir = 0,
    walljump_lock = 0,
    dashes = 1, dash_timer = 0, dashing = false, dash_vx = 0, dash_vy = 0,
    anim = new_anim({ SPR_PLAYER_RUN1, SPR_PLAYER_RUN3, SPR_PLAYER_RUN2, SPR_PLAYER_RUN3 }, 6),
  }
end

local function player_horizontal(input)
  if player.walljump_lock > 0 then return end
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

local function player_move_and_collide()
  local nx, ny_step, htouch = move_and_collide(player.pos.x, player.pos.y, player.w, player.h, player.vx, 0)
  player.pos.x = nx
  -- A step-up (climbing onto a one-tile ledge or a slope's tall edge)
  -- comes back as a raised y from the horizontal pass itself — apply it
  -- before the vertical pass runs, so that pass starts from the stepped-up
  -- height instead of re-deadlocking against the same ledge.
  player.pos.y = ny_step
  if htouch.left then player.wall_dir = -1
  elseif htouch.right then player.wall_dir = 1
  else player.wall_dir = 0 end

  local _, ny, touch = move_and_collide(player.pos.x, player.pos.y, player.w, player.h, 0, player.vy)
  player.pos.y = ny

  if touch.ground then
    if not player.on_ground then
      player.coyote_timer = COYOTE_MAX
      -- Hard-landing juice: a small camera jab and a puff of dust, only
      -- when falling fast enough for it to read as an impact.
      if player.vy > 3.0 then
        start_shake(1, 6)
        for i = 1, 4 do
          Particles.spawn(player.pos.x + player.w / 2, player.pos.y + player.h,
            random_float(-0.6, 0.6), random_float(-0.6, -0.1), 4, 10)
        end
      end
    end
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
      -player.dash_vx * 0.3, -player.dash_vy * 0.3, 11, 12)
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

local function read_input()
  return {
    left = button_down(2), right = button_down(3),
    up = button_down(0), down = button_down(1),
    jump_pressed = button_pressed(4), jump_released = button_released(4),
    dash_pressed = button_pressed(5),
  }
end

function _update()
  if GAME.mode == "title" then
    if button_pressed(4) then
      GAME.mode = "playing"
      play_music_song()
    end
    return
  end

  if GAME.mode == "playing" then
    local room = room_at(player.pos.x, player.pos.y)
    if room then
      GAME.last_room = room
    else
      -- Left the defined 4x2 room grid entirely (e.g. dashed/fell past a
      -- room's outer edge) — treat like a hazard: die and respawn rather
      -- than free-falling forever or crashing _draw's room lookup below.
      start_dying()
      return
    end
    physics_update(read_input())
    update_berries()
    anim_update(FLAG_ANIM)
    update_camera(player.pos.x, player.pos.y)
    if player_touches_hazard() then start_dying() end
    if player_touches_flag() then
      GAME.mode = "won"
      stop_music()
    end
    return
  end

  if GAME.mode == "dying" then
    handle_dying()
    return
  end

  if GAME.mode == "won" then
    if button_pressed(4) then reset_game() end
    return
  end
end

-- Picks the player's sprite from movement state, in priority order: a
-- dedicated frame beats reusing idle/run — dash and wall-slide read poorly
-- with the wrong pose, so they're checked before the general airborne case.
--
-- Resting contact on a tile boundary toggles move_and_collide's `touch.ground`
-- false for a frame or two even while standing still (the physics step
-- resolves it every few frames, not every single one) — coyote_timer already
-- exists to paper over exactly this gap for jump permission, so animation
-- reuses it here too, rather than flickering into the fall pose each time
-- raw on_ground drops for a frame.
local function player_sprite()
  if GAME.mode == "dying" then return SPR_PLAYER_DEATH end
  if player.dashing then return SPR_PLAYER_DASH end
  local grounded = player.on_ground or player.coyote_timer > 0
  if not grounded and player.wall_dir ~= 0 and player.vy > 0 then return SPR_PLAYER_WALL_SLIDE end
  if not grounded then
    return player.vy < 0 and SPR_PLAYER_JUMP or SPR_PLAYER_FALL
  end
  if math.abs(player.vx) > 0.1 then return anim_sprite(player.anim) end
  return SPR_PLAYER_IDLE
end

function _draw()
  clear_screen()
  if GAME.mode == "title" then
    fill_screen(COLOR_SKY)
    draw_text("PLATFORMER", 76, 50, 15)
    draw_text("PRESS A", 82, 66, 2)
    return
  end
  if GAME.mode == "won" then
    fill_screen(COLOR_SKY)
    draw_text("YOU WIN", 82, 40, 15)
    draw_text("DEATHS " .. GAME.deaths, 80, 56, 2)
    draw_text("BERRIES " .. GAME.berries .. "/" .. TOTAL_BERRIES, 74, 68, 2)
    draw_text("PRESS A", 82, 84, 2)
    return
  end
  -- Falls back to the last known room for one frame if the player is
  -- momentarily outside every room's bounds (see the "playing" branch in
  -- _update, which kills and respawns them before the next frame).
  local room = room_at(player.pos.x, player.pos.y) or GAME.last_room
  local ox, oy = room.col * ROOM_TILES_W, room.row * ROOM_TILES_H
  fill_screen(COLOR_SKY)
  draw_map(ox, oy, ox * TILE, oy * TILE, ROOM_TILES_W, ROOM_TILES_H)
  Particles.draw()
  sprite(player_sprite(), math.floor(player.pos.x), math.floor(player.pos.y), player.facing < 0)
  for _, e in ipairs(Entities.list) do
    if e.is_berry and e.room == room then
      sprite(SPR_BERRY, math.floor(e.pos.x), math.floor(e.pos.y))
    end
  end
  if room.flag then
    local wp = room_point(room, room.flag)
    sprite(anim_sprite(FLAG_ANIM), wp.x, wp.y)
  end
  if GAME.mode == "playing" or GAME.mode == "dying" then
    draw_text("DEATHS " .. GAME.deaths .. "  BERRIES " .. GAME.berries .. "/" .. TOTAL_BERRIES, 2, 2, 2)
  end
end
