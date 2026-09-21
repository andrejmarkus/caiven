# Hardware redesign — Phase 2 and Phase 3 change list

Steering document: `docs/product/design-charter.md`. The charter's §4 table is
the target; this file is how the code gets there.

Each item below is a **separate focused diff with its own tests**. Nothing is in
production and no user has seen the console, so there are **no migration paths,
no deprecation periods, and no compatibility shims** — old behaviour is deleted,
not aliased. Existing documentation is updated in the same diff as the change it
describes.

Order matters where noted; otherwise items are independent.

## Phase 2 — hardware

### 2.1 Screen 128 × 128 → 192 × 128 — **done**

Landed; kept as the record of what the item covered. Two things the original
list did not anticipate: `--scale fit --aspect square` filled the window height
unconditionally, which pushed a 3:2 console off the sides of a 4:3 handheld
panel (now clamped to the width budget), and every cover-art container in
Studio and Port was `aspect-square`.

- `crates/caiven-core/src/memory.rs:28-30` — `SCREEN_WIDTH` 128 → 192. Derived
  bases follow automatically through `MemRegion::span`/`base`.
- Renderer and framebuffer sizing in `caiven-vm` (anything assuming a square
  screen or reusing `SCREEN_HEIGHT` for width).
- `caiven-studio-ui` `ConsolePane` canvas dimensions and integer-scale logic.
- `caiven-web` player canvas.
- `caiven-machine` window sizing and handheld scaling
  (`docs/development/handheld-builds.md:171` states 128×128 explicitly).
- Showcase carts: anything that centres by hard-coded 64.

Test: a cart drawing `draw_text` at x=0 fits 24 columns; a pixel at x=191 is
in-bounds and x=192 errors.

### 2.2 Map 64 × 64 → 128 × 128 — **done**

Landed; kept as the record of what the item covered. The thing the original
list did not anticipate: **32 KiB of map + collision does not fit the 64 KiB
flat address space.** The old layout filled 64 KiB exactly, and the two regions
needed +24 KiB against a 22.5 KiB heap. Resolved by an explicit decision from
the project owner — the addressable space grew to 96 KiB, with Work still
16 KiB and Heap now ~30.5 KiB, so "64 KiB RAM" now names the general-purpose
memory (Work + Heap) and the asset windows are mapped alongside it rather than
carved out of it. The charter's §4 RAM row and its "does not eat guest RAM"
justification were updated to say so.

- `crates/caiven-core/src/memory.rs` — `MAP_W`/`MAP_H` 64 → 128, both region
  spans 0x1000 → 0x4000, `RAM_SIZE` 64 → 96 KiB. Every base downstream shifted
  automatically; only the golden-base test needed new numbers.
- `crates/caiven-cart/src/project.rs`, `asset_png.rs`, `section.rs` — map PNG
  dimensions and doc comments follow the constants; no code change was needed
  beyond the prose.
- `crates/caiven-studio-ui/src/lib/ipc.ts` — new `MAP_W`/`MAP_H`/`MAP_LEN`/
  `TILE_SIZE`/`MAP_PX_*`/`SCREEN_TILES_*` exports, guarded by a new drift test
  in `crates/caiven-core/tests/memory_map_sync.rs`.
- `MapCanvas.svelte` and `Workspace.svelte` — every hard-coded `64` and `512`
  now derives from those exports, including the minimap and the scroll-viewport
  fractions.
- `MapCanvas.svelte` repaint cost: 4× the tiles made the full redraw ~23 ms in
  a dev build, and it ran on every pointer move of a paint drag. The canvas
  image is now kept between renders and a stroke repaints only the cells it
  touched (measured back down to ~10 ms per move on the tile layer). Editor
  responsiveness is Clock A, so this was fixed in the same diff rather than
  filed.
- `projects/showcase/platformer/main.lua` — rooms re-authored from 16 × 16 to
  24 × 16 tiles (`ROOM_TILES_W`/`ROOM_TILES_H`), which is what 2.1 deferred.

Note for later items: at this point the map was **not** a whole number of
screens wide (128 / 24 = 5.33), so the map editor's screen grid had a partial
right-hand column; the editor drew it honestly rather than hiding it. See
Phase 4 below — the owner later resized the map to remove this remainder
rather than live with it.

Guest RAM is untouched: map and collision live in their own regions.

Test: a tile written at (127, 127) lands, one at (128, 0) is dropped rather
than wrapping onto row 1, and the collision region starts past the map's end.

### 2.3 Palette redesign — **done**

Landed; kept as the record of what the item covered. Count stayed 16, so no
format changed — only the RGB values and the slot meanings moved.

