#!/usr/bin/env bash
# Targeted Rust check: format + clippy + tests for changed packages only.
# Falls back to the whole workspace if package detection fails or -a is passed.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

all=false
[[ "${1:-}" == "-a" || "${1:-}" == "--all" ]] && all=true

run() {
  echo "+ $*"
  "$@"
}

run cargo fmt --all -- --check

if $all; then
  run cargo clippy --locked --all-targets -- -D warnings -A unused-imports
  run cargo test --locked --verbose
  exit 0
fi

changed_pkgs=$(git diff --name-only --diff-filter=ACMR -- 'crates/*/src/**/*.rs' 'crates/*/Cargo.toml' 2>/dev/null \
  | sed -E 's#^crates/([^/]+)/.*#\1#' | sort -u || true)

if [[ -z "$changed_pkgs" ]]; then
  echo "No changed Rust packages detected against working tree; running full clippy/test."
  run cargo clippy --locked --all-targets -- -D warnings -A unused-imports
  run cargo test --locked --verbose
  exit 0
fi

for pkg in $changed_pkgs; do
  echo "== $pkg =="
  run cargo clippy -p "$pkg" --locked --all-targets -- -D warnings -A unused-imports
  run cargo test -p "$pkg" --locked --verbose
done
