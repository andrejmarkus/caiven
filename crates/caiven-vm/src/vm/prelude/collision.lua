function aabb_overlap(x1, y1, w1, h1, x2, y2, w2, h2)
  return x1 < x2 + w2 and x2 < x1 + w1 and y1 < y2 + h2 and y2 < y1 + h1
end

function circle_overlap(x1, y1, r1, x2, y2, r2)
  local dx = x2 - x1
  local dy = y2 - y1
  local r = r1 + r2
  return dx * dx + dy * dy < r * r
end

function point_in_rect(px, py, x, y, w, h)
  return px >= x and px < x + w and py >= y and py < y + h
end

function point_in_circle(px, py, cx, cy, r)
  local dx = px - cx
  local dy = py - cy
  return dx * dx + dy * dy <= r * r
end

function tile_solid(tx, ty)
  return collision_is_solid(get_collision(tx, ty))
end

function box_touches_solid(x, y, w, h)
  local ss = SPRITE_SIZE
  local tx0 = math.floor(x / ss)
  local ty0 = math.floor(y / ss)
  local tx1 = math.floor((x + w - 1) / ss)
  local ty1 = math.floor((y + h - 1) / ss)
  for ty = ty0, ty1 do
    for tx = tx0, tx1 do
      if tile_solid(tx, ty) then return true end
    end
  end
  return false
end
