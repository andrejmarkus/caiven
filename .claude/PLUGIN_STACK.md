# Claude Code plugin stack for Caiven

This document records the repository policy, not the complete plugin inventory
of any developer's machine. User-scope plugins vary and should be inspected
with `/plugin` when diagnosing context cost.

## Runtime policy

All optional project integrations are installed on demand and disabled by the
checked-in `.claude/settings.json`. Start a focused session through
`scripts/claude-session.sh`; command-line settings enable only the selected
profile.

| Profile | Enabled integration(s) | Use for |
|---|---|---|
| `lean` | none | planning, docs, small edits, broad reasoning |
| `rust` | `rust-analyzer-lsp` | Rust implementation and diagnostics |
| `typescript` | `typescript-lsp` | Svelte/TypeScript implementation |
| `lua` | `lua-lsp` | Lua code and public runtime API work |
| `ui-test` | TypeScript LSP + `playwright` | repeatable browser actions and e2e |
| `ui-debug` | TypeScript LSP + `chrome-devtools-mcp` | interactive browser/runtime diagnosis |

Playwright and Chrome DevTools overlap and must not be enabled together by
default. Use `scripts/setup-claude-code.sh <profile...>` to install missing
integrations at user scope without changing the repository's lean default.

## Project workflows

The 11 `caiven-*` skills under `.claude/skills/` are project-specific and
remain directly invocable. Checked-in settings mark them user-only so their
descriptions do not occupy normal model context and Claude cannot chain them
automatically.

Use one primary workflow per task:

- `caiven-feature`, `caiven-debug`, `caiven-review`, `caiven-release`
- `caiven-studio-flow`, `caiven-lua-api`, `caiven-cart-compat`
- `caiven-benchmark`, `caiven-idea`, `caiven-game-prototype`, `caiven-status`

## Optional user-scope plugins

Developers may have plugins such as Superpowers, Code Review, Context7,
Frontend Design, Security Guidance, Code Simplifier, Skill Creator, or CaveKit
installed globally. They are not required by the repository and are not
force-enabled here. Enable only the plugin needed for the current task and
inspect `/context` after doing so.

For high-risk security or compatibility reviews, a stronger review plugin may
be enabled temporarily. Do not make broad reviewer stacks part of the default
session.

## Verification

- `/context` shows the actual startup cost and loaded tools.
- `/status` shows active settings layers.
- `/plugin` shows installed and enabled plugins.
- `/skills` shows project skills as user-only.
- `claude doctor` reports oversized memory or skill listings.

Recheck this policy after Claude Code configuration changes. Treat every
plugin, MCP server, hook, and setup script as executable code and review it
before trusting it.
