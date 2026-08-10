# Built-in API Reference

Math (`sin`/`cos`/`abs`/`floor`/`sqrt`/`max`/`min`/`random`), strings (`..`, `sub`, `tostring`, `string.*`), and tables are all just Lua's own stdlib — no bindings needed for those.

## Graphics

| Function                                                          | Description                                                                                                          |
| :----------------------------------------------------------------| :----------------------------------------------------------------------------------------------------------------- |
| `clear_screen()`                                                  | Clear screen and UI layer                                                                                            |
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
| `sprite(id, x, y, flip_x, flip_y, rotate)` | Draw 8×8 sprite (camera-aware); `flip_x`/`flip_y` mirror it (default `false`), `rotate` is `0`/`90`/`180`/`270` degrees clockwise (default `0`, applied before flipping — any other value is a Lua error) |
| `draw_map(cell_x, cell_y, sx, sy, w, h)`  | Draw a block of the tilemap                                    |
| `get_tile(x, y)` / `set_tile(x, y, tile)` | Read / write a map cell                                        |
| `load_sprite_bank(id)`                    | Copy sprite bank into sprite RAM; returns `false` when missing |
| `load_map_bank(id)`                       | Copy map bank into map RAM; returns `false` when missing       |

## Input

| Function             | Description                                               |
| :--------------------| :----------------------------------------------------------|
| `button_down(id)`    | Button held (0=Up 1=Down 2=Left 3=Right 4=A 5=B 6=Select) |
| `button_pressed(id)` | Button pressed this frame                                  |

START is reserved by the console. It opens the pause menu, which on a
handheld is the player's only way out of a running cart, so it never reaches
cartridge code — there is no index for it. Any index outside the table above
returns `false` rather than erroring.

## Audio

| Function         | Description                           |
| :----------------| :---------------------------------------|
| `play_sfx(id)`   | Play a sound effect from the SFX bank |
| `play_music(id)` | Play a music track                    |
| `stop_music()`   | Stop music                            |

## Persistent Data

| Function                | Description                                                                                          |
| :------------------------| :----------------------------------------------------------------------------------------------------|
| `dset(slot, value)`     | Write `value` into save slot `0-63`; errors if `slot` is out of range                                |
| `dget(slot)`            | Read save slot `0-63`; `0` if never set; errors if `slot` is out of range                             |
| `save_data(table)`      | Replace the persisted save blob (string/number/bool/nested-table only); errors over 4KiB packed or on an unserializable value |
| `load_data()`           | Return the persisted save blob, or `{}` if `save_data` has never been called                          |

Save data is per cart (keyed the same way save states already are — see
System Specifications below) and is written to disk by the host (Machine
or Studio), not by the Lua sandbox directly.

## System

| Function        | Description                                                      |
| :--------------- | :-----------------------------------------------------------------|
| `real_time()`   | Returns `(hour, minute, second)` from the host's real-time clock |
| `frame_count()` | Number of frames run since the cart loaded                       |
| `time()`        | Seconds since the cart loaded, assuming 60 frames per second     |

## Gameplay stdlib

Pure Lua, loaded into every cart's globals automatically (no `require`) — read `crates/caiven-vm/src/vm/prelude.lua` for the source. See it all in action in `games/carts/stdlib_demo.cav` (`cargo run -p caiven-machine -- games/carts/stdlib_demo.cav`): a tiny platformer with tile collision, a coin pickup that bursts particles, a walk-cycle sprite animation, and four side-by-side tweened dots comparing each easing curve.

