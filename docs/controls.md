# Key Bindings (Game)

| Button | Keys              |
| :------| :------------------|
| Up     | `ArrowUp`, `W`    |
| Down   | `ArrowDown`, `S`  |
| Left   | `ArrowLeft`, `A`  |
| Right  | `ArrowRight`, `D` |
| A      | `J`               |
| B      | `K`               |
| Select | `Shift`           |
| START  | `Enter`           |

A connected gamepad works out of the box — D-pad for direction, the south
face button (A / Cross) for A, the east one (B / Circle) for B, `Back` for
Select and `Start` for START. Handhelds expose their built-in buttons this
way, so this is the path that matters on device.

START belongs to the console, not to the cart: it opens the pause menu. On a
device with no physical START, **holding B for about half a second** does the
same thing, so the menu is always reachable. A short B press is unaffected.

Override by creating `controls.toml` next to the binary:

```toml
[controls]
up     = ["ArrowUp", "KeyW"]
down   = ["ArrowDown", "KeyS"]
left   = ["ArrowLeft", "KeyA"]
right  = ["ArrowRight", "KeyD"]
a      = ["KeyJ"]
b      = ["KeyK"]
select = ["ShiftLeft", "ShiftRight"]
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

Every field is optional, including `select` and `start` — a `controls.toml`
written before those existed keeps working and picks up the defaults above.
Binding the same input to both `start` and a cart button gives it to START;
the cart binding is dropped and a warning is logged.

Key names are physical positions, not layout characters: letters `KeyA`–`KeyZ`, digits `Digit0`–`Digit9`, `ArrowUp`/`ArrowDown`/`ArrowLeft`/`ArrowRight`, `Space`, `Enter`, `Escape`, `Backspace`, `Tab`, and the left/right `Shift`/`Control`/`Alt` pairs. Gamepad names follow SDL's controller vocabulary: `DPadUp`/`DPadDown`/`DPadLeft`/`DPadRight`, `South`/`East`/`West`/`North`, `LeftShoulder`/`RightShoulder`, `Start`, `Back`, `Guide`.

A missing file, an unparseable one, or an unknown name falls back to the defaults.

Handheld builds (Miyoo, TrimUI, Anbernic) are documented in [handheld-builds.md](development/handheld-builds.md).
