---
name: caiven-debug
description: Evidence-driven debugging for Caiven — reproduce, minimize, isolate the responsible boundary, add a failing regression test, fix root cause, verify, and record a durable invariant if the bug class could recur. Use for any bug report or crash, not for new-feature work.
---

# caiven-debug

Required sequence:

1. **Reproduce.** Get a concrete repro — a cart, a Studio action sequence,
   an API call — before touching any code.
2. **Minimize.** Cut the repro down to the smallest cart/input/steps that
   still trigger it.
3. **Identify the responsible boundary.** Which crate/module actually owns
   the faulty behavior — don't patch a symptom in a caller when the bug is
   in the callee (or vice versa). Cross-reference
   `docs/development/claude-code-audit.md`'s architecture map if the
   ownership isn't obvious.
4. **Add a failing regression test first** — it should fail before the fix
   and pass after. No regression test, no "fixed."
5. **Fix the root cause**, not the symptom. Avoid unrelated refactoring in
   the same change.
6. **Run nearby tests** — the crate's full test suite at minimum, plus any
   e2e coverage touching the affected flow.
7. **Record a durable invariant** in the relevant `.claude/rules/*.md` if
   this bug class could recur elsewhere (e.g. "cart format version field is
   currently unchecked" is exactly this kind of durable fact — see
   `.claude/rules/cart-format.md`).

## Notes

- If the bug is security-sensitive (see `.claude/rules/security.md`'s list
  of sensitive surfaces), treat the fix with that rule's extra scrutiny.
