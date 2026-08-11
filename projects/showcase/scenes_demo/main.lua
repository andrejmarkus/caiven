
PLAYER_SPEED = 1

local function make_player()
  return {
    pos = Vec2.new(64, 64),
    update = function(e)
      local dx, dy = 0, 0
      if button_down(2) then dx = dx - 1 end
      if button_down(3) then dx = dx + 1 end
      if button_down(0) then dy = dy - 1 end
      if button_down(1) then dy = dy + 1 end
      e.pos = e.pos + Vec2.new(dx, dy) * PLAYER_SPEED
    end,
    draw = function(e)
      fill_rect(math.floor(e.pos.x), math.floor(e.pos.y), 8, 8, 11)
    end,
  }
end

local function make_enemy(x, y)
  return {
    pos = Vec2.new(x, y),
    update = function(e)
      e.pos = e.pos + Vec2.new(0, 1)
      if e.pos.y > 200 then e.dead = true end
    end,
    draw = function(e)
      fill_rect(math.floor(e.pos.x), math.floor(e.pos.y), 8, 8, 9)
    end,
  }
end

title_scene = {
  update = function(s)
    if button_pressed(4) then
      Scenes.switch(play_scene)
    end
  end,
  draw = function(s)
    clear_screen()
    draw_text("SCENES DEMO", 20, 50, 7)
    draw_text("PRESS A TO PLAY", 15, 70, 6)
  end,
}

play_scene = {
  enter = function(s)
    Entities.clear()
    player = make_player()
    Entities.add(player)
    Entities.add(make_enemy(20, 0))
    Entities.add(make_enemy(80, 0))
    Camera.follow(player, { lerp = 0.2 })
    score = 0
  end,
  update = function(s)
    Entities.update_all()
    Camera.update()
    score = score + 1
    if score > 300 then
      Scenes.switch(gameover_scene)
    end
  end,
  draw = function(s)
    clear_screen()
    Entities.draw_all()
    draw_text("SURVIVE", 2, 2, 7)
  end,
}

gameover_scene = {
  update = function(s)
    if button_pressed(4) then
      Scenes.switch(title_scene)
    end
  end,
  draw = function(s)
    clear_screen()
    draw_text("GAME OVER", 25, 50, 8)
    draw_text("PRESS A FOR TITLE", 10, 70, 6)
  end,
}

function _init()
  Scenes.push(title_scene)
end

function _update()
  Scenes.update()
end

function _draw()
  Scenes.draw()
end
