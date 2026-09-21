# Caiven Design Charter

This document is binding. It settles what Caiven is, what its hardware is, and
which APIs are allowed to exist. It is not a wish list and not a roadmap — it is
the rule that every future feature argument is decided against.

A summary of this charter lives in `CLAUDE.md` so it loads into every session.
Where the two disagree, this document wins; keep them in sync.

## 1. Positioning

Caiven is a fantasy console: a small, fixed, imaginary machine you write games
for in real Lua 5.4. It is not a game engine, and the difference matters. An
engine grows to meet whatever you ask of it. A console does not — its limits are
the product, because a bounded machine is one you can finish a game on.

It ships in an era where writing code by hand has become optional. That is the
opportunity, not the threat. Caiven is where you still type it yourself: a place
to keep the part of your mind that composes logic in working order, and a place
where a beginner can acquire that part in the first place.

**Caiven ships no in-product LLM.** The console is the human-craft antidote to
the AI era. This is positioning, not an omission — deferred, not refused
forever, but reversible only by an explicit recorded decision, never by a
judgement call mid-task.

## 2. The spine — two clocks

Caiven holds two goals that pull in opposite directions:

1. **Brain gym.** You write the game yourself. Typing the code is the product,
   not a cost to be optimised away.
2. **Idea to playable in one sitting.** The audience has no patience.
   Time-to-fun is the metric that breaks every tie.

Taken naively, goal 2 argues for helpers that write the game for you, which
destroys goal 1. The resolution is to notice that these two goals measure
different clocks.

| Clock | What it measures | Rule |
| --- | --- | --- |
| **A — friction** | Everything *around* the code: boot, reload, sprite/map/sfx editing, templates, defaults, error messages, export | Drive to zero. Spend engineering freely. No simplicity budget applies here. |
| **B — authorship** | The game logic itself | Stays hand-typed. Small API, no autopilot, no framework that writes the game's structure for you. |

**The gate, in one sentence: does this remove friction, or does it remove
authorship?**

Friction is bought freely — there is no budget, no "that's enough polish", no
argument that a smoother editor is scope creep. Authorship is never traded for
speed, no matter how much time the trade would save. Speed comes from the
tooling being excellent, never from the API doing the game for you.

## 3. Audience

Two audiences, one API — deliberately, not as a compromise.

- **The absolute beginner.** Caiven teaches programming through something fun
  and non-violent, and it teaches *real* Lua, so the knowledge transfers. This
  is also the base skill for pair-programming with an AI later: you cannot
  supervise code you have never written.
- **The working programmer.** A gym. You keep the composition muscle from
  atrophying, on a machine small enough that a session ends in a finished thing
  rather than a backlog.

One API serves both because the beginner's needs and the gym's needs point the
same way: a small surface, readable names, immediate visible results, and no
black boxes. Anything that would help one at the other's expense is the wrong
design for both.

Assume the reader has no attention to spare. Every ritual before the first pixel
is a place the audience leaves.

## 4. Fixed console, expandable cartridge

Once the redesign phases land, **the console stops changing.** You never get a
bigger screen, more colors, or more voices. Growth happens exclusively through
the cartridge, via named banks.

Target hardware:

