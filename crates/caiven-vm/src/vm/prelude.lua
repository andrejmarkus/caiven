-- Deterministic by default: a fresh Lua VM has RTK_SEEDED unset, so this
-- seeds once. Hot reload re-runs prelude.lua on the *same* live VM, where
-- RTK_SEEDED is already true, so a hot reload during dev doesn't reset the
-- live RNG stream mid-game. Carts opt out via their own math.randomseed(x).
if not RTK_SEEDED then
  math.randomseed(1)
  RTK_SEEDED = true
end

function random_range(lo, hi)
  return math.random(lo, hi)
end

function random_float(lo, hi)
  return lo + math.random() * (hi - lo)
end

function choice(t)
  local n = #t
  if n == 0 then
    error("choice() requires a non-empty table", 2)
  end
  return t[math.random(n)]
end

function shuffle(t)
  for i = #t, 2, -1 do
    local j = math.random(i)
    t[i], t[j] = t[j], t[i]
  end
  return t
end

Vec2 = {}
Vec2.__index = Vec2

local function is_vec2(v)
  return type(v) == "table" and getmetatable(v) == Vec2
end

function Vec2.new(x, y)
  return setmetatable({ x = x, y = y }, Vec2)
end

function Vec2.__add(a, b)
  if not (is_vec2(a) and is_vec2(b)) then
    error("Vec2 '+' requires two Vec2 operands", 2)
  end
  return Vec2.new(a.x + b.x, a.y + b.y)
end

function Vec2.__sub(a, b)
  if not (is_vec2(a) and is_vec2(b)) then
    error("Vec2 '-' requires two Vec2 operands", 2)
  end
  return Vec2.new(a.x - b.x, a.y - b.y)
end

function Vec2.__mul(a, b)
  if is_vec2(a) and type(b) == "number" then
    return Vec2.new(a.x * b, a.y * b)
  elseif type(a) == "number" and is_vec2(b) then
    return Vec2.new(b.x * a, b.y * a)
  end
  error("Vec2 '*' requires a Vec2 and a number", 2)
end

function Vec2.__unm(v)
  return Vec2.new(-v.x, -v.y)
end

function Vec2.__eq(a, b)
  return a.x == b.x and a.y == b.y
end

function Vec2.__tostring(v)
  return "(" .. v.x .. ", " .. v.y .. ")"
end

function Vec2:length_squared()
  return self.x * self.x + self.y * self.y
end

function Vec2:length()
  return math.sqrt(self:length_squared())
end

function Vec2:normalize()
  local len = self:length()
  if len == 0 then
    return Vec2.new(0, 0)
  end
  return Vec2.new(self.x / len, self.y / len)
end

function Vec2:dot(other)
  return self.x * other.x + self.y * other.y
end

function Vec2:distance(other)
  return (self - other):length()
end

Sprite = {}
Sprite.__index = Sprite

function Sprite.new(opts)
  return setmetatable({
    sprite_id = opts.sprite_id,
    pos = opts.pos,
    flip_x = opts.flip_x or false,
    flip_y = opts.flip_y or false,
    rotate = opts.rotate or 0,
  }, Sprite)
end

function Sprite:draw()
  sprite(self.sprite_id, self.pos.x, self.pos.y, self.flip_x, self.flip_y, self.rotate)
end

function lerp(a, b, t)
  return a + (b - a) * t
end

function clamp(v, lo, hi)
  if v < lo then return lo end
  if v > hi then return hi end
  return v
end

function ease_linear(t) return t end
function ease_in_quad(t) return t * t end
function ease_out_quad(t) return 1 - (1 - t) * (1 - t) end
function ease_in_out_quad(t)
  if t < 0.5 then return 2 * t * t end
  return 1 - ((-2 * t + 2) ^ 2) / 2
end

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

function new_tween(from, to, frames, ease)
  return { from = from, to = to, frames = frames, ease = ease or ease_linear, t = 0, done = false }
end

function tween_update(tw)
  if tw.done then return tw.to end
  tw.t = tw.t + 1
  local p = tw.t / tw.frames
  if p >= 1 then
    p = 1
    tw.done = true
  end
  return tw.from + (tw.to - tw.from) * tw.ease(p)
end

function new_anim(frames, frame_len)
  return { frames = frames, frame_len = frame_len, timer = 0, index = 1 }
end

function anim_update(anim)
  anim.timer = anim.timer + 1
  if anim.timer >= anim.frame_len then
    anim.timer = 0
    anim.index = anim.index % #anim.frames + 1
  end
end

function anim_sprite(anim)
  return anim.frames[anim.index]
end

Particles = { list = {} }

function Particles.spawn(x, y, vx, vy, color, life)
  table.insert(Particles.list, { x = x, y = y, vx = vx, vy = vy, color = color, life = life, age = 0 })
end

function Particles.update()
  local alive = {}
  for _, p in ipairs(Particles.list) do
    p.x = p.x + p.vx
    p.y = p.y + p.vy
    p.age = p.age + 1
    if p.age < p.life then
      table.insert(alive, p)
    end
  end
  Particles.list = alive
end

function Particles.draw()
  for _, p in ipairs(Particles.list) do
    set_pixel(math.floor(p.x), math.floor(p.y), p.color)
  end
end

function Particles.clear()
  Particles.list = {}
end

function Particles.count()
  return #Particles.list
end

Scenes = { stack = {} }

function Scenes.push(scene)
  if scene.enter then scene.enter(scene) end
  table.insert(Scenes.stack, scene)
end

function Scenes.pop()
  local n = #Scenes.stack
  if n == 0 then
    error("Scenes.pop() called on an empty stack", 2)
  end
  local top = Scenes.stack[n]
  if top.exit then top.exit(top) end
  table.remove(Scenes.stack)
end

function Scenes.switch(scene)
  if #Scenes.stack == 0 then
    error("Scenes.switch() called on an empty stack", 2)
  end
  Scenes.pop()
  Scenes.push(scene)
end

function Scenes.update()
  local top = Scenes.stack[#Scenes.stack]
  if top and top.update then top.update(top) end
end

function Scenes.draw()
  local top = Scenes.stack[#Scenes.stack]
  if top and top.draw then top.draw(top) end
end

function Scenes.current()
  return Scenes.stack[#Scenes.stack]
end

local function make_entity_list()
  local self = { list = {} }

  function self.add(e)
    if type(e) ~= "table" then
      error("Entities.add() requires a table", 2)
    end
    table.insert(self.list, e)
  end

  -- Compacts in place (no per-frame table allocation): live entities are
  -- shifted down over dead slots, then the tail is trimmed.
  function self.update_all()
    local list = self.list
    local write = 1
    for read = 1, #list do
      local e = list[read]
      if e.update then e.update(e) end
      if not e.dead then
        list[write] = e
        write = write + 1
      end
    end
    for i = #list, write, -1 do
      list[i] = nil
    end
  end

  function self.draw_all()
    for _, e in ipairs(self.list) do
      if e.draw then e.draw(e) end
    end
  end

  function self.clear()
    self.list = {}
  end

  function self.count()
    return #self.list
  end

  return self
end

Entities = make_entity_list()
Entities.new = make_entity_list

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
