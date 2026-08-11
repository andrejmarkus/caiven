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
