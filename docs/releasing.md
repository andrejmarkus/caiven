# Publishing a Release

`.github/workflows/rust.yml` runs CI on `master` and pull requests. A
version tag also builds:

- Caiven Studio: Linux AppImage + Debian package, Windows NSIS + MSI installers,
  and macOS DMGs for Apple Silicon + Intel
- Caiven Machine: Linux, Windows, macOS Apple Silicon, and macOS Intel archives

Before tagging:

1. Set the same version in `crates/caiven-studio/tauri.conf.json`,
   `crates/caiven-studio/Cargo.toml`, and
   `crates/caiven-machine/Cargo.toml`.
2. Commit the version change.
3. Push a matching `v<version>` tag:

```bash
git tag v0.1.0
git push origin master
git push origin v0.1.0
```

The workflow rejects mismatched package versions and tags that do not match
the Studio bundle version. Once CI and every platform build succeed, one
GitHub Release is created with generated notes and all installers/archives.
`workflow_dispatch` builds the same artifacts for testing without publishing a
release.

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
