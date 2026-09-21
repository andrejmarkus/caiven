# Built-in API Reference

Math (`sin`/`cos`/`abs`/`floor`/`sqrt`/`max`/`min`/`random`), strings (`..`, `sub`, `tostring`, `string.*`), and tables are all just Lua's own stdlib — no bindings needed for those.

## Graphics

| Function                                                          | Description                                                                                                          |
| :----------------------------------------------------------------| :----------------------------------------------------------------------------------------------------------------- |
| `clear_screen()`                                                  | Clear screen and UI layer to pure black (not palette color 0; use `fill_screen(0)` for that)                          |
| `fill_screen(color)`                                              | Fill screen with a palette color                                                                                     |
| `set_pixel(x, y, color)`                                          | Set pixel (signed coords)                                                                                            |
| `draw_line(x0, y0, x1, y1, color)`                                | Line (camera-aware)                                                                                                  |
| `draw_rect(x, y, w, h, color)` / `fill_rect(x, y, w, h, color)`   | Rectangle outline / filled                                                                                           |
| `draw_circle(cx, cy, r, color)` / `fill_circle(cx, cy, r, color)` | Circle outline / filled                                                                                              |
| `set_palette_color(index, r, g, b)`                               | Set palette entry                                                                                                    |
| `set_camera(x, y)`                                                | Set camera offset                                                                                                    |
| `draw_text(text, x, y, color)`                                    | Draw a string (does **not** shadow Lua's real `print()` — Machine writes it to terminal; Studio writes it to Output) |
| `draw_number(value, x, y, color)`                                 | Draw an integer                                                                                                      |

## Sprites & Map

| Function                                  | Description                                                    |
| :----------------------------------------- | :--------------------------------------------------------------|
| `sprite(id, x, y, flip_x, flip_y, rotate, w, h)` | Draw 8×8 sprite (camera-aware); `flip_x`/`flip_y` mirror it (default `false`), `rotate` is `0`/`90`/`180`/`270` degrees clockwise (default `0`, applied before flipping — any other value is a Lua error). `w`/`h` are optional sizes in sprite units, not pixels (default `1`, `1`) — a bigger character draws in one call by covering adjacent sheet slots; a block that runs past the sheet edge, or a `w`/`h` below `1`, is a Lua error |
| `draw_map(cell_x, cell_y, sx, sy, w, h)`  | Draw a block of the tilemap                                    |
| `get_tile(x, y)` / `set_tile(x, y, tile)` | Read / write a map cell                                        |
| `load_sprite_bank(name)` / `load_map_bank(name)` | Switch the sprite / map RAM window to a named bank — see [Banking](#banking) |
| `get_collision(tx, ty)` / `set_collision(tx, ty, value)` | Read / write the collision-type id at a map cell; `0`/no-op if out of bounds |
| `collision_type_id(name)` / `collision_type_name(id)`    | Look up a collision type's id by name (`0` if unknown) / name by id (`""` if undefined) |
| `collision_is_solid(id)` / `collision_is_one_way(id)` / `collision_is_slope_left(id)` / `collision_is_slope_right(id)` | Whether a collision type is flagged solid / one-way / a left or right 45° slope; undefined ids are always `false` for every check |

## Banking

Sprites, map (+ its collision companion), palette, SFX, and music each have
one **default** bank that auto-loads at boot, plus any number of named
additional banks a cart creates in Studio. A bank name is 1-31 letters,
digits, `_`, or `-`.

| Function                    | Description                                                                 |
| :----------------------------| :---------------------------------------------------------------------------|
| `load_sprite_bank(name)`    | Copy the named sprite bank into sprite RAM; `false` if it does not exist    |
| `load_map_bank(name)`       | Copy the named map bank into map RAM, its collision bank along with it; `false` if it does not exist |
| `load_palette_bank(name)`   | Copy the named palette bank into the render-time palette; `false` if it does not exist |
| `load_sfx_bank(name)`       | Copy the named SFX bank into SFX RAM; `false` if it does not exist          |
| `load_music_bank(name)`     | Copy the named music bank into music RAM; `false` if it does not exist      |

`"default"` always exists and can be switched back to like any other name; it
is the only bank that cannot be created or removed. A cart that never calls a
`load_*_bank` function only ever sees its default banks.

## Input

| Function              | Description                                               |
| :---------------------| :----------------------------------------------------------|
| `button_down(id)`     | Button held (0=Up 1=Down 2=Left 3=Right 4=A 5=B 6=Select) |
| `button_pressed(id)`  | Button pressed this frame                                  |
| `button_released(id)` | Button released this frame                                 |

START is reserved by the console. It opens the pause menu, which on a
handheld is the player's only way out of a running cart, so it never reaches
cartridge code — there is no index for it. Any index outside the table above
returns `false` rather than erroring.

## Audio

| Function                 | Description                                                                                                                                             |
| :------------------------| :----------------------------------------------------------------------------------------------------------------------------------------------------- |
| `play_sfx(id, opts)`     | Start SFX `id` on a free (or, if all are busy, oldest) voice. `opts.volume` (0-1, default 1) is optional. Returns a handle. Polyphonic — concurrent calls get independent voices. |
| `stop_sfx(handle)`       | Stop the voice `handle` refers to. Silent no-op if it already finished or was reused.                                                                   |
| `is_sfx_playing(handle)` | True if `handle` refers to a voice still actively playing. Stale handle returns `false`, not an error.                                                  |
| `play_music(id)`         | Play a music track, looping. `id` is clamped into range. Cancels song playback if one is running.                                                        |
| `play_music_song(start_step)` | Play the bank's song order table from `start_step` (optional, default `0`), chaining patterns and honoring the loop point. `start_step` is clamped; a song with nothing to play is a silent no-op. |
| `stop_music()`           | Stop music (single-pattern or song)                                                                                                                       |
| `is_music_playing()`     | True while a music track is playing.                                                                                                                     |
| `set_master_volume(v)`   | Runtime-only output multiplier, `v` clamped to `[0, 1]`                                                                                                  |
| `set_music_volume(v)`    | Runtime-only music-channel multiplier, `v` clamped to `[0, 1]`                                                                                           |
| `set_sfx_volume(v)`      | Runtime-only SFX-voice multiplier, `v` clamped to `[0, 1]`                                                                                               |

Each SFX step is 4 bytes: `note, volume, wave, byte3`. `byte3` packs pan
(bits 0-3, index into a 16-position table, 0 = center) and attack/release
envelope levels (bits 4-5 / 6-7, each 0-3 mapping to instant/~15ms/~50ms/~150ms
ramps). `byte3 = 0` is center pan with an instant on/off envelope.

## Persistent Data

| Function                | Description                                                                                          |
| :------------------------| :----------------------------------------------------------------------------------------------------|
| `save_data(table)`      | Replace the persisted save blob (string/number/bool/nested-table only); errors over 4KiB packed or on an unserializable value |
| `load_data()`           | Return the persisted save blob, or `{}` if `save_data` has never been called                          |

`save_data`/`load_data` is the one obvious way to persist state — two save
APIs would violate that, so the numeric-slot `dset`/`dget` pair has been
removed.

Save data is per cart (keyed the same way save states already are — see
System Specifications below) and is written to disk by the host (Machine
or Studio), not by the Lua sandbox directly.

## System

| Function        | Description                                                      |
| :--------------- | :-----------------------------------------------------------------|
| `real_time()`   | Returns `(hour, minute, second)` — the host's real-time clock, **UTC**, not the player's local time zone |
| `frame_count()` | Number of frames run since the cart loaded                       |
| `time()`        | Seconds since the cart loaded, assuming 60 frames per second     |

## Gameplay stdlib

Pure Lua — read `crates/caiven-vm/src/vm/prelude/` for the source. Split into an
always-on **core** plus opt-in **modules** a cart declares in `caiven.toml`:

```toml
[stdlib]
modules = ["vec2", "scenes", "entities", "camera"]
```

**Breaking change:** a cart with no `[stdlib]` table gets **core only** — no
`Vec2`, `Scenes`, `Entities`, `Camera`, collision helpers, tweens, or
`Particles`. This replaced an old behavior where every cart got the entire
stdlib unconditionally. If your cart uses any non-core name from the tables
below, add `[stdlib] modules = [...]` listing every module it uses — the
missing name otherwise resolves to Lua `nil` and errors on first use (a
regular Lua runtime error, not a silent no-op). An unknown module name in
that list is a hard error at cart load, naming the bad entry.

See it all in action in `carts/dev/stdlib_demo.cav`
(`cargo run -p caiven-machine -- carts/dev/stdlib_demo.cav`): a tiny
platformer with tile collision, a coin pickup that bursts particles, a
walk-cycle sprite animation, and four side-by-side tweened dots comparing
each easing curve — declares `[stdlib] modules = ["collision", "tween",
"particles"]`.

The `Scenes`/`Entities`/`Camera` trio has its own example:
`carts/dev/scenes_demo.cav`
(`cargo run -p caiven-machine -- carts/dev/scenes_demo.cav`) — a title
screen, a play scene with a camera-followed player and two entities, and a
game-over screen — declares `[stdlib] modules = ["vec2", "scenes",
"entities", "camera"]`.

Two more minimal carts show the two ends of the opt-in range:
`carts/dev/stdlib_core_only.cav` declares `[stdlib] modules = []`
explicitly (core-only, same as omitting `[stdlib]` entirely) and
`carts/dev/stdlib_all_modules.cav` declares every module at once.

### `core` (always on, no declaration needed)

RNG is deterministic by default — the prelude core seeds `math.randomseed(1)` once per fresh cart load (not on hot reload, so live gameplay isn't disturbed by an editor save). Call `math.randomseed(os.time())` yourself for per-run variety.

| Function                                                   | Description                                                                  |
| :---------------------------------------------------------- | :------------------------------------------------------------------------- |
| `lerp(a, b, t)` / `clamp(v, lo, hi)`                       | Linear interpolate / clamp to range                                        |
| `ease_linear/in_quad/out_quad/in_out_quad(t)`              | Easing curves, `t` in `0..1`                                               |
| `random_range(lo, hi)` / `random_float(lo, hi)`            | Deterministic-by-default RNG (see above) — int inclusive / float `[lo, hi)` |
| `choice(t)` / `shuffle(t)`                                 | Random element of a non-empty table / in-place Fisher-Yates shuffle        |

### `vec2` — `Vec2`, `Sprite`

| Function                                            | Description                                                                                                                                   |
| :---------------------------------------------------- | :--------------------------------------------------------------------------------------------------------------------------------------------- |
| `Vec2.new(x, y)`                                    | 2D vector with `+`/`-`/unary `-`/`*` (scalar)/`==`; `v:length()`, `v:length_squared()`, `v:normalize()`, `v:dot(other)`, `v:distance(other)` |
| `Sprite.new{sprite_id, pos, flip_x, flip_y, rotate}` / `s:draw()` | Bundles a sprite_id + Vec2 pos (+ optional orientation) into a drawable object                                                     |

### `collision` — AABB/circle/point/tile helpers

| Function                                                                   | Description                                                       |
| :---------------------------------------------------------------------------| :----------------------------------------------------------------|
| `aabb_overlap(x1, y1, w1, h1, x2, y2, w2, h2)`                             | Axis-aligned box overlap test                                     |
| `circle_overlap(x1, y1, r1, x2, y2, r2)`                                   | Circle overlap test                                               |
| `point_in_rect(px, py, x, y, w, h)` / `point_in_circle(px, py, cx, cy, r)` | Point containment tests                                           |
| `tile_solid(tx, ty)`                                                       | Whether the per-cell collision value at `(tx, ty)` is `1` (solid) |
| `box_touches_solid(x, y, w, h)`                                            | Whether a pixel-space box overlaps any solid tile                 |

### `movement` — swept move + collision resolve

| Function                                                    | Description                                                    |
| :-------------------------------------------------------------| :----------------------------------------------------------|
| `move_and_collide(x, y, w, h, dx, dy)`                     | Axis-separated swept move against SOLID (both axes), ONE_WAY (vertical, landing only when descending from above), and slope tiles (vertical, per-column floor sampling); returns `nx, ny, touch = {ground, ceiling, left, right}` |

### `tween` — value tweens and sprite animation

| Function                                                    | Description                                                    |
| :-------------------------------------------------------------| :----------------------------------------------------------|
| `new_tween(from, to, frames, ease)` / `tween_update(tw)`    | Frame-driven value tween; `tw.done` flips true on arrival      |
| `new_anim(frames, frame_len)` / `anim_update(anim)` / `anim_sprite(anim)` | Frame-based sprite animation cycling through a sprite-id list |

### `particles` — `Particles`

| Function                                                                                          | Description                                 |
| :----------------------------------------------------------------------------------------------------| :------------------------------------------ |
| `Particles.spawn(x, y, vx, vy, color, life)` / `.update()` / `.draw()` / `.clear()` / `.count()` | Simple velocity + lifetime particle system |

### `scenes` — `Scenes`

| Function                                                                                     | Description                                                                    |
| :------------------------------------------------------------------------------------------------| :---------------------------------------------------------------------------- |
| `Scenes.push(scene)` / `.pop()` / `.switch(scene)` / `.update()` / `.draw()` / `.current()` | Stack-based scene manager; scene = table with optional enter/exit/update/draw |

### `entities` — `Entities`

| Function                                                                                  | Description                                                                                       |
| :----------------------------------------------------------------------------------------------| :------------------------------------------------------------------------------------------------ |
| `Entities.add(e)` / `.update_all()` / `.draw_all()` / `.clear()` / `.count()` / `.overlapping(x,y,w,h)` / `.new()` | Entity list with lifecycle (e.dead removes on next update_all()); overlapping() returns entries whose .pos(Vec2)+.w/.h box overlaps the query box (requires the collision module too, for aabb_overlap); .new() gives an independent list |

### `camera` — `Camera`

| Function                                                                                | Description                                                                        |
| :-------------------------------------------------------------------------------------------| :---------------------------------------------------------------------------- |
| `Camera.follow(entity, opts)` / `.unfollow()` / `.shake(amount, duration)` / `.update()` | Wraps set_camera() with smoothed follow (opts.lerp, default 1) and decaying shake |

## Hot reload (Studio)

Saving while a cart runs reloads its code without restarting it. `Ctrl+R` in `caiven-machine` is a full restart, not a hot reload.

| Kept | Refreshed | Needs Reset |
| :-- | :-- | :-- |
| Top-level `local` values, matched by name | Function bodies, including `local function` helpers | `_init` does not re-run: state built there stays as it was |
| Prelude state (`Scenes`, `Entities`, `Particles`, `Camera`, `Vec2` metatable) | Global functions | A renamed or newly added `local` starts from its initializer |
| RNG stream | | Edited constants: a kept `local` keeps its old value |
| | | Methods on objects built in `_init` keep their old code |

Top-level global assignments run again on reload, so a global set at file scope resets to its initializer. Keep state in `local`s or set it in `_init`. Reset when unsure.

## System Specifications

> [!IMPORTANT]
> The numbers below describe the console **as it is today**. The 192×128
> screen, the 192×128 map, the redesigned palette, the 6 typed audio
> voices, named banks, the `dset`/`dget` removal, and the per-frame
> execution budget watchdog have landed; the rest of the hardware redesign
> is still pending. Target spec:
> [design charter](product/design-charter.md) §4.
> Change list: [hardware redesign plan](product/hardware-redesign-plan.md).

| Component         | Specification                                                                     |
| :-----------------| :------------------------------------------------------------------------------------|
| **Script engine** | Lua 5.4 via `mlua` (vendored)                                                     |
| **Resolution**    | 192×128, 24×16 tiles (upscaled 4×)                                                |
| **RAM**           | 64 KiB general purpose (Work + Heap); the asset windows below are mapped alongside it, not carved out of it. Script state lives in the Lua VM, not guest RAM |
| **Cartridge**     | 128 KiB maximum packed `.cav` size                                                |
| **Palette**       | 16 colors: 4 hue ramps × 3 shades, plus black, white and 2 accents (see below)     |
| **Sprites**       | 256 × 8×8 pixels per bank; `"default"` bank always available                      |
| **Map**           | 192×128 tiles per bank (8×8 screens); `"default"` bank always available           |
| **Audio**         | 6 voices: 4 typed music channels (pulse 1, pulse 2, triangle, noise) + 2 voices reserved for sound effects (see below) |

Additional banks live in cartridge storage, not guest RAM. Studio writes them
as `sprites_<name>.png` and `map_<name>.png`; runtime calls copy the selected
bank into the fixed sprite/map RAM window. Changes made through RAM survive
later switches.

### Palette

The default 16 colors are laid out so shading never needs a color-theory
decision: slots 1–12 are four hue ramps in dark → mid → light order, so the
darker shade of any color is the slot before it and the highlight is the slot
after it. Slot 0 is black, slot 15 is white, and 13–14 are the two accents.

| Slot | Color | RGB | Typical use |
| :--- | :--- | :--- | :--- |
| `0` | black | `16, 16, 26` | background, outlines |
| `1` `2` `3` | ember dark / mid / light | `110, 31, 46` · `194, 55, 47` · `242, 128, 60` | fire, blood, brick, danger |
| `4` `5` `6` | moss dark / mid / light | `30, 58, 42` · `62, 138, 74` · `134, 207, 98` | foliage, grass, slime |
| `7` `8` `9` | sky dark / mid / light | `35, 52, 94` · `61, 109, 196` · `116, 192, 232` | water, sky, cold metal |
| `10` `11` `12` | stone dark / mid / light | `58, 51, 64` · `122, 110, 114` · `195, 181, 168` | ground, walls, wood, skin |
| `13` | gold accent | `245, 197, 66` | coins, highlights, sun |
| `14` | magenta accent | `224, 96, 160` | magic, focus, alarm |
| `15` | white | `244, 241, 230` | text, sparks |

`set_palette_color(index, r, g, b)` replaces any slot at runtime, so a cart that
wants its own colors is never stuck with these.

### Audio

Music is authored as 8 patterns of 16 rows. Each row has one cell per music
channel holding an SFX reference, and each channel's timbre is fixed by its
column — pulse 1, pulse 2, triangle, noise — so "which channel is that?" is
answered by ear. A channel plays the referenced SFX's notes and volumes; the
SFX's own wave byte is ignored there, because the column already decided it.

Patterns are chained into a song by the order table that follows the pattern
data in the same bank: 32 one-byte steps, each holding `pattern id + 1` so
`0` (and any value above the pattern count) means "nothing here". One more
byte after the table holds the loop point as `step + 1`, with `0` meaning
"no loop — stop when the song runs out". `play_music_song` walks this table;
`play_music` ignores it and loops one pattern. A bank written before songs
existed reads as all zeros here, so it simply has no song.

The two remaining voices are reserved for `play_sfx`. Nothing a cart plays
through them can silence a music channel: a jump sound landing on a busy
frame steals the older sound effect, never the melody. More than two
concurrent sound effects steal the least recently started one.

### Memory Map

| Range           | Region                                                         |
| :---------------| :----------------------------------------------------------------|
| `0x0000–0x3FFF` | Unused / reserved                                              |
| `0x4000–0x7FFF` | Sprite sheet — 256 sprites × 64 bytes (1 byte/pixel)           |
| `0x8000–0xDFFF` | Tilemap 192×128 (1 byte/cell)                                  |
| `0xE000–0xE0FF` | Palette (16 × 3 bytes RGB, rest padding)                       |
| `0xE100–0xE4FF` | SFX bank (16 × 64 bytes)                                       |
| `0xE500–0xE7FF` | Music bank (8 patterns × 16 rows × 4 channels, 1 byte/cell), then the 32-byte song order table and its loop-point byte |
| `0xE800–0xE802` | RTC (hour, minute, second)                                     |
| `0xE803–0x14802` | Collision — 192×128 (1 byte/cell: 0 walkable, 1 solid, 2 hazard) |
| `0x14803–0x1BFFF` | Reserved                                                     |
