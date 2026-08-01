---
paths:
  - "crates/caiven-port/web/**"
---

# Caiven Port web frontend

- Preserve keyboard workflows and add loading/empty/error/disabled states
  for creator-facing views, same bar as `.claude/rules/studio-ui.md`.
- This frontend shares `crates/caiven-ui` — run `npm run check:ui` after
  touching shared components.
- Test matrix already distinguishes mocked vs live e2e
  (`test:e2e:mock` / `test:e2e:live`, see `playwright.config.ts` vs
  `playwright.live.config.ts`). Add coverage in the mocked suite by default;
  only add to the live suite when the behavior genuinely requires a real
  backend/DB.
- Auth-adjacent UI (login, session, publish flow) is security-sensitive —
  coordinate with `.claude/rules/port-backend.md` and
  `.claude/rules/security.md` when touching it.