- `crates/caiven-vm/src/vm/palette.rs` — `DEFAULT_COLORS` rebuilt to the
  charter's structure. Slot layout: 0 black, 1-12 four ramps (ember, moss, sky,
  stone) in dark → mid → light order, 13-14 the gold and magenta accents, 15
  white. The ordering is the teaching aid: the shadow of any color is the slot
  before it and the highlight is the slot after it, in every ramp. Three unit
  tests hold that property (ramps climb in brightness, black and white bound the
  range, shade tiers stay separated across ramps).
- `docs/brand-colors.md` was checked and is **brand-only** — logo, Port and
  Studio chrome. It does not document the console palette, so it did not change.
- `crates/caiven-studio-ui/src/lib/ipc.ts` — the browser-fallback swatches had
  **already drifted to PICO-8's exact palette**, so Studio-in-a-browser showed
  colors the VM never rendered. Corrected, and a new drift test
  (`crates/caiven-vm/tests/palette_sync.rs`) now parses the copy and fails on
  any future divergence. The same file still had `map: Array(4096)` left over
  from 2.2; it now uses `MAP_LEN`.
- `crates/caiven-vm/tests/lua_script.rs` — four tests about input and bounds
  asserted hard-coded palette RGB as sentinels and broke on the recolor. They
  now derive the expected pixel from `DEFAULT_COLORS` through a `slot_rgba`
  helper, so a future palette change cannot break assertions that were never
  about color.
- Carts re-indexed: `stdlib_demo` (showcase and dev, Lua plus `sprites.png`
  pixel indices and PLTE), `scenes_demo` (showcase and dev), and the dev carts
  `demo_string`, `demo_table`, `platformer_demo`, `sprite_flip_rotate`.

Not re-indexed, deliberately: `catch`, `movement`, `tiles`, `sprite`, `smoke`,
`audio_test`, `stdlib_all_modules`, `stdlib_core_only` and the showcase
`platformer` all overwrite their slots with `set_palette_color` in `_init`, so
their indices name their own colors and have no default slot to move to.

**Open question for the owner:** the showcase `platformer` replaces all 16 slots
with hand-picked colors. That predates a palette worth using, and re-authoring
it onto the default ramps would make the flagship cart actually show the new
palette — but it is an art decision on a showcase cart, not a mechanical
re-index, so it was left alone rather than decided mid-task.

Test: each ramp is monotone in brightness; the `ipc.ts` copy matches
`DEFAULT_COLORS` slot for slot.

### 2.4 Audio 8 voices → 6 typed voices — **done**

Landed; kept as the record of what the item covered.

- `crates/caiven-vm/src/vm/audio.rs` — `VOICE_COUNT` is now
  `MUSIC_VOICE_COUNT + SFX_VOICE_COUNT` = 4 + 2. `MUSIC_VOICE_CH0`/`CH1`,
  `LEGACY_SFX_VOICE` and `SFX_POOL_START`/`LEN` are gone, replaced by
  `MUSIC_VOICE_START`/`SFX_VOICE_START` and a `MUSIC_VOICE_KINDS` table fixing
  each music channel's timbre by column: pulse, pulse, triangle, noise.
- `VoiceKind::Triangle` is new, so the synth gained a third waveform. A unit
  test asserts it ramps between its extremes instead of stepping — a triangle
  that jumps is just a square with extra code.
- **`LEGACY_SFX_VOICE` deleted.** Studio's SFX-editor preview used to own a
  third voice class of its own; it now borrows an ordinary sfx voice through
  `play_sfx_voice`, tracked by a `preview_sfx` handle. With only two sfx
  voices, reserving a third for an editor would have spent a third of the
  console's polyphony on Studio. `Vm::sfx_player()` consequently returns a
  `SfxPlayer` snapshot rather than a `&SfxPlayer` — it reports the preview's
  voice, or an idle player once that voice finished or was stolen.
- `crates/caiven-vm/src/vm/sfx.rs` — `MusicPlayer`'s `ch0`/`ch1` became a
  `channels: [SfxPlayer; MUSIC_VOICE_COUNT]` array, and `pattern_row_base`
  strides by channel count instead of a hard-coded 2.
- **Format change:** a music row is now one byte per typed channel, so
  `MUSIC_BANK_LEN` went 256 → 512 (`8 patterns × 16 rows × 4 channels`) and
  `MemRegion::Music`'s span 0x100 → 0x200. Unlike 2.2 there was no squeeze:
  `Heap` absorbs the shift, since the address space grew to 96 KiB in 2.2.
  Every region above Music moved up by 0x100 (RTC to 0xC700, collision to
  0xC703, heap to 0x10703).
- The pattern shape is now single-sourced in `caiven_core::memory` as
  `MUSIC_PATTERN_COUNT`/`MUSIC_PATTERN_ROWS`/`MUSIC_CHANNEL_COUNT`, with
  `MUSIC_BANK_LEN` derived from them and `audio::MUSIC_VOICE_COUNT` taking the
  channel count from there — one number, not four.
