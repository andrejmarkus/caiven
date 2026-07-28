//! Standalone cart loading for the studio: loads cart sections into VM RAM
//! and produces a [`CartMeta`] usable with `cart_io::save`.

use super::SourceFile;
use crate::app::cart_io::{CartMeta, SectionLayout};
use anyhow::{Context, Result};
use caiven_cart::SectionKind;
use caiven_core::memory::{
    MAP_LEN, MAP_RAM_BASE, MUSIC_BANK_LEN, MUSIC_RAM_BASE, PALETTE_RAM_BASE, SFX_BANK_LEN,
    SFX_RAM_BASE, SPRITE_FLAGS_LEN, SPRITE_FLAGS_RAM_BASE, SPRITE_SHEET_LEN, SPRITE_SHEET_RAM_BASE,
};
use caiven_vm::input::Input;
use caiven_vm::rendering::font::Font;
use caiven_vm::{AssetBankKind, Vm};
use std::path::{Path, PathBuf};

fn stored_cart_path(path: &Path) -> PathBuf {
    if caiven_cart::is_project(path) && path.is_file() {
        path.parent().map(Path::to_path_buf).unwrap_or_default()
    } else {
        path.to_path_buf()
    }
}

pub fn load_cart(vm: &mut Vm, path: &Path, input: &Input, font: &Font) -> Result<CartMeta> {
    let stored_path = stored_cart_path(path);
    let cart = caiven_cart::open(path)
        .with_context(|| format!("failed to load cart from {}", path.display()))?;

    for section in &cart.sections {
        if section.kind == SectionKind::ModManifest {
            let manifest = String::from_utf8_lossy(&section.data);
            let registered = vm.registered_peripheral_names();
            for required in manifest.lines().map(str::trim).filter(|s| !s.is_empty()) {
                if !registered.contains(&required) {
                    anyhow::bail!("cart requires mod '{}' but it is not loaded", required);
                }
            }
        }
    }

    let lua_source = vm.load_cart_sections(&cart.sections);

    let mut sections: Vec<SectionLayout> = Vec::new();
    for section in &cart.sections {
        if let Some(ram_base) = section_ram_base(section.kind) {
            sections.push(SectionLayout {
                kind: section.kind,
                ram_base,
                len: section.data.len(),
                preserved_data: None,
            });
        } else if !matches!(section.kind, SectionKind::Program | SectionKind::LuaSource) {
            // Manifest, metadata and custom sections are not RAM-backed, but
            // must survive Ctrl+S and binary export unchanged.
            sections.push(SectionLayout {
                kind: section.kind,
                ram_base: 0,
                len: section.data.len(),
                preserved_data: Some(section.data.clone()),
            });
        }
    }

    // Asset RAM must be in place before the Lua load, since it runs
    // `_init()` immediately.
    let src = lua_source
        .as_deref()
        .context("cart has no Lua source section (bytecode carts are no longer supported)")?;
    vm.load_lua_source(src, input, font)
        .map_err(|e| anyhow::anyhow!("{e}"))
        .with_context(|| format!("failed to load Lua cart {}", path.display()))?;

    if !sections.iter().any(|s| s.kind == SectionKind::Palette) {
        let palette_bytes: Vec<u8> = vm
            .get_palette()
            .iter()
            .flat_map(|c| [c.get_r(), c.get_g(), c.get_b()])
            .collect();
        vm.load_section_to_ram(PALETTE_RAM_BASE, &palette_bytes);
        sections.push(SectionLayout {
            kind: SectionKind::Palette,
            ram_base: PALETTE_RAM_BASE,
            len: palette_bytes.len(),
            preserved_data: None,
        });
    }
    for (kind, ram_base, len) in [
        (
            SectionKind::SpriteSheet,
            SPRITE_SHEET_RAM_BASE,
            SPRITE_SHEET_LEN,
        ),
        (SectionKind::Map, MAP_RAM_BASE, MAP_LEN),
        (
            SectionKind::SpriteFlags,
            SPRITE_FLAGS_RAM_BASE,
            SPRITE_FLAGS_LEN,
        ),
        (SectionKind::SfxBank, SFX_RAM_BASE, SFX_BANK_LEN),
        (SectionKind::MusicBank, MUSIC_RAM_BASE, MUSIC_BANK_LEN),
    ] {
        if !sections.iter().any(|s| s.kind == kind) {
            sections.push(SectionLayout {
                kind,
                ram_base,
                len,
                preserved_data: None,
            });
        }
    }

    Ok(CartMeta {
        path: stored_path,
        header: cart.header,
        program: cart.program,
        sections,
        lua_source,
    })
}

