#!/usr/bin/env bash
# Publishes every projects/remix/ starter to a Port as a remixable cart.
# Needs CAIVEN_PORT_URL and CAIVEN_PORT_API_KEY (Profile page → API tokens).
# Each run creates new carts; re-publish a starter with
# `caiven-studio publish <dir> --remixable --cart-id <id>` instead.
set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

: "${CAIVEN_PORT_URL:?set CAIVEN_PORT_URL, e.g. https://port.example.com}"
: "${CAIVEN_PORT_API_KEY:?set CAIVEN_PORT_API_KEY to a Port API token}"

cargo build -p caiven-studio
BIN=target/debug/caiven-studio

describe() {
  case "$1" in
    juggle) echo "Keep every ball in the air. Every 5 hits, another one drops in. Try BALLS = 30." ;;
    meteor) echo "Dodge the rocks, skim past them for CLOSE bonuses. Try ROCK_SIZE = 25." ;;
    hop) echo "Flap through the gaps, faster every pipe. Try GRAVITY = 0.05." ;;
    chain) echo "One shot. Set off the biggest chain reaction you can. Try DOTS = 200." ;;
    *) echo "A tiny game made to be remixed. Change one number and make it yours." ;;
  esac
}

for dir in projects/remix/*/; do
  name=$(basename "$dir")
  echo "== $name"
  "$BIN" publish "$dir" --remixable --frames 120 \
    --description "$(describe "$name")" --tags "remix-starter"
done
