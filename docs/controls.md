# Key Bindings (Game)

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

Keys are physical positions, so they stay put on any layout (on a QWERTZ
keyboard, `Z` is the key labelled Y). Machine, Studio and the web player share
these defaults; Studio's Controls dialog rebinds them for its preview.

A connected gamepad works out of the box — D-pad for direction, the south
face button (A / Cross) for A, the east one (B / Circle) for B, `Back` for
Select and `Start` for START. Handhelds expose their built-in buttons this
way, so this is the path that matters on device.

START belongs to the console, not to the cart: it opens the pause menu. On a
device with no physical START, **holding B for about half a second** does the
same thing, so the menu is always reachable. A short B press is unaffected.

**Save state** and **Load state** in that menu snapshot the cart's RAM and
palette only. Lua variables (globals, locals, upvalues) are not captured, so a
cart that keeps state in Lua rather than RAM can resume inconsistent. Carts
that need dependable persistence should use `save_data` / `load_data`.

Override with a `controls.toml` in Machine's data folder, which also holds
`carts/`, `saves/` and `settings.toml`:

| Where Machine runs | Data folder |
| :-- | :-- |
| A `carts/` folder sits next to the binary (handhelds, portable zips) | the binary's folder |
| Windows | `%APPDATA%\caiven-machine` |
| macOS | `~/Library/Application Support/caiven-machine` |
| Linux | `$XDG_DATA_HOME/caiven-machine`, default `~/.local/share/caiven-machine` |

Machine logs the library path at startup.

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

Every field is optional; a missing one keeps its default.
Binding the same input to both `start` and a cart button gives it to START;
the cart binding is dropped and a warning is logged.

Key names are physical positions, not layout characters: letters `KeyA`–`KeyZ`, digits `Digit0`–`Digit9`, `ArrowUp`/`ArrowDown`/`ArrowLeft`/`ArrowRight`, `Space`, `Enter`, `Escape`, `Backspace`, `Tab`, and the left/right `Shift`/`Control`/`Alt` pairs. Gamepad names follow SDL's controller vocabulary: `DPadUp`/`DPadDown`/`DPadLeft`/`DPadRight`, `South`/`East`/`West`/`North`, `LeftShoulder`/`RightShoulder`, `Start`, `Back`, `Guide`.

A missing file, an unparseable one, or an unknown name falls back to the defaults.

Handheld builds (Miyoo, TrimUI, Anbernic) are documented in [handheld-builds.md](development/handheld-builds.md).
