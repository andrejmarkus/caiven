#!/usr/bin/env bash
# Sanity check for public Lua API changes: registration/docs must move together.
# Not a substitute for cargo test — just a fast drift check.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

registry="crates/caiven-vm/src/vm/api_registry.rs"
exec_file="crates/caiven-vm/src/vm/lua_exec.rs"

if [[ ! -f "$registry" || ! -f "$exec_file" ]]; then
  echo "Expected files not found: $registry / $exec_file" >&2
  exit 1
fi

changed=$(git diff --name-only --diff-filter=ACMR -- "$registry" "$exec_file" 2>/dev/null || true)

if echo "$changed" | grep -q "$exec_file" && ! echo "$changed" | grep -q "$registry"; then
  echo "WARNING: lua_exec.rs changed but api_registry.rs did not." >&2
  echo "api_registry.rs documents itself as mirroring register_builtins() — check it stayed in sync." >&2
fi

echo "+ cargo test -p caiven-vm --locked"
cargo test -p caiven-vm --locked
