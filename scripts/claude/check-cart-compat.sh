#!/usr/bin/env bash
# Cart/project-format compatibility check: run caiven-cart's own tests plus a
# round-trip smoke test against the checked-in example carts.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

echo "+ cargo test -p caiven-cart --locked"
cargo test -p caiven-cart --locked

echo "+ cargo build -p caiven-studio --locked"
cargo build -p caiven-studio --locked

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

fail=0
for cart in carts/dev/*.cav; do
  name=$(basename "$cart" .cav)
  out="$tmp/$name"
  echo "+ unpack/inspect roundtrip: $cart"
  if ! cargo run -p caiven-studio --locked -- unpack "$cart" -o "$out" >/dev/null 2>&1; then
    echo "  FAILED to unpack $cart" >&2
    fail=1
    continue
  fi
  if ! cargo run -p caiven-studio --locked -- inspect "$out" >/dev/null 2>&1; then
    echo "  FAILED to inspect unpacked $cart" >&2
    fail=1
  fi
done

exit $fail
