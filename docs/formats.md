# Formats and public contracts

Every persisted format and the Port API are at **version 1**, the first public
baseline. A reader accepts exactly the version it knows and rejects anything
else with a readable error; older shapes are never read. A change that would
make old bytes misread bumps the version (see `.claude/rules/cart-format.md`).

| Contract | Version | Owner |
| --- | --- | --- |
| Cartridge `.cav` | 1 | `crates/caiven-cart/src/format.rs`, browser mirror `crates/caiven-port/web/src/lib/cav.js` |
| Project `caiven.toml` | 1 | `crates/caiven-cart/src/project.rs` |
| Save data (`save_data`/`load_data`) | 1 | `crates/caiven-vm/src/vm/save_data.rs` |
| Machine save state | 1 | `crates/caiven-machine/src/shell/save_state.rs` |
| Port REST API | `/api/v1` | `crates/caiven-port/src/handlers` — see [port.md](port.md) |
| Port database | baseline `m20260929_000001_initial_schema` | `crates/migration` |

## Cartridge `.cav`

All integers little-endian. Maximum size 128 KiB.

| Offset | Bytes | Field |
| --- | --- | --- |
| 0 | 6 | Magic `CAIVEN` |
| 6 | 2 | Format version `1` |
| 8 | 2 | Section count `n` |
| 10 | 32 | Title — UTF-8, zero-padded, cut on a character boundary |
| 42 | 32 | Author — same encoding |
| 74 | 14 × n | Section table: kind `u16`, offset `u32`, length `u32`, CRC-32 `u32` |
| … | … | Payloads at their offsets |

Readers reject: wrong magic or version, truncation, a payload overlapping the
header, table or another payload, a CRC mismatch, and anything but exactly one
`LuaSource`. Unknown kinds are carried through untouched.

| Id | Kind | Payload |
| --- | --- | --- |
| `0x01` | `LuaSource` | Lua 5.4 source; sibling modules bundled in |
| `0x02` | `SpriteSheet` | Default sprite bank |
| `0x03` | `Map` | Default map, 192 × 128 tile ids |
| `0x04` | `SfxBank` | Default SFX bank |
| `0x05` | `MusicBank` | Default music bank |
| `0x06` | `Palette` | Default palette, RGB triples |
| `0x07` | `Meta` | Studio metadata (JSON); runtimes ignore it |
| `0x08` | `ModManifest` | Required peripherals, one per line |
| `0x09`–`0x0D` | `SpriteBank`, `MapBank`, `PaletteBank`, `SfxBanks`, `MusicBanks` | Named bank: `[name_len u8][name][data]` |
| `0x0E` | `Collision` | Default collision layer, 1 byte per map cell |
| `0x0F` | `CollisionBank` | Named collision layer, same wrapper as other banks |
| `0x10` | `CollisionTypes` | `[count u8]`, then per type `id u8, flags u8, rgb[3], name_len u8, name` |
| `0x11` | `PreludeModules` | `[stdlib] modules`, one per line; present (possibly empty) only when `[stdlib]` is declared |

Asset payloads may be shorter than their region; loaders zero-pad them. Bank
names are 1–31 of `A-Z a-z 0-9 _ -`. The content hash Port uses for
duplicate detection covers the sections only (sorted), not the header.

## Project directory

The authoring format: `caiven.toml`, the entry Lua file and its sibling
modules, and one file per non-empty asset.

```toml
[cart]
version = 1        # required
title = "My Game"  # required
author = ""
entry = "main.lua"

[mods]
require = []

[stdlib]           # optional; absent means core only
modules = ["vec2", "collision"]
```

| File | Content |
| --- | --- |
| `sprites.png` / `.hex`, `map.png` / `.hex`, `palette.png` / `.hex` | Default banks; an existing `.hex` stays `.hex` on save |
| `sfx.hex`, `music.hex`, `collision.hex` | Default banks, hex only |
| `<asset>_<name>.png` / `.hex` | Named bank `name` |
| `collision_types.json` | Only when types differ from the built-ins: `[{id, name, color: [r, g, b], shape}]`, `shape` one of `none`, `solid`, `one_way`, `slope_left`, `slope_right` |

## Save data

`CVSD`, version `u16` = 1, blob length `u32`, then the JSON blob (at most
4096 bytes). Stored per cart: Machine `saves/<cart id>.cavdata`, Studio
`<cart>.cav.data` or `<project>/.caiven.data`, browser `localStorage`
`caiven:save:<key>` (base64).

## Machine save state

`CVST`, version `u16` = 1, RAM length `u32` + RAM, palette length `u16` +
palette. Stored as `saves/<cart id>.cavstate`.

## Studio debugger sidecar

TOML next to the cart (`<cart>.cav.dbg`) or inside the project
(`.caiven.dbg`): `watches = ["expr"]` and `[[breakpoints]]` tables with
`source` and `line`.

## Port database

One baseline migration creates the whole schema. Schema changes are new
migrations after it; a pre-baseline development database has to be
recreated. When an account is deleted its carts stay public: `owner_id`
becomes `NULL` (`ON DELETE SET NULL`) and the author reads `[deleted]`.
