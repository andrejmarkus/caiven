#!/usr/bin/env bash
# PostToolUseFailure hook for Bash: when a test/build command fails, remind
# Claude to root-cause rather than blindly retrying the same command.
# Informational only — never blocks.
set -euo pipefail

input=$(cat)
command=$(python3 -c "
import json, sys
try:
    print(json.load(sys.stdin).get('tool_input', {}).get('command', ''))
except Exception:
    print('')
" <<<"$input")

if [[ "$command" =~ (cargo[[:space:]]+test|npm[[:space:]]+.*test|playwright[[:space:]]+test) ]]; then
  echo "A test/build command failed. Follow caiven-debug: reproduce, minimize, find the responsible boundary, add a regression test, fix root cause — don't retry the same command blindly." >&2
fi

exit 0
