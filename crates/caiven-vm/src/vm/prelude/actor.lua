local Actor = {}
Actor.__index = Actor

function Actor.new(opts)
  return setmetatable({
    sprite_id = opts.sprite_id,
    pos = opts.pos,
    flip_x = opts.flip_x or false,
    flip_y = opts.flip_y or false,
    rotate = opts.rotate or 0,
  }, Actor)
end

function Actor:draw()
  sprite(self.sprite_id, self.pos.x, self.pos.y, self.flip_x, self.flip_y, self.rotate)
end

return Actor
