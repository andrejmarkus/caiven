# Formats and public contracts

Each file format Caiven writes, and the Port API, is at version 1, the first
public baseline. Readers accept only the version they know and reject
anything else with a readable error. They don't read older layouts. If a
change would make old files parse wrong, bump the version.

| Contract | Version | Owner |
| --- | --- | --- |
| Cartridge `.cav` | 1 | `crates/caiven-cart/src/format.rs`, browser mirror `crates/caiven-port/web/src/lib/cav.js` |
| Project `caiven.toml` | 1 | `crates/caiven-cart/src/project.rs` |
| Save data (`save_data`/`load_data`) | 1 | `crates/caiven-vm/src/vm/save_data.rs` |
| Machine save state | 1 | `crates/caiven-machine/src/shell/save_state.rs` |
| Port REST API | `/api/v1` | `crates/caiven-port/src/handlers`, see [port.md](port.md) |
| Port database | baseline `m20260929_000001_initial_schema` | `crates/migration` |

## Cartridge `.cav`

Integers are little-endian. A cart can be at most 128 KiB.

| Offset | Bytes | Field |
| --- | --- | --- |
| 0 | 6 | Magic `CAIVEN` |
| 6 | 2 | Format version `1` |
| 8 | 2 | Section count `n` |
| 10 | 32 | Title: UTF-8, zero-padded, cut on a character boundary |
| 42 | 32 | Author, same encoding |
| 74 | 14 × n | Section table: kind `u16`, offset `u32`, length `u32`, CRC-32 `u32` |
| … | … | Payloads at their offsets |

A reader rejects the file if the magic or version is wrong, the file is
truncated, a payload overlaps the header, the table or another payload, a
CRC doesn't match, or the cart has anything other than one `LuaSource`
section. Readers keep sections of unknown kinds as they are.

| Id | Kind | Payload |
| --- | --- | --- |
| `0x01` | `LuaSource` | Lua 5.4 source; sibling modules bundled in |
| `0x02` | `SpriteSheet` | Default sprite bank |
| `0x03` | `Map` | Default map, 192 × 128 tile ids |
| `0x04` | `SfxBank` | Default SFX bank |
| `0x05` | `MusicBank` | Default music bank |
| `0x06` | `Palette` | Default palette, RGB triples |
| `0x07` | `Meta` | Studio metadata (JSON); runtimes ignore it |
| `0x08`–`0x0C` | `SpriteBank`, `MapBank`, `PaletteBank`, `SfxBanks`, `MusicBanks` | Named bank: `[name_len u8][name][data]` |
| `0x0D` | `Collision` | Default collision layer, 1 byte per map cell |
| `0x0E` | `CollisionBank` | Named collision layer, same wrapper as other banks |
| `0x0F` | `CollisionTypes` | `[count u8]`, then per type `id u8, flags u8, rgb[3], name_len u8, name` |

An asset payload can be shorter than its region, and the loader pads it
with zeros. Bank names have 1 to 31 characters from `A-Z a-z 0-9 _ -`. Port
detects duplicate uploads with a hash of the sorted sections; the header
doesn't count.

## Project directory

You edit games in this format: `caiven.toml`, the entry Lua file and the
modules next to it, and one file per non-empty asset.

```toml
[cart]
version = 1        # required
title = "My Game"  # required
author = ""
entry = "main.lua"
```

The loader ignores an old `[mods]` table, and saving removes it.

| File | Content |
| --- | --- |
| `sprites.png` / `.hex`, `map.png` / `.hex`, `palette.png` / `.hex` | Default banks; an existing `.hex` stays `.hex` on save |
| `sfx.hex`, `music.hex`, `collision.hex` | Default banks, hex only |
| `<asset>_<name>.png` / `.hex` | Named bank `name` |
| `collision_types.json` | Only when types differ from the built-ins: `[{id, name, color: [r, g, b], shape}]`, `shape` one of `none`, `solid`, `one_way`, `slope_left`, `slope_right` |

## Save data

`CVSD`, version `u16` = 1, blob length `u32`, then the JSON blob (at most
4096 bytes). Each cart has its own file. Machine writes
`saves/<cart id>.cavdata`, Studio writes `<cart>.cav.data` or
`<project>/.caiven.data`, and the browser writes base64 to `localStorage`
under `caiven:save:<key>`.

## Machine save state

`CVST`, version `u16` = 1, RAM length `u32` + RAM, palette length `u16` +
palette. Machine writes it to `saves/<cart id>.cavstate`.

## Studio debugger sidecar

TOML next to the cart (`<cart>.cav.dbg`) or inside the project
(`.caiven.dbg`): `watches = ["expr"]` and `[[breakpoints]]` tables with
`source` and `line`.

## Port database

One baseline migration creates the whole schema, and later schema changes
go in new migrations after it. If you have a development database from
before the baseline, recreate it. Deleting an account leaves its carts
public: `owner_id` becomes `NULL` (`ON DELETE SET NULL`) and the author shows
as `[deleted]`.