/// Reads a project's entry `.lua` file and its sibling modules into
/// separate editable buffers (entry first) — unlike `load_cart`'s
/// `CartMeta.lua_source`, which is the already-bundled compile output, this
/// gives Studio's Code tab one buffer per on-disk file so `require()`d
/// modules are independently editable and saveable.
pub fn load_project_sources(path: &Path) -> Result<Vec<SourceFile>> {
    let (entry_path, module_paths) = caiven_cart::project_lua_files(path)
        .with_context(|| format!("failed to read project modules from {}", path.display()))?;

    let mut sources = Vec::with_capacity(1 + module_paths.len());
    let entry_text = std::fs::read_to_string(&entry_path)
        .with_context(|| format!("failed to read {}", entry_path.display()))?;
    sources.push(SourceFile {
        path: entry_path,
        text: entry_text,
        dirty: false,
    });
    for module_path in module_paths {
        let text = std::fs::read_to_string(&module_path)
            .with_context(|| format!("failed to read {}", module_path.display()))?;
        sources.push(SourceFile {
            path: module_path,
            text,
            dirty: false,
        });
    }
    Ok(sources)
}

/// Compile failure with the 1-based source line (when known) so the code
/// editor can highlight and jump to it.
pub struct CompileError {
    pub source: Option<String>,
    pub line: Option<usize>,
    pub message: String,
}

/// Compiles `sources[0]` (the entry buffer) plus any sibling module buffers,
/// bundled together exactly like the project loader does from disk (see
/// `caiven_cart::bundle_lua`), and (re)starts the VM. Embedded asset blocks
/// (`__gfx__` etc.) in the entry buffer are split off and applied to RAM
/// first, since loading Lua source runs `_init()` immediately. `dir` is the
/// project directory `sources` was loaded from — pass `None` for a
/// single-buffer `.cav`-sourced cart, which has no sibling modules to
/// bundle.
pub fn compile_sources_into_vm(
    vm: &mut Vm,
    dir: Option<&Path>,
    sources: &[SourceFile],
    input: &Input,
    font: &Font,
) -> std::result::Result<(), CompileError> {
    let Some(entry) = sources.first() else {
        return Err(CompileError {
            source: None,
            line: None,
            message: "no source loaded".to_string(),
        });
    };
    let (code, sections) =
        caiven_cart::text::split_source(&entry.text).map_err(|message| CompileError {
            source: None,
            line: None,
            message,
        })?;
    apply_sections(vm, &sections);

    let modules: Vec<(String, String)> = match dir {
        Some(dir) => sources[1..]
            .iter()
            .map(|s| (caiven_cart::module_key(dir, &s.path), s.text.clone()))
            .collect(),
        None => Vec::new(),
    };
    let bundled = caiven_cart::bundle_lua(&code, &modules);

    vm.load_lua_source(&bundled, input, font)
        .map_err(|error| caiven_vm::describe_lua_error_location(&error))
        .map_err(|(location, message)| CompileError {
            source: location.as_ref().map(|location| location.source.clone()),
            line: location.map(|location| location.line),
            message,
        })
}

/// Unpacks a binary `.cav` cart into an editable project directory at
/// `out`. Module structure isn't preserved across the binary format (it
/// only ever holds one flattened Lua source), so the result is always a
/// single `main.lua` plus asset files.
/// A unique path under the OS temp dir for packing a project into a
/// throwaway `.cav` (e.g. before uploading to the port) without touching
/// the project's own save location.
pub(crate) fn temp_cav_path() -> std::path::PathBuf {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    std::env::temp_dir().join(format!("caiven-pack-{}-{unique}.cav", std::process::id()))
}

pub(crate) fn unpack_cart(cart: &Path, out: &Path) -> Result<()> {
    ensure_empty_unpack_destination(out)?;
    let loaded = caiven_cart::load(cart)
        .with_context(|| format!("failed to load cart from {}", cart.display()))?;
    let lua = loaded
        .sections
        .iter()
        .find(|s| s.kind == SectionKind::LuaSource)
        .map(|s| String::from_utf8_lossy(&s.data).into_owned())
        .context("cart has no Lua source section (bytecode carts are no longer supported)")?;
    let extra: Vec<(SectionKind, Vec<u8>)> = loaded
        .sections
        .into_iter()
        .map(|s| (s.kind, s.data))
        .collect();
    caiven_cart::save_project(out, &loaded.header, &lua, &[], &extra)
        .with_context(|| format!("failed to write project to {}", out.display()))?;
    Ok(())
}

