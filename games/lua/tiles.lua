-- Tile maze — walk through a maze with collision detection
-- Sprite 0: player, sprite 1: floor, sprite 2: wall (marked solid via sprite flags)
local MAZE_W, MAZE_H = 16, 16
local FLAG_SOLID = 1

local maze = {
  "2222222222222222",
  "2111111111111112",
  "2122222222222212",
  "2121111111111212",
  "2121222222221212",
  "2121211111121212",
  "2121212222121212",
  "2121212112121212",
  "2121212112121212",
  "2121212222121212",
  "2121211111121212",
  "2121222222221212",
  "2121111111111212",
  "2122222222222212",
  "2111111111111112",
  "2222222222222222",
}

local player_x, player_y = 8, 8

local function solid_at(px, py)
  local cx = math.floor(px / 8)
  local cy = math.floor(py / 8)
  return get_sprite_flags(get_tile(cx, cy)) == FLAG_SOLID
end

function _init()
  set_palette_color(0, 0, 0, 0)
  set_palette_color(1, 60, 60, 60)
  set_palette_color(2, 120, 120, 120)
  set_palette_color(3, 255, 100, 100)
  set_palette_color(4, 200, 200, 200)

  set_sprite_flags(2, FLAG_SOLID)

  for y = 0, MAZE_H - 1 do
    for x = 0, MAZE_W - 1 do
      set_tile(x, y, tonumber(maze[y + 1]:sub(x + 1, x + 1)))
    end
  end

  player_x, player_y = 8, 8
end

function _update()
  clear_screen()

  draw_map(0, 0, 0, 0, MAZE_W, MAZE_H)
  sprite(0, player_x, player_y)

  if button_down(2) and not solid_at(player_x - 1, player_y) and not solid_at(player_x - 1, player_y + 7) then
    player_x = player_x - 1
  end
  if button_down(3) and not solid_at(player_x + 8, player_y) and not solid_at(player_x + 8, player_y + 7) then
    player_x = player_x + 1
  end
  if button_down(0) and not solid_at(player_x, player_y - 1) and not solid_at(player_x + 7, player_y - 1) then
    player_y = player_y - 1
  end
  if button_down(1) and not solid_at(player_x, player_y + 8) and not solid_at(player_x + 7, player_y + 8) then
    player_y = player_y + 1
  end
end

__gfx__
00000303030300000003030303030300030300030300030303030303030303030303000303000303030003030303000300030300000303000000030303030000
01010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101
02020202020202020204040404040402020402020202040202040202020204020204020202020402020402020202040202040404040404020202020202020202
