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

- Commit message format: `type(scope): summary` subject line, blank line,
  then a flat bullet list (`- ...`), one line per bullet, no blank lines
  between bullets, no trailing watermark/co-author line unless the user
  asks for one. Match existing history style (e.g. `b64eebd`).
- Never cite SPEC.md ids (`T*`, `V*`, `B*`, `§X`) in a commit message — git
  readers don't have the spec open. Say what changed and why in plain terms;
  the spec tracks its own ids.
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