- Existing music data was converted rather than dropped: each old row `[a, b]`
  became `[a, 0, 0, b]`. Old channel 0 was forced Square and old channel 1
  forced Noise, so this puts each one in the new column with the same timbre.
  Affects `projects/dev/catch`, `projects/showcase/catch` and
  `projects/showcase/platformer`; their `.cav` binaries were rebuilt.
- Studio's tracker went to four typed columns in this diff rather than waiting
  for 3.3. Leaving it at two would not have been "unchanged" — the row stride
  moved, so a two-column tracker would have written into the wrong rows. The
  deeper tracker work (step/pattern copy-paste, song order, chaining) is still
  3.3's.
- `crates/caiven-studio-ui/src/lib/ipc.ts` — the audio bank shape is now
  exported as constants and guarded by a new drift test in
  `memory_map_sync.rs`, the same prevention pattern 2.2 and 2.3 established.
  Two more stale copies in that file were found and fixed while there:
  `sfx: Array(1024)` and `ram: Array(65536)` (the latter wrong since the
  address space grew in 2.2).

Test: `play_sfx_does_not_disturb_concurrent_music_playback` now fills every
music channel, then makes more concurrent `play_sfx` calls than there are sfx
voices, and asserts no music channel went silent — the reserved-voice promise
stated as an executable claim.

**Noted, not fixed:** `projects/showcase/platformer`'s music references SFX ids
47 and 7, but a bank only has 16 slots. That data was already out of range
before this change — the conversion preserved it exactly — so the cart's music
was reading past the SFX bank both before and after. Fixing it means
re-authoring the platformer's music, which is an art decision on a showcase
cart, not part of this item.

### 2.5 Named banks — **done**

Numeric ids removed, not aliased.

- `crates/caiven-vm/src/vm/lua_exec.rs` — the five `load_*_bank` builtins
  take a string name.
- `crates/caiven-vm/src/vm/mod.rs` — `AssetBanks`'s bank maps are keyed by
  `String` instead of `u8`; the Map → Collision companion rule follows the
  name. The reserved name `"default"` (`caiven_cart::DEFAULT_BANK_NAME`) is
  the bank that auto-loads at boot — it can be switched back to like any
  other name but never created or removed (`create_asset_bank`/
  `remove_asset_bank` reject it, mirroring the old id-`0` special case).
