-- Table demo: create, read, write table fields

local t
local score = 0

function _init()
  t = {x = 60, y = 60, dx = 1, dy = 1}
end

function _update()
  clear_screen()

  if t.x >= 120 then t.dx = -1 end
  if t.x <= 0 then t.dx = 1 end
  if t.y >= 80 then t.dy = -1 end
  if t.y <= 0 then t.dy = 1 end

  t.x = t.x + t.dx
  t.y = t.y + t.dy

  score = score + 1

  draw_number(score, 2, 2, 7)
end
