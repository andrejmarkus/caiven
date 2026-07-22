-- Audio test — press buttons to trigger SFX bank slots
-- UP: slot 0   DOWN: slot 1   LEFT: slot 2   RIGHT: slot 3
-- Paint sounds into these slots in the FC Studio SFX tab (F4)

function _init()
  set_palette_color(0, 10, 10, 20)
  set_palette_color(1, 255, 255, 255)
end

function _update()
  clear_screen()

  draw_text("UP: HIGH SND", 4, 20, 1)
  draw_text("DOWN: LOW SND", 4, 36, 1)
  draw_text("LEFT: NOISE", 4, 52, 1)
  draw_text("RIGHT: NOISE 2", 4, 68, 1)

  if button_pressed(0) then play_sfx(0) end
  if button_pressed(1) then play_sfx(1) end
  if button_pressed(2) then play_sfx(2) end
  if button_pressed(3) then play_sfx(3) end
end

__sfx__
3d0c0000
3d0c0000
3d0c0000
3d0c0000
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
250c0000
250c0000
250c0000
250c0000
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
370c0100
370c0100
370c0100
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
3c0a0100
370a0100
320a0100
2d0a0100
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