| Spec | Value | Why |
| --- | --- | --- |
| **Screen** | 192 × 128 (24 × 16 tiles) | 48 text columns (3 px glyph + 1 px gap) fits a real sentence; 128 px gives 32 and breaks a beginner's first `draw_text`. 3:2 suits side-scrollers. 1.5× the tiles keeps scarcity intact. Unique among fantasy consoles. |
| **Palette** | 16 colors, hand-designed | More colors means more sprite-editor time, and one-sitting is the metric. Identity comes from *which* 16: 4 hue ramps × 3 shades + black + white + 2 accents, so shading works without color theory. |
| **Sprites** | 8 × 8, 256 per bank | 16×16 costs 4× the art time per sprite — the wall a no-patience beginner hits in the first ten minutes. Four 8×8 sprites make a 16×16 hero. The pain is paid off in tooling, not hardware. |
| **Map** | 192 × 128 tiles + collision layer | At 192×128 a 64×64 map is ~10 screens, which one platformer level exhausts in a sitting — the exact failure the spine forbids. 192×128 is exactly 8 × 8 screens (a whole number in both directions, no partial trailing column) and ~64 screens total, generous for a full level. |
| **Frame rate** | 60 Hz fixed | Non-negotiable for game feel. |
| **Audio** | 6 voices: 4 typed music (2 pulse, 1 triangle, 1 noise) + 2 dedicated sfx | Typed channels make the tracker four scannable columns and answer "which channel?" by timbre. Reserved sfx voices mean a jump sound can never cut the melody — the most confusing audio bug a beginner meets. Classic consoles stole channels; authenticity loses to one-sitting. |
| **Input** | 4 directions + 2 actions + Select; START reserved | Retro-correct, works on handhelds, and spares every cart a pointer-input branch. |
| **RAM** | ~45 KiB general purpose (Work 16 KiB + Heap ~29 KiB), reached from Studio's memory view, not from Lua (no `peek`/`poke`) | Screen, map and collision occupy their own regions outside it, so widening them does not eat the memory a cart writes its own data into. Total addressable space is 112 KiB. |
| **Save** | one blob (`save_data` / `load_data`) | Two save APIs violate "one obvious way". The blob is table-shaped, real Lua, and transferable. |
| **Watchdog** | per-frame execution budget | An infinite loop must fail with a line number and a plain-language message, not hang the console. |

### Banking

Banking is the only expansion axis, and it is deliberately boring.

- **Banks are named, not numbered.** `load_sprite_bank("forest")`, not
  `load_sprite_bank(2)`. Self-documenting, consistent with the
  long-descriptive-name rule, and it turns a hardware concept into a readable
  one. Numeric ids are dropped rather than kept as an alias — one obvious way.
- **Bank count is unbounded.** The 128 KiB packed cart cap is the real ceiling.
- **Banking is invisible until needed.** The default bank auto-loads, so a
  beginner finishes their first game without hearing the word. It appears in the
  docs only where a cart outgrows one sheet — the same layering as the optional
  size arguments on `sprite()`.

## 5. API tiers

| Tier | What | Rule |
| --- | --- | --- |
| **T0 — builtins** (Rust) | Hardware access: gfx, sprites, map, input, audio, storage, time | Only what *cannot* be written in Lua. If it can be Lua, it is not a builtin. |
| **T1 — prelude core** (always on) | `lerp`, `clamp`, `random_range`, easing | Math-shaped, no game structure. Stays tiny. |
| **T2 — opt-in modules** (`caiven.toml`) | `vec2`, `collision`, `tween`, `particles`, `scenes`, `entities`, `camera` | **Readable-lesson cap**: pure Lua, roughly ≤ 100 lines, source readable in Studio, understandable in one sitting. |

The T2 cap is how the two clocks coexist. A helper you could have written
yourself, and can actually read, accelerates you without taking authorship away.
One you cannot read has taken it. A T2 module is a teaching example, not a black
box — the line count is a proxy for "you can read this in one sitting", and when
it is exceeded the remedy is to split or simplify, never to raise the cap.

## 6. The gate

A proposed API must pass **all seven** points. Failing one is a rejection, not a
negotiation: report the failing point and stop.

1. Removes friction (Clock A) rather than authorship (Clock B).
2. Fits an API tier and satisfies that tier's rule.
3. Does not change the frozen hardware.
4. Is not on the no-list.
5. Produces a visible result on first use, with no setup ritual.
6. Is the only obvious way — does not duplicate an existing call.
7. Is explainable to a beginner in one sentence.

Point 1 fails most often. The test: a helper whose main value is saving the user
keystrokes inside their game loop fails; a tool that deletes a step before the
first pixel appears passes. Point 6 fails second most often — check
`api_registry.rs` for a call that already does the job before adding another.

### Worked verdicts

- **`entities` (the T2 mini-ECS)** — 57 lines, readable in a sitting, opt-in,
  and it does not write the game's structure so much as hold a list. **Passes**,
  on the strength of the readable-lesson cap. Had it been 400 lines of
  systems-and-components machinery, point 2 would have failed it.
