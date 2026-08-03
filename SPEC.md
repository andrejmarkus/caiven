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
  `runtime.rs:140`; asset PNG hard 128×128 `asset_png.rs:7`), hard 16-color palette
  (`palette.rs:6` `DEFAULT_COLORS[16]`), hard 64×64 tilemap (`caiven-core/src/memory.rs:44,46`
  `MAP_W=MAP_H=64`), sprites/shape primitives/camera. 60 FPS (`lua_exec.rs:154`).
  Max cart 128 KiB (`caiven-cart/src/lib.rs:26` `MAX_CART_BYTES`).
- Studio = Tauri 2 + Svelte 5. Port = rocket + sea-orm + webauthn-rs + argon2.
- Shared UI (`caiven-ui`, shadcn-svelte) consumed by studio-ui & port/web;
  boundary enforced `npm run check:ui`.
- License MPL-2.0. Creators own games/assets, sell royalty-free, source private.
- CI gate: fmt, clippy `-D warnings -A unused-imports`, build+test, cargo audit
  (ignore RUSTSEC-2023-0071), npm audit, cargo doc.
- No new `unwrap`/`expect`/panic/unchecked-index on production path
  (`unwrap_used = "warn"` workspace lint).

## §I INTERFACES

- lua-api: builtins in `caiven-vm/src/vm/api_registry.rs` + `lua_exec.rs`; pure-Lua
  stdlib `prelude.lua`. Entry hooks: `_init()` once, `_update()` /frame, `_draw()` /frame after.
  Descriptive names (`sprite`, `draw_rect`, `button_down`, `set_palette_color`, `draw_text`).
- cart: on-disk project = `caiven.toml` + loose `.lua`/assets (diffable);
  built `.cav` binary = magic `b"CAIVEN"` + `u16` version (=3) + n_sections + 72B header.
  Owner `caiven-cart` (`format.rs`,`header.rs`,`section.rs`,`bundle.rs`,`project.rs`,`asset_png.rs`,`minify.rs`,`text.rs`).
- machine: `caiven-machine` = cart-runner CLI (`app.rs:12` about="Caiven — cart runner") — runs project dir (hot-reload) or `.cav`. Studio launch = separate binary, ⊥ machine.
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

## §V INVARIANTS

V1: public Lua API behavior ⊥ change silently — breaking change ! explicit version/compat note.
V2: new/changed Lua builtin ! ship with impl + VM test + docs + Studio autocomplete + example cart + compat analysis + error-behavior doc.
V3: cart format change ! bump version field, backward-compat analysis, round-trip test (build→unpack→build stable), invalid-input test, migration-or-reject (⊥ silent misparse).
V4: every `.cav` = untrusted input → bounds-checked parse; truncated/corrupt/malicious ⊥ panic | OOB read → fail safe.
V5: `_update()`/`_draw()` hot path — per-frame alloc suspicious, needs reason. Perf claim ! measured (baseline before, same method after).
V6: timing/RTC/RNG deterministic where API implies — ⊥ silent timing-semantics change (`src/timing.rs`, `src/vm/rtc.rs`).
V7: audio path (`src/vm/audio.rs`, `sfx.rs`) adjacent real-time cpal thread → ⊥ block | unpredictable alloc.
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

## §R RESEARCH

id|finding|source
R1|mlua 0.10.5 `Debug` (hook payload) exposes event/names/source/curr_line/is_tail_call/stack only — ⊥ locals accessor|docs.rs/mlua/0.10.5/mlua/struct.Debug.html
R2|mlua has ⊥ separate "unsafe" cargo feature gating raw-state access. `Lua::exec_raw<R>(args\, \|state: *mut lua_State\| ...)` = inherently `unsafe fn`, ⊥ feature-gated, callable today w/ current `Cargo.toml:27` features (lua54,vendored)|docs.rs/mlua/0.10.5/mlua/struct.Lua.html#method.exec_raw
R3|`mlua_sys` 0.6.8 `lua54::lua` module exposes raw `lua_getlocal`/`lua_getstack` C bindings — mechanism exists in principle. Currently transitive-only dep (`Cargo.lock:4153-4157`), ⊥ direct `caiven-vm` dep yet. Exact fn signatures unconfirmed by doc fetch|docs.rs/mlua_sys/0.6.8, Cargo.lock:4153-4157
R4|`?` unresolved: is `exec_raw` safe to call reentrantly from inside an already-active `lua.set_hook` callback on the same `Lua` instance — mlua docs say instance "remains locked during execution," could mean reentrancy guard errors/panics if nested. Docs alone ⊥ settle this, needs a throwaway spike|docs.rs/mlua/0.10.5/mlua/struct.Lua.html#method.exec_raw

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
T10|.|new Tauri cmd (e.g. `studio_debug_locals`) + capabilities/`gen/schemas` regen for locals surface|T8,V9,I.studio-cmd
T11|.|expose locals in Studio debug UI alongside existing watches/call-stack (ConsolePane.svelte)|T10,I.studio-cmd
T12|.|audit README/docs for step-semantics claims (`ConsolePane.svelte:129` already accurate — "Step a frame at a time"; no mislabel found in checked surface, this is a repo-wide sweep not a fix)|V22
T13|.|Rust unit tests for debugger.rs + tauri_app.rs breakpoint/step/locals IPC path (audit-flagged: no direct tests today)|T8,T10,T12

## §B BUGS

id|date|cause|fix
