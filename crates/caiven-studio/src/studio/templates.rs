//! Built-in starting points for new cartridges.

use serde::Serialize;

pub struct CartTemplate {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub source: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CartTemplateSummary {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
}

const BLANK: &str = "function _init()\nend\n\nfunction _update()\n  clear_screen()\nend\n";

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

const SCORE: &str = r#"-- Tap to score: a bouncing ball, a table, a HUD score
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

  -- button 4/5 = extra buttons past the d-pad
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

const TILES: &str = r#"-- Tile world: a map with per-cell collision
-- Sprite 1 = floor, sprite 2 = wall
local MAZE_W, MAZE_H = 16, 16

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
  return tile_solid(cx, cy)
end

function _init()
  set_palette_color(0, 0, 0, 0)
  set_palette_color(1, 60, 60, 60)
  set_palette_color(2, 120, 120, 120)
  set_palette_color(3, 255, 100, 100)

  for y = 0, MAZE_H - 1 do
    for x = 0, MAZE_W - 1 do
      local tile = tonumber(maze[y + 1]:sub(x + 1, x + 1))
      set_tile(x, y, tile)
      set_collision(x, y, tile == 2 and 1 or 0)
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
        id: "top-down-mover",
        name: "Top-down mover",
        description: "Move a sprite around with arrow keys",
        source: MOVER,
    },
    CartTemplate {
        id: "tap-to-score",
        name: "Tap to score",
        description: "Bouncing ball with score and high-score HUD",
        source: SCORE,
    },
    CartTemplate {
        id: "tile-world",
        name: "Tile world",
        description: "Map drawing and per-cell collision",
        source: TILES,
    },
    CartTemplate {
        id: "blank",
        name: "Blank",
        description: "Empty _init and _update starting point",
        source: BLANK,
    },
];

pub fn find(id: &str) -> Option<&'static CartTemplate> {
    TEMPLATES.iter().find(|template| template.id == id)
}

pub fn summaries() -> Vec<CartTemplateSummary> {
    TEMPLATES
        .iter()
        .map(|template| CartTemplateSummary {
            id: template.id,
            name: template.name,
            description: template.description,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::studio::{SourceFile, cart};
    use caiven_vm::runtime::ConsoleCore;
    use std::collections::HashSet;
    use std::path::PathBuf;

    #[test]
    fn template_ids_are_stable_and_unique() {
        let ids = TEMPLATES
            .iter()
            .map(|template| template.id)
            .collect::<Vec<_>>();
        assert_eq!(
            ids,
            ["top-down-mover", "tap-to-score", "tile-world", "blank"]
        );
        assert_eq!(ids.iter().copied().collect::<HashSet<_>>().len(), ids.len());
    }

    #[test]
    fn every_template_has_runnable_hooks() {
        for template in &TEMPLATES {
            assert!(template.source.contains("function _init()"));
            assert!(template.source.contains("function _update()"));
        }
    }

    #[test]
    fn every_template_compiles_against_current_vm_api() {
        let mut console = ConsoleCore::new().expect("console core");
        for template in &TEMPLATES {
            console.reset_vm();
            let sources = [SourceFile {
                path: PathBuf::from("main.lua"),
                text: template.source.to_string(),
                dirty: false,
            }];
            if let Err(error) = cart::compile_sources_into_vm(
                &mut console.vm,
                None,
                &sources,
                &console.input,
                &console.font,
            ) {
                panic!("{} template failed: {}", template.id, error.message);
            }
        }
    }

    #[test]
    fn unknown_template_is_rejected() {
        assert!(find("not-a-template").is_none());
    }
}
