---
paths:
  - ".github/workflows/**"
  - "crates/caiven-studio/tauri.conf.json"
  - "Cargo.toml"
---

# Release

- Version consistency matters across `Cargo.toml` workspace members,
  `crates/caiven-studio/tauri.conf.json`, and any package.json version
  fields the release workflow checks (`release-check` job in
  `.github/workflows/rust.yml` verifies Studio version — keep it passing).
- Release builds run on tag push (`v*`) via `platform-builds`-style jobs in
  the same workflow file: `machine-artifacts` (Linux/Windows/macOS x64+arm64
  `caiven-machine` binaries) and `studio-bundles` (Tauri installers per
  platform: appimage/deb, nsis/msi, dmg).
- `cargo audit` (with the documented `RUSTSEC-2023-0071` exception for
  unused `rsa` via sqlx-mysql metadata) and `npm audit --omit=dev
  --audit-level=high` for both frontends must pass before a release.
- Use the `caiven-release` skill to assemble a release-readiness report
  before tagging — don't tag directly without it.
- Never push a tag or trigger `workflow_dispatch` release paths without
  explicit user approval.
