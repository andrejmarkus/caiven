# Claude Code workflow guide for Caiven

How to use the Claude Code setup in this repo day to day. See also root
`CLAUDE.md`, `.claude/rules/*.md`, `.claude/skills/caiven-*`, and
`.claude/PLUGIN_STACK.md`.

## Tool-selection policy

Classify the task first, then use the smallest matching set below. Never
invoke every plugin/skill for every task.

**Product idea**
- Product Management plugin (available externally via
  `anthropics/knowledge-work-plugins`, deferred — see
  `.claude/PLUGIN_STACK.md` — use `caiven-idea` instead for now)
- `caiven-idea`
- Playground for interactive exploration (deferred, install on demand)
- CaveKit only after an idea is approved for implementation

**New feature**
- CaveKit specification
- Superpowers (primary implementation workflow)
- Relevant LSP (rust-analyzer / typescript / lua, all project-installed)
- Context7 for current external library documentation
- Focused tests
- Code Review after implementation

**UI or creator workflow**
- Frontend Design
- Playground when rapid visual experimentation helps (deferred)
- `caiven-studio-flow`
- Playwright (project-installed)
- Chrome DevTools MCP for diagnosis (project-installed)
- Manual accessibility check (no dedicated a11y plugin exists — see
  `.claude/rules/studio-ui.md`)

**Bug**
- `caiven-debug`
- A regression test
- Targeted implementation
- CaveKit back-propagation or durable documentation when the bug reveals a
  reusable invariant

**Public Lua API**
- `caiven-lua-api`
- `caiven-cart-compat` when serialization is affected
- Example cartridge
- Full documentation and autocomplete review

**Performance**
- `caiven-benchmark`
- Baseline measurement
- Comparable after-measurement
- Correctness tests

**Security-sensitive change**
- Security Guidance during implementation
- Claude Security (deferred plugin — install with `claude plugin install
  claude-security -s project` when doing a deep pass) or `caiven-review`
  before completion
- Explicit threat analysis (see `.claude/rules/security.md`)

**Release**
- `caiven-release`
- Full repository gates (`scripts/claude/pre-commit-gate.sh`)
- Security review
- Documentation and compatibility review

## GitHub usage

The `github` plugin is not installed in this repo yet (see
`.claude/PLUGIN_STACK.md`) — install with `claude plugin install github -s
project` when issue/PR read access is wanted. Once available:

- Read issues, pull requests, and CI status when relevant to the task.
- Connect implementation work to an existing issue when one exists.
- Create focused draft issue/PR text only when requested.
- Never post comments, create issues, open PRs, merge, or push without
  explicit approval.
- Include tests, screenshots, and compatibility notes in any proposed PR
  description.

## Example requests

- "Use `caiven-idea` to find a small feature that improves first-time
  creator success."
- "Use CaveKit to specify this approved feature."
- "Use `caiven-feature` to implement the next specification task."
- "Use `caiven-studio-flow` to redesign and test the cartridge export
  workflow."
- "Use `caiven-debug` to reproduce and fix this crash."
- "Use `caiven-lua-api` to design this runtime API."
- "Use `caiven-review` on the current diff."
- "Use `caiven-release` to prepare version X."
