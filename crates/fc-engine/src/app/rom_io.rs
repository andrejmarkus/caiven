//! ROM/cart loading, source (re)compilation and cart saving for [`App`].

use super::App;
use anyhow::{Context, Result};
use fc_core::memory::{
    MAP_RAM_BASE, MUSIC_BANK_LEN, MUSIC_RAM_BASE, PALETTE_RAM_BASE, SFX_BANK_LEN, SFX_RAM_BASE,
    SPRITE_SHEET_RAM_BASE,
};
use fc_rom::{RomHeader, SectionKind};
use fc_vm::Vm;
use log::{error, info, warn};
use std::path::{Path, PathBuf};

pub struct SectionLayout {
    pub kind: SectionKind,
    pub ram_base: usize,
    pub len: usize,
}

pub struct CartMeta {
    pub path: PathBuf,
    pub header: RomHeader,
    pub program: Vec<u8>,
    pub sections: Vec<SectionLayout>,
    pub lua_source: Option<String>,
}

/// Reads each tracked RAM section from the VM and writes them back to the cart file.
/// Only sections that were copied into RAM (e.g. SpriteSheet) are round-tripped.
/// Other section kinds are not currently written back.
pub(crate) fn save(vm: &Vm, meta: &CartMeta) -> Result<()> {
    let mut extra: Vec<(SectionKind, Vec<u8>)> = meta
        .sections
        .iter()
        .map(|s| {
            let bytes: Vec<u8> = (0..s.len).map(|i| vm.peek_memory(s.ram_base + i)).collect();
            (s.kind, bytes)
        })
        .collect();

    let program: &[u8] = match &meta.lua_source {
        Some(src) => {
            extra.push((SectionKind::LuaSource, src.clone().into_bytes()));
            &[]
        }
        None => &meta.program,
    };

    fc_rom::write(&meta.path, &meta.header, program, &extra)
        .with_context(|| format!("failed to write cart to {}", meta.path.display()))
}

impl App {
    /// Loads a bare `.lua` file straight into the VM's embedded Lua path.
    /// No asset sections and no ROM packaging — for that, build a `.rom`
    /// with a `LuaSource` section instead (see `load_rom` below).
    pub(super) fn load_lua(&mut self, path: &Path) -> Result<()> {
        let src = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read Lua source from {}", path.display()))?;
        self.core
            .vm
            .load_lua_source(&src, &self.core.input, &self.core.font)
            .map_err(|e| anyhow::anyhow!("{e}"))
            .with_context(|| format!("failed to load Lua script {}", path.display()))
    }

