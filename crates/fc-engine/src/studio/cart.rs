//! Standalone cart loading for the studio: loads ROM sections into VM RAM
//! and produces a [`CartMeta`] usable with `rom_io::save`. Mirrors the
//! legacy `App::load_rom` path without depending on the old editor state.

use crate::app::rom_io::{CartMeta, SectionLayout};
use anyhow::{Context, Result};
use fc_core::memory::{
    MAP_LEN, MAP_RAM_BASE, MUSIC_BANK_LEN, MUSIC_RAM_BASE, PALETTE_RAM_BASE, SFX_BANK_LEN,
    SFX_RAM_BASE, SPRITE_FLAGS_LEN, SPRITE_FLAGS_RAM_BASE, SPRITE_SHEET_LEN, SPRITE_SHEET_RAM_BASE,
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
        let Some(ram_base) = section_ram_base(section.kind) else {
            continue;
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

/// Compile failure with the 1-based source line (when known) so the code
/// editor can highlight and jump to it.
pub struct CompileError {
    pub line: Option<usize>,
    pub message: String,
}

/// Compiles `.fc` source and loads it into the VM with its source map.
/// Embedded asset blocks (`__gfx__` etc.) are split off and loaded into RAM;
/// loading a program does not clear RAM, so assets painted in the editors
/// survive recompiles.
pub fn compile_into_vm(vm: &mut Vm, source: &str) -> std::result::Result<(), CompileError> {
    let (code, sections) = fc_rom::text::split_source(source).map_err(|message| CompileError {
        line: None,
        message,
    })?;
    match fc_lang::compile(&code) {
        Ok(out) => {
            vm.load_rom_with_source_map(out.program, out.source_map);
            vm.set_fc_source(&code);
            apply_sections(vm, &sections);
            Ok(())
        }
        Err(e) => Err(CompileError {
            line: e.line(),
            message: e.render(&code),
        }),
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
        let Some(ram_base) = section_ram_base(*kind) else {
            continue;
        };
        vm.load_section_to_ram(ram_base, data);
        if *kind == SectionKind::Palette {
            vm.set_palette_from_bytes(data);
        }
    }
}

/// Mirrors the VM's active palette into palette RAM so editors and cart
/// saving always see full 16×RGB bytes there.
pub fn sync_palette_to_ram(vm: &mut Vm) {
    let bytes: Vec<u8> = vm
        .get_palette()
        .iter()
        .flat_map(|c| [c.get_r(), c.get_g(), c.get_b()])
        .collect();
    vm.load_section_to_ram(PALETTE_RAM_BASE, &bytes);
}

/// Reads every asset region back out of VM RAM for embedding into `.fc`
/// text on save. Empty (all-zero) regions are dropped by `join_source`.
pub fn collect_ram_sections(vm: &Vm) -> Vec<(SectionKind, Vec<u8>)> {
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
    .map(|(kind, base, len)| {
        let bytes: Vec<u8> = (0..len).map(|i| vm.peek_memory(base + i)).collect();
        (kind, bytes)
    })
    .collect()
}
