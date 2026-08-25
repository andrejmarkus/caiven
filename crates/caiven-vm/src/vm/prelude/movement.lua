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

local function slope_in_column(tx, ty0, ty1)
  for ty = ty0, ty1 do
    local id = get_collision(tx, ty)
    if collision_is_slope_left(id) or collision_is_slope_right(id) then return true end
  end
  return false
end

-- How far (1..ss pixels) an entity moving horizontally into column `tx`
-- would need to rise to clear whatever's blocking it there, or nil if no
-- rise within one tile height clears it, or if `from_tx` (the column the
-- entity is currently standing in, i.e. the trailing column in the
-- direction of travel) isn't a slope. A slope one tile taller than its
-- flush neighboring floor is exactly a one-tile ledge at the seam where
-- they meet — without this, that ledge hard-stops horizontal movement
-- before the entity gets close enough for the slope's own vertical
-- resolve to lift it, and the vertical pass can't raise it further
-- without that horizontal approach, so the two axis-separated passes
-- deadlock against each other right at the seam. Gating on "was I just
-- standing on a slope" (rather than treating every one-tile-shorter solid
-- obstacle as a climbable ledge) keeps this a slope-specific fix, not a
-- blanket reinterpretation of what a solid wall blocks.
local function step_up_clearance(from_tx, tx, y, h, ss)
  local ty0 = math.floor(y / ss)
  local ty1 = math.floor((y + h - 1) / ss)
  if not slope_in_column(from_tx, ty0, ty1 + 1) then
    return nil
  end
  for lift = 1, ss do
    local lty0 = math.floor((y - lift) / ss)
    local lty1 = math.floor((y - lift + h - 1) / ss)
    if not solid_blocks_column(tx, lty0, lty1) then
      return lift
    end
  end
  return nil
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

  -- Horizontal: SOLID only (a step-up allowance clears a one-tile ledge or
  -- a slope's tall edge instead of hard-blocking it — see
  -- step_up_clearance). Slopes/one-way never block horizontal movement.
  local ny = y
  local nx = x + dx
  if dx > 0 then
    local tx = math.floor((nx + w - 1) / ss)
    local ty0 = math.floor(y / ss)
    local ty1 = math.floor((y + h - 1) / ss)
    if solid_blocks_column(tx, ty0, ty1) then
      local lift = step_up_clearance(tx - 1, tx, y, h, ss)
      if lift then
        ny = y - lift
      else
        nx = tx * ss - w
        touch.right = true
      end
    end
  elseif dx < 0 then
    local tx = math.floor(nx / ss)
    local ty0 = math.floor(y / ss)
    local ty1 = math.floor((y + h - 1) / ss)
    if solid_blocks_column(tx, ty0, ty1) then
      local lift = step_up_clearance(tx + 1, tx, y, h, ss)
      if lift then
        ny = y - lift
      else
        nx = (tx + 1) * ss
        touch.left = true
      end
    end
  end

  -- Vertical: SOLID, then ONE_WAY (descending-from-above only), then slopes.
  -- Reuses `ny` (not a fresh local) so a step-up applied by the horizontal
  -- pass above survives into this frame's result — the two passes are
  -- never both active at once (the caller always passes exactly one
  -- nonzero axis per call), so `ny + dy` here is `y + dy` unchanged for a
  -- pure vertical call, and `(y - lift) + 0` unchanged for a pure
  -- horizontal one.
  ny = ny + dy
  if dy > 0 then
    local prev_bottom = y + h
    local new_bottom = ny + h
    -- Row containing the bottom edge's last actually-occupied pixel. The
    -- edge itself is exclusive (a bottom of exactly 16 occupies pixel 15,
    -- not 16), so naively flooring new_bottom/ss would land one row too
    -- far down for an exact-multiple bottom. `ceil(new_bottom) - 1` finds
    -- that last occupied pixel correctly in both cases a plain `- 1` gets
    -- wrong: an exact-multiple bottom (ceil is a no-op, so still -1) *and*
    -- a bottom that has only fractionally crossed into the next row (ceil
    -- rounds up to the row boundary first, so the same -1 lands back in
    -- the row just entered instead of overshooting into the row above).
    -- A fixed one-pixel subtraction only got the first case right, which
    -- is why a one-way platform caught a resting entity once and then
    -- never again: the frame gravity resumed from rest moved the bottom
    -- edge a fraction of a pixel past the platform, `ty` resolved to the
    -- empty row above it, `platform_top` was computed from that wrong row,
    -- and the crossing condition below could never be satisfied again
    -- (the entity was already past the correct platform_top from then on).
    local ty = math.floor((math.ceil(new_bottom) - 1) / ss)
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
        -- A slope tile that steps *up* from a flush neighboring floor sits
        -- one row above that floor's own row — walking onto it isn't a
        -- "falling from above" case at all, it's "the row I'm already
        -- resting at depth for turns out to have a ramp one tile higher,
        -- right here". `ty` alone (the row matching the entity's current
        -- resting depth) never sees that ramp; also probing `ty - 1`
        -- covers exactly that walk-up-a-slope case without touching how a
        -- genuine fall onto a slope from well above already works (that
        -- one converges on `ty` normally, long before reaching row `ty - 1`
        -- would even matter).
        -- Sample at the entity's horizontal center, not across its whole
        -- width: the diagonal only spans one tile, so as soon as most of a
        -- multi-pixel-wide entity's footprint sits over the tile, "the
        -- highest point anywhere under the footprint" is already close to
        -- the tile's tall edge — the entity would snap most of the way up
        -- the very first frame it's caught, instead of climbing smoothly
        -- as it walks across. A single point tied to the entity's own
        -- position moves at the same continuous rate the entity does.
        local cx = nx + math.floor(w / 2)
        local floor_y = slope_floor_y(ty, cx, cx + 1, ss)
        if floor_y == nil then
          floor_y = slope_floor_y(ty - 1, cx, cx + 1, ss)
        end
        -- `+ ss`: a slope only ever bridges a one-tile height difference
        -- between two floors by construction, so an entity already resting
        -- within one tile-height of the slope's surface is exactly the
        -- "walking onto a ramp between two adjacent floors" case, not a
        -- tunneling risk — a genuine fast fall from well above already
        -- clears `prev_bottom <= floor_y` outright, long before this
        -- tolerance would matter. Without slack here, an entity walking
        -- onto the slope from its lower neighboring floor has a
        -- `prev_bottom` already past `floor_y` (their resting depth
        -- matches the *lower* floor's row, while `slope_floor_y` reports
        -- the surface height under whichever part of their footprint sits
        -- highest on the ramp) and the crossing test can never fire —
        -- they walk straight through the slope with the floor never
        -- catching them.
        if floor_y ~= nil and new_bottom >= floor_y and prev_bottom <= floor_y + ss then
          -- Climb toward `floor_y` at a capped rate rather than snapping
          -- straight to it: the first frame the crossing condition fires,
          -- `floor_y` already reflects wherever the (center-sampled) point
          -- happens to be on the ramp, which — for an entity several
          -- pixels wide, or one that only started this close-in check once
          -- its column lookup found the tile at all — can be a fair way up
          -- the slope already. Rising there in a single frame reads as a
          -- pop, not a climb. A capped rise still resolves in only a
          -- couple of frames (the whole ramp is one tile tall) but the
          -- ones a player can actually see land as motion, not a snap.
          local target = floor_y - h
          local rise_cap = ss / 2
          ny = math.max(target, y - rise_cap)
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
