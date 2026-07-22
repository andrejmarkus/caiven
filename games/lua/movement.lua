-- Simple movement demo
-- Draw sprite 0 in the sprite editor (F2) to see the player
local SPEED = 2

local x = 60
local y = 60

function _init()
  set_palette_color(0, 10, 10, 30)
end

function _update()
  clear_screen()
  if button_down(0) then y = y - SPEED end
  if button_down(1) then y = y + SPEED end
  if button_down(2) then x = x - SPEED end
  if button_down(3) then x = x + SPEED end
  sprite(0, x, y)
end
