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
  `crates/migration`, not ad hoc SQL. `m20260929_000001_initial_schema` is
  the baseline: never edit it — every schema change is a new migration after
  it, with an explicit up path, reversible or clearly documented as not.
- The only REST surface is `/api/v1`. A deleted account's carts stay public
  with `owner_id = NULL` (FK `ON DELETE SET NULL`) and author `[deleted]`;
  code reading `owner_id` must handle `None`.
- `src/handlers/community.rs`, `social.rs`, `discovery.rs` carry
  user-generated content and authorization checks (who can rate/comment/see
  what) — verify authorization on the handler, not just in the frontend.
- Personal data has promises attached: retention periods live in
  `src/retention.rs` and must match `web/src/pages/Privacy.svelte`; a new
  table holding user data needs a row in `export_data`, a deletion path
  (FK cascade or explicit scrub in `delete_account`), and a line in the
  privacy table. Admin removal of others' content goes through
  `admin::moderate` so the owner gets a DSA statement of reasons.
