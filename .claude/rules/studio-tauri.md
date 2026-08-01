---
paths:
  - "crates/caiven-studio/src/**"
  - "crates/caiven-studio/capabilities/**"
  - "crates/caiven-studio/gen/**"
---

# Studio Rust backend (Tauri)

- Tauri commands are the IPC surface exposed to the Svelte frontend — treat
  every new/changed `#[tauri::command]` as part of a public contract with
  the frontend and as a security boundary (see `.claude/rules/security.md`,
  "Tauri commands"): validate paths and inputs, don't trust the frontend
  blindly even though it's first-party.
- `capabilities/` and `gen/schemas/` define what the frontend is allowed to
  call — keep these in sync with actual command signatures; don't grant
  broader capability than a feature needs.
- Hot-reload, undo/redo, and debugger state (`.cavdbg` sidecar) are
  state-preserving by design (see recent git history) — don't reintroduce
  full-reload-loses-state regressions when touching this path.
- See `crates/caiven-studio/CLAUDE.md` for the local dev loop
  (`tauri-automation` MCP, `tauri-wd`, Vite dev server dependency) before
  attempting to drive Studio via MCP tools.
- Debug builds only: don't assume `tauri-plugin-webdriver-automation`
  behavior applies to release builds.
