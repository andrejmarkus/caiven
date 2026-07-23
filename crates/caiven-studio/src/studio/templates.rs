//! Cart templates: readable starting points for NEW CART / the welcome
//! screen, generalizing the old single `BOILERPLATE` const in `app.rs`.
//! Also doubles as the project's in-repo, commented example code.

pub struct CartTemplate {
    pub name: &'static str,
    pub description: &'static str,
    pub source: &'static str,
}

pub const BLANK: &str = "function _init()\nend\n\nfunction _update()\n  clear_screen()\nend\n";

const MOVER: &str = r#"-- Top-down mover: arrow keys move sprite 0 around the screen
local SPEED = 2

local x = 60
local y = 60

function _init()
  set_palette_color(0, 10, 10, 30)
  set_palette_color(1, 200, 200, 255)
end

function _update()
  clear_screen()
  if button_down(0) then y = y - SPEED end
  if button_down(1) then y = y + SPEED end
  if button_down(2) then x = x - SPEED end
  if button_down(3) then x = x + SPEED end
  sprite(0, x, y)
end
"#;

const SCORE: &str = r#"-- Tap to score: a bouncing ball, a table for its state, a HUD score
local ball
local score = 0
local hi = 0

function _init()
  set_palette_color(0, 10, 10, 20)
  set_palette_color(1, 255, 255, 255)
  set_palette_color(2, 220, 40, 40)
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

  -- button 4/5 = a couple of the extra buttons past the d-pad
  if button_down(4) then score = score + 1 end
  if button_down(5) then score = score - 1 end
  if score < 0 then score = 0 end
  if score > hi then hi = score end

  sprite(0, ball.x, ball.y)
  draw_text("SCORE:", 2, 2, 7)
  draw_number(score, 44, 2, 7)
  draw_text("HI:", 2, 10, 7)
  draw_number(hi, 44, 10, 5)
end
"#;

const TILES: &str = r#"-- Tile world: a map with per-tile collision via sprite flags
-- Sprite 1 = floor (flag 0), sprite 2 = wall (flag FLAG_SOLID)
local MAZE_W, MAZE_H = 16, 16
local FLAG_SOLID = 1

local maze = {
  "2222222222222222",
  "2111111111111112",
  "2122222222222212",
  "2121111111111212",
  "2121222222221212",
  "2121211111121212",
  "2121212222121212",
  "2121212112121212",
  "2121212112121212",
  "2121212222121212",
  "2121211111121212",
  "2121222222221212",
  "2121111111111212",
  "2122222222222212",
  "2111111111111112",
  "2222222222222222",
}

local player_x, player_y = 8, 8

local function solid_at(px, py)
  local cx = math.floor(px / 8)
  local cy = math.floor(py / 8)
  return get_sprite_flags(get_tile(cx, cy)) == FLAG_SOLID
end

function _init()
  set_palette_color(0, 0, 0, 0)
  set_palette_color(1, 60, 60, 60)
  set_palette_color(2, 120, 120, 120)
  set_palette_color(3, 255, 100, 100)

  set_sprite_flags(2, FLAG_SOLID)
  for y = 0, MAZE_H - 1 do
    for x = 0, MAZE_W - 1 do
      set_tile(x, y, tonumber(maze[y + 1]:sub(x + 1, x + 1)))
    end
  end
end

function _update()
  clear_screen()
  draw_map(0, 0, 0, 0, MAZE_W, MAZE_H)
  sprite(0, player_x, player_y)

  if button_down(2) and not solid_at(player_x - 1, player_y) and not solid_at(player_x - 1, player_y + 7) then
    player_x = player_x - 1
  end
  if button_down(3) and not solid_at(player_x + 8, player_y) and not solid_at(player_x + 8, player_y + 7) then
    player_x = player_x + 1
  end
  if button_down(0) and not solid_at(player_x, player_y - 1) and not solid_at(player_x + 7, player_y - 1) then
    player_y = player_y - 1
  end
  if button_down(1) and not solid_at(player_x, player_y + 8) and not solid_at(player_x + 7, player_y + 8) then
    player_y = player_y + 1
  end
end
"#;

pub const TEMPLATES: [CartTemplate; 4] = [
    CartTemplate {
        name: "Blank",
        description: "Empty _init/_update stub",
        source: BLANK,
    },
    CartTemplate {
        name: "Top-down mover",
        description: "Move a sprite around with the arrow keys",
        source: MOVER,
    },
    CartTemplate {
        name: "Tap to score",
        description: "Bouncing ball, a table, a HUD score counter",
        source: SCORE,
    },
    CartTemplate {
        name: "Tile world",
        description: "draw_map + per-tile collision via sprite flags",
        source: TILES,
    },
];
