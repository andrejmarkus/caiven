# SPEC

Distilled from repo (existing behavior + architectural invariants). No redesign.
`?` = inferred, confirm before trusting.

## §G GOAL

Retro fantasy console: Rust VM embedding real Lua 5.4 (`mlua`, vendored) for game
code, in-engine editor (Caiven Studio), optional self-host cart-sharing server
(Caiven Port). Creator authors project dir (`caiven.toml` + `.lua` + 1 asset/section)
→ distributes built `.cav` binary cartridge.

## §C CONSTRAINTS

- Rust workspace, `--locked`. mlua vendored — no system Lua.
- Lua 5.4 real, full stdlib. No custom bytecode, no arity caps.
- Fantasy-console limits: 128×128 default screen (configurable `config.width/height`
  `vm/config.rs:13`; asset PNG hard 128×128 `asset_png.rs:7`), hard 16-color palette
  (`palette.rs:6` `DEFAULT_COLORS[16]`), hard 64×64 tilemap (`caiven-core/src/memory.rs:44,46`
  `MAP_W=MAP_H=64`), sprites/shape primitives/camera. 60 FPS (`lua_exec.rs:154`).
  Max cart 128 KiB (`caiven-cart/src/lib.rs:26` `MAX_CART_BYTES`).
- Studio = Tauri 2 + Svelte 5. Port = rocket + sea-orm + webauthn-rs + argon2.
- Shared UI (`caiven-ui`, shadcn-svelte) consumed by studio-ui & port/web;
  boundary enforced `npm run check:ui`.
- License MPL-2.0. Creators own games/assets, sell royalty-free, source private.
- `rustfmt.toml` @ repo root pins `style_edition = "2024"` — bare `rustfmt`
  (editor/hook) otherwise defaults to 2015 import order & reverts committed
  formatting, then `cargo fmt --check` fails. B1.
- CI gate: fmt, clippy `-D warnings -A unused-imports`, build+test, cargo audit
  (ignore RUSTSEC-2023-0071), npm audit, cargo doc.
- No new `unwrap`/`expect`/panic/unchecked-index on production path
  (`unwrap_used = "warn"` workspace lint).
- caiven-lsp: tower-lsp (or lsp-server) crate, stdio transport. api_registry.rs
  stays sole source of truth — ⊥ hand-maintained duplicate symbol list. ⊥
  hand-rolled Lua parser — use existing/off-the-shelf crate.
- `?` caiven-lsp distribution unresolved: bundled w/ Studio installer vs
  `cargo install` vs VS Code marketplace ext — product decision, park for user.
- `?` Lua parser/analysis crate for scope-aware local-var completion
  unresolved — needs /research before build (blocks T14).
- caiven-machine targets: desktop (Win/Linux/macOS) + small Linux handhelds
  (Miyoo Mini/Plus SSD202D, A30, TrimUI, Anbernic RG35XX) + later Android/iOS.
  1 binary ∀ targets — ⊥ separate Tauri desktop shell.
- Machine platform layer = SDL2 (`sdl2` crate 0.38): window+render+gamepad+audio.
  ⊥ wgpu/GLES requirement (Miyoo Mini SSD202D = dual Cortex-A7 1.2GHz, 128MB RAM,
  ⊥ GPU, GLES only via SwiftShader — R5). ⊥ SDL3 (handheld distros ship SDL2 — R6).
- Machine shell UI = CPU raster (tiny-skia + fontdue) → RGBA buf → SDL texture.
  ⊥ egui | webview | Tauri ∈ Machine.
- Console shell design = Obsidian & Ember tokens, handoff `Caiven Machine.dc.html`.
  640×480 primary, 1280×720 desktop.
- Machine console shell = 11 screens: boot, library (hero carousel), empty,
  cart detail, hand-off/loading, playing, pause, settings, controls remap,
  Port browse, crash. Spec = handoff `Caiven Machine.dc.html` (static canvas) +
  `Caiven Machine Prototype.dc.html` (behavioral). HTML = design reference,
  ⊥ production code, ⊥ port line-for-line.
