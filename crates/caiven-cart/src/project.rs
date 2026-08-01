//! Project-directory authoring format: a `caiven.toml` manifest, a `.lua`
//! entry file (plus any sibling `.lua` modules, bundled together — see
//! `bundle.rs`), and one asset file per non-empty section. This is the
//! git-friendly authoring counterpart to the binary `.cav` distribution
//! format — code diffs line-by-line, sprites/palette/map are real PNGs,
//! sfx/music are per-line hex — no merge conflicts across unrelated
//! edits.
//!
//! ```text
//! my-game/
//!   caiven.toml
//!   main.lua
//!   sprites.png       (__gfx__,   or sprites.hex)
//!   sprites_1.png     (additional sprite bank 1)
//!   map.png           (__map__,   or map.hex)
//!   map_1.png         (additional map bank 1)
//!   palette.png       (__pal__,   or palette.hex)
//!   palette_1.png     (additional palette bank 1)
//!   sfx.hex           (__sfx__)
//!   sfx_1.hex         (additional SFX bank 1)
//!   music.hex         (__music__)
//!   music_1.hex       (additional music bank 1)
//!   collision.hex     (per-cell collision, companion of map)
//!   collision_1.hex   (additional collision bank 1, companion of map bank 1)
//!   collision_types.json  (cart-global collision-type table, if customized)
//! ```
//!
//! Sprites, map, and palette each support both `.png` (visual, editable in
//! any image tool) and `.hex` (per-line text diffs) — whichever file is
//! already on disk is preserved on save; a brand-new asset is written as
//! `.png`. SFX, music, and collision are index/audio data rather than
//! images, so they're `.hex` only. The collision-*type* table (names,
//! colors, solid flags) is cart-global metadata, not per-cell data, so it
//! gets its own `collision_types.json` — human-readable and git-friendly.
//! It's omitted when the table is exactly the three built-in types, so an
//! unmodified cart has no extra file.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::asset_png;
use crate::bundle::{bundle_lua, list_lua_files, module_key};
use crate::error::CartError;
use crate::format::Cart;
use crate::header::CartHeader;
use crate::section::{CartSection, SectionKind};
use crate::text::{decode_hex_block, encode_hex_block, trim_trailing_zeros};
use crate::{decode_asset_bank, decode_collision_types, encode_asset_bank, encode_collision_types};

const MANIFEST_FILE: &str = "caiven.toml";
const DEFAULT_ENTRY: &str = "main.lua";
const COLLISION_TYPES_FILE: &str = "collision_types.json";

/// Current `[cart].version` written to new/re-saved `caiven.toml` manifests.
const CURRENT_MANIFEST_VERSION: u16 = 1;

/// Oldest manifest version this build still loads. Existing carts predate
/// the `version` field entirely; those default to `CURRENT_MANIFEST_VERSION`
/// via `default_manifest_version` below, so this only guards against a
/// synthetic/future version this build genuinely doesn't understand.
const MIN_SUPPORTED_MANIFEST_VERSION: u16 = 1;

/// On-disk DTO for `collision_types.json` — a readable/diffable stand-in
/// for `caiven_core::CollisionType`, whose `flags` bitset is exposed here
/// as a plain `solid` bool (the only engine-affecting flag today).
#[derive(Serialize, Deserialize)]
struct CollisionTypeDto {
    id: u8,
    name: String,
    color: [u8; 3],
    solid: bool,
}

impl From<&caiven_core::CollisionType> for CollisionTypeDto {
    fn from(t: &caiven_core::CollisionType) -> Self {
        Self {
            id: t.id,
            name: t.name.clone(),
            color: t.color,
            solid: t.flags.is_solid(),
        }
    }
}

impl From<CollisionTypeDto> for caiven_core::CollisionType {
    fn from(dto: CollisionTypeDto) -> Self {
        let bits = if dto.solid {
            caiven_core::CollisionTypeFlags::SOLID
        } else {
            0
        };
        Self {
            id: dto.id,
            name: dto.name,
            color: dto.color,
            flags: caiven_core::CollisionTypeFlags::from_bits(bits),
        }
    }
}

