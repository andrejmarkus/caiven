# 🎮 Caiven

![Rust](https://img.shields.io/badge/rust-%23E32F26.svg?style=for-the-badge&logo=rust&logoColor=white)
![License](https://img.shields.io/badge/license-MIT-green.svg?style=for-the-badge)
[![CI and Release](https://github.com/andrejmarkus/caiven/actions/workflows/rust.yml/badge.svg)](https://github.com/andrejmarkus/caiven/actions/workflows/rust.yml)
[![Latest Release](https://img.shields.io/github/v/release/andrejmarkus/caiven?style=for-the-badge)](https://github.com/andrejmarkus/caiven/releases/latest)

**Caiven** is a retro-inspired fantasy console: a virtual machine and development environment written in Rust. Real embedded Lua 5.4 (via `mlua`) for game code, a full in-engine editor suite (Caiven Studio), and an optional cart-sharing port.

> [!TIP]
> Write real Lua — every tutorial and stdlib function (`math`, `string`, `table`, `pcall`, ...) just works. No custom bytecode language, no arity caps, no silent gaps.

---

## ✨ Features

- 🌙 **Real Lua 5.4** — embedded via `mlua` (vendored, no system Lua required); `_init()` runs once, `_update()` runs every frame, optional `_draw()` runs right after it
- 🎨 **Palette-based Graphics** — 128×128 resolution, 16-color swappable palette; sprites, 64×64 tilemap, shape primitives, camera
- 📦 **Descriptive Builtin API** — `sprite`, `draw_rect`, `button_down`, `set_palette_color`, etc. — no cryptic abbreviations; `print()` goes to Machine's terminal or Studio's Output drawer (screen text is `draw_text`)
- 🔊 **Audio Engine** — real-time sound synthesis, SFX and music banks, playback via CPAL
- 🧰 **Gameplay Stdlib** — tweens, easing curves, AABB/tile collision, a particle system, and sprite-frame animation, all pure Lua and preloaded into every cart
- 🖌️ **Caiven Studio** — Tauri 2 + Svelte 5 editor: live console, code and asset workspaces, diagnostics drawer, command palette, onboarding, and publishing flow
- 🔍 **Debugger** — line breakpoints (click the code editor gutter), pause/step-by-frame, script-globals inspector, live RAM view, `.cavdbg` sidecar persistence
- 🌐 **Caiven Port** — self-hostable cart sharing server with a Svelte web UI: accounts, cart versioning, ratings & comments, tag/author discovery

---

## 🚀 Getting Started

### ⚡ Quick Start (no Rust, no Node, no build step)

There are two separate downloads — grab whichever matches what you want to do:

- **Caiven Studio** — the editor. Use this to *make* a game (code, sprites, sound, map).
- **Caiven Machine** — the standalone player. Use this to just *run* a `.cav` cart someone shared with you, no editor.

All links point at the [latest GitHub release](https://github.com/andrejmarkus/caiven/releases/latest).

#### 🖌️ Caiven Studio (make a game)

Available for Windows (NSIS installer or MSI), macOS (Apple Silicon or Intel DMG),
and Linux (AppImage or .deb) — grab the one matching your OS from the
[latest GitHub release](https://github.com/andrejmarkus/caiven/releases/latest).

Install like any normal app, launch **Caiven Studio**, click **New cart**, and jump to the [tutorial below](#-tutorial-your-first-game).

#### 🕹️ Caiven Machine (just run a cart)

Available for Linux, Windows, macOS Apple Silicon, and macOS Intel (archives) —
grab the one matching your OS from the
[latest GitHub release](https://github.com/andrejmarkus/caiven/releases/latest).

Unpack the archive, then run the `caiven-machine` binary against a cart or project dir:

```bash
./caiven-machine my-game/    # project dir, hot-reloads with Ctrl+R
./caiven-machine game.cav    # distribution cartridge
```

That's it — the sections below (source build, CLI, Cargo workspace) are for
contributors working on Caiven itself, not for making or playing games with it.

### 🛠️ Building from Source (contributors)

#### Prerequisites

- [Rust stable](https://rustup.rs/)
- [Node.js 22](https://nodejs.org/) with npm
- [Tauri system dependencies](https://v2.tauri.app/start/prerequisites/) for your OS

#### Installation

```bash
git clone https://github.com/andrejmarkus/caiven.git
cd caiven
npm --prefix crates/caiven-studio-ui ci
npm --prefix crates/caiven-studio-ui run build
cargo build --release --workspace
```

#### Running

Launch Studio in development mode:

```bash
cd crates/caiven-studio
npm --prefix ../caiven-studio-ui exec tauri dev
```

Studio CLI commands run from repository root:

```bash
cargo run -p caiven-studio -- [command]
```

| Command | Description |
| :------ | :---------- |
| _(no command)_ | Launch Caiven Studio on its start screen |
| `edit [file]` | Launch Caiven Studio, optionally opening a project dir or `.cav` file |
| `inspect <path>` | Print cart section table (project dir or `.cav`) |
| `build <project> -o <out.cav>` | Build a project dir into a distribution `.cav` cartridge |
| `unpack <file.cav> -o <out>` | Unpack a binary `.cav` into an editable project dir |
| `publish <cart>` | Upload a cart (`.cav` or project dir) to a caiven-port instance |

To just run a cart (no editor), use `caiven-machine`:

```bash
cargo run -p caiven-machine -- my-game/    # project dir, hot-reloads with Ctrl+R
cargo run -p caiven-machine -- game.cav    # distribution cartridge
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
| `draw_text(text, x, y, color)` | Draw a string (does **not** shadow Lua's real `print()` — Machine writes it to terminal; Studio writes it to Output) |
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

### Gameplay stdlib

Pure Lua, loaded into every cart's globals automatically (no `require`) — read `crates/caiven-vm/src/vm/prelude.lua` for the source. See it all in action in `games/carts/stdlib_demo.cav` (`cargo run -p caiven-machine -- games/carts/stdlib_demo.cav`): a tiny platformer with tile collision, a coin pickup that bursts particles, a walk-cycle sprite animation, and four side-by-side tweened dots comparing each easing curve.

| Function | Description |
| :------- | :---------- |
| `lerp(a, b, t)` / `clamp(v, lo, hi)` | Linear interpolate / clamp to range |
| `ease_linear/in_quad/out_quad/in_out_quad(t)` | Easing curves, `t` in `0..1` |
| `aabb_overlap(x1, y1, w1, h1, x2, y2, w2, h2)` | Axis-aligned box overlap test |
| `tile_solid(tx, ty)` | Whether the map tile at `(tx, ty)` has sprite flag bit 0 set |
| `box_touches_solid(x, y, w, h)` | Whether a pixel-space box overlaps any solid tile |
| `new_tween(from, to, frames, ease)` / `tween_update(tw)` | Frame-driven value tween; `tw.done` flips true on arrival |
| `new_anim(frames, frame_len)` / `anim_update(anim)` / `anim_sprite(anim)` | Frame-based sprite animation cycling through a sprite-id list |
| `Particles.spawn(x, y, vx, vy, color, life)` / `.update()` / `.draw()` / `.clear()` / `.count()` | Simple velocity + lifetime particle system |

---

## 🖌️ Caiven Studio

Studio uses native Tauri shell with Svelte UI. Rust actor thread owns VM and
audio; webview receives framebuffer snapshots and sends typed project, input,
transport, sprite, and palette commands.

Press function keys to switch workspaces:

| Key | Workspace |
| :-- | :-------- |
| `F1` | Code |
| `F2` | Art → Sprites |
| `F3` | Art → Map |
| `F4` | Sound → Sound effects |
| `F5` | Sound → Music |
| `F6` | Art → Palette |
| `F7` | Cart details |
| `F8` | Library |
| `F9` | API docs |

`Cmd/Ctrl+S` saves, `Cmd/Ctrl+R` runs or pauses, and `Cmd/Ctrl+K`
opens command palette. Console stays visible at 4× integer scale on wide
windows and 3× at minimum supported 1280×800 size. Bottom drawer holds
Problems, Output, and Memory. Focus mode expands framebuffer without moving
VM into JavaScript.

Run native Studio with live Vite reload:

```bash
npm --prefix crates/caiven-studio-ui ci
cd crates/caiven-studio
npm --prefix ../caiven-studio-ui exec tauri dev
```

Build a native installer for current OS:

```bash
cd crates/caiven-studio
npm --prefix ../caiven-studio-ui exec tauri build
```

Bundles land under `target/release/bundle/`. For UI-only work, run
`npm --prefix crates/caiven-studio-ui run dev` from repository root.

Browser preview uses representative data; Tauri launch supplies live VM,
filesystem, input, API-registry, sprite, and palette state.

---

## 📦 Publishing a Release

`.github/workflows/rust.yml` runs CI on `master` and pull requests. A
version tag also builds:

- Caiven Studio: Linux AppImage + Debian package, Windows NSIS + MSI installers,
  and macOS DMGs for Apple Silicon + Intel
- Caiven Machine: Linux, Windows, macOS Apple Silicon, and macOS Intel archives

Before tagging:

1. Set same version in `crates/caiven-studio/tauri.conf.json`,
   `crates/caiven-studio/Cargo.toml`, and
   `crates/caiven-machine/Cargo.toml`.
2. Commit version change.
3. Push matching `v<version>` tag:

```bash
git tag v0.1.0
git push origin master
git push origin v0.1.0
```

Workflow rejects mismatched package versions and tags that do not match Studio
bundle version. Once CI and every platform build succeed, one GitHub Release is
created with generated notes and all installers/archives.
`workflow_dispatch` builds same artifacts for testing without publishing a
release.

macOS builds use ad-hoc signing, so they are not notarized; Windows installers
are unsigned. Public trusted releases need
[macOS signing/notarization](https://v2.tauri.app/distribute/sign/macos/) and
[Windows code signing](https://v2.tauri.app/distribute/sign/windows/).

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

Self-hostable cart gallery server: Rocket + Svelte web UI. Accounts, cart
versioning, ratings & comments, and tag/author/sort discovery. Everything —
including cart files and screenshots — is stored in the database (`BYTEA`),
so a PostgreSQL instance is the only stateful thing to provision or back up.

```bash
cd crates/caiven-port
cargo run --release
# or, for a real PostgreSQL-backed deploy:
docker compose up
```

Without `--database-url`/`DATABASE_URL` set, `cargo run` falls back to an
on-disk SQLite database under `--data-dir` — zero-setup for local dev.
`docker compose up` runs the real deploy path: a `postgres` service plus the
server, wired together via `DATABASE_URL`.

| Flag | Default | Description |
| :--- | :------ | :---------- |
| `--address` | `0.0.0.0` | Listen address |
| `--port` | `8080` | Listen port |
| `--database-url` (env `DATABASE_URL`) | unset | PostgreSQL connection string. When set, carts/screenshots/all data live in Postgres |
| `--data-dir` | `data` | Fallback SQLite database directory, used only when `--database-url` is unset |
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

### 🕹️ Web Play

Every cart on the hub has a **Play** button (gallery card and detail page) that
opens `/play/:id` — a zero-install browser build of the runtime, no download
required. Backed by `crates/caiven-web`, a WASM (`wasm32-unknown-emscripten`)
build of the VM that fetches the cart over the same REST API and renders to a
`<canvas>` at 60fps.

- **Controls:** arrows/WASD to move, `J`/`Z` = A, `K`/`X` = B, standard
  Gamepad API support, and an on-screen touch d-pad + A/B on coarse-pointer
  (mobile) viewports.
- **Audio:** the same square/noise synth used natively, driven by a
  `ScriptProcessorNode` instead of `cpal`.
- **Crash handling:** a Lua runtime error stops the cart and shows the error
  and line number over the last frame, instead of hanging silently.
- Click the canvas or press a key once to start audio — browsers require a
  user gesture before playing sound.

Rebuilding `caiven-web` requires the Emscripten SDK (`emcc`/`emar` on `PATH`).
A throwaway Docker recipe (run from the repo root):

```bash
docker run --rm -v "$(pwd):/work" -w /work emscripten/emsdk:latest \
  bash crates/caiven-web/build-web.sh
```

Then copy `target/wasm32-unknown-emscripten/release/caiven_web.{js,wasm}`
into `crates/caiven-port/web/public/wasm/` and `npm run build` in
`crates/caiven-port/web/` — the built artifact ships with the repo since
there's no CI wasm pipeline yet.

---

## 📂 Project Structure

Cargo workspace with nine crates:

| Crate | Description |
| :---- | :---------- |
| `crates/caiven-core` | Shared types and memory map — `Color`, `Vec2`, RAM layout constants |
| `crates/caiven-cart` | Cart formats: binary `.cav` (header, section layout, load/write) and the project-dir authoring format (`caiven.toml` + `.lua` + `.hex`/`.png`) |
| `crates/caiven-vm` | VM core: embedded Lua (`mlua`) execution, builtin API, renderer, audio, input, debugger hooks |
| `crates/caiven-studio` | Tauri shell, VM actor, Studio IPC, and CLI (`build`/`unpack`/`inspect`/`publish`) |
| `crates/caiven-studio-ui` | Svelte 5 + Vite Studio frontend shared with Port brand tokens |
| `crates/caiven-machine` | Standalone cart runner (run mode: project dir or `.cav`, no editor/port; `Ctrl+R` hot-reloads) |
| `crates/caiven-port` | Cart sharing server |
| `crates/caiven-web` | WASM cart player (`wasm32-unknown-emscripten`) served by caiven-port's `/play/:id` |
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
