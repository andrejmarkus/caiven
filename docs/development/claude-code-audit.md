# Claude Code repository audit

Snapshot taken 2026-08-01 while setting up the Claude Code development system
(`chore/claude-code-caiven-setup`). This is the first architecture document
in the repo beyond `docs/brand-colors.md` — treat it as a living doc, not a
one-time artifact, and update it when a structural boundary changes.

## Architecture map

Dependency direction is one-way: `caiven-core` → `caiven-cart` →
{`caiven-vm`, `migration`} → {`caiven-machine`, `caiven-studio`,
`caiven-port`, `caiven-web`}.

| Crate | Responsibility | Key entry points |
|---|---|---|
| `caiven-core` | Dependency-free shared types: memory-map layout, collision, color/palette, `Vec2`. | `src/{lib,memory,collision,color,vec2}.rs` |
| `caiven-cart` | Cartridge/project (de)serialization — binary `.cav` and `caiven.toml` project dirs. | `src/{format,header,section,project,bundle,asset_png,minify,text}.rs` |
| `caiven-vm` | The console itself: Lua execution (mlua), rendering, audio, input, memory, RTC. `native` feature gates `winit`/`pixels`/`cpal` (off for `caiven-web`). | `src/vm/{api_registry,lua_exec,audio,execution,camera,rtc,sfx}.rs`, `src/{rendering,input}`, `src/runtime.rs` (shared by Machine + Studio) |
| `caiven-machine` | Standalone cart player: thin `winit` shell around `caiven_vm::runtime::ConsoleCore`. | `src/{app,main}.rs` |
| `caiven-studio` | Tauri 2 desktop editor backend — IPC command surface, debugger, hot-reload, Port client. | `src/tauri_app.rs` (2431 lines, ~35+ `#[tauri::command]`s), `src/debugger.rs`, `src/studio/*`, `src/port_client.rs` |
| `caiven-studio-ui` | Svelte 5/Vite Studio frontend (not a Cargo member). CodeMirror editor, undo/redo (`src/lib/history.ts`), Playwright e2e. | `src/{components,lib}` |
| `caiven-port` | Rocket cart-sharing server: auth/sessions/MFA/WebAuthn/OAuth, sea-orm entities, cart/version/comment/rating/jam handlers. | `src/{auth,db,mailer,oauth,turnstile,models}.rs`, `src/entities/*` (16 files), `src/handlers/*` |
| `caiven-port/web` | Svelte Port frontend (not a Cargo member). Mock + live Playwright e2e. | `src` |
| `caiven-web` | Browser WASM cart player (emscripten `bin`, not cdylib — vendored Lua's C build isn't `-fPIC`). No auth/DB code. | `src/main.rs` |
| `migration` | sea-orm-migration schema history for Port's DB, 14 migrations `m20240101`…`m20260728`. Depends on `caiven-cart` for content hashing. | `src/*.rs` |
| `caiven-ui` | Shared shadcn-svelte component library for both frontends; `check-boundaries.mjs` enforces neither app forks its own local UI tree. | `src/components`, `scripts/check-boundaries.mjs` |

## Verified commands

See root `CLAUDE.md` "Canonical commands" — verified directly against
`Cargo.toml`, `crates/caiven-studio-ui/package.json`, and
`crates/caiven-port/web/package.json`, not assumed from the setup prompt.

## Existing safeguards

- CI (`.github/workflows/rust.yml`): build+test+e2e (Studio & Port, mocked
  3x-stress + Port live full-stack gate), lint (`fmt --check`, `clippy -D
  warnings -A unused-imports`), `security` job (`cargo audit` with a
  documented `RUSTSEC-2023-0071` exception, `npm audit` both frontends),
  `doc` (`cargo doc`), `release-check` (Studio version vs. tag), then
  cross-platform release artifacts.
- `.github/workflows/platform-builds.yml`: lighter PR-only "still builds
  everywhere" gate (Linux/Windows/macOS x64+arm64), no release publish.
- `crates/caiven-ui/scripts/check-boundaries.mjs`: enforces shared-component
  ownership between the two frontends.
- `Cargo.toml` workspace lint `unwrap_used = "warn"` — but only
  `caiven-cart`, `caiven-vm`, `caiven-port`, `caiven-web` opt in via
  `[lints] workspace = true`; `caiven-machine`, `caiven-studio`,
  `migration` do not have that section (CI's blanket `clippy --all-targets`
  still catches them, but the opt-in is inconsistent).
