---
paths:
  - "docs/**"
  - "README.md"
  - "CREATOR_RIGHTS.md"
---

# Documentation

- README is the primary creator- and contributor-facing doc — keep its
  command examples in sync with actual `package.json` scripts / Cargo
  invocations; don't let it drift.
- Public Lua API changes must update README's API reference (or a
  dedicated `docs/` API page if one exists for the area) — see
  `.claude/rules/lua-api.md`.
- Durable lessons (recurring bug classes, non-obvious build steps,
  compatibility traps) belong in a scoped `.claude/rules/*.md` file or a
  CaveKit spec, not only in commit messages or conversation history.
- Keep root `CLAUDE.md` under ~200 lines; put detail in scoped rules or
  `docs/development/` instead of growing the root file.
