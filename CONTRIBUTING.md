# Contributing to Caiven

Read [building from source](docs/building.md) and the
[design charter](docs/product/design-charter.md) first. The charter fixes
the hardware specs and sets the rules for new APIs. Changes must keep
existing projects and carts working the same way, and must not touch
creators' ownership of their games.

`CLAUDE.md` and `.claude/` hold instructions for the AI coding tools I use
in development. The console ships no AI features.

## Checks

Install the system dependencies listed in the build guide, then run `npm ci`
in each frontend folder. Use the lockfiles in the repository.

For Rust changes:

```bash
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings -A unused-imports
cargo test --locked --workspace
```

For each frontend you change, run `npm run check`, `npm test`,
`npm run build` and its browser tests: `npm run test:e2e` for Studio,
`npm run test:e2e:mock` and `npm run test:e2e:live` for Port. The browser
tests need Playwright Chromium and must be able to start local servers. If
you change shared UI code, also run
`npm --prefix crates/caiven-studio-ui run check:ui`.

Before a release, run the full set of gates in
[CI](.github/workflows/rust.yml). They add dependency audits, docs checks,
the shipped WASM, offline exports and repeated browser test runs that a
local subset skips.

### Creator workflow test

If you change Studio saving, export or Machine loading, run:

```bash
python3 scripts/creator-workflow/run.py
```

You need Python 3, the Rust and system build dependencies, built Studio
frontend assets and Port's Playwright Chromium.

The script creates a temporary project through Studio's backend commands. It
edits Lua (including a second module), sprite, palette and sound data, saves
and closes the project, then reopens it in a new process. Next it exports a
`.cav` and an offline HTML file, deletes the source project, and plays both
exports in Machine and Chromium, checking that input moves something on
screen. It deletes its temporary files whether it passes or fails, and each
process times out after ten minutes.

The test covers the Rust command handlers and Machine's real loader and
frame loop. It doesn't open the Studio window or an SDL window, doesn't test
Tauri IPC serialization, and can't tell you whether audio or controllers
work on real hardware. Plain `cargo test` skips its two Rust stage tests on
purpose; CI runs them through this script.

## Pull requests

- Describe the problem a user sees, what changes, and how you checked it.
- For a bug, add a test that fails before your fix.
- Test bad input, recovery from failure and permission checks where they
  apply.
- Update the docs in the same change when you alter a public API, CLI flag,
  environment variable or file format.
- Keep unknown cart sections intact and make sure carts round-trip. A format
  change needs a written compatibility decision and tests.
- Leave secrets, build output and unrelated formatting out of the diff.
- Say which platforms and integrations you didn't test.

Report security problems through [SECURITY.md](SECURITY.md). Release steps
are in [releasing.md](docs/releasing.md). Port deployment and recovery are in
[port-operations.md](docs/development/port-operations.md).
