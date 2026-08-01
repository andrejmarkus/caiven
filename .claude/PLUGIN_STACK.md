# Claude Code plugin stack for Caiven

Verified against `claude plugin list`, `~/.claude/plugins/installed_plugins.json`,
and the live `claude-plugins-official` marketplace catalog
(`~/.claude/plugins/marketplaces/claude-plugins-official/.claude-plugin/marketplace.json`)
on 2026-08-01. All currently-installed plugins are **user scope** (apply to every
project on this machine), installed before this setup. Nothing here was assumed
from the setup prompt without checking the actual catalog first — several
candidates named in the prompt do not exist under that name and are marked
`unavailable` below.

Re-verify with `claude plugin list` / `claude plugin marketplace list` if this
file grows stale.

## Core development

| Candidate | Identifier | Status | Reason |
|---|---|---|---|
| Superpowers | `superpowers@claude-plugins-official` | **installed** (user, v6.2.0) | Primary implementation methodology per orchestration rules. Brainstorming + subagent-driven dev + built-in review. |
| Feature Dev | `feature-dev@claude-plugins-official` | deferred | Exists in catalog, not installed. Overlaps Superpowers as a full workflow; per orchestration rule only run one primary methodology. Use only for a workflow shape Superpowers doesn't cover. |
| Code Review | `code-review@claude-plugins-official` | **installed** (user) | Default independent reviewer (`/code-review`), used by `caiven-review` skill. |
| PR Review Toolkit | `pr-review-toolkit@claude-plugins-official` | deferred | Confirmed exists in catalog, not installed. Reserve for unusually risky PRs (cartridge format changes, auth, sandbox boundary) per orchestration rule 8 — install on demand with `claude plugin install pr-review-toolkit -s project`. |
| Code Simplifier | `code-simplifier@claude-plugins-official` | **installed** (user, v1.0.0) | Use only after correctness/tests land, per orchestration rule 9. |
| Security Guidance | `security-guidance@claude-plugins-official` | **installed** (user, v2.0.6) | Pattern-based warnings during implementation of security-sensitive changes (auth, sessions, cart parsing, Tauri commands, sandbox). |
| Claude Security | `claude-security@claude-plugins-official` | deferred | Confirmed exists in catalog ("Deep vulnerability scanning... entirely inside your Claude Code session"). Overlaps Security Guidance. Use as the stronger reviewer immediately before completing a security-sensitive change per Phase 7 policy; don't run both on every diff. |
| Hookify | `hookify@claude-plugins-official` | deferred | Exists in catalog ("create custom hooks to prevent unwanted behaviors"). Repo's own hooks are handwritten in `scripts/claude/` + `.claude/settings.json` instead, since they need Caiven-specific logic (Rust/Lua/cart paths) that a generic hook generator wouldn't know without those specifics anyway. Revisit if hook authoring becomes frequent. |
| CLAUDE.md Management | `claude-md-management@claude-plugins-official` | **installed** (user, v1.0.0) | Audits/improves CLAUDE.md quality; used after "discovering repeatable lessons" per root CLAUDE.md instruction. |
| Context7 | `context7@claude-plugins-official` | **installed** (user) | Current external library docs (Tauri 2, Svelte 5, sea-orm, mlua ecosystem, webauthn-rs). |
| GitHub | `github@claude-plugins-official` | deferred | Confirmed exists (official GitHub MCP server). Not installed: no explicit ask to wire up issue/PR automation yet, and it requires GitHub auth/token setup. Install with `claude plugin install github -s project` when the user wants issue/PR reads wired in (Phase 8 policy already assumes read-only usage once available). |
| Rust Analyzer LSP | `rust-analyzer-lsp@claude-plugins-official` | deferred, **recommended** | Confirmed exists. Not installed this session — requires `rust-analyzer` binary on PATH, not auto-installed without permission. See `scripts/setup-claude-code.sh`. |
| TypeScript LSP | `typescript-lsp@claude-plugins-official` | deferred, **recommended** | Confirmed exists. For `caiven-studio-ui` and `caiven-port/web` Svelte/TS code. Needs `typescript-language-server`; see setup script. |
| Lua LSP | `lua-lsp@claude-plugins-official` | deferred, **recommended** | Confirmed exists. For Lua stdlib/example-cart authoring and `caiven-lua-api` work. Needs `lua-language-server`; see setup script. |

## UI and testing

