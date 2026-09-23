-- HOP: tap A to flap through the gaps. It gets faster every pipe.
-- Change a number, press Run. The comments have wild values to try.

local GRAVITY = 0.22       -- try 0.05
local FLAP = 3.2           -- try 6
local GAP = 46             -- try 20
local PIPE_SPEED = 1.2     -- try 3
local SPEED_UP = 0.03      -- try 0.3
local SHAKE = 5            -- try 20

local PIPE_SPACING = 84
local PIPE_WIDTH = 14
local BIRD_X, BIRD_R = 44, 4

local W, H = 192, 128
local GROUND = H - 8

local bird_y, velocity, wing = 0, 0, 0
local pipes, clouds, bits, popups = {}, {}, {}, {}
local score, best, speed, scroll = 0, 0, 1, 0
local shake, state, over_timer = 0, "ready", 0

-- Juice: feathers, floating text, screen shake.

local function burst(x, y, color, count, power)
  for _ = 1, count do
    local angle = random_float(0, math.pi * 2)
    local s = random_float(0.3, 1) * power
    bits[#bits + 1] = {
      x = x, y = y, dx = math.cos(angle) * s - 1, dy = math.sin(angle) * s,
      life = random_range(10, 25), color = color,
    }
  end
end

local function popup(x, y, text)
  popups[#popups + 1] = { x = x, y = y, text = text, life = 30 }
end

local function update_juice()
  for i = #bits, 1, -1 do
    local b = bits[i]
    b.x, b.y, b.dy = b.x + b.dx, b.y + b.dy, b.dy + 0.08
    b.life = b.life - 1
    if b.life <= 0 then table.remove(bits, i) end
  end
  for i = #popups, 1, -1 do
    local p = popups[i]
    p.y, p.life = p.y - 0.6, p.life - 1
    if p.life <= 0 then table.remove(popups, i) end
  end
  shake = shake * 0.85
  set_camera(random_float(-shake, shake), random_float(-shake, shake))
end

local function center(text, y, color)
  draw_text(text, (W - #text * 6) // 2, y, color)
end

-- The game.

local function start()
  bird_y, velocity = H / 2, 0
  pipes, bits, popups = {}, {}, {}
  score, speed, state = 0, 1, "ready"
end

local function flap_pressed()
  return button_pressed(4) or button_pressed(0)
end

local function flap()
  velocity, wing = -FLAP, 6
  burst(BIRD_X - 3, bird_y + 2, 15, 3, 1)
  play_sfx(3)
end

local function crash()
  burst(BIRD_X, bird_y, 13, 25, 3)
  burst(BIRD_X, bird_y, 15, 15, 2)
  shake = SHAKE * 2
  play_sfx(1)
  state, over_timer = "over", 0
  best = math.max(best, score)
end

local function add_pipe()
  pipes[#pipes + 1] = { x = W, gap_y = random_range(14, math.max(14, GROUND - GAP - 10)), passed = false }
end

function _init()
  local h, m, s = real_time()
  math.randomseed(h * 3600 + m * 60 + s)
  for i = 1, 5 do
    clouds[i] = { x = random_range(0, W), y = random_range(8, 60), r = random_range(5, 10) }
  end
  set_music_volume(0.5)
  play_music(1)
  start()
end

function _update()
  update_juice()
  wing = math.max(wing - 1, 0)

  if state == "ready" then
    bird_y = H / 2 + math.sin(frame_count() / 10) * 4
    if flap_pressed() then
      state = "playing"
      flap()
    end
    return
  elseif state == "over" then
    over_timer = over_timer + 1
    if over_timer > 30 and flap_pressed() then start() end
    return
  end

  local move = PIPE_SPEED * speed
  scroll = scroll + move
  for _, c in ipairs(clouds) do
    c.x = c.x - move * 0.3
    if c.x < -20 then c.x = W + 20 end
  end

  if flap_pressed() then flap() end
  velocity = velocity + GRAVITY
  bird_y = math.max(bird_y + velocity, BIRD_R)
  if bird_y > GROUND - BIRD_R then
    crash()
    return
  end

  if #pipes == 0 or pipes[#pipes].x < W - PIPE_SPACING then add_pipe() end
  for i = #pipes, 1, -1 do
    local p = pipes[i]
    p.x = p.x - move
    local in_column = BIRD_X + BIRD_R > p.x and BIRD_X - BIRD_R < p.x + PIPE_WIDTH
    local in_gap = bird_y - BIRD_R > p.gap_y and bird_y + BIRD_R < p.gap_y + GAP
    if in_column and not in_gap then
      crash()
      return
    end
    if not p.passed and p.x + PIPE_WIDTH < BIRD_X then
      p.passed = true
      score, speed = score + 1, speed + SPEED_UP
      popup(BIRD_X - 6, bird_y - 14, "+1")
      play_sfx(0)
      if score % 10 == 0 then
        popup(W / 2 - 30, 40, "ON FIRE!")
        play_sfx(5)
      end
    end
    if p.x < -PIPE_WIDTH then table.remove(pipes, i) end
  end
end

local function draw_pipe(p)
  local lip = 3
  fill_rect(p.x, 0, PIPE_WIDTH, p.gap_y, 5)
  fill_rect(p.x - 2, p.gap_y - lip, PIPE_WIDTH + 4, lip, 6)
  fill_rect(p.x, p.gap_y + GAP, PIPE_WIDTH, GROUND - p.gap_y - GAP, 5)
  fill_rect(p.x - 2, p.gap_y + GAP, PIPE_WIDTH + 4, lip, 6)
end

local function draw_bird()
  if state == "over" then return end
  fill_circle(BIRD_X, bird_y, BIRD_R, 13)
  fill_rect(BIRD_X + 1, bird_y - 2, 2, 2, 0)
  fill_rect(BIRD_X + 4, bird_y, 3, 2, 3)
  local wing_y = wing > 0 and bird_y - 3 or bird_y + 1
  fill_rect(BIRD_X - 4, wing_y, 4, 2, 15)
end

function _draw()
  clear_screen()
  fill_screen(9)
  for _, c in ipairs(clouds) do
    fill_circle(c.x, c.y, c.r, 15)
    fill_circle(c.x + c.r, c.y + 2, c.r * 0.7, 15)
  end
  for _, p in ipairs(pipes) do draw_pipe(p) end
  fill_rect(0, GROUND, W, H - GROUND, 10)
  for x = -(scroll % 12), W, 12 do fill_rect(x, GROUND, 6, 2, 11) end
  draw_bird()
  for _, b in ipairs(bits) do fill_rect(b.x, b.y, 2, 2, b.color) end
  for _, p in ipairs(popups) do draw_text(p.text, p.x, p.y, 15) end
  draw_number(score, 4, 3, 15)
  draw_text("BEST " .. best, 136, 3, 7)
  if state == "ready" then
    center("PRESS A TO FLAP", 84, 15)
  elseif state == "over" and over_timer > 30 then
    center("PRESS A", 60, 15)
  end
end
