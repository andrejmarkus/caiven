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
