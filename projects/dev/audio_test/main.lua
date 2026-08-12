-- Audio test — press buttons to trigger SFX bank slots
-- UP: slot 0 (left pan)   DOWN: slot 1 (right pan)
-- LEFT: slot 2 (noise)    RIGHT: slot 3, held (release on button-up)
-- SELECT: toggle background music, to show it keeps playing under SFX
-- Paint sounds into these slots in the Caiven Studio SFX tab (F4)

held_handle = nil
held_down = false
music_active = false

function _init()
  set_palette_color(0, 10, 10, 20)
  set_palette_color(1, 255, 255, 255)
end

function _update()
  clear_screen()

  draw_text("UP: LEFT PAN", 4, 20, 1)
  draw_text("DOWN: RIGHT PAN", 4, 36, 1)
  draw_text("LEFT: NOISE", 4, 52, 1)
  draw_text("RIGHT (hold): stop_sfx on release", 4, 68, 1)
  draw_text("SELECT: toggle music", 4, 84, 1)

  if button_pressed(0) then play_sfx(0) end
  if button_pressed(1) then play_sfx(1) end
  if button_pressed(2) then play_sfx(2) end

  if button_pressed(3) then
    held_handle = play_sfx(3, {volume = 0.8})
    held_down = true
  elseif held_down and not button_down(3) then
    stop_sfx(held_handle)
    held_handle = nil
    held_down = false
  end

  if button_pressed(6) then
    if music_active then stop_music() else play_music(0) end
    music_active = not music_active
  end
end
