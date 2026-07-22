x = 64
y = 64

function _update()
  clear_screen()
  if button_down(0) then y = y - 1 end
  if button_down(1) then y = y + 1 end
  if button_down(2) then x = x - 1 end
  if button_down(3) then x = x + 1 end
  set_pixel(x, y, 8)
  draw_text("mlua phase a", 4, 4, 7)
end
