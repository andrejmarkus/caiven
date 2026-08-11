---
name: caiven-release
description: Prepare a release-readiness report for Caiven — version consistency, CI, Rust/frontend tests, security scans, documentation, changelog, installer/artifact configuration, supported platforms, example cartridges, known issues, upgrade/compatibility info. Use before tagging a release, never to actually tag/push/publish without explicit approval.
---

# caiven-release

This skill produces a **report**. Tagging, pushing, or triggering
`workflow_dispatch` release paths always requires explicit user approval —
see `.claude/rules/release.md`.

Studio, Machine, and Port tag and release independently (`studio-vX.Y.Z`,
`machine-vX.Y.Z`, `port-vX.Y.Z`) — scope the report to whichever one is
being released, not all three. See `.claude/rules/release.md` for the full
split.

## Review checklist

- **Version consistency** — for the project being released, its version
  source matches the intended tag: Studio → `crates/caiven-studio/
  tauri.conf.json`, Machine → `crates/caiven-machine/Cargo.toml`, Port →
  `crates/caiven-port/Cargo.toml`. Verified by `release-check-studio`,
  `release-check-machine`, or `release-check-port` in
  `.github/workflows/rust.yml`. Don't bump versions of projects that
  didn't change.
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
  Windows/macOS x64+arm64 `caiven-machine`), `studio-bundles` (appimage/deb,
  nsis/msi, dmg via `tauri-apps/tauri-action`), or `port-image` (Docker
  image pushed to `ghcr.io/<owner>/caiven-port`) — whichever applies to
  this release.
- **Supported platforms** — matches what's actually built/tested, not
  aspirational.
- **Example cartridges** — `carts/dev/*.cav` still load correctly under the
  release build.
- **Known issues** — carry forward anything unresolved from
  `docs/development/claude-code-audit.md`'s gaps section that's relevant
  to this release.
- **Upgrade and compatibility information** — anything a creator upgrading
  Studio or a `.cav` from an older version needs to know.

## Output

A pass/blocked-per-item report, plus a clear go/no-go recommendation — not
just a status dump.

**If blocked**: stop here. List the blockers. Do not propose a version,
tag name, title, release notes, or touch any version field.

**If ready (go)**, additionally produce:

- **Next version** — inferred from changes since the last tag for this
  project (`git log <last-tag>..HEAD -- <project paths>`): breaking API/
  cart-format/Tauri-command change → bump minor (project is pre-1.0, so
  minor carries breaking changes per semver's 0.x convention); new
  feature → bump minor; fix/chore only → bump patch. State which changes
  drove the bump so the user can override.
- **Tag name** — `studio-vX.Y.Z` / `machine-vX.Y.Z` / `port-vX.Y.Z`.
- **Release title** — short, human, e.g. "Studio 0.5.0".
- **Release notes draft** — grouped bullets (Added/Changed/Fixed) from the
  commit range, in plain English, no SPEC.md ids (matches commit message
  rules in root `CLAUDE.md`). Output this in a fenced ```markdown code
  block, raw, so it can be copy-pasted straight into the GitHub release
  body as-is.
- **Apply the version bump to files**: write the new version into the
  project's version source (Studio → `crates/caiven-studio/
  tauri.conf.json`, Machine → `crates/caiven-machine/Cargo.toml`, Port →
  `crates/caiven-port/Cargo.toml`) using Edit. This is a local file edit
  only — never commit, tag, or push as part of this skill; leave that for
  the user to review and do explicitly.
