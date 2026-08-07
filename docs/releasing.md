# Publishing a Release

`.github/workflows/rust.yml` runs CI on `master` and pull requests. Studio,
Machine, and Port tag and release independently — each has its own tag
prefix, version source, and artifacts. See `.claude/rules/release.md` for
the full split; summary:

| Project | Tag | Version source | Artifacts |
|---|---|---|---|
| Caiven Studio | `studio-v<version>` | `crates/caiven-studio/tauri.conf.json` | Linux AppImage + Debian package, Windows NSIS + MSI installers, macOS DMGs (Apple Silicon + Intel) |
| Caiven Machine | `machine-v<version>` | `crates/caiven-machine/Cargo.toml` | Linux, Windows, macOS (Apple Silicon + Intel) archives, plus a Miyoo Mini build |
| Caiven Port | `port-v<version>` | `crates/caiven-port/Cargo.toml` | Docker image at `ghcr.io/<owner>/caiven-port` |

Bump only the version(s) that actually changed — a Studio-only change does
not require bumping Machine's or Port's version.

Use the `caiven-release` skill (`/caiven-release`) to check readiness
before tagging.

Before tagging, e.g. for Studio:

1. Set the new version in `crates/caiven-studio/tauri.conf.json`.
2. Commit the version change.
3. Push a matching tag:

```bash
git tag studio-v0.1.0
git push origin master
git push origin studio-v0.1.0
```

Machine and Port follow the same pattern with `machine-v<version>` /
`port-v<version>` tags against their own version source.

Each `release-check-*` job rejects a tag that doesn't match its project's
package version. Once CI and that project's platform builds succeed, one
GitHub Release is created with generated notes and (for Studio/Machine) all
installers/archives attached; Port's release links to the Docker image
instead of attaching files. `workflow_dispatch` builds the same artifacts
for testing without publishing a release.

## Downloads

- Studio and Machine releases: `https://github.com/andrejmarkus/caiven/releases?q=studio-v` and `?q=machine-v` respectively (GitHub's release search matches on tag name).
- Port: pull the image, `docker pull ghcr.io/andrejmarkus/caiven-port:latest` (or a pinned version tag).

## Code signing status

macOS builds use ad-hoc signing, so they are not notarized; Windows installers
are unsigned. Public trusted releases need
[macOS signing/notarization](https://v2.tauri.app/distribute/sign/macos/) and
[Windows code signing](https://v2.tauri.app/distribute/sign/windows/). The
release workflow (`studio-bundles` in `.github/workflows/rust.yml`) is wired
to sign automatically once the required secrets
(`APPLE_CERTIFICATE`/`APPLE_CERTIFICATE_PASSWORD`/`APPLE_SIGNING_IDENTITY`/
`APPLE_ID`/`APPLE_PASSWORD`/`APPLE_TEAM_ID` for macOS notarization,
`WINDOWS_CERTIFICATE`/`WINDOWS_CERTIFICATE_PASSWORD` for Windows) are added
to the repo — builds stay ad-hoc/unsigned when they're unset. Until then,
see the first-launch bypass steps in the [README quick start](../README.md#-quick-start).
