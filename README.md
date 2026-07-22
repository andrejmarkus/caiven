# 🎮 Fantasy Console

![Rust](https://img.shields.io/badge/rust-%23E32F26.svg?style=for-the-badge&logo=rust&logoColor=white)
![License](https://img.shields.io/badge/license-MIT-green.svg?style=for-the-badge)

**Fantasy Console** is a retro-inspired virtual machine and development environment written in Rust. Real embedded Lua 5.4 (via `mlua`) for game code, a full in-engine editor suite (FC Studio), and an optional cart-sharing hub.

> [!TIP]
> Write real Lua — every tutorial and stdlib function (`math`, `string`, `table`, `pcall`, ...) just works. No custom bytecode language, no arity caps, no silent gaps.

---

## ✨ Features

- 🌙 **Real Lua 5.4** — embedded via `mlua` (vendored, no system Lua required); `_init()` runs once, `_update()` runs every frame
- 🎨 **Palette-based Graphics** — 128×128 resolution, 16-color swappable palette; sprites, 64×64 tilemap, shape primitives, camera
- 📦 **Descriptive Builtin API** — `sprite`, `draw_rect`, `button_down`, `set_palette_color`, etc. — no PICO-8-style abbreviations, and `print()` stays wired to your terminal for real Lua debugging (screen text is `draw_text`)
- 🔊 **Audio Engine** — real-time sound synthesis, SFX and music banks, playback via CPAL
- 🖌️ **FC Studio** — egui-based editor suite: code, sprite, map, palette, SFX, music, cart-meta editors, local & hub cart browser, all in one window
- 🔍 **Debugger** — line breakpoints (click the code editor gutter), pause/step-by-frame, script-globals inspector, live RAM view, `.fcdbg` sidecar persistence
- 🌐 **Cart Sharing Hub** — self-hostable server with a Svelte web UI: accounts, cart versioning, ratings & comments, tag/author discovery

---

## 🚀 Getting Started

### Prerequisites

You'll need the [Rust toolchain](https://rustup.rs/) installed on your system.

### Installation

```bash
git clone https://github.com/your-username/fantasy-console.git
cd fantasy-console
cargo build --release
```

### Running

```bash
cargo run -p fc-engine -- [command]
```

| Command | Description |
| :------ | :---------- |
| _(no command)_ | Launch FC Studio (editor suite), opens on the cart browser |
| `edit [file]` | Launch FC Studio, optionally opening a `.rom` or `.lua` file |
| `run <file>` | Run a `.lua` source file or a `.rom` file |
| `build <source.lua> <output.rom>` | Package a `.lua` source and its asset blocks into a ROM |
| `inspect <file.rom>` | Print ROM section table |
| `publish <file.rom>` | Upload cart to an fc-hub instance |

Pass `--debug` (or `-d`) to `run` for the in-game debug overlay (fault/RAM/breakpoint status — FC Studio is the full debugging experience).

**Publish flags:**

| Flag | Default | Description |
| :--- | :------ | :---------- |
| `--url` | `http://localhost:8080` | Hub base URL (env: `FC_HUB_URL`) |
| `--api-key` | _(empty, required)_ | Per-user hub API token (env: `FC_HUB_API_KEY`) — mint one via the hub web UI Profile page or by logging into FC Studio's HUB tab |
| `--title` | ROM header | Override cart title |
| `--author` | ROM header | Override author |
| `--description` | _(empty)_ | Short description |
| `--tags` | _(empty)_ | Comma-separated tags |
| `--frames` | `30` | Frames to run before screenshot |
| `--no-screenshot` | — | Skip screenshot capture |

---

## 🐣 Tutorial: Your First Game

1. **Create `game.lua`:**

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

2. **Run it:**

```bash
cargo run -p fc-engine -- run game.lua
```

3. **Draw your player** — open FC Studio (`fc-engine edit game.lua`), press `F2` for the sprite tab and paint sprite 0.

4. **Iterate** — edit code in the `F1` code tab (or your external editor + `run` again); click the gutter to set a line breakpoint, `F1`'s Run/Pause/Reset toolbar drives execution. Lua errors show with a line number and message straight in the status bar.

5. **Ship it** — `build game.lua game.rom`, then `Ctrl+S` in FC Studio stores sprites/map/audio into the cart, and `publish game.rom` shares it on a hub.

### Cart source format

A `.lua` file is just a Lua chunk with two lifecycle functions:

| Function | Purpose |
| :------- | :------ |
| `_init()` | Runs once when the cart loads |
| `_update()` | Runs once per frame (called for you — no `wait()`/vsync call needed) |

Sprite/map/palette/SFX/music data lives in RAM, edited via FC Studio, and round-trips through the same file as hex asset blocks (`__gfx__`, `__map__`, etc.) appended after your code — you never hand-edit these, `Ctrl+S` manages them.

---

## 📝 Built-in API Reference

Math (`sin`/`cos`/`abs`/`floor`/`sqrt`/`max`/`min`/`random`), strings (`..`, `sub`, `tostring`, `string.*`), and tables are all just Lua's own stdlib — no bindings needed for those.

### Graphics

| Function | Description |
| :------- | :---------- |
| `clear_screen()` | Clear screen and UI layer |
| `fill_screen(color)` | Fill screen with a palette color |
| `set_pixel(x, y, color)` | Set pixel (signed coords) |
| `draw_line(x0, y0, x1, y1, color)` | Line (camera-aware) |
| `draw_rect(x, y, w, h, color)` / `fill_rect(x, y, w, h, color)` | Rectangle outline / filled |
| `draw_circle(cx, cy, r, color)` / `fill_circle(cx, cy, r, color)` | Circle outline / filled |
| `set_palette_color(index, r, g, b)` | Set palette entry |
| `set_camera(x, y)` | Set camera offset |
| `draw_text(text, x, y, color)` | Draw a string (does **not** shadow Lua's real `print()` — that still goes to your terminal) |
| `draw_number(value, x, y, color)` | Draw an integer |

### Sprites & Map

| Function | Description |
| :------- | :---------- |
| `sprite(id, x, y)` | Draw 8×8 sprite (camera-aware) |
| `draw_map(cell_x, cell_y, sx, sy, w, h)` | Draw a block of the tilemap |
| `get_tile(x, y)` / `set_tile(x, y, tile)` | Read / write a map cell |
| `get_sprite_flags(id)` / `set_sprite_flags(id, flags)` | Read / write per-sprite flag byte |

### Input

| Function | Description |
| :------- | :---------- |
| `button_down(id)` | Button held (0=Up 1=Down 2=Left 3=Right 4=A 5=B) |
| `button_pressed(id)` | Button pressed this frame |

### Audio

| Function | Description |
| :------- | :---------- |
| `play_sfx(id)` | Play a sound effect from the SFX bank |
| `play_music(id)` | Play a music track |
| `stop_music()` | Stop music |

---

## 🖌️ FC Studio

Press function keys at any time to switch tabs:

| Key | Tab |
| :-- | :--- |
| `F1` | 📝 Code |
| `F2` | 🖼️ Sprite |
| `F3` | 🗺️ Map |
| `F4` | 🎵 SFX |
| `F5` | 🎶 Music |
| `F6` | 🎨 Palette |
| `F7` | 📋 Cart meta |
| `F8` | 📂 Browser (local + hub) |

`Ctrl+S` saves the cart from any tab. The Run/Pause/Reset toolbar and FPS counter are always visible; the game view renders as an integer-scaled, nearest-neighbor 128×128 texture.

### 📝 Code Editor

Syntax highlighting for Lua keywords, this project's builtin API, and stdlib namespaces (`math`, `string`, `table`, ...). Click a line's gutter to toggle a breakpoint. `Ctrl+Z`/`Ctrl+Y` undo/redo, `Ctrl+F`/`Ctrl+G` find/find-next. A Lua error jumps the cursor to the offending line and shows the message in the status bar.

### 🖼️ Sprite Editor

8×8 canvas at 32× zoom: pencil/fill/line/rect tools (drag preview), right-click eyedropper, palette row, per-sprite flag checkboxes, 16×16 sheet picker, per-sprite undo/redo (`Ctrl+Z`/`Y`, `Ctrl+C`/`V` copy/paste).

### 🗺️ Map Editor

Scrollable 64×64 tile canvas, pencil/fill/rect tools, right-click tile eyedropper, 1×/2×/4× zoom, full-map undo/redo.

### 🎵 SFX / 🎶 Music Editors

16-step pitch/volume tracker per SFX slot (drag to draw notes, wave/fx toggles, playhead); 8-pattern music editor (16 rows × 2 channels referencing SFX slots, loop toggle, playhead). `Space` previews.

### 🔍 Debugger

Bottom panel below the game view. Breakpoints toggle from the code editor gutter and persist in a `.fcdbg` TOML sidecar next to the cart:

```toml
breakpoints = [9, 42]
```

Controls: Run/Pause/Step-one-frame; a script-globals inspector shows the script's current top-level variables (filters out builtins and Lua stdlib names); the RAM hex view (sprite/map/palette/SFX/music regions) stays available for low-level inspection. There's no instruction-level single-step or timeline-scrubber rewind — mlua's interpreter state isn't cheaply snapshotable, so stepping is frame-granular.

---

## 📟 System Specifications

| Component | Specification |
| :-------- | :------------ |
| **Script engine** | Lua 5.4 via `mlua` (vendored) |
| **Resolution** | 128×128 (upscaled 4×) |
| **RAM** | 64 KiB (asset/RAM regions below; script state lives in the Lua VM, not guest RAM) |
| **Palette** | 16 colors |
| **Sprites** | 256 × 8×8 pixels |
| **Map** | 64×64 tiles |

### Memory Map

| Range | Region |
| :---- | :----- |
| `0x0000–0x3FFF` | Unused / reserved |
| `0x4000–0x7FFF` | Sprite sheet — 256 sprites × 64 bytes (1 byte/pixel) |
| `0x8000–0x8FFF` | Tilemap 64×64 (1 byte/cell) |
| `0x9000–0x90FF` | Sprite flags (1 byte/sprite) |
| `0x9100–0x91FF` | Palette (16 × 3 bytes RGB, rest padding) |
| `0x9200–0x95FF` | SFX bank (16 × 64 bytes) |
| `0x9600–0x96FF` | Music bank (8 × 32 bytes) |
| `0x9700–0xFFFF` | Reserved |

---

## 🌐 fc-hub (Cart Sharing Server)

Self-hostable cart gallery server: Rocket + SQLite backend, Svelte web UI.
Accounts, cart versioning, ratings & comments, and tag/author/sort discovery.

```bash
cd crates/fc-hub
cargo run --release
# or
docker compose up
```

| Flag | Default | Description |
| :--- | :------ | :---------- |
| `--address` | `0.0.0.0` | Listen address |
| `--port` | `8080` | Listen port |
| `--data-dir` | `data` | Directory for `hub.db` + uploaded ROMs/screenshots (auto-created) |
| `--web-dir` | `crates/fc-hub/web/dist` | Built SPA directory (`npm run build` output in `crates/fc-hub/web/`) |

Open the base URL in a browser to register an account, browse/search/filter
carts by tag, author or sort (new/popular/top), upload new carts or versions,
rate and comment, and view author profile pages. The web UI uses a session
cookie; the same account can also mint per-user API tokens (Profile page) for
`fc-engine publish` or direct API calls — sent as an `X-Api-Key` header.

| Method | Path | Description |
| :----- | :--- | :---------- |
| `POST` | `/api/v2/auth/register` / `/login` / `/logout` | Account auth (session cookie) |
| `GET` | `/api/v2/auth/me` | Current user |
| `GET`/`POST`/`DELETE` | `/api/v2/auth/tokens` | Manage per-user API tokens |
| `GET` | `/api/v2/carts` | List/search carts (`page`, `per_page`, `q`, `tag`, `author`, `sort`) |
| `POST` | `/api/v2/carts` | Upload new cart (multipart: `rom` + JSON `meta`) |
| `GET`/`DELETE` | `/api/v2/carts/:id` | Cart detail / delete (owner or admin) |
| `POST` | `/api/v2/carts/:id/versions` | Upload a new version of an owned cart |
| `GET` | `/api/v2/carts/:id/rom` \| `/screenshot` | Download ROM/screenshot (`?version=n`, defaults to latest) |
| `PUT`/`DELETE` | `/api/v2/carts/:id/rating` | Rate a cart (1-5) |
| `GET`/`POST`/`DELETE` | `/api/v2/carts/:id/comments[/:cid]` | Comments |
| `GET` | `/api/v2/tags` \| `/api/v2/users/:username` | Discovery |

Legacy `/api/carts*` routes (v1 shape, single ROM per cart) remain for
backward compatibility — `fc-engine publish` still targets them internally.

---

## 📂 Project Structure

Cargo workspace with seven crates:

| Crate | Description |
| :---- | :---------- |
| `crates/fc-core` | Shared types and memory map — `Color`, `Vec2`, RAM layout constants |
| `crates/fc-rom` | ROM format: header, section layout, `.lua` source block splitting, load/write helpers |
| `crates/fc-vm` | VM core: embedded Lua (`mlua`) execution, builtin API, renderer, audio, input, debugger hooks |
| `crates/fc-engine` | Main binary: FC Studio editor suite, cart browser, CLI, app loop |
| `crates/fc-host` | Minimal standalone ROM runner (no editor/hub) |
| `crates/fc-hub` | Cart sharing server |
| `crates/migration` | `sea-orm` database migrations for fc-hub |

`games/lua/` — example `.lua` cart sources. `games/roms/` — the same carts prebuilt to `.rom` (run directly: `fc-engine run games/roms/catch.rom`, or open in FC Studio via `fc-engine edit`).

---

## ⌨️ Key Bindings (Game)

| Button | Keys |
| :----- | :--- |
| Up | `ArrowUp`, `W` |
| Down | `ArrowDown`, `S` |
| Left | `ArrowLeft`, `A` |
| Right | `ArrowRight`, `D` |
| A | `J` |
| B | `K` |

Override by creating `controls.toml` next to the binary:

```toml
[controls]
up    = ["ArrowUp", "KeyW"]
down  = ["ArrowDown", "KeyS"]
left  = ["ArrowLeft", "KeyA"]
right = ["ArrowRight", "KeyD"]
a     = ["KeyJ"]
b     = ["KeyK"]
```

Any `winit` physical key name is valid (e.g. `KeyZ`, `Digit1`, `Space`, `Enter`). Missing file falls back to defaults.

---

## 📜 License

This project is licensed under the MIT License.

---

<p align="center">Made with ❤️ and 🦀 by Andrej Markuš</p>
