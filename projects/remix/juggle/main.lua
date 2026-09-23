-- JUGGLE: keep every ball in the air. Every few hits, another ball joins.
-- Change a number, press Run. The comments have wild values to try.

local BALLS = 2            -- try 30
local PADDLE_WIDTH = 36    -- try 190
local GRAVITY = 0.08       -- try 0.01
local NEW_BALL_EVERY = 5   -- try 1
local BALL_SIZE = 3        -- try 10
local SHAKE = 4            -- try 20

local BOUNCE = 3.4
local MAX_BALLS = 60
local PADDLE_SPEED = 3.5
local COLORS = { 13, 14, 3, 6, 9, 15 }

local W, H = 192, 128
local PADDLE_Y = H - 10

local paddle_x, paddle_glow = 0, 0
local balls, bits, popups = {}, {}, {}
local score, best, hits = 0, 0, 0
local shake, game_over, over_timer = 0, false, 0

-- Juice: sparks, floating text, screen shake.

local function burst(x, y, color, count)
  if #bits > 300 then return end
  for _ = 1, count do
    local angle = random_float(0, math.pi * 2)
    local speed = random_float(0.5, 2.5)
    bits[#bits + 1] = {
      x = x, y = y, dx = math.cos(angle) * speed, dy = math.sin(angle) * speed,
      life = random_range(12, 28), color = color,
    }
  end
end

local function popup(x, y, text)
  popups[#popups + 1] = { x = x, y = y, text = text, life = 40 }
end

local function update_juice()
  for i = #bits, 1, -1 do
    local b = bits[i]
    b.x, b.y, b.dy = b.x + b.dx, b.y + b.dy, b.dy + 0.1
    b.life = b.life - 1
    if b.life <= 0 then table.remove(bits, i) end
  end
  for i = #popups, 1, -1 do
    local p = popups[i]
    p.y, p.life = p.y - 0.5, p.life - 1
    if p.life <= 0 then table.remove(popups, i) end
  end
  shake = shake * 0.85
  set_camera(random_float(-shake, shake), random_float(-shake, shake))
end

local function draw_juice()
  for _, b in ipairs(bits) do fill_rect(b.x, b.y, 2, 2, b.color) end
  for _, p in ipairs(popups) do draw_text(p.text, p.x, p.y, 15) end
end

local function center(text, y, color)
  draw_text(text, (W - #text * 6) // 2, y, color)
end

-- The game.

local function add_ball()
  balls[#balls + 1] = {
    x = random_range(10, W - 10), y = random_range(-40, 10),
    dx = random_float(-1, 1), dy = 0, color = choice(COLORS),
  }
end

local function start()
  paddle_x = (W - PADDLE_WIDTH) / 2
  balls, bits, popups = {}, {}, {}
  score, hits, game_over = 0, 0, false
  for _ = 1, BALLS do add_ball() end
end

local function hit(b)
  -- Where the ball lands on the paddle steers it.
  local aim = (b.x - (paddle_x + PADDLE_WIDTH / 2)) / (PADDLE_WIDTH / 2)
  b.dx, b.dy, b.y = aim * 2, -BOUNCE, PADDLE_Y - BALL_SIZE
  score, hits, paddle_glow = score + 1, hits + 1, 6
  burst(b.x, PADDLE_Y, b.color, 8)
  popup(b.x - 6, PADDLE_Y - 14, "+1")
  play_sfx(0)
  if hits % NEW_BALL_EVERY == 0 and #balls < MAX_BALLS then
    add_ball()
    popup(W / 2 - 27, 30, "NEW BALL!")
    play_sfx(2)
  end
end

local function drop(i)
  local b = table.remove(balls, i)
  burst(b.x, H - 2, 2, 20)
  shake = SHAKE
  play_sfx(1)
  if #balls == 0 then
    game_over, over_timer = true, 0
    best = math.max(best, score)
    shake = SHAKE * 2
  end
end

function _init()
  local h, m, s = real_time()
  math.randomseed(h * 3600 + m * 60 + s)
  set_music_volume(0.5)
  play_music(0)
  start()
end

function _update()
  update_juice()
  if game_over then
    over_timer = over_timer + 1
    if over_timer > 30 and button_pressed(4) then start() end
    return
  end

  if button_down(2) then paddle_x = paddle_x - PADDLE_SPEED end
  if button_down(3) then paddle_x = paddle_x + PADDLE_SPEED end
  paddle_x = clamp(paddle_x, 0, W - PADDLE_WIDTH)
  paddle_glow = math.max(paddle_glow - 1, 0)

  for i = #balls, 1, -1 do
    local b = balls[i]
    b.dy = b.dy + GRAVITY
    b.x, b.y = b.x + b.dx, b.y + b.dy
    if b.x < BALL_SIZE or b.x > W - BALL_SIZE then
      b.dx = -b.dx
      b.x = clamp(b.x, BALL_SIZE, W - BALL_SIZE)
    end
    local over_paddle = b.x > paddle_x - BALL_SIZE and b.x < paddle_x + PADDLE_WIDTH + BALL_SIZE
    if b.dy > 0 and over_paddle and b.y + BALL_SIZE >= PADDLE_Y and b.y < PADDLE_Y + 4 then
      hit(b)
    elseif b.y > H + BALL_SIZE then
      drop(i)
    end
  end
end

function _draw()
  clear_screen()
  fill_screen(7)
  for _, b in ipairs(balls) do fill_circle(b.x, b.y, BALL_SIZE, b.color) end
  fill_rect(paddle_x, PADDLE_Y, PADDLE_WIDTH, 4, paddle_glow > 0 and 13 or 15)
  draw_juice()
  draw_number(score, 4, 3, 15)
  draw_text("BEST " .. best, 136, 3, 12)
  if game_over then
    center("GAME OVER", 50, 15)
    if over_timer > 30 then center("PRESS A", 64, 13) end
  end
end
