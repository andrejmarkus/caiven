---
paths:
  - "crates/caiven-vm/**"
  - "crates/caiven-machine/**"
  - "crates/caiven-cart/**"
---

# Performance

- Never claim a performance improvement without measurement — use the
  `caiven-benchmark` skill: baseline before, comparable measurement after,
  same methodology both times.
- Likely-sensitive areas: VM frame loop (`caiven-vm/src/vm/lua_exec.rs`,
  `rendering/*`), map/tilemap operations, cartridge loading/parsing
  (`caiven-cart`), Studio editor responsiveness on large projects.
- A performance fix must not regress correctness — keep or add tests
  alongside it.
- Prefer removing unnecessary per-frame allocation/copying over
  micro-optimizing an already-cold path.
