# carts/

This folder holds built `.cav` files. Don't edit them by hand;
`scripts/demo-carts/build.sh` overwrites them.

`dev/` contains test carts built from `projects/dev/`. They're used for
manual testing, `scripts/claude/check-cart-compat.sh`, handheld packaging in
`scripts/miyoo/build-machine.sh`, and the Port web e2e suite. The e2e tests
load `dev/smoke.cav` by relative path from `crates/caiven-port/web/e2e/`, so
update those tests if you rename it.

The showcase examples in Studio's welcome screen build to
`crates/caiven-studio/resources/examples/` instead. Studio embeds them at
compile time (see `crates/caiven-studio/src/studio/examples.rs`).

To change a demo cart, edit its source under `projects/` and run
`scripts/demo-carts/build.sh`.