| Candidate | Identifier | Status | Reason |
|---|---|---|---|
| Playwright | `playwright@claude-plugins-official` | deferred, **recommended** | Confirmed exists (Microsoft Playwright MCP). Repo already runs Playwright via npm scripts (`test:e2e`, `test:e2e:stress`, `test:e2e:live` in both `caiven-studio-ui` and `caiven-port/web`) and CI. The MCP plugin adds live browser control for `caiven-studio-flow`; install via setup script since it needs the Playwright browser binaries. |
| Chrome DevTools | `chrome-devtools-mcp@claude-plugins-official` | deferred, **recommended** | Confirmed exists as `chrome-devtools-mcp` (not `chrome-devtools`). For live diagnosis per Phase 7 UI-workflow policy. Install via setup script. |
| Frontend Design | `frontend-design@claude-plugins-official` | **installed** (user) | Used during Studio/Port UI exploration per `caiven-studio-flow`. |
| Playground | `playground@claude-plugins-official` | deferred | Confirmed exists ("interactive HTML playgrounds"). Not installed — no session need yet; install on demand when rapid visual prototyping comes up (Phase 7 policy references it). |
| Accessibility-related tools | — | **unavailable** | No dedicated accessibility-scanning plugin exists in the official marketplace catalog under any obvious name. Accessibility checks are instead encoded as a requirement inside `.claude/rules/studio-ui.md` and `.claude/rules/port-web.md`, and as a checklist item in `caiven-studio-flow`. |

## Project specialization

| Candidate | Identifier | Status | Reason |
|---|---|---|---|
| Plugin Developer Toolkit | `plugin-dev@claude-plugins-official` | **unavailable in this session's need** | Confirmed exists in catalog as `plugin-dev`. Not installed — Caiven has no plan to author its own Claude Code plugin right now; nothing in this setup needs it. |
| Skill Creator | `skill-creator@claude-plugins-official` | **installed** (user) | Used to scaffold/refine the 11 `caiven-*` skills created in this setup. |
| Claude Code Setup | `claude-code-setup@claude-plugins-official` | **unavailable in this session's need** | Confirmed exists ("Analyze codebases and recommend tailored Claude Code automations"). Not installed — this manual setup pass supersedes what it would generate; revisit for future incremental automation suggestions. |
| CaveKit v4 | `ck@cavekit-marketplace` | **installed** (user, v4.1.0) | Confirmed lightweight v4 workflow (not the older autonomous multi-agent version — version string and marketplace source `JuliusBrussee/cavekit` checked directly). Durable specification system per orchestration rule 1. |

## Product and growth

| Candidate | Identifier | Status | Reason |
|---|---|---|---|
| Product Management | — | **unavailable** | No plugin under this or a close name exists in the official catalog. `caiven-idea` skill covers the product-ideation workflow the prompt wanted from it instead. |
| Marketing | — | **unavailable** | No plugin under this or a close name exists in the official catalog. Product positioning/growth work should use `caiven-idea` plus `docs/product/product-development-loop.md`. |

## Later-stage observability (per prompt: do not install unless already configured)

| Candidate | Identifier | Status | Reason |
|---|---|---|---|
| Sentry | `sentry@claude-plugins-official` | rejected (for now) | Exists in catalog. No Sentry project configured in this repo (no DSN, no `.env` key referencing it). Do not install until a Sentry project exists and the user decides to adopt it. |
| PostHog | `posthog@claude-plugins-official` | rejected (for now) | Exists in catalog. No PostHog config found. Ties into `docs/product/product-development-loop.md` metrics list as a future option only. |
| CodSpeed | `codspeed@claude-plugins-official` | rejected (for now) | Exists in catalog. No CodSpeed config in CI. `caiven-benchmark` skill uses local baseline/after measurement instead; revisit if continuous perf tracking in CI becomes a priority. |

## Security note on every plugin above

Per repository-rule 11 (treat all third-party plugins as executable code), every
`installed` row above was already present in `~/.claude/plugins/installed_plugins.json`
before this session touched anything — this setup did not blindly trust the
prompt's candidate list, it cross-checked against what's actually registered and
what actually exists in the marketplace catalog. Anything newly installed by
`scripts/setup-claude-code.sh` should be reviewed (`claude plugin details <name>`)
before first use in this repo, especially anything granting MCP server or hook
capability (Playwright, Chrome DevTools, GitHub).
