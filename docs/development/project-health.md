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
  and handhelds were not exercised here.
- ~~Windows/Linux installers and signing were not exercised here.~~ Partially
  verified 2026-09-13: `cargo tauri build` on macOS (this machine) produces a
  working `.app` and `.dmg` — the built app launches and runs, not just
  bundles. `spctl` rejects it since it's only ad-hoc signed (no Developer ID,
  no notarization) — expected without paid signing credentials, not a new
  defect. Windows/Linux installers still untested; real code-signing
  certificates for all platforms still need their own environments.
- ~~Live server coverage uses SQLite. PostgreSQL deployment...~~ Partially
  verified 2026-09-13: built the Docker image and ran it against real
  PostgreSQL via `crates/caiven-port/docker-compose.yml` — migrations applied
  cleanly, the API served cart data, and the built SPA (index, static assets,
  client-side routes) all returned 200 through the container. Real OAuth
  providers, SMTP delivery and production load still require their
  corresponding environments.
- Dependency warnings remain: `proc-macro-error2` future Rust compatibility.
  Dependency advisories were not independently refreshed during this pass.
- ~~Studio's approximately 891 kB minified main JavaScript chunk.~~ Fixed
  2026-09-13: `LuaEditor.svelte` (CodeMirror and its language packages, the
  single largest dependency) now loads via a dynamic `import()` in
  `Workspace.svelte`, triggered when the code screen opens instead of at
  startup. Main chunk dropped to ~508 kB; CodeMirror ships as its own ~384 kB
  chunk fetched on first visit to the code screen.
- ~~Machine Port requests block its frame loop.~~ Fixed 2026-09-13: a
  5s connect / 15s read timeout and `MAX_CART_BYTES` response cap were
  added first, then `port_client::list`/`download` moved off the frame
  thread entirely (`port_worker.rs`, one background thread per request,
  polled non-blockingly each frame). A slow or unreachable Port server no
  longer stalls input handling or rendering at all. The Port screen gained
  a `port_loading` flag to show "Loading…" instead of misreporting an
  empty/unreachable server while a fetch is in flight.
- ~~Docker publishing was not run... deployed container startup still need
  container verification.~~ Verified 2026-09-13 (build + run, not the
  publish workflow itself): `docker compose build` and `up` succeed end to
  end — Node web build, Rust release build, and the Debian runtime image all
  produce a working container. Publishing to the registry was still not
  exercised.

Saved Studio tokens from older builds require one fresh account link because
they lack a trustworthy server identity. No releases or deployments were created
by this audit.
