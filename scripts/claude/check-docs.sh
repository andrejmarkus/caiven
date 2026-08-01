#!/usr/bin/env bash
# Lint Markdown frontmatter for skills and rules.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

fail=0

echo "+ checking SKILL.md frontmatter"
for f in .claude/skills/*/SKILL.md; do
  [[ -f "$f" ]] || continue
  head -1 "$f" | grep -q '^---$' || { echo "  missing frontmatter: $f" >&2; fail=1; }
  grep -q '^name:' "$f" || { echo "  missing name: $f" >&2; fail=1; }
  grep -q '^description:' "$f" || { echo "  missing description: $f" >&2; fail=1; }
done

echo "+ checking rules frontmatter"
for f in .claude/rules/*.md; do
  [[ -f "$f" ]] || continue
  head -1 "$f" | grep -q '^---$' || { echo "  missing frontmatter: $f" >&2; fail=1; }
  grep -q '^paths:' "$f" || { echo "  missing paths: $f" >&2; fail=1; }
done

exit $fail
