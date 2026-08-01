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

## Version gating (current policy)

Both formats now validate their version field on read instead of ignoring
it:

- **Binary `.cav`** (`format.rs`): `CART_FORMAT_VERSION` is the version
  written by `write`. `load_bytes` rejects any version outside
  `MIN_SUPPORTED_CART_VERSION..=CART_FORMAT_VERSION` with
  `CartError::UnsupportedCartVersion { found, min_supported, max_supported }`.
- **`caiven.toml`** (`project.rs`): `[cart].version` defaults to
  `CURRENT_MANIFEST_VERSION` via serde (`#[serde(default =
  "default_manifest_version")]`) so manifests written before the field
  existed keep loading. `parse_manifest` rejects anything outside
  `MIN_SUPPORTED_MANIFEST_VERSION..=CURRENT_MANIFEST_VERSION` with
  `CartError::UnsupportedManifestVersion`.

Policy is **accept older, reject newer** — not "reject anything not
current":

- Accept older because the section table is additive/self-describing (an
  unrecognized `SectionKind` decodes to `Custom(id)` and is carried through
  rather than erroring), so every version shipped so far has stayed
  byte-compatible with the current reader; `MIN_SUPPORTED_CART_VERSION`/
  `MIN_SUPPORTED_MANIFEST_VERSION` only exist to reject a version below
  anything ever written (e.g. `0`), which can only mean corrupt/hostile
  input.
- Reject newer because this build has no way to know what a
  higher-than-`CART_FORMAT_VERSION`/`CURRENT_MANIFEST_VERSION` file means —
  silently misparsing it is exactly the failure mode this gate exists to
  prevent.

When a future change actually breaks byte-compatibility (not just adds a
section kind), bump `CART_FORMAT_VERSION`/`CURRENT_MANIFEST_VERSION` *and*
raise the corresponding `MIN_SUPPORTED_*` to reject the old shape
explicitly, or add real migration code before accepting it — never let the
range widen implicitly.
