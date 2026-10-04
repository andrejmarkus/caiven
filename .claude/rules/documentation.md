---
paths:
  - "docs/**"
  - "README.md"
  - "CREATOR_RIGHTS.md"
  - "CONTRIBUTING.md"
  - "SECURITY.md"
  - "TRADEMARKS.md"
  - "carts/README.md"
  - "projects/README.md"
---

# Documentation

- README is the primary creator- and contributor-facing doc — keep its
  command examples in sync with actual `package.json` scripts / Cargo
  invocations; don't let it drift.
- Public Lua API changes must update README's API reference (or a
  dedicated `docs/` API page if one exists for the area) — see
  `.claude/rules/lua-api.md`.
- Durable lessons (recurring bug classes, non-obvious build steps,
  compatibility traps) belong in a scoped `.claude/rules/*.md` file, not
  only in commit messages or conversation history.
- Keep root `CLAUDE.md` under ~200 lines; put detail in scoped rules or
  `docs/development/` instead of growing the root file.

## Writing style

- Write for the reader in the project owner's voice: plain sentences, short
  sections, no emoji headings, taglines or filler.
- Instructions for coding tools belong in `CLAUDE.md` or `.claude/`, not in
  `docs/` or root `*.md`.
- State only claims that can be checked.
- Don't state a rule the same doc breaks; record exceptions as decisions.
- Design vocabulary (Clock A/B, seven-point gate, readable-lesson cap) stays
  in `docs/product/`; README, tutorial and API reference use plain words.
- Keep the AI-disclosure lines in README and CONTRIBUTING accurate.
