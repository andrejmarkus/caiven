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
}

/// Reads each tracked RAM section from the VM and writes them back to the cart file.
/// Only sections that were copied into RAM (e.g. SpriteSheet) are round-tripped.
/// Other section kinds are not currently written back.
pub(crate) fn save(vm: &Vm, meta: &CartMeta) -> Result<()> {
    let extra: Vec<(SectionKind, Vec<u8>)> = meta
        .sections
        .iter()
        .map(|s| {
            let bytes: Vec<u8> = (0..s.len).map(|i| vm.peek_memory(s.ram_base + i)).collect();
            (s.kind, bytes)
        })
        .collect();

    fc_rom::write(&meta.path, &meta.header, &meta.program, &extra)
        .with_context(|| format!("failed to write cart to {}", meta.path.display()))
}

impl App {
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

        self.core.vm.load_rom(rom.program.clone());
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

        self.cart_meta = Some(CartMeta {
            path: path.to_path_buf(),
            header: rom.header,
            program: rom.program,
            sections,
        });

        info!("ROM loaded from {}", path.display());
        Ok(())
    }

    pub(super) fn load_source(&mut self, path: &Path) -> Result<()> {
        let source = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read source {}", path.display()))?;
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if ext == "fc" {
            let (code, sections) =
                fc_rom::text::split_source(&source).map_err(anyhow::Error::msg)?;
            let out = fc_lang::compile(&code).map_err(|e| {
                anyhow::anyhow!("compile error in {}:\n{}", path.display(), e.render(&code))
            })?;
            self.core
                .vm
                .load_rom_with_source_map(out.program, out.source_map);
            self.core.vm.set_fc_source(&code);
            for (kind, data) in &sections {
                if let Some(ram_base) = crate::studio::cart::section_ram_base(*kind) {
                    self.core.vm.load_section_to_ram(ram_base, data);
                    if *kind == SectionKind::Palette {
                        self.core.vm.set_palette_from_bytes(data);
                    }
                }
            }
            info!("fc-lang compiled from {}", path.display());
        } else {
            let out = fc_asm::assemble_with_sections(&source)
                .with_context(|| format!("failed to assemble {}", path.display()))?;
            self.core
                .vm
                .load_rom_with_source_map(out.program, out.source_map);
            for (wire_id, data) in &out.extra_sections {
                if *wire_id == fc_rom::SectionKind::SpriteSheet.to_u16() {
                    self.core
                        .vm
                        .load_section_to_ram(SPRITE_SHEET_RAM_BASE, data);
                }
            }
            info!("source assembled from {}", path.display());
        }
        self.debugger.set_fcdbg_path(path.with_extension("fcdbg"));
        Ok(())
    }

    pub(super) fn watch_source(&mut self, path: PathBuf) -> Result<()> {
        self.load_source(&path)?;
        let mtime = path.metadata().ok().and_then(|m| m.modified().ok());
        self.hot_reload.watch(path, mtime);
        Ok(())
    }

    pub(super) fn poll_hot_reload(&mut self) {
        if let Some(path) = self.hot_reload.poll() {
            info!("hot-reload: {}", path.display());
            if let Err(e) = self.load_source(&path) {
                warn!("hot-reload failed: {e}");
            }
        }
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
