# 🎮 Caiven

![Rust](https://img.shields.io/badge/rust-%23E32F26.svg?style=for-the-badge&logo=rust&logoColor=white)
![License](https://img.shields.io/badge/license-MPL--2.0-blue.svg?style=for-the-badge)

**Caiven** is a retro-inspired fantasy console written in Rust. It combines a Lua 5.4 game runtime, a complete desktop creation environment called **Caiven Studio**, a lightweight standalone player called **Caiven Machine**, and an optional self-hostable cart-sharing service.

> [!TIP]
> Write real Lua — standard libraries such as `math`, `string`, `table`, and `pcall` work without a custom dialect or bytecode language.

> [!NOTE]
> You own the games and assets you create. You may sell them without royalties, a commercial-use fee, or a requirement to publish your game source. See [Creator rights](CREATOR_RIGHTS.md).

---

## ⬇️ Downloads

Choose only the application you need. No Rust toolchain or source compilation is required.

### Caiven Studio

Use **Studio** to create, edit, debug, publish, and play Caiven carts.

| Platform | Download |
| :------- | :------- |
| **Windows 64-bit** | [Download Studio ZIP](https://github.com/andrejmarkus/caiven/releases/latest/download/caiven-studio-windows-x86_64.zip) |
| **Linux 64-bit** | [Download Studio tar.gz](https://github.com/andrejmarkus/caiven/releases/latest/download/caiven-studio-linux-x86_64.tar.gz) |
| **macOS Apple Silicon** | [Download Studio tar.gz](https://github.com/andrejmarkus/caiven/releases/latest/download/caiven-studio-macos-arm64.tar.gz) |

### Caiven Machine

Use **Machine** when you only want to run `.cav` games without installing the editor.

| Platform | Download |
| :------- | :------- |
| **Windows 64-bit** | [Download Machine ZIP](https://github.com/andrejmarkus/caiven/releases/latest/download/caiven-machine-windows-x86_64.zip) |
| **Linux 64-bit** | [Download Machine tar.gz](https://github.com/andrejmarkus/caiven/releases/latest/download/caiven-machine-linux-x86_64.tar.gz) |
| **macOS Apple Silicon** | [Download Machine tar.gz](https://github.com/andrejmarkus/caiven/releases/latest/download/caiven-machine-macos-arm64.tar.gz) |

### Demo carts

Download ready-to-run examples from the [demo carts directory](https://github.com/andrejmarkus/caiven/tree/master/games/carts).

> [!IMPORTANT]
> Direct download links become available after the first tagged release is published. Maintainers can create one by pushing a tag such as `v0.1.0`.

[View all releases](https://github.com/andrejmarkus/caiven/releases)

---

## ✨ Features

- 🌙 **Real Lua 5.4** — embedded through `mlua`, vendored with the runtime
- 🎨 **Palette graphics** — 128×128 resolution, 16-color swappable palette, sprites, tilemap, primitives, and camera
- 📦 **Readable built-in API** — names such as `sprite`, `draw_rect`, `button_down`, and `set_palette_color`
- 🔊 **Audio engine** — real-time sound synthesis, SFX banks, music banks, and CPAL playback
- 🧰 **Gameplay standard library** — tweens, easing, AABB and tile collision, particles, and sprite animation
- 🖌️ **Caiven Studio** — code, sprite, map, palette, SFX, music, metadata, browser, and help editors in one application
- 🔍 **Debugger** — line breakpoints, pause, frame stepping, globals inspection, RAM inspection, and `.cavdbg` persistence
- 🌐 **Caiven Port** — self-hostable cart gallery with accounts, versions, ratings, comments, discovery, and browser play

---

## 🚀 Getting Started

### Creating games with Studio

Extract the Studio archive and launch:

```bash
caiven-studio
```

On Windows, run:

```powershell
.\caiven-studio.exe
```

Open an existing cart directly:

```bash
caiven-studio edit game.cav
```

Studio opens on a welcome screen with **New Cart**, **Open**, recent carts, and starter templates.

### Playing games with Machine

Extract the Machine archive, place a `.cav` cart next to the executable, and run:

```bash
caiven-machine game.cav
```

On Windows:

```powershell
.\caiven-machine.exe game.cav
```

You can get example games from the [demo carts directory](https://github.com/andrejmarkus/caiven/tree/master/games/carts).

### Building from source

Install the [Rust toolchain](https://rustup.rs/), then clone the repository:

```bash
git clone https://github.com/andrejmarkus/caiven.git
cd caiven
```

Build Studio only:

```bash
cargo build --release -p caiven-studio
```

Build Machine only:

```bash
cargo build --release -p caiven-machine
```

Build both:

```bash
cargo build --release -p caiven-studio -p caiven-machine
```

Run Studio from the source tree:

```bash
cargo run -p caiven-studio
```

Run a cart through Machine from the source tree:

```bash
cargo run -p caiven-machine -- games/carts/catch.cav
```

### Studio commands

| Command | Description |
| :------ | :---------- |
| _(no command)_ | Launch Studio and open the cart browser |
| `edit [file]` | Launch Studio, optionally opening a `.cav` file |
| `inspect <file.cav>` | Print the cart section table |
| `publish <file.cav>` | Upload a cart to a Caiven Port instance |

### Publish flags

| Flag | Default | Description |
| :--- | :------ | :---------- |
| `--url` | `http://localhost:8080` | Port base URL; environment variable: `CAIVEN_PORT_URL` |
| `--api-key` | required | Per-user Port token; environment variable: `CAIVEN_PORT_API_KEY` |
| `--title` | cart header | Override the cart title |
| `--author` | cart header | Override the author |
| `--description` | empty | Short description |
| `--tags` | empty | Comma-separated tags |
| `--frames` | `30` | Frames to run before capturing a screenshot |
| `--no-screenshot` | off | Skip screenshot capture and upload |

---

## 🐣 Tutorial: Your First Game

Every cart is a single `.cav` binary bundle containing Lua source, sprites, map data, palette, SFX, music, and metadata.

1. Launch Studio and choose **New Cart**.
2. Open the code tab with `F1`.
3. Add game logic:

```lua
local SPEED = 2
local x = 60
local y = 60
local score = 0

function _init()
  set_palette_color(0, 10, 10, 30)
end

function _update()
  clear_screen()

  if button_down(2) then x = x - SPEED end
  if button_down(3) then x = x + SPEED end
  if button_down(0) then y = y - SPEED end
  if button_down(1) then y = y + SPEED end

  if button_pressed(4) then
    score = score + 1
    play_sfx(0)
  end

  sprite(0, x, y)
  draw_text("score", 2, 2, 7)
  draw_number(score, 26, 2, 7)
end
```

4. Press `F2` and draw sprite `0`.
5. Use Run, Pause, Reset, or `Ctrl+R` to iterate.
6. Press `Ctrl+S` to save the cart.
7. Run the result with `caiven-machine game.cav`, or publish it through the Port tab.

### Cart lifecycle functions

| Function | Purpose |
| :------- | :------ |
| `_init()` | Runs once when the cart loads |
| `_update()` | Runs once per frame |
| `_draw()` | Optional rendering callback, called after `_update()` |

---

## 📝 Built-in API Overview

Lua math, strings, tables, and other standard facilities use Lua 5.4 directly.

### Graphics

| Function | Description |
| :------- | :---------- |
| `clear_screen()` | Clear the screen and UI layer |
| `fill_screen(color)` | Fill the screen with a palette color |
| `set_pixel(x, y, color)` | Set one pixel |
| `draw_line(x0, y0, x1, y1, color)` | Draw a line |
| `draw_rect(...)` / `fill_rect(...)` | Draw outlined or filled rectangles |
| `draw_circle(...)` / `fill_circle(...)` | Draw outlined or filled circles |
| `set_palette_color(index, r, g, b)` | Change a palette entry |
| `set_camera(x, y)` | Set the camera offset |
| `draw_text(text, x, y, color)` | Draw text on screen |
| `draw_number(value, x, y, color)` | Draw an integer |

### Sprites and map

| Function | Description |
| :------- | :---------- |
| `sprite(id, x, y)` | Draw an 8×8 sprite |
| `draw_map(cell_x, cell_y, sx, sy, w, h)` | Draw part of the tilemap |
| `get_tile(x, y)` / `set_tile(x, y, tile)` | Read or write a map cell |
| `get_sprite_flags(id)` / `set_sprite_flags(id, flags)` | Read or write sprite flags |

### Input and audio

| Function | Description |
| :------- | :---------- |
| `button_down(id)` | Test whether a button is held |
| `button_pressed(id)` | Test whether a button was pressed this frame |
| `play_sfx(id)` | Play an SFX slot |
| `play_music(id)` | Play a music pattern |
| `stop_music()` | Stop music playback |

### System

| Function | Description |
| :------- | :---------- |
| `real_time()` | Return host hour, minute, and second |
| `frame_count()` | Return frames since the cart loaded |
| `time()` | Return elapsed game time in seconds |

### Gameplay standard library

The preloaded pure-Lua helpers include:

- `lerp`, `clamp`, and easing curves
- `aabb_overlap`, `tile_solid`, and `box_touches_solid`
- `new_tween` and `tween_update`
- `new_anim`, `anim_update`, and `anim_sprite`
- `Particles.spawn`, `Particles.update`, `Particles.draw`, and related helpers

See [`games/carts/stdlib_demo.cav`](games/carts/stdlib_demo.cav) for a working demonstration.

---

## 🖌️ Caiven Studio

Press a function key to switch editors:

| Key | Tab |
| :-- | :-- |
| `F1` | Code |
| `F2` | Sprite |
| `F3` | Map |
| `F4` | SFX |
| `F5` | Music |
| `F6` | Palette |
| `F7` | Cart metadata |
| `F8` | Local and Port browser |
| `F9` | Searchable help |

`Ctrl+S` saves from any tab. `Ctrl+P` or `Ctrl+Shift+P` opens the command palette. Run, Pause, Reset, and the FPS counter remain visible while editing.

### Code editor

- Lua syntax highlighting
- Autocomplete for built-ins, standard-library members, locals, and functions
- Hover documentation
- Signature help
- Find and find-next
- Undo and redo
- Error navigation to the offending line
- Clickable breakpoint gutter

### Sprite editor

- 8×8 canvas with sheet picker
- Pencil, fill, line, and rectangle tools
- Eyedropper and palette selection
- Sprite flags
- Copy, paste, undo, redo, flip, rotate, wrap shift, and clear

### Map editor

- Scrollable 64×64 tile canvas
- Pencil, fill, and rectangle tools
- Tile eyedropper
- Multiple zoom levels
- Undo and redo
- Optional sprite-flag tinting

### Audio editors

- 16-step pitch and volume SFX tracker
- Waveform and effect controls
- 8-pattern, 2-channel music editor
- Preview playback and playheads

### Debugger

- Persistent line breakpoints in a `.cavdbg` sidecar
- Run, pause, and one-frame stepping
- Top-level script globals inspector
- Live RAM hex view

---

## 📟 System Specifications

| Component | Specification |
| :-------- | :------------ |
| Script engine | Lua 5.4 through `mlua` |
| Resolution | 128×128 |
| RAM | 64 KiB |
| Palette | 16 colors |
| Sprites | 256 sprites, 8×8 pixels each |
| Map | 64×64 tiles |

### Memory map

| Range | Region |
| :---- | :----- |
| `0x0000–0x3FFF` | Reserved |
| `0x4000–0x7FFF` | Sprite sheet |
| `0x8000–0x8FFF` | Tilemap |
| `0x9000–0x90FF` | Sprite flags |
| `0x9100–0x91FF` | Palette |
| `0x9200–0x95FF` | SFX bank |
| `0x9600–0x96FF` | Music bank |
| `0x9700–0xFFFF` | Reserved |

---

## 🌐 Caiven Port

Caiven Port is a self-hostable Rocket, SQLite, and Svelte cart gallery.

It provides:

- user accounts and API tokens
- cart uploads and versioning
- screenshots
- ratings and comments
- tag, author, popularity, and text discovery
- browser-based WASM play
- keyboard, gamepad, and touch controls

Run it locally:

```bash
cd crates/caiven-port
cargo run --release
```

Or use Docker Compose:

```bash
cd crates/caiven-port
docker compose up
```

The default Port address is `http://localhost:8080`.

---

## 📂 Project Structure

| Path | Description |
| :--- | :---------- |
| `crates/caiven-core` | Shared types and memory-map constants |
| `crates/caiven-cart` | Cartridge format and load/write helpers |
| `crates/caiven-vm` | Lua execution, rendering, audio, input, and debugger hooks |
| `crates/caiven-studio` | Desktop creation environment |
| `crates/caiven-machine` | Standalone cart player |
| `crates/caiven-port` | Cart-sharing server and web UI |
| `crates/caiven-web` | WASM browser player |
| `crates/migration` | Database migrations |
| [`games/carts`](games/carts) | Ready-to-run demo carts |

---

## ⌨️ Game Controls

| Button | Default keys |
| :----- | :----------- |
| Up | Arrow Up, `W` |
| Down | Arrow Down, `S` |
| Left | Arrow Left, `A` |
| Right | Arrow Right, `D` |
| A | `J` |
| B | `K` |

Override controls by creating `controls.toml` next to the executable:

```toml
[controls]
up    = ["ArrowUp", "KeyW"]
down  = ["ArrowDown", "KeyS"]
left  = ["ArrowLeft", "KeyA"]
right = ["ArrowRight", "KeyD"]
a     = ["KeyJ"]
b     = ["KeyK"]
```

---

## 📜 License and Creator Policy

Caiven is licensed under the [Mozilla Public License 2.0](LICENSE).

Games and carts made with Caiven remain the creator's property and may be sold without royalties, revenue sharing, a commercial-use license, or mandatory source publication. See [Creator rights](CREATOR_RIGHTS.md).

Unofficial forks must not be presented as official Caiven releases. See the [trademark policy](TRADEMARKS.md).

---

<p align="center">Made with ❤️ and 🦀 by Andrej Markuš</p>
