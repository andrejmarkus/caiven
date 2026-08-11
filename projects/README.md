# projects/

Editable project sources (`caiven.toml` + `main.lua` + PNG/hex assets — see
`crates/caiven-cart/src/project.rs`) for Caiven's demo content. Two
independent sets, not shared files even where names overlap:

- `showcase/` — polished examples meant to be remixed by end users directly
  in Caiven Studio's Examples gallery
  (`crates/caiven-studio/src/studio/examples.rs`). Builds to
  `crates/caiven-studio/resources/examples/<name>.cav`.
- `dev/` — technical/edge-case projects for manual developer testing and
  automated tests/CI in equal measure (handheld packaging, cart-format
  compat checks, Port e2e smoke test). Builds to `carts/dev/<name>.cav`.

Run `scripts/demo-carts/build.sh` after editing any project here to
regenerate the corresponding `.cav` — never hand-edit the built binaries.