- `crates/caiven-cart/src/section.rs` — payload changed from `[bank_id
  u8][data…]` to `[name_len u8][name][data…]`. `is_valid_bank_name` (also
  exported) gates both the encoder and decoder: 1-31 ASCII letters, digits,
  `_`, or `-`. `decode_asset_bank` is the untrusted-input boundary — a
  truncated section, non-UTF-8 name bytes, or a name failing the charset
  all return `None` rather than panicking, and the charset excludes `.`,
  `/`, `\`, so a validated name can never escape a directory when later
  joined into a project-file path.
- **Format version bumped 3 → 4** (`crates/caiven-cart/src/format.rs`),
  `MIN_SUPPORTED_CART_VERSION` raised to 4. A pre-2.5 cart's numeric bank id
  would misparse as a name length under the new decoder, so old carts are
  rejected outright with `CartError::UnsupportedCartVersion` rather than
  silently misread — no migration exists because nothing is in production.
  All showcase/dev `.cav` files rebuilt via `scripts/demo-carts/build.sh`.
- `crates/caiven-cart/src/project.rs` — project filenames
  `sprites_1.png` → `sprites_forest.png`. The read side no longer scans
  `1..=u8::MAX` for candidate ids; it lists the project directory once per
  bank kind and keeps filenames matching `{stem}_{name}.{png,hex}` where
  `name` passes `is_valid_bank_name` (`bank_file_name`) — the same gate, so
  a hand-placed file with a path-traversal or oversized name is silently
  skipped rather than loaded.
- `crates/caiven-vm/src/vm/api_registry.rs` — signatures and doc strings.
- Studio: `tauri_app.rs`'s `AssetBankPayload`/`BootstrapPayload`/
  `TickPayload` fields and the `studio_asset_bank` Tauri command switched
  from `Vec<u8>`/`u8` to `Vec<String>`/`String`; `"create"` now requires the
  caller to supply a name (no more auto-picking the lowest free id) and
  validates it with `is_valid_bank_name` before touching the VM. Frontend
  (`Workspace.svelte`) prompts for a name on bank creation, validated
  client-side against the same charset before the round-trip; the bank
  `<select>` and delete button work off names, and delete is disabled on
  `"default"`.

The default bank stays auto-loading, so a cart that never calls `load_*_bank`
is unaffected.

### 2.6 Drop `dset` / `dget` — DONE, committed on `master`.

`save_data`/`load_data` is the one obvious way; the numeric-slot pair is
gone. Removed the `dset`/`dget` builtins from `lua_exec.rs`'s
`BUILTIN_NAMES` and `register_builtins`, and their entries from
`api_registry.rs`. `save_data.rs`'s `SaveData` dropped the 64-slot
`[f64; 64]` array and `get_slot`/`set_slot` entirely — nothing outside the
removed builtins used the numeric store, so there was no reason to keep it
alongside the blob. `SaveDataError::SlotOutOfRange` went with it, leaving
only `BlobTooLarge`.

The on-disk `SaveData::encode`/`decode` format lost its slot section
(`[magic][version][64×f64 slots][blob_len][blob]` →
`[magic][version][blob_len][blob]`), so `FORMAT_VERSION` bumped 1→2 —
otherwise an old file's leftover slot bytes would misparse as the blob
length. `decode` rejects any other version outright rather than
attempting a partial read; nothing is in production, so there's no
migration path, matching the same policy already applied to the cart
format in 2.5. `caiven-machine`'s and `caiven-studio`'s disk-persistence
tests (which had been round-tripping a value through `set_slot`/`get_slot`
as their marker) were rewritten to round-trip through the blob instead.

`docs/api-reference.md`'s Persistent Data table dropped the `dset`/`dget`
rows; the System Specifications pending-note now lists item 2.6 as landed.
No Studio Lua autocomplete list needed touching — Studio doesn't hardcode
a builtin-name list; it has no separate copy to drift.

### 2.7 Per-frame execution budget — DONE, committed on `master`.

Both real gameplay paths install an mlua `every_nth_instruction` count hook
in `crates/caiven-vm/src/vm/lua_exec.rs`: plain `run_frame_lua` (used by
`caiven-machine` and `caiven-studio`'s Port preview) and `run_frame_lua_bp`
(Studio's breakpoint-aware run, which previously only installed a hook —
`EVERY_LINE` — when breakpoints existed; the budget hook now always runs
there too, with `EVERY_LINE` layered on top only when breakpoints are set,
since both share one `lua.set_hook` call and are told apart via
`debug.event()`). The hook counts in strides of `INSTRUCTION_HOOK_STRIDE`
(10,000) and aborts the call once `FRAME_INSTRUCTION_BUDGET` (25,000,000)
is exceeded — chosen as generous headroom over a normal frame's real
instruction count while still bounding a runaway script to a fraction of a
second, confirmed by a test where an unguarded `while true do end` used to
hang the test process forever and now returns in ~50ms.

On trip, the hook captures source+line the same way the breakpoint hook
already did (factored into a shared `hook_debug_source` helper) and returns
a plain-language message — "your game did not finish drawing this frame —
is there a loop that never ends?" — instead of forwarding the raw `mlua`
error, per the charter's "not a Lua traceback" requirement. `VmFault` grew
an `ExecutionBudgetExceeded` variant, kept distinct from generic `LuaError`
so a host can eventually tell "your script has a bug" apart from "your
script never yields"; `LuaRunOutcome::Error` (already wired end to end to
Studio's Runtime-error diagnostic/pause UI) carries the same location and
message for the breakpoint-aware path. No `unwrap`/`expect`/panic on the
fault path — the location is `Option`-wrapped exactly like an ordinary
Lua-error location when mlua can't resolve a current line.

Tests added in `crates/caiven-vm/tests/lua_script.rs`:
`run_frame_aborts_an_infinite_loop_instead_of_hanging` (regression test —
would hang forever pre-fix) and
`run_frame_lua_bp_reports_execution_budget_exceeded_with_line`. An existing
test's stale comment (claiming plain `run_frame()` "never wires the hook at
all") was corrected to describe the new budget-only hook.

### 2.8 Optional `w` / `h` on `sprite()` — DONE, committed on `master`

`sprite()` gained two optional trailing args, `w` and `h`, in sprite units
(not pixels), defaulting to `1, 1`. `sprite_id` is the block's top-left
sprite; the rest of the block is filled from adjacent sheet slots, row-major
at the sheet's fixed 16-sprite width (`SPRITE_SHEET_COLS`, new constant in
`crates/caiven-core/src/memory.rs`, matching Studio's `.sprite-sheet` CSS
grid). This is the tooling half of the 8×8 decision: a 16×16 hero draws in
one call while the hardware stays 8×8.

Implementation in `crates/caiven-vm/src/vm/lua_exec.rs`'s `sprite` builtin:
the per-pixel loop now walks a `bw × bh` source block (`bw = sprite_size *
w`, `bh = sprite_size * h`) instead of one sprite's `ss × ss`; each source
pixel's tile is `sprite_id + tile_row * SPRITE_SHEET_COLS + tile_col`. The
existing rotate/flip math generalizes unchanged from the single-tile case to
the whole block — rotate 90/270 swaps the block's output width and height,
same as it swapped a single sprite's square before.

Two argument errors, both deliberate Lua errors (not silent no-ops, matching
the existing `rotate` argument's own error style): `w`/`h` below `1`, and a
block that would run past the sheet edge (checked against the sheet's fixed
16 columns × 16 rows before drawing anything, so the fault path never reads
past the loaded bank).

Fully additive — `w`/`h` are trailing optional args after the existing
`rotate`, so every existing `sprite(id, x, y, ...)` call (including the
prelude's `vec2.lua` sprite wrapper) is untouched. No cart-format change, no
compat break.

`api_registry.rs`'s `BUILTINS` doc entry updated to match; Studio's
autocomplete pulls from it directly (`tauri_app.rs`), so no separate
frontend copy to update. `docs/api-reference.md`'s `sprite()` row documents
the new args and error behavior inline. `projects/dev/sprite_flip_rotate`
(the existing flip/rotate demo cart) gained a `w`/`h` demo line.

Tests added in `crates/caiven-vm/tests/lua_script.rs`:
`lua_sprite_w_h_draws_adjacent_sheet_tiles` (checks the two-tile block reads
the right sheet slots), `lua_sprite_w_h_past_sheet_edge_errors`, and
`lua_sprite_w_h_below_one_errors`.

### 2.9 T2 module split — `collision` — DONE, committed on `master`

Charter Appendix A: `collision.lua` was 147 lines, the only breach of the
readable-lesson cap, caused by `move_and_collide` and its slope solver
(lines 80-147).

Split, not cut. `crates/caiven-vm/src/vm/prelude/collision.lua` now keeps the
6 predicates only — `aabb_overlap`, `circle_overlap`, `point_in_rect`,
`point_in_circle`, `tile_solid`, `box_touches_solid` — 38 lines. The
swept-movement solver (`move_and_collide` plus its local helpers
`solid_blocks_column`, `solid_blocks_row`, `slope_floor_y`) moved verbatim
into a new `crates/caiven-vm/src/vm/prelude/movement.lua`, 108 lines when split;
it has since grown to 229 lines (134 non-blank, non-comment), over the
charter's "roughly ≤ 100". Neither file's logic
changed — this is an organizational split, not a rewrite.

`crates/caiven-vm/src/vm/lua_exec.rs`'s `PRELUDE_MODULES` gained a
`"movement"` entry (`globals: &["move_and_collide"]`) right after
`"collision"`, whose own `globals` list dropped `"move_and_collide"`. A cart
that wants swept movement now declares `"movement"` in `[stdlib] modules`
alongside (or instead of) `"collision"`.

Breaking change, deliberately not migrated (nothing is in production): any
cart declaring `[stdlib] modules = ["collision", ...]` and calling
`move_and_collide` must add `"movement"` to that list, or the call resolves
to Lua `nil` and errors on first use — the standard "missing module" failure
mode already documented for every other `[stdlib]` entry, not a new error
path. Updated `caiven.toml` for the two carts that call it:
`projects/dev/platformer_demo` and `projects/showcase/platformer` (both
add `"movement"` next to `"collision"`), and `projects/dev/stdlib_all_modules`
(declares every module, so it gained `"movement"` too). Neither project ships
a packed `.cav`, so no rebuild step was needed.

`docs/api-reference.md`'s Gameplay stdlib section gained a `### movement`
table entry with `move_and_collide`'s row, moved out of the `### collision`
table. Studio's autocomplete pulls prelude module/global lists live from
`lua_exec::prelude_module_globals()` (via `prelude_entry_module` in
`api_registry.rs`), so no separate frontend source of truth needed updating
there — but two hardcoded browser-preview mocks of the same shape existed
(`crates/caiven-studio-ui/src/lib/ipc.ts`'s `preludeModules` fallback, and
its e2e-test twin in `crates/caiven-studio-ui/e2e/fixtures.ts`) and both
needed a `"movement"` entry added by hand — the "frontend copies of console
constants drift silently" trap from 2.3, present here too.