/// Unpacking creates a complete project, so only a fresh or empty directory
/// is safe. This guard is shared by Studio and the CLI entry point.
fn ensure_empty_unpack_destination(out: &Path) -> Result<()> {
    match std::fs::read_dir(out) {
        Ok(mut entries) => {
            if entries.next().transpose()?.is_some() {
                anyhow::bail!(
                    "unpack destination must be a new or empty directory: {}",
                    out.display()
                );
            }
            Ok(())
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error)
            .with_context(|| format!("failed to inspect unpack destination {}", out.display())),
    }
}

pub fn section_ram_base(kind: SectionKind) -> Option<usize> {
    Some(match kind {
        SectionKind::SpriteSheet => SPRITE_SHEET_RAM_BASE,
        SectionKind::Map => MAP_RAM_BASE,
        SectionKind::SpriteFlags => SPRITE_FLAGS_RAM_BASE,
        SectionKind::Palette => PALETTE_RAM_BASE,
        SectionKind::SfxBank => SFX_RAM_BASE,
        SectionKind::MusicBank => MUSIC_RAM_BASE,
        _ => return None,
    })
}

pub fn apply_sections(vm: &mut Vm, sections: &[(SectionKind, Vec<u8>)]) {
    for (kind, data) in sections {
        match kind {
            SectionKind::SpriteSheet => {
                vm.replace_asset_bank(AssetBankKind::Sprites, 0, data);
                continue;
            }
            SectionKind::Map => {
                vm.replace_asset_bank(AssetBankKind::Map, 0, data);
                continue;
            }
            _ => {}
        }
        let Some(ram_base) = section_ram_base(*kind) else {
            continue;
        };
        vm.load_section_to_ram(ram_base, data);
        if *kind == SectionKind::Palette {
            vm.set_palette_from_bytes(data);
        }
    }
}

/// The full set of asset regions a cart always round-trips, used to seed a
/// brand-new blank cart (a loaded cart instead builds this per-section from
/// what's actually present, see `load_cart`).
pub fn default_section_layout() -> Vec<SectionLayout> {
    [
        (
            SectionKind::SpriteSheet,
            SPRITE_SHEET_RAM_BASE,
            SPRITE_SHEET_LEN,
        ),
        (SectionKind::Map, MAP_RAM_BASE, MAP_LEN),
        (
            SectionKind::SpriteFlags,
            SPRITE_FLAGS_RAM_BASE,
            SPRITE_FLAGS_LEN,
        ),
        (SectionKind::Palette, PALETTE_RAM_BASE, 16 * 3),
        (SectionKind::SfxBank, SFX_RAM_BASE, SFX_BANK_LEN),
        (SectionKind::MusicBank, MUSIC_RAM_BASE, MUSIC_BANK_LEN),
    ]
    .into_iter()
    .map(|(kind, ram_base, len)| SectionLayout {
        kind,
        ram_base,
        len,
        preserved_data: None,
    })
    .collect()
}

#[cfg(test)]
mod tests {
    use super::{stored_cart_path, unpack_cart};
    use caiven_cart::{CartHeader, SectionKind};

    #[test]
    fn manifest_path_is_stored_as_project_directory() {
        let root = std::env::temp_dir().join(format!(
            "caiven-manifest-path-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&root).unwrap();
        let manifest = root.join("caiven.toml");
        std::fs::write(&manifest, "[cart]\ntitle = \"Test\"\n").unwrap();

        assert_eq!(stored_cart_path(&manifest), root);
        std::fs::remove_dir_all(manifest.parent().unwrap()).unwrap();
    }

    #[test]
    fn nonempty_unpack_destination_is_rejected() {
        let unique = format!(
            "caiven-unpack-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let root = std::env::temp_dir().join(unique);
        let destination = root.join("existing-project");
        let cart = root.join("game.cav");
        std::fs::create_dir(&root).unwrap();
        std::fs::create_dir(&destination).unwrap();
        std::fs::write(destination.join("existing-project-file"), "keep me").unwrap();
        caiven_cart::write(
            &cart,
            &CartHeader::new("Test", ""),
            &[],
            &[(SectionKind::LuaSource, b"-- test\n".to_vec())],
        )
        .unwrap();

        assert!(unpack_cart(&cart, &destination).is_err());
        assert_eq!(
            std::fs::read_to_string(destination.join("existing-project-file")).unwrap(),
            "keep me"
        );

        std::fs::remove_dir_all(root).unwrap();
    }
}
