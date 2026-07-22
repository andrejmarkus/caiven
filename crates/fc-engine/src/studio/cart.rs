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
use fc_vm::input::Input;
use fc_vm::rendering::font::Font;
use std::path::Path;

pub fn load_rom(vm: &mut Vm, path: &Path, input: &Input, font: &Font) -> Result<CartMeta> {
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

    let lua_source = rom
        .sections
        .iter()
        .find(|s| s.kind == SectionKind::LuaSource)
        .map(|s| String::from_utf8_lossy(&s.data).into_owned());

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

    // Asset RAM must be in place before the Lua load, since it runs
    // `_init()` immediately.
    let src = lua_source
        .as_deref()
        .context("ROM has no Lua source section (bytecode carts are no longer supported)")?;
    vm.load_lua_source(src, input, font)
        .map_err(|e| anyhow::anyhow!("{e}"))
        .with_context(|| format!("failed to load Lua ROM {}", path.display()))?;

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
        lua_source,
    })
}

/// Compile failure with the 1-based source line (when known) so the code
/// editor can highlight and jump to it.
pub struct CompileError {
    pub line: Option<usize>,
    pub message: String,
}

/// Loads `.lua` source into the VM. Embedded asset blocks (`__gfx__` etc.)
/// are split off and applied to RAM first: unlike the old bytecode path,
/// loading Lua source runs `_init()` immediately, so map/sprite/etc. RAM
/// needs to already be in place.
pub fn compile_lua_into_vm(
    vm: &mut Vm,
    source: &str,
    input: &Input,
    font: &Font,
) -> std::result::Result<(), CompileError> {
    let (code, sections) = fc_rom::text::split_source(source).map_err(|message| CompileError {
        line: None,
        message,
    })?;
    apply_sections(vm, &sections);
    vm.load_lua_source(&code, input, font)
        .map_err(|e| fc_vm::describe_lua_error(&e))
        .map_err(|(line, message)| CompileError { line, message })
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

/// Reads every asset region back out of VM RAM for embedding into `.lua`
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
