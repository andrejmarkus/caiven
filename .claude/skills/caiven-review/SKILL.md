---
name: caiven-review
description: Run independent review of a completed Caiven change (diff, branch, or file) for correctness, regressions, compatibility, security, hot-path allocations, error handling, test quality, documentation, creator experience, and unnecessary complexity. Use after implementation, before considering work done.
---

# caiven-review

Prefer the Code Review plugin as the default independent reviewer
(`.claude/PLUGIN_STACK.md`); use PR Review Toolkit instead for unusually
risky changes (cartridge format, auth, sandbox boundary); use Code
Simplifier only after correctness/tests are already established, as a
separate pass.

## Review checklist

- **Correctness** — does the change do what it claims, including edge cases?
- **Regressions** — anything that used to work and now doesn't, including
  behavior not covered by existing tests?
- **Compatibility** — public Lua API (`.claude/rules/lua-api.md`) or cart
  format (`.claude/rules/cart-format.md`) implications, even if the diff
  looks unrelated to them at a glance.
- **Security** — does it touch any surface in `.claude/rules/security.md`?
  If so, was it actually scrutinized, not just touched?
- **Hot-path allocations** — new per-frame allocation in `caiven-vm`'s
  frame loop or rendering path (`.claude/rules/vm-runtime.md`).
- **Error handling** — new `unwrap`/`expect`/panic/unchecked indexing on a
  production path (`.claude/rules/rust.md`).
- **Test quality** — tests that actually exercise the changed behavior, not
  just present for coverage theater; regression tests for bug fixes.
- **Documentation** — updated where behavior changed.
- **Creator experience** — keyboard workflows, loading/empty/error/disabled
  states for UI changes (`.claude/rules/studio-ui.md`,
  `.claude/rules/port-web.md`).
- **Unnecessary complexity** — unrelated refactors, premature abstraction,
  or scope creep beyond what the task needed.

## Output

Findings ranked by severity, each with file:line and a concrete failure
scenario — not vague concern. If nothing survives scrutiny, say so plainly
rather than manufacturing a finding.
