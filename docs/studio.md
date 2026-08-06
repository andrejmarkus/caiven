# Caiven Studio

Studio uses a native Tauri shell with a Svelte UI. A Rust actor thread owns
the VM and audio; the webview receives framebuffer snapshots and sends typed
project, input, transport, sprite, and palette commands.

Press function keys to switch workspaces:

| Key  | Workspace             |
| :--- | :--------------------- |
| `F1` | Code                  |
| `F2` | Art → Sprites         |
| `F3` | Art → Map             |
| `F4` | Sound → Sound effects |
| `F5` | Sound → Music         |
| `F6` | Art → Palette         |
| `F7` | Cart details          |
| `F8` | Library               |
| `F9` | API docs              |

`Cmd/Ctrl+S` saves, `Cmd/Ctrl+R` runs or pauses, and `Cmd/Ctrl+K`
opens the command palette. The console stays visible at 4× integer scale on
wide windows and 3× at the minimum supported 1280×800 size. The bottom drawer
holds Problems, Output, and Memory. Focus mode expands the framebuffer
without moving the VM into JavaScript.

Run native Studio with live Vite reload:

```bash
npm --prefix crates/caiven-studio-ui ci
cd crates/caiven-studio
npm --prefix ../caiven-studio-ui exec tauri dev
```

Build a native installer for the current OS:

```bash
cd crates/caiven-studio
npm --prefix ../caiven-studio-ui exec tauri build
```

Bundles land under `target/release/bundle/`. For UI-only work, run
`npm --prefix crates/caiven-studio-ui run dev` from the repository root.

Browser preview uses representative data; Tauri launch supplies live VM,
filesystem, input, API-registry, sprite, and palette state.