/// Asset section kinds and their file stem, in load order — `Palette` comes
/// first so its bytes are available for `SpriteSheet`'s PNG decode/encode
/// (an indexed PNG's own PLTE is used when present; the loaded palette is
/// only a fallback for nearest-color matching a plain RGB PNG).
const SECTION_STEMS: [(SectionKind, &str); 6] = [
    (SectionKind::Palette, "palette"),
    (SectionKind::SpriteSheet, "sprites"),
    (SectionKind::Map, "map"),
    (SectionKind::SfxBank, "sfx"),
    (SectionKind::MusicBank, "music"),
    (SectionKind::Collision, "collision"),
];

/// Additional-bank section kinds (id != 0): the wrapper `SectionKind` that
/// carries `encode_asset_bank`-wrapped payload, the base kind used to pick
/// a PNG/hex codec, and the shared file stem (`{stem}_{id}.png`/`.hex`).
const BANK_KINDS: [(SectionKind, SectionKind, &str); 6] = [
    (SectionKind::SpriteBank, SectionKind::SpriteSheet, "sprites"),
    (SectionKind::MapBank, SectionKind::Map, "map"),
    (SectionKind::PaletteBank, SectionKind::Palette, "palette"),
    (SectionKind::SfxBanks, SectionKind::SfxBank, "sfx"),
    (SectionKind::MusicBanks, SectionKind::MusicBank, "music"),
    (
        SectionKind::CollisionBank,
        SectionKind::Collision,
        "collision",
    ),
];

fn stem_for(kind: SectionKind) -> Option<&'static str> {
    SECTION_STEMS
        .iter()
        .find(|(k, _)| *k == kind)
        .map(|(_, s)| *s)
}

/// Sprites, palette, and map are images; SFX/music are index/audio data
/// with no meaningful visual form.
fn supports_png(kind: SectionKind) -> bool {
    matches!(
        kind,
        SectionKind::SpriteSheet | SectionKind::Palette | SectionKind::Map
    )
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
    /// Manifest format version. Absent on any `caiven.toml` written before
    /// this field existed — those default to `CURRENT_MANIFEST_VERSION`
    /// (the only version that has ever existed), not left unvalidated.
    #[serde(default = "default_manifest_version")]
    version: u16,
}

fn default_entry() -> String {
    DEFAULT_ENTRY.to_string()
}

