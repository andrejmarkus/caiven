//! Built-in starting points for new cartridges.

use serde::Serialize;

pub struct CartTemplate {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub source: &'static str,
    /// Sprite-sheet seeds as `(sprite_index, 8x8 palette-index pixels)`.
    /// Written into the sprite bank when the template is instantiated, so
    /// the first `Run` shows something visible instead of an invisible
    /// sprite (pixel value `0` is the transparent key, matching the `sprite`
    /// and `draw_map` builtins in `caiven-vm`). Empty for templates whose
    /// script never draws a sprite.
    pub sprite_seed: &'static [(u8, [u8; 64])],
}

/// A filled circle/blob, 8x8, drawn with palette index `fill`.
const fn blob(fill: u8) -> [u8; 64] {
    #[rustfmt::skip]
    const MASK: [u8; 64] = [
        0, 0, 1, 1, 1, 1, 0, 0,
        0, 1, 1, 1, 1, 1, 1, 0,
        1, 1, 1, 1, 1, 1, 1, 1,
        1, 1, 1, 1, 1, 1, 1, 1,
        1, 1, 1, 1, 1, 1, 1, 1,
        1, 1, 1, 1, 1, 1, 1, 1,
        0, 1, 1, 1, 1, 1, 1, 0,
        0, 0, 1, 1, 1, 1, 0, 0,
    ];
    let mut out = [0u8; 64];
    let mut i = 0;
    while i < 64 {
        out[i] = if MASK[i] != 0 { fill } else { 0 };
        i += 1;
    }
    out
}

/// A fully solid 8x8 tile, drawn with palette index `fill`.
const fn solid(fill: u8) -> [u8; 64] {
    [fill; 64]
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
        // MOVER's _init sets palette index 1 to the light color it draws with.
        sprite_seed: &[(0, blob(1))],
    },
    CartTemplate {
        id: "tap-to-score",
        name: "Tap to score",
        description: "Bouncing ball with score and high-score HUD",
        source: SCORE,
        // SCORE's _init sets palette index 1 to white for the ball.
        sprite_seed: &[(0, blob(1))],
    },
    CartTemplate {
        id: "tile-world",
        name: "Tile world",
        description: "Map drawing and per-cell collision",
        source: TILES,
        // TILES' _init sets index 1 = floor gray, index 2 = wall gray,
        // index 3 = player red; tile ids 1/2 index sprites 1/2 directly.
        sprite_seed: &[(0, blob(3)), (1, solid(1)), (2, solid(2))],
    },
    CartTemplate {
        id: "blank",
        name: "Blank",
        description: "Empty _init and _update starting point",
        source: BLANK,
        sprite_seed: &[],
    },
];

pub fn find(id: &str) -> Option<&'static CartTemplate> {
    TEMPLATES.iter().find(|template| template.id == id)
}

/// Builds a full sprite-sheet-length buffer (`SPRITE_SHEET_LEN` bytes,
/// `SPRITE_BYTES` per sprite) from a template's `sprite_seed`, ready to hand
/// to `cart::apply_sections` under `SectionKind::SpriteSheet`. Empty for
/// templates with no seed, matching today's blank-sheet behavior.
pub fn sprite_sheet_bytes(template: &CartTemplate) -> Vec<u8> {
    use caiven_core::memory::{SPRITE_BYTES, SPRITE_SHEET_LEN};

    let mut sheet = vec![0u8; SPRITE_SHEET_LEN];
    for (index, pixels) in template.sprite_seed {
        let offset = *index as usize * SPRITE_BYTES;
        sheet[offset..offset + SPRITE_BYTES].copy_from_slice(pixels);
    }
    sheet
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

    #[test]
    fn blank_template_has_no_sprite_seed() {
        let blank = find("blank").expect("blank template");
        assert!(blank.sprite_seed.is_empty());
        assert!(sprite_sheet_bytes(blank).iter().all(|&byte| byte == 0));
    }

    #[test]
    fn every_visual_template_seeds_a_visible_sprite_0() {
        // Every template whose script draws with `sprite(0, ...)` (all but
        // `blank`) must ship at least one non-transparent pixel in sprite 0,
        // otherwise the template's first Run renders nothing.
        for template in TEMPLATES.iter().filter(|t| t.id != "blank") {
            let sheet = sprite_sheet_bytes(template);
            let sprite_0 = &sheet[..caiven_core::memory::SPRITE_BYTES];
            assert!(
                sprite_0.iter().any(|&pixel| pixel != 0),
                "{} seeds no visible pixels in sprite 0",
                template.id
            );
        }
    }

    #[test]
    fn seeded_sprites_round_trip_through_vm_and_disk() {
        use crate::app::cart_io::{self, CartMeta};
        use caiven_cart::{CartHeader, SectionKind};
        use caiven_vm::AssetBankKind;

        let dir = std::env::temp_dir().join(format!(
            "caiven-template-seed-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).expect("create temp project dir");

        let template = find("tile-world").expect("tile-world template");
        let mut console = ConsoleCore::new().expect("console core");
        console.reset_vm();
        cart::apply_sections(
            &mut console.vm,
            &[(SectionKind::SpriteSheet, sprite_sheet_bytes(template))],
        );

        // Sprite bank in RAM reflects the seed immediately.
        let live = console
            .vm
            .asset_bank_bytes(AssetBankKind::Sprites, 0)
            .expect("sprite bank 0");
        assert!(live.iter().any(|&pixel| pixel != 0));

        // And it round-trips through a project save/load, exactly like a
        // real `new_project` -> disk -> reopen cycle.
        let meta = CartMeta {
            path: dir.clone(),
            header: CartHeader::default_for("seed-test"),
            program: Vec::new(),
            sections: cart::default_section_layout(),
            lua_source: Some(template.source.to_string()),
        };
        cart_io::save(&console.vm, &meta, &[]).expect("save project");

        let mut reloaded = ConsoleCore::new().expect("console core");
        reloaded.reset_vm();
        cart::load_cart(&mut reloaded.vm, &dir, &reloaded.input, &reloaded.font)
            .expect("reload project");
        let reloaded_bank = reloaded
            .vm
            .asset_bank_bytes(AssetBankKind::Sprites, 0)
            .expect("reloaded sprite bank 0");
        assert_eq!(reloaded_bank, live);

        std::fs::remove_dir_all(&dir).ok();
    }
}
