#!/usr/bin/env bash
# Launch Claude Code with one focused Caiven tool profile.
set -euo pipefail

root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
profile=${1:-lean}

usage() {
  cat <<'EOF'
Usage: scripts/claude-session.sh [profile] [claude arguments...]

Profiles:
  lean        no project LSP or browser plugins (default)
  rust        rust-analyzer only
  typescript  TypeScript LSP only
  lua         Lua LSP only
  ui-test     TypeScript LSP + Playwright MCP
  ui-debug    TypeScript LSP + Chrome DevTools MCP

Examples:
  scripts/claude-session.sh rust
  scripts/claude-session.sh ui-test --model sonnet
EOF
}

if [[ "$profile" == "-h" || "$profile" == "--help" ]]; then
  usage
  exit 0
fi

case "$profile" in
  lean|rust|typescript|lua|ui-test|ui-debug) ;;
  *)
    echo "Unknown profile: $profile" >&2
    usage >&2
    exit 2
    ;;
esac

shift || true

if ! command -v claude >/dev/null 2>&1; then
  echo "claude CLI not found on PATH" >&2
  exit 1
fi

exec claude --settings "$root/.claude/profiles/$profile.json" "$@"
