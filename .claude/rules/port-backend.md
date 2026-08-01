---
paths:
  - "crates/caiven-port/src/**"
  - "crates/migration/**"
---

# Caiven Port backend

- `src/handlers/auth.rs` is the largest, most security-sensitive file in the
  workspace (WebAuthn via `webauthn-rs`, sessions, tokens) — treat any
  change here as security-sensitive by default (`.claude/rules/security.md`).
- `src/handlers/carts.rs`, `versions.rs` handle cart upload/versioning —
  uploaded `.cav` files are untrusted input; reuse `caiven-cart`'s
  parsing/validation rather than re-parsing ad hoc.
- DB access goes through `sea-orm`; schema changes belong in
  `crates/migration`, not ad hoc SQL. Every migration needs an explicit
  up path and should be reversible or clearly documented as not.
- `src/handlers/community.rs`, `social.rs`, `discovery.rs` carry
  user-generated content and authorization checks (who can rate/comment/see
  what) — verify authorization on the handler, not just in the frontend.
- `src/handlers/legacy.rs` exists for compat — don't delete without checking
  what still depends on it.
