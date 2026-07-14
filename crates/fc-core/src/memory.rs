//! Canonical memory map and system dimensions for the fantasy console.
//!
//! Single source of truth shared by the assembler (section load targets),
//! the VM (RAM layout), the editors (direct RAM peeks/pokes), the compiler
//! (heap/scratch placement), and the host.
//!
//! RAM layout (64 KiB):
//! ```text
//! 0x0000 ─ 0x3FFF   general purpose / program data / compiler scratch
//! 0x4000 ─ 0x7FFF   sprite sheet (256 sprites × 8×8 px, 1 byte per px index)
//! 0x8000 ─ 0x8FFF   tile map (64 × 64 tiles, 1 byte per tile)
//! 0x9000 ─ 0x90FF   sprite flags (256 sprites × 1 byte)
//! 0x9100 ─ 0x91FF   palette (16 slots × 3 bytes RGB)
//! 0x9200 ─ 0x95FF   SFX bank (16 sfx × 64 bytes)
//! 0x9600 ─ 0x96FF   music bank (8 patterns × 32 bytes)
//! 0x9700 ─ 0xFFFF   heap (grows up) / stack (grows down from 0x10000)
//! ```

/// Screen width in pixels.
pub const SCREEN_WIDTH: u32 = 128;
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
/// Number of palette slots.
pub const PALETTE_SIZE: usize = 16;

/// Tile map width in tiles.
pub const MAP_W: usize = 64;
/// Tile map height in tiles.
pub const MAP_H: usize = 64;

/// Total RAM size in bytes.
pub const RAM_SIZE: usize = 64 * 1024;

/// RAM base address where the SpriteSheet ROM section is auto-loaded.
pub const SPRITE_SHEET_RAM_BASE: usize = 0x4000;
/// Sprite sheet length in bytes (256 sprites × 64 bytes).
pub const SPRITE_SHEET_LEN: usize = SPRITE_COUNT * SPRITE_BYTES;
/// RAM base address where the Map ROM section is auto-loaded.
pub const MAP_RAM_BASE: usize = 0x8000;
/// Tile map length in bytes (64 × 64 tiles).
pub const MAP_LEN: usize = MAP_W * MAP_H;
/// RAM base address of the sprite flags table (1 byte per sprite).
pub const SPRITE_FLAGS_RAM_BASE: usize = 0x9000;
/// Sprite flags table length in bytes.
pub const SPRITE_FLAGS_LEN: usize = SPRITE_COUNT;
/// RAM base address where the Palette ROM section is auto-loaded.
pub const PALETTE_RAM_BASE: usize = 0x9100;
/// RAM base address of the SFX bank.
pub const SFX_RAM_BASE: usize = 0x9200;
/// SFX bank length in bytes (16 sfx × 64 bytes).
pub const SFX_BANK_LEN: usize = 16 * 64;
/// RAM base address of the music bank.
pub const MUSIC_RAM_BASE: usize = 0x9600;
/// Music bank length in bytes (8 patterns × 32 bytes).
pub const MUSIC_BANK_LEN: usize = 8 * 32;
/// RAM base address of the runtime heap (compiler bump allocator; grows up
/// toward the stack, which grows down from the top of RAM).
pub const HEAP_RAM_BASE: usize = 0x9700;
