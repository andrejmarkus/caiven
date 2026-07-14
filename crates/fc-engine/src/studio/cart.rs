//! Standalone cart loading for the studio: loads ROM sections into VM RAM
//! and produces a [`CartMeta`] usable with `rom_io::save`. Mirrors the
//! legacy `App::load_rom` path without depending on the old editor state.

use crate::app::rom_io::{CartMeta, SectionLayout};
use anyhow::{Context, Result};
use fc_core::memory::{
    MAP_LEN, MAP_RAM_BASE, MUSIC_BANK_LEN, MUSIC_RAM_BASE, PALETTE_RAM_BASE, SFX_BANK_LEN,
    SFX_RAM_BASE, SPRITE_SHEET_RAM_BASE,
};
use fc_rom::SectionKind;
use fc_vm::Vm;
use std::path::Path;

pub fn load_rom(vm: &mut Vm, path: &Path) -> Result<CartMeta> {
    let rom = fc_rom::load(path)
        .with_context(|| format!("failed to load ROM from {}", path.display()))?;

    for section in &rom.sections {
        if section.kind == SectionKind::ModManifest {
            let manifest = String::from_utf8_lossy(&section.data);
            let registered = vm.registered_peripheral_names();
            for required in manifest.lines().map(str::trim).filter(|s| !s.is_empty()) {
                if !registered.contains(&required) {
                    anyhow::bail!("ROM requires mod '{}' but it is not loaded", required);
                }
            }
        }
    }

    vm.load_rom(rom.program.clone());

    let mut sections: Vec<SectionLayout> = Vec::new();
    for section in &rom.sections {
        let ram_base = match section.kind {
            SectionKind::SpriteSheet => SPRITE_SHEET_RAM_BASE,
            SectionKind::Map => MAP_RAM_BASE,
            SectionKind::Palette => PALETTE_RAM_BASE,
            SectionKind::SfxBank => SFX_RAM_BASE,
            SectionKind::MusicBank => MUSIC_RAM_BASE,
            _ => continue,
        };
        vm.load_section_to_ram(ram_base, &section.data);
        if section.kind == SectionKind::Palette {
            vm.set_palette_from_bytes(&section.data);
        }
        sections.push(SectionLayout {
            kind: section.kind,
            ram_base,
            len: section.data.len(),
        });
    }

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
        });
    }
    for (kind, ram_base, len) in [
        (SectionKind::Map, MAP_RAM_BASE, MAP_LEN),
        (SectionKind::SfxBank, SFX_RAM_BASE, SFX_BANK_LEN),
        (SectionKind::MusicBank, MUSIC_RAM_BASE, MUSIC_BANK_LEN),
    ] {
        if !sections.iter().any(|s| s.kind == kind) {
            sections.push(SectionLayout {
                kind,
                ram_base,
                len,
            });
        }
    }

    Ok(CartMeta {
        path: path.to_path_buf(),
        header: rom.header,
        program: rom.program,
        sections,
    })
}

/// Compiles `.fc` source and loads it into the VM with its source map.
pub fn load_fc_source(vm: &mut Vm, path: &Path, source: &str) -> Result<()> {
    let out = fc_lang::compile(source).map_err(|e| {
        anyhow::anyhow!("compile error in {}:\n{}", path.display(), e.render(source))
    })?;
    vm.load_rom_with_source_map(out.program, out.source_map);
    vm.set_fc_source(source);
    Ok(())
}
