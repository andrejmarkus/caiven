# Building from Source

For contributors working on Caiven itself. If you just want to make or play
games, use the [prebuilt downloads](../README.md#-quick-start) instead.

## Prerequisites

- [Rust stable](https://rustup.rs/)
- [Node.js 22](https://nodejs.org/) with npm
- [Tauri system dependencies](https://v2.tauri.app/start/prerequisites/) for your OS

## Installation

```bash
git clone https://github.com/andrejmarkus/caiven.git
cd caiven
npm --prefix crates/caiven-studio-ui ci
npm --prefix crates/caiven-studio-ui run build
npm --prefix crates/caiven-port/web ci
npm --prefix crates/caiven-port/web run build
cargo build --release --workspace
```

Port and Studio consume the same shadcn-svelte components and theme from
`crates/caiven-ui`. Run `npm --prefix crates/caiven-studio-ui run check:ui`
after UI dependency or component changes to verify ownership and version parity.

## Running

Launch Studio in development mode:

```bash
cd crates/caiven-studio
npm --prefix ../caiven-studio-ui exec tauri dev
```

Studio CLI commands run from repository root:

```bash
cargo run -p caiven-studio -- [command]
```

| Command                        | Description                                                           |
| :----------------------------- | :---------------------------------------------------------------------|
| _(no command)_                 | Launch Caiven Studio on its start screen                              |
| `edit [file]`                  | Launch Caiven Studio, optionally opening a project dir or `.cav` file |
| `inspect <path>`               | Print cart section table (project dir or `.cav`)                      |
| `build <project> -o <out.cav>` | Build a project dir into a distribution `.cav` cartridge              |
| `unpack <file.cav> -o <out>`   | Unpack a binary `.cav` into an editable project dir                   |
| `publish <cart>`               | Upload a cart (`.cav` or project dir) to a caiven-port instance       |

To just run a cart (no editor), use `caiven-machine`:

```bash
cargo run -p caiven-machine -- my-game/    # project dir, hot-reloads with Ctrl+R
cargo run -p caiven-machine -- game.cav    # distribution cartridge
```

**Publish flags:**

| Flag               | Default                       | Description                                                                                                                                  |
| :------------------| :------------------------------| :----------------------------------------------------------------------------------------------------------------------------------------------|
| `--url`            | `http://localhost:8080`       | Port base URL (env: `CAIVEN_PORT_URL`)                                                                                                       |
| `--api-key`        | _(empty, required)_           | Per-user port API token (env: `CAIVEN_PORT_API_KEY`) — mint one via the port web UI Profile page or by logging into Caiven Studio's PORT tab |
| `--title`          | cart header                   | Override cart title                                                                                                                          |
| `--author`         | cart header                   | Override author                                                                                                                              |
| `--description`    | _(empty)_                     | Short description                                                                                                                            |
| `--tags`           | _(empty)_                     | Comma-separated tags                                                                                                                         |
| `--frames`         | `30`                          | Frames to run before screenshot                                                                                                              |
| `--no-screenshot`  | —                              | Skip screenshot capture                                                                                                                      |

## Project Structure

Cargo workspace with nine crates:

| Crate                     | Description                                                                                                                                    |
| :------------------------ | :--------------------------------------------------------------------------------------------------------------------------------------------- |
| `crates/caiven-core`      | Shared types and memory map — `Color`, `Vec2`, RAM layout constants                                                                            |
| `crates/caiven-cart`      | Cart formats: binary `.cav` (header, section layout, load/write) and the project-dir authoring format (`caiven.toml` + `.lua` + `.hex`/`.png`) |
| `crates/caiven-vm`        | VM core: embedded Lua (`mlua`) execution, builtin API, renderer, audio, input, debugger hooks                                                  |
| `crates/caiven-studio`    | Tauri shell, VM actor, Studio IPC, and CLI (`build`/`unpack`/`inspect`/`publish`)                                                              |
| `crates/caiven-studio-ui` | Svelte 5 + Vite Studio frontend shared with Port brand tokens                                                                                  |
| `crates/caiven-machine`   | Standalone cart runner (run mode: project dir or `.cav`, no editor/port; `Ctrl+R` hot-reloads)                                                 |
| `crates/caiven-port`      | Cart sharing server                                                                                                                            |
| `crates/caiven-web`       | WASM cart player (`wasm32-unknown-emscripten`) served by caiven-port's `/play/:id`                                                             |
| `crates/migration`        | `sea-orm` database migrations for caiven-port                                                                                                  |

`games/carts/` — example carts, ready to run: `cargo run -p caiven-machine -- games/carts/catch.cav`, or open in Caiven Studio via `caiven-studio edit`.
