---
name: caiven-release
description: Prepare a release-readiness report for Caiven — version consistency, CI, Rust/frontend tests, security scans, documentation, changelog, installer/artifact configuration, supported platforms, example cartridges, known issues, upgrade/compatibility info. Use before tagging a release, never to actually tag/push/publish without explicit approval.
---

# caiven-release

This skill produces a **report**. Tagging, pushing, or triggering
`workflow_dispatch` release paths always requires explicit user approval —
see `.claude/rules/release.md`.

## Review checklist

- **Version consistency** — `Cargo.toml` workspace members,
  `crates/caiven-studio/tauri.conf.json`, and anything the `release-check`
  job in `.github/workflows/rust.yml` verifies.
- **CI** — current status of `build`, `lint`, `security`, `doc` jobs on the
  branch/commit being released.
- **Rust and frontend tests** — `cargo test --locked --verbose`, both
  frontends' `check` + e2e suites passing.
- **Security scans** — `cargo audit` (documented `RUSTSEC-2023-0071`
  exception only), `npm audit --omit=dev --audit-level=high` for both
  frontends.
- **Documentation** — README command examples still accurate; any Lua API
  or cart-format changes since the last release documented.
- **Changelog / release notes** — GitHub's auto-generated notes
  (`generate_release_notes: true`) plus anything that needs manual
  clarification (breaking changes, migration steps).
- **Installer and artifact configuration** — `machine-artifacts` (Linux/
  Windows/macOS x64+arm64 `caiven-machine`) and `studio-bundles`
  (appimage/deb, nsis/msi, dmg via `tauri-apps/tauri-action`) targets all
  accounted for.
- **Supported platforms** — matches what's actually built/tested, not
  aspirational.
- **Example cartridges** — `carts/*.cav` still load correctly under the
  release build.
- **Known issues** — carry forward anything unresolved from
  `docs/development/claude-code-audit.md`'s gaps section that's relevant
  to this release.
- **Upgrade and compatibility information** — anything a creator upgrading
  Studio or a `.cav` from an older version needs to know.

## Output

A pass/blocked-per-item report, plus a clear go/no-go recommendation — not
just a status dump.
