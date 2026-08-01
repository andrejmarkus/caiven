# Caiven — Claude Code project instructions

Caiven is a retro-inspired fantasy console: a Rust VM embedding real Lua 5.4
(`mlua`, vendored) for game code, a Tauri 2 + Svelte 5 editor ("Caiven
Studio"), and an optional self-hostable cart-sharing server ("Caiven Port").
Creators author a project dir (`caiven.toml` + `.lua` + one asset per
section) and distribute a built `.cav` binary cartridge.

## Architecture (top level)

- `crates/caiven-core` — shared types/utilities used across crates.
- `crates/caiven-cart` — cartridge/project format: parse, build, unpack,
  minify (`format.rs`, `header.rs`, `section.rs`, `project.rs`, `bundle.rs`).
- `crates/caiven-vm` — the VM: Lua API registration (`src/vm/api_registry.rs`,
  `src/vm/lua_exec.rs`), rendering (`src/rendering`), input (`src/input`),
  audio, palette, RTC.
- `crates/caiven-machine` — standalone player embedding caiven-vm; runs a
  project dir (hot-reload) or a `.cav` cartridge.
- `crates/caiven-studio` (+ `crates/caiven-studio-ui`) — Tauri Rust backend
  and Svelte frontend for the editor: code/asset workspaces, map editor,
  debugger, hot-reload, undo/redo, publish flow.
- `crates/caiven-port` (+ `crates/caiven-port/web`) — cart-sharing server:
  auth (webauthn-rs), accounts, cart versions, ratings/comments, discovery.
- `crates/caiven-web` — web runtime entry point.
- `crates/migration` — sea-orm-migration schema history for Port's DB.
- `crates/caiven-ui` — shared shadcn-svelte component library consumed by
  both `caiven-studio-ui` and `caiven-port/web`; boundaries checked by
  `npm run check:ui` in each frontend.

## Canonical commands

```bash
cargo build --locked                       # workspace build
cargo test --locked --verbose              # workspace tests
cargo fmt --all -- --check                 # CI formatting gate
cargo clippy --locked --all-targets -- -D warnings -A unused-imports
cargo audit                                # dependency security (ignores RUSTSEC-2023-0071, see rust.yml)

npm --prefix crates/caiven-studio-ui run check      # svelte-check + tsc
npm --prefix crates/caiven-studio-ui run check:ui   # shared-UI boundary check
npm --prefix crates/caiven-studio-ui test           # unit tests
npm --prefix crates/caiven-studio-ui run test:e2e   # Playwright

npm --prefix crates/caiven-port/web run check
npm --prefix crates/caiven-port/web run test:e2e:mock
npm --prefix crates/caiven-port/web run test:e2e:live   # full-stack, needs live DB
```

CI (`.github/workflows/rust.yml`) runs build+test+e2e, lint (fmt+clippy),
`cargo audit` + `npm audit`, `cargo doc`, then release artifacts on tags
(`.github/workflows/platform-builds.yml`-adjacent jobs in the same file:
`machine-artifacts`, `studio-bundles`). Prefer running the narrowest relevant
check locally before pushing; the full gate is `scripts/claude/pre-commit-gate.sh`.

## Core compatibility rules

- **Public Lua API** (`caiven-vm/src/vm/api_registry.rs`, `lua_exec.rs`,
  `prelude.lua`) must not change behavior silently — see
  `.claude/rules/lua-api.md`.
- **Cartridge/project format** (`caiven-cart`) changes need explicit
  versioning and round-trip tests — see `.claude/rules/cart-format.md`.
- **VM/frame-loop** hot paths must stay allocation-conscious and
  deterministic where expected — see `.claude/rules/vm-runtime.md`.
- **Security-sensitive surfaces** (auth, sessions, cart parsing, Tauri
  commands, sandbox boundary, file/archive handling) get extra scrutiny —
  see `.claude/rules/security.md`.

## Required workflow for changes

1. Classify the task (feature / bug / Lua API / UI / performance / security
   / release) and use the smallest matching skill/tool set — see
   `docs/development/claude-code-workflow.md`.
2. Read the relevant `.claude/rules/*.md` for the paths you're touching (they
   load automatically by path, but re-read when in doubt).
3. Explore the current implementation before writing code.
4. Write or update focused tests near the changed behavior.
5. Implement the smallest coherent change; avoid unrelated refactors.
6. Run the targeted check script under `scripts/claude/` for what you
   touched, not the full gate, unless finishing the task.
7. Get independent review (`caiven-review` skill / Code Review plugin) for
   anything beyond a trivial change.
8. Update docs and, if a bug revealed a reusable invariant, record it
   durably (CaveKit spec or the relevant `.claude/rules/*.md`).

## Definition of done

- Builds and targeted tests pass locally for every crate/frontend touched.
- No new `unwrap`/`expect`/panic/unchecked indexing on a production path.
- Public API or format changes include tests, docs, and (if Lua-facing)
  Studio autocomplete + an example cartridge.
- No unrelated formatting/refactor noise in the diff.
- Docs updated where behavior changed.

## Git and safety rules

- Commit message format: `type(scope): summary` subject line, blank line,
  then a flat bullet list (`- ...`), one line per bullet, no blank lines
  between bullets, no trailing watermark/co-author line unless the user
  asks for one. Match existing history style (e.g. `b64eebd`).
- Never push, merge, or open PRs without explicit approval.
- Never force-push, `reset --hard`, or discard uncommitted work without
  checking `git status` first and confirming.
- Never commit secrets; `.env` / `.env.example` stay out of prompts and logs.
- Create new commits rather than amending, unless told otherwise.
- Treat every plugin, MCP server, hook, or script as executable code — read
  it before trusting it (see `.claude/PLUGIN_STACK.md`).

## Where to look next

- Path-scoped rules: `.claude/rules/` (rust, vm-runtime, lua-api,
  cart-format, studio-tauri, studio-ui, port-backend, port-web, testing,
  security, performance, documentation, release).
- Project skills: `.claude/skills/caiven-*` — see
  `docs/development/claude-code-workflow.md` for when to invoke each.
- Repository audit: `docs/development/claude-code-audit.md`.
- Product loop: `docs/product/product-development-loop.md`.
- Nested `CLAUDE.md` files (e.g. `crates/caiven-studio/CLAUDE.md`) hold
  crate-specific operational detail — Claude Code loads these automatically
  when working in that directory.

When you discover a repeatable lesson (a bug class, a gotcha in a build
step, a compatibility trap), write it into the relevant scoped rule file or
CaveKit spec instead of letting it live only in conversation history.
