#!/usr/bin/env bash
# Install only the Claude Code integrations needed by selected Caiven profiles.
# Plugins are installed at user scope; checked-in project settings keep them
# disabled unless scripts/claude-session.sh enables a focused profile.
set -euo pipefail

need() { command -v "$1" >/dev/null 2>&1; }

usage() {
  cat <<'EOF'
Usage: scripts/setup-claude-code.sh [profile ...]

Profiles to install:
  rust        rust-analyzer LSP
  typescript  TypeScript LSP
  lua         Lua LSP
  ui-test     TypeScript LSP + Playwright MCP
  ui-debug    TypeScript LSP + Chrome DevTools MCP
  all         every integration above

With no arguments, nothing is installed. Claude Code remains usable in the
lean default profile.
EOF
}

if ! need claude; then
  echo "claude CLI not found on PATH; install Claude Code first" >&2
  exit 1
fi

if [[ $# -eq 0 ]]; then
  usage
  exit 0
fi

plugins=()
add_plugin() {
  local candidate=$1 existing
  for existing in "${plugins[@]}"; do
    [[ "$existing" == "$candidate" ]] && return 0
  done
  plugins+=("$candidate")
}

for profile in "$@"; do
  case "$profile" in
    rust)
      add_plugin rust-analyzer-lsp
      ;;
    typescript)
      add_plugin typescript-lsp
      ;;
    lua)
      add_plugin lua-lsp
      ;;
    ui-test)
      add_plugin typescript-lsp
      add_plugin playwright
      ;;
    ui-debug)
      add_plugin typescript-lsp
      add_plugin chrome-devtools-mcp
      ;;
    all)
      add_plugin rust-analyzer-lsp
      add_plugin typescript-lsp
      add_plugin lua-lsp
      add_plugin playwright
      add_plugin chrome-devtools-mcp
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown profile: $profile" >&2
      usage >&2
      exit 2
      ;;
  esac
done

echo "== Installing selected Claude Code plugins =="
for plugin in "${plugins[@]}"; do
  id="${plugin}@claude-plugins-official"
  if claude plugin list 2>/dev/null | grep -Fq "$id"; then
    echo "  already installed: $plugin"
  else
    echo "  installing at user scope: $plugin"
    claude plugin install "$plugin" -s user
  fi
done

echo
echo "== Optional language-server binaries =="
need rust-analyzer || echo "  rust-analyzer: rustup component add rust-analyzer"
need typescript-language-server || \
  echo "  typescript-language-server: npm install -g typescript-language-server typescript"
need lua-language-server || \
  echo "  lua-language-server: install from your package manager or LuaLS releases"

echo
echo "Playwright browser binaries, when using ui-test:"
echo "  npm --prefix crates/caiven-studio-ui exec playwright install chromium"
echo
echo "Launch a focused session with scripts/claude-session.sh <profile>."
