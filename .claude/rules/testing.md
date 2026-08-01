---
paths:
  - "**/tests/**"
  - "**/*.test.ts"
  - "**/e2e/**"
---

# Testing

- Add tests near the changed behavior, not a broad unrelated sweep.
- Rust: `crates/<crate>/tests/` for integration tests; keep unit tests
  inline with `#[cfg(test)]` next to the code they exercise.
- Frontend unit tests: `crates/caiven-studio-ui/tests/*.test.ts`
  (`node --experimental-strip-types --test`).
- E2E: Playwright in `crates/caiven-studio-ui/e2e` and
  `crates/caiven-port/web/e2e`. Studio and Port mocked suites run 3x in CI
  (`test:e2e:stress`) to catch flakiness — a new e2e test should be stable
  under repeat, not just pass once.
- Port's live e2e (`test:e2e:live`) hits a real backend — only required for
  behavior that genuinely can't be verified against mocks.
- A bug fix needs a regression test that fails before the fix and passes
  after (`caiven-debug` skill enforces this sequence).
- Don't delete or weaken an existing test to make a change pass; fix the
  actual regression.
