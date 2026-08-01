#!/usr/bin/env bash
# Targeted checks for Caiven Studio's Svelte frontend.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)/crates/caiven-studio-ui"

echo "+ npm run check:ui"
npm run check:ui
echo "+ npm run check"
npm run check
echo "+ npm test"
npm test

if [[ "${1:-}" == "-a" || "${1:-}" == "--all" ]]; then
  echo "+ npm run test:e2e"
  npm run test:e2e
fi
