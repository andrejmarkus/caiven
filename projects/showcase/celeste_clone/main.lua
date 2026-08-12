function _init()
  set_palette_color(0, 100, 160, 230)  -- sky
  set_palette_color(1, 92, 58, 33)     -- dirt dark
  set_palette_color(2, 132, 86, 48)    -- dirt light
  set_palette_color(3, 60, 168, 60)    -- grass
  set_palette_color(4, 220, 70, 90)    -- player body
  set_palette_color(5, 255, 220, 210)  -- player face
  set_palette_color(6, 220, 40, 60)    -- berry red
  set_palette_color(7, 60, 160, 70)    -- berry leaf
  set_palette_color(8, 230, 40, 40)    -- spike red
  set_palette_color(9, 255, 255, 255)  -- spike highlight
  set_palette_color(10, 250, 210, 40)  -- flag yellow
  set_palette_color(11, 255, 255, 255) -- flag white
  set_palette_color(12, 20, 20, 20)    -- outline/pole
  set_palette_color(13, 255, 255, 255) -- particle white
  set_palette_color(14, 255, 255, 0)   -- UI text
  set_palette_color(15, 40, 40, 40)    -- spare
end

function _update()
  if button_pressed(4) then play_sfx(0) end
  if button_pressed(5) then play_sfx(1) end
  if button_pressed(2) then play_sfx(2) end
  if button_pressed(3) then play_sfx(3) end
  if button_pressed(6) then
    if is_music_playing() then stop_music() else play_music(0) end
  end
end

function _draw()
  clear_screen()
  for id = 0, 10 do
    sprite(id, 4 + id * 11, 40)
  end
  draw_text("A/B/LEFT/RIGHT: SFX  SELECT: MUSIC", 2, 2, 14)
end
