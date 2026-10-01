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
- Mock e2e: `route.fulfill({ status: 204 })` with no body is reported by
  Chromium as `requestfailed … net::ERR_ABORTED`, which trips the browser
  guard. New fire-and-forget mock routes answer 200 with an empty body.
- Canvas pixel assertions that poll must read through a copy canvas made
  with `willReadFrequently`, or Chrome logs a readback warning the guard
  flags (see `e2e/mock/remix.spec.ts`).
- Pixel polls like `expect.poll(pixel).not.toEqual(before)` pass on any
  transient frame, and UI text from the previous step may still be visible.
  Re-baseline per step or poll toward a known value, and treat an alpha-0
  read as "no picture", never as the changed game (live CI flake, 2026-09).
- `e2e/support/mock-api.ts` matches routes with escaped regexes
  (`\/api\/v1\/carts`); an API path rename must grep for that form too, or
  mocked routes silently stop matching.
- The remix editor is CodeMirror (contenteditable): read it with
  `editorText` and set it with `fillEditor` from `e2e/support/editor.ts`,
  never `toHaveValue`/`fill`. Under `mobile-chromium` (Android UA)
  CodeMirror replays Enter without modifiers, so Ctrl+Enter inserts a
  newline there; click Run instead.
- Its completion data is `@caiven/ui/lua-api.json`, generated from
  `api_registry`; `caiven-vm/tests/lua_api_json_sync.rs` fails when stale
  and its header has the regenerate command.
- SPA `use:link` routes through `navigate()`, which handles `#hash` via
  `scrollToHash` after render. A page whose anchors need ids built at mount
  (e.g. `LegalPage`) must call `scrollToHash(location.hash)` itself for fresh
  loads. To test scrolling, click with `dispatchEvent('click')`: Playwright's
  `click()` scrolls the target into view first and hides the bug.
- `.container-page` / `.container-narrow` (caiven-ui `theme.css`) carry
  `flex: 1` in the utilities layer, after Tailwind's own, so `flex-none`
  can't override it. Don't put them on a flex child that must keep its size
  (e.g. the AppShell footer); use `mx-auto w-full max-w-* px-6` instead.
  Check layout positions on the text's top edge: flex items stretch, so a
  box's bottom can look right while its text sits mid-page.
