-- Catch the Fruit — move the player onto the falling fruit before time runs out

local INIT_POS = 60
local INIT_TIMER = 100
local HALF_SPR = 4

local FRUIT_X_MIN, FRUIT_X_RANGE = 10, 100
local FRUIT_Y_MIN, FRUIT_Y_RANGE = 20, 80

local player_x, player_y = INIT_POS, INIT_POS
local fruit_x, fruit_y = 0, 0
local score = 0
local timer = INIT_TIMER

local function spawn_fruit()
  fruit_x = FRUIT_X_MIN + math.random(0, FRUIT_X_RANGE - 1)
  fruit_y = FRUIT_Y_MIN + math.random(0, FRUIT_Y_RANGE - 1)
  timer = INIT_TIMER
end

function _init()
  set_palette_color(0, 0, 0, 0)
  set_palette_color(1, 255, 255, 255)
  set_palette_color(2, 255, 50, 50)
  set_palette_color(3, 50, 255, 50)

  player_x, player_y = INIT_POS, INIT_POS
  score = 0
  spawn_fruit()
  play_music(0)
end

function _update()
  clear_screen()

  draw_text("SCORE:", 5, 5, 1)
  draw_number(score, 45, 5, 3)
  draw_text("TIME:", 75, 5, 1)
  draw_number(timer, 105, 5, 3)

  sprite(0, player_x, player_y)
  sprite(1, fruit_x, fruit_y)

  if button_down(2) then player_x = player_x - 1 end
  if button_down(3) then player_x = player_x + 1 end
  if button_down(0) then player_y = player_y - 1 end
  if button_down(1) then player_y = player_y + 1 end

  local overlap_x = player_x + HALF_SPR > fruit_x and fruit_x + HALF_SPR > player_x
  local overlap_y = player_y + HALF_SPR > fruit_y and fruit_y + HALF_SPR > player_y
  if overlap_x and overlap_y then
    play_sfx(0)
    score = score + 1
    spawn_fruit()
  end

  timer = timer - 1
  if timer <= 0 then
    play_sfx(1)
    _init()
  end
end

__gfx__
00010101010101000101010101010101010001010101000101010101010101010101000000010101010101000101010100010101010101000000010101000000
00000003030000000000020202020000000202020202020002020202020202020202020202020202000202020202020000000202020200000000000000000000

__sfx__
310e0000
380e0000
00000000
00000000
00000000
00000000
00000000
00000000
00000000
00000000
00000000
00000000
00000000
00000000
00000000
00000000
2d0c0000
280c0000
230c0000
1e0c0000
00000000
00000000
00000000
00000000
00000000
00000000
00000000
00000000
00000000
00000000
00000000
00000000
19080000
19080000
19080000
19080000
19080000
19080000
19080000
19080000
19080000
19080000
19080000
19080000
19080000
19080000
19080000
19080000
14080000
14080000
14080000
14080000
14080000
14080000
14080000
14080000
14080000
14080000
14080000
14080000
14080000
14080000
14080000
14080000

__music__
0300
0300
0400
0300
0300
0300
0400
0300
0300
0300
0400
0300
0300
0300
0400
0300
