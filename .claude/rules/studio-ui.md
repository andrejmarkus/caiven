---
paths:
  - "crates/caiven-studio-ui/**"
  - "crates/caiven-ui/**"
---

# Studio and shared UI (Svelte frontend)

- Preserve keyboard workflows (command palette, editor shortcuts, gutter
  breakpoints) — these are core to the creator experience, don't regress
  them for a visual change.
- Every new interactive view needs loading, empty, error, and disabled
  states — not just the happy path.
- Check accessibility (focus order, labels, contrast) even though no
  dedicated a11y-scanning plugin is installed (see `.claude/PLUGIN_STACK.md`)
  — do it manually as part of review.
- Avoid generic visual redesigns unrelated to the task at hand.
- Add or update Playwright coverage (`crates/caiven-studio-ui/e2e`,
  `npm run test:e2e`) for creator-facing flow changes.
- `crates/caiven-ui` is shared with `caiven-port/web` — run
  `npm run check:ui` after touching shared components, and don't fork a
  component locally instead of updating the shared one.
- Run `npm run check` (svelte-check + tsc) before considering a UI change
  done.