- Library direction 1a (hero carousel) chosen. 1b (shelf grid) ⊥ build.
- Fonts Space Grotesk 600/700 (display) | Inter 400/500/600 (body) | JetBrains
  Mono (numerics). Bundled subset w/ binary — ⊥ fetch (Google Fonts = documented
  substitution). Icons = Lucide 1.5–2px stroke, ⊥ fill. ⊥ logo mark exists →
  wordmark ∀ mark positions.
- Type floor 10px. ⊥ photography | illustration | gradient bg | emoji.
- Cart art = existing 128×128 auto-screenshot @ 1:1, title bottom-aligned over it
  (form the mock placeholders already show). ⊥ new cart-label section this
  milestone ∴ ⊥ format bump. Revisit as own scoped change post-shell.
- Out of scope this milestone: tag/search filtering beyond sort chip, Port
  sign-in, save-state management screens, screenshot gallery, firmware update,
  multi-user profiles.
- Input set resolved: cart sees Up Down Left Right A B Select (idx 0-6).
  START = host-reserved (`SystemButton::Start`), ⊥ reach cart Lua — it opens
  pause menu = player's only exit on handheld. Strict 6-button device →
  hold B ≥600ms = START (`shell/input.rs`). Resolves earlier `?`.

## §I INTERFACES

- lua-api: builtins in `caiven-vm/src/vm/api_registry.rs` + `lua_exec.rs`; pure-Lua
  stdlib `prelude.lua`. Entry hooks: `_init()` once, `_update()` /frame, `_draw()` /frame after.
  Descriptive names (`sprite`, `draw_rect`, `button_down`, `set_palette_color`, `draw_text`).
- cart: on-disk project = `caiven.toml` + loose `.lua`/assets (diffable);
  built `.cav` binary = magic `b"CAIVEN"` + `u16` version (=3) + n_sections + 72B header.
  Owner `caiven-cart` (`format.rs`,`header.rs`,`section.rs`,`bundle.rs`,`project.rs`,`asset_png.rs`,`minify.rs`,`text.rs`).
- machine: `caiven-machine` = cart-runner CLI (`app.rs:12` about="Caiven — cart runner") — runs project dir (hot-reload) or `.cav`. Studio launch = separate binary, ⊥ machine.
- machine-platform: SDL2 owns window/render/audio/input ∈ `caiven-machine`.
  render: 1 streaming texture @ `config.width`×`config.height`, `Screen::construct`
  reused verbatim. audio: `SDL_OpenAudioDevice` AUDIO_S16SYS, honor obtained spec.
  input: `Scancode` → `Key` → `Button` | `SystemButton`; `SDL_GameController` idx 0.
  `controls.toml` additive `[gamepad]` table (`DPadUp`/`South`/`East`) +
  additive `select`/`start` fields ∈ both tables. defaults: select =
  ShiftLeft/ShiftRight | `Back`; start = Enter | `Start`.
  input bound to both start & cart button → START wins, cart binding dropped + warn.
  cli: `--fullscreen`, `--scale <fit|2x|3x>`, `--aspect <square|stretch>`.
- machine-shell: state = {screen ∈ Boot|Library|Detail|Loading|Playing|Pause|
  Settings|Controls|Port|Crash, sel (== carts.len() → PORT tile), detail_action
  ∈ {Play,Delete}, pause_index 0..6, pane/column/row, bind_index+listening,
  port_index+download, carts: Vec<CartMeta>, settings, binds}.
  chrome: status bar 30px (44 @1280×720) + legend bar 36px (52). Both absent ∀
  boot|loading|playing|pause|crash. Legend per-screen.
  persist: settings → TOML beside binary; binds → `controls.toml` (same format);
  saves → `saves/` keyed by cart id.
  CartMeta ← `.cav` header + section table (caiven-cart).
