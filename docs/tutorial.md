# Tutorial: your first game

A game lives in a project folder: `caiven.toml`, `main.lua`, any
other `.lua` files you `require()` from it, and one asset file per non-empty
section (`.png` by default, `.hex` also works). Plain files keep `git diff`
readable. When you want to share the game, you pack the folder into a single
`.cav` file with `caiven-studio build` or Studio's Pack Cartridge command.
`caiven-studio unpack` turns a `.cav` back into a folder. Studio edits
folders only, so if you open a `.cav` it asks to unpack it first.

## 1. Create a project

Start Studio and click **New cart** on the start screen:

```bash
cargo run -p caiven-studio -- edit
```

Pick an empty folder. Its name becomes the cart title. Studio writes a blank
project with `_init` and `_update` and opens the Code workspace.

## 2. Write the game

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
  draw_text("score", 2, 2, 15)
  draw_number(score, 26, 2, 15)
end
```

## 3. Draw the player

Press `F2` to open the sprite tab and paint sprite 0.

## 4. Run and debug

The toolbar has Run, Pause and Reset. `Ctrl+R` reruns the cart. Click the
gutter in the code editor to set a breakpoint. If your Lua code throws an
error, the status bar shows the message and line number.

## 5. Save and share

`Ctrl+S` saves code, sprites, map and audio into the project folder. Set
the title and author on the `F7` meta tab.

To play the game without the editor, run `caiven-machine my-game/` (`Ctrl+R`
restarts it). To share it, choose File → Export → Pack Cartridge (.cav),
then run `publish game.cav` to upload it to a Port.

## Lifecycle functions

| Function    | When it runs         |
| :---------- | :------------------- |
| `_init()`   | Once, when the cart loads |
| `_update()` | Once per frame. You don't call `wait()` or sync to the screen yourself. |
| `_draw()`   | Optional. Once per frame, after `_update()`. Use it if you want drawing code separate from game logic. |

The [API reference](api-reference.md) lists every builtin.
