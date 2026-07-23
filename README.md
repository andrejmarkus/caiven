# 🎮 Caiven

![Rust](https://img.shields.io/badge/rust-%23E32F26.svg?style=for-the-badge&logo=rust&logoColor=white)
![License](https://img.shields.io/badge/license-MIT-green.svg?style=for-the-badge)

**Caiven** is a retro-inspired fantasy console: a virtual machine and development environment written in Rust. Real embedded Lua 5.4 (via `mlua`) for game code, a full in-engine editor suite (Caiven Studio), and an optional cart-sharing port.

> [!TIP]
> Write real Lua — every tutorial and stdlib function (`math`, `string`, `table`, `pcall`, ...) just works. No custom bytecode language, no arity caps, no silent gaps.

---

## ✨ Features

- 🌙 **Real Lua 5.4** — embedded via `mlua` (vendored, no system Lua required); `_init()` runs once, `_update()` runs every frame, optional `_draw()` runs right after it
- 🎨 **Palette-based Graphics** — 128×128 resolution, 16-color swappable palette; sprites, 64×64 tilemap, shape primitives, camera
- 📦 **Descriptive Builtin API** — `sprite`, `draw_rect`, `button_down`, `set_palette_color`, etc. — no cryptic abbreviations, and `print()` stays wired to your terminal for real Lua debugging (screen text is `draw_text`)
- 🔊 **Audio Engine** — real-time sound synthesis, SFX and music banks, playback via CPAL
- 🖌️ **Caiven Studio** — egui-based editor suite: code, sprite, map, palette, SFX, music, cart-meta editors, local & port cart browser, all in one window
- 🔍 **Debugger** — line breakpoints (click the code editor gutter), pause/step-by-frame, script-globals inspector, live RAM view, `.cavdbg` sidecar persistence
- 🌐 **Caiven Port** — self-hostable cart sharing server with a Svelte web UI: accounts, cart versioning, ratings & comments, tag/author discovery

---

## 🚀 Getting Started

### Prerequisites

You'll need the [Rust toolchain](https://rustup.rs/) installed on your system.

### Installation

```bash
git clone https://github.com/your-username/caiven.git
cd caiven
cargo build --release
```

### Running

```bash
cargo run -p caiven-studio -- [command]
```

| Command | Description |
| :------ | :---------- |
| _(no command)_ | Launch Caiven Studio (editor suite), opens on the cart browser |
| `edit [file]` | Launch Caiven Studio, optionally opening a `.cav` file |
| `inspect <file.cav>` | Print cart section table |
| `publish <file.cav>` | Upload cart to a caiven-port instance |

To just run a cart (no editor), use `caiven-machine`:

```bash
cargo run -p caiven-machine -- game.cav
```

**Publish flags:**

| Flag | Default | Description |
| :--- | :------ | :---------- |
| `--url` | `http://localhost:8080` | Port base URL (env: `CAIVEN_PORT_URL`) |
| `--api-key` | _(empty, required)_ | Per-user port API token (env: `CAIVEN_PORT_API_KEY`) — mint one via the port web UI Profile page or by logging into Caiven Studio's PORT tab |
| `--title` | cart header | Override cart title |
| `--author` | cart header | Override author |
| `--description` | _(empty)_ | Short description |
| `--tags` | _(empty)_ | Comma-separated tags |
| `--frames` | `30` | Frames to run before screenshot |
| `--no-screenshot` | — | Skip screenshot capture |

---

## 🐣 Tutorial: Your First Game

Every cart is a single `.cav` file: a binary bundle (magic header + CRC32-checked
sections) holding your Lua code alongside sprites, map, palette, SFX and music —
authored entirely in Caiven Studio, no external text files involved.

1. **Launch Caiven Studio** and click **NEW CART** on the browser tab (`F8`):

```bash
cargo run -p caiven-studio -- edit
```

This opens a blank cart with a `_init`/`_update` stub in the `F1` code tab.

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

5. **Ship it** — `Ctrl+S` writes code + sprites + map + audio into the `.cav` (a new cart defaults to `untitled.cav` in the browser's folder — rename the file on disk, and set title/author on the `F7` meta tab), then run it standalone with `caiven-machine game.cav` (no editor needed), or `publish game.cav` to share it on a port.

### Cart lifecycle functions

| Function | Purpose |
| :------- | :------ |
| `_init()` | Runs once when the cart loads |
| `_update()` | Runs once per frame (called for you — no `wait()`/vsync call needed) |
| `_draw()` | Optional — runs once per frame, right after `_update()`. Split game logic from rendering if you like; carts with only `_update()` work exactly as before |

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

### System

| Function | Description |
| :------- | :---------- |
| `real_time()` | Returns `(hour, minute, second)` from the host's real-time clock |
| `frame_count()` | Number of frames run since the cart loaded |
| `time()` | Seconds since the cart loaded, assuming 60 frames per second |

---

## 🖌️ Caiven Studio

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
| `F8` | 📂 Browser (local + port) |
| `F9` | 📖 Help (searchable builtin/stdlib reference) |

`Ctrl+S` saves the cart from any tab. `Ctrl+P` (or `Ctrl+Shift+P`) opens a command palette — fuzzy search over every menu/toolbar action, tab switch, "new from template," and "insert builtin" call. The Run/Pause/Reset toolbar and FPS counter are always visible; the game view renders as an integer-scaled, nearest-neighbor 128×128 texture. Opening a cart (or launching `caiven-studio edit game.cav`) loads it **paused** — hit ▶ Run to start it.

Launching with no cart open shows a **welcome screen**: NEW CART / OPEN, a recent-carts list, and starter templates (top-down mover, tap-to-score, tile world) that compile and run immediately — a readable alternative to poking at a binary `.cav`.

`File > Export` (or the command palette) captures the live game view: **Screenshot (PNG)** grabs the current frame, **Record GIF (3s)** samples the next three seconds of gameplay at 30fps. Both prompt for a save location.

### 📝 Code Editor

Syntax highlighting for Lua keywords, this project's builtin API, and stdlib namespaces (`math`, `string`, `table`, ...). Click a line's gutter to toggle a breakpoint. `Ctrl+Z`/`Ctrl+Y` undo/redo, `Ctrl+F`/`Ctrl+G` find/find-next. A Lua error jumps the cursor to the offending line and shows the message in the status bar.

**Intellisense**, backed by a structured registry of every builtin/stdlib function's name, parameters, return type, and description:

- **Autocomplete** — pops up while typing an identifier, or after `namespace.` (e.g. `math.`); candidates include this buffer's own `local`/`function` declarations, not just the builtin API. `Ctrl+Space` opens it manually (e.g. with nothing typed yet, to browse everything). `↑`/`↓` to navigate, `Enter`/`Tab`/click to accept, `Esc` to dismiss without losing editor focus.
- **Hover docs** — hover any builtin, stdlib member, or local/function name for its signature and description.
- **Signature help** — while typing inside a call's `(...)`, an overlay above the cursor shows the full parameter list with the active parameter highlighted.

None of the three fire inside string literals or comments.

### 🖼️ Sprite Editor

8×8 canvas at 32× zoom: pencil/fill/line/rect tools (drag preview), right-click eyedropper, palette row, per-sprite flag checkboxes, 16×16 sheet picker, per-sprite undo/redo (`Ctrl+Z`/`Y`, `Ctrl+C`/`V` copy/paste). An ops row adds flip horizontal/vertical, rotate 90°, wrap-around shift (↑↓←→), and clear — all undoable.

### 🗺️ Map Editor

Scrollable 64×64 tile canvas, pencil/fill/rect tools, right-click tile eyedropper, 1×/2×/4× zoom, full-map undo/redo. A FLAGS toggle tints each tile by its sprite's flag byte, so solidity/metadata stays visible while painting.

### 🎵 SFX / 🎶 Music Editors

16-step pitch/volume tracker per SFX slot (drag to draw notes, wave/fx toggles, playhead); 8-pattern music editor (16 rows × 2 channels referencing SFX slots, loop toggle, playhead). `Space` previews.

### 🔍 Debugger

Bottom panel below the game view. Breakpoints toggle from the code editor gutter and persist in a `.cavdbg` TOML sidecar next to the cart:

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

## 🌐 Caiven Port (Cart Sharing Server)

Self-hostable cart gallery server: Rocket + SQLite backend, Svelte web UI.
Accounts, cart versioning, ratings & comments, and tag/author/sort discovery.

```bash
cd crates/caiven-port
cargo run --release
# or
docker compose up
```

| Flag | Default | Description |
| :--- | :------ | :---------- |
| `--address` | `0.0.0.0` | Listen address |
| `--port` | `8080` | Listen port |
| `--data-dir` | `data` | Directory for `port.db` + uploaded carts/screenshots (auto-created) |
| `--web-dir` | `crates/caiven-port/web/dist` | Built SPA directory (`npm run build` output in `crates/caiven-port/web/`) |

Open the base URL in a browser to register an account, browse/search/filter
carts by tag, author or sort (new/popular/top), upload new carts or versions,
rate and comment, and view author profile pages. The web UI uses a session
cookie; the same account can also mint per-user API tokens (Profile page) for
`caiven-studio publish` or direct API calls — sent as an `X-Api-Key` header.

| Method | Path | Description |
| :----- | :--- | :---------- |
| `POST` | `/api/v2/auth/register` / `/login` / `/logout` | Account auth (session cookie) |
| `GET` | `/api/v2/auth/me` | Current user |
| `GET`/`POST`/`DELETE` | `/api/v2/auth/tokens` | Manage per-user API tokens |
| `GET` | `/api/v2/carts` | List/search carts (`page`, `per_page`, `q`, `tag`, `author`, `sort`) |
| `POST` | `/api/v2/carts` | Upload new cart (multipart: `cart` + JSON `meta`) |
| `GET`/`DELETE` | `/api/v2/carts/:id` | Cart detail / delete (owner or admin) |
| `POST` | `/api/v2/carts/:id/versions` | Upload a new version of an owned cart |
| `GET` | `/api/v2/carts/:id/cart` \| `/screenshot` | Download cart/screenshot (`?version=n`, defaults to latest) |
| `PUT`/`DELETE` | `/api/v2/carts/:id/rating` | Rate a cart (1-5) |
| `GET`/`POST`/`DELETE` | `/api/v2/carts/:id/comments[/:cid]` | Comments |
| `GET` | `/api/v2/tags` \| `/api/v2/users/:username` | Discovery |

Legacy `/api/carts*` routes (v1 shape, single cart file per cart) remain for
backward compatibility — `caiven-studio publish` still targets them internally.

---

## 📂 Project Structure

Cargo workspace with seven crates:

| Crate | Description |
| :---- | :---------- |
| `crates/caiven-core` | Shared types and memory map — `Color`, `Vec2`, RAM layout constants |
| `crates/caiven-cart` | Cart format: binary header, section layout, load/write helpers |
| `crates/caiven-vm` | VM core: embedded Lua (`mlua`) execution, builtin API, renderer, audio, input, debugger hooks |
| `crates/caiven-studio` | Main binary: Caiven Studio editor suite (edit mode only), cart browser, CLI |
| `crates/caiven-machine` | Standalone cart runner (run mode: `.cav` only, no editor/port) |
| `crates/caiven-port` | Cart sharing server |
| `crates/migration` | `sea-orm` database migrations for caiven-port |

`games/carts/` — example carts, ready to run: `cargo run -p caiven-machine -- games/carts/catch.cav`, or open in Caiven Studio via `caiven-studio edit`.

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
