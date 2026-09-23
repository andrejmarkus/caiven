# Caiven — Claude Code instructions

Caiven is a fantasy console built from a Rust/Lua VM, a Tauri 2 + Svelte 5
Studio, and an optional cart-sharing server. Detailed architecture is in
`docs/development/claude-code-audit.md`; do not load that document unless the
task needs a broad architecture review.

## Design charter (binding)

Caiven's direction is settled. Treat everything in this section as a
constraint, not a preference. Changing any of it requires an explicit,
recorded decision from the user — never a judgement call mid-task.

Full record: `docs/product/design-charter.md` — read it when a decision needs
more than this summary. If the two ever disagree, the charter wins and this
section must be corrected.

**Spine — the two clocks.** Caiven is a brain gym (the user types the game
themselves) that still reaches playable in one sitting.

- **Clock A — friction** (boot, reload, editors, defaults, error messages,
  export): drive to zero. Spend engineering freely; no simplicity budget.
- **Clock B — authorship** (the game logic itself): stays hand-typed. Small
  API, no autopilot, no framework that writes the game's structure.
- **The gate, one sentence:** does this remove friction, or remove
  authorship? Friction is bought freely. Authorship is never traded for
  speed.

**The remix loop.** Caiven is a network of tiny playable programs; the loop
to optimise is play → remix → change → publish → get remixed → return.
Browser Quick Remix (`docs/product/quick-remix.md`) is Clock A; remixing is
opt-in per cart.

**Seven-point gate.** A proposed API must pass all seven:

1. Removes friction (Clock A), not authorship (Clock B).
2. Fits an API tier and satisfies that tier's rule.
3. Does not change frozen hardware.
4. Not on the no-list.
5. Visible result on first use, no setup ritual.
6. Only one obvious way — does not duplicate an existing call.
7. Explainable to a beginner in one sentence.

**Frozen hardware.** Target state; the code still carries the old numbers
until the redesign phases land. Do not propose changes to these values.

| Spec | Value |
| --- | --- |
| Screen | 192 × 128 (24 × 16 tiles) |
| Palette | 16 colors |
| Sprites | 8 × 8, 256 per bank |
| Map | 192 × 128 tiles + collision layer |
| Frame rate | 60 Hz fixed |
| Audio | 6 voices: 4 typed music (2 pulse, 1 triangle, 1 noise) + 2 sfx |
| Input | 4 directions + 2 actions + Select; START reserved |
| Save | one blob (`save_data`/`load_data`); no numeric slot API |

Growth happens only through **named banks** (`load_sprite_bank("forest")`),
unbounded in count, bounded by the 128 KiB cart cap. Banking is invisible
until needed: the default bank auto-loads.

**Permanent no-list.** No 3D. No external I/O (no net, filesystem, or
subprocess). No shaders, render targets, or blend-mode zoo. No engine
frameworks that own the game loop. No custom Lua dialect — real Lua 5.4
stays real. No telemetry or analytics SDK. **No in-product LLM** — the
console is the human-craft antidote to the AI era; this is positioning, not
an omission.

**API tiers.** T0 builtins (Rust): only what cannot be written in Lua. T1
prelude core: math-shaped, no game structure. T2 opt-in modules: the
*readable-lesson cap* — pure Lua, roughly ≤ 100 lines, source readable in
Studio, understandable in one sitting. A module is a teaching example, not a
black box.

**Deliberate non-limits, do not re-argue.** No token limit and no code-size
limit (they punish readable code). The 128 KiB cart cap is an operations
constraint protecting the Port database, not a design forcing function. Long
descriptive API names stay (`draw_line`, not `line`).

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
- Comments stay short: one line stating the non-obvious WHY, not a
  multi-paragraph story. If it needs more than ~3 lines to justify, that's a
  sign to shorten it, not a license to keep going.

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
are disabled by default, and `caiven-*` skills are manual commands. Enable a
plugin yourself with `/plugin` when a task needs one (e.g. `rust-analyzer-lsp`
for Rust work, `typescript-lsp` for Svelte/TypeScript, `lua-lsp` for Lua,
`playwright` or `chrome-devtools-mcp` for browser work), and disable it again
when done.

Use `/caiven-feature`, `/caiven-debug`, `/caiven-review`, and other project
skills only when their workflow is needed. Do not stack several workflow
skills by default. Use `/context` to inspect startup cost and `/clear` when
switching to an unrelated task.

## Git and completion

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

- Design charter: `docs/product/design-charter.md` — binding product
  direction, frozen hardware, and the seven-point API gate.
- Pending redesign: `docs/product/hardware-redesign-plan.md` — the Phase 2/3
  change list moving the code to the charter's target hardware. The frozen
  hardware table above is target state; the code still carries the old
  numbers until those phases land.
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
step, a compatibility trap), write it into the relevant scoped rule file
instead of letting it live only in conversation history.
