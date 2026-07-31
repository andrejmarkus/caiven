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
    LuaSource,
    /// Additional sprite sheet. Data starts with bank id, followed by pixels.
    SpriteBank,
    /// Additional tile map. Data starts with bank id, followed by tile ids.
    MapBank,
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
    /// Cart-global collision-type table (names/colors/solid flags). Small
    /// metadata, not RAM-backed — see `encode_collision_types`.
    CollisionTypes,
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
            Self::LuaSource => 0x000A,
            Self::SpriteBank => 0x000B,
            Self::MapBank => 0x000C,
            Self::PaletteBank => 0x000E,
            Self::SfxBanks => 0x000F,
            Self::MusicBanks => 0x0010,
            Self::Collision => 0x0011,
            Self::CollisionBank => 0x0012,
            Self::CollisionTypes => 0x0013,
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
            0x000A => Self::LuaSource,
            0x000B => Self::SpriteBank,
            0x000C => Self::MapBank,
            0x000E => Self::PaletteBank,
            0x000F => Self::SfxBanks,
            0x0010 => Self::MusicBanks,
            0x0011 => Self::Collision,
            0x0012 => Self::CollisionBank,
            0x0013 => Self::CollisionTypes,
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
            Self::LuaSource => "LuaSource",
            Self::SpriteBank => "SpriteBank",
            Self::MapBank => "MapBank",
            Self::PaletteBank => "PaletteBank",
            Self::SfxBanks => "SfxBanks",
            Self::MusicBanks => "MusicBanks",
            Self::Collision => "Collision",
            Self::CollisionBank => "CollisionBank",
            Self::CollisionTypes => "CollisionTypes",
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

/// Encodes the cart-global collision-type table. Layout: `u8` count, then
/// per entry `id:u8, flags:u8, color:[u8;3], name_len:u8, name:utf8`.
/// Self-describing and forward-compatible — unknown flag bits round-trip.
pub fn encode_collision_types(types: &[caiven_core::CollisionType]) -> Vec<u8> {
    let mut out = Vec::with_capacity(1 + types.len() * 8);
    out.push(types.len().min(u8::MAX as usize) as u8);
    for t in types.iter().take(u8::MAX as usize) {
        out.push(t.id);
        out.push(t.flags.bits());
        out.extend_from_slice(&t.color);
        let name_bytes = t.name.as_bytes();
        let name_len = name_bytes.len().min(u8::MAX as usize);
        out.push(name_len as u8);
        out.extend_from_slice(&name_bytes[..name_len]);
    }
    out
}

/// Decodes a collision-type table encoded by `encode_collision_types`.
/// Malformed/truncated data yields as many valid leading entries as
/// possible (never panics).
pub fn decode_collision_types(data: &[u8]) -> Vec<caiven_core::CollisionType> {
    let mut types = Vec::new();
    let Some((&count, mut rest)) = data.split_first() else {
        return types;
    };
    for _ in 0..count {
        let [id, flags, r, g, b, name_len, tail @ ..] = rest else {
            break;
        };
        let name_len = *name_len as usize;
        if tail.len() < name_len {
            break;
        }
        let (name_bytes, after) = tail.split_at(name_len);
        let name = String::from_utf8_lossy(name_bytes).into_owned();
        types.push(caiven_core::CollisionType {
            id: *id,
            name,
            color: [*r, *g, *b],
            flags: caiven_core::CollisionTypeFlags::from_bits(*flags),
        });
        rest = after;
    }
    types
}

pub struct CartSection {
    pub kind: SectionKind,
    pub data: Vec<u8>,
}

#[cfg(test)]
mod collision_type_tests {
    use super::*;
    use caiven_core::{CollisionType, CollisionTypeFlags};

    #[test]
    fn roundtrips_builtins_and_custom_type() {
        let mut types = caiven_core::builtin_collision_types();
        types.push(CollisionType {
            id: 3,
            name: "water".to_string(),
            color: [0, 128, 255],
            flags: CollisionTypeFlags::from_bits(0),
        });
        let encoded = encode_collision_types(&types);
        let decoded = decode_collision_types(&encoded);
        assert_eq!(decoded, types);
    }

    #[test]
    fn roundtrips_max_length_name_and_unknown_flag_bits() {
        let name: String = "a".repeat(255);
        let types = vec![CollisionType {
            id: 5,
            name: name.clone(),
            color: [1, 2, 3],
            flags: CollisionTypeFlags::from_bits(0b1000_0001),
        }];
        let decoded = decode_collision_types(&encode_collision_types(&types));
        assert_eq!(decoded.len(), 1);
        assert_eq!(decoded[0].name, name);
        assert_eq!(decoded[0].flags.bits(), 0b1000_0001);
        assert!(decoded[0].flags.is_solid());
    }

    #[test]
    fn decode_empty_and_truncated_data_does_not_panic() {
        assert!(decode_collision_types(&[]).is_empty());
        assert!(decode_collision_types(&[1]).is_empty());
        assert!(decode_collision_types(&[1, 0, 0]).is_empty());
    }
}
