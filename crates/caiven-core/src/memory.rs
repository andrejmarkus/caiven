//! Canonical memory map and system dimensions for the fantasy console.
//!
//! Single source of truth shared by the assembler (section load targets),
//! the VM (RAM layout), the editors (direct RAM peeks/pokes), the compiler
//! (heap/scratch placement), and the host.
//!
//! Every region's base address is *derived*, not hand-typed: [`MemRegion::ORDER`] lists
//! regions in ascending-address order, [`MemRegion::span`] gives each one's reserved size,
//! and [`MemRegion::base`] sums the spans of every region before it. To add, remove, resize,
//! or reorder a region, edit `ORDER`/`span`/`len` here — every base downstream shifts
//! automatically, and `cargo test -p caiven-core` (see `tests/memory_map_sync.rs`) will fail
//! naming any frontend/README literal that's now out of sync.
//!
//! Address space layout (112 KiB). The console's *general-purpose* RAM is the
//! ~45 KiB of Work + Heap; the asset windows (sprite sheet, map, palette, sfx,
//! music, collision) sit alongside it in their own regions, so enlarging one of
//! them never costs a cart the memory it writes its own data into.
//! ```text
//! 0x0000 ─ 0x3FFF   general purpose / program data / compiler scratch
//! 0x4000 ─ 0x7FFF   sprite sheet (256 sprites × 8×8 px, 1 byte per px index)
//! 0x8000 ─ 0xDFFF   tile map (192 × 128 tiles, 1 byte per tile)
//! 0xE000 ─ 0xE0FF   palette (16 slots × 3 bytes RGB)
//! 0xE100 ─ 0xE4FF   SFX bank (16 sfx × 64 bytes)
//! 0xE500 ─ 0xE7FF   music bank (8 patterns × 64 bytes, then the song order
//!                   table + its loop-point byte)
//! 0xE800 ─ 0xE802   RTC peripheral (hour, minute, second)
//! 0xE803 ─ 0x14802  per-cell collision (192 × 128 tiles, 1 byte per cell)
//! 0x14803 ─ 0x1BFFF general purpose / heap
//! ```

/// Screen width in pixels.
pub const SCREEN_WIDTH: u32 = 192;
/// Screen height in pixels.
pub const SCREEN_HEIGHT: u32 = 128;
/// Bytes per pixel in RGBA output buffers.
pub const RGBA_BYTES: usize = 4;

/// Sprite edge length in pixels (sprites are square).
pub const SPRITE_SIZE: u32 = 8;
/// Bytes per sprite (8×8 px, 1 byte per pixel).
pub const SPRITE_BYTES: usize = (SPRITE_SIZE * SPRITE_SIZE) as usize;
/// Number of sprites in the sprite sheet.
pub const SPRITE_COUNT: usize = 256;
/// Sprite sheet width in sprites — the sheet is laid out row-major at this
/// stride, so adjacent sprite ids sit side by side (matches Studio's sprite
/// picker grid, `.sprite-sheet` in `studio.css`).
pub const SPRITE_SHEET_COLS: usize = 16;
/// Number of palette slots.
pub const PALETTE_SIZE: usize = 16;

/// Tile map width in tiles — 8 screens wide (24-tile screen × 8), matching
/// the map's 8-screen height so a level is a whole number of screens in both
/// directions instead of a partial trailing column.
pub const MAP_W: usize = 192;
/// Tile map height in tiles.
pub const MAP_H: usize = 128;

/// Total addressable memory in bytes. Larger than the ~45 KiB of general-purpose
/// RAM (Work + Heap) because the asset windows are mapped alongside it rather
/// than carved out of it.
pub const RAM_SIZE: usize = 112 * 1024;

