# Tutorial: Your First Game

Caiven has two formats. You **author** a game as a plain project directory —
`caiven.toml` + `main.lua` (plus any sibling `.lua` modules, `require()`-able
from each other) + one asset file per non-empty section (`.png` by default,
`.hex` also supported) — so `git diff` shows real changes instead of a binary
blob. You **distribute** as a single `.cav` file built from the project dir
with `caiven-studio build` (or Studio's Pack Cartridge). `caiven-studio
unpack` goes the other way. Studio only edits project dirs — pointing it at a
`.cav` prompts to unpack first.

1. **Launch Caiven Studio** and click **New cart** on the start screen:

```bash
cargo run -p caiven-studio -- edit
```

A folder picker asks for an empty project directory (the folder name becomes
the cart title); Studio creates a blank `_init`/`_update` project and opens
the Code workspace.

2. **Write your game logic:**

```lua
local SPEED = 2

local x = 60
local y = 60
local score = 0

function _init()
  set_palette_color(0, 10, 10, 30)  -- dark blue background
end

function _update()
  clear_screen()

  if button_down(2) then x = x - SPEED end  -- left
  if button_down(3) then x = x + SPEED end  -- right
  if button_down(0) then y = y - SPEED end  -- up
  if button_down(1) then y = y + SPEED end  -- down

  if button_pressed(4) then  -- A pressed this frame
    score = score + 1
    play_sfx(0)
  end

  sprite(0, x, y)
  draw_text("score", 2, 2, 7)
  draw_number(score, 26, 2, 7)
end
```

3. **Draw your player** — press `F2` for the sprite tab and paint sprite 0.

4. **Iterate** — click the code editor's gutter to set a line breakpoint, the toolbar's Run/Pause/Reset drives execution (or `Ctrl+R` to rerun). Lua errors show with a line number and message straight in the status bar.

5. **Ship it** — `Ctrl+S` writes code + sprites + map + audio into the project dir (set title/author on the `F7` meta tab), then run it standalone with `caiven-machine my-game/` (hot-reloads with `Ctrl+R`, no editor needed), or build + publish a distribution cartridge: File → Export → Pack Cartridge (.cav), then `publish game.cav` to share it on a port.

## Cart lifecycle functions

| Function    | Purpose                                                                                                                                                  |
| :---------- | :----------------------------------------------------------------------------------------------------------------------------------------------------- |
| `_init()`   | Runs once when the cart loads                                                                                                                            |
| `_update()` | Runs once per frame (called for you — no `wait()`/vsync call needed)                                                                                     |
| `_draw()`   | Optional — runs once per frame, right after `_update()`. Split game logic from rendering if you like; carts with only `_update()` work exactly as before |

See the full [API reference](api-reference.md) for every builtin function.
