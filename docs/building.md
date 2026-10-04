# Building from source

This page is for people working on Caiven. To make or play games, use the
[prebuilt downloads](../README.md#getting-started).

## Prerequisites

- [Rust stable](https://rustup.rs/)
- [Node.js 22](https://nodejs.org/) with npm
- [Tauri system dependencies](https://v2.tauri.app/start/prerequisites/) for your OS
- On Linux, for Machine audio and input: `libasound2-dev libxkbcommon-dev pkg-config`

## Install

```bash
git clone https://github.com/andrejmarkus/caiven.git
cd caiven
npm --prefix crates/caiven-studio-ui ci
npm --prefix crates/caiven-studio-ui run build
npm --prefix crates/caiven-port/web ci
npm --prefix crates/caiven-port/web run build
cargo build --release --workspace
```

Port and Studio share their shadcn-svelte components and theme through
`crates/caiven-ui`. After you change UI dependencies or components, run
`npm --prefix crates/caiven-studio-ui run check:ui`. It checks that both
apps use the shared components and the same versions.

## Run

Studio in development mode:

```bash
cd crates/caiven-studio
npm --prefix ../caiven-studio-ui exec tauri dev
```

Studio CLI commands, from the repository root:

```bash
cargo run -p caiven-studio -- [command]
```

| Command                        | Description                                                           |
| :----------------------------- | :---------------------------------------------------------------------|
| _(no command)_                 | Open Studio on its start screen                                       |
| `edit [file]`                  | Open Studio, optionally with a project folder or `.cav` file          |
| `inspect <path>`               | Print the cart's section table (project folder or `.cav`)             |
| `build <project> -o <out.cav>` | Pack a project folder into a `.cav`                                   |
| `unpack <file.cav> -o <out>`   | Unpack a `.cav` into an editable project folder                       |
| `export <project> --web -o <out.html>` | Export a single-file browser player that works offline        |
| `publish <cart>`               | Upload a `.cav` or project folder to a Port server                    |

To play a cart without the editor, use `caiven-machine`:

```bash
cargo run -p caiven-machine -- my-game/    # project folder; Ctrl+R restarts the cart
cargo run -p caiven-machine -- game.cav    # packed cart
```

### Publish flags

| Flag               | Default                       | Description |
| :------------------| :------------------------------| :-----------|
| `--url`            | `http://localhost:8080`       | Port base URL (env: `CAIVEN_PORT_URL`) |
| `--api-key`        | _(empty, required)_           | Your Port API token (env: `CAIVEN_PORT_API_KEY`). Create one on the Profile page of the Port website, or sign in from Studio's PORT tab. |
| `--title`          | cart header                   | Override the cart title |
| `--author`         | cart header                   | Override the author |
| `--description`    | _(empty)_                     | Short description |
| `--tags`           | _(empty)_                     | Comma-separated tags |
| `--frames`         | `30`                          | Frames to run before taking the screenshot |
| `--no-screenshot`  |                               | Skip the screenshot |
| `--remixable`      |                               | Let others remix the cart in the browser (Quick Remix). Project folders upload unminified so the Lua stays readable. |

## Project layout

The Cargo workspace has eight Rust crates and two frontend packages:

| Crate                     | Description |
| :------------------------ | :---------- |
| `crates/caiven-core`      | Shared types and memory map: `Color`, `Vec2`, RAM layout constants |
| `crates/caiven-cart`      | Cart formats: binary `.cav` (header, sections, load and write) and the project folder format (`caiven.toml`, `.lua`, `.hex` or `.png`) |
| `crates/caiven-vm`        | The VM: Lua execution through `mlua`, builtin API, renderer, audio, input and debugger hooks |
| `crates/caiven-studio`    | Tauri shell, VM thread, Studio IPC and the CLI (`build`, `unpack`, `inspect`, `publish`) |
| `crates/caiven-studio-ui` | Studio frontend in Svelte 5 and Vite, using Port's brand tokens |
| `crates/caiven-ui`        | Svelte components and theme shared by Studio and Port |
| `crates/caiven-machine`   | Standalone player for a project folder or `.cav`, with no editor or Port features. `Ctrl+R` restarts the cart. |
| `crates/caiven-port`      | Cart sharing server |
| `crates/caiven-web`       | WASM player (`wasm32-unknown-emscripten`) that Port serves at `/play/:id` |
| `crates/migration`        | `sea-orm` database migrations for Port |

`carts/` holds packed carts and `projects/` holds their sources. For a quick
runtime check, run `cargo run -p caiven-machine -- carts/dev/smoke.cav`.

## Checks

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

Install the browser test dependencies once with
`npm --prefix crates/caiven-port/web exec playwright install chromium`.
Studio's browser tests run with
`npm --prefix crates/caiven-studio-ui run test:e2e`. Port's mocked and
live-server tests run with `npm --prefix crates/caiven-port/web run test:e2e`.

## Browser runtime

Port and Studio's HTML export both use the WASM runtime checked in under
`crates/caiven-port/web/public/wasm/`. If you change the cart format, the VM
or the browser export, activate an Emscripten SDK and run:

```bash
bash crates/caiven-web/build-web.sh
```

The script builds against the Cargo lockfile, updates both runtime files,
and smoke-tests them with a current cart, including the offline startup
hook. Rebuild the Port frontend and Studio afterwards so they pick up the
new runtime, and commit both generated files together.