fn default_manifest_version() -> u16 {
    CURRENT_MANIFEST_VERSION
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

fn parse_manifest(dir: &Path) -> Result<CaivenToml, CartError> {
    let manifest_path = dir.join(MANIFEST_FILE);
    let manifest_text = std::fs::read_to_string(&manifest_path)?;
    let manifest: CaivenToml = toml::from_str(&manifest_text)?;
    let version = manifest.cart.version;
    if !(MIN_SUPPORTED_MANIFEST_VERSION..=CURRENT_MANIFEST_VERSION).contains(&version) {
        return Err(CartError::UnsupportedManifestVersion {
            found: version,
            min_supported: MIN_SUPPORTED_MANIFEST_VERSION,
            max_supported: CURRENT_MANIFEST_VERSION,
        });
    }
    Ok(manifest)
}

/// Resolves a project's entry file and its sibling `.lua` module paths
/// (both absolute, entry first), without reading or bundling their
/// contents. Used by Caiven Studio to load each file into its own editable
/// buffer, unlike [`load_project`] whose `LuaSource` section is already the
/// bundled compile output.
pub fn project_lua_files(path: &Path) -> Result<(PathBuf, Vec<PathBuf>), CartError> {
    let dir = resolve_dir(path);
    let manifest = parse_manifest(&dir)?;
    let entry_rel = PathBuf::from(&manifest.cart.entry);
    let entry_path = dir.join(&entry_rel);
    Ok((entry_path, list_lua_files(&dir, &entry_rel)))
}

/// Loads a project directory (or its `caiven.toml`) into the same in-memory
/// `Cart` shape the binary `.cav` loader produces.
pub fn load_project(path: &Path) -> Result<Cart, CartError> {
    let dir = resolve_dir(path);
    let manifest = parse_manifest(&dir)?;

    let header = CartHeader {
        title: manifest.cart.title,
        author: manifest.cart.author,
        entry_point: manifest.cart.entry_point,
        flags: manifest.cart.flags,
    };

    let entry_rel = PathBuf::from(&manifest.cart.entry);
    let entry_path = dir.join(&entry_rel);
    let entry_src = std::fs::read_to_string(&entry_path)
        .map_err(|_| CartError::MissingEntry(entry_path.display().to_string()))?;

    let mut modules = Vec::new();
    for module_path in list_lua_files(&dir, &entry_rel) {
        let key = module_key(&dir, &module_path);
        let src = std::fs::read_to_string(&module_path)?;
        modules.push((key, src));
    }
    let lua = bundle_lua(&entry_src, &modules);

    let mut sections = vec![CartSection {
        kind: SectionKind::LuaSource,
        data: lua.into_bytes(),
    }];

    let mut palette: Vec<u8> = Vec::new();
    for (kind, stem) in SECTION_STEMS {
        let png_path = dir.join(format!("{stem}.png"));
        let hex_path = dir.join(format!("{stem}.hex"));

        let data = if supports_png(kind) && png_path.is_file() {
            let bytes = std::fs::read(&png_path)?;
            decode_png_section(kind, &bytes, &palette).map_err(|message| CartError::BadPng {
                file: format!("{stem}.png"),
                message,
            })?
        } else if hex_path.is_file() {
            let text = std::fs::read_to_string(&hex_path)?;
            decode_hex_block(&text).map_err(|message| CartError::BadHex {
                file: format!("{stem}.hex"),
                message,
            })?
        } else {
            continue;
        };

        if kind == SectionKind::Palette {
            palette = data.clone();
        }
        if !data.is_empty() {
            sections.push(CartSection { kind, data });
        }
    }

    for (kind, base_kind, stem) in BANK_KINDS {
        for id in 1..=u8::MAX {
            let png_path = dir.join(format!("{stem}_{id}.png"));
            let hex_path = dir.join(format!("{stem}_{id}.hex"));
            let data = if supports_png(base_kind) && png_path.is_file() {
                let bytes = std::fs::read(&png_path)?;
                decode_png_section(base_kind, &bytes, &palette).map_err(|message| {
                    CartError::BadPng {
                        file: format!("{stem}_{id}.png"),
                        message,
                    }
                })?
            } else if hex_path.is_file() {
                let text = std::fs::read_to_string(&hex_path)?;
                decode_hex_block(&text).map_err(|message| CartError::BadHex {
                    file: format!("{stem}_{id}.hex"),
                    message,
                })?
            } else {
                continue;
            };
            sections.push(CartSection {
                kind,
                data: encode_asset_bank(id, &data),
            });
        }
    }

    let types_path = dir.join(COLLISION_TYPES_FILE);
    if types_path.is_file() {
        let text = std::fs::read_to_string(&types_path)?;
        let dtos: Vec<CollisionTypeDto> =
            serde_json::from_str(&text).map_err(|e| CartError::BadJson {
                file: COLLISION_TYPES_FILE.to_string(),
                message: e.to_string(),
            })?;
        let types: Vec<caiven_core::CollisionType> = dtos.into_iter().map(Into::into).collect();
        sections.push(CartSection {
            kind: SectionKind::CollisionTypes,
            data: encode_collision_types(&types),
        });
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

/// Writes `header`, entry `lua` source, sibling `modules` (project-relative
/// path -> source, e.g. `ui/panel.lua`), and asset `sections` out as a
/// project directory at `dir`, creating it if needed. Sections with no asset
/// file mapping (`Program`, `Meta`, `LuaSource`) are ignored; `ModManifest`
/// is folded into `caiven.toml`'s `[mods].require` instead of a `.hex` file,
/// and `CollisionTypes` is written to `collision_types.json` (omitted when
/// the table is exactly the built-in types).
/// Asset sections that trim to empty have their `.hex`/`.png` file removed
/// if present, so deleting all sprites in the editor cleans up the file
/// instead of leaving zeros.
pub fn save_project(
    dir: &Path,
    header: &CartHeader,
    lua: &str,
    modules: &[(PathBuf, String)],
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
            version: CURRENT_MANIFEST_VERSION,
        },
        mods: ModsTable { require },
    };
    let manifest_text =
        toml::to_string_pretty(&manifest).map_err(|e| CartError::MissingEntry(e.to_string()))?;
    std::fs::write(dir.join(MANIFEST_FILE), manifest_text)?;
    std::fs::write(dir.join(DEFAULT_ENTRY), lua)?;

    let expected_banks: Vec<(&str, Vec<u8>)> = BANK_KINDS
        .iter()
        .map(|(kind, _, stem)| {
            let ids = sections
                .iter()
                .filter(|(k, _)| k == kind)
                .filter_map(|(_, data)| decode_asset_bank(data).map(|(id, _)| id))
                .collect();
            (*stem, ids)
        })
        .collect();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        let stale = expected_banks
            .iter()
            .any(|(stem, ids)| bank_file_id(&name, stem).is_some_and(|id| !ids.contains(&id)));
        if stale {
            std::fs::remove_file(entry.path())?;
        }
    }

    for (rel, src) in modules {
        let module_path = dir.join(rel);
        if let Some(parent) = module_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(module_path, src)?;
    }

    for (kind, data) in sections {
        if *kind == SectionKind::CollisionTypes {
            let types = decode_collision_types(data);
            let types_path = dir.join(COLLISION_TYPES_FILE);
            if types == caiven_core::builtin_collision_types() {
                let _ = std::fs::remove_file(&types_path);
            } else {
                let dtos: Vec<CollisionTypeDto> = types.iter().map(Into::into).collect();
                let json = serde_json::to_string_pretty(&dtos).map_err(|e| CartError::BadJson {
                    file: COLLISION_TYPES_FILE.to_string(),
                    message: e.to_string(),
                })?;
                std::fs::write(&types_path, json)?;
            }
            continue;
        }
        if let Some((_, base_kind, stem)) = BANK_KINDS.iter().find(|(k, _, _)| k == kind) {
            let Some((id, payload)) = decode_asset_bank(data) else {
                continue;
            };
            let png_path = dir.join(format!("{stem}_{id}.png"));
            let hex_path = dir.join(format!("{stem}_{id}.hex"));
            let write_png = supports_png(*base_kind) && (png_path.is_file() || !hex_path.is_file());
            if write_png {
                let palette = sections
                    .iter()
                    .find(|(k, _)| *k == SectionKind::Palette)
                    .map(|(_, d)| d.as_slice())
                    .unwrap_or(&[]);
                let bytes =
                    encode_png_section(*base_kind, payload, palette).map_err(|message| {
                        CartError::BadPng {
                            file: format!("{stem}_{id}.png"),
                            message,
                        }
                    })?;
                std::fs::write(&png_path, bytes)?;
                let _ = std::fs::remove_file(&hex_path);
            } else {
                std::fs::write(&hex_path, encode_hex_block(trim_trailing_zeros(payload)))?;
                let _ = std::fs::remove_file(&png_path);
            }
            continue;
        }
        let Some(stem) = stem_for(*kind) else {
            continue;
        };
        let png_path = dir.join(format!("{stem}.png"));
        let hex_path = dir.join(format!("{stem}.hex"));

        let trimmed = trim_trailing_zeros(data);
        if trimmed.is_empty() {
            let _ = std::fs::remove_file(&hex_path);
            let _ = std::fs::remove_file(&png_path);
            continue;
        }

        // Preserve whichever format is already on disk; a brand-new asset
        // (neither file present yet) defaults to PNG.
        let write_png = supports_png(*kind) && (png_path.is_file() || !hex_path.is_file());
        if write_png {
            let palette = sections
                .iter()
                .find(|(k, _)| *k == SectionKind::Palette)
                .map(|(_, d)| d.as_slice())
                .unwrap_or(&[]);
            let bytes =
                encode_png_section(*kind, data, palette).map_err(|message| CartError::BadPng {
                    file: format!("{stem}.png"),
                    message,
                })?;
            std::fs::write(&png_path, bytes)?;
            let _ = std::fs::remove_file(&hex_path);
        } else {
            std::fs::write(&hex_path, encode_hex_block(trimmed))?;
            let _ = std::fs::remove_file(&png_path);
        }
    }

    Ok(())
}

