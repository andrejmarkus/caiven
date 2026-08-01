---
paths:
  - "crates/caiven-port/src/handlers/auth.rs"
  - "crates/caiven-port/src/**"
  - "crates/caiven-cart/**"
  - "crates/caiven-studio/src/**"
  - "crates/caiven-vm/src/vm/**"
  - "crates/caiven-web/**"
---

# Security-sensitive surfaces

Treat these as security-sensitive and give them extra scrutiny (Security
Guidance plugin during implementation; Claude Security or `caiven-review`
before completion):

- Authentication, sessions, API tokens (`caiven-port/src/handlers/auth.rs`).
- Publishing (cart upload/versioning handlers).
- File uploads and archive/cartridge extraction (`caiven-cart` parsing —
  untrusted `.cav` input from Port).
- Paths and filesystem access (Studio Tauri commands, Machine CLI,
  cart unpack/build).
- Tauri commands (`caiven-studio/src`) — IPC boundary, validate inputs.
- WebAuthn (`webauthn-rs` usage in `auth.rs`).
- Database authorization (`sea-orm` queries in Port handlers — check
  authorization per-handler, not just at the route layer).
- User-generated content (community/social/discovery handlers).
- Lua sandbox boundaries (`caiven-vm/src/vm/*`) — a cart's Lua code must not
  be able to reach the filesystem, network, or process outside the
  sanctioned API surface.

For any change touching these paths:
- State the threat explicitly (what untrusted input, what boundary).
- Prefer rejecting malformed/oversized/malicious input early over trying to
  handle it downstream.
- Don't roll a new crypto/auth primitive when `webauthn-rs`/`argon2`
  already cover the case.
- Run `cargo audit` and the relevant `npm audit` before calling
  a security-adjacent change done.
