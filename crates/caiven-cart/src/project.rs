//! Project-directory authoring format: a `caiven.toml` manifest plus a plain
//! `.lua` entry file and one `.hex` file per non-empty asset section. This is
//! the git-friendly authoring counterpart to the binary `.cav` distribution
//! format — code diffs line-by-line, assets diff per hex line, no merge
//! conflicts across unrelated edits.
//!
//! ```text
//! my-game/
//!   caiven.toml
//!   main.lua
//!   sprites.hex       (__gfx__)
//!   sprite_flags.hex  (__flags__)
//!   map.hex           (__map__)
//!   palette.hex       (__pal__)
//!   sfx.hex           (__sfx__)
//!   music.hex         (__music__)
//! ```

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::CartError;
use crate::format::Cart;
use crate::header::CartHeader;
use crate::section::{CartSection, SectionKind};
use crate::text::{decode_hex_block, encode_hex_block, trim_trailing_zeros};

const MANIFEST_FILE: &str = "caiven.toml";
const DEFAULT_ENTRY: &str = "main.lua";

const SECTION_FILES: [(SectionKind, &str); 6] = [
    (SectionKind::SpriteSheet, "sprites.hex"),
    (SectionKind::Map, "map.hex"),
    (SectionKind::SpriteFlags, "sprite_flags.hex"),
    (SectionKind::Palette, "palette.hex"),
    (SectionKind::SfxBank, "sfx.hex"),
    (SectionKind::MusicBank, "music.hex"),
];

fn file_for(kind: SectionKind) -> Option<&'static str> {
    SECTION_FILES
        .iter()
        .find(|(k, _)| *k == kind)
        .map(|(_, f)| *f)
}

#[derive(Serialize, Deserialize)]
struct CaivenToml {
    cart: CartTable,
    #[serde(default)]
    mods: ModsTable,
}

#[derive(Serialize, Deserialize)]
struct CartTable {
    title: String,
    #[serde(default)]
    author: String,
    #[serde(default = "default_entry")]
    entry: String,
    #[serde(default)]
    entry_point: u32,
    #[serde(default)]
    flags: u32,
}

fn default_entry() -> String {
    DEFAULT_ENTRY.to_string()
}

#[derive(Serialize, Deserialize, Default)]
struct ModsTable {
    #[serde(default)]
    require: Vec<String>,
}

/// Returns `true` if `path` looks like a project (a directory containing
/// `caiven.toml`, or the `caiven.toml` file itself).
pub fn is_project(path: &Path) -> bool {
    if path.is_dir() {
        path.join(MANIFEST_FILE).is_file()
    } else {
        path.file_name().and_then(|n| n.to_str()) == Some(MANIFEST_FILE)
    }
}

fn resolve_dir(path: &Path) -> PathBuf {
    if path.is_dir() {
        path.to_path_buf()
    } else {
        path.parent().map(Path::to_path_buf).unwrap_or_default()
    }
}

/// Loads a project directory (or its `caiven.toml`) into the same in-memory
/// `Cart` shape the binary `.cav` loader produces.
pub fn load_project(path: &Path) -> Result<Cart, CartError> {
    let dir = resolve_dir(path);
    let manifest_path = dir.join(MANIFEST_FILE);
    let manifest_text = std::fs::read_to_string(&manifest_path)?;
    let manifest: CaivenToml = toml::from_str(&manifest_text)?;

    let header = CartHeader {
        title: manifest.cart.title,
        author: manifest.cart.author,
        entry_point: manifest.cart.entry_point,
        flags: manifest.cart.flags,
    };

    let entry_path = dir.join(&manifest.cart.entry);
    let lua = std::fs::read_to_string(&entry_path)
        .map_err(|_| CartError::MissingEntry(entry_path.display().to_string()))?;

    let mut sections = vec![CartSection {
        kind: SectionKind::LuaSource,
        data: lua.into_bytes(),
    }];

    for (kind, filename) in SECTION_FILES {
        let asset_path = dir.join(filename);
        if !asset_path.is_file() {
            continue;
        }
        let text = std::fs::read_to_string(&asset_path)?;
        let data = decode_hex_block(&text).map_err(|message| CartError::BadHex {
            file: filename.to_string(),
            message,
        })?;
        if !data.is_empty() {
            sections.push(CartSection { kind, data });
        }
    }

    if !manifest.mods.require.is_empty() {
        sections.push(CartSection {
            kind: SectionKind::ModManifest,
            data: manifest.mods.require.join("\n").into_bytes(),
        });
    }

    Ok(Cart {
        header,
        program: Vec::new(),
        sections,
    })
}

