# carts/

Built `.cav` binaries only — generated output, not hand-edited.

- `dev/` — dev/test carts, built from `projects/dev/`. Used by manual
  developer testing, `scripts/claude/check-cart-compat.sh`,
  `scripts/miyoo/build-machine.sh` (handheld packaging), and Caiven Port's
  web e2e suite (`dev/smoke.cav`, referenced by relative path from
  `crates/caiven-port/web/e2e/...` — don't rename it without updating those
  tests).

Creator-facing showcase examples (shown in Caiven Studio's welcome screen)
build to `crates/caiven-studio/resources/examples/` instead, and are
embedded into the Studio binary at build time
(`crates/caiven-studio/src/studio/examples.rs`).

To edit a demo cart: edit its project source under `projects/`, then run
`scripts/demo-carts/build.sh` to regenerate every `.cav`. Don't hand-edit
files in this directory — they're overwritten on the next build.
