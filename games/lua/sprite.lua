-- Sprite viewer — shows the sprite sheet at center of screen
-- Open the FC Studio sprite tab (F2) to paint sprites, switch back to see them live
local SPR_X, SPR_Y = 60, 60

function _init()
  set_palette_color(0, 0, 0, 0)
  set_palette_color(1, 255, 80, 80)
  set_palette_color(2, 80, 255, 80)
  set_palette_color(3, 80, 80, 255)
  set_palette_color(4, 255, 255, 80)
  set_palette_color(5, 255, 160, 40)
  set_palette_color(6, 200, 80, 255)
  set_palette_color(7, 255, 255, 255)
end

function _update()
  clear_screen()
  sprite(0, SPR_X, SPR_Y)
end

__gfx__
00000101010100000001010101010100010102010102010101010101010101010101030303010101010101030101010100010101010101000000010101000000
