# Claude Code context budget

Caiven's Claude Code setup is useful, but the first version enabled every
project integration and exposed every project workflow skill to the model in
every session. That front-loaded context before feature work began and made it
easy for several overlapping workflows to load into one conversation.

## Sources of recurring context

- The project-root `CLAUDE.md` is loaded at startup and re-injected after
  compaction.
- Skill names and descriptions are normally listed to the model at startup.
  An invoked skill body remains in the conversation for the task.
- Enabled MCP/LSP plugins advertise tools or integrations before the first
  prompt. Browser MCPs have broad inventories and should be task-specific.
- Hook output enters the working conversation when the hook fires. Silent
  enforcement hooks are cheap; repeated reminder hooks are not.
- File reads, verbose test output, and unrelated prior tasks accumulate for
  the rest of the session unless isolated or cleared.

## Caiven policy

1. The checked-in `.claude/settings.json` disables all five optional project
   integrations by default.
2. `caiven-*` skills are `user-invocable-only`: they stay in the slash menu but
   their descriptions are hidden from the model until explicitly invoked.
3. Enable one integration at a time with `/plugin` when a task needs it, and
   disable it again when done.
4. Playwright and Chrome DevTools are not enabled together.
5. The root `CLAUDE.md` contains only standing rules; derivable architecture
   detail remains in normal documentation.
6. The Stop reminder hook is removed. Safety blocking, targeted failure
   guidance, and silent Rust formatting remain.

## Measure before and after

Use fresh sessions so previous conversation history does not distort the
comparison.

1. On the old configuration, start Claude Code at the repository root and run
   `/context` before reading files. Record the Memory, Skills, and MCP/tools
   rows.
2. On this configuration, start `claude` with no plugins enabled, then run
   `/context`.
3. Repeat after enabling the one plugin used for normal feature work, for
   example `rust-analyzer-lsp`, via `/plugin`.
4. Compare startup totals and confirm only the selected plugin appears.
5. During a feature, run `/context` again after implementation and note which
   file reads, skill bodies, and command outputs became the largest entries.

The exact token reduction depends on user-level plugins, auto memory, model
context size, and local settings. The expected structural result is:

- lean session: no Caiven project MCP/LSP tools;
- project skills: no descriptions in model context until manually invoked;
- smaller root instruction payload;
- no repeated Stop-hook reminder;
- focused sessions: one LSP, or one LSP plus one browser MCP.

## Troubleshooting unexpected startup cost

- Run `/status` and inspect the active settings sources.
- Check `.claude/settings.local.json`; local settings override checked-in
  project settings.
- Use `/skills` to confirm project skills show as user-only.
- Use `/plugin` to inspect enabled user-scope plugins that are unrelated to
  Caiven's five project integrations.
- Run `/doctor` when a memory file or skill listing is reported as large.

Use `/clear` between unrelated tasks. Compaction preserves useful continuity,
but it does not make an unrelated conversation a clean implementation context.