RNG is deterministic by default — `prelude.lua` seeds `math.randomseed(1)` once per fresh cart load (not on hot reload, so live gameplay isn't disturbed by an editor save). Call `math.randomseed(os.time())` yourself for per-run variety.

| Function                                                                                         | Description                                                       |
| :--------------------------------------------------------------------------------------------------| :--------------------------------------------------------------- |
| `lerp(a, b, t)` / `clamp(v, lo, hi)`                                                             | Linear interpolate / clamp to range                               |
| `ease_linear/in_quad/out_quad/in_out_quad(t)`                                                    | Easing curves, `t` in `0..1`                                      |
| `aabb_overlap(x1, y1, w1, h1, x2, y2, w2, h2)`                                                   | Axis-aligned box overlap test                                     |
| `circle_overlap(x1, y1, r1, x2, y2, r2)`                                                         | Circle overlap test                                               |
| `point_in_rect(px, py, x, y, w, h)` / `point_in_circle(px, py, cx, cy, r)`                       | Point containment tests                                            |
| `tile_solid(tx, ty)`                                                                             | Whether the per-cell collision value at `(tx, ty)` is `1` (solid) |
| `box_touches_solid(x, y, w, h)`                                                                  | Whether a pixel-space box overlaps any solid tile                 |
| `new_tween(from, to, frames, ease)` / `tween_update(tw)`                                         | Frame-driven value tween; `tw.done` flips true on arrival         |
| `new_anim(frames, frame_len)` / `anim_update(anim)` / `anim_sprite(anim)`                        | Frame-based sprite animation cycling through a sprite-id list     |
| `Particles.spawn(x, y, vx, vy, color, life)` / `.update()` / `.draw()` / `.clear()` / `.count()` | Simple velocity + lifetime particle system                        |
| `Vec2.new(x, y)`                                                                                 | 2D vector with `+`/`-`/unary `-`/`*` (scalar)/`==`; `v:length()`, `v:length_squared()`, `v:normalize()`, `v:dot(other)`, `v:distance(other)` |
| `random_range(lo, hi)` / `random_float(lo, hi)`                                                  | Deterministic-by-default RNG (see above) — int inclusive / float `[lo, hi)`       |
| `choice(t)` / `shuffle(t)`                                                                       | Random element of a non-empty table / in-place Fisher-Yates shuffle              |
| `Sprite.new{sprite_id, pos, flip_x, flip_y, rotate}` / `s:draw()`                                | Bundles a sprite_id + Vec2 pos (+ optional orientation) into a drawable object    |

## System Specifications

| Component         | Specification                                                                     |
| :-----------------| :------------------------------------------------------------------------------------|
| **Script engine** | Lua 5.4 via `mlua` (vendored)                                                     |
| **Resolution**    | 128×128 (upscaled 4×)                                                             |
| **RAM**           | 64 KiB (asset/RAM regions below; script state lives in the Lua VM, not guest RAM) |
| **Cartridge**     | 128 KiB maximum packed `.cav` size                                                |
| **Palette**       | 16 colors                                                                         |
| **Sprites**       | 256 × 8×8 pixels per bank; bank 0 always available                                |
| **Map**           | 64×64 tiles per bank; bank 0 always available                                     |

Additional banks live in cartridge storage, not guest RAM. Studio writes them
as `sprites_<id>.png` and `map_<id>.png`; runtime calls copy selected bank into
fixed sprite/map RAM windows. Changes made through RAM survive later switches.

### Memory Map

| Range           | Region                                                         |
| :---------------| :----------------------------------------------------------------|
| `0x0000–0x3FFF` | Unused / reserved                                              |
| `0x4000–0x7FFF` | Sprite sheet — 256 sprites × 64 bytes (1 byte/pixel)           |
| `0x8000–0x8FFF` | Tilemap 64×64 (1 byte/cell)                                    |
| `0x9000–0x90FF` | Palette (16 × 3 bytes RGB, rest padding)                       |
| `0x9100–0x94FF` | SFX bank (16 × 64 bytes)                                       |
| `0x9500–0x95FF` | Music bank (8 × 32 bytes)                                      |
| `0x9600–0x9602` | RTC (hour, minute, second)                                     |
| `0x9603–0xA602` | Collision — 64×64 (1 byte/cell: 0 walkable, 1 solid, 2 hazard) |
| `0xA603–0xFFFF` | Reserved                                                       |