/// Writes `header`, `lua` source, and `sections` out as a project directory
/// at `dir`, creating it if needed. Sections with no asset file mapping
/// (`Program`, `Meta`, `LuaSource`) are ignored; `ModManifest` is folded into
/// `caiven.toml`'s `[mods].require` instead of a `.hex` file. Asset sections
/// that trim to empty have their `.hex` file removed if present, so deleting
/// all sprites in the editor cleans up the file instead of leaving zeros.
pub fn save_project(
    dir: &Path,
    header: &CartHeader,
    lua: &str,
    sections: &[(SectionKind, Vec<u8>)],
) -> Result<(), CartError> {
    std::fs::create_dir_all(dir)?;

    let mut require = Vec::new();
    for (kind, data) in sections {
        if *kind == SectionKind::ModManifest {
            let text = String::from_utf8_lossy(data);
            require.extend(
                text.lines()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_string),
            );
        }
    }

    let manifest = CaivenToml {
        cart: CartTable {
            title: header.title.clone(),
            author: header.author.clone(),
            entry: DEFAULT_ENTRY.to_string(),
            entry_point: header.entry_point,
            flags: header.flags,
        },
        mods: ModsTable { require },
    };
    let manifest_text =
        toml::to_string_pretty(&manifest).map_err(|e| CartError::MissingEntry(e.to_string()))?;
    std::fs::write(dir.join(MANIFEST_FILE), manifest_text)?;
    std::fs::write(dir.join(DEFAULT_ENTRY), lua)?;

    for (kind, data) in sections {
        let Some(filename) = file_for(*kind) else {
            continue;
        };
        let asset_path = dir.join(filename);
        let trimmed = trim_trailing_zeros(data);
        if trimmed.is_empty() {
            let _ = std::fs::remove_file(&asset_path);
            continue;
        }
        std::fs::write(&asset_path, encode_hex_block(trimmed))?;
    }

    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_preserves_header_lua_and_sections() {
        let dir = tempfile::tempdir().unwrap();
        let mut header = CartHeader::new("My Game", "andrej");
        header.entry_point = 5;
        header.flags = 2;
        let lua = "function _update() end\n";
        let sections = vec![
            (SectionKind::SpriteSheet, vec![1u8, 2, 3, 0]),
            (SectionKind::ModManifest, b"rtc\ninput".to_vec()),
        ];

        save_project(dir.path(), &header, lua, &sections).unwrap();
        let cart = load_project(dir.path()).unwrap();

        assert_eq!(cart.header.title, "My Game");
        assert_eq!(cart.header.author, "andrej");
        assert_eq!(cart.header.entry_point, 5);
        assert_eq!(cart.header.flags, 2);

        let lua_section = cart
            .sections
            .iter()
            .find(|s| s.kind == SectionKind::LuaSource)
            .unwrap();
        assert_eq!(String::from_utf8_lossy(&lua_section.data), lua);

        let gfx = cart
            .sections
            .iter()
            .find(|s| s.kind == SectionKind::SpriteSheet)
            .unwrap();
        assert_eq!(gfx.data, vec![1, 2, 3]);

        let manifest = cart
            .sections
            .iter()
            .find(|s| s.kind == SectionKind::ModManifest)
            .unwrap();
        assert_eq!(String::from_utf8_lossy(&manifest.data), "rtc\ninput");
    }

    #[test]
    fn missing_asset_files_are_simply_absent() {
        let dir = tempfile::tempdir().unwrap();
        let header = CartHeader::new("Blank", "");
        save_project(dir.path(), &header, "-- empty\n", &[]).unwrap();

        let cart = load_project(dir.path()).unwrap();
        assert!(
            cart.sections
                .iter()
                .all(|s| s.kind == SectionKind::LuaSource)
        );
    }

    #[test]
    fn missing_entry_file_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join(MANIFEST_FILE),
            "[cart]\ntitle = \"X\"\nentry = \"main.lua\"\n",
        )
        .unwrap();

        assert!(matches!(
            load_project(dir.path()),
            Err(CartError::MissingEntry(_))
        ));
    }

    #[test]
    fn is_project_detects_dir_and_manifest_path() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!is_project(dir.path()));
        std::fs::write(dir.path().join(MANIFEST_FILE), "[cart]\ntitle=\"x\"\n").unwrap();
        assert!(is_project(dir.path()));
        assert!(is_project(&dir.path().join(MANIFEST_FILE)));
    }

    #[test]
    fn saving_empty_asset_removes_stale_hex_file() {
        let dir = tempfile::tempdir().unwrap();
        let header = CartHeader::new("Game", "");
        save_project(
            dir.path(),
            &header,
            "-- code\n",
            &[(SectionKind::SpriteSheet, vec![1, 2, 3])],
        )
        .unwrap();
        assert!(dir.path().join("sprites.hex").is_file());

        save_project(
            dir.path(),
            &header,
            "-- code\n",
            &[(SectionKind::SpriteSheet, vec![0, 0, 0])],
        )
        .unwrap();
        assert!(!dir.path().join("sprites.hex").is_file());
    }
}
