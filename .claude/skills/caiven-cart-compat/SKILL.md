---
name: caiven-cart-compat
description: Review cartridge (.cav) and project-format (caiven.toml) changes in caiven-cart for compatibility risk. Use for any change touching crates/caiven-cart, or when a caiven-feature/caiven-lua-api change also affects serialization.
---

# caiven-cart-compat

Known baseline: `.cav`, `caiven.toml`, save data and Machine save state
are all version 1 (`docs/formats.md`). Each reader accepts exactly its
version and rejects the rest (`.claude/rules/cart-format.md`). The browser
mirror `crates/caiven-port/web/src/lib/cav.js` must change with `format.rs`.

## Must detect

- Silent incompatibility: a format change that changes what a `.cav` or
  `caiven.toml` means without any version signal.
- Missing version transitions: old format encountered, no defined
  behavior for what happens (should be: read old version explicitly,
  migrate or reject with a clear error — never silently misparse).
- Unsafe parsing: `.cav` is untrusted input shared through Caiven Port —
  no unchecked length reads, no panics on truncated/malformed sections.
- Incomplete round trips: build → unpack → build again should be stable,
  or the instability should be explicit and intentional.
- Asset corruption risks: PNG/hex asset encode-decode paths
  (`asset_png.rs`) losing data silently.
- Migration omissions: if the change affects Port's stored carts, check
  whether `crates/migration` needs a companion migration.

## Output

A pass/fail-style review listing each detected risk with file:line, and
whether it blocks the change or is an acceptable known limitation (state
which, explicitly — don't leave it ambiguous).
