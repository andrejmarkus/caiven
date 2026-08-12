#!/usr/bin/env bash
# Regenerates all demo .cav binaries from their checked-in project sources
# under projects/showcase/ and projects/dev/. Run after editing a demo
# project; never hand-edit the .cav outputs.
set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

cargo build -p caiven-studio
BIN=target/debug/caiven-studio

for dir in projects/showcase/*/; do
  name=$(basename "$dir")
  "$BIN" build "$dir" --out "crates/caiven-studio/resources/examples/${name}.cav" --no-minify
done

mkdir -p carts/dev
for dir in projects/dev/*/; do
  name=$(basename "$dir")
  "$BIN" build "$dir" --out "carts/dev/${name}.cav"
done

echo "Rebuilt $(ls projects/showcase | wc -l | tr -d ' ') showcase cart(s) and $(ls projects/dev | wc -l | tr -d ' ') dev cart(s)."
