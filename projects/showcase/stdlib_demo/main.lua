

GROUND_TILE_Y = 13
GROUND_Y = GROUND_TILE_Y * 8
PLAYER_W = 8
PLAYER_H = 8
px = 60
py = GROUND_Y - PLAYER_H
vy = 0
walk_anim = new_anim({ 2, 3 }, 8)
score = 0
high_score = 0
coin_x = 16
coin_y = GROUND_Y - 8
tweens = {}
function respawn_coin()
coin_x = math.random(0, 15) * 8
coin_y = GROUND_Y - 8
end
function _init()
high_score = dget(0)
tweens[1] = new_tween(24, 116, 90, ease_linear)
tweens[2] = new_tween(24, 116, 90, ease_in_quad)
tweens[3] = new_tween(24, 116, 90, ease_out_quad)
tweens[4] = new_tween(24, 116, 90, ease_in_out_quad)
for i = 1, 4 do
tweens[i].val = tweens[i].from
end
end
function _update()
local moving = false
if button_down(2) then
px = px - 2
moving = true
end
if button_down(3) then
px = px + 2
moving = true
end
px = clamp(px, 0, 128 - PLAYER_W)
local on_ground = box_touches_solid(px, py + PLAYER_H, PLAYER_W, 1)
if on_ground then
vy = 0
if button_down(0) or button_down(4) then
vy = -6
end
else
vy = clamp(vy + 1, -6, 4)
end
local next_y = py + vy
if vy > 0 and box_touches_solid(px, next_y + PLAYER_H, PLAYER_W, 1) then
next_y = GROUND_Y - PLAYER_H
vy = 0
end
py = next_y
if moving then anim_update(walk_anim) end
if aabb_overlap(px, py, PLAYER_W, PLAYER_H, coin_x, coin_y, 8, 8) then
for i = 1, 12 do
Particles.spawn(px + 4, py + 4, (math.random(0, 100) - 50) / 25, -math.random(0, 20) / 10, 10, 25)
end
score = score + 1
if score > high_score then
high_score = score
dset(0, high_score)
end
respawn_coin()
end
if button_pressed(5) then
local confetti = { 9, 10, 11, 14 }
for i = 1, 14 do
Particles.spawn(
px + 4,
py + 4,
(math.random(0, 100) - 50) / 20,
-1 - math.random(0, 30) / 10,
confetti[math.random(1, 4)],
35
)
end
end
Particles.update()
for i = 1, 4 do
local tw = tweens[i]
tw.val = tween_update(tw)
if tw.done then
local nxt = new_tween(tw.to, tw.from, 90, tw.ease)
nxt.val = tw.val
tweens[i] = nxt
end
end
end
function _draw()
local bg = math.floor(lerp(1, 2, (math.sin(frame_count() * 0.03) + 1) / 2))
fill_screen(bg)
draw_map(0, GROUND_TILE_Y, 0, GROUND_Y, 16, 1)
fill_rect(math.floor(coin_x), math.floor(coin_y), 8, 8, 10)
sprite(anim_sprite(walk_anim), math.floor(px), math.floor(py))
Particles.draw()
local ease_colors = { 11, 12, 14, 9 }
local labels = { "LIN", "EIN", "EOUT", "INOUT" }
for i = 1, 4 do
local y = 3 + (i - 1) * 7
draw_text(labels[i], 2, y, 7)
fill_rect(math.floor(tweens[i].val), y, 4, 4, ease_colors[i])
end
draw_text("ARROWS MOVE  A JUMP  B BURST", 2, 112, 6)
draw_text("SCORE", 2, 120, 7)
draw_number(score, 40, 120, 10)
draw_text("HI", 70, 120, 7)
draw_number(high_score, 90, 120, 10)
end
