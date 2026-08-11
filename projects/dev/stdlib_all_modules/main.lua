-- Exercises every opt-in prelude module at once: [stdlib] modules =
-- ["vec2", "collision", "tween", "particles", "scenes", "entities", "camera"].

local pos
local tw
local player

function _init()
  set_palette_color(0, 8, 8, 16)
  set_palette_color(1, 220, 220, 255)
  set_palette_color(2, 255, 120, 90)

  pos = Vec2.new(10, 64)
  tw = new_tween(10, 108, 90, ease_out_quad)

  player = { pos = Vec2.new(64, 64), dead = false }
  Entities.add(player)
  Camera.follow(player, { lerp = 0.15 })

  Scenes.push({
    update = function()
      Entities.update_all()
      pos.x = tween_update(tw)
      if tw.done then
        Particles.spawn(pos.x, pos.y, 0, -1, 2, 20)
        tw = new_tween(10, 108, 90, ease_out_quad)
      end
      Particles.update()
      Camera.update()
    end,
    draw = function()
      clear_screen()
      if box_touches_solid(pos.x - 2, pos.y - 2, 4, 4) then
        set_pixel(math.floor(pos.x), math.floor(pos.y), 2)
      else
        set_pixel(math.floor(pos.x), math.floor(pos.y), 1)
      end
      Particles.draw()
      Entities.draw_all()
    end,
  })
end

function _update()
  Scenes.update()
end

function _draw()
  Scenes.draw()
end
