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
            Self::Custom(_) => "Custom",
        }
    }
}

pub struct RomSection {
    pub kind: SectionKind,
    pub data: Vec<u8>,
}
