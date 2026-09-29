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
- Cross-platform (`src/lib/format.ts`): shortcut labels go through
  `shortcut('⌘K')` (⌘ on Mac, `Ctrl+K` elsewhere), paths through
  `fileName`/`tidyPath` (both separators), dialog default names through
  `safeFileName`. Game input uses `event.code` (physical key, the same names
  as controls.toml) with defaults matching Machine and the Port player.

## Verifying Studio's live behavior (CDP/Playwright/manual)

The debug binary only loads from Vite's `devUrl` when launched through the
real dev-mode supervisor (`npm run tauri dev` / `cargo tauri dev`, per
`crates/caiven-studio/CLAUDE.md`). Launching `caiven-studio.exe` directly —
even a freshly-built debug binary — silently falls back to whatever
`crates/caiven-studio-ui/dist` last built, and WebView2 will happily serve a
_cached_ copy of an old `devUrl` response if the dev server isn't actually
running when the window loads. Either looks like a normal, working app.

`caiven-studio/build.rs` now rebuilds `dist` (`npm run build`) whenever it is
missing or older than the UI sources and the `custom-protocol` feature is on,
so a plain `cargo build`/`run` can't embed a stale bundle any more (a stale
128×128 bundle once showed the demo placeholder instead of the game). Inside
the app, a frame of the wrong size shows an error, never the demo scene.
To inspect a running Studio, launch it with
`WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9333` and
attach Playwright with `chromium.connectOverCDP`.

Before trusting any DOM-level finding (a bug report, a "this is fixed"
verification, a screenshot) against a running Studio instance: confirm
`netstat` shows something actually `LISTENING` on `:1420`, not just that the
app opened and rendered something.
