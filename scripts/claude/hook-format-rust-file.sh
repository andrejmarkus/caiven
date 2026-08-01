#!/usr/bin/env bash
# PostToolUse hook for Write/Edit: if the touched file is a .rs file inside
# this repo, run rustfmt on just that file. Never blocks (always exits 0) —
# formatting-only, not a correctness gate.
set -euo pipefail

input=$(cat)
file_path=$(python3 -c "
import json, sys
try:
    ti = json.load(sys.stdin).get('tool_input', {})
    print(ti.get('file_path', ''))
except Exception:
    print('')
" <<<"$input")

if [[ "$file_path" == *.rs && -f "$file_path" ]]; then
  rustfmt --edition 2021 "$file_path" 2>/dev/null || true
fi

exit 0
