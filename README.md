# 🎮 Fantasy Console

![Rust](https://img.shields.io/badge/rust-%23E32F26.svg?style=for-the-badge&logo=rust&logoColor=white)
![License](https://img.shields.io/badge/license-MIT-green.svg?style=for-the-badge)

**Fantasy Console** is a retro-inspired virtual machine and development environment written in Rust. Custom 32-bit CPU, built-in assembler, high-level scripting language, full suite of in-engine editors, and an optional cart-sharing hub — inspired by PICO-8.

> [!TIP]
> Dive into the world of low-level programming and create your own retro games using a simple yet powerful assembly language — or skip straight to the high-level `.fc` scripting language.

---

## ✨ Features

- 🖥️ **Custom 32-bit Architecture** — 4 general-purpose registers (R0–R3), 32KB addressable RAM, 16:16 fixed-point math
- 🎨 **Palette-based Graphics** — 128×128 resolution, 16-color swappable palette, sprite, tilemap, and text rendering
- 🛠️ **Integrated Assembler** — tokeniser, parser, code emitter, source maps, label resolution
- 📝 **High-Level Language** (`fc-lang`) — Lua-like scripting that compiles to the native ISA; tables, closures, math/audio/graphics builtins
- 🔊 **Audio Engine** — real-time sound synthesis and music playback via CPAL
- 🖌️ **Full Editor Suite** — sprite, map, palette, SFX, music, and cart-meta editors; local & hub cart browser
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
| `run <file>` | Run a `.asm` (hot-reload) or `.rom` file |
| `build <source.asm> <output.rom>` | Assemble source and write ROM |
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
| **CPU** | Custom 32-bit |
| **Resolution** | 128×128 (upscaled 4×) |
| **RAM** | 32 KB |
| **Registers** | 4 (R0–R3) |
| **Palette** | 16 colors |
| **Sprite size** | 8×8 pixels |
| **Sprite sheet** | `0x4000` |
| **Map** | `0x5000` |
| **Palette RAM** | `0x5800` |
| **SFX** | `0x5C00` |
| **Music** | `0x6000` |

---

## 📜 Assembly Reference

### ⌨️ Instructions

<details>
<summary><b>🎨 Graphics & Rendering</b> (Click to expand)</summary>

| Instruction | Arguments | Description |
| :---------- | :-------- | :---------- |
| `CLS` | — | Clear screen (fill with color 0) |
| `FILL` | `color` | Fill entire screen with palette index |
| `DPX` | `x y r g b` | Draw literal RGB pixel |
| `DPXR` | `rx ry r g b` | Draw RGB pixel at register coords |
| `PAL` | `idx r g b` | Set palette entry |
| `SPT` | `rx ry rspr` | Draw 8×8 sprite at `(rx,ry)` |
| `TIL` | `rx ry raddr rpal w h` | Draw tilemap `w×h` tiles from address |
| `TXT` | `rx ry rcol raddr rlen` | Draw text from memory |
| `NUM` | `rx ry rcol rval` | Draw integer value |
| `TAT` | `rx ry raddr rcol flags` | Draw text from null-terminated address |
| `TSD` | `rx ry raddr` | Draw text (simple, default color) |

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
<summary><b>💾 Memory Operations</b> (Click to expand)</summary>

| Instruction | Arguments | Description |
| :---------- | :-------- | :---------- |
| `LDM` | `reg addr` | Load byte from absolute address |
| `STM` | `addr reg` | Store byte to absolute address |
| `LDMW` | `reg addr` | Load 16-bit word |
| `STMW` | `addr reg` | Store 16-bit word |
| `LDM32` | `reg addr` | Load 32-bit dword |
| `STM32` | `addr reg` | Store 32-bit dword |
| `LDMI` | `rd rs` | Indirect byte load (address in `rs`) |
| `STMI` | `ra rv` | Indirect byte store (address in `ra`) |
| `LDM32I` | `rd rs` | Indirect 32-bit load |
| `STM32I` | `ra rv` | Indirect 32-bit store |
| `CPY` | `dst src len` | Copy `len` bytes |

</details>

<details>
<summary><b>🔀 Control Flow & Stack</b> (Click to expand)</summary>

| Instruction | Arguments | Description |
| :---------- | :-------- | :---------- |
| `JMP` | `addr` | Unconditional jump |
| `JNZ` | `reg addr` | Jump if register ≠ 0 |
| `JZ` | `reg addr` | Jump if register = 0 |
| `JSR` | `addr` | Call subroutine (push return address) |
| `RET` | — | Return from subroutine |
| `PUSH` | `reg` | Push register onto stack |
| `POP` | `reg` | Pop from stack into register |
| `GETSP` | `reg` | Read stack pointer |
| `SETSP` | `reg` | Write stack pointer |
| `WAIT` | — | Wait for next frame (VSync) |

</details>

<details>
<summary><b>🎮 Input & Camera</b> (Click to expand)</summary>

| Instruction | Arguments | Description |
| :---------- | :-------- | :---------- |
| `IN` | `reg id` | Read button state (0=Up 1=Down 2=Left 3=Right 4=A 5=B) |
| `POSC` | `rx ry` | Set camera position |
| `MOVC` | `rx ry` | Move camera by delta |

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
| `MUS` | `idx` | Play music track by index |
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
    SPT R0 R1 0         ; Draw sprite 0 at (R0, R1)
    WAIT
    JMP loop
```

### fc-lang (High-Level)

```lua
local x = 60
local y = 60

while true do
    if input(2) then x = x - 1 end
    if input(3) then x = x + 1 end
    if input(0) then y = y - 1 end
    if input(1) then y = y + 1 end

    cls()
    spt(x, y, 0)
    wait()
end
```

---

## 📝 fc-lang Built-ins

| Function | Description |
| :------- | :---------- |
| `cls()` | Clear screen |
| `fill(c)` | Fill screen with color index |
| `spt(x,y,s)` | Draw sprite `s` at `(x,y)` |
| `til(x,y,addr,pal,w,h)` | Draw tilemap |
| `pal(i,r,g,b)` | Set palette entry |
| `pset(x,y,r,g,b)` | Draw pixel |
| `txt(x,y,col,addr,len)` | Draw text from memory |
| `num(x,y,col,val)` | Draw number |
| `input(id)` | Read button 0–5 |
| `rnd(max)` | Random integer `[0,max)` |
| `sin(t)` | Sine — 0..255 = full turn, returns −127..127 |
| `cos(t)` | Cosine |
| `abs(x)` | Absolute value |
| `sqrt(x)` | Integer square root |
| `max(a,b)` | Signed maximum |
| `min(a,b)` | Signed minimum |
| `sfx(idx)` | Play SFX |
| `mus(idx)` | Play music track |
| `nomus()` | Stop music |
| `wait()` | Wait for VSync |

---

## 🌐 fc-hub (Cart Sharing Server)

Self-hostable cart gallery server built with Axum + SQLite.

```bash
cd crates/fc-hub
cargo run --release
# or
docker compose up
```

| Method | Path | Description |
| :----- | :--- | :---------- |
| `GET` | `/carts` | List published carts |
| `POST` | `/carts` | Upload cart (multipart, API key required) |
| `GET` | `/carts/:id` | Download cart ROM |
| `GET` | `/gallery` | HTML gallery page |
| `POST` | `/screenshot/:id` | Upload screenshot for cart |

Configure via `FC_HUB_API_KEY` and `DATABASE_URL` environment variables. Release builds refuse to start with the default `changeme` API key — set a real one; debug builds only print a warning.

---

## 📂 Project Structure

Cargo workspace with eight crates:

| Crate | Description |
| :---- | :---------- |
| `crates/fc-core` | Shared types — `Color`, `Vec2` |
| `crates/fc-asm` | Assembler: tokeniser, parser, code emitter, source maps |
| `crates/fc-rom` | ROM format: header, section layout, load/write helpers |
| `crates/fc-vm` | VM core: CPU, ISA, renderer, audio, input, debugger |
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
