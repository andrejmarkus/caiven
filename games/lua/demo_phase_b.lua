for y = 0, 15 do
  for x = 0, 15 do
    set_tile(x, y, (x + y) % 2)
  end
end
set_sprite_flags(1, 1)

x = 64
y = 64
t = 0

function _update()
  t = t + 1
  clear_screen()
  fill_screen(1)

  set_camera(0, 0)
  draw_map(0, 0, 0, 0, 16, 8)

  if button_down(0) then y = y - 1 end
  if button_down(1) then y = y + 1 end
  if button_down(2) then x = x - 1 end
  if button_down(3) then x = x + 1 end
  if button_pressed(4) then play_sfx(0) end

  draw_rect(4, 20, 20, 12, 8)
  fill_rect(30, 20, 20, 12, 9)
  draw_circle(70, 26, 8, 11)
  fill_circle(100, 26, 8, 12)
  draw_line(4, 40, 120, 40, 7)

  sprite(1, x, y)
  draw_text("phase b", 4, 4, 7)
  draw_number(t, 90, 4, 7)

  if get_tile(0, 0) == 0 then
    set_pixel(2, 50, 10)
  end
  if get_sprite_flags(1) == 1 then
    set_pixel(4, 50, 11)
  end
end
