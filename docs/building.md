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
| `export <project> --web -o <out.html>` | Export a self-contained, offline browser player                |
| `publish <cart>`               | Upload a cart (`.cav` or project dir) to a caiven-port instance       |

To just run a cart (no editor), use `caiven-machine`:

```bash
cargo run -p caiven-machine -- my-game/    # project dir, Ctrl+R restarts the cart
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
| `--remixable`      | —                              | Let others remix the new cart in the browser (Quick Remix); a project dir is uploaded unminified so its Lua stays readable                  |

## Project Structure

Cargo workspace with eight Rust crates and two frontend packages:

| Crate                     | Description                                                                                                                                    |
| :------------------------ | :--------------------------------------------------------------------------------------------------------------------------------------------- |
| `crates/caiven-core`      | Shared types and memory map — `Color`, `Vec2`, RAM layout constants                                                                            |
| `crates/caiven-cart`      | Cart formats: binary `.cav` (header, section layout, load/write) and the project-dir authoring format (`caiven.toml` + `.lua` + `.hex`/`.png`) |
| `crates/caiven-vm`        | VM core: embedded Lua (`mlua`) execution, builtin API, renderer, audio, input, debugger hooks                                                  |
| `crates/caiven-studio`    | Tauri shell, VM actor, Studio IPC, and CLI (`build`/`unpack`/`inspect`/`publish`)                                                              |
| `crates/caiven-studio-ui` | Svelte 5 + Vite Studio frontend shared with Port brand tokens                                                                                  |
| `crates/caiven-ui`        | Shared Svelte components and theme consumed by Studio and Port                                                                                 |
| `crates/caiven-machine`   | Standalone cart runner (run mode: project dir or `.cav`, no editor/port; `Ctrl+R` restarts the cart)                                                 |
| `crates/caiven-port`      | Cart sharing server                                                                                                                            |
| `crates/caiven-web`       | WASM cart player (`wasm32-unknown-emscripten`) served by caiven-port's `/play/:id`                                                             |
| `crates/migration`        | `sea-orm` database migrations for caiven-port                                                                                                  |

`carts/` contains packed cartridges; `projects/` contains editable projects.
For a quick runtime check, use `cargo run -p caiven-machine -- carts/dev/smoke.cav`.

## Verification

```bash
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings -A unused-imports
npm --prefix crates/caiven-studio-ui run check:ui
npm --prefix crates/caiven-studio-ui run check
npm --prefix crates/caiven-studio-ui test
npm --prefix crates/caiven-port/web run check
npm --prefix crates/caiven-port/web test
node crates/caiven-web/smoke_test.mjs
```

Install browser test dependencies once with
`npm --prefix crates/caiven-port/web exec playwright install chromium`.
Run Studio's browser suite with `npm --prefix crates/caiven-studio-ui run test:e2e`.
Run Port's mocked and live server suites with
`npm --prefix crates/caiven-port/web run test:e2e`.

## Browser runtime

Port and Studio's HTML export share the checked-in runtime under
`crates/caiven-port/web/public/wasm/`. After changing cartridge formats, the VM,
or browser exports, activate an Emscripten SDK and run:

```bash
bash crates/caiven-web/build-web.sh
```

This builds with the Cargo lockfile, updates both shipped runtime files, and
smoke-tests the shipped artifact against a current cartridge, including the
offline instantiation hook. Rebuild the Port frontend and Studio afterward so
their assets include the refreshed runtime. Commit both generated files together.
