# API reference

For math (`sin`, `cos`, `abs`, `floor`, `sqrt`, `max`, `min`, `random`),
strings (`..`, `sub`, `tostring`, `string.*`) and tables, use Lua's own
standard library. We add no wrappers for those.

## Graphics

| Function                                                          | Description |
| :----------------------------------------------------------------| :---------- |
| `clear_screen()`                                                  | Clear the screen and text layer to black. This ignores palette color 0; use `fill_screen(0)` for that. |
| `fill_screen(color)`                                              | Fill the screen with a palette color. Text from `draw_text` and `draw_number` stays until `clear_screen()`. |
| `set_pixel(x, y, color)`                                          | Set one pixel (coordinates can be negative) |
| `draw_line(x0, y0, x1, y1, color)`                                | Line, offset by the camera |
| `draw_rect(x, y, w, h, color)` / `fill_rect(x, y, w, h, color)`   | Rectangle outline or filled rectangle |
| `draw_circle(cx, cy, r, color)` / `fill_circle(cx, cy, r, color)` | Circle outline or filled circle |
| `set_palette_color(index, r, g, b)`                               | Change a palette entry |
| `set_camera(x, y)`                                                | Set the camera offset |
| `draw_text(text, x, y, color)`                                    | Draw a string. Lua's `print()` still works: Machine sends it to the terminal and Studio to the Output tab. |
| `draw_number(value, x, y, color)`                                 | Draw an integer |

## Sprites and map

