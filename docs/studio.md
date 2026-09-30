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

The sprite and map canvases (Art → Sprites, Art → Map) are fully keyboard-
operable once focused: arrow keys move a cell cursor, Enter or Space paints
(pencil/erase/fill/autotile) or anchors and commits a stroke (line/rect/
rectangle outline/select — press again to commit, matching a mouse
drag-release), and Escape cancels an in-progress keyboard stroke without
committing it.

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

## Port accounts

Studio binds saved credentials to the Port server where you linked your account.
Changing the server requires linking there before publishing. Older saved tokens
without a server identity require one fresh link after upgrading. On Unix, token
files are readable and writable only by your user.

`CAIVEN_PORT_API_KEY` applies to `CAIVEN_PORT_URL` (or localhost when that URL is
unset); it is not forwarded to a different server selected in Studio.

## Debugger

A breakpoint pauses the frame at that line, and Run or Step continues it from
there, so no code runs twice. Top-level code and `_init()` stop too: the first
Run after opening a cart starts it fresh under the debugger. A breakpoint on a
line without code (blank, comment, `end`, a function header) stops at the next
line that runs; on a function header that is the first line of its body.
Breakpoints don't stop inside coroutines, inside callbacks that C code calls
(`table.sort` comparators) or in a module's top-level code, which `require`
runs.

When the game is paused, you can step one line at a time. The editor marks the line where the game stopped.

| Key         | Step                                        |
| :---------- | :------------------------------------------ |
| `F10`       | Over: next line here or in the caller       |
| `F11`       | Into: next line, entering calls             |
| `Shift+F11` | Out: next line in the caller                |

When a step reaches the end of a callback, it stops at the first line of the next callback, even if that is in the next frame. Step skips the same code that breakpoints skip.
In the Call stack tab, click a call to see its locals. Watches and hover values then read that call too.
While the game is paused, hover over a name in the editor to see its value.

Watches take a name with `.field` and `[index]` steps (`enemies[1].hp`). At a
breakpoint the name resolves like the stopped code sees it: its locals first,
then globals and file-scope locals. Watches never call `__index` or any other
cart code. Expanded tables stay open and refresh after every frame or step.

## Publishing

Studio remembers the Port cart each project last published to, per server.
Publishing again adds a version to that cart, keeping its ratings and
downloads together; tick "Publish as a new cart" in the dialog to fork instead.
`caiven-studio publish --cart-id <id>` does the same from the command line.
