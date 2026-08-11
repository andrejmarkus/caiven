
function _update() end

function _draw()
  clear_screen()
  fill_screen(1)

  draw_text("flip_x", 4, 4, 7)
  sprite(0, 4, 14)
  sprite(0, 20, 14, true, false)

  draw_text("flip_y", 4, 34, 7)
  sprite(0, 4, 44)
  sprite(0, 20, 44, false, true)

  draw_text("rotate", 4, 64, 7)
  sprite(0, 4, 74, false, false, 0)
  sprite(0, 20, 74, false, false, 90)
  sprite(0, 36, 74, false, false, 180)
  sprite(0, 52, 74, false, false, 270)
end
