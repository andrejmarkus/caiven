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

local function solid_blocks_column(tx, ty0, ty1)
  for ty = ty0, ty1 do
    if collision_is_solid(get_collision(tx, ty)) then return true end
  end
  return false
end

local function solid_blocks_row(ty, tx0, tx1)
  for tx = tx0, tx1 do
    if collision_is_solid(get_collision(tx, ty)) then return true end
  end
  return false
end

-- Sample the highest (smallest-y) slope floor under [x0, x1) at map row ty,
-- or nil if no slope tile in that row covers the span.
local function slope_floor_y(ty, x0, x1, ss)
  local best = nil
  local tx0 = math.floor(x0 / ss)
  local tx1 = math.floor((x1 - 1) / ss)
  for tx = tx0, tx1 do
    local id = get_collision(tx, ty)
    local is_left = collision_is_slope_left(id)
    local is_right = collision_is_slope_right(id)
    if is_left or is_right then
      local col_x0 = math.max(x0, tx * ss)
      local col_x1 = math.min(x1, (tx + 1) * ss)
      for px = col_x0, col_x1 - 1 do
        local lx = px - tx * ss
        local floor_y_in_tile = is_right and (ss - 1 - lx) or lx
        local floor_y = ty * ss + floor_y_in_tile
        if best == nil or floor_y < best then best = floor_y end
      end
    end
  end
  return best
end

-- Axis-separated swept move + resolve against SOLID/ONE_WAY/slope tiles.
-- Returns nx, ny, touch = { ground, ceiling, left, right }.
function move_and_collide(x, y, w, h, dx, dy)
  local ss = SPRITE_SIZE
  local touch = { ground = false, ceiling = false, left = false, right = false }

  -- Horizontal: SOLID only. Slopes/one-way never block horizontal movement.
  local nx = x + dx
  if dx > 0 then
    local tx = math.floor((nx + w - 1) / ss)
    local ty0 = math.floor(y / ss)
    local ty1 = math.floor((y + h - 1) / ss)
    if solid_blocks_column(tx, ty0, ty1) then
      nx = tx * ss - w
      touch.right = true
    end
  elseif dx < 0 then
    local tx = math.floor(nx / ss)
    local ty0 = math.floor(y / ss)
    local ty1 = math.floor((y + h - 1) / ss)
    if solid_blocks_column(tx, ty0, ty1) then
      nx = (tx + 1) * ss
      touch.left = true
    end
  end

  -- Vertical: SOLID, then ONE_WAY (descending-from-above only), then slopes.
  local ny = y + dy
  if dy > 0 then
    local prev_bottom = y + h
    local new_bottom = ny + h
    local ty = math.floor((new_bottom - 1) / ss)
    local tx0 = math.floor(nx / ss)
    local tx1 = math.floor((nx + w - 1) / ss)

    if solid_blocks_row(ty, tx0, tx1) then
      ny = ty * ss - h
      touch.ground = true
    else
      local platform_top = ty * ss
      local one_way_hit = false
      for tx = tx0, tx1 do
        local id = get_collision(tx, ty)
        if collision_is_one_way(id) and prev_bottom <= platform_top and new_bottom >= platform_top then
          one_way_hit = true
        end
      end
      if one_way_hit then
        ny = platform_top - h
        touch.ground = true
      else
        local floor_y = slope_floor_y(ty, nx, nx + w, ss)
        if floor_y ~= nil and new_bottom >= floor_y and prev_bottom <= floor_y then
          ny = floor_y - h
          touch.ground = true
        end
      end
    end
  elseif dy < 0 then
    local ty = math.floor(ny / ss)
    local tx0 = math.floor(nx / ss)
    local tx1 = math.floor((nx + w - 1) / ss)
    if solid_blocks_row(ty, tx0, tx1) then
      ny = (ty + 1) * ss
      touch.ceiling = true
    end
  end

  return nx, ny, touch
end
