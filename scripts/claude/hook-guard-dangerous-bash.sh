#!/usr/bin/env bash
# PreToolUse hook for Bash: blocks destructive git operations and release
# tag/publish actions unless explicitly confirmed by the user in-session.
# Reads the tool_input JSON from stdin, exits 2 (blocking) with a message
# on stderr if the command matches a denylisted pattern.
set -euo pipefail

input=$(cat)
command=$(python3 -c "
import json, sys
try:
    print(json.load(sys.stdin).get('tool_input', {}).get('command', ''))
except Exception:
    print('')
" <<<"$input")

deny_patterns=(
  'git[[:space:]]+push[[:space:]]+.*--force'
  'git[[:space:]]+push[[:space:]]+.*-f([[:space:]]|$)'
  'git[[:space:]]+reset[[:space:]]+--hard'
  'git[[:space:]]+clean[[:space:]]+-[a-z]*f'
  'git[[:space:]]+branch[[:space:]]+-D'
  'git[[:space:]]+tag[[:space:]]+.*(-f|--force)'
  'git[[:space:]]+push[[:space:]]+.*[[:space:]]v[0-9]'
  '--dangerously-skip-permissions'
  'gh[[:space:]]+release[[:space:]]+create'
)

for pattern in "${deny_patterns[@]}"; do
  if [[ "$command" =~ $pattern ]]; then
    echo "Blocked by hook-guard-dangerous-bash.sh: command matches denylisted pattern '$pattern'." >&2
    echo "This looks like a destructive/irreversible or release-publishing action. Confirm explicitly with the user before running it manually." >&2
    exit 2
  fi
done

exit 0