/// Sprite sheet length in bytes (256 sprites × 64 bytes).
pub const SPRITE_SHEET_LEN: usize = SPRITE_COUNT * SPRITE_BYTES;
/// Tile map length in bytes (192 × 128 tiles).
pub const MAP_LEN: usize = MAP_W * MAP_H;
/// SFX slots per bank.
pub const SFX_COUNT: usize = 16;
/// SFX bank length in bytes (`SFX_COUNT` sfx × 64 bytes).
pub const SFX_BANK_LEN: usize = SFX_COUNT * 64;
/// Music patterns per bank.
pub const MUSIC_PATTERN_COUNT: usize = 8;
/// Rows per music pattern.
pub const MUSIC_PATTERN_ROWS: usize = 16;
/// Typed music channels: 2 pulse, 1 triangle, 1 noise. One byte per channel
/// per row holds that row's SFX reference, so this is also the row stride.
pub const MUSIC_CHANNEL_COUNT: usize = 4;
/// Pattern data length in bytes (8 patterns × 16 rows × 4 channels).
pub const MUSIC_PATTERN_DATA_LEN: usize =
    MUSIC_PATTERN_COUNT * MUSIC_PATTERN_ROWS * MUSIC_CHANNEL_COUNT;
/// Song-order steps per music bank.
pub const MUSIC_ORDER_STEPS: usize = 32;
/// Song section length in bytes: the order table plus its loop-point byte.
pub const MUSIC_SONG_LEN: usize = MUSIC_ORDER_STEPS + 1;
/// Byte offset within the music bank where the song order table starts.
pub const MUSIC_ORDER_OFFSET: usize = MUSIC_PATTERN_DATA_LEN;
/// Byte offset within the music bank of the loop-point byte.
pub const MUSIC_LOOP_POINT_OFFSET: usize = MUSIC_ORDER_OFFSET + MUSIC_ORDER_STEPS;
/// Music bank length in bytes (pattern data plus the song order section).
pub const MUSIC_BANK_LEN: usize = MUSIC_PATTERN_DATA_LEN + MUSIC_SONG_LEN;
/// RTC register block length in bytes (hour, minute, second).
pub const RTC_LEN: usize = 3;
/// Collision layer length in bytes (128 × 128 cells, 1 byte per cell).
pub const COLLISION_LEN: usize = MAP_W * MAP_H;

/// One region of the RAM memory map, in the order it appears in address space.
///
/// This is the map's single source of truth: [`ORDER`](Self::ORDER) fixes the sequence,
/// [`span`](Self::span) fixes how much address space each region reserves (its stride to
/// the next region), and [`base`](Self::base) derives every address by summing the spans
/// of everything before it. Reorder, insert, or resize a region by editing `ORDER`/`span`
/// here — nothing else in this file needs to change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemRegion {
    /// General purpose / program data / compiler scratch.
    Work,
    SpriteSheet,
    Map,
    Palette,
    Sfx,
    Music,
    /// RTC peripheral's mapped registers.
    Rtc,
    /// Per-cell collision layer (companion of Map).
    Collision,
    /// General purpose / heap — everything left over at the top of RAM.
    Heap,
}

impl MemRegion {
    /// Every region, ascending by address. The order here *is* the memory map.
    pub const ORDER: [MemRegion; 9] = [
        MemRegion::Work,
        MemRegion::SpriteSheet,
        MemRegion::Map,
        MemRegion::Palette,
        MemRegion::Sfx,
        MemRegion::Music,
        MemRegion::Rtc,
        MemRegion::Collision,
        MemRegion::Heap,
    ];

    /// Address space this region reserves — its stride to the next region's base.
    /// Equal to [`len`](Self::len) except where a region pads out to a round boundary
    /// (e.g. `Palette` reserves 0x100 for 48 payload bytes).
    pub const fn span(self) -> usize {
        match self {
            MemRegion::Work => 0x4000,
            MemRegion::SpriteSheet => SPRITE_SHEET_LEN,
            MemRegion::Map => 0x6000,
            MemRegion::Palette => 0x100,
            MemRegion::Sfx => 0x400,
            MemRegion::Music => 0x300,
            MemRegion::Rtc => RTC_LEN,
            MemRegion::Collision => 0x6000,
            // Everything left over at the top of RAM.
            MemRegion::Heap => RAM_SIZE - MemRegion::Heap.base(),
        }
    }

