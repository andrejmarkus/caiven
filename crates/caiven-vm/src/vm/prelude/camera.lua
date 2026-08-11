Camera = { target = nil, opts = nil, x = 0, y = 0, shake_amount = 0, shake_duration = 0, shake_timer = 0 }

local function camera_entity_position(entity)
  if entity.pos then
    return entity.pos.x, entity.pos.y
  elseif entity.x and entity.y then
    return entity.x, entity.y
  end
  error("Camera.follow() requires an entity with .pos or .x/.y", 2)
end

function Camera.follow(entity, opts)
  camera_entity_position(entity) -- validate eagerly, fail at the call site
  Camera.target = entity
  Camera.opts = opts or {}
end

function Camera.unfollow()
  Camera.target = nil
  Camera.opts = nil
end

function Camera.shake(amount, duration)
  Camera.shake_amount = amount
  Camera.shake_duration = duration
  Camera.shake_timer = duration
end

function Camera.update()
  if Camera.target then
    local tx, ty = camera_entity_position(Camera.target)
    local lerp_t = (Camera.opts and Camera.opts.lerp) or 1
    local deadzone_x = (Camera.opts and Camera.opts.deadzone_x) or 0
    local deadzone_y = (Camera.opts and Camera.opts.deadzone_y) or 0
    local dx = tx - Camera.x
    local dy = ty - Camera.y
    if math.abs(dx) > deadzone_x then
      Camera.x = Camera.x + dx * lerp_t
    end
    if math.abs(dy) > deadzone_y then
      Camera.y = Camera.y + dy * lerp_t
    end
  end

  local shake_x, shake_y = 0, 0
  if Camera.shake_timer > 0 then
    local strength = Camera.shake_amount * (Camera.shake_timer / Camera.shake_duration)
    shake_x = random_float(-strength, strength)
    shake_y = random_float(-strength, strength)
    Camera.shake_timer = Camera.shake_timer - 1
  end

  local final_x = math.floor(clamp(Camera.x + shake_x, 0, math.huge))
  local final_y = math.floor(clamp(Camera.y + shake_y, 0, math.huge))
  set_camera(final_x, final_y)
end
