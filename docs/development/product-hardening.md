# Product hardening — 2026-09-14

This pass strengthens concrete trust boundaries and operating practices across
the shared cartridge library, VM bundler, browser runtime, Port service,
container, dependency gates, and contributor documentation. It builds on the
[previous project health audit](project-health.md).

## Changes

| Area | Result | Evidence |
| --- | --- | --- |
| Cartridge ingestion | Reject input above 128 KiB; cap disk reads; validate all ranges before copying; reject header/table overlap, payload overlap, and missing/duplicate Program sections | `caiven-cart/tests/validation.rs` |
| Cartridge writing | Reject duplicate Program wire IDs, including `Custom(1)`, before touching the destination | Writer regression tests |
| Compatibility | Preserve valid v5 carts, reordered tables, empty Program sections, repeated custom sections, and existing content hashes | Existing round-trip suite plus compatibility regression |
| Lua bundling | Escape module keys and chunk names as Lua strings; ignore symlinked files/directories during module discovery | Real VM require test with quotes, backslashes, control characters and UTF-8; Unix symlink/cycle tests |
| Browser runtime | Rebuild checked-in JS/WASM; test actual shipped parser rejection and recovery | `caiven-web/smoke_test.mjs`, desktop/mobile player tests, actual offline HTML export |
| Service operations | Add uncached liveness/readiness; bound database probe to two seconds; return generic 503 on outage | `caiven-port/tests/health.rs`, including saturated-pool recovery |
| Container | Run as UID/GID 10001; retain writable SQLite directory; add readiness healthcheck; exclude local secrets and development caches from build context | Local image build and runtime checks |
| Dependencies | Patch nanoid in both frontend lockfiles and PostCSS in Port; audit the complete frontend dependency trees in CI | Both npm audits report zero vulnerabilities |
| Engineering process | Add contributor expectations, vulnerability reporting, dependency exception record, and deployment/backup/recovery guidance | `CONTRIBUTING.md`, `SECURITY.md`, `port-operations.md` |

## Verification

- Rust workspace: **656 tests passed**; formatting and Clippy with CI flags passed.
- Rust documentation built with warnings denied.
- Studio: **35 unit tests**, **26 browser tests** passed.
- Port: **5 timing tests**, **23 mocked browser tests**, **1 live Rocket/SQLite
  browser workflow** passed. One pre-existing duplicate mobile contract test is
  intentionally skipped.
- Both frontend type checks and production builds passed; shared UI boundaries
  and dependency parity passed.
- Rebuilt WASM passed rendering/audio checks, oversized/overlapping cartridge
  rejection, and recovery after rejection. Desktop/mobile playback and lifecycle
  checks passed against that artifact.
- CLI-generated offline HTML rendered at 3:2 with working input, no network
  requests, and no browser errors.
- Docker startup verified UID/GID 10001, writable data directory, non-writable
  SPA directory, `200 /readyz` with `no-store`, and Docker `healthy` status.

Initial Rust baseline failed because the sandbox denied local socket binding;
rerunning with socket access resolved that environment failure. Initial browser
runs timed out during simultaneous builds and multiple browser worker pools.
One-worker reruns passed without changing deadlines or assertions. The CI 3x
stress configuration remains unchanged; this pass did not run every suite 3x.

## Remaining product release work

This is a verified hardening pass, not a certification of production readiness.

- **Dependency remediation:** the full Rust audit still reports the existing
  `h2` and `rkyv` advisories plus maintainership/unsoundness warnings. The scoped
  CI command passes with its two existing exceptions. See
  [SECURITY.md](../../SECURITY.md) for limitations and deployment mitigations.
- **Trusted distribution:** configure real signing/notarization credentials and
  verify clean-machine installs on supported operating systems. No release was
  signed or published in this pass.
- **Production operations:** validate real SMTP/OAuth providers, production
  proxy configuration, backup restoration, load limits, alert routing, and
  recovery targets in the intended environment.
- **Device and accessibility acceptance:** test physical controllers/audio,
  supported handhelds, screen readers, and complete keyboard-only workflows.
  Existing browser coverage does not certify these surfaces.
- **Performance budgets:** Studio still emits a main-chunk size warning;
  measure cold start, frame latency and memory on supported hardware before
  setting enforceable product budgets.

No production data, registry, release, or deployment was changed.