- Port auth: argon2 password hashing, CSRF double-submit cookie, per-IP rate
  limiter, TOTP MFA + backup codes, WebAuthn (`webauthn-rs`) — all in
  `caiven-port/src/auth.rs` (800 lines).
- Cart/version uploads are size-capped (e.g. screenshots at 512 KiB,
  `handlers/versions.rs`) and validated through `caiven-cart`'s own parser —
  no separate archive-extraction code path exists (no `zip`/`tar` usage
  found), which narrows that particular attack surface.

## Gaps in tests or documentation

- **`caiven-web` has zero tests.** It's the browser WASM player — currently
  unverified by anything beyond manual checking.
- **`migration` has zero tests**, though exercised transitively by Port's
  DB-backed integration tests.
- `caiven-vm`'s core runtime is thin relative to its central role: `audio.rs`,
  `execution.rs`, `camera.rs`, `rtc.rs`, `sfx.rs` are mostly only exercised
  indirectly via `tests/lua_script.rs`, not directly.
- `caiven-studio/src/tauri_app.rs` — 2431 lines, ~35+ IPC commands, **no
  direct Rust unit tests**; all coverage is indirect via Studio's Playwright
  e2e suite. High blast radius, low direct test coverage.
- ~~Cart binary format (`caiven-cart/src/format.rs`) writes a version byte
  but the reader ignores it~~ **Resolved.** `format.rs::load_bytes` now
  reads the `.cav` version and rejects anything outside
  `MIN_SUPPORTED_CART_VERSION..=CART_FORMAT_VERSION` with
  `CartError::UnsupportedCartVersion`; `caiven.toml`'s `[cart]` table now
  has a `version` field (`CURRENT_MANIFEST_VERSION`, default `1` for
  manifests written before the field existed), validated the same way with
  `CartError::UnsupportedManifestVersion`. See `.claude/rules/cart-format.md`
  for the accept-older/reject-newer policy and why it's safe for the
  section-table shape.
- No architecture docs existed before this one (only `docs/brand-colors.md`
  and the dev-workflow-focused `crates/caiven-studio/CLAUDE.md`).
- No benchmark harness exists anywhere (no `criterion`, no `benches/`, no
  `#[bench]`) — `caiven-benchmark` skill's baseline/after methodology has no
  existing scaffolding to build on; first use will need to establish one.
- The setup prompt referenced a `.cavdbg` sidecar debug-file format; it does
  **not** appear anywhere in the repository (checked via full-text search).
  The actual debugger state (`caiven-studio/src/debugger.rs`) has breakpoints
  + watches with backward-compatible untagged deserialization for a legacy
  breakpoint format, but no documented file extension in code. Treat any
  future reference to `.cavdbg` as unverified until confirmed against
  current code, not as an established fact.

## High-risk subsystem boundaries

1. **Cart format version handling** (`caiven-cart/src/format.rs`,
   `project.rs`) — now gated on read (see above); still worth extra
   scrutiny on any future version bump since it's the compatibility
   boundary for every published cart.
2. **Port auth** (`caiven-port/src/auth.rs`, `handlers/auth.rs`) — largest,
   most security-sensitive file in the workspace.
3. **Tauri IPC surface** (`caiven-studio/src/tauri_app.rs`) — large,
   untested directly, and the boundary between (trusted-ish but still
   validate) frontend input and filesystem/VM state.
4. **Lua sandbox boundary** (`caiven-vm/src/vm/*`) — a cart's Lua code must
   never reach outside the sanctioned builtin surface.
5. **Audio real-time thread** (`caiven-vm/src/vm/audio.rs`) — `cpal`
   callback thread shares `Sound` state via `Arc<Mutex<_>>`; must never
   block/allocate unpredictably.

## Recommended Claude Code extension points

- `.claude/rules/lua-api.md` explicitly calls out the manual-sync risk
  between `api_registry.rs`'s doc comment and
  `lua_exec.rs::register_builtins`.
- `caiven-benchmark` skill should establish a minimal `criterion` or
  hand-rolled timing harness on first real use, since none exists.
- `caiven-cart-compat` skill now has real version-gating logic to check
  against in `format.rs`/`project.rs` (see above) — use it to review any
  future version bump for a correct backward-compat decision, not just to
  flag that gating is missing.
- Consider (future, not part of this setup) adding `[lints] workspace =
  true` to `caiven-machine`, `caiven-studio`, and `migration` for
  consistency — flagged here, not changed, since it's outside this setup's
  scope and touches shipped crates.
