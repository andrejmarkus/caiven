-- Core-only: no [stdlib] modules declared. Every name used below (clamp,
-- lerp, easing, random_range, draw_text/draw_number/set_pixel/button_down)
-- is either always-on prelude core or a console builtin -- no opt-in module
-- needed for a cart this simple.

local t = 0
local seed_x, seed_y

function _init()
  set_palette_color(0, 12, 12, 24)
  set_palette_color(1, 255, 220, 90)
  set_palette_color(2, 90, 200, 255)
  seed_x = random_range(20, 108)
  seed_y = random_range(20, 108)
end

function _update()
  clear_screen()
  t = t + 1
  local phase = (t % 120) / 120
  local x = lerp(10, 118, ease_in_out_quad(phase))
  local y = clamp(64 + math.sin(t / 20) * 40, 4, 124)
  set_pixel(math.floor(x), math.floor(y), 1)
  set_pixel(seed_x, seed_y, 2)
  draw_text("core only", 2, 2, 1)
  draw_number(t, 100, 2, 1)
end
