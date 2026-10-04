# projects/

Source projects for the demo carts. Each one has a `caiven.toml`, a
`main.lua` and PNG or hex asset files (the loader lives in
`crates/caiven-cart/src/project.rs`). The three sets below are independent,
even where two projects share a name.

- `showcase/` holds the examples in Studio's Examples gallery
  (`crates/caiven-studio/src/studio/examples.rs`). They build to
  `crates/caiven-studio/resources/examples/<name>.cav`.
- `remix/` holds Quick Remix starters: one file, shapes only, a few
  constants at the top to change. `scripts/remix-seeds/publish.sh` publishes
  them to a Port as remixable carts. They have no checked-in `.cav`.
- `dev/` holds edge-case projects for manual testing and CI: handheld
  packaging, cart format checks and the Port e2e smoke test. They build to
  `carts/dev/<name>.cav`.

After editing a project, run `scripts/demo-carts/build.sh` to rebuild its
`.cav`.