- machine-shell-raster: `caiven-machine/src/shell/` — `theme.rs` tokens,
  `font.rs` faces+glyph cache, `icon.rs` Lucide paths, `surface.rs` raster.
  `Surface::new(w,h)` picks `METRICS_640` | `METRICS_1280` (wide only @ ≥1280×720,
  else clipped chrome). draw: `clear`/`fill_rect`/`stroke_rect`/`draw_icon`/
  `draw_text(style,x,baseline_y,align,text)`/`draw_pixmap`. `rgba()` =
  premultiplied RGBA, byte order == console framebuffer (V28).
  faces = static instances /weight (fontdue ⊥ variable axes): Inter 400/500/600,
  Space Grotesk 600/700, JetBrains Mono 400/500/700. subset = ASCII + `·×…’◄►←→`.
- machine-shell-input: `caiven-machine/src/shell/input.rs` — `ShellInput`
  translates physical down/up + dt → `ShellButton` stream. ∀ buttons resolve
  on press except B: B resolves on release (tap → B) | on hold ≥ `LONG_PRESS`
  600ms (→ Start, fires once, release silent). `b_hold_progress()` → 0..1 |
  ⊥ once fired. `shell_button(Button)` / `shell_button_from_system(SystemButton)`.
- machine-shell-nav: `caiven-machine/src/shell/state.rs` — `ShellState` = whole
  graph, ⊥ SDL | raster | fs ∴ fully unit-testable. `press(ShellButton) →
  Option<Effect>`; `tick(dt)` drives boot handover (wall-clock, V35).
  Host→shell: `set_cart_count`/`set_port_count`/`cart_ready`/`cart_failed`/
  `download_finished`/`download_failed`/`bind_captured`.
  `Effect` ∈ {LoadCart(i), CancelLoad, DeleteCart(i), ResetCart, QuitToLibrary,
  SaveState, LoadState, StartDownload(i), SettingsChanged, ListenForBind(i)}.
  `legend()` → Vec<Legend{chip,label,primary,trailing}> per screen (⊥ chrome
  screens → empty, except pause draws own). `Screen::has_chrome()`.
  `shell/settings.rs` — `Pane::ALL` ×5, `Row{id,label,sub,kind}`,
  `RowKind` ∈ {Choice,Toggle,Range,Action,Readout}; `is_adjustable()` decides
  ◄► = adjust vs leave column. `Settings::adjust(id,dir) → bool changed`.
- machine-shell-port: `GET /api/v2/carts` (page, per_page, q, tag, author, sort),
  thumb `/api/v2/carts/:id/screenshot`, download `/api/v2/carts/:id/cart`.
  Download complete → append to library immediately.
- studio-cmd: Tauri `#[tauri::command]` IPC surface (studio backend ↔ Svelte);
  `capabilities/` + `gen/schemas/` gate what frontend may call.
- port-api: rocket handlers `caiven-port/src/handlers/` — auth, carts, versions,
  community, social, discovery, legacy. DB via sea-orm, schema in `crates/migration`.
- release: tag `v*` → `machine-artifacts` (Linux/Win/macOS x64+arm64) + `studio-bundles`
  (appimage/deb, nsis/msi, dmg). Version synced across Cargo.toml, `tauri.conf.json`, package.json.
- export-web: `caiven export --web <project> -o game.html` + Studio export dialog "Web (HTML)"
  → 1 self-contained `.html` (inlined: base64 packed `.cav` + base64 wasm + emscripten `caiven_web.js`
  - audio-worklet + vanilla player js). Reuses `caiven-web` WASM, ⊥ rebuild. `Module.instantiateWasm`
    hook (⊥ `wasmBinary` — this emscripten build ignores that override) skips wasm fetch; worklet via
    `Blob` URL. ⊥ external network at runtime.
- studio-debugger: `.cavdbg` sidecar toml (breakpoints+watches), model owned `caiven-studio/src/debugger.rs`.
  Runtime hookup `caiven-vm/src/vm/lua_exec.rs::run_frame_lua_bp` — `EVERY_LINE` hook aborts `_update()`
  on hit (`mlua::Error::runtime("breakpoint")`), Studio resumes by re-invoking the frame. Breakpoint hit →
  globals/RAM readable (`Vm::lua_globals`, `peek_memory`) + call stack (`capture_call_stack`) + watch
  expressions (`lua_watch`); locals ⊥ readable — mlua safe hook API has no `lua_getlocal` binding
  (comment at `lua_exec.rs:1076-1084`). "Step" (`ipc.ts::transport('step')`) = re-run `_update()` from
  top to one more line, ⊥ true mid-statement suspend/resume (mlua hook ⊥ yield while `Lua::scope`
  borrows per-frame VM state).
