#!/usr/bin/env bash
# Stop hook: lightweight reminder to run the affected targeted check(s)
# before considering a task finished. Informational only — never blocks.
set -euo pipefail
cat <<'EOF' >&2
Reminder: before finishing, run the targeted gate(s) for what changed —
scripts/claude/check-rust.sh, check-studio-ui.sh, check-port-web.sh,
check-lua-api.sh, or check-cart-compat.sh — and scripts/claude/pre-commit-gate.sh
for a final full pass. Don't run the full CI suite after every small edit.
EOF
exit 0