fn bank_file_id(name: &str, stem: &str) -> Option<u8> {
    let rest = name.strip_prefix(&format!("{stem}_"))?;
    let id = rest
        .strip_suffix(".png")
        .or_else(|| rest.strip_suffix(".hex"))?
        .parse::<u8>()
        .ok()?;
    (id != 0).then_some(id)
}

fn decode_png_section(kind: SectionKind, bytes: &[u8], palette: &[u8]) -> Result<Vec<u8>, String> {
    match kind {
        SectionKind::SpriteSheet => asset_png::png_to_sprites(bytes, palette),
        SectionKind::Palette => asset_png::png_to_palette(bytes),
        SectionKind::Map => asset_png::png_to_map(bytes),
        _ => Err(format!("{kind:?} has no PNG codec")),
    }
}

fn encode_png_section(kind: SectionKind, data: &[u8], palette: &[u8]) -> Result<Vec<u8>, String> {
    match kind {
        SectionKind::SpriteSheet => asset_png::sprites_to_png(data, palette),
        SectionKind::Palette => asset_png::palette_to_png(data),
        SectionKind::Map => asset_png::map_to_png(data),
        _ => Err(format!("{kind:?} has no PNG codec")),
    }
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
        // Collision is hex-only (no PNG codec) so this test exercises the
        // generic save/load plumbing independent of asset format choice —
        // PNG-vs-hex behavior gets its own tests below.
        let sections = vec![
            (SectionKind::Collision, vec![1u8, 2, 3, 0]),
            (SectionKind::ModManifest, b"rtc\ninput".to_vec()),
        ];

        save_project(dir.path(), &header, lua, &[], &sections).unwrap();
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
        // No modules: the bundle is byte-identical to the entry source (no
        // line-number shift for the common single-file case).
        assert_eq!(String::from_utf8_lossy(&lua_section.data), lua);

        let collision = cart
            .sections
            .iter()
            .find(|s| s.kind == SectionKind::Collision)
            .unwrap();
        assert_eq!(collision.data, vec![1, 2, 3]);

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
        save_project(dir.path(), &header, "-- empty\n", &[], &[]).unwrap();

        let cart = load_project(dir.path()).unwrap();
        assert!(
            cart.sections
                .iter()
                .all(|s| s.kind == SectionKind::LuaSource)
        );
    }

    #[test]
    fn collision_types_json_written_only_when_customized() {
        let dir = tempfile::tempdir().unwrap();
        let header = CartHeader::new("Blank", "");

        // Built-ins only: no file.
        save_project(
            dir.path(),
            &header,
            "-- empty\n",
            &[],
            &[(
                SectionKind::CollisionTypes,
                encode_collision_types(&caiven_core::builtin_collision_types()),
            )],
        )
        .unwrap();
        assert!(!dir.path().join(COLLISION_TYPES_FILE).is_file());
        assert!(
            load_project(dir.path())
                .unwrap()
                .sections
                .iter()
                .all(|s| s.kind != SectionKind::CollisionTypes)
        );

        // Custom type added: file present and roundtrips.
        let mut types = caiven_core::builtin_collision_types();
        types.push(caiven_core::CollisionType {
            id: 3,
            name: "water".to_string(),
            color: [0, 128, 255],
            flags: caiven_core::CollisionTypeFlags::from_bits(0),
        });
        save_project(
            dir.path(),
            &header,
            "-- empty\n",
            &[],
            &[(SectionKind::CollisionTypes, encode_collision_types(&types))],
        )
        .unwrap();
        assert!(dir.path().join(COLLISION_TYPES_FILE).is_file());

        let cart = load_project(dir.path()).unwrap();
        let section = cart
            .sections
            .iter()
            .find(|s| s.kind == SectionKind::CollisionTypes)
            .unwrap();
        assert_eq!(decode_collision_types(&section.data), types);
    }

    #[test]
    fn manifest_without_version_field_still_loads() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join(MANIFEST_FILE),
            "[cart]\ntitle = \"X\"\nentry = \"main.lua\"\n",
        )
        .unwrap();
        std::fs::write(dir.path().join(DEFAULT_ENTRY), "-- empty\n").unwrap();

        let cart = load_project(dir.path()).unwrap();
        assert_eq!(cart.header.title, "X");
    }

    #[test]
    fn manifest_with_future_version_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join(MANIFEST_FILE),
            "[cart]\ntitle = \"X\"\nentry = \"main.lua\"\nversion = 9999\n",
        )
        .unwrap();
        std::fs::write(dir.path().join(DEFAULT_ENTRY), "-- empty\n").unwrap();

        assert!(matches!(
            load_project(dir.path()),
            Err(CartError::UnsupportedManifestVersion { found: 9999, .. })
        ));
    }

    #[test]
    fn saved_manifest_roundtrips_current_version() {
        let dir = tempfile::tempdir().unwrap();
        let header = CartHeader::new("Game", "");
        save_project(dir.path(), &header, "-- code\n", &[], &[]).unwrap();

        let manifest_text = std::fs::read_to_string(dir.path().join(MANIFEST_FILE)).unwrap();
        assert!(manifest_text.contains(&format!("version = {CURRENT_MANIFEST_VERSION}")));

        // And it loads cleanly through the normal path.
        load_project(dir.path()).unwrap();
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
            &[],
            &[(SectionKind::Collision, vec![1, 2, 3])],
        )
        .unwrap();
        assert!(dir.path().join("collision.hex").is_file());

        save_project(
            dir.path(),
            &header,
            "-- code\n",
            &[],
            &[(SectionKind::Collision, vec![0, 0, 0])],
        )
        .unwrap();
        assert!(!dir.path().join("collision.hex").is_file());
    }

    #[test]
    fn new_sprite_sheet_defaults_to_png() {
        let dir = tempfile::tempdir().unwrap();
        let header = CartHeader::new("Game", "");
        save_project(
            dir.path(),
            &header,
            "-- code\n",
            &[],
            &[(SectionKind::SpriteSheet, vec![9u8; 64])],
        )
        .unwrap();
        assert!(dir.path().join("sprites.png").is_file());
        assert!(!dir.path().join("sprites.hex").is_file());
    }

    #[test]
    fn resaving_preserves_existing_hex_format_over_png_default() {
        let dir = tempfile::tempdir().unwrap();
        let header = CartHeader::new("Game", "");
        // Simulate an existing hex-authored asset (as if hand-written or
        // migrated from before PNG support) that predates any save.
        std::fs::write(dir.path().join("sprites.hex"), "0909090909090909\n").unwrap();

        save_project(
            dir.path(),
            &header,
            "-- code\n",
            &[],
            &[(SectionKind::SpriteSheet, vec![9u8; 64])],
        )
        .unwrap();

        assert!(dir.path().join("sprites.hex").is_file());
        assert!(!dir.path().join("sprites.png").is_file());
    }

    #[test]
    fn load_prefers_png_over_hex_when_both_present() {
        let dir = tempfile::tempdir().unwrap();
        let header = CartHeader::new("Game", "");
        save_project(dir.path(), &header, "-- code\n", &[], &[]).unwrap();

        let palette = vec![0u8; 48];
        let mut sheet = vec![0u8; 64];
        sheet[0] = 5;
        std::fs::write(
            dir.path().join("sprites.png"),
            asset_png::sprites_to_png(&sheet, &palette).unwrap(),
        )
        .unwrap();
        // A stale/conflicting .hex sitting alongside it must be ignored.
        std::fs::write(dir.path().join("sprites.hex"), "ff\n").unwrap();

        let cart = load_project(dir.path()).unwrap();
        let gfx = cart
            .sections
            .iter()
            .find(|s| s.kind == SectionKind::SpriteSheet)
            .unwrap();
        assert_eq!(gfx.data[0], 5);
    }

    #[test]
    fn saved_module_is_bundled_and_require_key_matches_relative_path() {
        let dir = tempfile::tempdir().unwrap();
        let header = CartHeader::new("Game", "");
        let modules = [(
            PathBuf::from("ui").join("panel.lua"),
            "return { greet = function() return 'hi' end }".to_string(),
        )];
        save_project(
            dir.path(),
            &header,
            "local p = require('ui.panel')\n",
            &modules,
            &[],
        )
        .unwrap();

        assert!(dir.path().join("ui").join("panel.lua").is_file());

        let cart = load_project(dir.path()).unwrap();
        let lua = cart
            .sections
            .iter()
            .find(|s| s.kind == SectionKind::LuaSource)
            .unwrap();
        let bundled = String::from_utf8_lossy(&lua.data);
        assert!(bundled.contains("__pre[\"ui.panel\"]"));
        assert!(bundled.contains("require('ui.panel')"));
    }

    #[test]
    fn additional_asset_banks_roundtrip_and_delete_cleanly() {
        let dir = tempfile::tempdir().unwrap();
        let header = CartHeader::new("Banks", "");
        let bank = encode_asset_bank(1, &[3, 4, 0]);
        save_project(
            dir.path(),
            &header,
            "-- code\n",
            &[],
            &[(SectionKind::SpriteBank, bank)],
        )
        .unwrap();
        assert!(dir.path().join("sprites_1.png").is_file());

        let cart = load_project(dir.path()).unwrap();
        let loaded = cart
            .sections
            .iter()
            .find(|section| section.kind == SectionKind::SpriteBank)
            .unwrap();
        let (id, pixels) = decode_asset_bank(&loaded.data).unwrap();
        assert_eq!(id, 1);
        assert_eq!(&pixels[..3], &[3, 4, 0]);

        save_project(dir.path(), &header, "-- code\n", &[], &[]).unwrap();
        assert!(!dir.path().join("sprites_1.png").exists());
    }

    #[test]
    fn palette_and_sfx_additional_banks_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let header = CartHeader::new("Banks", "");
        let palette_bank = encode_asset_bank(1, &[10, 20, 30]);
        let sfx_bank = encode_asset_bank(1, &[5, 6, 7, 8]);
        save_project(
            dir.path(),
            &header,
            "-- code\n",
            &[],
            &[
                (SectionKind::PaletteBank, palette_bank),
                (SectionKind::SfxBanks, sfx_bank),
            ],
        )
        .unwrap();
        // Palette supports PNG; SFX is hex-only.
        assert!(dir.path().join("palette_1.png").is_file());
        assert!(dir.path().join("sfx_1.hex").is_file());
        assert!(!dir.path().join("sfx_1.png").exists());

        let cart = load_project(dir.path()).unwrap();
        let palette = cart
            .sections
            .iter()
            .find(|s| s.kind == SectionKind::PaletteBank)
            .unwrap();
        let (id, rgb) = decode_asset_bank(&palette.data).unwrap();
        assert_eq!(id, 1);
        assert_eq!(&rgb[..3], &[10, 20, 30]);

        let sfx = cart
            .sections
            .iter()
            .find(|s| s.kind == SectionKind::SfxBanks)
            .unwrap();
        let (id, bytes) = decode_asset_bank(&sfx.data).unwrap();
        assert_eq!(id, 1);
        assert_eq!(&bytes[..4], &[5, 6, 7, 8]);

        save_project(dir.path(), &header, "-- code\n", &[], &[]).unwrap();
        assert!(!dir.path().join("palette_1.png").exists());
        assert!(!dir.path().join("sfx_1.hex").exists());
    }

    #[test]
    fn collision_additional_bank_roundtrips() {
        let dir = tempfile::tempdir().unwrap();
        let header = CartHeader::new("Banks", "");
        let collision_bank = encode_asset_bank(1, &[0, 1, 2, 1]);
        save_project(
            dir.path(),
            &header,
            "-- code\n",
            &[],
            &[(SectionKind::CollisionBank, collision_bank)],
        )
        .unwrap();
        // Collision is index data, not an image: hex-only, like sprite flags.
        assert!(dir.path().join("collision_1.hex").is_file());
        assert!(!dir.path().join("collision_1.png").exists());

        let cart = load_project(dir.path()).unwrap();
        let collision = cart
            .sections
            .iter()
            .find(|s| s.kind == SectionKind::CollisionBank)
            .unwrap();
        let (id, cells) = decode_asset_bank(&collision.data).unwrap();
        assert_eq!(id, 1);
        assert_eq!(&cells[..4], &[0, 1, 2, 1]);

        save_project(dir.path(), &header, "-- code\n", &[], &[]).unwrap();
        assert!(!dir.path().join("collision_1.hex").exists());
    }

    #[test]
    fn empty_additional_bank_still_persists() {
        let dir = tempfile::tempdir().unwrap();
        let header = CartHeader::new("Banks", "");
        save_project(
            dir.path(),
            &header,
            "-- code\n",
            &[],
            &[(
                SectionKind::MapBank,
                encode_asset_bank(2, &vec![0; caiven_core::memory::MAP_LEN]),
            )],
        )
        .unwrap();
        let cart = load_project(dir.path()).unwrap();
        assert!(cart.sections.iter().any(|section| {
            section.kind == SectionKind::MapBank
                && decode_asset_bank(&section.data).is_some_and(|(id, _)| id == 2)
        }));
    }
}