Tests: `crates/caiven-vm/tests/lua_script.rs`'s shared `make_vm()` test
helper (opts every cart into every module) gained `"movement"`, so the six
existing `move_and_collide_*` tests kept passing unmodified against the
split. New `collision_and_movement_are_split_modules` test in
`crates/caiven-vm/tests/prelude_modules.rs` asserts each module exposes only
its own globals when declared alone (`collision` alone: `aabb_overlap`
defined, `move_and_collide` nil; `movement` alone: the reverse) — the
regression that would catch the split ever partially reverting.

## Phase 3 — editors (Clock A)

Unlimited budget applies here. The map editor is already near TIC-80 parity
(zoom, pan, minimap, stamps, select/copy/paste, 64-deep undo, collision type
manager) — it needs finishing, not building. The **sprite editor is the weak
surface** and is the priority, because it is where the 8×8 hardware decision
gets paid for.

### 3.1 Sprite editor catch-up (priority) — **done**

`SpriteCanvas.svelte` and `Workspace.svelte`'s sprite screen. Was missing:
marquee select, copy/paste between slots, an N × N group canvas spanning
adjacent 8×8 slots, zoom, vertical flip, counter-clockwise rotate.

- `SpriteCanvas.svelte` generalized from a fixed 8×8 canvas to `cols`/`rows`
  props (in 8px slots) plus a `zoom` prop, so it can render either a plain
  sprite or a composed N×M group at any display scale. Gained a `'select'`
  tool mirroring `MapCanvas.svelte`'s marquee (same anchor/current-drag
  pattern, same `onSelectionChange` callback shape).
