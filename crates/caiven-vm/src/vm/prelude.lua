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

function tile_solid(tx, ty)
  local tile = get_tile(tx, ty)
  local flags = get_sprite_flags(tile)
  return (flags & 1) ~= 0
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
