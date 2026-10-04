# Releasing

`.github/workflows/rust.yml` runs CI on `master` and on pull requests.
Studio, Machine and Port each have their own tag prefix, version file and
artifacts, and you release them separately.

| Project | Tag | Version source | Artifacts |
|---|---|---|---|
| Caiven Studio | `studio-v<version>` | `crates/caiven-studio/tauri.conf.json` | Linux AppImage and .deb, Windows NSIS and MSI installers, macOS DMGs for Apple Silicon and Intel |
| Caiven Machine | `machine-v<version>` | `crates/caiven-machine/Cargo.toml` | Archives for Linux, Windows and macOS (Apple Silicon and Intel), plus a Miyoo Mini build |
| Caiven Port | `port-v<version>` | `crates/caiven-port/Cargo.toml` | None yet. CI checks the tag; you build the image from `crates/caiven-port/Dockerfile` |

Bump only the projects you changed. A Studio fix doesn't need a new Machine
or Port version.

## Steps

Using Studio as the example:

1. Set the new version in `crates/caiven-studio/tauri.conf.json`.
2. Commit it.
3. Push a matching tag:

```bash
git tag studio-v0.1.0
git push origin master
git push origin studio-v0.1.0
```

Machine and Port work the same way with `machine-v<version>` and
`port-v<version>` tags and their own version files.

Each `release-check-*` job rejects a tag that doesn't match the package
version. When CI and the platform builds pass, the workflow creates a GitHub
Release with generated notes. Studio and Machine releases include their
installers and archives. Port has no release artifact yet. Run the workflow
with `workflow_dispatch` to build the same artifacts without publishing a
release.

## Downloads

Studio releases are at
`https://github.com/andrejmarkus/caiven/releases?q=studio-v` and Machine
releases at `?q=machine-v`. GitHub's release search matches tag names. For
Port, build the image as described in [port.md](port.md).

## Code signing status

macOS builds have an ad-hoc signature and no notarization. Windows
installers are unsigned. Trusted public releases need
[macOS signing and notarization](https://v2.tauri.app/distribute/sign/macos/)
and [Windows code signing](https://v2.tauri.app/distribute/sign/windows/).

The `studio-bundles` job in `.github/workflows/rust.yml` signs builds once
the repository has these secrets:

- macOS: `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`,
  `APPLE_SIGNING_IDENTITY`, `APPLE_ID`, `APPLE_PASSWORD`, `APPLE_TEAM_ID`
- Windows: `WINDOWS_CERTIFICATE`, `WINDOWS_CERTIFICATE_PASSWORD`

Without them, builds stay ad-hoc signed or unsigned. The README's
[getting started](../README.md#getting-started) section explains how to open
an unsigned build.
