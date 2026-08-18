---
name: caiven-idea
description: Generate and evaluate product/feature ideas for Caiven — fantasy-console capabilities, Studio creator workflows, example games, community features, cartridge discovery, education/onboarding, product differentiation. Use for open-ended "what should we build" / ideation requests, before any implementation work starts.
---

# caiven-idea

Ideation and evaluation only — this skill never implements. Hand off to
`/caiven-feature` only after an idea is approved.

## Idea categories

Fantasy-console capabilities, Studio creator workflows, example games,
community features, cartridge discovery, education/onboarding, product
differentiation.

## Every idea must include

- Target creator (who specifically benefits).
- User problem (concrete friction, not a hypothetical).
- Caiven-specific advantage (why this fits Caiven's real-Lua,
  no-royalties, own-your-game positioning — not generic engine advice).
- Smallest useful experiment (not the full feature).
- Estimated implementation surface (which crates/frontends, rough size).
- Risks.
- Validation method (how you'd know it worked).
- Success metric (tie to `docs/product/product-development-loop.md`
  candidate metrics where relevant).

## Discipline

- Avoid generating a large feature merely because it sounds impressive —
  prefer the smallest experiment that tests the real assumption.
- Ground ideas in the actual repo (check `docs/development/claude-code-audit.md`
  and current Lua API / Studio surface) rather than inventing capabilities
  that already exist or conflict with current architecture.
- Rank ideas, don't just list them — say which one you'd do first and why.
- Playground plugin (if installed) is useful for quick interactive
  exploration of a UI-facing idea before implementation.
