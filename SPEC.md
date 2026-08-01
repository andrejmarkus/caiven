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

## §T TASKS

id|status|task|cites
T1|x|distill initial SPEC.md from repo|-

## §B BUGS

id|date|cause|fix
