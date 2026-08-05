---
name: caiven-benchmark
description: Measure VM, graphics, map, audio, cartridge-loading, or editor performance in Caiven with a real before/after comparison. Use whenever a change claims or needs to verify a performance effect — never claim improvement without measurement.
---

# caiven-benchmark

No benchmark harness currently exists in the repo (no `criterion`, no
`benches/`, no `#[bench]` — confirmed via full-repo search during the
2026-08-01 audit). First real use of this skill should establish a minimal
one rather than hand-waving timing numbers.

## Required

1. **Baseline measurement** on the unmodified code, using a repeatable
   method (a `criterion` bench if you add one, or a hand-rolled timed loop
   run multiple times with reported variance — not a single anecdotal run).
2. Make the change.
3. **Comparable after-measurement** — same method, same machine, same
   input/cart, enough repetitions to see past noise.
4. Report both numbers and the delta, not just "it's faster."
5. Confirm correctness didn't regress — a benchmark change still needs
   passing tests.

## Likely-sensitive areas

VM frame loop (`caiven-vm/src/vm/lua_exec.rs`, `rendering/*`), map/tilemap
operations, cartridge loading/parsing (`caiven-cart`), Studio editor
responsiveness on large projects, audio callback timing
(`caiven-vm/src/vm/audio.rs` — must never block/allocate unpredictably on
the SDL2 audio callback thread).

## Do not

Claim "should be faster" or "this is clearly more efficient" without
running both sides. If measurement isn't feasible in the current session,
say so explicitly instead of asserting an unverified result.