- lsp: `crates/caiven-lsp` bin `caiven-lsp`, stdio JSON-RPC. Completion/hover/
  signature-help sourced from `api_registry.rs`. Go-to-def scoped to
  `prelude.lua` stdlib only (line-scan of prelude.lua source — `ApiEntry`
  carries no source span, so Rust builtins have no jump target, excluded).
  Reads `caiven.toml` for project root. Bare `.lua` outside any project →
  degrade to plain-Lua stdlib-only completions, ⊥ crash/empty-error.

## §V INVARIANTS

V1: public Lua API behavior ⊥ change silently — breaking change ! explicit version/compat note.
V2: new/changed Lua builtin ! ship with impl + VM test + docs + Studio autocomplete + example cart + compat analysis + error-behavior doc.
V3: cart format change ! bump version field, backward-compat analysis, round-trip test (build→unpack→build stable), invalid-input test, migration-or-reject (⊥ silent misparse).
V4: every `.cav` = untrusted input → bounds-checked parse; truncated/corrupt/malicious ⊥ panic | OOB read → fail safe.
V5: `_update()`/`_draw()` hot path — per-frame alloc suspicious, needs reason. Perf claim ! measured (baseline before, same method after).
V6: timing/RTC/RNG deterministic where API implies — ⊥ silent timing-semantics change (`src/timing.rs`, `src/vm/rtc.rs`).
V7: audio path (`src/vm/audio.rs`, `sfx.rs`) adjacent real-time callback thread (cpal native | `SDL_AudioDevice` on Machine) → ⊥ block | unpredictable alloc.
V8: Lua sandbox — cart Lua ⊥ reach filesystem | network | process outside sanctioned API.
V9: Tauri command = security boundary — validate paths/inputs, ⊥ trust frontend; `capabilities`/`gen/schemas` ! match signatures, ⊥ over-grant.
V10: Port authorization checked per-handler, ⊥ only route/frontend layer. Uploaded `.cav` reuse `caiven-cart` parse, ⊥ ad-hoc re-parse.
V11: auth/session/token/WebAuthn ⊥ roll new crypto — use `webauthn-rs`/`argon2`. `handlers/auth.rs` security-sensitive by default.
V12: DB schema change → `crates/migration` explicit up path, reversible-or-documented; ⊥ ad-hoc SQL.
V13: Studio hot-reload/undo-redo/debugger (`.cavdbg` sidecar) state-preserving — ⊥ reintroduce full-reload-loses-state.
V14: release version consistent (Cargo.toml ∀ members, tauri.conf.json, package.json) — `release-check` job ! pass.
V15: cargo audit (ignore RUSTSEC-2023-0071) + npm audit high pass before security-adjacent | release done.
V16: tag push | workflow_dispatch release ⊥ without explicit user approval.
V17: `handlers/legacy.rs` ⊥ delete without checking dependents.
V18: Port HTTP layer = `rocket` (`caiven-port/Cargo.toml:25`) — ⊥ introduce 2nd web framework (axum/actix/warp).
V19: exported `.html` self-contained — ⊥ external fetch/CDN/network at runtime; wasm+cart+worklet
inlined. Opening `file://` runs fully offline.
V20: web player = same `caiven-vm` via `caiven-web` WASM — API parity with native (⊥ divergent subset);
sandbox V8 holds in browser (cart Lua ⊥ reach fs/net/process).
V21: cart embedded verbatim (base64 of packed `.cav`); load path = same `caiven_cart::parse` (V4
bounds-check applies to the embedded bytes).
V22: debugger step semantics ! documented accurately in Studio UI as "step to next line, re-executes
frame from top" — ⊥ label/imply true step-into/step-over/pause (see I.studio-debugger; re-run
model can double-fire non-idempotent side effects in `_update()`, that's a known limitation not a bug).
V23: local-var inspection at breakpoint, if added, ! use `Lua::exec_raw` (mlua 0.10, always-available
`unsafe fn`, ⊥ needs new Cargo feature — R2) + `mlua_sys` `lua_getstack`/`lua_getlocal` (new direct
`caiven-vm` dep — R3), called only from inside the existing `EVERY_LINE` hook in `run_frame_lua_bp`,
read-only, invoked only from Rust-side debugger code. ⊥ widen Lua sandbox (V8 still holds — path
⊥ reachable from cart Lua itself, only from the Rust hook).
V24: caiven-lsp completion/hover/signature-help ! sourced from api_registry.rs
only — ⊥ 2nd hand-maintained symbol list (drift guard: automated test diffs
LSP symbol set vs registry entry count).
V25: caiven-lsp go-to-def ⊥ builtins (ApiEntry has no source span) — prelude.lua
stdlib only.
V26: caiven-lsp on Lua file outside caiven.toml project ⊥ crash/error —
degrade to plain-Lua stdlib completions.
V27: caiven-vm ⊥ own window/GPU. `WindowGfx` deleted; winit+pixels ∉ caiven-vm deps.
Machine owns window/process lifecycle (holds vm-runtime.md boundary).
V28: Machine render = SDL streaming texture @ `PixelFormatEnum::ABGR8888` (VM buf byte
order R,G,B,A, LE — `caiven-core/src/memory.rs:32`). nearest only
(`SDL_HINT_RENDER_SCALE_QUALITY=0`) — ⊥ smooth-scale.
V29: audio backend pluggable via `AudioOut` trait + `AudioFactory`, ⊥ cargo feature
(workspace feature unification would force Studio onto Machine's backend).
`ConsoleCore::new()` signature stable ∴ caiven-studio ⊥ edits.
V30: `controls.toml` backward-compatible — ∀ existing key names ! round-trip (documented
`README.md:518-540`, files on user disks). `[gamepad]` additive only.
V31: SDL link — desktop = `bundled`+`static-link`; handheld = dynamic vs device
`libSDL2.so` (device ports carry display/input patches — R6). ⊥ bundle SDL on handheld.
V32: Machine device acceptance = launches fullscreen 640×480 + holds 60fps on Cortex-A7.
Perf claim ! measured (V5).
V33: shell raster = CPU (tiny-skia + fontdue) → RGBA buf → 2nd SDL texture.
⊥ GPU | egui | webview. Redraw only on state change — ⊥ per-frame repaint
(A7 @1.2GHz budget).
V34: design tokens defined once (`theme.rs`), referenced by name. ⊥ inline hex
@ call site. `obsidian` #3B3E48 = logo body only, ⊥ UI surface.
V35: ∀ progress bar & timed transition driven by wall-clock elapsed vs real work
— ⊥ tick count (prototype stalled under throttling).
V36: loading stage text derived from `Vm::load_cart_sections` + section table —
⊥ faked/hardcoded counts.
V37: Playing screen ⊥ HUD | frame | border. ∀ 6 buttons belong to cart; only
START (| device menu) pauses. fps readout only if Settings › Video › Show fps.
V38: pause renders over frozen last frame — blur+dim computed once @ freeze,
⊥ per-frame.
V39: crash screen content ← real `mlua` error + `frame_count()` — ⊥ synthesized
message. Load failure → crash screen w/ `anyhow` context string.
V40: controls remap writes same `controls.toml` format (V30 round-trip holds).
Values = `Key`/`PadButton` names, ⊥ raw SDL scancode ints.
V41: fonts+icons bundled ∈ binary — ⊥ network at runtime, ⊥ system font
dependency (handheld has neither).
V42: shell holds 60fps @640×480 on Cortex-A7 (V32 gate extends to shell).
Perf claim ! measured (V5).
V43: shell face = static instance /weight — ⊥ variable font (fontdue ⊥ weight
axis ∴ variable file renders default instance only; Space Grotesk default 300
∉ design). New weight → new subset file via `assets/fonts/build_fonts.py`.
V44: shell copy ⊆ bundled subset (ASCII + `·×…’◄►←→`). New char outside set →
regen subset; drift guard = `font.rs` coverage test.
V45: ◄ ► set in Mono only — Space Grotesk subset ⊥ geometric-shapes block.
V46: `Surface::rgba()` = premultiplied — SDL texture ! told same, else overlay
(pause scrim) fringes dark. Opaque menu screens identical either way.
V47: Settings & Port screens return to origin screen, ⊥ always-library
(prototype's B→library silently drops running cart when opened from pause).
`settings_return`/`port_return` hold it.
V48: settings rows ⊆ what Machine can honor. Handoff mocks brightness,
scanline, vibration, sleep timer, mute-on-sleep — ⊥ impl behind them ∴ ⊥
listed. Dead control worse than absent one. Add row when impl lands.
V49: `Playing` screen consumes only START (V37) — ∀ other buttons → cart.
Remap `listening` swallows ∀ nav presses until `bind_captured()`.
V50: library cursor range = `0..=cart_count`; index == `cart_count` → PORT
tile. `set_cart_count` re-clamps ∴ delete/download ⊥ leave dangling cursor.
V51: `Button::Select` = idx 6, additive — idx 0-5 ⊥ move ∀ (carts on disk
hard-code them). idx ∉ 0..6 → `false`, ⊥ error. Parity: native + Studio
preview + web player all map it (V20).
V52: START ⊥ ∈ `Button` ∴ ⊥ reachable from cart Lua ∀ paths. `SystemButton`
separate map ∈ `InputMap`. Cart ⊥ able to hold pause menu hostage.
V53: ∀ device → shell reachable: no physical START → hold B ≥600ms. ⊥ ship
screen whose only exit needs a button the device may lack.

## §R RESEARCH

id|finding|source
R1|mlua 0.10.5 `Debug` (hook payload) exposes event/names/source/curr_line/is_tail_call/stack only — ⊥ locals accessor|docs.rs/mlua/0.10.5/mlua/struct.Debug.html
R2|mlua has ⊥ separate "unsafe" cargo feature gating raw-state access. `Lua::exec_raw<R>(args\, \|state: *mut lua_State\| ...)` = inherently `unsafe fn`, ⊥ feature-gated, callable today w/ current `Cargo.toml:27` features (lua54,vendored)|docs.rs/mlua/0.10.5/mlua/struct.Lua.html#method.exec_raw
R3|`mlua_sys` 0.6.8 `lua54::lua` module exposes raw `lua_getlocal`/`lua_getstack` C bindings — mechanism exists in principle. Currently transitive-only dep (`Cargo.lock:4153-4157`), ⊥ direct `caiven-vm` dep yet. Exact fn signatures unconfirmed by doc fetch|docs.rs/mlua_sys/0.6.8, Cargo.lock:4153-4157
R4|`?` unresolved: is `exec_raw` safe to call reentrantly from inside an already-active `lua.set_hook` callback on the same `Lua` instance — mlua docs say instance "remains locked during execution," could mean reentrancy guard errors/panics if nested. Docs alone ⊥ settle this, needs a throwaway spike|docs.rs/mlua/0.10.5/mlua/struct.Lua.html#method.exec_raw
R5|Miyoo Mini/Plus = SigmaStar SSD202D, dual Cortex-A7 1.2GHz, 128MB DDR3, 640×480 IPS, ⊥ GPU (2D blitter only)|retrogamingbanter.com/miyoo-mini-plus-guide/
R6|Handheld SDL2 = device-patched ports carrying display+input code, ⊥ upstream. GLES only via SwiftShader (software)|github.com/steward-fu/sdl2, github.com/OOPay/sdl2, github.com/XK9274/sdl2_miyoo
R7|PICO-8 runs on Miyoo via Raspberry Pi ARM binary = SDL2 ∴ SDL2 = the portability layer|lexaloffle.com/bbs/?tid=53599
R8|`sdl2` crate: `bundled` feat builds SDL from src (needs cc/cmake), `static-link` links it in. Works any arch|github.com/Rust-SDL2/rust-sdl2
R9|winit ⊥ fbdev/KMS backend & softbuffer ⊥ DRM backend ∴ winit+pixels stack desktop-only by construction, ⊥ by config|repo exploration + crate docs
R10|fontdue 0.9.4 rasterizes file's default instance only — ⊥ variable-axis API ∴ static instances required|docs.rs/fontdue/0.9.4
R11|`fonttools` `varLib.instancer` pins axes + `subset` trims charset → 8 faces, 121 KB total (Inter 19.1K ×3, Space Grotesk 16.1K ×2, JetBrains Mono 10.4K ×3)|local build, `assets/fonts/build_fonts.py`

## §T TASKS

id|status|task|cites
T1|x|distill initial SPEC.md from repo|-
T2|x|extract framework-agnostic vanilla player js from test.html + player.ts AudioEngine (configurable wasm/worklet source, instantiateWasm hook)|V20
T3|x|web-export builder: template include_str! + inline base64 wasm/cart + worklet + player js|V19,V21,I.export-web
T4|x|CLI \`export --web\` subcommand in cli.rs|I.export-web
T5|x|Studio studio_export_web Tauri cmd + export dialog "Web (HTML)" option|V9,I.studio-cmd
T6|x|verify exported html runs offline (file://, network-blocked)|V19,V20
T7|x|add `mlua_sys` as direct `caiven-vm` dep; spike: call `Lua::exec_raw` from inside `run_frame_lua_bp`'s existing `EVERY_LINE` hook, confirm ⊥ panic/deadlock (resolves R4) — gates T8|R4,V23
T8|x|implement local-var inspection at breakpoint via `Lua::exec_raw` + `mlua_sys` `lua_getstack`/`lua_getlocal`, read-only, scoped to hook callback|V23,I.studio-debugger,T7
T9|x|VM tests: (a) breakpoint hit exposes correct locals for nested-scope script (shadowed var, loop var, upvalue), (b) cart Lua ⊥ trigger/influence the locals-FFI path outside Studio's debugger-attached state|T8,V8,V23
T10|x|new Tauri cmd (e.g. `studio_debug_locals`) + capabilities/`gen/schemas` regen for locals surface|T8,V9,I.studio-cmd
T11|x|expose locals in Studio debug UI alongside existing watches/call-stack (ConsolePane.svelte)|T10,I.studio-cmd
T12|x|audit README/docs for step-semantics claims (`ConsolePane.svelte:129` already accurate — "Step a frame at a time"; no mislabel found in checked surface, this is a repo-wide sweep not a fix)|V22
T13|x|Rust unit tests for debugger.rs + tauri_app.rs breakpoint/step/locals IPC path (audit-flagged: no direct tests today)|T8,T10,T12
T14|.|research: Lua parser/analysis crate for scope-aware local-var completion|?
T15|.|scaffold crates/caiven-lsp bin crate, tower-lsp stdio skeleton|V24,I.lsp
T16|.|impl completion+hover+signature-help from api_registry.rs|V24,I.lsp,T15
T17|.|impl go-to-def for prelude.lua stdlib (line-scan)|V25,I.lsp,T16
T18|.|impl caiven.toml project-root detection + bare-.lua degrade path|V26,I.lsp,T15
T19|.|automated test: LSP symbol set vs api_registry.rs entry count, no drift|V24,T16
T20|.|manual verify: VS Code + generic lua-language-server → draw_rect completion/signature matches Studio autocomplete|I.lsp,T16
T21|x|portable `Key` enum `caiven-vm/src/input/key.rs`; re-key InputMap off `winit::KeyCode`; drop native cfg from `input/mod.rs`|V30,V27
T22|x|additive `[gamepad]` table ∈ controls.toml schema + parse|V30,I.machine-platform
T23|x|`AudioOut` trait + `AudioFactory` ∈ `vm/audio.rs`; ConsoleCore boxed audio + factory; `new()` sig unchanged|V29,V7
T24|x|delete `WindowGfx`; drop winit+pixels from caiven-vm; `native = ["dep:cpal"]`|V27
T25|x|`caiven-machine` platform/window.rs — SDL window+renderer+streaming ABGR8888 texture, nearest|V28,I.machine-platform
T26|x|platform/scaling.rs — pure `dst_rect(window,console,mode,aspect)` fit/2x/3x × square/stretch|V28
T27|x|platform/audio.rs — `SDL_AudioDevice` AUDIO_S16SYS impl AudioOut, honor obtained spec|V29,V7
T28|x|platform/input.rs — Scancode→Key, `SDL_GameController` open/connect/disconnect|I.machine-platform
T29|x|rewrite app.rs SDL event pump (drop ApplicationHandler); keep cart load, check_mod_manifest, Ctrl+R, frame_steps timestep; add --fullscreen/--scale/--aspect|V27,I.machine
T30|x|SDL link config: desktop bundled+static default, `sdl2-dynamic` feat for handheld; document cross-build|V31
T31|.|device verify: cross-build handheld, run on Miyoo, confirm fullscreen 640×480 + 60fps + D-pad/A/B|V32
T32|x|decide cart art source → screenshot @ 1:1, ⊥ new label section, ⊥ format bump|§C,V3
T33|x|`theme.rs` — Obsidian & Ember tokens + type scale (640×480 & 1280×720)|V34
T34|x|bundle subset Space Grotesk/Inter/JetBrains Mono + Lucide glyphs ∈ binary|V41,V43,V44,V45,R10,R11
T35|x|CPU raster surface: tiny-skia + fontdue glyph cache → RGBA buf, dirty-flag redraw (texture upload lands w/ T44)|V33,V42,V46,I.machine-shell-raster
T36|x|shell state machine + navigation graph|I.machine-shell,I.machine-shell-nav,V47,V48,V49,V50,T35
T37|.|cart library scan: carts dir → Vec<CartMeta> from `.cav` header/section table|I.machine-shell
T38|.|library screen 1a: hero carousel + shelf + dashed PORT tile|I.machine-shell,T35,T37
T39|.|chrome: status bar (RTC clock @0x9600, cart count, battery, volume) + per-screen legend bar|I.machine-shell,T35
T40|.|boot screen: ember radial, wordmark, progress, spec line from CARGO_PKG_VERSION+VmConfig|T35
T41|.|empty state screen|T35,T38
T42|.|cart detail: screenshot 2× nearest, spec card, Play/Delete actions|T37,T35
T43|.|hand-off/loading screen w/ real stage text + wall-clock progress|V35,V36,T35
T44|.|playing fullscreen: ⊥ HUD, scaling from settings, optional fps readout|V37,T35
T45|.|pause overlay over frozen frame (blur+dim once), 6 rows incl. save/load state|V38,T35
T46|.|settings: 5 panes (Video/Audio/Controls/Port/System), immediate save to TOML|I.machine-shell,T35
T47|.|controls remap screen → writes controls.toml round-trip|V40,T46
T48|.|Port browse: search/sort, result rows, download → append to library|I.machine-shell-port,T35
T49|.|crash screen from mlua error + frame_count|V39,T35
T50|.|save states per cart under saves/ keyed by cart id|I.machine-shell,T45
T51|.|measure shell fps @640×480 on Cortex-A7-class device|V42,V32
T52|x|pin `style_edition = "2024"` in root rustfmt.toml|B1
T53|x|`Button::Select` idx 6 + `SystemButton::Start`; controls.toml `select`/`start` ∈ both tables, defaults + START-wins collision rule|V51,V52,V30
T54|x|`shell/input.rs` — hold B ≥600ms → START; Button/SystemButton → ShellButton|V53,I.machine-shell-input
T55|x|docs+parity: README API & controls tables, api_registry doc strings, Studio keymap idx 6, web-export/port/test.html key+gamepad maps|V51,V20,V2

## §B BUGS

id|date|cause|fix
B1|2026-08-03|bare `rustfmt` (editor/hook) defaults to 2015 style edition → reverts 2024 import order in committed files, `cargo fmt --check` then fails|root `rustfmt.toml` `style_edition = "2024"`, T52
