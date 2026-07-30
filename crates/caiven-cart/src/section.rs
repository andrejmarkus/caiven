#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SectionKind {
    Program,
    SpriteSheet,
    Map,
    SfxBank,
    MusicBank,
    Palette,
    Meta,
    ModManifest,
    SpriteFlags,
    LuaSource,
    /// Additional sprite sheet. Data starts with bank id, followed by pixels.
    SpriteBank,
    /// Additional tile map. Data starts with bank id, followed by tile ids.
    MapBank,
    /// Additional sprite flags table, companion of a `SpriteBank`. Data
    /// starts with bank id, followed by one flag byte per sprite.
    SpriteFlagsBank,
    /// Additional palette. Data starts with bank id, followed by RGB triples.
    PaletteBank,
    /// Additional SFX bank. Data starts with bank id, followed by SFX bytes.
    SfxBanks,
    /// Additional music bank. Data starts with bank id, followed by pattern bytes.
    MusicBanks,
    /// Per-cell collision layer for the bank-0 map (64 × 64, one byte per cell).
    Collision,
    /// Additional collision layer, companion of a `MapBank`. Data starts
    /// with bank id, followed by one collision byte per cell.
    CollisionBank,
    Custom(u16),
}

impl SectionKind {
    pub fn to_u16(self) -> u16 {
        match self {
            Self::Program => 0x0001,
            Self::SpriteSheet => 0x0002,
            Self::Map => 0x0003,
            Self::SfxBank => 0x0004,
            Self::MusicBank => 0x0005,
            Self::Palette => 0x0006,
            Self::Meta => 0x0007,
            Self::ModManifest => 0x0008,
            Self::SpriteFlags => 0x0009,
            Self::LuaSource => 0x000A,
            Self::SpriteBank => 0x000B,
            Self::MapBank => 0x000C,
            Self::SpriteFlagsBank => 0x000D,
            Self::PaletteBank => 0x000E,
            Self::SfxBanks => 0x000F,
            Self::MusicBanks => 0x0010,
            Self::Collision => 0x0011,
            Self::CollisionBank => 0x0012,
            Self::Custom(n) => n,
        }
    }

    pub fn from_u16(v: u16) -> Self {
        match v {
            0x0001 => Self::Program,
            0x0002 => Self::SpriteSheet,
            0x0003 => Self::Map,
            0x0004 => Self::SfxBank,
            0x0005 => Self::MusicBank,
            0x0006 => Self::Palette,
            0x0007 => Self::Meta,
            0x0008 => Self::ModManifest,
            0x0009 => Self::SpriteFlags,
            0x000A => Self::LuaSource,
            0x000B => Self::SpriteBank,
            0x000C => Self::MapBank,
            0x000D => Self::SpriteFlagsBank,
            0x000E => Self::PaletteBank,
            0x000F => Self::SfxBanks,
            0x0010 => Self::MusicBanks,
            0x0011 => Self::Collision,
            0x0012 => Self::CollisionBank,
            n => Self::Custom(n),
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Program => "Program",
            Self::SpriteSheet => "SpriteSheet",
            Self::Map => "Map",
            Self::SfxBank => "SfxBank",
            Self::MusicBank => "MusicBank",
            Self::Palette => "Palette",
            Self::Meta => "Meta",
            Self::ModManifest => "ModManifest",
            Self::SpriteFlags => "SpriteFlags",
            Self::LuaSource => "LuaSource",
            Self::SpriteBank => "SpriteBank",
            Self::MapBank => "MapBank",
            Self::SpriteFlagsBank => "SpriteFlagsBank",
            Self::PaletteBank => "PaletteBank",
            Self::SfxBanks => "SfxBanks",
            Self::MusicBanks => "MusicBanks",
            Self::Collision => "Collision",
            Self::CollisionBank => "CollisionBank",
            Self::Custom(_) => "Custom",
        }
    }
}

/// Encodes an additional asset bank section. Bank 0 uses legacy
/// `SpriteSheet`/`Map` sections and must not use this wrapper.
pub fn encode_asset_bank(id: u8, data: &[u8]) -> Vec<u8> {
    let mut encoded = Vec::with_capacity(data.len() + 1);
    encoded.push(id);
    encoded.extend_from_slice(data);
    encoded
}

/// Decodes bank id and payload from an additional asset bank section.
pub fn decode_asset_bank(data: &[u8]) -> Option<(u8, &[u8])> {
    let (&id, payload) = data.split_first()?;
    (id != 0).then_some((id, payload))
}

pub struct CartSection {
    pub kind: SectionKind,
    pub data: Vec<u8>,
}
