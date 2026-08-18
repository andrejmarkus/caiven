# Claude Code workflow for Caiven

The default project configuration is deliberately lean. It disables project
LSP/browser plugins and keeps `caiven-*` skills user-invocable so startup
context stays available for implementation work.

## Start the smallest session

Normal `claude` startup uses the lean checked-in settings — nothing to
install, no plugins enabled. Enable the one integration a task needs with
`/plugin`, and disable it again when done:

| Task | Plugin |
|---|---|
| Rust implementation | `rust-analyzer-lsp` |
| Svelte/TypeScript implementation | `typescript-lsp` |
| Lua/API or cartridge examples | `lua-lsp` |
| Browser automation/e2e | `playwright` |
| Interactive browser diagnosis | `chrome-devtools-mcp` |

Do not enable both Playwright and Chrome DevTools at once. Playwright is for
repeatable browser actions and tests; Chrome DevTools is for interactive
runtime/network/performance diagnosis.

## Invoke project skills deliberately

Project skills remain in the slash menu but are hidden from model context until
you invoke one. This prevents automatic skill chaining and keeps unused skill
bodies out of the session.

- `/caiven-feature` — approved feature implementation
- `/caiven-debug` — reproduce and fix a bug
- `/caiven-studio-flow` — Studio or Port creator workflow
- `/caiven-lua-api` — public runtime API change
- `/caiven-cart-compat` — cartridge/project format review
- `/caiven-benchmark` — measured performance work
- `/caiven-review` — independent review after implementation
- `/caiven-release` — release preparation
- `/caiven-idea`, `/caiven-game-prototype`, `/caiven-status` — product or
  status workflows

Use one primary workflow skill. Add a second only when the change genuinely
crosses a boundary, such as `/caiven-feature` plus `/caiven-lua-api`.

## Implementation loop

1. Read the request/specification and inspect the current implementation.
2. Identify touched subsystems and matching path-scoped rules.
3. Define a narrow acceptance target and explicit non-goals.
4. Add focused tests, implement the smallest coherent change, and run the
   matching `scripts/claude/check-*.sh` command.
5. Review the diff for compatibility, security, and unrelated changes.
6. Run `/caiven-review` only after the implementation is stable.
7. Run the full pre-commit gate only for a final pass or release.

## Context budget habits

- Run `/context` near session start to verify which memory, skills, and tools
  loaded.
- Use `/clear` before an unrelated feature instead of carrying the previous
  task's files and outputs forward.
- Delegate broad repository research to an Explore subagent so large reads do
  not remain in the implementation context.
- Avoid reading `.claude/PLUGIN_STACK.md`, the full architecture audit, or all
  scoped rules unless the task actually needs them.
- Prefer targeted command output. Redirect or filter verbose build logs and
  inspect only the failing section.

See `docs/development/claude-code-context-budget.md` for the rationale and a
repeatable before/after measurement procedure.

## GitHub and remote actions

Read issues, pull requests, diffs, and CI when relevant. Never post comments,
create issues, push, open a pull request, merge, or alter remote state without
explicit user approval. Any proposed PR description should include tests,
compatibility notes, and screenshots when applicable.
