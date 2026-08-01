---
name: caiven-status
description: Summarize current Caiven repository state — active work, CI/test health, architecture drift, and the next highest-leverage actions. Use at the start of a session or when asked "what's the state of things" / "what should we work on next".
---

# caiven-status

Produce a repository-state summary. Do not start implementing anything from
this skill alone — it's diagnostic, not action-taking.

## Steps

1. `git status --short` and `git log --oneline -15` — what's in flight,
   what merged recently, any stale branches worth noting.
2. Check CI health: read `.github/workflows/rust.yml` job list if unsure
   what gates exist; if GitHub plugin/MCP is available, check latest run
   status. Otherwise note "CI status not queryable this session."
3. Run the narrowest reasonable local health check —
   `scripts/claude/check-rust.sh` and, if frontend files changed recently,
   `scripts/claude/check-studio-ui.sh` / `check-port-web.sh` — rather than
   the full gate, unless the user wants a deep check.
4. Compare current crate/module boundaries against
   `docs/development/claude-code-audit.md` — flag anything that looks like
   drift (new crate, moved responsibility, a rule file pointing at a path
   that no longer matches reality).
5. Look for weak spots already known (see audit doc's "Gaps" section —
   `caiven-web`/`migration` test coverage, cart format version handling,
   `tauri_app.rs` size) and note if recent commits touched them.

## Output

- Current branch + uncommitted state.
- CI/test health (or "not checked this session" if unavailable).
- Architecture drift, if any.
- Five highest-leverage next actions, each with: expected impact, rough
  implementation scope, main risk, and which `caiven-*` skill would drive it.
- Do not propose starting a large feature automatically — surface options,
  let the user pick.
