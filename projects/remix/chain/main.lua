-- CHAIN: you get one shot. Aim, press A, and set off the biggest chain
-- reaction you can. Pop enough dots to clear the level.
-- Change a number, press Run. The comments have wild values to try.

local DOTS = 30            -- try 200
local BLAST_SIZE = 12      -- try 30
local BLAST_TIME = 50      -- try 120
local DOT_SPEED = 0.5      -- try 3
local GOAL = 5             -- try 30
local SHAKE = 1            -- try 6

local CURSOR_SPEED = 2.5
local COLORS = { 13, 14, 3, 6, 9, 2 }

local W, H = 192, 128

local dots, blasts, bits = {}, {}, {}
local level, popped, goal = 1, 0, 0
local cursor_x, cursor_y = W / 2, H / 2
local shake, state, timer = 0, "aim", 0

-- Juice: sparks and screen shake.

local function burst(x, y, color, count)
  for _ = 1, count do
    local angle = random_float(0, math.pi * 2)
    local s = random_float(0.5, 2)
    bits[#bits + 1] = {
      x = x, y = y, dx = math.cos(angle) * s, dy = math.sin(angle) * s,
      life = random_range(10, 20), color = color,
    }
  end
end

local function update_juice()
  for i = #bits, 1, -1 do
    local b = bits[i]
    b.x, b.y = b.x + b.dx, b.y + b.dy
    b.life = b.life - 1
    if b.life <= 0 then table.remove(bits, i) end
  end
  shake = math.min(shake * 0.9, 10)
  set_camera(random_float(-shake, shake), random_float(-shake, shake))
end

local function center(text, y, color)
  draw_text(text, (W - #text * 6) // 2, y, color)
end

-- The game.

local function start_level()
  dots, blasts, bits = {}, {}, {}
  popped, state = 0, "aim"
  local count = DOTS + (level - 1) * 4
  goal = math.min(GOAL + (level - 1) * 3, count)
  for i = 1, count do
    local angle = random_float(0, math.pi * 2)
    dots[i] = {
      x = random_range(4, W - 4), y = random_range(12, H - 4),
      dx = math.cos(angle) * DOT_SPEED, dy = math.sin(angle) * DOT_SPEED,
      color = choice(COLORS),
    }
  end
end

local function blast(x, y, color)
  blasts[#blasts + 1] = { x = x, y = y, r = 0, age = 0, color = color }
end

function _init()
  local h, m, s = real_time()
  math.randomseed(h * 3600 + m * 60 + s)
  set_music_volume(0.5)
  play_music(1)
  start_level()
end

local function move_dots()
  for _, d in ipairs(dots) do
    d.x, d.y = d.x + d.dx, d.y + d.dy
    if d.x < 2 or d.x > W - 2 then d.dx = -d.dx end
    if d.y < 12 or d.y > H - 2 then d.dy = -d.dy end
  end
end

local function update_blasts()
  for i = #blasts, 1, -1 do
    local b = blasts[i]
    b.age = b.age + 1
    -- Grow fast, hold, then shrink away.
    if b.age < 10 then
      b.r = BLAST_SIZE * b.age / 10
    elseif b.age > BLAST_TIME - 10 then
      b.r = BLAST_SIZE * (BLAST_TIME - b.age) / 10
    end
    if b.age >= BLAST_TIME then table.remove(blasts, i) end
  end

  -- Any dot touching a blast becomes a blast too. That's the chain.
  for i = #dots, 1, -1 do
    local d = dots[i]
    for _, b in ipairs(blasts) do
      if (d.x - b.x) ^ 2 + (d.y - b.y) ^ 2 < (b.r + 2) ^ 2 then
        table.remove(dots, i)
        blast(d.x, d.y, d.color)
        burst(d.x, d.y, d.color, 6)
        popped = popped + 1
        shake = shake + SHAKE
        play_sfx(4)
        break
      end
    end
  end
end

function _update()
  update_juice()
  move_dots()

  if state == "aim" then
    if button_down(0) then cursor_y = cursor_y - CURSOR_SPEED end
    if button_down(1) then cursor_y = cursor_y + CURSOR_SPEED end
    if button_down(2) then cursor_x = cursor_x - CURSOR_SPEED end
    if button_down(3) then cursor_x = cursor_x + CURSOR_SPEED end
    cursor_x, cursor_y = clamp(cursor_x, 0, W), clamp(cursor_y, 10, H)
    if button_pressed(4) then
      blast(cursor_x, cursor_y, 15)
      play_sfx(1)
      state = "boom"
    end
  elseif state == "boom" then
    update_blasts()
    if #blasts == 0 then
      state, timer = popped >= goal and "cleared" or "failed", 0
      play_sfx(popped >= goal and 5 or 1)
    end
  else
    timer = timer + 1
    if timer > 30 and button_pressed(4) then
      if state == "cleared" then level = level + 1 end
      start_level()
    end
  end
end

function _draw()
  clear_screen()
  fill_screen(0)
  for _, b in ipairs(blasts) do
    fill_circle(b.x, b.y, b.r, b.color)
    draw_circle(b.x, b.y, b.r, 15)
  end
  for _, d in ipairs(dots) do fill_circle(d.x, d.y, 2, d.color) end
  for _, b in ipairs(bits) do fill_rect(b.x, b.y, 1, 1, b.color) end

  if state == "aim" then
    draw_circle(cursor_x, cursor_y, BLAST_SIZE, 15)
    draw_line(cursor_x - 3, cursor_y, cursor_x + 3, cursor_y, 15)
    draw_line(cursor_x, cursor_y - 3, cursor_x, cursor_y + 3, 15)
  end

  draw_text("LEVEL " .. level, 4, 2, 12)
  draw_text(popped .. "/" .. goal, 150, 2, popped >= goal and 6 or 15)
  if state == "cleared" then
    center(popped .. " IN A CHAIN!", 52, 13)
    if timer > 30 then center("PRESS A: NEXT LEVEL", 66, 15) end
  elseif state == "failed" then
    center(popped * 2 >= goal and "SO CLOSE" or "MISSED!", 52, 14)
    if timer > 30 then center("PRESS A: TRY AGAIN", 66, 15) end
  end
end
