---
name: caiven-studio-flow
description: Design, implement, and test an end-to-end creator workflow in Caiven Studio (or Caiven Port's web UI). Use for UI/creator-workflow work spanning the Tauri backend and Svelte frontend, not for isolated component tweaks.
---

# caiven-studio-flow

## Explore

Use Frontend Design or Playground (if installed — see
`.claude/PLUGIN_STACK.md`) during exploration of a new/changed workflow
shape before committing to an implementation.

## Implement

- Studio Rust IPC surface lives almost entirely in
  `crates/caiven-studio/src/tauri_app.rs` — treat new `#[tauri::command]`s
  as a contract with the frontend (`.claude/rules/studio-tauri.md`).
- Frontend lives in `crates/caiven-studio-ui` (or `crates/caiven-port/web`
  for Port-side flows) — follow `.claude/rules/studio-ui.md` /
  `.claude/rules/port-web.md` for state coverage (loading/empty/error/
  disabled) and keyboard-workflow preservation.
- Preserve state across hot-reload where the existing undo/redo
  (`crates/caiven-studio-ui/src/lib/history.ts`) and hot-reload
  (`caiven-studio/src/tauri_app.rs::hot_reload`) mechanisms already do —
  don't regress state-preservation for a workflow change.

## Verify

- Use Playwright for deterministic verification — add/update specs in
  `crates/caiven-studio-ui/e2e` or `crates/caiven-port/web/e2e` matching
  the mock/live split already in place.
- Use Chrome DevTools MCP (if installed) for live diagnosis of a rendering
  or interaction bug during development, not as a substitute for a
  Playwright regression test.
- Before calling the dev loop live for manual testing, check
  `crates/caiven-studio/CLAUDE.md` for the `tauri-automation` MCP
  prerequisites (Vite dev server + `tauri-wd` must both be running first).

## Definition of done for a studio-flow change

Keyboard workflow intact, all four UI states covered, `check:ui` clean if
shared components touched, Playwright coverage added/updated, no unrelated
visual redesign.
