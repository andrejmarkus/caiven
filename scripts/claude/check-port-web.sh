#!/usr/bin/env bash
# Targeted checks for Caiven Port's Svelte frontend.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)/crates/caiven-port/web"

echo "+ npm run check:ui"
npm run check:ui
echo "+ npm run check"
npm run check

if [[ "${1:-}" == "-a" || "${1:-}" == "--all" ]]; then
  echo "+ npm run test:e2e:mock"
  npm run test:e2e:mock
  echo "NOTE: test:e2e:live needs a live backend/DB — run manually, not from this script."
fi