- `crates/caiven-studio-ui/src/lib/editorMath.ts` gained the pure, unit-tested
  math the feature needed: `regionFromPoints`/`regionValues`/`pasteRegion`
  (marquee selection and clipboard paste, generalized from `MapCanvas`'s
  inline region math), `flipHorizontal`/`flipVertical`/`rotateClockwise`/
  `rotateCounterClockwise` (generic over width/height, not hardcoded 8×8),
  and `composeGroup`/`decomposeGroup` (splits/joins an N×M block of adjacent
  sprite-sheet slots into one flat pixel grid for the group canvas).
- `Workspace.svelte`: `spriteGroup` state tracks the selected group size;
  `groupSprite` composes it from `spriteSheet` on every render.
  `commitGroup` replaces the old single-sprite `commitSprite` — it
  decomposes an edited group canvas back to per-slot writes and records
  *one* history entry covering every touched slot, so undo/redo restore
  the whole group atomically. Rotate is disabled in the toolbar when the
  group isn't square (`groupCols !== groupRows`) — a non-square block would
  need a differently-shaped canvas after rotating, which the fixed slot
  grid can't hold. A drag across the sprite-sheet panel (mirroring the
  existing tile-picker marquee) picks the group; a plain click is a 1×1
  group, unchanged from before this landed.
- `ipc.ts` gained `SPRITE_COUNT`/`SPRITE_SHEET_COLS`, mirroring
  `caiven_core::memory`; a new `crates/caiven-vm/tests/sprite_sheet_sync.rs`
  drift test parses `ipc.ts` and fails if either constant goes out of sync —
  the same "frontend copies of console constants drift silently" trap named
  in 2.3 and 2.9, guarded before it could recur here.
- Purely additive to the sprite-editor UI and the `editorMath.ts`/`ipc.ts`
  surface; no cart format, save format, or Lua API change, so no
  compatibility note is needed.
