# Caiven design charter

This document sets what Caiven is, what its hardware is and which APIs may
exist. Settle feature arguments against it.

## 1. Positioning

Caiven is a fantasy console: a small, fixed, imaginary machine you program
in real Lua 5.4. It keeps the same size whatever you build, and that limit
helps you finish a game.

Other tools will write code for you. Here you type it yourself, to keep
your logic skills in shape or to learn them for the first time. The
console ships no in-product LLM.

## 2. Two clocks

We hold two goals that pull against each other:

1. **Brain gym.** You write the game yourself, and typing the code is the
   point.
2. **Idea to playable in one sitting.** Our players have little patience,
   so time to fun breaks ties.

Goal 2 tempts us to add helpers that write the game for you, and those
helpers would kill goal 1. We resolve the conflict by splitting the work
into two clocks:

| Clock | What it measures | Rule |
| --- | --- | --- |
| **A: friction** | Everything around the code: boot, reload, sprite, map and sfx editing, templates, defaults, error messages, export | Drive it to zero, and spend engineering time on it without a budget. |
| **B: authorship** | The game logic | You type it. The API stays small, with no autopilot and no framework that writes the game's structure. |

**The gate in one sentence: does this remove friction, or does it remove
authorship?**

We keep polishing the editors, reload and error messages, and nobody on the
project calls that scope creep. We never trade authorship for speed,
however much time the trade would save. You get speed from the tools and
write the game yourself.

## 3. Audience

We serve two audiences with one API:

- **Complete beginners.** You learn programming through a fun, non-violent
  toy in real Lua, and your skills carry over to any Lua project.
- **Working programmers.** You keep your skills sharp on a machine small
  enough to finish a game in one session.

Both groups want a small API, readable names, instant results and no hidden
behavior. Reject a feature that helps one group at the other's expense.

Treat every setup step before the first pixel as a reason someone quits.

## 4. Fixed console, expandable cartridge

We won't grow the screen, the palette or the voice count. Carts grow through
named banks only.

| Spec | Value | Why |
| --- | --- | --- |
| **Screen** | 192 × 128 (24 × 16 tiles) | 48 text columns (3 px glyph plus 1 px gap) fit a real sentence. A 128 px screen gives 32, so a beginner's first `draw_text` would wrap. The 3:2 shape suits side-scrollers, and 24 × 16 tiles keep space scarce. |
| **Palette** | 16 hand-picked colors | More colors mean more time in the sprite editor. Four hue ramps of 3 shades, plus black, white and 2 accents, let you shade without knowing color theory. |
| **Sprites** | 8 × 8, 256 per bank | A 16 × 16 sprite takes four times as long to draw, and an impatient beginner quits in the first ten minutes. Four 8 × 8 sprites make a 16 × 16 hero, and the sprite editor handles the grouping. |
| **Map** | 192 × 128 tiles plus a collision layer | 8 × 8 screens, 64 in total, with no partial column at the edge. One map holds a full level. |
| **Frame rate** | 60 Hz, fixed | Game feel depends on a fixed step. |
| **Audio** | 6 voices: 4 typed music channels (2 pulse, 1 triangle, 1 noise) plus 2 for sound effects | Typed channels turn the tracker into four columns you can scan, and you can tell them apart by ear. Reserved sfx voices stop a jump sound from cutting the melody, the audio bug beginners find most confusing. Classic consoles stole music channels for sfx; we picked the model a beginner can follow. |
| **Input** | 4 directions, 2 action buttons and Select; START reserved | Matches retro pads and handhelds, and keeps carts free of a pointer-input path. |
| **RAM** | 46 KiB general purpose (16 KiB work plus 30 KiB heap), visible in Studio's memory view and hidden from Lua (no `peek` or `poke`) | Screen, map and collision live in their own regions and leave the cart's data space alone. Total address space is 112 KiB. |
| **Save** | One blob (`save_data` / `load_data`) | One blob gives you one obvious way to save. It holds a real Lua table and moves between machines. |
| **Watchdog** | A time limit per frame | An infinite loop stops with a line number and a plain message, and the console keeps running. |

### Banking

Banks are the only way to grow a cart, and we keep them plain.

- **Banks have names.** You write `load_sprite_bank("forest")`. The name
  tells you what's inside and follows the long-name rule. Banks have no
  numeric ids, so you get one way to pick a bank.
- **A cart can hold any number of banks.** The 128 KiB packed cart cap sets
  the ceiling.
