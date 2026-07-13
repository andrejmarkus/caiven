//! Canonical memory map and system dimensions for the fantasy console.
//!
//! Single source of truth shared by the assembler (section load targets),
//! the VM (RAM layout), the editors (direct RAM peeks/pokes), and the host.
//!
//! RAM layout (32 KiB):
//! ```text
//! 0x0000 ─ 0x3FFF   general purpose / program data
//! 0x4000 ─ 0x4FFF   sprite sheet (256 sprites × 8×8 px, 1 byte per px index)
//! 0x5000 ─ 0x57FF   tile map
//! 0x5800 ─ 0x5BFF   palette (16 slots × 3 bytes RGB)
//! 0x5C00 ─ 0x5FFF   SFX bank (16 sfx × 64 bytes)
//! 0x6000 ─ 0x60FF   music bank (8 patterns × 32 bytes)
//! ```

/// Screen width in pixels.
pub const SCREEN_WIDTH: u32 = 128;
/// Screen height in pixels.
pub const SCREEN_HEIGHT: u32 = 128;
/// Bytes per pixel in RGBA output buffers.
pub const RGBA_BYTES: usize = 4;

/// Sprite edge length in pixels (sprites are square).
pub const SPRITE_SIZE: u32 = 8;
/// Number of palette slots.
pub const PALETTE_SIZE: usize = 16;

/// Total RAM size in bytes.
pub const RAM_SIZE: usize = 32 * 1024;

/// RAM base address where the SpriteSheet ROM section is auto-loaded.
pub const SPRITE_SHEET_RAM_BASE: usize = 0x4000;
/// RAM base address where the Map ROM section is auto-loaded.
pub const MAP_RAM_BASE: usize = 0x5000;
/// RAM base address where the Palette ROM section is auto-loaded.
pub const PALETTE_RAM_BASE: usize = 0x5800;
/// RAM base address of the SFX bank.
pub const SFX_RAM_BASE: usize = 0x5C00;
/// SFX bank length in bytes (16 sfx × 64 bytes).
pub const SFX_BANK_LEN: usize = 16 * 64;
/// RAM base address of the music bank.
pub const MUSIC_RAM_BASE: usize = 0x6000;
/// Music bank length in bytes (8 patterns × 32 bytes).
pub const MUSIC_BANK_LEN: usize = 8 * 32;
