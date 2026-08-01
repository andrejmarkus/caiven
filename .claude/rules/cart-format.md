---
paths:
  - "crates/caiven-cart/**"
---

# Cartridge / project format

`caiven-cart` owns both the on-disk project format (`caiven.toml` + loose
`.lua`/asset files, human-diffable) and the built binary `.cav` format
(`format.rs`, `header.rs`, `section.rs`, `bundle.rs`, `project.rs`,
`asset_png.rs`, `minify.rs`, `text.rs`).

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
6. Documentation of the format change (README + any `docs/` format spec).

Don't hand-roll parsing without bounds checks; treat every `.cav` as
untrusted input, since carts get shared through Caiven Port.
