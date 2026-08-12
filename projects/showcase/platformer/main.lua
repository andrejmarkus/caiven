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

  reset_game()
end

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

function spawn_berries()
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
    local room = room_at(player.pos.x, player.pos.y) or GAME.last_room
    spawn_player(room.spawn)
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
    anim = new_anim({ SPR_PLAYER_RUN1, SPR_PLAYER_IDLE, SPR_PLAYER_RUN2, SPR_PLAYER_IDLE }, 8),
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
      play_music(MUSIC_MAIN)
    end
    return
  end

  if GAME.mode == "playing" then
    local room = room_at(player.pos.x, player.pos.y)
    if room then GAME.last_room = room end
    physics_update(read_input())
    update_berries()
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

function _draw()
  clear_screen()
  if GAME.mode == "title" then
    draw_text("PLATFORMER", 46, 50, 14)
    draw_text("PRESS A", 46, 66, 7)
    return
  end
  if GAME.mode == "won" then
    draw_text("YOU WIN", 44, 40, 14)
    draw_text("DEATHS " .. GAME.deaths, 40, 56, 7)
    draw_text("BERRIES " .. GAME.berries .. "/8", 38, 68, 7)
    draw_text("PRESS A", 46, 84, 7)
    return
  end
  local room = room_at(player.pos.x, player.pos.y)
  local ox, oy = room.col * ROOM_TILES, room.row * ROOM_TILES
  draw_map(ox, oy, ox * TILE, oy * TILE, ROOM_TILES, ROOM_TILES)
  Particles.draw()
  local frame = player.on_ground and math.abs(player.vx) > 0.1 and anim_sprite(player.anim) or SPR_PLAYER_IDLE
  sprite(frame, math.floor(player.pos.x), math.floor(player.pos.y), player.facing < 0)
  for _, e in ipairs(Entities.list) do
    if e.is_berry and e.room == room then
      sprite(SPR_BERRY, math.floor(e.pos.x), math.floor(e.pos.y))
    end
  end
  if room.flag then
    sprite(SPR_FLAG, room.flag.x, room.flag.y)
  end
  if GAME.mode == "playing" or GAME.mode == "dying" then
    draw_text("DEATHS " .. GAME.deaths .. "  BERRIES " .. GAME.berries .. "/8", 2, 2, 14)
  end
end
