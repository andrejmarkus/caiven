# Caiven — Claude Code instructions

Caiven is a fantasy console built from a Rust/Lua VM, a Tauri 2 + Svelte 5
Studio, and an optional cart-sharing server. Detailed architecture is in
`docs/development/claude-code-audit.md`; do not load that document unless the
task needs a broad architecture review.

## Working rules

- Inspect the current implementation before editing; do not rely on specs or
  memory alone.
- Make the smallest coherent change. Avoid unrelated refactors and formatting
  noise.
- Add or update focused tests near changed behavior, then run the narrowest
  matching check under `scripts/claude/`.
- Treat public Lua APIs, cartridge formats, auth/session code, Tauri commands,
  file handling, and the Lua sandbox as compatibility or security boundaries.
- Do not introduce `unwrap`, `expect`, panic, or unchecked indexing on a
  production path.
- Keep generated files, secrets, `.env` content, and large command output out
  of prompts unless directly needed.

## Repository map

- `crates/caiven-core`, `caiven-cart`, `caiven-vm` — shared types, formats,
  runtime, rendering, input, and audio.
- `crates/caiven-machine`, `caiven-web` — native and browser players.
- `crates/caiven-studio`, `caiven-studio-ui` — Tauri backend and Svelte UI.
- `crates/caiven-port`, `crates/caiven-port/web`, `crates/migration` — sharing
  server, frontend, and database migrations.
- `crates/caiven-ui` — shared Svelte component library.

Path-scoped rules under `.claude/rules/` load when matching files are read.
Do not pre-read unrelated rule files.

## Checks

Prefer one targeted script while implementing:

- Rust: `scripts/claude/check-rust.sh`
- Studio UI: `scripts/claude/check-studio-ui.sh`
- Port web: `scripts/claude/check-port-web.sh`
- Lua API: `scripts/claude/check-lua-api.sh`
- Cart compatibility: `scripts/claude/check-cart-compat.sh`
- Final full pass only: `scripts/claude/pre-commit-gate.sh`

## Context discipline

The checked-in default is intentionally lean: project LSP and browser plugins
are disabled, and `caiven-*` skills are manual commands. Start normally for
most work, or use one focused profile:

```bash
scripts/claude-session.sh rust
scripts/claude-session.sh typescript
scripts/claude-session.sh lua
scripts/claude-session.sh ui-test
scripts/claude-session.sh ui-debug
```

Use `/caiven-feature`, `/caiven-debug`, `/caiven-review`, and other project
skills only when their workflow is needed. Do not stack several workflow
skills by default. Use `/context` to inspect startup cost and `/clear` when
switching to an unrelated task.

## Git and completion

- Commit format: `type(scope): summary`, blank line, then flat `- ...` bullets.
- Never push, open a pull request, merge, force-push, amend, reset hard, or
  discard work without explicit user approval.
- Before declaring completion, run the targeted checks for every area touched
  and report anything not verified.

See `docs/development/claude-code-workflow.md` for task-to-profile and
skill selection. See `.claude/PLUGIN_STACK.md` only when changing tooling.
