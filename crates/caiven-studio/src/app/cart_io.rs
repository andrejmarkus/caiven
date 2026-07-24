//! Cart saving: writes RAM-backed sections from the VM back to disk. A
//! `.cav` path writes the binary distribution cartridge; anything else is
//! treated as a project directory (the git-friendly authoring format).

use anyhow::{Context, Result};
use caiven_cart::{CartHeader, SectionKind};
use caiven_vm::Vm;
use std::path::PathBuf;

pub struct SectionLayout {
    pub kind: SectionKind,
    pub ram_base: usize,
    pub len: usize,
}

pub struct CartMeta {
    pub path: PathBuf,
    pub header: CartHeader,
    pub program: Vec<u8>,
    pub sections: Vec<SectionLayout>,
    pub lua_source: Option<String>,
}

/// Reads each tracked RAM asset section back from the VM.
fn gather_sections(vm: &Vm, meta: &CartMeta) -> Vec<(SectionKind, Vec<u8>)> {
    meta.sections
        .iter()
        .map(|s| {
            let bytes: Vec<u8> = (0..s.len).map(|i| vm.peek_memory(s.ram_base + i)).collect();
            (s.kind, bytes)
        })
        .collect()
}

/// Reads each tracked RAM section from the VM and writes them back to disk.
/// Only sections that were copied into RAM (e.g. SpriteSheet) are round-tripped.
/// Other section kinds are not currently written back.
pub(crate) fn save(vm: &Vm, meta: &CartMeta) -> Result<()> {
    let extra = gather_sections(vm, meta);
    let is_binary = meta.path.extension().and_then(|e| e.to_str()) == Some("cav");

    if is_binary {
        write_binary(&extra, meta, &meta.path)
    } else {
        let lua = meta.lua_source.as_deref().unwrap_or_default();
        caiven_cart::save_project(&meta.path, &meta.header, lua, &extra)
            .with_context(|| format!("failed to write project to {}", meta.path.display()))
    }
}

/// Builds a binary `.cav` cartridge at `dest` from the VM's current RAM
/// sections, regardless of where `meta.path` (the project dir) lives. Used
/// by the "Export Cartridge" action to produce a distribution artifact
/// without disturbing the project's own save location.
pub(crate) fn export_binary(vm: &Vm, meta: &CartMeta, dest: &std::path::Path) -> Result<()> {
    let extra = gather_sections(vm, meta);
    write_binary(&extra, meta, dest)
}

fn write_binary(
    extra: &[(SectionKind, Vec<u8>)],
    meta: &CartMeta,
    dest: &std::path::Path,
) -> Result<()> {
    let mut extra = extra.to_vec();
    let program: &[u8] = match &meta.lua_source {
        Some(src) => {
            extra.push((SectionKind::LuaSource, src.clone().into_bytes()));
            &[]
        }
        None => &meta.program,
    };
    caiven_cart::write(dest, &meta.header, program, &extra)
        .with_context(|| format!("failed to write cart to {}", dest.display()))
}
