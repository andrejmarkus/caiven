#!/usr/bin/env bash
# Full local gate — the closest local equivalent to CI's build+lint+security+doc
# jobs. Slower than the targeted check-*.sh scripts; run this before finishing
# a task, not after every edit.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

run() {
  echo
  echo "== $* =="
  "$@"
}

run cargo fmt --all -- --check
run cargo clippy --locked --all-targets -- -D warnings -A unused-imports
run cargo build --locked
run cargo test --locked --verbose
run cargo doc --locked --no-deps

if command -v cargo-audit >/dev/null 2>&1; then
  # Keep in sync with .github/workflows/rust.yml's cargo-audit step and its
  # rationale comment for why each of these is ignored.
  run cargo audit --ignore RUSTSEC-2023-0071 --ignore RUSTSEC-2026-0235
else
  echo "cargo-audit not installed; skipping (install: cargo install cargo-audit)" >&2
fi

run npm --prefix crates/caiven-studio-ui run check:ui
run npm --prefix crates/caiven-studio-ui run check
run npm --prefix crates/caiven-studio-ui test
run npm --prefix crates/caiven-port/web run check:ui
run npm --prefix crates/caiven-port/web run check

echo
echo "Skipped by default (slow / need extra setup — run manually if needed):"
echo "  npm --prefix crates/caiven-studio-ui run test:e2e"
echo "  npm --prefix crates/caiven-port/web run test:e2e:mock"
echo "  npm --prefix crates/caiven-port/web run test:e2e:live   (needs live DB)"
echo "  npm audit --omit=dev --audit-level=high  (both frontends)"
echo
echo "Full gate (fast portion) passed."