| Function                                  | Description |
| :----------------------------------------- | :---------- |
| `sprite(id, x, y, flip_x, flip_y, rotate, w, h)` | Draw an 8×8 sprite, offset by the camera. `flip_x` and `flip_y` mirror it (default `false`). `rotate` turns it clockwise by `0`, `90`, `180` or `270` degrees before flipping (default `0`); other values raise a Lua error. `w` and `h` set the size in sprites, not pixels (default `1` each), so you can draw a 16×16 character from four neighbouring sheet slots in one call. A `w` or `h` below `1`, or a block that runs off the sheet, raises a Lua error. |
| `draw_map(cell_x, cell_y, sx, sy, w, h)`  | Draw a block of the tilemap |
| `get_tile(x, y)` / `set_tile(x, y, tile)` | Read or write a map cell |
| `load_sprite_bank(name)` / `load_map_bank(name)` | Switch sprite or map RAM to a named bank. See [Banking](#banking). |
| `get_collision(tx, ty)` / `set_collision(tx, ty, value)` | Read or write the collision type id of a map cell. Out of bounds returns `0` or does nothing. |
| `collision_type_id(name)` / `collision_type_name(id)`    | Find a collision type's id by name (`0` if unknown), or its name by id (`""` if undefined) |
| `collision_is_solid(id)` / `collision_is_one_way(id)` / `collision_is_slope_left(id)` / `collision_is_slope_right(id)` | Check whether a collision type is solid, one-way, or a 45° slope rising to the left or right. Undefined ids return `false`. |

## Banking

Sprites, the map (with its collision layer), the palette, SFX and music each
have a `"default"` bank that loads at boot. In Studio you can add as many
named banks as you like. Names are 1 to 31 letters, digits, `_` or `-`.

| Function                    | Description |
| :----------------------------| :---------- |
| `load_sprite_bank(name)`    | Copy the named sprite bank into sprite RAM. Returns `false` if the bank doesn't exist. |
| `load_map_bank(name)`       | Copy the named map bank and its collision layer into map RAM. Returns `false` if the bank doesn't exist. |
| `load_palette_bank(name)`   | Copy the named palette bank into the active palette. Returns `false` if the bank doesn't exist. |
| `load_sfx_bank(name)`       | Copy the named SFX bank into SFX RAM. Returns `false` if the bank doesn't exist. |
| `load_music_bank(name)`     | Copy the named music bank into music RAM. Returns `false` if the bank doesn't exist. |

You can switch back to `"default"` like any other bank, but you can't
create or delete it. If your cart never calls a `load_*_bank` function, it
only uses the default banks.

## Input

| Function              | Description |
| :---------------------| :---------- |
| `button_down(id)`     | Button is held (0 Up, 1 Down, 2 Left, 3 Right, 4 A, 5 B, 6 Select) |
| `button_pressed(id)`  | Button went down this frame |
| `button_released(id)` | Button came up this frame |

START has no index. It opens the console's pause menu, which is the only way
to leave a game on a handheld, so carts never see it. Any id outside the
table returns `false` instead of raising an error.

## Audio

| Function                 | Description |
| :------------------------| :---------- |
| `play_sfx(id, opts)`     | Play SFX `id` on a free sound effect voice, or on the oldest one if both are busy. `opts.volume` (0 to 1, default 1) is optional. Returns a handle. Each call gets its own voice. |
| `stop_sfx(handle)`       | Stop the voice behind `handle`. Does nothing if the sound already ended or the voice was reused. |
| `is_sfx_playing(handle)` | `true` while the voice behind `handle` is still playing. An old handle returns `false`. |
| `play_music(id)`         | Loop one music pattern. Out-of-range ids are clamped. Stops a running song. |
| `play_music_song(start_step)` | Play the bank's song order table from `start_step` (optional, default `0`), chaining patterns and honoring the loop point. `start_step` is clamped. If the song is empty, nothing happens. |
| `stop_music()`           | Stop music, whether a single pattern or a song |
| `is_music_playing()`     | `true` while music plays |
| `set_master_volume(v)`   | Overall volume multiplier, clamped to `[0, 1]`. Not saved with the cart. |
| `set_music_volume(v)`    | Music volume multiplier, clamped to `[0, 1]`. Not saved with the cart. |
| `set_sfx_volume(v)`      | Sound effect volume multiplier, clamped to `[0, 1]`. Not saved with the cart. |

Each SFX step takes 4 bytes: `note, volume, wave, byte3`. In `byte3`, bits
0 to 3 pick one of 16 pan positions (0 is center). Bits 4-5 set the attack
and bits 6-7 the release, each from 0 to 3: instant, about 15 ms, 50 ms or
150 ms. `byte3 = 0` gives center pan with instant on and off.

## Saving

| Function                | Description |
| :------------------------| :---------- |
| `save_data(table)`      | Replace the cart's save data. Accepts strings, numbers, booleans and nested tables. Raises an error if the packed data exceeds 4 KiB or holds a value it can't store. |
| `load_data()`           | Return the save data, or `{}` if the cart has never saved |

Carts get one save API. The older numbered `dset`/`dget` slots are gone.

Each cart has its own save data, keyed the same way as save states (see
[System specifications](#system-specifications)). Machine or Studio writes
the file to disk; Lua code never touches the file system.

## System

| Function        | Description |
| :--------------- | :---------- |
| `real_time()`   | Returns `hour, minute, second` from the host clock in UTC, not the player's time zone |
| `frame_count()` | Frames run since the cart loaded |
| `time()`        | Seconds since the cart loaded, counted at 60 frames per second |

## Lua standard library

The library is plain Lua; the source is in
`crates/caiven-vm/src/vm/prelude/`. A small core is always loaded as
globals. Everything else comes in modules you load with `require`, like any
Lua 5.4 module: `require` returns a table and defines no globals, the same as
your own `.lua` files. Assign the result to a local. The headings below show
the usual names:

```lua
local Camera = require "camera"
local tween = require "tween"

local fade = tween.new(0, 1, 30)
```

Locals belong to one file, so each file that uses a module needs its own
`require`. Lua loads each module once and returns the same table after
that. If you use a module table without the local, you get a nil error that
ends with `add local Camera = require "camera" at the top of the file`, and
Studio offers that line as a quick fix. Requiring an unknown name raises a
normal Lua error with the name in it. If your cart has a file with the same
name as a module, such as `camera.lua`, your file wins.

Older carts listed modules in `caiven.toml` under `[stdlib]` and got them as
globals (`new_tween`, `aabb_overlap`, `Sprite`). The VM ignores that
table and its `.cav` section. To update such a cart, add the `local … =
require` lines and call through the table (`tween.new`,
`collision.aabb_overlap`, `Actor.new`).

`carts/dev/stdlib_demo.cav` shows most modules at work
(`cargo run -p caiven-machine -- carts/dev/stdlib_demo.cav`). It's a small
platformer with tile collision, a coin that bursts into particles, a walk
animation and four dots that compare the easing curves. It uses
`collision`, `tween`, `anim` and `particles`.

`carts/dev/scenes_demo.cav`
(`cargo run -p caiven-machine -- carts/dev/scenes_demo.cav`) covers
`Scenes`, `Entities` and `Camera`: a title screen, a level where the camera
follows the player past two entities, and a game over screen. It uses
`vec2`, `scenes`, `entities` and `camera`.

`carts/dev/stdlib_core_only.cav` loads no modules, and
`carts/dev/stdlib_all_modules.cav` loads all of them.

### Core (always loaded)

The random number generator starts from the same seed each time: the core
calls `math.randomseed(1)` when a cart loads (hot reload leaves the seed
alone, so saving in the editor doesn't disturb a running game). Call
`math.randomseed(os.time())` if you want different numbers on each run.

| Function                                                   | Description |
| :---------------------------------------------------------- | :---------- |
| `lerp(a, b, t)` / `clamp(v, lo, hi)`                       | Linear interpolation, and clamping to a range |
| `ease_linear/in_quad/out_quad/in_out_quad(t)`              | Easing curves, `t` from `0` to `1` |
| `random_range(lo, hi)` / `random_float(lo, hi)`            | Random integer in `[lo, hi]`, or float in `[lo, hi)` |
| `choice(t)` / `shuffle(t)`                                 | Random element of a non-empty table, or shuffle a table in place (Fisher-Yates) |

### `vec2`

`local Vec2 = require "vec2"`

| Function                                            | Description |
| :---------------------------------------------------- | :---------- |
| `Vec2.new(x, y)`                                    | 2D vector supporting `+`, `-`, unary `-`, `*` by a number and `==`. Methods: `v:length()`, `v:length_squared()`, `v:normalize()`, `v:dot(other)`, `v:distance(other)`. |

### `actor`

`local Actor = require "actor"`

| Function                                                         | Description |
| :----------------------------------------------------------------- | :---------- |
| `Actor.new{sprite_id, pos, flip_x, flip_y, rotate}` / `a:draw()` | An object holding a sprite id, a `Vec2` position and an optional orientation, with a `draw` method |

### `collision`

`local collision = require "collision"`

| Function                                                                                       | Description |
| :------------------------------------------------------------------------------------------------ | :---------- |
| `collision.aabb_overlap(x1, y1, w1, h1, x2, y2, w2, h2)`                                       | Do two axis-aligned boxes overlap? |
| `collision.circle_overlap(x1, y1, r1, x2, y2, r2)`                                             | Do two circles overlap? |
| `collision.point_in_rect(px, py, x, y, w, h)` / `.point_in_circle(px, py, cx, cy, r)`          | Is a point inside a rectangle or circle? |
| `collision.tile_solid(tx, ty)`                                                                 | Is the collision value at `(tx, ty)` `1` (solid)? |
| `collision.box_touches_solid(x, y, w, h)`                                                      | Does a box in pixel coordinates overlap a solid tile? |

### `movement`

`local movement = require "movement"`

| Function                                        | Description |
| :------------------------------------------------ | :---------- |
| `movement.move_and_collide(x, y, w, h, dx, dy)` | Moves a box one axis at a time. It stops against SOLID tiles on both axes, lands on ONE_WAY tiles only when falling from above, and follows slope tiles by sampling the floor per column. Returns `nx, ny, touch`, where `touch = {ground, ceiling, left, right}`. |

### `tween`

`local tween = require "tween"`

| Function                                                | Description |
| :-------------------------------------------------------- | :---------- |
| `tween.new(from, to, frames, ease)` / `tween.update(tw)` | Move a value from `from` to `to` over a number of frames. `tw.done` turns `true` at the end. |

### `anim`

`local anim = require "anim"`

| Function                                                          | Description |
| :------------------------------------------------------------------ | :---------- |
| `anim.new(frames, frame_len)` / `anim.update(a)` / `anim.sprite(a)` | Cycle through a list of sprite ids, `frame_len` frames each |

### `particles`

`local Particles = require "particles"`

| Function                                                                                          | Description |
| :----------------------------------------------------------------------------------------------------| :---------- |
| `Particles.spawn(x, y, vx, vy, color, life)` / `.update()` / `.draw()` / `.clear()` / `.count()` | Particles with a velocity and a lifetime |

### `scenes`

`local Scenes = require "scenes"`

| Function                                                                                     | Description |
| :------------------------------------------------------------------------------------------------| :---------- |
| `Scenes.push(scene)` / `.pop()` / `.switch(scene)` / `.update()` / `.draw()` / `.current()` | A stack of scenes. A scene is a table with optional `enter`, `exit`, `update` and `draw` functions. |

### `entities`

`local Entities = require "entities"`

| Function                                                                                  | Description |
| :----------------------------------------------------------------------------------------------| :---------- |
| `Entities.add(e)` / `.update_all()` / `.draw_all()` / `.clear()` / `.count()` / `.overlapping(x,y,w,h)` / `.new()` | A list of entities. Set `e.dead` and the next `update_all()` removes it. `overlapping()` returns the entities whose `.pos` (a `Vec2`) and `.w`/`.h` box overlaps the given box; it loads the `collision` module itself. `.new()` creates a separate list. |

### `camera`

`local Camera = require "camera"`

| Function                                                                                | Description |
| :-------------------------------------------------------------------------------------------| :---------- |
| `Camera.follow(entity, opts)` / `.unfollow()` / `.shake(amount, duration)` / `.update()` | Calls `set_camera()` for you, with smoothed following (`opts.lerp`, default 1) and a shake that fades out |

## Hot reload (Studio)

If you save while a cart runs, Studio reloads the code without restarting
the game. `Ctrl+R` in `caiven-machine` restarts the cart from scratch.

| Kept | Replaced | Needs a reset |
| :-- | :-- | :-- |
| Top-level `local` values, matched by name | Function bodies, including `local function` helpers | `_init` doesn't run again, so anything it built stays as it was |
| Library state (`Scenes`, `Entities`, `Particles`, `Camera`, the `Vec2` metatable) | Global functions | A renamed or new `local` starts from its initial value |
| Random number sequence | | A changed constant: a kept `local` keeps its old value |
| | | Methods on objects built in `_init` keep their old code |

Global assignments at the top level run again on reload, so a global set at
file scope goes back to its initial value. Keep state in `local`s or set it
in `_init`. When in doubt, reset.

## System specifications

These specs won't change. The [design charter](product/design-charter.md)
§4 explains each number.

| Component         | Specification |
| :-----------------| :------------ |
| **Script engine** | Lua 5.4 through `mlua` (vendored) |
| **Resolution**    | 192×128, 24×16 tiles (shown at 4×) |
| **RAM**           | About 45 KiB general purpose (16 KiB work plus about 29 KiB heap). Lua can't reach it: there is no `peek` or `poke`. The asset windows below sit next to it, outside that 45 KiB. Script state lives in the Lua VM, not in this RAM. |
| **Cartridge**     | 128 KiB maximum `.cav` size |
| **Palette**       | 16 colors: 4 hue ramps of 3 shades, plus black, white and 2 accents (see below) |
| **Sprites**       | 256 sprites of 8×8 pixels per bank; the `"default"` bank always exists |
| **Map**           | 192×128 tiles per bank (8×8 screens); the `"default"` bank always exists |
| **Audio**         | 6 voices: 4 music channels (pulse 1, pulse 2, triangle, noise) and 2 voices for sound effects (see below) |

Extra banks live in the cart file, outside guest RAM. Studio saves them as
`sprites_<name>.png` and `map_<name>.png`. Loading a bank copies it into the
fixed sprite or map window in RAM. Changes your code makes through RAM stay
after later switches.

### Palette

Slots 1 to 12 form four color ramps, each ordered dark, mid, light. To shade
a color, use the slot before it; to highlight it, use the slot after. Slot 0
is black, slot 15 is white, and 13 and 14 are accents.

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

`set_palette_color(index, r, g, b)` replaces any slot at runtime if you want
different colors.

### Audio

A music bank holds 8 patterns of 16 rows. Each row has one cell per music
channel, and each cell points to an SFX. The column sets the sound: pulse 1,
pulse 2, triangle or noise. The channel plays the notes and volumes of the
SFX it points to and ignores that SFX's wave byte.

After the pattern data, the bank holds a song order table of 32 one-byte
steps. Each step stores `pattern id + 1`, so `0`, or any value above the
pattern count, means an empty step. The byte after the table stores the loop
point as `step + 1`; `0` means the song stops at the end.
`play_music_song` follows this table, and `play_music` ignores it and loops
a single pattern. Banks saved before songs existed have zeros here, so they
have no song.

`play_sfx` uses the other two voices, so a sound effect can't cut off a
music channel. If a third sound effect starts while two are playing, it
replaces the one that started first.

### Memory map

| Range           | Region |
| :---------------| :----- |
| `0x0000–0x3FFF` | Unused / reserved |
| `0x4000–0x7FFF` | Sprite sheet: 256 sprites × 64 bytes (1 byte/pixel) |
| `0x8000–0xDFFF` | Tilemap 192×128 (1 byte/cell) |
| `0xE000–0xE0FF` | Palette (16 × 3 bytes RGB, rest padding) |
| `0xE100–0xE4FF` | SFX bank (16 × 64 bytes) |
| `0xE500–0xE7FF` | Music bank (8 patterns × 16 rows × 4 channels, 1 byte/cell), then the 32-byte song order table and its loop-point byte |
| `0xE800–0xE802` | RTC (hour, minute, second) |
| `0xE803–0x14802` | Collision: 192×128 (1 byte/cell: 0 walkable, 1 solid, 2 hazard) |
| `0x14803–0x1BFFF` | Reserved |