- Tests: 7 new `editorMath.test.ts` cases cover the new pure functions
  directly (region math, flip/rotate, compose/decompose round-trip). A
  Playwright case (`studio.spec.ts`) exercises the real flow: paint, flip,
  flip vertically, rotate counter-clockwise, undo ×3, marquee-select a
  corner and copy/paste it, drag a 2×2 group on the sheet panel and confirm
  the canvas becomes `16 by 16`, then zoom to 200%. Also fixed an unrelated,
  pre-existing stale assertion in that same test (`'64 by 64 tile map'`,
  left over from before 2.2's map resize to 128×128) that blocked running
  it at all.

### 3.2 Map autotile and selection ops — **done**

`MapCanvas.svelte` and `Workspace.svelte`'s map screen. Was missing: terrain/
autotile brushes, reusable named stamps, in-place paste, move/rotate/flip a
selection, rectangle outline.

- **Rectangle outline.** `editorMath.ts` gained `rectangleOutline` (border
  cells of the box `filledRectangle` would fill — PICO-8's rect/rectb split)
  and a `'rect-outline'` entry on `StrokeTool`/`MapTool`, wired into
  `MapCanvas.svelte` exactly like `'rect'` (live preview rebuilt from anchor
  each move). A new toolbar button sits next to the existing tools.
- **Terrain/autotile.** A terrain is a convention, not stored state: 16
  consecutive tile ids `base..base+15` (`base` a multiple of 16), and tile 0
  never belongs to any family — it's the map's "nothing here" value, so a
  terrain can't start at id 0-15's own base without losing its "isolated"
  variant; the in-editor hint says so. `editorMath.ts` gained
  `autotileBitmask` (cardinal N/E/S/W same-family neighbor bits) and
  `autotileEdits` (recomputes a just-painted seed cell and its neighbors'
  variants, touching only cells already in that seed's terrain family — an
  adjacent unrelated terrain is left alone). The new `'autotile'` tool paints
  like pencil (`strokeCells` bridges it identically), and `Workspace.svelte`'s
  `commitMap` runs `autotileEdits` over the post-stroke map before building
  the history entry, so painting and its neighbor recompute land as one
  atomic undo step. Only active for the `'autotile'` tool — the plain eraser
  does not re-run neighbor recompute, a known, documented limitation rather
  than an attempt to guess a painter's intent from tool-agnostic edits.
- **Move / rotate / flip a selection.** Reuses the sprite editor's own
  transform helpers (`flipHorizontal`/`flipVertical`/`rotateClockwise`/
  `rotateCounterClockwise`, already generic over width/height from 3.1) via a
  new `transformMapSelection` in `Workspace.svelte`, applied to
  `mapSelection`'s region and written back with `pasteRegion`. Rotate is
  disabled when the selection isn't square, same rule and same reasoning as
  3.1's sprite-group rotate. Move is four nudge buttons (arrow icons), not a
  canvas drag — `editorMath.ts`'s new `moveRegion` relocates a region and
  clears only the vacated cells the destination doesn't itself cover (so an
  overlapping move never leaves a stray copy), and the selection follows the
  moved content so repeated clicks walk it across the map.
- **In-place paste.** `pasteInPlace` (Ctrl+Shift+V, or a toolbar button)
  writes the clipboard straight into the current selection's top-left corner
  in one commit — no click-to-place needed, the alternative to the existing
  click-to-place stamp paste from 3a when the destination is the active
  marquee itself.
- **Reusable named stamps.** A `mapStampLibrary` list in `Workspace.svelte`,
  session-local (not part of the cart) — the current stamp or selection can
  be saved under a name (`window.prompt`, mirroring 2.5's bank-naming
  pattern) and reloaded or deleted from a small chip list under the tile
  picker. Kept session-local rather than plumbed through a new cart section:
  the plan item doesn't call for surviving a restart, and giving stamps cart
  persistence would need its own named-bank-style diff (new section kind,
  Tauri commands, format version bump) disproportionate to what "reusable"
  asks for here.
- Purely additive to the map-editor UI and the `editorMath.ts` surface; no
  cart format, save format, or Lua API change, so no compatibility note is
  needed.
- Tests: 7 new `editorMath.test.ts` cases cover `rectangleOutline`,
  `moveRegion` (including an overlapping move), `autotileBitmask`, and
  `autotileEdits` (including that an unrelated terrain family is left
  alone). A `studio.spec.ts` case extends the existing art/map flow: pick a
  terrain tile (id 17 — outside the reserved 0-15 block), paint two adjacent
  cells with the autotile tool and assert the resulting variants via
  `e2e.snapshot()`; paint and verify a rectangle outline's border-vs-interior
  cells; marquee-select the outline, flip, rotate, nudge, paste-in-place, and
  save/reload it as a named stamp. Found along the way: `dialog.accept()`
  with no argument submits an *empty string*, not the native prompt's
  default value — the test passes `dialog.defaultValue()` explicitly. This
  also means the pre-existing, already-broken `banking.spec.ts` "Create
  bank" step (no dialog handler at all, one of the 8 failures pre-dating
  this item) would still be broken by this fact alone even once its earlier,
  unrelated assertion is fixed — left alone, out of scope for this item.

### 3.3 Music / SFX gaps — **done**

`97f037d` (2026-08-23): each music bank gained a 33-byte song section (32-step
order table + loop-point byte; `MUSIC_BANK_LEN` 512 → 545, `MemRegion::Music`
span 0x200 → 0x300, RTC/Collision/Heap bases shifted accordingly). Order/loop
bytes reuse the existing "value+1, 0=empty" convention, so every pre-existing
bank zero-pads into a valid empty song — no migration or cart-format version
bump. New Lua builtin `play_music_song(start_step)` plays through the order
table honoring the loop point once (no chained loop-search). The Studio
tracker gained row-range copy/cut/paste (Ctrl+C/X/V), whole-pattern
copy/paste, and a song-order editor (32 click-to-cycle steps, one-at-a-time
loop-point toggle, play-song transport) — the order table *is* this plan
item's "pattern chaining": a song is a sequence of pattern-step references,
not a separate chaining primitive.

### 3.4 Keyboard-first editing — **done**

`feb0d95` (2026-08-23): arrow-key cursor, Enter/Space paint, Escape-cancel
added to `SpriteCanvas.svelte` and `MapCanvas.svelte`, the two mouse-only
raw-canvas editors — pointer begin/move/finish refactored into offset-based
`beginAt`/`moveAt`/`finishAt` shared by pointer and keyboard input. Tracker
cells were already native `<button>`s (keyboard-operable via default focus
behavior) and out of scope. This closed Phase 3.

## Phase 4 — map width

### 4.1 Map 128 × 128 → 192 × 128 — **done**

Approved 2026-08-24 by the project owner: 2.2's map was ~10.7 screens
(128/24 = 5.33 screens wide × 128/16 = 8 screens tall), so the map editor's
screen grid always had a partial trailing column on the right (see 2.2's
note above). Rather than live with the remainder, the owner chose to widen
the map to a whole number of screens in both directions — 8 columns to match
the existing 8 rows — instead of narrowing it to 120 (5 columns) or leaving
it as documented behavior.

