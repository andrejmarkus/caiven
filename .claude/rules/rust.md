---
paths:
  - "crates/**/*.rs"
  - "Cargo.toml"
  - "**/Cargo.toml"
---

# Rust rules

- Prefer explicit error handling (`Result`, `thiserror`/`anyhow` as already
  used per-crate). Avoid new `unwrap()`, `expect()`, panics, or unchecked
  indexing on a production code path. The workspace already lints
  `unwrap_used = "warn"` (`Cargo.toml`) — don't add more of what it's warning
  about.
- Keep public interfaces narrow: don't widen a crate's public surface (new
  `pub` items) unless the task requires it.
- Add focused tests near the changed behavior (`crates/<crate>/tests/` or
  inline `#[cfg(test)]`), not a separate broad test sweep.
- Avoid unrelated refactoring in the same diff as a fix or feature.
- Before finishing: `cargo fmt --all -- --check` and a targeted
  `cargo clippy -p <crate> --all-targets -- -D warnings -A unused-imports`
  (full workspace clippy only for the final gate — see
  `scripts/claude/check-rust.sh` and `pre-commit-gate.sh`).
- Cross-crate dependencies flow one way: `caiven-core` → `caiven-cart` /
  `caiven-vm` → `caiven-machine` / `caiven-studio` / `caiven-port`. Don't add
  a back-reference (e.g. `caiven-vm` depending on `caiven-studio`) without
  calling it out explicitly — it's almost always a boundary violation.