    /// Payload size actually read/written — may be smaller than [`span`](Self::span)
    /// when a region pads to a round boundary. `Work`/`Heap` have no fixed payload.
    #[allow(clippy::len_without_is_empty)] // not a collection; `len() == 0` is a valid region, not "empty"
    pub const fn len(self) -> usize {
        match self {
            MemRegion::Work => 0,
            MemRegion::SpriteSheet => SPRITE_SHEET_LEN,
            MemRegion::Map => MAP_LEN,
            MemRegion::Palette => PALETTE_SIZE * 3,
            MemRegion::Sfx => SFX_BANK_LEN,
            MemRegion::Music => MUSIC_BANK_LEN,
            MemRegion::Rtc => RTC_LEN,
            MemRegion::Collision => COLLISION_LEN,
            MemRegion::Heap => 0,
        }
    }

    /// Base address: the sum of every preceding region's [`span`](Self::span) in
    /// [`ORDER`](Self::ORDER).
    pub const fn base(self) -> usize {
        let mut total = 0usize;
        let mut i = 0usize;
        while i < MemRegion::ORDER.len() {
            let region = MemRegion::ORDER[i];
            if region as usize == self as usize {
                break;
            }
            total += region.span();
            i += 1;
        }
        total
    }
}

/// RAM base address where the SpriteSheet cart section is auto-loaded.
pub const SPRITE_SHEET_RAM_BASE: usize = MemRegion::SpriteSheet.base();
/// RAM base address where the Map cart section is auto-loaded.
pub const MAP_RAM_BASE: usize = MemRegion::Map.base();
/// RAM base address where the Palette cart section is auto-loaded.
pub const PALETTE_RAM_BASE: usize = MemRegion::Palette.base();
/// RAM base address of the SFX bank.
pub const SFX_RAM_BASE: usize = MemRegion::Sfx.base();
/// RAM base address of the music bank.
pub const MUSIC_RAM_BASE: usize = MemRegion::Music.base();
/// RAM base address of the RTC peripheral's mapped registers.
pub const RTC_RAM_BASE: usize = MemRegion::Rtc.base();
/// RAM base address of the per-cell collision layer (companion of Map).
///
/// Note: the collision-*type* table (names/colors/solid flags, see
/// `caiven_core::collision`) is small cart-global metadata carried
/// out-of-band in a cart section — it is deliberately not a `MemRegion`
/// and has no RAM window.
pub const COLLISION_RAM_BASE: usize = MemRegion::Collision.base();
/// RAM base address of general-purpose/heap space.
pub const HEAP_RAM_BASE: usize = MemRegion::Heap.base();

// Compile-time guard: the memory map must fit the address space. Resizing a region so
// the map overflows fails the build here instead of silently corrupting addresses.
const _: () = assert!(MemRegion::Heap.base() + MemRegion::Heap.span() == RAM_SIZE);

#[cfg(test)]
mod tests {
    use super::*;

    /// Golden guard: every derived base matches its known current address. Catches an
    /// accidental remap (wrong `ORDER`/`span` edit) as well as region overlap, since an
    /// overlap would shift every following base off its golden value.
    #[test]
    fn ram_regions_have_expected_bases() {
        let expected: [(MemRegion, usize); 9] = [
            (MemRegion::Work, 0x0000),
            (MemRegion::SpriteSheet, 0x4000),
            (MemRegion::Map, 0x8000),
            (MemRegion::Palette, 0xE000),
            (MemRegion::Sfx, 0xE100),
            (MemRegion::Music, 0xE500),
            (MemRegion::Rtc, 0xE800),
            (MemRegion::Collision, 0xE803),
            (MemRegion::Heap, 0x14803),
        ];

        for (region, want) in expected {
            let got = region.base();
            assert_eq!(
                got, want,
                "{region:?} base is 0x{got:04X}, expected 0x{want:04X}"
            );
        }
    }

    /// Every mapped RAM region, in ascending address order. Catches the class of bug
    /// this map has already hit (a shifted base silently overlapping its neighbor) by
    /// asserting no region's end runs past the next region's start.
    #[test]
    fn ram_regions_do_not_overlap_and_fit_in_ram() {
        let mut prev_end = 0usize;
        let mut prev = MemRegion::Work;
        for region in MemRegion::ORDER {
            let base = region.base();
            assert!(
                base >= prev_end,
                "{region:?} (0x{base:04X}) overlaps {prev:?} (ends 0x{prev_end:04X})"
            );
            prev_end = base + region.span();
            prev = region;
        }
        assert_eq!(
            prev_end, RAM_SIZE,
            "map does not exactly fill the address space"
        );
    }
}
