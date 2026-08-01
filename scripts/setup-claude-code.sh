#!/usr/bin/env bash
# Idempotent setup for the Claude Code development system used on Caiven.
# Safe to re-run. Installs nothing system-wide without the flags below being
# explicitly opted into by the person running this script.
#
# What this does NOT do: push, install system packages automatically, or
# touch anything outside the project-scope Claude Code plugin registry and
# optional dev-tool binaries you explicitly ask for.
set -euo pipefail

need() { command -v "$1" >/dev/null 2>&1; }

echo "== Claude Code project-scoped plugins =="
if ! need claude; then
  echo "claude CLI not found on PATH; install Claude Code first: https://claude.com/claude-code" >&2
  exit 1
fi

plugins=(
  rust-analyzer-lsp
  typescript-lsp
  lua-lsp
  playwright
  chrome-devtools-mcp
)

for p in "${plugins[@]}"; do
  if claude plugin list 2>/dev/null | grep -q "^  ❯ ${p}@claude-plugins-official"; then
    echo "  already installed: ${p}"
  else
    echo "  installing: ${p}"
    claude plugin install "${p}" -s project
  fi
done

echo
echo "== Optional language-server binaries =="
echo "These plugins register MCP/LSP integrations but do not install the"
echo "underlying binaries. Install manually if you want live diagnostics:"
echo

if ! need rust-analyzer; then
  echo "  rust-analyzer:            rustup component add rust-analyzer"
fi
if ! need typescript-language-server; then
  echo "  typescript-language-server: npm install -g typescript-language-server typescript"
fi
if ! need lua-language-server; then
  echo "  lua-language-server:       brew install lua-language-server   # or see https://github.com/LuaLS/lua-language-server"
fi

echo
echo "== Optional: Playwright browser binaries =="
echo "Studio and Port already vendor Playwright as an npm devDependency and CI"
echo "installs browsers itself. For local ad-hoc use with the Playwright MCP:"
echo "  npm --prefix crates/caiven-studio-ui exec playwright install --with-deps chromium"
echo

echo "== Done. Re-run any time; already-installed plugins are skipped. =="
