# Contributing to Caiven

Start with [building from source](docs/building.md) and the
[design charter](docs/product/design-charter.md). The charter defines the
hardware and API constraints; product hardening must preserve existing creator
projects, cartridge behavior, and ownership rights.

## Development checks

Install the documented system dependencies, then install both frontends with
`npm ci` from their directories. Use the checked-in lockfiles.

For Rust changes, run:

```bash
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings -A unused-imports
cargo test --locked --workspace
```

For each changed frontend, run `npm run check`, `npm test`, `npm run build`,
and its browser tests. Studio uses `npm run test:e2e`; Port uses
`npm run test:e2e:mock` and `npm run test:e2e:live`. Browser tests require
Playwright Chromium and permission to start local servers. Run
`npm --prefix crates/caiven-studio-ui run check:ui` for shared UI changes.

Before a release, use the complete gates in
[CI](.github/workflows/rust.yml), including dependency audits, documentation,
shipped WASM, offline exports, and repeated browser tests. A locally passing
subset does not replace the release gates.

Run the native creator workflow after changes to Studio persistence, export,
or Machine loading:

```bash
python3 scripts/creator-workflow/run.py
```

Requires Python 3, Rust/system build dependencies, built Studio frontend assets,
and Port's Playwright Chromium installation. The runner creates a temporary
project through real Studio actor commands, edits Lua (including a sibling
module), sprite, palette and sound bytes, saves and closes it, then reopens it
in a separate process. It exports `.cav` and offline HTML, removes the source
project, and checks Machine and Chromium playback, including visible movement
from input. Temporary projects and isolated Studio history are cleaned up on
success or failure. Each process has a ten-minute timeout.

This gate exercises native Rust handlers and Machine's actual loader/frame loop;
it does not launch the Studio webview or SDL window, test Tauri IPC serialization,
or certify physical audio/controllers. The two Rust stage tests are intentionally
ignored in ordinary `cargo test`; this runner executes them explicitly in CI.

## Review expectations

- Describe the user-visible problem, resulting behavior, and verification.
- For defects, add a test that reproduces the failure before applying the fix.
- Exercise malformed input, failure recovery, and authorization where relevant.
- Document public API, CLI, environment, and format changes alongside code.
- Preserve unknown cartridge sections and round-trip behavior. Format changes
  require explicit compatibility decisions and tests.
- Keep secrets, generated build output, and unrelated formatting out of patches.
- State untested platforms and integration dependencies honestly.

Report suspected vulnerabilities through [the security process](SECURITY.md).
Release procedures live in [releasing.md](docs/releasing.md); service deployment
and recovery live in [Port operations](docs/development/port-operations.md).
