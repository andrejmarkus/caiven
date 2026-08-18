# Caiven product development loop

A practical loop for validating product/feature ideas against real creator
behavior, not just internal conviction. Pairs with the `caiven-idea` skill
for ideation and `caiven-feature` for implementation.

1. **Observe creator friction** — from direct feedback, Studio usage
   patterns, or a repeatable pain point noticed while dogfooding.
2. **State the user problem** — concretely, for a specific target creator
   (see `caiven-idea`'s required fields).
3. **Propose the smallest experiment** — not the full feature.
4. **Build a prototype** — often a `caiven-game-prototype` tiny cart, or a
   minimal Studio/Port flow change.
5. **Test it through a real cartridge or Studio workflow** — not just unit
   tests; actually play/use it.
6. **Measure the result** against a chosen metric (below).
7. **Keep, revise, or remove it** — based on the measurement, not on how
   much work went into it.
8. **Update documentation** — README, relevant `.claude/rules/*.md` if a
   durable lesson emerged.

## Candidate metrics

- Time to first running cartridge
- Time to first saved cartridge
- Tutorial completion
- Publishing completion
- Crash-free sessions
- Repeat creator sessions
- Number of created and published cartridges
- Plays or engagement per cartridge
- Feature usage
- Creator-reported friction

No analytics SDK or telemetry is wired up in this repo, and none should be
added without an explicit privacy and product decision — these metrics
today are gathered manually (creator feedback, direct observation, Port's
existing DB counts) rather than through instrumentation. Sentry/PostHog are
deferred plugins (see `.claude/PLUGIN_STACK.md`) pending that decision.
