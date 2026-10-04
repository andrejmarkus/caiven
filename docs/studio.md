# Caiven Studio

Studio is a Tauri app with a Svelte UI. A Rust thread owns the VM and the
audio output. The webview gets framebuffer snapshots from that thread and
sends it typed commands for projects, input, playback, sprites and the
palette.

Function keys switch workspaces:

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

`Cmd/Ctrl+S` saves, `Cmd/Ctrl+R` runs or pauses, and `Cmd/Ctrl+K` opens the
command palette. The game screen scales 4× on wide windows and 3× at the
smallest supported size, 1280×800. The bottom drawer has Problems, Output
and Memory tabs. Focus mode enlarges the game screen; the VM keeps running
in Rust.

You can use the sprite and map canvases with the keyboard alone. Click a
canvas to focus it, then move the cell cursor with the arrow keys. Enter or
Space paints with the pencil, eraser, fill and autotile tools. The line,
rectangle, outline and select tools need two presses: the first sets the
start point and the second commits, like pressing and releasing the mouse.
Escape cancels a stroke you haven't committed.

## Running from source

Studio with live Vite reload:

```bash
npm --prefix crates/caiven-studio-ui ci
cd crates/caiven-studio
npm --prefix ../caiven-studio-ui exec tauri dev
```

A native installer for your OS:

```bash
cd crates/caiven-studio
npm --prefix ../caiven-studio-ui exec tauri build
```

The installers end up in `target/release/bundle/`. If you only work on the
UI, run `npm --prefix crates/caiven-studio-ui run dev` from the repository
root. That browser preview shows sample data; the Tauri build shows the live
VM, files, input, API list, sprites and palette.

## Port accounts

Studio ties your saved login to the Port server you linked it with. If you
switch servers, link your account there before you publish. Tokens saved by
older Studio versions have no server attached, so you need to link once
after upgrading. On Unix, only your user can read or write the token files.

`CAIVEN_PORT_API_KEY` goes to `CAIVEN_PORT_URL`, or to localhost if that
variable is unset. Studio won't send it to a different server you pick in
the UI.

## Debugger

A breakpoint pauses the frame on its line. Run or Step resumes from that
point, so no code runs twice. Breakpoints work in top-level code and in
`_init()` as well: the first Run after you open a cart restarts it under the
debugger. If you put a breakpoint on a line with no code (blank, comment,
`end` or a function header), it stops on the next line that runs. For a
function header, that's the first line of the body.

Breakpoints don't fire inside coroutines, inside callbacks that C code calls
(such as `table.sort` comparators), or in a module's top-level code, which
`require` runs.

While paused, you can step line by line. The editor highlights the line
where the game stopped.

| Key         | Step                                        |
| :---------- | :------------------------------------------ |
| `F10`       | Over: next line here or in the caller       |
| `F11`       | Into: next line, entering calls             |
| `Shift+F11` | Out: next line in the caller                |

If a step reaches the end of a callback, it stops on the first line of the
next callback, even when that falls in the next frame. Stepping skips the
same code breakpoints skip.

Click a call in the Call stack tab to see its locals. Watches and hover
values then read from that call. Hover a name in the editor while paused to
see its value.

A watch takes a name followed by `.field` and `[index]` steps, for example
`enemies[1].hp`. At a breakpoint Studio looks the name up the way the
stopped code would: its locals first, then globals and file-level locals.
Watches don't call `__index` or run any cart code. Tables you expand stay
open and refresh after each frame or step.

## Publishing

Studio remembers which Port cart each project last went to, per server.
Publishing again adds a new version to that cart, so its ratings and
download count stay in one place. To fork instead, tick "Publish as a new
cart" in the dialog. On the command line, `caiven-studio publish --cart-id
<id>` does the same thing.
