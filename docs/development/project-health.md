# Project health audit — 2026-09-13

Cross-component review of the Rust workspace, Studio IPC, Port API and browser
client, shipped WASM, offline export, build scripts, release gates, and contributor
documentation. The worktree was clean before this pass.

## Fixed

| Finding | Change | Regression check |
| --- | --- | --- |
| Shipped WASM accepted cartridge versions 1–3 while current carts use version 5 | Rebuilt both runtime artifacts; build script now updates the artifacts consumed by Port and Studio | Shipped-runtime smoke test, real browser Play, offline HTML smoke test |
| Studio silently discarded Port ratings, versions, sizes and screenshot flags | Deserialize server snake_case and serialize IPC camelCase separately | Rust server-to-Studio metadata conversion test |
| Browser game speed depended on display refresh rate | Shared 60 Hz accumulator for Port and offline exports, with bounded catch-up and visibility reset | 30/60/120/144 Hz timing tests and stall/reset test |
| Exported 192×128 framebuffer displayed as a square | Corrected canvas dimensions and 3:2 display ratio | Chromium test of actual CLI-generated HTML |
| Publish screenshots omitted text and concealed runtime faults | Shared embedded font across native, browser and screenshot hosts; fail capture on VM faults | Pixel-content and runtime-failure screenshot tests |
| Player listeners survived shutdown; held keys survived window blur; game controls intercepted text input | Focus-aware keyboard handling, blur release, removable click handler, frame-loop guard and complete touch-container removal | Desktop/mobile player lifecycle tests |
| Delayed player boots could finish after navigation or restart | Generation guard around asynchronous startup; dispose stale player instances | Existing restart/navigation flows; lifecycle regression test |
| Failed script/worklet loads prevented recovery | Clear failed load state; ignore late worklet completion after shutdown | Browser workflow coverage; failure branches reviewed |
| Saved credentials were reused after changing Port servers | Bind saved and environment credentials to their server; restrict Unix token-file permissions; parse server URLs | Credential-scope and URL-validation Rust tests |
| Admin endpoints missing from contract coverage | Added admin handler discovery, mock routes and moderation workflow | Contract guard and desktop/mobile moderation tests |
| Web build could use the wrong Rust installation and omit Lua's C++ runtime | Explicit rustup compiler selection and Emscripten C++ linker | Successful Emscripten 6.0.9 build and runtime smoke |
| Port image could publish before verification finished | Image publishing now depends on build, lint, security and docs; Docker Cargo build uses lockfile | Workflow/Dockerfile review; publishing not executed |
| Public Rust docs contained broken and private-item links | Corrected references; CI now rejects rustdoc warnings | Workspace documentation build with warnings denied |

## Verification

- Rust workspace: 640 tests passed; workspace build passed.
- Studio: 35 unit tests and 26 browser tests passed.
- Port: 5 timing tests, 23 mocked browser tests, and 1 live Rocket/SQLite test passed.
  One duplicate mobile static-contract check intentionally skipped.
- Both frontend type checks and production builds passed. Shared UI boundaries passed.
- Shipped WASM smoke passed: current cartridge, 192×128 pixels, audio API, no VM fault.
- Actual CLI-generated HTML passed Chromium offline rendering, aspect-ratio and
  input-event checks without network requests or browser errors.
- Rust formatting, Clippy with CI flags, and diff whitespace checks passed.

## Remaining limits and improvement candidates

- Native Studio browser tests mock Tauri IPC. Physical controllers, audio devices,
  handhelds, Windows/Linux installers and signing were not exercised here.
- Live server coverage uses SQLite. PostgreSQL deployment, real OAuth providers,
  SMTP delivery and production load require their corresponding environments.
- Dependency warnings remain: `proc-macro-error2` future Rust compatibility and
  Studio's approximately 891 kB minified main JavaScript chunk. Splitting heavy
  editor modules should follow startup profiling. Dependency advisories were not
  independently refreshed during this pass.
- Machine Port requests still block its frame loop. Moving network effects to a
  worker, with explicit timeouts and bounded response sizes, remains useful for
  slow or unavailable servers.
- Docker publishing was not run. Builder/runtime distribution compatibility and
  deployed container startup still need container verification.

Saved Studio tokens from older builds require one fresh account link because
they lack a trustworthy server identity. No releases or deployments were created
by this audit.
