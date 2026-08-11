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
