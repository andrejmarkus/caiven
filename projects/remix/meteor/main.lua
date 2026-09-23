-- METEOR: dodge the falling rocks. Skim right past one for a CLOSE bonus.
-- Change a number, press Run. The comments have wild values to try.

local ROCK_SPEED = 1.2        -- try 4
local ROCKS_PER_SECOND = 2    -- try 20
local ROCK_SIZE = 5           -- try 25
local PLAYER_SPEED = 2.5      -- try 8
local SPEED_UP = 0.002        -- try 0.02
local SHAKE = 6               -- try 25

local CLOSE = 7               -- clearance in pixels that counts as a near miss
local ROCK_COLORS = { 10, 11, 12 }

local W, H = 192, 128
local PLAYER_Y = H - 16

local player_x, speed, spawn = 0, 1, 0
local rocks, stars, bits, popups = {}, {}, {}, {}
local score, best = 0, 0
local shake, flash, game_over, over_timer = 0, 0, false, 0

-- Juice: sparks, floating text, screen shake.

local function burst(x, y, color, count, power)
  for _ = 1, count do
    local angle = random_float(0, math.pi * 2)
    local s = random_float(0.3, 1) * power
    bits[#bits + 1] = {
      x = x, y = y, dx = math.cos(angle) * s, dy = math.sin(angle) * s,
      life = random_range(15, 35), color = color,
    }
  end
end

local function popup(x, y, text, color)
  popups[#popups + 1] = { x = x, y = y, text = text, color = color, life = 45 }
end

local function update_juice()
  for i = #bits, 1, -1 do
    local b = bits[i]
    b.x, b.y, b.dy = b.x + b.dx, b.y + b.dy, b.dy + 0.05
    b.life = b.life - 1
    if b.life <= 0 then table.remove(bits, i) end
  end
  for i = #popups, 1, -1 do
    local p = popups[i]
    p.y, p.life = p.y - 0.4, p.life - 1
    if p.life <= 0 then table.remove(popups, i) end
  end
  shake = shake * 0.88
  flash = math.max(flash - 1, 0)
  set_camera(random_float(-shake, shake), random_float(-shake, shake))
end

local function center(text, y, color)
  draw_text(text, (W - #text * 6) // 2, y, color)
end

-- The game.

local function start()
  player_x, speed, spawn = W / 2, 1, 0
  rocks, bits, popups = {}, {}, {}
  score, game_over = 0, false
end

local function add_rock()
  rocks[#rocks + 1] = {
    x = random_range(0, W), y = -ROCK_SIZE * 2,
    r = ROCK_SIZE * random_float(0.6, 1.3),
    dx = random_float(-0.3, 0.3), dy = ROCK_SPEED * speed * random_float(0.7, 1.3),
    color = choice(ROCK_COLORS), passed = false,
  }
end

local function explode()
  burst(player_x, PLAYER_Y, 13, 30, 3)
  burst(player_x, PLAYER_Y, 15, 20, 2)
  burst(player_x, PLAYER_Y, 3, 20, 4)
  shake, flash = SHAKE * 2, 3
  play_sfx(1)
  game_over, over_timer = true, 0
  best = math.max(best, score)
end

function _init()
  local h, m, s = real_time()
  math.randomseed(h * 3600 + m * 60 + s)
  for i = 1, 40 do
    stars[i] = { x = random_range(0, W), y = random_range(0, H), s = random_float(0.2, 1) }
  end
  set_music_volume(0.5)
  play_music(0)
  start()
end

function _update()
  update_juice()
  for _, st in ipairs(stars) do
    st.y = st.y + st.s * speed * 2
    if st.y > H then st.y, st.x = 0, random_range(0, W) end
  end

  if game_over then
    over_timer = over_timer + 1
    if over_timer > 30 and button_pressed(4) then start() end
    return
  end

  if button_down(2) then player_x = player_x - PLAYER_SPEED end
  if button_down(3) then player_x = player_x + PLAYER_SPEED end
  player_x = clamp(player_x, 4, W - 4)

  speed = speed + SPEED_UP
  spawn = spawn + ROCKS_PER_SECOND * speed / 60
  while spawn >= 1 do
    add_rock()
    spawn = spawn - 1
  end

  for i = #rocks, 1, -1 do
    local r = rocks[i]
    r.x, r.y = r.x + r.dx, r.y + r.dy
    local gap = math.sqrt((r.x - player_x) ^ 2 + (r.y - PLAYER_Y) ^ 2) - r.r - 3
    if gap < 0 then
      explode()
      return
    end
    if not r.passed and r.y > PLAYER_Y then
      r.passed = true
      if gap < CLOSE then
        score = score + 5
        popup(player_x - 21, PLAYER_Y - 16, "CLOSE!", 14)
        burst(r.x, r.y, 14, 10, 2)
        play_sfx(2)
      else
        score = score + 1
      end
    end
    if r.y > H + r.r then table.remove(rocks, i) end
  end
end

local function draw_ship()
  if game_over then return end
  fill_rect(player_x - 1, PLAYER_Y - 4, 3, 8, 15)
  fill_rect(player_x - 4, PLAYER_Y, 9, 3, 9)
  fill_rect(player_x - 1, PLAYER_Y + 4, 3, random_range(1, 4), choice({ 3, 13 }))
end

function _draw()
  clear_screen()
  fill_screen(flash > 0 and 12 or 0)
  for _, st in ipairs(stars) do fill_rect(st.x, st.y, 1, 1 + st.s * 2, st.s > 0.6 and 12 or 10) end
  for _, r in ipairs(rocks) do
    fill_circle(r.x, r.y - r.r, r.r * 0.7, 2)
    fill_circle(r.x, r.y - r.r * 0.6, r.r * 0.6, 3)
    fill_circle(r.x, r.y, r.r, r.color)
  end
  draw_ship()
  for _, b in ipairs(bits) do fill_rect(b.x, b.y, 2, 2, b.color) end
  for _, p in ipairs(popups) do draw_text(p.text, p.x, p.y, p.color) end
  draw_number(score, 4, 3, 15)
  draw_text("BEST " .. best, 136, 3, 12)
  if game_over then
    center("BOOM", 48, 3)
    if over_timer > 30 then center("PRESS A", 62, 15) end
  end
end
