-- Simple movement demo — arrow keys move sprite 0 around the screen
local SPEED = 2

local x = 60
local y = 60

function _init()
  set_palette_color(0, 10, 10, 30)
  set_palette_color(1, 200, 200, 255)
  set_palette_color(2, 255, 220, 80)
  set_palette_color(3, 255, 80, 80)
end

function _update()
  clear_screen()
  if button_down(0) then y = y - SPEED end
  if button_down(1) then y = y + SPEED end
  if button_down(2) then x = x - SPEED end
  if button_down(3) then x = x + SPEED end
  sprite(0, x, y)
end

__gfx__
00000101010100000001010202010100010102010102010101010101010101010101030303010101010101030101010100010101010101000000010101000000
