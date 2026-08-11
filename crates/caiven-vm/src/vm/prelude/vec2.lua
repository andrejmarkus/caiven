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
