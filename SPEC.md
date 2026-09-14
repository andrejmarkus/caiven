# Product hardening

## §G
Harden existing Caiven product boundaries; preserve creator workflows; make release confidence repeatable.

## §C
- Existing Rust/Svelte/Tauri stack, cartridge v5, Lua API preserved.
- No production deployment, database migration, or signing claims without verification.
- Worktree initially clean; fixes backed by regression tests.
- Scope: cartridge ingestion, Lua bundling, service probes, container privileges, dependency/release gates, contributor operations.

## §I
- I.cart: `caiven_cart::{parse,load,write}` → valid v5 cart or `CartError`.
- I.bundle: `bundle_lua` → self-contained Lua; module names remain data.
- I.health: `GET /healthz` → process alive; `GET /readyz` → database reachable, bounded wait; JSON, no-store.
- I.release: existing CI/tag publishing; production artifacts require verification.

## §V
V1: cart input ≤ `MAX_CART_BYTES`; section payloads outside header/table, non-overlapping; reject before copying payloads.
V2: exactly one Program section; unknown sections and named-bank repetitions preserved; writer cannot emit rejected layout.
V3: module names containing quotes, backslashes, control characters remain literal strings; bundled code executes expected module only.
V4: Lua discovery ignores symlinks; directory cycles cannot recurse forever or include external files.
V5: health probes unauthenticated, uncached, no internal error disclosure; DB outage → readiness 503, liveness 200.
V6: runtime container non-root; writable SQLite directory explicit; healthcheck uses readiness.
V7: dependency audit includes bundled frontend devDependencies; release checks remain blocking.
V8: shipped WASM rejects oversized/overlapping carts and accepts valid cart after rejection.

## §T
id|status|task|cites
---|---|---|---
T1|x|bound cartridge parser; reject malformed tables; regression suite|V1,V2,I.cart
T2|x|escape Lua module names; skip symlinks; VM/discovery regressions|V3,V4,I.bundle
T3|x|add bounded service probes; non-root runtime; integration tests|V5,V6,I.health
T4|x|strengthen audit gates; rebuild/test WASM; document contributor operations; full verification|V7,V8,I.release

## §B
id|date|cause|fix
---|---|---|---
B1|2026-09-14|parser accepts overlapping payloads; file reader unbounded|V1,V2
B2|2026-09-14|module filenames interpolated as Lua source; discovery follows symlinks|V3,V4
B3|2026-09-14|npm audit omitted build dependencies; nanoid/PostCSS advisories hidden|V7
B4|2026-09-14|sandbox denied baseline socket binding; shared test lock poisoned|rerun with local socket access
B5|2026-09-14|test used NUL filename unsupported by Lua require and filesystem|test valid filename controls; keep byte-safe quoting
B6|2026-09-14|concurrent builds/browser workers timed out navigation and long UI flow|single-worker reruns; deadlines/assertions unchanged
B7|2026-09-14|Custom(1) serializes as Program despite distinct enum variant|V2; compare wire IDs in writer
