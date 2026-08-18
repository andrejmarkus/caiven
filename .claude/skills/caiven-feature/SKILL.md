---
name: caiven-feature
description: Take one already-approved Caiven feature from understanding through implementation, using the full required sequence (exploration, boundaries, non-goals, acceptance criteria, tests, implementation, checks, review, docs, risk report). Use once a feature is approved — not for open-ended ideation (use caiven-idea) and not for bugs (use caiven-debug).
---

# caiven-feature

Required sequence — don't skip or reorder steps:

1. Read the relevant request/requirements and any `.claude/rules/*.md`
   scoped to the paths you expect to touch.
2. Explore the current implementation for real (don't assume from the
   request alone — inspect the actual code).
3. Identify affected subsystem boundaries (which crates/frontends, and
   whether it crosses the Lua API / cart-format / security-sensitive lines
   in `.claude/rules/`).
4. Define non-goals explicitly — what this change deliberately does not do.
5. Define acceptance criteria before writing implementation code.
6. Write or update tests first where practical (or immediately alongside).
7. Implement the smallest coherent change — no unrelated refactors.
8. Run targeted checks (`scripts/claude/check-*.sh` matching what changed),
   not the full gate, until the final pass.
9. Get independent review — `caiven-review` skill / Code Review plugin.
10. Update documentation (README, `docs/`, Studio autocomplete if Lua-facing).
11. Report remaining risks explicitly — don't imply "done" if something is
    known-incomplete.

## When to pull in other skills/plugins

- Touches public Lua API → also use `caiven-lua-api`.
- Touches cart/project serialization → also use `caiven-cart-compat`.
- Touches Studio/Port UI → also use `caiven-studio-flow`.
- Touches auth/sessions/tokens/uploads/sandbox → follow
  `.claude/rules/security.md` and get Security Guidance / Claude Security
  involved before calling it done.
