# Game controls

| Button | Keys              |
| :------| :------------------|
| Up     | `ArrowUp`, `W`    |
| Down   | `ArrowDown`, `S`  |
| Left   | `ArrowLeft`, `A`  |
| Right  | `ArrowRight`, `D` |
| A      | `J`, `Z`, `Space` |
| B      | `K`, `X`          |
| Select | `Backspace`       |
| START  | `Enter`           |

We bind physical key positions, so your keyboard layout doesn't matter. On a
QWERTZ keyboard, `Z` means the key labelled Y. Machine, Studio and the web
player use the same defaults. Studio's Controls dialog rebinds keys for its
preview window.

Gamepads work without setup. The D-pad moves, the south face button (A or
Cross) is A, the east button (B or Circle) is B, `Back` is Select and
`Start` is START. Handhelds report their built-in buttons as a gamepad, so
this mapping is the one you'll use on a device.

START opens the console's pause menu and never reaches the cart. If your
device has no START button, hold B for about half a second instead. A short
B press still goes to the game.

The pause menu's Save state and Load state copy the cart's RAM and palette.
They skip Lua variables (globals, locals and upvalues), so a cart that keeps
its state in Lua can come back out of sync. If your game needs saves that
hold up, use `save_data` and `load_data`.

## Overrides

To change the bindings, put a `controls.toml` in Machine's data folder. The
same folder holds `carts/`, `saves/` and `settings.toml`.

| Where Machine runs | Data folder |
| :-- | :-- |
| A `carts/` folder sits next to the binary (handhelds, portable zips) | the binary's folder |
| Windows | `%APPDATA%\caiven-machine` |
| macOS | `~/Library/Application Support/caiven-machine` |
| Linux | `$XDG_DATA_HOME/caiven-machine`, default `~/.local/share/caiven-machine` |

Machine prints the path at startup.

```toml
[controls]
up     = ["ArrowUp", "KeyW"]
down   = ["ArrowDown", "KeyS"]
left   = ["ArrowLeft", "KeyA"]
right  = ["ArrowRight", "KeyD"]
a      = ["KeyJ", "KeyZ", "Space"]
b      = ["KeyK", "KeyX"]
select = ["Backspace"]
start  = ["Enter"]

# Optional. Omit the table entirely to keep the defaults below.
[gamepad]
up     = ["DPadUp"]
down   = ["DPadDown"]
left   = ["DPadLeft"]
right  = ["DPadRight"]
a      = ["South"]
b      = ["East"]
select = ["Back"]
start  = ["Start"]
```

You can leave out any field and it keeps its default. If you bind one input
to both `start` and a cart button, START wins and Machine logs a warning.

Key names describe physical positions: `KeyA` to `KeyZ`, `Digit0` to
`Digit9`, `ArrowUp`, `ArrowDown`, `ArrowLeft`, `ArrowRight`, `Space`,
`Enter`, `Escape`, `Backspace`, `Tab`, and left and right `Shift`,
`Control` and `Alt`. Gamepad names follow SDL: `DPadUp`, `DPadDown`,
`DPadLeft`, `DPadRight`, `South`, `East`, `West`, `North`, `LeftShoulder`,
`RightShoulder`, `Start`, `Back` and `Guide`.

If the file is missing, fails to parse, or uses an unknown name, Machine
falls back to the defaults.

For Miyoo, TrimUI and Anbernic builds, see
[handheld-builds.md](development/handheld-builds.md).
