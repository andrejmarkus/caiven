# carts/

Dev/test fixtures, **not** the in-app Examples gallery.

Creator-facing examples (shown in Caiven Studio's welcome screen) live under
`crates/caiven-studio/resources/examples/` and are embedded into the Studio
binary at build time (`crates/caiven-studio/src/studio/examples.rs`).

- `demo_smoke.cav` stays at the root of this directory — it's read by
  Caiven Port's web e2e suite via a relative path
  (`crates/caiven-port/web/e2e/...`). Don't move or rename it without
  updating those tests.
- `fixtures/` holds other `.cav` files used ad hoc by manual testing and
  development. None of them are currently referenced by any test or build
  script — check before deleting, but they're safe to move/add to.
