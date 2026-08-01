---
paths:
  - "crates/caiven-vm/**"
  - "crates/caiven-machine/**"
---

# VM and frame loop

- Treat per-frame allocations as suspicious. `_update()`/`_draw()` run every
  frame; anything allocating there (new `Vec`, `String`, boxed closures)
  needs a reason, not a shrug.
- Preserve deterministic behavior where the API implies it (RNG seeding,
  fixed timestep, RTC). Don't silently change timing semantics
  (`src/timing.rs`, `src/vm/rtc.rs`) — a change there affects every cartridge
  that assumes current behavior.
- Keep host/runtime responsibility boundaries clear: `caiven-vm` owns
  execution, rendering, input, audio primitives; `caiven-machine` owns
  process/window lifecycle and hot-reload orchestration around a VM
  instance. Don't blur which crate owns which.
- Hot paths of note: `src/vm/lua_exec.rs` (Lua call dispatch),
  `src/vm/api_registry.rs` (builtin registration), `src/rendering/*`,
  `src/input/*`. Changes here warrant a benchmark comparison
  (`caiven-benchmark` skill) if the change could affect per-frame cost.
- Audio (`src/vm/audio.rs`, `sfx.rs`) runs adjacent to a real-time thread via
  `cpal` — never block or allocate unpredictably on that path; see
  `.claude/rules/security.md` for the sandbox-boundary angle.