- **A hypothetical `draw_sprite_rotated_scaled`** — **fails point 3**. Arbitrary
  rotation and scaling is a different pixel pipeline, which is hardware. The
  frozen spec is the answer; there is no version of this that gets in.
- **Studio one-key "run cart"** — **passes all seven**, and is exactly the kind
  of thing Clock A says to build without arguing about budget. It removes a step
  before the first pixel and takes nothing from the game's code.

If a verdict reached through the gate feels wrong, the gate is wrong and must be
fixed by an explicit decision. Do not route around it case by case.

## 7. Permanent no-list

- **No 3D** — no mode 7, raycasting helpers, or matrix stack.
- **No external I/O** — no network, filesystem, or subprocess. Cart data plus
  the local save blob only. (This is a security boundary as well as a design
  one.)
- **No shaders, render targets, or blend-mode zoo.** Fixed pixel pipeline.
- **No engine frameworks** — nothing that owns the game loop or requires two
  documents to be read before the first pixel.
- **No custom Lua dialect.** Real Lua 5.4 stays real; transferable knowledge is
  the point.
- **No telemetry or analytics SDK.**
- **No in-product LLM** (§1).

## 8. Deliberate non-limits

Recorded so they are not re-argued:

- **No token limit and no code-size limit.** Token limits punish readable code,
  which is precisely what a beginner and a brain-gym reader need. This differs
  from PICO-8 on purpose.
- **The 128 KiB packed cart cap is an operations constraint**, not a design
  forcing function — it keeps the Caiven Port database bounded. Storage policy.
- **Long descriptive API names stay** (`draw_line`, not `line`). Readability
  serves both audiences; abbreviations serve neither.

## 9. Revision policy

This charter changes only by an explicit, recorded decision from the project
owner. It does not change through a judgement call taken mid-task, through an
implementation finding it inconvenient, or through a feature that would be nice
if only one line were relaxed. A rule that bends under pressure is not a
constraint, and the constraints are the product.

## Appendix A — T2 modules against the readable-lesson cap

Measured from `crates/caiven-vm/src/vm/prelude/*.lua`, not estimated.

| Module | Lines | Verdict |
| --- | --- | --- |
| `tween` | 30 | PASS |
| `particles` | 32 | PASS |
| `scenes` | 38 | PASS |
| `core` (T1) | 50 | PASS — T1, not subject to the T2 cap |
| `camera` | 56 | PASS |
| `entities` | 57 | PASS |
| `vec2` | 86 | PASS |
| `collision` | 38 | PASS (post-split) |
| `movement` | 229 (134 non-blank, non-comment) | OVER the ~100-line cap; kept as one solver, needs a further split or an explicit exception |

`collision.lua` was originally 147 lines and the only breach, caused by one
function: `move_and_collide`, carrying its slope solver
(`solid_blocks_column`, `solid_blocks_row`, `slope_floor_y`) with it. The rest
of the module — `aabb_overlap`, `circle_overlap`, `point_in_rect`,
`point_in_circle`, `tile_solid`, `box_touches_solid` — was 6 short predicates
in about 30 lines and already exemplary T2.

**Split, not cut** — done in Phase 2 item 2.9. `collision.lua` kept the 6
predicates (38 lines); the swept-movement solver moved into its own
`movement` module, declared separately in `caiven.toml`. Both halves now read
in one sitting, and a cart that only needs `aabb_overlap` stops paying for
the slope math.

## Appendix B — status

The hardware table in §4 is **target state**. The 192 × 128 screen (item 2.1),
the 128 × 128 map (item 2.2), the redesigned palette (item 2.3), the six
typed audio voices (item 2.4), named banks (item 2.5), the `dset`/`dget`
removal (item 2.6), the per-frame execution budget watchdog (item 2.7),
optional `w`/`h` sprite-unit args on `sprite()` (item 2.8), and the
`collision`/`movement` module split (item 2.9) have landed. The redesign
lands phase by phase; the charter is what those phases are steering toward,
and is authoritative in any disagreement with the code.
