---
paths:
  - "crates/caiven-cart/**"
---

# Cartridge / project format

`caiven-cart` owns both the on-disk project format (`caiven.toml` + loose
`.lua`/asset files, human-diffable) and the built binary `.cav` format
(`format.rs`, `header.rs`, `section.rs`, `bundle.rs`, `project.rs`,
`asset_png.rs`, `minify.rs`, `hex.rs`). The browser mirror
`crates/caiven-port/web/src/lib/cav.js` changes in the same commit as
`format.rs`; `tests/remix.test.js` round-trips a Rust-built cart through it.

Any format change must include:

1. An explicit versioning decision — bump whatever header/format version
   field exists rather than reusing it for a shape change.
2. A backward-compatibility analysis: can an old `.cav` still load? Can an
   old Studio still open a new project dir?
3. Round-trip tests: build → unpack → build again should be stable (or the
   instability should be intentional and documented).
4. Invalid-input tests: truncated/corrupted/malicious section data must fail
   safely, not panic or read out of bounds — this is also a security
   boundary (see `.claude/rules/security.md`, "cartridge parsing").
5. Migration or explicit-rejection behavior for old formats — never a silent
   misparse.
6. Documentation of the format change in `docs/formats.md`.

Don't hand-roll parsing without bounds checks; treat every `.cav` as
untrusted input, since carts get shared through Caiven Port.

## Version gating (current policy)

Version 1 is the first public baseline for `.cav`, `caiven.toml`, save
data (`caiven-vm/src/vm/save_data.rs`) and Machine save state. Each reader
accepts **exactly** its current version and rejects everything else:

- `.cav` (`format.rs`): `load_bytes` rejects `version != CART_FORMAT_VERSION`
  with `CartError::UnsupportedCartVersion { found, supported }`.
- `caiven.toml` (`project.rs`): `[cart] version` is required (missing is a
  parse error) and must equal `MANIFEST_VERSION`, else
  `CartError::UnsupportedManifestVersion`.

Adding a section kind is not a version change: unknown kinds decode to
`Custom(id)` and are carried through. A change that would make old bytes
misread bumps the version and ships the old shape's reader or migration in
the same change — never widen acceptance implicitly, never misparse silently.
