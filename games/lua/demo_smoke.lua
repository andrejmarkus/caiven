-- Smoke test: tables + strings + arithmetic + button input
-- "tap to score": ball bounces, score tracked with hi-score

local ball
local score = 0
local hi = 0
local frame = 0

function _init()
  ball = {x = 64, y = 64, dx = 2, dy = 1}
end

function _update()
  clear_screen()

  if ball.x >= 120 then ball.dx = -2 end
  if ball.x <= 4 then ball.dx = 2 end
  if ball.y >= 120 then ball.dy = -1 end
  if ball.y <= 4 then ball.dy = 1 end

  ball.x = ball.x + ball.dx
  ball.y = ball.y + ball.dy

  if button_down(4) then score = score + 1 end
  if button_down(5) then score = score - 1 end
  if score < 0 then score = 0 end
  if score > hi then hi = score end

  sprite(0, ball.x, ball.y)

  draw_text("SCORE:", 2, 2, 7)
  draw_number(score, 44, 2, 7)
  draw_text("HI:", 2, 10, 7)
  draw_number(hi, 44, 10, 5)

  frame = frame + 1
end
