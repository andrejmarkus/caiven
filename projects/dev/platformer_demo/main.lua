GROUND_TY = 15
LEDGE_TY = 14
PLATFORM_TY = 10

function _init()
  local ground = collision_type_id("solid")
  local platform = collision_type_id("platform")
  local ramp_right = collision_type_id("ramp_right")
  local ramp_left = collision_type_id("ramp_left")

  for tx = 0, 7 do
    set_tile(tx, GROUND_TY, 1)
    set_collision(tx, GROUND_TY, ground)
  end
  set_tile(8, LEDGE_TY, 1)
  set_collision(8, LEDGE_TY, ramp_right)
  for tx = 9, 11 do
    set_tile(tx, LEDGE_TY, 1)
    set_collision(tx, LEDGE_TY, ground)
  end
  set_tile(12, LEDGE_TY, 1)
  set_collision(12, LEDGE_TY, ramp_left)
  for tx = 13, 15 do
    set_tile(tx, GROUND_TY, 1)
    set_collision(tx, GROUND_TY, ground)
  end
  for tx = 2, 4 do
    set_tile(tx, PLATFORM_TY, 1)
    set_collision(tx, PLATFORM_TY, platform)
  end

  player = { pos = Vec2.new(16, GROUND_TY * SPRITE_SIZE - 8), w = 8, h = 8, vy = 0 }

  Entities.clear()
  Entities.add({ pos = Vec2.new(24, PLATFORM_TY * SPRITE_SIZE - 8), w = 8, h = 8, is_coin = true })

  Camera.follow(player, { lerp = 1 })
end

function _update()
  local dx = 0
  if button_down(2) then dx = dx - 2 end
  if button_down(3) then dx = dx + 2 end

  local nx = move_and_collide(player.pos.x, player.pos.y, player.w, player.h, dx, 0)
  player.pos.x = nx

  local _, ny, touch = move_and_collide(player.pos.x, player.pos.y, player.w, player.h, 0, player.vy)
  player.pos.y = ny
  if touch.ground then
    player.vy = 0
    if button_pressed(4) then player.vy = -6 end
  elseif touch.ceiling then
    player.vy = 0
  else
    player.vy = clamp(player.vy + 1, -6, 4)
  end

  for _, e in ipairs(Entities.overlapping(player.pos.x, player.pos.y, player.w, player.h)) do
    if e.is_coin then e.dead = true end
  end
  Entities.update_all()

  Camera.update()
end

function _draw()
  clear_screen()
  for ty = 0, 15 do
    for tx = 0, 15 do
      local id = get_collision(tx, ty)
      if id ~= 0 then
        local color = 6
        if collision_is_one_way(id) then
          color = 11
        elseif collision_is_slope_left(id) or collision_is_slope_right(id) then
          color = 10
        elseif collision_is_solid(id) then
          color = 5
        end
        fill_rect(tx * SPRITE_SIZE, ty * SPRITE_SIZE, SPRITE_SIZE, SPRITE_SIZE, color)
      end
    end
  end
  for _, e in ipairs(Entities.list) do
    fill_rect(math.floor(e.pos.x), math.floor(e.pos.y), e.w, e.h, 9)
  end
  fill_rect(math.floor(player.pos.x), math.floor(player.pos.y), player.w, player.h, 8)
  draw_text("ARROWS MOVE  A JUMP", 2, 2, 7)
end
