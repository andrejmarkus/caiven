# 🎮 Fantasy Console

![Rust](https://img.shields.io/badge/rust-%23E32F26.svg?style=for-the-badge&logo=rust&logoColor=white)
![License](https://img.shields.io/badge/license-MIT-green.svg?style=for-the-badge)

**Fantasy Console** is a retro-inspired virtual machine and development environment written in Rust. Custom 32-bit CPU, built-in assembler, high-level scripting language, full suite of in-engine editors, and an optional cart-sharing hub.

> [!TIP]
> Dive into the world of low-level programming and create your own retro games using a simple yet powerful assembly language — or skip straight to the high-level `.fc` scripting language.

---

## ✨ Features

- 🖥️ **Custom 32-bit Architecture** — 8 general-purpose registers (R0–R7), 64 KiB RAM, 16:16 fixed-point math
- 🎨 **Palette-based Graphics** — 128×128 resolution, 16-color swappable palette; sprites, 64×64 tilemap, shape primitives, camera
- 🛠️ **Integrated Assembler** — tokeniser, parser, code emitter, source maps, label resolution
- 📝 **High-Level Language** (`fc-lang`) — Lua-like scripting that compiles to the native ISA; native tables, closures, strings, full builtin API
- 🔊 **Audio Engine** — real-time sound synthesis, SFX and music banks, playback via CPAL
- 🖌️ **Full Editor Suite** — code, sprite, map, palette, SFX, music, and cart-meta editors; local & hub cart browser
- 🔥 **Hot Reload** — edit `.fc` / `.asm` source in any editor, the running game reloads on save
- 🧭 **Friendly Compile Errors** — source line and caret context, straight in the terminal and the code editor
- 🔍 **Pro Debugger** — breakpoints, source-level stepping, timeline scrubber, register aliases, live memory view, `.fcdbg` sidecar persistence
- 🌐 **Cart Sharing Hub** — self-hostable REST server; publish and download carts with screenshots and gallery

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
| _(no command)_ | Open the cart browser |
| `run <file>` | Run a `.fc` / `.asm` (hot-reload) or `.rom` file |
| `build <source> <output.rom>` | Compile `.fc` or assemble `.asm` and write ROM |
| `inspect <file.rom>` | Print ROM section table |
| `publish <file.rom>` | Upload cart to an fc-hub instance |

**Publish flags:**

| Flag | Default | Description |
| :--- | :------ | :---------- |
| `--url` | `http://localhost:8080` | Hub base URL (env: `FC_HUB_URL`) |
| `--api-key` | `changeme` | Upload auth key (env: `FC_HUB_API_KEY`) |
| `--title` | ROM header | Override cart title |
| `--author` | ROM header | Override author |
| `--description` | _(empty)_ | Short description |
| `--tags` | _(empty)_ | Comma-separated tags |
| `--frames` | `30` | Frames to run before screenshot |
| `--no-screenshot` | — | Skip screenshot capture |

---

## 🐣 Tutorial: Your First Game

1. **Create `game.fc`:**

```lua
const SPEED = 2

let x = 60
let y = 60
let score = 0

init:
  pal(0, 10, 10, 30)     -- dark blue background

loop:
  cls()

  if btn(2) then x -= SPEED end   -- left
  if btn(3) then x += SPEED end   -- right
  if btn(0) then y -= SPEED end   -- up
  if btn(1) then y += SPEED end   -- down

  if btnp(4) then                 -- A pressed this frame
    score += 1
    sfx(0)
  end

  spr(0, x, y)
  print("score", 2, 2, 7)
  num(score, 26, 2, 7)
  wait()
```

2. **Run it with hot reload:**

```bash
cargo run -p fc-engine -- run game.fc
```

3. **Draw your player** — press `F2` for the sprite editor and paint sprite 0. Press `F1` to jump back to the game.

4. **Iterate** — edit `game.fc` in your external editor (or press `F9` for the built-in one); every save reloads the running game instantly. Compile errors print with the offending line and a caret:

```text
line 14: undefined variable 'scroe'
  14 |   scroe += 1
     |   ^^^^^
```

5. **Ship it** — `build game.fc game.rom`, then `Ctrl+S` in-engine stores sprites/map/audio into the cart, and `publish game.rom` shares it on a hub.

### `.fc` program structure

| Block | Purpose |
| :---- | :------ |
| `const NAME = <number>` | Compile-time constant |
| `let name = <expr>` | Global variable |
| `fn name(a, b) ... end` | Function (also `function`); user functions shadow builtins |
| `init:` | Runs once at startup |
| `loop:` | Runs every frame (end with `wait()`) |

Statements: `if/elseif/else`, `while`, `repeat/until`, numeric `for i = a, b [, step]`, generic `for k, v in t`, `break`, `return`, `local`, compound `+=` / `-=`. Values are 32-bit; tables (`{}`) and strings are first-class.

---

## 📝 fc-lang Built-in Reference

All builtins accept arbitrary expressions as arguments.

### Graphics

| Function | Description |
| :------- | :---------- |
| `cls()` | Clear screen |
| `fill(c)` | Fill screen with palette color |
| `pset(x,y,c)` | Set pixel (signed coords, camera-aware) |
| `line(x1,y1,x2,y2,c)` | Line |
| `rect(x1,y1,x2,y2,c)` / `rectfill(...)` | Rectangle outline / filled |
| `circ(x,y,r,c)` / `circfill(x,y,r,c)` | Circle outline / filled |
| `pal(i,r,g,b)` | Set palette entry |
| `camera(x,y)` / `camera()` | Set / reset camera offset |
| `print(str,x,y,c)` (alias `txt`) | Draw text (literals and dynamic strings) |
| `num(val,x,y,c)` | Draw number |

### Sprites & Map

| Function | Description |
| :------- | :---------- |
| `spr(id,x,y[,flip])` | Draw 8×8 sprite; flip bit 0 = horizontal, bit 1 = vertical |
| `map(cel_x,cel_y,sx,sy,w,h)` | Draw tilemap block |
| `mget(x,y)` / `mset(x,y,tile)` | Read / write map cell |
| `fget(id)` / `fset(id,flags)` | Read / write sprite flags |

### Input

| Function | Description |
| :------- | :---------- |
| `btn(id)` | Button held (0=Up 1=Down 2=Left 3=Right 4=A 5=B) |
| `btnp(id)` | Button pressed this frame |

### Math

| Function | Description |
| :------- | :---------- |
| `rnd(max)` | Random integer `[0, max)` |
| `sin(t)` / `cos(t)` | 0–255 = full turn, returns −127..127 |
| `abs(x)` / `flr(x)` / `sqrt(x)` | Absolute value / floor / integer square root |
| `max(a,b)` / `min(a,b)` | Signed max / min |

### Tables & Strings

| Function | Description |
| :------- | :---------- |
| `{a=1, 2, 3}` | Table constructor; `t.x`, `t[i]` access |
| `len(t)` | Sequence length |
| `add(t,v)` | Append `v` at `len(t)+1` |
| `"a" .. "b"` | String concatenation |
| `sub(s,i,j)` | 1-based inclusive substring |
| `strlen(s)` | String length |
| `tostring(v)` | Number → string |

### Audio & System

| Function | Description |
| :------- | :---------- |
| `sfx(i)` | Play sound effect from SFX bank |
| `music(i)` (alias `mus`) | Play music track |
| `nomusic()` (alias `nomus`) | Stop music |
| `wait()` | Wait for next frame (VSync) |

---

## 🖌️ Editor Suite

Press function keys at any time to switch between modes:

| Key | Mode |
| :-- | :--- |
| `F1` | ▶️ Run (game) |
| `F2` | 🖼️ Sprite editor |
| `F3` | 🗺️ Map editor |
| `F4` | 🎵 SFX editor |
| `F5` | 🎶 Music editor |
| `F6` | 🎨 Palette editor |
| `F7` | 📋 Cart meta editor |
| `F8` | 📂 Cart browser |
| `F9` | 📝 Code editor |

### 🖼️ Sprite Editor

| Zone | Location | Controls |
| :--- | :------- | :------- |
| Zoom canvas | Top-left 64×64 | Click/drag to paint; `F` toggles fill mode |
| Sheet browser | Right 64×64 | Click to select active sprite |
| Palette strip | Row 64–71 | Click to select draw color |

| Key | Action |
| :-- | :----- |
| `C` | Copy active sprite to clipboard |
| `V` | Paste clipboard into active sprite |
| `F` | Toggle flood-fill mode (orange cursor) |

### 📝 Code Editor

Edit the loaded `.fc` source in-engine. `Ctrl+R` compiles and runs; on a compile error the cursor jumps to the offending line and the error shows in the status bar. `Ctrl+S` saves the source file.

### 📋 Cart Meta Editor

Editable title and author fields that sync into the ROM header on `Ctrl+S`.

| Key | Action |
| :-- | :----- |
| `Tab` | Switch focus between Title / Author |
| Type | Append character (A–Z, 0–9, punctuation) |
| `Backspace` | Delete last character |
| `Ctrl+S` | Save cart (writes ROM + updates header) |

### 🔍 Debugger

Active when `--debug` flag is passed with `run`. Arrow-key controls apply only while paused.

| Key | Action |
| :-- | :----- |
| `Space` | Pause / resume |
| `C` / `F10` | Step one instruction |
| `B` | Toggle breakpoint at cursor address |
| `↑` / `↓` | Move disassembly cursor |
| `←` / `→` | Scrub timeline (restores VM snapshot) |
| `N` / `M` | Previous / next RAM page |

Breakpoints and register aliases persist in a `.fcdbg` TOML sidecar:

```toml
breakpoints = [9, 42]
r0 = "player_x"
r1 = "player_y"
```

---

## 📟 System Specifications

| Component | Specification |
| :-------- | :------------ |
| **CPU** | Custom 32-bit, 8 registers (R0–R7; fc-lang reserves R3 as frame pointer) |
| **Resolution** | 128×128 (upscaled 4×) |
| **RAM** | 64 KiB |
| **Palette** | 16 colors |
| **Sprites** | 256 × 8×8 pixels |
| **Map** | 64×64 tiles |

### Memory Map

| Range | Region |
| :---- | :----- |
| `0x0000–0x3FFF` | General RAM: stack, globals (compiler scratch `0x3F80–0x3FAC`) |
| `0x4000–0x7FFF` | Sprite sheet — 256 sprites × 64 bytes (1 byte/pixel) |
| `0x8000–0x8FFF` | Tilemap 64×64 (1 byte/cell) |
| `0x9000–0x90FF` | Sprite flags (1 byte/sprite) |
| `0x9100–0x91FF` | Palette |
| `0x9200–0x95FF` | SFX bank (16 × 64 bytes) |
| `0x9600–0x96FF` | Music bank (8 × 32 bytes) |
| `0x9700–0xFFFF` | Heap (strings, dynamic data) |

---

## 📜 Assembly Reference

### ⌨️ Instructions

<details>
<summary><b>🎨 Graphics & Rendering</b> (Click to expand)</summary>

| Instruction | Arguments | Description |
| :---------- | :-------- | :---------- |
| `CLS` | — | Clear screen (fill with color 0) |
| `FILL` | `color` | Fill entire screen with palette index |
| `FILLR` | `rcol` | Fill screen, color from register |
| `PAL` | `idx r g b` | Set palette entry (literals) |
| `PALR` | `ri rr rg rb` | Set palette entry from registers |
| `PSET` | `rx ry rcol` | Set pixel (signed coords, camera-aware) |
| `LINE` | `rx1 ry1 rx2 ry2 rcol` | Draw line |
| `RECT` | `rx1 ry1 rx2 ry2 rcol` | Rectangle outline |
| `RECTF` | `rx1 ry1 rx2 ry2 rcol` | Filled rectangle |
| `CIRC` | `rx ry rr rcol` | Circle outline |
| `CIRCF` | `rx ry rr rcol` | Filled circle |
| `SPR` | `rid rx ry rflip` | Draw sprite by id; flip bits 0/1 = H/V |
| `MGET` | `rd rx ry` | Read tilemap cell |
| `MSET` | `rx ry rtile` | Write tilemap cell |
| `FGET` | `rd rid` | Read sprite flags |
| `FSET` | `rid rflags` | Write sprite flags |
| `MAPD` | `rcx rcy rsx rsy rw rh` | Draw tilemap block |
| `POSC` | `rx ry` | Set camera position |
| `MOVC` | `rx ry` | Move camera by delta |
| `TXT` | `rx ry rcol rbase len` | Draw text from memory (known length) |
| `TXTZ` | `rx ry rcol rbase` | Draw null-terminated text |
| `TAT` | `rx ry raddr rcol flags` | Draw text from null-terminated address |
| `TSD` | `rx ry raddr` | Draw text (simple, default color) |
| `NUM` | `rx ry rcol rval` | Draw integer value |
| `DPX` | `x y r g b` | Draw literal RGB pixel (legacy) |
| `DPXR` | `rx ry r g b` | Draw RGB pixel at register coords (legacy) |
| `SPT` | `rx ry rspr` | Draw sprite, id in register (legacy) |
| `TIL` | `rx ry raddr rpal w h` | Draw tiles from address (legacy) |

</details>

<details>
<summary><b>🔢 Arithmetic & Logic</b> (Click to expand)</summary>

| Instruction | Arguments | Description |
| :---------- | :-------- | :---------- |
| `MOV` | `reg val` | Load 16-bit literal into register |
| `MOV32` | `reg dword` | Load 32-bit literal into register |
| `MOVR` | `dst src` | Copy register |
| `ADD` | `reg val` | Add literal |
| `ADDR` | `dst src` | Add register |
| `SUB` | `reg val` | Subtract literal |
| `SUBR` | `dst src` | Subtract register |
| `DEC` | `reg` | Decrement by 1 |
| `MUL` | `dst src` | Multiply (wrapping) |
| `DIV` | `dst src` | Signed integer divide |
| `MOD` | `dst src` | Signed integer modulo |
| `FMUL` | `dst src` | 16:16 fixed-point multiply `(a*b)>>16` |
| `FDIV` | `dst src` | 16:16 fixed-point divide `(a<<16)/b` |
| `MATH1` | `dst src kind` | Unary math: 0=sin 1=cos 2=abs 3=flr 4=sqrt |
| `MAX` | `dst src` | Signed maximum |
| `MIN` | `dst src` | Signed minimum |
| `RND` | `reg max` | Random integer in `[0, max)` |
| `RNDR` | `rd rmax` | Random integer, max from register |
| `SLT` | `rd rs1 rs2` | Set `rd = (rs1 < rs2)` unsigned |
| `SLTS` | `rd rs1 rs2` | Set `rd = (rs1 < rs2)` signed |
| `EQ` | `rd rs1 rs2` | Set `rd = (rs1 == rs2)` |
| `AND` | `dst src` | Bitwise AND |
| `OR` | `dst src` | Bitwise OR |
| `XOR` | `dst src` | Bitwise XOR |
| `NOT` | `reg` | Bitwise NOT |
| `NEG` | `reg` | Arithmetic negation |
| `SHL` | `reg shift` | Logical shift left |
| `SHR` | `reg shift` | Logical shift right |
| `SAR` | `reg shift` | Arithmetic shift right |

`MATH1` sin/cos: input 0–255 = 256ths of a full turn; output −127..127.

</details>

<details>
<summary><b>💾 Memory & Tables</b> (Click to expand)</summary>

| Instruction | Arguments | Description |
| :---------- | :-------- | :---------- |
| `LDM` | `reg addr` | Load byte from absolute address |
| `STM` | `addr reg` | Store byte to absolute address |
| `LDMW` | `reg addr` | Load 16-bit word |
| `STMW` | `addr reg` | Store 16-bit word |
| `LDM32` | `reg addr` | Load 32-bit dword |
| `STM32` | `addr reg` | Store 32-bit dword |
| `LDMI` | `rd rs` | Indirect byte load from RAM (address in `rs`) |
| `STMI` | `ra rv` | Indirect byte store (address in `ra`) |
| `LDM32I` | `rd rs` | Indirect 32-bit load |
| `STM32I` | `ra rv` | Indirect 32-bit store |
| `CPY` | `dst src len` | Copy `len` bytes from program space to RAM |
| `TNEW` | `rd` | Create table, handle → `rd` |
| `TGET` | `rd rt rk` | `rd = table[key]` |
| `TSET` | `rt rk rv` | `table[key] = value` |
| `TLEN` | `rd rt` | Sequence length |
| `TIDX` | `rk rv rt ri` | Iterate entry `ri`; key/value out, key = `0xFFFFFFFF` at end |

Tables live outside guest RAM in the VM (unlimited growth, snapshot-aware); handles are opaque 32-bit values.

</details>

<details>
<summary><b>🔀 Control Flow & Stack</b> (Click to expand)</summary>

| Instruction | Arguments | Description |
| :---------- | :-------- | :---------- |
| `JMP` | `addr` | Unconditional jump |
| `JNZ` | `reg addr` | Jump if register ≠ 0 |
| `JZ` | `reg addr` | Jump if register = 0 |
| `JSR` | `addr` | Call subroutine (push return address) |
| `JREG` | `reg` | Jump to address in register |
| `RET` | — | Return from subroutine |
| `PUSH` | `reg` | Push register onto stack |
| `POP` | `reg` | Pop from stack into register |
| `GETSP` | `reg` | Read stack pointer |
| `SETSP` | `reg` | Write stack pointer |
| `WAIT` | — | Wait for next frame (VSync) |

</details>

<details>
<summary><b>🎮 Input & Debug</b> (Click to expand)</summary>

| Instruction | Arguments | Description |
| :---------- | :-------- | :---------- |
| `IN` | `reg id` | Button held (0=Up 1=Down 2=Left 3=Right 4=A 5=B) |
| `INR` | `rd rid` | Button held, id from register |
| `INP` | `reg id` | Button pressed this frame |
| `INPR` | `rd rid` | Button pressed this frame, id from register |
| `LOGR` | `reg` | Log register value to host console |
| `LOGV` | `val` | Log literal value to host console |

</details>

<details>
<summary><b>🔊 Audio</b> (Click to expand)</summary>

| Instruction | Arguments | Description |
| :---------- | :-------- | :---------- |
| `SND` | `rf rv rd` | Play tone: freq/vol/dur from registers |
| `SNDV` | `freq vol dur` | Play tone with literal values |
| `NOSND` | — | Stop all sound |
| `NSND` | `rf rv rd` | Play noise: freq/vol/dur from registers |
| `NSNDV` | `freq vol dur` | Play noise with literal values |
| `SSTOP` | — | Stop sound channel |
| `NSTOP` | — | Stop noise channel |
| `SFX` | `idx` | Play sound effect by index |
| `SFXR` | `ridx` | Play sound effect, index from register |
| `MUS` | `idx` | Play music track by index |
| `MUSR` | `ridx` | Play music track, index from register |
| `NOMUS` | — | Stop music |

</details>

### 📝 Directives

| Directive | Arguments | Description |
| :-------- | :-------- | :---------- |
| `.DB` | `val...` | **Define Byte**: insert 8-bit values or a quoted string |
| `.DW` | `val...` | **Define Word**: insert 16-bit values or labels |
| `.ORG` | `addr` | **Origin**: set the assembly start address |
| `.FILL` | `len val` | Fill `len` bytes with value |

---

## 📝 Example Code

### Assembly

```asm
; Move a sprite around the screen
init:
    PAL 1 255 80 0      ; Define palette index 1

loop:
    CLS
    IN R2 2             ; Read left button
    JZ R2 skip_left
    DEC R0              ; Move left

skip_left:
    IN R2 3             ; Read right button
    JZ R2 skip_right
    ADD R0 1            ; Move right

skip_right:
    MOV R4 0            ; Sprite id 0
    MOV R5 0            ; No flip
    SPR R4 R0 R1 R5     ; Draw sprite 0 at (R0, R1)
    WAIT
    JMP loop
```

### fc-lang (High-Level)

```lua
const SPEED = 2

let x = 60
let y = 60

init:
  pal(0, 10, 10, 30)

loop:
  cls()
  if btn(0) then y -= SPEED end
  if btn(1) then y += SPEED end
  if btn(2) then x -= SPEED end
  if btn(3) then x += SPEED end
  spr(0, x, y)
  wait()
```

---

## 🌐 fc-hub (Cart Sharing Server)

Self-hostable cart gallery server built with Rocket + SQLite.

```bash
cd crates/fc-hub
cargo run --release
# or
docker compose up
```

| Method | Path | Description |
| :----- | :--- | :---------- |
| `GET` | `/` | HTML gallery page (search + paging) |
| `GET` | `/api/carts` | List published carts (`page`, `per_page`, `q`) |
| `POST` | `/api/carts` | Upload cart (multipart, API key required) |
| `GET` | `/api/carts/:id` | Cart metadata |
| `GET` | `/api/carts/:id/rom` | Download cart ROM |
| `GET` | `/api/carts/:id/screenshot` | Cart screenshot PNG |
| `POST` | `/api/carts/:id/screenshot` | Upload screenshot (API key required) |

Configure via `FC_HUB_API_KEY` and `DATABASE_URL` environment variables. Release builds refuse to start with the default `changeme` API key — set a real one; debug builds only print a warning.

---

## 📂 Project Structure

Cargo workspace with eight crates:

| Crate | Description |
| :---- | :---------- |
| `crates/fc-core` | Shared types and memory map — `Color`, `Vec2`, RAM layout constants |
| `crates/fc-asm` | Assembler: tokeniser, parser, code emitter, source maps, ISA table |
| `crates/fc-rom` | ROM format: header, section layout, load/write helpers |
| `crates/fc-vm` | VM core: CPU, ISA interpreter, renderer, audio, input, debugger |
| `crates/fc-lang` | High-level language compiler (`.fc` → bytecode) |
| `crates/fc-engine` | Main binary: editor suite, cart browser, app loop |
| `crates/fc-hub` | Cart sharing server |

`games/` — example `.asm` and `.fc` sources with pre-built `.rom` files.

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