- `crates/caiven-core/src/memory.rs` — `MAP_W` 128 → 192 (`MAP_H` unchanged
  at 128). `Map` and `Collision` region spans 0x4000 → 0x6000 each (both
  exactly match the new `MAP_LEN`/`COLLISION_LEN`, no padding). `RAM_SIZE`
  96 KiB → 112 KiB — chosen so Heap keeps its exact prior size (30717 bytes);
  every base past `Map` shifted (Palette 0xC000 → 0xE000, Sfx 0xC100 →
  0xE100, Music 0xC500 → 0xE500, Rtc 0xC800 → 0xE800, Collision 0xC803 →
  0xE803, Heap 0x10803 → 0x14803). Golden-base test updated to match.
- `crates/caiven-studio-ui/src/lib/ipc.ts`, `drawerMath.ts` — `MAP_W`,
  `RAM_SIZE`, and the shifted `MEMORY`/`MEMORY_REGIONS` bases, caught by
  `crates/caiven-core/tests/memory_map_sync.rs` (the same drift guard from
  2.2/2.3/2.9 — no manual hunting needed, just fix until it's green).
- `docs/api-reference.md` — map spec row and the full memory-map address
  table, both covered by the same drift test.
- `crates/caiven-vm/tests/lua_script.rs` — `map_bounds_and_collision_companion_size`
  golden values moved from the (127, 127) corner to (191, 127); the
  off-map-write probe moved from x=128 (now a valid column) to x=192.
- `projects/dev/stdlib_demo/map.png` and its `projects/showcase/stdlib_demo/`
  copy — the only cart in the repo with a hand-painted map bitmap on disk.
  `asset_png.rs` rejects a mismatched-size map PNG on load, so packing this
  project would hard-fail until the asset was regenerated. Reflowed with a
  throwaway example (`png`/`caiven_cart::text` — write, run, delete, don't
  keep it) that decodes the old 128×128 grayscale image, copies each row into
  the left 128 columns of a new 192×128 image, and leaves the new 64 columns
  at tile 0. Its `collision.hex` companion (trimmed hex, `caiven_cart::text`)
  was reflowed the same way, row by row, since a flat byte offset means a
  straight append would have scrambled every row past the first. All 20 demo
  carts (`scripts/demo-carts/build.sh`) rebuilt clean afterward — every other
  cart builds its map procedurally in Lua or has none, so only this one asset
  needed touching.
- `crates/caiven-studio-ui/e2e/studio.spec.ts` — every `row * 128 + col`
  golden tile-offset formula and the `'128 by 128 tile map'` accessible name
  (driven by `MapCanvas.svelte`'s `aria-label`) moved to 192.
- `crates/caiven-studio-ui/e2e/fixtures.ts` — its mock `offsets`/`lengths`/
  `COLLISION_OFFSET`/`COLLISION_LEN` still had 2.2-era values (`map: 4096`,
  `COLLISION_OFFSET: 0x9703` — a pre-2.2 address, and the pre-2.3 palette/sfx/
  music bases) even before this item; updated to the current real addresses
  and lengths while touching this file for the row-stride fix. Its unrelated
  `studio_frame` mock buffer is still hardcoded to a pre-2.1 `128 * 128` —
  left alone, not something this item's tests exercise or this item broke.
- `docs/product/design-charter.md` §4 — Map row's dimensions and "~42
  screens" figure corrected to 192×128 / "exactly 8 × 8 screens (a whole
  number in both directions, no partial trailing column)"; RAM row's total
  addressable space corrected to 112 KiB.

`CART_FORMAT_VERSION` was bumped 4 → 5 for this change (`format.rs`): the
map/collision assets are also checked against `MAP_W`/`MAP_H`/`COLLISION_LEN`
at load time, so an old cart with a 128-wide map is rejected with a readable
message rather than silently misloading.

Test: a tile written at (191, 127) lands, one at (192, 0) is dropped rather
than wrapping onto row 1, and the collision region still starts exactly past
the map's new end.

## Verification per phase

- `scripts/claude/check-rust.sh` after each Phase 2 item.
- `scripts/claude/check-cart-compat.sh` after 2.2, 2.4, and 2.5 (format changes).
- `scripts/claude/check-lua-api.sh` after 2.5, 2.6, 2.8, and 2.9.
- `scripts/claude/check-studio-ui.sh` after any Phase 3 item.
- `scripts/claude/pre-commit-gate.sh` as the final pass only.
- Every item: the showcase carts still run, and the numbers quoted in
  `README.md`, `docs/api-reference.md`, and `docs/product/design-charter.md`
  match the constants in `memory.rs` / `audio.rs`.