    pub(super) fn load_rom(&mut self, path: &Path) -> Result<()> {
        let rom = fc_rom::load(path)
            .with_context(|| format!("failed to load ROM from {}", path.display()))?;

        for section in &rom.sections {
            if section.kind == SectionKind::ModManifest {
                let manifest = String::from_utf8_lossy(&section.data);
                let registered = self.core.vm.registered_peripheral_names();
                for required in manifest.lines().map(str::trim).filter(|s| !s.is_empty()) {
                    if !registered.contains(&required) {
                        anyhow::bail!("ROM requires mod '{}' but it is not loaded", required);
                    }
                }
            }
        }

        let lua_source = rom
            .sections
            .iter()
            .find(|s| s.kind == SectionKind::LuaSource)
            .map(|s| String::from_utf8_lossy(&s.data).into_owned());

        self.debugger.set_fcdbg_path(path.with_extension("fcdbg"));

        let mut sections: Vec<SectionLayout> = Vec::new();
        for section in &rom.sections {
            match section.kind {
                SectionKind::SpriteSheet => {
                    self.core
                        .vm
                        .load_section_to_ram(SPRITE_SHEET_RAM_BASE, &section.data);
                    sections.push(SectionLayout {
                        kind: SectionKind::SpriteSheet,
                        ram_base: SPRITE_SHEET_RAM_BASE,
                        len: section.data.len(),
                    });
                    info!(
                        "SpriteSheet loaded to RAM at 0x{:04X} ({} bytes)",
                        SPRITE_SHEET_RAM_BASE,
                        section.data.len()
                    );
                }
                SectionKind::Map => {
                    self.core
                        .vm
                        .load_section_to_ram(MAP_RAM_BASE, &section.data);
                    sections.push(SectionLayout {
                        kind: SectionKind::Map,
                        ram_base: MAP_RAM_BASE,
                        len: section.data.len(),
                    });
                    info!(
                        "Map loaded to RAM at 0x{:04X} ({} bytes)",
                        MAP_RAM_BASE,
                        section.data.len()
                    );
                }
                SectionKind::Palette => {
                    self.core
                        .vm
                        .load_section_to_ram(PALETTE_RAM_BASE, &section.data);
                    self.core.vm.set_palette_from_bytes(&section.data);
                    sections.push(SectionLayout {
                        kind: SectionKind::Palette,
                        ram_base: PALETTE_RAM_BASE,
                        len: section.data.len(),
                    });
                    info!(
                        "Palette loaded to RAM at 0x{:04X} ({} bytes)",
                        PALETTE_RAM_BASE,
                        section.data.len()
                    );
                }
                SectionKind::SfxBank => {
                    self.core
                        .vm
                        .load_section_to_ram(SFX_RAM_BASE, &section.data);
                    sections.push(SectionLayout {
                        kind: SectionKind::SfxBank,
                        ram_base: SFX_RAM_BASE,
                        len: section.data.len(),
                    });
                    info!(
                        "SfxBank loaded to RAM at 0x{:04X} ({} bytes)",
                        SFX_RAM_BASE,
                        section.data.len()
                    );
                }
                SectionKind::MusicBank => {
                    self.core
                        .vm
                        .load_section_to_ram(MUSIC_RAM_BASE, &section.data);
                    sections.push(SectionLayout {
                        kind: SectionKind::MusicBank,
                        ram_base: MUSIC_RAM_BASE,
                        len: section.data.len(),
                    });
                    info!(
                        "MusicBank loaded to RAM at 0x{:04X} ({} bytes)",
                        MUSIC_RAM_BASE,
                        section.data.len()
                    );
                }
                _ => {}
            }
        }

        // If no Palette section was in the ROM, sync VM's default palette to RAM
        if !sections.iter().any(|s| s.kind == SectionKind::Palette) {
            let palette_bytes: Vec<u8> = self
                .core
                .vm
                .get_palette()
                .iter()
                .flat_map(|c| [c.get_r(), c.get_g(), c.get_b()])
                .collect();
            self.core
                .vm
                .load_section_to_ram(PALETTE_RAM_BASE, &palette_bytes);
            sections.push(SectionLayout {
                kind: SectionKind::Palette,
                ram_base: PALETTE_RAM_BASE,
                len: palette_bytes.len(),
            });
        }

        // If no Map section was in the ROM, register it so Ctrl+S persists it
        if !sections.iter().any(|s| s.kind == SectionKind::Map) {
            sections.push(SectionLayout {
                kind: SectionKind::Map,
                ram_base: MAP_RAM_BASE,
                len: fc_core::memory::MAP_LEN,
            });
        }

        // If no SfxBank section, register for Ctrl+S persistence
        if !sections.iter().any(|s| s.kind == SectionKind::SfxBank) {
            sections.push(SectionLayout {
                kind: SectionKind::SfxBank,
                ram_base: SFX_RAM_BASE,
                len: SFX_BANK_LEN,
            });
        }

        // If no MusicBank section, register for Ctrl+S persistence
        if !sections.iter().any(|s| s.kind == SectionKind::MusicBank) {
            sections.push(SectionLayout {
                kind: SectionKind::MusicBank,
                ram_base: MUSIC_RAM_BASE,
                len: MUSIC_BANK_LEN,
            });
        }

        // Loaded after asset RAM is populated, since it runs `_init()`
        // immediately.
        let src = lua_source
            .as_deref()
            .context("ROM has no Lua source section (bytecode carts are no longer supported)")?;
        self.core
            .vm
            .load_lua_source(src, &self.core.input, &self.core.font)
            .map_err(|e| anyhow::anyhow!("{e}"))
            .with_context(|| format!("failed to load Lua ROM {}", path.display()))?;

        self.cart_meta = Some(CartMeta {
            path: path.to_path_buf(),
            header: rom.header,
            program: rom.program,
            sections,
            lua_source,
        });

        info!("ROM loaded from {}", path.display());
        Ok(())
    }

    pub(super) fn save_cart(&mut self) {
        let Some(meta) = &self.cart_meta else {
            warn!("Ctrl+S: no cart loaded");
            return;
        };
        match save(&self.core.vm, meta) {
            Ok(()) => info!("cart saved to {}", meta.path.display()),
            Err(e) => error!("cart save failed: {e}"),
        }
    }
}
