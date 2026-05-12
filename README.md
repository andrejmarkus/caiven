# 🎮 Fantasy Console

![Rust](https://img.shields.io/badge/rust-%23E32F26.svg?style=for-the-badge&logo=rust&logoColor=white)
![Pixels](https://img.shields.io/badge/Pixels-orange.svg?style=for-the-badge)
![Winit](https://img.shields.io/badge/Winit-blue.svg?style=for-the-badge)
![License](https://img.shields.io/badge/license-MIT-green.svg?style=for-the-badge)

**Fantasy Console** is a retro-inspired virtual machine and development environment written in Rust. It features a custom 16-bit CPU architecture, a built-in assembler, and a pixel-perfect rendering engine.

> [!TIP]
> Dive into the world of low-level programming and create your own retro games using a simple yet powerful assembly language.

---

## ✨ Features

- 🖥️ **Custom 16-bit Architecture**: 4 general-purpose registers, 32KB of addressable memory.
- 🎨 **Palette-based Graphics**: 128x128 resolution with a 16-color swappable palette.
- 🕹️ **Integrated Assembler**: Compile `.asm` source files directly into executable binary `.rom` files.
- 🔊 **Audio Engine**: Real-time sound synthesis using CPAL.
- 🛠️ **Debugger**: Step-through execution and memory inspection for easier development.
- ⌨️ **Input Handling**: Support for classic directional and button inputs with configurable key bindings.

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

| Command | Description |
| :------ | :---------- |
| `cargo run -- dev` | Development mode — loads `games/asm/movement.asm` if present |
| `cargo run -- run <file.rom>` | Run a compiled ROM |
| `cargo run -- build <source.asm> <output.rom>` | Assemble source and write ROM |
| `cargo run -- debug <source.asm>` | Run source with debugger enabled |

### Debugger Controls

| Key | Action |
| :-- | :----- |
| `Space` | Pause / resume execution |
| `C` | Step one instruction |
| `B` | Step back to previous snapshot |
| `N` / `M` | Previous / next RAM page |

### Key Bindings

Default game controls:

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

## 📟 System Specifications

| Component       | Specification           |
| :-------------- | :---------------------- |
| **CPU**         | Custom 16-bit           |
| **Resolution**  | 128 x 128 (Upscaled 4x) |
| **Memory**      | 32 KB RAM               |
| **Registers**   | 4 (R0, R1, R2, R3)      |
| **Palette**     | 16 Colors               |
| **Sprite Size** | 8 x 8 Pixels            |

---

## 📜 Assembly Reference

### ⌨️ Instructions

<details>
<summary><b>Graphics & Rendering</b> (Click to expand)</summary>

| Instruction | Arguments                           | Description                                                |
| :---------- | :---------------------------------- | :--------------------------------------------------------- |
| `CLS`       | -                                   | Clear the screen (fills with black/0).                     |
| `FILL`      | `color`                             | Fill the entire screen with a specific color index.        |
| `DPX`       | `x`, `y`, `r`, `g`, `b`             | Draw a literal RGB pixel at (x, y).                        |
| `DPXR`      | `rx`, `ry`, `r`, `g`, `b`           | Draw an RGB pixel using coordinates from registers.        |
| `PAL`       | `idx`, `r`, `g`, `b`                | Set palette index `idx` to the specified RGB color.        |
| `SPT`       | `rx`, `ry`, `addr`                  | Draw an 8x8 sprite from memory address `addr` at (rx, ry). |
| `TIL`       | `rx`, `ry`, `addr`, `pal`, `w`, `h` | Draw a tilemap of size `w*h` from address `addr`.          |
| `TXT`       | `rx`, `ry`, `color`, `addr`, `len`  | Draw text from memory at `addr` with length `len`.         |
| `NUM`       | `rx`, `ry`, `color`, `val`          | Draw the numeric value `val` at (rx, ry).                  |

</details>

<details>
<summary><b>Logic & Arithmetic</b> (Click to expand)</summary>

| Instruction | Arguments          | Description                                           |
| :---------- | :----------------- | :---------------------------------------------------- |
| `MOV`       | `reg`, `val`       | Move a 16-bit literal `val` into register `reg`.      |
| `MOVR`      | `dest`, `src`      | Copy the value from register `src` to `dest`.         |
| `ADD`       | `reg`, `val`       | Add literal `val` to register `reg`.                  |
| `ADDR`      | `dest`, `src`      | Add `src` register value to `dest` register.          |
| `SUB`       | `reg`, `val`       | Subtract literal `val` from register `reg`.           |
| `SUBR`      | `dest`, `src`      | Subtract `src` register value from `dest` register.   |
| `DEC`       | `reg`              | Decrement the value in register `reg` by 1.           |
| `RND`       | `reg`, `max`       | Generate a random number [0, max) and store in `reg`. |
| `SLT`       | `rd`, `rs1`, `rs2` | Set `rd` to 1 if `rs1 < rs2`, otherwise 0.            |

</details>

<details>
<summary><b>Memory Operations</b> (Click to expand)</summary>

| Instruction | Arguments           | Description                                                |
| :---------- | :------------------ | :--------------------------------------------------------- |
| `LDM`       | `reg`, `addr`       | Load a byte from memory `addr` into register `reg`.        |
| `LDMW`      | `reg`, `addr`       | Load a 16-bit word from memory `addr` into register `reg`. |
| `STM`       | `addr`, `reg`       | Store the low byte of `reg` into memory `addr`.            |
| `STMW`      | `addr`, `reg`       | Store the 16-bit `reg` into memory `addr`.                 |
| `LDMI`      | `rd`, `rs`          | Indirect Load: Load byte from address in `rs` into `rd`.   |
| `STMI`      | `ra`, `rv`          | Indirect Store: Store byte from `rv` into address in `ra`. |
| `CPY`       | `dst`, `src`, `len` | Copy `len` bytes from `src` address to `dst` address.      |

</details>

<details>
<summary><b>Control Flow & Audio</b> (Click to expand)</summary>

| Instruction | Arguments     | Description                                          |
| :---------- | :------------ | :--------------------------------------------------- |
| `JMP`       | `addr`        | Jump to memory address or label.                     |
| `JZ`        | `reg`, `addr` | Jump to `addr` if `reg == 0`.                        |
| `JNZ`       | `reg`, `addr` | Jump to `addr` if `reg != 0`.                        |
| `JSR`       | `addr`        | Jump to Subroutine: Pushes return address and jumps. |
| `RET`       | -             | Return from subroutine.                              |
| `WAIT`      | -             | Wait for the next frame (VSync).                     |
| `IN`        | `reg`, `id`   | Read input device `id` state into register `reg`.    |
| `SNDV`      | `f`, `v`, `d` | Play sound with literal freq, vol, dur.              |
| `NOSND`     | -             | Immediately stop all active audio.                   |

</details>

### 📝 Directives

| Directive | Arguments    | Description                                         |
| :-------- | :----------- | :-------------------------------------------------- |
| `.DB`     | `val...`     | **Define Byte**: Insert 8-bit values or a "string". |
| `.DW`     | `val...`     | **Define Word**: Insert 16-bit values or labels.    |
| `.ORG`    | `addr`       | **Origin**: Set the starting memory address.        |
| `.FILL`   | `len`, `val` | Fill `len` bytes with value `val`.                  |

---

## 📝 Example Code

```asm
; A simple "Hello World" of pixels
init:
    PAL 1 255 0 0       ; Define Palette index 1 as RED (RGB: 255, 0, 0)
    MOV R0 64           ; X coordinate
    MOV R1 64           ; Y coordinate

loop:
    CLS                 ; Clear Screen
    DPXR R0 R1 255 0 0  ; Draw RED pixel at (R0, R1)
    WAIT                ; Wait for VSync
    JMP loop            ; Infinite loop
```

---

## 📂 Project Structure

Cargo workspace with four crates:

| Crate | Description |
| :---- | :---------- |
| `crates/fc-core` | Shared types — `Color`, `Rgb` |
| `crates/fc-asm` | Assembler: tokeniser, parser, code emitter |
| `crates/fc-rom` | ROM format: header, load/write helpers |
| `crates/fc-host` | Host runtime: VM, renderer, audio, input, debugger |

- `games/`: Example `.asm` sources and pre-built `.rom` files.

---

## 📜 License

This project is licensed under the MIT License.

---

<p align="center">Made with ❤️ and 🦀 by Andrej Markuš</p>