- **You meet banks when you need them.** The default bank loads by itself,
  so a beginner can finish a first game without hearing the word. The docs
  introduce banks where a cart outgrows one sheet, as they do with the
  optional size arguments of `sprite()`.

## 5. API tiers

| Tier | What | Rule |
| --- | --- | --- |
| **T0: builtins** (Rust) | Hardware access: graphics, sprites, map, input, audio, storage, time | Only what Lua can't do. If you can write it in Lua, it stays out of T0. |
| **T1: core library** (always loaded) | `lerp`, `clamp`, `random_range`, easing | Math only, with no game structure. Keep it small. |
| **T2: opt-in modules** (`require`) | `actor`, `anim`, `camera`, `collision`, `entities`, `movement`, `particles`, `scenes`, `tween`, `vec2` | **Readable-lesson cap**: plain Lua, at most 100 lines of code (blank lines and comments don't count), source readable in Studio. |

You can read a T2 helper, and you could have written it, so using it speeds
you up and leaves the game yours. A helper you can't read takes authorship
away. Treat each T2 module as a teaching example. When a module outgrows the
cap, split or simplify it and keep the cap at 100.

**`movement` is the one exception.** It holds a single platformer solver
(solid tiles, one-way platforms and slopes) in about 135 lines of code. You
need the whole solver in view to follow it, so we keep it in one module. No
other module gets an exception.

## 6. The gate

A proposed API must pass all seven points. If it fails one, name the
failing point and reject it.

1. It removes friction (Clock A) and leaves authorship (Clock B) alone.
2. It fits an API tier and meets that tier's rule.
3. It keeps the hardware in §4 as it is.
4. It isn't on the no-list.
5. It shows a visible result the first time you use it, with no setup.
6. It is the one obvious way to do the job and duplicates no existing call.
7. You can explain it to a beginner in one sentence.

Point 1 fails most often. A helper whose main value is saving keystrokes in
your game loop fails it, and a tool that removes a step before the first
pixel passes. Point 6 fails second most often, so check `api_registry.rs`
for an existing call before you add one.

### Example verdicts

- **`entities`, the small T2 entity list**, **passes**. It is opt-in, you
  can read it in a sitting, and it holds a list without imposing a game
  structure. A 400-line systems-and-components version would fail point 2.
- **A made-up `draw_sprite_rotated_scaled`** **fails point 3**. Free
  rotation and scaling need a different pixel pipeline, and the pipeline is
  hardware.
- **A one-key "run cart" in Studio** **passes all seven**. It removes a step
  before the first pixel and leaves the game code alone.

## 7. Permanent no-list

- **No 3D**: no mode 7, raycasting helpers or matrix stack.
- **No external I/O**: no network, file system or subprocesses. A cart gets
  its own data and the local save blob, and this rule doubles as a security
  boundary.
- **No shaders, render targets or pile of blend modes.** The pixel pipeline
  stays fixed.
- **No engine frameworks**: nothing that owns the game loop or makes you
  read two documents before the first pixel.
- **No custom Lua dialect.** Carts run real Lua 5.4 so your knowledge
  transfers.
- **No telemetry or analytics SDK.**
- **No in-product LLM** (§1).

## 8. Limits we chose not to add

- **No token limit and no code-size limit.** Token limits punish readable
  code, and beginners learn from readable code. PICO-8 has a token limit;
  we left it out on purpose.
- **The 128 KiB packed cart cap protects Port's database** from unbounded
  growth. It's an operations limit, so don't cite it in design arguments.
- **Long descriptive API names stay** (`draw_line` over `line`). Full names
  help both audiences read code.

## 9. The remix loop

Caiven is a network of tiny playable programs. We optimize the loop
**play → remix → change → publish → get remixed → return**, and we count a
remix as a stronger signal than a like or a comment.

Browser Quick Remix ([Port](../port.md#quick-remix)) is Clock A work. It
cuts the install, download, file and account steps between playing a game
and changing it, and you still type the change in real Lua. Each creator
decides whether others may remix their cart, because sharing source is the
creator's call. Port counts the loop in its own database, which the §7
no-telemetry rule allows.

## 10. Changing this charter

Only the project owner changes this charter, by an explicit, recorded
decision. An awkward implementation or a feature that needs one rule
relaxed doesn't qualify. When the code disagrees with the charter, fix the
code. When the gate gives a verdict that feels wrong, change the gate
instead of working around it case by case.
