---
paths:
  - ".github/workflows/**"
  - "crates/caiven-studio/tauri.conf.json"
  - "Cargo.toml"
---

# Release

Studio, Machine, and Port release independently — each has its own tag
prefix, version source, and CI gate. Do not conflate their versions.

- **Studio**: tag `studio-vX.Y.Z`. Version source: `crates/caiven-studio/
  tauri.conf.json`. `release-check-studio` verifies tag matches that
  version. `studio-bundles` builds Tauri installers per platform
  (appimage/deb, nsis/msi, dmg). `release-studio` publishes the GitHub
  Release with those installers attached.
- **Machine**: tag `machine-vX.Y.Z`. Version source: `crates/caiven-machine/
  Cargo.toml`. `release-check-machine` verifies tag matches that version.
  `machine-artifacts` (Linux/Windows/macOS x64+arm64 binaries) and
  `machine-artifacts-miyoo` (Miyoo Mini build via `scripts/miyoo/`) build
  the artifacts. `release-machine` publishes the GitHub Release.
- **Port**: tag `port-vX.Y.Z`. Version source: `crates/caiven-port/
  Cargo.toml`. `release-check-port` verifies tag matches that version.
  `port-image` builds `crates/caiven-port/Dockerfile` and pushes to
  `ghcr.io/<owner>/caiven-port` (tagged with the stripped version and
  `latest`). `release-port` publishes a GitHub Release with no attached
  files — the Docker image *is* the release artifact.
- Bump only the version(s) that actually changed. A Studio-only change
  does not require bumping Machine's or Port's `Cargo.toml` — the point of
  splitting tags was to stop forcing unrelated version bumps.
- All three tag prefixes share the same `.github/workflows/rust.yml` file
  and the same `build`/`lint`/`security`/`doc` quality gate before any
  release job runs.
- `cargo audit` (with the documented `RUSTSEC-2023-0071` exception for
  unused `rsa` via sqlx-mysql metadata) and `npm audit --omit=dev
  --audit-level=high` for both frontends must pass before a release.
- Use the `caiven-release` skill to assemble a release-readiness report
  before tagging — don't tag directly without it.
- Never push a tag or trigger `workflow_dispatch` release paths without
  explicit user approval.
