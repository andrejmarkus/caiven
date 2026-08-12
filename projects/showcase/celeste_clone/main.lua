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
